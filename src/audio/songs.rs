//! The data-driven song format + the pure sequencer arithmetic.
//!
//! Everything here compiles NATIVELY (no `web-sys`): the song types, the
//! in-key pitch math, the section / step playhead and the voice-set
//! enumeration the bake queue works from. The songs themselves are data in
//! `songs/*.json`, compiled by `tools/gen_songs.py` into
//! [`super::songs_data`] (see `docs/SONGS_FORMAT.md`); the WebAudio engine
//! (`audio/engine.rs`, wasm-only) only *plays* what is described here.
//!
//! A song is *plain, `const`-able data*: a key (root frequency + scale), a
//! tempo, a set of oscillator voices, and an ordered list of SECTIONS.
//!
//! Each [`Section`] is its own multi-bar block of five step-sequenced
//! channels (bass, lead, pad, arp, drums). A [`SongSpec`] strings sections
//! together into a real arrangement — intro / verse / refrain / bridge /
//! variation — so a full play-through develops over time and the refrain
//! *returns* instead of a single bar looping forever. Sections are just
//! `&'static` slices of patterns, so a section can appear several times in
//! the order (that is how a refrain comes back) at zero extra cost.
//!
//! Melodic patterns are written as *scale degrees* (see [`degree_freq`]):
//! `0` is the root, `1` the next scale note up, `7` an octave up (for a
//! 7-note scale), negative degrees drop below the root. [`REST`] means
//! silence for that step. This keeps a song readable and in-key no matter
//! which root/scale it uses.
//!
//! Lanes inside a section may differ in length: a short 16-step bass simply
//! repeats under a longer 32-step lead. A section's length is its longest
//! lane, so authoring a 2-bar section only means writing one lane at 32
//! steps.
//!
//! The `pad` lane is special: each note blooms into a full triad (root +
//! third + fifth taken from the scale) with a slow attack, for sustained
//! chord beds.

use super::songs_data::{
    BLOOD_RUSH, CHROME_VEINS, DEEP_STATIC, DESCENT, MASK_OF_DREAD, NEON_LOUNGE, STATIC_PRAYER,
};

/// Sentinel used inside a pattern to mean "rest" (no note this step).
pub const REST: i32 = i32::MIN;

/// Number of sequenced channels (rows in the tracker view).
pub const NUM_CHANNELS: usize = 5;

/// Human-readable channel names, indexed 0..[`NUM_CHANNELS`].
pub const CHANNEL_NAMES: [&str; NUM_CHANNELS] = ["BASS", "LEAD", "PAD", "ARP", "DRUMS"];

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

/// Oscillator shape of a melodic voice. A plain enum so the song data is
/// host-compilable; the wasm engine maps it to `web_sys::OscillatorType`.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Wave {
    Sine,
    Square,
    Sawtooth,
    Triangle,
}

/// One step of the drum lane. Rendered from synthesized noise/tones only.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Drum {
    /// No percussion this step.
    Silent,
    /// Pitched sine thump + a lick of low noise.
    Kick,
    /// Very short high-passed noise tick.
    Hat,
    /// Noise burst + a short body tone on the backbeat.
    Snare,
}
use Drum::{Hat, Kick, Silent, Snare};

/// One block of an arrangement: a self-contained, multi-bar pattern across all
/// five channels. Songs are built by ordering these (a refrain section can be
/// listed several times so the hook comes back). A section's playable length is
/// the length of its longest lane; shorter lanes loop within it.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Section {
    /// Human-readable role (intro / verse / refrain / bridge / outro). Purely
    /// documentation + exposed via the tracker API; the scheduler ignores it.
    pub label: &'static str,
    /// Bass lane, one scale-degree (or `REST`) per step.
    pub bass: &'static [i32],
    /// Lead/melody lane, one scale-degree (or `REST`) per step.
    pub lead: &'static [i32],
    /// Pad/chord lane: each note blooms into a slow triad. `REST` sustains.
    pub pad: &'static [i32],
    /// Arp lane — a faster, higher counter-melody.
    pub arp: &'static [i32],
    /// Percussion lane, one `Drum` per step.
    pub drums: &'static [Drum],
}

/// A whole song as copyable data. Author one in `songs/`, list it in
/// `songs/index.json`, `make gen-songs`, done.
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
    /// Oscillator shape for the bass voice.
    pub bass_wave: Wave,
    /// Oscillator shape for the lead voice.
    pub lead_wave: Wave,
    /// Oscillator shape for the pad voice.
    pub pad_wave: Wave,
    /// Oscillator shape for the arp voice.
    pub arp_wave: Wave,
    /// The arrangement: an ordered list of sections played back to back, then
    /// looped as a whole. This is what makes a song long and developing.
    pub sections: &'static [Section],
    /// Overall punch/loudness feel (~0.5 lounge .. ~1.2 boss).
    pub intensity: f64,
}

/// Pick a song for a given floor, escalating darkness as you descend. Kept as a
/// plain mapping so the integrator can call it per level. The trailing
/// entries of [`super::songs_data::SONGS`] ("Razor Circuit", "Cold Storage")
/// are AUDITION CANDIDATES: listed so the `?viz` tracker can play them, but
/// mapped to no floor — promote one by referencing it here.
pub fn song_for_floor(level: usize) -> SongSpec {
    match level {
        0..=1 => NEON_LOUNGE,
        2..=3 => CHROME_VEINS,
        4..=5 => DESCENT,
        6..=7 => BLOOD_RUSH,
        8..=9 => DEEP_STATIC,
        10..=12 => STATIC_PRAYER,
        _ => MASK_OF_DREAD,
    }
}

// --- pure helpers ----------------------------------------------------------

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

/// Read a melodic lane at `step` (patterns loop). `None` = rest / empty lane.
pub fn degree_at(pattern: &[i32], step: usize) -> Option<i32> {
    if pattern.is_empty() {
        return None;
    }
    match pattern[step % pattern.len()] {
        REST => None,
        d => Some(d),
    }
}

/// Read the drum lane at `step` (loops). Empty lane == `Silent`.
pub fn drum_at(pattern: &[Drum], step: usize) -> Drum {
    if pattern.is_empty() {
        return Silent;
    }
    pattern[step % pattern.len()]
}

/// The playable length of a section: its longest lane (shorter lanes loop
/// inside it). Always at least 1 so the scheduler can never divide by zero.
pub fn section_len(sec: &Section) -> usize {
    sec.bass
        .len()
        .max(sec.lead.len())
        .max(sec.pad.len())
        .max(sec.arp.len())
        .max(sec.drums.len())
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

/// Shared cell sampler: does `channel` (0 bass, 1 lead, 2 pad, 3 arp, 4
/// drums — see [`CHANNEL_NAMES`]) fire at `step` within `sec`?
pub fn cell_active(sec: &Section, channel: usize, step: usize) -> bool {
    match channel {
        0 => degree_at(sec.bass, step).is_some(),
        1 => degree_at(sec.lead, step).is_some(),
        2 => degree_at(sec.pad, step).is_some(),
        3 => degree_at(sec.arp, step).is_some(),
        4 => !matches!(drum_at(sec.drums, step), Silent),
        _ => false,
    }
}

/// Compact density summary of a section: the fraction (0.0..=1.0) of all
/// grid cells that carry a note/hit. A cheap way to shade each miniature by
/// how busy/intense it is without drawing every cell.
pub fn section_density(sec: &Section) -> f32 {
    let steps = section_len(sec);
    let mut active = 0usize;
    for step in 0..steps {
        for chan in 0..NUM_CHANNELS {
            if cell_active(sec, chan, step) {
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

/// One pre-renderable MUSIC voice: a tracker channel role × the scale
/// degree it plays (drums carry no pitch; a pad key bakes its whole triad
/// into one buffer). The set of keys a song can ever schedule is FINITE —
/// its lanes are static pattern data — so [`music_keys`] enumerates it
/// exactly and each key is baked at its exact pitch (no `playback_rate`
/// transposition: the timbre is untouched).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum MusicKey {
    /// Bass lane note at this scale degree.
    Bass(i32),
    /// Lead lane note at this scale degree.
    Lead(i32),
    /// Pad lane note: the full triad (root + third + fifth) in one buffer.
    Pad(i32),
    /// Arp lane note at this scale degree.
    Arp(i32),
    /// Drum lane kick.
    Kick,
    /// Drum lane hat.
    Hat,
    /// Drum lane snare.
    Snare,
}

impl MusicKey {
    /// The voice a drum step needs (`None` for a silent step).
    pub fn of_drum(hit: Drum) -> Option<MusicKey> {
        match hit {
            Silent => None,
            Kick => Some(MusicKey::Kick),
            Hat => Some(MusicKey::Hat),
            Snare => Some(MusicKey::Snare),
        }
    }
}

/// Enumerate the exact, finite voice set `song` can ever schedule: the
/// distinct scale degrees of each melodic lane across every section,
/// plus the up-to-three drum voices — in bake-priority order (drums
/// first, then bass, lead, arp, pad). Typically 30–45 keys per song.
pub fn music_keys(song: &SongSpec) -> Vec<MusicKey> {
    fn add(keys: &mut Vec<MusicKey>, k: MusicKey) {
        if !keys.contains(&k) {
            keys.push(k);
        }
    }
    let mut keys = Vec::new();
    for sec in song.sections {
        for &d in sec.drums {
            if let Some(k) = MusicKey::of_drum(d) {
                add(&mut keys, k);
            }
        }
    }
    type Lane = (fn(&Section) -> &'static [i32], fn(i32) -> MusicKey);
    const LANES: [Lane; 4] = [
        (|s| s.bass, MusicKey::Bass),
        (|s| s.lead, MusicKey::Lead),
        (|s| s.arp, MusicKey::Arp),
        (|s| s.pad, MusicKey::Pad),
    ];
    for (pattern, mk) in LANES {
        for sec in song.sections {
            for &d in pattern(sec) {
                if d != REST {
                    add(&mut keys, mk(d));
                }
            }
        }
    }
    keys
}

#[cfg(test)]
mod tests {
    use super::super::songs_data::SONGS;
    use super::*;

    /// Every note any song can ever schedule must map to an enumerated
    /// [`MusicKey`], and the per-song voice set must stay small enough that
    /// baking each exact pitch (no `playback_rate` transposition) is cheap.
    /// Run with `--nocapture` to see the per-song counts.
    #[test]
    fn music_voice_sets_are_small_and_complete() {
        for song in SONGS {
            let keys = music_keys(song);
            assert!(!keys.is_empty(), "{}: empty voice set", song.name);
            assert!(
                keys.len() <= 64,
                "{}: {} voices — too many to bake each exact pitch",
                song.name,
                keys.len()
            );
            // No duplicates (the queue bakes each key exactly once).
            for (i, k) in keys.iter().enumerate() {
                assert!(!keys[..i].contains(k), "{}: duplicate {:?}", song.name, k);
            }
            // Completeness: every schedulable note has a key.
            for sec in song.sections {
                for &d in sec.bass {
                    assert!(d == REST || keys.contains(&MusicKey::Bass(d)));
                }
                for &d in sec.lead {
                    assert!(d == REST || keys.contains(&MusicKey::Lead(d)));
                }
                for &d in sec.pad {
                    assert!(d == REST || keys.contains(&MusicKey::Pad(d)));
                }
                for &d in sec.arp {
                    assert!(d == REST || keys.contains(&MusicKey::Arp(d)));
                }
                for &dr in sec.drums {
                    if let Some(key) = MusicKey::of_drum(dr) {
                        assert!(keys.contains(&key));
                    }
                }
            }
            let drums = keys
                .iter()
                .filter(|k| matches!(k, MusicKey::Kick | MusicKey::Hat | MusicKey::Snare))
                .count();
            let count = |f: fn(&MusicKey) -> bool| keys.iter().filter(|k| f(k)).count();
            println!(
                "{:14} {:2} voices (drums {} bass {:2} lead {:2} arp {:2} pad {:2})",
                song.name,
                keys.len(),
                drums,
                count(|k| matches!(k, MusicKey::Bass(_))),
                count(|k| matches!(k, MusicKey::Lead(_))),
                count(|k| matches!(k, MusicKey::Arp(_))),
                count(|k| matches!(k, MusicKey::Pad(_))),
            );
        }
    }

    /// Every section of every song must be whole bars long (its longest lane a
    /// multiple of 16 steps at 4 steps/beat, 4 beats/bar): `Playhead::advance`
    /// moves on when the LONGEST lane ends, so a ragged longest lane would
    /// shift every later section off the beat grid. Short lanes may still be
    /// any length — they loop inside the section (that is how Cold Storage's
    /// 12-step motif phases 3-against-4) — and every arrangement is non-empty.
    #[test]
    fn sections_are_bar_aligned() {
        for song in SONGS {
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

    /// The audition candidates are appended, unmapped extras: they must be in
    /// [`SONGS`] (so the `?viz` tracker lists them) but no floor may pick them
    /// yet, and the ending's calmest-track pick must not drift onto them.
    #[test]
    fn audition_candidates_are_listed_but_unmapped() {
        for name in ["Razor Circuit", "Cold Storage"] {
            assert!(SONGS.iter().any(|s| s.name == name), "{name} not in SONGS");
            for floor in 0..32 {
                assert_ne!(
                    song_for_floor(floor).name,
                    name,
                    "floor {floor} maps to audition candidate {name}"
                );
            }
        }
        let calmest = SONGS
            .iter()
            .min_by(|a, b| a.intensity.total_cmp(&b.intensity))
            .unwrap();
        assert_eq!(calmest.name, "Insert Coin");
    }

    /// Every floor's song is one of the listed songs (the `?viz` tracker can
    /// show whatever is playing), song names are unique (the engine detects
    /// a song switch by name) and every song is playable data: positive
    /// tempo / root, a non-empty scale, every note in-key resolvable.
    #[test]
    fn songs_are_well_formed_and_floor_mapping_is_listed() {
        for floor in 0..32 {
            let s = song_for_floor(floor);
            assert!(SONGS.iter().any(|x| x.name == s.name), "floor {floor}");
        }
        for (i, song) in SONGS.iter().enumerate() {
            assert!(
                !SONGS[..i].iter().any(|x| x.name == song.name),
                "duplicate song name {}",
                song.name
            );
            assert!(song.bpm > 0.0 && song.root > 0.0 && song.steps_per_beat >= 1);
            assert!(!song.scale.is_empty());
            assert!(step_dur(song) > 0.0 && step_dur(song) < 1.0);
            for sec in song.sections {
                for lane in [sec.bass, sec.lead, sec.pad, sec.arp] {
                    for &d in lane {
                        if d != REST {
                            let f = degree_freq(song.root, song.scale, d);
                            assert!(f.is_finite() && f > 0.0 && f < 20_000.0);
                        }
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

    /// Lanes loop, empty lanes read as silence.
    #[test]
    fn lanes_loop_and_empty_lanes_are_silent() {
        assert_eq!(degree_at(&[0, REST, 3], 4), None);
        assert_eq!(degree_at(&[0, REST, 3], 5), Some(3));
        assert_eq!(degree_at(&[], 5), None);
        assert_eq!(drum_at(&[Kick, Silent], 2), Kick);
        assert_eq!(drum_at(&[], 9), Silent);
        let sec = Section {
            label: "t",
            bass: &[0],
            lead: &[],
            pad: &[REST; 32],
            arp: &[],
            drums: &[Kick],
        };
        assert_eq!(section_len(&sec), 32);
        assert!(cell_active(&sec, 0, 31));
        assert!(!cell_active(&sec, 2, 3));
        assert!(cell_active(&sec, 4, 7));
        assert!(!cell_active(&sec, 5, 0));
    }

    /// The playhead walks every section's full length, crosses into the next
    /// section exactly at its longest lane's end, and wraps the arrangement.
    #[test]
    fn playhead_walks_the_arrangement() {
        for song in SONGS {
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
