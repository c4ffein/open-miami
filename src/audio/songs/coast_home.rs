//! TRACK 7 — "Coast Home": the ENDING RIDE (the credits' elevator home).
//! 60% wave / 40% synthwave. A natural minor, 88 BPM, 56 bars (~2:33).
//!
//! It's over. Grief and relief in the same breath; the warp trails carry
//! you out. Half-time, a dry sub-thump bass close to the ear while
//! everything else floats: a lush m11 bed (the pad's Am triad with the 9th
//! and 11th dripping over it from the raindrop layer), a 12-step raindrop
//! motif that phases 3-against-4 across the bar, and the title track's lead
//! motif QUOTED ONCE — at half speed and an octave down, the night
//! remembering itself. The arc is wave's: emerge from near-silence, thicken,
//! then strip back BELOW where it started — the last section is the pad
//! alone, thinning into the credits' silence.
//!
//! Wave checklist (docs/music/WAVE.md):
//! - [x] half-time feel with real emptiness (kick on 1, snare on 3, whole
//!   bars of nothing)
//! - [x] a lush extended chord sustained long (Am with B and D — m9 / m11 —
//!   from the raindrops, two bars per pad chord)
//! - [x] a repeating fragment that never resolves (`raindrops`: B – D – A,
//!   the 9th and 11th, 12 steps against a 16-step bar)
//! - [x] bass dry and close while everything floats: the sub thump is a
//!   triangle with no drive; pads + sine raindrops carry the space
//! - [x] aftermath: emerge → swell → remember → thin → gone
//!
//! Synthwave 40%: the home progression's bVI / bVII warmth (Am Am F G) and
//! the quoted title motif.

use super::super::compose::*;
use super::neon_checksum;
use super::{SongSpec, Wave, MINOR};
use std::sync::OnceLock;

/// The slow harmonic rhythm: two bars per chord — Am Am F G over 8 bars.
const CHORDS: [i32; 4] = [0, 0, 5, 6];

/// One bar of pad on `root`, retriggered every two beats (wave's slowest
/// bed — the slow attacks just about touch).
fn bed(root: i32) -> Lane {
    transpose(steps("0 . . . . . . . 0 . . . . . . ."), root)
}

/// 8 bars of the bed, each chord held for two bars.
fn chords() -> Lane {
    cat(CHORDS.map(|r| repeat(bed(r), 2)))
}

/// The raindrop layer: a 12-step (three-beat) sigh — the 9th, the 11th,
/// the root — looping inside 16-step bars so it drifts across the barline
/// and realigns every three bars. Never resolves: it ends on the 9th.
fn raindrops() -> Lane {
    steps("17 . . . 14 . . . . . 15 .")
}

/// The dry sub thump: beat 1 and the "and" of 3, following the chord roots
/// down in the sub octave.
fn thump(root: i32) -> Lane {
    transpose(steps("0 . . . . . . . . . 0 . . . . ."), root)
}

/// 8 bars of the sub, two bars per chord.
fn sub_line() -> Lane {
    cat(CHORDS.map(|r| repeat(thump(r), 2)))
}

/// The half-time skeleton: kick on 1, snare on 3, a lazy hat every other
/// bar, and every fourth bar left empty.
fn skeleton() -> DrumLane {
    cat_hits([
        hits("k.......s......."),
        hits("k.....h.s......."),
        hits("k.......s......."),
        hits("................"),
    ])
    .repeat(2)
}

/// The title track's motif quoted ONCE: its first four bars at half speed
/// (eight bars) and an octave down.
fn remembered_motif() -> Lane {
    let call = Lane::from(neon_checksum::motif().0[..64].to_vec());
    transpose(stretch(call, 2), -7)
}

/// Emerge: the bed and the raindrops out of nothing.
fn emerge() -> SectionSpec {
    section(
        "emerge",
        [pad(chords()).vel(0.8), arp(raindrops()).vel(0.7)],
    )
}

/// Swell: + the sub and the half-time skeleton.
fn swell() -> SectionSpec {
    section(
        "swell",
        [
            bass(sub_line()),
            pad(chords()),
            arp(raindrops()),
            drums(skeleton()),
        ],
    )
}

/// Remember: the title motif, slower and lower, over the full bed.
fn remember() -> SectionSpec {
    section(
        "remember",
        [
            bass(sub_line()),
            lead(remembered_motif()).vel(0.8),
            pad(chords()),
            arp(raindrops()).vel(0.6),
            drums(skeleton()),
        ],
    )
}

/// Drift: the drums fall away, the raindrops thin.
fn drift() -> SectionSpec {
    section(
        "drift",
        [
            bass(sub_line()).vel(0.8),
            pad(chords()),
            arp(sparsify(repeat(raindrops(), 10), 5, 0.3)).vel(0.7),
        ],
    )
}

/// Thin: the bed and a few last raindrops, quieter than the start.
fn thin() -> SectionSpec {
    section(
        "thin",
        [
            pad(chords()).vel(0.7),
            arp(sparsify(repeat(raindrops(), 10), 9, 0.6)).vel(0.5),
        ],
    )
}

/// Gone: the pad alone, one bloom every two beats, emptier than the
/// beginning — into the credits' silence.
fn gone() -> SectionSpec {
    section("gone", [pad(chords()).vel(0.5)])
}

fn build() -> SongSpec {
    song("Coast Home", Key::new(A1, MINOR), 88.0)
        .waves(Wave::Triangle, Wave::Triangle, Wave::DarkPad, Wave::Sine)
        .intensity(0.5)
        .arrange([
            emerge(),
            swell(),
            remember(),
            drift(),
            swell(),
            thin(),
            gone(),
        ])
        .build()
}

/// The finished song, built once.
pub fn spec() -> SongSpec {
    static SPEC: OnceLock<SongSpec> = OnceLock::new();
    *SPEC.get_or_init(build)
}
