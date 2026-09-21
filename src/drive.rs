//! The DRIVE — the glitchy synthwave ride home under the credits.
//!
//! A first-person OutRun-style scene: a banded dusk sky, a big cut-band sun
//! on the horizon, a dark road converging on the vanishing point with dashed
//! centre lines rushing at the camera, and palm silhouettes streaming past
//! on both sides, rasterized chunky on an art-pixel grid. The simulation
//! TEARS: horizontal slices displace sideways, colour channels split into
//! red/cyan ghosts, palms stutter on frozen time buckets, neon debris blocks
//! flash, and a faint digital rain shimmers in the sky. Everything loops
//! forever off `t` alone — all randomness is hashed time buckets, no RNG
//! state — so the scene is deterministic and replayable frame by frame.
//!
//! RENDERING IS ONE SHADER PASS (opcode 20 = DRIVE): every pixel of the
//! scene is computed in renderer.js's drive fragment shader,
//! shadertoy-style — this module only computes the schedules below each
//! frame and emits the opcode (`Graphics::drive`). It used to be built from
//! 2D primitives through pixel-art groups; that cost several full-screen
//! blended layers per frame and drowned fill-rate-limited GPUs.
//!
//! The emission only records a `Graphics` op; it and the projection / glitch
//! schedules below are unit-tested natively.

// ---------------------------------------------------------------------------
// Tunables — the scene geometry constants are MIRRORED in renderer.js's
// DRIVE shader (DRIVE_FS + its palm-placement JS); editing one side alone
// desyncs the native tests from the picture. `BANDS` and the schedule
// functions below are the live Rust side of the split.
//
// NOTE: `HORIZON_FRAC`, `SPEED`, `STRIPE`, `DASH`, `ROAD_HALF`, `PALM_*`,
// `Z_FAR`, `PPU_FRAC` and the `project_y` / `z_at` projection are the
// NATIVE-TEST MIRROR of the picture, not what draws it: the wasm path only
// ships the schedules + `BANDS`. The live values are the literals in
// renderer.js — DRIVE_FS (`horizon = h * 0.44`, `ppu = w * 0.14`, the
// `13.0` speed / `2.4` stripe / `1.4` dash / `3.0 * ppu / z` road half
// width in the road rows) and the palm-placement JS right after it
// (`SPEED = 13.0, SPACING = 6.5, PX = 4.6, PH = 3.4, ZFAR = 36.0`). Keep
// both sides equal so the unit tests here keep describing the shader —
// PINNED: `the_js_mirror_matches` parses those literals out of both JS files.
// ---------------------------------------------------------------------------

/// Horizon height as a fraction of the screen height.
pub const HORIZON_FRAC: f32 = 0.44;
/// Camera speed, world units per second.
pub const SPEED: f32 = 13.0;
/// World length of one light/dark road stripe.
pub const STRIPE: f32 = 2.4;
/// World length of one centre-line dash cycle (dash + gap).
pub const DASH: f32 = 1.4;
/// Half the road width, world units.
pub const ROAD_HALF: f32 = 3.0;
/// Palms stand this far from the road centre, world units.
pub const PALM_X: f32 = 4.6;
/// World spacing between consecutive palms on one side.
pub const PALM_SPACING: f32 = 6.5;
/// Palm height, world units.
pub const PALM_H: f32 = 3.4;
/// The far clip: everything fogs out toward the horizon by here.
pub const Z_FAR: f32 = 36.0;
/// Pixels per world unit at z = 1, as a fraction of the screen width.
pub const PPU_FRAC: f32 = 0.14;
/// The screen is cut into this many horizontal tear slices.
pub const BANDS: usize = 9;

// ---------------------------------------------------------------------------
// Deterministic helpers (native-testable)
// ---------------------------------------------------------------------------

/// Stateless hash -> 0..1. All the scene's randomness derives from hashed
/// (bucket, salt) pairs, so equal `t` always renders the exact same frame.
pub fn hash01(a: u32, b: u32) -> f32 {
    let mut x = a
        .wrapping_mul(HASH_MUL_A)
        .wrapping_add(b.wrapping_mul(HASH_MUL_B));
    x = (x ^ (x >> 13)).wrapping_mul(HASH_MUL_X);
    ((x ^ (x >> 16)) & 0xff_ffff) as f32 / 0xff_ffff as f32
}

/// `hash01`'s multipliers — `driveHash` in web/renderer/backgrounds.js is the same
/// function (it places the palms and the debris off the same buckets);
/// pinned by `the_js_mirror_matches`.
const HASH_MUL_A: u32 = 374_761_393;
const HASH_MUL_B: u32 = 668_265_263;
const HASH_MUL_X: u32 = 1_274_126_177;

/// Screen y of world depth `z` (`z` >= 1; z = 1 is the bottom edge).
pub fn project_y(horizon: f32, h: f32, z: f32) -> f32 {
    horizon + (h - horizon) / z
}

/// World depth at screen row `y` (`y` strictly below the horizon).
pub fn z_at(horizon: f32, h: f32, y: f32) -> f32 {
    (h - horizon) / (y - horizon)
}

/// Sideways displacement of tear slice `band`, as a fraction of the screen
/// width: 0 almost always, bursts on ~90 ms buckets, plus rare violent
/// tears on ~50 ms buckets. (The violent tears were single-frame ~16 ms
/// buckets when the game ran at 20 fps — each shown frame then held one for
/// ~50 ms; this keeps that read at 60 fps.) Zero everywhere at `glitch` = 0.
pub fn band_offset_frac(t: f32, band: u32, glitch: f32) -> f32 {
    if glitch <= 0.0 {
        return 0.0;
    }
    let mut dx = 0.0;
    let b = (t / 0.09).floor() as u32;
    if hash01(b, 700 + band) < glitch * 0.22 {
        dx += (hash01(b, 900 + band) - 0.5) * 0.09 * (0.4 + glitch);
    }
    let f = (t / 0.05).floor() as u32;
    if hash01(f, 1300 + band) < glitch * 0.05 {
        dx += (hash01(f, 1500 + band) - 0.5) * 0.30;
    }
    dx
}

/// Red/cyan channel-split offset in fractions of the screen width (signed);
/// 0 outside bursts. Bursts live on ~110 ms buckets.
pub fn channel_split_frac(t: f32, glitch: f32) -> f32 {
    if glitch <= 0.0 {
        return 0.0;
    }
    let b = (t / 0.11).floor() as u32;
    if hash01(b, 41) < glitch * 0.30 {
        let sign = if hash01(b, 43) < 0.5 { -1.0 } else { 1.0 };
        sign * (0.004 + 0.010 * hash01(b, 47) * glitch)
    } else {
        0.0
    }
}

/// The credits' glitch ramp: the simulation stabilizes as CL4-UD3 gets away —
/// 0.8 at the start of the roll down to 0.15 after 20 s, then steady.
pub fn ending_glitch(credits_time: f32) -> f32 {
    0.8 - 0.65 * (credits_time / 20.0).clamp(0.0, 1.0)
}

// ---------------------------------------------------------------------------
// Drawing
// ---------------------------------------------------------------------------

mod draw {
    use super::*;
    use crate::graphics::Graphics;

    /// Render the drive into the rect `(0, 0)..(w, h)` of the CURRENT
    /// transform (translate first to draw it elsewhere, e.g. a preview
    /// card). `t` is the loop clock in seconds, `glitch` 0..1 the tear
    /// intensity, `dim` 0..1 darkens the finished scene toward the menu's
    /// near-black inside the shader (the title passes 0.55; a preview 0).
    ///
    /// Since the shadertoy port this is ONE opcode: Rust computes the
    /// unit-tested glitch schedules (band tears, channel split) and the
    /// art-pixel size, and renderer.js's DRIVE fragment shader computes
    /// every pixel of the scene from them in a single opaque pass — no
    /// pixel groups, no stacked full-screen fills, so the cost on
    /// fill-rate-starved GPUs is one shaded write per pixel. The scene
    /// geometry (sky bands, cut-band sun, road rows, palms, debris) is
    /// mirrored from the tunables above into that shader.
    pub fn render_drive(g: &Graphics, w: f32, h: f32, t: f32, glitch: f32, dim: f32) {
        // Chunky art pixels, matched to the old pixel-group look.
        let px = 3.0f32.max((w / 1000.0).ceil()).max((h / 1000.0).ceil());
        let split = channel_split_frac(t, glitch) * w;
        let mut offs = [0.0f32; BANDS];
        for (i, o) in offs.iter_mut().enumerate() {
            *o = (band_offset_frac(t, i as u32, glitch) * w).clamp(-0.12 * w, 0.12 * w);
        }
        g.drive(w, h, t, glitch, split, px, dim, &offs);
    }
}

pub use draw::render_drive;

#[cfg(test)]
mod tests {
    use super::*;

    /// The number that follows `prefix` in `src` (the prefix must be unique:
    /// a second match means the pin no longer knows which literal it reads).
    fn num_after(src: &str, prefix: &str) -> f32 {
        assert_eq!(
            src.matches(prefix).count(),
            1,
            "`{prefix}` must appear exactly once in the JS mirror"
        );
        let rest = &src[src.find(prefix).unwrap() + prefix.len()..];
        let end = rest
            .find(|c: char| !(c.is_ascii_digit() || c == '.'))
            .unwrap_or(rest.len());
        rest[..end]
            .parse()
            .unwrap_or_else(|_| panic!("no number after `{prefix}`: `{}`", &rest[..end.min(12)]))
    }

    /// `src` between `start` and the next `end`.
    fn section<'a>(src: &'a str, start: &str, end: &str) -> &'a str {
        let a = src
            .find(start)
            .unwrap_or_else(|| panic!("`{start}` not found"));
        let len = src[a..]
            .find(end)
            .unwrap_or_else(|| panic!("`{end}` not found"));
        &src[a..a + len]
    }

    /// The scene geometry exists three times: the constants above (what the
    /// native tests describe), DRIVE_FS (what draws the sky + road) and
    /// `drawDrive` in web/renderer/backgrounds.js (what places the palms). Editing one alone
    /// desyncs the picture from its tests — or the palms from the road.
    #[test]
    fn the_js_mirror_matches() {
        let shaders = include_str!("../web/renderer/shaders.js");
        let fs = section(shaders, "export const DRIVE_FS = `", "`;");
        let scene: [(&str, f32); 5] = [
            ("float horizon = h * ", HORIZON_FRAC),
            ("float ppu = w * ", PPU_FRAC),
            ("uniform float uOffs[", BANDS as f32),
            ("float bandH = uSize.y / ", BANDS as f32),
            (
                "float band = clamp(floor(p.y / bandH), 0.0, ",
                (BANDS - 1) as f32,
            ),
        ];
        for (prefix, want) in scene {
            assert_eq!(num_after(fs, prefix), want, "DRIVE_FS: `{prefix}`");
        }
        // The road rows (the sun above them has a `halfW` of its own).
        let road = section(fs, "// Ground + road", "// Horizon glow line");
        let rows: [(&str, f32); 5] = [
            ("float pd = z + uT * ", SPEED),
            ("bool alt = mod(floor(pd / ", STRIPE),
            ("} else if (mod(floor(pd / ", DASH),
            ("float halfW = ", ROAD_HALF),
            ("float fog = pow(clamp(z / ", Z_FAR),
        ];
        for (prefix, want) in rows {
            assert_eq!(
                num_after(road, prefix),
                want,
                "DRIVE_FS road rows: `{prefix}`"
            );
        }
        // The band loop that picks this slice's offset.
        let tear = section(fs, "float dx = 0.0;", "p.x -= dx;");
        assert_eq!(num_after(tear, "for (int i = 0; i < "), BANDS as f32);

        let renderer = include_str!("../web/renderer/backgrounds.js");
        let js = section(renderer, "function drawDrive(", "// PASS 1");
        let palms: [(&str, f32); 7] = [
            ("const horizon = h * ", HORIZON_FRAC),
            (", ppu = w * ", PPU_FRAC),
            ("const SPEED = ", SPEED),
            (", SPACING = ", PALM_SPACING),
            (", PX = ", PALM_X),
            (", PH = ", PALM_H),
            (", ZFAR = ", Z_FAR),
        ];
        for (prefix, want) in palms {
            assert_eq!(num_after(js, prefix), want, "drawDrive: `{prefix}`");
        }

        // `driveHash` == `hash01`: same multipliers, same shifts, same mask.
        let hash = section(renderer, "function driveHash(a, b) {", "\n  }");
        for needle in [
            format!("Math.imul(a >>> 0, {HASH_MUL_A})"),
            format!("Math.imul(b >>> 0, {HASH_MUL_B})"),
            format!("Math.imul(x ^ (x >>> 13), {HASH_MUL_X})"),
            "((x ^ (x >>> 16)) & 0xffffff) / 0xffffff".to_string(),
        ] {
            assert!(hash.contains(&needle), "driveHash lost `{needle}`");
        }

        // The op carries one offset per band after its 7 scalars.
        assert_eq!(crate::graphics::stream::OP_ARGS[20], 7 + BANDS);
    }

    #[test]
    fn projection_round_trips_and_is_ordered() {
        let (h, horizon) = (720.0, 720.0 * HORIZON_FRAC);
        // z = 1 is the bottom edge; deeper is higher on screen.
        assert!((project_y(horizon, h, 1.0) - h).abs() < 1e-3);
        let mut last = h + 1.0;
        for i in 1..40 {
            let z = i as f32;
            let y = project_y(horizon, h, z);
            assert!(y < last, "further must be higher on screen");
            assert!(y > horizon, "never above the horizon");
            assert!((z_at(horizon, h, y) - z).abs() < 1e-3, "round trip");
            last = y;
        }
    }

    #[test]
    fn glitch_schedules_are_deterministic_and_quiet_at_zero() {
        for i in 0..200 {
            let t = i as f32 * 0.037;
            for band in 0..BANDS as u32 {
                assert_eq!(band_offset_frac(t, band, 0.0), 0.0);
                // Same time, same band, same intensity -> the same frame.
                assert_eq!(
                    band_offset_frac(t, band, 0.7),
                    band_offset_frac(t, band, 0.7)
                );
                assert!(band_offset_frac(t, band, 1.0).abs() < 0.25);
            }
            assert_eq!(channel_split_frac(t, 0.0), 0.0);
            assert_eq!(channel_split_frac(t, 0.6), channel_split_frac(t, 0.6));
            assert!(channel_split_frac(t, 1.0).abs() < 0.02);
        }
    }

    #[test]
    fn glitches_actually_fire_at_high_intensity() {
        let mut tears = 0;
        let mut splits = 0;
        for i in 0..2000 {
            let t = i as f32 * 0.016;
            if (0..BANDS as u32).any(|b| band_offset_frac(t, b, 0.8) != 0.0) {
                tears += 1;
            }
            if channel_split_frac(t, 0.8) != 0.0 {
                splits += 1;
            }
        }
        assert!(tears > 50, "tears fire regularly: {tears}");
        assert!(splits > 20, "channel splits fire: {splits}");
    }

    #[test]
    fn ending_glitch_ramps_down_then_settles() {
        assert!((ending_glitch(0.0) - 0.8).abs() < 1e-6);
        assert!(ending_glitch(10.0) < ending_glitch(5.0));
        assert!((ending_glitch(20.0) - 0.15).abs() < 1e-6);
        assert!((ending_glitch(500.0) - 0.15).abs() < 1e-6);
    }

    #[test]
    fn hash01_stays_in_range() {
        for a in 0..300 {
            for b in 0..30 {
                let v = hash01(a, b);
                assert!((0.0..=1.0).contains(&v));
            }
        }
    }
}
