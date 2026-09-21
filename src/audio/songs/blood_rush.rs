//! "Blood Rush" (AGGRESSIVE): feverish, blood-in-the-eyes rush. F#
//! harmonic minor, jagged saw bass, wailing square lead over a stabbing arp.
//!
//! `const` section literals against the full v2 format (`songs.rs`): ties,
//! velocity / chord lanes, stereo voices, echo + hall sends.

use super::Drum::{Hat, Kick, Silent, Snare};
use super::{
    Echo, Section, Sidechain, SongSpec, Voice, Wave, HARMONIC_MINOR, PERC_RIDE, PERC_RIDE_VEL, REST,
};

const BLOOD_INTRO: Section = Section {
    label: "intro",
    bass: &[
        0, REST, REST, REST, 0, REST, REST, REST, 0, REST, REST, REST, 4, REST, 6, REST,
    ],
    lead: &[
        REST, REST, REST, REST, REST, REST, 11, REST, REST, REST, REST, REST, REST, REST, 12, REST,
        REST, REST, REST, REST, REST, REST, 14, REST, REST, REST, REST, REST, REST, REST, 11, REST,
    ],
    pad: &[
        0, REST, REST, REST, REST, REST, REST, REST, 4, REST, REST, REST, REST, REST, REST, REST,
    ],
    arp: &[
        7, 9, 11, 9, 7, 9, 11, 14, 7, 9, 11, 9, 11, 9, 7, 4, 7, 9, 11, 9, 7, 9, 11, 14, 7, 9, 11,
        9, 11, 9, 7, 4,
    ],
    drums: &[
        Kick, Hat, Snare, Hat, Kick, Silent, Snare, Hat, Kick, Hat, Snare, Hat, Kick, Silent,
        Snare, Hat,
    ],
    ..Section::EMPTY
};
const BLOOD_VERSE: Section = Section {
    label: "verse",
    bass: &[
        0, 0, REST, 0, 6, REST, 0, 0, 0, 0, REST, 0, 4, REST, 6, REST,
    ],
    lead: &[
        11, REST, 12, 11, 9, REST, 11, REST, 12, REST, 14, 12, 11, 9, 11, REST, 12, REST, 14, 12,
        11, REST, 12, REST, 14, REST, 16, 14, 12, 11, 9, REST,
    ],
    pad: &[
        0, REST, REST, REST, REST, REST, REST, REST, 4, REST, REST, REST, 6, REST, REST, REST,
    ],
    arp: &[
        7, 9, 11, 9, 7, 9, 11, 14, 7, 9, 11, 9, 11, 9, 7, 4, 9, 11, 14, 11, 9, 11, 14, 16, 9, 11,
        14, 11, 14, 11, 9, 7,
    ],
    drums: &[
        Kick, Hat, Snare, Hat, Kick, Kick, Snare, Hat, Kick, Hat, Snare, Hat, Kick, Snare, Snare,
        Hat,
    ],
    ..Section::EMPTY
};
const BLOOD_REFRAIN: Section = Section {
    label: "refrain",
    bass: &[0, 0, 0, 0, 6, 6, 0, 0, 0, 0, 0, 0, 4, 4, 6, 6],
    lead: &[
        14, REST, 16, 14, 12, REST, 14, REST, 16, REST, 18, 16, 14, 12, 11, REST, 16, REST, 18, 16,
        14, REST, 16, REST, 18, REST, 19, 18, 16, 14, 12, REST,
    ],
    pad: &[
        4, REST, REST, REST, REST, REST, REST, REST, 6, REST, REST, REST, REST, REST, REST, REST,
    ],
    arp: &[
        11, 14, 16, 14, 11, 14, 16, 19, 11, 14, 16, 14, 16, 14, 11, 7, 14, 16, 19, 16, 14, 16, 19,
        21, 14, 16, 19, 16, 19, 16, 14, 11,
    ],
    drums: &[
        Kick, Kick, Snare, Hat, Kick, Kick, Snare, Kick, Kick, Kick, Snare, Hat, Kick, Snare,
        Snare, Snare,
    ],
    perc: PERC_RIDE,
    perc_vel: PERC_RIDE_VEL,
    ..Section::EMPTY
};
const BLOOD_BRIDGE: Section = Section {
    label: "bridge",
    bass: &[
        4, REST, REST, REST, 4, REST, REST, REST, 6, REST, REST, REST, 6, REST, REST, REST,
    ],
    lead: &[
        REST, REST, 12, 11, 9, REST, REST, REST, REST, REST, 11, 9, 7, REST, REST, REST, REST,
        REST, 14, 12, 11, REST, REST, REST, REST, REST, 12, 11, 9, REST, REST, REST,
    ],
    pad: &[
        4, REST, REST, REST, REST, REST, REST, REST, 6, REST, REST, REST, REST, REST, REST, REST,
    ],
    arp: &[
        7, 9, 11, 9, 7, 9, 11, 14, 7, 9, 11, 9, 11, 9, 7, 4, 11, 9, 7, 9, 11, 14, 11, 9, 7, 9, 11,
        9, 7, 4, 2, 0,
    ],
    drums: &[
        Kick, Silent, Snare, Silent, Kick, Silent, Snare, Silent, Kick, Hat, Snare, Hat, Kick, Hat,
        Snare, Hat,
    ],
    ..Section::EMPTY
};

pub const BLOOD_RUSH: SongSpec = SongSpec {
    name: "Blood Rush",
    root: 46.25, // F#1
    scale: HARMONIC_MINOR,
    bpm: 140.0,
    steps_per_beat: 4,
    voices: [
        // bass: a jagged saw, sharp pluck, driven hard
        Voice::mono(Wave::Sawtooth)
            .with_filter(240.0, 1800.0, 0.0, 0.08, 4.5)
            .with_drive(0.45),
        // lead: three squares wailing — wide vibrato, wow, drive, echo
        Voice::stack(Wave::Square, 0.2, 9.0, 0.5, 3)
            .with_filter(1400.0, 6000.0, 0.0, 0.14, 2.5)
            .with_vibrato(6.5, 18.0, 0.15)
            .with_echo(0.3)
            .with_drive(0.3),
        // pad: a supersaw bed with a quick half-second bloom
        Voice::stack(Wave::Sawtooth, 0.0, 12.0, 0.9, 5)
            .with_filter(550.0, 2400.0, 0.5, 0.0, 1.3)
            .with_reverb(0.4),
        // arp: a stabbing saw with echoes
        Voice::panned(Wave::Sawtooth, -0.3).with_echo(0.4),
        Voice::mono(Wave::Square), // keys: unused
    ],
    sections: &[
        BLOOD_INTRO,
        BLOOD_VERSE,
        BLOOD_VERSE,
        BLOOD_REFRAIN,
        BLOOD_VERSE,
        BLOOD_BRIDGE,
        BLOOD_REFRAIN,
        BLOOD_REFRAIN,
    ],
    intensity: 0.95,
    swing: 0.0,
    sidechain: Sidechain::new(0.5, 0.6),
    echo: Echo::DOTTED,
    humanize: 0.0,
    sweep: 1.0,
    // Mixed with the lane panners in place: no centre make-up.
    melodic_gain: 1.0,
};

/// The song (plain `const` data).
pub fn spec() -> SongSpec {
    BLOOD_RUSH
}
