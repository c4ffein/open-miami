//! "Descent" (AGGRESSIVE): tense mid-descent. D Phrygian (flat 2nd),
//! driving square bass hammering the root, restless saw arp, four-on-the-floor.
//!
//! `const` section literals against the full v2 format (`songs.rs`): ties,
//! velocity / chord lanes, stereo voices, echo + hall sends.

use super::Drum::{Hat, Kick, Silent, Snare};
use super::{
    Echo, Section, Sidechain, SongSpec, Voice, Wave, PERC_RIDE, PERC_RIDE_VEL, PHRYGIAN, REST,
};

const DESCENT_INTRO: Section = Section {
    label: "intro",
    bass: &[
        0, REST, REST, REST, 0, REST, REST, REST, 0, REST, REST, REST, 0, REST, REST, REST,
    ],
    lead: &[
        REST, REST, REST, REST, REST, REST, REST, REST, 7, REST, 8, REST, 10, REST, 8, REST, REST,
        REST, REST, REST, REST, REST, REST, REST, 7, REST, 10, REST, 8, REST, 7, REST,
    ],
    pad: &[
        0, REST, REST, REST, REST, REST, REST, REST, 5, REST, REST, REST, REST, REST, REST, REST,
    ],
    arp: &[
        14, REST, 15, REST, 17, REST, 15, REST, 14, REST, 17, REST, 19, REST, 17, REST, 14, REST,
        15, REST, 17, REST, 15, REST, 14, REST, 17, REST, 19, REST, 17, REST,
    ],
    drums: &[
        Kick, Silent, Hat, Silent, Snare, Silent, Hat, Silent, Kick, Silent, Hat, Silent, Snare,
        Silent, Hat, Silent,
    ],
    ..Section::EMPTY
};
const DESCENT_VERSE: Section = Section {
    label: "verse",
    bass: &[
        0, REST, 0, REST, 0, REST, 0, REST, 0, REST, 0, REST, 5, REST, 4, REST,
    ],
    lead: &[
        7, 8, 10, 8, 7, 10, 8, 10, 12, 11, 10, 8, 7, 8, 7, REST, 7, 8, 10, 8, 10, 11, 12, 10, 8,
        10, 12, 11, 10, 8, 7, REST,
    ],
    pad: &[
        0, REST, REST, REST, REST, REST, REST, REST, 5, REST, REST, REST, 4, REST, REST, REST,
    ],
    arp: &[
        14, REST, 15, REST, 17, REST, 15, REST, 14, REST, 17, REST, 19, REST, 17, REST, 14, REST,
        17, REST, 19, REST, 17, REST, 15, REST, 17, REST, 15, REST, 14, REST,
    ],
    drums: &[
        Kick, Hat, Hat, Hat, Snare, Hat, Hat, Hat, Kick, Hat, Kick, Hat, Snare, Hat, Hat, Hat,
    ],
    ..Section::EMPTY
};
const DESCENT_REFRAIN: Section = Section {
    label: "refrain",
    bass: &[
        0, 0, REST, 0, 0, 0, REST, 0, 0, 0, REST, 0, 5, REST, 4, REST,
    ],
    lead: &[
        12, REST, 11, REST, 10, REST, 8, REST, 7, REST, 8, REST, 10, REST, 12, REST, 14, REST, 12,
        REST, 11, REST, 10, REST, 8, REST, 10, REST, 12, REST, 14, REST,
    ],
    pad: &[
        5, REST, REST, REST, REST, REST, REST, REST, 4, REST, REST, REST, REST, REST, REST, REST,
    ],
    arp: &[
        17, REST, 19, REST, 21, REST, 19, REST, 17, REST, 19, REST, 22, REST, 19, REST, 21, REST,
        22, REST, 24, REST, 22, REST, 19, REST, 17, REST, 15, REST, 14, REST,
    ],
    drums: &[
        Kick, Hat, Snare, Hat, Kick, Hat, Snare, Hat, Kick, Kick, Snare, Hat, Kick, Snare, Snare,
        Hat,
    ],
    perc: PERC_RIDE,
    perc_vel: PERC_RIDE_VEL,
    ..Section::EMPTY
};
const DESCENT_BRIDGE: Section = Section {
    label: "bridge",
    bass: &[
        5, REST, REST, REST, 5, REST, REST, REST, 4, REST, REST, REST, 4, REST, REST, REST,
    ],
    lead: &[
        REST, REST, 10, REST, 8, REST, 7, REST, REST, REST, REST, REST, REST, REST, REST, REST,
        REST, REST, 12, REST, 10, REST, 8, REST, REST, REST, REST, REST, REST, REST, REST, REST,
    ],
    pad: &[
        5, REST, REST, REST, REST, REST, REST, REST, 4, REST, REST, REST, REST, REST, REST, REST,
    ],
    arp: &[
        14, REST, 15, REST, 17, REST, 15, REST, 14, REST, 15, REST, 17, REST, 19, REST, 17, REST,
        15, REST, 14, REST, 12, REST, 10, REST, 8, REST, 7, REST, 5, REST,
    ],
    drums: &[
        Kick, Silent, Hat, Silent, Snare, Silent, Hat, Silent, Kick, Silent, Hat, Silent, Snare,
        Silent, Hat, Hat,
    ],
    ..Section::EMPTY
};

pub const DESCENT: SongSpec = SongSpec {
    name: "Descent",
    root: 36.71, // D1
    scale: PHRYGIAN,
    bpm: 132.0,
    steps_per_beat: 4,
    voices: [
        // bass: a hammering square, tight resonant pluck, driven
        Voice::mono(Wave::Square)
            .with_filter(220.0, 1600.0, 0.0, 0.09, 4.0)
            .with_drive(0.35),
        // lead: a doubled saw, fast wow, nervous vibrato, a little drive
        Voice::wide(Wave::Sawtooth, 0.2, 9.0, 0.4)
            .with_filter(1200.0, 5500.0, 0.0, 0.15, 2.5)
            .with_vibrato(6.0, 14.0, 0.2)
            .with_echo(0.3)
            .with_drive(0.2),
        // pad: a wide supersaw, darker, opening over ~0.7 s
        Voice::stack(Wave::Sawtooth, 0.0, 12.0, 0.9, 5)
            .with_filter(500.0, 2000.0, 0.7, 0.0, 1.2)
            .with_reverb(0.45),
        // arp: a restless saw with echoes
        Voice::panned(Wave::Sawtooth, -0.3).with_echo(0.35),
        Voice::mono(Wave::Square), // keys: unused
    ],
    sections: &[
        DESCENT_INTRO,
        DESCENT_VERSE,
        DESCENT_VERSE,
        DESCENT_REFRAIN,
        DESCENT_VERSE,
        DESCENT_BRIDGE,
        DESCENT_REFRAIN,
        DESCENT_REFRAIN,
    ],
    intensity: 0.85,
    swing: 0.0,
    sidechain: Sidechain::new(0.5, 0.7),
    echo: Echo::DOTTED,
    humanize: 0.0,
    sweep: 1.0,
    // Mixed with the lane panners in place: no centre make-up.
    melodic_gain: 1.0,
};

/// The song (plain `const` data).
pub fn spec() -> SongSpec {
    DESCENT
}
