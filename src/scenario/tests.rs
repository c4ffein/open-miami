//! Triggers, comms, holds, looks, alerts, conversations, exits.

use super::*;
use crate::components::{AIState, Enemy, AI};
use crate::components::{Elevator, EnemyType, Health, Player, Position};
use crate::ecs::World;
use crate::game::spawn_enemy_with_type;
use crate::game::spawn_player;
use crate::math::Vec2;
use crate::systems::elevator::ElevatorSystem;

// A tiny hand-built floor exercising every trigger kind.
const T_SPAWNS: [SpawnDef; 1] = [SpawnDef::hostile(300.0, 300.0, EnemyType::Idle)];
const T_WAVE: [SpawnDef; 2] = [
    SpawnDef::hostile(320.0, 320.0, EnemyType::Patrolling),
    SpawnDef::hostile(340.0, 340.0, EnemyType::Wandering),
];
const T_ZONES: [ZoneDef; 1] = [ZoneDef {
    id: "z",
    rect: Rect::new(600.0, 600.0, 100.0, 100.0),
}];
const T_EXITS: [ElevatorDef; 2] = [
    ElevatorDef {
        id: "a",
        rect: Rect::new(455.0, 20.0, 90.0, 60.0),
        label: "A",
        to: 2,
        open: false,
        kind: ElevatorKind::Lift,
    },
    ElevatorDef {
        id: "b",
        rect: Rect::new(920.0, 355.0, 60.0, 90.0),
        label: "B",
        to: 2,
        open: false,
        kind: ElevatorKind::Lift,
    },
];
const T_STEPS: [StepDef; 6] = [
    StepDef {
        id: "intro",
        trigger: Trigger::Start,
        actions: &[
            Action::Say(SayDef {
                who: "HUNTER",
                text: "one",
                delay: 0.5,
            }),
            Action::Say(SayDef {
                who: "CL4-UD3",
                text: "two",
                delay: 0.0,
            }),
        ],
    },
    StepDef {
        id: "zone",
        trigger: Trigger::EnterZone {
            zone: "z",
            before: None,
        },
        actions: &[Action::Spawn(&T_WAVE), Action::Objective("wave")],
    },
    StepDef {
        id: "late",
        trigger: Trigger::Timer {
            seconds: 5.0,
            after: Some("intro"),
        },
        actions: &[Action::Sfx("ping")],
    },
    StepDef {
        id: "first_blood",
        trigger: Trigger::Kills(1),
        actions: &[Action::OpenExit("a")],
    },
    StepDef {
        id: "after_open",
        trigger: Trigger::ExitOpen(Some("a")),
        actions: &[Action::Objective("go")],
    },
    StepDef {
        id: "clear",
        trigger: Trigger::AllDead,
        actions: &[Action::OpenExit("b"), Action::CloseExit("a")],
    },
];
pub(super) const T_FLOOR: FloorDef = FloorDef {
    id: 1,
    name: "TEST",
    theme: "T",
    accent: "#37f0e6",
    flavor: "",
    objective: "start",
    width: 1000.0,
    height: 800.0,
    entry: ElevatorDef {
        id: "entry",
        rect: Rect::new(455.0, 720.0, 90.0, 60.0),
        label: "IN",
        to: SURFACE_EXIT,
        open: false,
        kind: ElevatorKind::Lift,
    },
    exits: &T_EXITS,
    walls: &[],
    rooms: &[],
    zones: &T_ZONES,
    spawns: &T_SPAWNS,
    pickups: &[],
    props: &[],
    scenario: &T_STEPS,
    surface: Surface::Checker,
};

pub(super) fn world_for(floor: &'static FloorDef) -> World {
    let mut world = World::new();
    spawn_player(&mut world, floor.player_spawn());
    for s in floor.spawns {
        spawn_from_def(&mut world, s);
    }
    spawn_floor_markers(&mut world, floor);
    world
}

pub(super) fn exit_open(world: &World, id: &str) -> bool {
    world.query::<Elevator>().iter().any(|&e| {
        world
            .get_component::<Elevator>(e)
            .map(|el| el.is_exit && el.id == id && el.open)
            .unwrap_or(false)
    })
}

pub(super) fn move_player(world: &mut World, to: Vec2) {
    let p = world.query::<Player>()[0];
    *world.get_component_mut::<Position>(p).unwrap() = Position::from_vec2(to);
}

fn kill_all(world: &mut World) {
    for e in world.query::<Enemy>() {
        world
            .get_component_mut::<Health>(e)
            .unwrap()
            .take_damage(9999);
    }
}

#[test]
fn start_fires_once_and_queues_comms_in_order() {
    let mut world = world_for(&T_FLOOR);
    let mut sc = ScenarioState::new(&T_FLOOR);
    assert_eq!(sc.objective, "start");
    sc.tick(&mut world, 0.016);
    assert!(sc.step_fired("intro"));
    // Nothing visible yet: the first line waits for its 0.5s delay, and the
    // second line waits behind the first even though its own delay is 0.
    assert_eq!(sc.comms.visible().len(), 0);
    assert_eq!(sc.comms.pending(), 2);
    sc.tick(&mut world, 0.5);
    assert_eq!(sc.comms.visible().len(), 1);
    assert_eq!(sc.comms.visible()[0].who, "HUNTER");
    // "one" types in 3/38 s; then a gap; then "two" starts.
    for _ in 0..30 {
        sc.tick(&mut world, 0.05);
    }
    assert_eq!(sc.comms.visible().len(), 2);
    assert_eq!(sc.comms.visible()[1].who, "CL4-UD3");
    assert_eq!(sc.comms.pending(), 0);
    // Start fired exactly once: no duplicate lines after many ticks.
    for _ in 0..100 {
        sc.tick(&mut world, 0.05);
    }
    assert_eq!(sc.comms.pending(), 0);
    assert_eq!(sc.comms.visible().len(), 2);
}

#[test]
fn typewriter_reveals_progressively() {
    let line = CommsLine {
        who: "HUNTER",
        text: "abcdefghij",
        age: 0.0,
    };
    assert_eq!(line.chars_shown(), 0);
    let mut mid = line.clone();
    mid.age = 5.0 / COMMS_CHARS_PER_SEC;
    assert_eq!(mid.chars_shown(), 5);
    let mut done = line.clone();
    done.age = 100.0;
    assert_eq!(done.chars_shown(), 10);
    assert!(done.fully_typed());
    assert!(done.expired());
    assert_eq!(done.alpha(), 0.0);
    assert_eq!(mid.alpha(), 1.0);
}

#[test]
fn timer_after_step_fires_at_the_right_time() {
    let mut world = world_for(&T_FLOOR);
    let mut sc = ScenarioState::new(&T_FLOOR);
    sc.tick(&mut world, 0.1); // intro fires at t=0.1
    for _ in 0..47 {
        sc.tick(&mut world, 0.1); // t = 4.8
    }
    assert!(!sc.step_fired("late"));
    assert!(sc.drain_sfx().is_empty());
    for _ in 0..4 {
        sc.tick(&mut world, 0.1); // t = 5.2 >= 0.1 + 5.0
    }
    assert!(sc.step_fired("late"));
    assert_eq!(sc.drain_sfx(), vec!["ping"]);
    assert!(sc.drain_sfx().is_empty(), "sfx drained once");
}

#[test]
fn enter_zone_spawns_wave_and_sets_objective() {
    let mut world = world_for(&T_FLOOR);
    let mut sc = ScenarioState::new(&T_FLOOR);
    sc.tick(&mut world, 0.016);
    assert_eq!(count_rogues(&world), (0, 1));
    move_player(&mut world, Vec2::new(650.0, 650.0));
    sc.tick(&mut world, 0.016);
    assert!(sc.step_fired("zone"));
    assert_eq!(count_rogues(&world), (0, 3));
    assert_eq!(sc.objective, "wave");
    // Staying in the zone does not re-fire the step.
    sc.tick(&mut world, 0.016);
    assert_eq!(count_rogues(&world), (0, 3));
}

#[test]
fn kills_opens_exit_and_exit_open_chains_all_dead_closes() {
    let mut world = world_for(&T_FLOOR);
    let mut sc = ScenarioState::new(&T_FLOOR);
    sc.tick(&mut world, 0.016);
    move_player(&mut world, Vec2::new(650.0, 650.0)); // wave -> 3 rogues
    sc.tick(&mut world, 0.016);
    assert!(!exit_open(&world, "a"));
    // Kill one rogue -> `kills 1` opens A, and `exit_open a` chains in the
    // same tick.
    let first = world.query::<Enemy>()[0];
    world
        .get_component_mut::<Health>(first)
        .unwrap()
        .take_damage(9999);
    sc.tick(&mut world, 0.016);
    assert!(exit_open(&world, "a"));
    assert!(sc.step_fired("after_open"));
    assert_eq!(sc.objective, "go");
    assert!(sc.drain_sfx().contains(&"elevator"));
    assert!(!exit_open(&world, "b"));
    // Everything dead -> B opens and A closes.
    kill_all(&mut world);
    sc.tick(&mut world, 0.016);
    assert!(sc.step_fired("clear"));
    assert!(exit_open(&world, "b"));
    assert!(!exit_open(&world, "a"));
    assert_eq!(sc.opened_exits(), &["a", "b"]);
}

const COMBAT_FLOOR: FloorDef = FloorDef {
    scenario: &[
        StepDef {
            id: "walk_in",
            trigger: Trigger::Start,
            actions: &[Action::Combat(false)],
        },
        StepDef {
            id: "fight",
            trigger: Trigger::Timer {
                seconds: 5.0,
                after: None,
            },
            actions: &[Action::Combat(true)],
        },
    ],
    ..T_FLOOR
};

#[test]
fn combat_action_toggles_fighting() {
    let mut world = world_for(&COMBAT_FLOOR);
    let mut sc = ScenarioState::new(&COMBAT_FLOOR);
    assert!(sc.combat_enabled(), "fighting is on by default");
    sc.tick(&mut world, 0.016);
    assert!(!sc.combat_enabled(), "the walk-in beat turned it off");
    for _ in 0..400 {
        sc.tick(&mut world, 0.016);
    }
    assert!(sc.combat_enabled(), "the 5s timer step turned it back on");
}

const ANCHOR_FLOOR: FloorDef = FloorDef {
    scenario: &[StepDef {
        id: "tut",
        trigger: Trigger::Start,
        actions: &[
            Action::Spawn(&[SpawnDef::hostile(333.0, 222.0, EnemyType::Idle)]),
            Action::Gate(GateDef {
                input: GateInput::Punch,
                text: "LEFT CLICK — PUNCH",
            }),
        ],
    }],
    ..T_FLOOR
};

#[test]
fn gate_anchors_on_its_spawn_and_tethers_the_player() {
    let mut world = world_for(&ANCHOR_FLOOR);
    let mut sc = ScenarioState::new(&ANCHOR_FLOOR);
    sc.tick(&mut world, 0.016);
    let anchor = sc.gate_anchor().expect("the gate anchored on its spawn");
    assert_eq!((anchor.x, anchor.y), (333.0, 222.0));
    // Drag the player far away: the tether's invisible walls clamp them
    // back onto the circle.
    let player = *world.query::<Player>().first().unwrap();
    if let Some(p) = world.get_component_mut::<Position>(player) {
        p.x = anchor.x + 500.0;
        p.y = anchor.y;
    }
    tether_player(&mut world, anchor, GATE_TETHER_RADIUS);
    let p = world.get_component::<Position>(player).unwrap();
    assert!((p.x - (anchor.x + GATE_TETHER_RADIUS)).abs() < 0.01);
    assert_eq!(p.y, anchor.y);
    // Inside the circle nothing moves.
    if let Some(p) = world.get_component_mut::<Position>(player) {
        p.x = anchor.x + 50.0;
    }
    tether_player(&mut world, anchor, GATE_TETHER_RADIUS);
    let p = world.get_component::<Position>(player).unwrap();
    assert_eq!(p.x, anchor.x + 50.0);
}

const LEGACY_FLOOR: FloorDef = FloorDef {
    scenario: &[StepDef {
        id: "intro",
        trigger: Trigger::Start,
        actions: &[Action::Say(SayDef {
            who: "SENTINEL",
            text: "hey",
            delay: 0.0,
        })],
    }],
    ..T_FLOOR
};

#[test]
fn floor_without_exit_opener_opens_all_exits_on_all_dead() {
    assert!(!LEGACY_FLOOR.has_exit_opener());
    assert!(T_FLOOR.has_exit_opener());
    let mut world = world_for(&LEGACY_FLOOR);
    let mut sc = ScenarioState::new(&LEGACY_FLOOR);
    sc.tick(&mut world, 0.016);
    assert!(!exit_open(&world, "a") && !exit_open(&world, "b"));
    kill_all(&mut world);
    sc.tick(&mut world, 0.016);
    assert!(exit_open(&world, "a") && exit_open(&world, "b"));
    assert_eq!(sc.opened_exits().len(), 2);
}

// A floor with NO initial rogues: `start` spawns the wave and `all_dead`
// is listed BEFORE it, so a naive single-pass evaluation would fire
// `all_dead` (0 alive) on the very first tick, before the spawn lands.
const WAVE_FIRST_STEPS: [StepDef; 3] = [
    StepDef {
        id: "clear",
        trigger: Trigger::AllDead,
        actions: &[Action::OpenExit("a")],
    },
    StepDef {
        id: "intro",
        trigger: Trigger::Start,
        actions: &[Action::Spawn(&T_WAVE)],
    },
    StepDef {
        id: "first_blood",
        trigger: Trigger::Kills(1),
        actions: &[Action::Objective("one down")],
    },
];
const WAVE_FIRST_FLOOR: FloorDef = FloorDef {
    spawns: &[],
    scenario: &WAVE_FIRST_STEPS,
    ..T_FLOOR
};

#[test]
fn same_tick_spawn_is_counted_before_all_dead_and_kills() {
    let mut world = world_for(&WAVE_FIRST_FLOOR);
    let mut sc = ScenarioState::new(&WAVE_FIRST_FLOOR);
    assert_eq!(count_rogues(&world), (0, 0));
    sc.tick(&mut world, 0.016);
    assert!(sc.step_fired("intro"));
    assert_eq!(count_rogues(&world), (0, 2), "the wave spawned");
    assert!(
        !sc.step_fired("clear"),
        "all_dead must not fire in the tick that spawned the wave"
    );
    assert!(!sc.step_fired("first_blood"));
    assert!(!exit_open(&world, "a"));
    for _ in 0..10 {
        sc.tick(&mut world, 0.1);
    }
    assert!(!sc.step_fired("clear"));
    // Kill one -> kills(1); kill all -> all_dead, in later ticks.
    let first = world.query::<Enemy>()[0];
    world
        .get_component_mut::<Health>(first)
        .unwrap()
        .take_damage(9999);
    sc.tick(&mut world, 0.016);
    assert!(sc.step_fired("first_blood"));
    assert!(!sc.step_fired("clear"));
    kill_all(&mut world);
    sc.tick(&mut world, 0.016);
    assert!(sc.step_fired("clear"));
    assert!(exit_open(&world, "a"));
}

// Legacy floor (no exit opener) with no initial rogues and a start wave:
// the auto-open must also see the spawn first.
const LEGACY_WAVE_FLOOR: FloorDef = FloorDef {
    spawns: &[],
    scenario: &[StepDef {
        id: "intro",
        trigger: Trigger::Start,
        actions: &[Action::Spawn(&T_WAVE)],
    }],
    ..T_FLOOR
};

#[test]
fn legacy_auto_open_waits_for_same_tick_spawns() {
    let mut world = world_for(&LEGACY_WAVE_FLOOR);
    let mut sc = ScenarioState::new(&LEGACY_WAVE_FLOOR);
    sc.tick(&mut world, 0.016);
    assert_eq!(count_rogues(&world), (0, 2));
    assert!(!exit_open(&world, "a") && !exit_open(&world, "b"));
    kill_all(&mut world);
    sc.tick(&mut world, 0.016);
    assert!(exit_open(&world, "a") && exit_open(&world, "b"));
}

const ENDING_STEPS: [StepDef; 3] = [
    StepDef {
        id: "boss_down",
        trigger: Trigger::BossDead,
        actions: &[Action::OpenExit("a"), Action::Objective("ride")],
    },
    StepDef {
        id: "uplink",
        trigger: Trigger::Extracted,
        actions: &[Action::Say(SayDef {
            who: "UPLINK",
            text: "carrier",
            delay: 0.0,
        })],
    },
    StepDef {
        id: "never",
        trigger: Trigger::AllDead,
        actions: &[Action::Objective("all dead")],
    },
];
const ENDING_FLOOR: FloorDef = FloorDef {
    spawns: &[],
    scenario: &ENDING_STEPS,
    ..T_FLOOR
};

#[test]
fn boss_dead_and_extracted_triggers() {
    use crate::components::{Boss, Enemy, Radius};
    use crate::ecs::System;
    let mut world = world_for(&ENDING_FLOOR);
    // A boss (an Enemy that carries the Boss marker) plus one plain rogue,
    // so `all_dead` stays false once the boss alone is down.
    let boss = world.spawn();
    world.add_component(boss, Enemy);
    world.add_component(boss, Boss::new());
    world.add_component(boss, Position::new(500.0, 400.0));
    world.add_component(boss, Health::new(100));
    world.add_component(boss, Radius::new(40.0));
    spawn_enemy_with_type(&mut world, Vec2::new(300.0, 300.0), EnemyType::Idle);
    let mut sc = ScenarioState::new(&ENDING_FLOOR);
    sc.tick(&mut world, 0.016);
    assert!(!sc.step_fired("boss_down"));
    assert!(!sc.step_fired("never"));
    world
        .get_component_mut::<Health>(boss)
        .unwrap()
        .take_damage(9999);
    sc.tick(&mut world, 0.016);
    assert!(
        sc.step_fired("boss_down"),
        "boss_dead fires once the boss dies"
    );
    assert!(!sc.step_fired("never"), "a plain rogue is still alive");
    assert!(exit_open(&world, "a"));
    assert_eq!(sc.objective, "ride");
    // Standing in the open exit for the dwell time = extraction -> uplink.
    assert!(!sc.step_fired("uplink"));
    move_player(&mut world, Vec2::new(500.0, 50.0));
    let mut lift = ElevatorSystem;
    for _ in 0..40 {
        lift.run(&mut world, 1.0 / 60.0);
        sc.tick(&mut world, 1.0 / 60.0);
    }
    assert!(sc.step_fired("uplink"));
    assert_eq!(sc.comms.visible()[0].who, "UPLINK");
    assert_eq!(speaker_rgb("UPLINK"), (200, 255, 222));
}

// A cold-open style floor: a gate to arrive through, a door out, an
// asphalt lot, a passive crowd strolling to the forecourt, and the
// `hold` / `look_at` / `alert` beats.
const C_ZONES: [ZoneDef; 2] = [
    ZoneDef {
        id: "forecourt",
        rect: Rect::new(380.0, 90.0, 240.0, 90.0),
    },
    ZoneDef {
        id: "lot",
        rect: Rect::new(180.0, 200.0, 640.0, 480.0),
    },
];
const C_SPAWNS: [SpawnDef; 3] = [
    SpawnDef {
        x: 300.0,
        y: 560.0,
        kind: EnemyType::Wandering,
        passive: true,
        walk_to: Some("forecourt"),
        face: Some(-90.0),
        group: Some("crowd"),
        unarmed: false,
    },
    SpawnDef {
        x: 700.0,
        y: 600.0,
        kind: EnemyType::Patrolling,
        passive: true,
        walk_to: Some("forecourt"),
        face: None,
        group: Some("crowd"),
        unarmed: false,
    },
    SpawnDef {
        x: 500.0,
        y: 400.0,
        kind: EnemyType::Idle,
        passive: true,
        walk_to: None,
        face: None,
        group: Some("valet"),
        unarmed: false,
    },
];
const C_EXITS: [ElevatorDef; 1] = [ElevatorDef {
    id: "doors",
    rect: Rect::new(440.0, 20.0, 120.0, 50.0),
    label: "MAIN DOORS",
    to: 1,
    open: true,
    kind: ElevatorKind::Door,
}];
const C_STEPS: [StepDef; 5] = [
    StepDef {
        id: "scan",
        trigger: Trigger::Start,
        actions: &[
            Action::Hold(HoldDef {
                seconds: 1.5,
                text: Some("SCANNING…"),
                until_comms_idle: false,
            }),
            Action::LookAt(LookAtDef {
                x: 500.0,
                y: 45.0,
                seconds: 3.0,
            }),
        ],
    },
    StepDef {
        id: "valet",
        trigger: Trigger::Timer {
            seconds: 2.0,
            after: None,
        },
        actions: &[Action::Alert(AlertTarget::Group("valet"))],
    },
    StepDef {
        id: "lot",
        trigger: Trigger::EnterZone {
            zone: "lot",
            before: None,
        },
        actions: &[Action::Alert(AlertTarget::Zone("lot"))],
    },
    StepDef {
        id: "turn",
        trigger: Trigger::Timer {
            seconds: 30.0,
            after: None,
        },
        actions: &[Action::Alert(AlertTarget::All)],
    },
    StepDef {
        id: "briefing",
        trigger: Trigger::Timer {
            seconds: 40.0,
            after: None,
        },
        actions: &[
            Action::Say(SayDef {
                who: "CL4-UD3",
                text: "a fairly long line so the feed stays busy for a while",
                delay: 0.0,
            }),
            Action::Hold(HoldDef {
                seconds: 60.0,
                text: None,
                until_comms_idle: true,
            }),
        ],
    },
];
const C_FLOOR: FloorDef = FloorDef {
    id: 0,
    name: "GATE",
    theme: "T",
    accent: "#8fd3ff",
    flavor: "",
    objective: "cross",
    width: 1000.0,
    height: 800.0,
    entry: ElevatorDef {
        id: "entry",
        rect: Rect::new(440.0, 720.0, 120.0, 60.0),
        label: "MAIN GATE",
        to: SURFACE_EXIT,
        open: false,
        kind: ElevatorKind::Gate,
    },
    exits: &C_EXITS,
    walls: &[],
    rooms: &[],
    zones: &C_ZONES,
    spawns: &C_SPAWNS,
    pickups: &[],
    scenario: &C_STEPS,
    surface: Surface::Asphalt,

    props: &[],
};

fn passives_left(world: &World) -> usize {
    crate::systems::passive::count_passives(world)
}

#[test]
fn hold_locks_for_its_seconds_and_shows_its_caption() {
    let mut world = world_for(&C_FLOOR);
    let mut sc = ScenarioState::new(&C_FLOOR);
    assert!(!sc.hold_active());
    sc.tick(&mut world, 1.0 / 60.0);
    assert!(sc.hold_active());
    assert_eq!(sc.hold_caption(), Some("SCANNING…"));
    // Still held just before 1.5 s...
    for _ in 0..80 {
        sc.tick(&mut world, 1.0 / 60.0);
    }
    assert!(sc.hold_active(), "held at {:.2}s", sc.time());
    // ...released right after.
    for _ in 0..12 {
        sc.tick(&mut world, 1.0 / 60.0);
    }
    assert!(!sc.hold_active(), "released at {:.2}s", sc.time());
    assert_eq!(sc.hold_caption(), None);
}

#[test]
fn hold_until_comms_idle_releases_when_the_feed_idles_and_is_capped() {
    let mut world = world_for(&C_FLOOR);
    let mut sc = ScenarioState::new(&C_FLOOR);
    // Jump to the briefing step (t = 40 s).
    while sc.time() < 40.5 {
        sc.tick(&mut world, 0.25);
    }
    assert!(sc.step_fired("briefing"));
    assert!(sc.hold_active());
    assert_eq!(sc.hold_caption(), None);
    // The line takes ~1.4 s to type (+ gap); the hold outlives it by
    // nothing: released once the feed is idle.
    let mut released_at = None;
    for _ in 0..600 {
        sc.tick(&mut world, 1.0 / 60.0);
        if !sc.hold_active() {
            released_at = Some(sc.time());
            break;
        }
    }
    let t = released_at.expect("hold released when the feed idled");
    assert!(
        t - 40.5 > 1.0 && t - 40.5 < 3.0,
        "released after {:.2}s",
        t - 40.5
    );

    // The cap: an until_comms_idle hold with a never-idle feed still ends
    // at HOLD_COMMS_IDLE_CAP (the 60 s asked for is clamped).
    let mut sc = ScenarioState::new(&C_FLOOR);
    let mut world = world_for(&C_FLOOR);
    while sc.time() < 40.5 {
        sc.tick(&mut world, 0.25);
    }
    // Keep the feed busy by re-queueing lines by hand.
    let mut end = None;
    for _ in 0..(HOLD_COMMS_IDLE_CAP as usize + 5) * 4 {
        sc.comms
            .enqueue("CL4-UD3", "still talking, still talking", sc.time());
        sc.tick(&mut world, 0.25);
        if !sc.hold_active() {
            end = Some(sc.time());
            break;
        }
    }
    let end = end.expect("capped hold ends");
    assert!(
        (end - 40.5 - HOLD_COMMS_IDLE_CAP).abs() < 0.6,
        "capped at {HOLD_COMMS_IDLE_CAP}s, ended after {:.2}s",
        end - 40.5
    );
}

#[test]
fn look_at_eases_in_holds_and_eases_out() {
    assert_eq!(look_at_weight(-1.0, 3.0), 0.0);
    assert_eq!(look_at_weight(0.0, 3.0), 0.0);
    assert!((look_at_weight(LOOK_AT_EASE_SECS, 3.0) - 1.0).abs() < 1e-5);
    assert!((look_at_weight(1.5, 3.0) - 1.0).abs() < 1e-5);
    let half = look_at_weight(LOOK_AT_EASE_SECS / 2.0, 3.0);
    assert!((half - 0.5).abs() < 1e-5, "smoothstep midpoint, got {half}");
    assert!(look_at_weight(3.0 - LOOK_AT_EASE_SECS / 2.0, 3.0) < 0.6);
    assert_eq!(look_at_weight(3.0, 3.0), 0.0);
    // Short looks still peak (ramps shrink to half the duration).
    assert!((look_at_weight(0.25, 0.5) - 1.0).abs() < 1e-5);

    let mut world = world_for(&C_FLOOR);
    let mut sc = ScenarioState::new(&C_FLOOR);
    assert!(sc.look_at().is_none());
    sc.tick(&mut world, 1.0 / 60.0);
    let (p, w) = sc.look_at().expect("look running");
    assert_eq!(p, Vec2::new(500.0, 45.0));
    assert!(w < 0.05);
    for _ in 0..60 {
        sc.tick(&mut world, 1.0 / 60.0);
    }
    let (_, w) = sc.look_at().unwrap();
    assert!(
        (w - 1.0).abs() < 1e-4,
        "fully on the point after 1 s, got {w}"
    );
    while sc.time() < 3.2 {
        sc.tick(&mut world, 1.0 / 60.0);
    }
    assert!(sc.look_at().is_none(), "look over after its seconds");
}

#[test]
fn alert_actions_flip_group_zone_and_all() {
    let mut world = world_for(&C_FLOOR);
    let mut sc = ScenarioState::new(&C_FLOOR);
    assert_eq!(passives_left(&world), 3);
    sc.tick(&mut world, 1.0 / 60.0);
    assert_eq!(passives_left(&world), 3, "no alert at start");
    // t = 2 s: the valet group turns.
    while sc.time() < 2.1 {
        sc.tick(&mut world, 1.0 / 60.0);
    }
    assert!(sc.step_fired("valet"));
    assert_eq!(passives_left(&world), 2);
    // Walk into the lot zone: only the passives standing in it flip; the
    // crowd is (still) down in the lot at t~2 s (they stroll at 55 px/s
    // and start at y 560 / 600, inside `lot`), so both flip.
    move_player(&mut world, Vec2::new(500.0, 400.0));
    sc.tick(&mut world, 1.0 / 60.0);
    assert!(sc.step_fired("lot"));
    assert_eq!(passives_left(&world), 0);
    for e in world.query::<Enemy>() {
        let ai = world.get_component::<AI>(e).unwrap();
        assert_eq!(ai.state, AIState::SurePlayerSeen);
    }
    // Alerted passives are rogues: killing them all fires all_dead-style
    // counts like any floor.
    assert_eq!(count_rogues(&world), (0, 3));
    kill_all(&mut world);
    assert_eq!(count_rogues(&world), (3, 0));
}

#[test]
fn alert_all_from_a_fresh_floor() {
    let mut world = world_for(&C_FLOOR);
    let mut sc = ScenarioState::new(&C_FLOOR);
    // Nobody entered the lot; at t = 30 s everyone turns.
    while sc.time() < 30.5 {
        sc.tick(&mut world, 0.5);
    }
    assert!(sc.step_fired("turn"));
    assert_eq!(passives_left(&world), 0);
}

// A cold-open style conversation: two talk lines in one step, and a
// doors-style timer counting from the conversation's END.
const TALK_STEPS: [StepDef; 3] = [
    StepDef {
        id: "conv",
        trigger: Trigger::Start,
        actions: &[
            Action::Talk(TalkDef {
                who: "SENTINEL",
                text: "STATE PURPOSE.",
            }),
            Action::Talk(TalkDef {
                who: "CL4-UD3",
                text: "Maintenance.",
            }),
        ],
    },
    StepDef {
        id: "doors",
        trigger: Trigger::Timer {
            seconds: 0.5,
            after: Some("conv"),
        },
        actions: &[Action::Sfx("elevator")],
    },
    StepDef {
        id: "plain",
        trigger: Trigger::Timer {
            seconds: 0.2,
            after: Some("doors"),
        },
        actions: &[Action::Objective("after doors")],
    },
];
const TALK_FLOOR: FloorDef = FloorDef {
    spawns: &[],
    scenario: &TALK_STEPS,
    ..T_FLOOR
};

#[test]
fn talk_lines_form_one_player_paced_conversation() {
    let mut world = world_for(&TALK_FLOOR);
    let mut sc = ScenarioState::new(&TALK_FLOOR);
    assert!(!sc.dialogue_active());
    sc.tick(&mut world, 1.0 / 60.0);
    assert!(sc.step_fired("conv"));
    assert!(sc.dialogue_active(), "the conversation opened");
    let v = sc.dialogue_view().unwrap();
    assert_eq!(v.who, "SENTINEL");
    assert_eq!(v.text, "STATE PURPOSE.");
    assert!(v.more, "a second line is queued");
    assert!(v.slide < 0.5, "the panel is still sliding in");
    // The panel finishes sliding in and the line types out.
    for _ in 0..60 {
        sc.tick(&mut world, 1.0 / 60.0);
    }
    let v = sc.dialogue_view().unwrap();
    assert!((v.slide - 1.0).abs() < 1e-4);
    assert!(v.fully_typed);
    // Player-paced: no amount of waiting advances or dismisses it.
    for _ in 0..600 {
        sc.tick(&mut world, 1.0 / 60.0);
    }
    assert_eq!(sc.dialogue_view().unwrap().who, "SENTINEL");
    assert!(
        !sc.step_fired("doors"),
        "the after-conv timer must not run while the conversation is up"
    );
    // Advance -> second line, typewriter restarted.
    sc.dialogue_advance();
    let v = sc.dialogue_view().unwrap();
    assert_eq!(v.who, "CL4-UD3");
    assert_eq!(v.chars_shown, 0);
    assert!(!v.more);
    // A press mid-typing reveals the whole line instead of advancing.
    sc.tick(&mut world, 1.0 / 60.0);
    assert!(!sc.dialogue_view().unwrap().fully_typed);
    sc.dialogue_advance();
    let v = sc.dialogue_view().unwrap();
    assert_eq!(v.who, "CL4-UD3");
    assert!(v.fully_typed);
    // Dismiss -> the panel slides out, input stays locked until gone.
    sc.dialogue_advance();
    assert!(sc.dialogue_active());
    for _ in 0..30 {
        sc.tick(&mut world, 1.0 / 60.0);
    }
    assert!(!sc.dialogue_active(), "panel gone after the slide-out");
}

#[test]
fn timer_after_talk_step_counts_from_the_conversation_end() {
    let mut world = world_for(&TALK_FLOOR);
    let mut sc = ScenarioState::new(&TALK_FLOOR);
    sc.tick(&mut world, 1.0 / 60.0);
    // Let a lot of time pass, then click through both lines.
    for _ in 0..300 {
        sc.tick(&mut world, 1.0 / 60.0);
    }
    sc.dialogue_advance(); // line 2 (line 1 fully typed long ago)
    for _ in 0..60 {
        sc.tick(&mut world, 1.0 / 60.0);
    }
    sc.dialogue_advance(); // dismiss
    let mut end = None;
    for _ in 0..60 {
        sc.tick(&mut world, 1.0 / 60.0);
        if !sc.dialogue_active() {
            end = Some(sc.time());
            break;
        }
    }
    let end = end.expect("conversation ended");
    assert!(!sc.step_fired("doors"), "0.5s not yet elapsed at {end}");
    while sc.time() < end + 0.6 {
        sc.tick(&mut world, 1.0 / 60.0);
    }
    assert!(
        sc.step_fired("doors"),
        "doors fires 0.5s after the conversation ended"
    );
    // A timer after a step WITHOUT talk still counts from its fire time.
    for _ in 0..20 {
        sc.tick(&mut world, 1.0 / 60.0);
    }
    assert!(sc.step_fired("plain"));
}

#[test]
fn same_tick_talk_steps_merge_into_one_conversation() {
    // Two start steps both talking: their lines join one conversation and
    // BOTH steps get their talk-done time when it ends.
    const MERGE_STEPS: [StepDef; 3] = [
        StepDef {
            id: "a",
            trigger: Trigger::Start,
            actions: &[Action::Talk(TalkDef {
                who: "SWARM",
                text: "one",
            })],
        },
        StepDef {
            id: "b",
            trigger: Trigger::Start,
            actions: &[Action::Talk(TalkDef {
                who: "CL4-UD3",
                text: "two",
            })],
        },
        StepDef {
            id: "after_b",
            trigger: Trigger::Timer {
                seconds: 0.1,
                after: Some("b"),
            },
            actions: &[Action::Objective("done")],
        },
    ];
    const MERGE_FLOOR: FloorDef = FloorDef {
        spawns: &[],
        scenario: &MERGE_STEPS,
        ..T_FLOOR
    };
    let mut world = world_for(&MERGE_FLOOR);
    let mut sc = ScenarioState::new(&MERGE_FLOOR);
    sc.tick(&mut world, 1.0 / 60.0);
    let v = sc.dialogue_view().unwrap();
    assert_eq!(v.who, "SWARM");
    assert!(v.more, "step b's line queued behind step a's");
    for _ in 0..30 {
        sc.tick(&mut world, 1.0 / 60.0);
    }
    sc.dialogue_advance();
    assert_eq!(sc.dialogue_view().unwrap().who, "CL4-UD3");
    for _ in 0..30 {
        sc.tick(&mut world, 1.0 / 60.0);
    }
    assert!(!sc.step_fired("after_b"));
    sc.dialogue_advance();
    for _ in 0..60 {
        sc.tick(&mut world, 1.0 / 60.0);
    }
    assert!(!sc.dialogue_active());
    assert!(
        sc.step_fired("after_b"),
        "b's talk-done time was recorded when the merged conversation ended"
    );
}

#[test]
fn surface_and_portal_kind_parse() {
    assert_eq!(Surface::parse("checker"), Some(Surface::Checker));
    assert_eq!(Surface::parse("asphalt"), Some(Surface::Asphalt));
    assert_eq!(Surface::parse("marble"), Some(Surface::Marble));
    assert_eq!(Surface::parse("concrete"), Some(Surface::Concrete));
    assert_eq!(Surface::parse("grating"), Some(Surface::Grating));
    assert_eq!(Surface::parse("lava"), None);
    assert_eq!(ElevatorKind::parse("lift"), Some(ElevatorKind::Lift));
    assert_eq!(ElevatorKind::parse("door"), Some(ElevatorKind::Door));
    assert_eq!(ElevatorKind::parse("gate"), Some(ElevatorKind::Gate));
    assert_eq!(ElevatorKind::parse("hatch"), None);
    assert_eq!(C_FLOOR.surface, Surface::Asphalt);
}

#[test]
fn door_exits_and_gate_entry_extract_like_lifts() {
    // A `door` exit is an ordinary portal for the elevator system: the
    // kind is rendering only. It carries through to the world entity.
    let mut world = world_for(&C_FLOOR);
    let doors = world
        .query::<Elevator>()
        .into_iter()
        .filter_map(|e| world.get_component::<Elevator>(e).copied())
        .collect::<Vec<_>>();
    let entry = doors.iter().find(|e| !e.is_exit).unwrap();
    let exit = doors.iter().find(|e| e.is_exit).unwrap();
    assert_eq!(entry.kind, ElevatorKind::Gate);
    assert_eq!(exit.kind, ElevatorKind::Door);
    assert!(exit.open && exit.to == 1);
    move_player(&mut world, Vec2::new(500.0, 45.0));
    use crate::ecs::System;
    let mut lift = ElevatorSystem;
    for _ in 0..40 {
        lift.run(&mut world, 1.0 / 60.0);
    }
    assert_eq!(ElevatorSystem::extraction(&world), Some(1));
}

#[test]
fn surface_exit_sentinel_is_never_a_floor_id() {
    assert_ne!(SURFACE_EXIT, 0, "floor 0 is the parking lot now");
    assert!(crate::levels::level_index_for_floor_id(SURFACE_EXIT).is_none());
    // 13½'s car goes to the surface; nothing else does.
    let boss = crate::levels::floor_def(crate::levels::BOSS_LEVEL);
    assert!(boss.exits.iter().all(|e| e.to == SURFACE_EXIT));
}
