//! TRACK 6 — "Crown of Static": FLOOR 13 and 13½ — the BOSS. 75% witch
//! house / 25% darksynth. F Locrian, 70 BPM, 80 bars (~4:34).
//!
//! The mask comes off. Ritual, not song: the sections are STATES — `drone`
//! (the pad refusing to cadence over a driven-bass drone), `chant` (a
//! four-note cell on the triangle repeated well past comfort, every fourth
//! bar shifted by the tritone instead of a logical step, with metallic hat
//! ticks), `procession` (the funeral beat: kick on 1, snare on 3, with the
//! kick DROPPED on every fourth bar — the missing heartbeat) and `seizure`
//! (16th hat clusters, snare stutters, the cell and the bass a tritone off,
//! no kick at all) — cycled with worsening corruption each pass. When the
//! mask cracks the darksynth kit enters at funeral tempo: `procession(Warm)`
//! brings the driven bass to 8ths under full duck, and the ONE bright
//! element — a high supersaw stab, rare, alarming — lands on the mask-crack
//! bars. `procession(Hot)` doubles the kick, stabs every other bar, and
//! lifts the chant an octave. Ends where it began: the drone.
//!
//! Witch-house checklist (docs/music/WITCH_HOUSE.md):
//! - [x] a drone / pedal that never cadences (the F pad, every section)
//! - [ ] audible detune beating — not available in the engine; the b2 (Gb)
//!   held against the root in the chant is the substitute
//! - [x] a tritone move where a normal one was expected (bar 4 of the
//!   chant, the whole of `seizure`)
//! - [x] a limping beat at least once per section (bar 4 of `procession`,
//!   no kick at all in `seizure`)
//! - [x] the soundtrack itself sounds corrupted: the darksynth kit at 70
//!   BPM with its heartbeat missing
//!
//! Darksynth 25%: the driven bass 8ths + the full duck from `procession
//! (Warm)` on, the supersaw stab.

use super::super::compose::*;
use super::{SongSpec, Wave, LOCRIAN};
use std::sync::OnceLock;
use Intensity::{Cool, Hot, Warm};

/// Four scale degrees up in Locrian = the b5, the tritone.
const TRITONE: i32 = 4;

/// One bar of pad on `root`, retriggered every beat: the pedal.
fn bed(root: i32) -> Lane {
    transpose(steps("0 . . . 0 . . . 0 . . . 0 . . ."), root)
}

/// The drone bass: the root on every 8th, felt as one continuous growl.
fn drone_bass() -> Lane {
    steps("0 . 0 . 0 . 0 . 0 . 0 . 0 . 0 .")
}

/// The procession bass, one bar: the root in 8ths, the tritone on beat 4.
fn march_bass() -> Lane {
    steps("0 . 0 . 0 . 0 . 0 . 0 . 4 . 4 .")
}

/// THE CHANT, one bar: F, the Gb rub, F, down to Db — four notes, then a
/// hole.
fn cell() -> Lane {
    steps("14 . . 15 . . 14 . 11 . . . . . . .")
}

/// Four bars of the chant: three as written, the fourth a tritone off.
fn chant_4() -> Lane {
    cat([cell(), cell(), cell(), transpose(cell(), TRITONE)])
}

/// Metallic ticks, one bar: sparse hats where the hats should be steady.
fn ticks() -> DrumLane {
    hits("..h.....h.h.....")
}

/// The funeral beat, four bars: kick on 1, snare on 3 — and on bar 4 the
/// kick is MISSING.
fn funeral(heat: Intensity) -> DrumLane {
    let bar = match heat {
        Cool => hits("k.......s......."),
        Warm => hits("k.......s.....h."),
        Hot => hits("k.k.....s.....h."),
    };
    let limp = match heat {
        Hot => hits("........s...hhhh"),
        _ => hits("........s......."),
    };
    cat_hits([bar.repeat(3), limp])
}

/// The seizure kit, four bars: 16th hat clusters that vanish, snare
/// stutters, no kick.
fn seizure_kit() -> DrumLane {
    cat_hits([
        hits("hhhh....hhhhhh.."),
        hits("........s......."),
        hits("hhhhhhhh........"),
        hits("s.s.s.s.ssss...."),
    ])
}

/// The bright stab: the high F, once, on a mask-crack bar. `every` bars.
fn stab(every: usize) -> Lane {
    steps("21 . . . . . . . . . . . . . . .").then(repeat(steps("."), 16 * (every - 1)))
}

/// Drone: the pedal and the growl. No drums.
fn drone() -> SectionSpec {
    section(
        "drone",
        [
            bass(repeat(drone_bass(), 8)).vel(0.6),
            pad(repeat(bed(0), 8)),
        ],
    )
}

/// Chant: the cell over the drone with the ticks. Hot = the chant an
/// octave up as well, with holes eaten into it.
fn chant(heat: Intensity) -> SectionSpec {
    let top = match heat {
        Hot => sparsify(transpose(repeat(chant_4(), 2), 7), 13, 0.4),
        _ => Lane::default(),
    };
    section(
        "chant",
        [
            bass(repeat(drone_bass(), 8)).vel(0.6),
            pad(repeat(bed(0), 8)),
            arp(repeat(chant_4(), 2)),
            arp(top),
            drums(ticks().repeat(8)),
        ],
    )
}

/// Procession: the funeral beat under the chant. Cool = the drone bass
/// still; Warm = the mask cracks: driven 8ths with the tritone on beat 4,
/// full duck, the stab every fourth bar; Hot = double kick, the stab every
/// other bar, the chant lifted an octave.
fn procession(heat: Intensity) -> SectionSpec {
    let bass_lane = match heat {
        Cool => repeat(drone_bass(), 8),
        Warm | Hot => repeat(march_bass(), 8),
    };
    let bright = match heat {
        Cool => Lane::default(),
        Warm => repeat(stab(4), 2),
        Hot => repeat(stab(2), 4),
    };
    let voice = match heat {
        Hot => transpose(repeat(chant_4(), 2), 7),
        _ => repeat(chant_4(), 2),
    };
    section(
        "procession",
        [
            bass(bass_lane),
            lead(bright),
            pad(repeat(bed(0), 8)),
            arp(voice),
            drums(funeral(heat).repeat(2)),
        ],
    )
    .ducked()
}

/// Seizure: four bars — the pad and the bass a tritone off, the chant
/// shifted with them, the kit convulsing without a kick.
fn seizure() -> SectionSpec {
    section(
        "seizure",
        [
            bass(transpose(repeat(drone_bass(), 4), TRITONE)),
            pad(repeat(bed(TRITONE), 4)),
            arp(transpose(chant_4(), TRITONE)),
            drums(seizure_kit()),
        ],
    )
    .ducked()
}

fn build() -> SongSpec {
    song("Crown of Static", Key::new(F1, LOCRIAN), 70.0)
        .waves(
            Wave::DrivenBass,
            Wave::Supersaw,
            Wave::DarkPad,
            Wave::Triangle,
        )
        .intensity(1.15)
        .arrange([
            drone(),
            chant(Cool),
            procession(Cool),
            seizure(),
            drone(),
            procession(Warm),
            seizure(),
            procession(Hot),
            chant(Hot),
            procession(Hot),
            drone(),
        ])
        .build()
}

/// The finished song, built once.
pub fn spec() -> SongSpec {
    static SPEC: OnceLock<SongSpec> = OnceLock::new();
    *SPEC.get_or_init(build)
}
