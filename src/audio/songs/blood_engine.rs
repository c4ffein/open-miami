//! "Blood Engine" (AGGRESSIVE / darksynth): the chase. E harmonic
//! minor, i–VI–iv–V in power chords, 126 bpm. A driven saw bass hammering
//! sixteenths with octave jumps, KEYS power-chord stabs with a resonant wow,
//! a three-square lead wailing over a five-saw fifths pad, octave arps in
//! dotted-eighth echoes, double-kick refrains, a tom fill into the drop.
//!
//! `const` section literals against the full v2 format (`songs.rs`): ties,
//! velocity / chord lanes, stereo voices, echo + hall sends.

use super::Drum::{Clap, Crash, Hat, Kick, OpenHat, Silent, Snare, Tom};
use super::{
    Chord, Drum, Echo, Section, Sidechain, SongSpec, Voice, Wave, HARMONIC_MINOR, HOLD, REST,
};

/// The bass per bar: root / octave hammer in sixteenths, one bar per chord
/// (E, C, A, B — the C, A and B below the root).
const ENGINE_BASS: &[i32] = &[
    0, 0, 7, 0, 0, 0, 7, 0, 0, 7, 0, 0, 0, 0, 7, 7, -2, -2, 5, -2, -2, -2, 5, -2, -2, 5, -2, -2,
    -2, -2, 5, 5, -4, -4, 3, -4, -4, -4, 3, -4, -4, 3, -4, -4, -4, -4, 3, 3, -3, -3, 4, -3, -3, -3,
    4, -3, -3, 4, -3, -3, -3, -3, 4, 4,
];
const ENGINE_BASS_VEL: &[u8] = &[9, 6, 7, 6, 9, 6, 7, 6, 9, 7, 6, 6, 9, 6, 8, 8];
/// Off-beat power-chord stabs on the chord roots (E3, C3, A3, B3).
const ENGINE_KEYS: &[i32] = &[
    REST, REST, 7, REST, REST, 7, REST, REST, 7, REST, REST, 7, REST, REST, 7, REST, REST, REST, 5,
    REST, REST, 5, REST, REST, 5, REST, REST, 5, REST, REST, 5, REST, REST, REST, 10, REST, REST,
    10, REST, REST, 10, REST, REST, 10, REST, REST, 10, REST, REST, REST, 11, REST, REST, 11, REST,
    REST, 11, REST, REST, 11, REST, REST, 11, REST,
];
const ENGINE_KEYS_VEL: &[u8] = &[0, 0, 9, 0, 0, 7, 0, 0, 8, 0, 0, 7, 0, 0, 9, 0];
/// A bar of fifths per chord, struck once and held twelve steps.
const ENGINE_PAD: &[i32] = &[
    7, HOLD, HOLD, HOLD, HOLD, HOLD, HOLD, HOLD, HOLD, HOLD, HOLD, HOLD, REST, REST, REST, REST, 5,
    HOLD, HOLD, HOLD, HOLD, HOLD, HOLD, HOLD, HOLD, HOLD, HOLD, HOLD, REST, REST, REST, REST, 10,
    HOLD, HOLD, HOLD, HOLD, HOLD, HOLD, HOLD, HOLD, HOLD, HOLD, HOLD, REST, REST, REST, REST, 11,
    HOLD, HOLD, HOLD, HOLD, HOLD, HOLD, HOLD, HOLD, HOLD, HOLD, HOLD, REST, REST, REST, REST,
];
const ENGINE_PAD_CHORDS: &[Chord] = &[Chord::Power];
/// Root / octave / fifth / octave arps on each chord (E4, C4, A4, B4).
const ENGINE_ARP: &[i32] = &[
    14, 21, 18, 21, 14, 21, 18, 21, 14, 21, 18, 21, 14, 21, 18, 21, 12, 19, 16, 19, 12, 19, 16, 19,
    12, 19, 16, 19, 12, 19, 16, 19, 17, 24, 21, 24, 17, 24, 21, 24, 17, 24, 21, 24, 17, 24, 21, 24,
    18, 25, 22, 25, 18, 25, 22, 25, 18, 25, 22, 25, 18, 25, 22, 25,
];
const ENGINE_ARP_VEL: &[u8] = &[9, 5, 7, 5];
const ENGINE_DRUMS: &[Drum] = &[
    Kick, Silent, Silent, Silent, Snare, Silent, Silent, Silent, Kick, Silent, Kick, Silent, Snare,
    Silent, Silent, Silent,
];
const ENGINE_DRUMS_DOUBLE: &[Drum] = &[
    Kick, Silent, Silent, Kick, Snare, Silent, Silent, Silent, Kick, Kick, Silent, Silent, Snare,
    Silent, Silent, Kick,
];
/// Sixteenth hats with claps under the snares and an open hat into the bar.
const ENGINE_PERC: &[Drum] = &[
    Hat, Hat, Hat, Hat, Clap, Hat, Hat, Hat, Hat, Hat, Hat, Hat, Clap, Hat, OpenHat, Hat,
];
const ENGINE_PERC_VEL: &[u8] = &[7, 3, 5, 3, 9, 3, 5, 3, 7, 3, 5, 3, 9, 3, 7, 3];

const ENGINE_INTRO: Section = Section {
    label: "intro",
    pad: ENGINE_PAD,
    pad_chord: ENGINE_PAD_CHORDS,
    arp: ENGINE_ARP,
    arp_vel: &[7, 3, 5, 3],
    perc: &[
        Crash, Silent, Silent, Silent, Silent, Silent, Silent, Silent, Silent, Silent, Silent,
        Silent, Silent, Silent, Silent, Silent, Silent, Silent, Silent, Silent, Silent, Silent,
        Silent, Silent, Silent, Silent, Silent, Silent, Silent, Silent, Silent, Silent, Silent,
        Silent, Silent, Silent, Silent, Silent, Silent, Silent, Silent, Silent, Silent, Silent,
        Silent, Silent, Silent, Silent, Hat, Silent, Hat, Silent, Hat, Silent, Hat, Silent, Hat,
        Hat, Hat, Hat, Hat, Hat, Hat, Hat,
    ],
    perc_vel: &[
        8, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 4, 0, 4, 0, 5, 0, 5, 0, 5, 4, 6, 5,
        7, 6, 8, 8,
    ],
    ..Section::EMPTY
};
const ENGINE_VERSE: Section = Section {
    label: "verse",
    bass: ENGINE_BASS,
    bass_vel: ENGINE_BASS_VEL,
    keys: ENGINE_KEYS,
    keys_vel: ENGINE_KEYS_VEL,
    keys_chord: &[Chord::Power],
    pad: ENGINE_PAD,
    pad_chord: ENGINE_PAD_CHORDS,
    drums: ENGINE_DRUMS,
    perc: ENGINE_PERC,
    perc_vel: ENGINE_PERC_VEL,
    ..Section::EMPTY
};
const ENGINE_REFRAIN: Section = Section {
    label: "refrain",
    bass: ENGINE_BASS,
    bass_vel: ENGINE_BASS_VEL,
    lead: &[
        14, HOLD, HOLD, HOLD, HOLD, HOLD, 13, HOLD, 12, HOLD, HOLD, HOLD, 11, HOLD, HOLD, HOLD, 12,
        HOLD, HOLD, HOLD, HOLD, HOLD, 11, HOLD, 9, HOLD, HOLD, HOLD, HOLD, HOLD, HOLD, HOLD, 11,
        HOLD, HOLD, HOLD, 12, HOLD, HOLD, HOLD, 13, HOLD, HOLD, HOLD, HOLD, HOLD, HOLD, HOLD, 14,
        HOLD, HOLD, HOLD, HOLD, HOLD, HOLD, HOLD, 16, HOLD, HOLD, HOLD, 13, HOLD, HOLD, HOLD,
    ],
    lead_vel: &[9, 9, 9, 9, 9, 9, 7, 9, 8, 9, 9, 9, 7, 9, 9, 9],
    keys: ENGINE_KEYS,
    keys_vel: ENGINE_KEYS_VEL,
    keys_chord: &[Chord::Power],
    pad: ENGINE_PAD,
    pad_chord: ENGINE_PAD_CHORDS,
    arp: ENGINE_ARP,
    arp_vel: ENGINE_ARP_VEL,
    drums: ENGINE_DRUMS_DOUBLE,
    perc: ENGINE_PERC,
    perc_vel: ENGINE_PERC_VEL,
    ..Section::EMPTY
};
const ENGINE_BREAK: Section = Section {
    label: "break",
    bass: &[
        0, HOLD, HOLD, HOLD, HOLD, HOLD, HOLD, HOLD, HOLD, HOLD, HOLD, HOLD, HOLD, HOLD, HOLD,
        HOLD, -2, HOLD, HOLD, HOLD, HOLD, HOLD, HOLD, HOLD, HOLD, HOLD, HOLD, HOLD, HOLD, HOLD,
        HOLD, HOLD,
    ],
    lead: &[
        REST, REST, REST, REST, 7, HOLD, HOLD, HOLD, 9, HOLD, HOLD, HOLD, 11, HOLD, HOLD, HOLD, 12,
        HOLD, HOLD, HOLD, HOLD, HOLD, HOLD, HOLD, 13, HOLD, HOLD, HOLD, HOLD, HOLD, HOLD, HOLD,
    ],
    lead_vel: &[7],
    pad: &[
        7, HOLD, HOLD, HOLD, HOLD, HOLD, HOLD, HOLD, HOLD, HOLD, HOLD, HOLD, HOLD, HOLD, HOLD,
        HOLD, 5, HOLD, HOLD, HOLD, HOLD, HOLD, HOLD, HOLD, HOLD, HOLD, HOLD, HOLD, HOLD, HOLD,
        HOLD, HOLD,
    ],
    pad_chord: &[Chord::Sus2],
    drums: &[
        Kick, Silent, Silent, Silent, Silent, Silent, Silent, Silent, Kick, Silent, Silent, Silent,
        Silent, Silent, Silent, Silent, Kick, Silent, Silent, Silent, Tom, Silent, Tom, Silent,
        Tom, Silent, Tom, Tom, Snare, Snare, Snare, Snare,
    ],
    drums_vel: &[
        8, 0, 0, 0, 0, 0, 0, 0, 7, 0, 0, 0, 0, 0, 0, 0, 8, 0, 0, 0, 7, 0, 7, 0, 8, 0, 8, 8, 6, 7,
        8, 9,
    ],
    perc: &[
        Crash, Silent, Silent, Silent, Silent, Silent, Silent, Silent, Silent, Silent, Silent,
        Silent, Silent, Silent, Silent, Silent, Silent, Silent, Silent, Silent, Silent, Silent,
        Silent, Silent, Silent, Silent, Silent, Silent, Silent, Silent, Silent, Silent,
    ],
    perc_vel: &[7],
    ..Section::EMPTY
};

pub const BLOOD_ENGINE: SongSpec = SongSpec {
    name: "Blood Engine",
    root: 82.41, // E2
    scale: HARMONIC_MINOR,
    bpm: 126.0,
    steps_per_beat: 4,
    voices: [
        // bass: a driven saw with a fast resonant snap and a sub under it
        Voice::mono(Wave::Sawtooth)
            .with_filter(200.0, 1500.0, 0.0, 0.07, 4.5)
            .with_drive(0.55)
            .with_sub(0.4),
        // lead: three squares, wide vibrato, a resonant wow, drive, echo,
        // sliding between the tied phrase notes
        Voice::stack(Wave::Square, 0.2, 9.0, 0.5, 3)
            .with_filter(1300.0, 6000.0, 0.0, 0.16, 2.8)
            .with_glide(0.06)
            .with_vibrato(6.0, 18.0, 0.2)
            .with_echo(0.3)
            .with_reverb(0.2)
            .with_drive(0.35),
        // pad: five saws in fifths, blooming over 0.6 s, back in the hall
        Voice::stack(Wave::Sawtooth, 0.0, 14.0, 0.9, 5)
            .with_filter(450.0, 2200.0, 0.6, 0.0, 1.3)
            .with_reverb(0.5),
        // arp: a square in dotted-eighth echoes, left
        Voice::panned(Wave::Square, -0.35).with_echo(0.45),
        // keys: short power-chord stabs — three saws, a big wow, crushed
        Voice::stack(Wave::Sawtooth, 0.15, 10.0, 0.6, 3)
            .with_env(0.003, 0.5)
            .with_filter(500.0, 3500.0, 0.0, 0.15, 2.5)
            .with_drive(0.5)
            .with_echo(0.15),
    ],
    sections: &[
        ENGINE_INTRO,
        ENGINE_VERSE,
        ENGINE_REFRAIN,
        ENGINE_VERSE,
        ENGINE_REFRAIN,
        ENGINE_BREAK,
        ENGINE_REFRAIN,
        ENGINE_REFRAIN,
    ],
    intensity: 1.05,
    swing: 0.0,
    sidechain: Sidechain::new(0.45, 0.7),
    echo: Echo::new(3.0, 0.3, 2400.0),
    humanize: 0.003,
    sweep: 0.5,
    // Mixed with the lane panners in place: no centre make-up.
    melodic_gain: 1.0,
};

/// The song (plain `const` data).
pub fn spec() -> SongSpec {
    BLOOD_ENGINE
}
