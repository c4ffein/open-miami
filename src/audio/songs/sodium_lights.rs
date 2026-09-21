//! "Sodium Lights" (WAVY / driving): the slow-burn night drive. D
//! minor, i–VI–III–VII, a wide detuned saw pad held a bar per chord, a
//! side-chain-pumped bass (retriggered sixteenths under a rising velocity
//! ramp), a tied square lead and an accented arp. Also the showcase of the
//! format's ties (`HOLD`), velocity lanes and stereo voices.
//!
//! `const` section literals against the full v2 format (`songs.rs`): ties,
//! velocity / chord lanes, stereo voices, echo + hall sends.

use super::Drum::{Clap, Hat, Kick, OpenHat, Silent, Snare};
use super::{Chord, Drum, Echo, Section, Sidechain, SongSpec, Voice, Wave, HOLD, MINOR, REST};

/// A bar of pad chord: struck once, held twelve steps, released for four.
const SODIUM_PAD: &[i32] = &[
    0, HOLD, HOLD, HOLD, HOLD, HOLD, HOLD, HOLD, HOLD, HOLD, HOLD, HOLD, REST, REST, REST, REST, 5,
    HOLD, HOLD, HOLD, HOLD, HOLD, HOLD, HOLD, HOLD, HOLD, HOLD, HOLD, REST, REST, REST, REST, 2,
    HOLD, HOLD, HOLD, HOLD, HOLD, HOLD, HOLD, HOLD, HOLD, HOLD, HOLD, REST, REST, REST, REST, 6,
    HOLD, HOLD, HOLD, HOLD, HOLD, HOLD, HOLD, HOLD, HOLD, HOLD, HOLD, REST, REST, REST, REST,
];
/// The pumped bass: every sixteenth retriggers the chord root, the velocity
/// ramp ducking on each kick and swelling back before the next.
const SODIUM_BASS: &[i32] = &[
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, -2, -2, -2, -2, -2, -2, -2, -2, -2, -2, -2, -2,
    -2, -2, -2, -2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, -1, -1, -1, -1, -1, -1, -1, -1,
    -1, -1, -1, -1, -1, -1, -1, -1,
];
const SODIUM_PUMP: &[u8] = &[3, 6, 8, 9];
const SODIUM_DRUMS: &[Drum] = &[
    Kick, Silent, Silent, Silent, Snare, Silent, Silent, Silent, Kick, Silent, Silent, Silent,
    Snare, Silent, Silent, Silent,
];
/// Off-beat closed hats riding over the kicks, a clap under each snare, an
/// open hat pushing into the next bar.
const SODIUM_PERC: &[Drum] = &[
    Hat, Silent, Hat, Silent, Clap, Silent, Hat, Silent, Hat, Silent, Hat, Silent, Clap, Silent,
    OpenHat, Silent,
];
const SODIUM_PERC_VEL: &[u8] = &[4, 0, 6, 0, 8, 0, 6, 0, 4, 0, 6, 0, 8, 0, 7, 0];

const SODIUM_INTRO: Section = Section {
    label: "intro",
    pad: SODIUM_PAD,
    pad_chord: SODIUM_PAD_CHORDS,
    arp: &[
        REST, REST, REST, REST, 14, REST, 16, REST, REST, REST, REST, REST, 18, REST, 16, REST,
    ],
    arp_vel: &[9, 0, 5, 0],
    drums: &[
        Silent, Silent, Hat, Silent, Silent, Silent, Hat, Silent, Silent, Silent, Hat, Silent,
        Silent, Silent, Hat, Hat,
    ],
    drums_vel: &[0, 0, 5, 0, 0, 0, 5, 0, 0, 0, 5, 0, 0, 0, 5, 3],
    ..Section::EMPTY
};
/// One voicing per bar under [`SODIUM_PAD`]: i7 – VI – III(add9) – VII.
const SODIUM_PAD_CHORDS: &[Chord] = &[
    Chord::Seventh,
    Chord::Seventh,
    Chord::Seventh,
    Chord::Seventh,
    Chord::Seventh,
    Chord::Seventh,
    Chord::Seventh,
    Chord::Seventh,
    Chord::Seventh,
    Chord::Seventh,
    Chord::Seventh,
    Chord::Seventh,
    Chord::Seventh,
    Chord::Seventh,
    Chord::Seventh,
    Chord::Seventh,
    Chord::Triad,
    Chord::Triad,
    Chord::Triad,
    Chord::Triad,
    Chord::Triad,
    Chord::Triad,
    Chord::Triad,
    Chord::Triad,
    Chord::Triad,
    Chord::Triad,
    Chord::Triad,
    Chord::Triad,
    Chord::Triad,
    Chord::Triad,
    Chord::Triad,
    Chord::Triad,
    Chord::Add9,
    Chord::Add9,
    Chord::Add9,
    Chord::Add9,
    Chord::Add9,
    Chord::Add9,
    Chord::Add9,
    Chord::Add9,
    Chord::Add9,
    Chord::Add9,
    Chord::Add9,
    Chord::Add9,
    Chord::Add9,
    Chord::Add9,
    Chord::Add9,
    Chord::Add9,
    Chord::Triad,
    Chord::Triad,
    Chord::Triad,
    Chord::Triad,
    Chord::Triad,
    Chord::Triad,
    Chord::Triad,
    Chord::Triad,
    Chord::Triad,
    Chord::Triad,
    Chord::Triad,
    Chord::Triad,
    Chord::Triad,
    Chord::Triad,
    Chord::Triad,
    Chord::Triad,
];

const SODIUM_VERSE: Section = Section {
    label: "verse",
    bass: SODIUM_BASS,
    bass_vel: SODIUM_PUMP,
    lead: &[
        11, HOLD, HOLD, HOLD, HOLD, HOLD, 9, HOLD, 7, HOLD, HOLD, HOLD, REST, REST, REST, REST, 12,
        HOLD, HOLD, HOLD, HOLD, HOLD, 11, HOLD, 9, HOLD, HOLD, HOLD, HOLD, HOLD, REST, REST, 9,
        HOLD, HOLD, HOLD, 11, HOLD, HOLD, HOLD, 13, HOLD, HOLD, HOLD, HOLD, HOLD, HOLD, HOLD, 14,
        HOLD, HOLD, HOLD, HOLD, HOLD, 13, HOLD, 11, HOLD, HOLD, HOLD, REST, REST, REST, REST,
    ],
    lead_vel: &[
        8, 9, 9, 9, 9, 9, 6, 9, 7, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 6, 9, 7, 9, 9, 9, 9, 9,
        9, 9, 7, 9, 9, 9, 8, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 7, 9, 6, 9, 9, 9,
        9, 9, 9, 9,
    ],
    pad: SODIUM_PAD,
    pad_chord: SODIUM_PAD_CHORDS,
    drums: SODIUM_DRUMS,
    perc: SODIUM_PERC,
    perc_vel: SODIUM_PERC_VEL,
    ..Section::EMPTY
};
const SODIUM_REFRAIN: Section = Section {
    label: "refrain",
    bass: SODIUM_BASS,
    bass_vel: SODIUM_PUMP,
    lead: &[
        14, HOLD, HOLD, 13, HOLD, HOLD, 11, HOLD, 9, HOLD, HOLD, HOLD, 11, HOLD, HOLD, HOLD, 12,
        HOLD, HOLD, 14, HOLD, HOLD, 12, HOLD, 11, HOLD, HOLD, HOLD, HOLD, HOLD, HOLD, HOLD, 13,
        HOLD, HOLD, 14, HOLD, HOLD, 16, HOLD, 14, HOLD, HOLD, HOLD, 13, HOLD, HOLD, HOLD, 14, HOLD,
        HOLD, HOLD, HOLD, HOLD, HOLD, HOLD, 11, HOLD, HOLD, HOLD, REST, REST, REST, REST,
    ],
    pad: SODIUM_PAD,
    pad_chord: SODIUM_PAD_CHORDS,
    arp: &[
        14, 16, 18, 16, 14, 16, 18, 21, 14, 16, 18, 16, 21, 18, 16, 14, 12, 14, 16, 14, 12, 14, 16,
        19, 12, 14, 16, 14, 19, 16, 14, 12, 16, 18, 20, 18, 16, 18, 20, 23, 16, 18, 20, 18, 23, 20,
        18, 16, 13, 15, 17, 15, 13, 15, 17, 20, 13, 15, 17, 15, 20, 17, 15, 13,
    ],
    arp_vel: &[9, 5, 7, 5, 8, 5, 7, 6],
    drums: SODIUM_DRUMS,
    perc: SODIUM_PERC,
    perc_vel: SODIUM_PERC_VEL,
    ..Section::EMPTY
};
const SODIUM_BREAK: Section = Section {
    label: "break",
    bass: &[
        0, HOLD, HOLD, HOLD, HOLD, HOLD, HOLD, HOLD, HOLD, HOLD, HOLD, HOLD, HOLD, HOLD, HOLD,
        HOLD, -2, HOLD, HOLD, HOLD, HOLD, HOLD, HOLD, HOLD, HOLD, HOLD, HOLD, HOLD, HOLD, HOLD,
        HOLD, HOLD, 2, HOLD, HOLD, HOLD, HOLD, HOLD, HOLD, HOLD, HOLD, HOLD, HOLD, HOLD, HOLD,
        HOLD, HOLD, HOLD, -1, HOLD, HOLD, HOLD, HOLD, HOLD, HOLD, HOLD, HOLD, HOLD, HOLD, HOLD,
        HOLD, HOLD, HOLD, HOLD,
    ],
    lead: &[
        REST, REST, REST, REST, REST, REST, REST, REST, 7, HOLD, HOLD, HOLD, 9, HOLD, HOLD, HOLD,
        11, HOLD, HOLD, HOLD, HOLD, HOLD, HOLD, HOLD, HOLD, HOLD, HOLD, HOLD, HOLD, HOLD, HOLD,
        HOLD, REST, REST, REST, REST, REST, REST, REST, REST, 9, HOLD, HOLD, HOLD, 11, HOLD, HOLD,
        HOLD, 13, HOLD, HOLD, HOLD, HOLD, HOLD, HOLD, HOLD, HOLD, HOLD, HOLD, HOLD, HOLD, HOLD,
        HOLD, HOLD,
    ],
    lead_vel: &[6],
    pad: SODIUM_PAD,
    pad_chord: SODIUM_PAD_CHORDS,
    // The riser: silent for two bars, then one 32-step noise swell.
    keys: &[
        REST, REST, REST, REST, REST, REST, REST, REST, REST, REST, REST, REST, REST, REST, REST,
        REST, REST, REST, REST, REST, REST, REST, REST, REST, REST, REST, REST, REST, REST, REST,
        REST, REST, 0, HOLD, HOLD, HOLD, HOLD, HOLD, HOLD, HOLD, HOLD, HOLD, HOLD, HOLD, HOLD,
        HOLD, HOLD, HOLD, HOLD, HOLD, HOLD, HOLD, HOLD, HOLD, HOLD, HOLD, HOLD, HOLD, HOLD, HOLD,
        HOLD, HOLD, HOLD, HOLD,
    ],
    keys_vel: &[7],
    drums: &[
        Kick, Silent, Silent, Silent, Silent, Silent, Silent, Silent, Kick, Silent, Silent, Silent,
        Silent, Silent, Hat, Silent,
    ],
    drums_vel: &[7, 0, 0, 0, 0, 0, 0, 0, 6, 0, 0, 0, 0, 0, 4, 0],
    ..Section::EMPTY
};

pub const SODIUM_LIGHTS: SongSpec = SongSpec {
    name: "Sodium Lights",
    root: 73.42, // D2
    scale: MINOR,
    bpm: 96.0,
    steps_per_beat: 4,
    voices: [
        // bass: a centred saw with a fast resonant pluck and a sine sub under it
        Voice::mono(Wave::Sawtooth)
            .with_filter(230.0, 1100.0, 0.0, 0.12, 3.0)
            .with_sub(0.5),
        // lead: a doubled square, a little right, late vibrato, a soft wow,
        // dotted-eighth echoes trailing into the hall
        Voice::wide(Wave::Square, 0.25, 6.0, 0.3)
            .with_vibrato(5.5, 12.0, 0.25)
            .with_filter(1800.0, 5200.0, 0.0, 0.18, 1.6)
            .with_echo(0.35)
            .with_reverb(0.25),
        // pad: a five-saw supersaw that blooms open over a second, deep in
        // the hall
        Voice::stack(Wave::Sawtooth, 0.0, 12.0, 0.85, 5)
            .with_filter(700.0, 2600.0, 1.1, 0.0, 1.1)
            .with_reverb(0.45),
        // arp: a triangle answering from the left, echoing to the right
        Voice::panned(Wave::Triangle, -0.35).with_echo(0.5),
        // keys: the noise RISER out of the break — a filter opening over
        // five seconds under a slow swell, deep in the hall
        Voice::mono(Wave::Noise)
            .with_env(2.5, 1.0)
            .with_filter(300.0, 7000.0, 4.8, 0.0, 1.8)
            .with_reverb(0.4),
    ],
    sections: &[
        SODIUM_INTRO,
        SODIUM_VERSE,
        SODIUM_REFRAIN,
        SODIUM_VERSE,
        SODIUM_REFRAIN,
        SODIUM_BREAK,
        SODIUM_REFRAIN,
        SODIUM_REFRAIN,
    ],
    intensity: 0.8,
    swing: 0.0,
    sidechain: Sidechain::new(0.55, 0.9),
    echo: Echo::new(3.0, 0.42, 2800.0),
    humanize: 0.004,
    sweep: 0.35,
    // Mixed with the lane panners in place: no centre make-up.
    melodic_gain: 1.0,
};

/// The song (plain `const` data).
pub fn spec() -> SongSpec {
    SODIUM_LIGHTS
}
