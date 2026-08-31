//! DETACHED HEADS — the physics of a robot head kicked clean off.
//!
//! The KICK finisher (`systems::finisher`) decapitates its victim at the
//! impact frame: the corpse gets the [`Headless`] marker and a
//! [`DetachedHead`] entity spawns at the victim's head, launched along the
//! kick. The head then behaves like a small thrown object mirroring the
//! thrown-weapon / knockback physics: it slides with an initial speed,
//! spins with its motion, decelerates under floor friction, BOUNCES off
//! walls (normal component reflected + damped, tangential scrubbed — the
//! same model as `movement`'s knockback bounce), and finally comes to REST
//! on the floor, where it persists as a corpse detail (like ground weapons
//! persist). A fixed ring cap despawns the OLDEST head if a floor somehow
//! accumulates an absurd number of them.
//!
//! Everything here is host-testable: the spawn velocity, the friction
//! integration, the wall bounce and the rest condition run natively (unit
//! tests below); only the drawing (lib.rs / renderer.js `HEAD` opcode) is
//! browser-side.

use crate::collision::circle_rect_collision;
use crate::components::{DetachedHead, EnemyType, Position};
use crate::drive::hash01;
use crate::ecs::{Entity, System, World};
use crate::math::Vec2;

/// Launch speed of a kicked-off head (px/s) — a solid punt: faster than the
/// finisher knockback shove, slower than a thrown weapon's 700.
pub const KICK_HEAD_SPEED: f32 = 430.0;
/// Random aim jitter around the kick direction (radians, +-): the head never
/// flies perfectly straight off the neck.
pub const KICK_JITTER: f32 = 0.22;
/// Floor friction deceleration (px/s^2): from launch speed the head slides
/// ~`v^2 / (2 * friction)` ≈ 220 px and stops in about a second.
pub const HEAD_FRICTION: f32 = 420.0;
/// Collision radius of the head against walls (px).
pub const HEAD_RADIUS: f32 = 6.0;
/// Below this slide speed (px/s) the head is done: velocity snaps to zero
/// and it rests where it is.
pub const HEAD_REST_SPEED: f32 = 24.0;
/// Spin rate per unit of slide speed (rad per px): at launch ~430 px/s the
/// head tumbles ~2 turns/s, easing out as friction bleeds the slide.
pub const HEAD_SPIN_PER_PX: f32 = 0.030;
/// How much of the into-wall speed survives a bounce.
pub const HEAD_BOUNCE_RESTITUTION: f32 = 0.45;
/// How much of the along-wall speed survives a bounce (slight scrub).
pub const HEAD_BOUNCE_TANGENT: f32 = 0.85;
/// A head slower than this along the wall normal STOPS at the wall instead
/// of bouncing (kills the resting-jitter tail, like the knockback bounce).
pub const HEAD_BOUNCE_MIN_SPEED: f32 = 60.0;
/// Ring cap: most detached heads alive on a floor at once. One more and the
/// OLDEST (smallest `seq`) despawns.
pub const MAX_HEADS: usize = 12;
/// How far (px) from the victim's centre the head sits when it pops off —
/// roughly where the sprawled sprite's head is drawn.
pub const HEAD_NECK_OFFSET: f32 = 16.0;

/// Robot colour-table index for an enemy flavour (matches the sprite colours
/// lib.rs picks: SENTINEL red, DRIFTER violet, HUNTER magenta).
pub fn color_for(initial_type: EnemyType) -> u32 {
    match initial_type {
        EnemyType::Idle => 1,
        EnemyType::Wandering => 2,
        EnemyType::Patrolling => 3,
    }
}

/// The launch velocity + spin direction of a kicked-off head. `dir` is the
/// kick direction (player -> victim, any length); `seed` makes the jitter
/// deterministic — equal (dir, seed) always launches identically.
pub fn kick_velocity(dir: Vec2, seed: u32) -> (Vec2, f32) {
    let len = dir.length();
    let (dx, dy) = if len > f32::EPSILON {
        (dir.x / len, dir.y / len)
    } else {
        (1.0, 0.0)
    };
    // Deterministic jitter: a small swing either side of the kick line.
    let jitter = (hash01(seed, 0x4EAD) - 0.5) * 2.0 * KICK_JITTER;
    let (s, c) = jitter.sin_cos();
    let vx = (dx * c - dy * s) * KICK_HEAD_SPEED;
    let vy = (dx * s + dy * c) * KICK_HEAD_SPEED;
    let spin_dir = if hash01(seed, 0x5F17) < 0.5 {
        -1.0
    } else {
        1.0
    };
    (Vec2::new(vx, vy), spin_dir)
}

/// One friction step: bleed `HEAD_FRICTION * dt` off the speed, snapping to
/// a dead stop under [`HEAD_REST_SPEED`]. Returns the new velocity.
pub fn apply_friction(vx: f32, vy: f32, dt: f32) -> (f32, f32) {
    let speed = (vx * vx + vy * vy).sqrt();
    let new_speed = speed - HEAD_FRICTION * dt;
    if new_speed <= HEAD_REST_SPEED {
        (0.0, 0.0)
    } else {
        let k = new_speed / speed;
        (vx * k, vy * k)
    }
}

/// Is this head at rest (its slide fully spent)?
pub fn is_resting(head: &DetachedHead) -> bool {
    head.vx == 0.0 && head.vy == 0.0
}

/// Advance one head by `dt` against `walls`: sub-stepped integration (the
/// head can never jump a thin wall), wall clamp + bounce mirroring the
/// knockback model, then friction. Returns (x, y, vx, vy).
pub fn step_head(
    x: f32,
    y: f32,
    vx: f32,
    vy: f32,
    dt: f32,
    walls: &[crate::ecs::Wall],
) -> (f32, f32, f32, f32) {
    let mut x = x;
    let mut y = y;
    let mut vx = vx;
    let mut vy = vy;

    let speed = (vx * vx + vy * vy).sqrt();
    let steps = if speed * dt > HEAD_RADIUS {
        ((speed * dt / HEAD_RADIUS).ceil() as usize).clamp(1, 16)
    } else {
        1
    };
    let sub_dt = dt / steps as f32;

    for _ in 0..steps {
        x += vx * sub_dt;
        y += vy * sub_dt;
        for wall in walls {
            if !circle_rect_collision(
                Vec2::new(x, y),
                HEAD_RADIUS,
                wall.x,
                wall.y,
                wall.width,
                wall.height,
            ) {
                continue;
            }
            // Push out the nearest edge; reflect the normal component
            // (damped) when moving in fast, else stop dead at the wall.
            let wall_right = wall.x + wall.width;
            let wall_bottom = wall.y + wall.height;
            let dist_left = x - (wall.x - HEAD_RADIUS);
            let dist_right = (wall_right + HEAD_RADIUS) - x;
            let dist_top = y - (wall.y - HEAD_RADIUS);
            let dist_bottom = (wall_bottom + HEAD_RADIUS) - y;
            let min_dist = dist_left.min(dist_right).min(dist_top).min(dist_bottom);
            if min_dist == dist_left {
                x = wall.x - HEAD_RADIUS;
                if vx > 0.0 {
                    vx = if vx > HEAD_BOUNCE_MIN_SPEED {
                        -vx * HEAD_BOUNCE_RESTITUTION
                    } else {
                        0.0
                    };
                    vy *= HEAD_BOUNCE_TANGENT;
                }
            } else if min_dist == dist_right {
                x = wall_right + HEAD_RADIUS;
                if vx < 0.0 {
                    vx = if vx < -HEAD_BOUNCE_MIN_SPEED {
                        -vx * HEAD_BOUNCE_RESTITUTION
                    } else {
                        0.0
                    };
                    vy *= HEAD_BOUNCE_TANGENT;
                }
            } else if min_dist == dist_top {
                y = wall.y - HEAD_RADIUS;
                if vy > 0.0 {
                    vy = if vy > HEAD_BOUNCE_MIN_SPEED {
                        -vy * HEAD_BOUNCE_RESTITUTION
                    } else {
                        0.0
                    };
                    vx *= HEAD_BOUNCE_TANGENT;
                }
            } else {
                y = wall_bottom + HEAD_RADIUS;
                if vy < 0.0 {
                    vy = if vy < -HEAD_BOUNCE_MIN_SPEED {
                        -vy * HEAD_BOUNCE_RESTITUTION
                    } else {
                        0.0
                    };
                    vx *= HEAD_BOUNCE_TANGENT;
                }
            }
        }
    }

    let (vx, vy) = apply_friction(vx, vy, dt);
    (x, y, vx, vy)
}

/// Spawn a detached head at `at`, launched along `dir` (the kick), tinted
/// `color_idx`. `seed` drives the deterministic jitter/spin. Enforces the
/// [`MAX_HEADS`] ring cap by despawning the oldest head first.
pub fn spawn_head(world: &mut World, at: Vec2, dir: Vec2, color_idx: u32, seed: u32) -> Entity {
    // Ring cap: despawn oldest (smallest seq) while at capacity, and find
    // the next seq in the same pass.
    let heads = world.query::<DetachedHead>();
    let mut next_seq: u32 = 0;
    for &h in &heads {
        if let Some(d) = world.get_component::<DetachedHead>(h) {
            next_seq = next_seq.max(d.seq.wrapping_add(1));
        }
    }
    if heads.len() >= MAX_HEADS {
        let mut extra = heads.len() + 1 - MAX_HEADS;
        let mut with_seq: Vec<(u32, Entity)> = heads
            .iter()
            .filter_map(|&h| world.get_component::<DetachedHead>(h).map(|d| (d.seq, h)))
            .collect();
        with_seq.sort_by_key(|&(seq, _)| seq);
        for &(_, h) in with_seq.iter() {
            if extra == 0 {
                break;
            }
            world.despawn(h);
            extra -= 1;
        }
    }

    let (vel, spin_dir) = kick_velocity(dir, seed);
    let head = world.spawn();
    world.add_component(head, Position::new(at.x, at.y));
    world.add_component(
        head,
        DetachedHead {
            color_idx,
            vx: vel.x,
            vy: vel.y,
            // Start the sprite spun to the flight direction (face along it).
            spin: vel.y.atan2(vel.x),
            spin_dir,
            seq: next_seq,
            origin_x: at.x,
            origin_y: at.y,
        },
    );
    head
}

/// System that slides, spins, bounces and rests every [`DetachedHead`].
pub struct HeadSystem;

impl System for HeadSystem {
    fn run(&mut self, world: &mut World, dt: f32) {
        for head in world.query::<DetachedHead>() {
            let (Some(d), Some(p)) = (
                world.get_component::<DetachedHead>(head).copied(),
                world.get_component::<Position>(head).copied(),
            ) else {
                continue;
            };
            if is_resting(&d) {
                continue; // settled: a persistent corpse detail, zero work
            }
            let (x, y, vx, vy) = {
                let walls = world.walls();
                step_head(p.x, p.y, d.vx, d.vy, dt, walls)
            };
            let speed = (vx * vx + vy * vy).sqrt();
            if let Some(pos) = world.get_component_mut::<Position>(head) {
                pos.x = x;
                pos.y = y;
            }
            if let Some(dh) = world.get_component_mut::<DetachedHead>(head) {
                dh.vx = vx;
                dh.vy = vy;
                // Spin follows the slide: fast head tumbles, resting head
                // holds whatever angle it stopped at.
                dh.spin += dh.spin_dir * speed * HEAD_SPIN_PER_PX * dt;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run_to_rest(world: &mut World, head: Entity, max_secs: f32) -> f32 {
        let mut sys = HeadSystem;
        let dt = 0.016;
        let mut t = 0.0;
        while t < max_secs {
            sys.run(world, dt);
            t += dt;
            let d = world.get_component::<DetachedHead>(head).unwrap();
            if is_resting(d) {
                break;
            }
        }
        t
    }

    #[test]
    fn test_kick_velocity_is_deterministic_and_at_speed() {
        let dir = Vec2::new(0.0, 3.0); // any length: normalized inside
        let (v1, s1) = kick_velocity(dir, 7);
        let (v2, s2) = kick_velocity(dir, 7);
        assert_eq!((v1.x, v1.y, s1), (v2.x, v2.y, s2));
        // Launch speed is exact, direction within the jitter cone of +y.
        assert!((v1.length() - KICK_HEAD_SPEED).abs() < 0.01);
        let angle = v1.y.atan2(v1.x) - std::f32::consts::FRAC_PI_2;
        assert!(angle.abs() <= KICK_JITTER + 0.001, "angle {angle}");
        // Different seeds land on different jitters eventually.
        assert!((0..32).any(|s| kick_velocity(dir, s).0.x != v1.x));
    }

    #[test]
    fn test_friction_slides_head_to_rest_at_the_expected_distance() {
        let mut world = World::new();
        let head = spawn_head(
            &mut world,
            Vec2::new(100.0, 100.0),
            Vec2::new(1.0, 0.0),
            1,
            3,
        );
        let start = *world.get_component::<Position>(head).unwrap();
        let t = run_to_rest(&mut world, head, 3.0);
        let d = world.get_component::<DetachedHead>(head).unwrap();
        assert!(is_resting(d), "head still sliding after {t}s");
        let end = *world.get_component::<Position>(head).unwrap();
        let dist = ((end.x - start.x).powi(2) + (end.y - start.y).powi(2)).sqrt();
        // Analytic slide-out: v^2 / (2 * friction), generous discretization slack.
        let expected = KICK_HEAD_SPEED * KICK_HEAD_SPEED / (2.0 * HEAD_FRICTION);
        assert!(
            (dist - expected).abs() < expected * 0.15,
            "slid {dist}, expected ~{expected}"
        );
        // It rests and STAYS: further ticks move nothing.
        HeadSystem.run(&mut world, 0.016);
        let after = *world.get_component::<Position>(head).unwrap();
        assert_eq!((after.x, after.y), (end.x, end.y));
    }

    #[test]
    fn test_head_spins_while_sliding_and_stops_spinning_at_rest() {
        let mut world = World::new();
        let head = spawn_head(
            &mut world,
            Vec2::new(100.0, 100.0),
            Vec2::new(1.0, 0.0),
            1,
            3,
        );
        let spin0 = world.get_component::<DetachedHead>(head).unwrap().spin;
        HeadSystem.run(&mut world, 0.016);
        let spin1 = world.get_component::<DetachedHead>(head).unwrap().spin;
        assert!(spin0 != spin1, "a flying head must tumble");
        run_to_rest(&mut world, head, 3.0);
        let rest_spin = world.get_component::<DetachedHead>(head).unwrap().spin;
        HeadSystem.run(&mut world, 0.016);
        let d = world.get_component::<DetachedHead>(head).unwrap();
        assert_eq!(d.spin, rest_spin, "a resting head holds its angle");
    }

    #[test]
    fn test_head_bounces_off_a_wall_and_rests_clear_of_it() {
        let mut world = World::new();
        world.add_wall(200.0, 0.0, 40.0, 400.0); // wall to the right
        let head = spawn_head(
            &mut world,
            Vec2::new(100.0, 100.0),
            Vec2::new(1.0, 0.0),
            1,
            5,
        );
        // Kill the jitter for a clean axis check.
        {
            let d = world.get_component_mut::<DetachedHead>(head).unwrap();
            d.vx = KICK_HEAD_SPEED;
            d.vy = 0.0;
        }
        let mut sys = HeadSystem;
        let mut bounced = false;
        for _ in 0..200 {
            sys.run(&mut world, 0.016);
            let d = world.get_component::<DetachedHead>(head).unwrap();
            if d.vx < 0.0 {
                bounced = true; // reflected off the wall's left face
            }
            if is_resting(d) {
                break;
            }
        }
        assert!(bounced, "the head should bounce back off the wall");
        let end = world.get_component::<Position>(head).unwrap();
        assert!(
            end.x <= 200.0 - HEAD_RADIUS + 0.01,
            "resting inside the wall: x = {}",
            end.x
        );
    }

    #[test]
    fn test_slow_head_stops_at_the_wall_instead_of_jittering() {
        // Straight at the wall but under the bounce threshold: it parks.
        let walls = [crate::ecs::Wall::new(120.0, 0.0, 40.0, 400.0)];
        let (x, _y, vx, vy) = step_head(115.0, 50.0, 40.0, 0.0, 0.016, &walls);
        assert_eq!(vx, 0.0);
        assert_eq!(vy, 0.0);
        assert!(x <= 120.0 - HEAD_RADIUS + 0.01);
    }

    #[test]
    fn test_ring_cap_despawns_the_oldest_head() {
        let mut world = World::new();
        let mut spawned = Vec::new();
        for i in 0..(MAX_HEADS + 3) {
            let h = spawn_head(
                &mut world,
                Vec2::new(50.0 + i as f32, 50.0),
                Vec2::new(1.0, 0.0),
                1,
                i as u32,
            );
            spawned.push(h);
        }
        let alive = world.query::<DetachedHead>();
        assert_eq!(alive.len(), MAX_HEADS);
        // The three OLDEST are gone, the newest all survive.
        for &old in &spawned[..3] {
            assert!(!world.has_component::<DetachedHead>(old));
        }
        for &new in &spawned[3..] {
            assert!(world.has_component::<DetachedHead>(new));
        }
    }

    #[test]
    fn test_color_for_matches_the_sprite_table() {
        assert_eq!(color_for(EnemyType::Idle), 1);
        assert_eq!(color_for(EnemyType::Wandering), 2);
        assert_eq!(color_for(EnemyType::Patrolling), 3);
    }
}
