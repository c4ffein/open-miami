//! "Sodium Lights" (WAVY / driving): the slow-burn night drive. D minor,
//! i–VI–III–VII, a wide detuned saw pad held a bar per chord, a
//! side-chain-pumped bass (retriggered sixteenths under a rising velocity
//! ramp), a tied square lead and an accented arp.
//!
//! THE TOUR of the `compose` v2 side: ties (`_`), [`held`], per-step
//! velocity ([`Part::accents`]), chord lanes ([`Part::voiced`]), the KEYS
//! and PERC parts, the big kit, stereo [`Voice`]s with sends, and the
//! song-level `.sidechain` / `.echo` / `.humanize` / `.sweep`. It was first
//! written as `const` section literals; the rewrite is pinned to be the
//! same song to the last velocity digit
//! (`the_compose_rewrite_of_sodium_lights_is_the_same_song`).

use super::super::compose::*;
use super::{Echo, Sidechain, SongSpec, Voice, Wave, MINOR};
use std::sync::OnceLock;

/// The progression, one chord root per bar: i – VI – III – VII.
const ROOTS: [i32; 4] = [0, 5, 2, 6];
/// The same roots in the bass register (VI and VII below the root).
const LOW_ROOTS: [i32; 4] = [0, -2, 2, -1];

/// The pad: struck once a bar, held twelve steps, released for four —
/// voiced i7 – VI – III(add9) – VII.
fn pad_bed() -> Part {
    let bar = |root| transpose(steps("0 _ _ _ _ _ _ _ _ _ _ _ . . . ."), root);
    pad(cat(ROOTS.map(bar))).voiced(chords("7 t 9 t").each(16))
}

/// The pumped bass: every sixteenth retriggers the chord root, the velocity
/// ramp ducking on each kick and swelling back before the next.
fn pumped_bass() -> Part {
    let bar = |root| transpose(repeat(steps("0"), 16), root);
    bass(cat(LOW_ROOTS.map(bar))).accents("3689")
}

/// Kick on 1 and 3, snare on 2 and 4.
fn beat() -> Part {
    drums("k...s...k...s...")
}

/// Off-beat closed hats riding over the kicks, a clap under each snare, an
/// open hat pushing into the next bar — the PERC lane, so it can sit on
/// the kick's own steps.
fn ride() -> Part {
    perc("h.h.c.h.h.h.c.o.").accents("4060806040608070")
}

fn intro() -> SectionSpec {
    section(
        "intro",
        [
            pad_bed(),
            arp(". . . . 14 . 16 . . . . . 18 . 16 .").accents("9050"),
            drums("..h...h...h...hh").accents("0050005000500053"),
        ],
    )
    .ducked()
}

fn verse() -> SectionSpec {
    section(
        "verse",
        [
            pumped_bass(),
            lead(
                "11 _ _ _ _ _ 9 _ 7 _ _ _ . . . .
                 12 _ _ _ _ _ 11 _ 9 _ _ _ _ _ . .
                 9 _ _ _ 11 _ _ _ 13 _ _ _ _ _ _ _
                 14 _ _ _ _ _ 13 _ 11 _ _ _ . . . .",
            )
            .accents("8999996979999999 | 9999996979999999 | 7999899999999999 | 9999997969999999"),
            pad_bed(),
            beat(),
            ride(),
        ],
    )
    .ducked()
}

fn refrain() -> SectionSpec {
    section(
        "refrain",
        [
            pumped_bass(),
            lead(
                "14 _ _ 13 _ _ 11 _ 9 _ _ _ 11 _ _ _
                 12 _ _ 14 _ _ 12 _ 11 _ _ _ _ _ _ _
                 13 _ _ 14 _ _ 16 _ 14 _ _ _ 13 _ _ _
                 14 _ _ _ _ _ _ _ 11 _ _ _ . . . .",
            ),
            pad_bed(),
            arp("14 16 18 16 14 16 18 21 14 16 18 16 21 18 16 14
                 12 14 16 14 12 14 16 19 12 14 16 14 19 16 14 12
                 16 18 20 18 16 18 20 23 16 18 20 18 23 20 18 16
                 13 15 17 15 13 15 17 20 13 15 17 15 20 17 15 13")
            .accents("95758576"),
            beat(),
            ride(),
        ],
    )
    .ducked()
}

/// The break: the bass holds a whole bar per chord, the lead answers in
/// two long phrases, and the noise RISER (keys) swells through the last
/// two bars into the refrain.
fn breakdown() -> SectionSpec {
    section(
        "break",
        [
            bass(held(steps("0 -2 2 -1"), 16)),
            lead(
                ". . . . . . . . 7 _ _ _ 9 _ _ _ | 11 _ _ _ _ _ _ _ _ _ _ _ _ _ _ _
                 . . . . . . . . 9 _ _ _ 11 _ _ _ | 13 _ _ _ _ _ _ _ _ _ _ _ _ _ _ _",
            )
            .accents("6"),
            pad_bed(),
            keys(repeat(steps("."), 32).then(held(steps("0"), 32))).accents("7"),
            drums("k.......k.....h.").accents("7000000060000040"),
        ],
    )
    .ducked()
}

fn build() -> SongSpec {
    song("Sodium Lights", Key::new(73.42, MINOR), 96.0) // D2
        .voices(
            // bass: a centred saw with a fast resonant pluck and a sine sub under it
            Voice::mono(Wave::Sawtooth)
                .with_filter(230.0, 1100.0, 0.0, 0.12, 3.0)
                .with_sub(0.5),
            // lead: a doubled square, a little right, late vibrato, a soft wow,
            // dotted-eighth echoes trailing into the hall
            Voice::wide(Wave::Square, 0.25, 6.0, 0.3)
                .with_vibrato(5.5, 12.0, 0.25)
                .with_filter(1800.0, 5200.0, 0.0, 0.18, 1.6)
                .with_echo(0.35)
                .with_reverb(0.25),
            // pad: a five-saw supersaw that blooms open over a second, deep in
            // the hall
            Voice::stack(Wave::Sawtooth, 0.0, 12.0, 0.85, 5)
                .with_filter(700.0, 2600.0, 1.1, 0.0, 1.1)
                .with_reverb(0.45),
            // arp: a triangle answering from the left, echoing to the right
            Voice::panned(Wave::Triangle, -0.35).with_echo(0.5),
            // keys: the noise RISER out of the break — a filter opening over
            // five seconds under a slow swell, deep in the hall
            Voice::mono(Wave::Noise)
                .with_env(2.5, 1.0)
                .with_filter(300.0, 7000.0, 4.8, 0.0, 1.8)
                .with_reverb(0.4),
        )
        .intensity(0.8)
        .sidechain(Sidechain::new(0.55, 0.9))
        .echo(Echo::new(3.0, 0.42, 2800.0))
        .humanize(0.004)
        .sweep(0.35)
        // Mixed with the lane panners in place: no centre make-up.
        .melodic_gain(1.0)
        .arrange([
            intro(),
            verse(),
            refrain(),
            verse(),
            refrain(),
            breakdown(),
            refrain(),
            refrain(),
        ])
        .build()
}

pub fn spec() -> SongSpec {
    static SPEC: OnceLock<SongSpec> = OnceLock::new();
    *SPEC.get_or_init(build)
}
