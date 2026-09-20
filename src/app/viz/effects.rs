//! The EFFECTS tab: every POSTFX shader kind + the 2D shoggoth glitch.

use super::*;

/// EFFECTS tab: the POSTFX shader menu — (kind, label, preview peak `t`,
/// colour). Mirrors the kind table in renderer.js / `Graphics::postfx`.
/// Peak `t` stays below 1 where full strength would blank the frame
/// (BLUR-OUT at t = 1 is a solid colour).
pub(crate) const POSTFX_PREVIEWS: [(u32, &str, f32, Color); 14] = [
    (0, "BLUR-OUT", 0.8, Color::new(0.05, 0.02, 0.10, 1.0)),
    (1, "SYNTHWAVE CRT", 1.0, Color::new(1.0, 0.25, 0.65, 1.0)),
    (2, "VHS TAPE", 1.0, Color::new(0.60, 0.60, 0.90, 1.0)),
    (3, "DRUNK SWAY", 1.0, Color::new(0.60, 0.20, 0.80, 1.0)),
    (4, "CRT TUBE", 1.0, Color::new(0.20, 0.90, 0.90, 1.0)),
    (5, "ACID TRIP", 1.0, Color::new(0.90, 0.30, 0.90, 1.0)),
    (6, "DATAMOSH", 1.0, Color::new(0.30, 0.90, 0.50, 1.0)),
    (7, "NEON BLOOM", 1.0, Color::new(0.55, 0.10, 0.60, 1.0)),
    (8, "PIXEL MOSAIC", 1.0, Color::new(0.90, 0.80, 0.30, 1.0)),
    (9, "TUNNEL RUSH", 1.0, Color::new(1.0, 0.40, 0.20, 1.0)),
    (10, "WARP TRAILS", 1.0, ending::WARP_TINT),
    (11, "UI GREY", 0.8, Color::new(0.80, 0.82, 0.90, 1.0)),
    // r/g = the demo modal's half extents (fractions of the screen).
    (12, "MODAL STATIC", 0.9, Color::new(0.25, 0.22, 0.0, 1.0)),
    (13, "TV STATIC", 0.3, Color::WHITE),
];

/// How long an EFFECTS-tab POSTFX preview plays (ramp in, hold, ramp out).
pub(crate) const POSTFX_PREVIEW_MS: f64 = 4000.0;

/// Deterministic (bucket, salt) -> 0..1 for the glitch effect: the same
/// hash the DRIVE's schedules use.
use crate::drive::hash01 as rand01;

/// Full-screen "shoggoth" glitch: live-cell tissue rendered as a pixelated
/// Voronoi field. The screen is scanned in chunky blocks; each block finds
/// its two nearest cell nuclei, and blocks nearly equidistant to both are
/// MEMBRANE — the wall *in between* neighbouring cells — drawn as a
/// green-or-black dithered pixel line. Cell interiors stay dark cytoplasm
/// with a small nucleus, and a subset of cells carry a blinking pale-yellow
/// eye. The nuclei re-seat to nearby spots every ~6 frames (~0.1s) so the
/// whole tissue squirms glitchily. 1.2s fade envelope; `elapsed_ms` is time
/// since the effect started.
pub(crate) fn draw_shoggoth_glitch(g: &Graphics, elapsed_ms: f32) {
    let (w, h) = (g.width(), g.height());
    let t = (elapsed_ms / 1200.0).clamp(0.0, 1.0);
    let env = if t < 0.1 {
        t / 0.1
    } else if t > 0.7 {
        ((1.0 - t) / 0.3).max(0.0)
    } else {
        1.0
    };

    // Dark takeover of the screen.
    g.draw_rectangle(
        Vec2::new(0.0, 0.0),
        w,
        h,
        Color::new(0.03, 0.03, 0.045, 0.9 * env),
    );

    // One nucleus (seed) per jittered grid cell; the layout re-seats every
    // ~6 frames, wobbling only to nearby spots — the glitchy squirm.
    let tick = (elapsed_ms / 100.0) as u32;
    let cols: i32 = 12;
    let rows: i32 = 9;
    let cw = w / cols as f32;
    let ch = h / rows as f32;
    let seed = |i: i32, j: i32| -> (f32, f32) {
        // wrap so blocks near the screen edge still see a full neighbourhood
        let (iw, jw) = (i.rem_euclid(cols), j.rem_euclid(rows));
        let id = (jw * cols + iw) as u32;
        let ax = (i as f32 + 0.5) * cw + (rand01(id, 3) - 0.5) * cw * 0.6;
        let ay = (j as f32 + 0.5) * ch + (rand01(id, 4) - 0.5) * ch * 0.6;
        let jx = (rand01(id, tick * 2 + 1) - 0.5) * cw * 0.22;
        let jy = (rand01(id, tick * 2 + 2) - 0.5) * ch * 0.22;
        (ax + jx, ay + jy)
    };

    // Pixelated scan: chunky blocks classified as membrane / nucleus / bg.
    let px = 10.0f32; // block size — the pixelization
    let membrane_w = 0.16; // boundary half-width (in nearest-distance ratio)
    let bx_n = (w / px).ceil() as i32;
    let by_n = (h / px).ceil() as i32;
    for byi in 0..by_n {
        for bxi in 0..bx_n {
            let cx = (bxi as f32 + 0.5) * px;
            let cy = (byi as f32 + 0.5) * px;
            let gi = (cx / cw).floor() as i32;
            let gj = (cy / ch).floor() as i32;
            // nearest + second-nearest nucleus over the 3x3 neighbourhood
            let (mut d1, mut d2) = (f32::MAX, f32::MAX);
            let mut best = (0i32, 0i32);
            for dj in -1..=1 {
                for di in -1..=1 {
                    let (sx, sy) = seed(gi + di, gj + dj);
                    let d = (sx - cx) * (sx - cx) + (sy - cy) * (sy - cy);
                    if d < d1 {
                        d2 = d1;
                        d1 = d;
                        best = (gi + di, gj + dj);
                    } else if d < d2 {
                        d2 = d;
                    }
                }
            }
            let (d1, d2) = (d1.sqrt(), d2.sqrt());
            // Membrane: this block sits on the wall BETWEEN two cells.
            if d2 - d1 < membrane_w * (d1 + d2) {
                // green-or-black dither, re-rolled with the glitch tick
                let roll = rand01((bxi * 977 + byi) as u32, tick + 41);
                let c = if roll > 0.45 {
                    Color::new(0.12, 0.55, 0.30, 0.95 * env) // membrane green
                } else {
                    Color::new(0.01, 0.05, 0.03, 0.95 * env) // membrane black
                };
                g.draw_rectangle(Vec2::new(bxi as f32 * px, byi as f32 * px), px, px, c);
            } else if d1 < cw.min(ch) * 0.16 {
                // Nucleus kernel at the middle of each cell.
                let id = (best.1.rem_euclid(rows) * cols + best.0.rem_euclid(cols)) as u32;
                let eyed = rand01(id, tick / 3 + 9) > 0.7;
                let c = if eyed {
                    let blink = 0.6 + 0.4 * rand01(id, tick + 1);
                    Color::new(1.0, 0.93, 0.5, blink * env) // pale-yellow eye
                } else {
                    Color::new(0.38, 0.15, 0.38, 0.9 * env) // plain nucleus
                };
                g.draw_rectangle(Vec2::new(bxi as f32 * px, byi as f32 * px), px, px, c);
            }
            // everything else stays dark cytoplasm (the takeover wash).
        }
    }
}

impl GameState {
    /// EFFECTS tab: trigger a full-screen effect to preview it. The POSTFX
    /// rows are the WebGL post shaders (played over this very pane for 4s,
    /// ramp in / hold / ramp out); below them, the 2D command-stream
    /// effects (1.2s).
    pub(crate) fn draw_viz_effects(&mut self, graphics: &Graphics, mouse: Vec2, click: bool) {
        let coral = Color::from_rgba(217, 119, 87, 255);
        let elapsed = self.last_time - self.effect_start;
        graphics.draw_text(
            "Full-screen effects. Click one to preview it over this pane.",
            Vec2::new(40.0, 96.0),
            18.0,
            Color::GRAY,
        );

        graphics.draw_text(
            "POST SHADERS (WebGL, POSTFX opcode)",
            Vec2::new(40.0, 126.0),
            16.0,
            coral,
        );
        for (i, &(_, name, _, _)) in POSTFX_PREVIEWS.iter().enumerate() {
            let x = 40.0 + (i % 4) as f32 * 178.0;
            let y = 138.0 + (i / 4) as f32 * 52.0;
            let active = self.effect_kind == i as i32
                && self.effect_start > 0.0
                && (0.0..POSTFX_PREVIEW_MS).contains(&elapsed);
            if viz_button(graphics, mouse, x, y, 168.0, 46.0, name, active) && click {
                self.effect_kind = i as i32;
                self.effect_start = self.last_time;
            }
        }

        let y2 = 138.0 + 3.0 * 52.0 + 18.0;
        graphics.draw_text(
            "COMMAND-STREAM EFFECTS (2D)",
            Vec2::new(40.0, y2),
            16.0,
            coral,
        );
        let active =
            self.effect_kind < 0 && self.effect_start > 0.0 && (0.0..1200.0).contains(&elapsed);
        if viz_button(
            graphics,
            mouse,
            40.0,
            y2 + 12.0,
            240.0,
            46.0,
            "Shoggoth glitch",
            active,
        ) && click
        {
            self.effect_kind = -1;
            self.effect_start = self.last_time;
        }

        // DRIVE — the glitchy synthwave backdrop behind the title screen
        // (src/drive.rs; the ending rides home in `ending::render_ride`),
        // previewed live at moderate glitch.
        let (dx0, dy0, dw, dh) = (320.0, y2 + 12.0, 600.0, 300.0);
        graphics.draw_text(
            "DRIVE (the title backdrop, live, glitch 0.5)",
            Vec2::new(dx0, y2),
            16.0,
            coral,
        );
        graphics.draw_rectangle(
            Vec2::new(dx0, dy0),
            dw,
            dh,
            Color::new(0.02, 0.01, 0.04, 1.0),
        );
        graphics.save();
        graphics.translate(dx0, dy0);
        crate::drive::render_drive(graphics, dw, dh, (self.last_time / 1000.0) as f32, 0.5, 0.0);
        graphics.restore();
        graphics.draw_rectangle_lines(Vec2::new(dx0, dy0), dw, dh, 2.0, coral);
    }
}
