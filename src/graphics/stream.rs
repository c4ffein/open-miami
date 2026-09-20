//! Reading a recorded frame back: the opcode ARITY table, a walker, a
//! structural validator and the mirror of renderer.js's 2D transform stack.
//!
//! This is the host-side half of the wasm->JS contract. `Graphics` records a
//! flat f32 stream; renderer.js walks it with its own `OP_ARGS` table. The
//! same walk here lets a native test record any draw code through
//! `Graphics::new_headless` and assert on what the renderer would receive —
//! no browser. `renderer_js_op_args_match` pins the two tables together.

use super::{op, TEXT_SEP};
use crate::static_geo::{OP_STATIC_BEGIN, OP_STATIC_END, OP_STATIC_REF};

/// Number of opcodes (the highest opcode + 1).
pub const OP_COUNT: usize = 26;

/// Argument count per opcode (index = opcode). Mirror of `OP_ARGS` in
/// renderer.js — the test below parses that file and compares.
pub const OP_ARGS: [usize; OP_COUNT] = [
    4,  // 0 CLEAR
    8,  // 1 RECT
    9,  // 2 RECT_LINES
    7,  // 3 CIRCLE
    9,  // 4 LINE
    9,  // 5 ARC
    8,  // 6 TEXT
    0,  // 7 SAVE
    0,  // 8 RESTORE
    2,  // 9 TRANSLATE
    1,  // 10 ROTATE
    8,  // 11 ROBOT
    2,  // 12 SCALE
    6,  // 13 SHOGGOTH
    5,  // 14 POSTFX
    4,  // 15 PIX_BEGIN
    2,  // 16 PIX_END
    6,  // 17 PORTRAIT
    5,  // 18 GUN_PICKUP
    6,  // 19 PIX_BLIT
    16, // 20 DRIVE
    1,  // 21 STATIC_BEGIN
    0,  // 22 STATIC_END
    1,  // 23 STATIC_REF
    8,  // 24 BACKDROP
    5,  // 25 HEAD
];

/// Pixel-art groups nest at most this deep (renderer.js `PIX_DEPTH`).
pub const PIX_DEPTH: usize = 4;

/// One decoded command: the opcode and its argument slice.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Cmd<'a> {
    pub op: f32,
    pub args: &'a [f32],
}

/// Decode a stream into commands. Errors on an unknown opcode or a command
/// truncated by the end of the stream — exactly what would desync the
/// renderer's walk.
pub fn walk(cmds: &[f32]) -> Result<Vec<Cmd<'_>>, String> {
    let mut out = Vec::new();
    let mut i = 0;
    while i < cmds.len() {
        let opf = cmds[i];
        if opf < 0.0 || opf.fract() != 0.0 || opf as usize >= OP_COUNT {
            return Err(format!("unknown opcode {opf} at float {i}"));
        }
        let n = OP_ARGS[opf as usize];
        if i + 1 + n > cmds.len() {
            return Err(format!("opcode {opf} at float {i} is truncated"));
        }
        out.push(Cmd {
            op: opf,
            args: &cmds[i + 1..i + 1 + n],
        });
        i += 1 + n;
    }
    Ok(out)
}

/// Validate a whole frame the way the renderer depends on it: it decodes,
/// every float is finite, SAVE/RESTORE balance (never popping an empty
/// stack), pixel groups balance and nest within [`PIX_DEPTH`], static
/// sections are framed and hold SOLID primitives only, and every TEXT index
/// points into the text arena. Returns the decoded commands.
pub fn check<'a>(cmds: &'a [f32], texts: &str) -> Result<Vec<Cmd<'a>>, String> {
    let decoded = walk(cmds)?;
    let text_count = if texts.is_empty() {
        0
    } else {
        texts.split(TEXT_SEP).count()
    };
    let mut saves: i32 = 0;
    // `saves` at each open PIX_BEGIN: a group's END discards the unbalanced
    // saves made inside it (see renderer.js pixEnd).
    let mut groups: Vec<i32> = Vec::new();
    let mut in_static = false;
    for (n, c) in decoded.iter().enumerate() {
        if let Some(bad) = c.args.iter().find(|v| !v.is_finite()) {
            return Err(format!("cmd {n} (op {}) has a non-finite arg {bad}", c.op));
        }
        let solid = [
            op::RECT,
            op::RECT_LINES,
            op::CIRCLE,
            op::LINE,
            op::ARC,
            op::SAVE,
            op::RESTORE,
            op::TRANSLATE,
            op::ROTATE,
            op::SCALE,
        ];
        if in_static && c.op != OP_STATIC_END && !solid.contains(&c.op) {
            return Err(format!("cmd {n}: op {} inside a static section", c.op));
        }
        if c.op == op::SAVE {
            saves += 1;
        } else if c.op == op::RESTORE {
            saves -= 1;
            if saves < groups.last().copied().unwrap_or(0) {
                return Err(format!("cmd {n}: RESTORE without a matching SAVE"));
            }
        } else if c.op == op::PIX_BEGIN {
            if groups.len() == PIX_DEPTH {
                return Err(format!("cmd {n}: pixel groups nested past {PIX_DEPTH}"));
            }
            if c.args[0] <= 0.0 || c.args[1] <= 0.0 || c.args[2] <= 0.0 {
                return Err(format!("cmd {n}: PIX_BEGIN with px/w/h <= 0"));
            }
            groups.push(saves);
        } else if c.op == op::PIX_END {
            match groups.pop() {
                Some(at_begin) => saves = at_begin,
                None => return Err(format!("cmd {n}: PIX_END without a PIX_BEGIN")),
            }
        } else if c.op == OP_STATIC_BEGIN {
            if in_static {
                return Err(format!("cmd {n}: nested STATIC_BEGIN"));
            }
            in_static = true;
        } else if c.op == OP_STATIC_END {
            if !in_static {
                return Err(format!("cmd {n}: STATIC_END without a STATIC_BEGIN"));
            }
            in_static = false;
        } else if c.op == OP_STATIC_REF && in_static {
            return Err(format!("cmd {n}: STATIC_REF inside a static section"));
        } else if c.op == op::TEXT {
            let idx = c.args[0];
            if idx < 0.0 || idx.fract() != 0.0 || idx as usize >= text_count {
                return Err(format!("cmd {n}: TEXT index {idx} of {text_count}"));
            }
        }
    }
    if saves != 0 {
        return Err(format!(
            "{saves} unbalanced SAVE(s) at the end of the frame"
        ));
    }
    if !groups.is_empty() {
        return Err(format!("{} pixel group(s) left open", groups.len()));
    }
    if in_static {
        return Err("static section left open".into());
    }
    Ok(decoded)
}

/// renderer.js's 2D affine (`tTranslate` / `tScale` / `tRotate`, same
/// column layout): `x' = a x + c y + e`, `y' = b x + d y + f`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Affine(pub [f32; 6]);

impl Affine {
    pub const IDENTITY: Affine = Affine([1.0, 0.0, 0.0, 1.0, 0.0, 0.0]);

    pub fn translate(&mut self, x: f32, y: f32) {
        let m = &mut self.0;
        m[4] += m[0] * x + m[2] * y;
        m[5] += m[1] * x + m[3] * y;
    }

    pub fn scale(&mut self, sx: f32, sy: f32) {
        let m = &mut self.0;
        m[0] *= sx;
        m[1] *= sx;
        m[2] *= sy;
        m[3] *= sy;
    }

    pub fn rotate(&mut self, angle: f32) {
        let (s, c) = angle.sin_cos();
        let m = &mut self.0;
        let (a0, b0, c0, d0) = (m[0], m[1], m[2], m[3]);
        m[0] = a0 * c + c0 * s;
        m[1] = b0 * c + d0 * s;
        m[2] = -a0 * s + c0 * c;
        m[3] = -b0 * s + d0 * c;
    }

    pub fn apply(&self, x: f32, y: f32) -> (f32, f32) {
        let m = &self.0;
        (m[0] * x + m[2] * y + m[4], m[1] * x + m[3] * y + m[5])
    }
}

/// The transform in force after running every transform op of `cmds` from
/// identity (SAVE / RESTORE honoured; pixel groups are not entered — use it
/// on camera-level streams).
pub fn final_transform(cmds: &[Cmd<'_>]) -> Affine {
    let mut m = Affine::IDENTITY;
    let mut stack = Vec::new();
    for c in cmds {
        if c.op == op::SAVE {
            stack.push(m);
        } else if c.op == op::RESTORE {
            if let Some(top) = stack.pop() {
                m = top;
            }
        } else if c.op == op::TRANSLATE {
            m.translate(c.args[0], c.args[1]);
        } else if c.op == op::ROTATE {
            m.rotate(c.args[0]);
        } else if c.op == op::SCALE {
            m.scale(c.args[0], c.args[1]);
        }
    }
    m
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graphics::Graphics;
    use crate::math::{Color, Vec2};

    /// Parse `const OP_ARGS = [ ... ];` out of a JS source.
    fn js_op_args(src: &str, file: &str) -> Vec<usize> {
        let at = src
            .find("const OP_ARGS = [")
            .unwrap_or_else(|| panic!("no `const OP_ARGS = [` in {file}"));
        let body = &src[at + "const OP_ARGS = [".len()..];
        let body = &body[..body.find(']').expect("unterminated OP_ARGS")];
        body.split(',')
            .map(|v| v.trim().parse().expect("OP_ARGS entry is not a number"))
            .collect()
    }

    #[test]
    fn renderer_js_op_args_match() {
        let js = js_op_args(include_str!("../../renderer.js"), "renderer.js");
        assert_eq!(
            js, OP_ARGS,
            "renderer.js OP_ARGS != graphics::stream::OP_ARGS"
        );
    }

    #[test]
    fn e2e_helpers_op_args_match() {
        let src = include_str!("../../tests/e2e/specs/helpers.js");
        assert_eq!(js_op_args(src, "helpers.js"), OP_ARGS);
    }

    /// Every `Graphics` draw method, called once each.
    fn record_everything(g: &Graphics) {
        let (p, c) = (Vec2::new(1.0, 2.0), Color::new(0.1, 0.2, 0.3, 1.0));
        g.clear(c);
        g.draw_rectangle(p, 3.0, 4.0, c);
        g.draw_rectangle_lines(p, 3.0, 4.0, 1.0, c);
        g.draw_circle(p, 5.0, c);
        g.draw_line(p, p, 1.0, c);
        g.draw_arc(p, 5.0, 0.0, 1.0, c);
        g.draw_text("Health: 3", p, 20.0, c);
        g.save();
        g.translate(1.0, 2.0);
        g.rotate(0.5);
        g.scale(2.0, 2.0);
        g.restore();
        g.draw_robot(0, 1, 2, p, 0.0, 64.0, 1.0);
        g.draw_shoggoth_live(p, 128.0, 0.0, 0.5, 1.0);
        g.postfx(13, 0.075, c);
        g.pixel_begin(2.0, 10.0, 10.0);
        g.pixel_end(0.0, 0.0);
        g.pixel_begin_smooth(2.0, 10.0, 10.0);
        g.pixel_end(0.0, 0.0);
        g.draw_robot_portrait(0, p, 64.0, 0.0, 1);
        g.draw_gun_pickup(1, p, 0.0, 32.0);
        g.pixel_blit(0.0, 0.0, 1.0, 1.0, 0.0, 0.0);
        g.drive(960.0, 720.0, 0.0, 0.0, 0.0, 4.0, 0.0, &[0.0; 9]);
        g.static_layer(1, || g.draw_rectangle(p, 1.0, 1.0, c));
        g.static_layer(1, || panic!("a cached key must not re-record"));
        g.backdrop(960.0, 720.0, 0.0, 6.0, Some([1.0, 2.0, 3.0, 4.0]));
        g.draw_head(0, p, 0.0, 16.0);
        g.draw_shoggoth(p, 20.0, false);
        g.draw_shoggoth(p, 20.0, true);
        g.draw_pixelated_sprite(p, 0.0, c, false);
    }

    #[test]
    fn every_draw_method_emits_its_declared_arity() {
        let g = Graphics::new_headless(960.0, 720.0);
        record_everything(&g);
        let frame = g.take_frame();
        let cmds = check(&frame.cmds, &frame.texts).expect("valid frame");
        // Every opcode of the table is reachable through the public API.
        for opcode in 0..OP_COUNT {
            assert!(
                cmds.iter().any(|c| c.op == opcode as f32),
                "no draw method emitted opcode {opcode}"
            );
        }
    }

    #[test]
    fn text_is_uppercased_at_the_boundary_and_indexed() {
        let g = Graphics::new_headless(100.0, 100.0);
        g.draw_text("Health: 3", Vec2::zero(), 10.0, Color::BLACK);
        g.draw_text("a → b", Vec2::zero(), 10.0, Color::BLACK);
        let frame = g.take_frame();
        let texts: Vec<&str> = frame.texts.split(TEXT_SEP).collect();
        assert_eq!(texts, ["HEALTH: 3", "A → B"]);
        let cmds = check(&frame.cmds, &frame.texts).unwrap();
        assert_eq!(cmds[0].args[0], 0.0);
        assert_eq!(cmds[1].args[0], 1.0);
    }

    #[test]
    fn take_frame_resets_the_recorder() {
        let g = Graphics::new_headless(100.0, 100.0);
        g.draw_text("one", Vec2::zero(), 10.0, Color::BLACK);
        assert!(!g.take_frame().cmds.is_empty());
        g.draw_text("two", Vec2::zero(), 10.0, Color::BLACK);
        let frame = g.take_frame();
        assert_eq!(frame.texts, "TWO");
        assert_eq!(walk(&frame.cmds).unwrap()[0].args[0], 0.0);
    }

    #[test]
    fn check_rejects_what_would_desync_the_renderer() {
        assert!(walk(&[99.0]).is_err(), "unknown opcode");
        assert!(walk(&[op::RECT, 1.0]).is_err(), "truncated");
        assert!(check(&[op::RESTORE], "").is_err(), "pop of an empty stack");
        assert!(check(&[op::SAVE], "").is_err(), "unbalanced save");
        assert!(check(&[op::PIX_END, 0.0, 0.0], "").is_err());
        assert!(check(&[op::PIX_BEGIN, 2.0, 8.0, 8.0, 0.0], "").is_err());
        assert!(check(&[op::TRANSLATE, f32::NAN, 0.0], "").is_err());
        let text = [op::TEXT, 0.0, 0.0, 0.0, 9.0, 1.0, 1.0, 1.0, 1.0];
        assert!(check(&text, "").is_err(), "index past the arena");
        assert!(check(&text, "HI").is_ok());
        // A group END discards the saves left open inside it.
        let group = [
            op::PIX_BEGIN,
            2.0,
            8.0,
            8.0,
            0.0,
            op::SAVE,
            op::PIX_END,
            0.0,
            0.0,
        ];
        assert!(check(&group, "").is_ok());
        // Sprites / text may not be baked into the static cache.
        let mut bad = vec![OP_STATIC_BEGIN, 1.0];
        bad.extend_from_slice(&text);
        bad.push(OP_STATIC_END);
        assert!(check(&bad, "HI").is_err());
    }
}
