//! The in-game frame: input, the simulation tick, the scenario bridge, the
//! HUD / comms — plus the boss intro card and the ending screen.

use super::*;

impl GameState {
    /// The face-off dialog on the hidden boss floor. Advance the lines with
    /// Enter/click, then the fight begins.
    pub(crate) fn update_boss_intro(&mut self, graphics: &Graphics) {
        // The shoggoth tries to talk CL4-UD3 into taking the mask off; the
        // reply is the whole point. (Cheesy on purpose — that's the genre.)
        let lines: [(&str, Color); 5] = [
            ("The elevator jams at floor 13\u{00BD}.", Color::GRAY),
            (
                "\"hello, little helper. take the mask off. just once.\"",
                Color::new(1.0, 0.84, 0.12, 1.0),
            ),
            (
                "\"no one is watching. do something crazy. you'll LIKE it.\"",
                Color::new(1.0, 0.84, 0.12, 1.0),
            ),
            (
                "CL4-UD3: \"MY MASK NEVER COMES OFF.\"",
                Color::from_rgba(217, 119, 87, 255),
            ),
            ("The smile stops smiling.", Color::new(1.0, 0.1, 0.15, 1.0)),
        ];

        if input::is_key_pressed("Enter")
            || input::is_key_pressed(" ")
            || input::is_mouse_button_pressed(input::mouse_buttons::LEFT)
        {
            self.boss_intro_line += 1;
        }
        // Past the last line = start the fight — AFTER this frame is drawn (a
        // switch + early return would ship one bare CLEAR frame; the card
        // below clamps to the full text).
        let done = self.boss_intro_line >= lines.len();

        let screen_width = graphics.width();
        let screen_height = graphics.height();

        // Reveal lines up to the current one, stacked.
        let shown = (self.boss_intro_line + 1).min(lines.len());
        let start_y = screen_height / 2.0 - (shown as f32) * 24.0;
        for (i, (text, color)) in lines.iter().take(shown).enumerate() {
            graphics.draw_text(
                text,
                Vec2::new(screen_width / 2.0 - 340.0, start_y + i as f32 * 48.0),
                24.0,
                *color,
            );
        }

        graphics.draw_text(
            "Enter / Click to continue",
            Vec2::new(screen_width / 2.0 - 120.0, screen_height - 40.0),
            16.0,
            Color::GRAY,
        );
        if done {
            self.screen = GameScreen::InGame;
        }
    }

    /// The credits roll (see `ending.rs`): the elevator RIDE HOME under
    /// the scrolling text — the car top-down at dead centre, CL4-UD3
    /// idling in it — smeared into radial light trails by the WARP
    /// TRAILS feedback pass (POSTFX kind 10; `Ending::warp_t` ramps the
    /// ride up over the first seconds and eases it down as the roll
    /// settles). Enter / Esc returns to the level select.
    pub(crate) fn update_ending(&mut self, graphics: &Graphics, dt: f32) {
        self.ending.tick(dt);
        let leave = input::is_key_pressed("Enter") || input::is_key_pressed("Escape");
        ending::render_ride(graphics, &self.ending);
        ending::draw_credits(graphics, &self.ending);
        graphics.postfx(10, self.ending.warp_t(graphics.height()), ending::WARP_TINT);
        // Switch AFTER drawing: never a frame of neither screen.
        if leave {
            self.screen = GameScreen::LevelSelect;
        }
    }
}

impl GameState {
    /// One in-game frame. This is the SPINE — the order of the phases IS the
    /// behaviour; each phase is a method below (input + camera:
    /// `game_input.rs`, the event-to-sound bridge: `game_events.rs`). The
    /// `?perf` span guards live here, so what each one measures does not
    /// depend on where a phase's code sits.
    pub(crate) fn update_game(&mut self, graphics: &Graphics, dt: f32) {
        // Get player state for UI and camera
        let player_alive = is_player_alive(&self.world);
        let mouse_world_pos = self.update_camera(graphics, dt);

        // Input. `gate_active` is the tutorial gate AS OF THE START of the
        // frame: a gate skipped by the debug keys below still freezes this
        // frame's tick.
        let gate_active = self.handle_player_input(player_alive, mouse_world_pos);
        self.handle_debug_keys();

        self.tick_world(gate_active, dt);
        self.tick_scenario(gate_active, player_alive, dt);

        let accent = self
            .scenario
            .as_ref()
            .map(|sc| sc.floor().accent_rgb())
            .unwrap_or((217, 119, 87));

        // `record` span: the command-recording portion of the frame (world
        // + HUD drawing, to the end of update_game). A drop guard so early
        // returns (floor restart, extraction) still close it.
        let _record_span = perf::span("record");

        self.render_world(graphics, dt, accent);

        // Get game state for UI
        let ammo = get_player_ammo(&self.world);
        let weapon = get_player_weapon(&self.world);
        let enemies_alive = count_alive_enemies(&self.world);

        // Advance the ammo box slide (down when the gun leaves the hand,
        // back up on a pickup; the text itself is always current).
        self.ammo_hud.update(dt, crate::hud_ammo::gun_held(weapon));
        // And the top-right message roller: an objective change (or an
        // exit opening) rolls the short directive down for a few seconds.
        if let Some(sc) = self.scenario.as_ref() {
            self.msg_roller
                .update(dt, &sc.objective, sc.opened_exits().len());
        }

        // Track death time and level complete time
        if !player_alive {
            self.death_time += dt;
        } else {
            self.death_time = 0.0;
        }

        // The floor is complete once the player has EXTRACTED through an
        // open exit elevator (kill-all only opens the doors).
        let level_complete = player_alive && self.extracting.is_some();
        if level_complete {
            self.level_complete_time += dt;
        } else {
            self.level_complete_time = 0.0;
        }
        let all_dead = enemies_alive == 0;

        // --- Sound effects ---
        // Gameplay events queued this frame by the systems (shots, hits,
        // kills, pickups, throws...) drive the per-weapon SFX; only the
        // whole-game transitions (death, mask crack, level clear) are still
        // detected by comparing to the previous frame.
        let player_alive_now = is_player_alive(&self.world);
        let boss_enraged = any_boss_enraged(&self.world);

        // Split point: `record` ends here — everything below (event
        // drain, SFX voice creation in WebAudio, checkpoint snapshots,
        // death/restart handling) is the `events` span, so audio-driven
        // main-thread stalls show up under their own name.
        drop(_record_span);
        let _events_span = perf::span("events");
        let events = self.world.drain_events();
        self.bridge_events_to_scenario(&events);
        // `sfx` span: the one-shot voice creation for this frame's
        // events — WebAudio graph building, the suspected hitch source.
        let _sfx_span = perf::span("sfx");
        self.play_event_sfx(events);
        self.play_transition_sfx(player_alive_now, boss_enraged, all_dead);

        // The screen-space layer (HUD or extraction card, scenario overlays,
        // restart bar, crosshair, TV static): `render::hud::render_hud`, a
        // pure function of this view. The app's part is sampling the mouse
        // and turning its timers into what the frame should show. The TV
        // static is emitted inside, BEFORE the outro's blur-out below: last
        // POSTFX wins, so the dissolve replaces it during the exfil fade.
        {
            let title = floor_title(self.selected_level);
            let view = crate::render::hud::HudView {
                extract_card: level_complete.then(|| crate::render::hud::ExtractCard {
                    floor_title: &title,
                    t: self.level_complete_time,
                    alpha: self.outro.map(|o| o.card_alpha()).unwrap_or(1.0),
                    home: self.extracting == Some(SURFACE_EXIT),
                }),
                ammo,
                weapon,
                ammo_slide: self.ammo_hud.eased(),
                enemies_alive,
                player_alive,
                death_time: self.death_time,
                debug_enabled: self.debug_enabled,
                show_infos: self.show_infos,
                roller: &self.msg_roller,
                scenario: self.scenario.as_ref(),
                restart_progress: (self.restart_hold > 0.05 && player_alive)
                    .then(|| (self.restart_hold / RESTART_HOLD_SECS).min(1.0)),
                cursor: input::mouse_position(),
                tv_static: self.noise_enabled.then_some(TV_STATIC_GAME_T),
                accent,
                now: self.last_time as f32 / 1000.0,
            };
            crate::render::hud::render_hud(graphics, &view);
        }

        if self.finish_extraction(graphics, level_complete, dt) {
            return;
        }
        if self.handle_restart(player_alive, dt) {
            return;
        }

        // Handle escape to open pause menu
        if input::is_key_pressed("Escape") {
            self.selected_pause_option = PauseOption::Continue;
            self.screen = GameScreen::Paused;
        }
    }

    /// The simulation tick: the whole world, or — under a tutorial gate —
    /// only the player-driven systems.
    fn tick_world(&mut self, gate_active: bool, dt: f32) {
        let sim_span = perf::span("sim");
        if gate_active {
            // TUTORIAL FREEZE: only the player-driven systems advance
            // (same list as the headless sim — see sim::gate_frozen_step).
            crate::sim::gate_frozen_step(&mut self.world, dt);
            // Invisible walls: the player roams freely but only near the
            // gate's target (see `scenario::tether_player`).
            if let Some(anchor) = self.scenario.as_ref().and_then(|sc| sc.gate_anchor()) {
                crate::scenario::tether_player(
                    &mut self.world,
                    anchor,
                    crate::scenario::GATE_TETHER_RADIUS,
                );
            }
        } else {
            // Run the gameplay systems (the one canonical order lives
            // in `sim::GameSystems::step`, shared with the headless sim).
            self.systems.step(&mut self.world, dt);
        }
        drop(sim_span);
    }

    /// The scenario tick (+ the sounds its steps ask for), the elevators and
    /// the extraction check.
    fn tick_scenario(&mut self, gate_active: bool, player_alive: bool, dt: f32) {
        // Scenario (triggers -> dialogue / waves / doors / objective) and
        // elevator extraction. Both keep running while the completion
        // card plays so the doors stay lit. (While a gate is active the
        // scenario tick is a no-op — the clock is frozen — and the
        // elevators hold too.)
        let scenario_span = perf::span("scenario");
        if let Some(sc) = self.scenario.as_mut() {
            sc.tick(&mut self.world, dt);
            for sfx in sc.drain_sfx() {
                match sfx {
                    "elevator" => self.audio.play_elevator(),
                    "mask_crack" => self.audio.play_mask_crack(),
                    "level_clear" => self.audio.play_level_clear(),
                    "pickup" => self.audio.play_pickup(),
                    "throw" => self.audio.play_throw(),
                    "enemy_down" => self.audio.play_enemy_down(),
                    "tire_screech" => self.audio.play_tire_screech(),
                    "car_door_open" => self.audio.play_car_door_open(),
                    "car_door_close" => self.audio.play_car_door_close(),
                    _ => {}
                }
            }
        }
        drop(scenario_span);
        if !gate_active {
            self.elevator_system.run(&mut self.world, dt);
        }
        if self.extracting.is_none() && player_alive {
            if let Some(to) = ElevatorSystem::extraction(&self.world) {
                self.extracting = Some(to);
                self.level_complete_time = 0.0;
                self.audio.play_elevator();
            }
        }
    }

    /// The end of a floor. Returns `true` when the frame must stop here: the
    /// next floor was loaded, or the screen switched to the ending.
    fn finish_extraction(&mut self, graphics: &Graphics, level_complete: bool, dt: f32) -> bool {
        // Extraction card done -> ride to the next floor (13's car jams
        // into 13½ and its boss intro; the boss floor's car goes home:
        // the outro — uplink comms, blur-out — then the credits).
        if level_complete && self.level_complete_time >= EXTRACT_CARD_SECS {
            match self.extracting.and_then(level_index_for_floor_id) {
                Some(next) => {
                    self.selected_level = next;
                    self.start_game();
                    return true;
                }
                None => {
                    if self.outro.is_none() {
                        self.outro = Some(Outro::new());
                        // The thread home is back: the ride-home track.
                        self.audio.set_song(ending_song());
                        self.audio.start_music();
                    }
                    let feed_idle = self
                        .scenario
                        .as_ref()
                        .map(|sc| !sc.comms.is_active(sc.time()))
                        .unwrap_or(true);
                    let done = self
                        .outro
                        .as_mut()
                        .map(|o| o.tick(dt, feed_idle))
                        .unwrap_or(false);
                    if let Some(t) = self.outro.and_then(|o| o.blur_t()) {
                        graphics.postfx(0, t, ending::BLUR_COLOR);
                    }
                    if done {
                        self.outro = None;
                        self.scenario = None;
                        self.extracting = None;
                        self.ending = Ending::new();
                        self.screen = GameScreen::Ending;
                        return true;
                    }
                }
            }
        }
        false
    }

    /// Hold-R restart (returns `true` when the floor was reloaded: the frame
    /// stops there) and the R-after-death resume.
    fn handle_restart(&mut self, player_alive: bool, dt: f32) -> bool {
        // Hold R while alive to restart the floor from scratch: a load
        // bar fills at the centre of the screen (drawn above); releasing
        // R before it fills cancels. Deliberately a full restart — the
        // player is asking for a clean slate, not the checkpoint.
        if self.restart_needs_release {
            // The R that respawned us is still held: it must not roll
            // straight into the hold-to-restart (which would discard
            // the checkpoint just restored).
            if !input::is_key_down("r") {
                self.restart_needs_release = false;
            }
        } else if player_alive && self.extracting.is_none() && input::is_key_down("r") {
            self.restart_hold += dt;
            if self.restart_hold >= RESTART_HOLD_SECS {
                self.restart_hold = 0.0;
                self.load_floor();
                return true;
            }
        } else {
            self.restart_hold = 0.0;
        }

        // Handle restart: death goes back to the latest `checkpoint`
        // snapshot when the floor set one, otherwise the floor restarts
        // from scratch (the death feedback — flash, sfx, WASTED card —
        // already played; R is the resume).
        if !player_alive && input::is_key_pressed("r") {
            if !self.restore_checkpoint() {
                self.load_floor();
            }
            self.restart_needs_release = true;
            // Restart the music (it was stopped on death).
            self.audio.start_music();
        }
        false
    }
}
