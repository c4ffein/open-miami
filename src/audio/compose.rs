//! The SONG-AUTHORING layer: one song = one Rust file of composable
//! functions (`src/audio/songs/<name>.rs`), built from the small builder
//! types here. See `docs/MUSIC_CODE.md` for the guided tour.
//!
//! The design goal is SELF-DOCUMENTING music code: a riff is a function, a
//! section is a named list of parts, a song is an arrangement of named
//! sections — so a reader sees "this is the refrain, we call it again here,
//! hotter". Everything compiles down to the *existing* playback structures
//! ([`Section`] / [`SongSpec`] — this module never redesigns playback): a
//! [`SongBuilder::build`] leaks its finished lanes into the `&'static`
//! slices the sequencer walks, and each song file memoizes that build in a
//! `OnceLock` so it happens exactly once.
//!
//! The atoms:
//! * [`Lane`] — a melodic pattern (scale degrees, [`REST`]s). Author one
//!   with [`steps`] (`"0 . 3 . | 5 . 3 ."` — `.` rest, `|` cosmetic) or
//!   from raw degrees, then shape it with the combinators: [`transpose`],
//!   [`repeat`], [`cat`], [`every_other_bar`], [`stretch`], [`sparsify`]
//!   (seeded, deterministic).
//! * [`DrumLane`] — percussion, authored with [`hits`] (`"k.h.k.h."`).
//! * [`Part`] — a lane bound to a channel: [`bass`], [`lead`], [`pad`],
//!   [`arp`], [`drums`], optionally scaled by [`with_velocity`].
//! * [`section`] — parts assembled into one named block; two parts on the
//!   same channel OVERLAY (later steps win where both sound), so
//!   `section("groove", [kick4(), hats(), snare24()])` layers a kit.
//! * [`song`] — key/tempo/waves plus `.arrange([...])`, the ordered list
//!   of sections (call a section function several times and the refrain
//!   comes back; give it an [`Intensity`] argument and it comes back
//!   hotter).

use super::songs::{Drum, Scale, Section, SongSpec, Wave, NUM_CHANNELS, REST};

// --- keys -------------------------------------------------------------------

/// A musical key: the tonic frequency + the scale (semitone offsets).
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Key {
    /// Tonic frequency in Hz (see the `*1` root constants below).
    pub root: f64,
    /// The mode: semitone offsets from `root` (the named scales in
    /// `songs.rs` — `MINOR`, `PHRYGIAN`, ...).
    pub scale: Scale,
}

impl Key {
    /// A key from a root frequency and a scale.
    pub const fn new(root: f64, scale: Scale) -> Key {
        Key { root, scale }
    }
}

/// First-octave root frequencies (Hz, 12-TET, A440) for [`Key::new`].
pub const C1: f64 = 32.7;
pub const CS1: f64 = 34.65;
pub const D1: f64 = 36.71;
pub const DS1: f64 = 38.89;
pub const E1: f64 = 41.2;
pub const F1: f64 = 43.65;
pub const FS1: f64 = 46.25;
pub const G1: f64 = 49.0;
pub const GS1: f64 = 51.91;
pub const A1: f64 = 55.0;
pub const AS1: f64 = 58.27;
pub const B1: f64 = 61.74;

/// How hot a variation of a section should run — the argument a section
/// function takes so its returns read as "the refrain, again, hotter":
/// `arrange([... refrain(Intensity::Cool), ..., refrain(Intensity::Hot)])`.
/// Purely an authoring-side selector; playback never sees it.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Intensity {
    Cool,
    Warm,
    Hot,
}

// --- melodic lanes ----------------------------------------------------------

/// An owned melodic pattern under construction: one scale degree (or
/// [`REST`]) per step. The builder-side counterpart of a [`Section`] lane.
#[derive(Clone, PartialEq, Debug, Default)]
pub struct Lane(pub Vec<i32>);

impl From<&str> for Lane {
    fn from(s: &str) -> Lane {
        steps(s)
    }
}

impl From<Vec<i32>> for Lane {
    fn from(v: Vec<i32>) -> Lane {
        Lane(v)
    }
}

impl Lane {
    /// Number of steps.
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// `true` when the lane has no steps at all (an all-silent lane).
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Method form of [`transpose`].
    pub fn transpose(self, by: i32) -> Lane {
        transpose(self, by)
    }

    /// Method form of [`repeat`].
    pub fn repeat(self, times: usize) -> Lane {
        repeat(self, times)
    }

    /// This lane followed by `next`.
    pub fn then(mut self, next: impl Into<Lane>) -> Lane {
        self.0.extend(next.into().0);
        self
    }

    /// Method form of [`sparsify`].
    pub fn sparsify(self, seed: u32, drop: f64) -> Lane {
        sparsify(self, seed, drop)
    }

    /// Method form of [`stretch`].
    pub fn stretch(self, factor: usize) -> Lane {
        stretch(self, factor)
    }
}

/// Parse a melodic lane from tokens: whitespace-separated, `.` = rest, an
/// integer = a scale degree (`0` root, `7` an octave up on 7-note scales,
/// negatives below the root), `|` a purely cosmetic bar separator. Newlines
/// are whitespace, so a multi-bar lane reads one bar per line. Panics on a
/// bad token — song builds run under `cargo test`, so an authoring typo
/// fails loudly in CI, exactly like the old generator's validation.
pub fn steps(s: &str) -> Lane {
    Lane(
        s.split_whitespace()
            .filter(|t| *t != "|")
            .map(|t| match t {
                "." => REST,
                d => d
                    .parse::<i32>()
                    .unwrap_or_else(|_| panic!("bad step token {d:?} in lane {s:?}")),
            })
            .collect(),
    )
}

/// Shift every note of a riff by `by` scale degrees (rests stay rests):
/// `transpose(riff, 7)` = up an octave on a 7-note scale.
pub fn transpose(lane: impl Into<Lane>, by: i32) -> Lane {
    let mut l = lane.into();
    for d in &mut l.0 {
        if *d != REST {
            *d += by;
        }
    }
    l
}

/// The riff played back to back `times` times.
pub fn repeat(lane: impl Into<Lane>, times: usize) -> Lane {
    let l = lane.into();
    let mut out = Vec::with_capacity(l.0.len() * times);
    for _ in 0..times {
        out.extend_from_slice(&l.0);
    }
    Lane(out)
}

/// Several riffs joined end to end (bars into a multi-bar lane).
pub fn cat<L: Into<Lane>>(parts: impl IntoIterator<Item = L>) -> Lane {
    let mut out = Vec::new();
    for p in parts {
        out.extend(p.into().0);
    }
    Lane(out)
}

/// Alternate two riffs bar by bar: `a` then `b`, looped by playback — write
/// each as one bar and the lane flips between them forever. A shorter riff
/// is padded with rests to the longer one so the alternation stays aligned.
pub fn every_other_bar(a: impl Into<Lane>, b: impl Into<Lane>) -> Lane {
    let (mut a, mut b) = (a.into(), b.into());
    let len = a.0.len().max(b.0.len());
    a.0.resize(len, REST);
    b.0.resize(len, REST);
    a.then(b)
}

/// Slow a riff down by `factor`: every step becomes `factor` steps (the
/// note, then `factor - 1` rests), so a one-bar cell at `stretch(2)` plays
/// over two bars at half speed — how a motif gets QUOTED slower elsewhere.
/// `factor` 0 is treated as 1 (the identity).
pub fn stretch(lane: impl Into<Lane>, factor: usize) -> Lane {
    let l = lane.into();
    let factor = factor.max(1);
    let mut out = Vec::with_capacity(l.0.len() * factor);
    for d in l.0 {
        out.push(d);
        out.extend(std::iter::repeat_n(REST, factor - 1));
    }
    Lane(out)
}

/// Deterministic seeded variation: drop each note with probability `drop`
/// (0.0 = untouched, 1.0 = silence). The same `(riff, seed, drop)` always
/// yields the same thinned riff — call it twice in an arrangement and both
/// occurrences match; change the seed and the holes move.
pub fn sparsify(lane: impl Into<Lane>, seed: u32, drop: f64) -> Lane {
    let mut l = lane.into();
    let mut state = seed | 1; // xorshift must never be 0
    for d in &mut l.0 {
        state ^= state << 13;
        state ^= state >> 17;
        state ^= state << 5;
        let r = (state >> 8) as f64 / (1u32 << 24) as f64;
        if *d != REST && r < drop {
            *d = REST;
        }
    }
    l
}

// --- drum lanes -------------------------------------------------------------

/// An owned percussion pattern under construction, one [`Drum`] per step.
#[derive(Clone, PartialEq, Debug, Default)]
pub struct DrumLane(pub Vec<Drum>);

impl From<&str> for DrumLane {
    fn from(s: &str) -> DrumLane {
        hits(s)
    }
}

impl From<Vec<Drum>> for DrumLane {
    fn from(v: Vec<Drum>) -> DrumLane {
        DrumLane(v)
    }
}

impl DrumLane {
    /// Number of steps.
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// `true` when the lane has no steps at all.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// The pattern played back to back `times` times.
    pub fn repeat(self, times: usize) -> DrumLane {
        let mut out = Vec::with_capacity(self.0.len() * times);
        for _ in 0..times {
            out.extend_from_slice(&self.0);
        }
        DrumLane(out)
    }

    /// This pattern followed by `next`.
    pub fn then(mut self, next: impl Into<DrumLane>) -> DrumLane {
        self.0.extend(next.into().0);
        self
    }
}

/// Parse a drum lane, one character per step: `.` silent, `k` kick, `h`
/// hat, `s` snare; whitespace and `|` are cosmetic. Panics on anything
/// else (fails loudly under `cargo test`, like [`steps`]).
pub fn hits(s: &str) -> DrumLane {
    DrumLane(
        s.chars()
            .filter(|c| !c.is_whitespace() && *c != '|')
            .map(|c| match c {
                '.' => Drum::Silent,
                'k' => Drum::Kick,
                'h' => Drum::Hat,
                's' => Drum::Snare,
                other => panic!("bad drum token {other:?} in lane {s:?}"),
            })
            .collect(),
    )
}

/// Several drum patterns joined end to end.
pub fn cat_hits<L: Into<DrumLane>>(parts: impl IntoIterator<Item = L>) -> DrumLane {
    let mut out = Vec::new();
    for p in parts {
        out.extend(p.into().0);
    }
    DrumLane(out)
}

// --- parts & sections -------------------------------------------------------

/// A lane bound to its tracker channel (plus a velocity), ready to be
/// listed in a [`section`]. Build one with [`bass`] / [`lead`] / [`pad`] /
/// [`arp`] / [`drums`].
#[derive(Clone, PartialEq, Debug)]
pub struct Part {
    channel: usize,
    lane: Lane,
    drum: DrumLane,
    vel: f32,
}

impl Part {
    fn melodic(channel: usize, lane: Lane) -> Part {
        Part {
            channel,
            lane,
            drum: DrumLane::default(),
            vel: 1.0,
        }
    }

    /// Method form of [`with_velocity`].
    pub fn vel(mut self, v: f32) -> Part {
        self.vel *= v.max(0.0);
        self
    }
}

/// The bass part of a section.
pub fn bass(lane: impl Into<Lane>) -> Part {
    Part::melodic(0, lane.into())
}

/// The lead/melody part of a section.
pub fn lead(lane: impl Into<Lane>) -> Part {
    Part::melodic(1, lane.into())
}

/// The pad part of a section (each note blooms into a slow triad).
pub fn pad(lane: impl Into<Lane>) -> Part {
    Part::melodic(2, lane.into())
}

/// The arp part of a section (the fast high counter-melody).
pub fn arp(lane: impl Into<Lane>) -> Part {
    Part::melodic(3, lane.into())
}

/// The percussion part of a section.
pub fn drums(lane: impl Into<DrumLane>) -> Part {
    Part {
        channel: 4,
        lane: Lane::default(),
        drum: lane.into(),
        vel: 1.0,
    }
}

/// Scale a part's playback level (1.0 = nominal; multiplies if applied
/// twice). Velocity is applied at schedule time, so it costs nothing in
/// the note-bake budget — the same pitch at two velocities is one buffer.
pub fn with_velocity(part: Part, vel: f32) -> Part {
    part.vel(vel)
}

/// One named block of an arrangement under construction (the builder-side
/// [`Section`]). Made by [`section`]; `.ducked()` opts the block into the
/// sidechain pump.
#[derive(Clone, PartialEq, Debug)]
pub struct SectionSpec {
    label: &'static str,
    lanes: [Lane; 4],
    drum: DrumLane,
    vel: [f32; NUM_CHANNELS],
    duck: bool,
}

impl SectionSpec {
    /// Enable the SIDECHAIN DUCK for this section: every kick step pumps
    /// the melodic voices down (see `songs::duck_gain`) — the darksynth
    /// "everything breathes with the kick" feel.
    pub fn ducked(mut self) -> SectionSpec {
        self.duck = true;
        self
    }
}

/// Assemble named parts into one section: `section("groove", [kick4(),
/// offbeat_bass(), arp16()])`. Parts landing on the same channel OVERLAY:
/// the shorter one is looped up to the longer (playback's own looping
/// rule) and later parts win on the steps where both sound — so a kick
/// pattern, a hat pattern and a snare pattern layer into one kit lane.
/// Velocities on the same channel multiply.
pub fn section(label: &'static str, parts: impl IntoIterator<Item = Part>) -> SectionSpec {
    let mut spec = SectionSpec {
        label,
        lanes: [
            Lane::default(),
            Lane::default(),
            Lane::default(),
            Lane::default(),
        ],
        drum: DrumLane::default(),
        vel: [1.0; NUM_CHANNELS],
        duck: false,
    };
    for part in parts {
        spec.vel[part.channel] *= part.vel;
        if part.channel == 4 {
            spec.drum = overlay_drums(std::mem::take(&mut spec.drum), part.drum);
        } else {
            let slot = &mut spec.lanes[part.channel];
            *slot = overlay(std::mem::take(slot), part.lane);
        }
    }
    spec
}

/// Overlay `over` onto `base`: both loop-extended to the longer length,
/// then `over`'s non-rest steps replace `base`'s.
fn overlay(base: Lane, over: Lane) -> Lane {
    if base.is_empty() {
        return over;
    }
    if over.is_empty() {
        return base;
    }
    let len = base.len().max(over.len());
    let mut out = Vec::with_capacity(len);
    for i in 0..len {
        let o = over.0[i % over.0.len()];
        out.push(if o != REST {
            o
        } else {
            base.0[i % base.0.len()]
        });
    }
    Lane(out)
}

/// [`overlay`] for drum lanes (`Silent` = transparent).
fn overlay_drums(base: DrumLane, over: DrumLane) -> DrumLane {
    if base.is_empty() {
        return over;
    }
    if over.is_empty() {
        return base;
    }
    let len = base.len().max(over.len());
    let mut out = Vec::with_capacity(len);
    for i in 0..len {
        let o = over.0[i % over.0.len()];
        out.push(if o != Drum::Silent {
            o
        } else {
            base.0[i % base.0.len()]
        });
    }
    DrumLane(out)
}

// --- the song builder -------------------------------------------------------

/// A whole song under construction. Start with [`song`], chain the
/// settings, `.arrange([...])` the sections, `.build()` once (memoize the
/// result in a `OnceLock` — see any file in `src/audio/songs/`).
#[derive(Clone, PartialEq, Debug)]
pub struct SongBuilder {
    name: &'static str,
    key: Key,
    bpm: f64,
    steps_per_beat: u32,
    waves: [Wave; 4],
    intensity: f64,
    sections: Vec<SectionSpec>,
}

/// A song in `key` at `bpm` beats per minute (sixteenth-note resolution by
/// default). Defaults: sawtooth everything, intensity 0.8, no sections.
pub fn song(name: &'static str, key: Key, bpm: f64) -> SongBuilder {
    SongBuilder {
        name,
        key,
        bpm,
        steps_per_beat: 4,
        waves: [Wave::Sawtooth; 4],
        intensity: 0.8,
        sections: Vec::new(),
    }
}

impl SongBuilder {
    /// Sequencer resolution in steps per beat (default 4 = sixteenths).
    pub fn steps_per_beat(mut self, spb: u32) -> SongBuilder {
        self.steps_per_beat = spb;
        self
    }

    /// The oscillator (or preset — supersaw / driven bass / dark pad) of
    /// each melodic voice: bass, lead, pad, arp.
    pub fn waves(mut self, bass: Wave, lead: Wave, pad: Wave, arp: Wave) -> SongBuilder {
        self.waves = [bass, lead, pad, arp];
        self
    }

    /// Overall punch/loudness feel (~0.5 lounge .. ~1.2 boss).
    pub fn intensity(mut self, intensity: f64) -> SongBuilder {
        self.intensity = intensity;
        self
    }

    /// The arrangement: sections played back to back, then looped as a
    /// whole. Call a section function several times to bring it back.
    pub fn arrange(mut self, sections: impl IntoIterator<Item = SectionSpec>) -> SongBuilder {
        self.sections.extend(sections);
        self
    }

    /// Finalize into the playback [`SongSpec`]: every lane is leaked into
    /// the `&'static` slices the sequencer walks. Call ONCE per song (each
    /// song file memoizes its build in a `OnceLock`); the leak is the
    /// static song data the old generated file used to carry.
    pub fn build(self) -> SongSpec {
        fn leak_lane(l: Lane) -> &'static [i32] {
            Box::leak(l.0.into_boxed_slice())
        }
        let sections: Vec<Section> = self
            .sections
            .into_iter()
            .map(|s| {
                let [b, l, p, a] = s.lanes;
                Section {
                    label: s.label,
                    bass: leak_lane(b),
                    lead: leak_lane(l),
                    pad: leak_lane(p),
                    arp: leak_lane(a),
                    drums: Box::leak(s.drum.0.into_boxed_slice()),
                    vel: s.vel,
                    duck: s.duck,
                }
            })
            .collect();
        SongSpec {
            name: self.name,
            root: self.key.root,
            scale: self.key.scale,
            bpm: self.bpm,
            steps_per_beat: self.steps_per_beat,
            bass_wave: self.waves[0],
            lead_wave: self.waves[1],
            pad_wave: self.waves[2],
            arp_wave: self.waves[3],
            sections: Box::leak(sections.into_boxed_slice()),
            intensity: self.intensity,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::songs::MINOR;
    use super::*;
    use Drum::{Hat, Kick, Silent, Snare};

    #[test]
    fn steps_parses_rests_degrees_and_bars() {
        assert_eq!(steps("0 . -2 | 14 .").0, vec![0, REST, -2, 14, REST]);
        assert_eq!(steps("0 .\n 3 .").0, vec![0, REST, 3, REST]);
        assert!(steps("").is_empty());
    }

    #[test]
    #[should_panic(expected = "bad step token")]
    fn steps_rejects_garbage() {
        steps("0 . x");
    }

    #[test]
    fn hits_parses_and_ignores_layout() {
        assert_eq!(hits("k.h s | .").0, vec![Kick, Silent, Hat, Snare, Silent]);
    }

    #[test]
    #[should_panic(expected = "bad drum token")]
    fn hits_rejects_garbage() {
        hits("k.z.");
    }

    #[test]
    fn transpose_shifts_notes_and_keeps_rests() {
        assert_eq!(transpose(steps("0 . -2 7"), 3).0, vec![3, REST, 1, 10]);
        assert_eq!(transpose(steps("0 7"), 0).0, vec![0, 7]);
    }

    #[test]
    fn repeat_and_cat_concatenate() {
        assert_eq!(repeat(steps("0 ."), 3).0, vec![0, REST, 0, REST, 0, REST]);
        assert_eq!(repeat(steps("0"), 0).0, Vec::<i32>::new());
        assert_eq!(cat([steps("0 ."), steps("3")]).0, vec![0, REST, 3]);
        assert_eq!(
            cat_hits([hits("k."), hits("s")]).0,
            vec![Kick, Silent, Snare]
        );
        assert_eq!(steps("0").then("5 .").0, vec![0, 5, REST]);
    }

    #[test]
    fn stretch_slows_a_riff_down() {
        assert_eq!(
            stretch(steps("0 . 3"), 2).0,
            vec![0, REST, REST, REST, 3, REST]
        );
        assert_eq!(stretch(steps("0 3"), 1).0, vec![0, 3]);
        assert_eq!(stretch(steps("0 3"), 0).0, vec![0, 3]);
        assert_eq!(steps("7 .").stretch(4).len(), 8);
    }

    #[test]
    fn every_other_bar_alternates_and_pads() {
        // Equal lengths: plain concatenation.
        assert_eq!(
            every_other_bar(steps("0 ."), steps("3 .")).0,
            vec![0, REST, 3, REST]
        );
        // The shorter riff is rest-padded so the flip stays bar-aligned.
        assert_eq!(
            every_other_bar(steps("0"), steps("3 5 7")).0,
            vec![0, REST, REST, 3, 5, 7]
        );
    }

    #[test]
    fn sparsify_is_deterministic_and_thins() {
        let riff = repeat(steps("0 1 2 3 4 5 6 7"), 32); // 256 notes
        let a = sparsify(riff.clone(), 7, 0.3);
        let b = sparsify(riff.clone(), 7, 0.3);
        assert_eq!(a, b, "same seed must give the same variation");
        let c = sparsify(riff.clone(), 8, 0.3);
        assert_ne!(a, c, "a different seed must move the holes");
        let notes = |l: &Lane| l.0.iter().filter(|&&d| d != REST).count();
        let kept = notes(&a);
        assert!(kept < 256, "sparsify must drop something");
        assert!(
            (0.5..=0.9).contains(&(kept as f64 / 256.0)),
            "kept {kept}/256"
        );
        // Rests stay rests; surviving notes are unchanged.
        for (orig, new) in riff.0.iter().zip(&a.0) {
            assert!(*new == REST || new == orig);
        }
        // drop = 0 is the identity; drop = 1 is silence.
        assert_eq!(sparsify(riff.clone(), 3, 0.0), riff);
        assert_eq!(notes(&sparsify(riff, 3, 1.0)), 0);
    }

    #[test]
    fn section_places_parts_on_their_channels() {
        let s = section(
            "t",
            [bass("0 ."), lead("7"), pad("0"), arp("14 ."), drums("k.h.")],
        );
        assert_eq!(s.label, "t");
        assert_eq!(s.lanes[0].0, vec![0, REST]);
        assert_eq!(s.lanes[1].0, vec![7]);
        assert_eq!(s.lanes[2].0, vec![0]);
        assert_eq!(s.lanes[3].0, vec![14, REST]);
        assert_eq!(s.drum.0, vec![Kick, Silent, Hat, Silent]);
        assert_eq!(s.vel, [1.0; NUM_CHANNELS]);
        assert!(!s.duck);
        assert!(section("d", []).ducked().duck);
    }

    #[test]
    fn same_channel_parts_overlay_and_velocities_multiply() {
        // A 4-step kick loops under the 8-step snare overlay.
        let s = section("kit", [drums("k..."), drums(".......s")]);
        assert_eq!(
            s.drum.0,
            vec![Kick, Silent, Silent, Silent, Kick, Silent, Silent, Snare]
        );
        // Melodic overlay: later non-rest steps win, rests are transparent.
        let s = section("m", [bass("0 . 3 ."), with_velocity(bass(". 5 . ."), 0.5)]);
        assert_eq!(s.lanes[0].0, vec![0, 5, 3, REST]);
        assert_eq!(s.vel[0], 0.5);
        let s = section("v", [bass(steps("0")).vel(0.5), bass(steps(".")).vel(0.5)]);
        assert_eq!(s.vel[0], 0.25);
    }

    #[test]
    fn build_produces_a_playable_song() {
        let key = Key::new(A1, MINOR);
        let riff = || steps("0 . 3 . 5 . 3 . 0 . 3 . 5 . 7 .");
        let sec = |label| {
            section(
                label,
                [
                    bass(riff()),
                    lead(transpose(riff(), 7)),
                    drums(hits("k.h.").repeat(4)),
                ],
            )
        };
        let s = song("Test Song", key, 120.0)
            .waves(
                Wave::DrivenBass,
                Wave::Supersaw,
                Wave::DarkPad,
                Wave::Square,
            )
            .intensity(0.9)
            .arrange([sec("verse"), sec("refrain").ducked(), sec("refrain")])
            .build();
        assert_eq!(s.name, "Test Song");
        assert_eq!(s.root, A1);
        assert_eq!(s.scale, MINOR);
        assert_eq!(s.bpm, 120.0);
        assert_eq!(s.steps_per_beat, 4);
        assert_eq!(s.bass_wave, Wave::DrivenBass);
        assert_eq!(s.sections.len(), 3);
        assert_eq!(s.sections[0].label, "verse");
        assert_eq!(s.sections[0].bass, &riff().0[..]);
        assert_eq!(s.sections[0].lead, &transpose(riff(), 7).0[..]);
        assert_eq!(s.sections[0].pad, &[] as &[i32]);
        assert_eq!(s.sections[0].drums.len(), 16);
        assert!(s.sections[1].duck && !s.sections[0].duck);
        // The two refrain() calls produce identical content.
        assert_eq!(s.sections[1].bass, s.sections[2].bass);
        assert_eq!(s.sections[1].drums, s.sections[2].drums);
    }
}
