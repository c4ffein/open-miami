//! The INSTRUMENT side of the song format: what a melodic lane's notes are
//! synthesized with ([`Voice`] and its [`Wave`] / [`Env`] / [`Filter`] /
//! [`Vibrato`]), how a note is voiced ([`Chord`]), and the song-level bus
//! effects ([`Sidechain`], [`Echo`]) with their pure curves ([`duck_level`],
//! [`swing_delay`]). Plain `const`-able data + arithmetic, compiled and
//! tested natively; `audio/engine` is what turns it into Web Audio nodes.
//! Re-exported wholesale by [`super::songs`] — a song file names everything
//! through there.

use super::songs::PAD;

/// Oscillator shape — or synthesis PRESET — of a melodic [`Voice`]. A plain
/// enum so the song data is host-compilable; the engine maps the four basic
/// shapes to `OscillatorType`, plays [`Wave::Noise`] from the shared noise
/// buffer and builds the presets from small node graphs (see the "music
/// voices" section of `audio/engine/music.rs`). Presets bake per note
/// exactly like the basic shapes, so they cost the same in the bake budget.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Wave {
    Sine,
    Square,
    Sawtooth,
    Triangle,
    /// DARKSYNTH PRESET: five detuned sawtooths with a slight spread — the
    /// wide, hissing supersaw lead/pad of every darksynth record.
    Supersaw,
    /// DARKSYNTH PRESET: a saw+square pair driven through a waveshaper-style
    /// soft clip (`WaveShaperNode`) — a growling, overdriven bass.
    DrivenBass,
    /// DARKSYNTH PRESET: a slow-attack detuned saw pair through a fixed
    /// lowpass — a dark, breathing chord bed.
    DarkPad,
    /// White noise instead of a pitched oscillator: the note's degree is
    /// ignored, its envelope and the voice's [`Filter`] shape the sound —
    /// a riser (long tie, slow filter attack), a snare-ish hit, wind.
    Noise,
    /// COMPUTED (`audio/dsp.rs`, rendered sample by sample into the bake):
    /// a plucked steel string — a picked guitar. Chords strum, low string
    /// first.
    Guitar,
    /// COMPUTED: a plucked bass string — a fingered bass guitar.
    BassGuitar,
    /// COMPUTED: a bowed string — a violin / viola line. Onsets swell (60 ms
    /// at least); give it `with_vibrato` and `with_glide`.
    Violin,
    /// COMPUTED: the REESE bass — two saws a few cents apart, their beating
    /// folded through a soft clip: the phasing growl under every dubstep /
    /// DnB drop. Put a [`Wobble`] on it.
    Reese,
    /// COMPUTED: two-operator FM — a sine carrier modulated at its own
    /// pitch by an index that decays over the note: a metallic growl that
    /// softens into a tone. Stabs, growls, bells at low index.
    Fm,
}

impl Wave {
    /// A PRESET brings its own node graph (its own stack, filter and drive):
    /// the [`Voice`]'s unison / filter / vibrato / glide do not apply to it,
    /// its envelope, ties, chords, sub, pan, lane drive and sends do.
    pub const fn is_preset(self) -> bool {
        matches!(self, Wave::Supersaw | Wave::DrivenBass | Wave::DarkPad)
    }

    /// A COMPUTED voice is rendered in Rust (`audio/dsp.rs`), not from Web
    /// Audio nodes; the [`Voice`]'s unison stack (a stereo bake when wide),
    /// filter envelope, vibrato, glide, bend, wobble, sub and chords all
    /// apply.
    pub const fn is_computed(self) -> bool {
        matches!(
            self,
            Wave::Guitar | Wave::BassGuitar | Wave::Violin | Wave::Reese | Wave::Fm
        )
    }
}

/// How a lane note is voiced: which scale degrees sound, relative to the
/// written one. In-key by construction — a `Triad` on the 5th degree of a
/// minor scale is whatever chord the scale spells there — so voicings move
/// with the key like the notes do. The pad's default is `Triad`, every
/// other lane's is `Single`; a `*_chord` lane changes it per step.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum Chord {
    /// Just the written degree.
    Single,
    /// Root + the octave above — the thick unison lead / bass.
    Octave,
    /// Root, fifth, octave: the power chord (no third; heavy).
    Power,
    /// Root, third, fifth in root position — the default chord bed.
    Triad,
    /// Root, second, fifth: suspended, open, unresolved.
    Sus2,
    /// Root, fourth, fifth: suspended, leaning to resolve.
    Sus4,
    /// Root, third, fifth, seventh: the lush four-note chord.
    Seventh,
    /// Root, third, fifth + the ninth on top: wide and dreamy.
    Add9,
    /// First inversion: third, fifth, root-up-an-octave (smoother voice
    /// leading between neighbouring chords).
    Inv1,
    /// Second inversion: fifth, root, third all up — bright and floating.
    Inv2,
    /// Root, fifth, tenth (the third an octave up): the wide-open voicing.
    Open,
}

impl Chord {
    /// The scale-degree offsets that sound, lowest first.
    pub const fn degrees(self) -> &'static [i32] {
        match self {
            Chord::Single => &[0],
            Chord::Octave => &[0, 7],
            Chord::Power => &[0, 4, 7],
            Chord::Triad => &[0, 2, 4],
            Chord::Sus2 => &[0, 1, 4],
            Chord::Sus4 => &[0, 3, 4],
            Chord::Seventh => &[0, 2, 4, 6],
            Chord::Add9 => &[0, 2, 4, 8],
            Chord::Inv1 => &[2, 4, 7],
            Chord::Inv2 => &[4, 7, 9],
            Chord::Open => &[0, 4, 9],
        }
    }

    /// The voicing a lane uses where its chord lane is empty.
    pub const fn default_for(lane: usize) -> Chord {
        if lane == PAD {
            Chord::Triad
        } else {
            Chord::Single
        }
    }
}

/// A voice's amplitude envelope, overriding the lane's built-in shape:
/// `attack` seconds up to peak, then (after the tied steps, held at peak)
/// an exponential decay to silence over `gate` STEPS. The lane defaults
/// are bass 1.9 / lead 0.9 / pad 4.0 / arp 0.7 / keys 1.2 steps of tail.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Env {
    pub attack: f64,
    pub gate: f64,
}

/// A per-note LOWPASS filter envelope — the "wow" of a synth stab and the
/// slow bloom of a pad both live here. The cutoff starts at `cutoff`, opens
/// to `peak` over `attack` seconds (instantly when `attack == 0`), then
/// falls back to `cutoff` over `decay` seconds (stays open when `decay ==
/// 0`). `q` is the resonance (0.7 flat … 8 screaming).
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Filter {
    pub cutoff: f64,
    pub peak: f64,
    pub attack: f64,
    pub decay: f64,
    pub q: f64,
}

/// Pitch vibrato: a sine LFO at `rate` Hz, `depth` cents peak, fading in
/// over `delay` seconds after the note starts (the singer's late vibrato).
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Vibrato {
    pub rate: f64,
    pub depth: f64,
    pub delay: f64,
}

/// A per-note PITCH BEND: every note starts `semitones` away from its
/// pitch (negative = from below) and arrives over `seconds` (the whole note
/// when 0) — the "yoy" of a synth lead, the dive of a laser, the scoop of
/// a bass. Baked into the note; a legato glide (`Voice::glide`) takes
/// precedence where it applies.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Bend {
    pub semitones: f64,
    pub seconds: f64,
}

/// The WOBBLE: a tempo-synced LFO on a resonant lowpass, retriggered by
/// every note. The cutoff swings between `cutoff` and `peak` Hz once every
/// `steps` sequencer steps (`4.0` = a beat, `2.0` = an eighth, `1.0` = a
/// sixteenth: the faster the wobble, the more the drop grinds), starting
/// CLOSED at the note's start; `q` is the resonance. Baked, so the LFO's
/// phase is per note — how a wobble bass is played.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Wobble {
    pub steps: f64,
    pub cutoff: f64,
    pub peak: f64,
    pub q: f64,
}

/// A live lane parameter a [`Ramp`] can move over a section.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum RampParam {
    /// The lane's live lowpass cutoff, Hz (open = 20 kHz).
    Cutoff,
    /// The lane's echo send, `0..1`.
    Echo,
    /// The lane's hall send, `0..1`.
    Reverb,
    /// The lane's stereo position, `-1..1`.
    Pan,
    /// The lane's live level, `0..1`.
    Level,
}

/// A start → end movement of one lane's live channel across ONE SECTION:
/// a filter opening over sixteen bars, a send rising into the drop, a pan
/// drifting. Automation on the live nodes — costs nothing to bake and
/// works on every voice. Where a section has no ramp for a parameter, the
/// voice's static value is restored at the section's start.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Ramp {
    pub lane: usize,
    pub param: RampParam,
    pub from: f64,
    pub to: f64,
    /// How many sections the ramp runs across, from the start of the one
    /// it is listed in (`1` = that section). The sections it runs through
    /// leave the parameter alone at their starts.
    pub span: u32,
}

impl Ramp {
    pub const fn new(lane: usize, param: RampParam, from: f64, to: f64) -> Ramp {
        Ramp {
            lane,
            param,
            from,
            to,
            span: 1,
        }
    }

    /// The ramp stretched over `sections` sections (a sixty-second swell
    /// is one ramp, not three chained ones).
    pub const fn over(self, sections: u32) -> Ramp {
        Ramp {
            span: if sections == 0 { 1 } else { sections },
            ..self
        }
    }
    /// The lane's live lowpass, `from` → `to` Hz (exponential).
    pub const fn cutoff(lane: usize, from: f64, to: f64) -> Ramp {
        Ramp::new(lane, RampParam::Cutoff, from, to)
    }
    /// The lane's echo send.
    pub const fn echo(lane: usize, from: f64, to: f64) -> Ramp {
        Ramp::new(lane, RampParam::Echo, from, to)
    }
    /// The lane's hall send.
    pub const fn reverb(lane: usize, from: f64, to: f64) -> Ramp {
        Ramp::new(lane, RampParam::Reverb, from, to)
    }
    /// The lane's stereo position.
    pub const fn pan(lane: usize, from: f64, to: f64) -> Ramp {
        Ramp::new(lane, RampParam::Pan, from, to)
    }
    /// The lane's live level (a fade in / out that no accent lane can do).
    pub const fn level(lane: usize, from: f64, to: f64) -> Ramp {
        Ramp::new(lane, RampParam::Level, from, to)
    }
}

/// One melodic instrument of a song: what a lane's notes are synthesized
/// with. Cheap on purpose — a voice is baked once per note it plays. Build
/// one with [`Voice::mono`] / [`Voice::panned`] / [`Voice::wide`] /
/// [`Voice::stack`] and refine it with the `with_*` builders.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Voice {
    /// Oscillator shape (or preset).
    pub wave: Wave,
    /// Stereo position of the lane, `-1.0` (hard left) … `1.0` (hard right).
    pub pan: f64,
    /// Unison detune in cents: with `unison >= 2` the oscillators spread
    /// evenly over `±detune` (the fat / supersaw thickness of synthwave
    /// pads and leads); `0` = every oscillator at pitch (a single one).
    pub detune: f64,
    /// Stereo WIDTH of the unison stack, `0.0` (all at the lane's pan) …
    /// `1.0` (spread hard left / right around it). Only with `detune > 0`;
    /// a wide voice bakes to a stereo buffer.
    pub width: f64,
    /// Oscillators per note, `1` … `7`. Two is the classic pair; five to
    /// seven is the supersaw.
    pub unison: u8,
    /// Lane-level soft-clip drive, `0.0` (clean) … `1.0` (crushed): a
    /// tanh waveshaper on the lane's summed output (chords intermodulate
    /// through it) that also lifts its quiet parts — density, grit, edge.
    pub drive: f64,
    /// Amplitude envelope override (`None` = the lane's built-in shape).
    pub env: Option<Env>,
    /// Per-note lowpass filter envelope (`None` = unfiltered).
    pub filter: Option<Filter>,
    /// Pitch vibrato (`None` = none).
    pub vibrato: Option<Vibrato>,
    /// Send level into the song's [`Echo`], `0.0` (dry) … `1.0`.
    pub echo: f64,
    /// Send level into the music hall reverb, `0.0` (dry) … `1.0`.
    pub reverb: f64,
    /// A sine SUB-oscillator one octave below the note's lowest partial, at
    /// this level relative to the note (`0.0` = none). Centred, unfiltered,
    /// undetuned: the weight under a bass.
    pub sub: f64,
    /// Portamento time in seconds: a note that starts exactly where the
    /// lane's previous note ends (legato — no rest between) GLIDES into
    /// its pitch from the previous one over this long. `0.0` = no glide.
    pub glide: f64,
    /// A pitch bend into every note (`None` = none). See [`Bend`].
    pub bend: Option<Bend>,
    /// A tempo-synced filter LFO on every note (`None` = none). See
    /// [`Wobble`].
    pub wobble: Option<Wobble>,
}

impl Voice {
    /// A single centred oscillator (or preset) — the plain sound.
    pub const fn mono(wave: Wave) -> Self {
        Self {
            wave,
            pan: 0.0,
            detune: 0.0,
            width: 0.0,
            unison: 1,
            drive: 0.0,
            env: None,
            filter: None,
            vibrato: None,
            echo: 0.0,
            reverb: 0.0,
            sub: 0.0,
            glide: 0.0,
            bend: None,
            wobble: None,
        }
    }

    /// A single oscillator placed at `pan`.
    pub const fn panned(wave: Wave, pan: f64) -> Self {
        Self {
            pan,
            ..Self::mono(wave)
        }
    }

    /// A detuned unison pair (`detune` cents) at `pan`, spread by `width`.
    pub const fn wide(wave: Wave, pan: f64, detune: f64, width: f64) -> Self {
        Self {
            pan,
            detune,
            width,
            unison: 2,
            ..Self::mono(wave)
        }
    }

    /// A unison STACK of `unison` oscillators spread over `±detune` cents
    /// and `±width` around `pan` — the supersaw.
    pub const fn stack(wave: Wave, pan: f64, detune: f64, width: f64, unison: u8) -> Self {
        Self {
            pan,
            detune,
            width,
            unison,
            ..Self::mono(wave)
        }
    }

    /// With an amplitude envelope override.
    pub const fn with_env(self, attack: f64, gate: f64) -> Self {
        Self {
            env: Some(Env { attack, gate }),
            ..self
        }
    }

    /// With a per-note lowpass filter envelope (see [`Filter`]).
    pub const fn with_filter(
        self,
        cutoff: f64,
        peak: f64,
        attack: f64,
        decay: f64,
        q: f64,
    ) -> Self {
        Self {
            filter: Some(Filter {
                cutoff,
                peak,
                attack,
                decay,
                q,
            }),
            ..self
        }
    }

    /// With pitch vibrato (see [`Vibrato`]).
    pub const fn with_vibrato(self, rate: f64, depth: f64, delay: f64) -> Self {
        Self {
            vibrato: Some(Vibrato { rate, depth, delay }),
            ..self
        }
    }

    /// With lane drive (see [`Voice::drive`]).
    pub const fn with_drive(self, drive: f64) -> Self {
        Self { drive, ..self }
    }

    /// With a send into the song's echo (see [`Echo`]).
    pub const fn with_echo(self, echo: f64) -> Self {
        Self { echo, ..self }
    }

    /// With a send into the hall reverb.
    pub const fn with_reverb(self, reverb: f64) -> Self {
        Self { reverb, ..self }
    }

    /// With a sine sub-oscillator an octave down at `sub` of the note.
    pub const fn with_sub(self, sub: f64) -> Self {
        Self { sub, ..self }
    }

    /// With legato portamento over `glide` seconds (see [`Voice::glide`]).
    pub const fn with_glide(self, glide: f64) -> Self {
        Self { glide, ..self }
    }

    /// With a pitch bend into every note: from `semitones` away, arriving
    /// over `seconds` (see [`Bend`]).
    pub const fn with_bend(self, semitones: f64, seconds: f64) -> Self {
        Self {
            bend: Some(Bend { semitones, seconds }),
            ..self
        }
    }

    /// With a WOBBLE: a resonant lowpass swinging `cutoff` → `peak` Hz once
    /// every `steps` steps, retriggered per note (see [`Wobble`]).
    pub const fn with_wobble(self, steps: f64, cutoff: f64, peak: f64, q: f64) -> Self {
        Self {
            wobble: Some(Wobble {
                steps,
                cutoff,
                peak,
                q,
            }),
            ..self
        }
    }

    /// How many oscillators a note of this voice actually runs: the stack
    /// only exists with a detune to spread it over, and only on a raw shape
    /// or a computed voice (noise has no pitch, a preset brings its own
    /// stack).
    pub fn oscillators(&self) -> usize {
        if self.detune > 0.0 && self.wave != Wave::Noise && !self.wave.is_preset() {
            (self.unison.clamp(1, 7)) as usize
        } else {
            1
        }
    }

    /// Whether a note of this voice is a stereo image of its own (a spread
    /// stack), as opposed to a point the lane's panner places.
    pub fn is_wide(&self) -> bool {
        self.oscillators() > 1 && self.width > 0.0
    }

    /// Whether legato notes of this voice glide (what puts a note's origin
    /// into its bake key — see `songs::note_key`).
    pub fn glides(&self) -> bool {
        self.glide > 0.0 && self.wave != Wave::Noise && !self.wave.is_preset()
    }
}

/// The music bus's SIDE-CHAIN ducker: every kick of a section with
/// `Section::duck` pulls the melodic lanes down by `depth` in ~4 ms and lets
/// them swell back with an exponential release — the pumping that glues a
/// synthwave mix to its four-on-the-floor. The drums are never ducked.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Sidechain {
    /// How far the lanes drop on a kick, `0.0` (off) … `1.0` (to silence).
    pub depth: f64,
    /// Recovery time in BEATS (tempo-synced): the lanes are ~95 % back this
    /// long after the kick. `1.0` = a full beat of pump.
    pub release_beats: f64,
}

impl Sidechain {
    /// No ducking.
    pub const OFF: Sidechain = Sidechain {
        depth: 0.0,
        release_beats: 1.0,
    };

    /// A ducker of `depth` recovering over `release_beats`.
    pub const fn new(depth: f64, release_beats: f64) -> Self {
        Self {
            depth,
            release_beats,
        }
    }

    /// Whether the ducker does anything.
    pub fn active(&self) -> bool {
        self.depth > 0.0
    }
}

/// The ducker's gain `dt` seconds into its exponential recovery (time
/// constant `tau`), having dropped to `1 - depth` at `dt = 0`. The engine
/// programs exactly this curve (`setTargetAtTime`) and uses this function to
/// pick a retriggering kick up from wherever the previous release is.
pub fn duck_level(depth: f64, tau: f64, dt: f64) -> f64 {
    if tau <= 0.0 {
        return 1.0;
    }
    (1.0 - depth * (-dt.max(0.0) / tau).exp()).clamp(0.0, 1.0)
}

/// How late step `step` fires under `swing` (0 = straight … 1 = full
/// triplet shuffle): every odd sixteenth is delayed by up to a third of a
/// step, the even ones stay on the grid.
pub fn swing_delay(swing: f64, step: usize, step_dur: f64) -> f64 {
    if step % 2 == 1 {
        swing.clamp(0.0, 1.0) * step_dur / 3.0
    } else {
        0.0
    }
}

/// The song's tempo-synced ECHO: one shared stereo delay line the lanes send
/// into ([`Voice::echo`]), its repeats fed back through a darkening
/// lowpass. Dotted-eighth repeats (`steps: 3.0` at four steps a beat) are
/// the synthwave lead / arp echo; a beat (`4.0`) the dub throw.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Echo {
    /// Delay time in STEPS (fractional allowed; clamped to 2 s).
    pub steps: f64,
    /// Repeat feedback, `0.0` (one repeat) … `0.9` (long trails).
    pub feedback: f64,
    /// Lowpass on the repeats, Hz (each repeat darker than the last).
    pub tone: f64,
}

impl Echo {
    /// Dotted-eighth repeats, three of them or so, slightly dark. Inert
    /// until a voice sends into it.
    pub const DOTTED: Echo = Echo {
        steps: 3.0,
        feedback: 0.35,
        tone: 3200.0,
    };

    /// A custom echo.
    pub const fn new(steps: f64, feedback: f64, tone: f64) -> Self {
        Self {
            steps,
            feedback,
            tone,
        }
    }
}

/// One-line summary of a voice for the tracker (`"SAW ×5 ±12C · BLOOM ·
/// HALL 45%"`): shape and stack, then which features are on.
pub fn voice_summary(v: &Voice) -> String {
    let wave = match v.wave {
        Wave::Sine => "SIN",
        Wave::Triangle => "TRI",
        Wave::Square => "SQR",
        Wave::Sawtooth => "SAW",
        Wave::Supersaw => "SUPERSAW",
        Wave::DrivenBass => "DRIVEN BASS",
        Wave::DarkPad => "DARK PAD",
        Wave::Noise => "NOISE",
        Wave::Guitar => "GUITAR",
        Wave::BassGuitar => "BASS GTR",
        Wave::Violin => "VIOLIN",
        Wave::Reese => "REESE",
        Wave::Fm => "FM",
    };
    let mut parts = Vec::new();
    let n = v.oscillators();
    if n > 1 {
        parts.push(format!("{wave} ×{n} ±{:.0}C", v.detune));
    } else {
        parts.push(wave.to_string());
    }
    if v.sub > 0.0 {
        parts.push(format!("SUB {:.0}%", v.sub * 100.0));
    }
    if v.glides() {
        parts.push(format!("GLIDE {:.0}MS", v.glide * 1000.0));
    }
    if let Some(f) = v.filter.filter(|_| !v.wave.is_preset()) {
        if f.attack > 0.0 {
            parts.push("BLOOM".to_string());
        }
        if f.decay > 0.0 || f.attack == 0.0 {
            parts.push(format!("WOW Q{:.0}", f.q));
        }
    }
    if let Some(vb) = v.vibrato.filter(|_| !v.wave.is_preset()) {
        parts.push(format!("VIB {:.0}C", vb.depth));
    }
    if let Some(b) = v.bend.filter(|_| !v.wave.is_preset()) {
        parts.push(format!("BEND {:+.0}ST", b.semitones));
    }
    if let Some(w) = v.wobble.filter(|_| !v.wave.is_preset()) {
        parts.push(format!("WOB 1/{:.0}", w.steps.max(0.1)));
    }
    if let Some(e) = v.env {
        parts.push(format!("GATE {:.1}", e.gate));
    }
    if v.drive > 0.0 {
        parts.push(format!("DRV {:.0}%", v.drive * 100.0));
    }
    if v.echo > 0.0 {
        parts.push(format!("ECHO {:.0}%", v.echo * 100.0));
    }
    if v.reverb > 0.0 {
        parts.push(format!("HALL {:.0}%", v.reverb * 100.0));
    }
    parts.join(" · ")
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The ducker drops to `1 - depth` at the kick, recovers monotonically
    /// and is ~95 % back after three time constants; no ducker = unity.
    #[test]
    fn duck_curve_dips_and_recovers() {
        assert_eq!(duck_level(0.6, 0.1, 0.0), 0.4);
        assert_eq!(duck_level(0.6, 0.1, -1.0), 0.4, "before the kick = at it");
        assert_eq!(duck_level(0.6, 0.0, 0.05), 1.0, "no time constant = off");
        assert_eq!(duck_level(0.0, 0.1, 0.0), 1.0);
        let mut last = 0.0;
        for i in 0..=100 {
            let g = duck_level(0.6, 0.1, 0.3 * f64::from(i) / 100.0);
            assert!(g >= last && (0.4..=1.0).contains(&g));
            last = g;
        }
        assert!(duck_level(0.6, 0.1, 0.3) > 1.0 - 0.6 * 0.05 - 1e-9);
        assert_eq!(duck_level(0.6, 0.1, f64::NEG_INFINITY.abs()), 1.0);
        assert!(!Sidechain::OFF.active() && Sidechain::new(0.5, 1.0).active());
    }

    /// Swing delays the odd sixteenths only, by at most a third of a step.
    #[test]
    fn swing_delays_the_off_sixteenths() {
        assert_eq!(swing_delay(1.0, 0, 0.12), 0.0);
        assert!((swing_delay(1.0, 1, 0.12) - 0.04).abs() < 1e-12);
        assert!((swing_delay(0.5, 3, 0.12) - 0.02).abs() < 1e-12);
        assert_eq!(swing_delay(0.0, 1, 0.12), 0.0);
        assert!((swing_delay(9.0, 1, 0.12) - 0.04).abs() < 1e-12, "clamped");
    }

    /// Voicings are in-key degree offsets, lowest first; the pad defaults
    /// to a triad, every other lane to the single note.
    #[test]
    fn chords_spell_degree_offsets() {
        assert_eq!(Chord::Single.degrees(), &[0]);
        assert_eq!(Chord::Triad.degrees(), &[0, 2, 4]);
        assert_eq!(Chord::Seventh.degrees().len(), 4);
        assert_eq!(Chord::default_for(PAD), Chord::Triad);
        assert_eq!(Chord::default_for(0), Chord::Single);
    }

    /// A stack only exists on a raw shape with a detune to spread it over;
    /// it is WIDE (a stereo bake) only with a width; presets and noise
    /// neither stack nor glide.
    #[test]
    fn voices_know_their_stack() {
        assert_eq!(Voice::mono(Wave::Sawtooth).oscillators(), 1);
        assert_eq!(Voice::wide(Wave::Sawtooth, 0.0, 7.0, 0.7).oscillators(), 2);
        assert!(Voice::wide(Wave::Sawtooth, 0.0, 7.0, 0.7).is_wide());
        assert!(!Voice::wide(Wave::Sawtooth, 0.0, 7.0, 0.0).is_wide());
        assert_eq!(
            Voice::stack(Wave::Sawtooth, 0.0, 12.0, 1.0, 99).oscillators(),
            7
        );
        assert_eq!(
            Voice::stack(Wave::Sawtooth, 0.0, 0.0, 1.0, 5).oscillators(),
            1
        );
        assert_eq!(Voice::stack(Wave::Noise, 0.0, 9.0, 1.0, 5).oscillators(), 1);
        let preset = Voice::stack(Wave::Supersaw, 0.0, 9.0, 1.0, 5).with_glide(0.1);
        assert_eq!(preset.oscillators(), 1);
        assert!(!preset.is_wide() && !preset.glides());
        assert!(Voice::mono(Wave::Square).with_glide(0.05).glides());
    }

    #[test]
    fn voice_summaries_name_what_is_on() {
        assert_eq!(voice_summary(&Voice::mono(Wave::Triangle)), "TRI");
        let v = Voice::stack(Wave::Sawtooth, 0.0, 12.0, 0.8, 5)
            .with_filter(400.0, 4000.0, 0.3, 0.0, 1.0)
            .with_reverb(0.45);
        assert_eq!(voice_summary(&v), "SAW ×5 ±12C · BLOOM · HALL 45%");
        assert_eq!(
            voice_summary(&Voice::mono(Wave::DrivenBass).with_sub(0.35)),
            "DRIVEN BASS · SUB 35%"
        );
        let w = Voice::mono(Wave::Reese)
            .with_wobble(2.0, 120.0, 2400.0, 6.0)
            .with_bend(-12.0, 0.1);
        assert_eq!(voice_summary(&w), "REESE · BEND -12ST · WOB 1/2");
        assert!(w.wave.is_computed() && Wave::Fm.is_computed());
        // A ramp names its lane, parameter and ends.
        let r = Ramp::cutoff(1, 200.0, 8000.0);
        assert_eq!(
            (r.lane, r.param, r.from, r.to),
            (1, RampParam::Cutoff, 200.0, 8000.0)
        );
        assert_eq!(Ramp::level(0, 1.0, 0.0).param, RampParam::Level);
        assert_eq!((r.span, r.over(3).span, r.over(0).span), (1, 3, 1));
        // A computed voice stacks and goes wide like a raw one.
        let v = Voice::wide(Wave::Violin, 0.0, 8.0, 0.6);
        assert_eq!(v.oscillators(), 2);
        assert!(v.is_wide());
    }
}
