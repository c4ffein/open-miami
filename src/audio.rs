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
pub use songs::{
    ending_song, note_name, scale_name, song_for_floor, title_song, voice_summary, GridCell,
    CHANNEL_NAMES, MAX_VEL, MELODIC, NUM_CHANNELS, SONGS,
};
