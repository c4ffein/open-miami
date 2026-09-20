# Architecture — the four layers

Open Miami is four layers. Each answers ONE question, and the question
decides where new code goes. (The frame's data flow — command stream, render
targets, caches — is [PIPELINE.md](PIPELINE.md); this page is about where
CODE lives and why.)

```mermaid
flowchart LR
    subgraph native["compiles natively — tested with cargo test (~1 s)"]
        SIM["SIM<br/>what is the state?"]
        RENDER["RENDER<br/>what does the state look like?"]
    end
    subgraph browser["browser only — tested with Playwright (~50 s)"]
        APP["APP<br/>what happens this frame?"]
        JS["RENDERER (JS / WebGL)<br/>how do commands become pixels?"]
    end
    APP -- "input + dt" --> SIM
    SIM -- "read-only state" --> RENDER
    APP -- "builds the view, picks the screen" --> RENDER
    RENDER -- "f32 command stream" --> JS
```

| Layer | Question | May touch | May NOT touch | Where |
|---|---|---|---|---|
| **sim** | What is the state of the world? | the `World`, its components, `dt` | `Graphics`, input, the browser, wall-clock time | `ecs/`, `components/`, `systems/`, `scenario.rs`, `game.rs`, `sim.rs`, `pathfinding.rs`, `collision.rs`, `hud_ammo.rs` / `hud_msg.rs` (HUD *state*), `editor.rs` (the editor *document*) |
| **render** | What does this state look like? | state READ-ONLY + `&Graphics` | input, mutation of game state, the browser, audio | `render.rs`, `render/` (`world`, `hud`, `robots`, `comms`, `dialogue`, `floor_props`, `title`), `level.rs`, `camera.rs`, the `draw` submodules of `props.rs` / `drive.rs` / `ending.rs`, `sparks::render_sparks` |
| **app** | What happens THIS FRAME? | everything: input, the clock, audio, settings, the URL, `&mut GameState` | — (but it should hold no drawing of its own, see below) | `app.rs`, `app/` (wasm-only), `editor_ui.rs`, `input.rs`, `audio/engine.rs` |
| **renderer** | How do commands become pixels? | the GPU | game state (it only ever sees the stream) | `web/`: `renderer.js` + `renderer/shaders.js`, `ops.js`, `robot-core.js`, `shoggoth-core.js`, `gpu-probe.js` |

## The test for "is this render code?"

> It is a function from state to draw commands: it reads no input, mutates
> nothing, calls no browser API.

If yes, it belongs in the render layer, takes its inputs as plain arguments
(or a view struct such as `render::world::WorldView`), and gets a native
stream test. If no — it decides *which* screen, *when*, reacts to a click,
starts a sound, saves a setting — it is app code.

Two consequences worth stating, because both were real bugs of layering once:

- **Render code never samples input.** The player's SHOOT pose depends on
  the fire button; the app samples it and passes `player_firing: bool`.
- **Render code never reads the mouse either.** The pixel crosshair is drawn
  at `HudView::cursor`; the app samples `input::mouse_position()`.
- **Render code never advances a timer.** The kill flash counts down in the
  app (`GameState::render_world`, the wrapper); the renderer receives the
  seconds left. Same for spark expiry.

**Immediate-mode UI is app code**, legitimately: a button's draw and its
click test are one call (`viz_button`, the menus, `editor_ui`). Those pages
interleave input and drawing by design and stay in `app/`.

## Why the line is drawn there: it is the test boundary

`Graphics` is a RECORDER with two surfaces — the browser canvas (wasm) and
`Graphics::new_headless(w, h)` + `take_frame()` (native). Everything on the
native side of the diagram runs under `cargo test`:

- `src/graphics/stream.rs` decodes a recorded frame (`walk`), validates it
  structurally (`check`: opcode arity, finite floats, balanced SAVE/RESTORE
  and pixel groups within `PIX_DEPTH`, framed solid-only static sections,
  valid TEXT indices) and mirrors the JS transform stack (`Affine`). Its
  tests also pin the Rust opcode table to `renderer.js` and the e2e helpers.
- `tests/render_stream.rs` records the REAL `render_world` on every floor
  (cached / referenced / debug / kill-flash frames, `?pixel=2|3|6`), the
  REAL `render_hud` (every floor's opening scenario — dialogue, captions,
  gate prompts —, death / debug / restart / extraction-card states) and
  every prop at every art-pixel size.

So the rule is not aesthetic: **code in the render layer costs ~1 s to
verify, code in the app layer costs a ~50 s browser round trip and can only
be checked by pixels.** Keep the app layer thin; do not gate a module
`cfg(target_arch = "wasm32")` merely because it takes a `&Graphics`.

What genuinely needs the browser, and nothing else should: `input.rs`
(DOM events), `audio/engine.rs` (WebAudio), `editor_ui.rs` + `app/` (they
read input), and the `Graphics` surface methods (`new`, `sync_size`,
`flush`).

## Inside the app layer

`app.rs` owns `GameState` (fields private to the `app` tree), the screen
dispatch (`update`), floor load / checkpoints, and the wasm entry (`start`
+ the `requestAnimationFrame` loop). Submodules `use super::*` and add
their own `impl GameState` blocks:

| Module | Holds |
|---|---|
| `game_loop` | `update_game`: input → `sim::GameSystems::step` → scenario / event / audio bridge → builds the `HudView` → transitions; the boss intro; the ending |
| `world_render` | the WRAPPER around `render::world::render_world`: the frame's two state changes + building the `WorldView` |
| `menus` | level select, the modal chrome, SETTINGS / ABOUT / PAUSE |
| `viz`, `viz/{effects,props_page,musics}` | the `?viz` toolbox |
| `url`, `perf` | query parameters, the `?perf` spans |

**Every frame must draw a screen.** A screen's `update_*` handles input AND
draws; a transition (`self.screen = …`, `pause_in_settings = …`) takes effect
AFTER the current screen has been drawn — record the intent, draw, then
switch. Switching and returning early ships a frame of NEITHER screen: a bare
CLEAR on the title, or — from PAUSED, where the world is already recorded —
the raw world with no modal and no static, a visible one-frame flash. Do not
"fix" that by re-dispatching to the new screen in the same frame: it would
see the same key press (Esc would pause and un-pause at once).
`tests/e2e/specs/menu-transitions.spec.js` drives the menus and requires
that no frame of the sequence lacks its POSTFX.

The simulation tick itself is shared, not duplicated: the browser loop and
the headless `sim::Simulation` both call `sim::GameSystems::step` /
`sim::gate_frozen_step`, so gameplay verified natively is the gameplay that
ships.

## Known debt (honest list)

- `update_game` (src/app/game_loop.rs) is still one long function, but it
  no longer DRAWS: input -> the shared tick -> the event / audio bridge ->
  building `WorldView` + `HudView` -> screen transitions. Both frames it
  shows are pure render-layer functions (`render::world::render_world`,
  `render::hud::render_hud`). What is left to split is orchestration
  (input handling, the event-to-sound bridge) — app code by nature, only
  reachable by Playwright.
- `web/renderer.js`'s `initRenderer` is one ~1,650-line closure, and that is
  a DECISION, not debt to pay down blindly. The dependency graph was
  measured before deciding: ~30 mutable closure variables (`m` — which
  `tSave` / `tRestore` REASSIGN —, `vCount`, `pix`, `batchFbo`, `boundTex`,
  …) are touched by nearly all 54 inner functions, and `vert` — called per
  vertex — reads four of them. Splitting it into modules means a shared
  context object: every one of those reads becomes a property load in the
  hottest loop of a renderer tuned on a fill-rate-poor GPU, across code
  whose pixels are only partly under test (postfx kinds, text, the drive
  have no pixel test). What WAS pure data is out: the GLSL
  (`renderer/shaders.js`) and the opcode table (`ops.js`). A further split
  should start with the subsystems that own their state — postfx + warp,
  drive + backdrop, the glyph atlas — as factories, each with a pixel test
  first.
- `audio/engine.rs` + `audio/engine/*` is split by concern but stays
  browser-only: unlike `Graphics` it is not a recorder — it builds live
  WebAudio node graphs — so none of the SFX / voice recipes are host-tested
  (the sequencer, song data and bake specs in `audio/songs.rs` /
  `compose.rs` / `sfx.rs` are). A recording `AudioGraph` seam would fix
  that; it is a real design change, not a move.
- Mirrored constants without a test yet: the drive scene geometry
  (`drive.rs` ↔ `DRIVE_FS`) and the robot rig's rotation order
  (`leg()` / `arm()` ↔ `rigVS`, covered by `rig-parity.js` in the browser).

## Roadmap — characters: Rust computes poses, GLSL evaluates rigs, JS ferries

The robots and the boss are the one place where the layering is not settled:
their ANIMATION (what the body does at time `t`) lives in JS, outside the
native test boundary. The target is one rule for both characters:

> **Rust computes poses (numbers). GLSL evaluates the rig. JS only ferries.**

Neither extreme is the goal. "All Rust" is impossible — the skeleton and
the inking must run on the GPU, so they stay GLSL. "All JS" is today, and
it has a real coupling: Rust owns the finisher timers and impact frames
(`FinisherKind::impacts`), JS owns the kick / stomp keyframes that must line
up with them — gameplay choreography mirrored across two languages, pinned by
nothing. What moves is only the middle piece: `(pose, time)` -> joint
numbers, which by the test above is render-layer code.

Where each character stands:

- **Robots — the seam already exists.** `posePlan(pose, time, relaxed)` in
  `web/robot-core.js` is a PURE ~150-line function (no state, no randomness,
  no browser) returning a handful of scalars, and the GPU rig (`rigVS`)
  already consumes exactly that: 16 floats per instance. The interface is
  there; it just sits between two pieces of JS.
- **The boss — no seam yet.** `web/shoggoth-core.js` builds a matrix per
  sphere on the CPU and issues one draw per sphere; there is no "pose
  numbers" layer to cut along. (The command is already minimal — 6 floats,
  `x y size heading reveal time` — so this is about where the expansion
  runs, not about stream size.) Its small wander state machine is the
  inspector's standalone preview, not game logic: in-game Rust sends the
  real `heading`.

Planned order (each step lands with its test FIRST):

1. **Robots: port `posePlan` to Rust** — IN PROGRESS, in three sub-steps:
   - **R1 (DONE): the port + the proof, no runtime change.**
     `src/render/pose.rs` (`pose_plan`, `PoseKind`, `Pose`) reproduces the
     JS BIT-EXACTLY: `tests/fixtures/pose_plan.txt` is generated FROM
     `posePlan()` (`make gen-pose`, `tools/gen_pose_fixture.ts` on Bun: every
     pose x relaxed x 54 times) and `matches_the_js_pose_plan` compares all
     9,504 scalars (measured: max difference 0). `make check-pose` (run by
     `check-render`) fails if the JS drifts from the fixture. The kick /
     stomp timing now DERIVES from `FinisherKind::impacts()` and is tested
     ("the foot is fully extended on the impact", "each stomp lands on its
     impact"). Until R3 the pose logic exists TWICE: edit both, regenerate.
   - **R2 (DONE): the game uses it.** The `ROBOT` op is `colorIdx weaponIdx
     flags x y angle sizePx` + the 11 pose scalars (18 args; flags bit 0 =
     the gun hand aims, bit 1 = headless): `render/robots.rs` calls
     `pose_plan`, the renderer unpacks with `planFromScalars` and
     `batchDraw` takes the ready-made `opts.plan` — the game runs NO JS
     animation any more, and renderer.js lost its `ROBOT_POSES` table. The
     scalar ORDER is one JS list, `POSE_SCALARS` (web/robot-core.js), which
     drives both the renderer's unpack and the fixture's columns; Rust is
     held to it by `scalar_order_matches_the_js`, and `make check-pose`
     proves `planFromScalars` inverts it. Since R1 proved the two pose
     functions bit-identical, the game's robots are unchanged by
     construction. JS `posePlan()` now only serves the tools + the bakes.
   - **R3: delete the JS copy** — needs roadmap step 4 (the portrait bake,
     `tools/inspector.html` and `tools/rig-parity.html` call `posePlan`
     with no wasm loaded); the fixture then stays as the golden record.

   Payoff: choreography in one language, host-testable.
2. **Boss: instanced spheres, still in JS** — one per-instance float block
   per sphere, one instanced draw (as the robots' GPU rig did). Worth it on
   its own (a few dozen draws -> one) and it CREATES the seam. Needs a
   pixel-parity page first, like `tools/rig-parity.html`: the boss has no
   pixel test today, and its animation was tuned by eye.
3. **Boss: move the sphere placement to Rust**, filling that instance list
   (`render/shoggoth.rs`). Removes the mirrored `MASK_OFF_SECS` /
   `BOSS_MASK_OFF_SECS`.
4. **Tools load the wasm for poses.** `tools/inspector.html` and
   `tools/rig-parity.html` import the JS pipelines with no engine today;
   they would call an exported `pose_plan(...)` instead.

Costs accepted knowingly: look-dev on a pose goes from edit + refresh to a
wasm rebuild (15 s release today; a dev-profile build should cut that — not
yet measured), and the tools pages gain a wasm dependency. Decide robots and
boss TOGETHER — moving only one leaves two conventions, which is worse than
either.

Interim guard until step 3: `BOSS_MASK_OFF_SECS` (src/systems/boss.rs) and
`MASK_OFF_SECS` (web/shoggoth-core.js) are PINNED by a `cargo test` that
parses the JS constant (`mask_off_secs_matches_shoggoth_core_js`, the
`web/ops.js` pattern).
