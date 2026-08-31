// Rendering system for drawing entities
use crate::components::*;
use crate::ecs::{Entity, World};
use crate::graphics::Graphics;
use crate::math::{Color, Vec2};

/// Render all entities in the world
/// `draw_bots` selects whether the player/rogue sprites are drawn here; the
/// game passes false and draws them as live 3D robots instead (the boss is
/// always drawn here, live too). `now` is the continuous animation clock in
/// seconds (drives the boss's writhing). `cull` = the camera's inflated view
/// rect: the expensive live sprites (ground guns, the boss) fully outside it
/// skip their commands — bullets / trails / debug overlays keep their own
/// cheap paths untouched.
pub fn render_entities(
    world: &World,
    graphics: &Graphics,
    show_infos: bool,
    draw_bots: bool,
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

    // Render bullets
    render_bullets(world, graphics);

    // Render weapons in flight
    render_thrown_weapons(world, graphics, cull);

    // Render enemies
    if draw_bots {
        render_enemies(world, graphics);
    }

    // Render the boss (big; under the player)
    render_bosses(world, graphics, now, cull);

    // Render player (on top)
    if draw_bots {
        render_player(world, graphics);
    }
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

        // Subtle ground marker: a faint weapon-coloured halo over a dark plate.
        let halo = Color::new(color.r, color.g, color.b, 0.16);
        graphics.draw_circle(Vec2::new(pos.x, pos.y), 17.0, halo);
        graphics.draw_circle(
            Vec2::new(pos.x, pos.y),
            13.0,
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

/// Render bullets
fn render_bullets(world: &World, graphics: &Graphics) {
    let bullets: Vec<Entity> = world.query::<Bullet>();

    for entity in bullets {
        let pos = match world.get_component::<Position>(entity) {
            Some(p) => p,
            None => continue,
        };

        let radius = world
            .get_component::<Radius>(entity)
            .map(|r| r.value)
            .unwrap_or(2.0);

        // Yellow bullets
        let color = Color::new(1.0, 0.9, 0.3, 1.0);
        graphics.draw_circle(Vec2::new(pos.x, pos.y), radius, color);
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

/// Render all enemies
fn render_enemies(world: &World, graphics: &Graphics) {
    let enemies: Vec<Entity> = world.query::<Enemy>();

    for entity in enemies {
        // The boss is drawn by render_bosses, not as a regular sprite.
        if world.has_component::<Boss>(entity) {
            continue;
        }

        let (pos, rotation, health, ai) = match (
            world.get_component::<Position>(entity),
            world.get_component::<Rotation>(entity),
            world.get_component::<Health>(entity),
            world.get_component::<AI>(entity),
        ) {
            (Some(p), Some(r), Some(h), Some(a)) => (p, r, h, a),
            _ => continue,
        };

        // Rogue AI palette, keyed by behavioral signature (flavor names in LORE.md).
        let base_color = match ai.initial_type {
            EnemyType::Idle => Color::from_rgba(224, 49, 66, 255), // SENTINEL - hostile red
            EnemyType::Wandering => Color::from_rgba(150, 70, 210, 255), // DRIFTER - glitch violet
            EnemyType::Patrolling => Color::from_rgba(224, 40, 160, 255), // HUNTER - predatory magenta
        };
        // Draw knocked-down (stunned) enemies as prone, like the dead pose.
        let prone = health.is_dead() || world.has_component::<Stunned>(entity);

        graphics.draw_pixelated_sprite(Vec2::new(pos.x, pos.y), rotation.angle, base_color, prone);
    }
}

/// Render the player
fn render_player(world: &World, graphics: &Graphics) {
    let players: Vec<Entity> = world.query::<Player>();
    let player = match players.first() {
        Some(&e) => e,
        None => return,
    };

    let pos = match world.get_component::<Position>(player) {
        Some(p) => p,
        None => return,
    };

    let rotation = world
        .get_component::<Rotation>(player)
        .map(|r| r.angle)
        .unwrap_or(0.0);

    let health = world
        .get_component::<Health>(player)
        .map(|h| h.current)
        .unwrap_or(0);

    if health > 0 {
        // Draw the friendly coral purge bot in warm coral.
        let base_color = Color::from_rgba(217, 119, 87, 255);
        graphics.draw_pixelated_sprite(
            Vec2::new(pos.x, pos.y),
            rotation,
            base_color,
            false, // Player is alive
        );
    }
}

/// Render UI (health, rogue count, the sliding ammo box, etc.). `weapon` is
/// the held weapon type (`None` = unarmed), `ammo` the rounds left in it and
/// `ammo_slide` the eased slide offset of the ammo box (`AmmoSlide::eased`:
/// 0 = in place, 1 = fully below the screen edge).
#[allow(clippy::too_many_arguments)]
pub fn render_ui(
    graphics: &Graphics,
    health: i32,
    ammo: i32,
    weapon: Option<WeaponType>,
    ammo_slide: f32,
    enemies_alive: usize,
    player_alive: bool,
    death_time: f32,
    debug_enabled: bool,
    show_infos: bool,
) {
    let screen_width = graphics.width();
    let screen_height = graphics.height();

    if player_alive {
        graphics.draw_text("Health:", Vec2::new(10.0, 30.0), 20.0, Color::WHITE);
        graphics.draw_text(
            &format!("{}", health),
            Vec2::new(100.0, 30.0),
            20.0,
            Color::WHITE,
        );

        // The held gun's rounds live in the sliding bottom-left AMMO BOX
        // (`render_ammo_box`), not up here — so ROGUES moves up under HEALTH.
        graphics.draw_text("Rogues:", Vec2::new(10.0, 60.0), 20.0, Color::WHITE);
        graphics.draw_text(
            &format!("{}", enemies_alive),
            Vec2::new(120.0, 60.0),
            20.0,
            Color::WHITE,
        );

        render_ammo_box(graphics, weapon, ammo, ammo_slide);
    } else if !player_alive {
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

    // Info display indicator
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
        graphics.draw_text(
            info_text,
            Vec2::new(screen_width - 280.0, 30.0),
            16.0,
            info_color,
        );
        if show_infos {
            graphics.draw_text(
                "K: purge all rogues / B: crack boss mask (debug)",
                Vec2::new(screen_width - 280.0, 50.0),
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

/// VT323 average advance per character at font size 1 (the approximation the
/// other panels use — editor_ui / render_dialogue / ending agree on 0.42).
const AMMO_CHAR_W: f32 = 0.42;
/// HUD font size inside the box (same as the HEALTH / ROGUES lines).
const AMMO_FS: f32 = 20.0;
/// Box height: the 20 px text plus comfortable inner padding.
const AMMO_BOX_H: f32 = 34.0;
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
/// away). `slide` is `AmmoSlide::eased` — 0 draws the box in place, 1 puts
/// it fully below the bottom screen edge (then nothing is drawn at all).
pub fn render_ammo_box(graphics: &Graphics, weapon: Option<WeaponType>, ammo: i32, slide: f32) {
    if slide >= 1.0 {
        return; // fully slid out
    }
    let text = crate::hud_ammo::ammo_box_text(weapon, ammo);
    let text_w = text.chars().count() as f32 * AMMO_FS * AMMO_CHAR_W;
    let box_w = text_w + 2.0 * AMMO_PAD_X;

    let screen_height = graphics.height();
    let shown_y = screen_height - AMMO_BOTTOM_GAP - AMMO_BOX_H;
    // Slide travel: from resting place to fully under the bottom border.
    let y = shown_y + slide * (AMMO_BOX_H + AMMO_BOTTOM_GAP);

    graphics.draw_rectangle(Vec2::new(0.0, y), box_w, AMMO_BOX_H, Color::BLACK);
    // Baseline sits so the 20 px caps centre in the 34 px box.
    graphics.draw_text(
        &text,
        Vec2::new(AMMO_PAD_X, y + AMMO_BOX_H - 10.0),
        AMMO_FS,
        Color::WHITE,
    );
}
