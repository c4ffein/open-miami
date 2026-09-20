//! The actors layer: every robot entity as a live 3D sprite (pose / weapon /
//! colour picked from its components), plus detached heads.

use crate::ecs::World;
use crate::graphics::Graphics;
use crate::math::{Color, Vec2};

/// On-screen size (px) of a robot sprite tile. The tile is square and the
/// robot fills ~55% of it, so this is tuned so the bot roughly matches the
/// actor hitbox (player radius 15 -> 30px dia, enemy radius 12 -> 24px
/// dia): a 60px tile draws a ~34px robot that sits over the hitbox like
/// the primitive did.
pub const ROBOT_TILE_PX: f32 = 60.0;

/// The robot sprite's gun/forward points DOWN (+Y in image) at facingDeg=0,
/// while the entity `angle` is atan2(aim) measured from +X. Rotating the
/// image by (angle - PI/2) makes the gun point along the aim/shoot
/// direction (where bullets actually fly), which reads correctly top-down.
pub const ROBOT_ANGLE_OFFSET: f32 = -std::f32::consts::FRAC_PI_2;

// Index tables shared with renderer.js (see Graphics::draw_robot).
pub const ROBOT_COLOR_CORAL: u32 = 0;
pub const ROBOT_POSE_IDLE: u32 = 0;
pub const ROBOT_POSE_WALK: u32 = 1;
pub const ROBOT_POSE_SHOOT: u32 = 2;
pub const ROBOT_POSE_DOWNED: u32 = 4;
/// The downed pose with the head cubes skipped (a KICK finisher victim).
pub const ROBOT_POSE_DOWNED_HEADLESS: u32 = 5;
/// The head-kick finisher: support leg planted, kicking leg sweeping
/// through, body leaning back. `time` = seconds into the finisher.
pub const ROBOT_POSE_KICK: u32 = 6;
/// The two-hit quick stomp finisher. `time` = seconds into the finisher.
pub const ROBOT_POSE_STOMP: u32 = 7;

/// Screen size (px) of a detached head's sprite quad — the 16-texel art
/// upscaled to a bit over its physical share of a 60 px robot tile so
/// the little trophy stays readable.
pub const HEAD_TILE_PX: f32 = 26.0;

/// Map a held weapon to the robot-core weapon model index
/// (0 fist, 1 pistol, 2 machinegun, 3 shotgun).
pub fn robot_weapon_idx(weapon: Option<crate::components::WeaponType>) -> u32 {
    use crate::components::WeaponType;
    match weapon {
        None | Some(WeaponType::Melee) => 0,
        Some(WeaponType::Pistol) => 1,
        Some(WeaponType::MachineGun) => 2,
        Some(WeaponType::Shotgun) => 3,
    }
}

/// Downed-pose time (seconds) a body with no live knockdown clock is
/// parked at: past the fall transition and the landing wobble, so corpses
/// lie still, fully settled, from the first frame.
pub const ROBOT_DOWNED_SETTLED: f32 = 2.0;

/// Draw the player and rogue enemies as live-rendered 3D robot sprites on
/// top of the primitive draw. Must be called while the camera transform is
/// applied (incl. its zoom: `camera::DEFAULT_ZOOM` = 1.6 x the viewport scale)
/// so
/// that world coordinates land on screen.
/// `now` is elapsed time in seconds and drives the pose animations; each
/// entity's clock is offset by its id so the squad doesn't move in
/// phase-locked unison, and knocked-down bots play the hit flinch synced
/// to the moment the stun landed.
///
/// `player_firing` = the fire input is held (the player's SHOOT pose): the
/// caller samples the input, this function only draws.
///
/// One pass of the robot sprites. `prone_pass` = draw only the downed
/// (dead / knocked-down) bodies; `!prone_pass` = only the upright ones.
/// Two passes let the ground weapons draw OVER the corpses (easy to spot)
/// yet UNDER anyone still standing.
///
/// Each ROBOT command costs robot-core an FBO round-trip and ~15-19 draw
/// calls, so bots fully outside `cull` are skipped (conservative
/// half-extent: a whole tile, downed bodies sprawl past their centre).
pub fn draw_robot_entities(
    world: &World,
    graphics: &Graphics,
    now: f32,
    prone_pass: bool,
    player_firing: bool,
    cull: &crate::camera::ViewCull,
) {
    use crate::components::{AIState, EnemyType};
    use crate::components::{
        Boss, DetachedHead, Enemy, Finisher, FinisherKind, Headless, Health, Player, Position,
        Rotation, Stunned, Velocity, Weapon, AI,
    };

    // --- Detached heads (prone pass: corpse details on the floor) ---
    // Kicked-off heads draw with the sprawled bodies: over the scenery,
    // under the ground weapons and everyone still standing. Each is a
    // baked pixel sprite on a quad spun smoothly by its physics, plus a
    // small dark oil splat where it detached.
    if prone_pass {
        for entity in world.query::<DetachedHead>() {
            let (Some(d), Some(pos)) = (
                world.get_component::<DetachedHead>(entity),
                world.get_component::<Position>(entity),
            ) else {
                continue;
            };
            // The splat: a dark oil puddle at the detach point with a
            // few blots thrown clear of the corpse's silhouette, all
            // deterministic per head (hashed off its seq) — chunky
            // stepped discs / 1-art-px dots on the world art grid, not
            // smooth circles (## Design).
            if cull.visible(d.origin_x, d.origin_y, 60.0) {
                use crate::render::{draw_pixel_disc, GROUND_ART_PX};
                let oil = Color::new(0.03, 0.02, 0.05, 0.9);
                let s = d.seq.wrapping_mul(0x27D4_EB2F);
                draw_pixel_disc(
                    graphics,
                    Vec2::new(d.origin_x, d.origin_y),
                    9.0,
                    GROUND_ART_PX,
                    oil,
                );
                for i in 0..4u32 {
                    let a = crate::drive::hash01(s, i * 2) * std::f32::consts::TAU;
                    let r = 10.0 + crate::drive::hash01(s, i * 2 + 1) * 14.0;
                    draw_pixel_disc(
                        graphics,
                        Vec2::new(d.origin_x + a.cos() * r, d.origin_y + a.sin() * r),
                        2.5 + crate::drive::hash01(s, i + 40) * 3.0,
                        GROUND_ART_PX,
                        oil,
                    );
                }
                // Drip trail along the head's slide: a few 1-art-px dots
                // between the neck and wherever the head is (settles
                // with it).
                for i in 1..4u32 {
                    let f = i as f32 / 4.0;
                    let jx = (crate::drive::hash01(s, 60 + i) - 0.5) * 6.0;
                    let jy = (crate::drive::hash01(s, 70 + i) - 0.5) * 6.0;
                    let half = GROUND_ART_PX * 0.5;
                    graphics.draw_rectangle(
                        Vec2::new(
                            d.origin_x + (pos.x - d.origin_x) * f + jx - half,
                            d.origin_y + (pos.y - d.origin_y) * f + jy - half,
                        ),
                        GROUND_ART_PX,
                        GROUND_ART_PX,
                        Color::new(0.03, 0.02, 0.05, 0.7),
                    );
                }
            }
            if cull.visible(pos.x, pos.y, HEAD_TILE_PX) {
                graphics.draw_head(d.color_idx, Vec2::new(pos.x, pos.y), d.spin, HEAD_TILE_PX);
            }
        }
    }

    // Determines a standing pose index from motion / combat state.
    fn pose_for(speed: f32, attacking: bool) -> u32 {
        if attacking {
            ROBOT_POSE_SHOOT
        } else if speed > 6.0 {
            ROBOT_POSE_WALK
        } else {
            ROBOT_POSE_IDLE
        }
    }

    // --- Enemies (rogue bots) ---
    for entity in world.query::<Enemy>() {
        if world.has_component::<Boss>(entity) {
            continue; // boss keeps its own draw
        }
        let (pos, rot, health, ai) = match (
            world.get_component::<Position>(entity),
            world.get_component::<Rotation>(entity),
            world.get_component::<Health>(entity),
            world.get_component::<AI>(entity),
        ) {
            (Some(p), Some(r), Some(h), Some(a)) => (p, r, h, a),
            _ => continue,
        };
        let color_idx = match ai.initial_type {
            EnemyType::Idle => 1,       // SENTINEL - red
            EnemyType::Wandering => 2,  // DRIFTER - violet
            EnemyType::Patrolling => 3, // HUNTER - magenta
        };
        let stunned = world.get_component::<Stunned>(entity);
        // Dead OR knocked down: sprawled flat in the DOWNED pose.
        let prone = health.is_dead() || stunned.is_some();
        if prone != prone_pass {
            continue;
        }
        if !cull.visible(pos.x, pos.y, ROBOT_TILE_PX) {
            continue;
        }
        let speed = world
            .get_component::<Velocity>(entity)
            .map(|v| (v.x * v.x + v.y * v.y).sqrt())
            .unwrap_or(0.0);
        let attacking = ai.state == AIState::SurePlayerSeen && ai.attack_timer > 0.0;
        let pose_idx = if prone {
            // A kick-finished corpse lost its head: same sprawl, no head.
            if world.has_component::<Headless>(entity) {
                ROBOT_POSE_DOWNED_HEADLESS
            } else {
                ROBOT_POSE_DOWNED
            }
        } else {
            pose_for(speed, attacking)
        };
        let weapon_idx =
            robot_weapon_idx(world.get_component::<Weapon>(entity).map(|w| w.weapon_type));
        // De-sync the squad: each bot's animation clock starts at a
        // different phase derived from its entity id.
        let phase = (entity.0 % 97) as f32 * 0.173;
        // Downed bodies face the blow's origin: the pose's backward topple
        // (robot-core's downed plan leans the rig onto its back) then lays
        // them out along `fall_angle` — head first, away from the blow.
        // Killing blows record the same convention (`record_corpse_fall`:
        // bullets, melee swings and thrown weapons all leave a `Stunned`
        // carrying the shot direction), so corpses sprawl away from the
        // shooter too. A corpse without a live knockdown clock keeps its
        // `Rotation` (the stun system wrote the matching facing there when
        // the stun ended).
        let angle = match stunned {
            Some(stun) => stun.fall_angle + std::f32::consts::PI,
            None => rot.angle,
        };
        let time = if let Some(stun) = stunned {
            // Seconds since the knockdown landed: plays the fall
            // transition once, then the body lies still.
            stun.age()
        } else if health.is_dead() {
            // Dead with no knockdown clock: parked fully settled.
            ROBOT_DOWNED_SETTLED
        } else {
            now + phase
        };
        graphics.draw_robot(
            color_idx,
            pose_idx,
            weapon_idx,
            Vec2::new(pos.x, pos.y),
            angle + ROBOT_ANGLE_OFFSET,
            ROBOT_TILE_PX,
            time,
        );
    }

    // --- Player (CL4-UD3, coral; upright pass only — dead players
    // render their WASTED state elsewhere) ---
    if prone_pass {
        return;
    }
    if let Some(player) = world.first::<Player>() {
        let pos = world.get_component::<Position>(player);
        let health = world.get_component::<Health>(player);
        if let (Some(pos), Some(health)) = (pos, health) {
            // Off-screen check matters even for the player: a scenario
            // `look_at` can carry the camera clean away from them.
            if !health.is_dead() && cull.visible(pos.x, pos.y, ROBOT_TILE_PX) {
                let mut angle = world
                    .get_component::<Rotation>(player)
                    .map(|r| r.angle)
                    .unwrap_or(0.0);
                let speed = world
                    .get_component::<Velocity>(player)
                    .map(|v| (v.x * v.x + v.y * v.y).sqrt())
                    .unwrap_or(0.0);
                let firing = player_firing;
                let mut pose_idx = pose_for(speed, firing);
                let mut draw_pos = Vec2::new(pos.x, pos.y);
                let mut draw_time = now;
                // Mid-finisher: locked over the victim in the strike pose,
                // lunging into each blow (one surge for the bar / the
                // point-blank shot, a pulse per pound when unarmed). The
                // kick and the stomp run their own dedicated poses, whose
                // clock is the finisher's own timer (the animation is
                // choreographed to the impact schedule, not free-running).
                if let Some(fin) = world.get_component::<Finisher>(player) {
                    let progress = (fin.timer / fin.kind.duration()).clamp(0.0, 1.0);
                    let lunge = match fin.kind {
                        FinisherKind::Pound => {
                            8.0 * (progress * 3.0 * std::f32::consts::PI).sin().abs()
                        }
                        FinisherKind::Stomp => {
                            6.0 * (progress * 2.0 * std::f32::consts::PI).sin().abs()
                        }
                        _ => 10.0 * (progress * std::f32::consts::PI).sin(),
                    };
                    pose_idx = match fin.kind {
                        FinisherKind::Kick => {
                            draw_time = fin.timer;
                            ROBOT_POSE_KICK
                        }
                        FinisherKind::Stomp => {
                            draw_time = fin.timer;
                            ROBOT_POSE_STOMP
                        }
                        _ => ROBOT_POSE_SHOOT,
                    };
                    angle = fin.dir_y.atan2(fin.dir_x);
                    draw_pos = Vec2::new(pos.x + fin.dir_x * lunge, pos.y + fin.dir_y * lunge);
                }
                let weapon_idx =
                    robot_weapon_idx(world.get_component::<Weapon>(player).map(|w| w.weapon_type));
                graphics.draw_robot(
                    ROBOT_COLOR_CORAL,
                    pose_idx,
                    weapon_idx,
                    draw_pos,
                    angle + ROBOT_ANGLE_OFFSET,
                    ROBOT_TILE_PX,
                    draw_time,
                );
            }
        }
    }
}
