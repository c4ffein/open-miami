//! Title / level select, the modal chrome, SETTINGS, ABOUT and PAUSE.

use super::*;

/// Height of the SETTINGS panel: the chrome + three 46-px rows + the hint.
const SETTINGS_MODAL_H: f32 = 358.0;

impl GameState {
    pub(crate) fn update_level_select(&mut self, graphics: &Graphics) {
        // Handle input - Left (Arrow, A for QWERTY, Q for AZERTY)
        if input::is_key_pressed("ArrowLeft")
            || input::is_key_pressed("a")
            || input::is_key_pressed("q")
        {
            self.selected_level = if self.selected_level == 0 {
                LEVEL_COUNT - 1
            } else {
                self.selected_level - 1
            };
        }
        // Handle input - Right (Arrow, D)
        if input::is_key_pressed("ArrowRight") || input::is_key_pressed("d") {
            self.selected_level = (self.selected_level + 1) % LEVEL_COUNT;
        }
        // Handle input - Down (Arrow, S)
        if input::is_key_pressed("ArrowDown") || input::is_key_pressed("s") {
            self.selected_menu_option = match self.selected_menu_option {
                MenuOption::Play => MenuOption::Settings,
                MenuOption::Settings => MenuOption::About,
                MenuOption::About => MenuOption::Play,
            };
        }
        // Handle input - Up (Arrow, W for QWERTY, Z for AZERTY)
        if input::is_key_pressed("ArrowUp")
            || input::is_key_pressed("w")
            || input::is_key_pressed("z")
        {
            self.selected_menu_option = match self.selected_menu_option {
                MenuOption::Play => MenuOption::About,
                MenuOption::Settings => MenuOption::Play,
                MenuOption::About => MenuOption::Settings,
            };
        }
        // A screen switch takes effect AFTER this frame is drawn: switching
        // and returning early would ship a frame of neither screen (a bare
        // CLEAR — a one-frame flash), and re-dispatching to the new screen
        // would hand it the same Enter press. `menu-transitions.spec.js`.
        let chosen = input::is_key_pressed("Enter").then_some(self.selected_menu_option);

        self.draw_level_select(graphics);

        match chosen {
            Some(MenuOption::Play) => self.start_game(),
            Some(MenuOption::Settings) => self.screen = GameScreen::Settings,
            Some(MenuOption::About) => self.screen = GameScreen::About,
            None => {}
        }
    }

    /// The title screen's drawing (no input): the drive backdrop, neon
    /// title, floor picker and menu. Also the live backdrop under the
    /// SETTINGS / ABOUT modals.
    pub(crate) fn draw_level_select(&mut self, graphics: &Graphics) {
        let screen_width = graphics.width();
        let screen_height = graphics.height();

        // The glitchy synthwave DRIVE (drive.rs) runs behind the whole
        // menu, dimmed (inside the drive shader — no full-screen blend)
        // so the text stays the star.
        crate::drive::render_drive(
            graphics,
            screen_width,
            screen_height,
            (self.last_time / 1000.0) as f32,
            0.5,
            0.55,
        );

        // The neon pixel title: OPEN / MIAMI, hollow pink letters
        // swaying slowly. (The ROGUE PURGE subtitle retired.)
        draw_neon_title(
            graphics,
            screen_width / 2.0,
            176.0,
            (self.last_time / 1000.0) as f32,
        );

        // Render level selection (a touch bigger and lower than the
        // title wants to sit).
        let level_y = screen_height / 2.0 - 22.0;

        // Left arrow
        let arrow_color = if self.selected_menu_option == MenuOption::Play {
            Color::WHITE
        } else {
            Color::GRAY
        };
        graphics.draw_text(
            "<",
            Vec2::new(screen_width / 2.0 - 172.0, level_y),
            48.0,
            arrow_color,
        );

        // Level number + the floor's name in its accent colour
        let level_text = floor_title(self.selected_level);
        graphics.draw_text(
            &level_text,
            Vec2::new(screen_width / 2.0 - 96.0, level_y),
            48.0,
            Color::WHITE,
        );
        let floor = floor_def(self.selected_level);
        let (ar, ag, ab) = floor.accent_rgb();
        let name_w = floor.name.chars().count() as f32 * 22.0 * 0.42;
        graphics.draw_text(
            floor.name,
            Vec2::new(screen_width / 2.0 - name_w / 2.0, level_y + 34.0),
            22.0,
            Color::from_rgba(ar, ag, ab, 255),
        );

        // Right arrow
        graphics.draw_text(
            ">",
            Vec2::new(screen_width / 2.0 + 142.0, level_y),
            48.0,
            arrow_color,
        );

        // Render menu options
        let menu_y = screen_height / 2.0 + 100.0;
        let menu_spacing = 50.0;

        let play_color = if self.selected_menu_option == MenuOption::Play {
            Color::new(1.0, 0.20, 0.60, 1.0) // the neon title's pink
        } else {
            Color::WHITE
        };
        graphics.draw_text(
            "PRESS ENTER TO PLAY",
            Vec2::new(screen_width / 2.0 - 150.0, menu_y),
            30.0,
            play_color,
        );

        let settings_color = if self.selected_menu_option == MenuOption::Settings {
            Color::new(1.0, 0.20, 0.60, 1.0)
        } else {
            Color::WHITE
        };
        graphics.draw_text(
            "SETTINGS",
            Vec2::new(screen_width / 2.0 - 46.0, menu_y + menu_spacing),
            24.0,
            settings_color,
        );

        let about_color = if self.selected_menu_option == MenuOption::About {
            Color::new(1.0, 0.20, 0.60, 1.0)
        } else {
            Color::WHITE
        };
        graphics.draw_text(
            "ABOUT",
            Vec2::new(screen_width / 2.0 - 30.0, menu_y + menu_spacing * 2.0),
            24.0,
            about_color,
        );

        // Controls hint
        graphics.draw_text(
            "Arrow Keys or WASD/ZQSD to navigate | Enter to select",
            Vec2::new(screen_width / 2.0 - 280.0, screen_height - 40.0),
            16.0,
            Color::GRAY,
        );

        // A faint TV-static shimmer over the whole title screen.
        // (Last POSTFX wins, so an open modal's kind 12 replaces it.)
        if self.noise_enabled {
            graphics.postfx(13, TV_STATIC_T, Color::WHITE);
        }
    }

    /// The shared SETTINGS / ABOUT modal chrome over the live title
    /// screen: a monochrome full-white-on-black panel, then POSTFX 12
    /// (MODAL STATIC) — the panel keeps the grey/tape wash while
    /// everything OUTSIDE it is blurred and buried under ~90% hard 6-px
    /// binary white noise, re-rolled every frame. Returns the panel
    /// origin for the caller's content.
    pub(crate) fn draw_menu_modal(
        &mut self,
        graphics: &Graphics,
        title: &str,
        mw: f32,
        mh: f32,
    ) -> Vec2 {
        self.draw_level_select(graphics);
        self.draw_modal_chrome(graphics, title, mw, mh, "ESC / ENTER — BACK")
    }

    /// Just the modal panel + POSTFX 12, over whatever is already drawn
    /// (the title modals over the level select, the pause menu over the
    /// frozen world; stacked modals each emit their POSTFX and only the
    /// LAST one applies, so the topmost panel wins and everything under
    /// it — including a deeper panel — melts into the static).
    pub(crate) fn draw_modal_chrome(
        &mut self,
        graphics: &Graphics,
        title: &str,
        mw: f32,
        mh: f32,
        hint: &str,
    ) -> Vec2 {
        let (w, h) = (graphics.width(), graphics.height());
        let (mx, my) = ((w - mw) / 2.0, (h - mh) / 2.0);
        // Pure, opaque black: the shader passes the inside through
        // untouched, and the frame (white ring, black ring) is drawn by
        // the POSTFX itself right at the panel edge.
        graphics.draw_rectangle(Vec2::new(mx, my), mw, mh, Color::new(0.0, 0.0, 0.0, 1.0));
        graphics.draw_text(title, Vec2::new(mx + 28.0, my + 40.0), 36.0, Color::WHITE);
        // The divider hugs the title (~24 px under its baseline) and
        // leaves the larger share of air (~28 px) to the body below.
        graphics.draw_rectangle(
            Vec2::new(mx + 28.0, my + 64.0),
            mw - 56.0,
            6.0,
            Color::WHITE,
        );
        graphics.draw_text(
            hint,
            Vec2::new(
                mx + mw - 28.0 - hint.chars().count() as f32 * 8.0,
                my + mh - 30.0,
            ),
            15.0,
            Color::WHITE,
        );
        // MODAL STATIC: r/g carry the panel's half extents; the shader
        // frames the exact edge with a 6-px white then 6-px black ring
        // before the noise starts.
        graphics.postfx(12, 0.9, Color::new(mw / 2.0 / w, mh / 2.0 / h, 0.0, 1.0));
        Vec2::new(mx, my)
    }

    /// The SETTINGS modal body — three rows (SOUND, MUSIC, FPS CAP),
    /// Up/Down to highlight, Enter/Space or a click on a row to act. Shared
    /// by the main menu and the pause menu's stacked settings (both size
    /// their panel with [`SETTINGS_MODAL_H`]).
    pub(crate) fn settings_modal_body(&mut self, graphics: &Graphics, p: Vec2, mw: f32) {
        const ROW_H: f32 = 46.0;
        const ROWS: usize = 3;
        let rows_y = p.y + 118.0;
        if input::is_key_pressed("ArrowDown") || input::is_key_pressed("s") {
            self.settings_row = (self.settings_row + 1) % ROWS;
        } else if input::is_key_pressed("ArrowUp")
            || input::is_key_pressed("w")
            || input::is_key_pressed("z")
        {
            self.settings_row = (self.settings_row + ROWS - 1) % ROWS;
        }
        let mut act: Option<usize> = None;
        if input::is_key_pressed("Enter") || input::is_key_pressed(" ") {
            act = Some(self.settings_row);
        } else if input::is_mouse_button_pressed(input::mouse_buttons::LEFT) {
            let m = input::mouse_position();
            if m.x >= p.x && m.x <= p.x + mw {
                for i in 0..ROWS {
                    let ry = rows_y + i as f32 * ROW_H;
                    if m.y >= ry - 8.0 && m.y <= ry + 32.0 {
                        self.settings_row = i;
                        act = Some(i);
                    }
                }
            }
        }
        match act {
            Some(0) => {
                let now = !self.audio.is_enabled();
                self.audio.set_enabled(now);
                set_setting("sound", if now { "on" } else { "off" });
            }
            Some(1) => {
                // 100 -> 75 -> 50 -> 25 -> 0 -> 100 ... (percent; the SFX
                // keep their level).
                let pct = (self.audio.music_level() * 100.0).round() as u32;
                let next = crate::audio::next_music_percent(pct);
                self.audio.set_music_level(f64::from(next) / 100.0);
                set_setting("music", &next.to_string());
            }
            Some(2) => {
                // 30 -> 60 -> 120 -> UNCAPPED -> 30 ...
                self.fps_cap = match self.fps_cap {
                    30 => 60,
                    60 => 120,
                    120 => 0,
                    _ => 30,
                };
                set_setting("fps_cap", &self.fps_cap.to_string());
            }
            _ => {}
        }

        let sound_label = if self.audio.is_enabled() {
            "[X]"
        } else {
            "[ ]"
        };
        let cap_label = if self.fps_cap == 0 {
            "UNCAPPED".to_string()
        } else {
            format!("{}", self.fps_cap)
        };
        let music_label = format!("{:.0}%", self.audio.music_level() * 100.0);
        let rows: [(&str, String); ROWS] = [
            ("SOUND", sound_label.to_string()),
            ("MUSIC", music_label),
            ("FPS CAP", cap_label),
        ];
        for (i, (name, value)) in rows.iter().enumerate() {
            let ry = rows_y + i as f32 * ROW_H;
            let color = if self.settings_row == i {
                Color::new(1.0, 0.20, 0.60, 1.0)
            } else {
                Color::WHITE
            };
            graphics.draw_text(name, Vec2::new(p.x + 28.0, ry), 24.0, color);
            graphics.draw_text(
                value,
                Vec2::new(p.x + mw - 28.0 - value.chars().count() as f32 * 11.0, ry),
                24.0,
                color,
            );
        }
        graphics.draw_text(
            "ENTER / SPACE / CLICK — CHANGE",
            Vec2::new(p.x + 28.0, rows_y + ROWS as f32 * ROW_H + 10.0),
            15.0,
            Color::new(1.0, 1.0, 1.0, 0.6),
        );
    }

    pub(crate) fn update_settings(&mut self, graphics: &Graphics) {
        let back = input::is_key_pressed("Escape");
        self.draw_level_select(graphics);
        let p = self.draw_modal_chrome(graphics, "SETTINGS", 564.0, SETTINGS_MODAL_H, "ESC — BACK");
        self.settings_modal_body(graphics, p, 564.0);
        // Switch AFTER drawing (see `update_level_select`).
        if back {
            self.screen = GameScreen::LevelSelect;
        }
    }

    pub(crate) fn update_about(&mut self, graphics: &Graphics) {
        let back = input::is_key_pressed("Escape") || input::is_key_pressed("Enter");
        let p = self.draw_menu_modal(graphics, "ABOUT", 660.0, 498.0);
        const LINES: [&str; 11] = [
            "THIS STARTED AS A VIBE CODED EXPERIMENT",
            "WITH SONNET 4.5 LAST YEAR",
            "",
            "I ASKED FABLE FOR AN OPINION ON THE PROJECT",
            "I GUESS THIS IS OUR PROJECT NOW",
            "",
            "OBVIOUSLY THIS IS AN HOMAGE TO HOTLINE MIAMI",
            "(BUY THIS AND THE SECOND ONE)",
            "",
            "YOU CAN CHECK THE SOURCES AT",
            "HTTPS://GITHUB.COM/C4FFEIN/OPEN-MIAMI",
        ];
        const URL_LINE: usize = 10;
        let mut url_rect = (0.0, 0.0, 0.0, 0.0);
        for (i, line) in LINES.iter().enumerate() {
            let pos = Vec2::new(p.x + 28.0, p.y + 112.0 + i as f32 * 26.0);
            if i == URL_LINE {
                // The link: neon pink, underlined, click -> new tab.
                let w = line.chars().count() as f32 * 20.0 * 0.44;
                graphics.draw_text(line, pos, 20.0, Color::new(1.0, 0.20, 0.60, 1.0));
                graphics.draw_rectangle(
                    Vec2::new(pos.x, pos.y + 24.0),
                    w,
                    6.0,
                    Color::new(1.0, 0.20, 0.60, 0.9),
                );
                url_rect = (pos.x, pos.y - 2.0, w, 32.0);
            } else {
                graphics.draw_text(line, pos, 20.0, Color::WHITE);
            }
        }
        // Click on the URL opens the repo in a new tab (still within the
        // click's transient user activation, so popup blockers allow it).
        if input::is_mouse_button_pressed(input::mouse_buttons::LEFT) {
            let m = input::mouse_position();
            let (rx, ry, rw, rh) = url_rect;
            if m.x >= rx && m.x <= rx + rw && m.y >= ry && m.y <= ry + rh {
                open_external("https://github.com/c4ffein/open-miami");
            }
        }
        graphics.draw_text(
            "LUV - C4FFEIN",
            Vec2::new(p.x + 660.0 - 170.0, p.y + 112.0 + 11.0 * 26.0 + 6.0),
            22.0,
            Color::WHITE,
        );
        // Switch AFTER drawing (see `update_level_select`).
        if back {
            self.screen = GameScreen::LevelSelect;
        }
    }

    pub(crate) fn update_paused(&mut self, graphics: &Graphics) {
        // The frozen game world behind the modal — same recipe as the
        // title's SETTINGS/ABOUT over the live level select: draw the
        // scene, then let POSTFX 12 blur it and bury it under the static
        // outside the panel. `dt = 0`: pure re-render, nothing advances.
        let accent = self
            .scenario
            .as_ref()
            .map(|sc| sc.floor().accent_rgb())
            .unwrap_or((217, 119, 87));
        self.render_world(graphics, 0.0, accent);

        // The stacked SETTINGS modal over the pause modal: Esc pops one
        // layer at a time (settings -> pause -> game). Both panels draw;
        // only the topmost POSTFX applies, so the pause panel behind
        // melts into the static.
        //
        // EVERY switch below takes effect AFTER the frame is drawn: the world
        // is already recorded at this point, so switching and returning early
        // shipped a frame of the RAW WORLD — no modal, no static — for one
        // frame (`menu-transitions.spec.js`).
        if self.pause_in_settings {
            let close = input::is_key_pressed("Escape");
            let pp = self.draw_modal_chrome(graphics, "PAUSED", 420.0, 340.0, "");
            self.draw_pause_rows(graphics, pp, false);
            let p =
                self.draw_modal_chrome(graphics, "SETTINGS", 564.0, SETTINGS_MODAL_H, "ESC — BACK");
            self.settings_modal_body(graphics, p, 564.0);
            if close {
                self.pause_in_settings = false;
            }
            return;
        }

        // Esc = CONTINUE, whatever row is selected.
        let mut chosen = input::is_key_pressed("Escape").then_some(PauseOption::Continue);
        if input::is_key_pressed("ArrowDown") || input::is_key_pressed("s") {
            self.selected_pause_option = match self.selected_pause_option {
                PauseOption::Continue => PauseOption::Settings,
                PauseOption::Settings => PauseOption::Stop,
                PauseOption::Stop => PauseOption::Continue,
            };
        }
        if input::is_key_pressed("ArrowUp")
            || input::is_key_pressed("w")
            || input::is_key_pressed("z")
        {
            self.selected_pause_option = match self.selected_pause_option {
                PauseOption::Continue => PauseOption::Stop,
                PauseOption::Settings => PauseOption::Continue,
                PauseOption::Stop => PauseOption::Settings,
            };
        }
        if chosen.is_none() && input::is_key_pressed("Enter") {
            chosen = Some(self.selected_pause_option);
        }

        let p = self.draw_modal_chrome(graphics, "PAUSED", 420.0, 340.0, "ESC — CONTINUE");
        self.draw_pause_rows(graphics, p, true);

        match chosen {
            Some(PauseOption::Continue) => self.screen = GameScreen::InGame,
            Some(PauseOption::Settings) => self.pause_in_settings = true,
            Some(PauseOption::Stop) => self.screen = GameScreen::LevelSelect,
            None => {}
        }
    }

    /// The pause modal's three rows. `active` = the pause layer has
    /// focus (rows dim to grey while the stacked SETTINGS modal is up).
    pub(crate) fn draw_pause_rows(&self, graphics: &Graphics, p: Vec2, active: bool) {
        const ROWS: [(PauseOption, &str); 3] = [
            (PauseOption::Continue, "CONTINUE"),
            (PauseOption::Settings, "SETTINGS"),
            (PauseOption::Stop, "QUIT TO MENU"),
        ];
        for (i, (opt, label)) in ROWS.iter().enumerate() {
            let selected = active && self.selected_pause_option == *opt;
            let color = if selected {
                Color::new(1.0, 0.20, 0.60, 1.0)
            } else if active {
                Color::WHITE
            } else {
                Color::new(0.6, 0.6, 0.6, 1.0)
            };
            graphics.draw_text(
                label,
                Vec2::new(p.x + 28.0, p.y + 116.0 + i as f32 * 52.0),
                26.0,
                color,
            );
        }
    }
}
