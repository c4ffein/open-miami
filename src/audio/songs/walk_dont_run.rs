//! TRACK 2 — "Walk Don't Run": FLOOR 0, the cold open (gate / parking lot).
//! 50% wave / 40% synthwave / 10% darksynth. G natural minor, 88 BPM,
//! 44 bars (~2:00).
//!
//! Forced calm. Act normal past the gate; your hands know what's coming
//! even if the guards don't. Half-time, an m9 bed (the pad's Gm with the
//! 9th and 11th falling over it as sine raindrops), a triangle sub that
//! GALLOPS on the offbeat 8ths — synthwave's pump slowed to a stroll — and a
//! three-note sigh of a lead that never lands. In the last full section a
//! darksynth bass pattern appears LOW and quiet under everything: a 16th
//! pump on the root with the bVII dragging it down at the end of each bar —
//! the violence idling under the politeness. Then the crowd noise again:
//! the bed alone, thinner than the opening.
//!
//! Wave checklist (docs/music/WAVE.md):
//! - [x] half-time feel with real emptiness (kick 1, snare 3, hats only on
//!   alternate bars, empty fourth bars)
//! - [x] a lush extended chord sustained long (Gm + A + C = m9 / m11; the
//!   `gaze` moves to Eb for four bars — Ebmaj7 over the G sub)
//! - [x] a repeating fragment that never resolves (`sigh`: Bb – A … C – A)
//! - [x] bass dry and close (triangle sub, no drive) while the sine
//!   raindrops and the dark pad float
//! - [x] aftermath — here, BEFORE-math: the same detachment, nothing has
//!   happened yet
//!
//! Synthwave 40%: the offbeat sub gallop. Darksynth 10%: `undertow`'s low
//! 16th pump, velocity 0.5.

use super::super::compose::*;
use super::{SongSpec, Wave, MINOR};
use std::sync::OnceLock;

/// One bar of pad on `root`, retriggered every two beats.
fn bed(root: i32) -> Lane {
    transpose(steps("0 . . . . . . . 0 . . . . . . ."), root)
}

/// 8 bars of one chord.
fn held(root: i32) -> Lane {
    repeat(bed(root), 8)
}

/// The raindrops: a 12-step cell — the 9th, the 11th, the root — phasing
/// 3-against-4 over the 16-step bars.
fn raindrops() -> Lane {
    steps("15 . . 17 . . . . 14 . . .")
}

/// The stroll: the synthwave offbeat gallop on the sub (rest-note 8ths),
/// felt more than heard.
fn stroll(root: i32) -> Lane {
    transpose(steps(". . 0 . . . 0 . . . 0 . . . 0 ."), root)
}

/// The sigh — a fragment that never resolves: Bb – A, then C – A, with a
/// bar of nothing between each.
fn sigh() -> Lane {
    steps(
        "16 . . . 15 . . . . . . . . . . . |
         . . . . . . . . . . . . . . . . |
         17 . . . 15 . . . . . 14 . . . . . |
         . . . . . . . . . . . . . . . . ",
    )
}

/// The darksynth undertow: a 16th pump on the root, resting on the kick,
/// dragged down to the bVII on the last two 16ths of every bar.
fn undertow(root: i32) -> Lane {
    transpose(steps(". 0 0 0 . 0 0 0 . 0 0 0 . 0 -1 -1"), root)
}

/// Half-time skeleton, four-bar phrase: hats on alternate bars, the fourth
/// bar empty.
fn skeleton() -> DrumLane {
    cat_hits([
        hits("k.......s......."),
        hits("k...h...s...h..."),
        hits("k.......s......."),
        hits("................"),
    ])
}

/// The undertow's kit: the same skeleton with a few 16th hats creeping in.
fn creeping_kit() -> DrumLane {
    cat_hits([
        hits("k.......s.....h."),
        hits("k...h...s...hh.."),
        hits("k.......s.....h."),
        hits("k.......s...hh.h"),
    ])
}

/// Drift: the bed and the raindrops. The lot at night.
fn drift() -> SectionSpec {
    section(
        "drift",
        [pad(held(0)).vel(0.85), arp(raindrops()).vel(0.75)],
    )
}

/// Walk: + the sub stroll and the half-time skeleton.
fn walk() -> SectionSpec {
    section(
        "walk",
        [
            bass(repeat(stroll(0), 8)),
            pad(held(0)),
            arp(raindrops()),
            drums(skeleton().repeat(2)),
        ],
    )
}

/// Gaze: the sigh on top; the bed lifts to Eb for the second half.
fn gaze() -> SectionSpec {
    section(
        "gaze",
        [
            bass(repeat(stroll(0), 8)),
            lead(repeat(sigh(), 2)).vel(0.8),
            pad(repeat(bed(0), 4).then(repeat(bed(5), 4))),
            arp(raindrops()),
            drums(skeleton().repeat(2)),
        ],
    )
}

/// Undertow: the darksynth pattern, low and quiet, under the sigh; the bed
/// alternates Gm and Cm.
fn undertow_section() -> SectionSpec {
    section(
        "undertow",
        [
            bass(repeat(undertow(0), 8)).vel(0.5),
            lead(repeat(sigh(), 2)).vel(0.7),
            pad(cat([
                repeat(bed(0), 2),
                repeat(bed(3), 2),
                repeat(bed(0), 2),
                repeat(bed(3), 2),
            ])),
            arp(raindrops()).vel(0.8),
            drums(creeping_kit().repeat(2)),
        ],
    )
}

/// Crowd: back to the bed, thinner than the drift it came from.
fn crowd() -> SectionSpec {
    section(
        "crowd",
        [
            pad(repeat(bed(0), 4)).vel(0.7),
            arp(sparsify(repeat(raindrops(), 5), 3, 0.4)).vel(0.5),
        ],
    )
}

fn build() -> SongSpec {
    song("Walk Don't Run", Key::new(G1, MINOR), 88.0)
        .waves(Wave::Triangle, Wave::Triangle, Wave::DarkPad, Wave::Sine)
        .intensity(0.55)
        .arrange([drift(), walk(), gaze(), walk(), undertow_section(), crowd()])
        .build()
}

/// The finished song, built once.
pub fn spec() -> SongSpec {
    static SPEC: OnceLock<SongSpec> = OnceLock::new();
    *SPEC.get_or_init(build)
}
