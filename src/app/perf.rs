//! Per-frame performance tracing (`?perf`): thin externs to the collector
//! in index.html (`window.__perf`). Every entry point checks [`enabled`]
//! first, so a run without the flag never crosses the wasm->JS boundary
//! (the JS side guards again, belt and braces).

use wasm_bindgen::prelude::*;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = window, js_name = perfSpan)]
    fn js_span(name: &str, start: f64, dur: f64);
    #[wasm_bindgen(js_namespace = window, js_name = perfFrameStart)]
    fn js_frame_start(t: f64);
    #[wasm_bindgen(js_namespace = window, js_name = perfFrameEnd)]
    fn js_frame_end(t: f64);
}

thread_local! {
    /// Read once from the URL on first use; wasm is single-threaded.
    static ENABLED: bool = super::url_flag("perf");
}

pub fn enabled() -> bool {
    ENABLED.with(|e| *e)
}

/// Same clock as the game loop's `performance.now()`.
fn now() -> f64 {
    web_sys::window()
        .and_then(|w| w.performance())
        .map(|p| p.now())
        .unwrap_or(0.0)
}

/// Open a trace frame at the rAF timestamp the loop already has.
/// Only called on frames that actually run (the FPS cap's skipped
/// frames must not open frames).
pub fn frame_start(t: f64) {
    if enabled() {
        js_frame_start(t);
    }
}

/// Close the trace frame (computes its own end timestamp).
pub fn frame_end() {
    if enabled() {
        js_frame_end(now());
    }
}

/// An open span; dropping it reports `[name, start, dur]` to the
/// collector — so it survives early returns. [`span`] returns `None`
/// when tracing is off: no clock read, no boundary crossing.
pub struct Span {
    name: &'static str,
    start: f64,
}

impl Drop for Span {
    fn drop(&mut self) {
        js_span(self.name, self.start, now() - self.start);
    }
}

pub fn span(name: &'static str) -> Option<Span> {
    enabled().then(|| Span { name, start: now() })
}
