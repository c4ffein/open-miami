//! The OUTDOOR family: props 24..41, one `fn(g, layer, time)` each.

use super::*;

/// The same wash sideways: from `x0`, `h0` tall (centred on `cy`),
/// spreading over `len` towards +x (negative = -x).
pub(super) fn wash_h(g: &Graphics, x0: f32, cy: f32, h0: f32, len: f32, c: Color, a0: f32) {
    let steps = 4;
    let step = len / steps as f32;
    let spread = len.abs() * 0.12;
    for i in 0..steps {
        let t = i as f32;
        let x = x0 + t * step;
        let (xa, w) = if step >= 0.0 {
            (x, step)
        } else {
            (x + step, -step)
        };
        rect(
            g,
            xa,
            cy - h0 / 2.0 - t * spread,
            w,
            h0 + 2.0 * t * spread,
            alpha(c, a0 * (1.0 - t / steps as f32)),
        );
    }
}

/// A car seen from above, nose towards +y: drop shadow, the wheel arches
/// peeking out at the sides, the shell with rounded ends, the roof panel.
pub(super) fn car_shell(g: &Graphics, x: f32, y: f32, w: f32, h: f32, body: Color, roof: Color) {
    shadow_rect(g, x, y, w, h);
    for &(ax, ay) in &[
        (x - 2.0, y + 8.0),
        (x + w - 1.0, y + 8.0),
        (x - 2.0, y + h - 18.0),
        (x + w - 1.0, y + h - 18.0),
    ] {
        rect(g, ax, ay, 3.0, 10.0, TYRE);
    }
    rect(g, x, y + 4.0, w, h - 8.0, body);
    rect(g, x + 4.0, y, w - 8.0, 4.0, body);
    rect(g, x + 4.0, y + h - 4.0, w - 8.0, 4.0, body);
    // Panel seams: hood / trunk lines and the roof panel.
    rect(g, x + 4.0, y + h * 0.30, w - 8.0, h * 0.42, roof);
    line(
        g,
        x + 3.0,
        y + 8.0,
        x + w - 3.0,
        y + 8.0,
        1.0,
        alpha(PANEL, 0.35),
    );
    line(
        g,
        x + 3.0,
        y + h - 8.0,
        x + w - 3.0,
        y + h - 8.0,
        1.0,
        alpha(PANEL, 0.35),
    );
}

/// A hazard LED pair blinking together on a car's four corners.
pub(super) fn hazards(g: &Graphics, x: f32, y: f32, w: f32, h: f32, time: f32) {
    if blink(time, 1.0, 0.0, 0.5) {
        for &(cx, cy) in &[
            (x + 1.0, y + 1.0),
            (x + w - 5.0, y + 1.0),
            (x + 1.0, y + h - 4.0),
            (x + w - 5.0, y + h - 4.0),
        ] {
            rect(g, cx, cy, 4.0, 3.0, LED_AMBER);
            circle(g, cx + 2.0, cy + 1.5, 4.0, alpha(LED_AMBER, 0.18));
        }
    }
}

/// A roof sensor puck seen from above, in ITS OWN frame: the layer spin
/// turns the lidar sweep.
pub(super) fn lidar(g: &Graphics, r: f32) {
    circle(g, 0.0, 0.0, r, STEEL_DARK);
    circle(g, 0.0, 0.0, r - 1.5, PANEL);
    g.draw_arc(
        Vec2::new(0.0, 0.0),
        r - 1.0,
        -0.35,
        0.35,
        alpha(GLOW_CYAN, 0.55),
    );
    line(g, 0.0, 0.0, r - 1.0, 0.0, 1.2, GLOW_CYAN);
    circle(g, 0.0, 0.0, 1.5, TRIM);
}

/// 24 — compact autonomous pod, parked nose-down (+y): pearl shell, glass
/// ends, the roof lidar puck sweeping, daytime-running strips and a
/// breathing status dot at the tail. Layers: body, glass, lidar (spin),
/// lights.
pub(super) fn car_pod(g: &Graphics, layer: usize, time: f32) {
    match layer {
        0 => car_shell(
            g,
            -19.0,
            -30.0,
            38.0,
            60.0,
            PEARL,
            Color::new(0.90, 0.91, 0.94, 1.0),
        ),
        1 => {
            rect(g, -15.0, 12.0, 30.0, 10.0, GLASS); // windscreen
            rect(g, -13.0, 13.0, 26.0, 1.5, alpha(GLASS_HI, 0.5));
            rect(g, -15.0, -22.0, 30.0, 7.0, GLASS); // rear glass
            rect(g, -13.0, -21.0, 26.0, 1.5, alpha(GLASS_HI, 0.35));
        }
        2 => lidar(g, 5.0),
        _ => {
            rect(g, -16.0, 28.0, 9.0, 2.0, alpha(CREAM, 0.85)); // DRL strips
            rect(g, 7.0, 28.0, 9.0, 2.0, alpha(CREAM, 0.85));
            rect(g, -15.0, -30.0, 7.0, 2.0, alpha(LED_RED, 0.7)); // tails
            rect(g, 8.0, -30.0, 7.0, 2.0, alpha(LED_RED, 0.7));
            let breath = 0.4 + 0.5 * (time * 1.5).sin().max(0.0);
            circle(g, 0.0, -27.0, 1.5, alpha(GLOW_CYAN, breath));
        }
    }
}

/// 25 — autonomous sedan rolling in, headlights on: teal shell, glass,
/// the roof lidar spinning, a warm headlight wash spilling forward onto
/// the asphalt. Layers: body, glass, lidar (spin), lights.
pub(super) fn car_sedan(g: &Graphics, layer: usize, time: f32) {
    match layer {
        0 => car_shell(
            g,
            -21.0,
            -44.0,
            42.0,
            80.0,
            TEAL,
            Color::new(0.18, 0.42, 0.50, 1.0),
        ),
        1 => {
            rect(g, -17.0, 14.0, 34.0, 10.0, GLASS); // windscreen
            rect(g, -15.0, 15.0, 30.0, 1.5, alpha(GLASS_HI, 0.5));
            rect(g, -17.0, -30.0, 34.0, 8.0, GLASS); // rear glass
            rect(g, -15.0, -29.0, 30.0, 1.5, alpha(GLASS_HI, 0.35));
            rect(g, -19.0, -14.0, 2.0, 26.0, GLASS); // side glass
            rect(g, 17.0, -14.0, 2.0, 26.0, GLASS);
        }
        2 => lidar(g, 5.0),
        _ => {
            let fl = 0.92 + 0.08 * (time * 11.0).sin();
            wash_v(g, 0.0, 39.0, 38.0, 16.0, WARM_LIGHT, 0.24 * fl);
            rect(g, -17.0, 36.0, 10.0, 3.0, CREAM); // headlights
            rect(g, 7.0, 36.0, 10.0, 3.0, CREAM);
            rect(g, -17.0, -44.0, 9.0, 2.0, LED_RED); // tails
            rect(g, 8.0, -44.0, 9.0, 2.0, LED_RED);
            rect(g, -19.0, -45.0, 38.0, 1.5, alpha(LED_RED, 0.18));
        }
    }
}

/// 26 — sedan with both doors open and the cabin lit: warm light spilling
/// out onto the ground either side, the doors as tilted layers off their
/// hinges, seats through the glass roof, hazards blinking.
/// Layers: spill, body, door l / r (static swing), cabin, lidar (idle),
/// lights.
pub(super) fn car_open(g: &Graphics, layer: usize, time: f32) {
    match layer {
        0 => {
            let fl = 0.9 + 0.1 * (time * 5.0).sin();
            for &x in &[-33.0f32, 33.0] {
                circle(g, x, -2.0, 15.0, alpha(WARM_LIGHT, 0.14 * fl));
                circle(g, x, -2.0, 9.0, alpha(WARM_LIGHT, 0.14 * fl));
            }
        }
        1 => car_shell(
            g,
            -21.0,
            -44.0,
            42.0,
            80.0,
            PEARL,
            Color::new(0.90, 0.91, 0.94, 1.0),
        ),
        2 | 3 => {
            // A door, hinge at the origin, panel along +y (the layer's
            // static rotation swings it out).
            rect(g, -2.0, 0.0, 4.0, 24.0, PEARL);
            rect(g, -1.0, 4.0, 2.0, 12.0, GLASS);
            rect(g, -2.0, 0.0, 4.0, 1.5, TRIM);
        }
        4 => {
            rect(g, -17.0, -30.0, 34.0, 8.0, GLASS); // rear glass
            rect(g, -17.0, -20.0, 34.0, 32.0, alpha(WARM_LIGHT, 0.55)); // lit cabin
            rect(g, -12.0, -8.0, 10.0, 11.0, STEEL_DARK); // seats
            rect(g, 2.0, -8.0, 10.0, 11.0, STEEL_DARK);
            rect(g, -17.0, -20.0, 34.0, 32.0, alpha(GLASS, 0.40)); // the glass roof
            rect(g, -17.0, 14.0, 34.0, 10.0, GLASS); // windscreen
            rect(g, -15.0, 15.0, 30.0, 1.5, alpha(GLASS_HI, 0.5));
        }
        5 => lidar(g, 5.0),
        _ => {
            hazards(g, -21.0, -44.0, 42.0, 80.0, time);
            rect(g, -17.0, -44.0, 9.0, 2.0, alpha(LED_RED, 0.7));
            rect(g, 8.0, -44.0, 9.0, 2.0, alpha(LED_RED, 0.7));
        }
    }
}

/// 27 — boxy delivery van, hazards on: orange cargo box with roof ribs and
/// a livery band, the cab and windscreen up front, mirrors, the roof
/// puck. Layers: body, glass, puck (spin), lights.
pub(super) fn delivery_van(g: &Graphics, layer: usize, time: f32) {
    match layer {
        0 => {
            shadow_rect(g, -23.0, -46.0, 46.0, 88.0);
            for &(ax, ay) in &[(-25.0, -30.0), (22.0, -30.0), (-25.0, 22.0), (22.0, 22.0)] {
                rect(g, ax, ay, 3.0, 12.0, TYRE);
            }
            rect(g, -23.0, -42.0, 46.0, 84.0, ORANGE);
            rect(g, -19.0, -46.0, 38.0, 4.0, ORANGE);
            // Cargo roof: ribbed, a rear-door seam at the back.
            rect(
                g,
                -20.0,
                -42.0,
                40.0,
                60.0,
                Color::new(0.82, 0.40, 0.14, 1.0),
            );
            for i in 0..7 {
                rect(
                    g,
                    -20.0,
                    -38.0 + i as f32 * 8.0,
                    40.0,
                    1.5,
                    alpha(PANEL, 0.30),
                );
            }
            line(g, 0.0, -46.0, 0.0, -38.0, 1.5, alpha(PANEL, 0.6));
            rect(g, -23.0, -8.0, 46.0, 4.0, alpha(PAINT, 0.85)); // livery band
            circle(g, 0.0, -20.0, 3.5, Color::new(0.70, 0.34, 0.12, 1.0)); // roof vent
            rect(
                g,
                -20.0,
                20.0,
                40.0,
                10.0,
                Color::new(0.94, 0.52, 0.22, 1.0),
            ); // cab roof
        }
        1 => {
            rect(g, -19.0, 31.0, 38.0, 9.0, GLASS); // windscreen
            rect(g, -17.0, 32.0, 34.0, 1.5, alpha(GLASS_HI, 0.5));
            rect(g, -25.0, 29.0, 4.0, 3.0, CHROME); // mirrors
            rect(g, 21.0, 29.0, 4.0, 3.0, CHROME);
        }
        2 => lidar(g, 4.0),
        _ => {
            hazards(g, -23.0, -46.0, 46.0, 88.0, time);
            rect(g, -18.0, -46.0, 10.0, 2.0, alpha(LED_RED, 0.75)); // tails
            rect(g, 8.0, -46.0, 10.0, 2.0, alpha(LED_RED, 0.75));
            rect(g, -6.0, -46.0, 4.0, 2.0, alpha(CREAM, 0.7)); // reversing
            rect(g, 2.0, -46.0, 4.0, 2.0, alpha(CREAM, 0.7));
            rect(g, -18.0, 40.0, 10.0, 2.0, alpha(CREAM, 0.6)); // dipped heads
            rect(g, 8.0, 40.0, 10.0, 2.0, alpha(CREAM, 0.6));
        }
    }
}

/// 28 — inductive charging pad flush with the asphalt: rubber pad, the
/// coil disc under it, a feed conduit to the back, green status LEDs on
/// the front edge, and the charge field pulsing outward.
/// Layers: pad, rings (pulsing).
pub(super) fn charge_pad(g: &Graphics, layer: usize, time: f32) {
    match layer {
        0 => {
            line(g, 0.0, -30.0, 0.0, -44.0, 3.0, COPPER); // conduit
            rect(g, -5.0, -46.0, 10.0, 6.0, STEEL_DARK);
            rect(g, -30.0, -30.0, 60.0, 60.0, RUBBER);
            frame(g, -30.0, -30.0, 60.0, 60.0, 2.0, CONCRETE_DARK);
            circle(g, 0.0, 0.0, 22.0, Color::new(0.13, 0.13, 0.17, 1.0));
            for &r in &[18.0f32, 12.0, 6.0] {
                ring(g, 0.0, 0.0, r, 1.5, alpha(COPPER, 0.5));
            }
            rect(g, -30.0, 26.0, 60.0, 4.0, STEEL_DARK);
            for i in 0..3u32 {
                let on = blink(time, 1.0, i as f32 * 0.33, 0.5);
                rect(
                    g,
                    -7.0 + i as f32 * 5.0,
                    27.0,
                    3.0,
                    2.0,
                    if on { LED_GREEN } else { PANEL },
                );
            }
        }
        _ => {
            for k in 0..3u32 {
                let ph = (time * 0.5 + k as f32 / 3.0).fract();
                ring(
                    g,
                    0.0,
                    0.0,
                    4.0 + ph * 22.0,
                    2.0,
                    alpha(LED_GREEN, 0.45 * (1.0 - ph)),
                );
            }
        }
    }
}

/// 29 — a pod parked on a charging pad, topping up: the pad's field
/// pulsing out from under the shell, the lidar barely turning, a charge
/// gauge filling on the flank and the amber charge LED at the tail.
/// Layers: pad, rings, body, glass, lidar (slow), charge.
pub(super) fn car_charging(g: &Graphics, layer: usize, time: f32) {
    // The pod sits 6 units up the pad so the pad's front LEDs show.
    match layer {
        0 => charge_pad(g, 0, time),
        1 => {
            for k in 0..3u32 {
                let ph = (time * 0.5 + k as f32 / 3.0).fract();
                ring(
                    g,
                    0.0,
                    -6.0,
                    14.0 + ph * 16.0,
                    2.5,
                    alpha(LED_GREEN, 0.6 * (1.0 - ph)),
                );
            }
        }
        2 | 3 => {
            g.save();
            g.translate(0.0, -6.0);
            car_pod(g, layer - 2, time);
            g.restore();
        }
        4 => lidar(g, 5.0),
        _ => {
            rect(g, -21.0, -18.0, 3.0, 24.0, PANEL);
            let fill = ((time * 0.15).fract() * 5.0) as u32;
            for i in 0..5u32 {
                let c = if i < fill { LED_GREEN } else { STEEL_DARK };
                rect(g, -20.5, 3.5 - i as f32 * 4.6, 2.0, 3.6, c);
            }
            if blink(time, 0.7, 0.0, 0.5) {
                circle(g, 0.0, -33.0, 1.8, LED_AMBER);
                circle(g, 0.0, -33.0, 4.0, alpha(LED_AMBER, 0.2));
            }
        }
    }
}

/// 30 — the main gate across the entry lane: kerbed asphalt with the
/// stop line and induction loop, two scanner posts, the scan beam
/// between them, and the swing arm — closed across the lane, then a slow
/// swing open along it and back (the layer's `Anim`, its shadow a second
/// layer on the same curve). Layers: lane, posts, scan, arm shadow, arm.
pub(super) fn main_gate(g: &Graphics, layer: usize, time: f32) {
    match layer {
        0 => {
            rect(g, -30.0, -50.0, 60.0, 100.0, ASPHALT);
            rect(g, -34.0, -50.0, 4.0, 100.0, CONCRETE_DARK); // kerbs
            rect(g, 30.0, -50.0, 4.0, 100.0, CONCRETE_DARK);
            rect(g, -26.0, 26.0, 52.0, 3.0, PAINT); // stop line
            frame(
                g,
                -20.0,
                32.0,
                40.0,
                16.0,
                1.0,
                Color::new(0.05, 0.05, 0.07, 1.0),
            ); // induction loop
            for i in 0..3 {
                rect(
                    g,
                    -1.0,
                    -46.0 + i as f32 * 14.0,
                    2.0,
                    8.0,
                    alpha(PAINT, 0.5),
                );
            }
        }
        1 => {
            for &x in &[-48.0f32, 30.0] {
                shadow_rect(g, x, 5.0, 12.0, 14.0);
                rect(g, x, 5.0, 12.0, 14.0, STEEL);
                frame(g, x, 5.0, 12.0, 14.0, 1.5, TRIM);
                let slot = if x < 0.0 { x + 10.0 } else { x };
                rect(g, slot, 8.0, 2.0, 8.0, PANEL); // scanner slot facing the lane
            }
        }
        2 => {
            let y = 12.0 + (time * 1.5).sin() * 4.0;
            line(g, -36.0, y, 30.0, y, 1.0, alpha(GLOW_CYAN, 0.45));
            let open = gate_angle(time).abs() > 1.0;
            for &x in &[-42.0f32, 36.0] {
                let on = blink(time, 0.5, if x < 0.0 { 0.0 } else { 0.5 }, 0.5);
                circle(g, x, 9.0, 1.8, if on { GLOW_CYAN } else { PANEL });
                circle(g, x, 15.0, 1.8, if open { LED_GREEN } else { LED_RED });
            }
        }
        3 => rect(g, 0.0, -2.5, 62.0, 5.0, SHADOW),
        _ => {
            circle(g, 0.0, 0.0, 5.0, STEEL_DARK);
            rect(g, 0.0, -2.5, 62.0, 5.0, PAINT);
            for i in 0..5 {
                rect(
                    g,
                    8.0 + i as f32 * 12.0,
                    -2.5,
                    6.0,
                    5.0,
                    Color::new(0.85, 0.15, 0.20, 1.0),
                );
            }
            circle(g, 0.0, 0.0, 2.0, TRIM);
            let tip = blink(time, 1.0, 0.0, 0.5);
            circle(g, 60.0, 0.0, 1.8, if tip { LED_RED } else { PANEL });
        }
    }
}

/// 31 — the guard booth beside the lane: a tall concrete box (long
/// shadow) with its lit window strip washing monitor light onto the
/// asphalt at its right, the roof AC unit's fan turning, a rotating amber
/// beacon on the roof corner. Layers: wash, booth, ac fan (spin), beacon
/// (spin).
pub(super) fn guard_booth(g: &Graphics, layer: usize, time: f32) {
    match layer {
        0 => {
            let fl = 0.9 + 0.1 * (time * 9.0).sin();
            wash_h(
                g,
                20.0,
                0.0,
                36.0,
                26.0,
                Color::new(0.75, 0.95, 0.70, 1.0),
                0.20 * fl,
            );
        }
        1 => {
            rect(g, -24.0, -22.0, 50.0, 56.0, SHADOW);
            rect(g, -30.0, -28.0, 50.0, 56.0, CONCRETE_DARK);
            frame(g, -30.0, -28.0, 50.0, 56.0, 2.0, CONCRETE);
            rect(
                g,
                -26.0,
                -24.0,
                42.0,
                48.0,
                Color::new(0.30, 0.30, 0.32, 1.0),
            );
            rect(g, 16.0, -20.0, 4.0, 40.0, alpha(WARM_LIGHT, 0.75)); // the lit window
            rect(g, -14.0, -30.0, 14.0, 4.0, STEEL); // door on the back wall
            rect(g, -22.0, -20.0, 16.0, 16.0, STEEL); // AC unit
            frame(g, -22.0, -20.0, 16.0, 16.0, 1.5, TRIM);
        }
        2 => fan(g, 6.0),
        _ => {
            g.draw_arc(Vec2::new(0.0, 0.0), 14.0, -0.3, 0.3, alpha(LED_AMBER, 0.28));
            circle(g, 0.0, 0.0, 3.0, LED_AMBER);
            circle(g, 0.0, 0.0, 1.5, CREAM);
        }
    }
}

/// 32 — a row of three bollards chained together, each capped with a
/// breathing blue marker light. Layers: posts, leds.
pub(super) fn bollards(g: &Graphics, layer: usize, time: f32) {
    match layer {
        0 => {
            for k in 0..2 {
                let x0 = -32.0 + k as f32 * 32.0;
                line(g, x0, 0.0, x0 + 16.0, 3.0, 1.5, alpha(CHROME, 0.8));
                line(g, x0 + 16.0, 3.0, x0 + 32.0, 0.0, 1.5, alpha(CHROME, 0.8));
            }
            for &x in &[-32.0f32, 0.0, 32.0] {
                post(g, x, 0.0, 6.0, CONCRETE, STEEL);
            }
        }
        _ => {
            for (k, &x) in [-32.0f32, 0.0, 32.0].iter().enumerate() {
                let a = 0.35 + 0.35 * (time * 2.0 + k as f32 * 2.1).sin();
                circle(g, x, 0.0, 3.0, alpha(GLOW_CYAN, a));
                circle(g, x, 0.0, 1.5, alpha(CREAM, a));
            }
        }
    }
}

/// 33 — concrete planter with a shrub rustling in it and a small solar
/// stake lamp. Layers: box, shrub (sway), lamp.
pub(super) fn planter(g: &Graphics, layer: usize, time: f32) {
    match layer {
        0 => {
            shadow_rect(g, -35.0, -20.0, 70.0, 40.0);
            rect(g, -35.0, -20.0, 70.0, 40.0, CONCRETE);
            frame(g, -35.0, -20.0, 70.0, 40.0, 2.0, CONCRETE_DARK);
            rect(
                g,
                -31.0,
                -16.0,
                62.0,
                32.0,
                Color::new(0.16, 0.11, 0.08, 1.0),
            );
        }
        1 => {
            for k in 0..7u32 {
                let x = -22.0 + rnd(k, 1) * 44.0;
                let y = -9.0 + rnd(k, 2) * 18.0;
                let r = 6.0 + rnd(k, 3) * 4.0;
                circle(g, x + 1.0, y + 1.0, r + 1.0, LEAF_DARK);
                circle(g, x, y, r, LEAF);
                circle(g, x - r * 0.3, y - r * 0.3, r * 0.4, LEAF_LIGHT);
            }
        }
        _ => {
            circle(g, 32.0, -12.0, 3.5, STEEL_DARK);
            let on = blink(time, 0.4, 0.0, 0.7);
            if on {
                circle(g, 32.0, -12.0, 5.5, alpha(WARM_LIGHT, 0.18));
            }
            circle(g, 32.0, -12.0, 2.0, if on { WARM_LIGHT } else { PANEL });
        }
    }
}

/// 34 — a lamp post: the pool of light it throws on the ground, the long
/// shadow of the mast, the plinth and arm, the lamp head, two moths.
/// Layers: pool, shadow, mast, head, moths.
pub(super) fn lamp_post(g: &Graphics, layer: usize, time: f32) {
    let (hx, hy) = (10.0f32, -6.0f32); // the lamp head
    let (bx, by) = (-14.0f32, 14.0f32); // the plinth
    match layer {
        0 => {
            let fl = if rnd(3, (time * 8.0) as u32) > 0.94 {
                0.7
            } else {
                1.0
            };
            circle(g, hx, hy, 44.0, alpha(WARM_LIGHT, 0.07 * fl));
            circle(g, hx, hy, 30.0, alpha(WARM_LIGHT, 0.07 * fl));
            circle(g, hx, hy, 16.0, alpha(WARM_LIGHT, 0.09 * fl));
        }
        1 => {
            line(g, bx, by, 22.0, 44.0, 3.0, alpha(SHADOW, 0.22));
            circle(g, 28.0, 40.0, 5.0, alpha(SHADOW, 0.22));
        }
        2 => {
            post(g, bx, by, 5.0, CONCRETE, STEEL_DARK);
            line(g, bx, by, hx, hy, 4.0, STEEL_DARK);
            line(g, bx, by, hx, hy, 1.5, TRIM);
        }
        3 => {
            rect(g, hx - 7.0, hy - 6.0, 16.0, 10.0, STEEL);
            frame(g, hx - 7.0, hy - 6.0, 16.0, 10.0, 1.5, TRIM);
            circle(g, hx, hy, 6.0, alpha(WARM_LIGHT, 0.5));
            circle(g, hx, hy, 4.0, WARM_LIGHT);
            circle(g, hx, hy, 2.0, CREAM);
        }
        _ => {
            for k in 0..2 {
                let a = time * (4.0 + k as f32) + k as f32 * 3.0;
                let r = 6.0 + 4.0 * (time * 1.7 + k as f32).sin();
                circle(g, a.cos() * r, a.sin() * r * 0.8, 1.0, alpha(CREAM, 0.7));
            }
        }
    }
}

/// 35 — an EV parking bay: painted bay lines, the EV glyph and a bolt
/// roundel on the asphalt. A flat decal. Layers: asphalt, lines, glyph.
pub(super) fn ev_bay(g: &Graphics, layer: usize, _time: f32) {
    match layer {
        0 => {
            rect(g, -45.0, -45.0, 90.0, 90.0, alpha(ASPHALT, 0.95));
            circle(g, 8.0, -6.0, 9.0, alpha(Color::BLACK, 0.25));
            circle(g, 4.0, -9.0, 5.0, alpha(Color::BLACK, 0.2));
        }
        1 => {
            rect(g, -40.0, -44.0, 4.0, 84.0, PAINT);
            rect(g, 36.0, -44.0, 4.0, 84.0, PAINT);
            rect(g, -40.0, -44.0, 80.0, 4.0, PAINT);
            for k in 0..4u32 {
                let x = -40.0 + rnd(k, 1) * 80.0;
                let y = -44.0 + rnd(k, 2) * 84.0;
                rect(g, x, y, 3.0, 2.0, alpha(ASPHALT, 0.8)); // wear
            }
        }
        _ => {
            let c = alpha(LED_GREEN, 0.85);
            rect(g, -18.0, 0.0, 4.0, 20.0, c); // E
            rect(g, -18.0, 0.0, 14.0, 4.0, c);
            rect(g, -18.0, 8.0, 11.0, 4.0, c);
            rect(g, -18.0, 16.0, 14.0, 4.0, c);
            line(g, 2.0, 0.0, 10.0, 20.0, 4.0, c); // V
            line(g, 18.0, 0.0, 10.0, 20.0, 4.0, c);
            ring(g, 0.0, -20.0, 8.0, 1.5, PAINT); // the bolt roundel
            line(g, 2.0, -27.0, -2.0, -20.0, 2.0, HAZARD_YELLOW);
            line(g, -2.0, -20.0, 2.0, -20.0, 2.0, HAZARD_YELLOW);
            line(g, 2.0, -20.0, -2.0, -13.0, 2.0, HAZARD_YELLOW);
        }
    }
}

/// 36 — a zebra crossing over the lane, stop line behind it. Flat decal.
/// Layers: asphalt, stripes.
pub(super) fn crosswalk(g: &Graphics, layer: usize, _time: f32) {
    match layer {
        0 => {
            rect(g, -45.0, -45.0, 90.0, 90.0, alpha(ASPHALT, 0.95));
            rect(g, -44.0, 34.0, 88.0, 4.0, PAINT);
        }
        _ => {
            for i in 0..6u32 {
                let x = -44.0 + i as f32 * 15.6;
                rect(g, x, -30.0, 9.0, 60.0, PAINT);
                for k in 0..2u32 {
                    rect(
                        g,
                        x + rnd(i, k) * 7.0,
                        -30.0 + rnd(k, i) * 56.0,
                        2.0,
                        3.0,
                        alpha(ASPHALT, 0.8),
                    );
                }
            }
        }
    }
}

/// 37 — delivery drone pad with a quadcopter landed on the H: the pad's
/// corner beacons chasing, the drone (arms, motor pods, gimbal, status
/// LED), four rotors that shiver now and then (each an `Anim` layer).
/// Layers: pad, beacons, drone, rotor a..d.
pub(super) fn drone_pad(g: &Graphics, layer: usize, time: f32) {
    match layer {
        0 => {
            let pad = Color::new(0.12, 0.12, 0.15, 1.0);
            circle(g, 0.0, 0.0, 44.0, pad);
            circle(g, 0.0, 0.0, 40.0, PAINT);
            circle(g, 0.0, 0.0, 37.0, pad);
            rect(g, -16.0, -14.0, 6.0, 28.0, HAZARD_YELLOW);
            rect(g, 10.0, -14.0, 6.0, 28.0, HAZARD_YELLOW);
            rect(g, -10.0, -3.0, 20.0, 6.0, HAZARD_YELLOW);
        }
        1 => {
            let lit = (time * 2.0) as u32 % 4;
            for (k, &(x, y)) in [
                (-31.0f32, -31.0f32),
                (31.0, -31.0),
                (31.0, 31.0),
                (-31.0, 31.0),
            ]
            .iter()
            .enumerate()
            {
                let on = lit == k as u32;
                if on {
                    circle(g, x, y, 5.0, alpha(LED_RED, 0.2));
                }
                circle(g, x, y, 2.5, if on { LED_RED } else { PANEL });
            }
        }
        2 => {
            circle(g, 3.0, 3.0, 8.0, SHADOW);
            for &(x, y) in &[
                (-16.0f32, -16.0f32),
                (16.0, -16.0),
                (-16.0, 16.0),
                (16.0, 16.0),
            ] {
                line(g, 3.0, 3.0, x + 3.0, y + 3.0, 3.0, alpha(SHADOW, 0.2));
                line(g, 0.0, 0.0, x, y, 3.0, STEEL_DARK);
                circle(g, x, y, 4.0, STEEL_DARK);
                circle(g, x, y, 2.0, TRIM);
            }
            circle(g, 0.0, 0.0, 7.0, STEEL);
            circle(g, 0.0, 0.0, 4.0, PANEL);
            circle(g, 0.0, 9.0, 2.5, PANEL); // the gimbal
            circle(g, 0.0, 9.0, 1.0, GLOW_CYAN);
            let on = blink(time, 1.0, 0.0, 0.15);
            circle(g, -4.0, -4.0, 1.5, if on { LED_RED } else { STEEL_DARK });
        }
        _ => {
            rect(g, -11.0, -1.2, 22.0, 2.4, alpha(CHROME, 0.85));
            circle(g, 0.0, 0.0, 1.5, TRIM);
        }
    }
}

/// One e-scooter docked front-up, in its layer's frame (deck stripe /
/// LED per `variant`).
pub(super) fn scooter(g: &Graphics, variant: u32, time: f32) {
    rect(g, -3.0, -28.0, 10.0, 60.0, SHADOW);
    rect(g, -3.0, -30.0, 6.0, 8.0, TYRE);
    rect(g, -3.0, 22.0, 6.0, 8.0, TYRE);
    rect(
        g,
        -5.0,
        -22.0,
        10.0,
        44.0,
        Color::new(0.16, 0.16, 0.20, 1.0),
    );
    let stripe = match variant {
        0 => Color::new(0.60, 0.90, 0.20, 1.0),
        1 => GLOW_CYAN,
        _ => NEON_PINK,
    };
    rect(g, -1.0, -18.0, 2.0, 36.0, stripe);
    line(g, 0.0, -24.0, 0.0, -28.0, 3.0, STEEL);
    rect(g, -9.0, -30.0, 18.0, 3.0, STEEL_DARK);
    rect(g, -9.0, -30.0, 3.0, 3.0, RUBBER);
    rect(g, 6.0, -30.0, 3.0, 3.0, RUBBER);
    let led = match variant {
        0 => {
            if blink(time, 1.0, 0.0, 0.5) {
                LED_GREEN
            } else {
                PANEL
            }
        }
        1 => LED_AMBER,
        _ => PANEL,
    };
    circle(g, 0.0, 14.0, 1.5, led);
}

/// 38 — a scooter rack: the docking rail, two scooters docked and a
/// third leaning in its slot (a static tilt). Layers: rail, scooter a /
/// b / c.
pub(super) fn scooter_rack(g: &Graphics, layer: usize, time: f32) {
    match layer {
        0 => {
            shadow_rect(g, -44.0, -34.0, 88.0, 6.0);
            rect(g, -44.0, -34.0, 88.0, 6.0, CHROME);
            rect(g, -44.0, -36.0, 6.0, 10.0, STEEL_DARK);
            rect(g, 38.0, -36.0, 6.0, 10.0, STEEL_DARK);
            for &x in &[-28.0f32, 0.0, 28.0] {
                rect(g, x - 6.0, -32.0, 12.0, 4.0, STEEL_DARK);
            }
        }
        n => scooter(g, (n - 1) as u32, time),
    }
}

/// 39 — a storm drain in a puddle: the wet patch, the grate, a sheen
/// drifting over the water and a drip's ripple. Flat. Layers: puddle,
/// grate, sheen.
pub(super) fn drain_grate(g: &Graphics, layer: usize, time: f32) {
    match layer {
        0 => {
            circle(g, 2.0, 6.0, 30.0, Color::new(0.05, 0.07, 0.12, 0.55));
            circle(g, 4.0, 8.0, 24.0, Color::new(0.08, 0.10, 0.16, 0.35));
        }
        1 => {
            rect(g, -24.0, -14.0, 48.0, 28.0, STEEL_DARK);
            frame(g, -24.0, -14.0, 48.0, 28.0, 2.0, CONCRETE_DARK);
            for i in 0..6 {
                rect(g, -20.0 + i as f32 * 6.5, -10.0, 3.0, 20.0, PANEL);
            }
            for &(x, y) in &[
                (-21.0f32, -11.0f32),
                (21.0, -11.0),
                (-21.0, 11.0),
                (21.0, 11.0),
            ] {
                circle(g, x, y, 1.5, CONCRETE);
            }
        }
        _ => {
            let x = -28.0 + (time * 0.12).fract() * 56.0;
            rect(g, x, -20.0, 6.0, 50.0, Color::new(0.6, 0.7, 0.9, 0.08));
            let ph = (time * 0.7).fract();
            ring(
                g,
                10.0,
                14.0,
                2.0 + ph * 10.0,
                1.5,
                alpha(GLASS_HI, 0.3 * (1.0 - ph)),
            );
        }
    }
}

/// 40 — a tall holo billboard: the sign slab edge-on with its neon front
/// edge, raised on two columns (long shadow), a big colour-cycling wash
/// down the ground in front of it and a glyph strip crawling along the
/// face. Layers: wash, shadow, slab, glyphs.
pub(super) fn holo_billboard(g: &Graphics, layer: usize, time: f32) {
    match layer {
        0 => {
            let c = mix(NEON_PINK, GLOW_CYAN, 0.5 + 0.5 * (time * 0.5).sin());
            for i in 0..5 {
                let t = i as f32;
                rect(
                    g,
                    -42.0 - t * 2.0,
                    -24.0 + t * 14.0,
                    84.0 + t * 4.0,
                    14.0,
                    alpha(c, 0.30 * (1.0 - t / 5.0)),
                );
            }
            let y = -24.0 + (time * 0.35).fract() * 70.0;
            rect(g, -50.0, y, 100.0, 3.0, alpha(CREAM, 0.10));
        }
        1 => {
            rect(g, -32.0, -24.0, 84.0, 10.0, Color::new(0.0, 0.0, 0.0, 0.22));
            circle(g, -20.0, -30.0, 5.0, Color::new(0.0, 0.0, 0.0, 0.22));
            circle(g, 40.0, -30.0, 5.0, Color::new(0.0, 0.0, 0.0, 0.22));
        }
        2 => {
            for &x in &[-30.0f32, 30.0] {
                circle(g, x, -40.0, 5.0, STEEL_DARK);
                circle(g, x, -40.0, 3.0, TRIM);
            }
            rect(g, -42.0, -34.0, 84.0, 10.0, PANEL);
            frame(g, -42.0, -34.0, 84.0, 10.0, 1.5, TRIM);
            let pulse = 0.7 + 0.3 * (time * 3.0).sin();
            rect(g, -42.0, -25.0, 84.0, 2.0, alpha(NEON_PINK, pulse));
            rect(g, -40.0, -33.0, 80.0, 1.0, alpha(CREAM, 0.15));
        }
        _ => glyph_strip(g, -40.0, 40.0, -32.0, time, GLOW_CYAN),
    }
}

/// 41 — a dumpster, one lid down and one flipped open over the back
/// edge, bags and a box in the open half, flies. Layers: body, lid l,
/// lid r (static tilt), flies.
pub(super) fn dumpster(g: &Graphics, layer: usize, time: f32) {
    let green = Color::new(0.16, 0.32, 0.22, 1.0);
    let lid = Color::new(0.20, 0.38, 0.27, 1.0);
    let lid_edge = Color::new(0.26, 0.46, 0.34, 1.0);
    match layer {
        0 => {
            rect(g, -25.0, -17.0, 60.0, 44.0, SHADOW);
            rect(g, -30.0, -22.0, 60.0, 44.0, green);
            frame(g, -30.0, -22.0, 60.0, 44.0, 2.0, lid_edge);
            rect(g, 0.0, -20.0, 28.0, 40.0, Color::new(0.05, 0.05, 0.06, 1.0));
            circle(g, 10.0, -8.0, 7.0, Color::new(0.12, 0.12, 0.14, 1.0));
            circle(g, 19.0, 6.0, 6.0, Color::new(0.14, 0.13, 0.15, 1.0));
            rect(g, 4.0, 4.0, 10.0, 8.0, Color::new(0.45, 0.35, 0.22, 1.0));
            for i in 0..10 {
                let c = if i % 2 == 0 {
                    HAZARD_YELLOW
                } else {
                    Color::new(0.05, 0.05, 0.06, 1.0)
                };
                rect(g, -30.0 + i as f32 * 6.0, 18.0, 6.0, 4.0, c);
            }
        }
        1 => {
            rect(g, -30.0, -22.0, 30.0, 44.0, lid);
            frame(g, -30.0, -22.0, 30.0, 44.0, 1.5, lid_edge);
            for &y in &[-12.0f32, 0.0, 12.0] {
                line(g, -28.0, y, -2.0, y, 1.0, alpha(PANEL, 0.3));
            }
            rect(g, -19.0, 16.0, 8.0, 3.0, STEEL_DARK); // handle
        }
        2 => {
            rect(g, 0.0, -9.0, 30.0, 8.0, lid);
            frame(g, 0.0, -9.0, 30.0, 8.0, 1.5, lid_edge);
            rect(g, 0.0, -1.0, 30.0, 1.0, STEEL_DARK); // the hinge
        }
        _ => {
            for k in 0..2 {
                let f = k as f32;
                let x = 14.0 + (time * 7.0 + f).cos() * 9.0;
                let y = -4.0 + (time * 9.3 + f * 2.0).sin() * 7.0;
                circle(g, x, y, 1.2, Color::new(0.05, 0.05, 0.05, 0.9));
            }
        }
    }
}
