//! Idle movement: the soldiers' patrol, the feral drifters' twitching and
//! their lunge cadence.

use super::rng::*;
use super::AISystem;
use crate::components::{EnemyType, Position, WanderState, AI};
use crate::ecs::world::Wall;
use std::f32::consts::PI;

// --- Feral (DRIFTER / `EnemyType::Wandering`) tuning -------------------------
// The corruptor's soldiers (Idle/Patrolling) pursue with discipline: they
// pathfind toward the player and hold at weapon range. The feral drifter has no
// such objective. When it locks on it *lunges* — a locked-in straight-line burst
// of speed at the player's position, no pathfinding, no braking at range — then
// briefly winds up and lunges again, burning a chip of itself out on every dash.

/// Speed multiplier applied to a feral's base speed during a lunge burst.
pub(super) const FERAL_LUNGE_SPEED_MULT: f32 = 2.4;
/// Speed multiplier while a feral is winding up between lunges (a slow creep).
pub(super) const FERAL_RECOVER_SPEED_MULT: f32 = 0.35;
/// Lunge-burst duration bounds (seconds).
const FERAL_LUNGE_MIN: f32 = 0.30;
const FERAL_LUNGE_MAX: f32 = 0.55;
/// Wind-up / recover duration bounds between lunges (seconds).
const FERAL_RECOVER_MIN: f32 = 0.20;
const FERAL_RECOVER_MAX: f32 = 0.40;
/// Chip of self-damage a feral takes each time it completes a lunge ("running
/// down like a dropped call"). Deterministic and discrete — no accumulator.
pub(super) const FERAL_LUNGE_SELF_DAMAGE: i32 = 1;
/// Erratic idle-wander burst duration bounds for a feral (seconds). Much shorter
/// and jitterier than the soldier patrol so the two read as different creatures.
const FERAL_WANDER_MIN: f32 = 0.20;
const FERAL_WANDER_MAX: f32 = 0.70;

impl AISystem {
    /// Check if position is within the allowed movement square
    fn is_within_movement_square(pos: &Position, spawn: &Position, square_size: f32) -> bool {
        let dx = (pos.x - spawn.x).abs();
        let dy = (pos.y - spawn.y).abs();
        dx <= square_size && dy <= square_size
    }

    /// Find the direction with the most open space using 36 direction rays
    fn find_most_open_direction(
        rng: &mut u32,
        pos: &Position,
        spawn: &Position,
        square_size: f32,
        walls: &[Wall],
    ) -> f32 {
        // Check 36 directions (every 10 degrees)
        let mut best_direction = 0.0;
        let mut best_distance = 0.0;

        for i in 0..36 {
            let angle = (i as f32) * (PI * 2.0 / 36.0);
            let max_check_distance = 200.0; // Check up to 200 pixels

            // Cast ray to find nearest obstacle
            let mut distance = max_check_distance;

            // Check against walls
            for step in 1..=20 {
                let check_distance = (step as f32) * 10.0;
                let check_x = pos.x + angle.cos() * check_distance;
                let check_y = pos.y + angle.sin() * check_distance;
                let check_pos = Position::new(check_x, check_y);

                // Check if hits wall
                let hit_wall = walls.iter().any(|wall| {
                    check_x >= wall.x
                        && check_x <= wall.x + wall.width
                        && check_y >= wall.y
                        && check_y <= wall.y + wall.height
                });

                // Check if outside movement square
                let outside_square =
                    !Self::is_within_movement_square(&check_pos, spawn, square_size);

                if hit_wall || outside_square {
                    distance = check_distance;
                    break;
                }
            }

            if distance > best_distance {
                best_distance = distance;
                best_direction = angle;
            }
        }

        // 50% chance: use best direction, 50% chance: random direction
        if coin_flip(rng) {
            best_direction
        } else {
            (next_random(rng) as f32 / u32::MAX as f32) * PI * 2.0
        }
    }
}

impl AISystem {
    /// Reset a feral's lunge cadence so its next tick winds up and dashes at the
    /// player. No-op for soldiers. Called at the moment a drifter locks on.
    pub(super) fn arm_feral_lunge(ai: &mut AI) {
        if ai.initial_type == EnemyType::Wandering {
            ai.wander_state = WanderState::Waiting;
            ai.wander_timer = 0.0;
        }
    }

    /// Erratic feral idle movement: short random bursts with no careful
    /// look-around sweep. On hitting a wall or its leash edge it just picks a new
    /// fully-random heading — twitchy and objective-less, unlike the soldier
    /// patrol which deliberately seeks the most open direction.
    pub(super) fn update_feral_wander(
        rng: &mut u32,
        ai: &mut AI,
        pos: &Position,
        walls: &[Wall],
        dt: f32,
    ) {
        ai.wander_state = WanderState::Moving;
        ai.wander_timer -= dt;

        let next_pos = Position::new(
            pos.x + ai.wander_direction.cos() * 5.0,
            pos.y + ai.wander_direction.sin() * 5.0,
        );
        let hit_wall = walls.iter().any(|wall| {
            next_pos.x >= wall.x
                && next_pos.x <= wall.x + wall.width
                && next_pos.y >= wall.y
                && next_pos.y <= wall.y + wall.height
        });
        let outside_square = !Self::is_within_movement_square(
            &next_pos,
            &ai.spawn_position,
            ai.movement_square_size,
        );

        if ai.wander_timer <= 0.0 || hit_wall || outside_square {
            ai.wander_direction = random_range(rng, 0.0, PI * 2.0);
            ai.wander_timer = random_range(rng, FERAL_WANDER_MIN, FERAL_WANDER_MAX);
        }
    }

    /// Advance a feral's lunge cadence toward `target`. Winds up (creeping) then
    /// locks a heading and dashes for a short burst. Returns `true` on the tick a
    /// dash completes, so the caller can apply burn-out self-damage.
    pub(super) fn update_feral_lunge(
        rng: &mut u32,
        ai: &mut AI,
        enemy_pos: &Position,
        target: &Position,
        dt: f32,
    ) -> bool {
        ai.wander_timer -= dt;
        match ai.wander_state {
            WanderState::Moving => {
                // Mid-dash: keep charging the locked heading until the burst ends.
                if ai.wander_timer <= 0.0 {
                    ai.wander_state = WanderState::Waiting;
                    ai.wander_timer = random_range(rng, FERAL_RECOVER_MIN, FERAL_RECOVER_MAX);
                    return true;
                }
                false
            }
            _ => {
                // Winding up: when the timer elapses, lock a fresh heading at the
                // target's *current* position and dash.
                if ai.wander_timer <= 0.0 {
                    let dx = target.x - enemy_pos.x;
                    let dy = target.y - enemy_pos.y;
                    ai.wander_direction = dy.atan2(dx);
                    ai.wander_state = WanderState::Moving;
                    ai.wander_timer = random_range(rng, FERAL_LUNGE_MIN, FERAL_LUNGE_MAX);
                }
                false
            }
        }
    }

    /// Update wandering/patrolling behavior
    pub(super) fn update_wander_behavior(
        rng: &mut u32,
        ai: &mut AI,
        pos: &Position,
        walls: &[Wall],
        dt: f32,
    ) {
        match ai.wander_state {
            WanderState::Moving => {
                ai.wander_timer -= dt;

                // Check if hit wall or outside square
                let next_pos = Position::new(
                    pos.x + ai.wander_direction.cos() * 5.0,
                    pos.y + ai.wander_direction.sin() * 5.0,
                );

                let hit_wall = walls.iter().any(|wall| {
                    next_pos.x >= wall.x
                        && next_pos.x <= wall.x + wall.width
                        && next_pos.y >= wall.y
                        && next_pos.y <= wall.y + wall.height
                });

                let outside_square = !Self::is_within_movement_square(
                    &next_pos,
                    &ai.spawn_position,
                    ai.movement_square_size,
                );

                if ai.wander_timer <= 0.0 || hit_wall || outside_square {
                    // Stop and look around
                    ai.wander_state = WanderState::LookingAround;
                    ai.wander_look_timer = 1.5; // Look around for 1.5 seconds

                    // If hit obstacle, find best direction
                    if hit_wall || outside_square {
                        ai.wander_direction = Self::find_most_open_direction(
                            rng,
                            pos,
                            &ai.spawn_position,
                            ai.movement_square_size,
                            walls,
                        );
                    }
                }
            }
            WanderState::LookingAround => {
                ai.wander_look_timer -= dt;
                if ai.wander_look_timer <= 0.0 {
                    // Done looking, wait before moving
                    ai.wander_state = WanderState::Waiting;
                    ai.wander_timer = random_range(rng, 1.0, 2.0);
                }
            }
            WanderState::Waiting => {
                ai.wander_timer -= dt;
                if ai.wander_timer <= 0.0 {
                    // Start moving in new direction
                    ai.wander_state = WanderState::Moving;
                    ai.wander_timer = random_range(rng, 1.0, 2.0);
                    ai.wander_direction = Self::find_most_open_direction(
                        rng,
                        pos,
                        &ai.spawn_position,
                        ai.movement_square_size,
                        walls,
                    );
                }
            }
        }
    }
}
