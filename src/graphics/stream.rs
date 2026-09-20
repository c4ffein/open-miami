//! Reading a recorded frame back: the opcode ARITY table, a walker, a
//! structural validator and the mirror of renderer.js's 2D transform stack.
//!
//! This is the host-side half of the wasm->JS contract. `Graphics` records a
//! flat f32 stream; renderer.js walks it with its own `OP_ARGS` table. The
//! same walk here lets a native test record any draw code through
//! `Graphics::new_headless` and assert on what the renderer would receive —
//! no browser. The tests pin this table to `web/ops.js` (names, values,
//! arities) and to the `case` labels of renderer.js's dispatch.

use super::{op, TEXT_SEP};
use crate::static_geo::{OP_STATIC_BEGIN, OP_STATIC_END, OP_STATIC_REF};

/// Number of opcodes (the highest opcode + 1).
pub const OP_COUNT: usize = 26;

/// Argument count per opcode (index = opcode). Mirror of the `TABLE` in
/// web/ops.js — the tests below parse that file and compare.
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
    18, // 11 ROBOT
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

    /// Every Rust opcode with the name `web/ops.js` must give it.
    const NAMED: [(f32, &str); OP_COUNT] = [
        (op::CLEAR, "CLEAR"),
        (op::RECT, "RECT"),
        (op::RECT_LINES, "RECT_LINES"),
        (op::CIRCLE, "CIRCLE"),
        (op::LINE, "LINE"),
        (op::ARC, "ARC"),
        (op::TEXT, "TEXT"),
        (op::SAVE, "SAVE"),
        (op::RESTORE, "RESTORE"),
        (op::TRANSLATE, "TRANSLATE"),
        (op::ROTATE, "ROTATE"),
        (op::ROBOT, "ROBOT"),
        (op::SCALE, "SCALE"),
        (op::SHOGGOTH, "SHOGGOTH"),
        (op::POSTFX, "POSTFX"),
        (op::PIX_BEGIN, "PIX_BEGIN"),
        (op::PIX_END, "PIX_END"),
        (op::PORTRAIT, "PORTRAIT"),
        (op::GUN_PICKUP, "GUN_PICKUP"),
        (op::PIX_BLIT, "PIX_BLIT"),
        (op::DRIVE, "DRIVE"),
        (OP_STATIC_BEGIN, "STATIC_BEGIN"),
        (OP_STATIC_END, "STATIC_END"),
        (OP_STATIC_REF, "STATIC_REF"),
        (op::BACKDROP, "BACKDROP"),
        (op::HEAD, "HEAD"),
    ];

    /// The `["NAME", args],` rows of `web/ops.js`, in order (row = opcode).
    fn js_table() -> Vec<(String, usize)> {
        let src = include_str!("../../web/ops.js");
        let body = &src[src.find("const TABLE = [").expect("TABLE in web/ops.js")..];
        let body = &body[..body.find("\n];").expect("unterminated TABLE")];
        body.lines()
            .filter_map(|l| l.trim().strip_prefix("[\""))
            .map(|row| {
                let (name, rest) = row.split_once('"').expect("a quoted name");
                let args = rest.trim_start_matches(',').trim();
                let args = &args[..args.find(']').expect("a closed row")];
                (name.to_string(), args.trim().parse().expect("an arity"))
            })
            .collect()
    }

    #[test]
    fn web_ops_js_matches_the_rust_table() {
        let js = js_table();
        assert_eq!(js.len(), OP_COUNT, "web/ops.js row count");
        for (value, name) in NAMED {
            let row = &js[value as usize];
            assert_eq!(row.0, name, "web/ops.js row {value} is named {}", row.0);
            assert_eq!(row.1, OP_ARGS[value as usize], "arity of {name}");
        }
    }

    /// The arity table may be written down in exactly ONE JS place, web/ops.js.
    /// A pasted copy keeps "working" until the day an opcode's arity changes,
    /// then silently desyncs that file's stream walk (it happened: two render
    /// scripts carried one inside `page.evaluate`). Build the telltale from the
    /// real table and look for it everywhere JS lives.
    #[test]
    fn no_js_file_pastes_the_arity_table() {
        let needle: Vec<String> = OP_ARGS[..10].iter().map(|n| n.to_string()).collect();
        let needle = needle.join(", ");
        let mut stack: Vec<std::path::PathBuf> = [
            "web",
            "tools",
            "tests/e2e",
            "index.html",
            "render-tests.html",
        ]
        .iter()
        .map(Into::into)
        .collect();
        let mut scanned = 0;
        while let Some(path) = stack.pop() {
            if path.is_dir() {
                let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
                if !matches!(
                    name,
                    "node_modules" | "test-results" | "playwright-report" | "playwright-deps"
                ) {
                    stack.extend(std::fs::read_dir(&path).unwrap().map(|e| e.unwrap().path()));
                }
                continue;
            }
            let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
            if !matches!(ext, "js" | "mjs" | "ts" | "html") || path.ends_with("web/ops.js") {
                continue;
            }
            scanned += 1;
            let text = std::fs::read_to_string(&path).unwrap_or_default();
            assert!(
                !text.contains(&needle),
                "{} pastes the opcode arity table — read it from web/ops.js instead \
                 (tests: `require('../ops')`, see tests/e2e/ops.js)",
                path.display()
            );
        }
        assert!(
            scanned > 10,
            "the scan found almost no files ({scanned}): wrong cwd?"
        );
    }

    /// renderer.js dispatches on numeric literals (`case 18: // GUN_PICKUP`):
    /// every label must agree with the table, and every opcode have a case.
    #[test]
    fn renderer_js_dispatch_matches_the_table() {
        let src = include_str!("../../web/renderer.js");
        let body = &src[src.find("switch (op) {").expect("the dispatch switch")..];
        let mut seen = [false; OP_COUNT];
        for line in body.lines() {
            let Some(rest) = line.trim().strip_prefix("case ") else {
                continue;
            };
            let Some((num, tail)) = rest.split_once(':') else {
                continue;
            };
            let Ok(value) = num.trim().parse::<usize>() else {
                continue;
            };
            let label = tail.split("//").nth(1).unwrap_or("").trim();
            let label = label.split([' ', '(']).next().unwrap_or("");
            assert!(value < OP_COUNT, "renderer.js has a case {value}");
            assert_eq!(label, NAMED[value].1, "renderer.js `case {value}` label");
            seen[value] = true;
        }
        assert!(seen.iter().all(|&s| s), "an opcode has no case: {seen:?}");
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
        let pose = crate::render::pose::pose_plan(crate::render::pose::PoseKind::Walk, 1.0, false);
        g.draw_robot(0, 2, p, 0.0, 64.0, &pose);
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
