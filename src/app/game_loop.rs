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
    pub(crate) fn update_game(&mut self, graphics: &Graphics, dt: f32) {
        // Get player state for UI and camera
        let player_alive = is_player_alive(&self.world);
        let player_pos = get_player_position(&self.world);

        // Update camera to follow player
        if let Some(pos) = player_pos {
            self.camera.follow_player(pos);
        }
        // Scenario `look_at`: ease the focus toward a point of interest.
        self.camera
            .set_cinematic(self.scenario.as_ref().and_then(|sc| sc.look_at()));
        self.camera
            .set_viewport(graphics.width(), graphics.height());
        self.camera.update_sway(self.last_time as f32 / 1000.0);

        // Shift = look-ahead: ease the view toward the mouse while held.
        let mouse_screen_pos = input::mouse_position();
        let looking = input::is_key_down(input::keys::SHIFT);
        self.camera.update_look(mouse_screen_pos, looking, dt);

        // Get mouse position in world coordinates
        let mouse_world_pos = self.camera.screen_to_world(mouse_screen_pos);

        // A scenario `hold` — or an active `talk` conversation — locks
        // movement / fire / throw / pickup (the world keeps running; Esc
        // below still works).
        let dialogue = self
            .scenario
            .as_ref()
            .is_some_and(|sc| sc.dialogue_active());
        let held = self
            .scenario
            .as_ref()
            .is_some_and(|sc| sc.hold_active() || sc.dialogue_active());

        // While a conversation is up, click / Space / Enter ADVANCES it
        // (and, `held` being set, can never fire the weapon).
        if dialogue
            && player_alive
            && (input::is_mouse_button_pressed(input::mouse_buttons::LEFT)
                || input::is_key_pressed(input::keys::SPACE)
                || input::is_key_pressed("Enter"))
        {
            if let Some(sc) = self.scenario.as_mut() {
                sc.dialogue_advance();
            }
        }

        // A running finisher locks the player out of everything: no
        // movement, no aiming (they stay turned onto the victim), no
        // fire / throw / pickup, until the animation completes.
        let finishing = FinisherSystem::active(&self.world);

        // The active tutorial GATE, if any: the world freezes (only the
        // player-driven systems run, below) and every input except the
        // gated one is masked. Aim and movement stay live so the player
        // can close the distance to the frozen target.
        let gate = self.scenario.as_ref().and_then(|sc| sc.gate_view());

        // The world holds its breath under a tutorial gate: the music
        // stops with it (everyone just stands there — wtf?) and comes
        // back once the gated action lands.
        let gate_active = gate.is_some();
        if gate_active != self.music_frozen {
            if gate_active {
                self.audio.stop_music();
            } else {
                self.audio.start_music();
            }
            self.music_frozen = gate_active;
        }

        // Handle input (only if the player is alive and hasn't left in
        // the car yet)
        if player_alive && self.extracting.is_none() && finishing {
            stop_player(&mut self.world);
        }
        if player_alive && self.extracting.is_none() && !finishing {
            if let Some(g) = gate {
                // Movement stays live so the player can close the distance
                // to the frozen target; everything else goes through the
                // shared, host-tested gate dispatch (game.rs).
                InputSystem::update_player_movement(&mut self.world);
                let intents = PlayerIntents {
                    left_pressed: input::is_mouse_button_pressed(input::mouse_buttons::LEFT),
                    left_down: input::is_mouse_button_down(input::mouse_buttons::LEFT),
                    right_pressed: input::is_mouse_button_pressed(input::mouse_buttons::RIGHT),
                    e_pressed: input::is_key_pressed("e"),
                    mouse_world: mouse_world_pos,
                };
                gated_player_input(&mut self.world, g, &intents);
            } else if held {
                InputSystem::update_player_rotation(&mut self.world, mouse_world_pos);
                stop_player(&mut self.world);
            } else {
                InputSystem::update_player_rotation(&mut self.world, mouse_world_pos);
                InputSystem::update_player_movement(&mut self.world);
                // Fighting can be scenario-disabled (`combat: false` —
                // the parking-lot walk): fire / punch / finisher / throw
                // are masked; walking, aiming and E stay live. Gates
                // bypass this (their branch above).
                let combat_ok = self.scenario.as_ref().is_none_or(|sc| sc.combat_enabled());
                // A fresh click over a DOWNED enemy in reach executes a
                // FINISHER instead of a normal attack; otherwise the trigger
                // behaves exactly as before.
                let finisher_started = combat_ok
                    && input::is_mouse_button_pressed(input::mouse_buttons::LEFT)
                    && FinisherSystem::try_start(&mut self.world);
                if combat_ok && !finisher_started {
                    InputSystem::handle_shoot_input(&mut self.world, mouse_world_pos);
                }

                // Press E to pick up / swap the weapon the player is standing on
                // (the Pickup event it emits plays the sound below).
                if input::is_key_pressed("e") {
                    PickupSystem::swap_for_player(&mut self.world);
                }

                // Right-click to throw the held weapon toward the cursor (the
                // Throw event it emits plays the sound below).
                if combat_ok && input::is_mouse_button_pressed(input::mouse_buttons::RIGHT) {
                    if let Some(player_pos) = get_player_position(&self.world) {
                        let aim = mouse_world_pos - player_pos;
                        ThrownWeaponSystem::throw_from_player(&mut self.world, aim);
                    }
                }
            }
        }

        // Handle info display toggle
        if self.debug_enabled && input::is_key_pressed("i") {
            self.show_infos = !self.show_infos;
        }
        // Tell the systems whether debug visualization is visible this
        // frame: DebugPath / DebugTrail are only recorded while it is.
        self.world
            .set_debug_viz(self.debug_enabled && self.show_infos);
        // Debug: with the overlays on, K downs every rogue (fast-forwards
        // the all-dead scenario steps / exit doors when testing a floor).
        if self.debug_enabled && self.show_infos && input::is_key_pressed("k") {
            purge_all_enemies(&mut self.world);
        }
        // Debug: B cracks the boss's mask (drops it to the enrage threshold)
        // to preview the mask-off transition / raw form without the fight.
        if self.debug_enabled && self.show_infos && input::is_key_pressed("b") {
            crate::systems::boss::crack_boss_masks(&mut self.world);
        }
        // Debug: G skips the active tutorial gate (releases it as if the
        // gated input had succeeded) so a gate can never softlock.
        if self.debug_enabled && self.show_infos && input::is_key_pressed("g") {
            if let Some(sc) = self.scenario.as_mut() {
                sc.gate_skip(&mut self.world);
            }
        }

        let sim_span = perf::span("sim");
        if gate.is_some() {
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
        if gate.is_none() {
            self.elevator_system.run(&mut self.world, dt);
        }
        if self.extracting.is_none() && player_alive {
            if let Some(to) = ElevatorSystem::extraction(&self.world) {
                self.extracting = Some(to);
                self.level_complete_time = 0.0;
                self.audio.play_elevator();
            }
        }

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

        // Cap per event kind per frame so a pile-up (a shotgun crowd, a
        // burst of kills) plays a few, not dozens. The machine gun fires a
        // round every tick (0.1 s) while the trigger is held and
        // `play_attack_machinegun` bakes ONE short crack: one call per
        // spawned round means hearing exactly as many bangs as bullets
        // (at most one MG round leaves per frame, so the cap never bites).
        const MAX_SFX_PER_KIND: u32 = 3;
        let mut fired = [0u32; 4];
        let mut hits = [0u32; 4];
        let mut counts = [0u32; 5];
        let slot = |t: crate::components::WeaponType| match t {
            crate::components::WeaponType::Pistol => 0,
            crate::components::WeaponType::MachineGun => 1,
            crate::components::WeaponType::Shotgun => 2,
            crate::components::WeaponType::Melee => 3,
        };
        // Split point: `record` ends here — everything below (event
        // drain, SFX voice creation in WebAudio, checkpoint snapshots,
        // death/restart handling) is the `events` span, so audio-driven
        // main-thread stalls show up under their own name.
        drop(_record_span);
        let _events_span = perf::span("events");
        let events = self.world.drain_events();
        // Bridge the frame's events into the scenario: a success on the
        // gated input releases the active tutorial gate (running the rest
        // of its step), and a `checkpoint` action that ran this frame is
        // snapshotted here, after the whole tick settled.
        let gate_notify_span = perf::span("scenario");
        if let Some(sc) = self.scenario.as_mut() {
            sc.gate_notify(&mut self.world, &events);
            if sc.take_checkpoint_request() {
                self.checkpoint = Some(Checkpoint {
                    world: self.world.clone(),
                    scenario: sc.clone(),
                });
            }
        }
        drop(gate_notify_span);
        // `sfx` span: the one-shot voice creation for this frame's
        // events — WebAudio graph building, the suspected hitch source.
        let _sfx_span = perf::span("sfx");
        for event in events {
            use crate::components::{GameEvent, WeaponType};
            match event {
                GameEvent::PlayerFired(t) => {
                    let s = slot(t);
                    if fired[s] < MAX_SFX_PER_KIND {
                        fired[s] += 1;
                        match t {
                            WeaponType::Pistol => self.audio.play_attack_gun(),
                            WeaponType::MachineGun => self.audio.play_attack_machinegun(),
                            WeaponType::Shotgun => self.audio.play_attack_shotgun(),
                            WeaponType::Melee => self.audio.play_attack_club(),
                        }
                    }
                }
                GameEvent::EnemyHit { by, at } => {
                    // Electric spark burst at the impact point (renders
                    // from the next frame — well inside its lifetime).
                    self.sparks.spawn(at, self.last_time as f32 / 1000.0);
                    let s = slot(by);
                    if hits[s] < MAX_SFX_PER_KIND {
                        hits[s] += 1;
                        match by {
                            WeaponType::Pistol => self.audio.play_hit_gun(),
                            WeaponType::MachineGun => self.audio.play_hit_machinegun(),
                            WeaponType::Shotgun => self.audio.play_hit_shotgun(),
                            WeaponType::Melee => self.audio.play_hit_club(),
                        }
                    }
                }
                GameEvent::EnemyDown => {
                    self.kill_flash = KILL_FLASH_SECS;
                    if counts[0] < MAX_SFX_PER_KIND {
                        counts[0] += 1;
                        self.audio.play_enemy_down();
                    }
                }
                GameEvent::PlayerHurt => {
                    if counts[1] < MAX_SFX_PER_KIND {
                        counts[1] += 1;
                        self.audio.play_player_hurt();
                    }
                }
                GameEvent::Pickup => {
                    if counts[2] < MAX_SFX_PER_KIND {
                        counts[2] += 1;
                        self.audio.play_pickup();
                    }
                }
                GameEvent::Throw => {
                    if counts[3] < MAX_SFX_PER_KIND {
                        counts[3] += 1;
                        self.audio.play_throw();
                    }
                }
                GameEvent::ThrownImpact => {
                    if counts[4] < MAX_SFX_PER_KIND {
                        counts[4] += 1;
                        self.audio.play_hit_club(); // reused: a weapon clonks a bot
                    }
                }
                GameEvent::DryFire => {
                    // TODO: no dry-fire click in the audio engine yet.
                }
                // Gate signals: their companion events above already
                // carry the sounds.
                GameEvent::PunchLanded | GameEvent::StrikeLanded | GameEvent::FinisherDone => {}
            }
        }
        if boss_enraged && !self.prev_boss_enraged {
            self.audio.play_mask_crack();
        }
        if !player_alive_now && self.prev_player_alive {
            self.audio.play_death();
            self.audio.stop_music();
        }
        if all_dead && !self.prev_all_dead {
            self.audio.play_level_clear();
        }

        self.prev_player_alive = player_alive_now;
        self.prev_boss_enraged = boss_enraged;
        self.prev_all_dead = all_dead;

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

        // Extraction card done -> ride to the next floor (13's car jams
        // into 13½ and its boss intro; the boss floor's car goes home:
        // the outro — uplink comms, blur-out — then the credits).
        if level_complete && self.level_complete_time >= EXTRACT_CARD_SECS {
            match self.extracting.and_then(level_index_for_floor_id) {
                Some(next) => {
                    self.selected_level = next;
                    self.start_game();
                    return;
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
                        return;
                    }
                }
            }
        }

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
                return;
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

        // Handle escape to open pause menu
        if input::is_key_pressed("Escape") {
            self.selected_pause_option = PauseOption::Continue;
            self.screen = GameScreen::Paused;
        }
    }
}
