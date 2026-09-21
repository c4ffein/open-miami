//! "Insert Coin" (WAVY): ominous, dreamy title theme. A-minor, slow,
//! soft triangle/sine voices, lush pad, sparse falling arp. The calm before it.
//!
//! `const` section literals against the full v2 format (`songs.rs`): ties,
//! velocity / chord lanes, stereo voices, echo + hall sends.

use super::Drum::{Hat, Kick, Silent, Snare};
use super::{Echo, Section, Sidechain, SongSpec, Voice, Wave, MINOR, REST};

const INSERT_INTRO: Section = Section {
    label: "intro",
    bass: &[
        0, REST, REST, REST, REST, REST, REST, REST, REST, REST, REST, REST, REST, REST, REST, REST,
    ],
    lead: &[
        REST, REST, REST, REST, REST, REST, 14, REST, REST, REST, REST, REST, 12, REST, REST, REST,
        REST, REST, REST, REST, REST, REST, 11, REST, REST, REST, REST, REST, 9, REST, REST, REST,
    ],
    pad: &[
        0, REST, REST, REST, REST, REST, REST, REST, REST, REST, REST, REST, REST, REST, REST, REST,
    ],
    arp: &[
        REST, REST, REST, REST, 7, REST, 9, REST, REST, REST, REST, REST, 11, REST, 9, REST, REST,
        REST, REST, REST, 9, REST, 11, REST, REST, REST, REST, REST, 12, REST, 9, REST,
    ],
    drums: &[
        Silent, Silent, Silent, Silent, Hat, Silent, Silent, Silent, Silent, Silent, Silent,
        Silent, Hat, Silent, Silent, Silent,
    ],
    ..Section::EMPTY
};
const INSERT_VERSE: Section = Section {
    label: "verse",
    bass: &[
        0, REST, REST, REST, REST, REST, REST, REST, 3, REST, REST, REST, REST, REST, REST, REST,
    ],
    lead: &[
        REST, REST, 14, REST, REST, REST, 12, REST, REST, REST, 11, REST, REST, REST, REST, REST,
        REST, REST, 12, REST, REST, REST, 10, REST, REST, REST, 9, REST, REST, REST, 7, REST,
    ],
    pad: &[
        7, REST, REST, REST, REST, REST, REST, REST, 10, REST, REST, REST, REST, REST, REST, REST,
    ],
    arp: &[
        REST, REST, REST, REST, 7, REST, 9, REST, REST, REST, REST, REST, 11, REST, 9, REST, REST,
        REST, REST, REST, 9, REST, 11, REST, REST, REST, REST, REST, 12, REST, 9, REST,
    ],
    drums: &[
        Silent, Silent, Silent, Silent, Hat, Silent, Silent, Silent, Silent, Silent, Silent,
        Silent, Hat, Silent, Silent, Silent,
    ],
    ..Section::EMPTY
};
const INSERT_REFRAIN: Section = Section {
    label: "refrain",
    bass: &[
        0, REST, REST, REST, 0, REST, 3, REST, 5, REST, REST, REST, 3, REST, 2, REST,
    ],
    lead: &[
        7, REST, 9, REST, 11, REST, 12, REST, REST, 14, REST, 12, 11, REST, 9, REST, 7, REST, 9,
        REST, 11, REST, 14, REST, REST, 16, REST, 14, 12, REST, 11, REST,
    ],
    pad: &[
        0, REST, REST, REST, REST, REST, REST, REST, 3, REST, REST, REST, REST, REST, REST, REST,
    ],
    arp: &[
        14, 16, 18, 16, 14, 16, 18, 21, 14, 16, 18, 16, 18, 16, 14, 11, 14, 16, 18, 21, 18, 16, 14,
        16, 18, 21, 23, 21, 18, 16, 14, 12,
    ],
    drums: &[
        Kick, Silent, Hat, Silent, Silent, Silent, Hat, Silent, Kick, Silent, Hat, Silent, Snare,
        Silent, Hat, Silent,
    ],
    ..Section::EMPTY
};
const INSERT_BRIDGE: Section = Section {
    label: "bridge",
    bass: &[
        5, REST, REST, REST, REST, REST, REST, REST, 4, REST, REST, REST, REST, REST, REST, REST,
    ],
    lead: &[
        REST, REST, REST, REST, REST, REST, REST, REST, 11, REST, REST, REST, 9, REST, 7, REST,
        REST, REST, REST, REST, REST, REST, REST, REST, 12, REST, REST, REST, 10, REST, 9, REST,
    ],
    pad: &[
        5, REST, REST, REST, REST, REST, REST, REST, 4, REST, REST, REST, REST, REST, REST, REST,
    ],
    arp: &[
        7, REST, 9, REST, 11, REST, 9, REST, 7, REST, 9, REST, 11, REST, 14, REST, 11, REST, 9,
        REST, 7, REST, 9, REST, 11, REST, 9, REST, 7, REST, 4, REST,
    ],
    drums: &[
        Silent, Silent, Silent, Silent, Silent, Silent, Silent, Silent, Hat, Silent, Silent,
        Silent, Silent, Silent, Silent, Silent,
    ],
    ..Section::EMPTY
};

pub const INSERT_COIN: SongSpec = SongSpec {
    name: "Insert Coin",
    root: 55.0, // A1
    scale: MINOR,
    bpm: 84.0,
    steps_per_beat: 4,
    voices: [
        // bass: a soft triangle sub
        Voice::mono(Wave::Triangle),
        // lead: a sine with a slow, late vibrato, echoing into the hall
        Voice::panned(Wave::Sine, 0.2)
            .with_vibrato(4.5, 8.0, 0.4)
            .with_echo(0.3)
            .with_reverb(0.35),
        // pad: three soft triangles blooming open over a second and a half
        Voice::stack(Wave::Triangle, 0.0, 7.0, 0.8, 3)
            .with_filter(600.0, 1800.0, 1.5, 0.0, 0.9)
            .with_reverb(0.5),
        // arp: a sine falling through dotted-eighth echoes
        Voice::panned(Wave::Sine, -0.3)
            .with_echo(0.4)
            .with_reverb(0.2),
        Voice::mono(Wave::Square), // keys: unused
    ],
    sections: &[
        INSERT_INTRO,
        INSERT_VERSE,
        INSERT_VERSE,
        INSERT_REFRAIN,
        INSERT_VERSE,
        INSERT_BRIDGE,
        INSERT_REFRAIN,
        INSERT_REFRAIN,
    ],
    intensity: 0.5,
    swing: 0.0,
    sidechain: Sidechain::OFF,
    echo: Echo::DOTTED,
    humanize: 0.006,
    sweep: 1.0,
    // Mixed with the lane panners in place: no centre make-up.
    melodic_gain: 1.0,
};

/// The song (plain `const` data).
pub fn spec() -> SongSpec {
    INSERT_COIN
}
