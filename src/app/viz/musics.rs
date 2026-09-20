//! The MUSICS tab: TRACKER (songs + the live step-sequencer) and SOUNDS.

use super::*;

impl GameState {
    /// MUSICS tab: two sub-pages — the TRACKER (songs + the live
    /// step-sequencer view) and the SOUNDS board (every one-shot SFX).
    pub(crate) fn draw_viz_musics(&mut self, graphics: &Graphics, mouse: Vec2, click: bool) {
        let pages = [(false, "TRACKER"), (true, "SOUNDS")];
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
                self.viz_sounds_page == page,
            );
            if over && click && self.viz_sounds_page != page {
                self.viz_sounds_page = page;
            }
        }
        if self.viz_sounds_page {
            self.draw_viz_sounds(graphics, mouse, click);
        } else {
            self.draw_viz_tracker(graphics, mouse, click);
        }
    }

    /// The TRACKER page of the MUSICS tab: a step-sequencer *tracker* for
    /// the live audio engine. A SECTIONS strip of clickable miniatures (one
    /// per arrangement section, shaded by note density, current section
    /// highlighted) sits above the PATTERN grid of the currently-playing
    /// section (five channels; filled cells are notes; playhead column;
    /// click a column to seek; M/S mute/solo per row). Song-select buttons
    /// above.
    pub(crate) fn draw_viz_tracker(&mut self, graphics: &Graphics, mouse: Vec2, click: bool) {
        let coral = Color::from_rgba(217, 119, 87, 255);
        graphics.draw_text(
            "TRACKER — click a song, a section miniature, or a grid column.",
            Vec2::new(40.0, 138.0),
            18.0,
            Color::GRAY,
        );

        // --- song select -----------------------------------------------
        graphics.draw_text("SONGS", Vec2::new(40.0, 162.0), 16.0, coral);
        let cur_name = self.audio.current_song().name;
        let songs = &*crate::audio::SONGS;
        for (i, song) in songs.iter().enumerate() {
            let x = 40.0 + (i % 4) as f32 * 168.0;
            let y = 172.0 + (i / 4) as f32 * 46.0;
            let active = song.name == cur_name && self.audio.is_playing();
            if viz_button(graphics, mouse, x, y, 158.0, 40.0, song.name, active) && click {
                self.audio.resume();
                self.audio.play_song(i);
            }
        }
        let song_rows = songs.len().div_ceil(4) as f32;
        let mut y = 172.0 + song_rows * 46.0 + 4.0;
        if viz_button(graphics, mouse, 40.0, y, 158.0, 40.0, "STOP", false) && click {
            self.audio.stop_music();
        }

        // --- section miniatures (the arrangement mini-map) ---------------
        y += 54.0;
        graphics.draw_text("SECTIONS", Vec2::new(40.0, y), 16.0, coral);
        let strip_top = y + 8.0;
        let n_sections = self.audio.section_count().max(1);
        let cur_section = self.audio.current_section();
        let gx = 210.0f32;
        let gw = (graphics.width() - gx - 40.0).max(160.0);
        let mh = 34.0f32; // miniature height
        let mw = gw / n_sections as f32;
        for sec in 0..n_sections {
            let mx = gx + sec as f32 * mw;
            let is_cur = sec == cur_section && self.audio.is_playing();
            let over = mouse.x >= mx
                && mouse.x <= mx + mw
                && mouse.y >= strip_top
                && mouse.y <= strip_top + mh;
            // Card, shaded by how dense the section is.
            let d = self.audio.section_density(sec);
            let bg = if is_cur {
                Color::new(1.0, 0.09, 0.26, 0.30)
            } else if over {
                Color::new(0.28, 0.22, 0.33, 1.0)
            } else {
                Color::new(0.10 + d * 0.10, 0.08 + d * 0.06, 0.14 + d * 0.10, 1.0)
            };
            graphics.draw_rectangle(Vec2::new(mx + 1.0, strip_top), mw - 2.0, mh, bg);
            // Miniature pattern: the section's cells squeezed into the card.
            let s_len = self.audio.section_pattern_len(sec).max(1);
            let cw_m = (mw - 6.0) / s_len as f32;
            let rh_m = (mh - 6.0) / crate::audio::NUM_CHANNELS as f32;
            for r in 0..crate::audio::NUM_CHANNELS {
                for s in 0..s_len {
                    if self.audio.section_cell(sec, r, s) {
                        graphics.draw_rectangle(
                            Vec2::new(
                                mx + 3.0 + s as f32 * cw_m,
                                strip_top + 3.0 + r as f32 * rh_m,
                            ),
                            cw_m.max(1.0),
                            rh_m.max(1.0),
                            if is_cur {
                                Color::new(1.0, 0.75, 0.6, 0.95)
                            } else {
                                Color::new(0.62, 0.5, 0.72, 0.9)
                            },
                        );
                    }
                }
            }
            let border = if is_cur {
                Color::new(1.0, 0.09, 0.26, 1.0)
            } else {
                Color::new(0.35, 0.28, 0.4, 1.0)
            };
            graphics.draw_rectangle_lines(
                Vec2::new(mx + 1.0, strip_top),
                mw - 2.0,
                mh,
                1.0,
                border,
            );
            if over && click {
                self.audio.resume();
                self.audio.jump_to_section(sec);
            }
        }
        // Section labels under the strip (current one highlighted).
        graphics.draw_text(
            self.audio.current_section_label(),
            Vec2::new(gx, strip_top + mh + 16.0),
            14.0,
            coral,
        );

        // --- tracker grid (the currently-playing section) ----------------
        y = strip_top + mh + 26.0;
        graphics.draw_text("PATTERN", Vec2::new(40.0, y), 16.0, coral);
        let grid_top = y + 12.0;
        let steps = self.audio.pattern_len().max(1);
        let cur_step = self.audio.current_step();
        let playing = self.audio.is_playing();
        let rows = crate::audio::NUM_CHANNELS;
        let names = crate::audio::CHANNEL_NAMES;
        let chan_col = [
            Color::from_rgba(217, 119, 87, 255), // bass
            Color::from_rgba(80, 200, 240, 255), // lead
            Color::from_rgba(150, 90, 210, 255), // pad
            Color::from_rgba(224, 80, 170, 255), // arp
            Color::from_rgba(230, 200, 60, 255), // drums
        ];
        let rh = 26.0f32;
        let cw = gw / steps as f32;

        // Playhead column highlight (drawn behind the cells).
        if playing {
            graphics.draw_rectangle(
                Vec2::new(gx + cur_step as f32 * cw, grid_top - 2.0),
                cw,
                rows as f32 * rh + 4.0,
                Color::new(1.0, 1.0, 1.0, 0.14),
            );
        }

        for r in 0..rows {
            let ry = grid_top + r as f32 * rh;
            let muted = self.audio.is_muted(r);
            let soloed = self.audio.is_solo(r);
            let col = chan_col[r.min(chan_col.len() - 1)];
            let name_col = if muted { Color::GRAY } else { col };
            graphics.draw_text(
                names[r],
                Vec2::new(40.0, ry + rh * 0.5 + 6.0),
                16.0,
                name_col,
            );
            if viz_button(graphics, mouse, 118.0, ry + 2.0, 26.0, rh - 6.0, "M", muted) && click {
                self.audio.toggle_mute(r);
            }
            if viz_button(
                graphics,
                mouse,
                150.0,
                ry + 2.0,
                26.0,
                rh - 6.0,
                "S",
                soloed,
            ) && click
            {
                self.audio.toggle_solo(r);
            }
            for s in 0..steps {
                let cx = gx + s as f32 * cw;
                // Beat markers: every 4th column reads a touch brighter.
                let bg = if s % 4 == 0 {
                    Color::new(0.16, 0.13, 0.20, 1.0)
                } else {
                    Color::new(0.10, 0.09, 0.13, 1.0)
                };
                graphics.draw_rectangle(Vec2::new(cx + 1.0, ry + 2.0), cw - 2.0, rh - 4.0, bg);
                if self.audio.channel_active(r, s) {
                    let c = if muted {
                        Color::new(col.r * 0.4, col.g * 0.4, col.b * 0.4, 1.0)
                    } else {
                        col
                    };
                    let inset = if playing && s == cur_step { 2.0 } else { 4.0 };
                    graphics.draw_rectangle(
                        Vec2::new(cx + inset, ry + inset),
                        (cw - inset * 2.0).max(2.0),
                        rh - inset * 2.0,
                        c,
                    );
                }
            }
        }
        graphics.draw_rectangle_lines(
            Vec2::new(gx, grid_top),
            steps as f32 * cw,
            rows as f32 * rh,
            1.5,
            Color::new(0.45, 0.35, 0.5, 1.0),
        );

        // Click anywhere in the grid to seek to that column's step.
        let grid_bottom = grid_top + rows as f32 * rh;
        if click
            && mouse.x >= gx
            && mouse.x <= gx + steps as f32 * cw
            && mouse.y >= grid_top
            && mouse.y <= grid_bottom
        {
            let s = ((mouse.x - gx) / cw) as usize;
            self.audio.seek(s.min(steps - 1));
        }
    }

    /// The SOUNDS page of the MUSICS tab: the full per-weapon SFX
    /// taxonomy, one button per one-shot.
    /// Row 1: attack (the weapon firing/swinging).
    /// Row 2: hit (that weapon's impact on a metal bot).
    /// Rows 3+: the rest of the one-shot game sounds.
    pub(crate) fn draw_viz_sounds(&mut self, graphics: &Graphics, mouse: Vec2, click: bool) {
        let coral = Color::from_rgba(217, 119, 87, 255);
        graphics.draw_text(
            "SFX — click a sound to play it.",
            Vec2::new(40.0, 138.0),
            18.0,
            Color::GRAY,
        );
        let mut sy = 162.0;
        graphics.draw_text("SFX", Vec2::new(40.0, sy), 16.0, coral);
        sy += 12.0;
        let bw_s = 158.0f32;
        let bh_s = 34.0f32;
        let attack = [
            "attack: club",
            "attack: gun",
            "attack: machinegun",
            "attack: shotgun",
        ];
        for (i, &name) in attack.iter().enumerate() {
            let x = 40.0 + i as f32 * 168.0;
            if viz_button(graphics, mouse, x, sy, bw_s, bh_s, name, false) && click {
                self.audio.resume();
                match i {
                    0 => self.audio.play_attack_club(),
                    1 => self.audio.play_attack_gun(),
                    2 => self.audio.play_attack_machinegun(),
                    _ => self.audio.play_attack_shotgun(),
                }
            }
        }
        sy += bh_s + 6.0;
        let hit = ["hit: club", "hit: gun", "hit: machinegun", "hit: shotgun"];
        for (i, &name) in hit.iter().enumerate() {
            let x = 40.0 + i as f32 * 168.0;
            if viz_button(graphics, mouse, x, sy, bw_s, bh_s, name, false) && click {
                self.audio.resume();
                match i {
                    0 => self.audio.play_hit_club(),
                    1 => self.audio.play_hit_gun(),
                    2 => self.audio.play_hit_machinegun(),
                    _ => self.audio.play_hit_shotgun(),
                }
            }
        }
        sy += bh_s + 6.0;
        let misc = [
            "Rogue down",
            "Pickup",
            "Throw",
            "Player hurt",
            "Death",
            "Level clear",
            "Mask crack",
            "Elevator",
        ];
        for (i, &name) in misc.iter().enumerate() {
            let x = 40.0 + (i % 4) as f32 * 168.0;
            let by = sy + (i / 4) as f32 * (bh_s + 6.0);
            if viz_button(graphics, mouse, x, by, bw_s, bh_s, name, false) && click {
                self.audio.resume();
                match i {
                    0 => self.audio.play_enemy_down(),
                    1 => self.audio.play_pickup(),
                    2 => self.audio.play_throw(),
                    3 => self.audio.play_player_hurt(),
                    4 => self.audio.play_death(),
                    5 => self.audio.play_level_clear(),
                    6 => self.audio.play_mask_crack(),
                    _ => self.audio.play_elevator(),
                }
            }
        }
    }
}
