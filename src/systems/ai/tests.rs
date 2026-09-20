#[test]
fn ai_coin_flip_has_entropy() {
    let mut rng = 12345u32;
    let flips: Vec<bool> = (0..2000).map(|_| super::rng::coin_flip(&mut rng)).collect();
    let same = flips.windows(2).filter(|p| p[0] == p[1]).count();
    assert!(
        (800..1200).contains(&same),
        "consecutive equal flips: {same}"
    );
    let looks: Vec<i32> = (0..2000)
        .map(|_| super::rng::random_int_range(&mut rng, 2, 3))
        .collect();
    let same = looks.windows(2).filter(|p| p[0] == p[1]).count();
    assert!(
        (800..1200).contains(&same),
        "consecutive equal looks: {same}"
    );
}

use super::*;
use crate::components::{DebugPath, EnemyType, NavPath, Player, UnsurePhase, WanderState};
use crate::math::Vec2;

/// A bare enemy of `kind` at `at`, facing +x, plus the player at
/// `player_at`. Returns (player, enemy).
fn spot_world(kind: EnemyType, at: Position, player_at: Position) -> (World, Entity, Entity) {
    let mut world = World::new();
    let player = world.spawn();
    world.add_component(player, Player);
    world.add_component(player, player_at);
    let enemy = world.spawn();
    world.add_component(enemy, Enemy);
    world.add_component(enemy, at);
    world.add_component(enemy, AI::new_with_type(kind, at));
    world.add_component(enemy, Velocity::zero());
    world.add_component(enemy, Speed::new(200.0));
    world.add_component(enemy, Health::new(100));
    world.add_component(enemy, Rotation::new(0.0));
    (world, player, enemy)
}

fn step(world: &mut World, ai: &mut AISystem, dt: f32) {
    ai.run(world, dt);
    crate::systems::MovementSystem.run(world, dt);
}

#[test]
fn spotted_unsure_investigates_then_returns_to_post() {
    // Regression: losing sight used to leave the bot parked at the last
    // known position in SpottedUnsure forever (its only exit was being
    // back at `check_position`, which nothing ever walked it to).
    let post = Position::new(0.0, 0.0);
    let (mut world, player, enemy) = spot_world(EnemyType::Idle, post, Position::new(200.0, 0.0));
    let mut sys = AISystem::default();
    let dt = 1.0 / 60.0;
    // A glimpse shorter than `spot_duration`.
    for _ in 0..5 {
        step(&mut world, &mut sys, dt);
    }
    {
        let ai = world.get_component::<AI>(enemy).unwrap();
        assert_eq!(ai.state, AIState::SpottedUnsure);
        assert_eq!(ai.unsure_phase, UnsurePhase::Spotting);
        assert_eq!(ai.last_known_player_position.map(|p| p.x), Some(200.0));
    }
    // The player vanishes (out of detection range).
    *world.get_component_mut::<Position>(player).unwrap() = Position::new(200.0, 5000.0);
    step(&mut world, &mut sys, dt);
    {
        let ai = world.get_component::<AI>(enemy).unwrap();
        assert_eq!(ai.state, AIState::SpottedUnsure);
        assert_eq!(ai.unsure_phase, UnsurePhase::Investigating);
    }
    // It walks to where the player was last seen…
    let mut reached = false;
    for _ in 0..180 {
        step(&mut world, &mut sys, dt);
        let ai = world.get_component::<AI>(enemy).unwrap();
        if ai.unsure_phase == UnsurePhase::Returning {
            reached = true;
            break;
        }
        let v = world.get_component::<Velocity>(enemy).unwrap();
        assert!(
            v.x > 0.0,
            "investigating: heading toward the last known position"
        );
    }
    assert!(reached, "reached the last known position within 3 s");
    let x = world.get_component::<Position>(enemy).unwrap().x;
    assert!(x > 150.0, "walked most of the way there (x = {x})");
    // …then back to its post, and resumes its routine.
    let mut home = false;
    for _ in 0..180 {
        step(&mut world, &mut sys, dt);
        let ai = world.get_component::<AI>(enemy).unwrap();
        if ai.state == AIState::Unaware {
            home = true;
            break;
        }
        let v = world.get_component::<Velocity>(enemy).unwrap();
        assert!(v.x < 0.0, "returning: heading back to the post");
    }
    assert!(home, "back to Unaware within 3 s");
    let ai = world.get_component::<AI>(enemy).unwrap();
    assert!(ai.check_position.is_none());
    let x = world.get_component::<Position>(enemy).unwrap().x;
    assert!(x < 10.0, "back at the post (x = {x})");
}

#[test]
fn spotted_unsure_gives_up_the_investigation_after_its_patience() {
    // An unreachable last-known position must not trap the bot: the
    // `unsure_check_duration` budget sends it home.
    let post = Position::new(0.0, 0.0);
    let (mut world, player, enemy) = spot_world(EnemyType::Idle, post, Position::new(200.0, 0.0));
    // Pin the bot: with no speed it can never reach the spot.
    world.get_component_mut::<Speed>(enemy).unwrap().value = 0.0;
    let mut sys = AISystem::default();
    let dt = 1.0 / 60.0;
    for _ in 0..5 {
        step(&mut world, &mut sys, dt);
    }
    *world.get_component_mut::<Position>(player).unwrap() = Position::new(200.0, 5000.0);
    // 2 s investigating + 2 s returning (both time out where it stands).
    for _ in 0..260 {
        step(&mut world, &mut sys, dt);
    }
    let ai = world.get_component::<AI>(enemy).unwrap();
    assert_eq!(
        ai.state,
        AIState::Unaware,
        "gave up and resumed the routine"
    );
}

#[test]
fn patroller_keeps_its_heading_while_pausing() {
    // Regression: `0.0` doubled as "keep rotation" and as a real heading,
    // so a HUNTER pausing between patrol legs snapped to face east.
    let post = Position::new(0.0, 0.0);
    let (mut world, _player, enemy) =
        spot_world(EnemyType::Patrolling, post, Position::new(5000.0, 5000.0));
    world.get_component_mut::<Rotation>(enemy).unwrap().angle = 1.0;
    {
        let ai = world.get_component_mut::<AI>(enemy).unwrap();
        ai.wander_state = WanderState::Waiting;
        ai.wander_timer = 5.0;
    }
    let mut sys = AISystem::default();
    for _ in 0..10 {
        step(&mut world, &mut sys, 1.0 / 60.0);
    }
    let ai = world.get_component::<AI>(enemy).unwrap();
    assert_eq!(ai.state, AIState::Unaware);
    assert_eq!(ai.wander_state, WanderState::Waiting);
    let rot = world.get_component::<Rotation>(enemy).unwrap().angle;
    assert_eq!(rot, 1.0, "a pausing patroller keeps facing where it looked");
}

#[test]
fn test_ai_system_chase_when_player_in_range() {
    let mut world = World::new();

    // Create player
    let player = world.spawn();
    world.add_component(player, Player);
    world.add_component(player, Position::new(200.0, 0.0));

    // Create enemy
    let enemy = world.spawn();
    world.add_component(enemy, Enemy);
    let enemy_pos = Position::new(0.0, 0.0);
    world.add_component(enemy, enemy_pos);
    world.add_component(enemy, AI::new_with_type(EnemyType::Idle, enemy_pos));
    world.add_component(enemy, Velocity::zero());
    world.add_component(enemy, Speed::new(100.0));
    world.add_component(enemy, Health::new(100));
    world.add_component(enemy, Rotation::new(0.0));

    let mut system = AISystem::default();
    // Run multiple frames to trigger state transitions (need > 0.3s to go from Unaware -> SpottedUnsure -> SurePlayerSeen)
    for _ in 0..30 {
        system.run(&mut world, 0.016);
    }

    let ai = world.get_component::<AI>(enemy).unwrap();
    assert_eq!(ai.state, AIState::SurePlayerSeen);

    let velocity = world.get_component::<Velocity>(enemy).unwrap();
    assert!(velocity.x > 0.0); // Moving toward player
}

#[test]
fn test_ai_system_attack_when_player_close() {
    let mut world = World::new();

    // Create player
    let player = world.spawn();
    world.add_component(player, Player);
    world.add_component(player, Position::new(30.0, 0.0));

    // Create enemy
    let enemy = world.spawn();
    world.add_component(enemy, Enemy);
    let enemy_pos = Position::new(0.0, 0.0);
    world.add_component(enemy, enemy_pos);
    world.add_component(enemy, AI::new_with_type(EnemyType::Idle, enemy_pos));
    world.add_component(enemy, Velocity::zero());
    world.add_component(enemy, Speed::new(100.0));
    world.add_component(enemy, Health::new(100));
    world.add_component(enemy, Rotation::new(0.0));

    let mut system = AISystem::default();
    // Run multiple frames to trigger state transitions
    for _ in 0..30 {
        system.run(&mut world, 0.016);
    }

    let ai = world.get_component::<AI>(enemy).unwrap();
    assert_eq!(ai.state, AIState::SurePlayerSeen);

    let velocity = world.get_component::<Velocity>(enemy).unwrap();
    assert_eq!(velocity.x, 0.0); // Stopped to attack
    assert_eq!(velocity.y, 0.0);
}

#[test]
fn test_ai_system_idle_when_player_far() {
    let mut world = World::new();

    // Create player
    let player = world.spawn();
    world.add_component(player, Player);
    world.add_component(player, Position::new(1000.0, 0.0)); // Far away (beyond 900 detection range)

    // Create enemy
    let enemy = world.spawn();
    world.add_component(enemy, Enemy);
    world.add_component(enemy, Position::new(0.0, 0.0));
    world.add_component(enemy, AI::new());
    world.add_component(enemy, Velocity::zero());
    world.add_component(enemy, Speed::new(100.0));
    world.add_component(enemy, Health::new(100));

    let mut system = AISystem::default();
    system.run(&mut world, 0.016);

    let ai = world.get_component::<AI>(enemy).unwrap();
    assert_eq!(ai.state, AIState::Unaware); // Changed from Idle to Unaware

    let velocity = world.get_component::<Velocity>(enemy).unwrap();
    assert_eq!(velocity.x, 0.0);
    assert_eq!(velocity.y, 0.0);
}

#[test]
fn test_ai_system_updates_attack_timer() {
    let mut world = World::new();

    let player = world.spawn();
    world.add_component(player, Player);
    world.add_component(player, Position::new(0.0, 0.0));

    let enemy = world.spawn();
    world.add_component(enemy, Enemy);
    world.add_component(enemy, Position::new(0.0, 0.0));
    let mut ai = AI::new();
    ai.reset_attack_timer();
    world.add_component(enemy, ai);
    world.add_component(enemy, Velocity::zero());
    world.add_component(enemy, Speed::new(100.0));
    world.add_component(enemy, Health::new(100));

    let mut system = AISystem::default();
    system.run(&mut world, 0.5);

    let ai = world.get_component::<AI>(enemy).unwrap();
    assert!((ai.attack_timer - 0.5).abs() < 0.001);
}

#[test]
fn test_ai_system_multiple_enemies() {
    let mut world = World::new();

    let player = world.spawn();
    world.add_component(player, Player);
    world.add_component(player, Position::new(100.0, 0.0));

    // Create 3 enemies at different distances
    for i in 0..3 {
        let enemy = world.spawn();
        world.add_component(enemy, Enemy);
        world.add_component(enemy, Position::new(i as f32 * 50.0, 0.0));
        world.add_component(enemy, AI::new());
        world.add_component(enemy, Velocity::zero());
        world.add_component(enemy, Speed::new(100.0));
        world.add_component(enemy, Health::new(100));
    }

    let mut system = AISystem::default();
    system.run(&mut world, 0.016);

    // All enemies should have updated AI states
    let enemies: Vec<_> = world.query::<Enemy>();
    assert_eq!(enemies.len(), 3);
}

#[test]
fn test_ai_pathfinding_no_obstacles() {
    use crate::ecs::world::World;

    let mut world = World::new();

    // Create player (within detection range)
    let player = world.spawn();
    world.add_component(player, Player);
    world.add_component(player, Position::new(300.0, 100.0));

    // Create enemy
    let enemy = world.spawn();
    world.add_component(enemy, Enemy);
    let enemy_pos = Position::new(100.0, 100.0);
    world.add_component(enemy, enemy_pos);
    world.add_component(enemy, AI::new_with_type(EnemyType::Idle, enemy_pos));
    world.add_component(enemy, Velocity::zero());
    world.add_component(enemy, Speed::new(100.0));
    world.add_component(enemy, Health::new(100));
    world.add_component(enemy, Rotation::new(0.0));

    let mut system = AISystem::default();
    // Run multiple frames for state transition
    for _ in 0..30 {
        system.run(&mut world, 0.016);
    }

    // Enemy should be chasing player (SurePlayerSeen state)
    let ai = world.get_component::<AI>(enemy).unwrap();
    assert_eq!(ai.state, AIState::SurePlayerSeen);

    let velocity = world.get_component::<Velocity>(enemy).unwrap();
    // Should have non-zero velocity
    assert!(velocity.x.abs() > 0.0 || velocity.y.abs() > 0.0);

    // Velocity should point directly toward player (with direct line-of-sight targeting)
    assert!(velocity.x > 0.0); // Moving right toward player
                               // Both at y=100, so no vertical movement expected with direct targeting
    assert!(velocity.y.abs() < 1.0); // Should be nearly zero
}

#[test]
fn test_ai_pathfinding_with_wall_obstacle() {
    use crate::ecs::world::World;

    let mut world = World::new();

    // Add a vertical wall between enemy and player
    world.add_wall(250.0, 0.0, 20.0, 300.0);

    // Create player on right side of wall (within detection range but no line of sight)
    let player = world.spawn();
    world.add_component(player, Player);
    world.add_component(player, Position::new(350.0, 150.0));

    // Create enemy on left side of wall
    let enemy = world.spawn();
    world.add_component(enemy, Enemy);
    world.add_component(enemy, Position::new(100.0, 150.0));
    world.add_component(enemy, AI::new());
    world.add_component(enemy, Velocity::zero());
    world.add_component(enemy, Speed::new(100.0));
    world.add_component(enemy, Health::new(100));

    let mut system = AISystem::default();
    system.run(&mut world, 0.016);

    // Enemy should NOT detect player (line of sight blocked by wall)
    let ai = world.get_component::<AI>(enemy).unwrap();
    assert_eq!(ai.state, AIState::Unaware);

    // Enemy should not be moving
    let velocity = world.get_component::<Velocity>(enemy).unwrap();
    assert_eq!(velocity.x, 0.0);
    assert_eq!(velocity.y, 0.0);
}

#[test]
fn test_ai_pathfinding_around_corner() {
    use crate::ecs::world::World;

    let mut world = World::new();

    // Create L-shaped wall
    world.add_wall(200.0, 200.0, 400.0, 20.0); // Horizontal wall
    world.add_wall(200.0, 200.0, 20.0, 200.0); // Vertical wall

    // Create player in the corner (line of sight blocked by walls)
    let player = world.spawn();
    world.add_component(player, Player);
    world.add_component(player, Position::new(100.0, 300.0));

    // Create enemy outside the corner (within detection range but no line of sight)
    let enemy = world.spawn();
    world.add_component(enemy, Enemy);
    world.add_component(enemy, Position::new(300.0, 100.0));
    world.add_component(enemy, AI::new());
    world.add_component(enemy, Velocity::zero());
    world.add_component(enemy, Speed::new(100.0));
    world.add_component(enemy, Health::new(100));

    let mut system = AISystem::default();
    system.run(&mut world, 0.016);

    // Enemy should NOT detect player (line of sight blocked by walls)
    let ai = world.get_component::<AI>(enemy).unwrap();
    assert_eq!(ai.state, AIState::Unaware);

    // Enemy should not be moving
    let velocity = world.get_component::<Velocity>(enemy).unwrap();
    assert_eq!(velocity.x, 0.0);
    assert_eq!(velocity.y, 0.0);
}

#[test]
fn test_ai_pathfinding_multiple_walls() {
    use crate::ecs::world::World;

    let mut world = World::new();

    // Create a maze of walls
    world.add_wall(200.0, 0.0, 20.0, 300.0);
    world.add_wall(400.0, 200.0, 20.0, 400.0);

    // Create player (within detection range but line of sight blocked by walls)
    let player = world.spawn();
    world.add_component(player, Player);
    world.add_component(player, Position::new(350.0, 250.0));

    // Create enemy
    let enemy = world.spawn();
    world.add_component(enemy, Enemy);
    world.add_component(enemy, Position::new(100.0, 100.0));
    world.add_component(enemy, AI::new());
    world.add_component(enemy, Velocity::zero());
    world.add_component(enemy, Speed::new(100.0));
    world.add_component(enemy, Health::new(100));

    let mut system = AISystem::default();
    system.run(&mut world, 0.016);

    // Enemy should NOT detect player (line of sight blocked by walls)
    let ai = world.get_component::<AI>(enemy).unwrap();
    assert_eq!(ai.state, AIState::Unaware);

    // Enemy should not be moving
    let velocity = world.get_component::<Velocity>(enemy).unwrap();
    assert_eq!(velocity.x, 0.0);
    assert_eq!(velocity.y, 0.0);
}

#[test]
fn test_ai_pathfinding_enemy_follows_over_time() {
    use crate::ecs::world::World;

    let mut world = World::new();

    // Create player (within detection range)
    let player = world.spawn();
    world.add_component(player, Player);
    world.add_component(player, Position::new(300.0, 100.0));

    // Create enemy
    let enemy = world.spawn();
    world.add_component(enemy, Enemy);
    world.add_component(enemy, Position::new(100.0, 100.0));
    world.add_component(enemy, AI::new());
    world.add_component(enemy, Velocity::zero());
    world.add_component(enemy, Speed::new(100.0));
    world.add_component(enemy, Health::new(100));

    // Get initial distance
    let initial_pos = *world.get_component::<Position>(enemy).unwrap();
    let player_pos_clone = *world.get_component::<Position>(player).unwrap();
    let initial_distance = initial_pos.distance_to(&player_pos_clone);

    let mut system = AISystem::default();

    // Simulate multiple frames
    for _ in 0..10 {
        system.run(&mut world, 0.016);

        // Apply velocity to position (simulate movement system)
        let enemies: Vec<_> = world.query::<Enemy>();
        for entity in enemies {
            // Get velocity first, then update position
            let vel = *world.get_component::<Velocity>(entity).unwrap();
            if let Some(pos) = world.get_component_mut::<Position>(entity) {
                pos.x += vel.x * 0.016;
                pos.y += vel.y * 0.016;
            }
        }
    }

    // Enemy should be closer to player
    let final_pos = world.get_component::<Position>(enemy).unwrap();
    let final_distance = final_pos.distance_to(&player_pos_clone);

    assert!(final_distance < initial_distance);
}

#[test]
fn test_ai_pathfinding_stops_when_attacking() {
    use crate::ecs::world::World;

    let mut world = World::new();

    // Create player very close to enemy (within attack range)
    let player = world.spawn();
    world.add_component(player, Player);
    world.add_component(player, Position::new(30.0, 0.0));

    // Create enemy
    let enemy = world.spawn();
    world.add_component(enemy, Enemy);
    let enemy_pos = Position::new(0.0, 0.0);
    world.add_component(enemy, enemy_pos);
    world.add_component(enemy, AI::new_with_type(EnemyType::Idle, enemy_pos));
    world.add_component(enemy, Velocity::zero());
    world.add_component(enemy, Speed::new(100.0));
    world.add_component(enemy, Health::new(100));
    world.add_component(enemy, Rotation::new(0.0));

    let mut system = AISystem::default();
    // Run multiple frames for state transition
    for _ in 0..30 {
        system.run(&mut world, 0.016);
    }

    // Enemy should be attacking
    let ai = world.get_component::<AI>(enemy).unwrap();
    assert_eq!(ai.state, AIState::SurePlayerSeen);

    // Enemy should stop moving when attacking
    let velocity = world.get_component::<Velocity>(enemy).unwrap();
    assert_eq!(velocity.x, 0.0);
    assert_eq!(velocity.y, 0.0);
}

#[test]
fn test_ai_pathfinding_respects_speed() {
    use crate::ecs::world::World;

    let mut world = World::new();

    // Create player (within detection range of both enemies)
    let player = world.spawn();
    world.add_component(player, Player);
    world.add_component(player, Position::new(250.0, 100.0));

    // Create slow enemy
    let slow_enemy = world.spawn();
    world.add_component(slow_enemy, Enemy);
    let slow_pos = Position::new(100.0, 100.0);
    world.add_component(slow_enemy, slow_pos);
    world.add_component(slow_enemy, AI::new_with_type(EnemyType::Idle, slow_pos));
    world.add_component(slow_enemy, Velocity::zero());
    world.add_component(slow_enemy, Speed::new(50.0));
    world.add_component(slow_enemy, Health::new(100));
    world.add_component(slow_enemy, Rotation::new(0.0));

    // Create fast enemy
    let fast_enemy = world.spawn();
    world.add_component(fast_enemy, Enemy);
    let fast_pos = Position::new(100.0, 120.0);
    world.add_component(fast_enemy, fast_pos);
    world.add_component(fast_enemy, AI::new_with_type(EnemyType::Idle, fast_pos));
    world.add_component(fast_enemy, Velocity::zero());
    world.add_component(fast_enemy, Speed::new(200.0));
    world.add_component(fast_enemy, Health::new(100));
    world.add_component(fast_enemy, Rotation::new(0.0));

    let mut system = AISystem::default();
    // Run multiple frames for state transition
    for _ in 0..30 {
        system.run(&mut world, 0.016);
    }

    // Both should be chasing
    let slow_ai = world.get_component::<AI>(slow_enemy).unwrap();
    let fast_ai = world.get_component::<AI>(fast_enemy).unwrap();
    assert_eq!(slow_ai.state, AIState::SurePlayerSeen);
    assert_eq!(fast_ai.state, AIState::SurePlayerSeen);

    // Fast enemy should have higher velocity magnitude
    let slow_vel = world.get_component::<Velocity>(slow_enemy).unwrap();
    let fast_vel = world.get_component::<Velocity>(fast_enemy).unwrap();

    let slow_mag = (slow_vel.x * slow_vel.x + slow_vel.y * slow_vel.y).sqrt();
    let fast_mag = (fast_vel.x * fast_vel.x + fast_vel.y * fast_vel.y).sqrt();

    assert!(fast_mag > slow_mag);
    assert!((slow_mag - 50.0).abs() < 1.0); // Should be approximately 50.0
    assert!((fast_mag - 200.0).abs() < 1.0); // Should be approximately 200.0
}

#[test]
fn test_ai_pathfinding_blocked_by_walls() {
    use crate::ecs::world::World;

    let mut world = World::new();

    // Add a wall between enemy and player
    world.add_wall(250.0, 0.0, 20.0, 500.0);

    // Create player on other side of wall (within detection range but no line of sight)
    let player = world.spawn();
    world.add_component(player, Player);
    world.add_component(player, Position::new(350.0, 250.0));

    // Create enemy
    let enemy = world.spawn();
    world.add_component(enemy, Enemy);
    world.add_component(enemy, Position::new(100.0, 250.0));
    world.add_component(enemy, AI::new());
    world.add_component(enemy, Velocity::zero());
    world.add_component(enemy, Speed::new(100.0));
    world.add_component(enemy, Health::new(100));

    let mut system = AISystem::default();
    system.run(&mut world, 0.016);

    // Enemy should NOT detect player through wall (line of sight blocked)
    // and remain Idle
    let ai = world.get_component::<AI>(enemy).unwrap();
    assert_eq!(ai.state, AIState::Unaware);

    // Enemy should not be moving
    let velocity = world.get_component::<Velocity>(enemy).unwrap();
    assert_eq!(velocity.x, 0.0);
    assert_eq!(velocity.y, 0.0);
}

#[test]
fn test_ai_cannot_see_player_outside_vision_cone() {
    use crate::ecs::world::World;

    let mut world = World::new();

    // Create player to the right of enemy
    let player = world.spawn();
    world.add_component(player, Player);
    world.add_component(player, Position::new(100.0, 0.0));

    // Create enemy facing DOWN (PI/2), player is to the right (outside 90-degree cone)
    let enemy = world.spawn();
    world.add_component(enemy, Enemy);
    let enemy_pos = Position::new(0.0, 0.0);
    world.add_component(enemy, enemy_pos);
    world.add_component(enemy, AI::new_with_type(EnemyType::Idle, enemy_pos));
    world.add_component(enemy, Velocity::zero());
    world.add_component(enemy, Speed::new(100.0));
    world.add_component(enemy, Health::new(100));
    world.add_component(enemy, Rotation::new(std::f32::consts::PI / 2.0)); // Facing down

    let mut system = AISystem::default();
    // Run multiple frames
    for _ in 0..30 {
        system.run(&mut world, 0.016);
    }

    // Enemy should NOT see player (outside vision cone)
    let ai = world.get_component::<AI>(enemy).unwrap();
    assert_eq!(
        ai.state,
        AIState::Unaware,
        "Enemy should not see player outside vision cone"
    );

    // Enemy should not be moving toward player
    let velocity = world.get_component::<Velocity>(enemy).unwrap();
    assert_eq!(velocity.x, 0.0);
    assert_eq!(velocity.y, 0.0);

    // Enemy rotation should not have changed to face player
    let rotation = world.get_component::<Rotation>(enemy).unwrap();
    assert_eq!(rotation.angle, std::f32::consts::PI / 2.0);
}

/// Helper: spawn a lone enemy of `ty` at `enemy_pos` facing right, with the
/// player straight ahead at `player_pos`, and run the AI for `frames` ticks.
fn drive_single_enemy(
    ty: EnemyType,
    enemy_pos: Position,
    player_pos: Position,
    speed: f32,
    frames: usize,
) -> (World, Entity) {
    let mut world = World::new();

    let player = world.spawn();
    world.add_component(player, Player);
    world.add_component(player, player_pos);

    let enemy = world.spawn();
    world.add_component(enemy, Enemy);
    world.add_component(enemy, enemy_pos);
    world.add_component(enemy, AI::new_with_type(ty, enemy_pos));
    world.add_component(enemy, Velocity::zero());
    world.add_component(enemy, Speed::new(speed));
    world.add_component(enemy, Health::new(50));
    world.add_component(enemy, Rotation::new(0.0)); // facing +x, toward player

    let mut system = AISystem::default();
    for _ in 0..frames {
        system.run(&mut world, 0.016);
    }
    (world, enemy)
}

fn vel_mag(world: &World, e: Entity) -> f32 {
    let v = world.get_component::<Velocity>(e).unwrap();
    (v.x * v.x + v.y * v.y).sqrt()
}

#[test]
fn test_feral_lunges_faster_than_base_speed_toward_player() {
    // The drifter locks on and dashes: at some point during the chase its
    // velocity must exceed its base speed (a burst), pointing at the player.
    let base = 100.0;
    let mut saw_burst = false;
    let mut burst_points_at_player = false;

    let mut world = World::new();
    let player = world.spawn();
    world.add_component(player, Player);
    world.add_component(player, Position::new(300.0, 0.0));
    let enemy = world.spawn();
    world.add_component(enemy, Enemy);
    let epos = Position::new(0.0, 0.0);
    world.add_component(enemy, epos);
    world.add_component(enemy, AI::new_with_type(EnemyType::Wandering, epos));
    world.add_component(enemy, Velocity::zero());
    world.add_component(enemy, Speed::new(base));
    world.add_component(enemy, Health::new(50));
    world.add_component(enemy, Rotation::new(0.0));

    let mut system = AISystem::default();
    for _ in 0..120 {
        system.run(&mut world, 0.016);
        let mag = vel_mag(&world, enemy);
        if mag > base + 1.0 {
            saw_burst = true;
            let v = world.get_component::<Velocity>(enemy).unwrap();
            // Player is at +x, so a lunge toward it has positive x velocity.
            if v.x > 0.0 {
                burst_points_at_player = true;
            }
        }
    }
    assert!(saw_burst, "feral should lunge above its base speed");
    assert!(burst_points_at_player, "the lunge should aim at the player");
}

#[test]
fn test_soldier_never_exceeds_base_speed_and_holds_at_range() {
    // A HUNTER soldier pursues with discipline: it caps at its base speed and
    // stops when within weapon range — it never bursts like a feral.
    let base = 100.0;
    let (world, enemy) = drive_single_enemy(
        EnemyType::Patrolling,
        Position::new(0.0, 0.0),
        Position::new(300.0, 0.0),
        base,
        120,
    );
    // Reached the chase state.
    let ai = world.get_component::<AI>(enemy).unwrap();
    assert_eq!(ai.state, AIState::SurePlayerSeen);
}

#[test]
fn test_soldier_velocity_capped_vs_feral_burst() {
    // Same geometry, same base speed: the feral's peak velocity outruns the
    // soldier's, proving the two allegiances move differently.
    let base = 100.0;

    let mut peak_soldier = 0.0_f32;
    let mut peak_feral = 0.0_f32;

    for (ty, peak) in [
        (EnemyType::Patrolling, &mut peak_soldier),
        (EnemyType::Wandering, &mut peak_feral),
    ] {
        let mut world = World::new();
        let player = world.spawn();
        world.add_component(player, Player);
        world.add_component(player, Position::new(400.0, 0.0));
        let enemy = world.spawn();
        world.add_component(enemy, Enemy);
        let epos = Position::new(0.0, 0.0);
        world.add_component(enemy, epos);
        world.add_component(enemy, AI::new_with_type(ty, epos));
        world.add_component(enemy, Velocity::zero());
        world.add_component(enemy, Speed::new(base));
        world.add_component(enemy, Health::new(50));
        world.add_component(enemy, Rotation::new(0.0));
        let mut system = AISystem::default();
        for _ in 0..90 {
            system.run(&mut world, 0.016);
            *peak = peak.max(vel_mag(&world, enemy));
        }
    }

    // Soldier is capped at base (small float slack); feral clearly exceeds it.
    assert!(
        peak_soldier <= base + 1.0,
        "soldier peaked at {peak_soldier}, expected <= {base}"
    );
    assert!(
        peak_feral > base + 1.0,
        "feral peaked at {peak_feral}, expected a burst above {base}"
    );
    assert!(
        peak_feral > peak_soldier,
        "feral should out-burst the soldier"
    );
}

#[test]
fn test_feral_burns_itself_out_while_lunging() {
    // "Running down like a dropped call": a feral loses health as it dashes,
    // even without ever being shot.
    let (world, enemy) = drive_single_enemy(
        EnemyType::Wandering,
        Position::new(0.0, 0.0),
        Position::new(300.0, 0.0),
        100.0,
        300,
    );
    let hp = world.get_component::<Health>(enemy).unwrap();
    assert!(
        hp.current < hp.max,
        "feral should decay from lunging (hp {} of {})",
        hp.current,
        hp.max
    );
}

#[test]
fn test_soldier_does_not_self_damage() {
    // Soldiers do not burn out — only ferals do.
    let (world, enemy) = drive_single_enemy(
        EnemyType::Patrolling,
        Position::new(0.0, 0.0),
        Position::new(300.0, 0.0),
        100.0,
        300,
    );
    let hp = world.get_component::<Health>(enemy).unwrap();
    assert_eq!(hp.current, hp.max, "soldier must not self-damage");
}

#[test]
fn test_feral_idle_wander_is_erratic() {
    // With no player in sight, a feral keeps twitching to new headings; over
    // many ticks it should visit several distinct directions (not the single
    // steady heading a disciplined patrol would hold between look-arounds).
    let mut world = World::new();
    let player = world.spawn();
    world.add_component(player, Player);
    world.add_component(player, Position::new(5000.0, 5000.0)); // far, unseen
    let enemy = world.spawn();
    world.add_component(enemy, Enemy);
    let epos = Position::new(400.0, 400.0);
    world.add_component(enemy, epos);
    world.add_component(enemy, AI::new_with_type(EnemyType::Wandering, epos));
    world.add_component(enemy, Velocity::zero());
    world.add_component(enemy, Speed::new(100.0));
    world.add_component(enemy, Health::new(50));
    world.add_component(enemy, Rotation::new(0.0));

    let mut system = AISystem::default();
    let mut directions = std::collections::HashSet::new();
    for _ in 0..600 {
        system.run(&mut world, 0.016);
        // Feral always moves while unaware.
        assert!(
            vel_mag(&world, enemy) > 0.0,
            "a feral is never still while unaware"
        );
        let dir = world.get_component::<AI>(enemy).unwrap().wander_direction;
        directions.insert((dir * 10.0).round() as i32);
    }
    assert!(
        directions.len() >= 3,
        "feral wander should be erratic (saw {} distinct headings)",
        directions.len()
    );
}

/// Pin the enemy into the chase state (like the perf harness's
/// `alert_all`) so throttle tests exercise the pathfinding branch every
/// tick regardless of vision.
fn pin_chase(world: &mut World, enemy: Entity, player_pos: Position) {
    if let Some(ai) = world.get_component_mut::<AI>(enemy) {
        ai.state = AIState::SurePlayerSeen;
        ai.last_known_player_position = Some(player_pos);
        ai.state_timer = 1e9;
    }
}

/// L-shaped corner world: player tucked behind the corner, enemy outside,
/// no (padded) line of sight between them.
fn corner_world() -> (World, Entity, Position) {
    let mut world = World::new();
    world.add_wall(200.0, 200.0, 400.0, 20.0); // horizontal wall
    world.add_wall(200.0, 200.0, 20.0, 200.0); // vertical wall

    let player_pos = Position::new(100.0, 300.0);
    let player = world.spawn();
    world.add_component(player, Player);
    world.add_component(player, player_pos);

    let enemy = world.spawn();
    world.add_component(enemy, Enemy);
    world.add_component(enemy, Position::new(300.0, 100.0));
    world.add_component(enemy, AI::new());
    world.add_component(enemy, Velocity::zero());
    world.add_component(enemy, Speed::new(100.0));
    world.add_component(enemy, Health::new(100));
    (world, enemy, player_pos)
}

#[test]
fn test_throttled_chaser_still_reaches_player_around_corner() {
    // With path recomputes throttled to ~5 Hz, a chaser must still round
    // the corner and close on the player by following its cached path.
    let (mut world, enemy, player_pos) = corner_world();
    let initial_distance = world
        .get_component::<Position>(enemy)
        .unwrap()
        .distance_to(&player_pos);

    let mut system = AISystem::default();
    for _ in 0..600 {
        pin_chase(&mut world, enemy, player_pos);
        system.run(&mut world, 0.016);
        // Integrate velocity (stand-in for the movement system).
        let vel = *world.get_component::<Velocity>(enemy).unwrap();
        if let Some(pos) = world.get_component_mut::<Position>(enemy) {
            pos.x += vel.x * 0.016;
            pos.y += vel.y * 0.016;
        }
    }

    let final_distance = world
        .get_component::<Position>(enemy)
        .unwrap()
        .distance_to(&player_pos);
    assert!(
        final_distance < 120.0 && final_distance < initial_distance,
        "chaser should round the corner: {initial_distance:.0} -> {final_distance:.0}"
    );
}

#[test]
fn test_path_recompute_is_throttled() {
    // A wall splits the world completely: the target is unreachable, so
    // every recompute is a worst-case full A* flood. The throttle must
    // keep that to a few recomputes per second, not one per tick (the
    // old behavior: 60 recomputes over these 60 ticks).
    let mut world = World::new();
    world.add_wall(250.0, 0.0, 20.0, 2000.0);

    let player_pos = Position::new(100.0, 1000.0);
    let player = world.spawn();
    world.add_component(player, Player);
    world.add_component(player, player_pos);

    let enemy = world.spawn();
    world.add_component(enemy, Enemy);
    world.add_component(enemy, Position::new(500.0, 1000.0));
    world.add_component(enemy, AI::new());
    world.add_component(enemy, Velocity::zero());
    world.add_component(enemy, Speed::new(100.0));
    world.add_component(enemy, Health::new(100));

    let mut system = AISystem::default();
    for _ in 0..60 {
        pin_chase(&mut world, enemy, player_pos);
        system.run(&mut world, 1.0 / 60.0);
    }

    let cache = world.get_component::<NavPath>(enemy).unwrap();
    assert!(
        cache.recomputes >= 2,
        "the path must still be re-evaluated periodically ({} recomputes)",
        cache.recomputes
    );
    assert!(
        cache.recomputes <= 8,
        "path recomputation must be throttled, got {} over 60 ticks (old code: 60)",
        cache.recomputes
    );
}

#[test]
fn test_throttle_repaths_immediately_when_target_moves_far() {
    // The drift rule: when the chase target jumps far from the cached
    // path's goal, the recompute happens NOW, not at the next 5 Hz slot.
    let (mut world, enemy, player_pos) = corner_world();
    let mut system = AISystem::default();
    pin_chase(&mut world, enemy, player_pos);
    system.run(&mut world, 0.016);
    let before = world.get_component::<NavPath>(enemy).unwrap().recomputes;

    // Teleport the chase target far away (still LOS-blocked, behind the
    // vertical wall further down) and tick once.
    let moved = Position::new(100.0, 390.0);
    pin_chase(&mut world, enemy, moved);
    system.run(&mut world, 0.016);
    let after = world.get_component::<NavPath>(enemy).unwrap().recomputes;
    assert_eq!(
        after,
        before + 1,
        "a big target move must force an immediate repath"
    );
}

#[test]
fn test_debug_structures_gated_by_debug_flag() {
    // Flag off (the default): chasing must not record DebugPath/DebugTrail.
    let (mut world, enemy, player_pos) = corner_world();
    let mut system = AISystem::default();
    for _ in 0..30 {
        pin_chase(&mut world, enemy, player_pos);
        system.run(&mut world, 0.016);
    }
    assert!(
        !world.has_component::<DebugPath>(enemy),
        "DebugPath must not be recorded with debug off"
    );
    assert!(
        !world.has_component::<DebugTrail>(enemy),
        "DebugTrail must not be recorded with debug off"
    );

    // Flag on: both appear (and the trail actually accumulates).
    world.set_debug_viz(true);
    for _ in 0..5 {
        pin_chase(&mut world, enemy, player_pos);
        system.run(&mut world, 0.016);
    }
    let dp = world.get_component::<DebugPath>(enemy).unwrap();
    assert!(!dp.waypoints.is_empty(), "DebugPath recorded with debug on");
    let trail = world.get_component::<DebugTrail>(enemy).unwrap();
    assert!(!trail.positions.is_empty());
}

#[test]
fn test_debug_trail_is_a_ring_buffer() {
    let mut trail = DebugTrail::new(3);
    for i in 0..5 {
        trail.add_position(Vec2::new(i as f32, 0.0));
    }
    assert_eq!(trail.positions.len(), 3);
    assert_eq!(trail.positions[0].x, 2.0); // oldest two dropped
    assert_eq!(trail.positions[2].x, 4.0);
}

#[test]
fn test_ai_can_see_player_behind_when_turned_around() {
    use crate::ecs::world::World;

    let mut world = World::new();

    // Create player behind the enemy
    let player = world.spawn();
    world.add_component(player, Player);
    world.add_component(player, Position::new(-100.0, 0.0));

    // Create enemy at origin facing RIGHT (0 degrees)
    let enemy = world.spawn();
    world.add_component(enemy, Enemy);
    let enemy_pos = Position::new(0.0, 0.0);
    world.add_component(enemy, enemy_pos);
    world.add_component(enemy, AI::new_with_type(EnemyType::Idle, enemy_pos));
    world.add_component(enemy, Velocity::zero());
    world.add_component(enemy, Speed::new(100.0));
    world.add_component(enemy, Health::new(100));
    world.add_component(enemy, Rotation::new(0.0)); // Facing right

    let mut system = AISystem::default();
    // Run a few frames - enemy shouldn't see player yet
    for _ in 0..5 {
        system.run(&mut world, 0.016);
    }

    // Enemy should NOT see player (behind them, outside vision cone)
    let ai = world.get_component::<AI>(enemy).unwrap();
    assert_eq!(
        ai.state,
        AIState::Unaware,
        "Enemy should not see player behind them"
    );

    // Now turn the enemy around to face left (PI)
    if let Some(rotation) = world.get_component_mut::<Rotation>(enemy) {
        rotation.angle = std::f32::consts::PI;
    }

    // Run more frames - now enemy should spot player
    for _ in 0..30 {
        system.run(&mut world, 0.016);
    }

    // Now enemy SHOULD see player (player is in front of vision cone)
    let ai = world.get_component::<AI>(enemy).unwrap();
    assert_eq!(
        ai.state,
        AIState::SurePlayerSeen,
        "Enemy should see player when facing them"
    );
}
