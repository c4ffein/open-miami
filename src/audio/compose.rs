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
//! The builders cover the WHOLE format: ties (`_` in [`steps`]), velocity
//! lanes ([`Part::accents`]), chord lanes ([`Part::voiced`]), the five
//! melodic lanes + two percussion lanes with the 8-piece kit, a [`Voice`]
//! per lane and the song-level settings (`.voices` / `.swing` /
//! `.sidechain` / `.echo` / `.humanize` / `.sweep`). `songs/sodium_lights.rs`
//! is the tour of the v2 side; the other v2 songs are still `const` section
//! literals — both kinds build the same [`SongSpec`].
//!
//! The atoms:
//! * [`Lane`] — a melodic pattern (scale degrees, [`REST`]s, [`HOLD`]
//!   ties). Author one with [`steps`] (`"0 _ _ . | 5 . 3 ."` — `.` rest,
//!   `_` tie, `|` cosmetic) or from raw degrees, then shape it with the
//!   combinators: [`transpose`], [`repeat`], [`cat`], [`every_other_bar`],
//!   [`stretch`], [`held`], [`sustain`], [`sparsify`] (seeded,
//!   deterministic).
//! * [`DrumLane`] — percussion, authored with [`hits`] (`"k.h.k.h."`, the
//!   whole kit: `k h s c o t r x`).
//! * [`Part`] — a lane bound to a channel: [`bass`], [`lead`], [`pad`],
//!   [`arp`], [`keys`], [`drums`], [`perc`], optionally scaled by
//!   [`with_velocity`], accented per step by [`Part::accents`], voiced by
//!   [`Part::voiced`].
//! * [`section`] — parts assembled into one named block; two parts on the
//!   same channel OVERLAY (later steps win where both sound), so
//!   `section("groove", [kick4(), hats(), snare24()])` layers a kit.
//! * [`song`] — key/tempo/waves plus `.arrange([...])`, the ordered list
//!   of sections (call a section function several times and the refrain
//!   comes back; give it an [`Intensity`] argument and it comes back
//!   hotter).

use super::songs::{
    Chord, Drum, Echo, Ramp, Scale, Section, Sidechain, SongSpec, Voice, Wave, ARP, BASS, DRUMS,
    HOLD, KEYS, LEAD, MAX_VEL, NUM_CHANNELS, NUM_VOICES, PAD, PERC, REST,
};

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

/// Parse a melodic lane from tokens: whitespace-separated, `.` = rest, `_`
/// = a TIE (the previous note holds through the step — `0 _ _ _` is one
/// quarter note), an integer = a scale degree (`0` root, `7` an octave up
/// on 7-note scales, negatives below the root), `|` a purely cosmetic bar
/// separator. Newlines are whitespace, so a multi-bar lane reads one bar
/// per line. Panics on a bad token — song builds run under `cargo test`, so
/// an authoring typo fails loudly in CI, exactly like the old generator's
/// validation.
pub fn steps(s: &str) -> Lane {
    Lane(
        s.split_whitespace()
            .filter(|t| *t != "|")
            .map(|t| match t {
                "." => REST,
                "_" => HOLD,
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
        if *d != REST && *d != HOLD {
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

/// [`stretch`] with SUSTAIN: every note becomes the note followed by
/// `factor - 1` ties (a rest becomes `factor` rests, a tie `factor` ties),
/// so a motif quoted at half speed keeps ringing instead of plucking and
/// resting. `factor` 0 is treated as 1.
pub fn held(lane: impl Into<Lane>, factor: usize) -> Lane {
    let l = lane.into();
    let factor = factor.max(1);
    let mut out = Vec::with_capacity(l.0.len() * factor);
    for d in l.0 {
        out.push(d);
        let fill = if d == REST { REST } else { HOLD };
        out.extend(std::iter::repeat_n(fill, factor - 1));
    }
    Lane(out)
}

/// Legato: every rest that follows a note becomes a tie, so each note
/// rings until the next one starts (rests BEFORE the first note stay).
/// `sustain("0 . . 3 . .")` = `0 _ _ 3 _ _`.
pub fn sustain(lane: impl Into<Lane>) -> Lane {
    let mut l = lane.into();
    let mut sounding = false;
    for d in &mut l.0 {
        match *d {
            REST if sounding => *d = HOLD,
            REST | HOLD => {}
            _ => sounding = true,
        }
    }
    l
}

/// Deterministic seeded variation: drop each note with probability `drop`
/// (0.0 = untouched, 1.0 = silence); a dropped note takes its ties with
/// it. The same `(riff, seed, drop)` always yields the same thinned riff —
/// call it twice in an arrangement and both occurrences match; change the
/// seed and the holes move.
pub fn sparsify(lane: impl Into<Lane>, seed: u32, drop: f64) -> Lane {
    let mut l = lane.into();
    let mut state = seed | 1; // xorshift must never be 0
    let mut dropping = false;
    for d in &mut l.0 {
        if *d == HOLD {
            if dropping {
                *d = REST;
            }
            continue;
        }
        dropping = false;
        if *d == REST {
            continue;
        }
        state ^= state << 13;
        state ^= state >> 17;
        state ^= state << 5;
        let r = (state >> 8) as f64 / (1u32 << 24) as f64;
        if r < drop {
            *d = REST;
            dropping = true;
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
/// hat, `s` snare, `c` clap, `o` open hat, `t` tom, `r` rim, `x` crash;
/// whitespace and `|` are cosmetic. Panics on anything else (fails loudly
/// under `cargo test`, like [`steps`]).
pub fn hits(s: &str) -> DrumLane {
    DrumLane(
        s.chars()
            .filter(|c| !c.is_whitespace() && *c != '|')
            .map(|c| match c {
                '.' => Drum::Silent,
                'k' => Drum::Kick,
                'h' => Drum::Hat,
                's' => Drum::Snare,
                'c' => Drum::Clap,
                'o' => Drum::OpenHat,
                't' => Drum::Tom,
                'r' => Drum::Rim,
                'x' => Drum::Crash,
                other => panic!("bad drum token {other:?} in lane {s:?}"),
            })
            .collect(),
    )
}

// --- velocity & chord lanes ---------------------------------------------------

/// A velocity lane under construction: one `0..=MAX_VEL` per step, looping
/// like the notes; empty = every note at full velocity.
#[derive(Clone, PartialEq, Debug, Default)]
pub struct VelLane(pub Vec<u8>);

impl From<&str> for VelLane {
    fn from(s: &str) -> VelLane {
        accents(s)
    }
}

impl From<Vec<u8>> for VelLane {
    fn from(v: Vec<u8>) -> VelLane {
        VelLane(v)
    }
}

/// Parse a velocity lane, one character per step: a digit `0`–`9`
/// (`MAX_VEL`) is that velocity, `.` is full; whitespace and `|` cosmetic.
/// `accents("9.6.")` accents the beat; `0` skips the note that step.
pub fn accents(s: &str) -> VelLane {
    VelLane(
        s.chars()
            .filter(|c| !c.is_whitespace() && *c != '|')
            .map(|c| match c {
                '.' => MAX_VEL,
                d if d.is_ascii_digit() => (d as u8 - b'0').min(MAX_VEL),
                other => panic!("bad velocity token {other:?} in lane {s:?}"),
            })
            .collect(),
    )
}

/// A chord (voicing) lane under construction: one voicing per step —
/// `None` = the lane's default — looping like the notes and read where a
/// note starts.
#[derive(Clone, PartialEq, Debug, Default)]
pub struct ChordLane(pub Vec<Option<Chord>>);

impl From<&str> for ChordLane {
    fn from(s: &str) -> ChordLane {
        chords(s)
    }
}

impl ChordLane {
    /// Every entry repeated `n` times — a voicing per BAR written as one
    /// token per bar: `chords("7 t 9 t").each(16)`.
    pub fn each(self, n: usize) -> ChordLane {
        let n = n.max(1);
        ChordLane(
            self.0
                .into_iter()
                .flat_map(|c| std::iter::repeat_n(c, n))
                .collect(),
        )
    }
}

/// Parse a chord lane, whitespace-separated: `.` the lane's default, `s`
/// single, `o` octave, `p` power, `t` triad, `2` sus2, `4` sus4, `7`
/// seventh, `9` add9, `i` first inversion, `j` second inversion, `w` the
/// wide open voicing; `|` cosmetic.
pub fn chords(s: &str) -> ChordLane {
    ChordLane(
        s.split_whitespace()
            .filter(|t| *t != "|")
            .map(|t| match t {
                "." => None,
                "s" => Some(Chord::Single),
                "o" => Some(Chord::Octave),
                "p" => Some(Chord::Power),
                "t" => Some(Chord::Triad),
                "2" => Some(Chord::Sus2),
                "4" => Some(Chord::Sus4),
                "7" => Some(Chord::Seventh),
                "9" => Some(Chord::Add9),
                "i" => Some(Chord::Inv1),
                "j" => Some(Chord::Inv2),
                "w" => Some(Chord::Open),
                other => panic!("bad chord token {other:?} in lane {s:?}"),
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

/// A lane bound to its tracker channel (plus a level, and optional
/// velocity / chord lanes), ready to be listed in a [`section`]. Build one
/// with [`bass`] / [`lead`] / [`pad`] / [`arp`] / [`keys`] / [`drums`] /
/// [`perc`].
#[derive(Clone, PartialEq, Debug)]
pub struct Part {
    channel: usize,
    lane: Lane,
    drum: DrumLane,
    vel: f32,
    accents: VelLane,
    chords: ChordLane,
}

impl Part {
    fn melodic(channel: usize, lane: Lane) -> Part {
        Part {
            channel,
            lane,
            drum: DrumLane::default(),
            vel: 1.0,
            accents: VelLane::default(),
            chords: ChordLane::default(),
        }
    }

    fn percussion(channel: usize, drum: DrumLane) -> Part {
        Part {
            channel,
            lane: Lane::default(),
            drum,
            vel: 1.0,
            accents: VelLane::default(),
            chords: ChordLane::default(),
        }
    }

    /// Method form of [`with_velocity`].
    pub fn vel(mut self, v: f32) -> Part {
        self.vel *= v.max(0.0);
        self
    }

    /// Per-step VELOCITY ([`accents`]): loops under the part's lane, `0`
    /// skips the note. Costs nothing in the bake budget (a play-time gain).
    pub fn accents(mut self, vel: impl Into<VelLane>) -> Part {
        self.accents = vel.into();
        self
    }

    /// Per-step VOICING ([`chords`]): loops under the part's lane, read
    /// where a note starts; `.` = the channel's default (a triad on the
    /// pad, the single note elsewhere). Melodic parts only.
    pub fn voiced(mut self, chords: impl Into<ChordLane>) -> Part {
        self.chords = chords.into();
        self
    }
}

/// The bass part of a section.
pub fn bass(lane: impl Into<Lane>) -> Part {
    Part::melodic(BASS, lane.into())
}

/// The lead/melody part of a section.
pub fn lead(lane: impl Into<Lane>) -> Part {
    Part::melodic(LEAD, lane.into())
}

/// The pad part of a section (each note blooms into a slow triad).
pub fn pad(lane: impl Into<Lane>) -> Part {
    Part::melodic(PAD, lane.into())
}

/// The arp part of a section (the fast high counter-melody).
pub fn arp(lane: impl Into<Lane>) -> Part {
    Part::melodic(ARP, lane.into())
}

/// The keys part of a section (stabs, a second lead, a counter-line, a
/// noise riser — whatever the four classic lanes leave no room for).
pub fn keys(lane: impl Into<Lane>) -> Part {
    Part::melodic(KEYS, lane.into())
}

/// The percussion part of a section.
pub fn drums(lane: impl Into<DrumLane>) -> Part {
    Part::percussion(DRUMS, lane.into())
}

/// The second percussion part (same kit): what must hit TOGETHER with the
/// drums — a hat riding over a kick, a clap under a snare.
pub fn perc(lane: impl Into<DrumLane>) -> Part {
    Part::percussion(PERC, lane.into())
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
    lanes: [Lane; NUM_VOICES],
    drum: [DrumLane; 2],
    vel: [f32; NUM_CHANNELS],
    accents: [VelLane; NUM_CHANNELS],
    chords: [ChordLane; NUM_VOICES],
    duck: bool,
    ramps: Vec<Ramp>,
}

impl SectionSpec {
    /// Enable the SIDECHAIN DUCK for this section: every kick step pumps
    /// the melodic voices down through the song's [`Sidechain`] (see
    /// [`DUCK_DEPTH`] / [`DUCK_RECOVERY`]) — the darksynth "everything
    /// breathes with the kick" feel.
    pub fn ducked(mut self) -> SectionSpec {
        self.duck = true;
        self
    }

    /// Start → end movements of the lanes' LIVE channels across this
    /// section (`Ramp::cutoff(LEAD, 300.0, 8000.0)`, `Ramp::reverb(..)`,
    /// `Ramp::level(..)`, …): automation, not baked, any voice.
    pub fn ramps(mut self, ramps: impl IntoIterator<Item = Ramp>) -> SectionSpec {
        self.ramps.extend(ramps);
        self
    }
}

/// Assemble named parts into one section: `section("groove", [kick4(),
/// offbeat_bass(), arp16()])`. Parts landing on the same channel OVERLAY:
/// the shorter one is looped up to the longer (playback's own looping
/// rule) and later parts win on the steps where both sound — so a kick
/// pattern, a hat pattern and a snare pattern layer into one kit lane.
/// Levels on the same channel multiply; a part's accents / chords follow
/// its notes through the overlay (the steps it wins keep its velocity and
/// voicing).
pub fn section(label: &'static str, parts: impl IntoIterator<Item = Part>) -> SectionSpec {
    let mut spec = SectionSpec {
        label,
        lanes: Default::default(),
        drum: Default::default(),
        vel: [1.0; NUM_CHANNELS],
        accents: Default::default(),
        chords: Default::default(),
        duck: false,
        ramps: Vec::new(),
    };
    for part in parts {
        let ch = part.channel;
        spec.vel[ch] *= part.vel;
        if ch == DRUMS || ch == PERC {
            let slot = &mut spec.drum[usize::from(ch == PERC)];
            let base = std::mem::take(slot);
            let wins = mask(&part.drum.0, |d| *d != Drum::Silent, base.len());
            let base_len = base.len();
            *slot = overlay_by(base, part.drum, &wins, Drum::Silent);
            let acc = &mut spec.accents[ch];
            *acc = overlay_vel(std::mem::take(acc), part.accents, &wins, base_len);
        } else {
            let slot = &mut spec.lanes[ch];
            let base = std::mem::take(slot);
            let wins = mask(&part.lane.0, |d| *d != REST, base.len());
            let base_len = base.len();
            *slot = overlay_by(base, part.lane, &wins, REST);
            let acc = &mut spec.accents[ch];
            *acc = overlay_vel(std::mem::take(acc), part.accents, &wins, base_len);
            let chd = &mut spec.chords[ch];
            *chd = overlay_chords(std::mem::take(chd), part.chords, &wins, base_len);
        }
    }
    spec
}

/// Which steps of the merged lane `over` WINS: those where `over` (looped)
/// sounds — all of them when there is no base. Length = the merged length.
fn mask<T>(over: &[T], sounds: impl Fn(&T) -> bool, base_len: usize) -> Vec<bool> {
    if base_len == 0 || over.is_empty() {
        return vec![true; over.len().max(base_len)];
    }
    (0..over.len().max(base_len))
        .map(|i| sounds(&over[i % over.len()]))
        .collect()
}

/// Overlay `over` onto `base` (a `Lane` or a `DrumLane` as raw vectors):
/// both loop-extended to the merged length, `over` on the steps it wins,
/// `base` elsewhere (`blank` where neither has anything).
fn overlay_by<L: From<Vec<T>> + Into<Vec<T>>, T: Copy>(
    base: L,
    over: L,
    wins: &[bool],
    blank: T,
) -> L {
    let (base, over): (Vec<T>, Vec<T>) = (base.into(), over.into());
    if base.is_empty() {
        return L::from(over);
    }
    if over.is_empty() {
        return L::from(base);
    }
    let out = wins
        .iter()
        .enumerate()
        .map(|(i, &w)| {
            if w {
                over[i % over.len()]
            } else if base.is_empty() {
                blank
            } else {
                base[i % base.len()]
            }
        })
        .collect();
    L::from(out)
}

impl From<Lane> for Vec<i32> {
    fn from(l: Lane) -> Vec<i32> {
        l.0
    }
}
impl From<DrumLane> for Vec<Drum> {
    fn from(l: DrumLane) -> Vec<Drum> {
        l.0
    }
}

/// The merged velocity lane: `over`'s velocity on the steps it wins,
/// `base`'s elsewhere; an EMPTY lane reads as full, and the result stays
/// empty when both are (no lane at all = nominal, nothing to bake or loop).
fn overlay_vel(base: VelLane, over: VelLane, wins: &[bool], base_len: usize) -> VelLane {
    if base.0.is_empty() && over.0.is_empty() {
        return VelLane::default();
    }
    if base_len == 0 {
        return over;
    }
    let at = |l: &VelLane, i: usize| {
        if l.0.is_empty() {
            MAX_VEL
        } else {
            l.0[i % l.0.len()]
        }
    };
    VelLane(
        wins.iter()
            .enumerate()
            .map(|(i, &w)| if w { at(&over, i) } else { at(&base, i) })
            .collect(),
    )
}

/// [`overlay_vel`] for chord lanes (`None` = the lane's default).
fn overlay_chords(base: ChordLane, over: ChordLane, wins: &[bool], base_len: usize) -> ChordLane {
    if base.0.is_empty() && over.0.is_empty() {
        return ChordLane::default();
    }
    if base_len == 0 {
        return over;
    }
    let at = |l: &ChordLane, i: usize| {
        if l.0.is_empty() {
            None
        } else {
            l.0[i % l.0.len()]
        }
    };
    ChordLane(
        wins.iter()
            .enumerate()
            .map(|(i, &w)| if w { at(&over, i) } else { at(&base, i) })
            .collect(),
    )
}

// --- the song builder -------------------------------------------------------

/// How far a `.ducked()` section's kicks pull the melodic lanes down (the
/// [`Sidechain::depth`] every built song gets: a dip to 0.35).
pub const DUCK_DEPTH: f64 = 0.65;

/// Seconds a duck takes to recover (~95 %) — fast, well under a beat at
/// combat tempi, so the pump breathes with the kick instead of smearing.
/// The builder converts it to the song's tempo ([`Sidechain::release_beats`]).
pub const DUCK_RECOVERY: f64 = 0.3;

/// A whole song under construction. Start with [`song`], chain the
/// settings, `.arrange([...])` the sections, `.build()` once (memoize the
/// result in a `OnceLock` — see any file in `src/audio/songs/`).
#[derive(Clone, PartialEq, Debug)]
pub struct SongBuilder {
    name: &'static str,
    key: Key,
    bpm: f64,
    steps_per_beat: u32,
    voices: [Voice; NUM_VOICES],
    intensity: f64,
    swing: f64,
    sidechain: Sidechain,
    echo: Echo,
    humanize: f64,
    sweep: f64,
    melodic_gain: f64,
    sections: Vec<SectionSpec>,
}

/// A song in `key` at `bpm` beats per minute (sixteenth-note resolution by
/// default). Defaults: plain centred sawtooths (a sine on KEYS), intensity
/// 0.8, straight sixteenths, the classic `.ducked()` pump
/// ([`DUCK_DEPTH`] / [`DUCK_RECOVERY`]), a dotted echo nothing sends into,
/// the full bar sweep, no sections.
pub fn song(name: &'static str, key: Key, bpm: f64) -> SongBuilder {
    let mut voices = [Voice::mono(Wave::Sawtooth); NUM_VOICES];
    voices[KEYS] = Voice::mono(Wave::Sine);
    SongBuilder {
        name,
        key,
        bpm,
        steps_per_beat: 4,
        voices,
        intensity: 0.8,
        swing: 0.0,
        sidechain: Sidechain::new(DUCK_DEPTH, DUCK_RECOVERY * bpm / 60.0),
        echo: Echo::DOTTED,
        humanize: 0.0,
        sweep: 1.0,
        // Plain centred lanes: make the panners' centre law up exactly
        // (see `SongSpec::melodic_gain`).
        melodic_gain: std::f64::consts::SQRT_2,
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
    /// each classic melodic voice, plain and centred: bass, lead, pad, arp.
    pub fn waves(mut self, bass: Wave, lead: Wave, pad: Wave, arp: Wave) -> SongBuilder {
        for (lane, wave) in [BASS, LEAD, PAD, ARP]
            .into_iter()
            .zip([bass, lead, pad, arp])
        {
            self.voices[lane] = Voice::mono(wave);
        }
        self
    }

    /// The full INSTRUMENT of each melodic lane (stereo position, unison
    /// stack, filter envelope, vibrato, sends … — the [`Voice`] builders):
    /// bass, lead, pad, arp, keys.
    pub fn voices(
        mut self,
        bass: Voice,
        lead: Voice,
        pad: Voice,
        arp: Voice,
        keys: Voice,
    ) -> SongBuilder {
        self.voices = [Voice::mono(Wave::Sine); NUM_VOICES];
        for (lane, v) in [BASS, LEAD, PAD, ARP, KEYS]
            .into_iter()
            .zip([bass, lead, pad, arp, keys])
        {
            self.voices[lane] = v;
        }
        self
    }

    /// Overall punch/loudness feel (~0.5 lounge .. ~1.2 boss).
    pub fn intensity(mut self, intensity: f64) -> SongBuilder {
        self.intensity = intensity;
        self
    }

    /// Shuffle, `0.0` (straight) … `1.0` (full triplet swing).
    pub fn swing(mut self, swing: f64) -> SongBuilder {
        self.swing = swing;
        self
    }

    /// The kick-driven ducker the `.ducked()` sections pump through:
    /// `depth` (0..1) and its release in BEATS ([`Sidechain::OFF`] = none).
    pub fn sidechain(mut self, sidechain: Sidechain) -> SongBuilder {
        self.sidechain = sidechain;
        self
    }

    /// The shared echo line the voices' `with_echo` sends feed.
    pub fn echo(mut self, echo: Echo) -> SongBuilder {
        self.echo = echo;
        self
    }

    /// Timing humanize in seconds (≤ 0.02; every note but the kicks).
    pub fn humanize(mut self, humanize: f64) -> SongBuilder {
        self.humanize = humanize;
        self
    }

    /// Depth of the bus lowpass's once-per-bar sweep, `1.0` … `0.0` (open).
    pub fn sweep(mut self, sweep: f64) -> SongBuilder {
        self.sweep = sweep;
        self
    }

    /// The bake-time gain of the melodic lanes (see
    /// `SongSpec::melodic_gain`): `√2` by default — a plain centred lane as
    /// loud as the drums; `1.0` for a song MIXED with its stereo voices.
    pub fn melodic_gain(mut self, gain: f64) -> SongBuilder {
        self.melodic_gain = gain;
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
        fn leak<T>(v: Vec<T>) -> &'static [T] {
            Box::leak(v.into_boxed_slice())
        }
        let sections: Vec<Section> = self
            .sections
            .into_iter()
            .map(|s| {
                let [b, l, p, a, k] = s.lanes;
                let [d, x] = s.drum;
                let [bv, lv, pv, av, kv, dv, xv] = s.accents;
                // `None` (the default) is resolved here, per channel.
                let voiced = |ch: usize, c: ChordLane| -> &'static [Chord] {
                    leak(
                        c.0.into_iter()
                            .map(|v| v.unwrap_or(Chord::default_for(ch)))
                            .collect(),
                    )
                };
                let [bc, lc, pc, ac, kc] = s.chords;
                Section {
                    label: s.label,
                    bass: leak(b.0),
                    lead: leak(l.0),
                    pad: leak(p.0),
                    arp: leak(a.0),
                    keys: leak(k.0),
                    drums: leak(d.0),
                    perc: leak(x.0),
                    bass_vel: leak(bv.0),
                    lead_vel: leak(lv.0),
                    pad_vel: leak(pv.0),
                    arp_vel: leak(av.0),
                    keys_vel: leak(kv.0),
                    drums_vel: leak(dv.0),
                    perc_vel: leak(xv.0),
                    bass_chord: voiced(BASS, bc),
                    lead_chord: voiced(LEAD, lc),
                    pad_chord: voiced(PAD, pc),
                    arp_chord: voiced(ARP, ac),
                    keys_chord: voiced(KEYS, kc),
                    level: s.vel,
                    duck: s.duck,
                    ramps: leak(s.ramps),
                }
            })
            .collect();
        SongSpec {
            name: self.name,
            root: self.key.root,
            scale: self.key.scale,
            bpm: self.bpm,
            steps_per_beat: self.steps_per_beat,
            voices: self.voices,
            sections: leak(sections),
            intensity: self.intensity,
            swing: self.swing,
            sidechain: self.sidechain,
            echo: self.echo,
            humanize: self.humanize,
            sweep: self.sweep,
            melodic_gain: self.melodic_gain,
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
        assert_eq!(
            hits("cotrx").0,
            vec![Drum::Clap, Drum::OpenHat, Drum::Tom, Drum::Rim, Drum::Crash]
        );
    }

    #[test]
    fn ties_parse_transpose_and_hold() {
        assert_eq!(steps("0 _ _ . 3 _").0, vec![0, HOLD, HOLD, REST, 3, HOLD]);
        assert_eq!(transpose(steps("0 _ . 3"), 7).0, vec![7, HOLD, REST, 10]);
        assert_eq!(
            held(steps("0 . 3"), 3).0,
            vec![0, HOLD, HOLD, REST, REST, REST, 3, HOLD, HOLD]
        );
        assert_eq!(held(steps("0 _"), 2).0, vec![0, HOLD, HOLD, HOLD]);
        assert_eq!(held(steps("0 3"), 0).0, vec![0, 3]);
        assert_eq!(
            sustain(steps(". 0 . . 3 . _ .")).0,
            vec![REST, 0, HOLD, HOLD, 3, HOLD, HOLD, HOLD]
        );
        // A dropped note takes its ties with it (no orphan HOLDs).
        let l = sparsify(steps("0 _ _ 3 _ 5 _"), 1, 1.0);
        assert_eq!(l.0, vec![REST; 7]);
        let l = sparsify(steps("0 _ _ 3 _"), 1, 0.0);
        assert_eq!(l.0, vec![0, HOLD, HOLD, 3, HOLD]);
    }

    #[test]
    fn accents_and_chords_parse() {
        assert_eq!(accents("9.60 | 5").0, vec![9, 9, 6, 0, 5]);
        assert_eq!(
            chords("7 . t p").0,
            vec![
                Some(Chord::Seventh),
                None,
                Some(Chord::Triad),
                Some(Chord::Power)
            ]
        );
        assert_eq!(chords("s o").each(2).0.len(), 4);
        assert_eq!(chords("s 2 4 9 i j w").0.len(), 7);
    }

    #[test]
    #[should_panic(expected = "bad velocity token")]
    fn accents_reject_garbage() {
        accents("9x");
    }

    #[test]
    #[should_panic(expected = "bad chord token")]
    fn chords_reject_garbage() {
        chords("t q");
    }

    /// A part's accents and voicing follow its notes through the overlay:
    /// the steps it wins keep its velocity / chord, the base keeps its own;
    /// an empty lane reads as full / default, and two empty lanes stay
    /// EMPTY (nothing to loop or bake).
    #[test]
    fn accents_and_chords_follow_the_overlay() {
        let s = section(
            "v",
            [
                lead("0 . 3 .").accents("5.5.").voiced("t . t ."),
                lead(". 5 . .").accents(".8..").voiced(". p . ."),
            ],
        );
        assert_eq!(s.lanes[LEAD].0, vec![0, 5, 3, REST]);
        assert_eq!(s.accents[LEAD].0, vec![5, 8, 5, 9]);
        assert_eq!(
            s.chords[LEAD].0,
            vec![
                Some(Chord::Triad),
                Some(Chord::Power),
                Some(Chord::Triad),
                None
            ]
        );
        // A 4-step accented part looping under an 8-step plain one.
        let s = section(
            "w",
            [lead("0 . 3 . 5 . 3 ."), lead(". 7 . .").accents(".4..")],
        );
        assert_eq!(s.lanes[LEAD].0, vec![0, 7, 3, REST, 5, 7, 3, REST]);
        assert_eq!(s.accents[LEAD].0, vec![9, 4, 9, 9, 9, 4, 9, 9]);
        let s = section("x", [lead("0 ."), lead(". 3")]);
        assert!(s.accents[LEAD].0.is_empty() && s.chords[LEAD].0.is_empty());
        // Percussion: the PERC lane is its own channel, with its own accents.
        let s = section(
            "p",
            [drums("k..."), perc("h.h.").accents("4060"), drums("...s")],
        );
        assert_eq!(s.drum[0].0, vec![Kick, Silent, Silent, Snare]);
        assert_eq!(s.drum[1].0, vec![Hat, Silent, Hat, Silent]);
        assert_eq!(s.accents[PERC].0, vec![4, 0, 6, 0]);
        assert!(s.accents[DRUMS].0.is_empty());
        assert_eq!(section("k", [keys("0 _")]).lanes[KEYS].0, vec![0, HOLD]);
    }

    /// The v2 song settings reach the `SongSpec`; a chord lane's `.` is
    /// resolved to the channel's default at build time.
    #[test]
    fn build_carries_the_v2_settings() {
        let s = song("V2", Key::new(A1, MINOR), 100.0)
            .voices(
                Voice::mono(Wave::Sine).with_sub(0.3),
                Voice::panned(Wave::Square, 0.4),
                Voice::wide(Wave::Sawtooth, 0.0, 9.0, 0.7),
                Voice::mono(Wave::Triangle),
                Voice::mono(Wave::Noise),
            )
            .swing(0.3)
            .sidechain(Sidechain::new(0.5, 1.5))
            .echo(Echo::new(4.0, 0.5, 2000.0))
            .humanize(0.005)
            .sweep(0.2)
            .melodic_gain(1.0)
            .arrange([section(
                "s",
                [
                    pad("0 . 3 .").voiced(". p . ."),
                    lead("7").voiced(". ."),
                    keys("0 _ _ _").accents("7"),
                    perc("h.").accents("4."),
                    drums("k...").accents("9"),
                ],
            )
            .ramps([
                Ramp::cutoff(PAD, 300.0, 6000.0),
                Ramp::level(LEAD, 0.0, 1.0),
            ])])
            .build();
        assert_eq!(s.voices[LEAD].pan, 0.4);
        assert_eq!(s.voices[KEYS].wave, Wave::Noise);
        assert_eq!(
            (s.swing, s.humanize, s.sweep, s.melodic_gain),
            (0.3, 0.005, 0.2, 1.0)
        );
        assert_eq!(s.sidechain, Sidechain::new(0.5, 1.5));
        assert_eq!(s.echo.steps, 4.0);
        let sec = &s.sections[0];
        assert_eq!(
            sec.pad_chord,
            &[Chord::Triad, Chord::Power, Chord::Triad, Chord::Triad]
        );
        assert_eq!(sec.lead_chord, &[Chord::Single, Chord::Single]);
        assert_eq!(sec.keys, &[0, HOLD, HOLD, HOLD]);
        assert_eq!(
            (sec.keys_vel, sec.perc_vel, sec.drums_vel),
            (&[7][..], &[4, 9][..], &[9][..])
        );
        assert_eq!(sec.perc, &[Drum::Hat, Silent]);
        assert!(sec.bass_vel.is_empty() && sec.arp_chord.is_empty());
        assert_eq!(sec.ramps.len(), 2);
        assert_eq!((sec.ramps[1].lane, sec.ramps[1].to), (LEAD, 1.0));
        // The default song: what the briefed tracks got before v2.
        let plain = song("P", Key::new(A1, MINOR), 120.0).build();
        assert_eq!(plain.voices[BASS], Voice::mono(Wave::Sawtooth));
        assert_eq!(plain.voices[KEYS], Voice::mono(Wave::Sine));
        assert_eq!(plain.melodic_gain, std::f64::consts::SQRT_2);
        assert!((plain.sidechain.release_beats - 0.6).abs() < 1e-12);
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
        assert_eq!(s.drum[0].0, vec![Kick, Silent, Hat, Silent]);
        assert_eq!(s.vel, [1.0; NUM_CHANNELS]);
        assert!(!s.duck);
        assert!(section("d", []).ducked().duck);
    }

    #[test]
    fn same_channel_parts_overlay_and_velocities_multiply() {
        // A 4-step kick loops under the 8-step snare overlay.
        let s = section("kit", [drums("k..."), drums(".......s")]);
        assert_eq!(
            s.drum[0].0,
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
        assert_eq!(s.voices[BASS], Voice::mono(Wave::DrivenBass));
        assert_eq!(s.voices[ARP].wave, Wave::Square);
        assert!(s.sidechain.active() && (s.sidechain.release_beats - 0.6).abs() < 1e-12);
        assert_eq!(s.sections[0].level, [1.0; NUM_CHANNELS]);
        assert_eq!(s.sections[0].keys, &[] as &[i32]);
        assert_eq!(s.sections.len(), 3);
        assert_eq!(s.sections[0].label, "verse");
        assert_eq!(s.sections[0].bass, &riff().0[..]);
        assert_eq!(s.sections[0].lead, &transpose(riff(), 7).0[..]);
        assert_eq!(s.sections[0].pad, &[] as &[i32]);
        assert_eq!(s.sections[0].drums.len(), 16);
        assert!(s.sections[1].duck && !s.sections[0].duck);
        assert!(s.sections[0].ramps.is_empty());
        // The two refrain() calls produce identical content.
        assert_eq!(s.sections[1].bass, s.sections[2].bass);
        assert_eq!(s.sections[1].drums, s.sections[2].drums);
    }
}
