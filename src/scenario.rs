//! Floor scenarios: static definitions (generated into `levels_data.rs` from
//! `levels/*.json` by `tools/gen_levels.py`) and the runtime that plays
//! them — triggers that fire once, timers, spawn waves, exit doors, the
//! objective line and the intercepted-comms feed.
//!
//! The runtime is pure engine state (no rendering, no browser) so it is
//! testable headlessly and shared by the wasm loop and the tests.

mod comms;
mod defs;
mod state;
mod world;

pub use comms::*;
pub use defs::*;
pub use state::*;
pub use world::*;

#[cfg(test)]
mod tests;
#[cfg(test)]
mod tests_gates;
