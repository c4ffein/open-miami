# History — how things got the way they are

The record of finished work, rejected experiments and removed tooling. Nothing
here is a RULE: the rules are in `CLAUDE.md`, the standing design is in
[ARCHITECTURE.md](ARCHITECTURE.md) / [RENDERING.md](RENDERING.md). Read this
page when a rule looks arbitrary and you want the evidence before questioning
it, or before re-trying something that was already tried. The sessions
themselves are in `docs/transcripts/`.

## Art direction: the antialiasing experiments (all rejected)

The rule — NO antialiasing of any kind, the aliasing is the HM2 look — was
decided after building the alternatives, not before:

- **MSAA (`?aa=1`)**: antialiased the walls beautifully AND dropped the 2018
  MacBook Air to 30 fps — 4x bandwidth per full-screen layer at Retina.
  `antialias: false` stays.
- **A texel-snap AA shader on the world composite** (sampling-side
  smoothing): built, verified pixel-perfect, and REMOVED on purpose — soft
  edges on a rotated pixel image are off-vibe even when they are correct.
- **FXAA**: never shipped; it would soften the sprite / text crunch, which is
  the look.
- **`?grain=fold`** (folding the TV static into the batch fragment shader to
  save the grain quad's full-screen layer): a TRADE, not a win — the layer
  goes (8.6 -> 6.8 ms GPU on the 2018 MacBook Air) but the second texture
  fetch makes every batch fragment ~47% dearer, break-even around 2
  full-screen layers of batch fill. Kept opt-in, pixel-proven by
  `tests/e2e/render/grain-fold.js`; full numbers in RENDERING.md. Two lessons:
  a fetch added to the batch shader taxes every fragment of every layer, and
  the FIRST verdict ("clear loss") was taken with the probe panel over the
  canvas and was void — a DOM element over the canvas changes what is being
  measured.

What came out of it and IS built: `?pixel=N` runs the world through a
world-anchored sub-pixel-composite group (the `smooth` flag = sub-pixel
PLACEMENT, never soft sampling), and the static geometry cache draws inside
it.

## The characters roadmap (COMPLETE) — moved here verbatim from ARCHITECTURE.md

STATUS: DONE, every step below. This section is kept as the record of WHY
the characters are built the way they are and HOW each move was proven — read
it before touching `render/pose.rs`, `render/shoggoth.rs`, `web/robot-core.js`
or `web/shoggoth-core.js`. For what is still open, see "What's next" at the
end of this page.

WHERE IT STARTED: the robots and the boss were the one place where the
layering was not settled — their ANIMATION (what the body does at time `t`)
lived in JS, outside the native test boundary. The target was one rule for
both characters:

> **Rust computes poses (numbers). GLSL evaluates the rig. JS only ferries.**

Neither extreme is the goal. "All Rust" is impossible — the skeleton and
the inking must run on the GPU, so they stay GLSL. "All JS" is today, and
it has a real coupling: Rust owns the finisher timers and impact frames
(`FinisherKind::impacts`), JS owns the kick / stomp keyframes that must line
up with them — gameplay choreography mirrored across two languages, pinned by
nothing. What moves is only the middle piece: `(pose, time)` -> joint
numbers, which by the test above is render-layer code.

Where each character STOOD at the start (historical):

- **Robots — the seam already existed.** `posePlan(pose, time, relaxed)` in
  `web/robot-core.js` is a PURE ~150-line function (no state, no randomness,
  no browser) returning a handful of scalars, and the GPU rig (`rigVS`)
  already consumes exactly that: 16 floats per instance. The interface is
  there; it just sits between two pieces of JS.
- **The boss — no seam yet.** `web/shoggoth-core.js` builds a matrix per
  sphere on the CPU and issues one draw per sphere; there is no "pose
  numbers" layer to cut along. (The command is already minimal — 6 floats,
  `x y size heading reveal time` — so this is about where the expansion
  runs, not about stream size.) (This page first claimed here that its
  small wander state machine was only the inspector's preview. WRONG — see
  step 3: the game takes the mask's look-up beats from it.)

Planned order (each step lands with its test FIRST):

1. **Robots: port `posePlan` to Rust** — DONE, in three sub-steps:
   - **R1 (DONE): the port + the proof, no runtime change.**
     `src/render/pose.rs` (`pose_plan`, `PoseKind`, `Pose`) reproduced the JS
     BIT-EXACTLY: `tests/fixtures/pose_plan.txt` was generated FROM
     `posePlan()` (every pose x relaxed x 54 times) and all 9,504 scalars
     compared (measured: max difference 0). The kick / stomp timing DERIVES
     from `FinisherKind::impacts()` and is tested ("the foot is fully
     extended on the impact", "each stomp lands on its impact").
   - **R2 (DONE): the game uses it.** The `ROBOT` op is `colorIdx weaponIdx
     flags x y angle sizePx` + the 11 pose scalars (18 args; flags bit 0 =
     the gun hand aims, bit 1 = headless): `render/robots.rs` calls
     `pose_plan`, the renderer unpacks with `planFromScalars` and
     `batchDraw` takes the ready-made `opts.plan` — the game runs NO JS
     animation any more, and renderer.js lost its `ROBOT_POSES` table. The
     scalar ORDER is one JS list, `POSE_SCALARS` (web/robot-core.js), which
     drives both the renderer's unpack and the fixture's columns; Rust is
     held to it by `scalar_order_matches_the_js` (+ `flag_bits_match_the_js`). Since R1 proved the two pose
     functions bit-identical, the game's robots are unchanged by
     construction. JS `posePlan()` now only serves the tools + the bakes.
   - **R3 (DONE): the JS copy is DELETED — one pose implementation.**
     `posePlan()` and the `POSES` name list are gone from web/robot-core.js,
     whose `render()` / `batchDraw()` / `bakeSprite()` now REQUIRE
     `opts.plan`. The three places that had no engine got one: the portrait
     bake receives its pose IN the `PORTRAIT` op (`portrait_pose()`, 11
     scalars + flags — renderer.js stays wasm-free, as the render-tests
     harness needs), and `tools/inspector.html` + `tools/rig-parity.html`
     load the wasm through `tools/engine-pose.js` and call the exports of
     `src/wasm_api.rs` (`pose_names`, `pose_plan_scalars` — the ENGINE also
     decides "unarmed = at ease", so that rule is not duplicated either).
     Loading the wasm does not start the game (`start` is explicit). The
     fixture is now a FROZEN golden record (`matches_the_golden_record`); its
     generator and `make gen-pose` / `check-pose` are gone with the JS they
     read. Pins: `scalar_order_matches_the_js`, `flag_bits_match_the_js`,
     `no_js_pose_logic_is_left`. Cost accepted: pose look-dev in the
     inspector needs `make build-wasm` (so do the tools at all).

   Payoff: choreography in one language, host-testable.
2. **Boss: instanced spheres, still in JS** — DONE. Every sphere the boss
   draws went through one choke point (`_sphere(model, colour, accent, id,
   emission)`), so a sphere became 20 per-INSTANCE floats (the model's three
   rows + two colour vec4s) and a frame became TWO instanced draws: the body
   with the depth test on, then the mask assembly with it OFF, in submission
   order — the mask is drawn depth-off on purpose, so ONE draw was not an
   option. MEASURED on the parity page: the per-sphere path submitted up to
   227 draws a frame (124 on average, six uniform uploads each); the
   instanced path submits 2. The original path stays as the REFERENCE
   (`pipe.instanced = false`; also the fallback without
   ANGLE_instanced_arrays), exactly like the robots' CPU rig. The "normal
   matrix" is the model's upper 3x3 as is (not an inverse-transpose — the
   look was tuned with it), derived in the shader from the same rows.
   `tools/shoggoth-parity.html` + `tests/e2e/render/shoggoth-parity.js` (in
   `make check-render`) render the whole mask-off arc x clocks / headings +
   the orbit camera, the wander drift and the fine tessellation through
   both, at the game's tile settings: 45 frames, 0 differing pixels; they
   also assert <= 2 scene draws, no GL error and NO instancing divisor left
   on the context. Mutation-tested (a shifted sphere; the mask drawn WITH
   depth) — both fail it. This is the boss's FIRST pixel test, and the seam
   step 3 needs: the instance list is what Rust will fill.
3. **Boss: move the sphere placement to Rust** — DONE.
   `src/render/shoggoth.rs` (`boss_spheres(&BossPose) -> BossSpheres`) places
   every sphere — mass, tentacles, dot eyes, the mask and its break-up — AND
   runs the WANDER behaviour. (A correction to an earlier belief: that little
   state machine is not an inspector toy. The game never passes `lookUp`, so
   the mask's "stop and look up" beats in-game come from it; it was ported
   faithfully, and made a pure function of time — the JS cached its state and
   so depended on the order it was queried in.) Proven BIT-EXACT before the JS
   was deleted: the placement code was run under Bun with a mock GL and its
   sphere queue captured (13 frames, 1,609 spheres, 32,180 floats — max
   difference 0); `tests/fixtures/shoggoth_spheres.txt` is the compact frozen
   golden record (`matches_the_golden_record`). What made bit-exactness
   possible, and must be kept: the JS matrices lived in `Float32Array`s, so
   every product was ROUNDED TO f32 at each step (mirrored in Rust), and the
   original's truncated literals (`6.283`, not `TAU`) are kept on purpose.
   WIRE FORMAT: a run of `SPHERE` ops (20 floats each) closed by
   `SHOGGOTH x y sizePx maskAt`, which consumes it — fixed-arity ops, so every
   stream walker keeps working; `graphics::stream::check` validates the run
   (closed, never interrupted, split inside it). web/shoggoth-core.js went
   620 -> 337 lines: camera, shading, the two draw paths, NO animation — its
   instanced path uploads the engine's list AS the instance buffer.
   `BOSS_MASK_OFF_SECS` is now the one definition (the JS constant and its
   pin are gone). Tools: `bossSpheres()` / `MASK_OFF_SECS` in
   tools/engine-pose.js -> `src/wasm_api.rs`. Pins:
   `the_sphere_layout_matches_the_op_and_the_js`,
   `no_js_boss_animation_is_left`.
4. **Tools load the wasm for poses** — DONE, robots (R3) and boss (step 3).

Costs accepted knowingly: look-dev on a pose goes from edit + refresh to a
wasm rebuild (15 s release today; a dev-profile build should cut that — not
yet measured), and the tools pages gain a wasm dependency. Decide robots and
boss TOGETHER — moving only one leaves two conventions, which is worse than
either.

THE ROADMAP IS COMPLETE: no character animation is left in JS. What the JS
still decides about a character is how it is LIT, INKED and FRAMED — renderer
questions by the layering above.

## Findings of the renderer-only pixel tests (2026-09)

Writing `postfx-kinds` / `text-glyphs` / `drive-backdrop` (docs/TESTING.md)
turned up one real mismatch and two non-bugs worth remembering:

- **Unknown POSTFX kinds were NOT no-ops.** `Graphics::postfx` documents "any
  other kind is a no-op", but a kind > 13 (or < 0) fell into the post
  shader's last `else` — the MODAL STATIC branch, reading the colour as
  panel extents. The game never emits one, so nothing was visible. Fixed in
  `frameRender` (the frame is no longer routed through the scene target).
- **Zeroing the glyph atlas on reset: tried, measured, removed.** The atlas
  is sampled LINEAR and its reset leaves the old texels in place, which
  looks like a bleed bug for magnified text. It is not: VT323 cells all have
  one width, every generation lays out on the same grid, and a stale
  neighbour presents its transparent padding. Zeroing changed no pixel
  (A/B, magnified text included). What DOES differ after a reset: up to +-1
  on a channel for a few dozen px — float rounding of the interpolated UVs
  at another atlas position. The test's tolerance is 1 for that reason.
- **DRIVE's torn band keeps one art column of scene in the vacated slice**:
  the shader's void test is `p.x < -0.5 * uPx` (a cell whose centre lands
  exactly half a cell outside still samples). By design; the test allows it.

## Removed tooling (so nobody looks for it)

- `build-wasm.sh` — replaced by `make build-wasm` (installs the wasm32 target
  and `wasm-bindgen-cli` pinned to Cargo.lock's `wasm-bindgen` version).
- `make gen-pose` / `check-pose` and the pose fixture's generator — gone with
  the JS `posePlan()` they read (roadmap step R3); the fixture is frozen.
- The JS `BOSS_MASK_OFF_SECS` constant and its pin — one definition now, in
  Rust (roadmap step 3).
- Two pasted copies of the opcode table in the test suites — they read
  `web/ops.js` now, and a guard fails on a new paste.
- CI's coverage / tarpaulin job — it compiled cargo-tarpaulin from source on
  every run (~5+ min, uncached) and could never fail. `make check-coverage`
  still exists locally.

## Incidents

- **The forked session (2026-09, during the boss port).** A dropped SSH
  connection left a `claude` session running unreachable; `claude --continue`
  then started a SECOND agent on the same working tree, and the two even
  shared one transcript (`docs/transcripts/architecture-refactor-session*.json`).
  Housekeeping that is not code: run `claude` inside `tmux` on the dev box.
- **CI's clippy job broke without a code change (2026-09).** CI installs
  `@stable`; Rust 1.98 added `clippy::chunks_exact_to_as_chunks`, which fired
  on the SPHERE loop in `Graphics::draw_shoggoth_live` (written on 1.96,
  where `make verify` was green). Fixed with `as_chunks`. To reproduce a
  CI-only lint: `rustup toolchain install <version>` and
  `cargo +<version> clippy …` — no need to move the default toolchain.
