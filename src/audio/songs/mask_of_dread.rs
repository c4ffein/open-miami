//! "Mask of Dread" (AGGRESSIVE / heavy BOSS): dread-filled and huge. C
//! Locrian (flat 2nd + tritone), slow but crushing; sustained saw bass lurching
//! to the tritone, high square wails, enormous slow kicks. The mask watches.
//!
//! `const` section literals against the full v2 format (`songs.rs`): ties,
//! velocity / chord lanes, stereo voices, echo + hall sends.

use super::Drum::{Kick, Silent, Snare};
use super::{
    Echo, Section, Sidechain, SongSpec, Voice, Wave, LOCRIAN, PERC_RIDE, PERC_RIDE_VEL, REST,
};

const MASK_INTRO: Section = Section {
    label: "intro",
    bass: &[
        0, REST, REST, REST, REST, REST, REST, REST, 0, REST, REST, REST, REST, REST, REST, REST,
    ],
    lead: &[
        7, REST, REST, REST, REST, REST, REST, REST, REST, REST, REST, REST, REST, REST, REST,
        REST, 8, REST, REST, REST, REST, REST, REST, REST, REST, REST, REST, REST, REST, REST,
        REST, REST,
    ],
    pad: &[
        0, REST, REST, REST, REST, REST, REST, REST, 4, REST, REST, REST, REST, REST, REST, REST,
    ],
    arp: &[
        REST, REST, 14, REST, 15, REST, REST, REST, REST, REST, 18, REST, 15, REST, 14, REST, REST,
        REST, 14, REST, 15, REST, REST, REST, REST, REST, 18, REST, 15, REST, 14, REST,
    ],
    drums: &[
        Kick, Silent, Silent, Silent, Snare, Silent, Silent, Silent, Kick, Silent, Silent, Silent,
        Snare, Silent, Silent, Silent,
    ],
    ..Section::EMPTY
};
const MASK_VERSE: Section = Section {
    label: "verse",
    bass: &[
        0, REST, REST, REST, 0, REST, 4, REST, 0, REST, REST, REST, 4, REST, 3, REST,
    ],
    lead: &[
        7, REST, REST, REST, REST, REST, REST, REST, 8, REST, REST, REST, REST, REST, 11, REST,
        REST, REST, REST, REST, 7, REST, REST, REST, 8, REST, REST, REST, REST, REST, 4, REST,
    ],
    pad: &[
        0, REST, REST, REST, REST, REST, REST, REST, 4, REST, REST, REST, REST, REST, REST, REST,
    ],
    arp: &[
        REST, REST, 14, REST, 15, REST, REST, REST, REST, REST, 18, REST, 15, REST, 14, REST, REST,
        REST, 15, REST, 18, REST, REST, REST, REST, REST, 14, REST, 11, REST, 14, REST,
    ],
    drums: &[
        Kick, Silent, Silent, Silent, Snare, Silent, Silent, Silent, Kick, Silent, Silent, Kick,
        Snare, Silent, Snare, Silent,
    ],
    ..Section::EMPTY
};
const MASK_REFRAIN: Section = Section {
    label: "refrain",
    bass: &[
        0, REST, 0, REST, 4, REST, 4, REST, 3, REST, 3, REST, 4, REST, 1, REST,
    ],
    lead: &[
        11, REST, REST, REST, 8, REST, REST, REST, 7, REST, REST, REST, 8, REST, 11, REST, 12,
        REST, REST, REST, 11, REST, REST, REST, 8, REST, REST, REST, 7, REST, 4, REST,
    ],
    pad: &[
        0, REST, REST, REST, REST, REST, REST, REST, 4, REST, REST, REST, 1, REST, REST, REST,
    ],
    arp: &[
        14, REST, 15, REST, 18, REST, 15, REST, 14, REST, 15, REST, 18, REST, 21, REST, 18, REST,
        15, REST, 14, REST, 11, REST, 14, REST, 15, REST, 18, REST, 15, REST,
    ],
    drums: &[
        Kick, Silent, Kick, Silent, Snare, Silent, Kick, Silent, Kick, Silent, Kick, Kick, Snare,
        Silent, Snare, Snare,
    ],
    perc: PERC_RIDE,
    perc_vel: PERC_RIDE_VEL,
    ..Section::EMPTY
};
const MASK_BRIDGE: Section = Section {
    label: "bridge",
    bass: &[
        4, REST, REST, REST, REST, REST, REST, REST, 3, REST, REST, REST, REST, REST, REST, REST,
    ],
    lead: &[
        REST, REST, REST, REST, 8, REST, REST, REST, REST, REST, REST, REST, 7, REST, REST, REST,
        REST, REST, REST, REST, 11, REST, REST, REST, REST, REST, REST, REST, 8, REST, REST, REST,
    ],
    pad: &[
        4, REST, REST, REST, REST, REST, REST, REST, 3, REST, REST, REST, REST, REST, REST, REST,
    ],
    arp: &[
        REST, REST, 14, REST, REST, REST, 15, REST, REST, REST, 18, REST, REST, REST, 21, REST,
        REST, REST, 18, REST, REST, REST, 15, REST, REST, REST, 14, REST, REST, REST, 11, REST,
    ],
    drums: &[
        Kick, Silent, Silent, Silent, Snare, Silent, Silent, Silent, Kick, Silent, Silent, Silent,
        Snare, Silent, Silent, Silent,
    ],
    ..Section::EMPTY
};

pub const MASK_OF_DREAD: SongSpec = SongSpec {
    name: "Mask of Dread",
    root: 32.70, // C1
    scale: LOCRIAN,
    bpm: 100.0,
    steps_per_beat: 4,
    voices: [
        // bass: a doubled saw, slow resonant bite, crushed, a sub beneath
        Voice::wide(Wave::Sawtooth, 0.0, 6.0, 0.3)
            .with_filter(160.0, 900.0, 0.0, 0.2, 3.0)
            .with_drive(0.6)
            .with_sub(0.35),
        // lead: three high squares, huge vibrato, driven, echoing in the hall
        Voice::stack(Wave::Square, 0.2, 12.0, 0.6, 3)
            .with_filter(900.0, 4500.0, 0.0, 0.25, 2.5)
            .with_vibrato(5.0, 25.0, 0.3)
            .with_echo(0.3)
            .with_reverb(0.3)
            .with_drive(0.35),
        // pad: a seven-saw wall opening over a second, gritty, deep in the hall
        Voice::stack(Wave::Sawtooth, 0.0, 16.0, 0.9, 7)
            .with_filter(300.0, 1500.0, 1.2, 0.0, 1.3)
            .with_reverb(0.55)
            .with_drive(0.2),
        // arp: driven square stabs with echoes
        Voice::panned(Wave::Square, -0.3)
            .with_echo(0.4)
            .with_drive(0.2),
        Voice::mono(Wave::Square), // keys: unused
    ],
    sections: &[
        MASK_INTRO,
        MASK_VERSE,
        MASK_VERSE,
        MASK_REFRAIN,
        MASK_VERSE,
        MASK_BRIDGE,
        MASK_REFRAIN,
        MASK_REFRAIN,
    ],
    intensity: 1.15,
    swing: 0.0,
    sidechain: Sidechain::new(0.5, 0.9),
    echo: Echo::DOTTED,
    humanize: 0.0,
    sweep: 1.0,
    // Mixed with the lane panners in place: no centre make-up.
    melodic_gain: 1.0,
};

/// The song (plain `const` data).
pub fn spec() -> SongSpec {
    MASK_OF_DREAD
}
