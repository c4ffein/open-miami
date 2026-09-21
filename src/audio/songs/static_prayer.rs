//! "Static Prayer" (WAVY): a crawling, hopeless dirge. G Locrian
//! (tritone), slow lurching bass, mournful pad drone, sparse detuned wails.
//!
//! `const` section literals against the full v2 format (`songs.rs`): ties,
//! velocity / chord lanes, stereo voices, echo + hall sends.

use super::Drum::{Hat, Kick, Silent, Snare};
use super::{Echo, Section, Sidechain, SongSpec, Voice, Wave, LOCRIAN, REST};

const PRAYER_INTRO: Section = Section {
    label: "intro",
    bass: &[
        0, REST, REST, REST, REST, REST, REST, REST, 0, REST, REST, REST, REST, REST, REST, REST,
    ],
    lead: &[
        REST, REST, REST, REST, REST, REST, REST, REST, REST, REST, REST, REST, REST, REST, REST,
        REST, REST, REST, REST, REST, REST, REST, REST, REST, REST, REST, REST, REST, REST, REST,
        REST, REST,
    ],
    pad: &[
        0, REST, REST, REST, REST, REST, REST, REST, 4, REST, REST, REST, REST, REST, REST, REST,
    ],
    arp: &[
        REST, REST, REST, REST, REST, REST, 14, REST, REST, REST, REST, REST, REST, REST, 15, REST,
        REST, REST, REST, REST, REST, REST, 18, REST, REST, REST, REST, REST, REST, REST, 15, REST,
    ],
    drums: &[
        Kick, Silent, Silent, Silent, Silent, Silent, Silent, Silent, Kick, Silent, Silent, Silent,
        Silent, Silent, Hat, Silent,
    ],
    ..Section::EMPTY
};
const PRAYER_VERSE: Section = Section {
    label: "verse",
    bass: &[
        0, REST, REST, REST, 0, REST, REST, 4, 0, REST, REST, REST, 1, REST, REST, REST,
    ],
    lead: &[
        REST, REST, REST, REST, 8, REST, REST, REST, REST, REST, 7, REST, REST, REST, REST, REST,
        REST, REST, REST, REST, 7, REST, REST, REST, REST, REST, 8, REST, REST, REST, REST, REST,
    ],
    pad: &[
        0, REST, REST, REST, REST, REST, REST, REST, 4, REST, REST, REST, REST, REST, REST, REST,
    ],
    arp: &[
        REST, REST, 14, REST, REST, REST, 15, REST, REST, REST, 18, REST, REST, REST, 15, REST,
        REST, REST, 15, REST, REST, REST, 18, REST, REST, REST, 14, REST, REST, REST, 11, REST,
    ],
    drums: &[
        Kick, Silent, Silent, Silent, Silent, Silent, Snare, Silent, Kick, Silent, Silent, Silent,
        Snare, Silent, Hat, Silent,
    ],
    ..Section::EMPTY
};
const PRAYER_REFRAIN: Section = Section {
    label: "refrain",
    bass: &[
        0, REST, REST, REST, 4, REST, REST, REST, 1, REST, REST, REST, 4, REST, REST, REST,
    ],
    lead: &[
        8, REST, REST, REST, 7, REST, REST, REST, 8, REST, REST, REST, 11, REST, REST, REST, 12,
        REST, REST, REST, 11, REST, REST, REST, 8, REST, REST, REST, 7, REST, REST, REST,
    ],
    pad: &[
        0, REST, REST, REST, REST, REST, REST, REST, 4, REST, REST, REST, 1, REST, REST, REST,
    ],
    arp: &[
        REST, 14, REST, 15, REST, 18, REST, 15, REST, 14, REST, 15, REST, 18, REST, 21, REST, 18,
        REST, 15, REST, 14, REST, 11, REST, 14, REST, 15, REST, 18, REST, 15,
    ],
    drums: &[
        Kick, Silent, Silent, Snare, Silent, Silent, Snare, Silent, Kick, Silent, Silent, Snare,
        Silent, Silent, Hat, Snare,
    ],
    ..Section::EMPTY
};
const PRAYER_BRIDGE: Section = Section {
    label: "bridge",
    bass: &[
        4, REST, REST, REST, REST, REST, REST, REST, 1, REST, REST, REST, REST, REST, REST, REST,
    ],
    lead: &[
        REST, REST, REST, REST, REST, REST, REST, REST, 11, REST, REST, REST, 8, REST, 7, REST,
        REST, REST, REST, REST, REST, REST, REST, REST, 12, REST, REST, REST, 11, REST, 8, REST,
    ],
    pad: &[
        4, REST, REST, REST, REST, REST, REST, REST, 1, REST, REST, REST, REST, REST, REST, REST,
    ],
    arp: &[
        REST, REST, 14, REST, REST, REST, 15, REST, REST, REST, 18, REST, REST, REST, 21, REST,
        REST, REST, 18, REST, REST, REST, 15, REST, REST, REST, 14, REST, REST, REST, 11, REST,
    ],
    drums: &[
        Kick, Silent, Silent, Silent, Silent, Silent, Silent, Silent, Kick, Silent, Silent, Silent,
        Snare, Silent, Silent, Silent,
    ],
    ..Section::EMPTY
};

pub const STATIC_PRAYER: SongSpec = SongSpec {
    name: "Static Prayer",
    root: 49.00, // G1
    scale: LOCRIAN,
    bpm: 92.0,
    steps_per_beat: 4,
    voices: [
        // bass: a slow, dull saw lurch
        Voice::mono(Wave::Sawtooth).with_filter(150.0, 600.0, 0.0, 0.3, 1.5),
        // lead: a detuned, wide triangle wail with a slow deep vibrato,
        // sliding into each touching note
        Voice::wide(Wave::Triangle, 0.2, 10.0, 0.6)
            .with_glide(0.15)
            .with_vibrato(4.5, 20.0, 0.5)
            .with_echo(0.4)
            .with_reverb(0.5),
        // pad: a mournful supersaw drone taking two seconds to open
        Voice::stack(Wave::Sawtooth, 0.0, 15.0, 0.9, 5)
            .with_filter(350.0, 1200.0, 2.0, 0.0, 1.0)
            .with_reverb(0.6),
        // arp: sparse triangle wails drowning in echo and hall
        Voice::panned(Wave::Triangle, -0.3)
            .with_echo(0.45)
            .with_reverb(0.3),
        Voice::mono(Wave::Square), // keys: unused
    ],
    sections: &[
        PRAYER_INTRO,
        PRAYER_VERSE,
        PRAYER_VERSE,
        PRAYER_REFRAIN,
        PRAYER_VERSE,
        PRAYER_BRIDGE,
        PRAYER_REFRAIN,
        PRAYER_REFRAIN,
    ],
    intensity: 0.8,
    swing: 0.0,
    sidechain: Sidechain::new(0.2, 1.2),
    echo: Echo::DOTTED,
    humanize: 0.0,
    sweep: 1.0,
    // Mixed with the lane panners in place: no centre make-up.
    melodic_gain: 1.0,
};

/// The song (plain `const` data).
pub fn spec() -> SongSpec {
    STATIC_PRAYER
}
