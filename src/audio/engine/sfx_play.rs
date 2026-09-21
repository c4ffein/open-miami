//! The one-shot SFX: each `play_*` (baked buffer when ready) and its live
//! `synth_*` recipe, stacked from the building blocks in `voices`.

use super::*;

impl AudioEngine {
    // --- one-shot SFX: attacks ---------------------------------------------
    //
    // The guns follow the measured profile of field-recorded gunshots
    // (.22LR / 5.56 / 7.62×39 / 7.62×54R singles; band energies, envelope
    // and decay times): a huge uncompressed bright crack (crest 18–22 dB,
    // centroid ~5–6 kHz at 5 ms, peak at +5–15 ms), a mid-dominant body
    // that holds a ~100 ms plateau at −5 dB then drops to −21 dB by 200 ms,
    // a low-mid layer a few dB under the mids, almost no sub, no growl, and
    // a long bright diffuse room tail (the reverb) carrying everything past
    // ~200 ms. Each shot is a [`RealShot`] recipe rendered by
    // [`Self::real_shot`] into a per-shot voice on the uncompressed bus, with
    // ±5 % pitch and ±3 ms timing jitter per play; mechanics (slide, bolt,
    // pump) follow.

    /// GUN attack — a 7.62×39 single: bright uncompressed crack,
    /// mid-dominant plateau body, the room, then the slide cycling.
    pub fn play_attack_gun(&self) {
        if !self.enabled.get() {
            return; // sound off: build NO nodes (the context is suspended anyway)
        }
        if self.play_baked(SfxKind::AttackGun) {
            return; // pre-rendered voice fired: 2–3 nodes instead of ~50
        }
        self.synth_attack_gun();
    }

    /// The live synthesis of [`Self::play_attack_gun`] — also what the
    /// offline pre-render runs (see [`Self::render_variant`]).
    pub(super) fn synth_attack_gun(&self) {
        let t = self.t0();
        let out = match self.voice_real(REAL_762X39.wet, 1.3) {
            Some(v) => v,
            None => return,
        };
        let j = self.jit(0.05);
        self.real_shot(&out, t, j, 1.0, &REAL_762X39);
        // Slide back / slide forward (bright, no low).
        let s1 = t + 0.08 + self.rand() * 0.012;
        let s2 = s1 + 0.055 + self.rand() * 0.012;
        self.tick(&out, s1, 2600.0 * j, 0.16);
        self.tick(&out, s2, 1900.0 * j, 0.13);
        if self.chance(0.35) {
            self.tinkle(&out, t + 0.30 + self.rand() * 0.10, 0.05);
        }
    }

    /// MACHINEGUN attack — ONE 5.56 round: a bright crack + plateau body
    /// with a bolt clack under it and the odd brass tinkle after. Called
    /// once PER BULLET (the game spawns a round every 0.1 s while the
    /// trigger is held), so sustained fire is exactly as many cracks as
    /// bullets and their room tails overlap into the burst wash live.
    pub fn play_attack_machinegun(&self) {
        if !self.enabled.get() {
            return; // sound off: build NO nodes (the context is suspended anyway)
        }
        if self.play_baked(SfxKind::AttackMachinegun) {
            return;
        }
        self.synth_attack_machinegun();
    }

    /// Live synthesis of [`Self::play_attack_machinegun`] (also pre-rendered).
    pub(super) fn synth_attack_machinegun(&self) {
        let t = self.t0();
        let out = match self.voice_real(REAL_556.wet, 1.3) {
            Some(v) => v,
            None => return,
        };
        let j = self.jit(0.05);
        self.real_shot(&out, t, j, 0.95, &REAL_556);
        // Bolt cycling under the round.
        self.tick(&out, t + 0.018, 1800.0 * j, 0.08);
        // Sometimes a spent case rings off the floor.
        if self.chance(0.35) {
            self.tinkle(&out, t + 0.12 + self.rand() * 0.05, 0.03);
        }
    }

    /// SHOTGUN attack — a 7.62×54R-sized single with a real
    /// low-mid body, the room, then a real pump reload: ~1 s of multiple
    /// bright 2–8 kHz clacks (pump back at +270 ms, forward, shell), no low.
    pub fn play_attack_shotgun(&self) {
        if !self.enabled.get() {
            return; // sound off: build NO nodes (the context is suspended anyway)
        }
        if self.play_baked(SfxKind::AttackShotgun) {
            return;
        }
        self.synth_attack_shotgun();
    }

    /// Live synthesis of [`Self::play_attack_shotgun`] (also pre-rendered).
    pub(super) fn synth_attack_shotgun(&self) {
        let t = self.t0();
        let out = match self.voice_real(REAL_762X54R.wet, 1.3) {
            Some(v) => v,
            None => return,
        };
        let j = self.jit(0.05);
        self.real_shot(&out, t, j, 1.0, &REAL_762X54R);
        self.real_pump(&out, t + 0.27 + self.rand() * 0.03, j);
    }

    /// CLUB attack — just the swing: a clean wind-like WHOOSH of air ripping
    /// past the bar. Bandpass noise that swells and sweeps up as the bar
    /// accelerates, then a closing, darker layer as it passes, an airy top
    /// and a faint low doppler-ish dip. No clang, no thud — the impact lives
    /// in [`Self::play_hit_club`]. Style-independent (the round-2 recipe,
    /// restored verbatim).
    pub fn play_attack_club(&self) {
        if !self.enabled.get() {
            return; // sound off: build NO nodes (the context is suspended anyway)
        }
        if self.play_baked(SfxKind::AttackClub) {
            return;
        }
        self.synth_attack_club();
    }

    /// Live synthesis of [`Self::play_attack_club`] (also pre-rendered).
    pub(super) fn synth_attack_club(&self) {
        let t = self.t0();
        let out = match self.voice(0.16, 1.0) {
            Some(v) => v,
            None => return,
        };
        let j = self.jit(0.06);
        // Opening whoosh: swells over ~90 ms while sweeping up in pitch.
        self.noise_env(
            &out,
            t,
            0.09,
            0.24,
            0.6,
            BiquadFilterType::Bandpass,
            320.0 * j,
            1500.0 * j,
            1.1,
        );
        // Closing tail: darker, sweeping back down as the bar passes.
        self.noise_env(
            &out,
            t + 0.10,
            0.04,
            0.20,
            0.42,
            BiquadFilterType::Bandpass,
            1400.0 * j,
            380.0 * j,
            1.0,
        );
        // Airy top layer.
        self.noise_env(
            &out,
            t + 0.02,
            0.07,
            0.20,
            0.16,
            BiquadFilterType::Highpass,
            1800.0,
            3200.0,
            0.7,
        );
        // Wind body: low-passed rush under the whoosh.
        self.noise_env(
            &out,
            t + 0.01,
            0.09,
            0.24,
            0.4,
            BiquadFilterType::Lowpass,
            520.0 * j,
            240.0,
            0.9,
        );
        // Faint doppler-ish pitch dip as the bar goes by.
        self.tone_out(
            &out,
            135.0 * j,
            88.0 * j,
            t + 0.06,
            0.20,
            0.07,
            0.06,
            OscillatorType::Sine,
        );
    }

    // --- one-shot SFX: hits (impact on a metal bot) ------------------------
    //
    // Analysis-driven resynthesis of a reference bullet-on-metal recording
    // (spectral modeling: the measured sine partials + noise bands +
    // transients in [`sms_tables`], replayed by [`Self::sms_play`]), on the
    // uncompressed bus with the bright room, ±5 % pitch per play. The bigger
    // hits stack / slow / lower the same model and add pellet debris, a bat
    // contact or a hollow knock on top.

    /// GUN hit — analysis-driven resynthesis of the reference
    /// recording BulletImpactMetal02 (see [`sms_tables`]): the 16 measured
    /// partials + 9 noise bands replayed through [`Self::sms_play`] on the
    /// real bus (wet 0.5, drive 1.3), ±5 % pitch; nothing else on top but
    /// the 40 % quiet ricochet.
    pub fn play_hit_gun(&self) {
        if !self.enabled.get() {
            return; // sound off: build NO nodes (the context is suspended anyway)
        }
        if self.play_baked(SfxKind::HitGun) {
            return;
        }
        self.synth_hit_gun();
    }

    /// Live synthesis of [`Self::play_hit_gun`] (also pre-rendered).
    pub(super) fn synth_hit_gun(&self) {
        let t = self.t0();
        let out = match self.voice_real(0.5, 1.3) {
            Some(v) => v,
            None => return,
        };
        let pitch = self.jit(0.05) as f32;
        self.sms_metal02(&out, t, SMS_HIT_GAIN, pitch, 1.0);
        if self.chance(0.4) {
            self.real_ricochet(&out, t + 0.008, pitch as f64, 0.35);
        }
    }

    /// MACHINEGUN hit — the METAL02 resynthesis per round at the
    /// burst rate, level 0.75, ±8 % pitch, a ricochet on about one round in
    /// three.
    pub fn play_hit_machinegun(&self) {
        if !self.enabled.get() {
            return; // sound off: build NO nodes (the context is suspended anyway)
        }
        if self.play_baked(SfxKind::HitMachinegun) {
            return;
        }
        self.synth_hit_machinegun();
    }

    /// Live synthesis of [`Self::play_hit_machinegun`] (also pre-rendered).
    pub(super) fn synth_hit_machinegun(&self) {
        let t = self.t0();
        let out = match self.voice_real(0.5, 1.3) {
            Some(v) => v,
            None => return,
        };
        let rounds = 8;
        let spacing = 0.058;
        let mut at = t;
        for _ in 0..rounds {
            let pitch = self.jit(0.08) as f32;
            self.sms_metal02(&out, at, SMS_HIT_GAIN * 0.75, pitch, 1.0);
            if self.chance(0.33) {
                self.real_ricochet(&out, at + 0.008, pitch as f64, 0.25);
            }
            at += spacing * self.jit(0.05);
        }
    }

    /// SHOTGUN hit — 3–4 overlapping METAL02 plays spread over
    /// 25 ms at pitch 0.85–1.0 (a bigger plate) plus the pellet debris ticks.
    pub fn play_hit_shotgun(&self) {
        if !self.enabled.get() {
            return; // sound off: build NO nodes (the context is suspended anyway)
        }
        if self.play_baked(SfxKind::HitShotgun) {
            return;
        }
        self.synth_hit_shotgun();
    }

    /// Live synthesis of [`Self::play_hit_shotgun`] (also pre-rendered).
    pub(super) fn synth_hit_shotgun(&self) {
        let t = self.t0();
        let out = match self.voice_real(0.55, 1.3) {
            Some(v) => v,
            None => return,
        };
        let plays = 3 + (self.rand() * 2.0) as usize;
        for k in 0..plays {
            let at = if k == 0 { t } else { t + self.rand() * 0.025 };
            let pitch = (0.85 + self.rand() * 0.15) as f32;
            let lvl = if k == 0 { 1.0 } else { 0.7 };
            self.sms_metal02(&out, at, SMS_HIT_GAIN * 0.8 * lvl, pitch, 1.0);
        }
        // Pellet debris: sparse bright ticks over the first ~120 ms.
        for _ in 0..8 {
            let at = t + 0.004 + self.rand() * 0.12;
            let hz = 4000.0 + self.rand() * 6000.0;
            self.noise_full(
                &out,
                at,
                0.0,
                0.001,
                0.001,
                0.5 * (0.3 + self.rand() * 0.7),
                BiquadFilterType::Bandpass,
                hz,
                hz,
                1.0,
            );
        }
    }

    /// CLUB hit — one METAL02 play at pitch 0.7–0.8 and time
    /// scale 1.25 (a bigger, slower body) with the bat contact tick and a
    /// hollow 130 Hz body knock kept on top.
    pub fn play_hit_club(&self) {
        if !self.enabled.get() {
            return; // sound off: build NO nodes (the context is suspended anyway)
        }
        if self.play_baked(SfxKind::HitClub) {
            return;
        }
        self.synth_hit_club();
    }

    /// Live synthesis of [`Self::play_hit_club`] (also pre-rendered).
    pub(super) fn synth_hit_club(&self) {
        let t = self.t0();
        let out = match self.voice_real(0.5, 1.3) {
            Some(v) => v,
            None => return,
        };
        let j = self.jit(0.05);
        // Bat contact: 2–3 bounces ~8 ms apart.
        let mut at = t;
        for k in 0..3 {
            if k > 0 {
                at += 0.008 * (0.5 + self.rand());
            }
            let amp = if k == 0 { 1.0 } else { 0.4 + self.rand() * 0.6 };
            self.click(&out, at, 0.28 * amp);
            self.noise_full(
                &out,
                at,
                0.0,
                0.0025,
                0.0025,
                0.35 * amp,
                BiquadFilterType::Bandpass,
                5000.0 * j,
                4000.0 * j,
                0.6,
            );
        }
        let pitch = (0.7 + self.rand() * 0.1) as f32;
        self.sms_metal02(&out, t + 0.002, SMS_HIT_GAIN * 1.1, pitch, 1.25);
        self.hollow_knock(&out, t + 0.004, 130.0 * j, 0.08, 0.6);
    }

    /// A rogue AI goes down — a metal robot collapsing — a quiet dying servo, then
    /// three falling METAL02 plays (pitch 0.9 / 0.8 / 0.7, time scale 1.2)
    /// with loose-part rattle between and after, ending on a low rumble.
    pub fn play_enemy_down(&self) {
        if !self.enabled.get() {
            return; // sound off: build NO nodes (the context is suspended anyway)
        }
        if self.play_baked(SfxKind::EnemyDown) {
            return;
        }
        self.synth_enemy_down();
    }

    /// Live synthesis of [`Self::play_enemy_down`] (also pre-rendered).
    pub(super) fn synth_enemy_down(&self) {
        let t = self.t0();
        let out = match self.voice_real(0.55, 1.3) {
            Some(v) => v,
            None => return,
        };
        let j = self.jit(0.05);
        // Small servo whine sagging (quiet).
        self.swell_tone(
            &out,
            640.0 * j,
            420.0 * j,
            t,
            0.01,
            0.06,
            0.10,
            OscillatorType::Sawtooth,
            0.05,
        );
        // Falling: knee, hip, then the whole hull.
        let c1 = t + 0.05 + self.rand() * 0.03;
        let c2 = c1 + 0.14 + self.rand() * 0.05;
        let c3 = c2 + 0.16 + self.rand() * 0.06;
        let pj = j as f32;
        self.sms_metal02(&out, c1, SMS_HIT_GAIN * 0.6, 0.9 * pj, 1.2);
        self.hollow_knock(&out, c1 + 0.004, 120.0 * j, 0.08, 0.35);
        self.rattle(&out, c1 + 0.04, c2 - 0.01, 0.35 * j);
        self.sms_metal02(&out, c2, SMS_HIT_GAIN * 0.8, 0.8 * pj, 1.2);
        self.hollow_knock(&out, c2 + 0.004, 110.0 * j, 0.08, 0.45);
        self.rattle(&out, c2 + 0.04, c3 - 0.01, 0.4 * j);
        self.sms_metal02(&out, c3, SMS_HIT_GAIN, 0.7 * pj, 1.2);
        self.hollow_knock(&out, c3 + 0.004, 100.0 * j, 0.09, 0.6);
        // Loose parts rattling after, and the low rumble tail.
        self.rattle(&out, c3 + 0.06, c3 + 0.35, 0.3 * j);
        self.noise_full(
            &out,
            c3 + 0.02,
            0.08,
            0.65,
            0.65,
            0.4,
            BiquadFilterType::Bandpass,
            240.0 * j,
            150.0 * j,
            0.9,
        );
    }

    /// A hollow body knock: a short sine (`hz`, gliding down 10 %) plus a
    /// low-passed thump, `len` seconds, at `level`.
    pub(super) fn hollow_knock(
        &self,
        out: &webaudio::AudioNode,
        t: f64,
        hz: f64,
        len: f64,
        level: f64,
    ) {
        let f = hz * self.jit(0.05);
        self.swell_tone(
            out,
            f,
            f * 0.9,
            self.jt(t, 0.002),
            0.004,
            len * 0.25,
            level,
            OscillatorType::Sine,
            0.02,
        );
        self.noise_full(
            out,
            self.jt(t, 0.002),
            0.0,
            len,
            len,
            level * 0.8,
            BiquadFilterType::Lowpass,
            f * 2.2,
            f * 1.2,
            1.0,
        );
    }

    /// The METAL02 model through [`Self::sms_play`].
    pub(super) fn sms_metal02(
        &self,
        out: &webaudio::AudioNode,
        t0: f64,
        gain: f32,
        pitch: f32,
        time_scale: f32,
    ) {
        self.sms_play(
            out,
            t0,
            sms_tables::METAL02_HOP,
            sms_tables::METAL02_PARTIALS,
            sms_tables::METAL02_NOISE,
            sms_tables::METAL02_TRANSIENTS,
            gain,
            pitch,
            time_scale,
        );
    }

    /// The empirical trim for an SMS noise band centred on `fc` (see
    /// [`SMS_BAND_TRIMS`]).
    pub(super) fn sms_band_trim(fc: f64) -> f64 {
        SMS_BAND_TRIMS
            .iter()
            .find(|(edge, _)| fc < *edge)
            .map(|(_, k)| *k)
            .unwrap_or(1.0)
    }

    /// Generic spectral-modeling (sines + noise) player. Replays an analysed
    /// sound from its tables: each partial is a sine oscillator at
    /// `freq × pitch` (±0.4 % per-play detune) whose gain follows the
    /// partial's amplitude curve (`N` points every `hop × time_scale`
    /// seconds, via `setValueCurveAtTime`); each noise band is the looped
    /// noise buffer through a bandpass at `center × pitch` with
    /// `Q = sqrt(r) / (r − 1)` (`r` = hi/lo edge ratio, clamped 0.5–4),
    /// whose gain follows the band's curve, normalised so the band's RMS
    /// matches the curve (bandpass output RMS ≈ 0.577·sqrt(fc/Q/(sr/2)) for
    /// the ±1 uniform noise buffer, then the empirical `SMS_BAND_TRIMS`);
    /// partials above `SMS_HIGH_PARTIAL_HZ` get `SMS_HIGH_PARTIAL_GAIN`;
    /// each transient `(t, amp)` is a 0.5 ms click through a random
    /// 4–12 kHz Q 1 bandpass at `t0 + t × time_scale`, level
    /// `amp × gain × 0.6`. `gain` scales everything.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn sms_play<const N: usize>(
        &self,
        out: &webaudio::AudioNode,
        t0: f64,
        hop: f32,
        partials: &[(f32, [f32; N])],
        noise: &[(f32, f32, [f32; N])],
        transients: &[(f32, f32)],
        gain: f32,
        pitch: f32,
        time_scale: f32,
    ) {
        let (ctx, buf) = match (self.bctx(), &self.noise) {
            (Some(c), Some(b)) => (c, b),
            _ => return,
        };
        if N == 0 {
            return;
        }
        let dur = (N as f64) * (hop as f64) * (time_scale as f64).max(0.05);
        let sr = ctx.sample_rate() as f64;
        let nyq = sr * 0.5;
        // Partials.
        for (freq, table) in partials {
            let f = (*freq as f64) * (pitch as f64) * self.jit(0.004);
            if f < 20.0 || f > nyq * 0.95 {
                continue;
            }
            let (osc, g) = match (ctx.create_oscillator(), ctx.create_gain()) {
                (Ok(o), Ok(g)) => (o, g),
                _ => continue,
            };
            osc.set_type(OscillatorType::Sine);
            let _ = osc.frequency().set_value_at_time(f as f32, t0);
            let pg = if f > SMS_HIGH_PARTIAL_HZ {
                gain * SMS_HIGH_PARTIAL_GAIN
            } else {
                gain
            };
            let mut curve: Vec<f32> = table.iter().map(|a| a * pg).collect();
            let _ = g.gain().set_value_curve_at_time(&mut curve, t0, dur);
            let _ = osc.connect_with_audio_node(&g);
            let _ = g.connect_with_audio_node(out);
            let sched: &webaudio::AudioScheduledSourceNode = osc.as_ref();
            let _ = sched.start_with_when(t0);
            let _ = sched.stop_with_when(t0 + dur + 0.02);
        }
        // Noise bands.
        for (center, ratio, table) in noise {
            let fc = (*center as f64) * (pitch as f64);
            if fc < 15.0 || fc > nyq * 0.95 {
                continue;
            }
            let r = (*ratio as f64).max(1.01);
            let q = (r.sqrt() / (r - 1.0)).clamp(0.5, 4.0);
            let (src, filt, g) = match (
                ctx.create_buffer_source(),
                ctx.create_biquad_filter(),
                ctx.create_gain(),
            ) {
                (Ok(s), Ok(f), Ok(g)) => (s, f, g),
                _ => continue,
            };
            src.set_buffer(Some(buf));
            src.set_loop(true);
            filt.set_type(BiquadFilterType::Bandpass);
            let _ = filt.frequency().set_value_at_time(fc as f32, t0);
            let _ = filt.q().set_value_at_time(q as f32, t0);
            // RMS normalisation of the band-passed uniform noise.
            let rms = 0.577 * (fc / q / nyq).min(1.0).sqrt();
            let norm = (SMS_NOISE_TRIM * Self::sms_band_trim(fc) / rms.max(1e-4)) as f32;
            let mut curve: Vec<f32> = table.iter().map(|a| a * gain * norm).collect();
            let _ = g.gain().set_value_curve_at_time(&mut curve, t0, dur);
            let _ = src.connect_with_audio_node(&filt);
            let _ = filt.connect_with_audio_node(&g);
            let _ = g.connect_with_audio_node(out);
            let sched: &webaudio::AudioScheduledSourceNode = src.as_ref();
            let offset = self.rand() * (NOISE_SECONDS - 0.05);
            let _ = src.start_with_when_and_grain_offset(t0, offset);
            let _ = sched.stop_with_when(t0 + dur + 0.02);
        }
        // Transients.
        for (tt, amp) in transients {
            let at = t0 + (*tt as f64) * (time_scale as f64);
            let hz = 4000.0 + self.rand() * 8000.0;
            self.noise_full(
                out,
                at,
                0.0,
                0.0005,
                0.0005,
                (*amp as f64) * (gain as f64) * 0.6,
                BiquadFilterType::Bandpass,
                hz,
                hz,
                1.0,
            );
        }
    }

    /// Loose-part rattle debris between `from` and `to`: a handful of small
    /// bright ticks and micro-clicks at random times, quiet.
    pub(super) fn rattle(&self, out: &webaudio::AudioNode, from: f64, to: f64, level: f64) {
        let span = (to - from).max(0.02);
        let n = 3 + (self.rand() * 4.0) as usize;
        for _ in 0..n {
            let at = from + self.rand() * span;
            let g = level * (0.3 + self.rand() * 0.7);
            if self.chance(0.5) {
                self.tick(out, at, 700.0 + self.rand() * 900.0, g * 0.25);
            } else {
                let hz = 3500.0 + self.rand() * 5000.0;
                self.noise_full(
                    out,
                    at,
                    0.0,
                    0.003,
                    0.003,
                    g,
                    BiquadFilterType::Bandpass,
                    hz,
                    hz,
                    1.0,
                );
            }
        }
    }

    // --- one-shot SFX: non-combat ------------------------------------------

    /// Bright rising two-tone — weapon pickup / swap.
    pub fn play_pickup(&self) {
        if !self.enabled.get() {
            return; // sound off: build NO nodes (the context is suspended anyway)
        }
        if self.play_baked(SfxKind::Pickup) {
            return;
        }
        self.synth_pickup();
    }

    /// Live synthesis of [`Self::play_pickup`] (also pre-rendered).
    pub(super) fn synth_pickup(&self) {
        let t = self.t0();
        self.tone(523.25, 523.25, t, 0.08, 0.20, OscillatorType::Triangle);
        self.tone(
            783.99,
            783.99,
            t + 0.07,
            0.12,
            0.22,
            OscillatorType::Triangle,
        );
    }

    /// Filtered noise whoosh — a thrown weapon.
    pub fn play_throw(&self) {
        if !self.enabled.get() {
            return; // sound off: build NO nodes (the context is suspended anyway)
        }
        if self.play_baked(SfxKind::Throw) {
            return;
        }
        self.synth_throw();
    }

    /// Live synthesis of [`Self::play_throw`] (also pre-rendered).
    pub(super) fn synth_throw(&self) {
        let t = self.t0();
        self.noise(t, 0.22, 0.22, BiquadFilterType::Highpass, 200.0, 1600.0);
    }

    /// The player takes a hit — mean: a hard body blow (click, low-passed
    /// slam, a big 130 → 45 Hz thump, a crunch band) driven hot, with a
    /// loud, longer clipped grunt.
    pub fn play_player_hurt(&self) {
        if !self.enabled.get() {
            return; // sound off: build NO nodes (the context is suspended anyway)
        }
        if self.play_baked(SfxKind::PlayerHurt) {
            return;
        }
        self.synth_player_hurt();
    }

    /// Live synthesis of [`Self::play_player_hurt`] (also pre-rendered).
    pub(super) fn synth_player_hurt(&self) {
        let t = self.t0();
        let out = match self.voice(0.30, 4.0) {
            Some(v) => v,
            None => return,
        };
        let j = self.jit(0.05);
        self.click(&out, t, 1.0);
        self.noise_env(
            &out,
            t,
            0.0,
            0.12,
            1.4,
            BiquadFilterType::Lowpass,
            900.0,
            100.0,
            0.8,
        );
        self.noise_env(
            &out,
            t,
            0.0,
            0.08,
            0.9,
            BiquadFilterType::Bandpass,
            600.0 * j,
            200.0,
            0.7,
        );
        self.wham(&out, t, 1.3);
        // Grunt: a rough low buzz through a swept vowel-ish bandpass.
        self.grunt(&out, t + 0.015, 128.0 * j, 0.20, 1.0);
    }

    /// Longer downward dive — the player dies / SYSTEM HALTED.
    pub fn play_death(&self) {
        if !self.enabled.get() {
            return; // sound off: build NO nodes (the context is suspended anyway)
        }
        if self.play_baked(SfxKind::Death) {
            return;
        }
        self.synth_death();
    }

    /// Live synthesis of [`Self::play_death`] (also pre-rendered).
    pub(super) fn synth_death(&self) {
        let t = self.t0();
        self.tone(420.0, 40.0, t, 0.65, 0.32, OscillatorType::Sawtooth);
        self.noise(t, 0.60, 0.18, BiquadFilterType::Lowpass, 1800.0, 120.0);
    }

    /// Short triumphant arp — SECTOR PURGED.
    pub fn play_level_clear(&self) {
        if !self.enabled.get() {
            return; // sound off: build NO nodes (the context is suspended anyway)
        }
        if self.play_baked(SfxKind::LevelClear) {
            return;
        }
        self.synth_level_clear();
    }

    /// Live synthesis of [`Self::play_level_clear`] (also pre-rendered).
    pub(super) fn synth_level_clear(&self) {
        let t = self.t0();
        let notes = [523.25, 659.25, 783.99, 1046.50];
        for (i, f) in notes.iter().enumerate() {
            let at = t + i as f64 * 0.09;
            self.tone(*f, *f, at, 0.14, 0.20, OscillatorType::Square);
        }
    }

    /// Nasty shattering noise burst — a boss's mask breaks. A special hit.
    pub fn play_mask_crack(&self) {
        if !self.enabled.get() {
            return; // sound off: build NO nodes (the context is suspended anyway)
        }
        if self.play_baked(SfxKind::MaskCrack) {
            return;
        }
        self.synth_mask_crack();
    }

    /// Live synthesis of [`Self::play_mask_crack`] (also pre-rendered).
    pub(super) fn synth_mask_crack(&self) {
        let t = self.t0();
        self.noise(t, 0.25, 0.40, BiquadFilterType::Highpass, 6000.0, 800.0);
        self.tone(300.0, 90.0, t, 0.18, 0.22, OscillatorType::Square);
        self.tone(
            1700.0,
            400.0,
            t + 0.03,
            0.10,
            0.18,
            OscillatorType::Sawtooth,
        );
    }

    /// A rising, ominous elevator ding — the doors close and the floor drops.
    /// A swelling detuned drone climbs to a pair of bright bell dings.
    pub fn play_elevator(&self) {
        if !self.enabled.get() {
            return; // sound off: build NO nodes (the context is suspended anyway)
        }
        if self.play_baked(SfxKind::Elevator) {
            return;
        }
        self.synth_elevator();
    }

    /// Live synthesis of [`Self::play_elevator`] (also pre-rendered).
    pub(super) fn synth_elevator(&self) {
        let t = self.t0();
        // Slow ominous swell rising a fifth.
        self.tone(110.0, 165.0, t, 0.95, 0.16, OscillatorType::Sawtooth);
        self.tone(110.6, 166.5, t, 0.95, 0.10, OscillatorType::Triangle);
        // Airy noise rising underneath the swell.
        self.noise(t, 0.9, 0.05, BiquadFilterType::Highpass, 400.0, 3000.0);
        // The "ding" at the top — two chiming sines a fifth apart.
        self.tone(880.0, 880.0, t + 0.72, 0.5, 0.18, OscillatorType::Sine);
        self.tone(1318.5, 1318.5, t + 0.78, 0.45, 0.11, OscillatorType::Sine);
    }

    // --- car SFX -----------------------------------------------------------

    /// The offline build of the ENGINE IDLE loop buffer (never played as a
    /// one-shot — see [`Self::start_engine_idle`]). A smooth low motor:
    /// two barely-detuned sawtooths (their 0.25 Hz beat is the breathing)
    /// and a quiet pulse an octave up, all through one dark lowpass; a sine
    /// subharmonic under-thump; a slow sine LFO wobbling the saws' pitch
    /// and another breathing the master gain. Everything starts at t = 0
    /// with constant levels (no envelopes — a loop must not pump) and every
    /// periodic component completes whole cycles over the loop region, so
    /// the buffer wraps seamlessly (see the car SFX tunables).
    ///
    /// Variants differ only in layer BALANCE and wobble depth (loop-safe);
    /// the locked frequencies are never jittered — that would break the
    /// whole-cycle wrap.
    pub(super) fn synth_engine_idle(&self) {
        let ctx = match self.bctx() {
            Some(c) => c,
            None => return,
        };
        let out = match self.sfx_out() {
            Some(o) => o,
            None => return,
        };
        let t = self.now(); // 0.0 offline: the warm-up head starts the buffer
        let len = ENGINE_LOOP_WARMUP + ENGINE_LOOP_SECONDS;
        // Master gain (breathed by the amp LFO) into the sink.
        let master = match ctx.create_gain() {
            Ok(g) => g,
            Err(_) => return,
        };
        let _ = master.gain().set_value_at_time(1.0, t);
        let _ = master.connect_with_audio_node(&out);
        let mout: &webaudio::AudioNode = master.as_ref();
        // The shared dark lowpass for the buzzy layers.
        let lp = match ctx.create_biquad_filter() {
            Ok(f) => f,
            Err(_) => return,
        };
        lp.set_type(BiquadFilterType::Lowpass);
        let _ = lp.frequency().set_value_at_time(230.0, t);
        let _ = lp.q().set_value_at_time(0.9, t);
        let _ = lp.connect_with_audio_node(mout);
        let lout: &webaudio::AudioNode = lp.as_ref();
        // One flat-gain oscillator layer; returns the oscillator so the
        // LFOs can be wired to the pitched ones.
        let layer = |wave: OscillatorType, f: f64, level: f64, dest: &webaudio::AudioNode| {
            let (osc, g) = match (ctx.create_oscillator(), ctx.create_gain()) {
                (Ok(o), Ok(g)) => (o, g),
                _ => return None,
            };
            osc.set_type(wave);
            let _ = osc.frequency().set_value_at_time(f as f32, t);
            let _ = g.gain().set_value_at_time(level as f32, t);
            let _ = osc.connect_with_audio_node(&g);
            let _ = g.connect_with_audio_node(dest);
            let sched: &webaudio::AudioScheduledSourceNode = osc.as_ref();
            let _ = sched.start_with_when(t);
            let _ = sched.stop_with_when(t + len);
            Some(osc)
        };
        let saw1 = layer(
            OscillatorType::Sawtooth,
            ENGINE_F0,
            0.50 * self.jit(0.15),
            lout,
        );
        let saw2 = layer(
            OscillatorType::Sawtooth,
            ENGINE_F0_DETUNED,
            0.34 * self.jit(0.15),
            lout,
        );
        let _ = layer(
            OscillatorType::Square,
            ENGINE_PULSE_F,
            0.11 * self.jit(0.2),
            lout,
        );
        // The subharmonic sine bypasses the filter (already pure).
        let _ = layer(
            OscillatorType::Sine,
            ENGINE_SUB_F,
            0.55 * self.jit(0.1),
            mout,
        );
        // Pitch LFO → the saws' frequency params (whole cycles per loop:
        // zero net phase added, the wrap stays exact).
        if let (Ok(lfo), Ok(depth)) = (ctx.create_oscillator(), ctx.create_gain()) {
            lfo.set_type(OscillatorType::Sine);
            let _ = lfo
                .frequency()
                .set_value_at_time(ENGINE_PITCH_LFO_HZ as f32, t);
            let _ = depth
                .gain()
                .set_value_at_time((ENGINE_PITCH_LFO_DEPTH * self.jit(0.3)) as f32, t);
            let _ = lfo.connect_with_audio_node(&depth);
            for o in [&saw1, &saw2].into_iter().flatten() {
                let _ = depth.connect_with_audio_param(&o.frequency());
            }
            let sched: &webaudio::AudioScheduledSourceNode = lfo.as_ref();
            let _ = sched.start_with_when(t);
            let _ = sched.stop_with_when(t + len);
        }
        // Amp LFO → the master gain param (base 1.0 ± depth).
        if let (Ok(lfo), Ok(depth)) = (ctx.create_oscillator(), ctx.create_gain()) {
            lfo.set_type(OscillatorType::Sine);
            let _ = lfo
                .frequency()
                .set_value_at_time(ENGINE_AMP_LFO_HZ as f32, t);
            let _ = depth
                .gain()
                .set_value_at_time((0.12 * self.jit(0.3)) as f32, t);
            let _ = lfo.connect_with_audio_node(&depth);
            let _ = depth.connect_with_audio_param(&master.gain());
            let sched: &webaudio::AudioScheduledSourceNode = lfo.as_ref();
            let _ = sched.start_with_when(t);
            let _ = sched.stop_with_when(t + len);
        }
    }

    /// Start the looping ENGINE IDLE under the title menu. Baked-only:
    /// until its variants are rendered this is a silent no-op (a per-frame
    /// caller simply retries — the loop has no live fallback, per-frame
    /// graph construction being exactly what the bake system avoids).
    /// Idempotent while running. Plays the loop region (`loop_start` =
    /// after the warm-up head) through a low gain into the room voice,
    /// eased in so returning to the title never bumps.
    pub fn start_engine_idle(&self) {
        if !self.enabled.get() || self.engine_idle.borrow().is_some() {
            return;
        }
        let ctx = match &self.ctx {
            Some(c) => c,
            None => return,
        };
        let bufs = self.baked.bufs.borrow();
        let set = &bufs[SfxKind::EngineIdle as usize];
        if set.len() < SFX_VARIANTS {
            return; // bake not ready: skip silently (same bar as play_baked)
        }
        let out = match self.sfx_out() {
            Some(o) => o,
            None => return,
        };
        let (src, gain) = match (ctx.create_buffer_source(), ctx.create_gain()) {
            (Ok(s), Ok(g)) => (s, g),
            _ => return,
        };
        let variant = ((self.rand() * SFX_VARIANTS as f64) as usize).min(SFX_VARIANTS - 1);
        src.set_buffer(Some(&set[variant]));
        src.set_loop(true);
        src.set_loop_start(ENGINE_LOOP_WARMUP);
        src.set_loop_end(ENGINE_LOOP_WARMUP + ENGINE_LOOP_SECONDS);
        // A tiny per-start transposition (pure rate change: the wrap stays
        // seamless) so the idle never sits on the exact same pitch twice.
        let _ = src
            .playback_rate()
            .set_value_at_time(self.jit(0.03) as f32, 0.0);
        let now = self.now();
        let g = gain.gain();
        let _ = g.set_value_at_time(0.0001, now);
        let _ = g.linear_ramp_to_value_at_time(ENGINE_IDLE_GAIN as f32, now + 0.6);
        let _ = src.connect_with_audio_node(AsRef::<webaudio::AudioNode>::as_ref(&gain));
        let _ = gain.connect_with_audio_node(&out);
        // Start inside the loop region, skipping the warm-up head.
        let _ = src.start_with_when_and_grain_offset(now, ENGINE_LOOP_WARMUP);
        *self.engine_idle.borrow_mut() = Some((src, gain));
    }

    /// Stop the engine idle with a short fade (no-op when not running).
    /// No `enabled` guard: a stop must always land, even with sound off.
    pub fn stop_engine_idle(&self) {
        let Some((src, gain)) = self.engine_idle.borrow_mut().take() else {
            return;
        };
        let now = self.now();
        let g = gain.gain();
        let _ = g.cancel_scheduled_values(now);
        let _ = g.set_value_at_time(g.value(), now);
        let _ = g.linear_ramp_to_value_at_time(0.0001, now + 0.15);
        let sched: &webaudio::AudioScheduledSourceNode = src.as_ref();
        let _ = sched.stop_with_when(now + 0.2);
    }

    /// TIRE SCREECH — a locked-wheel skid, ~0.9 s.
    pub fn play_tire_screech(&self) {
        if !self.enabled.get() {
            return; // sound off: build NO nodes (the context is suspended anyway)
        }
        if self.play_baked(SfxKind::TireScreech) {
            return;
        }
        self.synth_tire_screech();
    }

    /// Live synthesis of [`Self::play_tire_screech`] (also pre-rendered):
    /// a high-Q bandpass noise squeal whining DOWN as the car scrubs
    /// speed, a weaker inharmonic upper squeal mode, a rubber-on-asphalt
    /// rumble underneath and a faint bright broadband grit layer.
    pub(super) fn synth_tire_screech(&self) {
        let t = self.t0();
        let out = match self.voice(0.25, 1.4) {
            Some(v) => v,
            None => return,
        };
        let j = self.jit(0.06);
        // The main squeal: resonant bandpass noise gliding ~2.1k → 850 Hz.
        self.noise_full(
            &out,
            t,
            0.03,
            0.9,
            0.9,
            0.55,
            BiquadFilterType::Bandpass,
            2100.0 * j,
            850.0 * j,
            14.0,
        );
        // A weaker upper mode, out of tune with the first (a real screech
        // carries several inharmonic squeal resonances).
        self.noise_full(
            &out,
            t + 0.02,
            0.02,
            0.7,
            0.7,
            0.30,
            BiquadFilterType::Bandpass,
            3150.0 * j,
            1300.0 * j,
            9.0,
        );
        // Rubber-on-asphalt rumble under it.
        self.noise_env(
            &out,
            t,
            0.01,
            0.75,
            0.22,
            BiquadFilterType::Lowpass,
            520.0,
            240.0,
            1.0,
        );
        // Slight grit: a faint bright broadband layer over the squeal.
        self.noise_env(
            &out,
            t,
            0.0,
            0.45,
            0.10,
            BiquadFilterType::Highpass,
            3800.0,
            2600.0,
            0.7,
        );
    }

    /// CAR DOOR OPEN — latch click + brief hinge creak, short.
    pub fn play_car_door_open(&self) {
        if !self.enabled.get() {
            return; // sound off: build NO nodes (the context is suspended anyway)
        }
        if self.play_baked(SfxKind::CarDoorOpen) {
            return;
        }
        self.synth_car_door_open();
    }

    /// Live synthesis of [`Self::play_car_door_open`] (also pre-rendered):
    /// the latch release (click + two metallic ticks), then two stick-slip
    /// hinge creak segments — narrow resonant noise sweeping UP — and a
    /// whiff of cabin air.
    pub(super) fn synth_car_door_open(&self) {
        let t = self.t0();
        let out = match self.sfx_out() {
            Some(o) => o,
            None => return,
        };
        let j = self.jit(0.06);
        // Latch: the release click, then the handle springing back.
        self.click(&out, t, 0.7);
        self.tick(&out, t + 0.004, 1750.0 * j, 0.22);
        self.tick(&out, t + 0.052, 1150.0 * j, 0.13);
        // Hinge creak: two stick-slip squeaks sweeping up.
        self.noise_full(
            &out,
            t + 0.07,
            0.02,
            0.16,
            0.16,
            0.16,
            BiquadFilterType::Bandpass,
            640.0 * j,
            1350.0 * j,
            10.0,
        );
        self.noise_full(
            &out,
            t + 0.19,
            0.015,
            0.12,
            0.12,
            0.11,
            BiquadFilterType::Bandpass,
            900.0 * j,
            1700.0 * j,
            10.0,
        );
        // A whiff of cabin air as the seal breaks.
        self.noise_env(
            &out,
            t + 0.06,
            0.03,
            0.22,
            0.05,
            BiquadFilterType::Highpass,
            1800.0,
            2600.0,
            0.7,
        );
    }

    /// CAR DOOR CLOSE — the classic thunk, short.
    pub fn play_car_door_close(&self) {
        if !self.enabled.get() {
            return; // sound off: build NO nodes (the context is suspended anyway)
        }
        if self.play_baked(SfxKind::CarDoorClose) {
            return;
        }
        self.synth_car_door_close();
    }

    /// Live synthesis of [`Self::play_car_door_close`] (also pre-rendered):
    /// a click + low-passed slap front, the latch snapping shut, a low
    /// sine drop and the multi-partial `wham` body, then a quick hollow
    /// body-shell resonance tail — all glued by the voice's soft clip.
    pub(super) fn synth_car_door_close(&self) {
        let t = self.t0();
        let out = match self.voice(0.22, 2.2) {
            Some(v) => v,
            None => return,
        };
        let j = self.jit(0.05);
        // The slam front: pressure click + a dark slap.
        self.click(&out, t, 0.9);
        self.noise_env(
            &out,
            t,
            0.0,
            0.06,
            0.8,
            BiquadFilterType::Lowpass,
            2200.0,
            350.0,
            0.8,
        );
        // The latch snapping shut on top.
        self.tick(&out, t + 0.010, 2300.0 * j, 0.14);
        // The body: a low sine/triangle-ish drop plus the heavy wham.
        self.tone_out(
            &out,
            95.0 * j,
            52.0,
            t + 0.004,
            0.16,
            0.55,
            0.003,
            OscillatorType::Sine,
        );
        self.wham(&out, t + 0.002, 0.5);
        // Quick hollow body-resonance tail (the door panel ringing out).
        self.noise_env(
            &out,
            t + 0.02,
            0.0,
            0.28,
            0.22,
            BiquadFilterType::Bandpass,
            190.0 * j,
            130.0,
            3.0,
        );
    }
}
