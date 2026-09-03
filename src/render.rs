// Rendering system for drawing entities
use crate::components::*;
use crate::ecs::{Entity, World};
use crate::graphics::Graphics;
use crate::math::{Color, Vec2};

/// Render all entities in the world except the upright player / rogue
/// bots — the game draws those as live 3D robots afterwards (the boss IS
/// drawn here, live too). `now` is the continuous animation clock in
/// seconds (drives the boss's writhing). `cull` = the camera's inflated view
/// rect: the expensive live sprites (ground guns, the boss) fully outside it
/// skip their commands — bullets / trails / debug overlays keep their own
/// cheap paths untouched.
pub fn render_entities(
    world: &World,
    graphics: &Graphics,
    show_infos: bool,
    now: f32,
    cull: &crate::camera::ViewCull,
) {
    // Render debug pathfinding info first (behind everything)
    if show_infos {
        render_debug_pathfinding(world, graphics);
    }

    // Render vision cones first (behind everything) - only if info display is enabled
    if show_infos {
        render_enemy_vision_cones(world, graphics);
    }

    // Render dropped weapon pickups (beneath actors)
    render_pickups(world, graphics, cull);

    // Render projectile trails
    render_projectile_trails(world, graphics);

    // Render bullets (tracer dashes; view-culled like the other sprites)
    render_bullets(world, graphics, cull);

    // Render weapons in flight
    render_thrown_weapons(world, graphics, cull);

    // Render the boss (big; under the player)
    render_bosses(world, graphics, now, cull);
}

/// On-screen size (px) of the boss's live tile relative to its hitbox radius.
/// shoggoth-core frames 3.8 world units per half-tile and the mass core is
/// 1.65 units, so 4.6x the radius puts the core right over the hitbox and
/// leaves the rest of the tile to the lobes and tentacles.
const BOSS_TILE_PER_RADIUS: f32 = 4.6;

/// Render the shoggoth boss: a LIVE 3D render through shoggoth-core (see
/// `Graphics::draw_shoggoth_live`) — mask on while `Boss::reveal` is 0, the
/// consume-inward mask-off as it runs to 1, the raw tentacled form after.
fn render_bosses(world: &World, graphics: &Graphics, now: f32, cull: &crate::camera::ViewCull) {
    for entity in world.query::<Boss>() {
        let (pos, boss, health) = match (
            world.get_component::<Position>(entity),
            world.get_component::<Boss>(entity),
            world.get_component::<Health>(entity),
        ) {
            (Some(p), Some(b), Some(h)) => (p, b, h),
            _ => continue,
        };
        if health.is_dead() {
            continue;
        }
        let radius = world
            .get_component::<Radius>(entity)
            .map(|r| r.value)
            .unwrap_or(42.0);
        let tile = radius * BOSS_TILE_PER_RADIUS;
        // ~250k verts per frame when drawn: skip it while it is fully
        // off-screen (its tentacles reach past the tile, hence 0.75 * tile
        // as the half-extent = a 1.5x-tile footprint).
        if !cull.visible(pos.x, pos.y, tile * 0.75) {
            continue;
        }
        // The mask leans toward where it is heading (the boss always turns to
        // face its prey, so its rotation is its movement direction).
        let heading = world
            .get_component::<Rotation>(entity)
            .map(|r| r.angle)
            .unwrap_or(0.0);
        graphics.draw_shoggoth_live(Vec2::new(pos.x, pos.y), tile, heading, boss.reveal, now);
    }
}

/// How far the baseboard strip extends past each wall rect (world units;
/// >= 3 so it survives `?pixel=3` art-res rasterization).
const WALL_BASEBOARD_EXTEND: f32 = 4.0;

/// Render walls from the world.
///
/// Two passes: first every wall's BASEBOARD (a dark strip extending
/// [`WALL_BASEBOARD_EXTEND`] units around the rect, so walls seat into the
/// floor instead of floating), then every wall slab — so where two wall rects
/// abut, a neighbour's baseboard never draws over a wall fill.
pub fn render_walls(world: &World, graphics: &Graphics, show_infos: bool) {
    let e = WALL_BASEBOARD_EXTEND;
    for wall in world.walls() {
        graphics.draw_rectangle(
            Vec2::new(wall.x - e, wall.y - e),
            wall.width + e * 2.0,
            wall.height + e * 2.0,
            crate::palette::WALL_BASEBOARD,
        );
    }
    for wall in world.walls() {
        // Draw inflated wall boundaries (debug visualization)
        if show_infos {
            let wall_padding = 25.0; // Same as pathfinding inflation
            graphics.draw_rectangle_lines(
                Vec2::new(wall.x - wall_padding, wall.y - wall_padding),
                wall.width + wall_padding * 2.0,
                wall.height + wall_padding * 2.0,
                1.0,
                Color::new(1.0, 1.0, 0.0, 0.3), // Semi-transparent yellow
            );
        }

        draw_wall(graphics, wall.x, wall.y, wall.width, wall.height);
    }
}

/// Draw one wall rectangle the way the game does (dark purple slab with a
/// lighter border) — shared with the native level editor. Colours live in
/// `src/palette.rs`.
pub fn draw_wall(graphics: &Graphics, x: f32, y: f32, w: f32, h: f32) {
    graphics.draw_rectangle(Vec2::new(x, y), w, h, crate::palette::WALL_FILL);
    // Border for visual depth (3 units thick: stays visible at `?pixel=3`).
    graphics.draw_rectangle_lines(Vec2::new(x, y), w, h, 3.0, crate::palette::WALL_EDGE);
}

/// Render debug pathfinding visualization
fn render_debug_pathfinding(world: &World, graphics: &Graphics) {
    use crate::components::{AIState, DebugPath, DebugTrail, Enemy, Position, AI};
    use crate::ecs::Entity;

    let enemies: Vec<Entity> = world.query::<Enemy>();

    for entity in enemies {
        let (pos, ai, debug_path, debug_trail) = match (
            world.get_component::<Position>(entity),
            world.get_component::<AI>(entity),
            world.get_component::<DebugPath>(entity),
            world.get_component::<DebugTrail>(entity),
        ) {
            (Some(p), Some(a), dp, dt) => (p, a, dp, dt),
            _ => continue,
        };

        // Only show pathfinding for enemies that are chasing (SpottedUnsure or SurePlayerSeen)
        match ai.state {
            AIState::SpottedUnsure | AIState::SurePlayerSeen => {
                // Draw actual movement trail first (cyan/blue - behind planned path)
                if let Some(trail) = debug_trail {
                    if trail.positions.len() > 1 {
                        let mut prev_pos = trail.positions[0];
                        for current_pos in trail.positions.iter().skip(1) {
                            graphics.draw_line(
                                prev_pos,
                                *current_pos,
                                2.0,
                                Color::new(0.0, 0.8, 1.0, 0.6), // Cyan, semi-transparent
                            );
                            prev_pos = *current_pos;
                        }
                    }
                }

                // Draw pathfinding waypoints if available
                if let Some(debug_path) = debug_path {
                    if !debug_path.waypoints.is_empty() {
                        // Draw line to final target (semi-transparent red)
                        graphics.draw_line(
                            pos.to_vec2(),
                            debug_path.target,
                            2.0,
                            Color::new(1.0, 0.0, 0.0, 0.3),
                        );

                        // Draw waypoint path (bright green)
                        let mut prev_point = pos.to_vec2();
                        for waypoint in &debug_path.waypoints {
                            // Draw line from previous point to this waypoint
                            graphics.draw_line(
                                prev_point,
                                *waypoint,
                                2.0,
                                Color::new(0.0, 1.0, 0.0, 0.8),
                            );

                            // Draw waypoint as small circle
                            graphics.draw_circle(*waypoint, 4.0, Color::new(0.0, 1.0, 0.0, 1.0));

                            prev_point = *waypoint;
                        }

                        // Draw final segment to target
                        if let Some(last_waypoint) = debug_path.waypoints.last() {
                            graphics.draw_line(
                                *last_waypoint,
                                debug_path.target,
                                2.0,
                                Color::new(0.0, 1.0, 0.0, 0.8),
                            );
                        }

                        // Draw target as larger circle
                        graphics.draw_circle(
                            debug_path.target,
                            6.0,
                            Color::new(1.0, 0.0, 0.0, 1.0),
                        );
                    }
                }
            }
            _ => {} // Don't show pathfinding for other states
        }
    }
}

/// Render enemy vision cones
fn render_enemy_vision_cones(world: &World, graphics: &Graphics) {
    let enemies: Vec<Entity> = world.query::<Enemy>();

    for entity in enemies {
        let (pos, rotation, ai, health) = match (
            world.get_component::<Position>(entity),
            world.get_component::<Rotation>(entity),
            world.get_component::<AI>(entity),
            world.get_component::<Health>(entity),
        ) {
            (Some(p), Some(r), Some(a), Some(h)) => (p, r, a, h),
            _ => continue,
        };

        // Only draw vision cone for alive enemies — and passive civilians
        // have no vision at all.
        if health.is_dead() || ai.state == AIState::Passive {
            continue;
        }

        // Draw a 90-degree cone in the direction the enemy is facing
        let cone_angle = std::f32::consts::PI / 2.0; // 90 degrees
        let start_angle = rotation.angle - cone_angle / 2.0;
        let end_angle = rotation.angle + cone_angle / 2.0;

        // Semi-transparent red cone
        let color = Color::new(1.0, 0.0, 0.0, 0.1);
        graphics.draw_arc(
            Vec2::new(pos.x, pos.y),
            ai.detection_range,
            start_angle,
            end_angle,
            color,
        );
    }
}

/// Art-pixel size (world units) of the hand-drawn chunky ground stamps
/// (pickup markers, oil splats) — the same referential `?pixel=3` rasterizes
/// the scenery at, so actor-layer decals share the world's crunch.
pub const GROUND_ART_PX: f32 = 3.0;

/// A filled disc drawn as stacked horizontal bars on an `apx` art grid — the
/// chunky stand-in for `draw_circle` in the WORLD layer (## Design: hard
/// stair-stepped edges; smooth vector circles outside a pixel group are
/// off-vibe). The stamp itself is rigid; `center` may still move smoothly
/// (chunky asset, continuous motion). Rows never overlap, so translucent
/// colours blend exactly once.
pub fn draw_pixel_disc(graphics: &Graphics, center: Vec2, radius: f32, apx: f32, color: Color) {
    let half_rows = ((radius / apx).round() as i32).max(1);
    for i in -half_rows..half_rows {
        let yc = (i as f32 + 0.5) * apx;
        let chord = (radius * radius - yc * yc).max(0.0).sqrt();
        let hw = (chord / apx).round().max(1.0) * apx;
        graphics.draw_rectangle(
            Vec2::new(center.x - hw, center.y + i as f32 * apx),
            hw * 2.0,
            apx,
            color,
        );
    }
}

/// Color used to represent a weapon type on the ground / in the UI.
fn weapon_color(weapon_type: WeaponType) -> Color {
    match weapon_type {
        WeaponType::Pistol => Color::new(0.9, 0.9, 0.9, 1.0), // Light gray
        WeaponType::Shotgun => Color::new(1.0, 0.55, 0.1, 1.0), // Orange
        WeaponType::MachineGun => Color::new(0.2, 0.8, 1.0, 1.0), // Cyan
        WeaponType::Melee => Color::new(0.7, 0.7, 0.75, 1.0), // Steel
    }
}

/// GUNPICKUP weapon-model index for a weapon type (robot-core.js
/// `GROUND_WEAPON_MODELS`: 0 bar, 1 pistol, 2 machinegun, 3 shotgun).
fn ground_weapon_idx(weapon_type: WeaponType) -> u32 {
    match weapon_type {
        WeaponType::Melee => 0,
        WeaponType::Pistol => 1,
        WeaponType::MachineGun => 2,
        WeaponType::Shotgun => 3,
    }
}

/// On-screen size (px) of a ground weapon's sprite quad. The 3D render frames
/// 1.44 model units across the quad, so the bar (~1.15 units) comes out
/// ~37 px long and a pistol ~18 px — real relative scales.
const GROUND_GUN_PX: f32 = 50.0;

/// A stable "dropped there" resting angle derived from the pickup's position
/// (a hash, not a random: the same spot always yields the same angle, so a
/// lying weapon never flickers between frames).
fn resting_angle(x: f32, y: f32) -> f32 {
    let h = (x * 12.9898 + y * 78.233).sin() * 43758.547;
    (h - h.floor()) * std::f32::consts::TAU
}

/// Render dropped weapon pickups: the actual 3D gun model lying flat on the
/// ground at a stable scattered angle (pixel-art render, opcode GUNPICKUP)
/// over a subtle glow marker so it still reads as pickup-able.
fn render_pickups(world: &World, graphics: &Graphics, cull: &crate::camera::ViewCull) {
    let pickups: Vec<Entity> = world.query::<WeaponPickup>();

    for entity in pickups {
        let (pos, pickup) = match (
            world.get_component::<Position>(entity),
            world.get_component::<WeaponPickup>(entity),
        ) {
            (Some(p), Some(w)) => (p, w),
            _ => continue,
        };

        // Skip pickups fully off-screen (conservative half-extent = the
        // whole quad).
        if !cull.visible(pos.x, pos.y, GROUND_GUN_PX) {
            continue;
        }

        let color = weapon_color(pickup.weapon_type);

        // Subtle ground marker: a faint weapon-coloured halo over a dark
        // plate — stepped chunky discs on the world art grid, not smooth
        // circles (## Design).
        let halo = Color::new(color.r, color.g, color.b, 0.16);
        draw_pixel_disc(graphics, Vec2::new(pos.x, pos.y), 17.0, GROUND_ART_PX, halo);
        draw_pixel_disc(
            graphics,
            Vec2::new(pos.x, pos.y),
            13.0,
            GROUND_ART_PX,
            Color::new(0.0, 0.0, 0.0, 0.35),
        );

        // The weapon itself, lying where it fell.
        graphics.draw_gun_pickup(
            ground_weapon_idx(pickup.weapon_type),
            Vec2::new(pos.x, pos.y),
            resting_angle(pos.x, pos.y),
            GROUND_GUN_PX,
        );
    }
}

/// Render projectile trails
fn render_projectile_trails(world: &World, graphics: &Graphics) {
    let trails: Vec<Entity> = world.query::<ProjectileTrail>();

    for entity in trails {
        let trail = match world.get_component::<ProjectileTrail>(entity) {
            Some(t) => t,
            None => continue,
        };

        // Calculate alpha based on remaining lifetime (fade out effect)
        let alpha = trail.alpha();
        let color = Color::new(1.0, 0.9, 0.3, alpha); // Yellow-ish color with fade

        graphics.draw_line(
            Vec2::new(trail.start.x, trail.start.y),
            Vec2::new(trail.end.x, trail.end.y),
            2.0, // Line width
            color,
        );
    }
}

/// Art-pixel size of a bullet tracer's pixel group, in WORLD units.
const TRACER_PX: f32 = 2.5;
/// Tracer dash length: 6 art pixels along the velocity.
const TRACER_LEN: f32 = 6.0 * TRACER_PX;
/// Tracer dash height: 2 art pixels.
const TRACER_H: f32 = 2.0 * TRACER_PX;

/// Render bullets as HM-style TRACERS: an elongated 6x2-art-pixel dash
/// oriented along the velocity — a warm off-white core with a slightly
/// dimmer/warmer 2-pixel tail, no outline. Each bullet is one small
/// pixel-art group (the props/sparks idiom: `pixel_begin`, plain snapped
/// composite) opened UNDER the heading rotation ("Before" mode), so the
/// finished chunky dash rotates and glides as a rigid pixel image at native
/// resolution (CLAUDE.md ## Design). Actors layer: this runs after the
/// `?pixel=N` scenery group closes.
fn render_bullets(world: &World, graphics: &Graphics, cull: &crate::camera::ViewCull) {
    let bullets: Vec<Entity> = world.query::<Bullet>();

    // Warm off-white core, dimmer/warmer tail (the measured HM dash).
    let core = Color::new(1.0, 240.0 / 255.0, 220.0 / 255.0, 1.0);
    let tail = Color::new(230.0 / 255.0, 200.0 / 255.0, 160.0 / 255.0, 1.0);

    for entity in bullets {
        let pos = match world.get_component::<Position>(entity) {
            Some(p) => p,
            None => continue,
        };
        if !cull.visible(pos.x, pos.y, TRACER_LEN) {
            continue;
        }
        // Heading from the live velocity (a degenerate stationary round
        // points +x).
        let angle = world
            .get_component::<Velocity>(entity)
            .map(|v| v.y.atan2(v.x))
            .unwrap_or(0.0);

        graphics.save();
        graphics.translate(pos.x, pos.y);
        graphics.rotate(angle);
        // Rotate, THEN open the group: the dash rasterizes on its own grid
        // and PIX_END's quad — drawn through the rotated transform — turns
        // the rigid pixel image (sampling stays NEAREST, edges stay hard).
        graphics.pixel_begin(TRACER_PX, TRACER_LEN, TRACER_H);
        // Tail: rear 2 art pixels, full height.
        graphics.draw_rectangle(Vec2::new(0.0, 0.0), 2.0 * TRACER_PX, TRACER_H, tail);
        // Core: front 4 art pixels.
        graphics.draw_rectangle(
            Vec2::new(2.0 * TRACER_PX, 0.0),
            4.0 * TRACER_PX,
            TRACER_H,
            core,
        );
        // The bullet's physical position leads the dash: ~1 art pixel of
        // tracer ahead of the round, the rest streaking behind.
        graphics.pixel_end(-(TRACER_LEN - TRACER_PX), -TRACER_H * 0.5);
        graphics.restore();
    }
}

/// Render weapons currently flying through the air after being thrown.
fn render_thrown_weapons(world: &World, graphics: &Graphics, cull: &crate::camera::ViewCull) {
    let thrown: Vec<Entity> = world.query::<ThrownWeapon>();

    for entity in thrown {
        let (pos, tw) = match (
            world.get_component::<Position>(entity),
            world.get_component::<ThrownWeapon>(entity),
        ) {
            (Some(p), Some(t)) => (p, t),
            _ => continue,
        };

        // Same GUNPICKUP sprite as a resting pickup: cull off-screen.
        if !cull.visible(pos.x, pos.y, GROUND_GUN_PX) {
            continue;
        }

        // The actual 3D weapon model tumbling flat across the floor: the same
        // GUNPICKUP render as a resting pickup, spun by the throw.
        graphics.draw_gun_pickup(
            ground_weapon_idx(tw.weapon_type),
            Vec2::new(pos.x, pos.y),
            tw.spin,
            GROUND_GUN_PX,
        );
    }
}

/// Render UI (the chromatic rogue counter, the sliding ammo box, the message
/// roller, etc.). `weapon` is the held weapon type (`None` = unarmed), `ammo`
/// the rounds left in it, `ammo_slide` the eased slide offset of the ammo box
/// (`AmmoSlide::eased`: 0 = in place, 1 = fully below the screen edge), and
/// `t` the game clock in seconds (drives the chroma cycle + wobble).
#[allow(clippy::too_many_arguments)]
pub fn render_ui(
    graphics: &Graphics,
    ammo: i32,
    weapon: Option<WeaponType>,
    ammo_slide: f32,
    enemies_alive: usize,
    player_alive: bool,
    death_time: f32,
    debug_enabled: bool,
    show_infos: bool,
    roller: &crate::hud_msg::MsgRoller,
    t: f32,
) {
    let screen_width = graphics.width();
    let screen_height = graphics.height();

    if player_alive {
        // Top-right, under the message roller's resting spot: the compact
        // HM-style rogue counter (always visible; the top-LEFT stays empty
        // during play — HEALTH is gone, the game is one-hit death).
        let rogues = format!("{} ROGUES", enemies_alive);
        let rx = screen_width - MSG_PAD_X - chroma_text_width(&rogues, ROGUES_FS);
        draw_chroma_text(
            graphics,
            &rogues,
            Vec2::new(rx, ROGUES_BASELINE),
            ROGUES_FS,
            t,
        );

        render_msg_roller(graphics, roller, t);
        render_ammo_box(graphics, weapon, ammo, ammo_slide, t);
    } else {
        // Death screen with animations

        // "SYSTEM HALTED" - reveal left to right
        let message = "SYSTEM HALTED";
        let reveal_duration = 1.0; // 1 second to fully reveal
        let reveal_progress = (death_time / reveal_duration).min(1.0);
        let chars_to_show = (message.len() as f32 * reveal_progress) as usize;
        let revealed_text = &message[0..chars_to_show.min(message.len())];

        graphics.draw_text(
            revealed_text,
            Vec2::new(screen_width / 2.0 - 190.0, screen_height / 2.0),
            60.0,
            Color::RED,
        );

        // "Press R to restart" - wobbling animation
        // Only show after main message is fully revealed
        if death_time > reveal_duration {
            let anim_time = death_time - reveal_duration;

            // Wobble position (move up and down)
            let y_amplitude = 5.0; // pixels
            let y_speed = 1.5; // Hz
            let y_offset = y_amplitude * (anim_time * y_speed * 2.0 * std::f32::consts::PI).sin();

            graphics.draw_text(
                "Press R to reboot",
                Vec2::new(
                    screen_width / 2.0 - 120.0,
                    screen_height / 2.0 + 80.0 + y_offset,
                ),
                30.0,
                Color::WHITE,
            );
        }
    }

    // Info display indicator — top-LEFT now (the play HUD keeps it empty,
    // and the top-right belongs to the roller + rogue counter).
    if debug_enabled {
        let info_text = if show_infos {
            "Infos: ON (Press I to toggle)"
        } else {
            "Infos: OFF (Press I to toggle)"
        };
        let info_color = if show_infos {
            Color::new(0.0, 1.0, 0.0, 1.0) // Green when active
        } else {
            Color::GRAY // Gray when inactive
        };
        graphics.draw_text(info_text, Vec2::new(10.0, 30.0), 16.0, info_color);
        if show_infos {
            graphics.draw_text(
                "K: purge all rogues / B: crack boss mask (debug)",
                Vec2::new(10.0, 50.0),
                14.0,
                Color::GRAY,
            );
        }
    }

    // Controls info (no weapon-select keys: one weapon in hand, swap on the floor)
    graphics.draw_text(
        "WASD move · Mouse aim · LClick fire · RClick throw · E pick up · Shift look · Esc menu",
        Vec2::new(10.0, screen_height - 20.0),
        16.0,
        Color::GRAY,
    );
}

// ---------------------------------------------------------------------------
// The HM chromatic HUD text: two layers, hard color steps, wobble
// ---------------------------------------------------------------------------

/// VT323 average advance per character at font size 1. The other panels'
/// 0.42 estimate runs narrow on wide-cap directives ("WARDENS"): the HUD
/// boxes size with a touch of headroom so the wobbling text never hangs
/// off the black.
const AMMO_CHAR_W: f32 = 0.46;
/// The hard-step colour cycle: light cyan -> light magenta -> pure white.
const CHROMA_COLORS: [Color; 3] = [
    Color::new(150.0 / 255.0, 240.0 / 255.0, 1.0, 1.0),
    Color::new(1.0, 150.0 / 255.0, 230.0 / 255.0, 1.0),
    Color::new(1.0, 1.0, 1.0, 1.0),
];
/// Colour / wobble steps per second (hard steps, no fades).
pub const CHROMA_HZ: f32 = 9.0;
/// The BACK layer's down-left offset from the front layer, px.
const CHROMA_BACK_OFF: f32 = 2.5;
/// Wobble rotation amplitude, degrees (alternates sign every step).
const CHROMA_WOBBLE_DEG: f32 = 3.5;
/// Vertical bob amplitude, px (alternates every two steps).
const CHROMA_BOB_PX: f32 = 1.5;
/// Approximate half cap-height above the baseline at font size 1 (where the
/// wobble pivot sits so the text rocks about its own centre).
const CHROMA_HALF_H: f32 = 0.35;

/// Width the chroma text will occupy (the same VT323 average-advance
/// estimate everything else uses).
pub fn chroma_text_width(text: &str, font_size: f32) -> f32 {
    text.chars().count() as f32 * font_size * AMMO_CHAR_W
}

/// The classic HM two-layer chromatic HUD text: the string drawn TWICE — a
/// back layer offset down-left, a front layer on top — both cycling hard
/// through light cyan / light magenta / white at [`CHROMA_HZ`], the back
/// layer one step ahead so the two are never the same colour. The whole
/// thing wobbles: rotation flips ±[`CHROMA_WOBBLE_DEG`]° with the colour
/// steps and a small vertical bob rides on top. Fully deterministic from
/// `t` (the game clock, seconds). `pos` is the baseline-left of the front
/// layer at rest, like `draw_text`.
pub fn draw_chroma_text(graphics: &Graphics, text: &str, pos: Vec2, font_size: f32, t: f32) {
    let step = (t * CHROMA_HZ).floor() as i64;
    let front = CHROMA_COLORS[step.rem_euclid(3) as usize];
    let back = CHROMA_COLORS[(step + 1).rem_euclid(3) as usize];
    let rot = if step % 2 == 0 {
        CHROMA_WOBBLE_DEG.to_radians()
    } else {
        -CHROMA_WOBBLE_DEG.to_radians()
    };
    let bob = if step.rem_euclid(4) < 2 {
        CHROMA_BOB_PX
    } else {
        -CHROMA_BOB_PX
    };

    let w = chroma_text_width(text, font_size);
    let half_h = font_size * CHROMA_HALF_H;
    // Pivot at the text's visual centre so the wobble ROCKS in place.
    graphics.save();
    graphics.translate(pos.x + w / 2.0, pos.y - half_h + bob);
    graphics.rotate(rot);
    graphics.draw_text(
        text,
        Vec2::new(-w / 2.0 - CHROMA_BACK_OFF, half_h + CHROMA_BACK_OFF),
        font_size,
        back,
    );
    graphics.draw_text(text, Vec2::new(-w / 2.0, half_h), font_size, front);
    graphics.restore();
}

// ---------------------------------------------------------------------------
// The sliding bottom-left AMMO BOX
// ---------------------------------------------------------------------------

/// HUD font size inside the ammo box: ~70% of the box height (the HM ratio).
const AMMO_FS: f32 = 26.0;
/// Box height: snug around the text at the 70% fill.
const AMMO_BOX_H: f32 = 37.0;
/// Horizontal inner padding (the box hugs the LEFT screen border; this pads
/// the text on both sides inside it).
const AMMO_PAD_X: f32 = 14.0;
/// Resting gap between the box's bottom edge and the bottom screen border.
/// The bottom-left controls hint line sits in that band (baseline at
/// `screen_height - 20`), so the box floats ABOVE it with clear separation
/// instead of the bare ~16 px edge padding.
const AMMO_BOTTOM_GAP: f32 = 44.0;

/// The sliding bottom-left AMMO BOX: a pure black rectangle flush with the
/// left screen border showing only `12/12 RNDS` (or `NO GUN` as it slides
/// away) in the two-layer chromatic wobble style. `slide` is
/// `AmmoSlide::eased` — 0 draws the box in place, 1 puts it fully below the
/// bottom screen edge (then nothing is drawn at all); `t` is the game clock
/// (the chroma cycle).
pub fn render_ammo_box(
    graphics: &Graphics,
    weapon: Option<WeaponType>,
    ammo: i32,
    slide: f32,
    t: f32,
) {
    if slide >= 1.0 {
        return; // fully slid out
    }
    let text = crate::hud_ammo::ammo_box_text(weapon, ammo);
    let text_w = chroma_text_width(&text, AMMO_FS);
    let box_w = text_w + 2.0 * AMMO_PAD_X;

    let screen_height = graphics.height();
    let shown_y = screen_height - AMMO_BOTTOM_GAP - AMMO_BOX_H;
    // Slide travel: from resting place to fully under the bottom border.
    let y = shown_y + slide * (AMMO_BOX_H + AMMO_BOTTOM_GAP);

    graphics.draw_rectangle(Vec2::new(0.0, y), box_w, AMMO_BOX_H, Color::BLACK);
    // Baseline sits so the caps centre in the box.
    draw_chroma_text(
        graphics,
        &text,
        Vec2::new(AMMO_PAD_X, y + AMMO_BOX_H / 2.0 + AMMO_FS * CHROMA_HALF_H),
        AMMO_FS,
        t,
    );
}

// ---------------------------------------------------------------------------
// The top-right MESSAGE ROLLER + rogue counter
// ---------------------------------------------------------------------------

/// Roller font size: ~70% of the box height, like the ammo box.
const MSG_FS: f32 = 25.0;
/// Roller box height.
const MSG_BOX_H: f32 = 36.0;
/// Horizontal inner padding of the roller box (and the rogue counter's
/// right-edge margin, so the two right-align together).
const MSG_PAD_X: f32 = 14.0;
/// Resting gap between the roller box's top edge and the top screen border.
const MSG_TOP_GAP: f32 = 18.0;
/// Rogue counter font size (small, always visible).
const ROGUES_FS: f32 = 20.0;
/// Rogue counter baseline: under the roller's resting spot.
const ROGUES_BASELINE: f32 = MSG_TOP_GAP + MSG_BOX_H + 28.0;

/// The top-right MESSAGE ROLLER: a black box glued to the RIGHT screen edge
/// that rolls DOWN from above the top border when a directive arrives and
/// rolls back up when it expires (`hud_msg::MsgRoller` owns the state; this
/// only draws). The text is the short chromatic directive.
pub fn render_msg_roller(graphics: &Graphics, roller: &crate::hud_msg::MsgRoller, t: f32) {
    if roller.hidden() {
        return; // fully rolled away
    }
    let msg = roller.message();
    if msg.is_empty() {
        return;
    }
    let text_w = chroma_text_width(msg, MSG_FS);
    let box_w = text_w + 2.0 * MSG_PAD_X;
    let screen_width = graphics.width();

    // Roll travel: from resting place to fully above the top border.
    let y = MSG_TOP_GAP - roller.eased() * (MSG_TOP_GAP + MSG_BOX_H + CHROMA_BACK_OFF + 4.0);

    graphics.draw_rectangle(
        Vec2::new(screen_width - box_w, y),
        box_w,
        MSG_BOX_H,
        Color::BLACK,
    );
    draw_chroma_text(
        graphics,
        msg,
        Vec2::new(
            screen_width - box_w + MSG_PAD_X,
            y + MSG_BOX_H / 2.0 + MSG_FS * CHROMA_HALF_H,
        ),
        MSG_FS,
        t,
    );
}
