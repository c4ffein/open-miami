//! TRACK 5 — "Signal Rot": FLOORS 9–12. 55% darksynth / 35% witch house /
//! 10% wave. G Locrian, 112 BPM, 96 bars (~3:26).
//!
//! Something is wrong with the building. Combat energy intact, but the
//! track keeps glitching toward ritual: the darksynth `drive` (a 16th riff
//! on the driven bass, four-on-the-floor, gated arp, machine lead)
//! alternates with `rot` — the SAME riff shifted by the tritone (in Locrian,
//! four scale degrees up IS the b5), the lead slowed to half speed and
//! shifted with it, the pad parked on the tritone chord, the kick limping
//! (dropped, then displaced) — and each cycle the corrupted variant lasts
//! LONGER: 4 bars, 8, 12, then 20. This is "the refrain, hotter" inverted:
//! the refrain returns SICKER. `haze` is the wave 10%: eight bars of an
//! Eb / Cm bed with the arp thinned to raindrops, half-time and nearly
//! empty. The outro is the rot with the beat gone.
//!
//! Darksynth checklist (docs/music/DARKSYNTH.md):
//! - [x] the bass riff carries 8 bars alone (`intro`)
//! - [x] the b2 appears (Ab in the riff, the bII pad every fourth bar)
//! - [x] everything pumps on the kick (drive + rot ducked)
//! - [x] drive (four-on-the-floor) and half-time (`rot`'s limp) alternate
//! - [x] the refrain returns — sicker each time (`rot(4)` → `rot(20)`)
//! - [x] it slaps with the lead muted (the intro's riff + kit)
//!
//! Witch-house checklist (docs/music/WITCH_HOUSE.md), for the rot:
//! - [x] a drone that never cadences (the Db pad under the G riff)
//! - [ ] audible detune beating — not available in the engine; the tritone
//!   pad against the root riff is the substitute
//! - [x] a tritone move where a normal one was expected (`transpose(4)`)
//! - [x] a limping beat at least once per section (`limp_kit`)
//! - [x] the soundtrack itself sounds corrupted: same material, wrong key,
//!   half speed

use super::super::compose::*;
use super::{SongSpec, Wave, LOCRIAN};
use std::sync::OnceLock;

/// In Locrian the fifth scale degree above the root is the b5: shifting a
/// riff by four degrees shifts it by a tritone.
const TRITONE: i32 = 4;

/// THE RIFF, one bar of 16ths: root, the octave, the Ab rub.
fn riff_bar() -> Lane {
    steps("0 0 . 0 0 0 . 0 7 . 0 0 1 . 0 0")
}

/// The 4-bar riff: three of the cell, then a bar that already lands on the
/// tritone — the rot foreshadowed in the healthy riff.
fn riff() -> Lane {
    cat([
        riff_bar(),
        riff_bar(),
        riff_bar(),
        steps("0 0 . 0 0 0 . 0 4 . 4 4 1 . 1 ."),
    ])
}

/// One bar of pad, retriggered every beat.
fn bed(root: i32) -> Lane {
    transpose(steps("0 . . . 0 . . . 0 . . . 0 . . ."), root)
}

/// The drive's 4 bars of pad: i i i bII.
fn chords() -> Lane {
    cat([bed(0), bed(0), bed(0), bed(1)])
}

/// The machine lead, two bars: G, the Ab rub, the fall — and the Db
/// (tritone) reached for in bar 2.
fn cell() -> Lane {
    steps(
        "14 . 14 . . 15 . . 14 . . . 11 . 12 . |
         14 . 14 . . 15 . . 18 . . . 15 . 14 . ",
    )
}

/// One bar of the gated arp: G Bb Db Bb G Bb Ab Bb — the diminished triad
/// with the b2 bitten in.
fn gate() -> Lane {
    repeat(Lane::from(vec![14, 16, 18, 16, 14, 16, 15, 16]), 2)
}

/// The drive kit: four-on-the-floor, offbeat hats, a snare double every
/// fourth bar.
fn drive_kit(bars: usize) -> DrumLane {
    cat_hits([hits("k.h.k.s.k.h.k.s.").repeat(3), hits("k.h.k.s.k.h.k.ss")]).repeat(bars / 4)
}

/// The limping kit, four bars: half-time, one hat burst, then the kick
/// DROPPED on bar 3 and DISPLACED on bar 4.
fn limp_kit(bars: usize) -> DrumLane {
    cat_hits([
        hits("k.......s......."),
        hits("k.......s...hhhh"),
        hits("........s......."),
        hits("......k.s.....k."),
    ])
    .repeat(bars / 4)
}

/// Intro: the riff, hat ticks, then the kick under it from bar 5.
fn intro() -> SectionSpec {
    section(
        "intro",
        [
            bass(repeat(riff(), 2)),
            drums(cat_hits([
                hits("..h...h...h...h.").repeat(4),
                hits("k.h.k.h.k.h.k.h.").repeat(4),
            ])),
        ],
    )
    .ducked()
}

/// Drive, `bars` long (a multiple of 4): the healthy darksynth section.
fn drive(bars: usize) -> SectionSpec {
    let n = bars / 4;
    section(
        "drive",
        [
            bass(repeat(riff(), n)),
            lead(repeat(cell(), 2 * n)),
            pad(repeat(chords(), n)).vel(0.8),
            arp(repeat(gate(), 4 * n)),
            drums(drive_kit(bars)),
        ],
    )
    .ducked()
}

/// Rot, `bars` long (a multiple of 4): the same section, corrupted — riff
/// and lead a tritone off, the lead at half speed, the pad parked on the
/// tritone chord, the arp decaying, the kick limping.
fn rot(bars: usize) -> SectionSpec {
    let n = bars / 4;
    section(
        "rot",
        [
            bass(transpose(repeat(riff(), n), TRITONE)),
            lead(transpose(stretch(repeat(cell(), n), 2), TRITONE)).vel(0.9),
            pad(repeat(bed(TRITONE), 4 * n)),
            arp(sparsify(repeat(gate(), 4 * n), bars as u32, 0.5)).vel(0.7),
            drums(limp_kit(bars)),
        ],
    )
    .ducked()
}

/// Haze: the wave interlude. An Eb / Cm bed, the arp thinned to raindrops,
/// the lead sighing, half-time and mostly empty.
fn haze() -> SectionSpec {
    section(
        "haze",
        [
            bass(repeat(
                steps("5 . . . . . . . . . . . . . . . | . . . . . . . . . . . . . . . . "),
                4,
            )),
            lead(repeat(
                steps(
                    "16 . . . 15 . . . . . . . . . . . |
                     . . . . . . . . . . . . . . . . |
                     18 . . . 15 . . . . . 14 . . . . . |
                     . . . . . . . . . . . . . . . . ",
                ),
                2,
            ))
            .vel(0.7),
            pad(cat([repeat(bed(5), 4), repeat(bed(3), 4)])),
            arp(sparsify(repeat(gate(), 8), 7, 0.75)).vel(0.5),
            drums(cat_hits([hits("k.......s......."), hits("................")]).repeat(4)),
        ],
    )
}

/// Outro: the rot with the beat gone — pad, the decayed arp and the
/// slowed lead alone, eight bars, then it stops.
fn outro() -> SectionSpec {
    section(
        "outro",
        [
            lead(transpose(stretch(repeat(cell(), 2), 2), TRITONE)).vel(0.6),
            pad(repeat(bed(TRITONE), 8)),
            arp(sparsify(repeat(gate(), 8), 99, 0.7)).vel(0.5),
        ],
    )
}

fn build() -> SongSpec {
    song("Signal Rot", Key::new(G1, LOCRIAN), 112.0)
        .waves(
            Wave::DrivenBass,
            Wave::Supersaw,
            Wave::DarkPad,
            Wave::Square,
        )
        .intensity(1.0)
        .arrange([
            intro(),
            drive(8),
            rot(4),
            drive(8),
            rot(8),
            haze(),
            drive(8),
            rot(12),
            drive(4),
            rot(20),
            outro(),
        ])
        .build()
}

/// The finished song, built once.
pub fn spec() -> SongSpec {
    static SPEC: OnceLock<SongSpec> = OnceLock::new();
    *SPEC.get_or_init(build)
}
