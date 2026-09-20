//! Tutorial gates + checkpoints, and floor 1's tutorial played headlessly.

use super::tests::*;
use super::*;
use crate::components::{
    AIState, Enemy, EnemyType, GameEvent, Health, Position, Weapon, WeaponType, AI,
};
use crate::ecs::World;
use crate::math::Vec2;
use crate::sim::Simulation;

#[test]
fn gate_input_parse_masking_and_satisfaction() {
    assert_eq!(GateInput::parse("punch"), Some(GateInput::Punch));
    assert_eq!(GateInput::parse("finish"), Some(GateInput::Finish));
    assert_eq!(GateInput::parse("pickup"), Some(GateInput::Pickup));
    assert_eq!(GateInput::parse("strike"), Some(GateInput::Strike));
    assert_eq!(GateInput::parse("fire"), Some(GateInput::Fire));
    assert_eq!(GateInput::parse("throw"), Some(GateInput::Throw));
    assert_eq!(GateInput::parse("dance"), None);

    // The left click only works with the matching tool in hand.
    assert!(GateInput::Punch.allows_primary(None));
    assert!(!GateInput::Punch.allows_primary(Some(WeaponType::Melee)));
    assert!(GateInput::Strike.allows_primary(Some(WeaponType::Melee)));
    assert!(!GateInput::Strike.allows_primary(Some(WeaponType::Pistol)));
    assert!(!GateInput::Strike.allows_primary(None));
    assert!(GateInput::Fire.allows_primary(Some(WeaponType::Shotgun)));
    assert!(!GateInput::Fire.allows_primary(Some(WeaponType::Melee)));
    assert!(!GateInput::Pickup.allows_primary(None));
    assert!(GateInput::Finish.allows_finisher());
    assert!(!GateInput::Punch.allows_finisher());
    // E stays live on every weapon-dependent gate (recovery path).
    assert!(GateInput::Pickup.allows_pickup());
    assert!(GateInput::Strike.allows_pickup());
    assert!(GateInput::Fire.allows_pickup());
    assert!(GateInput::Throw.allows_pickup());
    assert!(!GateInput::Punch.allows_pickup());
    assert!(!GateInput::Finish.allows_pickup());
    assert!(GateInput::Throw.allows_throw());
    assert!(!GateInput::Pickup.allows_throw());

    // Success events, one per gate kind.
    assert!(GateInput::Punch.satisfied_by(&GameEvent::PunchLanded));
    assert!(!GateInput::Punch.satisfied_by(&GameEvent::StrikeLanded));
    assert!(GateInput::Finish.satisfied_by(&GameEvent::FinisherDone));
    assert!(GateInput::Pickup.satisfied_by(&GameEvent::Pickup));
    assert!(GateInput::Strike.satisfied_by(&GameEvent::StrikeLanded));
    assert!(!GateInput::Strike.satisfied_by(&GameEvent::EnemyHit {
        by: WeaponType::Melee,
        at: crate::math::Vec2::zero(),
    }));
    assert!(GateInput::Fire.satisfied_by(&GameEvent::PlayerFired(WeaponType::Pistol)));
    assert!(
        !GateInput::Fire.satisfied_by(&GameEvent::PlayerFired(WeaponType::Melee)),
        "a melee swing is not a shot"
    );
    assert!(GateInput::Throw.satisfied_by(&GameEvent::ThrownImpact));
    assert!(
        !GateInput::Throw.satisfied_by(&GameEvent::Throw),
        "a throw satisfies only when it CONNECTS"
    );
}

// A tutorial-style floor: entering the zone disarms the player, drops a
// checkpoint, spawns a lone rogue and gates on PUNCH, then chains a
// FINISH gate through step_done, with timers watching the frozen clock.
const GT_WAVE: [SpawnDef; 1] = [SpawnDef::hostile(650.0, 650.0, EnemyType::Idle)];
const GT_STEPS: [StepDef; 4] = [
    StepDef {
        id: "teach",
        trigger: Trigger::EnterZone {
            zone: "z",
            before: None,
        },
        actions: &[
            Action::Disarm,
            Action::Checkpoint,
            Action::Spawn(&GT_WAVE),
            Action::Gate(GateDef {
                input: GateInput::Punch,
                text: "LEFT CLICK — PUNCH",
            }),
            Action::Objective("punched"),
        ],
    },
    StepDef {
        id: "next",
        trigger: Trigger::StepDone("teach"),
        actions: &[
            Action::Gate(GateDef {
                input: GateInput::Finish,
                text: "LEFT CLICK — FINISH",
            }),
            Action::Objective("finished"),
        ],
    },
    StepDef {
        id: "late",
        trigger: Trigger::Timer {
            seconds: 0.5,
            after: Some("teach"),
        },
        actions: &[Action::Sfx("ping")],
    },
    StepDef {
        id: "clock",
        trigger: Trigger::Timer {
            seconds: 0.3,
            after: None,
        },
        actions: &[Action::Sfx("clock")],
    },
];
const GT_FLOOR: FloorDef = FloorDef {
    spawns: &[],
    scenario: &GT_STEPS,
    ..T_FLOOR
};

const DT: f32 = 1.0 / 60.0;

fn sim_for(floor: &'static FloorDef) -> (Simulation, ScenarioState) {
    (
        Simulation::from_world(world_for(floor)),
        ScenarioState::new(floor),
    )
}

fn teleport(sim: &mut Simulation, to: Vec2) {
    let p = sim.player().unwrap();
    *sim.world.get_component_mut::<Position>(p).unwrap() = Position::from_vec2(to);
}

/// Advance `frames` full frames; returns the checkpoint snapshot taken
/// at the frame a `checkpoint` action requested one (if any).
fn run(
    sim: &mut Simulation,
    sc: &mut ScenarioState,
    frames: usize,
) -> Option<(World, ScenarioState)> {
    let mut cp = None;
    for _ in 0..frames {
        if sim.scenario_step(sc, DT) {
            cp = Some((sim.world.clone(), sc.clone()));
        }
    }
    cp
}

#[test]
fn gate_ignores_events_from_the_frame_that_installed_it() {
    // Frame order is systems -> tick (installs the gate) -> gate_notify
    // with that same frame's events. Those events were produced under
    // UNFROZEN input: a punch that landed just before the zone step ran
    // must not release a gate whose prompt was never drawn.
    let (mut sim, mut sc) = sim_for(&GT_FLOOR);
    teleport(&mut sim, Vec2::new(650.0, 620.0));
    sim.world
        .push_event(crate::components::GameEvent::PunchLanded);
    run(&mut sim, &mut sc, 1);
    assert!(sc.step_fired("teach"));
    assert_eq!(
        sc.gate_view().map(|g| g.input),
        Some(GateInput::Punch),
        "a same-frame event must not release the gate it predates"
    );
    assert_eq!(sc.objective, "start");
    // The next frame's punch does.
    sim.world
        .push_event(crate::components::GameEvent::PunchLanded);
    run(&mut sim, &mut sc, 1);
    assert_ne!(sc.gate_view().map(|g| g.input), Some(GateInput::Punch));
    assert_eq!(sc.objective, "punched");
}

#[test]
fn gate_freezes_the_world_and_releases_on_the_punch() {
    let (mut sim, mut sc) = sim_for(&GT_FLOOR);
    // Walk into the zone: the teach step disarms, checkpoints, spawns
    // the target and freezes on the PUNCH gate.
    teleport(&mut sim, Vec2::new(650.0, 620.0));
    let cp = run(&mut sim, &mut sc, 1);
    assert!(sc.step_fired("teach"));
    let gate = sc.gate_view().expect("gate installed");
    assert_eq!(gate.input, GateInput::Punch);
    assert_eq!(gate.text, "LEFT CLICK — PUNCH");
    assert!(cp.is_some(), "the checkpoint action requested a snapshot");
    let player = sim.player().unwrap();
    assert!(
        sim.world.get_component::<Weapon>(player).is_none(),
        "disarmed"
    );
    assert_eq!(sc.objective, "start", "post-gate actions did NOT run yet");
    let t_frozen = sc.time();

    // Make the enemy hostile and shoving-fast: under the freeze it still
    // never moves and never attacks, and the clock never advances.
    let enemy = sim.world.query::<Enemy>()[0];
    sim.world.get_component_mut::<AI>(enemy).unwrap().state = AIState::SurePlayerSeen;
    sim.world
        .get_component_mut::<crate::components::Velocity>(enemy)
        .map(|v| {
            v.x = 150.0;
            v.y = 0.0;
        })
        .unwrap();
    let enemy_pos = *sim.world.get_component::<Position>(enemy).unwrap();
    let health_before = crate::game::get_player_health(&sim.world);
    run(&mut sim, &mut sc, 90); // 1.5 s frozen
    assert_eq!(sc.time(), t_frozen, "scenario clock frozen under the gate");
    assert!(
        !sc.step_fired("clock"),
        "absolute timers do not advance under a gate"
    );
    let now = *sim.world.get_component::<Position>(enemy).unwrap();
    assert_eq!((now.x, now.y), (enemy_pos.x, enemy_pos.y), "enemy pinned");
    assert_eq!(
        crate::game::get_player_health(&sim.world),
        health_before,
        "no enemy attacks under the freeze"
    );

    // The PLAYER still moves under the freeze (to close the distance).
    let before = sim.player_position().unwrap();
    sim.set_player_velocity(Vec2::new(0.0, 200.0));
    run(&mut sim, &mut sc, 6);
    assert!(sim.player_position().unwrap().y > before.y + 10.0);

    // The punch connects: the gate releases, the post-gate actions run.
    teleport(&mut sim, Vec2::new(650.0, 620.0));
    sim.set_player_velocity(Vec2::zero());
    assert!(sim.player_fire(Vec2::new(650.0, 650.0)), "punch landed");
    run(&mut sim, &mut sc, 1);
    assert!(sc.gate_view().is_none() || sc.gate_view().unwrap().input != GateInput::Punch);
    assert_eq!(sc.objective, "punched");
    assert!(
        sim.world.has_component::<crate::components::Stunned>(enemy),
        "knocked down"
    );

    // step_done chains only now: the FINISH gate installs on the next
    // tick, and the after-teach timer counts from the RELEASE.
    run(&mut sim, &mut sc, 1);
    assert!(sc.step_fired("next"));
    assert_eq!(sc.gate_view().unwrap().input, GateInput::Finish);
    assert!(!sc.step_fired("late"));

    // The victim never gets back up under the freeze, however long the
    // player dawdles (KNOCKDOWN_SECS is 3).
    run(&mut sim, &mut sc, 300); // 5 s frozen
    assert!(sim.world.has_component::<crate::components::Stunned>(enemy));

    // Finish it: step over the body (the punch shoved it away) — the
    // finisher animation runs THROUGH the freeze.
    let ep = *sim.world.get_component::<Position>(enemy).unwrap();
    teleport(&mut sim, Vec2::new(ep.x - 30.0, ep.y));
    assert!(sim.player_finisher(), "finisher started on the downed bot");
    run(&mut sim, &mut sc, 60);
    assert!(sc.gate_view().is_none(), "finish gate released");
    assert_eq!(sc.objective, "finished");
    assert!(sim.world.get_component::<Health>(enemy).unwrap().is_dead());

    // Unfrozen again: both timers resume from the frozen clock.
    run(&mut sim, &mut sc, 40); // ~0.66 s
    assert!(sc.step_fired("late"), "0.5 s after teach's gate release");
    assert!(sc.step_fired("clock"));
}

#[test]
fn gate_skip_releases_like_a_success() {
    let (mut sim, mut sc) = sim_for(&GT_FLOOR);
    teleport(&mut sim, Vec2::new(650.0, 620.0));
    run(&mut sim, &mut sc, 1);
    assert_eq!(sc.gate_view().unwrap().input, GateInput::Punch);
    sc.gate_skip(&mut sim.world);
    assert!(sc.gate_view().is_none());
    assert_eq!(sc.objective, "punched");
    run(&mut sim, &mut sc, 1);
    assert_eq!(sc.gate_view().unwrap().input, GateInput::Finish);
    sc.gate_skip(&mut sim.world);
    assert_eq!(sc.objective, "finished");
    run(&mut sim, &mut sc, 40);
    assert!(sc.step_fired("late"));
}

#[test]
fn checkpoint_restores_the_run_on_death() {
    let (mut sim, mut sc) = sim_for(&GT_FLOOR);
    teleport(&mut sim, Vec2::new(650.0, 620.0));
    let (cp_world, cp_sc) = run(&mut sim, &mut sc, 1).expect("checkpoint requested");

    // Play on: punch the bot down, then "die".
    assert!(sim.player_fire(Vec2::new(650.0, 650.0)));
    run(&mut sim, &mut sc, 2);
    assert_eq!(sc.objective, "punched");
    let player = sim.player().unwrap();
    sim.world
        .get_component_mut::<Health>(player)
        .unwrap()
        .take_damage(9999);
    assert!(!sim.player_alive());

    // Death -> restore the snapshot: the run is back at the gate.
    sim.world = cp_world.clone();
    sc = cp_sc.clone();
    assert!(sim.player_alive());
    assert_eq!(
        crate::game::get_player_health(&sim.world),
        100,
        "health restored"
    );
    assert!(
        sim.world
            .get_component::<Weapon>(sim.player().unwrap())
            .is_none(),
        "still disarmed, as snapshotted"
    );
    assert!(
        sc.step_fired("teach"),
        "fired steps travel with the snapshot"
    );
    assert!(!sc.step_fired("next"));
    assert_eq!(sc.gate_view().unwrap().input, GateInput::Punch);
    assert_eq!(sc.objective, "start", "post-gate objective rolled back");
    let enemy = sim.world.query::<Enemy>()[0];
    assert!(
        !sim.world.has_component::<crate::components::Stunned>(enemy),
        "the knockdown after the snapshot rolled back"
    );
    let pos = sim.world.get_component::<Position>(enemy).unwrap();
    assert_eq!((pos.x, pos.y), (650.0, 650.0), "enemy back at its spawn");
    // The restored run plays out normally.
    assert!(sim.player_fire(Vec2::new(650.0, 650.0)));
    run(&mut sim, &mut sc, 2);
    assert_eq!(sc.objective, "punched");
}

#[test]
fn enter_zone_before_disarms_once_the_other_step_fires() {
    const B_STEPS: [StepDef; 2] = [
        StepDef {
            id: "blocked",
            trigger: Trigger::EnterZone {
                zone: "z",
                before: Some("point"),
            },
            actions: &[Action::Objective("blocked")],
        },
        StepDef {
            id: "point",
            trigger: Trigger::Timer {
                seconds: 1.0,
                after: None,
            },
            actions: &[Action::Sfx("pt")],
        },
    ];
    const B_FLOOR: FloorDef = FloorDef {
        spawns: &[],
        scenario: &B_STEPS,
        ..T_FLOOR
    };
    // Entering the zone BEFORE the point fires the step...
    let mut world = world_for(&B_FLOOR);
    let mut sc = ScenarioState::new(&B_FLOOR);
    move_player(&mut world, Vec2::new(650.0, 650.0));
    sc.tick(&mut world, 0.016);
    assert!(sc.step_fired("blocked"));

    // ...but once the point has fired, the step is disarmed forever.
    let mut world = world_for(&B_FLOOR);
    let mut sc = ScenarioState::new(&B_FLOOR);
    while sc.time() < 1.1 {
        sc.tick(&mut world, 0.1);
    }
    assert!(sc.step_fired("point"));
    move_player(&mut world, Vec2::new(650.0, 650.0));
    for _ in 0..20 {
        sc.tick(&mut world, 0.016);
    }
    assert!(!sc.step_fired("blocked"));
}

#[test]
fn floor_1_tutorial_plays_through_headlessly() {
    let floor = crate::levels::floor_def(1);
    let mut sim = Simulation::new(1);
    let mut sc = ScenarioState::new(floor);
    run(&mut sim, &mut sc, 2);
    assert!(sc.step_fired("intro"));
    assert_eq!(
        sim.world.query::<Enemy>().len(),
        4,
        "the passive lobby crowd"
    );
    assert_eq!(sim.enemies_alive(), 0, "bystanders are not rogues yet");
    assert!(
        !sc.step_fired("clear"),
        "all_dead must not fire on a floor whose rogues have not shown up"
    );

    // Straight to the desk: the cover-blown conversation opens.
    teleport(&mut sim, Vec2::new(500.0, 420.0));
    run(&mut sim, &mut sc, 2);
    assert!(sc.step_fired("desk"));
    assert!(sc.dialogue_active());
    // Click through the CORRUPTOR scene.
    for _ in 0..30 {
        if !sc.dialogue_active() {
            break;
        }
        sc.dialogue_advance();
        run(&mut sim, &mut sc, 5);
    }
    assert!(!sc.dialogue_active(), "conversation dismissed");

    // 0.4 s later: disarm + checkpoint + the lone SENTINEL + PUNCH gate.
    let cp1 = run(&mut sim, &mut sc, 40);
    assert!(sc.step_fired("tut_punch"));
    assert!(cp1.is_some(), "first checkpoint taken");
    assert_eq!(sc.gate_view().unwrap().input, GateInput::Punch);
    assert!(sim
        .world
        .get_component::<Weapon>(sim.player().unwrap())
        .is_none());
    let t_frozen = sc.time();
    run(&mut sim, &mut sc, 30);
    assert_eq!(sc.time(), t_frozen, "frozen on the prompt");

    // PUNCH the rushing SENTINEL (spawned at 580,380).
    teleport(&mut sim, Vec2::new(545.0, 380.0));
    assert!(sim.player_fire(Vec2::new(580.0, 380.0)));
    run(&mut sim, &mut sc, 2);
    assert_eq!(sc.gate_view().unwrap().input, GateInput::Finish);

    // FINISH it (pound — bare hands): step over the shoved body first.
    let downed1 = sim
        .world
        .query::<Enemy>()
        .into_iter()
        .find(|&e| sim.world.has_component::<crate::components::Stunned>(e))
        .expect("the punched SENTINEL is down");
    let d1 = *sim.world.get_component::<Position>(downed1).unwrap();
    teleport(&mut sim, Vec2::new(d1.x - 30.0, d1.y));
    assert!(sim.player_finisher());
    run(&mut sim, &mut sc, 70);
    assert!(sc.step_fired("tut_bar"));
    assert_eq!(sc.gate_view().unwrap().input, GateInput::Pickup);

    // THE TRAP THAT SOFT-LOCKED REAL PLAYERS: pressing E right where
    // the finisher victim died must grab NOTHING — tutorial victims
    // spawn `unarmed`, so no corpse pistol can release the pickup gate
    // and then dead-end the strike gate (a strike gate masks gun clicks).
    assert_eq!(sim.player_pickup(), None, "the corpse dropped nothing");
    run(&mut sim, &mut sc, 2);
    assert_eq!(
        sc.gate_view().unwrap().input,
        GateInput::Pickup,
        "the pickup gate is still waiting for the bar"
    );

    // TAKE THE BAR (the melee pickup by the desk).
    teleport(&mut sim, Vec2::new(420.0, 370.0));
    assert_eq!(sim.player_pickup(), Some(WeaponType::Melee));
    run(&mut sim, &mut sc, 2);
    assert!(sc.step_fired("tut_strike"));
    assert_eq!(sc.gate_view().unwrap().input, GateInput::Strike);

    // SWING THE BAR at the second bot (600,320).
    teleport(&mut sim, Vec2::new(560.0, 320.0));
    assert!(sim.player_fire(Vec2::new(600.0, 320.0)));
    run(&mut sim, &mut sc, 2);
    assert!(sc.step_fired("tut_throw"));
    assert_eq!(sc.gate_view().unwrap().input, GateInput::Throw);

    // THROW THE BAR at the third bot (400,300): flies under the freeze.
    teleport(&mut sim, Vec2::new(340.0, 300.0));
    assert!(sim.player_throw(Vec2::new(400.0, 300.0)));
    run(&mut sim, &mut sc, 30);
    assert!(sc.step_fired("tut_retrieve"), "the throw connected");
    assert_eq!(sc.gate_view().unwrap().input, GateInput::Pickup);

    // GET IT BACK: the bar landed as a pickup where it struck.
    let bar = sim
        .world
        .query::<crate::components::WeaponPickup>()
        .into_iter()
        .find(|&e| {
            sim.world
                .get_component::<crate::components::WeaponPickup>(e)
                .is_some_and(|p| p.weapon_type == WeaponType::Melee)
        })
        .expect("the thrown bar is on the floor");
    let bar_pos = *sim.world.get_component::<Position>(bar).unwrap();
    teleport(&mut sim, Vec2::new(bar_pos.x, bar_pos.y));
    assert_eq!(sim.player_pickup(), Some(WeaponType::Melee));
    run(&mut sim, &mut sc, 2);
    assert!(sc.step_fired("tut_overhead"));
    assert_eq!(sc.gate_view().unwrap().input, GateInput::Finish);

    // OVERHEAD on the downed bot (kept down by the frozen knockdown).
    let downed = sim
        .world
        .query::<Enemy>()
        .into_iter()
        .find(|&e| {
            sim.world.has_component::<crate::components::Stunned>(e)
                && sim
                    .world
                    .get_component::<Health>(e)
                    .is_some_and(|h| h.is_alive())
        })
        .expect("the thrown-down bot");
    let dp = *sim.world.get_component::<Position>(downed).unwrap();
    teleport(&mut sim, Vec2::new(dp.x - 30.0, dp.y));
    assert!(sim.player_finisher());
    let cp2 = run(&mut sim, &mut sc, 80);
    assert!(sc.gate_view().is_none(), "tutorial complete");
    assert!(sc.step_fired("wake"));
    let cp2 = cp2.expect("second checkpoint at the free wave");
    assert_eq!(
        sim.enemies_alive(),
        7,
        "4 alerted lobby bots + the free wave of 3"
    );
    assert_eq!(sc.objective, "They know. Purge reception.");

    // Die in the free fight -> restore: straight back to the wave.
    let player = sim.player().unwrap();
    sim.world
        .get_component_mut::<Health>(player)
        .unwrap()
        .take_damage(9999);
    assert!(!sim.player_alive());
    sim.world = cp2.0.clone();
    sc = cp2.1.clone();
    assert!(sim.player_alive());
    assert!(sc.step_fired("wake"));
    assert_eq!(sim.enemies_alive(), 7, "the wave is inside the snapshot");
    assert_eq!(
        crate::game::get_player_weapon(&sim.world),
        Some(WeaponType::Melee),
        "the bar came back with the snapshot"
    );

    // Clear the floor: the lift opens.
    crate::game::purge_all_enemies(&mut sim.world);
    run(&mut sim, &mut sc, 2);
    assert!(sc.step_fired("clear"));
    assert!(exit_open(&sim.world, "lift"));
}

/// Regression for the floor-1 "LEFT CLICK — SWING THE BAR" soft-lock:
/// replays the WHOLE tutorial through the browser's own gate input
/// dispatch ([`crate::game::gated_player_input`] — the exact function
/// `app/game_loop.rs` forwards `input::` state to), with realistic play: the player
/// WALKS (velocity + the movement system, frame by frame, under the
/// freeze) instead of being teleported onto each target, and both melee
/// gates are first attempted from ~90 px — the distance at which two
/// 60 px robot sprites LOOK adjacent on screen, where the pre-fix 50 px
/// reach whiffed with no feedback and the gate never released.
#[test]
fn floor_1_tutorial_plays_through_the_browser_gate_dispatch() {
    use crate::components::Position;
    use crate::game::{gated_player_input, get_player_weapon, PlayerIntents};

    /// One browser frame: the gate dispatch (if a gate is up), then the
    /// engine tick + scenario tick + gate notify — the `app/game_loop.rs` order.
    fn frame(sim: &mut Simulation, sc: &mut ScenarioState, intents: &PlayerIntents) {
        if let Some(g) = sc.gate_view() {
            gated_player_input(&mut sim.world, g, intents);
        }
        sim.scenario_step(sc, DT);
    }

    fn idle() -> PlayerIntents {
        PlayerIntents {
            left_pressed: false,
            left_down: false,
            right_pressed: false,
            e_pressed: false,
            mouse_world: Vec2::zero(),
        }
    }
    fn swing_at(mouse: Vec2) -> PlayerIntents {
        PlayerIntents {
            left_pressed: true,
            left_down: true,
            mouse_world: mouse,
            ..idle()
        }
    }
    fn press_e() -> PlayerIntents {
        PlayerIntents {
            e_pressed: true,
            ..idle()
        }
    }
    fn throw_at(mouse: Vec2) -> PlayerIntents {
        PlayerIntents {
            right_pressed: true,
            mouse_world: mouse,
            ..idle()
        }
    }

    /// Walk the player to `to` at full speed, one frame at a time (the
    /// movement system does the moving — under a gate freeze too, like
    /// the browser). Holds / conversations lock movement, as `app/game_loop.rs`
    /// does.
    fn walk_to(sim: &mut Simulation, sc: &mut ScenarioState, to: Vec2) {
        for _ in 0..1500 {
            if sc.hold_active() || sc.dialogue_active() {
                sim.set_player_velocity(Vec2::zero());
                sim.scenario_step(sc, DT);
                continue;
            }
            let pos = sim.player_position().unwrap();
            if (to - pos).length() <= 4.0 {
                break;
            }
            sim.set_player_velocity((to - pos).normalize() * 200.0);
            sim.scenario_step(sc, DT);
        }
        sim.set_player_velocity(Vec2::zero());
        let pos = sim.player_position().unwrap();
        assert!(
            (to - pos).length() <= 4.0,
            "walked to ({}, {}) but stopped at ({}, {})",
            to.x,
            to.y,
            pos.x,
            pos.y
        );
    }

    let floor = crate::levels::floor_def(1);
    let mut sim = Simulation::new(1);
    let mut sc = ScenarioState::new(floor);
    run(&mut sim, &mut sc, 2);
    assert!(sc.step_fired("intro"));

    // WALK from the main doors straight up to just short of the desk
    // (the old scanner-arch "SIGNATURE CHECK" hold was cut — walking in
    // stays uninterrupted until the desk scene).
    walk_to(&mut sim, &mut sc, Vec2::new(500.0, 455.0));
    // Step onto the desk: the takeover conversation opens.
    for _ in 0..60 {
        if sc.dialogue_active() {
            break;
        }
        sim.set_player_velocity(Vec2::new(0.0, -200.0));
        sim.scenario_step(&mut sc, DT);
    }
    sim.set_player_velocity(Vec2::zero());
    assert!(sc.step_fired("desk"));
    assert!(sc.dialogue_active());
    for _ in 0..30 {
        if !sc.dialogue_active() {
            break;
        }
        sc.dialogue_advance();
        run(&mut sim, &mut sc, 5);
    }
    assert!(!sc.dialogue_active());

    // 0.4 s later: disarm + checkpoint + the PUNCH gate on the SENTINEL
    // spawned at (580, 380).
    run(&mut sim, &mut sc, 40);
    assert!(sc.step_fired("tut_punch"));
    assert_eq!(sc.gate_view().unwrap().input, GateInput::Punch);
    assert!(get_player_weapon(&sim.world).is_none(), "disarmed");

    // A jab from ~90 px — sprites LOOK adjacent — whiffs: no release.
    // (Approach from the north: the cone stays clear of the passive
    // civilians milling around the desk.)
    walk_to(&mut sim, &mut sc, Vec2::new(580.0, 290.0));
    frame(&mut sim, &mut sc, &swing_at(Vec2::new(580.0, 380.0)));
    assert_eq!(
        sc.gate_view().unwrap().input,
        GateInput::Punch,
        "a 90 px jab still misses"
    );
    // Fist cooldown, step up to 50 px, jab again: down it goes.
    run(&mut sim, &mut sc, 30);
    walk_to(&mut sim, &mut sc, Vec2::new(580.0, 330.0));
    frame(&mut sim, &mut sc, &swing_at(Vec2::new(580.0, 380.0)));
    run(&mut sim, &mut sc, 2); // the released gate's follow-up step fires
    assert!(sc.step_fired("tut_finish"), "the jab connected");
    assert_eq!(sc.gate_view().unwrap().input, GateInput::Finish);

    // FINISH: walk over the shoved body and click.
    let downed1 = sim
        .world
        .query::<Enemy>()
        .into_iter()
        .find(|&e| sim.world.has_component::<crate::components::Stunned>(e))
        .expect("the punched SENTINEL is down");
    let d1 = *sim.world.get_component::<Position>(downed1).unwrap();
    walk_to(&mut sim, &mut sc, Vec2::new(d1.x - 40.0, d1.y));
    frame(&mut sim, &mut sc, &swing_at(Vec2::new(d1.x, d1.y)));
    for _ in 0..70 {
        frame(&mut sim, &mut sc, &idle());
    }
    assert!(sc.step_fired("tut_bar"));
    assert_eq!(sc.gate_view().unwrap().input, GateInput::Pickup);

    // TAKE THE BAR: walk to the pickup by the desk and press E.
    walk_to(&mut sim, &mut sc, Vec2::new(420.0, 370.0));
    frame(&mut sim, &mut sc, &press_e());
    assert_eq!(get_player_weapon(&sim.world), Some(WeaponType::Melee));
    run(&mut sim, &mut sc, 2);
    assert!(sc.step_fired("tut_strike"));
    assert_eq!(sc.gate_view().unwrap().input, GateInput::Strike);
    let strike_bot = sim
        .world
        .query::<Enemy>()
        .into_iter()
        .find(|&e| {
            sim.world
                .get_component::<Position>(e)
                .is_some_and(|p| (p.x - 600.0).abs() < 1.0 && (p.y - 320.0).abs() < 1.0)
        })
        .expect("the strike target spawned at (600, 320)");

    // THE BUG: swing the bar from ~90 px — where the two sprites already
    // look adjacent on screen. It whiffs (only the whoosh, no impact, no
    // feedback), exactly what soft-locked browser players when the reach
    // was 50 px and nothing hinted they had to step INSIDE the target.
    walk_to(&mut sim, &mut sc, Vec2::new(510.0, 320.0));
    frame(&mut sim, &mut sc, &swing_at(Vec2::new(600.0, 320.0)));
    assert_eq!(
        sc.gate_view().unwrap().input,
        GateInput::Strike,
        "a 90 px swing still whiffs"
    );
    // Weapon cooldown, then close to arm's length (~65 px, sprites
    // touching): the 70 px reach lands the swing WITHOUT demanding that
    // the player physically overlap the frozen target.
    run(&mut sim, &mut sc, 32);
    walk_to(&mut sim, &mut sc, Vec2::new(535.0, 320.0));
    frame(&mut sim, &mut sc, &swing_at(Vec2::new(600.0, 320.0)));
    run(&mut sim, &mut sc, 2);
    assert!(sc.step_fired("tut_throw"), "the strike released the gate");
    assert_eq!(sc.gate_view().unwrap().input, GateInput::Throw);
    // The 100-damage swing killed the bot: its corpse fell along the
    // blow (attacker west of it -> fall east), head away from the player.
    assert!(sim
        .world
        .get_component::<Health>(strike_bot)
        .unwrap()
        .is_dead());
    let fall = sim
        .world
        .get_component::<crate::components::Stunned>(strike_bot)
        .expect("the killing swing recorded the corpse's fall")
        .fall_angle;
    assert!(fall.abs() < 0.01, "fell due east, got {fall}");

    // THROW THE BAR at the third bot (400, 300).
    walk_to(&mut sim, &mut sc, Vec2::new(340.0, 300.0));
    frame(&mut sim, &mut sc, &throw_at(Vec2::new(400.0, 300.0)));
    run(&mut sim, &mut sc, 30);
    assert!(sc.step_fired("tut_retrieve"), "the throw connected");
    assert_eq!(sc.gate_view().unwrap().input, GateInput::Pickup);

    // GET IT BACK: walk to where the bar landed and press E.
    let bar = sim
        .world
        .query::<crate::components::WeaponPickup>()
        .into_iter()
        .find(|&e| {
            sim.world
                .get_component::<crate::components::WeaponPickup>(e)
                .is_some_and(|p| p.weapon_type == WeaponType::Melee)
        })
        .expect("the thrown bar is on the floor");
    let bar_pos = *sim.world.get_component::<Position>(bar).unwrap();
    walk_to(&mut sim, &mut sc, Vec2::new(bar_pos.x, bar_pos.y));
    frame(&mut sim, &mut sc, &press_e());
    run(&mut sim, &mut sc, 2);
    assert!(sc.step_fired("tut_overhead"));
    assert_eq!(sc.gate_view().unwrap().input, GateInput::Finish);

    // PUT IT DOWN: overhead finisher on the thrown-down bot.
    let downed = sim
        .world
        .query::<Enemy>()
        .into_iter()
        .find(|&e| {
            sim.world.has_component::<crate::components::Stunned>(e)
                && sim
                    .world
                    .get_component::<Health>(e)
                    .is_some_and(|h| h.is_alive())
        })
        .expect("the thrown-down bot");
    let dp = *sim.world.get_component::<Position>(downed).unwrap();
    walk_to(&mut sim, &mut sc, Vec2::new(dp.x - 40.0, dp.y));
    frame(&mut sim, &mut sc, &swing_at(Vec2::new(dp.x, dp.y)));
    for _ in 0..90 {
        frame(&mut sim, &mut sc, &idle());
    }
    assert!(sc.gate_view().is_none(), "tutorial complete");
    assert!(sc.step_fired("wake"));
    assert_eq!(
        sim.enemies_alive(),
        7,
        "4 alerted lobby bots + the free wave of 3"
    );
}

#[test]
fn speaker_colours_and_accent_parse() {
    assert_eq!(speaker_rgb("CL4-UD3"), (255, 111, 97));
    assert_eq!(speaker_rgb("nobody"), (255, 255, 255));
    assert_eq!(parse_hex_rgb("#37f0e6"), Some((0x37, 0xf0, 0xe6)));
    assert_eq!(parse_hex_rgb("37f0e6"), None);
    assert_eq!(T_FLOOR.accent_rgb(), (0x37, 0xf0, 0xe6));
    assert_eq!(T_FLOOR.player_spawn(), Vec2::new(500.0, 750.0));
}
