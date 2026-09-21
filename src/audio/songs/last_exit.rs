//! "Last Exit" (WAVY / slow): the 3 a.m. ballad. A minor, 91 bpm,
//! i–VI–III–VII in sevenths, a wide pad blooming a bar per chord, a gliding
//! sine sub bass in half notes, a long tied lead line with late vibrato in
//! dotted-eighth echoes, kick on 1 and 3, a clap on 2 and 4, eighth hats.
//!
//! `const` section literals against the full v2 format (`songs.rs`): ties,
//! velocity / chord lanes, stereo voices, echo + hall sends.

use super::Drum::{Clap, Hat, Kick, Silent};
use super::{Chord, Drum, Echo, Section, Sidechain, SongSpec, Voice, Wave, HOLD, MINOR, REST};

/// Two half notes a bar: the chord root and its fifth (or the root again),
/// legato — the bass slides between them.
const EXIT_BASS: &[i32] = &[
    0, HOLD, HOLD, HOLD, HOLD, HOLD, HOLD, HOLD, 4, HOLD, HOLD, HOLD, HOLD, HOLD, HOLD, HOLD, -2,
    HOLD, HOLD, HOLD, HOLD, HOLD, HOLD, HOLD, 2, HOLD, HOLD, HOLD, HOLD, HOLD, HOLD, HOLD, -5,
    HOLD, HOLD, HOLD, HOLD, HOLD, HOLD, HOLD, -1, HOLD, HOLD, HOLD, HOLD, HOLD, HOLD, HOLD, -1,
    HOLD, HOLD, HOLD, HOLD, HOLD, HOLD, HOLD, 3, HOLD, HOLD, HOLD, HOLD, HOLD, HOLD, HOLD,
];
/// A bar per chord, struck once, held 14 steps (the bloom needs the time).
const EXIT_PAD: &[i32] = &[
    0, HOLD, HOLD, HOLD, HOLD, HOLD, HOLD, HOLD, HOLD, HOLD, HOLD, HOLD, HOLD, HOLD, REST, REST, 5,
    HOLD, HOLD, HOLD, HOLD, HOLD, HOLD, HOLD, HOLD, HOLD, HOLD, HOLD, HOLD, HOLD, REST, REST, 2,
    HOLD, HOLD, HOLD, HOLD, HOLD, HOLD, HOLD, HOLD, HOLD, HOLD, HOLD, HOLD, HOLD, REST, REST, 6,
    HOLD, HOLD, HOLD, HOLD, HOLD, HOLD, HOLD, HOLD, HOLD, HOLD, HOLD, HOLD, HOLD, REST, REST,
];
const EXIT_PAD_CHORDS: &[Chord] = &[Chord::Seventh];
const EXIT_DRUMS: &[Drum] = &[
    Kick, Silent, Silent, Silent, Silent, Silent, Silent, Silent, Kick, Silent, Silent, Silent,
    Silent, Silent, Silent, Silent,
];
const EXIT_PERC: &[Drum] = &[
    Hat, Silent, Hat, Silent, Clap, Silent, Hat, Silent, Hat, Silent, Hat, Silent, Clap, Silent,
    Hat, Silent,
];
const EXIT_PERC_VEL: &[u8] = &[5, 0, 3, 0, 8, 0, 3, 0, 5, 0, 3, 0, 8, 0, 4, 0];

const EXIT_INTRO: Section = Section {
    label: "intro",
    pad: EXIT_PAD,
    pad_chord: EXIT_PAD_CHORDS,
    pad_vel: &[7],
    arp: &[
        REST, REST, REST, REST, REST, REST, REST, REST, 14, REST, 16, REST, 18, REST, 16, REST,
    ],
    arp_vel: &[5],
    perc: &[
        Silent, Silent, Silent, Silent, Silent, Silent, Silent, Silent, Silent, Silent, Silent,
        Silent, Silent, Silent, Silent, Silent, Silent, Silent, Silent, Silent, Silent, Silent,
        Silent, Silent, Silent, Silent, Silent, Silent, Silent, Silent, Silent, Silent, Hat,
        Silent, Hat, Silent, Hat, Silent, Hat, Silent, Hat, Silent, Hat, Silent, Hat, Silent, Hat,
        Silent, Hat, Silent, Hat, Silent, Hat, Silent, Hat, Silent, Hat, Silent, Hat, Silent, Hat,
        Silent, Hat, Hat,
    ],
    perc_vel: &[4, 0, 3, 0],
    ..Section::EMPTY
};
const EXIT_VERSE: Section = Section {
    label: "verse",
    bass: EXIT_BASS,
    lead: &[
        REST, REST, REST, REST, 11, HOLD, HOLD, HOLD, 12, HOLD, HOLD, HOLD, 11, HOLD, HOLD, HOLD,
        9, HOLD, HOLD, HOLD, HOLD, HOLD, HOLD, HOLD, HOLD, HOLD, HOLD, HOLD, REST, REST, REST,
        REST, REST, REST, REST, REST, 9, HOLD, HOLD, HOLD, 11, HOLD, HOLD, HOLD, 12, HOLD, HOLD,
        HOLD, 14, HOLD, HOLD, HOLD, HOLD, HOLD, HOLD, HOLD, 13, HOLD, HOLD, HOLD, HOLD, HOLD, HOLD,
        HOLD,
    ],
    lead_vel: &[7],
    pad: EXIT_PAD,
    pad_chord: EXIT_PAD_CHORDS,
    drums: EXIT_DRUMS,
    perc: EXIT_PERC,
    perc_vel: EXIT_PERC_VEL,
    ..Section::EMPTY
};
const EXIT_REFRAIN: Section = Section {
    label: "refrain",
    bass: EXIT_BASS,
    lead: &[
        14, HOLD, HOLD, HOLD, HOLD, HOLD, 16, HOLD, 14, HOLD, HOLD, HOLD, 12, HOLD, HOLD, HOLD, 11,
        HOLD, HOLD, HOLD, HOLD, HOLD, HOLD, HOLD, HOLD, HOLD, HOLD, HOLD, 12, HOLD, HOLD, HOLD, 9,
        HOLD, HOLD, HOLD, HOLD, HOLD, 11, HOLD, 12, HOLD, HOLD, HOLD, 14, HOLD, HOLD, HOLD, 13,
        HOLD, HOLD, HOLD, HOLD, HOLD, HOLD, HOLD, HOLD, HOLD, HOLD, HOLD, 11, HOLD, HOLD, HOLD,
    ],
    lead_vel: &[9, 9, 9, 9, 9, 9, 7, 9, 8, 9, 9, 9, 7, 9, 9, 9],
    pad: EXIT_PAD,
    pad_chord: EXIT_PAD_CHORDS,
    arp: &[
        14, 16, 18, 21, 16, 18, 21, 23, 12, 14, 16, 19, 14, 16, 19, 21,
    ],
    arp_vel: &[6, 3, 4, 3, 5, 3, 4, 3],
    drums: EXIT_DRUMS,
    perc: EXIT_PERC,
    perc_vel: EXIT_PERC_VEL,
    ..Section::EMPTY
};
const EXIT_OUTRO: Section = Section {
    label: "outro",
    bass: &[
        0, HOLD, HOLD, HOLD, HOLD, HOLD, HOLD, HOLD, HOLD, HOLD, HOLD, HOLD, HOLD, HOLD, HOLD,
        HOLD, HOLD, HOLD, HOLD, HOLD, HOLD, HOLD, HOLD, HOLD, HOLD, HOLD, HOLD, HOLD, HOLD, HOLD,
        HOLD, HOLD,
    ],
    lead: &[
        REST, REST, REST, REST, REST, REST, REST, REST, 11, HOLD, HOLD, HOLD, 9, HOLD, HOLD, HOLD,
        7, HOLD, HOLD, HOLD, HOLD, HOLD, HOLD, HOLD, HOLD, HOLD, HOLD, HOLD, HOLD, HOLD, HOLD,
        HOLD,
    ],
    lead_vel: &[6],
    pad: &[
        0, HOLD, HOLD, HOLD, HOLD, HOLD, HOLD, HOLD, HOLD, HOLD, HOLD, HOLD, HOLD, HOLD, HOLD,
        HOLD, HOLD, HOLD, HOLD, HOLD, HOLD, HOLD, HOLD, HOLD, HOLD, HOLD, HOLD, HOLD, HOLD, HOLD,
        HOLD, HOLD,
    ],
    pad_chord: &[Chord::Add9],
    perc: &[
        Hat, Silent, Hat, Silent, Silent, Silent, Hat, Silent, Hat, Silent, Hat, Silent, Silent,
        Silent, Hat, Silent,
    ],
    perc_vel: &[3, 0, 2, 0, 0, 0, 2, 0, 3, 0, 2, 0, 0, 0, 2, 0],
    ..Section::EMPTY
};

pub const LAST_EXIT: SongSpec = SongSpec {
    name: "Last Exit",
    root: 110.0, // A2
    scale: MINOR,
    bpm: 91.0,
    steps_per_beat: 4,
    voices: [
        // bass: a soft triangle over a big sine sub, sliding between its two
        // notes a bar
        Voice::mono(Wave::Triangle)
            .with_env(0.02, 1.5)
            .with_filter(120.0, 500.0, 0.0, 0.4, 1.0)
            .with_sub(0.7)
            .with_glide(0.12),
        // lead: a doubled square, a slow late vibrato, a long dotted echo
        // in the hall, sliding into each tied note
        Voice::wide(Wave::Square, 0.15, 5.0, 0.35)
            .with_env(0.03, 1.5)
            .with_filter(900.0, 2400.0, 0.0, 0.35, 1.2)
            .with_vibrato(4.8, 10.0, 0.5)
            .with_glide(0.09)
            .with_echo(0.45)
            .with_reverb(0.4),
        // pad: seven saws, very wide, blooming over 1.8 s deep in the hall
        Voice::stack(Wave::Sawtooth, 0.0, 10.0, 0.95, 7)
            .with_filter(400.0, 1900.0, 1.8, 0.0, 0.9)
            .with_reverb(0.6),
        // arp: a quiet sine sparkle, echoing to the right
        Voice::panned(Wave::Sine, -0.4)
            .with_echo(0.55)
            .with_reverb(0.3),
        Voice::mono(Wave::Square), // keys: unused
    ],
    sections: &[
        EXIT_INTRO,
        EXIT_VERSE,
        EXIT_REFRAIN,
        EXIT_VERSE,
        EXIT_REFRAIN,
        EXIT_OUTRO,
        EXIT_REFRAIN,
    ],
    intensity: 0.6,
    swing: 0.0,
    sidechain: Sidechain::new(0.3, 1.2),
    echo: Echo::new(3.0, 0.45, 2600.0),
    humanize: 0.006,
    sweep: 0.2,
    // Mixed with the lane panners in place: no centre make-up.
    melodic_gain: 1.0,
};

/// The song (plain `const` data).
pub fn spec() -> SongSpec {
    LAST_EXIT
}
