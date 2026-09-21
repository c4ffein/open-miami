//! Pre-rendered voices: the offline-render queue that bakes every SFX variant
//! and every music voice x pitch, and `play_baked`.

use super::*;

impl AudioEngine {
    // --- pre-rendered voices -----------------------------------------------
    //
    // Building a fresh Web Audio graph per shot (oscillators + envelopes +
    // WaveShaper + sends — ~50 nodes for a gunshot, ~250 for a burst)
    // intermittently stalls the main thread 30–100 ms on macOS Chrome. So at
    // startup each one-shot kind's voice is rendered — by the SAME synthesis
    // code, redirected into an `OfflineAudioContext` — into SFX_VARIANTS dry
    // mono `AudioBuffer`s (in the background, up to `pump_budget` renders in
    // flight, driven from `update`). Once a kind's variants are all in, its
    // `play_*` becomes ONE `AudioBufferSourceNode` + the 1–2 gain nodes of
    // its bus routing: the room reverb, compressor and bus soft-clip stay
    // live and identical because only the pre-send dry signal is baked.
    // Per-play variety: a random variant + `playback_rate` jitter matching
    // the live pitch jitter.

    /// Play `kind` from its pre-rendered buffers. `false` = not ready yet
    /// (or no context): the caller falls back to live synthesis.
    pub(super) fn play_baked(&self, kind: SfxKind) -> bool {
        let bufs = self.baked.bufs.borrow();
        let set = &bufs[kind as usize];
        if set.len() < SFX_VARIANTS {
            return false;
        }
        let ctx = match &self.ctx {
            Some(c) => c,
            None => return false,
        };
        let spec = kind.spec();
        let out = match spec.route {
            // Same dry input + wet send the live voice uses; drive 1.0 — the
            // per-voice soft-clip is already baked into the buffer.
            SfxRoute::Real(wet) => self.voice_route(wet, 1.0, true),
            SfxRoute::Melee(wet) => self.voice_route(wet, 1.0, false),
            SfxRoute::Room => self.sfx_out(),
        };
        let out = match out {
            Some(o) => o,
            None => return false,
        };
        let src = match ctx.create_buffer_source() {
            Ok(s) => s,
            Err(_) => return false,
        };
        let variant = ((self.rand() * SFX_VARIANTS as f64) as usize).min(SFX_VARIANTS - 1);
        src.set_buffer(Some(&set[variant]));
        if spec.rate_jitter > 0.0 {
            let _ = src
                .playback_rate()
                .set_value_at_time(self.jit(spec.rate_jitter) as f32, 0.0);
        }
        let _ = src.connect_with_audio_node(&out);
        // The buffer carries the live path's SFX_LEAD of silence at its
        // head, so "as soon as possible" keeps the same transient safety.
        let sched: &webaudio::AudioScheduledSourceNode = src.as_ref();
        let _ = sched.start();
        true
    }

    /// Run `kind`'s live synthesis builder (used both by the `play_*`
    /// fallbacks — indirectly — and by the offline pre-render).
    pub(super) fn synth(&self, kind: SfxKind) {
        match kind {
            SfxKind::AttackGun => self.synth_attack_gun(),
            SfxKind::AttackMachinegun => self.synth_attack_machinegun(),
            SfxKind::AttackShotgun => self.synth_attack_shotgun(),
            SfxKind::AttackClub => self.synth_attack_club(),
            SfxKind::HitGun => self.synth_hit_gun(),
            SfxKind::HitMachinegun => self.synth_hit_machinegun(),
            SfxKind::HitShotgun => self.synth_hit_shotgun(),
            SfxKind::HitClub => self.synth_hit_club(),
            SfxKind::EnemyDown => self.synth_enemy_down(),
            SfxKind::PlayerHurt => self.synth_player_hurt(),
            SfxKind::Pickup => self.synth_pickup(),
            SfxKind::Throw => self.synth_throw(),
            SfxKind::DryFire => self.synth_dry_fire(),
            SfxKind::Death => self.synth_death(),
            SfxKind::LevelClear => self.synth_level_clear(),
            SfxKind::MaskCrack => self.synth_mask_crack(),
            SfxKind::Elevator => self.synth_elevator(),
            SfxKind::EngineIdle => self.synth_engine_idle(),
            SfxKind::TireScreech => self.synth_tire_screech(),
            SfxKind::CarDoorOpen => self.synth_car_door_open(),
            SfxKind::CarDoorClose => self.synth_car_door_close(),
        }
    }

    /// Advance the background pre-render: kick at most one offline render
    /// per call (`update` calls it up to `pump_budget` times per frame, and
    /// it declines while that many are in flight). Priority order:
    /// the combat one-shots first (the sounds a first firefight needs —
    /// the first [`SFX_COMBAT_KINDS`] of [`SFX_KINDS`]), then the current
    /// song's music note voices, then the rare one-shots (death,
    /// level-clear, mask-crack, elevator). If `OfflineAudioContext` is
    /// unavailable the whole queue is abandoned and everything stays on
    /// live synthesis.
    pub(super) fn pump_prerender(&self) {
        if self.ctx.is_none()
            || self.renders_in_flight.get() >= self.pump_budget.get().max(1)
            || self.render_dead.get()
        {
            return;
        }
        let combat = SFX_COMBAT_KINDS.min(SFX_KINDS.len()) * SFX_VARIANTS;
        let total = SFX_KINDS.len() * SFX_VARIANTS;
        let i = self.baked.next.get();
        if i < combat {
            self.kick_sfx_render(i);
            return;
        }
        if self.pump_music_bake() {
            return; // a music voice render was kicked this frame
        }
        if i < total {
            self.kick_sfx_render(i);
        }
    }

    /// Kick the offline render of SFX queue entry `i` (one variant of one
    /// kind), advancing the queue on success and abandoning all baking on
    /// failure (graceful: live synthesis forever).
    pub(super) fn kick_sfx_render(&self, i: usize) {
        if self.render_variant(SFX_KINDS[i / SFX_VARIANTS]) {
            self.baked.next.set(i + 1);
        } else {
            self.render_dead.set(true);
        }
    }

    /// Kick the next unbaked music voice render, if any. `true` = one was
    /// kicked (or baking just died) — the caller should not also kick an
    /// SFX render this frame; `false` = every music slot is baked.
    pub(super) fn pump_music_bake(&self) -> bool {
        let m = &self.baked_music;
        let len = m.slots.borrow().len();
        let mut i = m.next.get();
        while i < len && m.slots.borrow()[i].buf.is_some() {
            i += 1;
        }
        m.next.set(i);
        if i >= len {
            return false;
        }
        if self.render_music_slot(i) {
            m.next.set(i + 1);
        } else {
            self.render_dead.set(true);
        }
        true
    }

    /// Build one offline variant of `kind`: redirect the voice builders into
    /// a fresh mono `OfflineAudioContext` (same sample rate as the live one,
    /// [`SfxSpec::len`] seconds), run the kind's live synthesis code
    /// unchanged, then start the async render; its completion callback
    /// stores the `AudioBuffer` and decrements [`Self::renders_in_flight`]. Returns
    /// `false` if the offline context can't even be created.
    pub(super) fn render_variant(&self, kind: SfxKind) -> bool {
        let live = match &self.ctx {
            Some(c) => c,
            None => return false,
        };
        let sr = live.sample_rate();
        let spec = kind.spec();
        let frames = ((sr as f64) * spec.len).ceil().max(1.0) as u32;
        let off = match OfflineAudioContext::new_with_number_of_channels_and_length_and_sample_rate(
            1, frames, sr,
        ) {
            Ok(o) => o,
            Err(_) => return false,
        };
        let sink = AsRef::<webaudio::AudioNode>::as_ref(&off.destination()).clone();
        *self.render.borrow_mut() = Some(OfflineRender {
            ctx: AsRef::<BaseAudioContext>::as_ref(&off).clone(),
            sink,
        });
        self.synth(kind);
        *self.render.borrow_mut() = None;
        let promise = match off.start_rendering() {
            Ok(p) => p,
            Err(_) => return false,
        };
        self.renders_in_flight.set(self.renders_in_flight.get() + 1);
        let store = Rc::clone(&self.baked);
        let inflight = Rc::clone(&self.renders_in_flight);
        let kidx = kind as usize;
        let done = Closure::once(move |v: JsValue| {
            if let Ok(buf) = v.dyn_into::<AudioBuffer>() {
                store.bufs.borrow_mut()[kidx].push(buf);
            }
            inflight.set(inflight.get().saturating_sub(1));
        });
        let inflight = Rc::clone(&self.renders_in_flight);
        // A rejected render skips this variant: the kind never completes its
        // set and permanently keeps the live path (graceful).
        let fail = Closure::once(move |_e: JsValue| inflight.set(inflight.get().saturating_sub(1)));
        let _ = promise.then2(&done, &fail);
        // Keep the pair alive until it has fired. Concurrent renders may be
        // pending: never clear here — the pile is pruned from update() once
        // nothing is in flight.
        let mut pending = self.baked.pending.borrow_mut();
        pending.push(done);
        pending.push(fail);
        true
    }

    /// Build the offline render of music voice slot `i`: same recipe as
    /// [`Self::render_variant`] — redirect the note builders into a fresh
    /// `OfflineAudioContext` (live sample rate, [`Self::music_key_len`]
    /// seconds, stereo for a wide voice — [`Self::music_key_channels`]), run
    /// the note's FULL synthesis at t = 0 and nominal gain, then
    /// start the async render; its completion callback stores the
    /// `AudioBuffer` into the slot (unless the song changed meanwhile — the
    /// [`BakedMusic::gen`] guard) and decrements [`Self::renders_in_flight`].
    pub(super) fn render_music_slot(&self, i: usize) -> bool {
        let live = match &self.ctx {
            Some(c) => c,
            None => return false,
        };
        let key = match self.baked_music.slots.borrow().get(i) {
            Some(slot) => slot.key,
            None => return false,
        };
        let sr = live.sample_rate();
        let frames = ((sr as f64) * self.music_key_len(key)).ceil().max(1.0) as u32;
        let off = match OfflineAudioContext::new_with_number_of_channels_and_length_and_sample_rate(
            self.music_key_channels(key),
            frames,
            sr,
        ) {
            Ok(o) => o,
            Err(_) => return false,
        };
        let sink = AsRef::<webaudio::AudioNode>::as_ref(&off.destination()).clone();
        *self.render.borrow_mut() = Some(OfflineRender {
            ctx: AsRef::<BaseAudioContext>::as_ref(&off).clone(),
            sink,
        });
        self.synth_music_note(key, 0.0, 1.0, true);
        *self.render.borrow_mut() = None;
        let promise = match off.start_rendering() {
            Ok(p) => p,
            Err(_) => return false,
        };
        self.renders_in_flight.set(self.renders_in_flight.get() + 1);
        let store = Rc::clone(&self.baked_music);
        let inflight = Rc::clone(&self.renders_in_flight);
        let gen = self.baked_music.gen.get();
        let done = Closure::once(move |v: JsValue| {
            if store.gen.get() == gen {
                if let Ok(buf) = v.dyn_into::<AudioBuffer>() {
                    if let Some(slot) = store.slots.borrow_mut().get_mut(i) {
                        slot.buf = Some(buf);
                    }
                }
            }
            inflight.set(inflight.get().saturating_sub(1));
        });
        let inflight = Rc::clone(&self.renders_in_flight);
        // A rejected render leaves the slot unbaked forever: that one voice
        // permanently keeps the live per-note path (graceful).
        let fail = Closure::once(move |_e: JsValue| inflight.set(inflight.get().saturating_sub(1)));
        let _ = promise.then2(&done, &fail);
        // Keep the pair alive until it has fired. Concurrent renders may be
        // pending: never clear here — the pile is pruned from update() once
        // nothing is in flight.
        let mut pending = self.baked_music.pending.borrow_mut();
        pending.push(done);
        pending.push(fail);
        true
    }
}
