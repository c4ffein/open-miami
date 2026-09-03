//! TRACK 1 — "Neon Checksum": the TITLE SCREEN. 80% synthwave / 20%
//! darksynth. A natural minor, 100 BPM, 64 bars (~2:33), loops clean.
//!
//! The promise of the night: you're in the car, the tower glows ahead,
//! nothing has gone wrong yet. Home progression i – bVI – III – bVII
//! (Am – F – C – G, the sunset chords), an offbeat 8th bass pump kept OUT of
//! the lowest octave so the engine idle under the title can breathe, a
//! gentle 16th arp as the motion, and a singable lead that only fully
//! arrives on the second refrain. The bridge moves to bVI as a temporary
//! home (the genre's lift), then the refrain returns hotter — with the lead
//! doubled an octave up and the fill rolls in. The outro strips back to
//! pad and arp so the loop point lands on a breath. The darksynth 20% is the
//! sidechain pump armed under the refrains only.
//!
//! Synthwave checklist (docs/music/SYNTHWAVE.md):
//! - [x] home progression uses bVI / bVII warmth (F and G under an Am home)
//! - [x] offbeat bass pump present (`pump` — rest-note 8ths, all sections
//!   with drums)
//! - [x] a singable lead phrase that returns (`motif` — half of it on the
//!   first refrain, all of it on the second and third)
//! - [x] a bridge (bVI as the temporary home) before the last refrain
//! - [x] does it glide: one chord per bar, constant 16th arp, soft kick,
//!   snare rolls telegraph every section change
//!
//! `motif` is also QUOTED by the ending ("Coast Home") — slower and lower.

use super::super::compose::*;
use super::{SongSpec, Wave, MINOR};
use std::sync::OnceLock;
use Intensity::{Cool, Hot, Warm};

/// The home progression as pad roots, one chord per bar: Am F C G.
const HOME: [i32; 4] = [0, 5, 2, 6];

/// The bridge's temporary home: F F C G (bVI held for two bars).
const LIFT: [i32; 4] = [5, 5, 2, 6];

/// One bar of pad: the triad retriggered every beat so the slow attacks
/// overlap into one continuous bed.
fn bed(root: i32) -> Lane {
    transpose(steps("0 . . . 0 . . . 0 . . . 0 . . ."), root)
}

/// The chord bed over a 4-bar progression.
fn chords(roots: [i32; 4]) -> Lane {
    cat(roots.map(bed))
}

/// One bar of the offbeat 8th bass pump (rest-note, rest-note): the bass
/// alternates with the kick. Voiced an octave above the tonic (degree 7 =
/// A2) so the sub octave stays empty for the title screen's engine idle.
fn pump(root: i32) -> Lane {
    transpose(steps(". 7 . 7 . 7 . 7 . 7 . 7 . 7 . 7"), root)
}

/// The bass line over a 4-bar progression, with a walk-up into the next
/// cycle on the last two 8ths of bar 4.
fn bass_line(roots: [i32; 4]) -> Lane {
    let [a, b, c, d] = roots;
    cat([pump(a), pump(b), pump(c)]).then(transpose(steps(". 7 . 7 . 7 . 7 . 7 . 7 . 8 . 9"), d))
}

/// One bar of the gentle 16th arp: the chord tones `[a, b, c, d]` cycled
/// up and back (the engine of the genre — constant, never showy).
fn arp_bar(tones: [i32; 4]) -> Lane {
    let [a, b, c, d] = tones;
    repeat(Lane::from(vec![a, b, c, d, c, b, a, c]), 2)
}

/// The 4-bar arp over the home progression: Am / F / C / G chord tones
/// two octaves up.
fn home_arp() -> Lane {
    cat([
        arp_bar([14, 16, 18, 21]),
        arp_bar([12, 14, 16, 19]),
        arp_bar([16, 18, 20, 23]),
        arp_bar([13, 15, 17, 20]),
    ])
}

/// The bridge arp: F / F / C / G.
fn lift_arp() -> Lane {
    cat([
        arp_bar([12, 14, 16, 19]),
        arp_bar([12, 14, 16, 19]),
        arp_bar([16, 18, 20, 23]),
        arp_bar([13, 15, 17, 20]),
    ])
}

/// THE MOTIF — the 8-bar singable lead over Am F C G, two bars per chord:
/// a call on the chord's downbeat, an answer that hangs. Quoted by the
/// ending.
pub fn motif() -> Lane {
    steps(
        "14 . . . 16 . 17 . . . 16 . 14 . . . |
         . . 12 . 14 . . . . . . . . . . . |
         12 . . . 14 . 16 . . . 14 . 12 . . . |
         . . 9 . 12 . . . . . . . . . . . |
         16 . . . 17 . 18 . . . 17 . 16 . . . |
         . . 14 . 16 . . . . . . . . . . . |
         13 . . . 14 . 13 . . . 11 . 13 . . . |
         14 . . . . . . . . . . . . . . . ",
    )
}

/// The lead's bridge answer over the bVI lift — the same shape, resting
/// on the F.
fn lift_melody() -> Lane {
    steps(
        "12 . . . 14 . 16 . . . 14 . 12 . . . |
         . . . . . . . . . . . . . . . . |
         16 . . . 17 . 16 . . . 14 . 12 . . . |
         14 . . . . . . . . . . . . . . . |
         12 . . . 14 . 16 . . . 14 . 12 . . . |
         . . 9 . 12 . . . . . . . . . . . |
         13 . . . 14 . 16 . . . 17 . 18 . . . |
         . . . . . . . . . . . . . . . . ",
    )
}

/// The kit: a soft four-on-the-floor, snare on 2 and 4, offbeat 8th hats.
fn beat() -> DrumLane {
    hits("k.h.s.h.k.h.s.h.")
}

/// Bar 4 of a phrase: the snare roll that telegraphs the next section.
fn fill() -> DrumLane {
    hits("k.h.s.h.k.h.s.ss")
}

/// 8 bars of drums: two 4-bar phrases, each ending in the fill.
fn drums_8() -> DrumLane {
    cat_hits([beat().repeat(3), fill(), beat().repeat(3), fill()])
}

/// Intro: the chord loop and the arp, nothing else — the glow before the
/// pulse.
fn intro() -> SectionSpec {
    section(
        "intro",
        [
            pad(repeat(chords(HOME), 2)),
            arp(repeat(home_arp(), 2)).vel(0.8),
        ],
    )
}

/// Verse: + the bass pump and the kit.
fn verse() -> SectionSpec {
    section(
        "verse",
        [
            bass(repeat(bass_line(HOME), 2)),
            pad(repeat(chords(HOME), 2)),
            arp(repeat(home_arp(), 2)),
            drums(drums_8()),
        ],
    )
}

/// The refrain: the verse with the lead on top. Cool = the first half of
/// the motif only (the call without the answer); Warm = the whole motif;
/// Hot = the motif doubled an octave up in the arp channel and 16th hats.
fn refrain(heat: Intensity) -> SectionSpec {
    let melody = match heat {
        Cool => Lane::from(motif().0[..64].to_vec()).then(repeat(steps("."), 64)),
        Warm | Hot => motif(),
    };
    let top = match heat {
        Hot => transpose(motif(), 7),
        _ => Lane::default(),
    };
    let kit = match heat {
        Hot => cat_hits([
            hits("k.h.s.h.k.h.s.hh").repeat(3),
            fill(),
            hits("k.h.s.h.k.h.s.hh").repeat(3),
            hits("khhhshhhkhhhssss"),
        ]),
        _ => drums_8(),
    };
    section(
        "refrain",
        [
            bass(repeat(bass_line(HOME), 2)),
            lead(melody),
            pad(repeat(chords(HOME), 2)),
            arp(repeat(home_arp(), 2)).vel(0.85),
            arp(top),
            drums(kit),
        ],
    )
    .ducked()
}

/// The bridge: bVI becomes home for two bars at a time — the emotional
/// lift — with the lead answering over it.
fn bridge() -> SectionSpec {
    section(
        "bridge",
        [
            bass(repeat(bass_line(LIFT), 2)),
            lead(lift_melody()),
            pad(repeat(chords(LIFT), 2)),
            arp(repeat(lift_arp(), 2)),
            drums(drums_8()),
        ],
    )
}

/// Outro: back to pad + arp, thinning, so the loop lands on a breath.
fn outro() -> SectionSpec {
    section(
        "outro",
        [
            pad(repeat(chords(HOME), 2)).vel(0.8),
            arp(sparsify(repeat(home_arp(), 2), 41, 0.35)).vel(0.6),
        ],
    )
}

fn build() -> SongSpec {
    song("Neon Checksum", Key::new(A1, MINOR), 100.0)
        .waves(
            Wave::Sawtooth,
            Wave::Sawtooth,
            Wave::DarkPad,
            Wave::Triangle,
        )
        .intensity(0.7)
        .arrange([
            intro(),
            verse(),
            refrain(Cool),
            verse(),
            bridge(),
            refrain(Warm),
            refrain(Hot),
            outro(),
        ])
        .build()
}

/// The finished song, built once.
pub fn spec() -> SongSpec {
    static SPEC: OnceLock<SongSpec> = OnceLock::new();
    *SPEC.get_or_init(build)
}
