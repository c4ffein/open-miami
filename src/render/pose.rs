//! `pose_plan`: what a robot's body does at time `t` — the joint scalars the
//! rig turns into a skeleton. Pure numbers: no `Graphics`, no browser.
//!
//! THE ONE pose implementation (roadmap: docs/ARCHITECTURE.md). It began as a
//! port of `posePlan()` in web/robot-core.js, proven BIT-EXACT against it
//! (`tests/fixtures/pose_plan.txt`, generated from that JS: 9,504 scalars,
//! max difference 0) before anything switched over; then the game's `ROBOT`
//! op carried these scalars (R2); then the JS function was DELETED (R3): the
//! portrait bake receives its pose in the `PORTRAIT` op and the tool pages
//! ask the wasm (`src/wasm_api.rs` <- tools/engine-pose.js). The fixture
//! stays as a frozen golden record (`matches_the_golden_record`).
//!
//! What living here buys: the finisher choreography is in ONE language. The
//! JS hard-coded "the kick lands at 0.28 s" / "stomps at 0.14 and 0.34 s"
//! next to a comment pointing at `FinisherKind::impacts`; here those times
//! ARE `FinisherKind::impacts()`, and tests can say "the foot is fully
//! extended at the impact".

use crate::components::FinisherKind;

/// The poses, in the order of `render::robots::ROBOT_POSE_*` (and of
/// `pose_names()`, what the tool pages list) (the engine's own pose indices — they no
/// longer cross the boundary: the `ROBOT` op carries the pose as numbers).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PoseKind {
    Idle,
    Walk,
    Shoot,
    Hit,
    Downed,
    /// A KICK victim: the downed sprawl with the head cubes skipped.
    DownedHeadless,
    Kick,
    Stomp,
}

impl PoseKind {
    pub const ALL: [PoseKind; 8] = [
        PoseKind::Idle,
        PoseKind::Walk,
        PoseKind::Shoot,
        PoseKind::Hit,
        PoseKind::Downed,
        PoseKind::DownedHeadless,
        PoseKind::Kick,
        PoseKind::Stomp,
    ];

    /// The engine-side pose index (`render::robots::ROBOT_POSE_*`).
    pub fn index(self) -> u32 {
        Self::ALL.iter().position(|&k| k == self).unwrap_or(0) as u32
    }

    /// Unknown indices fall back to `Idle`, as the JS does for unknown names.
    pub fn from_index(index: u32) -> Self {
        Self::ALL
            .get(index as usize)
            .copied()
            .unwrap_or(PoseKind::Idle)
    }

    /// The pose's name (the tools' buttons and `?pose=` URL values).
    pub fn name(self) -> &'static str {
        match self {
            PoseKind::Idle => "idle",
            PoseKind::Walk => "walk",
            PoseKind::Shoot => "shoot",
            PoseKind::Hit => "hit",
            PoseKind::Downed => "downed",
            PoseKind::DownedHeadless => "downed_headless",
            PoseKind::Kick => "kick",
            PoseKind::Stomp => "stomp",
        }
    }
}

/// The joint drive of one robot at one instant. Angles in radians, offsets in
/// model units; what each does to the skeleton is the rig's business
/// (`leg()` / `arm()` and `rigVS` in web/robot-core.js).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Pose {
    /// Vertical body offset.
    pub bob: f32,
    /// Whole-body backward lean (1.42 = flat on its back).
    pub lean: f32,
    /// Backward slide of the body.
    pub zback: f32,
    /// Gun-arm kick-back while shooting.
    pub recoil: f32,
    pub leg_a: f32,
    pub leg_b: f32,
    /// Left / right arm pitch.
    pub arm_lp: f32,
    pub arm_rp: f32,
    /// Extra shoulder raise of both arms (defensive fling / idle).
    pub arm_raise: f32,
    /// Sideways splay of both arms (the relaxed hang).
    pub arm_out: f32,
    /// Forearm bend at the elbow (the relaxed hang).
    pub elbow: f32,
    /// The gun hand aims forward.
    pub shoot: bool,
    /// Skip the head cubes (`DownedHeadless`).
    pub headless: bool,
}

impl Pose {
    /// A neutral standing rig.
    pub const NEUTRAL: Pose = Pose {
        bob: 0.0,
        lean: 0.0,
        zback: 0.0,
        recoil: 0.0,
        leg_a: 0.0,
        leg_b: 0.0,
        arm_lp: 0.05,
        arm_rp: 0.05,
        arm_raise: 0.0,
        arm_out: 0.0,
        elbow: 0.0,
        shoot: false,
        headless: false,
    };

    /// The JS names of [`Pose::scalars`], in order: must equal `POSE_SCALARS`
    /// in web/robot-core.js — the list the renderer unpacks the `ROBOT` op
    /// with (`scalar_order_matches_the_js`).
    pub const SCALAR_NAMES: [&'static str; 11] = [
        "bob", "lean", "zback", "recoil", "legA", "legB", "armLp", "armRp", "armRaise", "armOut",
        "elbow",
    ];

    /// How the two booleans cross the boundary: bit 0 = the gun hand aims
    /// (`shoot`), bit 1 = skip the head cubes (`headless`) — unpacked by
    /// `planFromScalars` in web/robot-core.js (`flag_bits_match_the_js`).
    pub fn flags(&self) -> u32 {
        self.shoot as u32 | (self.headless as u32) << 1
    }

    /// The eleven scalars, in the order they cross the wasm boundary (the tail
    /// of the `ROBOT` / `PORTRAIT` ops).
    pub fn scalars(&self) -> [f32; 11] {
        [
            self.bob,
            self.lean,
            self.zback,
            self.recoil,
            self.leg_a,
            self.leg_b,
            self.arm_lp,
            self.arm_rp,
            self.arm_raise,
            self.arm_out,
            self.elbow,
        ]
    }
}

/// The frozen clock of the dialogue PORTRAIT bake: a neutral idle frame.
pub const PORTRAIT_BAKE_TIME: f32 = 0.35;

/// The pose every dialogue portrait is baked in (once per colour x framing):
/// unarmed, at ease, frozen at [`PORTRAIT_BAKE_TIME`].
pub fn portrait_pose() -> Pose {
    pose_plan(PoseKind::Idle, PORTRAIT_BAKE_TIME, true)
}

/// Smoothstep of `v` clamped to 0..1.
fn ss(v: f64) -> f64 {
    let v = v.clamp(0.0, 1.0);
    v * v * (3.0 - 2.0 * v)
}

// KICK: the leg cocks back, sweeps through so that the sweep ENDS on the
// impact, holds, then eases back down.
const KICK_SWEEP_SECS: f64 = 0.12;
const KICK_HOLD_SECS: f64 = 0.08;
const KICK_SETTLE_SECS: f64 = 0.19;
// STOMP: the knee jerks up ahead of each impact and slams down ON it.
const STOMP_LIFT_LEAD: f64 = 0.13;
const STOMP_LIFT_SECS: f64 = 0.085;
const STOMP_SLAM_LEAD: f64 = 0.045;

/// The joint drive for `kind` at `time` seconds. `relaxed` (no weapon held)
/// softens idle / walk into an off-duty stance — arms hanging loose at the
/// sides, slightly splayed, a soft elbow bend, an easy walk swing; combat
/// and impact poses ignore it. `time` is the continuous animation clock for
/// the looping poses, and seconds SINCE THE EVENT for `Downed*` (the
/// knockdown), `Kick` and `Stomp` (the finisher's own timer).
///
/// Computed in f64 and rounded once, like the JS (doubles into a
/// `Float32Array`).
pub fn pose_plan(kind: PoseKind, time: f32, relaxed: bool) -> Pose {
    use std::f64::consts::PI;
    let time = time as f64;
    let walk_phase = time * 2.0 * PI;
    let swing = walk_phase.sin() * 0.6;
    let swing2 = (walk_phase + PI).sin() * 0.6;

    // (bob, lean, zback, recoil, legA, legB, armLp, armRp, armRaise, armOut, elbow)
    let n = Pose::NEUTRAL;
    let mut p: [f64; 11] = [
        0.0,
        0.0,
        0.0,
        0.0,
        0.0,
        0.0,
        n.arm_lp as f64,
        n.arm_rp as f64,
        0.0,
        0.0,
        0.0,
    ];
    const BOB: usize = 0;
    const LEAN: usize = 1;
    const ZBACK: usize = 2;
    const RECOIL: usize = 3;
    const LEG_A: usize = 4;
    const LEG_B: usize = 5;
    const ARM_LP: usize = 6;
    const ARM_RP: usize = 7;
    const ARM_RAISE: usize = 8;
    const ARM_OUT: usize = 9;
    const ELBOW: usize = 10;
    // The literal defaults, not the f32-rounded ones, for parity.
    p[ARM_LP] = 0.05;
    p[ARM_RP] = 0.05;

    match kind {
        PoseKind::Walk => {
            p[BOB] = walk_phase.sin().abs() * 0.08;
            p[LEG_A] = swing;
            p[LEG_B] = swing2;
            // Arms counter-swing to the legs.
            p[ARM_LP] = swing2;
            p[ARM_RP] = swing;
            if relaxed {
                // Natural unarmed walk: an easy half swing, arms loose.
                p[ARM_LP] = swing2 * 0.55;
                p[ARM_RP] = swing * 0.55;
                p[ARM_OUT] = 0.10;
                p[ELBOW] = 0.28;
            }
        }
        PoseKind::Shoot => {
            p[LEG_A] = 0.12;
            p[LEG_B] = -0.12;
            p[ARM_LP] = 0.5; // the support arm braces forward-ish
            p[RECOIL] = (time * 10.0).sin().max(0.0) * 0.18;
        }
        PoseKind::Idle => {
            let breath = (time * 1.9).sin();
            p[BOB] = breath * 0.045; // gentle chest / torso bob
            p[LEG_A] = 0.015; // weight shift, feet planted
            p[LEG_B] = -0.015;
            p[ARM_LP] = 0.08 + breath * 0.05; // arms sway slightly out of phase
            p[ARM_RP] = 0.08 - breath * 0.05;
            if relaxed {
                // At ease: arms hang straight down the sides, breathing.
                p[ARM_LP] = 0.02 + breath * 0.03;
                p[ARM_RP] = 0.02 - breath * 0.03;
                p[ARM_OUT] = 0.14 + breath * 0.02;
                p[ELBOW] = 0.14;
            }
        }
        PoseKind::Hit => {
            // A periodic flinch: a sharp recoil back that decays, then repeats.
            // (`%` is the truncated remainder in both JS and Rust.)
            let period = 1.3;
            let phase = (time % period) / period;
            let env = (-phase * 7.0).exp(); // spike at impact, quick decay
            p[LEAN] = 0.55 * env; // the whole body rocks backward
            p[ZBACK] = -0.28 * env; // and shoves back off its feet
            p[BOB] = -0.05 * env;
            p[LEG_A] = -0.25 * env;
            p[LEG_B] = 0.18 * env;
            p[ARM_RAISE] = 0.9 * env; // arms fling up defensively
            p[ARM_LP] = 0.2;
            p[ARM_RP] = 0.2;
        }
        PoseKind::Downed | PoseKind::DownedHeadless => {
            // Knocked flat on its back, limbs askew. The first ~0.25 s eases
            // from upright to sprawled (the fall) with a decaying landing
            // wobble, then the body lies still. The game sets the facing so
            // the body topples AWAY from the blow.
            let k = (time / 0.25).min(1.0);
            let e = k * k * (3.0 - 2.0 * k); // smoothstep fall
            let s = if time > 0.25 {
                ((time - 0.25) * 9.0).sin() * (-(time - 0.25) * 3.0).exp()
            } else {
                0.0
            };
            p[LEAN] = 1.42 * e + 0.10 * s; // topple flat onto its back
            p[ZBACK] = 0.35 * e; // slide with the blow's momentum
            p[BOB] = -0.06 * e;
            p[LEG_A] = 0.55 * e; // legs splayed
            p[LEG_B] = -0.38 * e;
            p[ARM_LP] = -0.45 * e; // arms askew...
            p[ARM_RP] = 0.35 * e;
            p[ARM_RAISE] = 1.25 * e; // ...flung up past the head
        }
        PoseKind::Kick => {
            // The head-kick finisher: the kicking leg cocks back, sweeps
            // through horizontally so the sweep ENDS on the impact, then
            // eases back down while the body leans back off the kick.
            let hit = kick_impact();
            let sweep_start = hit - KICK_SWEEP_SECS;
            let t = time.max(0.0);
            let wind = ss(t / sweep_start);
            let sweep = ss((t - sweep_start) / KICK_SWEEP_SECS);
            let settle = ss((t - (hit + KICK_HOLD_SECS)) / KICK_SETTLE_SECS);
            let k = 1.0 - settle * 0.85;
            p[LEG_A] = 0.14; // support leg planted
            p[LEG_B] = (0.60 * wind - 2.20 * sweep) * k; // windup -> full extension
            p[LEAN] = (0.14 * wind + 0.38 * sweep) * k; // torso leans back
            p[BOB] = -0.05 * sweep * k;
            p[ARM_LP] = -0.70 * sweep * k; // arms scissor for balance:
            p[ARM_RP] = 0.55 * sweep * k; // left forward, right back
            p[ARM_OUT] = 0.16;
            p[ELBOW] = 0.20;
        }
        PoseKind::Stomp => {
            // The two-hit quick stomp: the knee jerks up ahead of each
            // scheduled impact and slams down ON it.
            let t = time.max(0.0);
            let lift = FinisherKind::Stomp
                .impacts()
                .iter()
                .map(|&ti| {
                    let ti = round_cs(ti);
                    (ss((t - (ti - STOMP_LIFT_LEAD)) / STOMP_LIFT_SECS)
                        - ss((t - (ti - STOMP_SLAM_LEAD)) / STOMP_SLAM_LEAD))
                    .max(0.0)
                })
                .fold(0.0, f64::max);
            p[LEG_A] = 0.10; // support leg planted
            p[LEG_B] = -1.05 * lift; // stomping knee hiked up forward
            p[LEAN] = -0.10 * lift; // slight crouch over the body
            p[BOB] = 0.05 * lift - 0.02;
            p[ARM_RAISE] = 0.35 * lift; // arms pump with each stomp
            p[ARM_LP] = 0.25;
            p[ARM_RP] = 0.25;
        }
    }

    Pose {
        bob: p[BOB] as f32,
        lean: p[LEAN] as f32,
        zback: p[ZBACK] as f32,
        recoil: p[RECOIL] as f32,
        leg_a: p[LEG_A] as f32,
        leg_b: p[LEG_B] as f32,
        arm_lp: p[ARM_LP] as f32,
        arm_rp: p[ARM_RP] as f32,
        arm_raise: p[ARM_RAISE] as f32,
        arm_out: p[ARM_OUT] as f32,
        elbow: p[ELBOW] as f32,
        shoot: kind == PoseKind::Shoot,
        headless: kind == PoseKind::DownedHeadless,
    }
}

/// An impact time as the exact decimal the design means (`0.28f32` widens to
/// 0.2800000011920929; the choreography is authored in centiseconds).
fn round_cs(t: f32) -> f64 {
    (t as f64 * 100.0).round() / 100.0
}

/// When the KICK lands — the finisher's one impact.
fn kick_impact() -> f64 {
    round_cs(FinisherKind::Kick.impacts()[0])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::render::robots as idx;

    #[test]
    fn pose_indices_match_the_robot_op_tables() {
        assert_eq!(PoseKind::Idle.index(), idx::ROBOT_POSE_IDLE);
        assert_eq!(PoseKind::Walk.index(), idx::ROBOT_POSE_WALK);
        assert_eq!(PoseKind::Shoot.index(), idx::ROBOT_POSE_SHOOT);
        assert_eq!(PoseKind::Downed.index(), idx::ROBOT_POSE_DOWNED);
        assert_eq!(
            PoseKind::DownedHeadless.index(),
            idx::ROBOT_POSE_DOWNED_HEADLESS
        );
        assert_eq!(PoseKind::Kick.index(), idx::ROBOT_POSE_KICK);
        assert_eq!(PoseKind::Stomp.index(), idx::ROBOT_POSE_STOMP);
        for k in PoseKind::ALL {
            assert_eq!(PoseKind::from_index(k.index()), k);
        }
        assert_eq!(PoseKind::from_index(99), PoseKind::Idle);
    }

    /// The ORDER the scalars travel in: the fixture's header is written from
    /// `POSE_SCALARS` (web/robot-core.js), the very list `planFromScalars`
    /// unpacks the ROBOT op with. So Rust's order == the renderer's, by test.
    #[test]
    fn scalar_order_matches_the_js() {
        let fixture = include_str!("../../tests/fixtures/pose_plan.txt");
        let header = fixture.lines().nth(1).expect("a header line");
        let mut want = vec!["#", "pose", "relaxed", "time"];
        want.extend(Pose::SCALAR_NAMES);
        want.extend(["shoot", "headless"]);
        assert_eq!(header.split(' ').collect::<Vec<_>>(), want);
        // ...and robot-core.js still declares that list, in that order.
        let js = include_str!("../../web/robot-core.js");
        let decl = js
            .split("export const POSE_SCALARS = [")
            .nth(1)
            .expect("POSE_SCALARS");
        let decl = &decl[..decl.find(']').unwrap()];
        let names: Vec<&str> = decl
            .split(',')
            .map(|n| n.trim().trim_matches('"'))
            .collect();
        assert_eq!(names, Pose::SCALAR_NAMES);
    }

    /// The golden record: every row of the fixture — generated from the JS
    /// `posePlan` this module replaced, before it was deleted — must keep
    /// coming out of `pose_plan`. Measured bit-exact when it was recorded; the
    /// tolerance only absorbs a last-bit `sin` / `exp` difference between
    /// platforms' libm. A deliberate pose change updates the rows it affects.
    #[test]
    fn matches_the_golden_record() {
        let fixture = include_str!("../../tests/fixtures/pose_plan.txt");
        let mut rows = 0;
        for line in fixture
            .lines()
            .filter(|l| !l.starts_with('#') && !l.is_empty())
        {
            let f: Vec<&str> = line.split(' ').collect();
            assert_eq!(f.len(), 16, "malformed fixture row: {line}");
            let kind = PoseKind::ALL
                .into_iter()
                .find(|k| k.name() == f[0])
                .unwrap_or_else(|| panic!("unknown pose {}", f[0]));
            let relaxed = f[1] == "1";
            let time: f32 = f[2].parse().unwrap();
            let got = pose_plan(kind, time, relaxed);
            for (i, want) in f[3..14].iter().enumerate() {
                let want: f32 = want.parse().unwrap();
                let have = got.scalars()[i];
                assert!(
                    (have - want).abs() <= 1e-5,
                    "{} relaxed={relaxed} t={time}: scalar {i} = {have}, the golden record says {want}",
                    f[0]
                );
            }
            assert_eq!(got.shoot, f[14] == "1", "{line}");
            assert_eq!(got.headless, f[15] == "1", "{line}");
            rows += 1;
        }
        assert!(rows > 500, "the fixture looks truncated: {rows} rows");
    }

    /// The two booleans travel as flag bits; `planFromScalars` must read the
    /// bits `Pose::flags` writes.
    #[test]
    fn flag_bits_match_the_js() {
        let shoot = Pose {
            shoot: true,
            ..Pose::NEUTRAL
        };
        let headless = Pose {
            headless: true,
            ..Pose::NEUTRAL
        };
        assert_eq!(
            (Pose::NEUTRAL.flags(), shoot.flags(), headless.flags()),
            (0, 1, 2)
        );
        let js = include_str!("../../web/robot-core.js");
        assert!(
            js.contains("plan.shoot = (flags & 1) !== 0;"),
            "shoot is not bit 0"
        );
        assert!(
            js.contains("plan.headless = (flags & 2) !== 0;"),
            "headless is not bit 1"
        );
    }

    /// There is ONE pose implementation: no JS file may grow another.
    #[test]
    fn no_js_pose_logic_is_left() {
        for (name, src) in [
            ("web/robot-core.js", include_str!("../../web/robot-core.js")),
            ("web/renderer.js", include_str!("../../web/renderer.js")),
            (
                "tools/engine-pose.js",
                include_str!("../../tools/engine-pose.js"),
            ),
        ] {
            assert!(
                !src.contains("function posePlan"),
                "{name} defines a posePlan again"
            );
            assert!(
                !src.contains("walkPhase"),
                "{name} animates a walk cycle in JS"
            );
        }
        assert_eq!(
            portrait_pose(),
            pose_plan(PoseKind::Idle, PORTRAIT_BAKE_TIME, true)
        );
    }

    /// What the port is FOR: the choreography and the gameplay timing are one
    /// source now. The kicking leg reaches full forward extension exactly on
    /// the impact the finisher system schedules...
    #[test]
    fn the_kick_is_fully_extended_on_the_impact() {
        let hit = FinisherKind::Kick.impacts()[0];
        let at = |t: f32| pose_plan(PoseKind::Kick, t, true).leg_b;
        let extended = at(hit);
        assert!(extended < -1.5, "leg_b at the impact = {extended}");
        // ...never further before or after it,
        for i in 0..=100 {
            let t = i as f32 * 0.01;
            assert!(
                at(t) >= extended - 1e-4,
                "leg_b({t}) = {} < {extended}",
                at(t)
            );
        }
        // ...and it is still cocking BACK a tenth of a second earlier.
        assert!(at(hit - 0.14) > 0.0);
    }

    /// ...and the stomping knee is DOWN on each scheduled impact, up between.
    #[test]
    fn each_stomp_lands_on_its_impact() {
        let impacts = FinisherKind::Stomp.impacts();
        let knee = |t: f32| -pose_plan(PoseKind::Stomp, t, true).leg_b; // 0 = down
        for &t in impacts {
            assert!(
                knee(t) < 1e-4,
                "the knee is still up ({}) at the {t} s impact",
                knee(t)
            );
            assert!(knee(t - 0.05) > 0.9, "no wind-up before the {t} s impact");
        }
        assert_eq!(
            impacts.len(),
            2,
            "the pose is choreographed for a two-hit stomp"
        );
    }

    #[test]
    fn a_downed_body_settles_flat_and_stays_there() {
        let late = pose_plan(PoseKind::Downed, 5.0, false);
        assert!((late.lean - 1.42).abs() < 1e-3, "lean = {}", late.lean);
        let later = pose_plan(PoseKind::Downed, 9.0, false);
        assert!(
            (late.lean - later.lean).abs() < 1e-4,
            "a corpse must not keep wobbling"
        );
        // The headless variant is the same body, minus the head.
        let headless = pose_plan(PoseKind::DownedHeadless, 5.0, false);
        assert_eq!(headless.scalars(), late.scalars());
        assert!(headless.headless && !late.headless);
    }

    #[test]
    fn only_idle_and_walk_relax_and_every_scalar_stays_finite() {
        for kind in PoseKind::ALL {
            for i in -5..400 {
                let t = i as f32 * 0.025;
                let (armed, relaxed) = (pose_plan(kind, t, false), pose_plan(kind, t, true));
                assert!(armed
                    .scalars()
                    .iter()
                    .chain(relaxed.scalars().iter())
                    .all(|v| v.is_finite()));
                if !matches!(kind, PoseKind::Idle | PoseKind::Walk) {
                    assert_eq!(armed, relaxed, "{kind:?} must ignore `relaxed`");
                }
            }
        }
    }
}
