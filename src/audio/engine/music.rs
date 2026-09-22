//! The tracker: transport + introspection API, the look-ahead scheduler
//! (`update`: swing, humanize, velocity, the side-chain), the per-song lane
//! settings (`apply_voices`), note voices (raw stacks, noise, the supersaw /
//! driven / darkpad presets, the live sketch) and the drum kit.

use super::*;
use crate::audio::dsp;

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
    /// lands its notes fall back to the live sketch per step.
    pub fn set_song(&mut self, spec: SongSpec) {
        let changed = self.song.name != spec.name;
        self.song = spec;
        self.playhead = Playhead::START;
        if changed {
            self.apply_voices();
            self.rebuild_music_bake();
        }
    }

    /// Point the persistent lane channels at the current song: each lane's
    /// pan, drive curve, echo and reverb sends, and the echo line's time /
    /// feedback / tone. (`set_value_at_time(v, 0.0)` is the engine's idiom
    /// for a static value: an event in the past applies at once.)
    pub(super) fn apply_voices(&self) {
        let voice = |lane: usize| self.song.voices.get(lane);
        for (lane, panner) in self.music_pan.iter().enumerate() {
            let pan = voice(lane).map_or(0.0, |v| v.pan);
            let _ = panner
                .pan()
                .set_value_at_time(pan.clamp(-1.0, 1.0) as f32, 0.0);
        }
        if let Some(fx) = &self.music_fx {
            for (lane, send) in fx.echo_send.iter().enumerate() {
                let v = voice(lane).map_or(0.0, |v| v.echo);
                let _ = send.gain().set_value_at_time(v.clamp(0.0, 1.0) as f32, 0.0);
            }
            for (lane, send) in fx.verb_send.iter().enumerate() {
                let v = voice(lane).map_or(0.0, |v| v.reverb);
                let _ = send.gain().set_value_at_time(v.clamp(0.0, 1.0) as f32, 0.0);
            }
            let e = self.song.echo;
            let secs = (e.steps.max(0.0) * self.step_dur()).clamp(0.001, ECHO_MAX_SECONDS);
            let _ = fx.delay.delay_time().set_value_at_time(secs as f32, 0.0);
            let _ = fx
                .feedback
                .gain()
                .set_value_at_time(e.feedback.clamp(0.0, 0.95) as f32, 0.0);
            let _ = fx
                .tone
                .frequency()
                .set_value_at_time(e.tone.clamp(200.0, 18000.0) as f32, 0.0);
        }
        for (lane, shaper) in self.music_drive.iter().enumerate() {
            let drive = voice(lane).map_or(0.0, |v| v.drive);
            if drive > 0.0 {
                let mut curve = Self::drive_curve(drive, 2048);
                shaper.set_curve_opt_f32_slice(Some(curve.as_mut_slice()));
            } else {
                shaper.set_curve_opt_f32_slice(None);
            }
        }
    }

    /// The lane drive transfer curve over an input of ±1: `y = r ·
    /// tanh(k x) / tanh(k r)` with `k = 2 + 40·drive` and `r` = 0.15 (a
    /// loud lane peak) — peaks that reach `r` come out at `r`, everything
    /// under it is lifted and squashed toward it. Music signals are small
    /// (`MUSIC_GAIN`), which is why the knee is scaled to `r`, not to 1.
    pub(super) fn drive_curve(drive: f64, n: usize) -> Vec<f32> {
        let k = 2.0 + 40.0 * drive.clamp(0.0, 1.0);
        let r = 0.15f64;
        let norm = r / (k * r).tanh();
        (0..n)
            .map(|i| {
                let x = i as f64 / (n - 1) as f64 * 2.0 - 1.0;
                ((k * x).tanh() * norm) as f32
            })
            .collect()
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
    // channels 0..NUM_CHANNELS (see CHANNEL_NAMES): the five melodic lanes,
    // then DRUMS and PERC.
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

    /// What `channel` shows at `step` in the current section — a note / hit
    /// (with its velocity), a tie, or nothing. Drives the tracker grid cells.
    pub fn channel_cell(&self, channel: usize, step: usize) -> GridCell {
        match self.section_ref() {
            Some(sec) => cell_at(sec, channel, step),
            None => GridCell::Off,
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

    /// Sample any section's grid: what `channel` shows at `step` in section
    /// `section`. A per-section previewer for drawing the miniatures (the
    /// `section == current_section()` one mirrors [`Self::channel_cell`]).
    pub fn section_cell(&self, section: usize, channel: usize, step: usize) -> GridCell {
        match self.song.sections.get(section) {
            Some(sec) => cell_at(sec, channel, step),
            None => GridCell::Off,
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
            // The grid time; the swung time is what the notes fire at.
            let t = self.next_note_time;
            // At the top of each bar, arm the synthwave filter sweep for it.
            if self.playhead.at_bar_start(&self.song) {
                self.schedule_filter_sweep(t, step_dur * bar_steps as f64);
            }
            let step = self.playhead.step;
            self.schedule_step(step, t + swing_delay(self.song.swing, step, step_dur));
            self.next_note_time += step_dur;
            // The section advances when its LONGEST lane ends (whole bars —
            // asserted by the songs tests).
            self.playhead.advance(&self.song);
        }
    }

    /// Sweep the music-bus lowpass cutoff up and back down across one bar — the
    /// signature synthwave "filter wah". Darker songs sweep a narrower, lower
    /// band so the mix stays muffled and oppressive; the song's `sweep`
    /// scales how far below the peak the filter closes at the bar lines
    /// (1.0 = all the way to 420 Hz, 0 = it stays open).
    pub(super) fn schedule_filter_sweep(&self, start: f64, bar_dur: f64) {
        let filt = match &self.music_filter {
            Some(f) => f,
            None => return,
        };
        // Higher intensity => lower/tighter peak, for a darker, closed sound.
        let peak_hz = (5200.0 / self.song.intensity.max(0.4)).clamp(1400.0, 6000.0);
        let depth = self.song.sweep.clamp(0.0, 1.0);
        let low_hz = peak_hz * (420.0 / peak_hz).powf(depth);
        let f = filt.frequency();
        if depth <= 0.0 {
            let _ = f.set_value_at_time(peak_hz as f32, start);
            return;
        }
        let _ = f.set_value_at_time(low_hz as f32, start);
        let _ = f.exponential_ramp_to_value_at_time(peak_hz as f32, start + bar_dur * 0.5);
        let _ = f.exponential_ramp_to_value_at_time(low_hz as f32, start + bar_dur);
    }

    /// Schedule one step of the current section (all channels) at time `t`.
    /// Every note goes through [`Self::music_note`]: one pre-baked
    /// `AudioBufferSourceNode` when the voice's buffer is ready, the live
    /// sketch otherwise. A note's level is its step velocity × its
    /// channel's section level.
    pub(super) fn schedule_step(&self, step: usize, t: f64) {
        let sec = match self.section_ref() {
            Some(s) => s,
            None => return,
        };
        for lane in MELODIC {
            if !self.channel_audible(lane) {
                continue;
            }
            if let Some(n) = note_at(sec.lane(lane), step) {
                let key = note_key(&self.song, sec, lane, step, &n);
                let vel = vel_at(sec.vel_lane(lane), step);
                self.music_note(key, self.humanized(t), Self::vel_gain(vel, sec, lane));
            }
        }
        let mut kicked = false;
        for lane in [DRUMS, PERC] {
            if !self.channel_audible(lane) {
                continue;
            }
            let vel = vel_at(sec.vel_lane(lane), step);
            let drum = drum_at(sec.drum_lane(lane), step);
            if drum == Silent || vel == 0 {
                continue;
            }
            // Kicks stay on the grid (they ARE the grid); the rest breathe.
            let at = if drum == Kick { t } else { self.humanized(t) };
            self.music_note(MusicKey::Drum(drum), at, Self::vel_gain(vel, sec, lane));
            kicked |= drum == Kick;
        }
        // SIDECHAIN DUCK: in a ducked section every sounding kick pumps the
        // melodic lanes (drums enter the bus past the ducker and never duck
        // themselves).
        if kicked && sec.duck {
            self.duck(t);
        }
    }

    /// `t` moved by a fresh timing offset in `±humanize` seconds — never
    /// into the past of the audio clock (a source started there fires late
    /// and its envelope is smeared).
    pub(super) fn humanized(&self, t: f64) -> f64 {
        let h = self.song.humanize.clamp(0.0, 0.02);
        if h <= 0.0 {
            return t;
        }
        (t + (self.rand() * 2.0 - 1.0) * h).max(self.now())
    }

    /// Side-chain: a kick at `t` pulls the melodic lanes down to
    /// `1 - depth` over 4 ms and releases them exponentially (time constant
    /// a third of the song's release, so they are ~95 % back by its end).
    /// The curve is picked up from wherever the previous kick's release
    /// currently is (computed by `duck_level`, not read — `AudioParam.value`
    /// is not sample-accurate at a future time), so overlapping kicks never
    /// jump.
    pub(super) fn duck(&self, t: f64) {
        let sc = self.song.sidechain;
        let g = match &self.music_duck {
            Some(d) if sc.active() => d.gain(),
            _ => return,
        };
        let depth = sc.depth.clamp(0.0, 1.0);
        let beat = self.step_dur() * f64::from(self.song.steps_per_beat.max(1));
        let tau = (sc.release_beats.max(0.05) * beat / 3.0).max(0.01);
        let now_level = duck_level(depth, tau, t - self.last_duck.get());
        let bottom = t + 0.004;
        let _ = g.set_value_at_time(now_level as f32, t);
        let _ = g.linear_ramp_to_value_at_time((1.0 - depth) as f32, bottom);
        let _ = g.set_target_at_time(1.0, bottom, tau);
        self.last_duck.set(bottom);
    }

    /// Linear play-time gain of a note: its step velocity (`MAX_VEL` = 1.0)
    /// × its channel's section level.
    pub(super) fn vel_gain(vel: u8, sec: &Section, channel: usize) -> f64 {
        f64::from(vel.min(MAX_VEL)) / f64::from(MAX_VEL) * f64::from(sec.level_of(channel).max(0.0))
    }

    /// Play one music voice at absolute time `t` at play-time gain `vel`
    /// (0 = skipped): the pre-baked buffer if it landed (a single source
    /// node into the lane's channel — pan, drive, sends, duck and the
    /// per-bar filter sweep still shape it downstream), else the live
    /// sketch. Velocity is applied at play time, so one baked buffer serves
    /// every level.
    pub(super) fn music_note(&self, key: MusicKey, t: f64, vel: f64) {
        if vel <= 0.0 {
            return;
        }
        if self.play_music_baked(key, t, vel) {
            return;
        }
        self.synth_music_note(key, t, vel, false);
    }

    /// The channel of the music bus a key plays into: the lane's panner for
    /// a melodic note, the bus itself for the drums.
    pub(super) fn key_out(&self, key: MusicKey) -> Option<webaudio::AudioNode> {
        match key {
            MusicKey::Note { lane, .. } => self.lane_out(lane),
            MusicKey::Drum(_) => self.music_out(),
        }
    }

    /// Fire `key` from its pre-rendered buffer at time `t`. `false` = not
    /// baked yet (or no context): the caller falls back to live synthesis.
    /// A nominal note is one source node straight into its channel; a
    /// non-nominal `vel` inserts one constant gain node (the only extra
    /// node velocity ever costs).
    pub(super) fn play_music_baked(&self, key: MusicKey, t: f64, vel: f64) -> bool {
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
        let out = match self.key_out(key) {
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
                    let _ = g.gain().set_value_at_time(vel.max(0.0) as f32, 0.0);
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
        let sched: &webaudio::AudioScheduledSourceNode = src.as_ref();
        let _ = sched.start_with_when(t);
        true
    }

    /// The synthesis of one music voice at absolute time `t` and play-time
    /// gain `vel`. `full` is what the offline pre-render runs (at t = 0 and
    /// nominal gain, see [`Self::render_music_slot`]): the whole instrument
    /// — preset graph or unison stack, per-oscillator filter envelopes,
    /// vibrato, sub. The LIVE fallback for a not-yet-baked note runs it
    /// `!full`: the same pitches, chord, envelope and channel through ONE
    /// plain oscillator per partial, so a stall can never scale with how
    /// rich a voice is (a full seven-saw seventh chord is ~110 nodes; the
    /// sketch is 8). The baked buffer replaces the sketch the moment it
    /// lands.
    pub(super) fn synth_music_note(&self, key: MusicKey, t: f64, vel: f64, full: bool) {
        let s = &self.song;
        let step_dur = self.step_dur();
        let gain = MUSIC_GAIN * s.intensity * vel.max(0.0);
        match key {
            MusicKey::Note {
                lane,
                degree,
                len,
                chord,
                from,
            } => {
                let (gate, level, attack) = voice_shape(s, lane);
                let voice = s
                    .voices
                    .get(lane)
                    .copied()
                    .unwrap_or(Voice::mono(Wave::Sine));
                let voice = if full { voice } else { Self::sketch(&voice) };
                let hold = step_dur * f64::from(len.max(1) - 1);
                let dur = step_dur * gate;
                // Every partial of the voicing at 1/√n of the level, so a
                // chord is about as loud as a single note of the lane
                // (the pad's level is calibrated for its default triad).
                let partials = chord.degrees();
                let split = level / (partials.len().max(1) as f64).sqrt();
                for (i, &interval) in partials.iter().enumerate() {
                    let f = degree_freq(s.root, s.scale, degree + interval);
                    // A glide starts every partial from the previous note's
                    // matching partial.
                    let from_f = from
                        .filter(|_| voice.glides())
                        .map(|d| degree_freq(s.root, s.scale, d + interval));
                    let note = LaneNote {
                        f,
                        from: from_f,
                        start: t,
                        attack,
                        hold,
                        dur,
                        peak: gain * s.melodic_gain * split,
                    };
                    self.lane_tone(lane, &voice, &note);
                    if i == 0 && voice.sub > 0.0 {
                        self.sub_tone(lane, &voice, &note);
                    }
                }
            }
            MusicKey::Drum(d) => self.drum(d, t, gain),
        }
    }

    /// The cheap live stand-in for `voice` (see [`Self::synth_music_note`]):
    /// one raw oscillator (a preset or a computed string plays as its
    /// nearest shape, [`osc`]), no filter envelope, no vibrato, no sub; pan,
    /// envelope override, glide and the lane's drive / sends are kept (they
    /// live on the channel).
    pub(super) fn sketch(voice: &Voice) -> Voice {
        Voice {
            wave: match voice.wave {
                Wave::Supersaw | Wave::DrivenBass | Wave::DarkPad | Wave::Violin => Wave::Sawtooth,
                Wave::Guitar | Wave::BassGuitar => Wave::Triangle,
                w => w,
            },
            detune: 0.0,
            width: 0.0,
            unison: 1,
            filter: None,
            vibrato: None,
            sub: 0.0,
            ..*voice
        }
    }

    // --- music voices --------------------------------------------------------
    //
    // One melodic note through its `Voice`: a raw shape is a (stack of)
    // enveloped oscillator(s) with optional per-oscillator filter envelope
    // and vibrato; `Wave::Noise` is the noise buffer under the same
    // envelope; the darksynth PRESETS are small node graphs built from the
    // same primitives. Every one targets `lane_out` (the lane's channel —
    // the offline sink during a pre-render), so all of them bake per note.

    /// One melodic-lane partial through its [`Voice`]. Envelope: `attack`
    /// to peak, held `hold` seconds (the tied steps), then the lane's
    /// `dur`-long pluck. A raw shape is a single oscillator, or — with
    /// unison `detune` — a stack spread over `±detune` cents, each at
    /// 0.78/√n of the level and, with `width`, panned across `∓width`
    /// around the lane (the lane's own panner then places the whole image).
    /// Live: into the lane's panner; baking: into the offline sink (mono,
    /// or the stereo stack).
    pub(super) fn lane_tone(&self, lane: usize, voice: &Voice, note: &LaneNote) {
        let out = match self.lane_out(lane) {
            Some(o) => o,
            None => return,
        };
        let LaneNote {
            f,
            from,
            start,
            attack,
            hold,
            dur,
            peak,
        } = *note;
        // A raw voice's note lasts attack + hold + dur; its filter envelope
        // is cut there (nothing is scheduled past the bake's end).
        let end = start + attack.max(0.0) + hold + dur;
        match voice.wave {
            Wave::Supersaw => return self.supersaw_voice(&out, note),
            Wave::DrivenBass => return self.driven_voice(&out, note),
            Wave::DarkPad => return self.darkpad_voice(&out, note),
            Wave::Noise => {
                let filtered = voice
                    .filter
                    .and_then(|flt| self.note_filter(&out, start, end, &flt));
                let dst = filtered.as_ref().unwrap_or(&out);
                return self.noise_env_out(dst, start, attack, hold, dur, peak);
            }
            _ => {}
        }
        let wave = osc(voice.wave);
        let n = voice.oscillators();
        // A stack sums to about one note's loudness (0.78/√n per voice:
        // 0.55 each for the classic pair).
        let level = if n > 1 {
            peak * 0.78 / (n as f64).sqrt()
        } else {
            peak
        };
        for i in 0..n {
            // Spread position −1 … +1 across the stack (0 for a single).
            let frac = if n > 1 {
                -1.0 + 2.0 * i as f64 / (n - 1) as f64
            } else {
                0.0
            };
            let spread = 2f64.powf(frac * voice.detune / 1200.0);
            let f_i = f * spread;
            let target = if n > 1 && voice.width > 0.0 {
                self.side_panner(&out, frac * voice.width)
            } else {
                None
            };
            let dst = target.as_ref().unwrap_or(&out);
            let filtered = voice
                .filter
                .and_then(|flt| self.note_filter(dst, start, end, &flt));
            let dst = filtered.as_ref().unwrap_or(dst);
            self.tone_env(
                dst,
                &Tone {
                    f0: from.map_or(f_i, |ff| ff * spread),
                    f1: f_i,
                    glide: voice.glide,
                    start,
                    attack,
                    hold,
                    dur,
                    peak: level,
                    wave,
                    vibrato: voice.vibrato,
                },
            );
        }
    }

    /// The voice's sine sub-oscillator an octave under `note` (which glides
    /// with it), straight into the lane — no stack, no filter, no vibrato.
    pub(super) fn sub_tone(&self, lane: usize, voice: &Voice, note: &LaneNote) {
        let out = match self.lane_out(lane) {
            Some(o) => o,
            None => return,
        };
        self.tone_env(
            &out,
            &Tone {
                f0: note.from.unwrap_or(note.f) * 0.5,
                f1: note.f * 0.5,
                glide: voice.glide,
                start: note.start,
                attack: note.attack,
                hold: note.hold,
                dur: note.dur,
                peak: note.peak * voice.sub.clamp(0.0, 1.0),
                wave: OscillatorType::Sine,
                vibrato: None,
            },
        );
    }

    /// The shared envelope of the PRESET voices: a gain node into `out`
    /// rising to the note's peak over `attack` (linear for the dark pad's
    /// swell, exponential otherwise), held for the tied steps, then decaying
    /// so that an UNTIED note ends `dur` after its start (the attack is
    /// inside the note, as the presets have always sounded). Returns the
    /// gain node and the note's end time.
    fn preset_env(
        &self,
        ctx: &BaseAudioContext,
        out: &webaudio::AudioNode,
        note: &LaneNote,
        attack: f64,
        linear: bool,
    ) -> Option<(GainNode, f64)> {
        let env = ctx.create_gain().ok()?;
        let t = note.start;
        let attack = attack.max(0.001);
        // The decay needs room after the attack even under an envelope
        // override with a very long attack.
        let end = t + note.hold + note.dur.max(attack + 0.01);
        let peak = note.peak.max(0.0002) as f32;
        let g = env.gain();
        let _ = g.set_value_at_time(0.0001, t);
        let _ = if linear {
            g.linear_ramp_to_value_at_time(peak, t + attack)
        } else {
            g.exponential_ramp_to_value_at_time(peak, t + attack)
        };
        if note.hold > 0.0 {
            let _ = g.set_value_at_time(peak, t + attack + note.hold);
        }
        let _ = g.exponential_ramp_to_value_at_time(0.0001, end);
        env.connect_with_audio_node(out).ok()?;
        Some((env, end))
    }

    /// SUPERSAW: five sawtooths detuned across ±12 cents (center loudest,
    /// outer pairs quieter — the "slight spread"), summed into one shared
    /// envelope gain. The classic wide, hissing darksynth stack.
    pub(super) fn supersaw_voice(&self, out: &webaudio::AudioNode, note: &LaneNote) {
        let ctx = match self.bctx() {
            Some(c) => c,
            None => return,
        };
        let (env, end) = match self.preset_env(&ctx, out, note, note.attack, false) {
            Some(e) => e,
            None => return,
        };
        let t = note.start;
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
            let detuned = note.f * 2f64.powf(cents / 1200.0);
            let _ = osc.frequency().set_value_at_time(detuned as f32, t);
            let _ = mix.gain().set_value_at_time(level as f32, t);
            let _ = osc.connect_with_audio_node(&mix);
            let _ = mix.connect_with_audio_node(&env);
            let sched: &webaudio::AudioScheduledSourceNode = osc.as_ref();
            let _ = sched.start_with_when(t);
            let _ = sched.stop_with_when(end + 0.02);
        }
    }

    /// DRIVEN BASS: a sawtooth + a square an octave below it, driven hot
    /// into a waveshaper-style soft clip ([`Self::soft_clipper`], low knee =
    /// heavy saturation) and shaped by the envelope AFTER the clipper (so
    /// the decay stays clean while the tone growls).
    pub(super) fn driven_voice(&self, out: &webaudio::AudioNode, note: &LaneNote) {
        let ctx = match self.bctx() {
            Some(c) => c,
            None => return,
        };
        let drive = match ctx.create_gain() {
            Ok(d) => d,
            Err(_) => return,
        };
        let t = note.start;
        // Hot into the clipper: the shaper's curve covers ±2, so ~1.6 of
        // summed oscillator drive saturates hard without folding.
        let _ = drive.gain().set_value_at_time(1.6, t);
        let drive_node: webaudio::AudioNode = AsRef::<webaudio::AudioNode>::as_ref(&drive).clone();
        let post = Self::soft_clipper(&ctx, &drive_node, 0.25).unwrap_or(drive_node);
        let (env, end) = match self.preset_env(&ctx, out, note, note.attack, false) {
            Some(e) => e,
            None => return,
        };
        if post.connect_with_audio_node(&env).is_err() {
            return;
        }
        for (shape, freq, level) in [
            (OscillatorType::Sawtooth, note.f, 0.7),
            (OscillatorType::Square, note.f * 0.5, 0.45),
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
            let sched: &webaudio::AudioScheduledSourceNode = osc.as_ref();
            let _ = sched.start_with_when(t);
            let _ = sched.stop_with_when(end + 0.02);
        }
    }

    /// DARK PAD: a detuned sawtooth pair (±7 cents) through a fixed dark
    /// lowpass (~900 Hz, gentle resonance) under a slow-attack envelope (a
    /// quarter of the note at least) — a breathing chord bed that sits
    /// under the bus's own bar sweep.
    pub(super) fn darkpad_voice(&self, out: &webaudio::AudioNode, note: &LaneNote) {
        let ctx = match self.bctx() {
            Some(c) => c,
            None => return,
        };
        let filt = match ctx.create_biquad_filter() {
            Ok(f) => f,
            Err(_) => return,
        };
        let t = note.start;
        filt.set_type(BiquadFilterType::Lowpass);
        let _ = filt.frequency().set_value_at_time(900.0, t);
        let _ = filt.q().set_value_at_time(0.8, t);
        let attack = note.attack.max(0.25 * note.dur);
        let (env, end) = match self.preset_env(&ctx, out, note, attack, true) {
            Some(e) => e,
            None => return,
        };
        if filt.connect_with_audio_node(&env).is_err() {
            return;
        }
        for cents in [-7.0f64, 7.0] {
            let osc = match ctx.create_oscillator() {
                Ok(o) => o,
                Err(_) => continue,
            };
            osc.set_type(OscillatorType::Sawtooth);
            let detuned = note.f * 2f64.powf(cents / 1200.0);
            let _ = osc.frequency().set_value_at_time(detuned as f32, t);
            let _ = osc.connect_with_audio_node(&filt);
            let sched: &webaudio::AudioScheduledSourceNode = osc.as_ref();
            let _ = sched.start_with_when(t);
            let _ = sched.stop_with_when(end + 0.02);
        }
    }

    /// The bake of a COMPUTED voice's note (`Wave::is_computed`): the same
    /// pitches, voicing, level and envelope times [`Self::synth_music_note`]
    /// would build nodes for, rendered by `audio/dsp.rs` into a mono buffer
    /// of the live context. `None` for a drum or a node-built voice (the
    /// caller then runs the offline render).
    pub(super) fn computed_bake(&self, key: MusicKey) -> Option<AudioBuffer> {
        let ctx = self.ctx.as_ref()?;
        let MusicKey::Note {
            lane,
            degree,
            len,
            chord,
            from,
        } = key
        else {
            return None;
        };
        let s = &self.song;
        let voice = *s.voices.get(lane)?;
        if !voice.wave.is_computed() {
            return None;
        }
        let step_dur = self.step_dur();
        let (gate, level, attack) = voice_shape(s, lane);
        let gain = MUSIC_GAIN * s.intensity * s.melodic_gain;
        let shape = dsp::Shape {
            attack,
            hold: step_dur * f64::from(len.max(1) - 1),
            dur: step_dur * gate,
        };
        let degrees = chord.degrees();
        let split = level / (degrees.len().max(1) as f64).sqrt();
        // A plucked chord strums, low string first; a bowed one speaks at once.
        let strum = if voice.wave == Wave::Violin {
            0.0
        } else {
            dsp::STRUM_SECONDS
        };
        let partials: Vec<dsp::Partial> = degrees
            .iter()
            .enumerate()
            .map(|(i, &interval)| dsp::Partial {
                f: degree_freq(s.root, s.scale, degree + interval),
                from: from
                    .filter(|_| voice.glides())
                    .map(|d| degree_freq(s.root, s.scale, d + interval)),
                peak: gain * split,
                delay: i as f64 * strum,
            })
            .collect();
        let sr = ctx.sample_rate();
        let samples = dsp::render_note(&voice, &partials, shape, f64::from(sr))?;
        let buf = ctx.create_buffer(1, samples.len() as u32, sr).ok()?;
        buf.copy_to_channel(&samples, 0).ok()?;
        Some(buf)
    }

    /// Seconds of dry signal one music voice needs when baked
    /// ([`key_seconds`]: attack + tied hold + the lane's decay tail, or the
    /// longest layer of a kit piece, plus the stop margin).
    pub(super) fn music_key_len(&self, key: MusicKey) -> f64 {
        key_seconds(&self.song, key)
    }

    /// Channels a key bakes to: 2 for a note of a WIDE unison voice (its
    /// stack is spread left / right inside the buffer), 1 otherwise.
    pub(super) fn music_key_channels(&self, key: MusicKey) -> u32 {
        match key {
            MusicKey::Note { lane, .. } => match self.song.voices.get(lane) {
                Some(v) if v.is_wide() => 2,
                _ => 1,
            },
            MusicKey::Drum(_) => 1,
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
        use BiquadFilterType::{Bandpass, Highpass, Lowpass};
        match hit {
            Silent => {}
            Kick => {
                self.music_tone(140.0, 45.0, t, 0.18, gain * 1.6, OscillatorType::Sine);
                self.music_noise(t, 0.05, gain * 0.4, Lowpass, 400.0, 80.0);
            }
            Hat => self.music_noise(t, 0.03, gain * 0.5, Highpass, 9000.0, 9000.0),
            Snare => {
                self.music_noise(t, 0.13, gain * 0.7, Highpass, 1800.0, 1400.0);
                self.music_tone(220.0, 170.0, t, 0.10, gain * 0.5, OscillatorType::Triangle);
            }
            Clap => {
                // Three slaps 11 ms apart (the hands never land together)
                // through a mid bandpass, then a softer 120 ms tail.
                for (i, level) in [1.0, 0.8, 0.7].iter().enumerate() {
                    let at = t + i as f64 * 0.011;
                    self.music_noise(at, 0.022, gain * 0.8 * level, Bandpass, 1300.0, 1100.0);
                }
                self.music_noise(t + 0.03, 0.14, gain * 0.45, Bandpass, 1200.0, 900.0);
            }
            OpenHat => self.music_noise(t, 0.28, gain * 0.45, Highpass, 7500.0, 6000.0),
            Tom => {
                self.music_tone(210.0, 95.0, t, 0.28, gain * 1.2, OscillatorType::Sine);
                self.music_noise(t, 0.02, gain * 0.3, Lowpass, 1200.0, 300.0);
            }
            Rim => {
                self.music_tone(1700.0, 1500.0, t, 0.018, gain * 0.5, OscillatorType::Square);
                self.music_noise(t, 0.012, gain * 0.35, Bandpass, 3200.0, 3200.0);
            }
            Crash => {
                self.music_noise(t, 0.95, gain * 0.5, Highpass, 5200.0, 3500.0);
                self.music_noise(t, 0.25, gain * 0.35, Bandpass, 8000.0, 6000.0);
            }
        }
    }
}
