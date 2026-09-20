//! The scenario's hands on the `World`: spawning from defs, counting rogues,
//! the gate tether, the floor's markers.

use super::defs::*;
use crate::components::{Boss, Elevator, Health, Zone};
use crate::ecs::World;
use crate::game::spawn_enemy_with_type;
use crate::math::Vec2;

/// How far from the gate's anchor the player may wander while a tutorial
/// gate holds ([`tether_player`]'s invisible walls).
pub const GATE_TETHER_RADIUS: f32 = 180.0;

/// Invisible walls while a gate holds: clamp the player inside the circle
/// (`anchor`, `radius`). Call after movement resolved (browser loop and the
/// headless sim share it).
pub fn tether_player(world: &mut World, anchor: Vec2, radius: f32) {
    let player = match world.query::<crate::components::Player>().first() {
        Some(&p) => p,
        None => return,
    };
    if let Some(pos) = world.get_component_mut::<crate::components::Position>(player) {
        let dx = pos.x - anchor.x;
        let dy = pos.y - anchor.y;
        let d = (dx * dx + dy * dy).sqrt();
        if d > radius {
            pos.x = anchor.x + dx / d * radius;
            pos.y = anchor.y + dy / d * radius;
        }
    }
}

/// The position of the stunned (downed) enemy nearest to the player (a
/// `finish` gate's anchor — its victim went down in an earlier step).
pub(super) fn nearest_stunned_to_player(world: &World) -> Option<Vec2> {
    let player = *world.query::<crate::components::Player>().first()?;
    let p = *world.get_component::<crate::components::Position>(player)?;
    world
        .query::<crate::components::Stunned>()
        .into_iter()
        .filter(|&e| world.has_component::<crate::components::Enemy>(e))
        .filter_map(|e| world.get_component::<crate::components::Position>(e))
        .map(|q| {
            (
                Vec2::new(q.x, q.y),
                (q.x - p.x).powi(2) + (q.y - p.y).powi(2),
            )
        })
        .min_by(|a, b| a.1.total_cmp(&b.1))
        .map(|(v, _)| v)
}

/// Spawn one placement: a hostile rogue of `kind`, or a passive bot when
/// `def.passive` (see `systems::passive`).
pub fn spawn_from_def(world: &mut World, def: &SpawnDef) -> crate::ecs::Entity {
    if def.passive {
        crate::systems::passive::spawn_passive(world, def)
    } else {
        let e = spawn_enemy_with_type(world, Vec2::new(def.x, def.y), def.kind);
        if def.unarmed {
            world.remove_component::<crate::components::Weapon>(e);
        }
        e
    }
}

/// `(dead, alive)` rogue counts on the floor (every `Enemy`, boss included).
pub fn count_rogues(world: &World) -> (usize, usize) {
    let mut dead = 0;
    let mut alive = 0;
    for entity in world.query::<crate::components::Enemy>() {
        if crate::systems::passive::is_passive(world, entity) {
            // An un-alerted civilian is a bystander, not a rogue: it must not
            // hold `all_dead` / `kills` hostage (nor pad the HUD count).
            continue;
        }
        match world.get_component::<Health>(entity) {
            Some(h) if h.is_alive() => alive += 1,
            Some(_) => dead += 1,
            None => {}
        }
    }
    (dead, alive)
}

/// Whether the floor has a boss and it is dead (`boss_dead` trigger).
pub fn any_boss_dead(world: &World) -> bool {
    world.query::<Boss>().iter().any(|&e| {
        world
            .get_component::<Health>(e)
            .map(|h| h.is_dead())
            .unwrap_or(false)
    })
}

/// Spawn the entry + exit elevators and the trigger zones of a floor into the
/// world (as entities carrying [`Elevator`] / [`Zone`] components).
pub fn spawn_floor_markers(world: &mut World, floor: &'static FloorDef) {
    let e = world.spawn();
    world.add_component(e, Elevator::from_def(&floor.entry, false));
    for exit in floor.exits {
        let e = world.spawn();
        world.add_component(e, Elevator::from_def(exit, true));
    }
    for zone in floor.zones {
        let e = world.spawn();
        world.add_component(e, Zone::from_def(zone));
    }
}
