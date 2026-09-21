//! SFX building blocks (ricochet, shot, pump, wham, click, …) and the voice /
//! tone / noise primitives every sound is made of.

use super::*;

impl AudioEngine {
    // --- SFX building blocks -----------------------------------------------
    //
    // Small, physically-motivated layers. Each takes the voice node to render
    // into and an absolute start time; the `play_*` methods stack them.

    /// A realistic ricochet: mostly NOISE through a moving high-Q bandpass
    /// whining down 3 kHz → 600 Hz over 250–400 ms, plus a faint pitched
    /// sweep under it. Quiet.
    pub(super) fn real_ricochet(&self, out: &webaudio::AudioNode, t: f64, j: f64, peak: f64) {
        let dur = 0.25 + self.rand() * 0.15;
        let f0 = 3000.0 * j * self.jit(0.08);
        let f1 = 600.0 * j * self.jit(0.1);
        self.noise_full(
            out,
            t,
            0.012,
            dur,
            dur * 0.9,
            peak * 3.0,
            BiquadFilterType::Bandpass,
            f0,
            f1,
            14.0,
        );
        self.tone_out(
            out,
            f0 * 1.01,
            f1 * 1.01,
            t + 0.005,
            dur * 0.9,
            peak * 0.18,
            0.012,
            OscillatorType::Sine,
        );
    }

    /// Render one gunshot from a [`RealShot`] recipe at `t`:
    /// crack (bright cluster + 3–8 kHz band + air, rising over `crack_rise`
    /// so the peak lands at +5–15 ms, uncompressed) → mid body plateau
    /// (bandpass ~1.2 kHz, Q 0.5, held ~100 ms at −5 dB, −21 dB by +200 ms)
    /// with hi / air companions → low-mid layer → faint thump. No sub, no
    /// growl, no AM; ±3 ms onset jitter per layer; the room comes from the
    /// bright gun-bus reverb via the voice's wet send.
    pub(super) fn real_shot(
        &self,
        out: &webaudio::AudioNode,
        t: f64,
        j: f64,
        level: f64,
        s: &RealShot,
    ) {
        // Crack.
        let bus = self.crack_bus(out);
        let n = 3 + (self.rand() * 3.0) as usize;
        for k in 0..n {
            let at = if k == 0 { t } else { t + self.rand() * 0.006 };
            self.click(&bus, at, s.crack * level * (0.6 + self.rand() * 0.6));
        }
        let rise = s.crack_rise * self.jit(0.3);
        self.noise_full(
            &bus,
            t,
            rise,
            rise + 0.014,
            rise + 0.014,
            1.8 * s.crack * level,
            BiquadFilterType::Bandpass,
            5500.0 * j,
            3200.0 * j,
            0.5,
        );
        self.noise_full(
            &bus,
            self.jt(t + 0.001, 0.001),
            rise,
            rise + 0.010,
            rise + 0.010,
            1.4 * s.crack * s.air * level,
            BiquadFilterType::Highpass,
            8000.0 * j,
            8000.0 * j,
            0.7,
        );
        self.noise_full(
            &bus,
            self.jt(t + 0.001, 0.001),
            rise,
            rise + 0.022,
            rise + 0.022,
            1.6 * s.crack * level,
            BiquadFilterType::Lowpass,
            12000.0 * j,
            1500.0 * j,
            0.7,
        );
        // Mid body plateau + hi / air companions.
        let b = self.jt(t + 0.002, 0.003);
        self.noise_plateau(
            out,
            b,
            s.body * level,
            s.plateau,
            s.drop,
            BiquadFilterType::Bandpass,
            s.body_hz * j,
            s.body_hz * 0.8 * j,
            0.5,
        );
        self.noise_plateau(
            out,
            self.jt(b, 0.003),
            s.body_hi * level,
            s.plateau * 0.9,
            s.drop,
            BiquadFilterType::Bandpass,
            4200.0 * j,
            3000.0 * j,
            0.6,
        );
        self.noise_plateau(
            out,
            self.jt(b, 0.003),
            s.body_air * level,
            s.plateau * 0.8,
            s.drop,
            BiquadFilterType::Highpass,
            8000.0,
            8000.0,
            0.7,
        );
        // Low-mid layer (130–300 Hz): bandpass noise (Q 0.8, centred on
        // `low_hz`, sagging to ~150 Hz) with the plateau shape, plus a pair
        // of decaying pitched partials (~150 and ~220 Hz, triangle, ~120 ms).
        // The gun-bus reverb low-cut sits at 120 Hz so this reaches the room.
        self.noise_plateau(
            out,
            self.jt(b + 0.002, 0.003),
            s.low * level,
            s.plateau * 1.1,
            s.drop * 1.2,
            BiquadFilterType::Bandpass,
            s.low_hz * 0.95 * j,
            s.low_hz * 0.7 * j,
            0.8,
        );
        for f in [150.0, 220.0] {
            let f = f * j * self.jit(0.02);
            self.swell_tone(
                out,
                f,
                f * 0.97,
                self.jt(b + 0.003, 0.003),
                0.004,
                0.04,
                s.low * 0.22 * level,
                OscillatorType::Triangle,
                0.015,
            );
        }
        // Faint thump (≤ −15 dB): a short, wobbled 60–90 Hz component.
        if s.thump > 0.0 {
            self.swell_tone(
                out,
                75.0 * j * self.jit(0.08),
                70.0 * j,
                self.jt(t + 0.004, 0.003),
                0.006,
                0.05,
                s.thump * level,
                OscillatorType::Sine,
                0.03,
            );
        }
    }

    /// A noise layer with the measured real-gunshot body envelope: 3 ms rise
    /// to `peak`, a plateau that only sags 5 dB over `plateau` seconds, a
    /// drop to −21 dB over the next `drop` seconds, then a slide to silence
    /// (−80 dB) over a further 3× `drop`; filter sweeps `f0 → f1` across
    /// the plateau + drop.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn noise_plateau(
        &self,
        out: &webaudio::AudioNode,
        start: f64,
        peak: f64,
        plateau: f64,
        drop: f64,
        filter: BiquadFilterType,
        f0: f64,
        f1: f64,
        q: f64,
    ) {
        let (ctx, buf) = match (self.bctx(), &self.noise) {
            (Some(c), Some(b)) => (c, b),
            _ => return,
        };
        let (src, filt, gain) = match (
            ctx.create_buffer_source(),
            ctx.create_biquad_filter(),
            ctx.create_gain(),
        ) {
            (Ok(s), Ok(f), Ok(g)) => (s, f, g),
            _ => return,
        };
        let peak = peak.max(0.0002) as f32;
        let t1 = start + 0.003;
        let t2 = t1 + plateau.max(0.005);
        let t3 = t2 + drop.max(0.005);
        let t4 = t3 + 3.0 * drop.max(0.005);
        src.set_buffer(Some(buf));
        src.set_loop(true);
        filt.set_type(filter);
        let ff = filt.frequency();
        let _ = ff.set_value_at_time(f0 as f32, start);
        if (f1 - f0).abs() > 1.0 {
            let _ = ff.exponential_ramp_to_value_at_time(f1.max(1.0) as f32, t3);
        }
        let _ = filt.q().set_value_at_time(q as f32, start);
        let g = gain.gain();
        let _ = g.set_value_at_time(0.0001, start);
        let _ = g.linear_ramp_to_value_at_time(peak, t1);
        let _ = g.exponential_ramp_to_value_at_time(peak * 0.56, t2); // −5 dB
        let _ = g.exponential_ramp_to_value_at_time(peak * 0.089, t3); // −21 dB
        let _ = g.exponential_ramp_to_value_at_time(0.0001, t4);
        let _ = src.connect_with_audio_node(&filt);
        let _ = filt.connect_with_audio_node(&gain);
        let _ = gain.connect_with_audio_node(out);
        let sched: &webaudio::AudioScheduledSourceNode = src.as_ref();
        let offset = self.rand() * (NOISE_SECONDS - 0.05);
        let _ = src.start_with_when_and_grain_offset(start, offset);
        let _ = sched.stop_with_when(t4 + 0.02);
    }

    /// A real pump reload starting at `t`: ~1 s of multiple bright 2–8 kHz
    /// clacks and slide scrapes — pump back (two clacks), forward (two),
    /// the shell / lifter — with essentially no low content.
    pub(super) fn real_pump(&self, out: &webaudio::AudioNode, t: f64, j: f64) {
        // (offset, scrape level, clack base Hz, clack level) — kept ~4 dB
        // under the shot's tail so the rack sits inside it, like a real one.
        let events: [(f64, f64, f64, f64); 6] = [
            (0.0, 0.22, 3300.0, 0.21),
            (0.045, 0.0, 2500.0, 0.16),
            (0.28, 0.20, 2900.0, 0.20),
            (0.32, 0.0, 2100.0, 0.15),
            (0.55, 0.09, 3800.0, 0.11),
            (0.72, 0.0, 2700.0, 0.09),
        ];
        for (off, scrape, base, clack) in events {
            let at = t + off + self.rand() * 0.02;
            if scrape > 0.0 {
                self.noise_full(
                    out,
                    at,
                    0.012,
                    0.11,
                    0.11,
                    scrape,
                    BiquadFilterType::Bandpass,
                    4200.0 * j,
                    3000.0 * j,
                    0.6,
                );
            }
            self.tick(out, at + 0.008, base * j * self.jit(0.05), clack);
            self.noise_full(
                out,
                at + 0.008,
                0.0,
                0.03,
                0.03,
                clack * 0.9,
                BiquadFilterType::Highpass,
                3000.0,
                3000.0,
                0.7,
            );
        }
        self.tinkle(out, t + 0.40 + self.rand() * 0.05, 0.03);
    }

    /// A gain node feeding `out` directly plus, through 2–3 short DelayNodes
    /// (6–25 ms, lowpassed at ~3 kHz, decaying), the early reflections of
    /// whatever is played into it. Falls back to `out` if nodes fail.
    pub(super) fn crack_bus(&self, out: &webaudio::AudioNode) -> webaudio::AudioNode {
        let ctx = match self.bctx() {
            Some(c) => c,
            None => return out.clone(),
        };
        let bus = match ctx.create_gain() {
            Ok(g) => g,
            Err(_) => return out.clone(),
        };
        let _ = bus.gain().set_value_at_time(1.0, 0.0);
        let _ = bus.connect_with_audio_node(out);
        let taps = [
            (0.006 + self.rand() * 0.004, 0.5),
            (0.013 + self.rand() * 0.006, 0.35),
            (0.020 + self.rand() * 0.006, 0.22),
        ];
        let n_taps = 2 + (self.rand() * 2.0) as usize;
        for (d, g) in taps.iter().take(n_taps) {
            if let (Ok(delay), Ok(lpf), Ok(gain)) = (
                ctx.create_delay_with_max_delay_time(0.05),
                ctx.create_biquad_filter(),
                ctx.create_gain(),
            ) {
                let _ = delay.delay_time().set_value_at_time(*d as f32, 0.0);
                lpf.set_type(BiquadFilterType::Lowpass);
                let _ = lpf.frequency().set_value_at_time(3000.0, 0.0);
                let _ = gain.gain().set_value_at_time(*g as f32, 0.0);
                let _ = bus.connect_with_audio_node(&delay);
                let _ = delay.connect_with_audio_node(&lpf);
                let _ = lpf.connect_with_audio_node(&gain);
                let _ = gain.connect_with_audio_node(out);
            }
        }
        AsRef::<webaudio::AudioNode>::as_ref(&bus).clone()
    }

    /// `t` jittered by ±`a` seconds (layer de-synchronisation).
    pub(super) fn jt(&self, t: f64, a: f64) -> f64 {
        t + (self.rand() * 2.0 - 1.0) * a
    }

    /// A noise-derived LFO added to `param` for `dur` seconds from `start`:
    /// the noise buffer through a bandpass centred `fc` (Q `q`), scaled so
    /// the modulation has an RMS of `amount` (in the param's units). Used for
    /// envelope roughness (40–90 Hz on a gain) and pitch random-walks (10–30
    /// Hz on a frequency).
    #[allow(clippy::too_many_arguments)]
    pub(super) fn noise_lfo(
        &self,
        param: &webaudio::AudioParam,
        fc: f64,
        q: f64,
        amount: f64,
        start: f64,
        dur: f64,
        clamp: Option<f64>,
    ) {
        let (ctx, buf) = match (self.bctx(), &self.noise) {
            (Some(c), Some(b)) => (c, b),
            _ => return,
        };
        let (src, filt, gain) = match (
            ctx.create_buffer_source(),
            ctx.create_biquad_filter(),
            ctx.create_gain(),
        ) {
            (Ok(s), Ok(f), Ok(g)) => (s, f, g),
            _ => return,
        };
        // Uniform ±1 noise has RMS 0.577; the bandpass keeps a fraction
        // fc/q of the 0..sr/2 band, so its output RMS is 0.577·sqrt(...).
        let sr = ctx.sample_rate() as f64;
        let rms = 0.577 * (fc / q / (sr * 0.5)).sqrt();
        src.set_buffer(Some(buf));
        src.set_loop(true);
        filt.set_type(BiquadFilterType::Bandpass);
        let _ = filt.frequency().set_value_at_time(fc as f32, start);
        let _ = filt.q().set_value_at_time(q as f32, start);
        let _ = gain
            .gain()
            .set_value_at_time((amount / rms.max(1e-4)) as f32, start);
        let _ = src.connect_with_audio_node(&filt);
        let _ = filt.connect_with_audio_node(&gain);
        // Optional hard clamp of the modulation to ±`clamp` (in the param's
        // units, |clamp| ≤ 1) through a WaveShaper, so an AM peak can never
        // push a gain past 1 + clamp.
        let mut tail: webaudio::AudioNode = AsRef::<webaudio::AudioNode>::as_ref(&gain).clone();
        if let Some(c) = clamp {
            if let Ok(shaper) = ctx.create_wave_shaper() {
                let c = c.clamp(0.01, 1.0) as f32;
                let n = 1024usize;
                let mut curve: Vec<f32> = (0..n)
                    .map(|i| (i as f32 / (n - 1) as f32 * 2.0 - 1.0).clamp(-c, c))
                    .collect();
                shaper.set_curve_opt_f32_slice(Some(curve.as_mut_slice()));
                if tail.connect_with_audio_node(&shaper).is_ok() {
                    tail = AsRef::<webaudio::AudioNode>::as_ref(&shaper).clone();
                }
            }
        }
        let _ = tail.connect_with_audio_param(param);
        let sched: &webaudio::AudioScheduledSourceNode = src.as_ref();
        let offset = self.rand() * (NOISE_SECONDS - 0.05);
        let _ = src.start_with_when_and_grain_offset(start, offset);
        let _ = sched.stop_with_when(start + dur + 0.02);
    }

    /// Roughness stage: returns a unity gain node into `out` whose gain is
    /// modulated by a 40–90 Hz (random centre, Q 0.7) noise LFO with an RMS
    /// of 0.45·`depth`, so a layer rendered into it comes out as
    /// `x(t)·(1 + m(t))` — its envelope growls / rattles at 20–150 Hz
    /// instead of decaying smoothly; the modulation is clamped to ±0.5 so
    /// the stage's gain never exceeds 1.5. `depth` ≤ 0 returns `out` itself.
    pub(super) fn roughen(
        &self,
        out: &webaudio::AudioNode,
        start: f64,
        dur: f64,
        depth: f64,
    ) -> webaudio::AudioNode {
        if depth <= 0.0 {
            return out.clone();
        }
        let ctx = match self.bctx() {
            Some(c) => c,
            None => return out.clone(),
        };
        let stage = match ctx.create_gain() {
            Ok(g) => g,
            Err(_) => return out.clone(),
        };
        let _ = stage.gain().set_value_at_time(1.0, 0.0);
        if stage.connect_with_audio_node(out).is_err() {
            return out.clone();
        }
        let fc = 40.0 + self.rand() * 50.0;
        self.noise_lfo(&stage.gain(), fc, 0.7, 0.45 * depth, start, dur, Some(0.5));
        AsRef::<webaudio::AudioNode>::as_ref(&stage).clone()
    }

    /// FM growl: a low sine (`f`, 55–70 Hz) whose frequency is modulated by
    /// a second oscillator at `rate` (30–60 Hz) with `depth_hz` (40–120 Hz)
    /// of swing — sidebands at ±rate make a snarling roar once saturated —
    /// swelling over `attack` and falling 20 dB over `d20`.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn growl(
        &self,
        out: &webaudio::AudioNode,
        t: f64,
        f: f64,
        rate: f64,
        depth_hz: f64,
        attack: f64,
        d20: f64,
        peak: f64,
    ) {
        let ctx = match self.bctx() {
            Some(c) => c,
            None => return,
        };
        let (car, modo, mg, gain) = match (
            ctx.create_oscillator(),
            ctx.create_oscillator(),
            ctx.create_gain(),
            ctx.create_gain(),
        ) {
            (Ok(a), Ok(b), Ok(c), Ok(d)) => (a, b, c, d),
            _ => return,
        };
        let attack = attack.max(0.001);
        let end = t + attack + 4.0 * d20.max(0.01);
        car.set_type(OscillatorType::Sine);
        let _ = car.frequency().set_value_at_time(f as f32, t);
        modo.set_type(OscillatorType::Sine);
        let _ = modo.frequency().set_value_at_time(rate as f32, t);
        let _ = mg.gain().set_value_at_time(depth_hz as f32, t);
        let _ = modo.connect_with_audio_node(&mg);
        let _ = mg.connect_with_audio_param(&car.frequency());
        let g = gain.gain();
        let _ = g.set_value_at_time(0.0001, t);
        let _ = g.linear_ramp_to_value_at_time(peak.max(0.0002) as f32, t + attack);
        let _ = g.exponential_ramp_to_value_at_time(0.0001, end);
        let _ = car.connect_with_audio_node(&gain);
        let _ = gain.connect_with_audio_node(out);
        for o in [&car, &modo] {
            let sched: &webaudio::AudioScheduledSourceNode = o.as_ref();
            let _ = sched.start_with_when(t);
            let _ = sched.stop_with_when(end + 0.02);
        }
    }

    /// A short, heavy body impact without a clean glide: three close low
    /// components (≈ 61 / 88 / 126 Hz) roughened, plus an FM growl and a
    /// low-passed thump — the "wham" of a big object being hit.
    pub(super) fn wham(&self, out: &webaudio::AudioNode, t: f64, peak: f64) {
        const C: [(f64, f64); 3] = [(88.0, 1.0), (126.0, 0.5), (61.0, 0.7)];
        for (f, lvl) in C {
            let f = f * self.jit(0.05);
            let at = self.jt(t, 0.003);
            let o = self.roughen(out, at, 0.3, 0.8);
            self.swell_tone(
                &o,
                f,
                f * 0.96,
                at,
                0.004,
                0.06,
                peak * lvl,
                OscillatorType::Sine,
                0.04,
            );
        }
        let o = self.roughen(out, t, 0.3, 0.6);
        self.growl(
            &o,
            self.jt(t, 0.003),
            56.0 * self.jit(0.06),
            35.0 + self.rand() * 25.0,
            60.0 + self.rand() * 60.0,
            0.01,
            0.05,
            peak * 0.7,
        );
        self.noise_env(
            &o,
            t,
            0.0,
            0.10,
            peak * 0.9,
            BiquadFilterType::Lowpass,
            220.0,
            70.0,
            1.2,
        );
    }

    /// A sub-millisecond broadband click — the pressure step that fronts every
    /// shot and impact. Without it nothing sounds "hit".
    pub(super) fn click(&self, out: &webaudio::AudioNode, t: f64, peak: f64) {
        self.noise_env(
            out,
            t,
            0.0,
            0.0015,
            peak,
            BiquadFilterType::Highpass,
            700.0,
            700.0,
            0.7,
        );
    }

    /// A short mechanical tick — a slide, bolt, pump or a loose part: a
    /// tiny click plus three fast-decaying inharmonic partials.
    pub(super) fn tick(&self, out: &webaudio::AudioNode, t: f64, base: f64, peak: f64) {
        const R: [f64; 3] = [1.0, 1.83, 2.94];
        self.noise_env(
            out,
            t,
            0.0,
            0.004,
            peak * 0.9,
            BiquadFilterType::Bandpass,
            base * 2.0,
            base * 2.0,
            0.6,
        );
        for (i, r) in R.iter().enumerate() {
            let f = base * r;
            let g = peak * 0.7 / (1.0 + i as f64 * 0.8);
            let d = 0.032 - 0.007 * i as f64;
            self.tone_out(out, f, f * 0.98, t, d, g, 0.001, OscillatorType::Sine);
        }
    }

    /// A brass casing hitting the floor: two bright, tiny pings that bounce
    /// twice with decreasing height.
    pub(super) fn tinkle(&self, out: &webaudio::AudioNode, t: f64, peak: f64) {
        const R: [f64; 3] = [1.0, 1.42, 2.31];
        let base = 3900.0 * self.jit(0.08);
        let mut at = t;
        let mut g = peak;
        for bounce in 0..3 {
            for (i, r) in R.iter().enumerate() {
                let f = base * r;
                self.tone_out(
                    out,
                    f,
                    f * 0.995,
                    at,
                    0.07 - 0.012 * i as f64,
                    g / (1.0 + i as f64),
                    0.0005,
                    OscillatorType::Sine,
                );
            }
            at += 0.075 + self.rand() * 0.03 - bounce as f64 * 0.015;
            g *= 0.55;
        }
    }

    /// A short vocal grunt: a rough sawtooth (with a detuned partner for
    /// hoarseness) whose pitch sags, through a swept, resonant bandpass so it
    /// reads as an "uh!" rather than a buzz.
    pub(super) fn grunt(&self, out: &webaudio::AudioNode, t: f64, f: f64, dur: f64, peak: f64) {
        let ctx = match self.bctx() {
            Some(c) => c,
            None => return,
        };
        let filt = match ctx.create_biquad_filter() {
            Ok(f) => f,
            Err(_) => return,
        };
        filt.set_type(BiquadFilterType::Bandpass);
        let ff = filt.frequency();
        let _ = ff.set_value_at_time(760.0, t);
        let _ = ff.exponential_ramp_to_value_at_time(420.0, t + dur);
        let _ = filt.q().set_value_at_time(2.5, t);
        let _ = filt.connect_with_audio_node(out);
        let fo: &webaudio::AudioNode = filt.as_ref();
        self.tone_out(
            fo,
            f,
            f * 0.72,
            t,
            dur,
            peak,
            0.012,
            OscillatorType::Sawtooth,
        );
        self.tone_out(
            fo,
            f * 1.012,
            f * 0.72 * 1.012,
            t,
            dur,
            peak * 0.6,
            0.012,
            OscillatorType::Sawtooth,
        );
    }

    /// Build a per-shot voice: an input gain that (optionally through a
    /// soft-clip WaveShaper, see [`Self::soft_clipper`] — `drive` ≤ 1 = clean)
    /// feeds the dry bus and, at level `wet`, the reverb send. All the layers
    /// of one sound connect to the returned node so they clip and reverberate
    /// *together* like a single recorded event. Falls back to the plain SFX
    /// output when the bus is unavailable; `None` only if there is no context.
    pub(super) fn voice(&self, wet: f64, drive: f64) -> Option<webaudio::AudioNode> {
        self.voice_route(wet, drive, false)
    }

    /// A voice on the gun / hit bus path: no compressor / limiter (crest
    /// preserved) and the longer, brighter reverb.
    pub(super) fn voice_real(&self, wet: f64, drive: f64) -> Option<webaudio::AudioNode> {
        self.voice_route(wet, drive, true)
    }

    /// The voice builder behind [`Self::voice`] / [`Self::voice_real`]:
    /// `real` selects the bus path (dry + reverb) the voice feeds.
    ///
    /// During an offline pre-render the voice is built DRY: the same input
    /// gain and per-voice soft-clip, but feeding the offline destination
    /// with no wet send — the send (and the whole live room / compressor /
    /// bus clip) is reapplied at play time by [`Self::play_baked`], so the
    /// baked buffer captures exactly the signal that live synthesis hands
    /// to the bus.
    pub(super) fn voice_route(
        &self,
        wet: f64,
        drive: f64,
        real: bool,
    ) -> Option<webaudio::AudioNode> {
        if let Some(r) = self.render.borrow().as_ref() {
            let input = r.ctx.create_gain().ok()?;
            let _ = input.gain().set_value_at_time(1.0, 0.0);
            let mut post: webaudio::AudioNode =
                AsRef::<webaudio::AudioNode>::as_ref(&input).clone();
            if drive > 1.0 {
                if let Some(clip) = Self::soft_clipper(&r.ctx, &post, (1.0 / drive) as f32) {
                    post = clip;
                }
            }
            let _ = post.connect_with_audio_node(&r.sink);
            return Some(AsRef::<webaudio::AudioNode>::as_ref(&input).clone());
        }
        let (ctx, bus) = match (&self.ctx, &self.sfx) {
            (Some(c), Some(b)) => (c, b),
            _ => return self.sfx_out(),
        };
        let (dry, reverb_in) = if real {
            (&bus.dry_real, &bus.reverb_real_in)
        } else {
            (&bus.dry, &bus.reverb_in)
        };
        let input = ctx.create_gain().ok()?;
        let _ = input.gain().set_value_at_time(1.0, 0.0);
        let mut post: webaudio::AudioNode = AsRef::<webaudio::AudioNode>::as_ref(&input).clone();
        if drive > 1.0 {
            if let Some(clip) = Self::soft_clipper(ctx, &post, (1.0 / drive) as f32) {
                post = clip;
            }
        }
        let _ = post.connect_with_audio_node(dry);
        if wet > 0.0 {
            if let Ok(send) = ctx.create_gain() {
                let _ = send.gain().set_value_at_time(wet as f32, 0.0);
                let _ = post.connect_with_audio_node(&send);
                let _ = send.connect_with_audio_node(reverb_in);
            }
        }
        Some(AsRef::<webaudio::AudioNode>::as_ref(&input).clone())
    }

    /// Insert a soft clipper after `from`: a 0.5 pre-gain into a WaveShaper
    /// whose curve is the identity below `knee` and a `tanh` squash above it,
    /// with a ceiling of 1 (the pre-gain lets the curve cover ±2 so peaks up
    /// to 2 saturate smoothly instead of hard-clipping at the curve's edge).
    /// A low `knee` (0.2–0.3) is a hot, overloaded crunch on every transient;
    /// 0.7+ only rounds off the loudest peaks. Returns the shaper as the new
    /// tail of the chain, or `None` (chain untouched) if a node fails.
    pub(super) fn soft_clipper(
        ctx: &BaseAudioContext,
        from: &webaudio::AudioNode,
        knee: f32,
    ) -> Option<webaudio::AudioNode> {
        let pre = ctx.create_gain().ok()?;
        let _ = pre.gain().set_value_at_time(0.5, 0.0);
        let shaper = ctx.create_wave_shaper().ok()?;
        let mut curve = Self::softclip_curve(knee, 4096);
        shaper.set_curve_opt_f32_slice(Some(curve.as_mut_slice()));
        shaper.set_oversample(OverSampleType::N2x);
        from.connect_with_audio_node(&pre).ok()?;
        pre.connect_with_audio_node(&shaper).ok()?;
        Some(AsRef::<webaudio::AudioNode>::as_ref(&shaper).clone())
    }

    /// The soft-clip transfer curve for [`Self::soft_clipper`], sampled over an
    /// input range of ±2: `y = u` for `|u| < knee`, then
    /// `knee + (1 - knee)·tanh((|u| - knee) / (1 - knee))` — continuous in
    /// value and slope at the knee, asymptotically 1.
    pub(super) fn softclip_curve(knee: f32, n: usize) -> Vec<f32> {
        let a = knee.clamp(0.05, 0.95);
        (0..n)
            .map(|i| {
                let u = (i as f32 / (n - 1) as f32 * 2.0 - 1.0) * 2.0;
                let m = u.abs();
                let y = if m < a {
                    m
                } else {
                    a + (1.0 - a) * ((m - a) / (1.0 - a)).tanh()
                };
                y.copysign(u)
            })
            .collect()
    }

    /// SFX tone — enveloped oscillator into the SFX bus.
    pub(super) fn tone(
        &self,
        f0: f64,
        f1: f64,
        start: f64,
        dur: f64,
        peak: f64,
        wave: OscillatorType,
    ) {
        if let Some(out) = self.sfx_out() {
            self.tone_out(&out, f0, f1, start, dur, peak, 0.005, wave);
        }
    }

    /// Music tone — enveloped oscillator into the filtered music bus.
    pub(super) fn music_tone(
        &self,
        f0: f64,
        f1: f64,
        start: f64,
        dur: f64,
        peak: f64,
        wave: OscillatorType,
    ) {
        if let Some(out) = self.music_out() {
            self.tone_out(&out, f0, f1, start, dur, peak, 0.005, wave);
        }
    }

    /// A single enveloped oscillator tone connected to `out`. If `f0 != f1` the
    /// pitch glides (exponentially) from `f0` to `f1` over `dur` for glitchy
    /// dives/sweeps. Rises to `peak` over `attack` then decays to near-silence.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn tone_out(
        &self,
        out: &webaudio::AudioNode,
        f0: f64,
        f1: f64,
        start: f64,
        dur: f64,
        peak: f64,
        attack: f64,
        wave: OscillatorType,
    ) {
        let ctx = match self.bctx() {
            Some(c) => c,
            None => return,
        };
        let (osc, gain) = match (ctx.create_oscillator(), ctx.create_gain()) {
            (Ok(o), Ok(g)) => (o, g),
            _ => return,
        };
        osc.set_type(wave);
        let freq = osc.frequency();
        let _ = freq.set_value_at_time(f0 as f32, start);
        if (f1 - f0).abs() > 0.01 {
            let _ = freq.exponential_ramp_to_value_at_time(f1.max(1.0) as f32, start + dur);
        }
        let g = gain.gain();
        let _ = g.set_value_at_time(0.0001, start);
        let _ = g.exponential_ramp_to_value_at_time(peak.max(0.0002) as f32, start + attack);
        let _ = g.exponential_ramp_to_value_at_time(0.0001, start + dur);
        let _ = osc.connect_with_audio_node(&gain);
        let _ = gain.connect_with_audio_node(out);
        let sched: &webaudio::AudioScheduledSourceNode = osc.as_ref();
        let _ = sched.start_with_when(start);
        let _ = sched.stop_with_when(start + dur + 0.02);
    }

    /// SFX noise burst — into the SFX bus.
    pub(super) fn noise(
        &self,
        start: f64,
        dur: f64,
        peak: f64,
        filter: BiquadFilterType,
        f0: f64,
        f1: f64,
    ) {
        if let Some(out) = self.sfx_out() {
            self.noise_out(&out, start, dur, peak, filter, f0, f1);
        }
    }

    /// Music noise burst — into the filtered music bus.
    pub(super) fn music_noise(
        &self,
        start: f64,
        dur: f64,
        peak: f64,
        filter: BiquadFilterType,
        f0: f64,
        f1: f64,
    ) {
        if let Some(out) = self.music_out() {
            self.noise_out(&out, start, dur, peak, filter, f0, f1);
        }
    }

    /// A burst of the shared white-noise buffer through a sweeping biquad
    /// filter and a decaying gain envelope, connected to `out` — used for hits,
    /// whooshes, cracks, and the drum lane. Instant attack, filter Q of 1.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn noise_out(
        &self,
        out: &webaudio::AudioNode,
        start: f64,
        dur: f64,
        peak: f64,
        filter: BiquadFilterType,
        f0: f64,
        f1: f64,
    ) {
        self.noise_env(out, start, 0.0, dur, peak, filter, f0, f1, 1.0);
    }

    /// The general noise layer: `noise_full` with the filter sweep spanning
    /// the whole duration.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn noise_env(
        &self,
        out: &webaudio::AudioNode,
        start: f64,
        attack: f64,
        dur: f64,
        peak: f64,
        filter: BiquadFilterType,
        f0: f64,
        f1: f64,
        q: f64,
    ) {
        self.noise_full(out, start, attack, dur, dur, peak, filter, f0, f1, q);
    }

    /// The fully general noise layer: the shared noise buffer, read from a
    /// random offset (so no two bursts share a waveform), through a biquad
    /// of type `filter` sweeping `f0 → f1` over `sweep` seconds with
    /// resonance `q`, shaped by an envelope that rises to `peak` over
    /// `attack` (0 = instant) and decays exponentially to silence at
    /// `start + dur`.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn noise_full(
        &self,
        out: &webaudio::AudioNode,
        start: f64,
        attack: f64,
        dur: f64,
        sweep: f64,
        peak: f64,
        filter: BiquadFilterType,
        f0: f64,
        f1: f64,
        q: f64,
    ) {
        let (ctx, buf) = match (self.bctx(), &self.noise) {
            (Some(c), Some(b)) => (c, b),
            _ => return,
        };
        let (src, filt, gain) = match (
            ctx.create_buffer_source(),
            ctx.create_biquad_filter(),
            ctx.create_gain(),
        ) {
            (Ok(s), Ok(f), Ok(g)) => (s, f, g),
            _ => return,
        };
        src.set_buffer(Some(buf));
        src.set_loop(true);
        filt.set_type(filter);
        let ff = filt.frequency();
        let _ = ff.set_value_at_time(f0 as f32, start);
        if (f1 - f0).abs() > 1.0 {
            let _ =
                ff.exponential_ramp_to_value_at_time(f1.max(1.0) as f32, start + sweep.max(0.001));
        }
        let _ = filt.q().set_value_at_time(q as f32, start);
        let g = gain.gain();
        let peak = peak.max(0.0002) as f32;
        if attack > 0.0 {
            let _ = g.set_value_at_time(0.0001, start);
            let _ = g.linear_ramp_to_value_at_time(peak, start + attack);
        } else {
            let _ = g.set_value_at_time(peak, start);
        }
        let _ = g.exponential_ramp_to_value_at_time(0.0001, start + dur);
        let _ = src.connect_with_audio_node(&filt);
        let _ = filt.connect_with_audio_node(&gain);
        let _ = gain.connect_with_audio_node(out);
        let sched: &webaudio::AudioScheduledSourceNode = src.as_ref();
        // Random read offset into the (looped) noise buffer.
        let offset = self.rand() * (NOISE_SECONDS - 0.05);
        let _ = src.start_with_when_and_grain_offset(start, offset);
        let _ = sched.stop_with_when(start + dur + 0.02);
    }

    /// A swelling tone: linear attack from silence to `peak` over `attack`,
    /// then an exponential decay that is 20 dB down `d20` seconds after the
    /// peak (silence at 4× that); the pitch glides `f0 → f1` over the whole
    /// life (keep it tiny — clean glides read as cartoon), and `wobble` > 0
    /// adds a 10–30 Hz random walk to the frequency with that RMS fraction
    /// so it never sits still as a clean tone.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn swell_tone(
        &self,
        out: &webaudio::AudioNode,
        f0: f64,
        f1: f64,
        start: f64,
        attack: f64,
        d20: f64,
        peak: f64,
        wave: OscillatorType,
        wobble: f64,
    ) {
        let ctx = match self.bctx() {
            Some(c) => c,
            None => return,
        };
        let (osc, gain) = match (ctx.create_oscillator(), ctx.create_gain()) {
            (Ok(o), Ok(g)) => (o, g),
            _ => return,
        };
        let attack = attack.max(0.001);
        let d20 = d20.max(0.01);
        let end = start + attack + 4.0 * d20;
        osc.set_type(wave);
        let freq = osc.frequency();
        let _ = freq.set_value_at_time(f0 as f32, start);
        if (f1 - f0).abs() > 0.01 {
            let _ = freq.exponential_ramp_to_value_at_time(f1.max(1.0) as f32, end);
        }
        if wobble > 0.0 {
            let fc = 10.0 + self.rand() * 20.0;
            self.noise_lfo(&freq, fc, 0.7, wobble * f0, start, end - start, None);
        }
        let g = gain.gain();
        let _ = g.set_value_at_time(0.0001, start);
        let _ = g.linear_ramp_to_value_at_time(peak.max(0.0002) as f32, start + attack);
        let _ = g.exponential_ramp_to_value_at_time(0.0001, end);
        let _ = osc.connect_with_audio_node(&gain);
        let _ = gain.connect_with_audio_node(out);
        let sched: &webaudio::AudioScheduledSourceNode = osc.as_ref();
        let _ = sched.start_with_when(start);
        let _ = sched.stop_with_when(end + 0.02);
    }
}
