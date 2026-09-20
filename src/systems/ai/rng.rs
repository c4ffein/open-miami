//! The AI's deterministic RNG helpers (state lives in the `World`).

// Pseudo-random number generation. The RNG state lives in the `World` (so every
// `Simulation` is independently reproducible); these free helpers operate on a
// borrowed copy of that state threaded through the AI update. They use the same
// LCG constants as the world so the produced sequence is unchanged.
pub(super) fn next_random(state: &mut u32) -> u32 {
    *state = state.wrapping_mul(1664525).wrapping_add(1013904223);
    *state
}

/// Random float between min and max.
pub(super) fn random_range(state: &mut u32, min: f32, max: f32) -> f32 {
    let r = next_random(state) as f32 / u32::MAX as f32;
    r * (max - min) + min
}

/// Random integer between min and max (inclusive).
pub(super) fn random_int_range(state: &mut u32, min: i32, max: i32) -> i32 {
    let range = (max - min + 1) as u32;
    // High bits, like `World::random_int_range` (the LCG's low bits cycle
    // with tiny periods).
    min + ((next_random(state) >> 16) % range) as i32
}

/// A fair coin: the LCG's top bit (bit 0 strictly alternates).
pub(super) fn coin_flip(state: &mut u32) -> bool {
    next_random(state) >> 31 == 1
}
