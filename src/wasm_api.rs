//! Functions the engine EXPORTS to the JS tool pages (`tools/inspector.html`,
//! `tools/rig-parity.html`, through `tools/engine-pose.js`).
//!
//! The tools draw robots with the same WebGL pipeline as the game, and must
//! pose them with the same code: there is ONE pose implementation
//! (`render::pose`), so a tool asks the engine for a pose instead of keeping
//! a JS copy that could drift. Loading the wasm does not start the game
//! (`app::start` is an explicit export), so a tool page can `init()` it and
//! call these alone.

use crate::render::pose::PoseKind;
use crate::render::robots::robot_pose;
use crate::render::shoggoth::{boss_spheres as engine_boss_spheres, BossPose, MASK_OFF_SECS};
use wasm_bindgen::prelude::*;

/// The pose names, comma-separated, in engine index order.
#[wasm_bindgen]
pub fn pose_names() -> String {
    let names: Vec<&str> = PoseKind::ALL.iter().map(|k| k.name()).collect();
    names.join(",")
}

/// The pose a robot holding `weapon_idx` (0 fist / 1 pistol / 2 machinegun /
/// 3 shotgun — unarmed robots stand at ease) is drawn in: 11 scalars in
/// `Pose::scalars()` order (= `POSE_SCALARS` in web/robot-core.js), then the
/// flags (`Pose::flags`). Exactly what the game's `ROBOT` op carries.
#[wasm_bindgen]
pub fn pose_plan_scalars(pose_idx: u32, time: f32, weapon_idx: u32) -> Vec<f32> {
    let pose = robot_pose(pose_idx, time, weapon_idx);
    let mut out = pose.scalars().to_vec();
    out.push(pose.flags() as f32);
    out
}

/// Every sphere of the boss for one frame (`render::shoggoth`): element 0 is
/// the mask split (spheres from there on draw depth-OFF), then 20 floats per
/// sphere — exactly the `SPHERE` run + `SHOGGOTH` the game records. `heading`
/// / `look_up` = NaN leaves them to the boss's wander behaviour (the game
/// does that for `look_up`); `wander` adds its drift (preview only).
#[wasm_bindgen]
pub fn boss_spheres(time: f32, reveal: f32, heading: f32, look_up: f32, wander: bool) -> Vec<f32> {
    let some = |v: f32| (!v.is_nan()).then_some(v);
    let boss = engine_boss_spheres(&BossPose {
        time,
        reveal,
        heading: some(heading),
        look_up: some(look_up),
        wander,
    });
    let mut out = Vec::with_capacity(1 + boss.data.len());
    out.push(boss.mask_at as f32);
    out.extend_from_slice(&boss.data);
    out
}

/// Seconds the boss's mask-off takes (the inspector's "transition" phase).
#[wasm_bindgen]
pub fn boss_mask_off_secs() -> f32 {
    MASK_OFF_SECS
}
