use crate::components::{
    AIState, DebugTrail, Enemy, Health, Position, Rotation, Speed, Stunned, Velocity, AI,
};
use crate::ecs::world::Wall;
use crate::ecs::{Entity, System, World};
use crate::pathfinding::NavigationGrid;

mod nav;
mod rng;
mod senses;
mod steer;
mod think;
mod wander;

pub(crate) use nav::*;
use steer::{steer, Nav};
use think::think;
use wander::FERAL_LUNGE_SELF_DAMAGE;

#[cfg(test)]
mod tests;

/// System that handles enemy AI behavior
#[derive(Default)]
pub struct AISystem {
    /// Cached navigation grid, rebuilt only when the wall layout changes.
    /// Building the grid scans every cell against every wall, far too much
    /// work to redo each tick for what is static level geometry.
    nav_cache: Option<(Vec<Wall>, NavigationGrid)>,
}

impl AISystem {
    /// The cached navigation grid + the wall list it was built from, rebuilt
    /// only when the walls changed (a level swap). Borrowed from `self`, not
    /// `world`, so it outlives the mutable component borrows of the enemy
    /// loop; the wall clone is only paid when the walls actually changed.
    fn nav(&mut self, world: &World) -> Nav<'_> {
        let walls_changed = match &self.nav_cache {
            Some((cached_walls, _)) => cached_walls.as_slice() != world.walls(),
            None => true,
        };
        if walls_changed {
            let walls = world.walls().to_vec();
            let grid = NavigationGrid::new(&walls);
            self.nav_cache = Some((walls, grid));
        }
        let (walls, grid) = self.nav_cache.as_ref().expect("filled just above");
        Nav { walls, grid }
    }
}

/// Dead and knocked-down bots stop where they are.
fn halt(world: &mut World, entity: Entity) {
    if let Some(velocity) = world.get_component_mut::<Velocity>(entity) {
        velocity.x = 0.0;
        velocity.y = 0.0;
    }
}

/// The debug overlay's movement trail: recorded for chasing bots only, and
/// only while the overlays are visible — headless sims and normal play skip
/// the bookkeeping entirely.
fn record_trail(world: &mut World, entity: Entity, chasing: bool, pos: Position) {
    if !world.debug_viz() {
        return;
    }
    if !chasing {
        if let Some(trail) = world.get_component_mut::<DebugTrail>(entity) {
            trail.clear();
        }
    } else if let Some(trail) = world.get_component_mut::<DebugTrail>(entity) {
        trail.add_position(pos.to_vec2());
    } else {
        let mut trail = DebugTrail::default();
        trail.add_position(pos.to_vec2());
        world.add_component(entity, trail);
    }
}

impl System for AISystem {
    fn run(&mut self, world: &mut World, dt: f32) {
        let Some(player_pos) = Self::find_player_position(world) else {
            return; // No player, nothing to do
        };

        // The world's RNG state is threaded through the update as a local (no
        // conflict with the component borrows) and written back at the end,
        // so the sequence continues across ticks.
        let mut rng = world.rng_state();
        let nav = self.nav(world);

        // Set when a passive bot took damage this tick: the crowd turns.
        let mut passive_hurt = false;

        for entity in world.query::<Enemy>() {
            let (enemy_pos, speed, health) = match (
                world.get_component::<Position>(entity),
                world.get_component::<Speed>(entity),
                world.get_component::<Health>(entity),
            ) {
                (Some(pos), Some(spd), Some(hp)) => (*pos, *spd, *hp),
                _ => continue,
            };

            // Passive (civilian) bots have their own brain: no vision, no
            // aggro. Being hurt — or downed outright — flips the whole crowd
            // (after the loop, once the borrows are released), so this runs
            // before the dead / stunned skips.
            if world
                .get_component::<AI>(entity)
                .is_some_and(|ai| ai.state == AIState::Passive)
            {
                passive_hurt |= crate::systems::passive::update_passive(
                    world, entity, &mut rng, nav.grid, nav.walls, dt,
                );
                continue;
            }

            // The dead don't think; neither do the knocked-down.
            if health.is_dead() || world.has_component::<Stunned>(entity) {
                halt(world, entity);
                continue;
            }

            // 1. PERCEIVE, 2. THINK (the state machine), 3. STEER (movement).
            let Some(percept) = Self::perceive(world, entity, enemy_pos, player_pos, nav.walls)
            else {
                continue;
            };
            let Some(ai) = world.get_component_mut::<AI>(entity) else {
                continue;
            };
            let feral_lunge_completed = think(ai, &percept, &mut rng, nav.walls, dt);
            let ai = *ai;

            // A feral that just finished a lunge burns a chip of itself out.
            if feral_lunge_completed {
                if let Some(hp) = world.get_component_mut::<Health>(entity) {
                    hp.take_damage(FERAL_LUNGE_SELF_DAMAGE);
                }
            }

            let (vx, vy, heading) = steer(
                world,
                entity,
                &ai,
                &percept,
                speed.value,
                &nav,
                &mut rng,
                dt,
            );
            if let Some(velocity) = world.get_component_mut::<Velocity>(entity) {
                velocity.x = vx;
                velocity.y = vy;
            }
            if let Some(heading) = heading {
                if let Some(rotation) = world.get_component_mut::<Rotation>(entity) {
                    rotation.angle = heading;
                }
            }

            let chasing = matches!(ai.state, AIState::SpottedUnsure | AIState::SurePlayerSeen);
            record_trail(world, entity, chasing, enemy_pos);
        }

        // Persist the advanced RNG state back into the world so the next tick
        // continues the same deterministic sequence.
        world.set_rng_state(rng);

        if passive_hurt {
            crate::systems::passive::alert_passives(world, crate::scenario::AlertTarget::All);
        }
    }
}
