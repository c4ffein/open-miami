//! Procedural audio engine for Open Miami // Rogue Purge.
//!
//! Everything here is synthesized at runtime with the Web Audio API (via
//! `web-sys`): oscillators for tones, a white-noise buffer for hits and
//! whooshes, all shaped by gain envelopes for a punchy, glitchy synthwave feel.
//! No audio files, no extra dependencies.
//!
//! The music runs through a dedicated bus — every note flows into a shared
//! lowpass [`web_sys::BiquadFilterNode`] whose cutoff is swept once per bar for
//! that classic synthwave/darksynth filter motion, then a safety soft-clip.
//! Each MELODIC lane has its own persistent channel in front of it (see
//! [`MusicFx`]): a `StereoPannerNode` (the voice's pan), a `WaveShaperNode`
//! (its drive), sends into one tempo-synced echo and one hall reverb, and
//! the side-chain ducker the kicks pump; the drums enter the bus directly.
//!
//! One-shot SFX go through their own bus, built to make synthesized weapon
//! audio read as *recorded* weapon audio: every sound is a per-event voice
//! (optionally soft-clipped by a [`web_sys::WaveShaperNode`]) that feeds a
//! dry path and a send into a shared [`web_sys::ConvolverNode`] loaded with
//! a synthesized stereo room impulse response. Guns and hits follow measured
//! profiles of field recordings (an uncompressed bright crack, a plateau
//! body, the room; hits are a spectral-modeling resynthesis of a reference
//! recording); melee, misc. and hurt sounds are layered click / crack / body
//! designs through a [`web_sys::DynamicsCompressorNode`]. Everything gets
//! per-play pitch / timing jitter (see the SFX section of [`AudioEngine`]).
//!
//! Because building those per-event node graphs live can stall the main
//! thread (measured 30–100 ms on macOS Chrome), each one-shot kind is
//! pre-rendered at startup: the same voice builders run into an
//! [`web_sys::OfflineAudioContext`] and the resulting dry buffers replace
//! live graph construction with a single `AudioBufferSourceNode` per play —
//! see the "pre-rendered voices" section of [`AudioEngine`]. Until (or
//! unless) a kind's buffers are ready, its `play_*` uses the live path.
//!
//! The MUSIC notes get the same treatment (the tracker's oscillator+gain
//! construction per note was the last measured stall source, 70–113 ms):
//! a song's note set is finite pattern data, so every distinct voice ×
//! pitch × tied length × voicing it can schedule is baked at its exact
//! frequency into a short buffer (mono; stereo for a WIDE unison voice) by
//! the note builders (see [`music_keys`] / `BakedMusic`), and
//! `schedule_step` then fires one buffer source per note into the lane's
//! live channel — pan, drive, sends, duck and the per-bar lowpass sweep all
//! stay live. The bake queue is prioritized: combat SFX first, then the
//! current song's voices, then the rare SFX; a song switch re-enumerates
//! and bakes in the background while unbaked notes fall back to a LIGHT
//! live sketch (one plain oscillator per partial — bounded cost, see
//! `AudioEngine::sketch`). The COMPUTED voices (`Wave::is_computed`: the
//! plucked / bowed strings of `audio/dsp.rs`) skip the offline context:
//! their samples are rendered in Rust and copied into the buffer at once
//! (`AudioEngine::computed_bake`).
//!
//! Robustness first: if the `AudioContext` (or any node) fails to build we
//! silently degrade to silence. Nothing in here ever panics or unwraps a
//! fallible Web Audio call — every `Result` is swallowed so the game runs fine
//! even when audio is unavailable or blocked by the browser.

use std::cell::{Cell, RefCell};
use std::rc::Rc;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::closure::Closure;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::{JsCast, JsValue};
use webaudio::{
    AudioBuffer, AudioContext, AudioDestinationNode, BaseAudioContext, BiquadFilterNode,
    BiquadFilterType, DelayNode, GainNode, OfflineAudioContext, OscillatorType, OverSampleType,
    StereoPannerNode, WaveShaperNode,
};
#[cfg(not(target_arch = "wasm32"))]
use webaudio::{Closure, JsValue};

use super::sfx::*;
use super::songs::Drum::{Clap, Crash, Hat, Kick, OpenHat, Rim, Silent, Snare, Tom};
use super::songs::*;

/// Look-ahead window (seconds) for the music scheduler: we queue notes this far
/// in advance of the audio clock so playback never gaps between frames.
const LOOKAHEAD: f64 = 0.15;

/// Master music level — kept low so the looping backing never buries the SFX.
const MUSIC_GAIN: f64 = 0.07;

/// Output trim after the SFX compressor / soft-clip.
const SFX_GAIN: f64 = 0.9;

/// Level of the reverb return into the SFX compressor.
const REVERB_RETURN: f64 = 0.7;

/// Length of the synthesized room impulse response for the melee / misc.
/// bus (seconds).
const IR_SECONDS: f64 = 1.1;

/// Length of the gun / hit bus impulse response (seconds): RT ~1.5 s.
const IR_REAL_SECONDS: f64 = 1.7;

/// Length of the MUSIC hall's impulse response (seconds): RT60 ≈ 2.4 s.
const IR_HALL_SECONDS: f64 = 2.6;

/// The music echo line's maximum delay (seconds): what `Echo::steps` is
/// clamped to at any tempo.
const ECHO_MAX_SECONDS: f64 = 2.0;

/// Overall gain of a resynthesised (SMS) metal hit: the model's loudest
/// track (a 1.0 sine partial) lands at this peak; the sub noise band, whose
/// RMS is normalised to its curve, peaks ~3× higher and is what the voice's
/// gentle soft-clip rounds off.
const SMS_HIT_GAIN: f32 = 0.45;

/// Target RMS, per unit of curve value, of an SMS noise band (1.0 = the
/// band's RMS equals its table amplitude — the same scale as the sine
/// partials' peak amplitude).
const SMS_NOISE_TRIM: f64 = 1.0;

/// Empirical per-band trims applied after the analytic RMS normalisation
/// of the SMS noise bands (measured against the METAL02 reference: the
/// 2nd-order bandpass skirts of the low bands leak more than the fc/Q
/// estimate assumes, the high bands land quiet). `(upper edge Hz, factor)`
/// — a band with centre `fc` uses the first row whose edge exceeds `fc`.
const SMS_BAND_TRIMS: &[(f64, f64)] = &[
    (100.0, 2.0),
    (200.0, 0.5),
    (400.0, 0.6),
    (1600.0, 0.7),
    (6000.0, 1.6),
    (12000.0, 1.3),
    (f64::MAX, 1.3),
];

/// Extra gain on SMS sine partials above this frequency (the 6–11 kHz ring
/// cluster measured ~2 dB quiet).
const SMS_HIGH_PARTIAL_HZ: f64 = 5000.0;
const SMS_HIGH_PARTIAL_GAIN: f32 = 1.25;

/// Level of the gun / hit bus reverb return: with a wet send of 1.0 the room
/// tail lands ≈ −21 dB @200 ms … −50 dB @1500 ms under the crack peak.
const REVERB_REAL_RETURN: f64 = 2.5;

/// Length of the shared white-noise buffer (seconds); bursts read it from a
/// random offset so no two share a waveform.
const NOISE_SECONDS: f64 = 2.0;

/// SFX are scheduled this far ahead of the audio clock so their sub-ms
/// transients are never dropped for being "in the past".
const SFX_LEAD: f64 = 0.012;

mod bake;
mod bus;
mod music;
mod sfx_play;
/// Analysis tables for spectral-modeling resynthesis (see
/// [`AudioEngine::sms_play`]). Generated offline; kept verbatim.
#[allow(clippy::excessive_precision, clippy::approx_constant)]
#[rustfmt::skip]
mod sms_tables;
#[cfg(test)]
mod tests;
mod voices;
mod webaudio;

/// A JS promise callback kept alive in [`BakedSfx::pending`].
type RenderCallback = Closure<dyn FnMut(JsValue)>;

/// The pre-rendered one-shot voices, shared with the async offline-render
/// completion callbacks via `Rc`. A kind switches to its baked buffers —
/// permanently — once all [`SFX_VARIANTS`] of them landed; until then its
/// `play_*` keeps the live per-node synthesis (the fallback), so nothing
/// changes while rendering is still in flight (or unavailable).
struct BakedSfx {
    /// `bufs[kind as usize]` = the finished variants for that kind (mono,
    /// dry, at the live context's sample rate).
    bufs: RefCell<Vec<Vec<AudioBuffer>>>,
    /// Next variant index to kick, `kind * SFX_VARIANTS + variant`.
    next: Cell<usize>,
    /// The in-flight renders' completion closures (a `done` / `fail` pair
    /// per kicked render), kept alive until they have fired: up to
    /// `pump_budget` renders run concurrently, so the pile grows while any
    /// is in flight and `update` clears it once nothing is.
    pending: RefCell<Vec<RenderCallback>>,
}

/// One music voice's bake slot: `None` until its offline render lands, then
/// the finished buffer (song gain and envelope baked in; mono, or stereo
/// for a wide unison voice). Velocity is applied at play time, so a
/// full-velocity note still needs no gain node at all.
struct MusicSlot {
    key: MusicKey,
    buf: Option<AudioBuffer>,
}

/// The pre-rendered note voices of the CURRENT song (see the
/// "pre-rendered voices" section of [`AudioEngine`]): `schedule_step`
/// swaps a note's per-node oscillator synthesis for one
/// `AudioBufferSourceNode` into the same music bus the moment its slot is
/// baked; unbaked slots fall back to the live path per note. Shared with
/// the async completion callbacks via `Rc`.
struct BakedMusic {
    /// The finite voice set of the current song, in bake-priority order
    /// (drums first — the densest lanes — then the melodic lanes per
    /// [`MELODIC`]).
    slots: RefCell<Vec<MusicSlot>>,
    /// Next slot index to kick.
    next: Cell<usize>,
    /// Bumped by every [`AudioEngine::rebuild_music_bake`] (song switch): a
    /// completion callback for a previous song's render sees the mismatch
    /// and drops its buffer instead of landing it in the wrong slot.
    gen: Cell<u32>,
    /// The in-flight renders' completion closures (same lifecycle as
    /// [`BakedSfx::pending`]).
    pending: RefCell<Vec<RenderCallback>>,
}

/// The music send effects: one tempo-synced echo line and one hall reverb,
/// each fed by a per-lane send gain taken after the lane's drive (so a
/// driven lead echoes driven) and returning into the ducker (echoes and
/// tails pump with everything else). Built once; the echo's time /
/// feedback / tone and every send level follow the song
/// ([`AudioEngine::apply_voices`]).
///
/// ```text
///  note ─► lane panner ─► drive ─┬────────────────────────────► ducker ─► bus ─► lowpass ─► soft-clip ─► out
///                                ├─ echo send ─► delay ─► tone ─► return ──┤
///                                │                 ▲           └─ feedback ─┘
///                                └─ verb send ─► convolver (hall) ─► return ─┘
///  drum ──────────────────────────────────────────────────────────────────► bus
/// ```
struct MusicFx {
    /// Per-lane send gains into the echo.
    echo_send: Vec<GainNode>,
    /// The delay line and its feedback loop.
    delay: DelayNode,
    feedback: GainNode,
    tone: BiquadFilterNode,
    /// Per-lane send gains into the hall.
    verb_send: Vec<GainNode>,
}

/// One enveloped oscillator note for [`AudioEngine::tone_env`].
struct Tone {
    /// Start and end pitch (a glide when they differ).
    f0: f64,
    f1: f64,
    /// Seconds the `f0 → f1` glide takes; `0` = it spans the whole note.
    glide: f64,
    /// Absolute start time.
    start: f64,
    /// Seconds to peak.
    attack: f64,
    /// Seconds held at peak after the attack (the tied steps).
    hold: f64,
    /// Seconds of exponential decay after the hold.
    dur: f64,
    /// Peak amplitude.
    peak: f64,
    wave: OscillatorType,
    vibrato: Option<Vibrato>,
}

/// One melodic-lane note as handed to [`AudioEngine::lane_tone`]: the
/// partial's pitch (and the pitch it glides in from), its envelope times
/// and its level.
#[derive(Clone, Copy)]
struct LaneNote {
    f: f64,
    from: Option<f64>,
    start: f64,
    attack: f64,
    hold: f64,
    dur: f64,
    peak: f64,
}

/// The offline render target while a pre-render is being *built*: the voice
/// builders create their nodes in `ctx` and the voice front-end feeds `sink`
/// (the offline destination) instead of the live bus.
struct OfflineRender {
    ctx: BaseAudioContext,
    sink: webaudio::AudioNode,
}

/// The persistent SFX bus. Every one-shot flows in through a per-shot *voice*
/// (see [`AudioEngine::voice`]) that splits into a dry path and a wet send.
/// There are two parallel paths: the melee / misc. one drawn below
/// (compressor plus limiter, darkening IR) and the gun / hit one (dry_real
/// and reverb_real_in: uncompressed but for a −1 dB / 4:1 safety, a longer
/// brighter IR), both summing into the bus soft-clip:
///
/// ```text
///  voice ──(soft-clip)──┬──────────────► dry ─────────────┐
///                       └─ send(wet) ──► reverb_in ─► HPF ─► convolver ─► return ─┤
///                                                                                  ▼
///                                        compressor ─► limiter ─► bus soft-clip ─► trim ─► out
/// ```
///
/// The convolver holds a synthesized stereo impulse response (see
/// [`AudioEngine::make_impulse`]) so every shot and clang gets the room tail a
/// real recording has; the compressor glues the layers of a shot into one
/// punchy transient and the gentle bus clipper adds the "hot mic" edge.
struct SfxBus {
    /// Dry input — sums straight into the compressor.
    dry: GainNode,
    /// Reverb input — feeds the convolver (through a low-cut).
    reverb_in: GainNode,
    /// Gun / hit dry input: bypasses the compressor and the limiter (only a
    /// gentle −1 dB / 4:1 safety) so a crack keeps its 18–22 dB crest.
    dry_real: GainNode,
    /// Gun / hit reverb input: a longer, brighter impulse response.
    reverb_real_in: GainNode,
    /// A pre-wired default voice (dry + a light send) for the misc. SFX that
    /// don't build their own voice (pickup, throw, elevator, ...).
    room: GainNode,
}

/// The self-contained audio engine. Construct once, hand it around, drive
/// `update()` from the game loop.
pub struct AudioEngine {
    /// `None` if the browser refused to give us an audio context.
    ctx: Option<AudioContext>,
    /// Master enable (the SETTINGS sound toggle): off = the whole
    /// `AudioContext` is suspended — music and SFX alike — and the autoplay
    /// unlock refuses to resume it. `Cell` because `play_*` take `&self`.
    enabled: Cell<bool>,
    /// Pre-rendered white noise, reused (via cheap buffer-source nodes) for
    /// every percussive/whoosh sound.
    noise: Option<AudioBuffer>,
    /// The SFX bus (reverb + compressor + soft clip), if it could be built.
    sfx: Option<SfxBus>,
    /// Tiny xorshift state for per-shot randomization (pitch / timing jitter,
    /// ricochet chance). `Cell` because the `play_*` API takes `&self`.
    rng: Cell<u32>,
    /// Input gain for the whole music mix — every music note connects here.
    /// Its gain IS the SETTINGS music level.
    music_bus: Option<GainNode>,
    /// The SETTINGS music level, `0.0..=1.0` (the music bus's gain; the SFX
    /// are untouched). `Cell` because the settings UI holds `&self`.
    music_level: Cell<f64>,
    /// The SIDECHAIN DUCK stage: the melodic lanes (and the echo / hall
    /// returns) enter the bus through this gain, drums bypass it. In a
    /// `Section` with `duck` set, every kick pulls it down by the song's
    /// `Sidechain::depth` and releases it exponentially (the pure curve of
    /// `voice::duck_level`, see [`Self::duck`]).
    music_duck: Option<GainNode>,
    /// When the ducker last bottomed out (audio clock) — where a
    /// retriggering kick picks the release curve up from.
    last_duck: Cell<f64>,
    /// One `StereoPannerNode` per melodic lane (indexed like
    /// `SongSpec::voices`): what the lane's notes connect to. Empty if they
    /// could not be built (notes then enter the ducker / bus directly).
    music_pan: Vec<StereoPannerNode>,
    /// One drive `WaveShaperNode` per melodic lane, after its panner (`None`
    /// curve = bypass). Empty if they could not be built.
    music_drive: Vec<WaveShaperNode>,
    /// The echo line + the hall and their per-lane sends ([`MusicFx`]).
    music_fx: Option<MusicFx>,
    /// Lowpass filter on the music bus, cutoff swept once per bar (synthwave).
    music_filter: Option<BiquadFilterNode>,
    music_playing: bool,
    /// Absolute audio-clock time of the next music step to schedule.
    next_note_time: f64,
    /// Where the scheduler is inside the song's arrangement (section + step).
    playhead: Playhead,
    /// The song currently driving the scheduler.
    song: SongSpec,
    /// Per-channel mute flags (indexed like `CHANNEL_NAMES`).
    mute: [bool; NUM_CHANNELS],
    /// Per-channel solo flags. If any is set, only soloed channels sound.
    solo: [bool; NUM_CHANNELS],
    /// Pre-rendered one-shot voices (see [`BakedSfx`]); `Rc` so the async
    /// offline-render completion callbacks can write finished buffers in.
    baked: Rc<BakedSfx>,
    /// Pre-rendered music note voices of the current song ([`BakedMusic`]).
    baked_music: Rc<BakedMusic>,
    /// How many offline renders — SFX or music — are in flight right now,
    /// capped at `pump_budget` (so graph construction never bursts onto a
    /// single frame). Shared with the completion callbacks via `Rc`.
    renders_in_flight: Rc<Cell<u32>>,
    /// How many offline renders may run concurrently (the game loop sets it
    /// per screen: gentle in-game, aggressive on loading/menu screens where
    /// a construction hitch cannot be seen).
    pump_budget: Cell<u32>,
    /// Set when `OfflineAudioContext` turns out to be unavailable: the whole
    /// bake queue (SFX and music) is abandoned, everything stays live.
    render_dead: Cell<bool>,
    /// `Some` only while an offline pre-render is being built: the voice
    /// builders then target this context/sink instead of the live bus.
    render: RefCell<Option<OfflineRender>>,
    /// The running title-screen engine-idle loop (source + its gain),
    /// `None` while stopped. See [`Self::start_engine_idle`].
    engine_idle: RefCell<Option<(webaudio::AudioBufferSourceNode, GainNode)>>,
}

impl AudioEngine {
    /// Try to create the audio context. Never fails hard — on any error the
    /// engine simply stays silent.
    pub fn new() -> Self {
        let ctx = AudioContext::new().ok();
        let noise = ctx.as_ref().and_then(Self::make_noise);
        let sfx = ctx.as_ref().and_then(Self::make_sfx_bus);
        let (music_bus, music_filter) = ctx
            .as_ref()
            .map(Self::make_music_bus)
            .unwrap_or((None, None));
        let music_duck = ctx
            .as_ref()
            .zip(music_bus.as_ref())
            .and_then(|(c, bus)| Self::make_duck(c, bus));
        // The lane channels feed the ducker (or the plain bus without one).
        let lanes = ctx.as_ref().zip(music_duck.as_ref().or(music_bus.as_ref()));
        let music_drive = lanes
            .map(|(c, into)| Self::make_lane_drives(c, into))
            .unwrap_or_default();
        let music_pan = lanes
            .map(|(c, into)| Self::make_lane_panners(c, into, &music_drive))
            .unwrap_or_default();
        let music_fx = lanes
            .filter(|_| !music_pan.is_empty())
            .and_then(|(c, into)| Self::make_music_fx(c, into, &music_pan, &music_drive));
        let engine = Self {
            ctx,
            noise,
            sfx,
            enabled: Cell::new(true),
            rng: Cell::new(0x2545_F491),
            music_bus,
            music_level: Cell::new(1.0),
            music_duck,
            last_duck: Cell::new(f64::NEG_INFINITY),
            music_pan,
            music_drive,
            music_fx,
            music_filter,
            music_playing: false,
            next_note_time: 0.0,
            playhead: Playhead::START,
            song: title_song(),
            mute: [false; NUM_CHANNELS],
            solo: [false; NUM_CHANNELS],
            baked: Rc::new(BakedSfx {
                bufs: RefCell::new(vec![Vec::new(); SFX_KINDS.len()]),
                next: Cell::new(0),
                pending: RefCell::new(Vec::new()),
            }),
            baked_music: Rc::new(BakedMusic {
                slots: RefCell::new(Vec::new()),
                next: Cell::new(0),
                gen: Cell::new(0),
                pending: RefCell::new(Vec::new()),
            }),
            renders_in_flight: Rc::new(Cell::new(0)),
            pump_budget: Cell::new(1),
            render_dead: Cell::new(false),
            render: RefCell::new(None),
            engine_idle: RefCell::new(None),
        };
        engine.apply_voices();
        engine.rebuild_music_bake();
        engine
    }

    /// Resume the context. Browsers start it suspended until a user gesture,
    /// so the integrator should call this on the first input (e.g. pressing
    /// Enter to start the game).
    pub fn resume(&self) {
        if !self.enabled.get() {
            return;
        }
        if let Some(ctx) = &self.ctx {
            let _ = ctx.resume();
        }
    }

    /// The SETTINGS sound toggle: `false` suspends the whole `AudioContext`
    /// (music and SFX go silent instantly, nothing else changes — schedulers
    /// idle against the frozen audio clock), `true` resumes it.
    pub fn set_enabled(&self, on: bool) {
        self.enabled.set(on);
        if let Some(ctx) = &self.ctx {
            if on {
                let _ = ctx.resume();
            } else {
                let _ = ctx.suspend();
            }
        }
    }

    /// Whether sound is currently enabled (the SETTINGS checkbox state).
    pub fn is_enabled(&self) -> bool {
        self.enabled.get()
    }

    /// The SETTINGS music level, `0.0` (music off, SFX untouched) … `1.0`
    /// (nominal): the gain of the music bus, applied at once.
    pub fn set_music_level(&self, level: f64) {
        let level = if level.is_finite() {
            level.clamp(0.0, 1.0)
        } else {
            1.0
        };
        self.music_level.set(level);
        if let Some(bus) = &self.music_bus {
            let _ = bus.gain().set_value_at_time(level as f32, 0.0);
        }
    }

    /// The current SETTINGS music level.
    pub fn music_level(&self) -> f64 {
        self.music_level.get()
    }

    /// How many offline pre-renders may run concurrently (see `update`).
    pub fn set_pump_budget(&self, n: u32) {
        self.pump_budget.set(n.max(1));
    }

    /// Pre-render progress `(done, total)` across the SFX variants and the
    /// current song's note voices — the loading screen's PRECOMPUTING bar.
    pub fn bake_progress(&self) -> (u32, u32) {
        let sfx_done: usize = self.baked.bufs.borrow().iter().map(Vec::len).sum();
        let sfx_total = SFX_KINDS.len() * SFX_VARIANTS;
        let slots = self.baked_music.slots.borrow();
        let music_done = slots.iter().filter(|s| s.buf.is_some()).count();
        let music_total = slots.len();
        (
            (sfx_done + music_done) as u32,
            (sfx_total + music_total) as u32,
        )
    }

    /// Whether the pre-render queue is finished (or can never finish — no
    /// `OfflineAudioContext`, or renders died: the live path serves forever
    /// and the loading screen must not wait).
    pub fn bake_complete(&self) -> bool {
        if self.ctx.is_none() || self.render_dead.get() {
            return true;
        }
        let (done, total) = self.bake_progress();
        done >= total
    }

    // --- helpers -----------------------------------------------------------

    /// Current audio-clock time, or 0.0 if we have no context. During an
    /// offline pre-render this is the offline clock (0.0 — rendering has not
    /// started), so [`Self::t0`] lands the voice [`SFX_LEAD`] into the buffer,
    /// exactly the lead it gets live.
    fn now(&self) -> f64 {
        self.bctx().map(|c| c.current_time()).unwrap_or(0.0)
    }

    /// The context nodes are currently built in: the [`OfflineAudioContext`]
    /// while a pre-render is being assembled, the live [`AudioContext`]
    /// otherwise. Node-creation methods live on [`BaseAudioContext`], which
    /// both deref to — this is the whole trick that lets every voice builder
    /// target either context unchanged.
    fn bctx(&self) -> Option<BaseAudioContext> {
        if let Some(r) = self.render.borrow().as_ref() {
            return Some(r.ctx.clone());
        }
        self.ctx
            .as_ref()
            .map(|c| AsRef::<BaseAudioContext>::as_ref(c).clone())
    }

    fn destination(&self) -> Option<AudioDestinationNode> {
        self.ctx.as_ref().map(|c| c.destination())
    }

    /// The node music voices connect to: the filtered music bus if we built it,
    /// otherwise the raw destination (graceful fallback). During an offline
    /// pre-render: the offline destination, so a note bakes its dry signal
    /// only — the bus (and its per-bar filter sweep) stays live and is
    /// reapplied at play time by [`Self::play_music_baked`].
    fn music_out(&self) -> Option<webaudio::AudioNode> {
        if let Some(r) = self.render.borrow().as_ref() {
            return Some(r.sink.clone());
        }
        if let Some(bus) = &self.music_bus {
            Some(AsRef::<webaudio::AudioNode>::as_ref(bus).clone())
        } else {
            self.destination()
                .map(|d| AsRef::<webaudio::AudioNode>::as_ref(&d).clone())
        }
    }

    /// The node melodic lane `lane`'s notes connect to: its panner (then
    /// its drive, its sends, the ducker, the bus) — falling back to the
    /// ducker, then the plain bus, where those could not be built. During
    /// an offline pre-render: the offline destination — pan, drive, sends,
    /// the duck and the bar filter sweep are live-channel processing,
    /// reapplied at play time, so baked buffers stay dry.
    fn lane_out(&self, lane: usize) -> Option<webaudio::AudioNode> {
        if self.render.borrow().is_some() {
            return self.music_out();
        }
        if let Some(p) = self.music_pan.get(lane) {
            return Some(AsRef::<webaudio::AudioNode>::as_ref(p).clone());
        }
        if let Some(duck) = &self.music_duck {
            return Some(AsRef::<webaudio::AudioNode>::as_ref(duck).clone());
        }
        self.music_out()
    }

    /// Start time for a freshly-triggered SFX: a hair after "now" so that the
    /// sub-millisecond transients are never scheduled in the past (and thus
    /// silently skipped) — 12 ms is well under any perceptible latency.
    fn t0(&self) -> f64 {
        self.now() + SFX_LEAD
    }

    /// Next pseudo-random number in `[0, 1)` (xorshift32, deterministic).
    fn rand(&self) -> f64 {
        let mut s = self.rng.get();
        s ^= s << 13;
        s ^= s >> 17;
        s ^= s << 5;
        self.rng.set(s);
        (s >> 8) as f64 / (1u32 << 24) as f64
    }

    /// A random multiplier in `[1 - amount, 1 + amount]` (pitch/timing jitter).
    fn jit(&self, amount: f64) -> f64 {
        1.0 + (self.rand() * 2.0 - 1.0) * amount
    }

    /// `true` with probability `p`.
    fn chance(&self, p: f64) -> bool {
        self.rand() < p
    }

    /// The generic SFX output: the bus's default "room" voice if we have one,
    /// otherwise the raw destination (graceful fallback). During an offline
    /// pre-render: the offline destination, so the room-voice kinds render
    /// their dry signal (their light room send is pre-wired into `room` and
    /// stays live).
    fn sfx_out(&self) -> Option<webaudio::AudioNode> {
        if let Some(r) = self.render.borrow().as_ref() {
            return Some(r.sink.clone());
        }
        if let Some(bus) = &self.sfx {
            Some(AsRef::<webaudio::AudioNode>::as_ref(&bus.room).clone())
        } else {
            self.destination()
                .map(|d| AsRef::<webaudio::AudioNode>::as_ref(&d).clone())
        }
    }
}

/// The `web_sys` oscillator shape of a song voice's [`Wave`]. The preset
/// waves and the noise are never dispatched here (`lane_tone` builds their
/// node graphs first), but map to their nearest raw shape as a total
/// fallback — which is also what the live SKETCH of a preset plays.
fn osc(wave: Wave) -> OscillatorType {
    match wave {
        Wave::Sine => OscillatorType::Sine,
        Wave::Square => OscillatorType::Square,
        Wave::Sawtooth
        | Wave::Supersaw
        | Wave::DrivenBass
        | Wave::DarkPad
        | Wave::Noise
        | Wave::Violin => OscillatorType::Sawtooth,
        Wave::Triangle | Wave::Guitar | Wave::BassGuitar => OscillatorType::Triangle,
    }
}

impl Default for AudioEngine {
    fn default() -> Self {
        Self::new()
    }
}
