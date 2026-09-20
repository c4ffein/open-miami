//! Movement: from a bot's (already updated) state to a velocity + heading.
//! The only part of the AI that pathfinds.

use super::nav::{clear_path_cache, throttled_path_target, REPATH_INTERVAL};
use super::rng::*;
use super::senses::Percept;
use super::wander::{FERAL_LUNGE_SPEED_MULT, FERAL_RECOVER_SPEED_MULT};
use crate::collision::has_line_of_sight_with_padding;
use crate::components::{AIState, EnemyType, Position, UnsurePhase, WanderState, AI};
use crate::ecs::world::Wall;
use crate::ecs::{Entity, World};
use crate::pathfinding::NavigationGrid;
use std::f32::consts::PI;

/// Walls are inflated by this much (px) when deciding between walking
/// straight at a target and pathfinding: a target close to a wall is reached
/// through the pathfinder, which prevents wall grinding.
const WALL_PADDING: f32 = 25.0;

/// `(vx, vy, heading)`. A `None` heading = keep the current one (a parked
/// sentinel, a patroller pausing between legs, a bot standing on its target).
pub(super) type Steering = (f32, f32, Option<f32>);

const STAY: Steering = (0.0, 0.0, None);

/// The static level geometry the steering reads.
pub(super) struct Nav<'a> {
    pub walls: &'a [Wall],
    pub grid: &'a NavigationGrid,
}

pub(super) fn steer(
    world: &mut World,
    entity: Entity,
    ai: &AI,
    p: &Percept,
    speed: f32,
    nav: &Nav,
    rng: &mut u32,
    dt: f32,
) -> Steering {
    match ai.state {
        AIState::Unaware => routine(ai, speed),
        AIState::SpottedUnsure => {
            // Investigating: walk to where the player was last seen;
            // returning: back to where this bot stood when it spotted them.
            let target = match ai.unsure_phase {
                UnsurePhase::Returning => ai
                    .check_position
                    .or(ai.last_known_player_position)
                    .unwrap_or(p.player),
                _ => ai.last_known_player_position.unwrap_or(p.player),
            };
            walk_to(world, entity, p.pos, target, speed, nav, dt)
        }
        AIState::SurePlayerSeen if ai.initial_type == EnemyType::Wandering => {
            feral_charge(ai, p, speed)
        }
        AIState::SurePlayerSeen => {
            let target = if p.sees_player {
                p.player
            } else {
                ai.last_known_player_position.unwrap_or(p.player)
            };
            if p.sees_player && p.pos.distance_to(&target) < ai.attack_range {
                // Stop and face player when close enough and can see them
                let dx = p.player.x - p.pos.x;
                let dy = p.player.y - p.pos.y;
                (0.0, 0.0, Some(dy.atan2(dx)))
            } else {
                walk_to(world, entity, p.pos, target, speed, nav, dt)
            }
        }
        AIState::Confused => {
            // A fresh look picks a new heading; otherwise keep facing.
            let rot = if ai.confusion_look_timer == ai.confusion_look_duration {
                Some(random_range(rng, 0.0, PI * 2.0))
            } else {
                None
            };
            (0.0, 0.0, rot)
        }
        _ => STAY,
    }
}

/// Unaware: the sentinel stays parked; patrollers and drifters follow their
/// wander state (the look-around sweeps 70 degrees left, then 140 back right).
fn routine(ai: &AI, speed: f32) -> Steering {
    match ai.initial_type {
        EnemyType::Idle => STAY,
        EnemyType::Wandering | EnemyType::Patrolling => match ai.wander_state {
            WanderState::Moving => (
                ai.wander_direction.cos() * speed,
                ai.wander_direction.sin() * speed,
                Some(ai.wander_direction),
            ),
            WanderState::LookingAround => {
                let look_progress = 1.5 - ai.wander_look_timer;
                let rot = if look_progress < 0.5 {
                    let angle_offset = -(70.0 * PI / 180.0) * (look_progress / 0.5);
                    ai.wander_direction + angle_offset
                } else if look_progress < 1.5 {
                    let left_angle = ai.wander_direction - (70.0 * PI / 180.0);
                    let angle_offset = (140.0 * PI / 180.0) * ((look_progress - 0.5) / 1.0);
                    left_angle + angle_offset
                } else {
                    ai.wander_direction
                };
                (0.0, 0.0, Some(rot))
            }
            WanderState::Waiting => STAY,
        },
    }
}

/// Feral drifter: reckless straight-line lunge. It does NOT pathfind and does
/// NOT brake at weapon range — during a burst it charges along its locked
/// direction (into walls, past the player, whatever), and between bursts it
/// only creeps while re-aiming.
fn feral_charge(ai: &AI, p: &Percept, speed: f32) -> Steering {
    match ai.wander_state {
        WanderState::Moving => {
            let s = speed * FERAL_LUNGE_SPEED_MULT;
            (
                ai.wander_direction.cos() * s,
                ai.wander_direction.sin() * s,
                Some(ai.wander_direction),
            )
        }
        _ => {
            let target = ai.last_known_player_position.unwrap_or(p.player);
            toward(
                target.x - p.pos.x,
                target.y - p.pos.y,
                speed * FERAL_RECOVER_SPEED_MULT,
            )
        }
    }
}

/// Walk to `target`: straight at it with a clear (padded) line of sight,
/// otherwise along the throttled cached path.
fn walk_to(
    world: &mut World,
    entity: Entity,
    from: Position,
    target: Position,
    speed: f32,
    nav: &Nav,
    dt: f32,
) -> Steering {
    let has_clear_path =
        has_line_of_sight_with_padding(from.to_vec2(), target.to_vec2(), nav.walls, WALL_PADDING);
    let movement_target = if has_clear_path {
        clear_path_cache(world, entity);
        target.to_vec2()
    } else {
        throttled_path_target(
            world,
            entity,
            nav.grid,
            from.to_vec2(),
            target.to_vec2(),
            REPATH_INTERVAL,
            dt,
        )
    };
    toward(
        movement_target.x - from.x,
        movement_target.y - from.y,
        speed,
    )
}

/// Full speed along `(dx, dy)`, facing it; standing on the target = stay.
fn toward(dx: f32, dy: f32, speed: f32) -> Steering {
    let dist = (dx * dx + dy * dy).sqrt();
    if dist > 0.0 {
        ((dx / dist) * speed, (dy / dist) * speed, Some(dy.atan2(dx)))
    } else {
        STAY
    }
}
