//! Audio: the composable song-authoring layer ([`compose`]), the songs
//! themselves (`songs/*.rs`, collected in [`SONGS`]), the pure sequencer
//! math ([`songs`]) and the one-shot SFX catalogue ([`sfx`]) are
//! host-compiled and unit-tested natively. The WebAudio engine
//! ([`AudioEngine`]) SHIPS on wasm only, but it names every Web Audio type
//! through `engine/webaudio.rs` — `web_sys` re-exports on wasm, a recording
//! mock under `cargo test` — so its voice builders run natively too and the
//! node graphs they build are host-tested (`engine/tests.rs`).
pub mod compose;
pub mod sfx;
pub mod songs;
pub mod voice;

#[cfg(any(target_arch = "wasm32", test))]
mod engine;
#[cfg(any(target_arch = "wasm32", test))]
pub use engine::AudioEngine;
/// The SETTINGS "MUSIC" row's cycle, in percent: 100 → 75 → 50 → 25 → 0 →
/// 100 … (anything off the grid steps down to the next quarter).
pub fn next_music_percent(pct: u32) -> u32 {
    match pct.min(100) {
        0 => 100,
        p => (p - 1) / 25 * 25,
    }
}

/// A saved `om.music` value as a level `0.0..=1.0` (`None` = not a number:
/// keep the default).
pub fn music_level_from_setting(saved: &str) -> Option<f64> {
    let pct: u32 = saved.trim().parse().ok()?;
    Some(f64::from(pct.min(100)) / 100.0)
}

pub use songs::{
    ending_song, note_name, scale_name, song_for_floor, title_song, voice_summary, GridCell,
    CHANNEL_NAMES, MAX_VEL, MELODIC, NUM_CHANNELS, SONGS,
};

#[cfg(test)]
mod tests {
    use super::*;

    /// The MUSIC row walks the quarters down and wraps; a saved value is a
    /// percentage, clamped, and garbage keeps the default.
    #[test]
    fn the_music_setting_cycles_and_parses() {
        let mut pct = 100;
        let mut seen = vec![pct];
        for _ in 0..5 {
            pct = next_music_percent(pct);
            seen.push(pct);
        }
        assert_eq!(seen, [100, 75, 50, 25, 0, 100]);
        assert_eq!(next_music_percent(60), 50);
        assert_eq!(next_music_percent(1), 0);
        assert_eq!(next_music_percent(900), 75);
        assert_eq!(music_level_from_setting("75"), Some(0.75));
        assert_eq!(music_level_from_setting(" 0 "), Some(0.0));
        assert_eq!(music_level_from_setting("250"), Some(1.0));
        assert_eq!(music_level_from_setting("off"), None);
        assert_eq!(music_level_from_setting("-3"), None);
    }
}
