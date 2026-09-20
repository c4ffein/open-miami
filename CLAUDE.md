## Where things are documented
- This file = the RULES + a short map, loaded into every session: keep it
  short. Detail lives in `docs/` — read the relevant page BEFORE working in
  an area, and update it (not this file) when detail changes:
  `ARCHITECTURE.md` (the four layers, where code goes, known debt, roadmap) ·
  `RENDERING.md` (every opcode, the why, the GPU measurements) ·
  `PIPELINE.md` (one frame, as a diagram) · `CODEMAP.md` (what lives where) ·
  `TESTING.md` (the four suites; where a new test belongs) · `TOOLS.md`
  (`?viz`, the level editor) · `ECS.md` · `URL_PARAMS.md` ·
  `SCENARIO_FORMAT.md` · `PROPS_FORMAT.md` · `MUSIC_CODE.md` + `music/`

## Development Constraints
- NEVER add any additional dependency
- TOOLING LANGUAGE: NEW tooling (generators, checkers, build / deploy
  scripts, dev utilities) is written in TYPESCRIPT and run with BUN (already
  the e2e toolchain; `bun build` is the bundler — no extra dependency). Do
  NOT add new Python scripts. The existing Python 3 stdlib tools
  (`tools/gen_levels.py`, `tools/gen_props.py`, `gen-title`, `serve.py`)
  stay as they are for now — port one only when it needs real work anyway,
  and keep `make verify` runnable while doing so. A check that can be a
  Rust host test (`cargo test`, e.g. the opcode-table sync in
  `src/graphics/stream.rs`) should be one: no toolchain beyond cargo

## Design
- THE VIBE — "pixelated assets in a modern engine", not real pixel art
  (the Hotline-Miami approach): assets are BAKED at a low art resolution
  (robots/portraits/guns at their tile px, props at their saved px, the
  drive at its art grid) and then MOVED SMOOTHLY — rotated, translated,
  swayed — at native resolution and full frame rate. The crunch comes from
  the ASSET resolution, never from a screen-space pixel grid: rotation
  angles and motion are continuous (a baked sprite tilts as a rigid chunky
  image), animation stays 60 fps, and upscaling is NEAREST at real screen
  pixels (see SIZING). The planned endgame applies this to the WHOLE WORLD:
  the level rasterizes into an art-res pixel group and that finished image
  is what the camera moves/sways/rotates — rigid-pixel-image rotation at
  the COMPOSITE quad ("Before" mode, like the props/portraits), rendered
  with a bleed margin so tilting never exposes void; NOT rotation inside
  the group (per-frame re-rasterization = edge crawl). Consequences:
  screen-space quantization of motion / rotation / camera is off-vibe;
  BLURRY downscale is off-vibe (art-res rasterization + NEAREST upscale is
  the vibe — the crunch must come from the art grid); and — DECIDED,
  do not re-litigate — THE ALIASING IS PART OF THE ART DIRECTION (the
  HM2 look): tilted geometry and rotated pixel images keep hard,
  stair-stepped, all-or-nothing edges. NO ANTIALIASING of any kind: no
  MSAA (`antialias: false` stays — an `?aa=1` experiment antialiased the
  walls beautifully AND dropped the 2018 MacBook to 30 fps, 4x bandwidth
  per full-screen layer at Retina), no sampling-side smoothing (a
  texel-snap AA shader on the world composite was built, verified
  pixel-perfect, and REMOVED on purpose), no FXAA (it would soften the
  sprite/text crunch). Smoothness lives ONLY in MOTION: the `smooth`
  composite flag means sub-pixel PLACEMENT (no origin snap), never soft
  sampling. All of this is BUILT: `?pixel=N` runs the world through a
  world-anchored sub-pixel-composite group (see the `?floor=N` bullet),
  and the static geometry cache draws inside it
- ALL on-screen text is UPPERCASE — VT323 renders far better in caps and
  all-caps is the game's look. Enforced STRUCTURALLY: `Graphics::draw_text`
  ASCII-uppercases every string at the single rendering boundary, so write
  strings (code literals, level-JSON dialogue / objectives / captions,
  editor labels) in whatever case reads best at the source — the screen
  always shows caps, and non-ASCII glyphs (`·`, `→`, …) pass through.
  Consequently the e2e text-arena probes see uppercase: match `'HEALTH:'`,
  never `'Health:'`

## Layering (docs/ARCHITECTURE.md — read it before adding a module)
- FOUR LAYERS, one question each: SIM "what is the state?" (`ecs`,
  `systems`, `scenario`, `game`, `sim` — no `Graphics`, no browser), RENDER
  "what does the state look like?" (`render.rs` + `render/*`, `level`,
  `camera`, the props / drive / ending `draw` code — state READ-ONLY +
  `&Graphics`), APP "what happens this frame?" (`app.rs` + `app/*`,
  `editor_ui`, `input`, `audio/engine` — wasm-only: input, clock, audio,
  settings, `&mut GameState`), RENDERER "commands -> pixels" (the JS)
- THE TEST for render code: a function from state to draw commands — reads
  NO input, MUTATES nothing, calls NO browser API. If it passes, it goes in
  the render layer, takes plain arguments / a view struct
  (`render::world::WorldView`, `render::hud::HudView` — an in-game frame is
  exactly those two) and gets a native stream test. So: render code never
  samples input (the app passes `player_firing: bool`, `cursor: Vec2`) and never
  advances a timer (the kill flash / spark expiry count down in
  `GameState::render_world`, the app-side wrapper). Immediate-mode UI (a
  button = draw + click test in one call: menus, `?viz`, `editor_ui`) is
  legitimately APP code
- CHARACTERS ROADMAP (docs/ARCHITECTURE.md "Roadmap" — decided direction,
  robots R1 + R2 DONE): Rust computes poses (numbers), GLSL evaluates rigs,
  JS only ferries. THE GAME's robots are animated by `pose_plan` in
  src/render/pose.rs (kick / stomp timing derived from
  `FinisherKind::impacts()`); the `ROBOT` op carries its 11 scalars, in the
  order of `POSE_SCALARS` (web/robot-core.js — the ONE list behind the
  renderer's `planFromScalars` and the fixture's columns; Rust pinned to it).
  UNTIL R3 THE POSE LOGIC STILL EXISTS TWICE: JS `posePlan()` serves the
  portrait bake + tools/inspector.html + tools/rig-parity.html (no wasm
  there) and must stay BIT-IDENTICAL — change a pose = edit BOTH, `make
  gen-pose`, `cargo test` (`matches_the_js_pose_plan` vs
  tests/fixtures/pose_plan.txt; `make check-pose`, run by `check-render`,
  fails if the JS drifts). Robots first (`posePlan` is pure and the GPU rig's 16 instance
  floats are already the seam; port behind a scalar-parity test), then the
  boss (instanced spheres in JS behind a pixel-parity page, THEN placement
  in Rust). Do not move only one of the two; do not add new animation logic
  to JS that gameplay timers depend on without noting it there
- EVERY FRAME DRAWS A SCREEN: a screen switch (`self.screen = …`, a modal
  flag) takes effect AFTER the current screen is drawn — record the intent,
  draw, then switch. Never `switch; return;` before drawing (a one-frame
  flash of neither screen — from PAUSED, the raw world with no modal), and
  never re-dispatch to the new screen in the same frame (it would see the
  same key press). Pinned by `tests/e2e/specs/menu-transitions.spec.js`
- WHY: it is the test boundary — render-layer code verifies in ~1 s under
  `cargo test`, app-layer code needs a ~50 s browser round trip. Keep the
  app layer thin

## Rendering (RULES — the full reference is docs/RENDERING.md: read the relevant part BEFORE changing the renderer, a shader or an opcode)
- The Rust/wasm engine owns the simulation only; **all rendering is WebGL in
  JS** (`web/`). Each frame `Graphics` (src/graphics.rs) records a flat f32
  command stream + a text arena and hands both to `window.frameRender` ONCE
  — a single zero-copy wasm->JS crossing. Coordinates are CSS px; the canvas
  buffer is CSS x devicePixelRatio (`Graphics::sync_size`, `data-dpr`), so
  primitives rasterize at real screen pixels
- `Graphics` is a RECORDER with two surfaces: the browser canvas (wasm-only:
  `new`, `sync_size`, `flush`) and HEADLESS (`Graphics::new_headless(w, h)` +
  `take_frame()`, native). Every draw method is plain Rust, so everything
  that only records builds and is TESTED natively — do NOT gate a module
  `cfg(target_arch = "wasm32")` just because it takes a `&Graphics`.
  `src/graphics/stream.rs` reads a frame back (`walk`; `check` = arity,
  finite floats, balanced SAVE/RESTORE + pixel groups <= `PIX_DEPTH`, static
  sections framed + solid-only, TEXT indices; `Affine` = renderer.js's
  transform stack). `tests/render_stream.rs` runs the REAL
  `render::world::render_world` + `render::hud::render_hud` on every floor
  and every prop at every px. A
  new draw path gets a stream test there FIRST (~1 s vs ~50 s in a browser)
- THE OPCODE TABLE (26 ops; what each does: docs/RENDERING.md) exists twice,
  PINNED by `cargo test` (`src/graphics/stream.rs`): Rust (`mod op` in
  graphics.rs + `OP_ARGS` in stream.rs; ops 21-23 in `src/static_geo.rs`) and
  JS (`web/ops.js`: `["NAME", args]` rows, row index = opcode — read by
  renderer.js + gpu-probe.js, parsed from disk by the CommonJS
  tests/e2e/specs/helpers.js). A new opcode = a Rust const + arity, an
  `ops.js` row, a `case N: // NAME` in renderer.js: the tests fail until all
  three agree
- MIRRORED ACROSS THE BOUNDARY — edit both or neither (pinned where noted):
  the POSTFX kind table (`Graphics::postfx` doc <-> renderer.js + the `?viz`
  EFFECTS list in src/app/viz/effects.rs); the DRIVE scene geometry
  (`src/drive.rs` tunables <-> `DRIVE_FS`) and its integer hash (`hash01` <->
  `driveHash`); the robot index tables (src/render/robots.rs <-> renderer.js
  `ROBOT_COLORS` / `ROBOT_WEAPONS` — no pose table: poses cross as numbers);
  the pose logic itself until R3 (PINNED bit-exact, see Layering); the rig's ROTATION ORDER
  per joint chain inside web/robot-core.js (`leg()` / `arm()`, the CPU rig =
  the reference <-> `rigVS`, the GPU rig; `tests/e2e/render/rig-parity.js`);
  `PIX_DEPTH` (stream.rs <-> renderer.js); `BOSS_MASK_OFF_SECS` <->
  `MASK_OFF_SECS` (PINNED, src/systems/boss.rs)
- PIXEL-ART GROUPS (`pixel_begin` / `pixel_end`, ops 15/16): never average or
  point-sample a hi-res image — RASTERIZE AT THE ART RESOLUTION, upscale
  NEAREST. Groups nest <= 4 deep, <= 1024 texels a side (beyond = pass-through).
  `smooth` = sub-pixel PLACEMENT only, never soft sampling. A camera-sized
  group must be a WHOLE number of texels (the composite's v flip is anchored
  at the integer row count — a fractional height makes content swim;
  `composite-coherence.js` + `the_pixel_world_is_one_smooth_group…`). Actors
  draw AFTER the `?pixel=N` world group closes: never re-quantize a moving
  sprite onto the world grid
- THE STATIC GEOMETRY CACHE (`Graphics::static_layer`, ops 21-23; floor tiles
  + walls only): content must be SOLID primitives (no text / sprites),
  recorded UNCULLED (`Level::full_bounds`) and FRAME-INVARIANT — anything
  that varies per frame (kill-flash tint, debug overlays) must BYPASS it and
  draw plainly. One key live; `load_floor` bumps it
- THE BACKDROP's occlusion rect (`src/backdrop_clip.rs`, op 24) must stay
  CONSERVATIVE (only what the floor is guaranteed to cover); `backdrop-clip.js`
  requires clipped == full, pixel for pixel
- PERF RULES, each one measured on the 2018 MacBook Air (numbers + method in
  docs/RENDERING.md): `antialias: false` ALWAYS; never add a texture fetch to
  the batch fragment shader (it taxes every fragment of every layer — the
  `?grain=fold` lesson); a full-screen layer is the unit of GPU cost (~1-2 ms
  there) — prefer computing at ART resolution into a tiny target + ONE
  upscaled quad (DRIVE / BACKDROP economics); batch flushes ORPHAN the buffer
  (`bufferData`, never `bufferSubData` into a live store); the context is
  `alpha: true` on Apple; NEVER leave a DOM element over the game canvas
  during play (it changes how the browser presents the canvas and ~doubled
  GPU cost in measurements); `initRenderer` stays ONE closure (decision +
  evidence: docs/ARCHITECTURE.md). MEASURE with `?gpuprobe` BEFORE optimizing
  a layer; `?perf` + **P** = the CPU trace (viewer: tools/perf.html)
- Robots render LIVE every frame through web/robot-core.js as ONE instanced
  batch (the GPU rig); the boss through web/shoggoth-core.js; portraits, guns
  and heads are baked ONCE into a persistent NEAREST atlas and drawn as rigid
  pixel sprites rotated in 2D. Where their animation should live long-term:
  the Roadmap in docs/ARCHITECTURE.md

## Code map (short — the detailed tour is docs/CODEMAP.md; tools + editor: docs/TOOLS.md)
- `src/lib.rs` is just the module list. SIM: `ecs/`, `components/`,
  `systems/` (incl. `passive.rs` bystanders, `head.rs`, `finisher.rs`),
  `scenario.rs`, `game.rs`, `sim.rs` (the SHARED tick `GameSystems::step` +
  the headless `Simulation`), `pathfinding.rs`, `collision.rs`. RENDER:
  `render.rs` + `render/{world,hud,robots,pose,comms,dialogue,floor_props,title}.rs`,
  `level.rs`, `camera.rs`, `props.rs` + `props/` (by FAMILY:
  `layers/{datacenter,outdoor,lobby}.rs` + `draw/…`), `drive.rs`,
  `ending.rs`, `sparks.rs`; `hud_ammo.rs` / `hud_msg.rs` are HUD STATE
  machines. APP (wasm-only): `app.rs` + `app/{game_loop,world_render,menus,
  viz,viz/*,url,perf}.rs`, `editor_ui.rs`, `input.rs`. AUDIO: `audio/`
  (music is CODE: one song = one Rust file in `audio/songs/`, authoring API
  `audio/compose.rs`, docs/MUSIC_CODE.md; the WebAudio engine =
  `audio/engine.rs` + `audio/engine/*`, wasm-only, not host-tested)
- `web/` = the hand-written JS runtime (plain ES modules, no build step in
  dev): `renderer.js` + `renderer/shaders.js`, `ops.js`, `robot-core.js`,
  `shoggoth-core.js`, `gpu-probe.js`. Root `open_miami.js` / `_bg.wasm` are
  GENERATED (gitignored). `make bundle` (`bun build`) = DEPLOY ONLY: one
  minified `web/renderer.js` at the same path (.github/workflows/wasm-build.yml)
- GENERATED RUST — never hand-edit: `src/levels_data.rs` from `levels/*.json`
  (`make gen-levels`; format docs/SCENARIO_FORMAT.md; level INDEX = position
  in `levels/index.json`, `?floor=N` takes the floor ID) and
  `src/props_data.rs` from `props/props.json` (`make gen-props`;
  docs/PROPS_FORMAT.md). `make check-levels` / `check-props` are in `verify`
- FILES THAT TOOLS PARSE (moving / renaming breaks a generator):
  `PROP_NAMES` in `src/props.rs` (tools/gen_props.py), `title_glyph` in
  `src/render/title.rs` (tools/gen_title.py -> index.html's loading SVG),
  `posePlan` / `POSES` / `POSE_SCALARS` / `planFromScalars` exported by
  `web/robot-core.js` (tools/gen_pose_fixture.ts + a cargo test), the `TABLE` rows of `web/ops.js` + `MASK_OFF_SECS` in
  `web/shoggoth-core.js` + the `case N: // NAME` labels of `web/renderer.js`
  (cargo tests)
- NEW PROPS are APPENDED (ids are persisted in props/props.json): a name in
  `PROP_NAMES`, a table entry + a draw fn in its family's two files, a
  dispatcher arm in `props/draw.rs`
- TOOLS: `/?viz` (SPRITES / MUSICS / LEVELS / EFFECTS — docs/TOOLS.md),
  `/?floor=N[&pixel=N][&debug][&noise=0][&precompute=0]` (ALL flags:
  docs/URL_PARAMS.md), `serve.py` (dev server :8080, no-store, the editor
  write API guarded by `X-Editor-Token`), `/docs`, `/render-tests/<name>`
  (renderer-only harness), `tools/` (the `?viz` panels, generators, perf
  viewer). Editor SAVE -> then run `make gen-levels` / `make gen-props`

## Verification Requirements
- ALWAYS run `make verify` before declaring any task complete or saying "we're done"
- The `make verify` command runs the core CI pipeline checks locally
  (`CORE_CHECKS` in the Makefile — CI's `.github/workflows/ci.yml` runs
  exactly these Makefile targets, one job each):
  - Code formatting (rustfmt) - `make check-fmt`
  - Linting (clippy, native + wasm32) - `make check-clippy`
  - Test suite (all tests including doc tests) - `make check-test`
  - Release build - `make check-build`
  - wasm32 compile check - `make check-wasm-build`
  - Generated data current - `make check-levels`, `make check-props`
- ALL checks must pass before completing a task
- If any check fails, fix the issues and re-run `make verify`
- `[profile.release]` in Cargo.toml is `lto` + `codegen-units = 1` +
  `panic = "abort"` + `strip` — release only; `cargo test` keeps unwinding

### Note on the browser suites (E2E + render tests)
- Two browser suites, both excluded from `make verify` and run by
  `make verify-all` (= `verify` + `check-e2e` + `check-render`) and by
  `.github/workflows/e2e-tests.yml` (one matrix job each):
  - `make check-e2e` — the Playwright specs (`tests/e2e/specs`: floor-1
    gameplay + the lift to floor 2, `menu-transitions.spec.js` = no menu /
    sub-menu switch ever ships a frame without its POSTFX, and `all-floors.spec.js` = EVERY floor
    boots, keeps rendering well-formed frames and logs no error — the JS
    side of what `tests/render_stream.rs` proves natively)
  - `make check-render` — the standalone renderer acceptance scripts (in
    `tests/e2e/render/`, apart from the Playwright specs; they share
    `tests/e2e/node_modules`)
    `tests/e2e/render/composite-coherence.js` (~7 s) + `props-stability.js` (~60 s,
    fixed-sleep bound) + `rig-parity.js` (~5 s, the robots' GPU rig vs the
    CPU rig) + `grain-fold.js` (~15 s, the opt-in folded TV static vs the quad) +
    `backdrop-clip.js` (~30 s, the floor-occluded backdrop vs the full quad), in parallel against a `serve.py` the target starts on
    `RENDER_PORT` (a free ephemeral port by default) and kills; logs in `tests/e2e/test-results/render-*.log`
- Both depend on `make e2e-prep`: `make build-wasm` (installs the wasm32
  target and `wasm-bindgen-cli` pinned to the `wasm-bindgen` version in
  Cargo.lock when missing or mismatched — `build-wasm.sh` is gone), `bun
  install`, the Chromium install (+ rootless system-libs fallback)
- wasm-bindgen and web-sys are already in Cargo.toml as dependencies (no new dependencies needed)
- The e2e toolchain runs on **Bun** (`bun install` / `bunx playwright ...`), not npm/node

#### **E2E Test Timeout Enforcement**
- Prefer running the suites via `make check-e2e` / `make check-render` — they
  wire up the toolchain, `ulimit -c 0` (no GB-sized Chromium core dumps in
  tests/e2e/) and the timeouts for you
- Timeouts so a run cannot hang: Playwright caps each TEST at 60 s, the
  Makefile caps the whole run at `E2E_TIMEOUT` (180 s — measured: the 3
  gameplay specs ~32 s + `all-floors.spec.js`, one smoke test per floor of
  levels/index.json, ~42 s serially; it loads floors with `?precompute=0` so
  they skip the audio pre-render gate, while the gameplay specs keep the
  real boot path); the render scripts get `timeout
  $(RENDER_TIMEOUT)` (180 s) each

## Debug Mode
- The game has a built-in debug mode that can be toggled by pressing **I** during gameplay
- Debug mode is OFF by default; it is enabled only when the URL carries `?debug` (`debug_enabled` in GameState, e.g. `/?floor=14&debug`). Without it, I/K/B/G and the debug HUD line do nothing
- With debug overlays on (I), **G** skips the active tutorial `gate` (releases it as if the gated input succeeded — anti-softlock escape; see `docs/SCENARIO_FORMAT.md`)
- When debug mode is active, pressing **I** toggles the display of debug information

### Debug Visualizations
When debug info is enabled (press I), the following visualizations are shown:

1. **Enemy Vision Cones**: Shows the 90-degree vision cone for each enemy
2. **Inflated Wall Boundaries**: Yellow semi-transparent rectangles showing the 25px padding around walls used for pathfinding
3. **Pathfinding Waypoints**: For enemies in chasing mode (SpottedUnsure or SurePlayerSeen):
   - **Cyan line**: Actual movement trail showing where the enemy has traveled (last 100 positions)
   - **Red semi-transparent line**: Direct line from enemy to final target
   - **Green lines and dots**: Pathfinding waypoints showing the planned path the enemy will follow
   - **Red dot**: Final target position
   - **Green dots**: Individual waypoints along the path

These visualizations help understand and debug:
- Enemy AI behavior and detection
- Pathfinding algorithm results (A* + string pulling + wall-hugging)
- How inflated wall boundaries prevent wall grinding
- The difference between direct movement vs pathfinding
- Compare actual path taken (cyan) vs planned path (green)

## Artifact Server
- An artifact server is available at `$ARTIFACTER_API_URL`
- Use PUT requests to upload files to any route - the files will become available via GET requests
- Authentication requires `$ARTIFACTER_API_KEY` header
- This enables fast iteration by uploading wasm and HTML files for immediate testing
