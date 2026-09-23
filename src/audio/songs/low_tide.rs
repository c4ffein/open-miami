//! "Low Tide" (WAVY / slow-burn): the long one. A minor, 100 bpm, eight
//! sections of eight bars — i–VI–iv–VII under a pad that blooms two bars a
//! chord, a sixteenth-note arpeggio that pulses the whole way through, a
//! sub bass holding whole bars, a late sine lead and a few plucked guitar
//! notes in the middle. Almost no drums: a soft four-on-the-floor arrives
//! in the third section (with a gentle side-chain, so the pad breathes with
//! it), a clap only for the peak, and everything leaves the way it came.
//!
//! The MOVEMENT is the section ramps: the pad's and the arpeggio's filters
//! open across whole sections, the arpeggio fades in over the first
//! twenty seconds, the lead sinks into the hall as the drums leave, the pad
//! fades out at the end — nothing here is a jump. The showcase of the
//! slow-burn kind (`docs/MUSIC_CODE.md`, "Section ramps").

use super::super::compose::*;
use super::{Echo, Ramp, Sidechain, SongSpec, Voice, Wave, ARP, BASS, KEYS, LEAD, MINOR, PAD};
use std::sync::OnceLock;

/// The progression, one chord every TWO bars: i – VI – iv – VII.
const ROOTS: [i32; 4] = [0, 5, 3, 6];
/// The same roots in the bass register.
const LOW_ROOTS: [i32; 4] = [-7, -2, -4, -1];

/// The pad: one add9 chord, blooming, held two bars.
fn pad_bed() -> Part {
    let two_bars = |root| held(transpose(steps("0"), root), 32);
    pad(cat(ROOTS.map(two_bars))).voiced("9")
}

/// The pulse: a sixteenth-note arpeggio over each chord — root, fifth,
/// octave, tenth, back — two bars per chord, accents every other note.
fn pulse() -> Part {
    let cell = |root| transpose(steps("0 4 7 9 7 4 0 4 | 7 9 11 9 7 4 0 4"), root);
    let two_bars = |root| repeat(cell(root), 2);
    arp(cat(ROOTS.map(two_bars))).accents("9575")
}

/// The sub bass: the root held a whole bar, twice per chord.
fn sub_bass() -> Part {
    let two_bars = |root| held(transpose(steps("0 0"), root), 16);
    bass(cat(LOW_ROOTS.map(two_bars)))
}

/// The lead: long sine notes, two phrases across the eight bars.
fn lead_line() -> Lane {
    steps(
        "14 _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ | 16 _ _ _ _ _ _ _ _ _ _ _ _ _ _ _
         19 _ _ _ _ _ _ _ _ _ _ _ 18 _ _ _ | 16 _ _ _ _ _ _ _ _ _ _ _ . . . .
         14 _ _ _ _ _ _ _ 12 _ _ _ _ _ _ _ | 13 _ _ _ _ _ _ _ _ _ _ _ _ _ _ _
         11 _ _ _ _ _ _ _ _ _ _ _ 12 _ _ _ | 14 _ _ _ _ _ _ _ _ _ _ _ _ _ _ _",
    )
}

/// A few plucked guitar notes answering the lead, in the peak.
fn plucks() -> Part {
    keys(steps(
        ". . . . . . . . 7 . . . 9 . . . | . . . . . . . . . . . . . . . .
         . . . . . . . . 11 . . . 9 . . . | . . . . . . . . 7 . . . . . . .
         . . . . . . . . 7 . . . 5 . . . | . . . . . . . . . . . . . . . .
         . . . . . . . . 9 . . . 7 . . . | . . . . . . . . 4 . . . . . . .",
    ))
    .accents("6")
}

/// A soft four-on-the-floor.
fn soft_kick() -> Part {
    drums("k...k...k...k...").accents("6...5...6...5...")
}

/// Off-beat hats, quiet.
fn hats() -> Part {
    perc("..h...h...h...h.").accents("..3...3...3...3.")
}

/// The peak's clap, on two and four.
fn clap() -> Part {
    drums("....c.......c...").accents("....5.......5...")
}

/// The pad's filter opens from 250 Hz to 5 kHz across dawn AND pulse: one
/// ramp over two sections, forty seconds long.
fn dawn() -> SectionSpec {
    section("dawn", [pad_bed(), pulse().vel(0.5)]).ramps([
        Ramp::cutoff(PAD, 250.0, 5000.0).over(2),
        Ramp::level(ARP, 0.0, 1.0),
        Ramp::cutoff(ARP, 300.0, 900.0),
    ])
}

/// The bass arrives by a level ramp, not at once (measured: it entered at
/// a jump of 3.5× the section before).
fn pulse_in() -> SectionSpec {
    section("pulse", [pad_bed(), pulse().vel(0.7), sub_bass()]).ramps([
        Ramp::cutoff(ARP, 900.0, 2600.0),
        Ramp::level(BASS, 0.0, 1.0),
    ])
}

fn rise() -> SectionSpec {
    section("rise", [pad_bed(), pulse(), sub_bass(), soft_kick()])
        .ducked()
        .ramps([
            Ramp::cutoff(ARP, 2600.0, 8000.0),
            Ramp::reverb(ARP, 0.15, 0.35),
        ])
}

fn open() -> SectionSpec {
    section(
        "open",
        [
            pad_bed(),
            pulse(),
            sub_bass(),
            lead(lead_line()).accents("7"),
            soft_kick(),
            hats(),
        ],
    )
    .ducked()
}

fn peak() -> SectionSpec {
    section(
        "peak",
        [
            pad_bed(),
            pulse(),
            sub_bass(),
            lead(transpose(lead_line(), 7)),
            plucks(),
            soft_kick(),
            clap(),
            hats(),
        ],
    )
    .ducked()
}

/// The drums leave, the lead sinks into the hall, the arpeggio darkens.
fn fall() -> SectionSpec {
    section(
        "fall",
        [
            pad_bed(),
            pulse().vel(0.8),
            sub_bass(),
            lead(lead_line()).accents("6"),
        ],
    )
    .ramps([
        Ramp::reverb(LEAD, 0.4, 0.9),
        Ramp::cutoff(ARP, 8000.0, 1200.0),
        Ramp::level(KEYS, 1.0, 0.0),
    ])
}

fn tide() -> SectionSpec {
    section("tide", [pad_bed(), pulse().vel(0.5), sub_bass().vel(0.7)])
        .ramps([Ramp::cutoff(ARP, 1200.0, 400.0), Ramp::level(ARP, 1.0, 0.3)])
}

fn gone() -> SectionSpec {
    section("gone", [pad_bed()])
        .ramps([Ramp::cutoff(PAD, 5000.0, 400.0), Ramp::level(PAD, 1.0, 0.0)])
}

fn build() -> SongSpec {
    song("Low Tide", Key::new(110.0, MINOR), 100.0) // A2
        .voices(
            // bass: a dark saw with a big sine under it, holding whole bars
            Voice::mono(Wave::Sawtooth)
                .with_filter(120.0, 400.0, 0.4, 0.0, 0.8)
                .with_env(0.05, 2.0)
                .with_sub(0.4),
            // lead: a sine, late vibrato, sliding between legato notes,
            // echoing into the hall
            Voice::panned(Wave::Sine, 0.1)
                .with_vibrato(4.8, 8.0, 0.6)
                .with_glide(0.12)
                .with_env(0.25, 4.0)
                .with_echo(0.25)
                .with_reverb(0.4),
            // pad: a seven-saw stack, slow bloom (its lane lowpass is what
            // the ramps open), deep in the hall
            Voice::stack(Wave::Sawtooth, 0.0, 10.0, 0.9, 7)
                .with_env(1.2, 6.0)
                .with_reverb(0.55),
            // arp: a square pulse with a short wow, a dotted echo to the
            // right (its lane lowpass is what the ramps open)
            Voice::panned(Wave::Square, 0.25)
                .with_filter(900.0, 2400.0, 0.0, 0.1, 1.4)
                .with_env(0.005, 0.6)
                .with_echo(0.4)
                .with_reverb(0.15),
            // keys: a picked guitar, left, far away
            Voice::panned(Wave::Guitar, -0.35)
                .with_env(0.005, 6.0)
                .with_echo(0.3)
                .with_reverb(0.5),
        )
        .intensity(0.75)
        .sidechain(Sidechain::new(0.35, 1.0))
        .echo(Echo::new(3.0, 0.4, 2400.0))
        .humanize(0.003)
        .sweep(0.0)
        .melodic_gain(1.1)
        .arrange([
            dawn(),
            pulse_in(),
            rise(),
            open(),
            peak(),
            fall(),
            tide(),
            gone(),
        ])
        .build()
}

pub fn spec() -> SongSpec {
    static SPEC: OnceLock<SongSpec> = OnceLock::new();
    *SPEC.get_or_init(build)
}
