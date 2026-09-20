//! The DATACENTER family: props 0..23, one `fn(g, layer, time)` each.

use super::*;

/// A row of status LEDs winking on a front-edge service strip.
pub(super) fn front_leds(g: &Graphics, x0: f32, y: f32, n: u32, time: f32) {
    for i in 0..n {
        let on = blink(time, 1.2 + rnd(i, 3) * 3.5, rnd(7, i), 0.6);
        let c = if !on {
            PANEL
        } else if rnd(i, 5) > 0.35 {
            LED_GREEN
        } else {
            LED_AMBER
        };
        rect(g, x0 + i as f32 * 8.0, y, 5.0, 4.0, c);
    }
}

/// 0 — closed rack seen from above: sealed top panel, twin exhaust fans,
/// cabling ducking out the back, status LEDs on the front edge.
/// Layers: body, fan a, fan b (spinning), leds.
pub(super) fn rack_closed(g: &Graphics, layer: usize, time: f32) {
    match layer {
        0 => {
            shadow_rect(g, -35.0, -45.0, 70.0, 90.0);
            rect(g, -35.0, -45.0, 70.0, 90.0, STEEL);
            frame(g, -35.0, -45.0, 70.0, 90.0, 2.0, TRIM);
            // Lid seam, then the rear cable cutout with feeds heading off
            // to the trunking behind.
            frame(g, -29.0, -37.0, 58.0, 72.0, 1.0, alpha(PANEL, 0.6));
            rect(g, -14.0, -45.0, 28.0, 7.0, PANEL);
            line(g, -8.0, -42.0, -12.0, -49.0, 2.0, COPPER);
            line(g, 3.0, -42.0, 6.0, -49.0, 2.0, GLOW_CYAN);
        }
        // Twin roof fans, counter-rotating (the layer rotation spins them).
        1 | 2 => fan(g, 13.0),
        _ => {
            // Front service strip.
            rect(g, -35.0, 37.0, 70.0, 8.0, STEEL_DARK);
            front_leds(g, -30.0, 39.0, 5, time);
        }
    }
}

/// 1 — rack with the lid off, looking straight down into the chassis:
/// board, chip grid, finned heatsink, loose cabling, a live internal fan.
/// Layers: chassis, fan (spinning), leds.
pub(super) fn rack_open(g: &Graphics, layer: usize, time: f32) {
    match layer {
        0 => {
            shadow_rect(g, -35.0, -45.0, 70.0, 90.0);
            rect(g, -35.0, -45.0, 70.0, 90.0, STEEL_DARK); // chassis walls
            frame(g, -35.0, -45.0, 70.0, 90.0, 2.0, TRIM);
            rect(g, -30.0, -40.0, 60.0, 80.0, PANEL); // the open bay
                                                      // Main board and its chip grid.
            rect(
                g,
                -27.0,
                -37.0,
                42.0,
                74.0,
                Color::new(0.07, 0.17, 0.11, 1.0),
            );
            for row in 0..4u32 {
                for col in 0..3u32 {
                    if rnd(row * 5 + col, 3) > 0.75 {
                        continue; // an unpopulated pad
                    }
                    rect(
                        g,
                        -23.0 + col as f32 * 13.0,
                        -33.0 + row as f32 * 12.0,
                        8.0,
                        6.0,
                        Color::new(0.11, 0.24, 0.16, 1.0),
                    );
                }
            }
            // CPU under a finned heatsink.
            rect(g, -21.0, 15.0, 24.0, 20.0, STEEL);
            for i in 0..5 {
                rect(g, -19.5 + i as f32 * 4.6, 16.0, 2.0, 18.0, STEEL_DARK);
            }
            // Loose cabling along the right wall.
            line(g, 20.0, -36.0, 25.0, -10.0, 2.0, COPPER);
            line(g, 25.0, -10.0, 19.0, 22.0, 2.0, COPPER);
            line(g, 23.0, -36.0, 20.0, 8.0, 1.5, GLOW_CYAN);
        }
        // The internal fan, still running with the lid off.
        1 => fan(g, 10.0),
        _ => {
            // Board LEDs.
            for i in 0..3u32 {
                let on = blink(time, 2.0 + rnd(i, 9) * 4.0, rnd(i, 4), 0.5);
                rect(
                    g,
                    -26.0,
                    24.0 + i as f32 * 5.0,
                    3.0,
                    3.0,
                    if on { LED_GREEN } else { PANEL },
                );
            }
        }
    }
}

/// 2 — burnt-out rack from above: charred top blown open, an ember still
/// cooling in the hole, the lid flat on the floor beside it, stray sparks.
/// Layers: lid (static tilt), hull, sparks.
pub(super) fn rack_burnt(g: &Graphics, layer: usize, time: f32) {
    match layer {
        0 => {
            // The blown-off lid lies on the floor to the right (flat: no
            // shadow); its tilt is the layer's static rotation.
            rect(
                g,
                -8.0,
                -27.0,
                16.0,
                54.0,
                Color::new(0.11, 0.09, 0.13, 1.0),
            );
            frame(
                g,
                -8.0,
                -27.0,
                16.0,
                54.0,
                1.5,
                Color::new(0.20, 0.17, 0.22, 1.0),
            );
        }
        1 => {
            shadow_rect(g, -40.0, -45.0, 62.0, 90.0);
            rect(
                g,
                -40.0,
                -45.0,
                62.0,
                90.0,
                Color::new(0.09, 0.08, 0.11, 1.0),
            );
            frame(
                g,
                -40.0,
                -45.0,
                62.0,
                90.0,
                2.0,
                Color::new(0.20, 0.17, 0.22, 1.0),
            );
            // The blast hole through the top, soot streaked outward.
            for k in 0..4u32 {
                let a = rnd(k, 1) * TAU;
                line(
                    g,
                    -9.0 + a.cos() * 12.0,
                    -6.0 + a.sin() * 12.0,
                    -9.0 + a.cos() * 26.0,
                    -6.0 + a.sin() * 26.0,
                    4.0,
                    Color::new(0.04, 0.03, 0.05, 0.8),
                );
            }
            circle(g, -9.0, -6.0, 16.0, Color::new(0.03, 0.02, 0.04, 1.0));
            circle(g, 0.0, 6.0, 9.0, Color::new(0.03, 0.02, 0.04, 1.0));
            let ember = 0.25 + 0.20 * (time * 3.3).sin();
            circle(g, -9.0, -5.0, 6.0, Color::new(0.9, 0.25, 0.08, ember));
        }
        _ => {
            // Something in there still shorts now and then.
            for k in 0..3u32 {
                if ((time * (11.0 + k as f32 * 3.7) + k as f32 * 1.9).sin()) > 0.94 {
                    let x = -24.0 + rnd(k, 5) * 30.0;
                    let y = -28.0 + rnd(k, 9) * 44.0;
                    rect(g, x, y, 3.0, 3.0, Color::new(1.0, 0.92, 0.4, 0.95));
                    line(g, x + 1.0, y + 1.0, x + 5.0, y - 4.0, 1.0, LED_AMBER);
                }
            }
        }
    }
}

/// 3 — open blade enclosure from above: vertical blade fins with the hot
/// exhaust glow breathing in the gaps, service strip along the front.
/// Layers: enclosure, leds.
pub(super) fn blade_stack(g: &Graphics, layer: usize, time: f32) {
    match layer {
        0 => {
            shadow_rect(g, -40.0, -42.0, 80.0, 84.0);
            rect(g, -40.0, -42.0, 80.0, 84.0, STEEL_DARK);
            frame(g, -40.0, -42.0, 80.0, 84.0, 2.0, TRIM);
            for i in 0..8u32 {
                let x = -34.0 + i as f32 * 9.0;
                // The hot aisle glow in the gap before each fin.
                let pulse = 0.22 + 0.18 * (time * 2.2 + i as f32 * 0.7).sin();
                rect(g, x - 2.5, -34.0, 2.5, 66.0, alpha(GLOW_CYAN, pulse));
                let fin = if i % 2 == 0 {
                    STEEL
                } else {
                    Color::new(0.28, 0.26, 0.35, 1.0)
                };
                rect(g, x, -36.0, 6.0, 70.0, fin);
            }
        }
        _ => {
            rect(g, -40.0, 34.0, 80.0, 8.0, PANEL);
            front_leds(g, -34.0, 36.0, 8, time);
        }
    }
}

/// 4 — core switch from above: low vented top, uplinks breathing, the
/// port field blinking along the front edge, cables snaking off across
/// the floor. Layers: chassis, ports, cables.
pub(super) fn core_switch(g: &Graphics, layer: usize, time: f32) {
    match layer {
        0 => {
            shadow_rect(g, -45.0, -22.0, 90.0, 40.0);
            rect(g, -45.0, -22.0, 90.0, 40.0, STEEL);
            frame(g, -45.0, -22.0, 90.0, 40.0, 2.0, TRIM);
            for i in 0..3 {
                rect(g, -38.0, -16.0 + i as f32 * 7.0, 60.0, 2.5, PANEL);
            }
            // The uplink pair, breathing cyan on the top surface.
            let pulse = 0.5 + 0.5 * (time * 3.0).sin();
            rect(
                g,
                28.0,
                -15.0,
                9.0,
                9.0,
                alpha(GLOW_CYAN, 0.35 + 0.6 * pulse),
            );
            rect(
                g,
                28.0,
                -2.0,
                9.0,
                9.0,
                alpha(GLOW_CYAN, 0.95 - 0.6 * pulse),
            );
        }
        1 => {
            // Front edge: the port field, traffic winking at the cable heads.
            rect(g, -45.0, 12.0, 90.0, 6.0, STEEL_DARK);
            let tick = (time * 6.0) as u32;
            for i in 0..10u32 {
                let x = -41.0 + i as f32 * 8.4;
                let r = rnd(i, tick);
                let c = if r > 0.6 {
                    LED_GREEN
                } else if r > 0.45 {
                    LED_AMBER
                } else {
                    PANEL
                };
                rect(g, x, 13.5, 4.0, 3.0, c);
            }
        }
        _ => {
            // Patched cables dropping off the front edge onto the floor.
            let cols = [COPPER, GLOW_CYAN, GLOW_MAGENTA, LED_GREEN];
            for (k, &c) in cols.iter().enumerate() {
                let x = -34.0 + k as f32 * 21.0;
                let sway = (time * 0.8 + k as f32).sin() * 2.0;
                line(g, x, 18.0, x + 4.0 + sway, 30.0, 2.0, c);
                line(g, x + 4.0 + sway, 30.0, x - 2.0 + sway, 42.0, 2.0, c);
            }
        }
    }
}

/// 5 — cable junction: colour-coded runs crossing the floor between two
/// pull boxes. Layers: boxes, cables.
pub(super) fn cable_junction(g: &Graphics, layer: usize, time: f32) {
    match layer {
        0 => {
            for &x in &[-45.0f32, 27.0] {
                shadow_rect(g, x, -30.0, 18.0, 60.0);
                rect(g, x, -30.0, 18.0, 60.0, STEEL);
                frame(g, x, -30.0, 18.0, 60.0, 2.0, TRIM);
                for i in 0..6 {
                    circle(g, x + 9.0, -24.0 + i as f32 * 9.5, 2.5, PANEL);
                }
            }
        }
        _ => {
            let cable_colors = [COPPER, GLOW_CYAN, GLOW_MAGENTA, LED_GREEN, LED_AMBER, TRIM];
            // A fixed shuffle: left gland k runs to right gland (k * 5 + 2) % 6.
            for (k, &c) in cable_colors.iter().enumerate() {
                let ly = -24.0 + k as f32 * 9.5;
                let ry = -24.0 + ((k * 5 + 2) % 6) as f32 * 9.5;
                let slack = 2.0 * ((time * 0.5 + k as f32).sin()); // loose runs shift
                let mid = (ly + ry) / 2.0 + slack;
                line(g, -27.0, ly, 0.0, mid, 2.0, c);
                line(g, 0.0, mid, 27.0, ry, 2.0, c);
            }
        }
    }
}

/// 6 — operator desk from above: an edge-on monitor slab washing terminal
/// light across the desk, keyboard, mouse, paperwork, the mug that never
/// gets finished. Layers: desk, paper a, paper b (static tilts), mug.
pub(super) fn operator_desk(g: &Graphics, layer: usize, time: f32) {
    match layer {
        0 => {
            shadow_rect(g, -45.0, -28.0, 90.0, 52.0);
            rect(
                g,
                -45.0,
                -28.0,
                90.0,
                52.0,
                Color::new(0.28, 0.20, 0.24, 1.0),
            );
            frame(
                g,
                -45.0,
                -28.0,
                90.0,
                52.0,
                2.0,
                Color::new(0.40, 0.30, 0.34, 1.0),
            );
            // Screen light spilling toward the operator: a beam that
            // starts at the screen's own width and spreads as it fades.
            let flick = 0.9 + 0.1 * (time * 13.0).sin();
            for i in 0..3 {
                let t = i as f32;
                rect(
                    g,
                    -16.0 - t * 3.0,
                    -14.0 + t * 6.0,
                    32.0 + t * 6.0,
                    6.0,
                    Color::new(0.25, 0.85, 0.45, (0.20 - t * 0.06) * flick),
                );
            }
            rect(g, -5.0, -26.0, 10.0, 5.0, STEEL_DARK); // stand foot behind
            rect(g, -17.0, -22.0, 34.0, 7.0, PANEL); // the monitor slab
            rect(
                g,
                -17.0,
                -15.5,
                34.0,
                2.0,
                Color::new(0.35, 0.95, 0.55, 0.8 * flick),
            );
            // Keyboard + mouse.
            rect(g, -18.0, 2.0, 30.0, 12.0, STEEL_DARK);
            for r in 0..2 {
                for i in 0..7 {
                    rect(
                        g,
                        -16.0 + i as f32 * 4.0,
                        4.0 + r as f32 * 5.0,
                        2.5,
                        3.0,
                        TRIM,
                    );
                }
            }
            circle(g, 21.0, 8.0, 3.0, TRIM);
        }
        // Paperwork drift on the left (each sheet a tilted layer).
        1 => rect(g, -8.0, -10.0, 16.0, 20.0, alpha(CREAM, 0.8)),
        2 => {
            rect(g, -8.0, -10.0, 16.0, 20.0, alpha(CREAM, 0.9));
            for i in 0..4 {
                let y = -6.0 + i as f32 * 4.0;
                line(g, -5.0, y, 5.0, y, 1.0, alpha(PANEL, 0.5));
            }
        }
        _ => {
            // The mug, seen from above, handle out.
            circle(g, 36.0, 14.0, 5.5, Color::new(0.85, 0.47, 0.34, 1.0));
            circle(g, 42.0, 14.0, 2.0, Color::new(0.85, 0.47, 0.34, 1.0));
            circle(g, 36.0, 14.0, 3.5, Color::new(0.16, 0.09, 0.07, 1.0));
        }
    }
}

/// 7 — control console from above: three angled screen slabs washing
/// green / amber / static across a winged desk, the chair pushed back.
/// Layers: shadow, wing l, wing r (static angles), desk, screen l / c / r
/// (static angles), chair.
pub(super) fn control_console(g: &Graphics, layer: usize, time: f32) {
    match layer {
        0 => shadow_rect(g, -34.0, -34.0, 68.0, 26.0),
        // The desk wings (angled by the layer rotation).
        1 | 2 => rect(
            g,
            -16.0,
            -12.0,
            32.0,
            24.0,
            Color::new(0.22, 0.19, 0.27, 1.0),
        ),
        3 => rect(
            g,
            -34.0,
            -34.0,
            68.0,
            26.0,
            Color::new(0.24, 0.21, 0.29, 1.0),
        ),
        4..=6 => {
            // The three feeds; the right one has dropped to flickering static.
            let k = layer - 4;
            let c = match k {
                0 => Color::new(0.30, 0.90, 0.50, 1.0),
                1 => Color::new(1.0, 0.72, 0.20, 1.0),
                _ => Color::new(0.30, 0.95, 1.0, 1.0),
            };
            let fl = if k == 2 {
                0.4 + 0.6 * rnd(3, (time * 14.0) as u32)
            } else {
                0.9 + 0.1 * (time * (9.0 + k as f32 * 2.0)).sin()
            };
            // The wash: the screen's width at the slab, spreading out.
            for i in 0..3 {
                let t = i as f32;
                rect(
                    g,
                    -11.0 - t * 2.5,
                    4.0 + t * 6.0,
                    22.0 + t * 5.0,
                    6.0,
                    alpha(c, (0.18 - t * 0.05) * fl),
                );
            }
            rect(g, -12.0, -3.0, 24.0, 6.0, PANEL);
            rect(g, -12.0, 3.0, 24.0, 1.8, alpha(c, 0.85 * fl));
        }
        _ => {
            // The chair, drifting as if someone just left it.
            let j = (time * 0.4).sin() * 1.5;
            shadow_circle(g, j, 26.0, 11.0);
            g.draw_arc(Vec2::new(j, 26.0), 13.5, 0.35, PI - 0.35, STEEL_DARK); // backrest
            circle(g, j, 26.0, 9.5, Color::new(0.30, 0.26, 0.36, 1.0));
            circle(g, j, 26.0, 4.0, STEEL_DARK);
        }
    }
}

/// 8 — holo table: a spinning wireframe projection over a round pedestal.
/// Layers: pedestal, hologram (self-animated: an elliptical orbit, so it
/// is not a rigid layer rotation).
pub(super) fn holo_table(g: &Graphics, layer: usize, time: f32) {
    match layer {
        0 => {
            shadow_circle(g, 0.0, 14.0, 27.0);
            circle(g, 0.0, 14.0, 27.0, TRIM);
            circle(g, 0.0, 14.0, 23.0, STEEL_DARK);
            circle(
                g,
                0.0,
                14.0,
                6.0,
                alpha(GLOW_CYAN, 0.5 + 0.3 * (time * 2.0).sin()),
            );
        }
        _ => {
            // The projected wireframe: a triangle spinning above the emitter.
            let mut pts = [(0.0f32, 0.0f32); 3];
            for (k, p) in pts.iter_mut().enumerate() {
                let a = time * 0.9 + k as f32 * (TAU / 3.0);
                *p = (a.cos() * 26.0, a.sin() * 10.0 - 18.0);
            }
            let flick = 0.55 + 0.25 * (time * 17.0).sin();
            for k in 0..3 {
                let (x1, y1) = pts[k];
                let (x2, y2) = pts[(k + 1) % 3];
                line(g, x1, y1, x2, y2, 1.5, alpha(GLOW_CYAN, flick));
                // Projection beams back down to the emitter.
                line(g, x1, y1, 0.0, 12.0, 1.0, alpha(GLOW_CYAN, 0.18));
            }
        }
    }
}

/// 9 — CRAC cooling unit from above: big housing, top vents, the main
/// blower set into the roof, a heartbeat status LED on the front edge.
/// Layers: housing, blower (spinning), led.
pub(super) fn crac_cooler(g: &Graphics, layer: usize, time: f32) {
    match layer {
        0 => {
            shadow_rect(g, -40.0, -45.0, 80.0, 90.0);
            rect(g, -40.0, -45.0, 80.0, 90.0, STEEL);
            frame(g, -40.0, -45.0, 80.0, 90.0, 2.0, TRIM);
            for i in 0..3 {
                rect(g, -32.0, -39.0 + i as f32 * 6.0, 64.0, 2.5, PANEL);
            }
        }
        1 => {
            circle(g, 0.0, 0.0, 26.0, PANEL);
            for k in 0..4 {
                let a = k as f32 * FRAC_PI_2;
                g.draw_arc(Vec2::new(0.0, 0.0), 23.0, a, a + 0.62, STEEL_DARK);
                g.draw_arc(Vec2::new(0.0, 0.0), 23.0, a + 0.1, a + 0.5, TRIM);
            }
            circle(g, 0.0, 0.0, 5.0, TRIM);
        }
        _ => {
            rect(g, -40.0, 39.0, 80.0, 6.0, STEEL_DARK);
            let ok = blink(time, 1.2, 0.0, 0.9);
            rect(
                g,
                28.0,
                40.0,
                6.0,
                4.0,
                if ok { LED_GREEN } else { LED_RED },
            );
        }
    }
}

/// 10 — raised-floor vent tile with a faint airflow shimmer.
/// Layers: grille, airflow.
pub(super) fn floor_vent(g: &Graphics, layer: usize, time: f32) {
    match layer {
        0 => {
            rect(
                g,
                -42.0,
                -42.0,
                84.0,
                84.0,
                Color::new(0.13, 0.12, 0.17, 1.0),
            );
            frame(g, -42.0, -42.0, 84.0, 84.0, 2.0, TRIM);
            for i in 0..7 {
                let o = -36.0 + i as f32 * 12.0;
                rect(g, o, -36.0, 3.0, 72.0, PANEL);
                rect(g, -36.0, o, 72.0, 3.0, PANEL);
            }
        }
        _ => {
            // Cold air rippling out of the grille.
            for k in 0..2u32 {
                let ph = (time * 0.5 + k as f32 * 0.5).fract();
                let r = 8.0 + ph * 30.0;
                circle(g, 0.0, 0.0, r, Color::new(0.5, 0.8, 1.0, 0.10 * (1.0 - ph)));
            }
        }
    }
}

/// 11 — floor exhaust duct: five blades spinning under a safety cross.
/// Layers: duct, blades (spinning), guard.
pub(super) fn exhaust_fan(g: &Graphics, layer: usize, _time: f32) {
    match layer {
        0 => {
            rect(g, -42.0, -42.0, 84.0, 84.0, STEEL_DARK);
            frame(g, -42.0, -42.0, 84.0, 84.0, 2.0, TRIM);
            circle(g, 0.0, 0.0, 36.0, PANEL);
        }
        1 => {
            for k in 0..5 {
                let a = k as f32 * (TAU / 5.0);
                g.draw_arc(Vec2::new(0.0, 0.0), 32.0, a, a + 0.7, STEEL);
            }
            circle(g, 0.0, 0.0, 8.0, TRIM);
        }
        _ => {
            line(g, -36.0, 0.0, 36.0, 0.0, 3.0, alpha(TRIM, 0.85));
            line(g, 0.0, -36.0, 0.0, 36.0, 3.0, alpha(TRIM, 0.85));
        }
    }
}

/// 12 — coolant tank seen from above: liquid, rising bubbles, bolted hatch.
/// Layers: tank, bubbles, hatch.
pub(super) fn coolant_tank(g: &Graphics, layer: usize, time: f32) {
    match layer {
        0 => {
            shadow_circle(g, 0.0, 0.0, 38.0);
            circle(g, 0.0, 0.0, 38.0, TRIM);
            circle(g, 0.0, 0.0, 34.0, Color::new(0.08, 0.20, 0.26, 1.0));
            circle(g, 0.0, 0.0, 30.0, Color::new(0.10, 0.42, 0.52, 0.85));
        }
        1 => {
            for k in 0..5u32 {
                let ph = (time * 0.28 + k as f32 * 0.37).fract();
                let x = (k as f32 * 2.4).sin() * 17.0 * (1.0 - ph * 0.4);
                let y = 20.0 - ph * 40.0;
                let r = 1.5 + rnd(k, 2) * 2.5;
                circle(g, x, y, r, Color::new(0.6, 0.9, 1.0, 0.5 * (1.0 - ph)));
            }
        }
        _ => {
            circle(g, 0.0, 0.0, 9.0, STEEL);
            for k in 0..4 {
                let a = k as f32 * FRAC_PI_2 + 0.4;
                circle(g, a.cos() * 6.0, a.sin() * 6.0, 1.5, PANEL);
            }
        }
    }
}

/// 13 — overhead pipe run seen from below the camera: two runs casting
/// floor shadows, flanges, a red valve wheel creeping.
/// Layers: pipes, valve (swaying).
pub(super) fn pipe_run(g: &Graphics, layer: usize, _time: f32) {
    match layer {
        0 => {
            // The pipes hang overhead, so their shadows fall well below them.
            rect(g, -50.0, -13.0, 100.0, 13.0, alpha(SHADOW, 0.20));
            rect(g, -50.0, 14.0, 100.0, 13.0, alpha(SHADOW, 0.20));
            rect(
                g,
                -50.0,
                -20.0,
                100.0,
                13.0,
                Color::new(0.30, 0.26, 0.34, 1.0),
            );
            rect(g, -50.0, -19.0, 100.0, 3.0, alpha(CREAM, 0.25)); // glint
            rect(g, -50.0, 7.0, 100.0, 13.0, COPPER);
            rect(g, -50.0, 8.0, 100.0, 3.0, alpha(CREAM, 0.2));
            for &x in &[-30.0, 10.0] {
                rect(g, x, -22.0, 6.0, 17.0, TRIM);
            }
            for &x in &[-12.0, 34.0] {
                rect(g, x, 5.0, 6.0, 17.0, TRIM);
            }
        }
        _ => {
            // The valve wheel creeps as pressure is trimmed (the sway).
            circle(g, 0.0, 0.0, 9.5, Color::new(0.72, 0.16, 0.20, 1.0));
            circle(g, 0.0, 0.0, 3.0, STEEL_DARK);
            for k in 0..3 {
                let a = k as f32 * (PI / 3.0);
                line(
                    g,
                    -a.cos() * 9.0,
                    -a.sin() * 9.0,
                    a.cos() * 9.0,
                    a.sin() * 9.0,
                    2.0,
                    Color::new(0.5, 0.10, 0.14, 1.0),
                );
            }
        }
    }
}

/// 14 — UPS cabinet from above: hazard strip and cable glands at the
/// back, vented lid with the bolt painted on, charge LEDs up front.
/// Layers: cabinet, bolt, charge.
pub(super) fn ups_cabinet(g: &Graphics, layer: usize, time: f32) {
    match layer {
        0 => {
            shadow_rect(g, -30.0, -45.0, 60.0, 90.0);
            rect(g, -30.0, -45.0, 60.0, 90.0, STEEL);
            frame(g, -30.0, -45.0, 60.0, 90.0, 2.0, TRIM);
            // Rear cable glands, feeds ducking out the back.
            for k in 0..2u32 {
                let x = -12.0 + k as f32 * 24.0;
                line(
                    g,
                    x,
                    -40.0,
                    x + (k as f32 - 0.5) * 10.0,
                    -49.0,
                    3.0,
                    if k == 0 { COPPER } else { STEEL_DARK },
                );
                circle(g, x, -38.0, 4.5, STEEL_DARK);
                circle(g, x, -38.0, 2.0, PANEL);
            }
            // Hazard strip across the lid.
            for i in 0..7 {
                let c = if i % 2 == 0 {
                    HAZARD_YELLOW
                } else {
                    Color::new(0.05, 0.05, 0.06, 1.0)
                };
                rect(g, -28.0 + i as f32 * 8.0, -28.0, 8.0, 6.0, c);
            }
            // Vent grille.
            for i in 0..4 {
                rect(g, -22.0, -16.0 + i as f32 * 7.0, 44.0, 2.5, PANEL);
            }
        }
        1 => {
            // The bolt, painted on the lid.
            let on = blink(time, 1.0, 0.25, 0.7);
            let c = if on {
                HAZARD_YELLOW
            } else {
                alpha(HAZARD_YELLOW, 0.35)
            };
            line(g, 4.0, 16.0, -2.0, 24.0, 2.5, c);
            line(g, -2.0, 24.0, 3.0, 24.0, 2.5, c);
            line(g, 3.0, 24.0, -4.0, 32.0, 2.5, c);
        }
        _ => {
            // Front edge: charge readout breathing between 4 and 5 bars.
            rect(g, -30.0, 37.0, 60.0, 8.0, STEEL_DARK);
            let fill = 4 + if blink(time, 0.5, 0.0, 0.5) { 1 } else { 0 };
            for i in 0..5 {
                let c = if i < fill { LED_GREEN } else { PANEL };
                rect(g, -24.0 + i as f32 * 10.0, 39.0, 7.0, 4.0, c);
            }
        }
    }
}

/// 15 — backup generator from above: engine block with cooling fins, the
/// round alternator, exhaust rings drifting off the stack.
/// Layers: block, smoke, stack, gauge (needle = the layer rotation).
pub(super) fn generator(g: &Graphics, layer: usize, time: f32) {
    match layer {
        0 => {
            shadow_rect(g, -45.0, -25.0, 90.0, 64.0);
            rect(g, -45.0, 25.0, 90.0, 14.0, STEEL_DARK); // skid
            frame(g, -45.0, 25.0, 90.0, 14.0, 1.5, TRIM);
            rect(
                g,
                -40.0,
                -25.0,
                55.0,
                50.0,
                Color::new(0.30, 0.28, 0.24, 1.0),
            );
            frame(
                g,
                -40.0,
                -25.0,
                55.0,
                50.0,
                2.0,
                Color::new(0.42, 0.39, 0.33, 1.0),
            );
            for i in 0..5 {
                rect(
                    g,
                    -34.0 + i as f32 * 10.0,
                    -20.0,
                    4.0,
                    40.0,
                    Color::new(0.20, 0.19, 0.16, 1.0),
                );
            }
            circle(g, 28.0, 0.0, 16.0, STEEL);
            circle(g, 28.0, 0.0, 5.0, TRIM);
        }
        1 => {
            // The exhaust stack pokes up past the block; from above its
            // smoke spreads as widening rings.
            for k in 0..3u32 {
                let ph = (time * 0.45 + k as f32 / 3.0).fract();
                circle(
                    g,
                    0.0,
                    0.0,
                    7.0 + ph * 13.0,
                    Color::new(0.5, 0.5, 0.55, 0.22 * (1.0 - ph)),
                );
            }
        }
        2 => {
            rect(g, -33.0, -31.0, 4.0, 8.0, STEEL_DARK); // stack feed
            circle(g, -31.0, -36.0, 6.5, STEEL_DARK);
            circle(g, -31.0, -36.0, 3.0, PANEL);
        }
        _ => {
            // Fuel gauge on the block; the needle (along +x here) trembles
            // through the layer rotation while it runs.
            circle(g, 0.0, 0.0, 7.0, CREAM);
            line(g, 0.0, 0.0, 5.5, 0.0, 1.5, LED_RED);
        }
    }
}

/// 16 — open cable tray: a river of colour-coded runs.
/// Layers: tray, cables.
pub(super) fn cable_tray(g: &Graphics, layer: usize, time: f32) {
    match layer {
        0 => {
            rect(
                g,
                -48.0,
                -20.0,
                96.0,
                40.0,
                Color::new(0.14, 0.13, 0.18, 1.0),
            );
            rect(g, -48.0, -24.0, 96.0, 4.0, TRIM);
            rect(g, -48.0, 20.0, 96.0, 4.0, TRIM);
        }
        _ => {
            let colors = [COPPER, GLOW_CYAN, LED_GREEN, GLOW_MAGENTA, LED_AMBER];
            for (c_idx, &col) in colors.iter().enumerate() {
                let y0 = -13.0 + c_idx as f32 * 6.5;
                let mut lx = -46.0;
                let mut ly = y0;
                for seg in 1..=8 {
                    let nx = -46.0 + seg as f32 * 11.5;
                    // The live cable (cyan) hums; the rest lie still.
                    let wob = if c_idx == 1 {
                        (time * 2.0 + seg as f32).sin()
                    } else {
                        0.0
                    };
                    let ny = y0 + ((nx * 0.15) + c_idx as f32 * 2.1).sin() * 2.5 + wob;
                    line(g, lx, ly, nx, ny, 2.5, col);
                    lx = nx;
                    ly = ny;
                }
            }
        }
    }
}

/// 17 — spare cable coil with its loose end and connector.
/// Layers: coil, glints (spinning), lead.
pub(super) fn cable_coil(g: &Graphics, layer: usize, _time: f32) {
    match layer {
        0 => {
            shadow_circle(g, 0.0, 0.0, 33.0);
            circle(g, 0.0, 0.0, 33.0, Color::new(0.48, 0.28, 0.20, 1.0));
            circle(g, 0.0, 0.0, 26.0, COPPER);
            circle(g, 0.0, 0.0, 19.0, Color::new(0.48, 0.28, 0.20, 1.0));
            circle(g, 0.0, 0.0, 12.0, Color::new(0.10, 0.09, 0.13, 1.0));
        }
        1 => {
            // Wind glints slowly circling the coil.
            for k in 0..3 {
                let a = k as f32 * (TAU / 3.0);
                g.draw_arc(Vec2::new(0.0, 0.0), 29.5, a, a + 0.9, alpha(CREAM, 0.18));
            }
        }
        _ => {
            line(g, 28.0, 14.0, 42.0, 26.0, 4.0, COPPER);
            rect(g, 40.0, 24.0, 8.0, 8.0, STEEL);
        }
    }
}

/// 18 — tape library from above, lid off: cartridge racks down both long
/// walls, the picker robot riding the centre rail and reaching into the
/// shelves. Layers: chassis, picker (self-animated along the rail).
pub(super) fn tape_library(g: &Graphics, layer: usize, time: f32) {
    match layer {
        0 => {
            shadow_rect(g, -42.0, -45.0, 84.0, 90.0);
            rect(g, -42.0, -45.0, 84.0, 90.0, STEEL_DARK);
            frame(g, -42.0, -45.0, 84.0, 90.0, 2.0, TRIM);
            rect(g, -37.0, -40.0, 74.0, 80.0, PANEL); // the open bay
                                                      // Cartridge racks along the two long walls.
            for side in 0..2u32 {
                let x = if side == 0 { -35.0 } else { 21.0 };
                for row in 0..7u32 {
                    let y = -38.0 + row as f32 * 11.0;
                    if rnd(row * 7 + side, 11) > 0.82 {
                        rect(g, x, y, 14.0, 9.0, Color::new(0.05, 0.05, 0.09, 1.0));
                    // empty
                    } else {
                        let v = 0.2 + rnd(side * 3 + row, row) * 0.15;
                        rect(g, x, y, 14.0, 9.0, Color::new(v, v * 0.95, v * 1.2, 1.0));
                        rect(g, x + 2.0, y + 3.0, 10.0, 2.0, alpha(CREAM, 0.35));
                    }
                }
            }
            // Centre rail.
            rect(g, -2.0, -38.0, 4.0, 76.0, TRIM);
        }
        _ => {
            // The carriage rides the rail, arm reaching into a shelf.
            let ay = (time * 0.7).sin() * 28.0;
            let reach = ((time * 1.1).sin() * 0.5 + 0.5) * 14.0;
            let side = if (time * 0.23).sin() > 0.0 { 1.0 } else { -1.0 };
            line(g, 0.0, ay, side * (6.0 + reach), ay, 4.0, alpha(CREAM, 0.9));
            rect(g, side * (6.0 + reach) - 3.0, ay - 4.0, 6.0, 8.0, CREAM); // gripper
            rect(g, -7.0, ay - 8.0, 14.0, 16.0, CREAM); // carriage
            let seek = blink(time, 5.0, 0.0, 0.5);
            rect(
                g,
                -2.0,
                ay - 2.0,
                4.0,
                4.0,
                if seek { LED_RED } else { PANEL },
            );
        }
    }
}

/// 19 — strapped supply crate, stencilled for the datacenter.
/// Layers: crate, stencil.
pub(super) fn supply_crate(g: &Graphics, layer: usize, _time: f32) {
    match layer {
        0 => {
            shadow_rect(g, -38.0, -32.0, 76.0, 64.0);
            rect(
                g,
                -38.0,
                -32.0,
                76.0,
                64.0,
                Color::new(0.35, 0.27, 0.20, 1.0),
            );
            frame(
                g,
                -38.0,
                -32.0,
                76.0,
                64.0,
                2.5,
                Color::new(0.24, 0.18, 0.13, 1.0),
            );
            rect(
                g,
                -38.0,
                -6.0,
                76.0,
                12.0,
                Color::new(0.22, 0.17, 0.13, 1.0),
            );
            rect(
                g,
                -8.0,
                -32.0,
                16.0,
                64.0,
                Color::new(0.26, 0.20, 0.15, 1.0),
            );
            for &(x, y) in &[(-33.0, -27.0), (28.0, -27.0), (-33.0, 22.0), (28.0, 22.0)] {
                circle(g, x + 2.5, y + 2.5, 2.0, Color::new(0.15, 0.11, 0.08, 1.0));
            }
        }
        _ => g.draw_text(
            "CLD-01",
            Vec2::new(-22.0, 24.0),
            13.0,
            Color::new(0.9, 0.85, 0.7, 0.55),
        ),
    }
}

/// 20 — security camera from above: a pivot on its wall stub, panning a
/// long watch cone across the floor (the same read as the rogues' vision
/// cones). Layers: mount, cone (swaying), head (swaying with it).
pub(super) fn security_cam(g: &Graphics, layer: usize, time: f32) {
    match layer {
        0 => {
            rect(g, -16.0, -48.0, 32.0, 10.0, STEEL_DARK); // the wall stub
            frame(g, -16.0, -48.0, 32.0, 10.0, 1.5, TRIM);
        }
        1 => {
            // The watch cone sweeps the floor first, under the body.
            g.draw_arc(
                Vec2::new(0.0, 0.0),
                64.0,
                FRAC_PI_2 - 0.30,
                FRAC_PI_2 + 0.30,
                Color::new(1.0, 0.2, 0.2, 0.07),
            );
            g.draw_arc(
                Vec2::new(0.0, 0.0),
                30.0,
                FRAC_PI_2 - 0.30,
                FRAC_PI_2 + 0.30,
                Color::new(1.0, 0.2, 0.2, 0.06),
            );
        }
        _ => {
            // Body seen from above: housing, head, lens looking down the cone.
            rect(g, -5.0, -2.0, 10.0, 14.0, STEEL);
            rect(g, -7.0, 12.0, 14.0, 8.0, STEEL_DARK);
            circle(g, 0.0, 20.0, 3.5, PANEL);
            circle(g, 0.0, 20.0, 1.5, GLOW_CYAN);
            circle(g, 0.0, 0.0, 6.0, TRIM); // the pivot
            let recording = blink(time, 1.0, 0.0, 0.12);
            circle(g, 4.5, 4.0, 1.8, if recording { LED_RED } else { PANEL });
        }
    }
}

/// 21 — fire suppression pair from above: two agent tanks, handwheels up,
/// plumbed into the discharge manifold along the back wall.
/// Layers: manifold, tanks, wheel l, wheel r (creeping), tag.
pub(super) fn fire_suppressor(g: &Graphics, layer: usize, _time: f32) {
    match layer {
        0 => {
            rect(g, -30.0, -44.0, 60.0, 7.0, STEEL); // the manifold
            for k in 0..4 {
                circle(g, -21.0 + k as f32 * 14.0, -36.0, 2.0, PANEL); // nozzles
            }
            for &x in &[-18.0f32, 18.0] {
                line(g, x, -37.0, x, -12.0, 4.0, TRIM); // feed pipe
            }
        }
        1 => {
            for &x in &[-18.0f32, 18.0] {
                shadow_circle(g, x, 8.0, 17.0);
                circle(g, x, 8.0, 17.0, TRIM);
                circle(g, x, 8.0, 14.5, Color::new(0.72, 0.14, 0.18, 1.0));
                circle(g, x, 8.0, 6.5, Color::new(0.55, 0.10, 0.14, 1.0)); // shoulder
            }
        }
        2 | 3 => {
            // The handwheel on top, creeping as pressure is trimmed (the
            // layer rotation).
            for s in 0..3 {
                let a = s as f32 * (PI / 3.0);
                line(
                    g,
                    -a.cos() * 8.5,
                    -a.sin() * 8.5,
                    a.cos() * 8.5,
                    a.sin() * 8.5,
                    1.8,
                    STEEL,
                );
            }
            circle(g, 0.0, 0.0, 2.2, TRIM);
        }
        // Inspection tag on the floor between them.
        _ => rect(g, -4.0, 32.0, 8.0, 10.0, alpha(HAZARD_YELLOW, 0.8)),
    }
}

/// 22 — hazard floor pad: striped border around a KEEP CLEAR zone.
/// Layers: pad, sign.
pub(super) fn hazard_pad(g: &Graphics, layer: usize, _time: f32) {
    match layer {
        0 => {
            rect(
                g,
                -44.0,
                -44.0,
                88.0,
                88.0,
                Color::new(0.12, 0.11, 0.15, 1.0),
            );
            let black = Color::new(0.05, 0.05, 0.06, 1.0);
            for i in 0..11 {
                let o = -44.0 + i as f32 * 8.0;
                let c = if i % 2 == 0 { HAZARD_YELLOW } else { black };
                let c2 = if i % 2 == 0 { black } else { HAZARD_YELLOW };
                rect(g, o, -44.0, 8.0, 8.0, c);
                rect(g, o, 36.0, 8.0, 8.0, c2);
                rect(g, -44.0, o, 8.0, 8.0, c2);
                rect(g, 36.0, o, 8.0, 8.0, c);
            }
        }
        _ => {
            // Warning triangle painted in the middle.
            line(g, 0.0, -18.0, 16.0, 12.0, 3.0, HAZARD_YELLOW);
            line(g, 16.0, 12.0, -16.0, 12.0, 3.0, HAZARD_YELLOW);
            line(g, -16.0, 12.0, 0.0, -18.0, 3.0, HAZARD_YELLOW);
            rect(g, -1.5, -8.0, 3.0, 10.0, HAZARD_YELLOW);
            rect(g, -1.5, 5.0, 3.0, 3.0, HAZARD_YELLOW);
        }
    }
}

/// 23 — the uplink obelisk from above: a diamond monolith top with seams
/// bleeding light toward its points, an escort of motes orbiting it.
/// Layers: aura, shadow, monolith (both static 45°), seams, escort.
pub(super) fn uplink_obelisk(g: &Graphics, layer: usize, time: f32) {
    let breath = 0.5 + 0.5 * (time * 2.0).sin();
    match layer {
        0 => {
            circle(g, 0.0, 0.0, 44.0, alpha(GLOW_MAGENTA, 0.07 + 0.05 * breath));
            circle(g, 0.0, 0.0, 30.0, alpha(GLOW_MAGENTA, 0.06 + 0.05 * breath));
        }
        // The monolith reads as a diamond from above (its shadow first).
        1 => rect(g, -16.0, -16.0, 32.0, 32.0, SHADOW),
        2 => {
            rect(
                g,
                -16.0,
                -16.0,
                32.0,
                32.0,
                Color::new(0.07, 0.05, 0.10, 1.0),
            );
            frame(
                g,
                -16.0,
                -16.0,
                32.0,
                32.0,
                1.5,
                Color::new(0.35, 0.15, 0.35, 1.0),
            );
        }
        3 => {
            // Seams bleeding light toward the four points, the core white-hot.
            let seam = alpha(GLOW_MAGENTA, 0.35 + 0.4 * breath);
            line(g, 0.0, 0.0, 21.0, 0.0, 2.0, seam);
            line(g, 0.0, 0.0, -21.0, 0.0, 2.0, seam);
            line(g, 0.0, 0.0, 0.0, 21.0, 2.0, seam);
            line(g, 0.0, 0.0, 0.0, -21.0, 2.0, seam);
            circle(g, 0.0, 0.0, 6.0, alpha(GLOW_MAGENTA, 0.5 + 0.4 * breath));
            circle(g, 0.0, 0.0, 2.5, alpha(CREAM, 0.8));
        }
        _ => {
            // The escort.
            for k in 0..3 {
                let a = time * 1.3 + k as f32 * 2.1;
                let c = if k % 2 == 0 { GLOW_MAGENTA } else { GLOW_CYAN };
                circle(g, a.cos() * 32.0, a.sin() * 36.0, 2.5, alpha(c, 0.85));
            }
        }
    }
}
