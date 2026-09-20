// Core modules
pub mod math;
pub mod palette;

// Browser integration. `graphics` is the frame RECORDER: only its canvas
// surface is wasm-only, the draw API compiles natively (headless recording
// for tests). `input` and the audio engine really are browser-only.
pub mod audio;
pub mod graphics;
#[cfg(target_arch = "wasm32")]
pub mod input;

// Library module for game logic (enables testing)
pub mod collision;
pub mod components;
pub mod drive;
pub mod ecs;
pub mod editor;
pub mod ending;
pub mod game;
pub mod hud_ammo;
pub mod hud_msg;
pub mod levels;
#[rustfmt::skip]
pub mod levels_data;
pub mod pathfinding;
pub mod props;
#[rustfmt::skip]
pub mod props_data;
pub mod render;
pub mod scenario;
pub mod sim;
pub mod sparks;
pub mod static_geo;
pub mod systems;

// Where the floor hides the void backdrop (pure math, host-tested)
pub mod backdrop_clip;
// Camera and level rendering: they only record into `Graphics`, so they
// build (and are tested) natively too
pub mod camera;
pub mod level;
// The level editor's immediate-mode UI reads the browser input
#[cfg(target_arch = "wasm32")]
pub mod editor_ui;

// The browser app: game state, screens, the frame loop and the wasm entry
#[cfg(target_arch = "wasm32")]
mod app;
// What the engine exports to the JS tool pages (poses)
#[cfg(target_arch = "wasm32")]
mod wasm_api;
