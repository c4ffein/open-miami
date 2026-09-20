//! The `?viz` toolbox shell: the tab bar, the shared buttons, and the
//! SPRITES tab (CHARACTERS page; PROPS is `props_page`).

use super::*;

mod effects;
mod musics;
mod props_page;

pub(crate) use effects::*;
pub(crate) use props_page::*;

/// Tabs of the `?viz` tool.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum VizTab {
    Sprites,
    Musics,
    Levels,
    Effects,
}

/// Draw a clickable button; returns true if the mouse is currently over it
/// (the caller decides what a click does). `active` highlights it.
#[allow(clippy::too_many_arguments)]
pub(crate) fn viz_button(
    g: &Graphics,
    mouse: Vec2,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    label: &str,
    active: bool,
) -> bool {
    viz_button_styled(g, mouse, x, y, w, h, label, active, false)
}

/// A compact `viz_button` (13 px label, roughly centred) for dense rows.
#[allow(clippy::too_many_arguments)]
pub(crate) fn viz_small_button(
    g: &Graphics,
    mouse: Vec2,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    label: &str,
    active: bool,
) -> bool {
    viz_button_styled(g, mouse, x, y, w, h, label, active, true)
}

/// The one button drawer behind `viz_button` / `viz_small_button`:
/// `small` = thin 1 px border and a 13 px label roughly centred (vs the
/// regular 1.5 px border and 18 px left-aligned label).
#[allow(clippy::too_many_arguments)]
pub(crate) fn viz_button_styled(
    g: &Graphics,
    mouse: Vec2,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    label: &str,
    active: bool,
    small: bool,
) -> bool {
    let over = mouse.x >= x && mouse.x <= x + w && mouse.y >= y && mouse.y <= y + h;
    let bg = if active {
        Color::new(1.0, 0.09, 0.26, 0.85)
    } else if over {
        Color::new(0.28, 0.22, 0.33, 1.0)
    } else {
        Color::new(0.14, 0.10, 0.18, 1.0)
    };
    g.draw_rectangle(Vec2::new(x, y), w, h, bg);
    let border = if small { 1.0 } else { 1.5 };
    g.draw_rectangle_lines(
        Vec2::new(x, y),
        w,
        h,
        border,
        Color::new(0.45, 0.35, 0.5, 1.0),
    );
    let (tx, ty, fs) = if small {
        let tw = label.chars().count() as f32 * 6.0;
        (x + (w - tw).max(0.0) / 2.0, y + h / 2.0 + 5.0, 13.0)
    } else {
        (x + 14.0, y + h / 2.0 + 6.0, 18.0)
    };
    g.draw_text(label, Vec2::new(tx, ty), fs, Color::WHITE);
    over
}

impl GameState {
    /// Asset visualizer (`?viz`): a small tabbed inspector — sprites, sounds,
    /// and level maps — for looking at the game's pieces in isolation.
    pub(crate) fn update_visualizer(&mut self, graphics: &Graphics) {
        let mouse = input::mouse_position();
        let click = input::is_mouse_button_pressed(input::mouse_buttons::LEFT);

        // The LEVELS tab is the native level editor; it draws first (its
        // map may overflow anywhere) and the tab bar goes on top.
        if self.viz_tab == VizTab::Levels {
            self.editor.update(graphics, mouse, click, self.last_time);
        }

        // Top tab bar.
        let tabs = [
            (VizTab::Sprites, "SPRITES"),
            (VizTab::Musics, "MUSICS"),
            (VizTab::Levels, "LEVELS"),
            (VizTab::Effects, "EFFECTS"),
        ];
        for (i, &(tab, name)) in tabs.iter().enumerate() {
            let x = 20.0 + i as f32 * 168.0;
            let over = viz_button(
                graphics,
                mouse,
                x,
                14.0,
                158.0,
                46.0,
                name,
                self.viz_tab == tab,
            );
            if over && click && self.viz_tab != tab {
                self.viz_tab = tab;
                // A click is a user gesture -> unlock audio.
                self.audio.resume();
                // Switching tabs closes the iframe panel (the sprites
                // gallery re-opens it when an item is clicked; the LEVELS
                // editor's SCENARIO (web) button opens the web editor).
                viz_inspect_hide();
                self.viz_selected = -1;
                self.editor.hide_web();
            }
        }

        match self.viz_tab {
            VizTab::Sprites => self.draw_viz_sprites(graphics, mouse, click),
            VizTab::Musics => self.draw_viz_musics(graphics, mouse, click),
            VizTab::Levels => {} // drawn above, under the tab bar
            VizTab::Effects => self.draw_viz_effects(graphics, mouse, click),
        }

        // A previewing effect draws full-screen, on top of everything: the
        // 2D shoggoth glitch as commands, a POSTFX kind as a real post pass
        // over this whole viz frame.
        let elapsed = self.last_time - self.effect_start;
        if self.effect_start > 0.0 {
            if self.effect_kind < 0 {
                if (0.0..1200.0).contains(&elapsed) {
                    draw_shoggoth_glitch(graphics, elapsed as f32);
                }
            } else if (0.0..POSTFX_PREVIEW_MS).contains(&elapsed) {
                let (kind, _, peak, color) = POSTFX_PREVIEWS[self.effect_kind as usize];
                let p = (elapsed / POSTFX_PREVIEW_MS) as f32;
                // Envelope: ramp in over 15%, hold, ramp out the last 20%.
                let env = (p / 0.15).min((1.0 - p) / 0.2).clamp(0.0, 1.0);
                graphics.postfx(kind, peak * env, color);
            }
        }
    }
}

impl GameState {
    /// SPRITES tab: two sub-pages — the character gallery (each item opens
    /// the 3D inspector iframe) and the datacenter prop library (an
    /// all-wasm gallery of animated primitive-drawn set dressing).
    pub(crate) fn draw_viz_sprites(&mut self, graphics: &Graphics, mouse: Vec2, click: bool) {
        let pages = [(false, "CHARACTERS"), (true, "PROPS")];
        for (i, &(page, name)) in pages.iter().enumerate() {
            let x = 40.0 + i as f32 * 168.0;
            let over = viz_button(
                graphics,
                mouse,
                x,
                76.0,
                158.0,
                38.0,
                name,
                self.viz_props_page == page,
            );
            if over && click && self.viz_props_page != page {
                self.viz_props_page = page;
                // The prop gallery draws its own big preview pane; the
                // iframe inspector belongs to the character page only.
                viz_inspect_hide();
                self.viz_selected = -1;
            }
        }
        if self.viz_props_page {
            self.draw_viz_props(graphics, mouse, click);
        } else {
            self.draw_viz_characters(graphics, mouse, click);
        }
    }
}

impl GameState {
    /// The CHARACTERS page of the SPRITES tab: a clickable gallery; an item
    /// opens the right-hand inspector iframe (3D orbit + baked 2D) via
    /// `viz_inspect`.
    pub(crate) fn draw_viz_characters(&mut self, graphics: &Graphics, mouse: Vec2, click: bool) {
        graphics.draw_text(
            "Click a character to inspect it in 3D  \u{2192}",
            Vec2::new(40.0, 138.0),
            18.0,
            Color::GRAY,
        );

        let coral = Color::from_rgba(217, 119, 87, 255);
        let red = Color::from_rgba(224, 49, 66, 255);
        let violet = Color::from_rgba(150, 70, 210, 255);
        let magenta = Color::from_rgba(224, 40, 160, 255);

        // (inspector kind, label): the four robots and the boss's two phases.
        // Thumbnails are the small 2D-primitive icons; the iframe shows the
        // live 3D character (tools/inspector.html).
        let items: [(&str, &str); 6] = [
            ("coral", "CL4-UD3"),
            ("red", "SENTINEL"),
            ("violet", "DRIFTER"),
            ("magenta", "HUNTER"),
            ("shoggoth_masked", "SHOGGOTH mask"),
            ("shoggoth_enraged", "SHOGGOTH raw"),
        ];

        // Two columns on the LEFT half; the right half is the inspector iframe.
        let (x0, y0, dx, dy) = (120.0f32, 200.0f32, 190.0f32, 140.0f32);
        for (i, &(kind, label)) in items.iter().enumerate() {
            let c = Vec2::new(x0 + (i % 2) as f32 * dx, y0 + (i / 2) as f32 * dy);
            let (bx, by, bw, bh) = (c.x - 85.0, c.y - 58.0, 170.0, 116.0);
            let over = mouse.x >= bx && mouse.x <= bx + bw && mouse.y >= by && mouse.y <= by + bh;
            let selected = self.viz_selected == i as i32;
            let bg = if selected {
                Color::new(1.0, 0.09, 0.26, 0.30)
            } else if over {
                Color::new(0.28, 0.22, 0.33, 1.0)
            } else {
                Color::new(0.13, 0.09, 0.17, 1.0)
            };
            let border = if selected {
                Color::new(1.0, 0.09, 0.26, 1.0)
            } else {
                Color::new(0.4, 0.3, 0.45, 1.0)
            };
            graphics.draw_rectangle(Vec2::new(bx, by), bw, bh, bg);
            graphics.draw_rectangle_lines(Vec2::new(bx, by), bw, bh, 1.5, border);

            match kind {
                "shoggoth_masked" => graphics.draw_shoggoth(c, 30.0, false),
                "shoggoth_enraged" => graphics.draw_shoggoth(c, 30.0, true),
                _ => {
                    let color = match kind {
                        "coral" => coral,
                        "red" => red,
                        "violet" => violet,
                        _ => magenta,
                    };
                    graphics.draw_pixelated_sprite(c, 0.0, color, false);
                }
            }
            graphics.draw_text(label, Vec2::new(c.x - 60.0, c.y + 50.0), 15.0, Color::WHITE);

            if over && click {
                self.viz_selected = i as i32;
                viz_inspect(kind);
            }
        }

        if self.viz_selected < 0 {
            graphics.draw_text(
                "pick one \u{2192}",
                Vec2::new(600.0, 360.0),
                20.0,
                Color::GRAY,
            );
        }
    }
}
