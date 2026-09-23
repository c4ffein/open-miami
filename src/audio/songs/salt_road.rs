//! "Salt Road" (WAVY / acoustic): the coast at dusk, played on STRINGS. E
//! minor, 76 bpm, i–III–VII–VI. A fingered bass guitar on the roots, a
//! picked guitar arpeggio, strummed guitar chords, and a violin that enters
//! on the second verse and is joined by a second one, a third below, for
//! the last chorus and the bridge duet.
//!
//! The showcase of the COMPUTED voices (`audio/dsp.rs`): `Wave::Guitar`,
//! `Wave::BassGuitar`, `Wave::Violin` — plucked and bowed strings rendered
//! sample by sample into the bake; the violins are WIDE detuned pairs (a
//! stereo bake). Written with the `compose` builders.

use super::super::compose::*;
use super::{Echo, Sidechain, SongSpec, Voice, Wave, MINOR};
use std::sync::OnceLock;

/// The progression, one chord root per bar: i – III – VII – VI (Em G D C).
const ROOTS: [i32; 4] = [0, 2, 6, 5];
/// The same roots in the bass register.
const LOW_ROOTS: [i32; 4] = [-7, -5, -1, -2];

/// One bar of the picked arpeggio on `root`: root, third, fifth, octave
/// and back, in eighths — chord tones, so it follows every chord in key.
fn picked_bar(root: i32) -> Lane {
    transpose(steps("0 . 2 . 4 . 7 . 4 . 2 . 4 . 2 ."), root)
}

/// The picked guitar, four bars round the progression.
fn picked() -> Part {
    arp(cat(ROOTS.map(picked_bar)))
}

/// The strummed guitar: one wide chord on the one, a softer one on the
/// three, each ringing through its half bar.
fn strums() -> Part {
    let bar = |root| transpose(steps("0 _ _ _ _ _ _ _ 0 _ _ _ _ _ _ _"), root);
    pad(cat(ROOTS.map(bar)))
        .voiced("w")
        .accents("9.......6.......")
}

/// The bass: the root on the one and the "and" of three.
fn bass_line() -> Part {
    let bar = |root| transpose(steps("0 . . . . . . . . . 0 . . . . ."), root);
    bass(cat(LOW_ROOTS.map(bar)))
}

/// The bass holding whole bars (the bridge).
fn bass_held() -> Part {
    bass(held(steps("-7 -5 -1 -2"), 16))
}

/// The verse violin: a low, sparse line — E G B, B held, G B D, C held.
fn verse_line() -> Lane {
    steps(
        "7 _ _ _ _ _ _ _ 9 _ _ _ 11 _ _ _
         11 _ _ _ _ _ _ _ _ _ _ _ . . . .
         9 _ _ _ _ _ 11 _ 13 _ _ _ _ _ _ _
         12 _ _ _ _ _ _ _ _ _ _ _ . . . .",
    )
}

/// The chorus violin: the hook, chord tones over every bar, landing home
/// on the E under the C chord.
fn chorus_line() -> Lane {
    steps(
        "14 _ _ _ _ _ 16 _ 18 _ _ _ _ _ _ _
         20 _ _ _ 18 _ 16 _ 18 _ _ _ _ _ _ _
         17 _ _ _ _ _ 18 _ 20 _ _ _ _ _ _ _
         19 _ _ _ 18 _ 16 _ 14 _ _ _ _ _ _ _",
    )
}

/// Brushes: a soft kick on the one, the rim on two and four.
fn brushes() -> Part {
    drums("k...r.......r...").accents("5...6.......6...")
}

/// The full kit, still gentle: kick on one and three, snare on two and
/// four, eighth hats on the perc lane.
fn kit() -> Part {
    drums("k...s...k...s...").accents("7...6...6...6...")
}

fn hats() -> Part {
    perc("h.h.h.h.h.h.h.h.").accents("4.3.4.3.4.3.4.3.")
}

fn intro() -> SectionSpec {
    section("intro", [picked().vel(0.9), bass_line().accents("7")])
}

fn verse(with_violin: bool) -> SectionSpec {
    let mut parts = vec![picked(), strums().vel(0.7), bass_line(), brushes()];
    if with_violin {
        parts.push(lead(verse_line()).accents("7"));
    }
    section("verse", parts)
}

/// The chorus; `hot` adds the second violin a third below.
fn chorus(hot: bool) -> SectionSpec {
    let mut parts = vec![
        picked(),
        strums(),
        bass_line(),
        lead(chorus_line()),
        kit(),
        hats(),
    ];
    if hot {
        parts.push(keys(transpose(chorus_line(), -2)).accents("7"));
    }
    section("chorus", parts)
}

/// The bridge: no drums, the bass holds, the guitar thins out, and the two
/// violins take the verse line in octaves.
fn bridge() -> SectionSpec {
    section(
        "bridge",
        [
            arp(sparsify(cat(ROOTS.map(picked_bar)), 11, 0.4)).vel(0.8),
            strums().accents("6.......4......."),
            bass_held(),
            lead(transpose(verse_line(), 7)),
            keys(verse_line()).accents("6"),
        ],
    )
}

fn outro() -> SectionSpec {
    section(
        "outro",
        [
            picked().vel(0.85),
            bass_line().accents("6"),
            lead(held(steps("14 . . ."), 16)).accents("6"),
        ],
    )
}

fn build() -> SongSpec {
    song("Salt Road", Key::new(82.41, MINOR), 76.0) // E2
        .voices(
            // bass: a fingered bass guitar with a sine under it
            Voice::mono(Wave::BassGuitar)
                .with_env(0.005, 5.0)
                .with_sub(0.25),
            // lead: the violins — a detuned pair spread across the image,
            // a touch right —, late vibrato, a short slide into legato
            // notes, echoing into the hall
            Voice::wide(Wave::Violin, 0.15, 7.0, 0.6)
                .with_vibrato(5.2, 10.0, 0.35)
                .with_glide(0.08)
                .with_env(0.12, 2.0)
                .with_echo(0.15)
                .with_reverb(0.45),
            // pad: the strummed guitar, left of centre, ringing six steps
            Voice::panned(Wave::Guitar, -0.2)
                .with_env(0.005, 6.0)
                .with_reverb(0.3),
            // arp: the picked guitar, right, a dotted echo behind it
            Voice::panned(Wave::Guitar, 0.3)
                .with_env(0.005, 6.0)
                .with_echo(0.2)
                .with_reverb(0.25),
            // keys: the second violins, left, slower vibrato, deeper in the hall
            Voice::wide(Wave::Violin, -0.3, 6.0, 0.5)
                .with_vibrato(5.0, 9.0, 0.4)
                .with_glide(0.08)
                .with_env(0.14, 2.0)
                .with_reverb(0.5),
        )
        .intensity(0.9)
        .sidechain(Sidechain::OFF)
        .echo(Echo::new(6.0, 0.3, 2500.0))
        .humanize(0.006)
        .sweep(0.15)
        // Plucked strings are spiky and the kit is brushed: lifted so the
        // piece sits near the synth tracks (measured: 2–3× under Sodium
        // Lights at 1.0).
        .melodic_gain(1.3)
        .arrange([
            intro(),
            verse(false),
            chorus(false),
            verse(true),
            chorus(false),
            bridge(),
            chorus(true),
            outro(),
        ])
        .build()
}

pub fn spec() -> SongSpec {
    static SPEC: OnceLock<SongSpec> = OnceLock::new();
    *SPEC.get_or_init(build)
}
