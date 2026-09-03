//! TRACK 3 — "Service Corridor": FLOORS 1–4. 85% darksynth / 15%
//! synthwave. E Phrygian, 118 BPM, 88 bars (~2:59).
//!
//! The grind begins. Confident, athletic hostility — the early floors where
//! the player feels strong. The BASS RIFF IS THE HOOK: a one-bar 16th cell
//! on the driven bass (root, octave jump, the b2 F rubbing the root) with a
//! turnaround bar, cold-opened alone, then built by ADDING LAYERS — kick,
//! hats, the gated arp, the dark pad — never by changing the riff. The
//! supersaw lead is a machine part: a two-bar cell that returns as the
//! refrain three times, hotter each pass (Cool: the cell; Warm: the pad up
//! and 16th hats; Hot: the cell doubled an octave up in the arp channel and
//! a double-kick fill). Drive and half-time sections alternate. The
//! synthwave 15% is the bridge: one warm bVI (C) lift for four bars, then
//! back to the grind. Ends cold on the riff and the kick.
//!
//! Darksynth checklist (docs/music/DARKSYNTH.md):
//! - [x] the bass riff carries 8 bars alone (`open`)
//! - [x] the b2 appears (F in the riff, the bII pad every fourth bar, the
//!   lead's F stab)
//! - [x] everything pumps on the kick (`.ducked()` on every section with
//!   a kick)
//! - [x] drive and half-time alternate (`drive` / `refrain` vs `half_time`)
//! - [x] the refrain returns three times, hotter each time
//! - [x] it slaps with the lead muted (`drive` = the refrain without it)

use super::super::compose::*;
use super::{SongSpec, Wave, PHRYGIAN};
use std::sync::OnceLock;
use Intensity::{Cool, Hot, Warm};

/// THE RIFF, one bar: root hammering in 16ths, the octave, the b2.
fn riff_bar() -> Lane {
    steps("0 . 0 0 . 0 . 0 7 . 0 0 1 . 0 .")
}

/// The turnaround bar: up to the 4th, the b2 dragging it back down.
fn turnaround() -> Lane {
    steps("0 . 0 0 . 0 . 0 3 . 3 3 1 . 1 .")
}

/// The 4-bar riff: three of the cell and the turnaround.
fn riff() -> Lane {
    cat([riff_bar(), riff_bar(), riff_bar(), turnaround()])
}

/// One bar of pad, retriggered every beat.
fn bed(root: i32) -> Lane {
    transpose(steps("0 . . . 0 . . . 0 . . . 0 . . ."), root)
}

/// The pad's 4 bars: i i i bII — the F major triad over the E riff.
fn chords() -> Lane {
    cat([bed(0), bed(0), bed(0), bed(1)])
}

/// One bar of the gated 16th arp: the minor triad `[a, b, c]` with the b2
/// `d` bitten into the second half.
fn gate_bar(t: [i32; 4]) -> Lane {
    let [a, b, c, d] = t;
    repeat(Lane::from(vec![a, b, c, b, a, b, d, b]), 2)
}

/// The 4-bar arp: E minor + F for three bars, F major + G on the bII bar.
fn gated_arp() -> Lane {
    cat([
        gate_bar([14, 16, 18, 15]),
        gate_bar([14, 16, 18, 15]),
        gate_bar([14, 16, 18, 15]),
        gate_bar([15, 17, 19, 16]),
    ])
}

/// The lead cell: 4 bars — the machine part (E, the F rub, the fall to B)
/// and its answer that drops to the octave below.
fn cell() -> Lane {
    steps(
        "14 . . 14 . 15 . 14 . . . . 12 . 11 . |
         14 . . 14 . 15 . 17 . . . . 15 . 14 . |
         14 . . 14 . 15 . 14 . . . . 12 . 11 . |
         12 . . 12 . 11 . 9 . . . . 8 . 7 . ",
    )
}

/// The bridge's warm answer over C: C E G, resting.
fn lift_cell() -> Lane {
    steps(
        "12 . . . 14 . 16 . . . 14 . 12 . . . |
         . . 9 . 12 . . . . . . . . . . . |
         12 . . . 14 . 16 . . . 17 . 16 . . . |
         14 . . . . . . . . . . . . . . . ",
    )
}

/// Drive kit: four-on-the-floor (snare taking 2 and 4), offbeat 8th hats.
fn drive_bar() -> DrumLane {
    hits("k.h.k.s.k.h.k.s.")
}

/// Drive kit with 16th hats — the lift.
fn drive_bar_16() -> DrumLane {
    hits("khhhksh.khhhksh.")
}

/// The fill bar: the last-16th snare doubles.
fn fill() -> DrumLane {
    hits("k.h.k.s.k.h.k.ss")
}

/// The double-kick fill bar for the hot refrain.
fn fill_hot() -> DrumLane {
    hits("k.h.k.s.kkh.ksss")
}

/// 8 bars of the drive kit; `hot` = 16th hats in the second phrase and the
/// double-kick fill.
fn drive_kit(heat: Intensity) -> DrumLane {
    match heat {
        Cool => cat_hits([drive_bar().repeat(3), fill(), drive_bar().repeat(3), fill()]),
        Warm => cat_hits([
            drive_bar().repeat(3),
            fill(),
            drive_bar_16().repeat(3),
            fill(),
        ]),
        Hot => cat_hits([
            drive_bar_16().repeat(3),
            fill(),
            drive_bar_16().repeat(3),
            fill_hot(),
        ]),
    }
}

/// The half-time kit: kick on 1, snare on 3, one hat — sparse = heavier.
fn half_kit() -> DrumLane {
    cat_hits([hits("k...h...s.....h.").repeat(3), hits("k...h...s...ssss")]).repeat(2)
}

/// Cold open: the riff alone for four bars, then the kick under it.
fn open() -> SectionSpec {
    section(
        "open",
        [
            bass(repeat(riff(), 2)),
            drums(cat_hits([
                hits("................").repeat(4),
                hits("k...k...k...k...").repeat(4),
            ])),
        ],
    )
    .ducked()
}

/// Build: + hats and the gated arp, the pad far back.
fn build_up() -> SectionSpec {
    section(
        "build",
        [
            bass(repeat(riff(), 2)),
            pad(repeat(chords(), 2)).vel(0.6),
            arp(repeat(gated_arp(), 2)),
            drums(hits("k.h.k.h.k.h.k.h.").repeat(8)),
        ],
    )
    .ducked()
}

/// Drive: the full kit under riff + arp + pad — the refrain with the lead
/// muted. It must slap on its own.
fn drive() -> SectionSpec {
    section(
        "drive",
        [
            bass(repeat(riff(), 2)),
            pad(repeat(chords(), 2)).vel(0.8),
            arp(repeat(gated_arp(), 2)),
            drums(drive_kit(Cool)),
        ],
    )
    .ducked()
}

/// The refrain: drive + the supersaw cell. Returns hotter each time.
fn refrain(heat: Intensity) -> SectionSpec {
    let pad_vel = match heat {
        Cool => 0.7,
        Warm | Hot => 1.0,
    };
    let top = match heat {
        Hot => transpose(repeat(cell(), 2), 7),
        _ => Lane::default(),
    };
    section(
        "refrain",
        [
            bass(repeat(riff(), 2)),
            lead(repeat(cell(), 2)),
            pad(repeat(chords(), 2)).vel(pad_vel),
            arp(repeat(gated_arp(), 2)),
            arp(top),
            drums(drive_kit(heat)),
        ],
    )
    .ducked()
}

/// Half-time: the riff keeps its 16ths, the kit drops to 1 and 3, the lead
/// only stabs the b2.
fn half_time() -> SectionSpec {
    section(
        "half-time",
        [
            bass(repeat(riff(), 2)),
            lead(repeat(
                steps(
                    "15 . . . . . . . . . . . . . . . |
                     . . . . . . . . 14 . . . . . . . ",
                ),
                4,
            )),
            pad(repeat(chords(), 2)),
            drums(half_kit()),
        ],
    )
    .ducked()
}

/// The bridge: the one warm lift — the riff and the pad on bVI (C) for
/// four bars, the lead answering, then straight back to the grind.
fn bridge() -> SectionSpec {
    section(
        "bridge",
        [
            bass(transpose(riff(), 5).then(riff())),
            lead(lift_cell().then(cell())),
            pad(repeat(bed(5), 4).then(chords())),
            arp(transpose(gated_arp(), 5).then(gated_arp())),
            drums(drive_kit(Warm)),
        ],
    )
    .ducked()
}

/// Breakdown: pad + arp only, four bars, the snare roll into the drop.
fn breakdown() -> SectionSpec {
    section(
        "breakdown",
        [
            pad(chords()),
            arp(gated_arp()).vel(0.8),
            drums(cat_hits([
                hits("................").repeat(3),
                hits("............ssss"),
            ])),
        ],
    )
}

/// Outro: the riff and the kick, four bars, then nothing. Cold.
fn outro() -> SectionSpec {
    section(
        "outro",
        [bass(riff()), drums(hits("k...k...k...k...").repeat(4))],
    )
    .ducked()
}

fn build() -> SongSpec {
    song("Service Corridor", Key::new(E1, PHRYGIAN), 118.0)
        .waves(
            Wave::DrivenBass,
            Wave::Supersaw,
            Wave::DarkPad,
            Wave::Square,
        )
        .intensity(0.95)
        .arrange([
            open(),
            build_up(),
            refrain(Cool),
            drive(),
            half_time(),
            refrain(Warm),
            bridge(),
            breakdown(),
            drive(),
            refrain(Hot),
            half_time(),
            outro(),
        ])
        .build()
}

/// The finished song, built once.
pub fn spec() -> SongSpec {
    static SPEC: OnceLock<SongSpec> = OnceLock::new();
    *SPEC.get_or_init(build)
}
