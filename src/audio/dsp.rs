//! COMPUTED voices: notes rendered sample by sample in plain Rust — a
//! physical plucked string (guitar, bass guitar) and a bowed string (violin)
//! — straight into the bake buffer. Web Audio nodes cannot build these: a
//! feedback loop through nodes has a minimum delay of one render quantum
//! (128 samples ≈ 2.7 ms), which caps a string model near 370 Hz. Here the
//! loop is a ring buffer and any pitch works.
//!
//! Everything is pure: `(voice, note, sample rate) -> Vec<f32>`, seeded,
//! deterministic, host-tested (`tests` below check pitch, decay, level and
//! silence at the end). The engine (`audio/engine/music.rs`,
//! `computed_bake`) copies the result into an `AudioBuffer`; the live
//! fallback for an unbaked computed note is the usual one-oscillator sketch.

use super::voice::{Filter, Vibrato, Voice, Wave};

/// One partial of a computed note: its pitch (and the pitch it glides in
/// from), its level, and how late it starts (a strum spreads a chord).
#[derive(Clone, Copy, Debug)]
pub struct Partial {
    pub f: f64,
    pub from: Option<f64>,
    pub peak: f64,
    pub delay: f64,
}

/// The envelope times of a computed note, in seconds: `attack` to peak,
/// `hold` at peak (the tied steps), then `dur` of release. A plucked
/// string decays on its own during the hold; the release only fades what
/// is left so the note ends inside its bake.
#[derive(Clone, Copy, Debug)]
pub struct Shape {
    pub attack: f64,
    pub hold: f64,
    pub dur: f64,
}

impl Shape {
    /// The note's whole length in seconds (plus the bake's 30 ms margin).
    pub fn total(&self) -> f64 {
        self.attack.max(0.0) + self.hold + self.dur + 0.03
    }
}

/// Seconds a chord's partials are staggered by on a plucked string: a
/// strum, low string first.
pub const STRUM_SECONDS: f64 = 0.014;

/// Samples per CONTROL block: pitch, loop loss and filter coefficients are
/// recomputed this often (0.7 ms at 48 kHz — well under any audible
/// modulation), not per sample. A note costs its samples, not its `sin`s.
const BLOCK: usize = 32;

/// Render one note of a computed [`Voice`] (`wave` must satisfy
/// [`Wave::is_computed`]): every partial summed, the sub-oscillator, the
/// voice's filter envelope, the release fade. `None` for a wave that is not
/// computed. Mono.
pub fn render_note(voice: &Voice, partials: &[Partial], shape: Shape, sr: f64) -> Option<Vec<f32>> {
    if !voice.wave.is_computed() || sr <= 0.0 {
        return None;
    }
    let frames = (shape.total() * sr).ceil().max(1.0) as usize;
    let mut out = vec![0f32; frames];
    let mut rng = Rng(0x9E37_79B9 ^ (voice.wave as u32).wrapping_mul(0x85EB_CA6B));
    for p in partials {
        let pitch = Pitch {
            f0: p.from.unwrap_or(p.f),
            f1: p.f,
            glide: voice.glide,
            vibrato: voice.vibrato,
        };
        let one = match voice.wave {
            Wave::Guitar => pluck(&pitch, shape, sr, &GUITAR, &mut rng),
            Wave::BassGuitar => pluck(&pitch, shape, sr, &BASS_GUITAR, &mut rng),
            Wave::Violin => bowed(&pitch, shape, sr, &mut rng),
            _ => return None,
        };
        let at = (p.delay.max(0.0) * sr) as usize;
        for (j, s) in one.iter().enumerate() {
            if let Some(o) = out.get_mut(at + j) {
                *o += s * p.peak as f32;
            }
        }
    }
    if voice.sub > 0.0 {
        if let Some(p) = partials.first() {
            let pitch = Pitch {
                f0: p.from.unwrap_or(p.f) * 0.5,
                f1: p.f * 0.5,
                glide: voice.glide,
                vibrato: None,
            };
            let level = (p.peak * voice.sub.clamp(0.0, 1.0)) as f32;
            let mut phase = 0.0f64;
            let mut dphase = 0.0;
            for (i, o) in out.iter_mut().enumerate() {
                let t = i as f64 / sr;
                if i % BLOCK == 0 {
                    dphase = pitch.at(t, shape.total()) / sr;
                }
                phase += dphase;
                *o += (phase * std::f64::consts::TAU).sin() as f32 * level * env(t, shape) as f32;
            }
        }
    }
    if let Some(flt) = voice.filter {
        filter_envelope(&mut out, &flt, sr);
    }
    // The release: whatever is still ringing fades out over `dur`.
    let release_from = shape.attack.max(0.0) + shape.hold;
    let first = ((release_from * sr) as usize).min(out.len());
    let mut fade = 1f32;
    for (i, o) in out[first..].iter_mut().enumerate() {
        if i % BLOCK == 0 {
            let x = (i as f64 / sr / shape.dur.max(1e-3)).min(1.0);
            fade = ((-6.9 * x).exp() * (1.0 - x)) as f32;
        }
        *o *= fade;
    }
    Some(out)
}

/// The amplitude envelope of a sustaining (bowed) note at `t`: a linear
/// swell over `attack`, unity through the hold, then [`render_note`]'s
/// release fade.
fn env(t: f64, shape: Shape) -> f64 {
    let a = shape.attack.max(0.001);
    if t < a {
        t / a
    } else {
        1.0
    }
}

/// A note's pitch over time: a glide `f0 → f1` (exponential, over `glide`
/// seconds, or the whole note when 0) and a delayed vibrato on top.
struct Pitch {
    f0: f64,
    f1: f64,
    glide: f64,
    vibrato: Option<Vibrato>,
}

impl Pitch {
    fn at(&self, t: f64, total: f64) -> f64 {
        let span = if self.glide > 0.0 {
            self.glide.min(total)
        } else {
            total
        };
        let x = (t / span.max(1e-6)).clamp(0.0, 1.0);
        let mut f = self.f0 * (self.f1 / self.f0).powf(x);
        if let Some(v) = self.vibrato.filter(|v| v.depth > 0.0 && v.rate > 0.0) {
            let fade = (t / v.delay.max(0.001)).min(1.0);
            let cents = v.depth * fade * (std::f64::consts::TAU * v.rate * t).sin();
            f *= 2f64.powf(cents / 1200.0);
        }
        f.max(1.0)
    }

    /// The lowest pitch the note reaches (sizes the string's delay line).
    fn min(&self) -> f64 {
        let depth = self.vibrato.map_or(0.0, |v| v.depth.abs());
        self.f0.min(self.f1) * 2f64.powf(-depth / 1200.0)
    }
}

/// xorshift32, seeded per note: the same note bakes the same samples.
struct Rng(u32);

impl Rng {
    fn next(&mut self) -> f32 {
        let mut s = self.0;
        s ^= s << 13;
        s ^= s >> 17;
        s ^= s << 5;
        self.0 = s;
        (s as f32 / u32::MAX as f32) * 2.0 - 1.0
    }
}

/// A Chamberlin state-variable filter: one `tick` per sample, cutoff and
/// resonance free to move every sample (what the filter envelope and the
/// body resonances need).
#[derive(Default, Clone, Copy)]
struct Svf {
    lp: f64,
    bp: f64,
}

/// A filter's `(f, damp)` coefficients: `cutoff` in Hz (kept under a
/// quarter of the sample rate for stability), `q` ≥ 0.5. Compute once per
/// block, run [`Svf::tick`] per sample.
fn svf_coef(cutoff: f64, q: f64, sr: f64) -> (f64, f64) {
    let f = 2.0 * (std::f64::consts::PI * cutoff.clamp(10.0, sr * 0.24) / sr).sin();
    (f * 0.5, (1.0 / q.max(0.5)).min(2.0))
}

impl Svf {
    /// One sample through the filter with [`svf_coef`]'s `(f, damp)`;
    /// returns `(lowpass, bandpass)`.
    #[inline]
    fn tick(&mut self, x: f64, (f, damp): (f64, f64)) -> (f64, f64) {
        // Two passes per sample halve the effective step: stable to sr/4.
        for _ in 0..2 {
            let hp = x - self.lp - damp * self.bp;
            self.bp += f * hp;
            self.lp += f * self.bp;
        }
        (self.lp, self.bp)
    }
}

/// The voice's per-note LOWPASS envelope applied in place: the cutoff
/// opens from `cutoff` to `peak` over the attack, then falls back over the
/// decay (the same curve the node voices get from `note_filter`).
fn filter_envelope(out: &mut [f32], flt: &Filter, sr: f64) {
    let lo = flt.cutoff.clamp(20.0, 18000.0);
    let hi = flt.peak.clamp(20.0, 18000.0).max(lo);
    let mut svf = Svf::default();
    let mut coef = (0.0, 0.0);
    for (i, o) in out.iter_mut().enumerate() {
        if i % BLOCK == 0 {
            let t = i as f64 / sr;
            let cutoff = if flt.attack > 0.0 && t < flt.attack {
                lo * (hi / lo).powf(t / flt.attack)
            } else if flt.decay > 0.0 {
                let since = t - flt.attack.max(0.0);
                hi * (lo / hi).powf((since / flt.decay).clamp(0.0, 1.0))
            } else {
                hi
            };
            coef = svf_coef(cutoff, flt.q, sr);
        }
        *o = svf.tick(f64::from(*o), coef).0 as f32;
    }
}

/// A fractional delay line: the string. The fraction is an ALLPASS
/// interpolation, not a linear one — linear interpolation is a lowpass
/// that would darken every trip round the loop and kill the string in a
/// few hundred milliseconds; the allpass keeps the harmonics and only
/// shifts their phase.
struct Delay {
    buf: Vec<f32>,
    w: usize,
    /// The allpass interpolator's previous output.
    ap: f32,
}

impl Delay {
    fn new(max: usize) -> Delay {
        Delay {
            buf: vec![0.0; (max + 4).next_power_of_two()],
            w: 0,
            ap: 0.0,
        }
    }

    /// The sample written `d` samples ago (`d` ≥ 1, fractional). One call
    /// per written sample (the interpolator carries state).
    #[inline]
    fn read(&mut self, d: f64) -> f32 {
        let n = self.buf.len();
        let mask = n - 1;
        let d = d.max(1.0).min((n - 2) as f64);
        let i = d.floor() as usize;
        let frac = (d - i as f64) as f32;
        let a = self.buf[(self.w + n - i) & mask];
        let b = self.buf[(self.w + n - i - 1) & mask];
        // First-order allpass: y = c·x[n−i] + x[n−i−1] − c·y[n−1].
        let c = (1.0 - frac) / (1.0 + frac);
        let y = c * a + b - c * self.ap;
        self.ap = y;
        y
    }

    #[inline]
    fn write(&mut self, x: f32) {
        self.w = (self.w + 1) & (self.buf.len() - 1);
        self.buf[self.w] = x;
    }
}

/// What makes one plucked string sound like itself.
struct StringModel {
    /// Seconds the fundamental takes to fall 60 dB when left alone.
    t60: f64,
    /// Pick softness: the excitation's one-pole lowpass coefficient
    /// (0 = white burst, 0.9 = a thumb).
    pick_soft: f64,
    /// Where along the string it is plucked, `0..0.5` (a comb notch).
    pick_pos: f64,
    /// The loop filter's darkening (0 = a bright steel string that keeps
    /// its highs, 0.5 = a dull nylon / flatwound one).
    damping: f64,
    /// A body resonance `(Hz, q, mix)` added to the string.
    body: (f64, f64, f64),
}

const GUITAR: StringModel = StringModel {
    t60: 3.0,
    pick_soft: 0.7,
    pick_pos: 0.22,
    damping: 0.25,
    body: (190.0, 2.5, 0.25),
};

const BASS_GUITAR: StringModel = StringModel {
    t60: 4.0,
    pick_soft: 0.9,
    pick_pos: 0.15,
    damping: 0.45,
    body: (95.0, 1.5, 0.2),
};

/// Karplus–Strong with a fractional delay line, a one-pole loop filter and
/// a pitch-dependent loss (every pitch decays over the model's `t60`, not
/// over the same number of periods): a noise burst, comb-notched at the
/// pick position, circulates and darkens.
fn pluck(pitch: &Pitch, shape: Shape, sr: f64, m: &StringModel, rng: &mut Rng) -> Vec<f32> {
    let frames = (shape.total() * sr).ceil() as usize;
    let mut out = vec![0f32; frames];
    let max_len = (sr / pitch.min()).ceil() as usize + 2;
    let mut line = Delay::new(max_len);
    // The excitation: one period of softened, comb-notched noise.
    let period = (sr / pitch.at(0.0, shape.total())).round().max(2.0) as usize;
    let notch = ((period as f64) * m.pick_pos).round().max(1.0) as usize;
    let mut burst = Vec::with_capacity(period);
    let mut lp = 0f32;
    for _ in 0..period {
        lp = m.pick_soft as f32 * lp + (1.0 - m.pick_soft as f32) * rng.next();
        burst.push(lp);
    }
    let combed: Vec<f32> = (0..period)
        .map(|i| burst[i] - if i >= notch { burst[i - notch] } else { 0.0 })
        .collect();
    // Level the burst so the RINGING string (a few periods in, once the
    // loop has darkened the burst) sits near a saw of amplitude 1 — the
    // plucked and the bowed strings at the same loudness for the same
    // `peak`. The pick's own crest may go past 1 (the bus soft-clip has the
    // headroom: voices peak around 0.07).
    let peak = combed.iter().fold(0f32, |a, &v| a.max(v.abs())).max(1e-6);
    let rms = (combed.iter().map(|v| v * v).sum::<f32>() / period as f32)
        .sqrt()
        .max(1e-6);
    let scale = (1.2 / rms).min(2.5 / peak);
    let mut body = Svf::default();
    let body_coef = svf_coef(m.body.0, m.body.1, sr);
    let mut s = 0f32;
    let total = shape.total();
    let (damp, keep) = (m.damping as f32, 1.0 - m.damping as f32);
    let (mut d, mut g) = (0.0, 0f32);
    for (i, o) in out.iter_mut().enumerate() {
        if i % BLOCK == 0 {
            let f = pitch.at(i as f64 / sr, total);
            // The loop's own delay: the line minus the one-pole's half
            // sample.
            d = sr / f - 0.5;
            // Loss per TRIP round the loop (one period) so the fundamental
            // hits −60 dB at t60 whatever the pitch.
            g = (-6.907_755 / (m.t60 * f)).exp() as f32;
        }
        let x = line.read(d);
        s = damp * s + keep * x;
        let y = g * s + if i < period { combed[i] * scale } else { 0.0 };
        line.write(y);
        let (_, bp) = body.tick(f64::from(y), body_coef);
        *o = y + (bp * m.body.2) as f32;
    }
    out
}

/// A bowed string: a band-limited (polyBLEP) sawtooth — the stick-slip of
/// the bow — with a breath of bow noise, through three body resonances and
/// a top-end roll-off, under the note's swell / hold / release envelope.
fn bowed(pitch: &Pitch, shape: Shape, sr: f64, rng: &mut Rng) -> Vec<f32> {
    let frames = (shape.total() * sr).ceil() as usize;
    let mut out = vec![0f32; frames];
    let total = shape.total();
    // Bowed onsets are never instant.
    let shape = Shape {
        attack: shape.attack.max(0.06),
        ..shape
    };
    let mut phase = 0.0f64;
    let mut body = [Svf::default(); 3];
    let mut noise_bp = Svf::default();
    let mut roll = Svf::default();
    const MODES: [(f64, f64, f64); 3] =
        [(275.0, 3.0, 0.5), (520.0, 4.0, 0.35), (1300.0, 3.0, 0.25)];
    let modes = MODES.map(|(hz, q, level)| (svf_coef(hz, q, sr), level));
    let hiss_coef = svf_coef(3200.0, 1.2, sr);
    let roll_coef = svf_coef(4200.0, 0.7, sr);
    let mut dt = 0.0;
    for (i, o) in out.iter_mut().enumerate() {
        let t = i as f64 / sr;
        if i % BLOCK == 0 {
            dt = pitch.at(t, total) / sr;
        }
        phase += dt;
        if phase >= 1.0 {
            phase -= 1.0;
        }
        let saw = 2.0 * phase - 1.0 - poly_blep(phase, dt);
        let (_, hiss) = noise_bp.tick(f64::from(rng.next()), hiss_coef);
        let dry = saw * 0.6 + hiss * 0.035;
        let mut mix = dry * 0.55;
        for (k, (coef, level)) in modes.iter().enumerate() {
            mix += body[k].tick(dry, *coef).1 * level;
        }
        let (lp, _) = roll.tick(mix, roll_coef);
        *o = (lp * env(t, shape)) as f32;
    }
    out
}

/// The polyBLEP correction that band-limits a sawtooth's discontinuity.
fn poly_blep(t: f64, dt: f64) -> f64 {
    if t < dt {
        let x = t / dt;
        x + x - x * x - 1.0
    } else if t > 1.0 - dt {
        let x = (t - 1.0) / dt;
        x * x + x + x + 1.0
    } else {
        0.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SR: f64 = 48_000.0;

    fn one(voice: Voice, f: f64, shape: Shape) -> Vec<f32> {
        let p = Partial {
            f,
            from: None,
            peak: 0.5,
            delay: 0.0,
        };
        render_note(&voice, &[p], shape, SR).unwrap()
    }

    /// The fundamental of `s` between `from` and `to` seconds, by
    /// autocorrelation over the plausible period range.
    fn pitch_of(s: &[f32], from: f64, to: f64) -> f64 {
        let a = (from * SR) as usize;
        let b = ((to * SR) as usize).min(s.len());
        let w = &s[a..b];
        let corr: Vec<f64> = (0..2400)
            .map(|lag| {
                let n = w.len() - lag;
                (0..n)
                    .map(|i| f64::from(w[i]) * f64::from(w[i + lag]))
                    .sum::<f64>()
                    / n as f64
            })
            .collect();
        // The SMALLEST local-maximum lag that correlates about as well as
        // the best one (a periodic signal correlates at every multiple of
        // its period), refined between samples by a parabola.
        let best = corr[20..].iter().cloned().fold(f64::MIN, f64::max);
        let lag = (20..2399)
            .find(|&l| corr[l] >= best * 0.9 && corr[l] >= corr[l - 1] && corr[l] >= corr[l + 1])
            .unwrap();
        let (a, b, c) = (corr[lag - 1], corr[lag], corr[lag + 1]);
        let shift = 0.5 * (a - c) / (a - 2.0 * b + c).min(-1e-12);
        SR / (lag as f64 + shift)
    }

    fn rms(s: &[f32], from: f64, to: f64) -> f64 {
        let a = (from * SR) as usize;
        let b = ((to * SR) as usize).min(s.len());
        (s[a..b].iter().map(|v| f64::from(*v).powi(2)).sum::<f64>() / (b - a).max(1) as f64).sqrt()
    }

    #[test]
    fn plucked_strings_ring_at_pitch_decay_and_end_silent() {
        for (wave, f) in [
            (Wave::Guitar, 220.0),
            (Wave::Guitar, 660.0),
            (Wave::BassGuitar, 41.2),
        ] {
            let shape = Shape {
                attack: 0.005,
                hold: 1.0,
                dur: 0.6,
            };
            let s = one(Voice::mono(wave), f, shape);
            assert_eq!(s.len(), (shape.total() * SR).ceil() as usize);
            // The pick's crest may pass the ringing level (2.5 × the peak).
            assert!(
                s.iter().all(|v| v.is_finite() && v.abs() <= 2.0),
                "{wave:?}"
            );
            let measured = pitch_of(&s, 0.1, 0.5);
            assert!(
                (measured / f - 1.0).abs() < 0.015,
                "{wave:?} at {f} Hz rings at {measured:.1} Hz"
            );
            let (early, mid, late) = (rms(&s, 0.02, 0.12), rms(&s, 0.4, 0.5), rms(&s, 0.8, 0.9));
            assert!(early > 0.05, "{wave:?}: too quiet ({early})");
            // −60 dB at t60 is about −10 dB at 0.45 s: still clearly ringing.
            assert!(
                mid > early * 0.15,
                "{wave:?}: dies too fast ({early} -> {mid} at 0.45 s)"
            );
            assert!(
                late < early * 0.5,
                "{wave:?}: does not decay ({early} -> {late})"
            );
            assert!(
                rms(&s, shape.total() - 0.02, shape.total()) < 1e-3,
                "{wave:?}: not silent at the end"
            );
        }
    }

    #[test]
    fn the_bowed_string_swells_holds_at_pitch_and_releases() {
        let shape = Shape {
            attack: 0.15,
            hold: 0.8,
            dur: 0.3,
        };
        let v = Voice::mono(Wave::Violin).with_vibrato(5.5, 8.0, 0.4);
        let s = one(v, 440.0, shape);
        assert!(s.iter().all(|v| v.is_finite() && v.abs() <= 1.0));
        assert!(rms(&s, 0.0, 0.03) < rms(&s, 0.3, 0.5) * 0.5, "no swell");
        let held = rms(&s, 0.3, 0.9);
        assert!(held > 0.05, "too quiet: {held}");
        let measured = pitch_of(&s, 0.3, 0.9);
        assert!(
            (measured / 440.0 - 1.0).abs() < 0.015,
            "rings at {measured:.1} Hz"
        );
        assert!(rms(&s, shape.total() - 0.02, shape.total()) < 1e-3);
        // Deterministic: the same note bakes the same samples.
        assert_eq!(s, one(v, 440.0, shape));
    }

    #[test]
    fn a_glide_arrives_at_the_target_pitch_and_a_strum_staggers() {
        let shape = Shape {
            attack: 0.1,
            hold: 1.0,
            dur: 0.2,
        };
        let v = Voice::mono(Wave::Violin).with_glide(0.3);
        let p = Partial {
            f: 440.0,
            from: Some(330.0),
            peak: 0.5,
            delay: 0.0,
        };
        let s = render_note(&v, &[p], shape, SR).unwrap();
        assert!((pitch_of(&s, 0.6, 1.0) / 440.0 - 1.0).abs() < 0.015);
        assert!(pitch_of(&s, 0.02, 0.12) < 400.0, "no glide from below");
        // A strummed chord: the late partial is silent before its delay.
        let chord = [
            Partial {
                f: 220.0,
                from: None,
                peak: 0.3,
                delay: 0.0,
            },
            Partial {
                f: 330.0,
                from: None,
                peak: 0.3,
                delay: 0.2,
            },
        ];
        let g = render_note(&Voice::mono(Wave::Guitar), &chord, shape, SR).unwrap();
        let alone = render_note(&Voice::mono(Wave::Guitar), &chord[..1], shape, SR).unwrap();
        let before = (0.19 * SR) as usize;
        assert_eq!(
            &g[..before],
            &alone[..before],
            "the second string sounds early"
        );
        assert_ne!(
            &g[before + 1000..before + 2000],
            &alone[before + 1000..before + 2000]
        );
    }

    #[test]
    fn the_filter_envelope_sub_and_non_computed_waves() {
        let shape = Shape {
            attack: 0.005,
            hold: 0.5,
            dur: 0.3,
        };
        let bright = one(Voice::mono(Wave::Guitar), 220.0, shape);
        let dark = one(
            Voice::mono(Wave::Guitar).with_filter(300.0, 300.0, 0.0, 0.0, 0.7),
            220.0,
            shape,
        );
        // A 300 Hz lowpass on a 220 Hz string: the fundamental stays, the
        // partials go — less energy, same pitch.
        assert!(rms(&dark, 0.05, 0.3) < rms(&bright, 0.05, 0.3));
        assert!((pitch_of(&dark, 0.05, 0.4) / 220.0 - 1.0).abs() < 0.015);
        let with_sub = one(Voice::mono(Wave::BassGuitar).with_sub(0.8), 82.4, shape);
        let without = one(Voice::mono(Wave::BassGuitar), 82.4, shape);
        assert!(rms(&with_sub, 0.1, 0.4) > rms(&without, 0.1, 0.4));
        assert!(render_note(&Voice::mono(Wave::Sawtooth), &[], shape, SR).is_none());
        assert!(render_note(&Voice::mono(Wave::Guitar), &[], shape, 0.0).is_none());
    }

    /// A big note (a strummed triad held 1.6 s, ringing 1.2 s more, with a
    /// filter envelope) renders in a few milliseconds: the bake happens on
    /// the main thread, one note per frame. (Debug builds are ~10× slower
    /// than release; the bound is for either.)
    #[test]
    fn a_big_note_renders_fast() {
        let shape = Shape {
            attack: 0.005,
            hold: 1.6,
            dur: 1.2,
        };
        let chord: Vec<Partial> = [220.0, 277.0, 330.0]
            .iter()
            .enumerate()
            .map(|(i, &f)| Partial {
                f,
                from: None,
                peak: 0.07,
                delay: i as f64 * STRUM_SECONDS,
            })
            .collect();
        let v = Voice::mono(Wave::Guitar).with_filter(300.0, 3000.0, 0.0, 0.2, 1.0);
        let t = std::time::Instant::now();
        let s = render_note(&v, &chord, shape, SR).unwrap();
        let ms = t.elapsed().as_secs_f64() * 1e3;
        println!("{} samples in {ms:.1} ms", s.len());
        assert!(ms < 150.0, "{ms:.1} ms for one note");
    }

    #[test]
    fn the_svf_is_stable_and_selective() {
        let mut svf = Svf::default();
        let mut lp_energy = 0.0;
        let coef = svf_coef(200.0, 0.7, SR);
        for i in 0..48_000 {
            let t = i as f64 / SR;
            let x = (std::f64::consts::TAU * 5000.0 * t).sin();
            let (lp, _) = svf.tick(x, coef);
            assert!(lp.is_finite() && lp.abs() < 2.0);
            if i > 24_000 {
                lp_energy += lp * lp;
            }
        }
        assert!(
            lp_energy / 24_000.0 < 0.01,
            "a 5 kHz tone leaks through a 200 Hz lowpass"
        );
        let mut svf = Svf::default();
        let coef = svf_coef(20_000.0, 30.0, SR);
        for _ in 0..1000 {
            let (lp, _) = svf.tick(1.0, coef);
            assert!(
                lp.is_finite() && lp.abs() < 100.0,
                "unstable at an extreme cutoff / q"
            );
        }
    }
}
