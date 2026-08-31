//! The game's Hotline-Miami-inspired colour palette — the single home of the
//! world's surface / wall tones and the hot neon accents.
//!
//! Design rules (see CLAUDE.md ## Design):
//! - floors stay MID-TO-DARK value so the baked actor sprites and the white
//!   HUD always read on top of them;
//! - saturation lives only in SMALL accents (a carpet mark, a paint remnant,
//!   a brass inlay) — never in a full surface;
//! - every tone is opaque and pre-mixed (the world renders through the static
//!   geometry cache: solid primitives, no blending-order surprises).
//!
//! `src/level.rs` (surfaces) and `src/render.rs` (walls) must take every
//! world colour from here — no ad-hoc RGB in those paths.

use crate::math::Color;

// ---------------------------------------------------------------- neon accents
// The hot trims. Used sparingly and usually through a muted sibling below —
// full-strength neon on the floor would fight the actors.
pub const NEON_PINK: Color = Color::rgb(1.0, 0.18, 0.60);
pub const NEON_CYAN: Color = Color::rgb(0.15, 0.90, 0.95);
pub const ACID_GREEN: Color = Color::rgb(0.55, 0.95, 0.25);
pub const SODIUM_ORANGE: Color = Color::rgb(0.98, 0.60, 0.16);

// --------------------------------------------------------------------- walls
/// Wall slab fill — the historical dark mauve, unchanged on purpose.
pub const WALL_FILL: Color = Color::rgb(80.0 / 255.0, 60.0 / 255.0, 70.0 / 255.0);
/// Wall border / top-edge highlight.
pub const WALL_EDGE: Color = Color::rgb(100.0 / 255.0, 80.0 / 255.0, 90.0 / 255.0);
/// The dark strip walls seat into (drawn under/around every wall rect).
pub const WALL_BASEBOARD: Color = Color::rgb(0.055, 0.045, 0.065);

// --------------------------------------------------- checker (interior carpet)
// The server-floor carpet: two-tone purple squares with a woven border ring
// inside each tile, an occasional hot mark and an occasional stain.
pub const CHECKER_A: Color = Color::rgb(40.0 / 255.0, 35.0 / 255.0, 45.0 / 255.0);
pub const CHECKER_B: Color = Color::rgb(35.0 / 255.0, 30.0 / 255.0, 40.0 / 255.0);
/// Border-ring tone on the lighter (A) tiles — a shade darker than the base.
pub const CHECKER_RING_DARK: Color = Color::rgb(0.110, 0.094, 0.133);
/// Border-ring tone on the darker (B) tiles — a shade lighter than the base.
pub const CHECKER_RING_LIGHT: Color = Color::rgb(0.196, 0.160, 0.216);
/// Muted hot-pink carpet mark (rare hash-picked tiles).
pub const CHECKER_ACCENT: Color = Color::rgb(0.46, 0.13, 0.30);
/// A dark stain soaked into the pile (rare hash-picked tiles).
pub const CHECKER_STAIN: Color = Color::rgb(0.098, 0.084, 0.114);

// ---------------------------------------------------------- marble (the lobby)
pub const MARBLE_A: Color = Color::rgb(0.385, 0.37, 0.40);
pub const MARBLE_B: Color = Color::rgb(0.34, 0.325, 0.355);
/// Grout between slabs.
pub const MARBLE_SEAM: Color = Color::rgb(0.235, 0.22, 0.25);
pub const MARBLE_VEIN_LIGHT: Color = Color::rgb(0.445, 0.43, 0.455);
pub const MARBLE_VEIN_DARK: Color = Color::rgb(0.285, 0.27, 0.30);
/// Rare brass floor inlay — the lobby's muted sodium glint.
pub const MARBLE_INLAY: Color = Color::rgb(0.56, 0.43, 0.20);

// ------------------------------------------------------------------- concrete
pub const CONCRETE_A: Color = Color::rgb(0.272, 0.270, 0.284);
pub const CONCRETE_B: Color = Color::rgb(0.292, 0.288, 0.302);
pub const CONCRETE_C: Color = Color::rgb(0.254, 0.252, 0.268);
/// Expansion joints (every 4 tiles).
pub const CONCRETE_JOINT: Color = Color::rgb(0.17, 0.168, 0.182);
/// Broad damp / oil stain.
pub const CONCRETE_STAIN: Color = Color::rgb(0.213, 0.209, 0.226);
/// Hairline crack (drawn as thin L-shaped rects).
pub const CONCRETE_CRACK: Color = Color::rgb(0.188, 0.184, 0.20);
/// Faded acid-green safety paint remnant.
pub const CONCRETE_MARK: Color = Color::rgb(0.30, 0.40, 0.17);

// ------------------------------------------------- asphalt (floor 0, outdoor)
pub const ASPHALT_DARK: Color = Color::rgb(0.066, 0.070, 0.086);
pub const ASPHALT_MID: Color = Color::rgb(0.082, 0.086, 0.102);
pub const ASPHALT_LIGHT: Color = Color::rgb(0.102, 0.106, 0.124);
/// Fresh-tar repair patch (darker than any base tone).
pub const ASPHALT_PATCH: Color = Color::rgb(0.044, 0.047, 0.060);
/// Faded sodium-yellow lane paint remnant.
pub const ASPHALT_PAINT: Color = Color::rgb(0.50, 0.42, 0.17);
/// Faded white lane paint remnant.
pub const ASPHALT_PAINT_WHITE: Color = Color::rgb(0.40, 0.41, 0.45);
pub const ASPHALT_CRACK: Color = Color::rgb(0.038, 0.041, 0.054);

// -------------------------------------------------------------------- grating
/// The void under the mesh.
pub const GRATING_VOID: Color = Color::rgb(0.055, 0.05, 0.075);
pub const GRATING_BAR: Color = Color::rgb(0.16, 0.15, 0.20);
/// Lit leading edges of a deck module (top / left bars).
pub const GRATING_BAR_LIT: Color = Color::rgb(0.215, 0.20, 0.265);
/// Occasional solid deck plate over the mesh.
pub const GRATING_PLATE: Color = Color::rgb(0.128, 0.118, 0.158);
pub const GRATING_RIVET: Color = Color::rgb(0.205, 0.192, 0.245);

#[cfg(test)]
mod tests {
    use super::*;

    fn value(c: Color) -> f32 {
        c.r.max(c.g).max(c.b)
    }

    #[test]
    fn floor_bases_stay_mid_to_dark() {
        // Actor / HUD readability: no surface base tone may approach white.
        for c in [
            CHECKER_A,
            CHECKER_B,
            MARBLE_A,
            MARBLE_B,
            CONCRETE_A,
            CONCRETE_B,
            CONCRETE_C,
            ASPHALT_DARK,
            ASPHALT_MID,
            ASPHALT_LIGHT,
            GRATING_VOID,
            GRATING_PLATE,
            WALL_FILL,
        ] {
            assert!(value(c) < 0.5, "base tone too bright: {c:?}");
        }
    }

    #[test]
    fn baseboard_darker_than_every_wall_and_floor_tone() {
        assert!(value(WALL_BASEBOARD) < value(WALL_FILL));
        assert!(value(WALL_BASEBOARD) < value(CHECKER_B));
        assert!(value(WALL_BASEBOARD) < value(MARBLE_SEAM));
    }
}
