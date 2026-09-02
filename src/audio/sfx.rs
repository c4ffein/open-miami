//! The one-shot SFX catalogue: the pre-render queue's kinds, their bake
//! specs and the measured gunshot / engine-idle tunables. Pure data —
//! host-compiled and unit-tested natively; the WebAudio engine
//! (`audio/engine.rs`, wasm-only) does the synthesis.

// Only the wasm engine consumes most of this; natively it is test data.
#![cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]

/// Recipe for one gunshot (see the attacks section comment): a bright
/// crack, a mid-dominant body with a plateau, a low-mid layer, a faint
/// thump — and the room (reverb send) for everything after ~200 ms.
pub(crate) struct RealShot {
    /// Crack level and its rise time (peak lands at +5–15 ms).
    pub(crate) crack: f64,
    pub(crate) crack_rise: f64,
    /// Air (>8 kHz) share of the crack.
    pub(crate) air: f64,
    /// Mid body: bandpass centre, level; holds `plateau` seconds at −5 dB
    /// then falls to −21 dB by `plateau + drop`, then on to silence.
    pub(crate) body_hz: f64,
    pub(crate) body: f64,
    pub(crate) plateau: f64,
    pub(crate) drop: f64,
    /// Hi (2–8 kHz) and air companions of the body plateau, levels.
    pub(crate) body_hi: f64,
    pub(crate) body_air: f64,
    /// Low-mid (130–300 Hz) layer: bandpass centre and level (real "body"
    /// of a big caliber; ~5 dB under the mids for the small ones).
    pub(crate) low_hz: f64,
    pub(crate) low: f64,
    /// Faint sub thump (60–90 Hz), level (≤ −15 dB).
    pub(crate) thump: f64,
    /// Reverb send (the room is most of the sound past 200 ms).
    pub(crate) wet: f64,
}

/// 7.62×39 single (our pistol): mid −3.5 dB dominant, low −8.6, hi −6.6,
/// air −7.7, sub −15.8; crest 20–23 dB; peak at +7.7 ms; centroid 6.2 kHz
/// @5 ms → ~1.3 kHz after; t20 131 ms.
pub(crate) const REAL_762X39: RealShot = RealShot {
    crack: 1.0,
    crack_rise: 0.005,
    air: 0.6,
    body_hz: 1200.0,
    body: 1.0,
    plateau: 0.10,
    drop: 0.10,
    body_hi: 0.26,
    body_air: 0.32,
    low_hz: 230.0,
    low: 1.4,
    thump: 0.03,
    wet: 1.0,
};

/// 5.56 single (our machinegun round): mid −2.7 dominant, low −8.1, hi
/// −8.5, air −8.1, sub −18.8; crest 20–24 dB; peak +6.5 ms; centroid
/// 5.8 kHz @5 ms → 1.2–1.3 kHz; t20 133 ms; roughness ~0.6.
pub(crate) const REAL_556: RealShot = RealShot {
    crack: 0.9,
    crack_rise: 0.004,
    air: 0.65,
    body_hz: 1300.0,
    body: 0.9,
    plateau: 0.09,
    drop: 0.10,
    body_hi: 0.21,
    body_air: 0.30,
    low_hz: 240.0,
    low: 1.1,
    thump: 0.02,
    wet: 1.0,
};

/// 7.62×54R single (our shotgun): the biggest — mid −2.4, a real low-mid
/// body (low −7.4), hi −8.9, air −11.2, sub −14.9; crest 19–22 dB; peak at
/// +14.8 ms; centroid 4.1 kHz @5 ms → 579 Hz @30 → ~1.3 kHz; t20 147 ms.
pub(crate) const REAL_762X54R: RealShot = RealShot {
    crack: 1.1,
    crack_rise: 0.010,
    air: 0.28,
    body_hz: 1000.0,
    body: 1.2,
    plateau: 0.11,
    drop: 0.10,
    body_hi: 0.12,
    body_air: 0.15,
    low_hz: 210.0,
    low: 2.0,
    thump: 0.05,
    wet: 1.0,
};

/// How many pre-rendered jitter variants each one-shot SFX kind gets (see
/// `BakedSfx`): a random one is picked per play, plus a small
/// `playback_rate` jitter, so repeated shots never sound stamped.
pub(crate) const SFX_VARIANTS: usize = 3;

// --- car SFX tunables ----------------------------------------------------
//
// The title-screen ENGINE IDLE is baked as ONE seamless loop. The buffer is
// a warm-up head (the lowpass ring-in settles into its periodic steady
// state) followed by the loop region; playback uses `loop_start`/`loop_end`
// so the wrap always lands steady-state → steady-state. Every periodic
// component — oscillators, the pitch LFO, the amp LFO, the detune beat —
// completes a WHOLE number of cycles over [`ENGINE_LOOP_SECONDS`], so the
// waveform phase at `loop_end` equals the phase at `loop_start` and the
// loop point is inaudible by construction (unit-tested below).

/// Warm-up head (seconds) rendered before the loop region: long enough for
/// the shared lowpass to settle onto the periodic steady state.
pub(crate) const ENGINE_LOOP_WARMUP: f64 = 0.5;
/// Loop region length (seconds). All `ENGINE_LOOP_LOCKED_HZ` frequencies
/// complete whole cycles in this span.
pub(crate) const ENGINE_LOOP_SECONDS: f64 = 4.0;
/// Engine fundamental: the main sawtooth "motor buzz" (180 cycles/loop).
pub(crate) const ENGINE_F0: f64 = 45.0;
/// Detuned partner saw: exactly ONE extra cycle per loop = a 0.25 Hz beat
/// against [`ENGINE_F0`] that makes the idle breathe and still wraps.
pub(crate) const ENGINE_F0_DETUNED: f64 = 45.25;
/// Pulse layer an octave up — firing-order character (360 cycles/loop).
pub(crate) const ENGINE_PULSE_F: f64 = 90.0;
/// Subharmonic sine under-thump at half the fundamental (90 cycles/loop).
pub(crate) const ENGINE_SUB_F: f64 = 22.5;
/// Slow pitch wobble on the saws (3 cycles/loop; a zero-mean sine over
/// whole cycles adds zero net phase, so the oscillators still wrap).
pub(crate) const ENGINE_PITCH_LFO_HZ: f64 = 0.75;
/// Pitch wobble depth in Hz.
pub(crate) const ENGINE_PITCH_LFO_DEPTH: f64 = 0.9;
/// Slow amplitude swell on the master gain (2 cycles/loop).
pub(crate) const ENGINE_AMP_LFO_HZ: f64 = 0.5;
/// Playback gain of the idle loop under the title menu — a low underlay
/// beneath the music, never a lead.
pub(crate) const ENGINE_IDLE_GAIN: f64 = 0.055;

/// Every pre-renderable one-shot SFX voice. The discriminant indexes
/// `BakedSfx::bufs`.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum SfxKind {
    AttackGun,
    AttackMachinegun,
    AttackShotgun,
    AttackClub,
    HitGun,
    HitMachinegun,
    HitShotgun,
    HitClub,
    EnemyDown,
    PlayerHurt,
    Pickup,
    Throw,
    Death,
    LevelClear,
    MaskCrack,
    Elevator,
    /// The looping title-screen car idle (see the car SFX tunables above).
    /// Not a one-shot: played via `AudioEngine::start_engine_idle` /
    /// `AudioEngine::stop_engine_idle` only — never through `play_baked`
    /// — and it has NO live fallback (skipped silently until baked).
    EngineIdle,
    TireScreech,
    CarDoorOpen,
    CarDoorClose,
}

/// All kinds, in pre-render order (the combat sounds first — they are the
/// expensive ones and the ones a firefight needs early; the car sounds
/// last — menu / rare-scenario sounds, the lowest priority tier).
pub(crate) const SFX_KINDS: [SfxKind; 20] = [
    SfxKind::AttackGun,
    SfxKind::AttackMachinegun,
    SfxKind::AttackShotgun,
    SfxKind::AttackClub,
    SfxKind::HitGun,
    SfxKind::HitMachinegun,
    SfxKind::HitShotgun,
    SfxKind::HitClub,
    SfxKind::EnemyDown,
    SfxKind::PlayerHurt,
    SfxKind::Pickup,
    SfxKind::Throw,
    SfxKind::Death,
    SfxKind::LevelClear,
    SfxKind::MaskCrack,
    SfxKind::Elevator,
    SfxKind::EngineIdle,
    SfxKind::TireScreech,
    SfxKind::CarDoorOpen,
    SfxKind::CarDoorClose,
];

/// Where a pre-rendered voice plugs back into the live bus at play time: the
/// same dry input + wet send its live synthesis uses, so the room, compressor
/// and soft-clip behavior stay live and identical (only the *synthesis* is
/// baked — always dry, before the reverb send).
#[derive(Clone, Copy)]
pub(crate) enum SfxRoute {
    /// The gun / hit path (`AudioEngine::voice_real`) at this wet level.
    Real(f64),
    /// The melee / misc. compressed path (`AudioEngine::voice`) at this
    /// wet level.
    Melee(f64),
    /// The bus's pre-wired default room voice (`AudioEngine::sfx_out`).
    Room,
}

/// Per-kind pre-render parameters (mirrors the values inside each `play_*`).
pub(crate) struct SfxSpec {
    /// The live-bus routing reapplied at play time.
    pub(crate) route: SfxRoute,
    /// Seconds of dry voice to render (covers the longest tail incl. jitter;
    /// the reverb tail is added live by the convolver, not baked).
    pub(crate) len: f64,
    /// Per-play `playback_rate` jitter — matches the magnitude of the live
    /// path's top-level per-play pitch jitter (0 for the musical chimes,
    /// which the live path never detunes).
    pub(crate) rate_jitter: f64,
}

impl SfxKind {
    /// The pre-render spec for this kind. `route`/`wet` mirror the `voice*`
    /// call at the top of the kind's `synth_*` builder.
    pub(crate) fn spec(self) -> SfxSpec {
        let (route, len, rate_jitter) = match self {
            SfxKind::AttackGun => (SfxRoute::Real(REAL_762X39.wet), 1.1, 0.05),
            SfxKind::AttackMachinegun => (SfxRoute::Real(REAL_556.wet), 1.4, 0.05),
            SfxKind::AttackShotgun => (SfxRoute::Real(REAL_762X54R.wet), 1.4, 0.05),
            SfxKind::AttackClub => (SfxRoute::Melee(0.16), 0.55, 0.06),
            SfxKind::HitGun => (SfxRoute::Real(0.5), 0.85, 0.05),
            SfxKind::HitMachinegun => (SfxRoute::Real(0.5), 1.35, 0.08),
            SfxKind::HitShotgun => (SfxRoute::Real(0.55), 0.85, 0.05),
            SfxKind::HitClub => (SfxRoute::Real(0.5), 0.95, 0.05),
            SfxKind::EnemyDown => (SfxRoute::Real(0.55), 1.5, 0.05),
            SfxKind::PlayerHurt => (SfxRoute::Melee(0.30), 1.6, 0.05),
            SfxKind::Pickup => (SfxRoute::Room, 0.35, 0.0),
            SfxKind::Throw => (SfxRoute::Room, 0.4, 0.0),
            SfxKind::Death => (SfxRoute::Room, 0.85, 0.0),
            SfxKind::LevelClear => (SfxRoute::Room, 0.6, 0.0),
            SfxKind::MaskCrack => (SfxRoute::Room, 0.55, 0.0),
            SfxKind::Elevator => (SfxRoute::Room, 1.5, 0.0),
            // Warm-up head + loop region; rate jitter would be harmless
            // (a pure transposition keeps the wrap seamless) but the loop's
            // per-start jitter is applied by `start_engine_idle` instead.
            SfxKind::EngineIdle => (
                SfxRoute::Room,
                ENGINE_LOOP_WARMUP + ENGINE_LOOP_SECONDS,
                0.0,
            ),
            SfxKind::TireScreech => (SfxRoute::Melee(0.25), 1.1, 0.06),
            SfxKind::CarDoorOpen => (SfxRoute::Room, 0.55, 0.06),
            SfxKind::CarDoorClose => (SfxRoute::Melee(0.22), 0.7, 0.05),
        };
        SfxSpec {
            route,
            len,
            rate_jitter,
        }
    }
}

/// How many kinds at the head of [`SFX_KINDS`] are baked BEFORE the music
/// voices: the combat sounds (attacks, hits, enemy-down, hurt, pickup,
/// throw) a first firefight needs — the rare tail (death, level-clear,
/// mask-crack, elevator) bakes after the music voices instead.
pub(crate) const SFX_COMBAT_KINDS: usize = 12;

#[cfg(test)]
mod tests {
    use super::*;

    /// The bake-priority split must keep every attack and hit kind in the
    /// combat prefix that renders before the music voices.
    #[test]
    fn combat_sfx_prefix_covers_attacks_and_hits() {
        assert!(SFX_COMBAT_KINDS <= SFX_KINDS.len());
        for kind in &SFX_KINDS[..SFX_COMBAT_KINDS] {
            assert!(!matches!(
                kind,
                SfxKind::Death
                    | SfxKind::LevelClear
                    | SfxKind::MaskCrack
                    | SfxKind::Elevator
                    | SfxKind::EngineIdle
                    | SfxKind::TireScreech
                    | SfxKind::CarDoorOpen
                    | SfxKind::CarDoorClose
            ));
        }
        for combat in [
            SfxKind::AttackGun,
            SfxKind::AttackMachinegun,
            SfxKind::AttackShotgun,
            SfxKind::AttackClub,
            SfxKind::HitGun,
            SfxKind::HitMachinegun,
            SfxKind::HitShotgun,
            SfxKind::HitClub,
        ] {
            let pos = SFX_KINDS.iter().position(|k| *k == combat).unwrap();
            assert!(pos < SFX_COMBAT_KINDS);
        }
    }

    /// The engine idle bakes as a seamless loop by construction: every
    /// periodic component must complete a WHOLE number of cycles over the
    /// loop region, so the waveform phase at `loop_end` equals the phase at
    /// `loop_start` (a zero-mean pitch LFO over whole cycles adds zero net
    /// phase to the oscillators it modulates).
    #[test]
    fn engine_idle_loop_wraps_seamlessly() {
        // Every frequency the idle's synthesis locks to the loop grid.
        for f in [
            ENGINE_F0,
            ENGINE_F0_DETUNED,
            ENGINE_PULSE_F,
            ENGINE_SUB_F,
            ENGINE_PITCH_LFO_HZ,
            ENGINE_AMP_LFO_HZ,
        ] {
            let cycles = f * ENGINE_LOOP_SECONDS;
            assert!(
                (cycles - cycles.round()).abs() < 1e-9,
                "{f} Hz: {cycles} cycles per loop is not whole"
            );
        }
        // The detune beat between the saw pair wraps too: exactly one
        // extra cycle per loop (a 1/ENGINE_LOOP_SECONDS Hz breathing beat).
        let beat_cycles = (ENGINE_F0_DETUNED - ENGINE_F0) * ENGINE_LOOP_SECONDS;
        assert!((beat_cycles - 1.0).abs() < 1e-9);
        // A real warm-up head exists so the lowpass settles onto its
        // periodic steady state before the loop region starts, and the
        // baked buffer covers warm-up + loop exactly.
        let spec = SfxKind::EngineIdle.spec();
        assert!(spec.len - ENGINE_LOOP_SECONDS >= 0.25, "no warm-up head");
        assert_eq!(spec.len, ENGINE_LOOP_WARMUP + ENGINE_LOOP_SECONDS);
        // The loop is transposed per start, never rate-jittered per play
        // through the one-shot path.
        assert_eq!(spec.rate_jitter, 0.0);
    }

    /// The car sounds are menu / rare-scenario sounds: they must sit at the
    /// very tail of the bake queue (after even the other rare one-shots),
    /// and each kind appears exactly once.
    #[test]
    fn car_sfx_bake_last() {
        let pos = |k: SfxKind| SFX_KINDS.iter().position(|x| *x == k).unwrap();
        let tail = [
            SfxKind::EngineIdle,
            SfxKind::TireScreech,
            SfxKind::CarDoorOpen,
            SfxKind::CarDoorClose,
        ];
        for k in tail {
            assert!(pos(k) >= SFX_KINDS.len() - tail.len());
            assert_eq!(SFX_KINDS.iter().filter(|x| **x == k).count(), 1);
        }
        assert!(pos(SfxKind::Elevator) < pos(SfxKind::EngineIdle));
    }
}
