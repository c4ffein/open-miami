//! The persistent graph: music bus + sidechain duck, the SFX bus (compressor,
//! soft-clip, convolver room) and its synthesized impulse responses / noise.

use super::*;

impl AudioEngine {
    /// Build the persistent music bus: a gain node feeding a lowpass biquad
    /// (whose cutoff we sweep per bar) into the destination. Returns
    /// `(None, None)` if any node fails to build.
    pub(super) fn make_music_bus(
        ctx: &AudioContext,
    ) -> (Option<GainNode>, Option<BiquadFilterNode>) {
        let (gain, filt) = match (ctx.create_gain(), ctx.create_biquad_filter()) {
            (Ok(g), Ok(f)) => (g, f),
            _ => return (None, None),
        };
        filt.set_type(BiquadFilterType::Lowpass);
        let _ = filt.frequency().set_value_at_time(3000.0, 0.0);
        // A little resonance makes the sweep sing (that synthwave edge).
        let _ = filt.q().set_value_at_time(3.0, 0.0);
        let _ = gain.gain().set_value_at_time(1.0, 0.0);
        let _ = gain.connect_with_audio_node(&filt);
        let _ = filt.connect_with_audio_node(&ctx.destination());
        (Some(gain), Some(filt))
    }

    /// Build the SIDECHAIN DUCK stage: one gain node feeding the music bus.
    /// Melodic voices enter through it; `schedule_step` automates it in
    /// ducked sections. `None` (melodics fall back to the plain bus) if the
    /// node cannot be built or wired.
    pub(super) fn make_duck(ctx: &AudioContext, bus: &GainNode) -> Option<GainNode> {
        let duck = ctx.create_gain().ok()?;
        let _ = duck.gain().set_value_at_time(1.0, 0.0);
        duck.connect_with_audio_node(bus).ok()?;
        Some(duck)
    }

    /// Build ~0.5s of white noise into an `AudioBuffer` we can reuse forever.
    /// Build the persistent SFX bus (see [`SfxBus`]): dry + reverb paths
    /// summing into a compressor, then a gentle soft-clip and an output trim.
    /// Returns `None` if any essential node fails to build (the reverb is
    /// optional: without it the bus is dry but still compressed).
    pub(super) fn make_sfx_bus(ctx: &AudioContext) -> Option<SfxBus> {
        let comp = ctx.create_dynamics_compressor().ok()?;
        // Fast and firm: grabs the crack of a shot and audibly pumps back
        // over ~180 ms; the mechanical tails sit below threshold and keep
        // their dynamics.
        let _ = comp.threshold().set_value_at_time(-14.0, 0.0);
        let _ = comp.knee().set_value_at_time(6.0, 0.0);
        let _ = comp.ratio().set_value_at_time(4.0, 0.0);
        let _ = comp.attack().set_value_at_time(0.002, 0.0);
        let _ = comp.release().set_value_at_time(0.18, 0.0);

        // Bus limiter after the compressor: fills the wall (waveform
        // kurtosis ~2–2.5, crest ~8–12 dB on a single shot, ~12–15 dB on a
        // hit) without brick-walling.
        let mut last: web_sys::AudioNode = AsRef::<web_sys::AudioNode>::as_ref(&comp).clone();
        if let Ok(lim) = ctx.create_dynamics_compressor() {
            let _ = lim.threshold().set_value_at_time(-6.0, 0.0);
            let _ = lim.knee().set_value_at_time(0.0, 0.0);
            let _ = lim.ratio().set_value_at_time(20.0, 0.0);
            let _ = lim.attack().set_value_at_time(0.001, 0.0);
            let _ = lim.release().set_value_at_time(0.10, 0.0);
            if last.connect_with_audio_node(&lim).is_ok() {
                last = AsRef::<web_sys::AudioNode>::as_ref(&lim).clone();
            }
        }
        // Both paths sum here, into the bus soft-clipper.
        let sum = ctx.create_gain().ok()?;
        let _ = sum.gain().set_value_at_time(1.0, 0.0);
        let _ = last.connect_with_audio_node(&sum);

        // Gun / hit path: no compressor, no limiter — only a gentle
        // −1 dB / 4:1 safety so a crack keeps its 18–22 dB crest.
        let dry_real = ctx.create_gain().ok()?;
        let _ = dry_real.gain().set_value_at_time(1.0, 0.0);
        let real_sum: web_sys::AudioNode = match ctx.create_dynamics_compressor() {
            Ok(safe) => {
                let _ = safe.threshold().set_value_at_time(-1.0, 0.0);
                let _ = safe.knee().set_value_at_time(1.0, 0.0);
                let _ = safe.ratio().set_value_at_time(4.0, 0.0);
                let _ = safe.attack().set_value_at_time(0.0005, 0.0);
                let _ = safe.release().set_value_at_time(0.08, 0.0);
                let _ = safe.connect_with_audio_node(&sum);
                AsRef::<web_sys::AudioNode>::as_ref(&safe).clone()
            }
            Err(_) => AsRef::<web_sys::AudioNode>::as_ref(&sum).clone(),
        };
        let _ = dry_real.connect_with_audio_node(&real_sum);

        // Bus soft-clipper: barely touches normal peaks, rounds off the sum
        // of simultaneous shots instead of letting the DAC hard-clip.
        let trim = ctx.create_gain().ok()?;
        let _ = trim.gain().set_value_at_time(SFX_GAIN as f32, 0.0);
        last = AsRef::<web_sys::AudioNode>::as_ref(&sum).clone();
        if let Some(clip) = Self::soft_clipper(ctx, &last, 0.7) {
            last = clip;
        }
        let _ = last.connect_with_audio_node(&trim);
        let _ = trim.connect_with_audio_node(&ctx.destination());

        let dry = ctx.create_gain().ok()?;
        let _ = dry.gain().set_value_at_time(1.0, 0.0);
        let _ = dry.connect_with_audio_node(&comp);

        // Reverb: send -> low-cut -> convolver(IR) -> return -> compressor.
        let reverb_in = ctx.create_gain().ok()?;
        let _ = reverb_in.gain().set_value_at_time(1.0, 0.0);
        let wired = (|| {
            let hpf = ctx.create_biquad_filter().ok()?;
            hpf.set_type(BiquadFilterType::Highpass);
            let _ = hpf.frequency().set_value_at_time(180.0, 0.0);
            let _ = hpf.q().set_value_at_time(0.7, 0.0);
            let conv = ctx.create_convolver().ok()?;
            conv.set_normalize(true);
            conv.set_buffer(Some(&Self::make_impulse(ctx)?));
            let ret = ctx.create_gain().ok()?;
            let _ = ret.gain().set_value_at_time(REVERB_RETURN as f32, 0.0);
            reverb_in.connect_with_audio_node(&hpf).ok()?;
            hpf.connect_with_audio_node(&conv).ok()?;
            conv.connect_with_audio_node(&ret).ok()?;
            ret.connect_with_audio_node(&comp).ok()?;
            Some(())
        })();

        // Gun / hit reverb: send -> low-cut -> convolver(bright, long IR)
        // -> return -> the real path's safety limiter.
        let reverb_real_in = ctx.create_gain().ok()?;
        let _ = reverb_real_in.gain().set_value_at_time(1.0, 0.0);
        let _ = (|| {
            let hpf = ctx.create_biquad_filter().ok()?;
            hpf.set_type(BiquadFilterType::Highpass);
            let _ = hpf.frequency().set_value_at_time(120.0, 0.0);
            let _ = hpf.q().set_value_at_time(0.7, 0.0);
            let conv = ctx.create_convolver().ok()?;
            conv.set_normalize(true);
            conv.set_buffer(Some(&Self::make_impulse_real(ctx)?));
            let ret = ctx.create_gain().ok()?;
            let _ = ret.gain().set_value_at_time(REVERB_REAL_RETURN as f32, 0.0);
            reverb_real_in.connect_with_audio_node(&hpf).ok()?;
            hpf.connect_with_audio_node(&conv).ok()?;
            conv.connect_with_audio_node(&ret).ok()?;
            ret.connect_with_audio_node(&real_sum).ok()?;
            Some(())
        })();

        // Default voice for the misc. SFX: dry + a light room send.
        let room = ctx.create_gain().ok()?;
        let _ = room.gain().set_value_at_time(1.0, 0.0);
        let _ = room.connect_with_audio_node(&dry);
        if wired.is_some() {
            if let Ok(send) = ctx.create_gain() {
                let _ = send.gain().set_value_at_time(0.18, 0.0);
                let _ = room.connect_with_audio_node(&send);
                let _ = send.connect_with_audio_node(&reverb_in);
            }
        }

        Some(SfxBus {
            dry,
            reverb_in,
            dry_real,
            reverb_real_in,
            room,
        })
    }

    /// The gun / hit impulse response: `IR_REAL_SECONDS` of stereo noise
    /// with the measured field-recording room envelope after the peak —
    /// −10 dB @100 ms, −21 @200, −25 @300, −31 @600, −41 @1000, −50 @1500 —
    /// (piecewise-linear in dB), hotter sparse early reflections, and a
    /// FIXED one-pole lowpass (~3 kHz) so the tail stays mid-bright
    /// (centroid ~1.1–1.5 kHz) instead of darkening.
    pub(super) fn make_impulse_real(ctx: &AudioContext) -> Option<AudioBuffer> {
        let sr = ctx.sample_rate();
        let len = (sr as f64 * IR_REAL_SECONDS) as u32;
        if len == 0 {
            return None;
        }
        let buf = ctx.create_buffer(2, len, sr).ok()?;
        let predelay = (sr as f64 * 0.006) as usize;
        let early = (sr as f64 * 0.04) as usize;
        const PTS: [(f64, f64); 8] = [
            (0.0, 0.0),
            (0.1, -10.0),
            (0.2, -21.0),
            (0.3, -25.0),
            (0.6, -31.0),
            (1.0, -41.0),
            (1.5, -50.0),
            (1.7, -60.0),
        ];
        let env_db = |t: f64| -> f64 {
            for w in PTS.windows(2) {
                let (t0, d0) = w[0];
                let (t1, d1) = w[1];
                if t <= t1 {
                    return d0 + (d1 - d0) * ((t - t0) / (t1 - t0)).clamp(0.0, 1.0);
                }
            }
            -60.0
        };
        let a = (-2.0 * std::f64::consts::PI * 3000.0 / sr as f64).exp() as f32;
        let mut data = vec![0f32; len as usize];
        for ch in 0..2u32 {
            let mut state: u32 = 0x7F4A_7C15 ^ (ch.wrapping_mul(0x9E37_79B9) + 3);
            let mut lp = 0f32;
            for (i, x) in data.iter_mut().enumerate() {
                if i < predelay {
                    *x = 0.0;
                    continue;
                }
                let n = i - predelay;
                let t = n as f64 / sr as f64;
                state ^= state << 13;
                state ^= state >> 17;
                state ^= state << 5;
                let white = (state as f32 / u32::MAX as f32) * 2.0 - 1.0;
                let mut env = 10f64.powf(env_db(t) / 20.0);
                if n < early {
                    let sparse = if state & 3 == 0 { 1.0 } else { 0.4 };
                    env *= (1.0 + 0.5 * (1.0 - n as f64 / early as f64)) * sparse;
                }
                lp = a * lp + (1.0 - a) * white;
                *x = lp * env as f32;
            }
            buf.copy_to_channel(&data, ch as i32).ok()?;
        }
        Some(buf)
    }

    /// Synthesize a stereo room impulse response: `IR_SECONDS` of white noise
    /// (independent per channel, for width) under an exponential decay, with a
    /// short pre-delay, a denser/louder first 40 ms of early reflections, and
    /// a one-pole lowpass whose cutoff falls over the tail (air absorption —
    /// the highs die first, exactly as in a real medium-sized concrete room).
    pub(super) fn make_impulse(ctx: &AudioContext) -> Option<AudioBuffer> {
        let sr = ctx.sample_rate();
        let len = (sr as f64 * IR_SECONDS) as u32;
        if len == 0 {
            return None;
        }
        let buf = ctx.create_buffer(2, len, sr).ok()?;
        let predelay = (sr as f64 * 0.009) as usize;
        let early = (sr as f64 * 0.04) as usize;
        // RT60 ≈ 6.9·tau; tau chosen for a ~0.9 s tail.
        let tau = 0.13f64;
        let mut data = vec![0f32; len as usize];
        for ch in 0..2u32 {
            let mut state: u32 = 0xA511_E9B3 ^ (ch.wrapping_mul(0x6C8E_9CF5) + 1);
            let mut lp = 0f32;
            for (i, x) in data.iter_mut().enumerate() {
                if i < predelay {
                    *x = 0.0;
                    continue;
                }
                let n = i - predelay;
                let t = n as f64 / sr as f64;
                state ^= state << 13;
                state ^= state >> 17;
                state ^= state << 5;
                let white = (state as f32 / u32::MAX as f32) * 2.0 - 1.0;
                let mut env = (-t / tau).exp();
                if n < early {
                    // Sparse-ish, hotter early reflections, blending into
                    // the diffuse tail by the end of the window.
                    let sparse = if state & 3 == 0 { 1.0 } else { 0.35 };
                    let hot = 1.0 + 0.7 * (1.0 - n as f64 / early as f64);
                    env *= hot * sparse;
                }
                // Air absorption: cutoff slides from ~7 kHz to ~1.5 kHz.
                let fc = 7000.0 * (-t / 0.45).exp() + 1500.0;
                let a = (-2.0 * std::f64::consts::PI * fc / sr as f64).exp() as f32;
                lp = a * lp + (1.0 - a) * white;
                *x = lp * env as f32;
            }
            buf.copy_to_channel(&data, ch as i32).ok()?;
        }
        Some(buf)
    }

    /// Build `NOISE_SECONDS` of white noise into an `AudioBuffer` we can reuse
    /// forever (bursts read it from random offsets). Uses a tiny xorshift PRNG
    /// so we need no `rand`/`js_sys` dependency.
    pub(super) fn make_noise(ctx: &AudioContext) -> Option<AudioBuffer> {
        let sr = ctx.sample_rate();
        let len = (sr as f64 * NOISE_SECONDS) as u32;
        if len == 0 {
            return None;
        }
        let buf = ctx.create_buffer(1, len, sr).ok()?;
        let mut data = vec![0f32; len as usize];
        let mut state: u32 = 0x9E37_79B9;
        for x in data.iter_mut() {
            state ^= state << 13;
            state ^= state >> 17;
            state ^= state << 5;
            *x = (state as f32 / u32::MAX as f32) * 2.0 - 1.0;
        }
        buf.copy_to_channel(&data, 0).ok()?;
        Some(buf)
    }
}
