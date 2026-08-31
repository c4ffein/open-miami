//! ELECTRIC SPARKS — the impact burst when a player attack lands on a robot.
//!
//! When a bullet (or a melee blow) damages a bot, a short-lived crackle of
//! pixelated lightning pops at the impact point: 2-3 jagged cyan/white arcs
//! plus a couple of stray spark dots, with a hot-pink flash on the very first
//! flicker frame. The look follows the project vibe (CLAUDE.md ## Design):
//! each spark renders as a small PIXEL-ART GROUP (`Graphics::pixel_begin`,
//! plain snapped composite — the props idiom) so the lightning is chunky
//! art-res pixels upscaled NEAREST, never smoothed.
//!
//! Determinism: the arc shapes RE-ROLL at [`FLICKER_HZ`] (~20 Hz), not per
//! frame — all geometry derives from `hash01(seed, salt(step, ...))` where
//! `step = floor(elapsed * FLICKER_HZ)`, so each flicker step holds stable
//! for a few frames (electric strobing, not 60 Hz shimmer) and equal
//! `(seed, step)` always yields the exact same shapes.
//!
//! The pool ([`SparkPool`]) is a fixed-capacity ring buffer owned by
//! `GameState`: spawning past capacity overwrites the oldest slot, expiry
//! frees slots, and nothing allocates per frame. The pure parts (pool
//! insert/wrap/expiry, flicker-step derivation, arc/dot geometry) compile
//! natively and are unit-tested below; only [`render_sparks`] is wasm-only.

use crate::drive::hash01;
use crate::math::Vec2;

/// Fixed capacity of the spark pool (a ring: overflow overwrites oldest).
pub const SPARK_CAP: usize = 32;
/// How long one spark burst lives, seconds.
pub const SPARK_LIFETIME: f32 = 0.20;
/// Flicker rate: the arc shapes re-roll this many times per second.
pub const FLICKER_HZ: f32 = 20.0;
/// Art-pixel size of a spark's pixel group, in WORLD units (chunky texels).
pub const SPARK_PX: f32 = 2.5;
/// Side of the square pixel group a spark draws into, world units
/// (the burst footprint: arcs reach ~`0.45 * SPARK_SIZE` from the centre).
pub const SPARK_SIZE: f32 = 40.0;
/// Most arcs a single flicker step can show (2 or 3 are rolled).
pub const MAX_ARCS: u32 = 3;
/// Most polyline points one arc can have (up to 6 segments).
pub const MAX_ARC_POINTS: usize = 7;
/// Stray spark dots rolled per flicker step (each may or may not exist).
pub const MAX_DOTS: u32 = 3;

/// One live spark burst: world position, its hash seed, and its clock.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Spark {
    pub pos: Vec2,
    pub seed: u32,
    pub spawn_time: f32,
    pub lifetime: f32,
}

/// Fixed-capacity ring-buffer pool of active sparks. No heap traffic after
/// construction: spawn writes a slot (overwriting the oldest once full),
/// expiry clears slots in place.
pub struct SparkPool {
    slots: [Option<Spark>; SPARK_CAP],
    /// Next slot to write (ring cursor).
    cursor: usize,
    /// Total sparks ever spawned — the per-spark seed source.
    counter: u32,
}

impl SparkPool {
    pub fn new() -> Self {
        SparkPool {
            slots: [None; SPARK_CAP],
            cursor: 0,
            counter: 0,
        }
    }

    /// Insert a spark at `pos`, born `now`. Overwrites the oldest slot when
    /// the ring is full (a fresh burst beats a nearly-expired one).
    pub fn spawn(&mut self, pos: Vec2, now: f32) {
        // Decorrelate consecutive seeds (hash01 mixes, but keep them far
        // apart anyway) — deterministic per spawn order.
        let seed = self.counter.wrapping_mul(0x9E37_79B9).wrapping_add(1);
        self.slots[self.cursor] = Some(Spark {
            pos,
            seed,
            spawn_time: now,
            lifetime: SPARK_LIFETIME,
        });
        self.cursor = (self.cursor + 1) % SPARK_CAP;
        self.counter = self.counter.wrapping_add(1);
    }

    /// Free every slot whose spark has run out (or was born in the future —
    /// a restarted clock drops stale sparks instead of replaying them).
    pub fn expire(&mut self, now: f32) {
        for slot in self.slots.iter_mut() {
            if let Some(s) = slot {
                let elapsed = now - s.spawn_time;
                if !(0.0..s.lifetime).contains(&elapsed) {
                    *slot = None;
                }
            }
        }
    }

    /// Drop every spark (floor load / checkpoint restore).
    pub fn clear(&mut self) {
        self.slots = [None; SPARK_CAP];
    }

    /// The live sparks, in slot order.
    pub fn iter(&self) -> impl Iterator<Item = &Spark> {
        self.slots.iter().flatten()
    }

    pub fn len(&self) -> usize {
        self.slots.iter().flatten().count()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

impl Default for SparkPool {
    fn default() -> Self {
        Self::new()
    }
}

/// Which flicker step `elapsed` seconds of life falls in: the arc shapes
/// re-roll only when this changes (~every 3 frames at 60 fps).
pub fn flicker_step(elapsed: f32) -> u32 {
    (elapsed.max(0.0) * FLICKER_HZ) as u32
}

/// Hash salt for one item of one flicker step (arc / dot index + channel).
fn salt(step: u32, item: u32, channel: u32) -> u32 {
    step.wrapping_mul(4096)
        .wrapping_add(item.wrapping_mul(64))
        .wrapping_add(channel)
}

/// How many arcs this flicker step shows (2 or 3).
pub fn arc_count(seed: u32, step: u32) -> u32 {
    2 + (hash01(seed, salt(step, 60, 0)) < 0.6) as u32
}

/// Build one jagged lightning arc for `(seed, step, arc)` at life fraction
/// `life` (0 = fresh, 1 = expired). Points are offsets from the impact
/// centre (world/group units). Fully deterministic per `(seed, step, arc)`;
/// `life` shrinks the reach and DROPS trailing segments so the burst ends
/// clean. Returns how many of `out`'s points are used (>= 2, a polyline).
pub fn arc_points(
    seed: u32,
    step: u32,
    arc: u32,
    life: f32,
    out: &mut [Vec2; MAX_ARC_POINTS],
) -> usize {
    use std::f32::consts::{PI, TAU};
    let life = life.clamp(0.0, 1.0);
    // Whole-burst rotation per step + per-arc sector spread: arcs cover the
    // circle instead of clumping, and every flicker re-aims the fan.
    let sector = TAU / MAX_ARCS as f32;
    let angle = hash01(seed, salt(step, 61, 0)) * TAU
        + arc as f32 * sector
        + hash01(seed, salt(step, arc, 1)) * sector;
    // Reach shrinks as the spark dies.
    let reach = SPARK_SIZE * (0.28 + 0.17 * hash01(seed, salt(step, arc, 2))) * (1.0 - 0.45 * life);
    // 3..=6 segments rolled; later life drops trailing ones (min 2 kept).
    let segs = 3 + (hash01(seed, salt(step, arc, 3)) * 3.999) as u32;
    let live = ((segs as f32) * (1.0 - 0.6 * life)).ceil().max(2.0) as u32;
    let live = live.min(segs);
    let (dir_x, dir_y) = (angle.cos(), angle.sin());
    out[0] = Vec2::zero();
    for i in 1..=live {
        let frac = i as f32 / segs as f32;
        // Perpendicular displacement, largest mid-arc (sin envelope keeps
        // the root anchored at the impact and the tip pointy).
        let wob = (hash01(seed, salt(step, arc, 8 + i)) - 0.5) * reach * 0.55 * (PI * frac).sin();
        out[i as usize] = Vec2::new(
            dir_x * reach * frac - dir_y * wob,
            dir_y * reach * frac + dir_x * wob,
        );
    }
    (live + 1) as usize
}

/// One stray spark dot for `(seed, step, dot)` at life fraction `life`:
/// `Some(offset)` from the impact centre, or `None` when this step rolled
/// the dot away. Dots fly outward as the spark ages.
pub fn dot_offset(seed: u32, step: u32, dot: u32, life: f32) -> Option<Vec2> {
    use std::f32::consts::TAU;
    if hash01(seed, salt(step, 32 + dot, 0)) > 0.75 {
        return None;
    }
    let angle = hash01(seed, salt(step, 32 + dot, 1)) * TAU;
    let dist = SPARK_SIZE * (0.12 + 0.22 * hash01(seed, salt(step, 32 + dot, 2)) + 0.14 * life);
    Some(Vec2::new(angle.cos() * dist, angle.sin() * dist))
}

/// Draw every live spark as a small snapped pixel-art group (world space,
/// under the camera transform — call it in the ACTORS layer of
/// `render_world`, after the robots and OUTSIDE the `?pixel=N` scenery
/// group: sparks belong to the hit bot, over it, at native placement).
#[cfg(target_arch = "wasm32")]
pub fn render_sparks(
    pool: &SparkPool,
    graphics: &crate::graphics::Graphics,
    now: f32,
    cull: &crate::camera::ViewCull,
) {
    use crate::math::Color;

    const CYAN: Color = Color::new(0.30, 0.95, 1.0, 1.0);
    const WHITE: Color = Color::new(1.0, 1.0, 1.0, 1.0);
    const PINK: Color = Color::new(1.0, 0.25, 0.75, 1.0);

    let half = SPARK_SIZE * 0.5;
    let mut pts = [Vec2::zero(); MAX_ARC_POINTS];
    for spark in pool.iter() {
        let elapsed = now - spark.spawn_time;
        if !(0.0..spark.lifetime).contains(&elapsed) {
            continue;
        }
        if !cull.visible(spark.pos.x, spark.pos.y, half) {
            continue;
        }
        let life = elapsed / spark.lifetime;
        let step = flicker_step(elapsed);
        // The very first flicker frame is the hot-pink flash; after that the
        // arcs settle into electric cyan with a white lead arc.
        let flash = step == 0;
        let centre = Vec2::new(half, half);

        // A ~16x16-texel group; the plain snapped composite (the props
        // idiom) keeps a burst rock-stable on its art grid. Group-local
        // units are the caller's (world) units; lines of thickness 1 world
        // unit get clamped up to exactly 1 texel by the pixel-art rule.
        graphics.pixel_begin(SPARK_PX, SPARK_SIZE, SPARK_SIZE);
        for arc in 0..arc_count(spark.seed, step) {
            let n = arc_points(spark.seed, step, arc, life, &mut pts);
            let color = if flash {
                if arc == 0 {
                    WHITE
                } else {
                    PINK
                }
            } else if arc == 0 {
                WHITE
            } else {
                CYAN
            };
            for i in 0..n - 1 {
                graphics.draw_line(centre + pts[i], centre + pts[i + 1], 1.0, color);
            }
        }
        // Stray spark dots: single texels thrown off the impact.
        for dot in 0..MAX_DOTS {
            if let Some(off) = dot_offset(spark.seed, step, dot, life) {
                let color = if flash { PINK } else { CYAN };
                let p = centre + off - Vec2::new(SPARK_PX * 0.5, SPARK_PX * 0.5);
                graphics.draw_rectangle(p, SPARK_PX, SPARK_PX, color);
            }
        }
        // Hot core on the young half of the life: one white texel at the
        // impact point itself.
        if life < 0.5 {
            let p = centre - Vec2::new(SPARK_PX * 0.5, SPARK_PX * 0.5);
            graphics.draw_rectangle(p, SPARK_PX, SPARK_PX, WHITE);
        }
        graphics.pixel_end(spark.pos.x - half, spark.pos.y - half);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pool_wraps_overwriting_oldest() {
        let mut pool = SparkPool::new();
        assert!(pool.is_empty());
        for i in 0..SPARK_CAP + 5 {
            pool.spawn(Vec2::new(i as f32, 0.0), 0.0);
        }
        // Capped, and the 5 oldest were overwritten by the 5 newest.
        assert_eq!(pool.len(), SPARK_CAP);
        assert!(pool.iter().all(|s| s.pos.x >= 5.0), "oldest overwritten");
        assert!(pool.iter().any(|s| s.pos.x == (SPARK_CAP + 4) as f32));
        // Seeds stay unique across the wrap.
        let mut seeds: Vec<u32> = pool.iter().map(|s| s.seed).collect();
        seeds.sort_unstable();
        seeds.dedup();
        assert_eq!(seeds.len(), SPARK_CAP);
    }

    #[test]
    fn expiry_frees_slots_and_guards_clock_jumps() {
        let mut pool = SparkPool::new();
        pool.spawn(Vec2::zero(), 1.0);
        pool.spawn(Vec2::zero(), 1.1);
        pool.expire(1.0 + SPARK_LIFETIME * 0.5);
        assert_eq!(pool.len(), 2, "mid-life sparks stay");
        pool.expire(1.0 + SPARK_LIFETIME); // first exactly out, second not
        assert_eq!(pool.len(), 1);
        pool.expire(10.0);
        assert!(pool.is_empty());
        // A spark 'born in the future' (clock restarted) is dropped.
        pool.spawn(Vec2::zero(), 5.0);
        pool.expire(4.0);
        assert!(pool.is_empty());
    }

    #[test]
    fn geometry_is_deterministic_per_seed_and_step() {
        let mut a = [Vec2::zero(); MAX_ARC_POINTS];
        let mut b = [Vec2::zero(); MAX_ARC_POINTS];
        let na = arc_points(77, 3, 1, 0.2, &mut a);
        let nb = arc_points(77, 3, 1, 0.2, &mut b);
        assert_eq!(na, nb);
        assert_eq!(a[..na], b[..nb], "same (seed, step, arc) = same shape");
        assert!((3..=MAX_ARC_POINTS).contains(&na), "3-6 segments = 4-7 pts");
        assert_eq!(a[0], Vec2::zero(), "arcs root at the impact point");
        // A different flicker step re-rolls the shape.
        let nc = arc_points(77, 4, 1, 0.2, &mut b);
        assert!(na != nc || a[..na] != b[..nc], "steps differ");
        // Dots too: deterministic, and re-rolled across steps.
        assert_eq!(dot_offset(77, 3, 0, 0.2), dot_offset(77, 3, 0, 0.2));
        // Arc counts stay in the promised band.
        for step in 0..50 {
            let n = arc_count(77, step);
            assert!((2..=MAX_ARCS).contains(&n));
        }
    }

    #[test]
    fn arcs_shrink_and_drop_segments_over_life() {
        let mut fresh = [Vec2::zero(); MAX_ARC_POINTS];
        let mut dying = [Vec2::zero(); MAX_ARC_POINTS];
        let nf = arc_points(9, 2, 0, 0.0, &mut fresh);
        let nd = arc_points(9, 2, 0, 0.95, &mut dying);
        assert!(nd <= nf, "late life never has more segments");
        let reach = |pts: &[Vec2], n: usize| {
            pts[..n]
                .iter()
                .map(|p| p.length())
                .fold(0.0f32, |m, l| m.max(l))
        };
        assert!(reach(&dying, nd) < reach(&fresh, nf), "arcs shrink");
        // Everything stays inside the pixel-group footprint.
        for step in 0..40 {
            for arc in 0..MAX_ARCS {
                let n = arc_points(9, step, arc, 0.0, &mut fresh);
                assert!(reach(&fresh, n) <= SPARK_SIZE * 0.5, "fits the group");
                for d in 0..MAX_DOTS {
                    if let Some(off) = dot_offset(9, step, d, 1.0) {
                        assert!(off.length() <= SPARK_SIZE * 0.5);
                    }
                }
            }
        }
    }

    #[test]
    fn flicker_step_re_rolls_at_20hz_not_per_frame() {
        assert_eq!(flicker_step(0.0), 0);
        assert_eq!(flicker_step(0.016), 0, "frames 1-2 share step 0");
        assert_eq!(flicker_step(0.049), 0);
        assert_eq!(flicker_step(0.051), 1);
        assert_eq!(flicker_step(0.149), 2);
        assert_eq!(flicker_step(-1.0), 0, "pre-birth clamps to step 0");
    }
}
