//! The RENDER layer, headless: record real draw code through
//! `Graphics::new_headless` and validate the command stream renderer.js would
//! receive (`graphics::stream::check`: decodes against the arity table,
//! finite floats, balanced SAVE/RESTORE and pixel groups, framed static
//! sections holding solid primitives only, valid TEXT indices).
//!
//! The browser suites prove the PIXELS on a few frames; this proves the
//! STREAM on every floor and every prop, in well under a second.

use open_miami::camera::{Camera, ViewCull};
use open_miami::ecs::World;
use open_miami::floor_props::render_floor_props;
use open_miami::game::{get_player_position, initialize_game};
use open_miami::graphics::stream::{check, Cmd};
use open_miami::graphics::{op, Graphics};
use open_miami::level::Level;
use open_miami::levels::{floor_def, LEVEL_COUNT};
use open_miami::math::Vec2;
use open_miami::props::{draw_prop, draw_prop_ex, PropDrawOpts, MAX_PX, PROP_COUNT, PROP_NAMES};
use open_miami::render::{render_entities, render_walls};
use open_miami::render_comms::{render_elevators, render_zones_debug};
use open_miami::static_geo::{OP_STATIC_BEGIN, OP_STATIC_REF};

const VIEW: (f32, f32) = (1280.0, 800.0);

/// One world frame of floor `level`, in `render_world`'s scenery order. With
/// `debug` the static cache is bypassed and the overlays draw (the I key).
fn record_floor(g: &Graphics, level: usize, world: &World, key: u32, t: f32, debug: bool) {
    let def = floor_def(level);
    let mut lvl = Level::new();
    lvl.set_surface(def.surface);
    lvl.set_size(def.width, def.height);
    let mut cam = Camera::new();
    cam.set_viewport(VIEW.0, VIEW.1);
    cam.follow_player(get_player_position(world).unwrap_or(Vec2::zero()));
    cam.update_sway(t);

    g.backdrop(
        VIEW.0,
        VIEW.1,
        t,
        6.0,
        cam.floor_occlusion(def.width, def.height, 2.0),
    );
    cam.apply(g);
    let (view_min, view_max) = cam.visible_bounds(VIEW.0, VIEW.1);
    if debug {
        lvl.render(g, view_min, view_max, None);
        render_walls(world, g, true);
    } else {
        let (floor_min, floor_max) = lvl.full_bounds();
        g.static_layer(key, || {
            lvl.render(g, floor_min, floor_max, None);
            render_walls(world, g, false);
        });
    }
    let cull = cam.view_cull(VIEW.0, VIEW.1);
    render_floor_props(g, def.props, t, &cull);
    render_elevators(world, g, (255, 80, 160), t);
    if debug {
        render_zones_debug(world, g);
    }
    render_entities(world, g, debug, t, &cull);
    cam.reset(g);
}

fn count(cmds: &[Cmd<'_>], opcode: f32) -> usize {
    cmds.iter().filter(|c| c.op == opcode).count()
}

#[test]
fn every_floor_records_a_valid_frame() {
    for level in 0..LEVEL_COUNT {
        let mut world = World::new();
        initialize_game(&mut world, level);
        let g = Graphics::new_headless(VIEW.0, VIEW.1);
        let key = level as u32 + 1;

        // Frame 1 records the static section in full...
        record_floor(&g, level, &world, key, 0.0, false);
        let first = g.take_frame();
        let cmds = check(&first.cmds, &first.texts)
            .unwrap_or_else(|e| panic!("floor index {level}, frame 1: {e}"));
        assert_eq!(count(&cmds, OP_STATIC_BEGIN), 1, "floor index {level}");
        assert_eq!(count(&cmds, op::BACKDROP), 1);

        // ...frame 2 only references it: the cache is the command-stream win.
        record_floor(&g, level, &world, key, 1.0 / 60.0, false);
        let second = g.take_frame();
        let cmds = check(&second.cmds, &second.texts)
            .unwrap_or_else(|e| panic!("floor index {level}, frame 2: {e}"));
        assert_eq!(count(&cmds, OP_STATIC_REF), 1);
        assert_eq!(count(&cmds, OP_STATIC_BEGIN), 0);
        assert!(
            second.cmds.len() < first.cmds.len(),
            "floor index {level}: the cached frame is not smaller ({} vs {})",
            second.cmds.len(),
            first.cmds.len()
        );

        // The debug-overlay path bypasses the cache and must be valid too.
        record_floor(&g, level, &world, key, 2.0 / 60.0, true);
        let debug = g.take_frame();
        let cmds = check(&debug.cmds, &debug.texts)
            .unwrap_or_else(|e| panic!("floor index {level}, debug frame: {e}"));
        assert_eq!(
            count(&cmds, OP_STATIC_REF) + count(&cmds, OP_STATIC_BEGIN),
            0
        );
    }
}

#[test]
fn a_floors_static_section_is_camera_independent() {
    // The cache is recorded once and must hold for every camera position:
    // the section's floats may not depend on where the player stands.
    let level = 1;
    let section = |t: f32| {
        let mut world = World::new();
        initialize_game(&mut world, level);
        let g = Graphics::new_headless(VIEW.0, VIEW.1);
        record_floor(&g, level, &world, 1, t, false);
        let frame = g.take_frame();
        let begin = frame
            .cmds
            .iter()
            .position(|&v| v == OP_STATIC_BEGIN)
            .unwrap();
        // The camera is `SAVE TRANSLATE ROTATE SCALE TRANSLATE` before it.
        frame.cmds[begin..].to_vec()
    };
    let (a, b) = (section(0.0), section(5.0));
    let end = a
        .iter()
        .position(|&v| v == open_miami::static_geo::OP_STATIC_END);
    let end = end.expect("framed section");
    assert_eq!(
        a[..end],
        b[..end],
        "the static section moved with the sway clock"
    );
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
