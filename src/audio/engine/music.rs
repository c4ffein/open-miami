//! The tracker: transport + introspection API, the look-ahead scheduler
//! (`update`), note voices (incl. the supersaw / driven / darkpad presets) and drums.

use super::*;

impl AudioEngine {
    // --- music -------------------------------------------------------------

    /// Begin the looping backing track using the current song (idempotent).
    /// Sound off is not a reason to refuse: `update` idles while disabled and
    /// catches up when the SETTINGS toggle re-enables it, so a floor entered
    /// with sound off still gets its music the moment sound comes back.
    pub fn start_music(&mut self) {
        if self.music_playing {
            return;
        }
        self.music_playing = true;
        self.playhead = Playhead::START;
        self.next_note_time = self.now() + 0.1;
    }

    /// Stop the loop. Already-queued notes ring out; no new ones are scheduled.
    pub fn stop_music(&mut self) {
        self.music_playing = false;
    }

    /// Swap the active song. Takes effect from the next scheduled step, so a
    /// switch while playing is seamless (no gap, no restart of the audio clock).
    /// The arrangement restarts from its first section. A NEW song's note
    /// voices start baking in the background ([`BakedMusic`]); until each
    /// lands its notes fall back to live oscillator synthesis per step.
    pub fn set_song(&mut self, spec: SongSpec) {
        let changed = self.song.name != spec.name;
        self.song = spec;
        self.playhead = Playhead::START;
        if changed {
            self.rebuild_music_bake();
        }
    }

    /// Select a song by index into [`SONGS`] (clamped) and start playing it.
    /// This is the primary entry point for the integrator: `play_song(floor)`
    /// via [`song_for_floor`], or a direct index from the `?viz` tracker.
    pub fn play_song(&mut self, index: usize) {
        let idx = index.min(SONGS.len().saturating_sub(1));
        self.set_song(SONGS[idx]);
        self.start_music();
    }

    // --- tracker API -------------------------------------------------------
    //
    // Read/seek hooks for the `?viz` MUSICS tracker view. All indices are
    // channels 0..NUM_CHANNELS (see CHANNEL_NAMES): 0 bass, 1 lead, 2 pad,
    // 3 arp, 4 drums.
    //
    // IMPORTANT: every read here reflects the *currently-playing section*, so
    // the tracker grid + playhead always mirror what is actually sounding as
    // the arrangement moves from section to section.

    /// The song currently loaded into the scheduler (copyable data).
    pub fn current_song(&self) -> SongSpec {
        self.song
    }

    /// Whether the music scheduler is currently running.
    pub fn is_playing(&self) -> bool {
        self.music_playing
    }

    /// Index of the section currently playing within the song's arrangement.
    pub fn current_section(&self) -> usize {
        self.playhead.section
    }

    /// Human-readable label of the currently-playing section (e.g. "refrain").
    pub fn current_section_label(&self) -> &'static str {
        self.section_ref().map(|s| s.label).unwrap_or("")
    }

    /// How many sections the current song's arrangement contains.
    pub fn section_count(&self) -> usize {
        self.song.sections.len()
    }

    /// Number of steps in the currently-playing section's pattern (its longest
    /// lane). This is the width of the live tracker grid.
    pub fn pattern_len(&self) -> usize {
        self.loop_len()
    }

    /// The step currently *sounding* within the current section (accounts for
    /// the scheduler look-ahead), for drawing the moving playhead. `0` when
    /// stopped.
    pub fn current_step(&self) -> usize {
        let loop_len = self.loop_len();
        if !self.music_playing || loop_len == 0 {
            return 0;
        }
        let step_dur = self.step_dur();
        let ahead = ((self.next_note_time - self.now()) / step_dur).ceil();
        let ahead = if ahead.is_finite() && ahead > 0.0 {
            ahead as usize
        } else {
            0
        };
        self.playhead.sounding_step(&self.song, ahead)
    }

    /// Does `channel` have a note/hit at `step` in the current section? Drives
    /// the tracker grid cells.
    pub fn channel_active(&self, channel: usize, step: usize) -> bool {
        match self.section_ref() {
            Some(sec) => cell_active(sec, channel, step),
            None => false,
        }
    }

    // --- section mini-map API ---------------------------------------------
    //
    // For a clickable strip of section miniatures above the main grid: read any
    // section (not just the playing one) and jump the playhead between them.

    /// Human-readable label of section `i` in the arrangement (e.g. "verse"),
    /// or `""` if `i` is out of range. Use to caption each miniature.
    pub fn section_label(&self, i: usize) -> &'static str {
        self.song.sections.get(i).map(|s| s.label).unwrap_or("")
    }

    /// Number of steps in section `i` (its longest lane), or `0` if out of
    /// range. Lets a miniature size its own little grid.
    pub fn section_pattern_len(&self, i: usize) -> usize {
        self.song.sections.get(i).map(section_len).unwrap_or(0)
    }

    /// Sample any section's grid: does `channel` have a note/hit at `step` in
    /// section `section`? A per-section previewer for drawing the miniatures
    /// (the `section == current_section()` one mirrors [`Self::channel_active`]).
    pub fn section_cell(&self, section: usize, channel: usize, step: usize) -> bool {
        match self.song.sections.get(section) {
            Some(sec) => cell_active(sec, channel, step),
            None => false,
        }
    }

    /// Compact density summary of section `i`: the fraction (0.0..=1.0) of all
    /// grid cells that carry a note/hit. A cheap way to shade each miniature by
    /// how busy/intense it is without drawing every cell.
    pub fn section_density(&self, i: usize) -> f32 {
        self.song
            .sections
            .get(i)
            .map(section_density)
            .unwrap_or(0.0)
    }

    /// Jump the playhead to `step` within the current section (wrapped). Music
    /// keeps playing from there on the next scheduled note; the section is not
    /// changed.
    pub fn seek(&mut self, step: usize) {
        self.playhead.seek(&self.song, step);
        self.next_note_time = self.now() + 0.02;
    }

    /// Jump the playhead to the start of section `i` in the arrangement,
    /// clamped into range. Music continues seamlessly from that section's first
    /// step on the next scheduled note. Drives clicking a section miniature.
    pub fn jump_to_section(&mut self, i: usize) {
        self.playhead.jump_to_section(&self.song, i);
        self.next_note_time = self.now() + 0.02;
    }

    /// Toggle mute for `channel` (out of range is ignored).
    pub fn toggle_mute(&mut self, channel: usize) {
        if channel < NUM_CHANNELS {
            self.mute[channel] = !self.mute[channel];
        }
    }

    /// Toggle solo for `channel` (out of range is ignored).
    pub fn toggle_solo(&mut self, channel: usize) {
        if channel < NUM_CHANNELS {
            self.solo[channel] = !self.solo[channel];
        }
    }

    /// Is `channel` muted?
    pub fn is_muted(&self, channel: usize) -> bool {
        channel < NUM_CHANNELS && self.mute[channel]
    }

    /// Is `channel` soloed?
    pub fn is_solo(&self, channel: usize) -> bool {
        channel < NUM_CHANNELS && self.solo[channel]
    }

    /// Should `channel` actually be heard right now? Muted channels are silent;
    /// if any channel is soloed, only soloed channels sound.
    pub(super) fn channel_audible(&self, channel: usize) -> bool {
        if channel >= NUM_CHANNELS || self.mute[channel] {
            return false;
        }
        let any_solo = self.solo.iter().any(|&s| s);
        !any_solo || self.solo[channel]
    }

    /// The section currently playing, if the arrangement is non-empty.
    pub(super) fn section_ref(&self) -> Option<&Section> {
        self.playhead.section_ref(&self.song)
    }

    /// Length of one sequencer step (seconds) for the current song's tempo.
    pub(super) fn step_dur(&self) -> f64 {
        step_dur(&self.song)
    }

    /// Number of steps before the *current section* repeats.
    pub(super) fn loop_len(&self) -> usize {
        self.playhead.loop_len(&self.song)
    }

    /// Drive the look-ahead scheduler. Call every frame; `now_seconds` is
    /// unused (we trust the audio clock), kept for a stable game-loop signature.
    pub fn update(&mut self, _now_seconds: f64) {
        // Chip away at the pre-render queue — combat SFX, then the song's
        // music voices, then rare SFX — independent of the sound toggle, so
        // the baked voices are ready the moment sound comes (back) on. Up to
        // `pump_budget` renders run concurrently (the game loop raises the
        // budget on loading/menu screens to burn the queue down before
        // gameplay, where a construction hitch would be visible).
        if self.renders_in_flight.get() == 0 {
            self.baked.pending.borrow_mut().clear();
            self.baked_music.pending.borrow_mut().clear();
        }
        for _ in 0..self.pump_budget.get().max(1) {
            self.pump_prerender();
        }
        if !self.enabled.get() {
            return; // sound off: the scheduler idles entirely
        }
        if !self.music_playing {
            return;
        }
        let now = self.now();
        // Catch up if we fell behind (e.g. after a tab was backgrounded).
        if self.next_note_time < now {
            self.next_note_time = now + 0.05;
        }
        let step_dur = self.step_dur();
        let bar_steps = bar_steps(&self.song);
        while self.next_note_time < now + LOOKAHEAD {
            let t = self.next_note_time;
            // At the top of each bar, arm the synthwave filter sweep for it.
            if self.playhead.at_bar_start(&self.song) {
                self.schedule_filter_sweep(t, step_dur * bar_steps as f64);
            }
            self.schedule_step(self.playhead.step, t);
            self.next_note_time += step_dur;
            // The section advances when its LONGEST lane ends (whole bars —
            // asserted by the songs tests).
            self.playhead.advance(&self.song);
        }
    }

    /// Sweep the music-bus lowpass cutoff up and back down across one bar — the
    /// signature synthwave "filter wah". Darker songs sweep a narrower, lower
    /// band so the mix stays muffled and oppressive.
    pub(super) fn schedule_filter_sweep(&self, start: f64, bar_dur: f64) {
        let filt = match &self.music_filter {
            Some(f) => f,
            None => return,
        };
        // Higher intensity => lower/tighter peak, for a darker, closed sound.
        let peak_hz = (5200.0 / self.song.intensity.max(0.4)).clamp(1400.0, 6000.0) as f32;
        let low_hz = 420.0f32;
        let f = filt.frequency();
        let _ = f.set_value_at_time(low_hz, start);
        let _ = f.exponential_ramp_to_value_at_time(peak_hz, start + bar_dur * 0.5);
        let _ = f.exponential_ramp_to_value_at_time(low_hz, start + bar_dur);
    }

    /// Schedule one step of the current section (all channels) at time `t`.
    /// Every note goes through [`Self::music_note`]: one pre-baked
    /// `AudioBufferSourceNode` when the voice's buffer is ready, the live
    /// oscillator synthesis otherwise.
    pub(super) fn schedule_step(&self, step: usize, t: f64) {
        let sec = match self.section_ref() {
            Some(s) => s,
            None => return,
        };
        if self.channel_audible(0) {
            if let Some(d) = degree_at(sec.bass, step) {
                self.music_note(MusicKey::Bass(d), t, sec.vel[0]);
            }
        }
        if self.channel_audible(1) {
            if let Some(d) = degree_at(sec.lead, step) {
                self.music_note(MusicKey::Lead(d), t, sec.vel[1]);
            }
        }
        if self.channel_audible(2) {
            if let Some(d) = degree_at(sec.pad, step) {
                self.music_note(MusicKey::Pad(d), t, sec.vel[2]);
            }
        }
        if self.channel_audible(3) {
            if let Some(d) = degree_at(sec.arp, step) {
                self.music_note(MusicKey::Arp(d), t, sec.vel[3]);
            }
        }
        if self.channel_audible(4) {
            if let Some(key) = MusicKey::of_drum(drum_at(sec.drums, step)) {
                self.music_note(key, t, sec.vel[4]);
            }
        }
        // SIDECHAIN DUCK: in a ducked section every sounding kick pumps the
        // melodic stage of the bus down and lets it recover — the exact
        // retrigger-and-recover curve of `songs::duck_gain`, programmed as
        // one set + ramp pair per kick on the duck gain node (drums enter
        // the bus past it and never duck themselves).
        if sec.duck && self.channel_audible(4) && is_kick_step(sec, step) {
            if let Some(duck) = &self.music_duck {
                let g = duck.gain();
                let _ = g.set_value_at_time(DUCK_FLOOR as f32, t);
                let _ = g.linear_ramp_to_value_at_time(1.0, t + DUCK_RECOVERY);
            }
        }
    }

    /// Play one music voice at absolute time `t` at velocity `vel` (the
    /// section's per-channel level): the pre-baked buffer if it landed (a
    /// single source node into the live music bus — the per-bar filter
    /// sweep still shapes it downstream), else the live synthesis.
    /// Velocity is applied at play time, so one baked buffer serves every
    /// level.
    pub(super) fn music_note(&self, key: MusicKey, t: f64, vel: f32) {
        if self.play_music_baked(key, t, vel) {
            return;
        }
        self.synth_music_note_vel(key, t, vel as f64);
    }

    /// Fire `key` from its pre-rendered buffer at time `t`. `false` = not
    /// baked yet (or no context): the caller falls back to live synthesis.
    /// Melodic keys enter the bus through the duck stage, drums bypass it;
    /// a non-nominal `vel` inserts one gain node (nominal notes stay a
    /// single source node).
    pub(super) fn play_music_baked(&self, key: MusicKey, t: f64, vel: f32) -> bool {
        let ctx = match &self.ctx {
            Some(c) => c,
            None => return false,
        };
        let slots = self.baked_music.slots.borrow();
        let buf = match slots
            .iter()
            .find(|s| s.key == key)
            .and_then(|s| s.buf.as_ref())
        {
            Some(b) => b,
            None => return false,
        };
        let is_drum = matches!(key, MusicKey::Kick | MusicKey::Hat | MusicKey::Snare);
        let out = match if is_drum {
            self.music_out()
        } else {
            self.melodic_out()
        } {
            Some(o) => o,
            None => return false,
        };
        let src = match ctx.create_buffer_source() {
            Ok(s) => s,
            Err(_) => return false,
        };
        src.set_buffer(Some(buf));
        let wired = if (vel - 1.0).abs() > 1e-3 {
            match ctx.create_gain() {
                Ok(g) => {
                    let _ = g.gain().set_value_at_time(vel.max(0.0), 0.0);
                    src.connect_with_audio_node(&g).is_ok()
                        && g.connect_with_audio_node(&out).is_ok()
                }
                Err(_) => false,
            }
        } else {
            src.connect_with_audio_node(&out).is_ok()
        };
        if !wired {
            return false;
        }
        let sched: &web_sys::AudioScheduledSourceNode = src.as_ref();
        let _ = sched.start_with_when(t);
        true
    }

    /// The LIVE synthesis of one music voice at absolute time `t` — also
    /// what the offline pre-render runs (at t = 0 and nominal velocity, see
    /// [`Self::render_music_slot`]), so a baked note is the identical
    /// signal, just rendered ahead of time.
    pub(super) fn synth_music_note(&self, key: MusicKey, t: f64) {
        self.synth_music_note_vel(key, t, 1.0);
    }

    /// [`Self::synth_music_note`] scaled by a section velocity (the live
    /// fallback path; the bake always renders at 1.0 and velocity is a
    /// play-time gain).
    pub(super) fn synth_music_note_vel(&self, key: MusicKey, t: f64, vel: f64) {
        let s = &self.song;
        let step_dur = self.step_dur();
        let gain = MUSIC_GAIN * s.intensity * vel.max(0.0);
        match key {
            MusicKey::Bass(d) => {
                let f = degree_freq(s.root, s.scale, d);
                self.music_voice(s.bass_wave, f, t, step_dur * 1.9, gain * 1.3, 0.005);
            }
            MusicKey::Lead(d) => {
                let f = degree_freq(s.root, s.scale, d);
                self.music_voice(s.lead_wave, f, t, step_dur * 0.9, gain, 0.005);
            }
            MusicKey::Pad(d) => {
                // Bloom the pad note into a triad (root + third + fifth), held
                // across several steps with a slow attack for a chord bed.
                for interval in [0, 2, 4] {
                    let f = degree_freq(s.root, s.scale, d + interval);
                    self.music_voice(s.pad_wave, f, t, step_dur * 4.0, gain * 0.45, 0.06);
                }
            }
            MusicKey::Arp(d) => {
                let f = degree_freq(s.root, s.scale, d);
                self.music_voice(s.arp_wave, f, t, step_dur * 0.7, gain * 0.7, 0.005);
            }
            MusicKey::Kick => self.drum(Kick, t, gain),
            MusicKey::Hat => self.drum(Hat, t, gain),
            MusicKey::Snare => self.drum(Snare, t, gain),
        }
    }

    // --- music voices --------------------------------------------------------
    //
    // One melodic note of a given `Wave`: the four basic shapes are a single
    // enveloped oscillator; the darksynth PRESETS are small node graphs built
    // from the same primitives. Every one targets `melodic_out` (the duck
    // stage of the bus — the offline sink during a pre-render), so presets
    // bake per pitch exactly like plain waves.

    /// Play one melodic music voice: dispatch `wave` to its builder.
    pub(super) fn music_voice(&self, wave: Wave, f: f64, t: f64, dur: f64, peak: f64, attack: f64) {
        let out = match self.melodic_out() {
            Some(o) => o,
            None => return,
        };
        match wave {
            Wave::Supersaw => self.supersaw_voice(&out, f, t, dur, peak, attack),
            Wave::DrivenBass => self.driven_voice(&out, f, t, dur, peak, attack),
            Wave::DarkPad => self.darkpad_voice(&out, f, t, dur, peak, attack.max(0.25 * dur)),
            w => self.tone_out(&out, f, f, t, dur, peak, attack, osc(w)),
        }
    }

    /// SUPERSAW: five sawtooths detuned across ±12 cents (center loudest,
    /// outer pairs quieter — the "slight spread"), summed into one shared
    /// envelope gain. The classic wide, hissing darksynth stack.
    pub(super) fn supersaw_voice(
        &self,
        out: &web_sys::AudioNode,
        f: f64,
        t: f64,
        dur: f64,
        peak: f64,
        attack: f64,
    ) {
        let ctx = match self.bctx() {
            Some(c) => c,
            None => return,
        };
        let env = match ctx.create_gain() {
            Ok(g) => g,
            Err(_) => return,
        };
        let g = env.gain();
        let _ = g.set_value_at_time(0.0001, t);
        let _ = g.exponential_ramp_to_value_at_time(peak.max(0.0002) as f32, t + attack.max(0.001));
        let _ = g.exponential_ramp_to_value_at_time(0.0001, t + dur);
        if env.connect_with_audio_node(out).is_err() {
            return;
        }
        // (detune cents, level) — normalized so the stack sums to ~1.
        const SAWS: [(f64, f64); 5] = [
            (-12.0, 0.18),
            (-5.0, 0.22),
            (0.0, 0.28),
            (5.0, 0.22),
            (12.0, 0.18),
        ];
        for (cents, level) in SAWS {
            let (osc, mix) = match (ctx.create_oscillator(), ctx.create_gain()) {
                (Ok(o), Ok(m)) => (o, m),
                _ => continue,
            };
            osc.set_type(OscillatorType::Sawtooth);
            let detuned = f * 2f64.powf(cents / 1200.0);
            let _ = osc.frequency().set_value_at_time(detuned as f32, t);
            let _ = mix.gain().set_value_at_time(level as f32, t);
            let _ = osc.connect_with_audio_node(&mix);
            let _ = mix.connect_with_audio_node(&env);
            let sched: &web_sys::AudioScheduledSourceNode = osc.as_ref();
            let _ = sched.start_with_when(t);
            let _ = sched.stop_with_when(t + dur + 0.02);
        }
    }

    /// DRIVEN BASS: a sawtooth + a square an octave below it, driven hot
    /// into a waveshaper-style soft clip ([`Self::soft_clipper`], low knee =
    /// heavy saturation) and shaped by the envelope AFTER the clipper (so
    /// the decay stays clean while the tone growls).
    pub(super) fn driven_voice(
        &self,
        out: &web_sys::AudioNode,
        f: f64,
        t: f64,
        dur: f64,
        peak: f64,
        attack: f64,
    ) {
        let ctx = match self.bctx() {
            Some(c) => c,
            None => return,
        };
        let (drive, env) = match (ctx.create_gain(), ctx.create_gain()) {
            (Ok(d), Ok(e)) => (d, e),
            _ => return,
        };
        // Hot into the clipper: the shaper's curve covers ±2, so ~1.6 of
        // summed oscillator drive saturates hard without folding.
        let _ = drive.gain().set_value_at_time(1.6, t);
        let drive_node: web_sys::AudioNode = AsRef::<web_sys::AudioNode>::as_ref(&drive).clone();
        let post = Self::soft_clipper(&ctx, &drive_node, 0.25).unwrap_or(drive_node);
        let g = env.gain();
        let _ = g.set_value_at_time(0.0001, t);
        let _ = g.exponential_ramp_to_value_at_time(peak.max(0.0002) as f32, t + attack.max(0.001));
        let _ = g.exponential_ramp_to_value_at_time(0.0001, t + dur);
        if post.connect_with_audio_node(&env).is_err() || env.connect_with_audio_node(out).is_err()
        {
            return;
        }
        for (shape, freq, level) in [
            (OscillatorType::Sawtooth, f, 0.7),
            (OscillatorType::Square, f * 0.5, 0.45),
        ] {
            let (osc, mix) = match (ctx.create_oscillator(), ctx.create_gain()) {
                (Ok(o), Ok(m)) => (o, m),
                _ => continue,
            };
            osc.set_type(shape);
            let _ = osc.frequency().set_value_at_time(freq as f32, t);
            let _ = mix.gain().set_value_at_time(level, t);
            let _ = osc.connect_with_audio_node(&mix);
            let _ = mix.connect_with_audio_node(&drive);
            let sched: &web_sys::AudioScheduledSourceNode = osc.as_ref();
            let _ = sched.start_with_when(t);
            let _ = sched.stop_with_when(t + dur + 0.02);
        }
    }

    /// DARK PAD: a detuned sawtooth pair (±7 cents) through a fixed dark
    /// lowpass (~900 Hz, gentle resonance) under a slow-attack envelope —
    /// a breathing chord bed that sits under the bus's own bar sweep.
    pub(super) fn darkpad_voice(
        &self,
        out: &web_sys::AudioNode,
        f: f64,
        t: f64,
        dur: f64,
        peak: f64,
        attack: f64,
    ) {
        let ctx = match self.bctx() {
            Some(c) => c,
            None => return,
        };
        let (filt, env) = match (ctx.create_biquad_filter(), ctx.create_gain()) {
            (Ok(f), Ok(e)) => (f, e),
            _ => return,
        };
        filt.set_type(BiquadFilterType::Lowpass);
        let _ = filt.frequency().set_value_at_time(900.0, t);
        let _ = filt.q().set_value_at_time(0.8, t);
        let g = env.gain();
        let _ = g.set_value_at_time(0.0001, t);
        let _ = g.linear_ramp_to_value_at_time(peak.max(0.0002) as f32, t + attack.max(0.001));
        let _ = g.exponential_ramp_to_value_at_time(0.0001, t + dur);
        if filt.connect_with_audio_node(&env).is_err() || env.connect_with_audio_node(out).is_err()
        {
            return;
        }
        for cents in [-7.0f64, 7.0] {
            let osc = match ctx.create_oscillator() {
                Ok(o) => o,
                Err(_) => continue,
            };
            osc.set_type(OscillatorType::Sawtooth);
            let detuned = f * 2f64.powf(cents / 1200.0);
            let _ = osc.frequency().set_value_at_time(detuned as f32, t);
            let _ = osc.connect_with_audio_node(&filt);
            let sched: &web_sys::AudioScheduledSourceNode = osc.as_ref();
            let _ = sched.start_with_when(t);
            let _ = sched.stop_with_when(t + dur + 0.02);
        }
    }

    /// Seconds of dry signal one music voice needs when baked: the note
    /// duration its live envelope uses (a function of the song's step
    /// length — see [`Self::synth_music_note`]) plus the builders' small
    /// stop margin.
    pub(super) fn music_key_len(&self, key: MusicKey) -> f64 {
        let sd = self.step_dur();
        match key {
            MusicKey::Bass(_) => sd * 1.9 + 0.03,
            MusicKey::Lead(_) => sd * 0.9 + 0.03,
            MusicKey::Pad(_) => sd * 4.0 + 0.03,
            MusicKey::Arp(_) => sd * 0.7 + 0.03,
            MusicKey::Kick => 0.21, // 0.18 s tone + stop margin (noise is 0.05)
            MusicKey::Hat => 0.06,  // 0.03 s noise tick + margin
            MusicKey::Snare => 0.16, // 0.13 s noise + margin (tone is 0.10)
        }
    }

    /// Reset the music bake queue for the current song: enumerate its voice
    /// set fresh (all unbaked — notes fall back live until each render
    /// lands) and invalidate any in-flight render of the previous song.
    pub(super) fn rebuild_music_bake(&self) {
        let m = &self.baked_music;
        m.gen.set(m.gen.get().wrapping_add(1));
        *m.slots.borrow_mut() = music_keys(&self.song)
            .into_iter()
            .map(|key| MusicSlot { key, buf: None })
            .collect();
        m.next.set(0);
    }

    /// Render one synthesized drum hit at absolute time `t` (routed to the bus).
    pub(super) fn drum(&self, hit: Drum, t: f64, gain: f64) {
        match hit {
            Silent => {}
            Kick => {
                self.music_tone(140.0, 45.0, t, 0.18, gain * 1.6, OscillatorType::Sine);
                self.music_noise(t, 0.05, gain * 0.4, BiquadFilterType::Lowpass, 400.0, 80.0);
            }
            Hat => {
                self.music_noise(
                    t,
                    0.03,
                    gain * 0.5,
                    BiquadFilterType::Highpass,
                    9000.0,
                    9000.0,
                );
            }
            Snare => {
                self.music_noise(
                    t,
                    0.13,
                    gain * 0.7,
                    BiquadFilterType::Highpass,
                    1800.0,
                    1400.0,
                );
                self.music_tone(220.0, 170.0, t, 0.10, gain * 0.5, OscillatorType::Triangle);
            }
        }
    }
}
