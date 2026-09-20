//! The RENDER layer, headless: record real draw code through
//! `Graphics::new_headless` and validate the command stream renderer.js would
//! receive (`graphics::stream::check`: decodes against the arity table,
//! finite floats, balanced SAVE/RESTORE and pixel groups, framed static
//! sections holding solid primitives only, valid TEXT indices).
//!
//! The browser suites prove the PIXELS on a few frames; this proves the
//! STREAM on every floor and every prop, in well under a second.

use open_miami::camera::{Camera, ViewCull};
use open_miami::components::{Enemy, Headless, Health, WeaponType};
use open_miami::ecs::World;
use open_miami::game::{get_player_position, initialize_game};
use open_miami::graphics::stream::{check, Cmd};
use open_miami::graphics::{op, Graphics};
use open_miami::hud_msg::MsgRoller;
use open_miami::level::Level;
use open_miami::levels::{floor_def, LEVEL_COUNT};
use open_miami::math::Vec2;
use open_miami::props::{draw_prop, draw_prop_ex, PropDrawOpts, MAX_PX, PROP_COUNT, PROP_NAMES};
use open_miami::render::floor_props::render_floor_props;
use open_miami::render::hud::{render_hud, ExtractCard, HudView};
use open_miami::render::world::{render_world, WorldView, KILL_FLASH_SECS};
use open_miami::scenario::ScenarioState;
use open_miami::sparks::SparkPool;
use open_miami::static_geo::{OP_STATIC_BEGIN, OP_STATIC_END, OP_STATIC_REF};

const VIEW: (f32, f32) = (1280.0, 800.0);

/// What varies between the frames under test (the rest of a `WorldView` is
/// derived from the floor).
#[derive(Clone, Copy, Default)]
struct Shot {
    now: f32,
    pixel_world: u32,
    kill_flash: Option<f32>,
    show_infos: bool,
    player_firing: bool,
}

/// A floor as the game sets it up, with a corpse and a headless corpse so
/// the actors layer draws its prone pass too.
struct Stage {
    level: usize,
    world: World,
    lvl: Level,
    sparks: SparkPool,
}

impl Stage {
    fn new(level: usize) -> Self {
        let def = floor_def(level);
        let mut world = World::new();
        initialize_game(&mut world, level);
        let enemies = world.query::<Enemy>();
        for (n, &e) in enemies.iter().take(2).enumerate() {
            if let Some(h) = world.get_component_mut::<Health>(e) {
                h.current = 0;
            }
            if n == 1 {
                world.add_component(e, Headless);
            }
        }
        let mut lvl = Level::new();
        lvl.set_surface(def.surface);
        lvl.set_size(def.width, def.height);
        let mut sparks = SparkPool::new();
        sparks.spawn(get_player_position(&world).unwrap_or(Vec2::zero()), 0.0);
        Stage {
            level,
            world,
            lvl,
            sparks,
        }
    }

    /// Record one frame through the REAL `render_world`.
    fn record(&self, g: &Graphics, shot: Shot) {
        let focus = get_player_position(&self.world).unwrap_or(Vec2::zero());
        let mut camera = Camera::new();
        camera.set_viewport(VIEW.0, VIEW.1);
        camera.follow_player(focus);
        camera.update_sway(shot.now);
        let view = WorldView {
            world: &self.world,
            level: &self.lvl,
            camera: &camera,
            sparks: &self.sparks,
            props: floor_def(self.level).props,
            gate_anchor: Some(focus),
            now: shot.now,
            pixel_world: shot.pixel_world,
            kill_flash: shot.kill_flash,
            show_infos: shot.show_infos,
            floor_static_key: self.level as u32 + 1,
            accent: (255, 80, 160),
            player_firing: shot.player_firing,
        };
        render_world(g, &view);
    }
}

fn count(cmds: &[Cmd<'_>], opcode: f32) -> usize {
    cmds.iter().filter(|c| c.op == opcode).count()
}

fn checked<'a>(frame: &'a open_miami::graphics::Frame, what: &str) -> Vec<Cmd<'a>> {
    check(&frame.cmds, &frame.texts).unwrap_or_else(|e| panic!("{what}: {e}"))
}

#[test]
fn every_floor_records_a_valid_frame() {
    for level in 0..LEVEL_COUNT {
        let stage = Stage::new(level);
        let g = Graphics::new_headless(VIEW.0, VIEW.1);

        // Frame 1 records the static section in full...
        stage.record(&g, Shot::default());
        let first = g.take_frame();
        let cmds = checked(&first, &format!("floor index {level}, frame 1"));
        assert_eq!(count(&cmds, OP_STATIC_BEGIN), 1, "floor index {level}");
        assert_eq!(count(&cmds, op::BACKDROP), 1);
        assert!(
            count(&cmds, op::ROBOT) >= 1,
            "floor index {level}: no player robot"
        );

        // ...frame 2 only references it: the cache is the command-stream win.
        stage.record(
            &g,
            Shot {
                now: 1.0 / 60.0,
                ..Shot::default()
            },
        );
        let second = g.take_frame();
        let cmds = checked(&second, &format!("floor index {level}, frame 2"));
        assert_eq!(count(&cmds, OP_STATIC_REF), 1);
        assert_eq!(count(&cmds, OP_STATIC_BEGIN), 0);
        assert!(
            second.cmds.len() < first.cmds.len(),
            "floor index {level}: the cached frame is not smaller ({} vs {})",
            second.cmds.len(),
            first.cmds.len()
        );

        // The bypass paths (debug overlays, the kill flash) skip the cache
        // and must be valid too.
        for shot in [
            Shot {
                show_infos: true,
                ..Shot::default()
            },
            Shot {
                kill_flash: Some(KILL_FLASH_SECS * 0.5),
                ..Shot::default()
            },
            Shot {
                kill_flash: Some(0.0),
                player_firing: true,
                ..Shot::default()
            },
        ] {
            stage.record(&g, shot);
            let frame = g.take_frame();
            let cmds = checked(&frame, &format!("floor index {level}, bypass frame"));
            assert_eq!(
                count(&cmds, OP_STATIC_REF) + count(&cmds, OP_STATIC_BEGIN),
                0
            );
        }
    }
}

#[test]
fn the_pixel_world_is_one_smooth_group_with_the_actors_outside_it() {
    for level in 0..LEVEL_COUNT {
        for px in [2, 3, 6] {
            let stage = Stage::new(level);
            let g = Graphics::new_headless(VIEW.0, VIEW.1);
            stage.record(
                &g,
                Shot {
                    pixel_world: px,
                    ..Shot::default()
                },
            );
            let frame = g.take_frame();
            let what = format!("floor index {level}, ?pixel={px}");
            // `check` also proves the props' own groups nest within PIX_DEPTH.
            let cmds = checked(&frame, &what);

            // The scenery group: the FIRST group, smooth, at the asked px...
            let begin = cmds
                .iter()
                .position(|c| c.op == op::PIX_BEGIN)
                .expect("a group");
            let args = cmds[begin].args;
            assert_eq!((args[0], args[3]), (px as f32, 1.0), "{what}");
            // ...a whole number of texels (the v-flip anchor)...
            assert_eq!((args[1] / args[0]).fract(), 0.0, "{what}: fractional width");
            assert_eq!(
                (args[2] / args[0]).fract(),
                0.0,
                "{what}: fractional height"
            );
            // ...holding the static floor, and closed before any robot draws.
            let mut depth = 0;
            let mut end = begin;
            for (n, c) in cmds.iter().enumerate().skip(begin) {
                depth += (c.op == op::PIX_BEGIN) as i32 - (c.op == op::PIX_END) as i32;
                if depth == 0 {
                    end = n;
                    break;
                }
            }
            let inside = &cmds[begin..end];
            assert_eq!(
                count(inside, OP_STATIC_BEGIN),
                1,
                "{what}: floor not in the group"
            );
            assert_eq!(
                count(inside, op::ROBOT),
                0,
                "{what}: a robot was re-quantized"
            );
            assert!(count(&cmds[end..], op::ROBOT) >= 1, "{what}");
        }
    }
}

#[test]
fn a_floors_static_section_is_camera_independent() {
    // The cache is recorded once and must hold for every camera position:
    // the section's floats may not depend on the sway clock.
    let section = |now: f32| {
        let g = Graphics::new_headless(VIEW.0, VIEW.1);
        Stage::new(1).record(
            &g,
            Shot {
                now,
                ..Shot::default()
            },
        );
        let cmds = g.take_frame().cmds;
        let begin = cmds.iter().position(|&v| v == OP_STATIC_BEGIN).unwrap();
        let len = cmds[begin..]
            .iter()
            .position(|&v| v == OP_STATIC_END)
            .expect("framed");
        cmds[begin..begin + len].to_vec()
    };
    assert_eq!(
        section(0.0),
        section(5.0),
        "the static section moved with the sway"
    );
}

#[test]
fn the_fire_input_only_changes_the_players_pose() {
    let stage = Stage::new(1);
    let g = Graphics::new_headless(VIEW.0, VIEW.1);
    stage.record(&g, Shot::default()); // warm the static cache: REF frames below
    g.take_frame();
    stage.record(&g, Shot::default());
    let idle = g.take_frame();
    stage.record(
        &g,
        Shot {
            player_firing: true,
            ..Shot::default()
        },
    );
    let firing = g.take_frame();
    assert_eq!(idle.cmds.len(), firing.cmds.len());
    let changed: Vec<usize> = (0..idle.cmds.len())
        .filter(|&i| idle.cmds[i] != firing.cmds[i])
        .collect();
    assert_eq!(
        changed.len(),
        1,
        "one float differs: the player's pose index"
    );
    assert_eq!(firing.cmds[changed[0]], 2.0, "ROBOT_POSE_SHOOT");
}

#[test]
fn every_prop_records_a_valid_frame_at_every_pixel_size() {
    assert_eq!(PROP_NAMES.len(), PROP_COUNT);
    for (kind, name) in PROP_NAMES.iter().enumerate() {
        for t in [0.0, 0.37, 12.5] {
            let g = Graphics::new_headless(VIEW.0, VIEW.1);
            draw_prop(&g, kind, Vec2::new(200.0, 200.0), 100.0, t);
            for px in 1..=MAX_PX {
                let opts = PropDrawOpts::saved(kind);
                draw_prop_ex(&g, kind, Vec2::new(400.0, 300.0), 137.0, t, px, &opts);
            }
            let frame = g.take_frame();
            let cmds = check(&frame.cmds, &frame.texts)
                .unwrap_or_else(|e| panic!("prop {kind} ({name}), t = {t}: {e}"));
            assert!(!cmds.is_empty(), "prop {kind} drew nothing");
            // Props are set dressing made of primitives: no sprites, no post.
            for banned in [op::ROBOT, op::SHOGGOTH, op::POSTFX, op::CLEAR] {
                assert_eq!(count(&cmds, banned), 0, "prop {kind} emitted op {banned}");
            }
        }
    }
}

#[test]
fn culled_props_cost_no_commands() {
    let g = Graphics::new_headless(VIEW.0, VIEW.1);
    let level = (0..LEVEL_COUNT)
        .find(|&l| !floor_def(l).props.is_empty())
        .expect("a floor with props");
    let nowhere = ViewCull::new(Vec2::new(-9e5, -9e5), Vec2::new(-8e5, -8e5));
    render_floor_props(&g, floor_def(level).props, 0.0, &nowhere);
    assert!(g.take_frame().cmds.is_empty());
}

/// A plain in-game HUD view: alive, a pistol, no scenario, grain on.
fn hud<'a>(roller: &'a MsgRoller, scenario: Option<&'a ScenarioState>) -> HudView<'a> {
    HudView {
        extract_card: None,
        ammo: 7,
        weapon: Some(WeaponType::Pistol),
        ammo_slide: 0.0,
        enemies_alive: 3,
        player_alive: true,
        death_time: 0.0,
        debug_enabled: false,
        show_infos: false,
        roller,
        scenario,
        restart_progress: None,
        cursor: Vec2::new(640.0, 400.0),
        tv_static: Some(0.04),
        accent: (255, 80, 160),
        now: 1.5,
    }
}

fn record_hud(v: &HudView) -> open_miami::graphics::Frame {
    let g = Graphics::new_headless(VIEW.0, VIEW.1);
    render_hud(&g, v);
    g.take_frame()
}

#[test]
fn every_floors_opening_hud_records_a_valid_frame() {
    // Each floor's scenario, run through its opening seconds: dialogue panels
    // (robot headshots, the swarm, the corruptor), hold captions, gate
    // prompts — whatever the floor opens on — over the HUD.
    for level in 0..LEVEL_COUNT {
        let mut world = World::new();
        initialize_game(&mut world, level);
        let mut sc = ScenarioState::new(floor_def(level));
        let mut roller = MsgRoller::new();
        let mut overlays = 0;
        for frame in 0..600 {
            sc.tick(&mut world, 1.0 / 60.0);
            roller.update(1.0 / 60.0, "PURGE THE FLOOR", 0);
            if frame % 20 != 0 {
                continue;
            }
            let mut view = hud(&roller, Some(&sc));
            view.now = frame as f32 / 60.0;
            let f = record_hud(&view);
            checked(&f, &format!("floor index {level}, HUD frame {frame}"));
            overlays += (sc.dialogue_view().is_some()
                || sc.hold_caption().is_some()
                || sc.gate_view().is_some()) as usize;
        }
        // Floor 0 opens on a conversation: make sure that path really ran.
        if level == 0 {
            assert!(overlays > 0, "floor 0 showed no scenario overlay in 10 s");
        }
    }
}

#[test]
fn the_hud_states_all_record_valid_frames() {
    let roller = MsgRoller::new();
    let dead = HudView {
        player_alive: false,
        death_time: 2.0,
        ..hud(&roller, None)
    };
    let debug = HudView {
        debug_enabled: true,
        show_infos: true,
        ..hud(&roller, None)
    };
    let unarmed = HudView {
        weapon: None,
        ammo: 0,
        ammo_slide: 1.0,
        ..hud(&roller, None)
    };
    let restarting = HudView {
        restart_progress: Some(0.6),
        ..hud(&roller, None)
    };
    for (what, view) in [
        ("dead", dead),
        ("debug", debug),
        ("unarmed", unarmed),
        ("restarting", restarting),
    ] {
        let f = record_hud(&view);
        checked(&f, what);
    }
    for home in [false, true] {
        let card = ExtractCard {
            floor_title: "FLOOR 3 // INFERENCE PIT",
            t: 1.2,
            alpha: 0.8,
            home,
        };
        let f = record_hud(&HudView {
            extract_card: Some(card),
            ..hud(&roller, None)
        });
        checked(&f, "extraction card");
        // The card REPLACES the HUD: no rogue counter under it.
        assert!(
            !f.texts.contains("ROGUES"),
            "the HUD drew under the card: {}",
            f.texts
        );
    }
    let f = record_hud(&hud(&roller, None));
    assert!(
        f.texts.contains("ROGUES"),
        "no rogue counter in: {}",
        f.texts
    );
    let f = record_hud(&HudView {
        restart_progress: Some(0.6),
        ..hud(&roller, None)
    });
    assert!(f.texts.contains("RESTARTING"));
}

#[test]
fn the_tv_static_is_the_frames_last_command_and_optional() {
    let roller = MsgRoller::new();
    let f = record_hud(&hud(&roller, None));
    let cmds = checked(&f, "grain on");
    let last = cmds.last().unwrap();
    // Emitted LAST so the outro's blur-out, recorded after it, replaces it.
    assert_eq!(
        (last.op, last.args[0], last.args[1]),
        (op::POSTFX, 13.0, 0.04)
    );
    let f = record_hud(&HudView {
        tv_static: None,
        ..hud(&roller, None)
    });
    assert_eq!(count(&checked(&f, "grain off"), op::POSTFX), 0);
}

#[test]
fn the_crosshair_follows_the_cursor_and_nothing_else_does() {
    let roller = MsgRoller::new();
    let a = record_hud(&hud(&roller, None));
    let b = record_hud(&HudView {
        cursor: Vec2::new(640.0 + 30.0, 400.0),
        ..hud(&roller, None)
    });
    assert_eq!(a.cmds.len(), b.cmds.len());
    let moved: Vec<usize> = (0..a.cmds.len())
        .filter(|&i| a.cmds[i] != b.cmds[i])
        .collect();
    // 6 + 6 cells of the 7x7 cross (empty centre), one x each.
    assert_eq!(moved.len(), 12, "only the crosshair's 12 rects move");
    assert!(moved.iter().all(|&i| b.cmds[i] - a.cmds[i] == 30.0));
}
