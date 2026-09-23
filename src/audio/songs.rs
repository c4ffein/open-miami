//! The song format + the pure sequencer arithmetic.
//!
//! Everything here compiles NATIVELY (no `web-sys`): the song types, the
//! in-key pitch math, the section / step playhead and the voice-set
//! enumeration the bake queue works from. The songs themselves are RUST
//! CODE — one file per song under `src/audio/songs/`, written with the
//! composable authoring layer in [`super::compose`] (see
//! `docs/MUSIC_CODE.md`) and collected in [`SONGS`]; the WebAudio engine
//! (`audio/engine.rs`, wasm-only) only *plays* what is described here.
//!
//! A song is *plain data*: a key (root frequency + scale), a tempo, a set
//! of oscillator voices, and an ordered list of SECTIONS.
//!
//! Each [`Section`] is its own multi-bar block of seven step-sequenced
//! channels (bass, lead, pad, arp, keys + two percussion lanes). A
//! [`SongSpec`] strings sections
//! together into a real arrangement — intro / verse / refrain / bridge /
//! variation — so a full play-through develops over time and the refrain
//! *returns* instead of a single bar looping forever. Sections are just
//! `&'static` slices of patterns, so a section can appear several times in
//! the order (that is how a refrain comes back) at zero extra cost.
//!
//! Melodic patterns are written as *scale degrees* (see [`degree_freq`]):
//! `0` is the root, `1` the next scale note up, `7` an octave up (for a
//! 7-note scale), negative degrees drop below the root. [`REST`] means
//! silence for that step; [`HOLD`] TIES the previous note through the step
//! (a note's length = 1 + the `HOLD`s that follow it: `0, HOLD, HOLD, HOLD`
//! is one quarter-note root, `0, 0, 0, 0` is four retriggered sixteenths).
//! This keeps a song readable and in-key no matter which root/scale it uses.
//!
//! Every lane has an optional parallel VELOCITY lane (`bass_vel` …
//! `perc_vel`): one `0..=`[`MAX_VEL`] per step, looping like the notes (an
//! empty lane = every note at full velocity, `0` = skip the note). Velocity
//! scales the note's amplitude linearly — accents, ghost notes, a
//! retriggered pump — on top of the section's per-channel `level`. The
//! melodic lanes also have a CHORD lane (`*_chord`: one [`Chord`] voicing
//! per step, read where a note starts).
//!
//! Lanes inside a section may differ in length: a short 16-step bass simply
//! repeats under a longer 32-step lead. A section's length is its longest
//! lane, so authoring a 2-bar section only means writing one lane at 32
//! steps.
//!
//! The `pad` lane is special: by default each note blooms into a full triad
//! (root + third + fifth taken from the scale — [`Chord::default_for`]) with
//! a slow attack, for sustained chord beds.
//!
//! The INSTRUMENTS (a [`Voice`] per melodic lane) and the song-level bus
//! effects ([`Sidechain`], [`Echo`]) live in [`super::voice`], re-exported
//! here.

use std::sync::LazyLock;

pub use super::voice::*;

// The songs themselves: one Rust file per song (src/audio/songs/<name>.rs),
// written with the `compose` authoring layer. Listed in `SONGS` below.
pub mod blood_engine;
pub mod blood_rush;
pub mod chrome_veins;
pub mod coast_home;
pub mod crown_of_static;
pub mod deep_static;
pub mod descent;
pub mod insert_coin;
pub mod last_exit;
pub mod low_tide;
pub mod mask_of_dread;
pub mod neon_checksum;
pub mod neon_lounge;
pub mod salt_road;
pub mod service_corridor;
pub mod signal_rot;
pub mod sodium_lights;
pub mod static_prayer;
pub mod static_teeth;
pub mod thermal_mass;
pub mod walk_dont_run;

/// Sentinel used inside a pattern to mean "rest" (no note this step).
pub const REST: i32 = i32::MIN;

/// Sentinel used inside a pattern to mean "tie": the previous note of the
/// lane sustains through this step instead of a new one starting.
pub const HOLD: i32 = i32::MIN + 1;
/// Full velocity: the top of a velocity lane's `0..=MAX_VEL` scale (tracker
/// style, one digit per step). An empty velocity lane plays everything here.
pub const MAX_VEL: u8 = 9;

/// Number of sequenced channels (rows in the tracker view).
pub const NUM_CHANNELS: usize = 7;
/// Channel (lane) indices, `0..NUM_CHANNELS`.
pub const BASS: usize = 0;
pub const LEAD: usize = 1;
pub const PAD: usize = 2;
pub const ARP: usize = 3;
/// The fifth melodic lane: chord stabs, a second lead, a counter-line —
/// whatever the four classic lanes leave no room for.
pub const KEYS: usize = 4;
pub const DRUMS: usize = 5;
/// The second percussion lane: same kit as `DRUMS`, so a hat can ride over
/// a kick, a clap can layer a snare, a crash can top a downbeat.
pub const PERC: usize = 6;
/// The melodic lanes, in bake-priority order (densest / most exposed first).
pub const MELODIC: [usize; 5] = [BASS, LEAD, ARP, KEYS, PAD];
/// How many melodic lanes there are (= `SongSpec::voices.len()`; a voice is
/// indexed by its lane: `voices[BASS]` … `voices[KEYS]`).
pub const NUM_VOICES: usize = MELODIC.len();

/// Human-readable channel names, indexed 0..[`NUM_CHANNELS`].
pub const CHANNEL_NAMES: [&str; NUM_CHANNELS] =
    ["BASS", "LEAD", "PAD", "ARP", "KEYS", "DRUMS", "PERC"];

/// Scale = semitone offsets from the root, one octave's worth. Darker modes
/// (flat 2nd, tritone) read as more menacing — we escalate them across floors.
pub type Scale = &'static [i32];

/// Aeolian / natural minor — the classic neon-noir minor key.
pub const MINOR: Scale = &[0, 2, 3, 5, 7, 8, 10];
/// Dorian — minor with a raised 6th; cool, driving, a touch hopeful.
pub const DORIAN: Scale = &[0, 2, 3, 5, 7, 9, 10];
/// Harmonic minor — minor with a raised 7th; a sharp, gothic bite.
pub const HARMONIC_MINOR: Scale = &[0, 2, 3, 5, 7, 8, 11];
/// Phrygian — natural minor with a flat 2nd; tense and claustrophobic.
pub const PHRYGIAN: Scale = &[0, 1, 3, 5, 7, 8, 10];
/// Phrygian dominant — flat 2nd + major 3rd; exotic, aggressive, menacing.
pub const PHRYGIAN_DOMINANT: Scale = &[0, 1, 4, 5, 7, 8, 10];
/// Locrian — flat 2nd *and* a diminished 5th (tritone); maximally unstable.
pub const LOCRIAN: Scale = &[0, 1, 3, 5, 6, 8, 10];

/// One step of a percussion lane (`drums` / `perc`). Rendered from
/// synthesized noise/tones only.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum Drum {
    /// No percussion this step.
    Silent,
    /// Pitched sine thump + a lick of low noise. Drives the side-chain.
    Kick,
    /// Very short high-passed noise tick (closed hat).
    Hat,
    /// Noise burst + a short body tone on the backbeat.
    Snare,
    /// Three tight noise slaps and a short tail — the 808-style hand clap
    /// that layers a snare or answers it.
    Clap,
    /// A longer, sizzling high-passed noise — the open hat on the off-beat.
    OpenHat,
    /// A pitched tom: sine dropping an octave with a knock on top.
    Tom,
    /// Rimshot / click: a tiny bright ping.
    Rim,
    /// A long bright wash — the crash on a downbeat.
    Crash,
}
use Drum::{Kick, Silent};

impl Drum {
    /// Every sounding drum, in bake order (the four-on-the-floor core first).
    pub const KIT: [Drum; 8] = [
        Drum::Kick,
        Drum::Hat,
        Drum::Snare,
        Drum::Clap,
        Drum::OpenHat,
        Drum::Tom,
        Drum::Rim,
        Drum::Crash,
    ];
}

/// One block of an arrangement: a self-contained, multi-bar pattern across all
/// seven channels. Songs are built by ordering these (a refrain section can be
/// listed several times so the hook comes back). A section's playable length is
/// the length of its longest note lane; shorter lanes loop within it.
///
/// Author a section literal with `..Section::EMPTY` so the lanes you don't
/// write (the velocity and chord lanes, typically) default to empty — or
/// build one with [`super::compose`].
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Section {
    /// Human-readable role (intro / verse / refrain / bridge / outro). Purely
    /// documentation + exposed via the tracker API; the scheduler ignores it.
    pub label: &'static str,
    /// Bass lane, one scale-degree (or `REST` / `HOLD`) per step.
    pub bass: &'static [i32],
    /// Lead/melody lane, one scale-degree (or `REST` / `HOLD`) per step.
    pub lead: &'static [i32],
    /// Pad/chord lane: each note blooms into a slow triad; `HOLD` sustains it.
    pub pad: &'static [i32],
    /// Arp lane — a faster, higher counter-melody.
    pub arp: &'static [i32],
    /// Keys lane — stabs, a second lead, a counter-line.
    pub keys: &'static [i32],
    /// Percussion lane, one `Drum` per step.
    pub drums: &'static [Drum],
    /// Second percussion lane (same kit) — for what has to hit together.
    pub perc: &'static [Drum],
    /// Velocity lanes (`0..=MAX_VEL` per step, looping; empty = all full).
    pub bass_vel: &'static [u8],
    pub lead_vel: &'static [u8],
    pub pad_vel: &'static [u8],
    pub arp_vel: &'static [u8],
    pub keys_vel: &'static [u8],
    pub drums_vel: &'static [u8],
    pub perc_vel: &'static [u8],
    /// Chord (voicing) lanes: one [`Chord`] per step, looping; empty = the
    /// lane's default ([`Chord::default_for`]). Read where a note STARTS.
    pub bass_chord: &'static [Chord],
    pub lead_chord: &'static [Chord],
    pub pad_chord: &'static [Chord],
    pub arp_chord: &'static [Chord],
    pub keys_chord: &'static [Chord],
    /// WOBBLE-RATE lanes: the voice's [`Wobble`] period in STEPS per step
    /// (`0` = the voice's own `steps`), looping, read where a note starts —
    /// how a drop goes from an eighth-note wobble to sixteenths mid-bar.
    /// Only a lane whose voice wobbles reads it (it enters the bake key).
    pub bass_wob: &'static [u8],
    pub lead_wob: &'static [u8],
    pub pad_wob: &'static [u8],
    pub arp_wob: &'static [u8],
    pub keys_wob: &'static [u8],
    /// Per-channel LEVEL (gain multipliers, 1.0 = nominal), indexed like
    /// [`CHANNEL_NAMES`]; multiplies every step velocity of the channel.
    /// Applied at schedule time (a play-time gain, never baked into the
    /// note buffers), so it costs nothing in the bake budget.
    pub level: [f32; NUM_CHANNELS],
    /// SIDECHAIN DUCK flag: while this section plays, every kick pumps the
    /// melodic lanes through the song's [`Sidechain`] (no effect when that
    /// is [`Sidechain::OFF`]; the drums never duck themselves).
    pub duck: bool,
    /// Start → end movements of the lanes' LIVE channels across this section
    /// (see [`Ramp`]): a filter opening, a send rising, a fade.
    pub ramps: &'static [Ramp],
}

impl Section {
    /// The all-empty section: the `..Section::EMPTY` base of every literal
    /// (nominal levels; kicks pump whenever the song has a side-chain).
    pub const EMPTY: Section = Section {
        label: "",
        bass: &[],
        lead: &[],
        pad: &[],
        arp: &[],
        keys: &[],
        drums: &[],
        perc: &[],
        bass_vel: &[],
        lead_vel: &[],
        pad_vel: &[],
        arp_vel: &[],
        keys_vel: &[],
        drums_vel: &[],
        perc_vel: &[],
        bass_chord: &[],
        lead_chord: &[],
        pad_chord: &[],
        arp_chord: &[],
        keys_chord: &[],
        bass_wob: &[],
        lead_wob: &[],
        pad_wob: &[],
        arp_wob: &[],
        keys_wob: &[],
        level: [1.0; NUM_CHANNELS],
        duck: true,
        ramps: &[],
    };

    /// The note lane of melodic channel `lane` ([`BASS`] … [`KEYS`]); empty
    /// for the drums or an unknown index.
    pub fn lane(&self, lane: usize) -> &'static [i32] {
        match lane {
            BASS => self.bass,
            LEAD => self.lead,
            PAD => self.pad,
            ARP => self.arp,
            KEYS => self.keys,
            _ => &[],
        }
    }

    /// The chord lane of melodic channel `lane`; empty for the drums.
    pub fn chord_lane(&self, lane: usize) -> &'static [Chord] {
        match lane {
            BASS => self.bass_chord,
            LEAD => self.lead_chord,
            PAD => self.pad_chord,
            ARP => self.arp_chord,
            KEYS => self.keys_chord,
            _ => &[],
        }
    }

    /// The wobble-rate lane of melodic channel `lane`; empty for the drums.
    pub fn wob_lane(&self, lane: usize) -> &'static [u8] {
        match lane {
            BASS => self.bass_wob,
            LEAD => self.lead_wob,
            PAD => self.pad_wob,
            ARP => self.arp_wob,
            KEYS => self.keys_wob,
            _ => &[],
        }
    }

    /// The percussion lane of channel `lane` ([`DRUMS`] / [`PERC`]); empty
    /// for a melodic channel.
    pub fn drum_lane(&self, lane: usize) -> &'static [Drum] {
        match lane {
            DRUMS => self.drums,
            PERC => self.perc,
            _ => &[],
        }
    }

    /// The velocity lane of channel `lane` (all seven).
    pub fn vel_lane(&self, lane: usize) -> &'static [u8] {
        match lane {
            BASS => self.bass_vel,
            LEAD => self.lead_vel,
            PAD => self.pad_vel,
            ARP => self.arp_vel,
            KEYS => self.keys_vel,
            DRUMS => self.drums_vel,
            PERC => self.perc_vel,
            _ => &[],
        }
    }

    /// The level of channel `lane` (1.0 for an unknown index).
    pub fn level_of(&self, lane: usize) -> f32 {
        self.level.get(lane).copied().unwrap_or(1.0)
    }
}

/// A whole song as copyable data. Author one as a Rust file in
/// `src/audio/songs/` — with the [`super::compose`] builders or as `const`
/// section literals —, list it in [`SONGS`], done (see `docs/MUSIC_CODE.md`).
///
/// The key/tempo/voices live here; the *notes* live in the ordered `sections`.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct SongSpec {
    /// Human-readable name (shown in the `?viz` "Musics" tracker).
    pub name: &'static str,
    /// Root/tonic frequency in Hz (e.g. `55.0` = A1). Lower == darker/deeper.
    pub root: f64,
    /// The key/mode: semitone offsets from `root`.
    pub scale: Scale,
    /// Tempo in beats per minute.
    pub bpm: f64,
    /// Sequencer resolution: steps per beat (`4` = sixteenth notes).
    pub steps_per_beat: u32,
    /// The five melodic instruments, indexed by lane ([`BASS`], [`LEAD`],
    /// [`PAD`], [`ARP`], [`KEYS`]).
    pub voices: [Voice; NUM_VOICES],
    /// The arrangement: an ordered list of sections played back to back, then
    /// looped as a whole. This is what makes a song long and developing.
    pub sections: &'static [Section],
    /// Overall punch/loudness feel (~0.5 lounge .. ~1.2 boss).
    pub intensity: f64,
    /// Shuffle, `0.0` (straight sixteenths) … `1.0` (full triplet swing):
    /// see [`swing_delay`].
    pub swing: f64,
    /// The kick-driven ducker on the melodic lanes ([`Sidechain::OFF`] = none),
    /// armed per section by [`Section::duck`].
    pub sidechain: Sidechain,
    /// The shared echo line the voices' `echo` sends feed.
    pub echo: Echo,
    /// Timing HUMANIZE: every note except the kicks lands up to this many
    /// seconds early or late (uniform; clamped to 20 ms). `0.0` = machine
    /// tight; 3–6 ms loosens a groove without smearing it.
    pub humanize: f64,
    /// Depth of the bus lowpass's once-per-bar sweep, `1.0` (the classic
    /// synthwave wah, closing to 420 Hz at the bar lines) … `0.0` (the bus
    /// filter stays open — for songs whose voices carry their own filter
    /// motion).
    pub sweep: f64,
    /// BAKE-time gain of the melodic lanes relative to the drums (`1.0` =
    /// none). Every lane plays through an equal-power `StereoPannerNode`,
    /// which puts a CENTRED lane 3 dB under the drums (they enter the bus
    /// directly): `SQRT_2` makes that up exactly — what the `compose`
    /// builder writes, so a plain centred lane is as loud as it was before
    /// the lane graph existed; a song mixed WITH the panners in place
    /// (the `const`-literal ones) says `1.0`.
    pub melodic_gain: f64,
}

/// Number of songs in [`SONGS`].
pub const SONG_COUNT: usize = 21;

/// How many of [`SONGS`] — the first ones — are the BRIEFED soundtrack
/// (`docs/music/TRACKS.md`): the tracks the game plays by role and the role
/// / length tests pin. The rest are tracker-listed songs without a role.
pub const BRIEFED_SONGS: usize = 7;

/// EVERY SONG — built once on first use, in the `?viz` tracker's order: the
/// briefed soundtrack first (each one a Rust file in `src/audio/songs/`
/// whose memoized `spec()` assembles its arrangement through the
/// [`super::compose`] builders; briefs in `docs/music/TRACKS.md`), then the
/// `const`-literal songs written against the full v2 instrument (stereo
/// voices, ties, velocity / chord lanes, echo + hall sends). The game picks
/// tracks by ROLE — [`title_song`], [`song_for_floor`], [`ending_song`] —
/// never by index.
pub static SONGS: LazyLock<[SongSpec; SONG_COUNT]> = LazyLock::new(|| {
    [
        neon_checksum::spec(),
        walk_dont_run::spec(),
        service_corridor::spec(),
        thermal_mass::spec(),
        signal_rot::spec(),
        crown_of_static::spec(),
        coast_home::spec(),
        insert_coin::spec(),
        neon_lounge::spec(),
        last_exit::spec(),
        sodium_lights::spec(),
        chrome_veins::spec(),
        descent::spec(),
        blood_rush::spec(),
        deep_static::spec(),
        blood_engine::spec(),
        static_prayer::spec(),
        mask_of_dread::spec(),
        salt_road::spec(),
        static_teeth::spec(),
        low_tide::spec(),
    ]
});

/// The title screen's track (what the engine holds before any floor).
pub fn title_song() -> SongSpec {
    neon_checksum::spec()
}

/// The track under the EXFILTRATED uplink and the credits' ride home.
pub fn ending_song() -> SongSpec {
    coast_home::spec()
}

/// The track for a floor, by FLOOR ID (0 = the gate cold open, 1..=13 the
/// tower, 14 = 13½): the soundtrack escalates as you climb — forced calm,
/// the grind, the tower pushing back, the signal rotting, the mask coming
/// off. Total: any id maps to a listed song.
pub fn song_for_floor(floor_id: usize) -> SongSpec {
    match floor_id {
        0 => walk_dont_run::spec(),
        1..=4 => service_corridor::spec(),
        5..=8 => thermal_mass::spec(),
        9..=12 => signal_rot::spec(),
        _ => crown_of_static::spec(),
    }
}

// --- reading the format ------------------------------------------------------

/// The name of a known mode, for the tracker's info line (`"CUSTOM"` for
/// any other set of offsets).
pub fn scale_name(scale: Scale) -> &'static str {
    match scale {
        s if s == MINOR => "MINOR",
        s if s == DORIAN => "DORIAN",
        s if s == HARMONIC_MINOR => "HARMONIC MINOR",
        s if s == PHRYGIAN => "PHRYGIAN",
        s if s == PHRYGIAN_DOMINANT => "PHRYGIAN DOMINANT",
        s if s == LOCRIAN => "LOCRIAN",
        _ => "CUSTOM",
    }
}

/// The nearest note name + octave of a frequency (`55.0` → `"A1"`,
/// `73.42` → `"D2"`), scientific pitch, A4 = 440 Hz.
pub fn note_name(hz: f64) -> String {
    const NAMES: [&str; 12] = [
        "C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B",
    ];
    if hz <= 0.0 || !hz.is_finite() {
        return "?".to_string();
    }
    // Semitones above C0 (16.352 Hz).
    let n = (12.0 * (hz / 16.351_6).log2()).round() as i64;
    let name = NAMES[n.rem_euclid(12) as usize];
    format!("{}{}", name, n.div_euclid(12))
}

/// Does `step` of `sec` fire a kick on either percussion lane? (What
/// retriggers the duck.)
pub fn is_kick_step(sec: &Section, step: usize) -> bool {
    drum_at(sec.drums, step) == Kick || drum_at(sec.perc, step) == Kick
}

/// Resolve a scale-degree (root = 0, +1 = next scale note up, +scale.len() = an
/// octave up, negatives drop below root) to a frequency in Hz, in-key.
pub fn degree_freq(root: f64, scale: Scale, degree: i32) -> f64 {
    if scale.is_empty() {
        return root;
    }
    let n = scale.len() as i32;
    let octave = degree.div_euclid(n);
    let idx = degree.rem_euclid(n) as usize;
    let semitones = octave * 12 + scale[idx];
    root * 2f64.powf(semitones as f64 / 12.0)
}

/// A note starting at some step of a melodic lane.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct NoteOn {
    /// Scale degree.
    pub degree: i32,
    /// Length in steps: 1 + the `HOLD`s tied onto it.
    pub len: u16,
    /// The degree of the lane's previous note when it runs right into this
    /// one (its tail ends where this starts, no rest between) and differs —
    /// what a legato glide comes from. `None` after a rest, at the loop's
    /// first note of an otherwise empty lane, or for a repeated pitch.
    pub from: Option<i32>,
}

/// Read a melodic lane at `step` (patterns loop): `Some` only where a note
/// STARTS — a `REST`, a `HOLD` (the tail of an earlier note) or an empty
/// lane is `None`. The length counts the `HOLD`s that follow, wrapping
/// around the looping lane, so a note tied across the lane's end sustains
/// into its next repeat.
pub fn note_at(pattern: &[i32], step: usize) -> Option<NoteOn> {
    if pattern.is_empty() {
        return None;
    }
    let n = pattern.len();
    let degree = pattern[step % n];
    if degree == REST || degree == HOLD {
        return None;
    }
    let mut len = 1;
    while len < n && pattern[(step + len) % n] == HOLD {
        len += 1;
    }
    // Legato: walk back over the previous note's HOLDs to its start; a
    // REST anywhere on the way (or nothing but HOLDs) means no glide.
    let mut from = None;
    for back in 1..n {
        match pattern[(step + n - back) % n] {
            HOLD => continue,
            REST => break,
            d => {
                if d != degree {
                    from = Some(d);
                }
                break;
            }
        }
    }
    Some(NoteOn {
        degree,
        len: len.min(u16::MAX as usize) as u16,
        from,
    })
}

/// Read a velocity lane at `step` (loops): `MAX_VEL` for an empty lane,
/// otherwise the step's value clamped to `MAX_VEL`.
pub fn vel_at(vels: &[u8], step: usize) -> u8 {
    if vels.is_empty() {
        return MAX_VEL;
    }
    vels[step % vels.len()].min(MAX_VEL)
}

/// Read a chord lane at `step` (loops): the lane's default voicing for an
/// empty lane.
pub fn chord_at(lane: usize, chords: &[Chord], step: usize) -> Chord {
    if chords.is_empty() {
        return Chord::default_for(lane);
    }
    chords[step % chords.len()]
}

/// Read a wobble-rate lane at `step` (loops): `0` (the voice's own rate)
/// for an empty lane.
pub fn wob_at(wobs: &[u8], step: usize) -> u8 {
    if wobs.is_empty() {
        return 0;
    }
    wobs[step % wobs.len()]
}

/// Read the drum lane at `step` (loops). Empty lane == `Silent`.
pub fn drum_at(pattern: &[Drum], step: usize) -> Drum {
    if pattern.is_empty() {
        return Silent;
    }
    pattern[step % pattern.len()]
}

/// The playable length of a section: its longest note lane (shorter lanes
/// loop inside it). Always at least 1 so the scheduler can never divide by
/// zero. Velocity and chord lanes don't count — they only decorate notes.
pub fn section_len(sec: &Section) -> usize {
    sec.bass
        .len()
        .max(sec.lead.len())
        .max(sec.pad.len())
        .max(sec.arp.len())
        .max(sec.keys.len())
        .max(sec.drums.len())
        .max(sec.perc.len())
        .max(1)
}

/// Length of one sequencer step (seconds) for the song's tempo.
pub fn step_dur(song: &SongSpec) -> f64 {
    let spb = song.steps_per_beat.max(1) as f64;
    60.0 / song.bpm.max(1.0) / spb
}

/// Number of steps in one bar (used to pace the per-bar filter sweep).
/// Assumes a 4-beat bar.
pub fn bar_steps(song: &SongSpec) -> usize {
    (song.steps_per_beat.max(1) as usize) * 4
}

/// A lane's built-in note shape: `(gate, level, attack)` — the decay tail
/// in STEPS an untied note rings, its mix level, and its attack in seconds.
/// (The pad level is per chord: its default triad lands each partial at
/// 0.78/√3 = 0.45 after the 1/√n split.)
pub fn lane_shape(lane: usize) -> (f64, f64, f64) {
    match lane {
        BASS => (1.9, 1.3, 0.005),
        LEAD => (0.9, 1.0, 0.005),
        PAD => (4.0, 0.78, 0.06),
        KEYS => (1.2, 0.8, 0.005),
        _ => (0.7, 0.7, 0.005),
    }
}

/// The lane's shape with the song voice's [`Env`] override applied.
pub fn voice_shape(song: &SongSpec, lane: usize) -> (f64, f64, f64) {
    let (gate, level, attack) = lane_shape(lane);
    match song.voices.get(lane).and_then(|v| v.env) {
        Some(e) => (e.gate.max(0.05), level, e.attack.max(0.0)),
        None => (gate, level, attack),
    }
}

/// Seconds of signal one baked voice needs: a note's attack + its tied
/// hold + the lane's decay tail (plus the builders' 30 ms stop margin), or
/// the longest layer of a kit piece.
pub fn key_seconds(song: &SongSpec, key: MusicKey) -> f64 {
    match key {
        MusicKey::Note { lane, len, .. } => {
            let (gate, _, attack) = voice_shape(song, lane);
            attack + step_dur(song) * (gate + f64::from(len.max(1) - 1)) + 0.03
        }
        MusicKey::Drum(d) => match d {
            Drum::Kick => 0.21,  // 0.18 s tone + stop margin (noise is 0.05)
            Drum::Hat => 0.06,   // 0.03 s noise tick + margin
            Drum::Snare => 0.16, // 0.13 s noise + margin (tone is 0.10)
            Drum::Clap => 0.20,
            Drum::OpenHat => 0.31,
            Drum::Tom => 0.31,
            Drum::Rim => 0.06,
            Drum::Crash => 1.0,
            Drum::Silent => 0.03,
        },
    }
}

/// What a tracker cell shows for one channel at one step.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum GridCell {
    /// Nothing sounds (a rest, or an empty lane).
    Off,
    /// A note (or drum) starts here, at this velocity.
    On(u8),
    /// A note started earlier is tied through this step.
    Hold,
}

/// Sample the tracker cell of `channel` (see [`CHANNEL_NAMES`]) at `step`
/// within `sec`.
pub fn cell_at(sec: &Section, channel: usize, step: usize) -> GridCell {
    if channel == DRUMS || channel == PERC {
        return match drum_at(sec.drum_lane(channel), step) {
            Silent => GridCell::Off,
            _ => GridCell::On(vel_at(sec.vel_lane(channel), step)),
        };
    }
    let lane = sec.lane(channel);
    if lane.is_empty() {
        return GridCell::Off;
    }
    match lane[step % lane.len()] {
        REST => GridCell::Off,
        HOLD => {
            // A HOLD only sustains if some note precedes it in the loop.
            if lane.iter().any(|&d| d != REST && d != HOLD) {
                GridCell::Hold
            } else {
                GridCell::Off
            }
        }
        _ => GridCell::On(vel_at(sec.vel_lane(channel), step)),
    }
}

/// Compact density summary of a section: the fraction (0.0..=1.0) of all
/// grid cells that carry a note/hit or a tie. A cheap way to shade each
/// miniature by how busy/intense it is without drawing every cell.
pub fn section_density(sec: &Section) -> f32 {
    let steps = section_len(sec);
    let mut active = 0usize;
    for step in 0..steps {
        for chan in 0..NUM_CHANNELS {
            if cell_at(sec, chan, step) != GridCell::Off {
                active += 1;
            }
        }
    }
    active as f32 / (steps * NUM_CHANNELS) as f32
}

// --- the playhead ------------------------------------------------------------

/// Where the sequencer is inside a song's arrangement: the section index and
/// the step inside it. Pure state — the engine owns one next to its audio
/// clock and asks it what to schedule.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct Playhead {
    /// Index of the currently-playing section within the song's arrangement.
    pub section: usize,
    /// Current step index inside the currently-playing section.
    pub step: usize,
}

impl Playhead {
    /// The start of the arrangement.
    pub const START: Playhead = Playhead {
        section: 0,
        step: 0,
    };

    /// The section the playhead is in, if the arrangement is non-empty.
    pub fn section_ref<'a>(&self, song: &'a SongSpec) -> Option<&'a Section> {
        song.sections.get(self.section)
    }

    /// Number of steps before the *current section* repeats.
    pub fn loop_len(&self, song: &SongSpec) -> usize {
        self.section_ref(song).map(section_len).unwrap_or(1)
    }

    /// Is the current step the first of a bar (where the per-bar filter
    /// sweep is armed)?
    pub fn at_bar_start(&self, song: &SongSpec) -> bool {
        self.step.is_multiple_of(bar_steps(song))
    }

    /// Move to the next step; when the current section's longest lane ends,
    /// advance to the next section of the arrangement (wrapping back to the
    /// first when the play-through completes). Returns `true` when a section
    /// boundary was crossed.
    pub fn advance(&mut self, song: &SongSpec) -> bool {
        self.step += 1;
        if self.step < self.loop_len(song) {
            return false;
        }
        self.step = 0;
        let n = song.sections.len();
        self.section = if n == 0 { 0 } else { (self.section + 1) % n };
        true
    }

    /// Jump to `step` within the current section (wrapped); the section is
    /// not changed.
    pub fn seek(&mut self, song: &SongSpec, step: usize) {
        let loop_len = self.loop_len(song);
        self.step = if loop_len == 0 { 0 } else { step % loop_len };
    }

    /// Jump to the start of section `i` in the arrangement, clamped into
    /// range.
    pub fn jump_to_section(&mut self, song: &SongSpec, i: usize) {
        let n = song.sections.len();
        self.section = if n == 0 { 0 } else { i.min(n - 1) };
        self.step = 0;
    }

    /// The step currently *sounding* when the scheduler has already queued
    /// `ahead` steps past this playhead (the look-ahead), wrapped inside the
    /// current section — for drawing the moving tracker playhead.
    pub fn sounding_step(&self, song: &SongSpec, ahead: usize) -> usize {
        let loop_len = self.loop_len(song);
        if loop_len == 0 {
            return 0;
        }
        (self.step + loop_len - ahead % loop_len) % loop_len
    }
}

// --- the bakeable voice set ---------------------------------------------------

/// One pre-renderable MUSIC voice: what the engine bakes once per song and
/// fires per scheduled note. Velocity is NOT part of the key (it is a
/// play-time gain); pitch, LENGTH and voicing are (the envelope and the
/// chord are baked in). The set of keys a song can ever schedule is FINITE —
/// its lanes are static pattern data — so [`music_keys`] enumerates it
/// exactly and each key is baked at its exact pitch (no `playback_rate`
/// transposition: the timbre is untouched).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum MusicKey {
    /// A melodic lane ([`MELODIC`]) note at this scale degree, this many
    /// steps long (1 = untied), voiced as `chord`, gliding in from `from`
    /// (only ever `Some` on a lane whose voice glides), its wobble at `wob`
    /// steps a period (`0` = the voice's own; only ever non-zero on a lane
    /// whose voice wobbles).
    Note {
        lane: usize,
        degree: i32,
        len: u16,
        chord: Chord,
        from: Option<i32>,
        wob: u8,
    },
    /// One kit piece (never `Silent`) — shared by both percussion lanes.
    Drum(Drum),
}

/// Enumerate the exact, finite voice set `song` can ever schedule: the
/// distinct (degree, length, voicing) triples of each melodic lane across
/// every section, plus the kit pieces its percussion lanes use — in
/// bake-priority order (drums first — the densest lanes — then the melodic
/// lanes per [`MELODIC`]). Typically 15–60 keys per song.
pub fn music_keys(song: &SongSpec) -> Vec<MusicKey> {
    fn add(keys: &mut Vec<MusicKey>, k: MusicKey) {
        if !keys.contains(&k) {
            keys.push(k);
        }
    }
    let mut keys = Vec::new();
    // Drums in kit order (kick first), only the pieces the song uses.
    for drum in Drum::KIT {
        let used = song
            .sections
            .iter()
            .any(|sec| sec.drums.contains(&drum) || sec.perc.contains(&drum));
        if used {
            add(&mut keys, MusicKey::Drum(drum));
        }
    }
    for lane in MELODIC {
        for sec in song.sections {
            let pattern = sec.lane(lane);
            if pattern.is_empty() {
                continue;
            }
            // Over the SECTION's steps, not the lane's: a lane looping under
            // a chord lane of another length meets other voicings on its
            // later passes, and those are keys too.
            for step in 0..section_len(sec) {
                if let Some(n) = note_at(pattern, step) {
                    add(&mut keys, note_key(song, sec, lane, step, &n));
                }
            }
        }
    }
    keys
}

/// The bake key of the note `n` starting at `step` of `lane` in `sec`: its
/// voicing from the chord lane, its glide origin only if the lane's voice
/// glides, its wobble rate only if the voice wobbles (so a lane never
/// multiplies its keys by context it cannot hear).
pub fn note_key(song: &SongSpec, sec: &Section, lane: usize, step: usize, n: &NoteOn) -> MusicKey {
    let voice = song.voices.get(lane);
    let glides = voice.is_some_and(|v| v.glides());
    let wobbles = voice.is_some_and(|v| v.wobble.is_some());
    MusicKey::Note {
        lane,
        degree: n.degree,
        len: n.len,
        chord: chord_at(lane, sec.chord_lane(lane), step),
        from: if glides { n.from } else { None },
        wob: if wobbles {
            wob_at(sec.wob_lane(lane), step)
        } else {
            0
        },
    }
}

// --- shared patterns ----------------------------------------------------------

/// A shared PERC ride for the driving songs' refrains: closed hats on the
/// off sixteenths (between the drum lane's own even-step hats — never
/// doubling them) and an open hat pushing into the next bar.
pub const PERC_RIDE: &[Drum] = &[
    Drum::Silent,
    Drum::Hat,
    Drum::Silent,
    Drum::Hat,
    Drum::Silent,
    Drum::Hat,
    Drum::Silent,
    Drum::Hat,
    Drum::Silent,
    Drum::Hat,
    Drum::Silent,
    Drum::Hat,
    Drum::Silent,
    Drum::Hat,
    Drum::Silent,
    Drum::OpenHat,
];
/// The velocities of [`PERC_RIDE`]: ghosted hats, the open hat a touch up.
pub const PERC_RIDE_VEL: &[u8] = &[0, 4, 0, 3, 0, 4, 0, 3, 0, 4, 0, 3, 0, 4, 0, 5];

#[cfg(test)]
mod tests {
    use super::Drum::{Clap, Crash, Hat, OpenHat, Rim, Snare, Tom};
    use super::*;

    /// FNV-1a of a song's full `Debug` form: every lane, velocity, chord,
    /// level, voice and setting. Two songs with the same fingerprint are the
    /// same data — what pins a song REWRITTEN from `const` literals to the
    /// `compose` builders to exactly the notes it had.
    fn fingerprint(song: &SongSpec) -> u64 {
        format!("{song:?}")
            .bytes()
            .fold(0xcbf2_9ce4_8422_2325u64, |h, b| {
                (h ^ u64::from(b)).wrapping_mul(0x0100_0000_01b3)
            })
    }

    /// "Sodium Lights" was written as `const` literals (the v2 format's tour)
    /// and REWRITTEN with the `compose` v2 builders: the rewrite is the same
    /// song, to the last velocity digit. (A FORMAT change — a new `Voice`
    /// or `Section` field — changes the `Debug` form and re-pins this; a
    /// change to the song's notes must never be what re-pins it.)
    #[test]
    fn the_compose_rewrite_of_sodium_lights_is_the_same_song() {
        let song = SONGS.iter().find(|s| s.name == "Sodium Lights").unwrap();
        println!("Sodium Lights fingerprint: {:#x}", fingerprint(song));
        assert_eq!(fingerprint(song), 0x485f_adbd_4f30_8815);
    }

    /// The briefed soundtrack (the tracks the game plays by role).
    fn briefed() -> &'static [SongSpec] {
        &SONGS[..BRIEFED_SONGS]
    }

    /// Every note any song can ever schedule must map to an enumerated
    /// [`MusicKey`], and the per-song voice set must stay small enough that
    /// baking each exact pitch × length × voicing (no `playback_rate`
    /// transposition) is cheap. Run with `--nocapture` for the counts.
    #[test]
    fn music_voice_sets_are_small_and_complete() {
        for song in SONGS.iter() {
            let keys = music_keys(song);
            assert!(!keys.is_empty(), "{}: empty voice set", song.name);
            assert!(
                keys.len() <= 96,
                "{}: {} voices — too many to bake each exact pitch",
                song.name,
                keys.len()
            );
            // No duplicates (the queue bakes each key exactly once).
            for (i, k) in keys.iter().enumerate() {
                assert!(!keys[..i].contains(k), "{}: duplicate {:?}", song.name, k);
            }
            // Completeness: every note the SCHEDULER can ask for (it reads
            // at the section's step, lanes looping under it) has a key.
            for sec in song.sections {
                for lane in MELODIC {
                    let p = sec.lane(lane);
                    for step in 0..section_len(sec) {
                        if let Some(n) = note_at(p, step) {
                            let key = note_key(song, sec, lane, step, &n);
                            assert!(keys.contains(&key), "{}: missing {:?}", song.name, key);
                        }
                    }
                }
                for &dr in sec.drums.iter().chain(sec.perc) {
                    if dr != Silent {
                        assert!(
                            keys.contains(&MusicKey::Drum(dr)),
                            "{}: {:?}",
                            song.name,
                            dr
                        );
                    }
                }
            }
            let count = |f: fn(&MusicKey) -> bool| keys.iter().filter(|k| f(k)).count();
            println!(
                "{:16} {:2} voices (drums {} bass {:2} lead {:2} arp {:2} keys {:2} pad {:2})",
                song.name,
                keys.len(),
                count(|k| matches!(k, MusicKey::Drum(_))),
                count(|k| matches!(k, MusicKey::Note { lane: BASS, .. })),
                count(|k| matches!(k, MusicKey::Note { lane: LEAD, .. })),
                count(|k| matches!(k, MusicKey::Note { lane: ARP, .. })),
                count(|k| matches!(k, MusicKey::Note { lane: KEYS, .. })),
                count(|k| matches!(k, MusicKey::Note { lane: PAD, .. })),
            );
        }
    }

    /// A lane looping under a chord lane of another length meets other
    /// voicings on its later passes: those are keys too (the scheduler reads
    /// both at the SECTION's step).
    #[test]
    fn looping_lanes_meet_every_chord_lane_step() {
        const SEC: Section = Section {
            lead: &[0, REST],
            lead_chord: &[Chord::Single, Chord::Single, Chord::Power, Chord::Single],
            drums: &[Silent; 4],
            ..Section::EMPTY
        };
        let song = SongSpec {
            sections: &[SEC],
            ..SONGS[0]
        };
        let keys = music_keys(&song);
        let n = note_at(SEC.lead, 2).unwrap();
        assert!(keys.contains(&note_key(&song, &SEC, LEAD, 2, &n)));
        assert_eq!(keys.len(), 2, "{keys:?}");
    }

    /// Every section of every song must be whole bars long (its longest lane a
    /// multiple of 16 steps at 4 steps/beat, 4 beats/bar): `Playhead::advance`
    /// moves on when the LONGEST lane ends, so a ragged longest lane would
    /// shift every later section off the beat grid. Short lanes may still be
    /// any length — they loop inside the section (that is how the raindrop layers'
    /// 12-step motif phases 3-against-4) — and every arrangement is non-empty.
    #[test]
    fn sections_are_bar_aligned() {
        for song in SONGS.iter() {
            assert!(
                !song.sections.is_empty(),
                "{}: empty arrangement",
                song.name
            );
            let bar = bar_steps(song);
            for sec in song.sections {
                let len = section_len(sec);
                assert!(
                    len >= bar && len.is_multiple_of(bar),
                    "{}: section '{}' is {} steps — not whole bars of {}",
                    song.name,
                    sec.label,
                    len,
                    bar
                );
            }
        }
    }

    /// The soundtrack's ROLES: the title, the ending and every floor id
    /// map to the briefed tracks (`docs/music/TRACKS.md`), every briefed
    /// track has a role (the songs listed after them are tracker-only) and
    /// the ending is the calmest of the briefed tracks.
    #[test]
    fn soundtrack_roles_follow_the_briefs() {
        assert_eq!(title_song().name, "Neon Checksum");
        assert_eq!(ending_song().name, "Coast Home");
        let floor_track = |id: usize| song_for_floor(id).name;
        assert_eq!(floor_track(0), "Walk Don't Run");
        for id in 1..=4 {
            assert_eq!(floor_track(id), "Service Corridor", "floor {id}");
        }
        for id in 5..=8 {
            assert_eq!(floor_track(id), "Thermal Mass", "floor {id}");
        }
        for id in 9..=12 {
            assert_eq!(floor_track(id), "Signal Rot", "floor {id}");
        }
        for id in [13, 14, 99] {
            assert_eq!(floor_track(id), "Crown of Static", "floor {id}");
        }
        for song in briefed() {
            let has_role = song.name == title_song().name
                || song.name == ending_song().name
                || (0..32).any(|id| song_for_floor(id).name == song.name);
            assert!(has_role, "{} has no role in the game", song.name);
        }
        let calmest = briefed()
            .iter()
            .min_by(|a, b| a.intensity.total_cmp(&b.intensity))
            .unwrap();
        assert_eq!(calmest.name, ending_song().name);
    }

    /// Seconds one play-through of `song` lasts.
    fn song_secs(song: &SongSpec) -> f64 {
        let steps: usize = song.sections.iter().map(section_len).sum();
        steps as f64 * step_dur(song)
    }

    /// Every briefed track runs the length its brief asks for — the
    /// soundtrack range is 1:30 to 5:00, and each track's own target (±20 s)
    /// is pinned so a rewrite cannot quietly shrink one back to a 30-second
    /// loop. Run with `--nocapture` for every song's length.
    #[test]
    fn tracks_run_the_briefed_length() {
        const TARGET_SECS: [(&str, f64); BRIEFED_SONGS] = [
            ("Neon Checksum", 150.0),
            ("Walk Don't Run", 120.0),
            ("Service Corridor", 180.0),
            ("Thermal Mass", 210.0),
            ("Signal Rot", 210.0),
            ("Crown of Static", 270.0),
            ("Coast Home", 150.0),
        ];
        for (name, target) in TARGET_SECS {
            let song = briefed()
                .iter()
                .find(|s| s.name == name)
                .unwrap_or_else(|| panic!("{name} not in the briefed SONGS"));
            let secs = song_secs(song);
            assert!((90.0..=300.0).contains(&secs), "{name}: {secs:.1} s");
            assert!(
                (secs - target).abs() <= 20.0,
                "{name}: {secs:.1} s, brief says ~{target} s"
            );
        }
        for song in SONGS.iter() {
            println!(
                "{:16} {:5.1} s ({:2} sections)",
                song.name,
                song_secs(song),
                song.sections.len()
            );
        }
    }

    /// The darksynth-led tracks PUMP (at least one ducked section; every
    /// section with a kick in the pure darksynth ones) and the wave-led
    /// ones never do — the duck is a genre marker, not a default.
    #[test]
    fn the_duck_follows_the_genre() {
        let find = |name: &str| *SONGS.iter().find(|s| s.name == name).unwrap();
        let ducked = |name: &str| find(name).sections.iter().filter(|s| s.duck).count();
        for name in [
            "Service Corridor",
            "Thermal Mass",
            "Signal Rot",
            "Crown of Static",
        ] {
            assert!(ducked(name) > 0, "{name} never pumps");
            assert!(find(name).sidechain.active(), "{name}: no side-chain");
        }
        for name in ["Service Corridor", "Thermal Mass"] {
            for sec in find(name).sections {
                let kicked = sec.drums.contains(&Kick);
                assert!(
                    !kicked || sec.duck,
                    "{name}: '{}' has a kick and no duck",
                    sec.label
                );
            }
        }
        for name in ["Walk Don't Run", "Coast Home"] {
            assert_eq!(ducked(name), 0, "{name} pumps — wave doesn't");
        }
    }

    /// Every floor's song is one of the listed songs (the `?viz` tracker can
    /// show whatever is playing), song names are unique (the engine detects
    /// a song switch by name) and every song is playable data: positive
    /// tempo / root, a non-empty scale, every note in-key resolvable, every
    /// song-level and voice setting inside the range the engine assumes, no
    /// `HOLD` that has nothing to hold, velocities within range.
    #[test]
    fn songs_are_well_formed_and_floor_mapping_is_listed() {
        for floor in 0..32 {
            let s = song_for_floor(floor);
            assert!(SONGS.iter().any(|x| x.name == s.name), "floor {floor}");
        }
        for (i, song) in SONGS.iter().enumerate() {
            let name = song.name;
            assert!(
                !SONGS[..i].iter().any(|x| x.name == name),
                "duplicate song name {name}"
            );
            assert!(song.bpm > 0.0 && song.root > 0.0 && song.steps_per_beat >= 1);
            assert!(!song.scale.is_empty());
            assert!(step_dur(song) > 0.0 && step_dur(song) < 1.0);
            assert!((0.0..=1.0).contains(&song.swing), "{name}: swing");
            assert!((0.0..=1.0).contains(&song.sidechain.depth), "{name}: duck");
            assert!(song.sidechain.release_beats > 0.0, "{name}: duck release");
            assert!(
                song.echo.steps > 0.0 && song.echo.tone > 0.0,
                "{name}: echo"
            );
            assert!((0.0..0.95).contains(&song.echo.feedback), "{name}: echo fb");
            assert!((0.0..=1.0).contains(&song.sweep), "{name}: sweep");
            assert!((0.0..=0.02).contains(&song.humanize), "{name}: humanize");
            assert!(
                (0.25..=4.0).contains(&song.melodic_gain),
                "{name}: melodic gain"
            );
            for v in song.voices {
                assert!((-1.0..=1.0).contains(&v.pan), "{name}: pan");
                if let Some(b) = v.bend {
                    assert!(
                        b.semitones.abs() <= 48.0 && b.seconds >= 0.0,
                        "{name}: bend"
                    );
                }
                if let Some(w) = v.wobble {
                    assert!(
                        w.steps > 0.0 && w.cutoff >= 20.0 && w.peak >= w.cutoff && w.q > 0.0,
                        "{name}: wobble"
                    );
                }
                assert!((0.0..=1.0).contains(&v.width), "{name}: width");
                assert!(v.detune >= 0.0, "{name}: detune");
                assert!((1..=7).contains(&v.unison), "{name}: unison");
                assert!((0.0..=1.0).contains(&v.drive), "{name}: drive");
                assert!((0.0..=1.0).contains(&v.echo), "{name}: echo send");
                assert!((0.0..=1.0).contains(&v.reverb), "{name}: reverb send");
                assert!((0.0..=1.0).contains(&v.sub), "{name}: sub");
                assert!(v.glide >= 0.0, "{name}: glide");
                if let Some(e) = v.env {
                    assert!(e.attack >= 0.0 && e.gate > 0.0, "{name}: env");
                }
                if let Some(f) = v.filter {
                    assert!(f.cutoff >= 20.0 && f.peak >= f.cutoff, "{name}: filter");
                    assert!(f.attack >= 0.0 && f.decay >= 0.0 && f.q > 0.0, "{name}");
                }
                if let Some(vb) = v.vibrato {
                    assert!(
                        vb.rate > 0.0 && vb.depth >= 0.0 && vb.delay >= 0.0,
                        "{name}"
                    );
                }
            }
            for sec in song.sections {
                for lane in MELODIC {
                    let p = sec.lane(lane);
                    for &d in p {
                        if d != REST && d != HOLD {
                            for &interval in Chord::Add9.degrees() {
                                let f = degree_freq(song.root, song.scale, d + interval);
                                assert!(f.is_finite() && f > 0.0 && f < 20_000.0);
                            }
                        }
                    }
                    // An all-REST lane is fine (it pads the section's
                    // length); a HOLD with no note anywhere to hold is a typo.
                    if p.contains(&HOLD) {
                        assert!(
                            p.iter().any(|&d| d != REST && d != HOLD),
                            "{name} / {}: lane {} ties nothing",
                            sec.label,
                            CHANNEL_NAMES[lane]
                        );
                    }
                }
                for ch in 0..NUM_CHANNELS {
                    for &v in sec.vel_lane(ch) {
                        assert!(v <= MAX_VEL, "{name} / {}: velocity {v}", sec.label);
                    }
                    assert!(sec.level_of(ch) >= 0.0 && sec.level_of(ch).is_finite());
                }
                for r in sec.ramps {
                    assert!(
                        (1..=song.sections.len() as u32).contains(&r.span),
                        "{name}: ramp span"
                    );
                    assert!(
                        MELODIC.contains(&r.lane),
                        "{name} / {}: ramp lane",
                        sec.label
                    );
                    let (lo, hi) = match r.param {
                        RampParam::Cutoff => (20.0, 20_000.0),
                        RampParam::Pan => (-1.0, 1.0),
                        _ => (0.0, 1.0),
                    };
                    for v in [r.from, r.to] {
                        assert!(
                            (lo..=hi).contains(&v),
                            "{name} / {}: ramp {:?} {v}",
                            sec.label,
                            r.param
                        );
                    }
                }
                let density = section_density(sec);
                assert!((0.0..=1.0).contains(&density));
            }
        }
    }

    /// In-key pitch math: octaves double, the root is degree 0, negative
    /// degrees wrap below the root.
    #[test]
    fn degree_freq_is_in_key() {
        assert_eq!(degree_freq(55.0, MINOR, 0), 55.0);
        assert!((degree_freq(55.0, MINOR, 7) - 110.0).abs() < 1e-9);
        assert!((degree_freq(55.0, MINOR, -7) - 27.5).abs() < 1e-9);
        // Degree 2 of A minor is C (3 semitones up).
        let c = 55.0 * 2f64.powf(3.0 / 12.0);
        assert!((degree_freq(55.0, MINOR, 2) - c).abs() < 1e-9);
        assert!((degree_freq(55.0, MINOR, -5) - c / 2.0).abs() < 1e-9);
        assert_eq!(degree_freq(55.0, &[], 3), 55.0);
    }

    /// A note's length is 1 + the HOLDs tied onto it (wrapping around the
    /// looping lane); a HOLD or a REST is not a note start.
    #[test]
    fn ties_extend_the_note_they_follow() {
        let on = |degree, len, from| Some(NoteOn { degree, len, from });
        let lane = [0, HOLD, HOLD, HOLD, 3, REST, HOLD, 5];
        // The lane loops, so the 0 runs straight on from the 5 at its end.
        assert_eq!(note_at(&lane, 0), on(0, 4, Some(5)));
        assert_eq!(note_at(&lane, 1), None, "a HOLD is not a note start");
        // 3 starts right where the held 0 ends: legato from 0.
        assert_eq!(note_at(&lane, 4), on(3, 1, Some(0)));
        assert_eq!(note_at(&lane, 5), None);
        assert_eq!(note_at(&lane, 6), None, "a HOLD after a REST is silent");
        // Counting wraps around the lane but stops at the next note start
        // (step 0 here), so the last note is one step long.
        assert_eq!(note_at(&lane, 7), on(5, 1, None));
        // Wrapping tie: a note at the end sustains into the lane's repeat —
        // and it runs straight on from the 2 before it (legato).
        let wrap = [HOLD, HOLD, 2, 4];
        assert_eq!(note_at(&wrap, 3), on(4, 3, Some(2)));
        // Steps beyond the lane length loop.
        assert_eq!(note_at(&wrap, 7), note_at(&wrap, 3));
        assert_eq!(note_at(&[], 0), None);
        // An all-HOLD lane never sounds (and never loops forever counting).
        assert_eq!(note_at(&[HOLD, HOLD], 0), None);
    }

    /// A glide origin needs two DIFFERENT notes that touch, and it only
    /// enters the bake key on a gliding voice.
    #[test]
    fn legato_origin_needs_touching_different_notes() {
        let touching = [0, 3, 3, REST, 5, HOLD, 7];
        let from = |step| note_at(&touching, step).and_then(|n| n.from);
        assert_eq!(from(1), Some(0));
        assert_eq!(from(2), None, "same pitch");
        assert_eq!(from(4), None, "after a rest");
        assert_eq!(from(6), Some(5), "after a tie");
        const SEC: Section = Section {
            lead: &[0, 3, 0, 3],
            ..Section::EMPTY
        };
        let mut voices = [Voice::mono(Wave::Square); NUM_VOICES];
        let plain = SongSpec {
            sections: &[SEC],
            voices,
            ..SONGS[0]
        };
        assert_eq!(music_keys(&plain).len(), 2);
        voices[LEAD] = voices[LEAD].with_glide(0.1);
        let gliding = SongSpec { voices, ..plain };
        // 0←3 and 3←0 (each is reached from the other around the loop).
        assert_eq!(music_keys(&gliding).len(), 2);
        assert!(music_keys(&gliding)
            .iter()
            .all(|k| matches!(k, MusicKey::Note { from: Some(_), .. })));
        // A PRESET never glides: its origin stays out of the key.
        voices[LEAD] = Voice::mono(Wave::Supersaw).with_glide(0.1);
        let preset = SongSpec { voices, ..plain };
        assert!(music_keys(&preset)
            .iter()
            .all(|k| matches!(k, MusicKey::Note { from: None, .. })));
    }

    /// Lanes loop, empty lanes read as silence / full velocity / the lane's
    /// default voicing; a section is as long as its longest NOTE lane.
    #[test]
    fn lanes_loop_and_empty_lanes_are_silent() {
        assert_eq!(note_at(&[0, REST, 3], 4), None);
        assert_eq!(note_at(&[0, REST, 3], 5).map(|n| n.degree), Some(3));
        assert_eq!(drum_at(&[Kick, Silent], 2), Kick);
        assert_eq!(drum_at(&[], 9), Silent);
        assert_eq!(vel_at(&[], 12), MAX_VEL);
        assert_eq!(vel_at(&[3, 6, 8, 9], 0), 3);
        assert_eq!(vel_at(&[3, 6, 8, 9], 7), 9);
        assert_eq!(vel_at(&[200], 0), MAX_VEL, "clamped");
        assert_eq!(chord_at(PAD, &[], 5), Chord::Triad);
        assert_eq!(chord_at(LEAD, &[], 5), Chord::Single);
        assert_eq!(
            chord_at(LEAD, &[Chord::Power, Chord::Octave], 3),
            Chord::Octave
        );
        assert_eq!(section_len(&Section::EMPTY), 1);
        let sec = Section {
            label: "t",
            bass: &[0],
            pad: &[REST; 32],
            drums: &[Kick],
            bass_vel: &[9; 64],
            ..Section::EMPTY
        };
        assert_eq!(section_len(&sec), 32, "velocity lanes don't count");
        assert_eq!(cell_at(&sec, BASS, 31), GridCell::On(MAX_VEL));
        assert_eq!(cell_at(&sec, PAD, 3), GridCell::Off);
        assert_eq!(cell_at(&sec, DRUMS, 7), GridCell::On(MAX_VEL));
        assert_eq!(cell_at(&sec, NUM_CHANNELS, 0), GridCell::Off);
    }

    /// A wobble-rate lane changes the bake key only on a lane whose voice
    /// wobbles; the rate loops like every lane.
    #[test]
    fn wobble_rate_lanes_enter_the_key_of_wobbling_voices_only() {
        const SEC: Section = Section {
            bass: &[0, 0, 0, 0],
            bass_wob: &[2, 2, 1, 1],
            ..Section::EMPTY
        };
        let mut voices = [Voice::mono(Wave::Sawtooth); NUM_VOICES];
        let plain = SongSpec {
            sections: &[SEC],
            voices,
            ..SONGS[0]
        };
        assert_eq!(music_keys(&plain).len(), 1);
        voices[BASS] = Voice::mono(Wave::Reese).with_wobble(4.0, 100.0, 2000.0, 4.0);
        let wobbling = SongSpec { voices, ..plain };
        let keys = music_keys(&wobbling);
        assert_eq!(keys.len(), 2);
        assert!(keys
            .iter()
            .any(|k| matches!(k, MusicKey::Note { wob: 2, .. })));
        assert!(keys
            .iter()
            .any(|k| matches!(k, MusicKey::Note { wob: 1, .. })));
        assert_eq!(wob_at(&[], 7), 0);
        assert_eq!(wob_at(&[2, 1], 3), 1);
    }

    #[test]
    fn tracker_cells_reflect_notes_ties_and_velocity() {
        let sec = Section {
            lead: &[7, HOLD, REST, 9],
            lead_vel: &[9, 9, 9, 4],
            drums: &[Kick, Silent],
            drums_vel: &[6],
            ..Section::EMPTY
        };
        assert_eq!(cell_at(&sec, LEAD, 0), GridCell::On(9));
        assert_eq!(cell_at(&sec, LEAD, 1), GridCell::Hold);
        assert_eq!(cell_at(&sec, LEAD, 2), GridCell::Off);
        assert_eq!(cell_at(&sec, LEAD, 3), GridCell::On(4));
        assert_eq!(cell_at(&sec, BASS, 0), GridCell::Off, "empty lane");
        assert_eq!(cell_at(&sec, DRUMS, 0), GridCell::On(6));
        assert_eq!(cell_at(&sec, DRUMS, 1), GridCell::Off);
        assert_eq!(cell_at(&sec, PERC, 0), GridCell::Off, "empty perc lane");
        let orphan = Section {
            arp: &[HOLD, HOLD],
            ..Section::EMPTY
        };
        assert_eq!(cell_at(&orphan, ARP, 1), GridCell::Off);
    }

    /// A chord lane changes the bake key; voicings are spelled lowest-first
    /// at or above the written note, and stay in key.
    #[test]
    fn chords_enter_the_key_and_stay_in_key() {
        for c in [
            Chord::Single,
            Chord::Octave,
            Chord::Power,
            Chord::Triad,
            Chord::Sus2,
            Chord::Sus4,
            Chord::Seventh,
            Chord::Add9,
            Chord::Inv1,
            Chord::Inv2,
            Chord::Open,
        ] {
            let d = c.degrees();
            assert!(!d.is_empty());
            assert!(d.windows(2).all(|w| w[0] < w[1]), "{c:?} not ascending");
            assert!(d[0] >= 0, "{c:?} below the root");
        }
        // A Triad in A minor on the root is A C E (0, 3, 7 semitones).
        let f: Vec<f64> = Chord::Triad
            .degrees()
            .iter()
            .map(|&d| degree_freq(55.0, MINOR, d))
            .collect();
        assert!((f[1] / f[0] - 2f64.powf(3.0 / 12.0)).abs() < 1e-9);
        assert!((f[2] / f[0] - 2f64.powf(7.0 / 12.0)).abs() < 1e-9);
        const SEC: Section = Section {
            lead: &[0, 0],
            lead_chord: &[Chord::Single, Chord::Power],
            ..Section::EMPTY
        };
        let keys = music_keys(&SongSpec {
            sections: &[SEC],
            ..SONGS[0]
        });
        assert_eq!(keys.len(), 2);
    }

    #[test]
    fn the_kit_lists_every_sounding_drum_once() {
        assert!(!Drum::KIT.contains(&Silent));
        for (i, d) in Drum::KIT.iter().enumerate() {
            assert!(!Drum::KIT[..i].contains(d), "{d:?} twice");
        }
        // A song using every piece on either lane enumerates all of them.
        const SEC: Section = Section {
            drums: &[Kick, Hat, Snare, Clap],
            perc: &[OpenHat, Tom, Rim, Crash],
            ..Section::EMPTY
        };
        let song = SongSpec {
            sections: &[SEC],
            ..SONGS[0]
        };
        let keys = music_keys(&song);
        for d in Drum::KIT {
            assert!(keys.contains(&MusicKey::Drum(d)), "{d:?}");
        }
        assert_eq!(cell_at(&SEC, PERC, 3), GridCell::On(MAX_VEL));
        assert_eq!(section_len(&SEC), 4);
        // The kick probe reads BOTH percussion lanes, with their looping.
        assert!(is_kick_step(&SEC, 0) && is_kick_step(&SEC, 4));
        assert!(!is_kick_step(&SEC, 1));
        const PERC_KICK: Section = Section {
            drums: &[Hat, Hat],
            perc: &[Silent, Kick],
            ..Section::EMPTY
        };
        assert!(is_kick_step(&PERC_KICK, 1) && !is_kick_step(&PERC_KICK, 0));
    }

    /// A baked voice is long enough for its note: attack + the tied steps +
    /// the lane's tail; an envelope override is baked in full.
    #[test]
    fn bake_lengths_cover_the_note() {
        let song = SONGS[0];
        let sd = step_dur(&song);
        let note = |lane, len| MusicKey::Note {
            lane,
            degree: 0,
            len,
            chord: Chord::Single,
            from: None,
            wob: 0,
        };
        // A held note bakes its 7 extra steps on top of the untied length.
        let (one, held) = (note(LEAD, 1), note(LEAD, 8));
        assert!((key_seconds(&song, held) - key_seconds(&song, one) - 7.0 * sd).abs() < 1e-9);
        let mut voices = song.voices;
        voices[KEYS] = Voice::mono(Wave::Noise).with_env(2.5, 1.0);
        let riser = SongSpec { voices, ..song };
        assert!(key_seconds(&riser, note(KEYS, 1)) > 2.5 + sd);
        assert_eq!(voice_shape(&riser, KEYS).0, 1.0);
        assert_eq!(voice_shape(&riser, LEAD), lane_shape(LEAD));
        // Every kit piece is baked at least as long as its layers.
        for d in Drum::KIT {
            assert!(key_seconds(&song, MusicKey::Drum(d)) >= 0.05, "{d:?}");
        }
    }

    #[test]
    fn info_line_helpers() {
        assert_eq!(scale_name(MINOR), "MINOR");
        assert_eq!(scale_name(LOCRIAN), "LOCRIAN");
        assert_eq!(scale_name(&[0, 5]), "CUSTOM");
        assert_eq!(note_name(55.0), "A1");
        assert_eq!(note_name(73.42), "D2");
        assert_eq!(note_name(440.0), "A4");
        assert_eq!(note_name(32.70), "C1");
        assert_eq!(note_name(0.0), "?");
        assert_eq!(MELODIC.len(), NUM_VOICES);
        assert_eq!(CHANNEL_NAMES[KEYS], "KEYS");
    }

    /// The playhead walks every section's full length, crosses into the next
    /// section exactly at its longest lane's end, and wraps the arrangement.
    #[test]
    fn playhead_walks_the_arrangement() {
        for song in SONGS.iter() {
            let mut ph = Playhead::START;
            let total: usize = song.sections.iter().map(section_len).sum();
            let mut crossings = 0;
            for _ in 0..total {
                assert_eq!(
                    ph.at_bar_start(song),
                    ph.step.is_multiple_of(bar_steps(song)),
                    "{}",
                    song.name
                );
                if ph.advance(song) {
                    crossings += 1;
                    assert_eq!(ph.step, 0);
                }
            }
            assert_eq!(crossings, song.sections.len(), "{}", song.name);
            assert_eq!(ph, Playhead::START, "{}: did not wrap", song.name);
        }
        let song = SONGS[0];
        let mut ph = Playhead::START;
        ph.jump_to_section(&song, 999);
        assert_eq!(ph.section, song.sections.len() - 1);
        ph.seek(&song, ph.loop_len(&song) + 3);
        assert_eq!(ph.step, 3);
        assert_eq!(ph.sounding_step(&song, 2), 1);
        assert_eq!(ph.sounding_step(&song, 4), ph.loop_len(&song) - 1);
    }
}
