//! The in-game frame's INPUT side: the camera (it needs the mouse), the
//! player's controls — conversations, tutorial gates, holds, combat — and the
//! `?debug` keys. Phases of `update_game` (game_loop.rs), in its order.

use super::*;

impl GameState {
    /// Follow the player, apply the scenario's `look_at` and the Shift
    /// look-ahead. Returns the mouse position in WORLD coordinates.
    pub(super) fn update_camera(&mut self, graphics: &Graphics, dt: f32) -> Vec2 {
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
        self.camera.screen_to_world(mouse_screen_pos)
    }

    /// The player's controls for this frame. Returns whether a tutorial gate
    /// was active when the frame started.
    pub(super) fn handle_player_input(
        &mut self,
        player_alive: bool,
        mouse_world_pos: Vec2,
    ) -> bool {
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
        gate_active
    }

    /// The `?debug` keys: I toggles the overlays; with them on, K / B / G.
    pub(super) fn handle_debug_keys(&mut self) {
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
    }
}
