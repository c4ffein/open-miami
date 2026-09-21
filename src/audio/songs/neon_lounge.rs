//! "Neon Lounge" (WAVY): cool, loungey opening groove. A-minor,
//! laid-back, mellow syncopated lead over light hats — neon at dusk.
//!
//! `const` section literals against the full v2 format (`songs.rs`): ties,
//! velocity / chord lanes, stereo voices, echo + hall sends.

use super::Drum::{Hat, Kick, Silent, Snare};
use super::{Echo, Section, Sidechain, SongSpec, Voice, Wave, MINOR, REST};

const NEON_INTRO: Section = Section {
    label: "intro",
    bass: &[
        0, REST, REST, REST, REST, REST, REST, REST, 3, REST, REST, REST, REST, REST, REST, REST,
    ],
    lead: &[
        REST, REST, REST, REST, 7, REST, 9, REST, REST, REST, REST, REST, REST, REST, REST, REST,
        REST, REST, REST, REST, 11, REST, 9, REST, REST, REST, REST, REST, REST, REST, REST, REST,
    ],
    pad: &[
        0, REST, REST, REST, REST, REST, REST, REST, 3, REST, REST, REST, REST, REST, REST, REST,
    ],
    arp: &[
        REST, 14, REST, 16, REST, 14, REST, 11, REST, 14, REST, 16, REST, 18, REST, 16, REST, 14,
        REST, 16, REST, 14, REST, 11, REST, 14, REST, 16, REST, 18, REST, 16,
    ],
    drums: &[
        Kick, Silent, Hat, Silent, Silent, Silent, Hat, Silent, Kick, Silent, Hat, Silent, Silent,
        Silent, Hat, Silent,
    ],
    ..Section::EMPTY
};
const NEON_VERSE: Section = Section {
    label: "verse",
    bass: &[
        0, REST, REST, REST, 0, REST, 4, REST, 3, REST, REST, REST, 2, REST, 2, REST,
    ],
    lead: &[
        7, REST, 9, REST, REST, 11, REST, 7, REST, REST, 9, REST, 10, REST, REST, REST, 7, REST, 9,
        REST, REST, 11, REST, 12, REST, REST, 10, REST, 9, REST, 7, REST,
    ],
    pad: &[
        0, REST, REST, REST, REST, REST, REST, REST, 3, REST, REST, REST, REST, REST, REST, REST,
    ],
    arp: &[
        REST, 14, REST, 16, REST, 14, REST, 11, REST, 14, REST, 16, REST, 18, REST, 16, REST, 16,
        REST, 18, REST, 16, REST, 14, REST, 16, REST, 18, REST, 21, REST, 18,
    ],
    drums: &[
        Kick, Silent, Hat, Silent, Silent, Silent, Hat, Silent, Kick, Silent, Hat, Silent, Silent,
        Silent, Hat, Snare,
    ],
    ..Section::EMPTY
};
const NEON_REFRAIN: Section = Section {
    label: "refrain",
    bass: &[
        0, REST, 0, REST, 4, REST, 4, REST, 3, REST, 3, REST, 2, REST, 5, REST,
    ],
    lead: &[
        11, REST, 12, REST, 14, REST, 12, REST, 11, REST, 9, REST, 7, REST, 9, REST, 11, REST, 12,
        REST, 14, REST, 16, REST, 14, REST, 12, REST, 11, REST, 9, REST,
    ],
    pad: &[
        3, REST, REST, REST, REST, REST, REST, REST, 5, REST, REST, REST, REST, REST, REST, REST,
    ],
    arp: &[
        18, 16, 14, 16, 18, 16, 14, 11, 18, 16, 14, 16, 18, 21, 18, 16, 14, 16, 18, 21, 18, 16, 14,
        16, 18, 21, 23, 21, 18, 16, 14, 12,
    ],
    drums: &[
        Kick, Silent, Hat, Snare, Silent, Silent, Hat, Silent, Kick, Silent, Hat, Snare, Silent,
        Silent, Hat, Snare,
    ],
    ..Section::EMPTY
};
const NEON_BRIDGE: Section = Section {
    label: "bridge",
    bass: &[
        5, REST, REST, REST, REST, REST, REST, REST, 4, REST, REST, REST, REST, REST, REST, REST,
    ],
    lead: &[
        REST, REST, 9, REST, 7, REST, REST, REST, REST, REST, 11, REST, 9, REST, REST, REST, REST,
        REST, 9, REST, 7, REST, REST, REST, REST, REST, 12, REST, 10, REST, 9, REST,
    ],
    pad: &[
        5, REST, REST, REST, REST, REST, REST, REST, 4, REST, REST, REST, REST, REST, REST, REST,
    ],
    arp: &[
        14, REST, 16, REST, 18, REST, 16, REST, 14, REST, 16, REST, 18, REST, 21, REST, 18, REST,
        16, REST, 14, REST, 16, REST, 14, REST, 11, REST, 9, REST, 7, REST,
    ],
    drums: &[
        Kick, Silent, Silent, Silent, Silent, Silent, Hat, Silent, Kick, Silent, Silent, Silent,
        Silent, Silent, Hat, Silent,
    ],
    ..Section::EMPTY
};

pub const NEON_LOUNGE: SongSpec = SongSpec {
    name: "Neon Lounge",
    root: 55.0, // A1
    scale: MINOR,
    bpm: 108.0,
    steps_per_beat: 4,
    voices: [
        // bass: a triangle with a soft, round pluck
        Voice::mono(Wave::Triangle).with_filter(200.0, 700.0, 0.0, 0.15, 2.0),
        // lead: a mellow triangle, light vibrato, a touch of echo and room
        Voice::panned(Wave::Triangle, 0.2)
            .with_vibrato(5.0, 10.0, 0.3)
            .with_echo(0.3)
            .with_reverb(0.2),
        // pad: a warm three-saw bed opening over a second
        Voice::stack(Wave::Sawtooth, 0.0, 8.0, 0.8, 3)
            .with_filter(500.0, 1600.0, 0.9, 0.0, 1.0)
            .with_reverb(0.4),
        // arp: a triangle with dotted-eighth echoes
        Voice::panned(Wave::Triangle, -0.3).with_echo(0.3),
        Voice::mono(Wave::Square), // keys: unused
    ],
    sections: &[
        NEON_INTRO,
        NEON_VERSE,
        NEON_VERSE,
        NEON_REFRAIN,
        NEON_VERSE,
        NEON_BRIDGE,
        NEON_REFRAIN,
        NEON_REFRAIN,
    ],
    intensity: 0.55,
    swing: 0.0,
    sidechain: Sidechain::new(0.25, 0.8),
    echo: Echo::DOTTED,
    humanize: 0.005,
    sweep: 1.0,
    // Mixed with the lane panners in place: no centre make-up.
    melodic_gain: 1.0,
};

/// The song (plain `const` data).
pub fn spec() -> SongSpec {
    NEON_LOUNGE
}
