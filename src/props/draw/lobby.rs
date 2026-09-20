//! The LOBBY family: props 42..59, one `fn(g, layer, time)` each.

use super::*;

/// A chevron (arrow head) pointing towards -y, centred on (x, y).
pub(super) fn chevron(g: &Graphics, x: f32, y: f32, s: f32, th: f32, c: Color) {
    line(g, x - s, y + s * 0.6, x, y - s * 0.6, th, c);
    line(g, x, y - s * 0.6, x + s, y + s * 0.6, th, c);
}

/// A round chair from above, seat facing +y (backrest at -y).
pub(super) fn chair(g: &Graphics, x: f32, y: f32) {
    shadow_circle(g, x, y, 11.0);
    g.draw_arc(Vec2::new(x, y), 12.0, PI + 0.35, TAU - 0.35, STEEL_DARK);
    circle(g, x, y, 9.5, Color::new(0.30, 0.26, 0.36, 1.0));
    circle(g, x, y, 4.0, STEEL_DARK);
}

/// 42 — the reception desk: a long walnut counter with angled return
/// wings (static layers), a breathing light strip along the visitor
/// side, the desk lamp's pool, a bell, paperwork, an edge-on terminal
/// washing green light back over the receptionist's side, and the chair
/// pushed back behind. Layers: shadow, wing l / r, desk, terminal, chair.
pub(super) fn reception_desk(g: &Graphics, layer: usize, time: f32) {
    match layer {
        0 => shadow_rect(g, -40.0, -10.0, 80.0, 20.0),
        1 | 2 => {
            let x = if layer == 1 { -26.0 } else { 0.0 };
            rect(g, x + 4.0, -6.0, 26.0, 20.0, SHADOW);
            rect(g, x, -10.0, 26.0, 20.0, WALNUT);
            rect(g, x, 4.0, 26.0, 6.0, WALNUT_LIGHT);
            rect(g, x, 8.0, 26.0, 2.0, alpha(GLOW_CYAN, 0.55));
        }
        3 => {
            rect(g, -40.0, -10.0, 80.0, 20.0, WALNUT);
            rect(g, -40.0, 4.0, 80.0, 6.0, WALNUT_LIGHT); // the raised transaction top
            let breath = 0.45 + 0.3 * (time * 1.2).sin();
            rect(g, -40.0, 8.0, 80.0, 2.0, alpha(GLOW_CYAN, breath));
            circle(g, -24.0, -3.0, 11.0, alpha(WARM_LIGHT, 0.22)); // lamp pool
            circle(g, -24.0, -3.0, 6.0, alpha(WARM_LIGHT, 0.22));
            circle(g, -24.0, -4.0, 4.0, STEEL); // lamp head
            circle(g, -24.0, -4.0, 2.5, WARM_LIGHT);
            circle(g, 24.0, -2.0, 3.5, BRASS); // the bell
            circle(g, 24.0, -2.0, 1.5, Color::new(0.95, 0.85, 0.55, 1.0));
            rect(g, 8.0, -6.0, 10.0, 8.0, alpha(CREAM, 0.85)); // paperwork
            line(g, 10.0, -3.0, 16.0, -3.0, 1.0, alpha(PANEL, 0.4));
            line(g, 10.0, -1.0, 15.0, -1.0, 1.0, alpha(PANEL, 0.4));
        }
        4 => {
            let fl = 0.9 + 0.1 * (time * 13.0).sin();
            wash_v(g, 0.0, -3.0, 24.0, -22.0, LED_GREEN, 0.18 * fl);
            rect(g, -12.0, -2.0, 24.0, 5.0, PANEL);
            rect(g, -12.0, -3.0, 24.0, 1.5, alpha(LED_GREEN, 0.8 * fl));
        }
        _ => chair(g, (time * 0.4).sin() * 1.5, -26.0),
    }
}

/// 43 — a pair of turnstile lanes: three housings with their card
/// readers, lane chevrons on the floor, the left arm locked (red), the
/// right arm swinging through with each walker (the layer's `Anim`,
/// green when open). Layers: floor, housings, arm l, arm r, leds.
pub(super) fn turnstiles(g: &Graphics, layer: usize, time: f32) {
    match layer {
        0 => {
            rect(g, -46.0, -2.0, 92.0, 4.0, RUBBER);
            for &x in &[-17.0f32, 17.0] {
                chevron(g, x, 18.0, 6.0, 2.5, alpha(PAINT, 0.5));
                chevron(g, x, 27.0, 6.0, 2.5, alpha(PAINT, 0.3));
            }
        }
        1 => {
            for &x in &[-40.0f32, 0.0, 40.0] {
                shadow_rect(g, x - 6.0, -20.0, 12.0, 40.0);
                rect(g, x - 6.0, -20.0, 12.0, 40.0, STEEL_DARK);
                frame(g, x - 6.0, -20.0, 12.0, 40.0, 1.5, TRIM);
                rect(g, x - 4.0, -16.0, 8.0, 20.0, GLASS);
                rect(g, x - 3.0, -6.0, 6.0, 4.0, alpha(GLOW_CYAN, 0.6)); // the reader
            }
        }
        2 => {
            rect(g, 2.0, 2.0, 28.0, 3.0, SHADOW);
            circle(g, 0.0, 0.0, 3.5, STEEL);
            rect(g, 0.0, -2.0, 28.0, 4.0, CHROME);
        }
        3 => {
            rect(g, -26.0, 2.0, 28.0, 3.0, SHADOW);
            circle(g, 0.0, 0.0, 3.5, STEEL);
            rect(g, -28.0, -2.0, 28.0, 4.0, CHROME);
        }
        _ => {
            let open = turnstile_angle(time) > 0.2;
            rect(g, -42.0, 14.0, 4.0, 3.0, LED_RED); // left lane: locked
            rect(g, -2.0, 14.0, 4.0, 3.0, LED_RED);
            let go = if open || blink(time, 1.0, 0.0, 0.5) {
                LED_GREEN
            } else {
                PANEL
            };
            rect(g, 3.0, 14.0, 4.0, 3.0, go);
            rect(g, 38.0, 14.0, 4.0, 3.0, go);
        }
    }
}

/// 44 — the security scanner arch over a rubber mat: two pillars joined
/// by the beam overhead (long shadows), a scan line sweeping the gap,
/// pass / deny LEDs on the pillar tops. Layers: mat, arch, sweep, leds.
pub(super) fn scanner_arch(g: &Graphics, layer: usize, time: f32) {
    match layer {
        0 => {
            rect(g, -40.0, -26.0, 80.0, 52.0, RUBBER);
            frame(g, -40.0, -26.0, 80.0, 52.0, 1.5, CONCRETE_DARK);
            circle(g, -4.0, 12.0, 3.0, alpha(CREAM, 0.12));
            circle(g, 4.0, 12.0, 3.0, alpha(CREAM, 0.12));
        }
        1 => {
            rect(g, -30.0, -14.0, 12.0, 40.0, SHADOW);
            rect(g, 30.0, -14.0, 12.0, 40.0, SHADOW);
            rect(g, -18.0, 1.0, 48.0, 10.0, SHADOW);
            rect(g, -36.0, -20.0, 12.0, 40.0, STEEL);
            frame(g, -36.0, -20.0, 12.0, 40.0, 1.5, TRIM);
            rect(g, 24.0, -20.0, 12.0, 40.0, STEEL);
            frame(g, 24.0, -20.0, 12.0, 40.0, 1.5, TRIM);
            rect(g, -24.0, -5.0, 48.0, 10.0, TRIM);
            rect(g, -24.0, -3.0, 48.0, 6.0, STEEL);
        }
        2 => {
            let y = (time * 1.5).sin() * 16.0;
            let y2 = (time * 1.5 - 0.15).sin() * 16.0;
            line(g, -24.0, y2, 24.0, y2, 1.2, alpha(GLOW_CYAN, 0.2));
            line(g, -24.0, y, 24.0, y, 1.2, alpha(GLOW_CYAN, 0.55));
        }
        _ => {
            let deny = (time * 0.5) as u32 % 3 == 2;
            let c = if deny {
                if blink(time, 4.0, 0.0, 0.5) {
                    LED_RED
                } else {
                    PANEL
                }
            } else {
                LED_GREEN
            };
            for &x in &[-33.0f32, 31.0] {
                rect(g, x, -17.0, 4.0, 3.0, c);
            }
        }
    }
}

/// A slatted bench `w` wide, in its layer's frame: end frames, backrest
/// rail along the back (-y), the slats over a dark gap.
pub(super) fn bench(g: &Graphics, w: f32) {
    let hw = w / 2.0;
    shadow_rect(g, -hw, -16.0, w, 30.0);
    rect(
        g,
        -hw + 5.0,
        -14.0,
        w - 10.0,
        28.0,
        Color::new(0.05, 0.05, 0.06, 1.0),
    );
    rect(g, -hw, -16.0, 5.0, 30.0, STEEL_DARK);
    rect(g, hw - 5.0, -16.0, 5.0, 30.0, STEEL_DARK);
    rect(g, -hw + 5.0, -18.0, w - 10.0, 4.0, WALNUT);
    for i in 0..5 {
        let c = if i % 2 == 0 { WALNUT_LIGHT } else { WALNUT };
        rect(g, -hw + 5.0, -13.0 + i as f32 * 5.4, w - 10.0, 4.0, c);
    }
}

/// 45 — the long waiting bench, a coffee cup and a folded paper left on
/// it. Layers: bench, items.
pub(super) fn bench_long(g: &Graphics, layer: usize, _time: f32) {
    match layer {
        0 => bench(g, 84.0),
        _ => {
            circle(g, 20.0, -2.0, 3.5, CREAM);
            circle(g, 20.0, -2.0, 2.0, Color::new(0.35, 0.20, 0.10, 1.0));
            rect(g, -26.0, -5.0, 12.0, 8.0, alpha(CREAM, 0.85));
            line(g, -24.0, -2.0, -16.0, -2.0, 1.0, alpha(PANEL, 0.4));
            line(g, -24.0, 0.5, -18.0, 0.5, 1.0, alpha(PANEL, 0.4));
        }
    }
}

/// 46 — the short bench, someone's backpack on it. Layers: bench, items.
pub(super) fn bench_short(g: &Graphics, layer: usize, _time: f32) {
    match layer {
        0 => bench(g, 56.0),
        _ => {
            rect(g, 8.0, -7.0, 10.0, 13.0, Color::new(0.15, 0.22, 0.40, 1.0));
            rect(g, 8.0, -7.0, 10.0, 3.0, Color::new(0.20, 0.30, 0.50, 1.0));
            line(g, 9.0, 6.0, 4.0, 10.0, 1.5, STEEL_DARK); // strap
        }
    }
}

/// 47 — a potted plant: terracotta pot, fronds fanning out and swaying,
/// the nursery tag. Layers: pot, leaves (sway), tag.
pub(super) fn potted_plant(g: &Graphics, layer: usize, _time: f32) {
    match layer {
        0 => {
            shadow_circle(g, 0.0, 0.0, 20.0);
            circle(g, 0.0, 0.0, 20.0, Color::new(0.62, 0.35, 0.25, 1.0));
            circle(g, 0.0, 0.0, 17.0, Color::new(0.16, 0.11, 0.08, 1.0));
            circle(g, -13.0, -13.0, 3.0, alpha(CREAM, 0.25));
        }
        1 => {
            for k in 0..8 {
                let a = k as f32 * (TAU / 8.0) + 0.3;
                let (tx, ty) = (a.cos() * 26.0, a.sin() * 26.0);
                line(g, 0.0, 0.0, tx, ty, 5.0, LEAF);
                circle(g, tx * 0.85, ty * 0.85, 4.0, LEAF_LIGHT);
                let b = a + TAU / 16.0;
                line(g, 0.0, 0.0, b.cos() * 16.0, b.sin() * 16.0, 4.0, LEAF_DARK);
            }
            circle(g, 0.0, 0.0, 5.0, LEAF_DARK);
        }
        _ => {
            line(g, 12.0, 6.0, 13.0, 11.0, 1.0, CREAM);
            rect(g, 10.0, 10.0, 7.0, 6.0, alpha(CREAM, 0.9));
        }
    }
}

/// 48 — the big lobby holo-screen: a wall-mounted edge-on slab, a wide
/// cool wash down the floor with a scan band rolling through it, glyphs
/// crawling along the face and a live bar graph. Layers: wash, slab,
/// glyphs.
pub(super) fn lobby_holo(g: &Graphics, layer: usize, time: f32) {
    match layer {
        0 => {
            let c = mix(
                GLOW_CYAN,
                Color::new(0.6, 0.5, 1.0, 1.0),
                0.5 + 0.5 * (time * 0.3).sin(),
            );
            for i in 0..5 {
                let t = i as f32;
                rect(
                    g,
                    -44.0 - t * 1.5,
                    -28.0 + t * 13.6,
                    88.0 + t * 3.0,
                    13.6,
                    alpha(c, 0.28 * (1.0 - t / 5.0)),
                );
            }
            let y = -28.0 + (time * 0.3).fract() * 68.0;
            rect(g, -50.0, y, 100.0, 3.0, alpha(CREAM, 0.08));
        }
        1 => {
            rect(g, -30.0, -44.0, 60.0, 6.0, STEEL_DARK); // wall bracket
            rect(g, -44.0, -38.0, 88.0, 10.0, PANEL);
            frame(g, -44.0, -38.0, 88.0, 10.0, 1.5, TRIM);
            let fl = 0.85 + 0.15 * (time * 7.0).sin();
            rect(g, -44.0, -29.0, 88.0, 2.0, alpha(GLOW_CYAN, 0.85 * fl));
        }
        _ => {
            glyph_strip(g, -42.0, 26.0, -36.0, time, GLOW_CYAN);
            let tick = (time * 3.0) as u32;
            for i in 0..5u32 {
                let h = 1.0 + rnd(i, tick) * 5.0;
                rect(g, 28.0 + i as f32 * 3.0, -30.0 - h, 2.0, h, GLOW_MAGENTA);
            }
        }
    }
}

/// 49 — the floor directory totem: a slim kiosk whose top face is a
/// display scrolling a listing, its front edge lit and washing the floor.
/// Layers: body, screen (scrolling), wash.
pub(super) fn directory_totem(g: &Graphics, layer: usize, time: f32) {
    match layer {
        0 => {
            rect(g, -5.0, -17.0, 20.0, 44.0, SHADOW);
            rect(g, -10.0, -22.0, 20.0, 44.0, STEEL);
            frame(g, -10.0, -22.0, 20.0, 44.0, 1.5, TRIM);
        }
        1 => {
            rect(g, -8.0, -18.0, 16.0, 30.0, PANEL);
            for k in 0..7u32 {
                let y = -16.0 + (k as f32 * 4.5 + time * 3.0).rem_euclid(26.0);
                let w = 4.0 + rnd(k, 1) * 7.0;
                let c = if k == 0 { CREAM } else { alpha(GLOW_CYAN, 0.8) };
                rect(g, -6.0, y, w, 2.0, c);
            }
        }
        _ => {
            rect(g, -10.0, 20.0, 20.0, 2.0, alpha(GLOW_CYAN, 0.7));
            wash_v(g, 0.0, 22.0, 20.0, 14.0, GLOW_CYAN, 0.16);
        }
    }
}

/// 50 — a vending machine: vented top, the lit front glass with its
/// product rows glowing along the front edge, the select panel, and the
/// fluorescent wash on the floor in front (with the odd flicker).
/// Layers: body, front, wash.
pub(super) fn vending_machine(g: &Graphics, layer: usize, time: f32) {
    match layer {
        0 => {
            rect(g, -19.0, -15.0, 48.0, 40.0, SHADOW);
            rect(g, -24.0, -20.0, 48.0, 40.0, STEEL);
            frame(g, -24.0, -20.0, 48.0, 40.0, 2.0, TRIM);
            for i in 0..4 {
                rect(g, -16.0, -14.0 + i as f32 * 5.0, 32.0, 2.0, PANEL);
            }
            rect(g, -24.0, 6.0, 48.0, 3.0, alpha(LED_RED, 0.7)); // brand band
        }
        1 => {
            rect(g, -22.0, 13.0, 44.0, 7.0, GLASS);
            let cols = [
                LED_RED,
                LED_AMBER,
                GLOW_CYAN,
                GLOW_MAGENTA,
                LED_GREEN,
                CREAM,
            ];
            for row in 0..2u32 {
                for i in 0..8u32 {
                    if rnd(i, row + 7) > 0.85 {
                        continue; // sold out
                    }
                    let c = cols[(rnd(i, row) * cols.len() as f32) as usize % cols.len()];
                    rect(
                        g,
                        -20.0 + i as f32 * 4.2,
                        14.5 + row as f32 * 3.0,
                        2.5,
                        2.0,
                        c,
                    );
                }
            }
            rect(g, 14.0, 13.0, 8.0, 7.0, STEEL_DARK); // select panel
            let on = blink(time, 1.5, 0.0, 0.5);
            rect(g, 17.0, 15.0, 2.0, 2.0, if on { LED_GREEN } else { PANEL });
        }
        _ => {
            let fl = if rnd(1, (time * 6.0) as u32) > 0.9 {
                0.6
            } else {
                1.0
            };
            wash_v(
                g,
                0.0,
                20.0,
                44.0,
                18.0,
                Color::new(0.85, 0.95, 1.0, 1.0),
                0.16 * fl,
            );
        }
    }
}

/// 51 — the coffee corner: a small counter, the espresso machine with its
/// heating LEDs, cups lined up, a sugar jar, steam wisping off the
/// machine. Layers: counter, machine, cups, steam.
pub(super) fn coffee_corner(g: &Graphics, layer: usize, time: f32) {
    match layer {
        0 => {
            shadow_rect(g, -35.0, -14.0, 70.0, 28.0);
            rect(g, -35.0, -14.0, 70.0, 28.0, WALNUT);
            rect(g, -35.0, 11.0, 70.0, 3.0, alpha(CHROME, 0.6));
            rect(g, 24.0, 4.0, 8.0, 6.0, CREAM); // napkins
        }
        1 => {
            rect(g, -28.0, -10.0, 22.0, 20.0, STEEL_DARK);
            frame(g, -28.0, -10.0, 22.0, 20.0, 1.5, TRIM);
            rect(g, -26.0, -8.0, 18.0, 6.0, CHROME);
            circle(g, -17.0, 2.0, 3.0, STEEL);
            line(g, -17.0, 2.0, -8.0, 6.0, 2.0, STEEL_DARK); // portafilter handle
            rect(g, -24.0, 6.0, 14.0, 3.0, PANEL); // drip tray
            let heat = blink(time, 0.7, 0.0, 0.7);
            circle(g, -26.0, -6.0, 1.2, if heat { LED_RED } else { PANEL });
            circle(g, -23.0, -6.0, 1.2, GLOW_CYAN);
        }
        2 => {
            for &x in &[4.0f32, 11.0, 18.0, 25.0] {
                circle(g, x, -6.0, 3.0, CREAM);
                circle(g, x, -6.0, 1.8, Color::new(0.90, 0.85, 0.75, 1.0));
            }
            circle(g, 28.0, 4.0, 4.0, alpha(GLASS_HI, 0.7)); // sugar jar
            circle(g, 28.0, 4.0, 2.5, CREAM);
            line(g, 4.0, 4.0, 14.0, 6.0, 1.5, CHROME); // a spoon
        }
        _ => {
            for k in 0..3u32 {
                let ph = (time * 0.4 + k as f32 / 3.0).fract();
                let x = (ph * 6.0 + k as f32 * 2.0).sin() * 3.0;
                circle(
                    g,
                    x,
                    -ph * 14.0,
                    1.5 + ph * 1.5,
                    alpha(CREAM, 0.35 * (1.0 - ph)),
                );
            }
        }
    }
}

/// 52 — a bank of charging lockers: the cabinet with its door grid, the
/// front LED matrix (free / charging / done / dark per locker), a cable
/// left dangling on the floor. Layers: cabinet, leds, cable.
pub(super) fn charge_lockers(g: &Graphics, layer: usize, time: f32) {
    match layer {
        0 => {
            shadow_rect(g, -40.0, -16.0, 80.0, 32.0);
            rect(g, -40.0, -16.0, 80.0, 32.0, STEEL);
            frame(g, -40.0, -16.0, 80.0, 32.0, 2.0, TRIM);
            for c in 0..8 {
                for r in 0..2 {
                    frame(
                        g,
                        -38.0 + c as f32 * 9.5,
                        -14.0 + r as f32 * 12.0,
                        9.0,
                        11.0,
                        1.0,
                        alpha(PANEL, 0.5),
                    );
                }
            }
        }
        1 => {
            rect(g, -40.0, 10.0, 80.0, 6.0, STEEL_DARK);
            for c in 0..8u32 {
                for r in 0..2u32 {
                    let s = rnd(c, r + 1);
                    let col = if s < 0.35 {
                        LED_GREEN
                    } else if s < 0.7 {
                        alpha(
                            LED_AMBER,
                            0.4 + 0.6 * (0.5 + 0.5 * (time * 3.0 + c as f32).sin()),
                        )
                    } else if s < 0.85 {
                        GLOW_CYAN
                    } else {
                        PANEL
                    };
                    rect(
                        g,
                        -37.0 + c as f32 * 9.5,
                        11.0 + r as f32 * 2.8,
                        3.0,
                        2.0,
                        col,
                    );
                }
            }
        }
        _ => {
            line(g, 14.0, 16.0, 18.0, 26.0, 2.0, COPPER);
            line(g, 18.0, 26.0, 30.0, 32.0, 2.0, COPPER);
            rect(g, 29.0, 30.0, 6.0, 5.0, STEEL_DARK);
        }
    }
}

/// 53 — the tower's mark inlaid in the lobby floor: brass rings in the
/// marble, the obelisk's diamond at the centre (a static 45° layer) and
/// its four seams. A flat decal. Layers: inlay, mark, seams.
pub(super) fn floor_logo(g: &Graphics, layer: usize, _time: f32) {
    match layer {
        0 => {
            circle(g, 0.0, 0.0, 40.0, alpha(BRASS, 0.9));
            circle(g, 0.0, 0.0, 37.0, MARBLE);
            circle(g, 0.0, 0.0, 26.0, BRASS);
            circle(g, 0.0, 0.0, 24.0, MARBLE_DARK);
        }
        1 => {
            rect(
                g,
                -12.0,
                -12.0,
                24.0,
                24.0,
                Color::new(0.10, 0.08, 0.13, 1.0),
            );
            frame(g, -12.0, -12.0, 24.0, 24.0, 2.0, BRASS);
            rect(g, -5.0, -5.0, 10.0, 10.0, BRASS);
        }
        _ => {
            for k in 0..4 {
                let a = k as f32 * FRAC_PI_2;
                line(
                    g,
                    a.cos() * 17.0,
                    a.sin() * 17.0,
                    a.cos() * 24.0,
                    a.sin() * 24.0,
                    2.5,
                    BRASS,
                );
            }
        }
    }
}

/// 54 — the elevator lobby: the lift doors in their wall stub and the
/// call panel beside them, up / down arrows lighting in turn and pooling
/// amber on the floor. Layers: wall, panel, arrows.
pub(super) fn call_panel(g: &Graphics, layer: usize, time: f32) {
    match layer {
        0 => {
            rect(g, -30.0, -46.0, 60.0, 12.0, STEEL_DARK);
            frame(g, -30.0, -46.0, 60.0, 12.0, 1.5, TRIM);
            rect(g, -24.0, -44.0, 32.0, 8.0, CHROME); // the doors
            line(g, -8.0, -44.0, -8.0, -36.0, 1.5, PANEL);
            rect(g, -24.0, -34.0, 32.0, 2.0, alpha(CHROME, 0.5)); // threshold
        }
        1 => {
            rect(g, 12.0, -34.0, 14.0, 10.0, PANEL);
            frame(g, 12.0, -34.0, 14.0, 10.0, 1.5, TRIM);
        }
        _ => {
            let c = (time * 0.4).fract();
            let up = c < 0.42;
            let down = (0.5..0.92).contains(&c);
            let (lit, dim) = (LED_AMBER, alpha(LED_AMBER, 0.25));
            chevron(g, 16.0, -29.0, 2.5, 1.5, if up { lit } else { dim });
            line(
                g,
                20.5,
                -31.0,
                23.0,
                -27.5,
                1.5,
                if down { lit } else { dim },
            );
            line(
                g,
                23.0,
                -27.5,
                25.5,
                -31.0,
                1.5,
                if down { lit } else { dim },
            );
            if up || down {
                circle(g, 19.0, -22.0, 6.0, alpha(LED_AMBER, 0.12));
            }
        }
    }
}

/// 55 — a velvet rope between two brass posts, the rope's bow breathing
/// as the queue brushes it. Layers: shadow, rope (swaying), posts.
pub(super) fn velvet_rope(g: &Graphics, layer: usize, time: f32) {
    match layer {
        0 => {
            for i in 0..10 {
                let x0 = -36.0 + i as f32 * 7.2;
                let x1 = x0 + 7.2;
                let f = |x: f32| 5.0 * (1.0 - (x / 36.0) * (x / 36.0));
                line(
                    g,
                    x0 + 3.0,
                    f(x0) + 4.0,
                    x1 + 3.0,
                    f(x1) + 4.0,
                    3.0,
                    alpha(SHADOW, 0.2),
                );
            }
        }
        1 => {
            let b = 5.0 + 1.5 * (time * 0.7).sin();
            for i in 0..10 {
                let x0 = -36.0 + i as f32 * 7.2;
                let x1 = x0 + 7.2;
                let f = |x: f32| b * (1.0 - (x / 36.0) * (x / 36.0));
                line(g, x0, f(x0), x1, f(x1), 3.5, VELVET);
            }
            circle(g, -36.0, 0.0, 2.0, BRASS);
            circle(g, 36.0, 0.0, 2.0, BRASS);
        }
        _ => {
            for &x in &[-36.0f32, 36.0] {
                post(g, x, 0.0, 5.5, Color::new(0.55, 0.42, 0.18, 1.0), BRASS);
            }
        }
    }
}

/// 56 — a wall-mounted fire extinguisher: the wall stub and bracket, the
/// red cylinder from above with its valve and hose, the fire-point mark
/// on the floor. Layers: mount, tank, sign.
pub(super) fn extinguisher(g: &Graphics, layer: usize, _time: f32) {
    let red = Color::new(0.72, 0.14, 0.18, 1.0);
    match layer {
        0 => {
            rect(g, -16.0, -46.0, 32.0, 10.0, STEEL_DARK);
            frame(g, -16.0, -46.0, 32.0, 10.0, 1.5, TRIM);
            rect(g, -3.0, -36.0, 6.0, 4.0, STEEL);
        }
        1 => {
            shadow_circle(g, 0.0, -28.0, 9.0);
            circle(g, 0.0, -28.0, 9.0, red);
            circle(g, 0.0, -28.0, 6.5, Color::new(0.85, 0.25, 0.28, 1.0));
            circle(g, 0.0, -28.0, 3.5, CHROME); // the valve
            circle(g, 0.0, -28.0, 1.5, STEEL_DARK);
            line(
                g,
                3.0,
                -30.0,
                9.0,
                -24.0,
                2.0,
                Color::new(0.08, 0.08, 0.09, 1.0),
            );
            line(
                g,
                9.0,
                -24.0,
                8.0,
                -16.0,
                2.0,
                Color::new(0.08, 0.08, 0.09, 1.0),
            );
            rect(g, 6.0, -16.0, 4.0, 4.0, STEEL_DARK); // nozzle
        }
        _ => {
            rect(g, -8.0, -6.0, 16.0, 16.0, alpha(red, 0.8));
            circle(g, 0.0, 3.0, 4.0, CREAM);
            rect(g, -1.5, -4.0, 3.0, 6.0, CREAM);
        }
    }
}

/// 57 — a credit terminal kiosk: the pedestal with card slot and keypad,
/// its top-face screen (header, a progress bar filling, a blinking
/// cursor) and the screen's glow spilling toward the user. Layers: body,
/// screen, wash.
pub(super) fn credit_kiosk(g: &Graphics, layer: usize, time: f32) {
    match layer {
        0 => {
            rect(g, -10.0, -15.0, 30.0, 40.0, SHADOW);
            rect(g, -15.0, -20.0, 30.0, 40.0, STEEL);
            frame(g, -15.0, -20.0, 30.0, 40.0, 2.0, TRIM);
            rect(g, -8.0, 2.0, 16.0, 2.0, PANEL); // card slot
            let on = blink(time, 1.0, 0.0, 0.5);
            circle(g, 10.0, 3.0, 1.5, if on { LED_GREEN } else { PANEL });
            for c in 0..3 {
                for r in 0..4 {
                    rect(
                        g,
                        -7.0 + c as f32 * 5.0,
                        6.0 + r as f32 * 3.0,
                        3.0,
                        2.0,
                        TRIM,
                    );
                }
            }
            rect(g, -15.0, 18.0, 30.0, 2.0, alpha(GLOW_CYAN, 0.6));
        }
        1 => {
            rect(g, -11.0, -16.0, 22.0, 14.0, PANEL);
            rect(g, -9.0, -14.0, 18.0, 2.0, alpha(GLOW_CYAN, 0.7));
            rect(g, -9.0, -9.0, 18.0, 3.0, Color::new(0.10, 0.20, 0.25, 1.0));
            rect(g, -9.0, -9.0, 18.0 * (time * 0.25).fract(), 3.0, LED_GREEN);
            rect(g, -9.0, -4.0, 12.0, 1.5, alpha(CREAM, 0.5));
            rect(g, -9.0, -1.5, 8.0, 1.5, alpha(CREAM, 0.5));
            if blink(time, 2.0, 0.0, 0.5) {
                rect(g, 4.0, -3.0, 2.0, 2.0, CREAM);
            }
        }
        _ => wash_v(g, 0.0, 20.0, 30.0, 14.0, GLOW_CYAN, 0.14),
    }
}

/// 58 — a holo clock projected on the floor: the ring and ticks, the
/// hour hand (static), the minute and second hands (spinning layers).
/// Layers: face, hour, minute, second.
pub(super) fn wall_clock(g: &Graphics, layer: usize, _time: f32) {
    match layer {
        0 => {
            ring(g, 0.0, 0.0, 34.0, 2.5, alpha(GLOW_CYAN, 0.7));
            circle(g, 0.0, 0.0, 31.0, Color::new(0.05, 0.15, 0.20, 0.35));
            for k in 0..12 {
                let a = k as f32 * (TAU / 12.0);
                let len = if k % 3 == 0 { 5.0 } else { 2.5 };
                line(
                    g,
                    a.cos() * 29.0,
                    a.sin() * 29.0,
                    a.cos() * (29.0 - len),
                    a.sin() * (29.0 - len),
                    1.5,
                    alpha(GLOW_CYAN, 0.8),
                );
            }
            circle(g, 0.0, 0.0, 2.0, CREAM);
        }
        1 => rect(g, -2.5, -18.0, 5.0, 20.0, alpha(CREAM, 0.9)),
        2 => rect(g, -1.5, -26.0, 3.0, 30.0, alpha(CREAM, 0.85)),
        _ => {
            rect(g, -0.75, -29.0, 1.5, 34.0, alpha(NEON_PINK, 0.9));
            circle(g, 0.0, 3.0, 1.5, NEON_PINK);
        }
    }
}

/// 59 — the welcome mat inside the doors: coir texture, a border,
/// chevrons pointing in, some wear. A flat decal. Layers: mat, pattern.
pub(super) fn welcome_mat(g: &Graphics, layer: usize, _time: f32) {
    match layer {
        0 => {
            rect(
                g,
                -35.0,
                -20.0,
                70.0,
                40.0,
                Color::new(0.16, 0.14, 0.14, 1.0),
            );
            for i in 0..9 {
                rect(
                    g,
                    -33.0,
                    -17.0 + i as f32 * 4.0,
                    66.0,
                    1.0,
                    alpha(PANEL, 0.25),
                );
            }
            frame(
                g,
                -35.0,
                -20.0,
                70.0,
                40.0,
                2.5,
                Color::new(0.34, 0.30, 0.28, 1.0),
            );
        }
        _ => {
            let accent = Color::new(1.0, 0.44, 0.38, 0.8);
            for &x in &[-18.0f32, 0.0, 18.0] {
                chevron(g, x, 0.0, 7.0, 2.5, accent);
            }
            for k in 0..5u32 {
                rect(
                    g,
                    -30.0 + rnd(k, 3) * 58.0,
                    -15.0 + rnd(k, 4) * 28.0,
                    3.0,
                    2.0,
                    alpha(CREAM, 0.12),
                );
            }
        }
    }
}
