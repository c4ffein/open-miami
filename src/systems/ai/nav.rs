//! The throttled path follower shared by the chasers and the passive crowd.

use crate::components::{DebugPath, NavPath};
use crate::ecs::{Entity, World};
use crate::math::Vec2;
use crate::pathfinding::{GridCoord, NavigationGrid};

// --- Pathfinding throttle -----------------------------------------------------
// A* is far too expensive to run per chaser per tick (an unreachable target —
// e.g. the player tucked behind a wall — floods the whole grid before failing).
// Chasers recompute their path at ~5 Hz, staggered per entity so a crowd
// alerted on the same tick doesn't recompute in lockstep; between recomputes
// they keep following the cached waypoints. A recompute is forced early when
// the target has drifted away from the cached path's goal or the cached path
// has been walked to its end.

/// Base seconds between path recomputes for a chaser (~5 Hz).
pub(crate) const REPATH_INTERVAL: f32 = 0.2;
/// Base seconds between path recomputes for a strolling passive (its target
/// is a fixed point in a zone, so ~1.5 Hz is plenty).
pub(crate) const PASSIVE_REPATH_INTERVAL: f32 = 0.7;
/// Recompute immediately when the target is this far from the cached goal.
const REPATH_TARGET_DRIFT: f32 = 80.0;
/// Per-entity stagger: entity id modulo this many slots...
const REPATH_STAGGER_SLOTS: u64 = 8;
/// ...times this much extra delay per slot (one 60 Hz tick).
const REPATH_STAGGER_STEP: f32 = 1.0 / 60.0;
/// A cached waypoint counts as reached within this distance (px); the
/// follower then advances to the next one.
const WAYPOINT_ARRIVE: f32 = 20.0;

/// The movement target for an entity whose padded line of sight to `target`
/// is blocked: follow the cached [`NavPath`], recomputing it only on the
/// throttle described above. Writes the [`DebugPath`] visualization only when
/// the world's debug flag is on ([`World::debug_viz`]).
pub(crate) fn throttled_path_target(
    world: &mut World,
    entity: Entity,
    nav_grid: &NavigationGrid,
    here: Vec2,
    target: Vec2,
    interval: f32,
    dt: f32,
) -> Vec2 {
    let debug_viz = world.debug_viz();
    if !world.has_component::<NavPath>(entity) {
        // Fresh cache: timer 0 forces an immediate first compute below.
        world.add_component(entity, NavPath::default());
    }
    let (waypoint, debug_waypoints) = {
        let stagger = (entity.0 % REPATH_STAGGER_SLOTS) as f32 * REPATH_STAGGER_STEP;
        let cache = world.get_component_mut::<NavPath>(entity).unwrap();
        cache.timer -= dt;
        // Walked the cached path to its end but LOS is still blocked.
        let exhausted = !cache.waypoints.is_empty() && cache.next >= cache.waypoints.len();
        let drifted = cache.target.distance(target) > REPATH_TARGET_DRIFT;
        if cache.timer <= 0.0 || drifted || exhausted {
            // An unreachable target leaves the waypoints empty; the throttle
            // then also rate-limits the (worst-case, full-flood) retries.
            cache.waypoints = nav_grid.find_path(here, target).unwrap_or_default();
            cache.target = target;
            cache.next = 0;
            cache.timer = interval + stagger;
            cache.recomputes = cache.recomputes.saturating_add(1);
        }
        // Follow the cached path: consume waypoints we're standing in / on.
        let here_cell = GridCoord::from_world_pos(here.x, here.y);
        while let Some(wp) = cache.waypoints.get(cache.next) {
            let reached = GridCoord::from_world_pos(wp.x, wp.y) == here_cell
                || wp.distance(here) < WAYPOINT_ARRIVE;
            if reached {
                cache.next += 1;
            } else {
                break;
            }
        }
        let waypoint = match cache.waypoints.get(cache.next) {
            Some(wp) => *wp,
            // No path (unreachable target) or path fully walked: head
            // straight at the target (the previous behavior); an exhausted
            // path forces a recompute on the next tick.
            None => target,
        };
        let debug_waypoints =
            debug_viz.then(|| cache.waypoints[cache.next.min(cache.waypoints.len())..].to_vec());
        (waypoint, debug_waypoints)
    };
    if let Some(waypoints) = debug_waypoints {
        if waypoints.is_empty() {
            if let Some(dp) = world.get_component_mut::<DebugPath>(entity) {
                *dp = DebugPath::clear();
            }
        } else if let Some(dp) = world.get_component_mut::<DebugPath>(entity) {
            *dp = DebugPath::new(waypoints, target);
        } else {
            world.add_component(entity, DebugPath::new(waypoints, target));
        }
    }
    waypoint
}

/// Called when an entity has clear (padded) line of sight and moves directly:
/// expire its cached path so the next blocked tick recomputes immediately,
/// and clear the debug overlay.
pub(crate) fn clear_path_cache(world: &mut World, entity: Entity) {
    if let Some(cache) = world.get_component_mut::<NavPath>(entity) {
        cache.waypoints.clear();
        cache.timer = 0.0;
    }
    if let Some(dp) = world.get_component_mut::<DebugPath>(entity) {
        if !dp.waypoints.is_empty() {
            *dp = DebugPath::clear();
        }
    }
}
