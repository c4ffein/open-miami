//! What a bot can perceive: where the player is, the vision cone.

use super::AISystem;
use crate::collision::has_line_of_sight;
use crate::components::{Player, Position, Rotation, AI};
use crate::ecs::world::Wall;
use crate::ecs::{Entity, World};
use std::f32::consts::PI;

/// What one bot knows this tick: computed once, read by `think` and `steer`.
pub(super) struct Percept {
    /// Where the bot is.
    pub pos: Position,
    /// Where the player is (the bot only ACTS on it through `sees_player` /
    /// its last known position).
    pub player: Position,
    /// Line of sight + within detection range + inside the vision cone.
    pub sees_player: bool,
}

impl AISystem {
    /// `None` = the entity has no [`AI`] to think with.
    pub(super) fn perceive(
        world: &World,
        entity: Entity,
        pos: Position,
        player: Position,
        walls: &[Wall],
    ) -> Option<Percept> {
        let detection_range = world.get_component::<AI>(entity)?.detection_range;
        let distance = pos.distance_to(&player);
        let has_los = has_line_of_sight(pos.to_vec2(), player.to_vec2(), walls);
        let rotation = world
            .get_component::<Rotation>(entity)
            .map(|r| r.angle)
            .unwrap_or(0.0);
        let in_vision_cone = Self::is_within_vision_cone(&pos, &player, rotation);
        Some(Percept {
            pos,
            player,
            sees_player: has_los && distance < detection_range && in_vision_cone,
        })
    }

    pub(super) fn find_player_position(world: &World) -> Option<Position> {
        world
            .first::<Player>()
            .and_then(|entity| world.get_component::<Position>(entity))
            .copied()
    }

    /// Check if target position is within the vision cone
    /// Vision cone is 90 degrees (PI/2), so 45 degrees on each side of facing direction
    pub(super) fn is_within_vision_cone(
        enemy_pos: &Position,
        target_pos: &Position,
        enemy_rotation: f32,
    ) -> bool {
        // Calculate angle from enemy to target
        let dx = target_pos.x - enemy_pos.x;
        let dy = target_pos.y - enemy_pos.y;
        let angle_to_target = dy.atan2(dx);

        // Calculate angle difference
        let mut angle_diff = angle_to_target - enemy_rotation;

        // Normalize angle difference to [-PI, PI]
        while angle_diff > PI {
            angle_diff -= 2.0 * PI;
        }
        while angle_diff < -PI {
            angle_diff += 2.0 * PI;
        }

        // Check if within 90-degree cone (45 degrees on each side)
        let cone_half_angle = PI / 4.0; // 45 degrees
        angle_diff.abs() <= cone_half_angle
    }
}
