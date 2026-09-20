//! `boss_spheres`: where every sphere of the shoggoth is at time `t` — the
//! writhing mass, the smiley mask and its break-up, the lashing tentacles,
//! the pale dot eyes. Pure numbers: no `Graphics`, no browser.
//!
//! The boss is NOTHING BUT SPHERES: web/shoggoth-core.js draws a list of
//! them (two instanced draws; it owns the camera, the shading and the
//! inking) and holds no animation. This module is that list's one source
//! (roadmap: docs/ARCHITECTURE.md): the game ships it in the stream
//! (`Graphics::draw_shoggoth_live`: `SPHERE` ops closed by a `SHOGGOTH`), the
//! tool pages ask the wasm (`src/wasm_api.rs`).
//!
//! It began as a port of the JS it replaced and reproduces it BIT FOR BIT
//! (`tests/fixtures/shoggoth_spheres.txt`, captured from that JS before it
//! was deleted). Two things make that possible and must be kept:
//!   - the JS matrices lived in `Float32Array`s: every product was computed in
//!     f64 from f32 inputs and ROUNDED TO f32 at every step ([`Mat`]);
//!   - everything else (angles, radii, colours, ids) was f64 until the moment
//!     it was stored.

/// Floats per sphere: the model's three rows (12), then `r g b id`, then the
/// accent `r g b` + emission — the instance layout of web/shoggoth-core.js.
pub const SPHERE_FLOATS: usize = 20;

/// Seconds the mask-off takes (`reveal` 0 -> 1): the sim's clock, and what the
/// inspector's "transition" plays. ONE definition (`systems::boss`).
pub const MASK_OFF_SECS: f32 = crate::systems::boss::BOSS_MASK_OFF_SECS;

/// "A full turn" AS THE ORIGINAL WROTE IT — deliberately NOT `TAU`. The
/// animation was tuned with these truncated literals and the golden record
/// holds this module to them bit for bit: `TAU` here would be a (tiny, but
/// real) change of the boss's motion.
#[allow(clippy::approx_constant)]
const TURN_3: f64 = 6.283;
#[allow(clippy::approx_constant)]
const TURN_2: f64 = 6.28;

/// What the boss is doing, as the engine sends it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BossPose {
    /// The continuous animation clock, seconds.
    pub time: f32,
    /// Mask-off progress: 0 = mask intact, (0, 1) = cracking / being consumed
    /// while the tentacles grow, 1 = the raw enraged form.
    pub reveal: f32,
    /// Where the mask leans toward, radians in screen convention. `None` =
    /// the wander behaviour's own heading (the tools' standalone preview).
    pub heading: Option<f32>,
    /// 0..1, how flat toward the camera the mask tilts. `None` = the wander
    /// behaviour's "stop and look up" beats — WHAT THE GAME USES.
    pub look_up: Option<f32>,
    /// Drift the body along the wander path (the tools' preview; the game
    /// moves the boss itself).
    pub wander: bool,
}

/// One frame of the boss: `count()` spheres of [`SPHERE_FLOATS`] floats. The
/// first `mask_at` are the BODY (drawn with the depth test on); the rest are
/// the MASK ASSEMBLY, drawn depth-OFF in order so the mask always sits over
/// the lobes and its shards stay visible while they are consumed.
#[derive(Debug, Clone, PartialEq)]
pub struct BossSpheres {
    pub data: Vec<f32>,
    pub mask_at: usize,
}

impl BossSpheres {
    pub fn count(&self) -> usize {
        self.data.len() / SPHERE_FLOATS
    }
}

// ---- palette (the boss is not recoloured) ----------------------------------
type Rgb = [f64; 3];
const C_BODY: Rgb = [0.11, 0.14, 0.13]; // dark flesh
const C_BODYA: Rgb = [0.17, 0.22, 0.19]; // its top-lit accent
const C_PURPLE: Rgb = [0.14, 0.10, 0.17]; // bruised purple lobes
const C_PURPLEA: Rgb = [0.20, 0.15, 0.24];
const C_TIP: Rgb = [0.34, 0.12, 0.13]; // tentacle tips, wet red
const C_TIPA: Rgb = [0.48, 0.18, 0.17];
const C_MASK: Rgb = [1.00, 0.83, 0.14]; // friendly yellow smiley
const C_INK: Rgb = [0.05, 0.04, 0.02]; // dark features on the mask
const C_YEYE: Rgb = [1.00, 0.95, 0.55]; // pale-yellow dot eyes (raw form)
const C_YHOT: Rgb = [1.00, 1.00, 0.86]; // hot core of the dot

fn clamp01(x: f64) -> f64 {
    x.clamp(0.0, 1.0)
}
fn smoothstep(a: f64, b: f64, x: f64) -> f64 {
    let t = clamp01((x - a) / (b - a));
    t * t * (3.0 - 2.0 * t)
}
fn ease(x: f64) -> f64 {
    smoothstep(0.0, 1.0, x)
}
fn mix(a: f64, b: f64, t: f64) -> f64 {
    a + (b - a) * t
}
fn mix3(a: Rgb, b: Rgb, t: f64) -> Rgb {
    [mix(a[0], b[0], t), mix(a[1], b[1], t), mix(a[2], b[2], t)]
}
/// Deterministic pseudo-random in 0..1.
fn hash(i: f64) -> f64 {
    let s = (i * 127.1 + 0.7).sin() * 43758.5453;
    s - s.floor()
}

/// A column-major 4x4 with f32 STORAGE and f64 arithmetic — the JS `M4` over
/// `Float32Array`s, rounding included (see the module docs).
#[derive(Clone, Copy)]
struct Mat([f32; 16]);

impl Mat {
    fn ident() -> Mat {
        let mut m = [0.0f32; 16];
        m[0] = 1.0;
        m[5] = 1.0;
        m[10] = 1.0;
        m[15] = 1.0;
        Mat(m)
    }
    fn translate(x: f64, y: f64, z: f64) -> Mat {
        let mut m = Mat::ident();
        m.0[12] = x as f32;
        m.0[13] = y as f32;
        m.0[14] = z as f32;
        m
    }
    fn scale(x: f64, y: f64, z: f64) -> Mat {
        let mut m = Mat::ident();
        m.0[0] = x as f32;
        m.0[5] = y as f32;
        m.0[10] = z as f32;
        m
    }
    fn rot_x(a: f64) -> Mat {
        let (s, c) = a.sin_cos();
        let mut m = Mat::ident();
        m.0[5] = c as f32;
        m.0[6] = s as f32;
        m.0[9] = (-s) as f32;
        m.0[10] = c as f32;
        m
    }
    fn rot_y(a: f64) -> Mat {
        let (s, c) = a.sin_cos();
        let mut m = Mat::ident();
        m.0[0] = c as f32;
        m.0[2] = (-s) as f32;
        m.0[8] = s as f32;
        m.0[10] = c as f32;
        m
    }
    fn rot_z(a: f64) -> Mat {
        let (s, c) = a.sin_cos();
        let mut m = Mat::ident();
        m.0[0] = c as f32;
        m.0[1] = s as f32;
        m.0[4] = (-s) as f32;
        m.0[5] = c as f32;
        m
    }
    /// `self * b`: each element summed left to right in f64, then rounded.
    fn mul(&self, b: &Mat) -> Mat {
        let (a, b) = (&self.0, &b.0);
        let mut o = [0.0f32; 16];
        for r in 0..4 {
            for c in 0..4 {
                let v = a[r] as f64 * b[c * 4] as f64
                    + a[4 + r] as f64 * b[c * 4 + 1] as f64
                    + a[8 + r] as f64 * b[c * 4 + 2] as f64
                    + a[12 + r] as f64 * b[c * 4 + 3] as f64;
                o[c * 4 + r] = v as f32;
            }
        }
        Mat(o)
    }
}

/// The sphere list being built.
struct Out {
    data: Vec<f32>,
}

impl Out {
    fn sphere(&mut self, m: &Mat, col: Rgb, accent: Rgb, id: f64, emis: f64) {
        let m = &m.0;
        // row r of a column-major matrix = m[r], m[4 + r], m[8 + r], m[12 + r]
        for r in 0..3 {
            self.data
                .extend_from_slice(&[m[r], m[4 + r], m[8 + r], m[12 + r]]);
        }
        self.data
            .extend_from_slice(&[col[0] as f32, col[1] as f32, col[2] as f32, id as f32]);
        self.data.extend_from_slice(&[
            accent[0] as f32,
            accent[1] as f32,
            accent[2] as f32,
            emis as f32,
        ]);
    }
    #[allow(clippy::too_many_arguments)]
    fn blob(&mut self, root: &Mat, p: [f64; 3], r: [f64; 3], col: Rgb, acc: Rgb, id: f64, em: f64) {
        let m = root.mul(&Mat::translate(p[0], p[1], p[2]).mul(&Mat::scale(r[0], r[1], r[2])));
        self.sphere(&m, col, acc, id, em);
    }
}

// ---- THE WANDER BEHAVIOUR ----------------------------------------------------
// A tiny deterministic state machine stepped at a fixed dt: it drifts along a
// heading, occasionally stops and looks up, then picks a new heading. The GAME
// uses its "look up" beats for the mask's tilt (it sends no `look_up`); the
// tools also use its heading and drift.

#[derive(Debug, Clone, Copy, PartialEq)]
struct Wander {
    x: f64,
    z: f64,
    heading: f64,
    tgt: f64,
    mode: u8,
    mode_t: f64,
    dur: f64,
    look: f64,
    dec: f64,
}

impl Wander {
    fn new() -> Wander {
        Wander {
            x: 0.0,
            z: 0.0,
            heading: hash(1.0) * TURN_3,
            tgt: hash(1.0) * TURN_3,
            mode: 0,
            mode_t: 0.0,
            dur: 3.5 + hash(2.0) * 2.5,
            look: 0.0,
            dec: 1.0,
        }
    }

    fn step(&mut self, dt: f64) {
        use std::f64::consts::PI;
        self.mode_t += dt;
        if self.mode == 0 {
            // drifting
            let mut d = self.tgt - self.heading;
            while d > PI {
                d -= TURN_3;
            }
            while d < -PI {
                d += TURN_3;
            }
            self.heading += d * (dt * 1.6).min(1.0); // steer smoothly toward the target
            let spd = 0.55;
            self.x += self.heading.cos() * spd * dt;
            self.z += self.heading.sin() * spd * dt;
            if self.x.hypot(self.z) > 1.35 {
                // stay in frame: steer back inward
                self.tgt = (-self.z).atan2(-self.x) + (hash(self.dec + 7.0) - 0.5) * 0.8;
            }
            self.look += (0.0 - self.look) * (dt * 3.0).min(1.0);
            if self.mode_t > self.dur {
                self.mode = 1;
                self.mode_t = 0.0;
                self.dur = 1.8 + hash(self.dec + 3.0) * 1.6;
                self.dec += 1.0;
            }
        } else {
            // stopped, looking up
            self.look += (1.0 - self.look) * (dt * 2.6).min(1.0);
            if self.mode_t > self.dur {
                self.mode = 0;
                self.mode_t = 0.0;
                self.dur = 3.2 + hash(self.dec + 3.0) * 2.6;
                self.tgt = self.heading + (hash(self.dec + 5.0) - 0.5) * 3.2;
                self.dec += 1.0;
            }
        }
    }
}

/// Whole 1/60 s steps already simulated, so a frame costs O(1) instead of
/// re-simulating from 0. A CACHE, not state: `wander_at(t)` is a pure function
/// of `t` — whole steps, then one partial step on a COPY — whatever order
/// times are asked in (the JS persisted the partial step, which made its
/// result depend on the frame timing; a fresh JS pipeline asked once for `t`
/// gives exactly this, and that is what the golden record holds).
struct WanderCache {
    state: Wander,
    sim_t: f64,
}

thread_local! {
    static WANDER: std::cell::RefCell<WanderCache> =
        std::cell::RefCell::new(WanderCache { state: Wander::new(), sim_t: 0.0 });
}

fn wander_at(t: f64) -> Wander {
    const DT: f64 = 1.0 / 60.0;
    WANDER.with(|cache| {
        let mut c = cache.borrow_mut();
        if t < c.sim_t - 1e-6 {
            *c = WanderCache {
                state: Wander::new(),
                sim_t: 0.0,
            };
        }
        let mut guard = 0;
        while c.sim_t < t - 1e-9 && guard < 300_000 {
            let s = DT.min(t - c.sim_t);
            if s < DT {
                // the partial last step: on a copy, never persisted
                let mut now = c.state;
                now.step(s);
                return now;
            }
            c.state.step(s);
            c.sim_t += s;
            guard += 1;
        }
        c.state
    })
}

/// The boss's heading / look-up / drift at `time` when the caller leaves them
/// to the wander behaviour: `[heading, look, x, z]`.
pub fn wander(time: f32) -> [f32; 4] {
    let w = wander_at(time as f64);
    [w.heading as f32, w.look as f32, w.x as f32, w.z as f32]
}

// ---- THE PARTS ---------------------------------------------------------------

/// THE WRITHING MASS: one core + orbiting satellite lobes.
fn mass(out: &mut Out, root: &Mat, time: f64, frantic: bool) {
    const LOBES: usize = 9;
    let core_sq = 1.0 + 0.06 * (time * 0.9).sin();
    out.blob(
        root,
        [0.0, 0.0, 0.0],
        [1.65, 1.35 * core_sq, 1.65],
        C_BODY,
        C_BODYA,
        0.14,
        0.0,
    );
    out.blob(
        root,
        [0.25 * (time * 0.6).sin(), 0.15, -0.2 * (time * 0.5).cos()],
        [1.25, 1.15, 1.3],
        C_PURPLE,
        C_PURPLEA,
        0.22,
        0.0,
    );
    for k in 0..LOBES {
        let kf = k as f64;
        let a = (kf / LOBES as f64) * std::f64::consts::PI * 2.0;
        let spd = 0.4 + hash(kf) * 0.5;
        let ph = hash(kf + 10.0) * TURN_2;
        let wob = if frantic { 0.55 } else { 0.28 };
        let rad = 1.15 + hash(kf + 3.0) * 0.5 + (time * spd + ph).sin() * wob;
        let yb = -0.35
            + (time * spd * 1.3 + ph).sin() * (if frantic { 0.5 } else { 0.28 })
            + hash(kf + 7.0) * 0.5;
        let orbit = a + time * (if frantic { 0.5 } else { 0.22 });
        let (x, z) = (orbit.cos() * rad, orbit.sin() * rad);
        let r = 0.62 + hash(kf + 5.0) * 0.45;
        let pr = 1.0 + 0.14 * (time * 1.7 + ph).sin();
        let purple = hash(kf + 2.0) > 0.55;
        out.blob(
            root,
            [x, yb, z],
            [r * pr, r * (0.85 + 0.2 * (time + ph).sin()), r * pr],
            if purple { C_PURPLE } else { C_BODY },
            if purple { C_PURPLEA } else { C_BODYA },
            0.30 + kf * 0.055,
            0.0,
        );
    }
}

/// THE SMILEY MASK / MASK-OFF TRANSITION: a ring of yellow wedge shards + a
/// centre cap + the ink features. At `reveal` 0 the shards overlap into a
/// clean dome; as it climbs the shell cracks, then shards and features are
/// SUCKED INWARD and down into the maw — shrinking, spiralling, darkening to
/// dead flesh. `mroot` already places / rotates the mask on the crown.
fn mask_assembly(out: &mut Out, mroot: &Mat, time: f64, reveal: f64) {
    use std::f64::consts::PI;
    let s = ease(clamp01(reveal * 1.05)); // consume progress 0..1
    let jitter = if reveal > 0.02 && reveal < 0.55 {
        reveal * 0.05
    } else {
        0.0
    };
    let dk = smoothstep(0.42, 1.0, reveal);
    let shard_col = mix3(C_MASK, C_BODY, dk * 0.95);
    let shard_em = 0.85 * (1.0 - smoothstep(0.30, 1.0, reveal));
    let shrink = mix(1.0, 0.06, s); // shards shrink as they are pulled in
    let in_r = 1.0 - s; // the ring radius collapses toward the maw
    let sink = s * 0.6; // slight downward drift into the maw

    // hairline cracks that appear just before the shell lets go
    if reveal > 0.02 && reveal < 0.5 {
        let ca = smoothstep(0.02, 0.16, reveal) * (1.0 - smoothstep(0.36, 0.5, reveal));
        for c in 0..3 {
            let ang = c as f64 * 1.05 + 0.3;
            let m = mroot
                .mul(&Mat::translate(0.0, 0.30, 0.0))
                .mul(&Mat::rot_y(ang))
                .mul(&Mat::scale(1.15 * ca, 0.05, 0.055));
            out.sphere(&m, C_INK, C_INK, 0.58, 0.0);
        }
    }

    // ring of wedge shards: sucked inward + down while spiralling
    const SH: usize = 6;
    for k in 0..SH {
        let kf = k as f64;
        let dir = if k % 2 == 1 { 1.0 } else { -1.0 };
        let a = (kf / SH as f64) * PI * 2.0;
        let swirl = a + s * 2.4 * dir; // spiral into the maw
        let jx = (time * 23.0 + kf).sin() * jitter;
        let jz = (time * 21.0 + kf).cos() * jitter;
        let px = swirl.cos() * 0.60 * in_r + jx;
        let pz = swirl.sin() * 0.60 * in_r + jz;
        let py = -sink * (0.6 + hash(kf + 41.0) * 0.5);
        let spin = s * (4.0 + hash(kf + 42.0) * 3.0);
        let m = Mat::translate(px, py, pz)
            .mul(&Mat::rot_z(spin * dir))
            .mul(&Mat::rot_x(spin * 0.6))
            .mul(&Mat::scale(0.62 * shrink, 0.26 * shrink, 0.62 * shrink));
        out.sphere(
            &mroot.mul(&m),
            shard_col,
            shard_col,
            0.80 + kf * 0.006,
            shard_em,
        );
    }
    // centre cap: shrinks down into the maw
    {
        let m = Mat::translate(0.0, -sink * 0.8, 0.0)
            .mul(&Mat::rot_x(s * 4.0))
            .mul(&Mat::scale(0.66 * shrink, 0.30 * shrink, 0.66 * shrink));
        out.sphere(&mroot.mul(&m), shard_col, shard_col, 0.79, shard_em);
    }

    // ink features (two eyes + an upward smile), drawn in toward the centre
    let fy = 0.42;
    let feat = |x: f64, z: f64| -> Mat {
        let m = Mat::translate(x * in_r, fy - sink, z * in_r)
            .mul(&Mat::rot_z(s * 5.0))
            .mul(&Mat::scale(shrink, shrink, shrink));
        mroot.mul(&m)
    };
    for x in [-0.44, 0.44] {
        let m = feat(x, -0.42).mul(&Mat::scale(0.20, 0.15, 0.24));
        out.sphere(&m, C_INK, C_INK, 0.55, 0.0);
    }
    const N: usize = 7;
    for i in 0..N {
        let tt = (i as f64 / (N - 1) as f64) * 2.0 - 1.0;
        let x = tt * 0.66;
        let z = 0.16 + 0.34 * (1.0 - tt * tt);
        let m = feat(x, z).mul(&Mat::scale(0.135, 0.12, 0.135));
        out.sphere(&m, C_INK, C_INK, 0.55, 0.0);
    }
}

/// RAW FORM: pale-yellow glowing dot eyes over the crown.
fn yellow_eyes(out: &mut Out, root: &Mat, time: f64, fade: f64) {
    const N: usize = 15;
    for i in 0..N {
        let f = i as f64;
        let a = f * 2.399; // golden-angle scatter
        let rr = 0.18 + hash(f + 30.0) * 0.92; // cluster over the crown
        let x = a.cos() * rr + 0.08 * (time * 1.2 + f).sin();
        let z = a.sin() * rr * 0.9 + 0.08 * (time * 1.0 + f).cos();
        let y = 1.35 + hash(f + 31.0) * 0.65; // up on the crown, in front of the camera
        let tw = 0.85 + 0.15 * (time * 2.5 + f * 1.3).sin().abs(); // gentle twinkle
                                                                   // big enough that the yellow core survives the ink outline
        let s = (0.16 + hash(f + 32.0) * 0.07) * (0.55 + 0.45 * fade);
        out.blob(
            root,
            [x, y, z],
            [s, s, s],
            C_YEYE,
            C_YEYE,
            0.62 + f * 0.012,
            tw * fade,
        );
        let h = s * 0.45;
        out.blob(
            root,
            [x, y + 0.03, z],
            [h, h, h],
            C_YHOT,
            C_YHOT,
            0.92,
            fade,
        );
    }
}

/// Many chunky lashing tentacles. `grow` scales them in during the transition
/// (0 hidden .. 1 full length); `eye_fade` fades in the little pale-yellow dot
/// eyes that also stud the arms and tips.
fn tentacles(out: &mut Out, root: &Mat, time: f64, grow: f64, frantic: bool, eye_fade: f64) {
    const T: usize = 11;
    const SEG: usize = 8;
    for k in 0..T {
        let kf = k as f64;
        let a = (kf / T as f64) * std::f64::consts::PI * 2.0 + 0.3;
        let mut m = root
            .mul(&Mat::translate(a.cos() * 1.20, -0.05, a.sin() * 1.20))
            .mul(&Mat::rot_y(-a)) // face outward
            .mul(&Mat::rot_z(-1.05)); // tip the up-axis outward
        let seglen = 0.60 * grow;
        for i in 0..SEG {
            let f = i as f64;
            let bend = (time * 2.5 + f * 0.9 + kf * 1.7).sin() * (if frantic { 0.62 } else { 0.4 });
            let sweep =
                (time * 1.9 + f * 0.7 + kf * 2.1).cos() * (if frantic { 0.55 } else { 0.35 });
            m = m.mul(&Mat::rot_x(sweep)).mul(&Mat::rot_z(bend * 0.5));
            let tp = f / (SEG - 1) as f64;
            let r = 0.50 * (1.0 - tp * 0.70);
            let seg = m.mul(&Mat::translate(0.0, seglen * 0.5, 0.0).mul(&Mat::scale(
                r,
                seglen * 0.62,
                r,
            )));
            out.sphere(
                &seg,
                mix3(C_BODY, C_TIP, tp),
                mix3(C_BODYA, C_TIPA, tp),
                0.30 + kf * 0.03 + f * 0.005,
                0.0,
            );
            // dot eyes on the arm: always one at the tip, plus a few scattered along
            if eye_fade > 0.02 && grow > 0.6 && (i == SEG - 1 || hash(kf * 13.0 + f + 50.0) > 0.62)
            {
                let es = (r * 0.75).max(0.13);
                let tw = 0.82 + 0.18 * (time * 2.6 + kf * 1.3 + f).sin().abs();
                out.blob(
                    &m,
                    [0.0, seglen * 0.5, r * 0.85],
                    [es, es, es],
                    C_YEYE,
                    C_YEYE,
                    0.66 + kf * 0.02 + f * 0.006,
                    tw * eye_fade,
                );
                let h = es * 0.45;
                out.blob(
                    &m,
                    [0.0, seglen * 0.55, r * 0.9],
                    [h, h, h],
                    C_YHOT,
                    C_YHOT,
                    0.94,
                    eye_fade,
                );
            }
            m = m.mul(&Mat::translate(0.0, seglen, 0.0));
        }
    }
}

/// The whole boss for one frame.
pub fn boss_spheres(pose: &BossPose) -> BossSpheres {
    let time = pose.time as f64;
    let reveal = clamp01(pose.reveal as f64);

    // behaviour: heading / look-up beat / drift, unless the caller drives them
    let b = wander_at(time);
    let heading = pose.heading.map_or(b.heading, |h| h as f64);
    let mut look_up = pose.look_up.map_or(b.look, |l| clamp01(l as f64));
    // it rears up to stare as the mask lets go
    if reveal > 0.0 && reveal < 1.0 {
        look_up = look_up.max(1.0 - smoothstep(0.06, 0.32, reveal));
    }
    let drift = if pose.wander { [b.x, b.z] } else { [0.0, 0.0] };

    // drift eases to centre as it rears up; spin & list ramp in with the raw form
    let dr = 1.0 - ease(reveal);
    let spin = ease(reveal) * 0.32;
    let list = 0.05 + ease(reveal) * 0.07;
    let body_root = Mat::translate(drift[0] * dr, 0.0, drift[1] * dr)
        .mul(&Mat::rot_y(time * spin))
        .mul(&Mat::rot_z((time * 0.5).sin() * list))
        .mul(&Mat::rot_x((time * 0.42).cos() * list));

    let frantic = reveal > 0.5;
    let grow = smoothstep(0.12, 0.95, reveal);
    let eye_fade = smoothstep(0.35, 1.0, reveal);

    let mut out = Out {
        data: Vec::with_capacity(240 * SPHERE_FLOATS),
    };
    if grow > 0.02 {
        tentacles(&mut out, &body_root, time, grow, frantic, eye_fade);
    }
    mass(&mut out, &body_root, time, frantic);
    if eye_fade > 0.02 {
        yellow_eyes(&mut out, &body_root, time, eye_fade);
    }

    // the mask / its break-up (skipped once fully gone)
    let mask_at = out.data.len() / SPHERE_FLOATS;
    if reveal < 0.999 {
        let y_top = 1.55; // ride high on the crown, above the lobes
        let lean = (1.0 - look_up) * 0.34; // lean toward the heading when moving; flat when looking up
        let mroot = body_root
            .mul(&Mat::translate(0.0, y_top, 0.05))
            // face (the smile side, local +z) toward the heading, then tilt forward
            .mul(&Mat::rot_y(std::f64::consts::PI / 2.0 - heading))
            .mul(&Mat::rot_x(lean));
        mask_assembly(&mut out, &mroot, time, reveal);
    }
    BossSpheres {
        data: out.data,
        mask_at,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pose(time: f32, reveal: f32) -> BossPose {
        BossPose {
            time,
            reveal,
            heading: Some(0.7),
            look_up: None,
            wander: false,
        }
    }

    /// The golden record: sphere lists captured from the JS placement code
    /// this module replaced (web/shoggoth-core.js, run under a mock GL), before
    /// it was deleted. Small frames in full; big ones every 4th sphere + the
    /// last — a wrong matrix chain corrupts every later sphere anyway. Measured
    /// BIT-EXACT when recorded; the tolerance only absorbs a last-bit `sin` /
    /// `cos` difference between platforms' libm.
    #[test]
    fn matches_the_golden_record() {
        let fixture = include_str!("../../tests/fixtures/shoggoth_spheres.txt");
        let (mut cases, mut rows, mut worst) = (0, 0, 0.0f32);
        let mut current: Option<BossSpheres> = None;
        for line in fixture
            .lines()
            .filter(|l| !l.starts_with('#') && !l.is_empty())
        {
            let f: Vec<&str> = line.split(' ').collect();
            if f[0] == "case" {
                let opt = |s: &str| (s != "-").then(|| s.parse::<f32>().unwrap());
                let got = boss_spheres(&BossPose {
                    reveal: f[1].parse().unwrap(),
                    time: f[2].parse().unwrap(),
                    heading: opt(f[3]),
                    look_up: opt(f[4]),
                    wander: f[5] == "1",
                });
                assert_eq!(
                    got.count(),
                    f[6].parse::<usize>().unwrap(),
                    "sphere count: {line}"
                );
                assert_eq!(
                    got.mask_at,
                    f[7].parse::<usize>().unwrap(),
                    "mask split: {line}"
                );
                current = Some(got);
                cases += 1;
                continue;
            }
            let got = current.as_ref().expect("a `case` line first");
            let i: usize = f[0].parse().unwrap();
            assert_eq!(f.len(), 1 + SPHERE_FLOATS, "malformed row: {line}");
            for (k, want) in f[1..].iter().enumerate() {
                let want: f32 = want.parse().unwrap();
                let have = got.data[i * SPHERE_FLOATS + k];
                worst = worst.max((have - want).abs());
                assert!(
                    (have - want).abs() <= 2e-5,
                    "case {cases}, sphere {i}, float {k}: {have}, the golden record says {want}"
                );
            }
            rows += 1;
        }
        assert!(
            cases >= 10 && rows > 300,
            "the fixture looks truncated: {cases} cases, {rows} rows"
        );
        println!("golden record: {cases} cases, {rows} spheres, worst |diff| = {worst:e}");
    }

    #[test]
    fn the_mask_is_the_tail_of_the_list_and_goes_away() {
        // masked: mass (2 + 9 lobes) then the mask (6 shards + cap + 2 eyes + 7 smile dots)
        let masked = boss_spheres(&pose(1.0, 0.0));
        assert_eq!((masked.mask_at, masked.count()), (11, 27));
        // cracking: three hairline cracks join the mask group
        assert_eq!(boss_spheres(&pose(1.0, 0.1)).count(), 30);
        // raw form: tentacles + eyes, and NO mask spheres at all
        let raw = boss_spheres(&pose(1.0, 1.0));
        assert_eq!(raw.mask_at, raw.count());
        assert!(raw.count() > 150);
        // the renderer's instance buffer grows on demand, but stay sane
        for r in 0..=20 {
            let s = boss_spheres(&pose(3.3, r as f32 / 20.0));
            assert!(s.count() <= 260 && s.mask_at <= s.count());
            assert!(s.data.iter().all(|v| v.is_finite()));
        }
    }

    #[test]
    fn a_frame_is_a_pure_function_of_its_pose() {
        // The wander cache must not leak: any order of queries, same answers.
        let a = boss_spheres(&pose(12.5, 0.0));
        let _ = boss_spheres(&pose(40.0, 0.4));
        let _ = boss_spheres(&pose(0.1, 1.0));
        assert_eq!(boss_spheres(&pose(12.5, 0.0)), a);
        assert_eq!(wander(7.25), wander(7.25));
        let (early, late) = (wander(0.0), wander(30.0));
        assert_ne!(early, late, "the behaviour never moved");
    }

    #[test]
    fn the_game_gets_its_look_up_beats_from_the_wander() {
        // The game sends a heading but no `look_up`: the mask's tilt follows the
        // behaviour's "stop and stare" beats, so it must change over time...
        let looks: Vec<f32> = (0..400).map(|i| wander(i as f32 * 0.1)[1]).collect();
        let (lo, hi) = looks
            .iter()
            .fold((1.0f32, 0.0f32), |(l, h), &v| (l.min(v), h.max(v)));
        assert!(lo < 0.05 && hi > 0.9, "look-up never beats: {lo}..{hi}");
        // ...and an explicit value overrides it.
        let fixed = BossPose {
            look_up: Some(1.0),
            ..pose(2.0, 0.0)
        };
        assert_ne!(boss_spheres(&fixed), boss_spheres(&pose(2.0, 0.0)));
    }

    /// The 20-float sphere exists on both sides of the boundary: the `SPHERE`
    /// op's arity, this module's layout and the instance layout of `instVS`.
    #[test]
    fn the_sphere_layout_matches_the_op_and_the_js() {
        use crate::graphics::{op, stream::OP_ARGS};
        assert_eq!(OP_ARGS[op::SPHERE as usize], SPHERE_FLOATS);
        let js = include_str!("../../web/shoggoth-core.js");
        assert!(
            js.contains("export const SPHERE_FLOATS = 20;"),
            "shoggoth-core.js SPHERE_FLOATS"
        );
        // rows 0..2, then colour + id, then accent + emission: 5 vec4 attributes
        for attr in ["aM0", "aM1", "aM2", "aColId", "aAccEm"] {
            assert!(
                js.contains(&format!("attribute vec4 {attr};")),
                "instVS lost {attr}"
            );
        }
        assert_eq!(MASK_OFF_SECS, crate::systems::boss::BOSS_MASK_OFF_SECS);
    }

    /// There is ONE place the boss is animated: no JS file may grow another.
    #[test]
    fn no_js_boss_animation_is_left() {
        for (name, src) in [
            (
                "web/shoggoth-core.js",
                include_str!("../../web/shoggoth-core.js"),
            ),
            ("web/renderer.js", include_str!("../../web/renderer.js")),
            (
                "tools/engine-pose.js",
                include_str!("../../tools/engine-pose.js"),
            ),
        ] {
            for gone in ["_drawTentacles", "_drawMass", "stepBeh", "function hash("] {
                assert!(!src.contains(gone), "{name} contains `{gone}` again");
            }
        }
        // The mask-off duration is the ENGINE's: the runtime never names it, and
        // the tools read it from the wasm instead of hard-coding a number.
        assert!(!include_str!("../../web/shoggoth-core.js").contains("MASK_OFF_SECS"));
        assert!(include_str!("../../tools/engine-pose.js")
            .contains("export const MASK_OFF_SECS = boss_mask_off_secs();"));
    }
}
