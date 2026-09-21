//! "Deep Static" (AGGRESSIVE): menacing deep-floor pressure. E
//! Phrygian-dominant, relentless saw sub-bass in 16ths, dissonant stabs.
//!
//! `const` section literals against the full v2 format (`songs.rs`): ties,
//! velocity / chord lanes, stereo voices, echo + hall sends.

use super::Drum::{Hat, Kick, Silent, Snare};
use super::{
    Echo, Section, Sidechain, SongSpec, Voice, Wave, PERC_RIDE, PERC_RIDE_VEL, PHRYGIAN_DOMINANT,
    REST,
};

const DEEP_INTRO: Section = Section {
    label: "intro",
    bass: &[
        0, REST, REST, REST, 0, REST, REST, REST, 0, REST, REST, REST, 4, REST, 1, 0,
    ],
    lead: &[
        REST, REST, REST, REST, REST, REST, 7, REST, REST, REST, REST, REST, REST, REST, 8, REST,
        REST, REST, REST, REST, REST, REST, 7, REST, REST, REST, REST, REST, REST, REST, 11, REST,
    ],
    pad: &[
        0, REST, REST, REST, 1, REST, REST, REST, 0, REST, REST, REST, 4, REST, REST, REST,
    ],
    arp: &[
        REST, 14, 15, REST, 14, REST, 18, REST, REST, 14, 15, REST, 18, REST, 15, 14, REST, 14, 15,
        REST, 14, REST, 18, REST, REST, 14, 15, REST, 18, REST, 15, 14,
    ],
    drums: &[
        Kick, Silent, Kick, Silent, Snare, Silent, Kick, Silent, Kick, Silent, Kick, Silent, Snare,
        Silent, Kick, Silent,
    ],
    ..Section::EMPTY
};
const DEEP_VERSE: Section = Section {
    label: "verse",
    bass: &[
        0, 0, 0, REST, 1, REST, 0, REST, 0, 0, 0, REST, 4, REST, 1, 0,
    ],
    lead: &[
        REST, REST, 7, REST, 8, REST, REST, 7, REST, 11, REST, REST, 8, REST, 7, REST, REST, REST,
        8, REST, 7, REST, REST, 8, REST, 11, REST, REST, 7, REST, 8, REST,
    ],
    pad: &[
        0, REST, REST, REST, 1, REST, REST, REST, 0, REST, REST, REST, 4, REST, REST, REST,
    ],
    arp: &[
        REST, 14, 15, REST, 14, REST, 18, REST, REST, 14, 15, REST, 18, REST, 15, 14, 14, REST, 15,
        REST, 18, REST, 15, REST, 14, REST, 18, REST, 21, REST, 18, 15,
    ],
    drums: &[
        Kick, Silent, Kick, Silent, Snare, Silent, Kick, Kick, Kick, Silent, Kick, Silent, Snare,
        Hat, Kick, Snare,
    ],
    ..Section::EMPTY
};
const DEEP_REFRAIN: Section = Section {
    label: "refrain",
    bass: &[0, 0, 0, 0, 1, 1, 0, 0, 0, 0, 0, 0, 4, 4, 1, 0],
    lead: &[
        11, REST, 8, REST, 7, REST, 8, REST, 11, REST, 12, REST, 11, REST, 8, REST, 14, REST, 11,
        REST, 8, REST, 7, REST, 8, REST, 11, REST, 14, REST, 11, REST,
    ],
    pad: &[
        4, REST, REST, REST, 1, REST, REST, REST, 0, REST, REST, REST, 4, REST, REST, REST,
    ],
    arp: &[
        14, 15, 18, 15, 14, 15, 18, 21, 14, 15, 18, 15, 18, 15, 14, 11, 18, 15, 14, 15, 18, 21, 18,
        15, 14, 15, 18, 21, 22, 21, 18, 15,
    ],
    drums: &[
        Kick, Kick, Kick, Snare, Snare, Kick, Kick, Kick, Kick, Kick, Kick, Snare, Snare, Kick,
        Kick, Snare,
    ],
    perc: PERC_RIDE,
    perc_vel: PERC_RIDE_VEL,
    ..Section::EMPTY
};
const DEEP_BRIDGE: Section = Section {
    label: "bridge",
    bass: &[
        4, REST, REST, REST, 4, REST, REST, REST, 1, REST, REST, REST, 1, REST, REST, REST,
    ],
    lead: &[
        REST, REST, 8, REST, 7, REST, REST, REST, REST, REST, 11, REST, 8, REST, REST, REST, REST,
        REST, 7, REST, 8, REST, REST, REST, REST, REST, 11, REST, 12, REST, REST, REST,
    ],
    pad: &[
        4, REST, REST, REST, REST, REST, REST, REST, 1, REST, REST, REST, REST, REST, REST, REST,
    ],
    arp: &[
        REST, 14, 15, REST, 14, REST, 18, REST, REST, 14, 15, REST, 18, REST, 15, 14, 18, REST, 15,
        REST, 14, REST, 11, REST, 8, REST, 7, REST, 4, REST, 1, REST,
    ],
    drums: &[
        Kick, Silent, Silent, Silent, Snare, Silent, Kick, Silent, Kick, Silent, Silent, Silent,
        Snare, Silent, Kick, Kick,
    ],
    ..Section::EMPTY
};

pub const DEEP_STATIC: SongSpec = SongSpec {
    name: "Deep Static",
    root: 41.20, // E1
    scale: PHRYGIAN_DOMINANT,
    bpm: 144.0,
    steps_per_beat: 4,
    voices: [
        // bass: a relentless saw, tight and heavily driven, a sine sub under it
        Voice::mono(Wave::Sawtooth)
            .with_filter(180.0, 1200.0, 0.0, 0.07, 5.0)
            .with_drive(0.55)
            .with_sub(0.4),
        // lead: a three-saw stack with a screaming resonant wow
        Voice::stack(Wave::Sawtooth, 0.2, 10.0, 0.5, 3)
            .with_filter(1000.0, 5000.0, 0.0, 0.12, 3.0)
            .with_vibrato(6.0, 16.0, 0.2)
            .with_echo(0.25)
            .with_drive(0.35),
        // pad: a dark, wide supersaw pressure bed
        Voice::stack(Wave::Sawtooth, 0.0, 14.0, 0.9, 5)
            .with_filter(400.0, 1800.0, 0.6, 0.0, 1.4)
            .with_reverb(0.45),
        // arp: dissonant square stabs, a little grit, echoes
        Voice::panned(Wave::Square, -0.3)
            .with_echo(0.35)
            .with_drive(0.2),
        Voice::mono(Wave::Square), // keys: unused
    ],
    sections: &[
        DEEP_INTRO,
        DEEP_VERSE,
        DEEP_VERSE,
        DEEP_REFRAIN,
        DEEP_VERSE,
        DEEP_BRIDGE,
        DEEP_REFRAIN,
        DEEP_REFRAIN,
    ],
    intensity: 1.0,
    swing: 0.0,
    sidechain: Sidechain::new(0.55, 0.6),
    echo: Echo::DOTTED,
    humanize: 0.0,
    sweep: 1.0,
    // Mixed with the lane panners in place: no centre make-up.
    melodic_gain: 1.0,
};

/// The song (plain `const` data).
pub fn spec() -> SongSpec {
    DEEP_STATIC
}
