//! "Static Teeth" (AGGRESSIVE / dubstep): the half-time drop. F minor, 140
//! bpm, i–i–VI–iv. A REESE bass under an eighth-note WOBBLE, a second Reese
//! wobbling in sixteenths for the second drop, FM growl stabs, a square
//! lead that "yoys" into every note (a −7 st bend), a dark saw stack in
//! power chords, and a build whose pad filter opens across the whole
//! section into a snare roll.
//!
//! The showcase of the MODIFIERS: `with_bend`, `with_wobble`, `Wave::Reese`
//! / `Wave::Fm`, and the section RAMPS on the live lane channels
//! (`Ramp::cutoff` opening the pad through the build, `Ramp::reverb`
//! drowning the lead in the break, `Ramp::level` fading the outro).

use super::super::compose::*;
use super::{Echo, Ramp, SongSpec, Voice, Wave, LEAD, MINOR, PAD};
use std::sync::OnceLock;

/// The chord roots, one per bar: i – i – VI – iv (Fm Fm Db Bbm).
const ROOTS: [i32; 4] = [0, 0, 5, 3];
/// The same roots in the bass register.
const LOW_ROOTS: [i32; 4] = [0, 0, -2, -4];

/// The pad: one power chord a bar, the whole bar long.
fn pad_bed() -> Part {
    let bar = |root| held(transpose(steps("0"), root), 16);
    pad(cat(ROOTS.map(bar))).voiced("p")
}

/// The wobble bass: the root held half a bar, restruck (the wobble
/// restarts with every note), then two quarter-note pushes.
fn wobble_bar(root: i32) -> Lane {
    transpose(steps("0 _ _ _ _ _ _ _ 0 _ _ _ 0 _ _ _"), root)
}

fn wobble_bass() -> Part {
    bass(cat(LOW_ROOTS.map(wobble_bar)))
}

/// The second drop's fast wobble, on the KEYS lane's Reese, in
/// alternation with the bass: bars one and two the bass, three and four
/// the fast one.
fn fast_wobble() -> Part {
    let rest_bar = repeat(steps("."), 16);
    keys(cat([
        rest_bar.clone(),
        rest_bar,
        wobble_bar(-2),
        wobble_bar(-4),
    ]))
}

fn bass_first_half() -> Part {
    let rest_bar = repeat(steps("."), 16);
    bass(cat([
        wobble_bar(0),
        wobble_bar(0),
        rest_bar.clone(),
        rest_bar,
    ]))
}

/// FM growl stabs: a stab on the one, two answers.
fn growls() -> Part {
    let bar = |root| transpose(steps("0 . . . . . 7 . . . 3 _ . . . ."), root);
    arp(cat(ROOTS.map(bar))).accents("9.....6...7.....")
}

/// The "yoy" lead: sparse, every note bending in from below.
fn yoy() -> Lane {
    steps(
        "14 _ _ _ . . . . 12 _ _ _ 14 _ _ _
         16 _ _ _ _ _ _ _ . . . . 14 _ _ _
         12 _ _ _ . . . . 14 _ _ _ 16 _ _ _
         19 _ _ _ _ _ _ _ 14 _ _ _ _ _ _ _",
    )
}

/// The half-time kit: kick on the one, snare on the three.
fn halftime() -> Part {
    drums("k.......s.......").accents("9.......9.......")
}

fn hats() -> Part {
    perc("h.h.h.h.h.h.h.h.").accents("5.3.5.3.5.3.5.3.")
}

fn intro() -> SectionSpec {
    section(
        "intro",
        [
            pad_bed().vel(0.8),
            perc("h...h...h...h...").accents("3...3...3...3..."),
        ],
    )
    .ramps([Ramp::cutoff(PAD, 250.0, 2400.0)])
}

/// The build: four-on-the-floor, the pad opening all the way, a snare
/// roll that doubles every bar into the drop.
fn build_up() -> SectionSpec {
    section(
        "build",
        [
            pad_bed(),
            bass(held(steps("0 0 -2 -4"), 16)).accents("6"),
            lead(yoy()).vel(0.8),
            drums(hits("k...k...k...k...").repeat(4)),
            perc(cat_hits([
                hits("s.......s......."),
                hits("s...s...s...s..."),
                hits("s.s.s.s.s.s.s.s."),
                hits("ssssssssssssssss"),
            ]))
            .accents("4444555566667777 | 5555666677778888 | 6666777788889999 | 7777888899999999"),
        ],
    )
    .ducked()
    .ramps([
        Ramp::cutoff(PAD, 2400.0, 16_000.0),
        Ramp::reverb(LEAD, 0.2, 0.6),
    ])
}

fn drop() -> SectionSpec {
    section(
        "drop",
        [
            wobble_bass(),
            growls(),
            pad_bed().vel(0.7),
            lead(yoy()),
            halftime(),
            hats(),
            perc("x...............").accents("7"),
        ],
    )
    .ducked()
}

fn drop_fast() -> SectionSpec {
    section(
        "drop 2",
        [
            bass_first_half(),
            fast_wobble(),
            growls(),
            pad_bed().vel(0.7),
            halftime(),
            hats(),
        ],
    )
    .ducked()
}

/// The break: the drums out, the lead alone over the pad, sinking into the
/// hall as the section goes.
fn breakdown() -> SectionSpec {
    section("break", [pad_bed().vel(0.6), lead(yoy()).accents("7")]).ramps([
        Ramp::reverb(LEAD, 0.2, 0.9),
        Ramp::cutoff(PAD, 6000.0, 400.0),
    ])
}

fn outro() -> SectionSpec {
    section(
        "outro",
        [
            pad_bed().vel(0.6),
            perc("h...h...h...h...").accents("3...3...3...3..."),
        ],
    )
    .ramps([Ramp::level(PAD, 1.0, 0.0)])
}

fn build() -> SongSpec {
    song("Static Teeth", Key::new(43.65, MINOR), 140.0) // F1
        .voices(
            // bass: the Reese, an eighth-note wobble, a sine under it
            Voice::mono(Wave::Reese)
                .with_wobble(2.0, 90.0, 2600.0, 5.0)
                .with_env(0.01, 1.0)
                .with_sub(0.5),
            // lead: a square yoying in from a fifth below, a little wide
            Voice::wide(Wave::Square, 0.1, 8.0, 0.4)
                .with_bend(-7.0, 0.12)
                .with_filter(600.0, 5000.0, 0.0, 0.25, 2.0)
                .with_echo(0.3)
                .with_reverb(0.2),
            // pad: a dark five-saw stack (its cutoff is what the ramps move)
            Voice::stack(Wave::Sawtooth, 0.0, 14.0, 0.9, 5)
                .with_env(0.05, 4.0)
                .with_drive(0.2)
                .with_reverb(0.3),
            // arp: the FM growl, driven, dry
            Voice::panned(Wave::Fm, -0.15)
                .with_env(0.005, 1.5)
                .with_drive(0.5),
            // keys: the second Reese, wobbling in sixteenths
            Voice::mono(Wave::Reese)
                .with_wobble(1.0, 90.0, 3200.0, 6.0)
                .with_env(0.01, 1.0)
                .with_sub(0.4),
        )
        .intensity(1.0)
        .sidechain(super::Sidechain::new(0.7, 0.6))
        .echo(Echo::new(3.0, 0.35, 2200.0))
        .sweep(0.0)
        .melodic_gain(1.0)
        .arrange([
            intro(),
            build_up(),
            drop(),
            drop_fast(),
            breakdown(),
            build_up(),
            drop(),
            drop_fast(),
            outro(),
        ])
        .build()
}

pub fn spec() -> SongSpec {
    static SPEC: OnceLock<SongSpec> = OnceLock::new();
    *SPEC.get_or_init(build)
}
