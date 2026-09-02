//! Audio: the song data + the pure sequencer math ([`songs`], generated
//! [`songs_data`]) and the one-shot SFX catalogue ([`sfx`]) are host-compiled
//! and unit-tested natively; the WebAudio engine ([`AudioEngine`]) is
//! wasm-only.
pub mod sfx;
pub mod songs;
#[rustfmt::skip]
pub mod songs_data;

#[cfg(target_arch = "wasm32")]
mod engine;
#[cfg(target_arch = "wasm32")]
pub use engine::AudioEngine;
pub use songs::{song_for_floor, CHANNEL_NAMES, NUM_CHANNELS};
pub use songs_data::SONGS;
