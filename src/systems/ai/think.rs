//! The state machine: Unaware -> SpottedUnsure -> SurePlayerSeen -> Confused.
//! One function per state; none of them moves the bot (that is `steer`).

use super::rng::*;
use super::senses::Percept;
use super::AISystem;
use crate::components::{AIState, EnemyType, UnsurePhase, WanderState, AI};
use crate::ecs::world::Wall;

/// Advance one bot's state machine by a tick. Returns `true` on the tick a
/// feral completes a lunge, so the caller can apply its burn-out self-damage
/// once the (exclusive) AI borrow is released.
pub(super) fn think(ai: &mut AI, p: &Percept, rng: &mut u32, walls: &[Wall], dt: f32) -> bool {
    // Update timers
    if ai.attack_timer > 0.0 {
        ai.attack_timer -= dt;
    }
    ai.state_timer -= dt;

    match ai.state {
        AIState::Unaware => think_unaware(ai, p, rng, walls, dt),
        AIState::SpottedUnsure => think_unsure(ai, p, rng),
        AIState::SurePlayerSeen => return think_chasing(ai, p, rng, dt),
        AIState::Confused => think_confused(ai, p, rng, dt),
        _ => {} // Legacy states
    }
    false
}

fn think_unaware(ai: &mut AI, p: &Percept, rng: &mut u32, walls: &[Wall], dt: f32) {
    if p.sees_player {
        ai.state_timer = ai.spot_duration;
        ai.state = AIState::SpottedUnsure;
        ai.unsure_phase = UnsurePhase::Spotting;
        ai.check_position = Some(p.pos);
        ai.last_known_player_position = Some(p.player);
        return;
    }
    // Perform initial behavior based on type
    match ai.initial_type {
        EnemyType::Idle => {
            // SENTINEL soldier: parked at its rack, guarding.
        }
        EnemyType::Patrolling => {
            // HUNTER soldier: disciplined patrol (move, sweep a
            // look-around, pause, repeat).
            AISystem::update_wander_behavior(rng, ai, &p.pos, walls, dt);
        }
        EnemyType::Wandering => {
            // DRIFTER feral: erratic, objective-less twitching — no careful
            // look-around.
            AISystem::update_feral_wander(rng, ai, &p.pos, walls, dt);
        }
    }
}

fn think_unsure(ai: &mut AI, p: &Percept, rng: &mut u32) {
    if p.sees_player {
        ai.last_known_player_position = Some(p.player);
        if ai.unsure_phase != UnsurePhase::Spotting {
            // Caught sight again mid-investigation: spot afresh.
            ai.unsure_phase = UnsurePhase::Spotting;
            ai.state_timer = ai.spot_duration;
        }
        if ai.state_timer <= 0.0 {
            // Seen player long enough, transition to sure
            ai.state = AIState::SurePlayerSeen;
            AISystem::arm_feral_lunge(ai);
        }
        return;
    }
    match ai.unsure_phase {
        UnsurePhase::Spotting => {
            // Lost sight: go and check where they were.
            ai.unsure_phase = UnsurePhase::Investigating;
            ai.state_timer = ai.unsure_check_duration;
        }
        UnsurePhase::Investigating => {
            // Reached the spot (or ran out of patience getting there): head
            // back.
            let reached = ai
                .last_known_player_position
                .is_none_or(|t| p.pos.distance_to(&t) < 5.0);
            if reached || ai.state_timer <= 0.0 {
                ai.unsure_phase = UnsurePhase::Returning;
                ai.state_timer = ai.unsure_check_duration;
            }
        }
        UnsurePhase::Returning => {
            // Back at the post (or gave up on the way): nothing there, resume
            // the routine.
            let home = ai
                .check_position
                .is_none_or(|c| p.pos.distance_to(&c) < 5.0);
            if home || ai.state_timer <= 0.0 {
                ai.state = AIState::Unaware;
                ai.unsure_phase = UnsurePhase::Spotting;
                ai.check_position = None;
                ai.wander_state = WanderState::Waiting;
                ai.wander_timer = random_range(rng, 1.0, 2.0);
            }
        }
    }
}

/// Returns `true` on the tick a feral's lunge completes.
fn think_chasing(ai: &mut AI, p: &Percept, rng: &mut u32, dt: f32) -> bool {
    // Ferals don't "chase" — they drive a lunge cadence, locking a burst
    // direction at each wind-up and burning a chip of themselves out on every
    // completed dash.
    let mut lunge_completed = false;
    if ai.initial_type == EnemyType::Wandering {
        let target = ai.last_known_player_position.unwrap_or(p.player);
        lunge_completed = AISystem::update_feral_lunge(rng, ai, &p.pos, &target, dt);
    }
    if p.sees_player {
        // Keep chasing
        ai.last_known_player_position = Some(p.player);
        ai.state_timer = ai.lost_player_duration;
    } else if ai.state_timer <= 0.0 {
        // Lost sight, and been at the last known position too long: get
        // confused
        ai.state = AIState::Confused;
        ai.confusion_looks_remaining = random_int_range(rng, 2, 3);
        ai.confusion_look_timer = ai.confusion_look_duration;
    }
    lunge_completed
}

fn think_confused(ai: &mut AI, p: &Percept, rng: &mut u32, dt: f32) {
    if p.sees_player {
        // Found player again!
        ai.state = AIState::SurePlayerSeen;
        AISystem::arm_feral_lunge(ai);
        ai.last_known_player_position = Some(p.player);
        ai.state_timer = ai.lost_player_duration;
        return;
    }
    // Continue looking around
    ai.confusion_look_timer -= dt;
    if ai.confusion_look_timer <= 0.0 {
        ai.confusion_looks_remaining -= 1;
        if ai.confusion_looks_remaining <= 0 {
            // Done looking, back to the routine
            ai.state = AIState::Unaware;
            ai.wander_state = WanderState::Waiting;
            ai.wander_timer = random_range(rng, 1.0, 2.0);
        } else {
            // Look in another direction
            ai.confusion_look_timer = ai.confusion_look_duration;
        }
    }
}
