//! Audio: the composable song-authoring layer ([`compose`]), the songs
//! themselves (`songs/*.rs`, collected in [`SONGS`]), the pure sequencer
//! math ([`songs`]) and the one-shot SFX catalogue ([`sfx`]) are
//! host-compiled and unit-tested natively; the WebAudio engine
//! ([`AudioEngine`]) is wasm-only.
pub mod compose;
pub mod sfx;
pub mod songs;

#[cfg(target_arch = "wasm32")]
mod engine;
#[cfg(target_arch = "wasm32")]
pub use engine::AudioEngine;
pub use songs::{ending_song, song_for_floor, title_song, CHANNEL_NAMES, NUM_CHANNELS, SONGS};
