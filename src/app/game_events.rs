//! The in-game frame's EVENT side: what the tick queued (shots, hits, kills,
//! pickups, throws…) bridged into the scenario and into sound. Phases of
//! `update_game` (game_loop.rs), in its order.

use super::*;

impl GameState {
    /// Bridge the frame's events into the scenario: a success on the gated
    /// input releases the active tutorial gate (running the rest of its
    /// step), and a `checkpoint` action that ran this frame is snapshotted
    /// here, after the whole tick settled.
    pub(super) fn bridge_events_to_scenario(&mut self, events: &[crate::components::GameEvent]) {
        let gate_notify_span = perf::span("scenario");
        if let Some(sc) = self.scenario.as_mut() {
            sc.gate_notify(&mut self.world, events);
            if sc.take_checkpoint_request() {
                self.checkpoint = Some(Checkpoint {
                    world: self.world.clone(),
                    scenario: sc.clone(),
                });
            }
        }
        drop(gate_notify_span);
    }

    /// The per-weapon one-shots of this frame's events (+ the sparks and the
    /// kill flash they trigger).
    pub(super) fn play_event_sfx(&mut self, events: Vec<crate::components::GameEvent>) {
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
    }

    /// The whole-game transitions, detected by comparing to the previous
    /// frame: the mask cracking, the player's death, the floor going quiet.
    pub(super) fn play_transition_sfx(
        &mut self,
        player_alive_now: bool,
        boss_enraged: bool,
        all_dead: bool,
    ) {
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
    }
}
