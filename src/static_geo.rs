//! STATIC GEOMETRY CACHE — the bookkeeping half, host-testable.
//!
//! The floor tiles and walls do not change between frames, yet they used to be
//! re-recorded into the command stream (and re-tessellated by renderer.js's
//! CPU opcode walk) every frame — most of the ~2400 floats a frame chewed.
//! Opcodes 21/22/23 fix that with a build-once contract:
//!
//!   STATIC_BEGIN key   (21) — renderer.js tessellates everything until the
//!                       matching STATIC_END into a PERSISTENT VBO stored
//!                       under `key`, in WORLD coordinates (the CPU transform
//!                       is treated as the camera transform and excluded),
//!                       and also draws it this frame. One key is live at a
//!                       time; a new key evicts (deletes) the old buffer.
//!   STATIC_END         (22) — closes the recording.
//!   STATIC_REF key     (23) — draw the cached VBO for `key`: the renderer
//!                       applies its current CPU-side transform (the camera)
//!                       in the vertex shader via a uniform, so the cache is
//!                       valid for every camera position/zoom/sway.
//!
//! The wasm side ([`crate::graphics::Graphics::static_layer`]) emits
//! BEGIN..content..END the first time it sees a key and a 2-float REF on
//! every later frame; [`StaticKey`] is that decision, and
//! [`open_ops`]/[`close_ops`] are the exact framing ops — both shared with
//! the wasm path and unit-tested here on the host.
//!
//! Constraints (by design, see CLAUDE.md):
//!   - a static section must contain SOLID primitives only (rects, circles,
//!     lines, arcs — everything that samples the white texture): text and
//!     sprites would be baked with the wrong texture;
//!   - the section is recorded UNCULLED (the whole floor) so the cache is
//!     valid wherever the camera goes — the GPU clips off-screen quads;
//!   - callers must bypass the cache (plain draws, no static ops) whenever
//!     the section's content is not frame-invariant (kill-flash tint, debug
//!     wall overlays) or is being redirected into a pixel group (`?pixel=N`).

/// Opcode values — mirror of `mod op` in src/graphics.rs, OP_ARGS in
/// renderer.js and tests/e2e/specs/helpers.js. Keep in sync.
pub const OP_STATIC_BEGIN: f32 = 21.0; // key
pub const OP_STATIC_END: f32 = 22.0; //
pub const OP_STATIC_REF: f32 = 23.0; // key

/// Which key has been emitted in full (BEGIN..content..END) to the renderer.
/// The renderer's cache and this tracker live and die together (one page,
/// one `Graphics`), so "already emitted" == "the renderer holds the VBO".
#[derive(Default)]
pub struct StaticKey {
    emitted: Option<u32>,
}

impl StaticKey {
    pub const fn new() -> Self {
        StaticKey { emitted: None }
    }

    /// Decide one static-layer emission: `true` = the caller must record
    /// BEGIN + the full content + END (first sight of `key`, or a key
    /// change = cache invalidation); `false` = a REF suffices.
    pub fn needs_record(&mut self, key: u32) -> bool {
        if self.emitted == Some(key) {
            false
        } else {
            self.emitted = Some(key);
            true
        }
    }
}

/// The ops that OPEN one static-layer emission: `[STATIC_BEGIN, key]` when
/// recording, `[STATIC_REF, key]` when the cache is warm.
pub fn open_ops(record: bool, key: u32) -> [f32; 2] {
    if record {
        [OP_STATIC_BEGIN, key as f32]
    } else {
        [OP_STATIC_REF, key as f32]
    }
}

/// The ops that CLOSE it: `[STATIC_END]` when recording, nothing for a REF.
pub fn close_ops(record: bool) -> &'static [f32] {
    if record {
        &[OP_STATIC_END]
    } else {
        &[]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// One `Graphics::static_layer` call, mirrored on a plain Vec: the same
    /// tracker decision + framing ops the wasm path pushes, with `content`
    /// standing in for the section's draw commands.
    fn emit(tracker: &mut StaticKey, key: u32, content: &[f32], out: &mut Vec<f32>) {
        let record = tracker.needs_record(key);
        out.extend_from_slice(&open_ops(record, key));
        if record {
            out.extend_from_slice(content);
        }
        out.extend_from_slice(close_ops(record));
    }

    #[test]
    fn static_layer_stream_layout() {
        // First frame: BEGIN key, the full content, END — exactly.
        let mut tracker = StaticKey::new();
        let content = [1.0, 10.0, 20.0, 30.0, 40.0, 0.5, 0.5, 0.5, 1.0]; // one RECT
        let mut frame = Vec::new();
        emit(&mut tracker, 7, &content, &mut frame);
        let mut expected = vec![OP_STATIC_BEGIN, 7.0];
        expected.extend_from_slice(&content);
        expected.push(OP_STATIC_END);
        assert_eq!(frame, expected);
    }

    #[test]
    fn static_layer_records_then_refs() {
        // Second frame with the same key: a 2-float REF, no content.
        let mut tracker = StaticKey::new();
        let content = [1.0, 0.0, 0.0, 50.0, 50.0, 1.0, 1.0, 1.0, 1.0];
        let mut first = Vec::new();
        emit(&mut tracker, 3, &content, &mut first);
        let mut second = Vec::new();
        emit(&mut tracker, 3, &content, &mut second);
        assert_eq!(second, vec![OP_STATIC_REF, 3.0]);
        // And it stays a REF for every later frame.
        let mut third = Vec::new();
        emit(&mut tracker, 3, &content, &mut third);
        assert_eq!(third, vec![OP_STATIC_REF, 3.0]);
    }

    #[test]
    fn static_layer_rerecords_on_key_change() {
        // A new key (floor load bumps the revision) re-records in full —
        // and the old key would re-record too if it ever came back.
        let mut tracker = StaticKey::new();
        let content = [1.0, 0.0, 0.0, 50.0, 50.0, 1.0, 1.0, 1.0, 1.0];
        let mut out = Vec::new();
        emit(&mut tracker, 1, &content, &mut out);
        out.clear();
        emit(&mut tracker, 2, &content, &mut out);
        assert_eq!(out[0], OP_STATIC_BEGIN);
        assert_eq!(out[1], 2.0);
        assert_eq!(*out.last().unwrap(), OP_STATIC_END);
        // Going back to key 1: the renderer evicted it, so record again.
        out.clear();
        emit(&mut tracker, 1, &content, &mut out);
        assert_eq!(out[0], OP_STATIC_BEGIN);
    }
}
