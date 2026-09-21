//! "Chrome Veins" (AGGRESSIVE): chromed, forward-leaning drive. B
//! Dorian, pulsing square bass, bright square arp, warm saw pad. City blur.
//!
//! `const` section literals against the full v2 format (`songs.rs`): ties,
//! velocity / chord lanes, stereo voices, echo + hall sends.

use super::Drum::{Hat, Kick, Silent, Snare};
use super::{
    Echo, Section, Sidechain, SongSpec, Voice, Wave, DORIAN, PERC_RIDE, PERC_RIDE_VEL, REST,
};

const CHROME_INTRO: Section = Section {
    label: "intro",
    bass: &[
        0, REST, REST, REST, 7, REST, REST, REST, 0, REST, REST, REST, 5, REST, 3, REST,
    ],
    lead: &[
        REST, REST, REST, REST, REST, REST, 7, REST, REST, REST, REST, REST, REST, REST, 9, REST,
        REST, REST, REST, REST, REST, REST, 11, REST, REST, REST, REST, REST, REST, REST, 7, REST,
    ],
    pad: &[
        0, REST, REST, REST, REST, REST, REST, REST, 4, REST, REST, REST, REST, REST, REST, REST,
    ],
    arp: &[
        14, 16, 18, 16, 14, 16, 18, 21, 14, 16, 18, 16, 18, 16, 14, 11, 14, 16, 18, 16, 14, 16, 18,
        21, 14, 16, 18, 16, 18, 16, 14, 11,
    ],
    drums: &[
        Kick, Silent, Hat, Silent, Silent, Silent, Hat, Silent, Kick, Silent, Hat, Silent, Silent,
        Silent, Hat, Silent,
    ],
    ..Section::EMPTY
};
const CHROME_VERSE: Section = Section {
    label: "verse",
    bass: &[
        0, REST, 0, REST, 7, REST, 0, REST, 0, REST, 0, REST, 5, REST, 3, REST,
    ],
    lead: &[
        REST, REST, 7, REST, 9, REST, 11, REST, REST, 12, REST, 11, 9, REST, 7, REST, REST, REST,
        7, REST, 9, REST, 12, REST, REST, 14, REST, 12, 11, REST, 9, REST,
    ],
    pad: &[
        0, REST, REST, REST, REST, REST, REST, REST, 4, REST, REST, REST, REST, REST, REST, REST,
    ],
    arp: &[
        14, 16, 18, 16, 14, 16, 18, 21, 14, 16, 18, 16, 18, 16, 14, 11, 14, 16, 18, 21, 18, 16, 14,
        16, 18, 21, 23, 21, 18, 16, 14, 11,
    ],
    drums: &[
        Kick, Silent, Hat, Silent, Snare, Silent, Hat, Silent, Kick, Silent, Hat, Kick, Snare,
        Silent, Hat, Silent,
    ],
    ..Section::EMPTY
};
const CHROME_REFRAIN: Section = Section {
    label: "refrain",
    bass: &[
        0, REST, 0, 7, 0, REST, 0, 7, 5, REST, 5, REST, 3, REST, 3, REST,
    ],
    lead: &[
        12, REST, 11, REST, 9, REST, 7, REST, 9, REST, 11, REST, 12, REST, 14, REST, 16, REST, 14,
        REST, 12, REST, 11, REST, 9, REST, 11, REST, 12, REST, 14, REST,
    ],
    pad: &[
        3, REST, REST, REST, REST, REST, REST, REST, 7, REST, REST, REST, REST, REST, REST, REST,
    ],
    arp: &[
        18, 16, 14, 16, 18, 21, 18, 16, 14, 16, 18, 21, 23, 21, 18, 16, 14, 16, 18, 21, 23, 21, 18,
        16, 18, 21, 23, 26, 23, 21, 18, 16,
    ],
    drums: &[
        Kick, Hat, Hat, Silent, Snare, Silent, Hat, Kick, Kick, Hat, Hat, Kick, Snare, Silent, Hat,
        Snare,
    ],
    perc: PERC_RIDE,
    perc_vel: PERC_RIDE_VEL,
    ..Section::EMPTY
};
const CHROME_BRIDGE: Section = Section {
    label: "bridge",
    bass: &[
        5, REST, REST, REST, 5, REST, REST, REST, 4, REST, REST, REST, 4, REST, REST, REST,
    ],
    lead: &[
        REST, REST, 12, REST, 11, REST, 9, REST, REST, REST, REST, REST, REST, REST, REST, REST,
        REST, REST, 14, REST, 12, REST, 11, REST, REST, REST, REST, REST, REST, REST, REST, REST,
    ],
    pad: &[
        5, REST, REST, REST, REST, REST, REST, REST, 4, REST, REST, REST, REST, REST, REST, REST,
    ],
    arp: &[
        14, REST, 16, REST, 18, REST, 16, REST, 14, REST, 16, REST, 18, REST, 21, REST, 18, REST,
        16, REST, 14, REST, 16, REST, 18, REST, 14, REST, 11, REST, 9, REST,
    ],
    drums: &[
        Kick, Silent, Silent, Silent, Snare, Silent, Silent, Silent, Kick, Silent, Silent, Silent,
        Snare, Silent, Hat, Silent,
    ],
    ..Section::EMPTY
};

pub const CHROME_VEINS: SongSpec = SongSpec {
    name: "Chrome Veins",
    root: 61.74, // B1
    scale: DORIAN,
    bpm: 118.0,
    steps_per_beat: 4,
    voices: [
        // bass: a square with a resonant pluck and a little grit
        Voice::mono(Wave::Square)
            .with_filter(260.0, 1400.0, 0.0, 0.1, 3.5)
            .with_drive(0.25),
        // lead: a doubled saw with a filter wow, vibrato and echo
        Voice::wide(Wave::Sawtooth, 0.2, 8.0, 0.4)
            .with_filter(1500.0, 5000.0, 0.0, 0.2, 2.0)
            .with_vibrato(5.5, 10.0, 0.25)
            .with_echo(0.3),
        // pad: a five-saw supersaw blooming over most of a second
        Voice::stack(Wave::Sawtooth, 0.0, 10.0, 0.85, 5)
            .with_filter(650.0, 2200.0, 0.8, 0.0, 1.1)
            .with_reverb(0.4),
        // arp: a bright square with dotted-eighth echoes
        Voice::panned(Wave::Square, -0.3).with_echo(0.35),
        Voice::mono(Wave::Square), // keys: unused
    ],
    sections: &[
        CHROME_INTRO,
        CHROME_VERSE,
        CHROME_VERSE,
        CHROME_REFRAIN,
        CHROME_VERSE,
        CHROME_BRIDGE,
        CHROME_REFRAIN,
        CHROME_REFRAIN,
    ],
    intensity: 0.72,
    swing: 0.0,
    sidechain: Sidechain::new(0.4, 0.8),
    echo: Echo::DOTTED,
    humanize: 0.0,
    sweep: 1.0,
    // Mixed with the lane panners in place: no centre make-up.
    melodic_gain: 1.0,
};

/// The song (plain `const` data).
pub fn spec() -> SongSpec {
    CHROME_VEINS
}
