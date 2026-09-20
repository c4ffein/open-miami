//! Drawing the props: the shared palette + primitives, the `draw_prop_ex`
//! driver and the per-layer dispatcher. The props themselves live one file per
//! FAMILY (`datacenter`, `outdoor`, `lobby`). Drawing only RECORDS into
//! `Graphics`, so all of it builds natively too (headless recording — see
//! tests/render_stream.rs).

mod datacenter;
mod lobby;
mod outdoor;

use datacenter::*;
use lobby::*;
use outdoor::*;

use super::{
    gate_angle, prop_px, rot_box, snap_box, turnstile_angle, PixelMode, PropDrawOpts, MAX_LAYERS,
    PROP_COUNT, PROP_LAYERS,
};
use crate::graphics::Graphics;
use crate::math::{Color, Vec2};
use std::f32::consts::{FRAC_PI_2, PI, TAU};

// ---- the shared datacenter palette --------------------------------------
const STEEL_DARK: Color = Color::new(0.15, 0.14, 0.20, 1.0);
const STEEL: Color = Color::new(0.24, 0.22, 0.30, 1.0);
const TRIM: Color = Color::new(0.38, 0.34, 0.46, 1.0);
const PANEL: Color = Color::new(0.08, 0.07, 0.12, 1.0);
const LED_GREEN: Color = Color::new(0.30, 1.0, 0.55, 1.0);
const LED_AMBER: Color = Color::new(1.0, 0.72, 0.20, 1.0);
const LED_RED: Color = Color::new(1.0, 0.22, 0.28, 1.0);
const GLOW_CYAN: Color = Color::new(0.30, 0.95, 1.0, 1.0);
const GLOW_MAGENTA: Color = Color::new(0.95, 0.28, 0.78, 1.0);
const CREAM: Color = Color::new(0.96, 0.93, 0.86, 1.0);
const HAZARD_YELLOW: Color = Color::new(0.95, 0.78, 0.10, 1.0);
const COPPER: Color = Color::new(0.62, 0.38, 0.28, 1.0);
const SHADOW: Color = Color::new(0.0, 0.0, 0.0, 0.30);

// ---- tiny drawing / animation helpers -----------------------------------
fn rect(g: &Graphics, x: f32, y: f32, w: f32, h: f32, c: Color) {
    g.draw_rectangle(Vec2::new(x, y), w, h, c);
}
fn frame(g: &Graphics, x: f32, y: f32, w: f32, h: f32, t: f32, c: Color) {
    g.draw_rectangle_lines(Vec2::new(x, y), w, h, t, c);
}
fn circle(g: &Graphics, x: f32, y: f32, r: f32, c: Color) {
    g.draw_circle(Vec2::new(x, y), r, c);
}
fn line(g: &Graphics, x1: f32, y1: f32, x2: f32, y2: f32, th: f32, c: Color) {
    g.draw_line(Vec2::new(x1, y1), Vec2::new(x2, y2), th, c);
}
fn alpha(c: Color, a: f32) -> Color {
    Color::new(c.r, c.g, c.b, a)
}
/// Drop shadow under a tall box footprint (light from the top-left).
fn shadow_rect(g: &Graphics, x: f32, y: f32, w: f32, h: f32) {
    rect(g, x + 4.0, y + 4.0, w, h, SHADOW);
}
/// Drop shadow under a tall round footprint.
fn shadow_circle(g: &Graphics, x: f32, y: f32, r: f32) {
    circle(g, x + 4.0, y + 4.0, r, SHADOW);
}
/// Small deterministic hash -> 0..1, for per-cell variety that stays put.
fn rnd(a: u32, b: u32) -> f32 {
    let mut x = a
        .wrapping_mul(374_761_393)
        .wrapping_add(b.wrapping_mul(668_265_263));
    x = (x ^ (x >> 13)).wrapping_mul(1_274_126_177);
    ((x ^ (x >> 16)) & 0xff_ffff) as f32 / 0xff_ffff as f32
}
/// Square-wave blink with `duty` fraction on, at `hz`, offset by `phase`.
fn blink(time: f32, hz: f32, phase: f32, duty: f32) -> bool {
    (time * hz + phase).fract() < duty
}
/// A fan set into a top panel, seen from above, in ITS OWN frame (the
/// layer's rotation spins it): dark well, four blades, hub.
fn fan(g: &Graphics, r: f32) {
    circle(g, 0.0, 0.0, r, PANEL);
    for k in 0..4 {
        let a = k as f32 * FRAC_PI_2;
        g.draw_arc(Vec2::new(0.0, 0.0), r - 1.5, a, a + 0.62, STEEL_DARK);
    }
    circle(g, 0.0, 0.0, r * 0.24, TRIM);
}

// ---- the OUTDOOR / LOBBY additions to the palette -----------------------
const PEARL: Color = Color::new(0.85, 0.86, 0.90, 1.0);
const TEAL: Color = Color::new(0.14, 0.36, 0.44, 1.0);
const ORANGE: Color = Color::new(0.90, 0.46, 0.16, 1.0);
const TYRE: Color = Color::new(0.06, 0.06, 0.08, 1.0);
const GLASS: Color = Color::new(0.10, 0.14, 0.20, 1.0);
const GLASS_HI: Color = Color::new(0.45, 0.65, 0.80, 1.0);
const ASPHALT: Color = Color::new(0.11, 0.11, 0.14, 1.0);
const PAINT: Color = Color::new(0.90, 0.90, 0.86, 1.0);
const CONCRETE: Color = Color::new(0.52, 0.50, 0.48, 1.0);
const CONCRETE_DARK: Color = Color::new(0.36, 0.35, 0.34, 1.0);
const LEAF: Color = Color::new(0.18, 0.44, 0.24, 1.0);
const LEAF_LIGHT: Color = Color::new(0.30, 0.60, 0.34, 1.0);
const LEAF_DARK: Color = Color::new(0.10, 0.28, 0.16, 1.0);
const NEON_PINK: Color = Color::new(1.0, 0.32, 0.62, 1.0);
const WARM_LIGHT: Color = Color::new(1.0, 0.85, 0.60, 1.0);
const MARBLE: Color = Color::new(0.72, 0.68, 0.64, 1.0);
const MARBLE_DARK: Color = Color::new(0.58, 0.54, 0.52, 1.0);
const WALNUT: Color = Color::new(0.36, 0.24, 0.16, 1.0);
const WALNUT_LIGHT: Color = Color::new(0.48, 0.33, 0.22, 1.0);
const BRASS: Color = Color::new(0.80, 0.64, 0.30, 1.0);
const VELVET: Color = Color::new(0.62, 0.10, 0.20, 1.0);
const CHROME: Color = Color::new(0.72, 0.74, 0.80, 1.0);
const RUBBER: Color = Color::new(0.10, 0.10, 0.12, 1.0);

// ---- helpers shared by the OUTDOOR / LOBBY families ---------------------
/// A ring (outline circle) as short chords, so it works over any
/// background (a filled circle pair would need a known fill).
fn ring(g: &Graphics, x: f32, y: f32, r: f32, th: f32, c: Color) {
    let n = 20;
    for k in 0..n {
        let a0 = k as f32 / n as f32 * TAU;
        let a1 = (k + 1) as f32 / n as f32 * TAU;
        line(
            g,
            x + a0.cos() * r,
            y + a0.sin() * r,
            x + a1.cos() * r,
            y + a1.sin() * r,
            th,
            c,
        );
    }
}
/// The edge-on light wash of a screen / lamp: a beam that starts `w0`
/// wide at `y0` (centred on `cx`) and spreads as it fades over `len`
/// (negative `len` = towards -y), in four steps.
fn wash_v(g: &Graphics, cx: f32, y0: f32, w0: f32, len: f32, c: Color, a0: f32) {
    let steps = 4;
    let step = len / steps as f32;
    let spread = len.abs() * 0.12;
    for i in 0..steps {
        let t = i as f32;
        let y = y0 + t * step;
        let (ya, h) = if step >= 0.0 {
            (y, step)
        } else {
            (y + step, -step)
        };
        rect(
            g,
            cx - w0 / 2.0 - t * spread,
            ya,
            w0 + 2.0 * t * spread,
            h,
            alpha(c, a0 * (1.0 - t / steps as f32)),
        );
    }
}
/// A tall post seen from above (bollard, rope post): shadow, base plate,
/// cap.
fn post(g: &Graphics, x: f32, y: f32, r: f32, base: Color, cap: Color) {
    shadow_circle(g, x, y, r + 2.0);
    circle(g, x, y, r + 2.0, base);
    circle(g, x, y, r, cap);
    circle(g, x - r * 0.3, y - r * 0.3, r * 0.3, alpha(CREAM, 0.35));
}
/// A block of the neon glyph strip on a holo sign: pixel blocks
/// scrolling along the front face between `x0` and `x1` at row `y`.
fn glyph_strip(g: &Graphics, x0: f32, x1: f32, y: f32, time: f32, c: Color) {
    let span = x1 - x0;
    for k in 0..9u32 {
        let x = x0 + (time * 14.0 + k as f32 * 13.0).rem_euclid(span);
        let w = (3.0 + rnd(k, 2) * 4.0).min(x1 - x);
        let h = 2.0 + rnd(k, 4) * 3.0;
        let col = if rnd(k, 6) > 0.7 { CREAM } else { c };
        rect(g, x, y + 5.0 - h, w, h, alpha(col, 0.85));
    }
}

/// Draw prop `idx` (see [`super::PROP_NAMES`]) centred on `center` at
/// `size_px` px, animated by the continuous clock `time` (seconds), with
/// its SAVED art-pixel size and layer modes (`props/props.json`).
pub fn draw_prop(g: &Graphics, idx: usize, center: Vec2, size_px: f32, time: f32) {
    let idx = idx % PROP_COUNT;
    draw_prop_ex(
        g,
        idx,
        center,
        size_px,
        time,
        prop_px(idx),
        &PropDrawOpts::saved(idx),
    );
}

/// Draw prop `idx` layer by layer: each visible layer in its own frame
/// (pivot, rotation from its [`super::LayerRot`] at `time`), and — when
/// `px >= 2` (design units: an art pixel is `px / 100` of the prop's
/// box) — inside its own pixel-art group, rasterized BEFORE or AFTER its
/// rotation per `opts.modes` (see the module docs). `px <= 1` draws the
/// layers directly. This is the call a floor renderer makes.
pub fn draw_prop_ex(
    g: &Graphics,
    idx: usize,
    center: Vec2,
    size_px: f32,
    time: f32,
    px: u32,
    opts: &PropDrawOpts,
) {
    let idx = idx % PROP_COUNT;
    g.save();
    g.translate(center.x, center.y);
    let s = size_px / 100.0;
    g.scale(s, s);
    for (li, l) in PROP_LAYERS[idx].iter().enumerate().take(MAX_LAYERS) {
        if opts.visible & (1 << li) == 0 {
            continue;
        }
        let angle = l.rot.angle(time);
        g.save();
        g.translate(l.pivot.0, l.pivot.1);
        if px <= 1 {
            if angle != 0.0 {
                g.rotate(angle);
            }
            draw_prop_layer(g, idx, li, time);
        } else {
            let pxf = px as f32;
            match opts.modes[li] {
                PixelMode::Before => {
                    // Rotate, THEN open the group: the layer is rasterized
                    // unrotated on its own grid (the group resets the
                    // transform) and PIX_END's quad — drawn through the
                    // rotated outer transform — turns the pixel image.
                    if angle != 0.0 {
                        g.rotate(angle);
                    }
                    let (gx, gy, gw, gh) = snap_box(l.bounds, pxf);
                    g.pixel_begin(pxf, gw, gh);
                    g.translate(-gx, -gy);
                    draw_prop_layer(g, idx, li, time);
                    g.pixel_end(gx, gy);
                }
                PixelMode::After => {
                    // Open the group in the parent's frame (sized to hold
                    // the layer at any angle if it turns), rotate INSIDE
                    // it: re-rasterized on the parent's grid every frame.
                    let (gx, gy, gw, gh) = if l.rot.is_none() {
                        snap_box(l.bounds, pxf)
                    } else {
                        rot_box(l.bounds, pxf)
                    };
                    g.pixel_begin(pxf, gw, gh);
                    g.translate(-gx, -gy);
                    if angle != 0.0 {
                        g.rotate(angle);
                    }
                    draw_prop_layer(g, idx, li, time);
                    g.pixel_end(gx, gy);
                }
            }
        }
        g.restore();
    }
    g.restore();
}

/// Draw layer `layer` of prop `idx` in the layer's own frame: origin at
/// its pivot, unrotated (the caller applies the [`super::LayerRot`]).
pub fn draw_prop_layer(g: &Graphics, idx: usize, layer: usize, time: f32) {
    match idx % PROP_COUNT {
        0 => rack_closed(g, layer, time),
        1 => rack_open(g, layer, time),
        2 => rack_burnt(g, layer, time),
        3 => blade_stack(g, layer, time),
        4 => core_switch(g, layer, time),
        5 => cable_junction(g, layer, time),
        6 => operator_desk(g, layer, time),
        7 => control_console(g, layer, time),
        8 => holo_table(g, layer, time),
        9 => crac_cooler(g, layer, time),
        10 => floor_vent(g, layer, time),
        11 => exhaust_fan(g, layer, time),
        12 => coolant_tank(g, layer, time),
        13 => pipe_run(g, layer, time),
        14 => ups_cabinet(g, layer, time),
        15 => generator(g, layer, time),
        16 => cable_tray(g, layer, time),
        17 => cable_coil(g, layer, time),
        18 => tape_library(g, layer, time),
        19 => supply_crate(g, layer, time),
        20 => security_cam(g, layer, time),
        21 => fire_suppressor(g, layer, time),
        22 => hazard_pad(g, layer, time),
        23 => uplink_obelisk(g, layer, time),
        24 => car_pod(g, layer, time),
        25 => car_sedan(g, layer, time),
        26 => car_open(g, layer, time),
        27 => delivery_van(g, layer, time),
        28 => charge_pad(g, layer, time),
        29 => car_charging(g, layer, time),
        30 => main_gate(g, layer, time),
        31 => guard_booth(g, layer, time),
        32 => bollards(g, layer, time),
        33 => planter(g, layer, time),
        34 => lamp_post(g, layer, time),
        35 => ev_bay(g, layer, time),
        36 => crosswalk(g, layer, time),
        37 => drone_pad(g, layer, time),
        38 => scooter_rack(g, layer, time),
        39 => drain_grate(g, layer, time),
        40 => holo_billboard(g, layer, time),
        41 => dumpster(g, layer, time),
        42 => reception_desk(g, layer, time),
        43 => turnstiles(g, layer, time),
        44 => scanner_arch(g, layer, time),
        45 => bench_long(g, layer, time),
        46 => bench_short(g, layer, time),
        47 => potted_plant(g, layer, time),
        48 => lobby_holo(g, layer, time),
        49 => directory_totem(g, layer, time),
        50 => vending_machine(g, layer, time),
        51 => coffee_corner(g, layer, time),
        52 => charge_lockers(g, layer, time),
        53 => floor_logo(g, layer, time),
        54 => call_panel(g, layer, time),
        55 => velvet_rope(g, layer, time),
        56 => extinguisher(g, layer, time),
        57 => credit_kiosk(g, layer, time),
        58 => wall_clock(g, layer, time),
        _ => welcome_mat(g, layer, time),
    }
}

// ======================= OUTDOOR: gate / parking lot =======================

/// Linear blend of two colours.
fn mix(a: Color, b: Color, t: f32) -> Color {
    Color::new(
        a.r + (b.r - a.r) * t,
        a.g + (b.g - a.g) * t,
        a.b + (b.b - a.b) * t,
        a.a + (b.a - a.a) * t,
    )
}

// ========================= LOBBY: the welcome hall =========================
