//! TRACK 4 — "Thermal Mass": FLOORS 5–8. 95% darksynth / 5% witch house.
//! B Phrygian, 100 BPM, 88 bars (~3:31).
//!
//! The tower pushing back. Heavier, slower, meaner — the fights stop being
//! free. Weight comes from the low end, not speed: a two-bar 8th-note riff
//! on the driven bass, sparse enough to be heavy, with the b2 (C) and the
//! bVII below the root (A) as its menace notes. Half-time sections dominate
//! (`procession` / `slab` / `crush`), the four-on-the-floor `drive` is the
//! alternation, and every section with a kick is ducked — the deepest pump
//! of the soundtrack. The supersaw only STABS (root, then the b2 a bar
//! later). The witch-house 5% is `drone`: the pad refusing to move while the
//! lead buzzes the b9 (C) on every 16th at a whisper — the first hint of the
//! CORRUPTOR under the breakdown. Ends cold on the procession.
//!
//! Darksynth checklist (docs/music/DARKSYNTH.md):
//! - [x] the bass riff carries 8 bars alone (`procession`, with just the
//!   half-time kick and snare)
//! - [x] the b2 appears (the riff's C, the bII pad, the b2 stab, the b9 drone)
//! - [x] everything pumps on the kick (every kicked section is ducked)
//! - [x] drive and half-time alternate (`drive` between the crushes)
//! - [x] the refrain (`crush`) returns hotter each pass: Cool = the stabs,
//!   Warm = + 8th hats + the gated arp, Hot = + stabs an octave up + double
//!   snares
//! - [x] it slaps with the lead muted (`slab`)

use super::super::compose::*;
use super::{SongSpec, Wave, PHRYGIAN};
use std::sync::OnceLock;
use Intensity::{Cool, Hot, Warm};

/// THE RIFF, two bars of heavy 8ths: root, root, the C rub; then the climb
/// to E and the drop to the A below.
fn riff() -> Lane {
    steps(
        "0 . . . 0 . 0 . . . 0 . 1 . 0 . |
         0 . . . 0 . 0 . . . 3 . 1 . -1 . ",
    )
}

/// 8 bars of the riff.
fn riff_8() -> Lane {
    repeat(riff(), 4)
}

/// One bar of pad, retriggered every beat.
fn bed(root: i32) -> Lane {
    transpose(steps("0 . . . 0 . . . 0 . . . 0 . . ."), root)
}

/// 8 bars of the pad: i i i bII, twice.
fn chords() -> Lane {
    repeat(cat([bed(0), bed(0), bed(0), bed(1)]), 2)
}

/// The stabs: the root on bar 1, the b2 on bar 2, an answer on bar 4.
fn stabs() -> Lane {
    steps(
        "14 . . . . . . . . . . . . . . . |
         15 . . . . . . . . . . . . . . . |
         . . . . 14 . . . . . 12 . . . . . |
         15 . . . . . . . 14 . . . . . . . ",
    )
}

/// The gated arp, one bar: B, D, C — a minor third and the b2, gated.
fn gate() -> Lane {
    steps("14 . 14 . 16 . 15 . 14 . 14 . 16 . 15 .")
}

/// Half-time kit, one bar: kick on 1, snare on 3, one hat.
fn half_bar() -> DrumLane {
    hits("k.......s.....h.")
}

/// The half-time kit's phrase-ending bar: the last-16th snare doubles.
fn half_fill() -> DrumLane {
    hits("k.......s.....ss")
}

/// 8 bars of the half-time kit; `hot` adds 8th hats and a double snare.
fn half_kit(heat: Intensity) -> DrumLane {
    let bar = match heat {
        Cool => half_bar(),
        Warm => hits("k.h.h.h.s.h.h.h."),
        Hot => hits("k.h.h.h.s.h.h.s."),
    };
    cat_hits([
        bar.clone().repeat(3),
        half_fill(),
        bar.repeat(3),
        half_fill(),
    ])
}

/// 8 bars of the drive kit: four-on-the-floor, the snare on 2 and 4.
fn drive_kit() -> DrumLane {
    cat_hits([hits("k.h.k.s.k.h.k.s.").repeat(7), hits("k.h.k.s.k.s.ksss")])
}

/// Procession: the riff and the half-time kick / snare. Nothing else.
fn procession() -> SectionSpec {
    section(
        "procession",
        [
            bass(riff_8()),
            drums(cat_hits([
                hits("k.......s.......").repeat(7),
                hits("k.......s.....ss"),
            ])),
        ],
    )
    .ducked()
}

/// Slab: + the pad and the hat. The lead-muted refrain.
fn slab() -> SectionSpec {
    section(
        "slab",
        [
            bass(riff_8()),
            pad(chords()).vel(0.8),
            drums(half_kit(Cool)),
        ],
    )
    .ducked()
}

/// Crush: the slab with the supersaw stabs — the refrain, half-time,
/// hotter each return.
fn crush(heat: Intensity) -> SectionSpec {
    let arp_lane = match heat {
        Cool => Lane::default(),
        Warm | Hot => repeat(gate(), 8),
    };
    let top = match heat {
        Hot => transpose(repeat(stabs(), 2), 7),
        _ => Lane::default(),
    };
    section(
        "crush",
        [
            bass(riff_8()),
            lead(repeat(stabs(), 2)),
            lead(top),
            pad(chords()),
            arp(arp_lane).vel(0.8),
            drums(half_kit(heat)),
        ],
    )
    .ducked()
}

/// Drive: the same riff under four-on-the-floor and the gated arp — the
/// alternation.
fn drive() -> SectionSpec {
    section(
        "drive",
        [
            bass(riff_8()),
            pad(chords()).vel(0.8),
            arp(repeat(gate(), 8)),
            drums(drive_kit()),
        ],
    )
    .ducked()
}

/// Drone: the breakdown. The pad refuses to move, the bass holds one note a
/// bar, and the lead buzzes the b9 on every 16th at a whisper — the
/// corruption under the floor. One kick every two bars.
fn drone() -> SectionSpec {
    section(
        "drone",
        [
            bass(repeat(steps("0 . . . . . . . . . . . . . . ."), 8)).vel(0.8),
            lead(repeat(steps("8"), 128)).vel(0.35),
            pad(repeat(bed(0), 8)),
            drums(hits("k...............................").repeat(4)),
        ],
    )
    .ducked()
}

fn build() -> SongSpec {
    song("Thermal Mass", Key::new(B1, PHRYGIAN), 100.0)
        .waves(
            Wave::DrivenBass,
            Wave::Supersaw,
            Wave::DarkPad,
            Wave::Square,
        )
        .intensity(1.05)
        .arrange([
            procession(),
            slab(),
            crush(Cool),
            drive(),
            crush(Warm),
            drone(),
            slab(),
            crush(Hot),
            drive(),
            crush(Hot),
            procession(),
        ])
        .build()
}

/// The finished song, built once.
pub fn spec() -> SongSpec {
    static SPEC: OnceLock<SongSpec> = OnceLock::new();
    *SPEC.get_or_init(build)
}
