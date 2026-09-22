## Where things are documented
- This file = the RULES + a short map, loaded into every session: keep it
  short, and keep it to what is TRUE NOW. No history here: a finished
  roadmap, a rejected experiment, a removed tool goes in `docs/HISTORY.md`;
  detail goes in the page below that owns it — read that page BEFORE working
  in an area, and update it (not this file) when detail changes:
  `ARCHITECTURE.md` (the four layers, where code goes, known debt, the
  characters rule, and "What's next" = THE OPEN-WORK LIST a new session
  starts from) · `RENDERING.md` (every opcode, the why, the GPU
  measurements) · `PIPELINE.md` (one frame, as a diagram) · `CODEMAP.md`
  (what lives where) · `TESTING.md` (the suites; where a new test belongs) ·
  `TOOLS.md` (`?viz`, the level editor, debug mode) · `HISTORY.md` (the
  evidence behind the rules: what was tried, measured, finished, removed) ·
  `ECS.md` · `URL_PARAMS.md` · `SCENARIO_FORMAT.md` · `PROPS_FORMAT.md` ·
  `MUSIC_CODE.md` + `music/`

## Development Constraints
- NEVER add any additional dependency (`wasm-bindgen` + `web-sys` are the
  only crates, wasm-only)
- TOOLING LANGUAGE: NEW tooling (generators, checkers, build / deploy
  scripts, dev utilities) is written in TYPESCRIPT and run with BUN (already
  the e2e toolchain; `bun build` is the bundler — no extra dependency). Do
  NOT add new Python scripts. The existing Python 3 stdlib tools
  (`tools/gen_levels.py`, `tools/gen_props.py`, `gen-title`, `serve.py`)
  stay as they are for now — port one only when it needs real work anyway,
  and keep `make verify` runnable while doing so. A check that can be a
  Rust host test (`cargo test`, e.g. the opcode-table sync in
  `src/graphics/stream.rs`) should be one: no toolchain beyond cargo
- CI installs Rust `@stable`, which can be NEWER than the local toolchain: a
  clippy lint added upstream breaks CI with no code change. Reproduce with
  `rustup toolchain install <version>` + `cargo +<version> clippy …`

## Design
- THE VIBE — "pixelated assets in a modern engine", not real pixel art (the
  Hotline-Miami approach): assets are BAKED at a low art resolution
  (robots/portraits/guns at their tile px, props at their saved px, the
  drive at its art grid) and then MOVED SMOOTHLY — rotated, translated,
  swayed — at native resolution and full frame rate. The crunch comes from
  the ASSET resolution, never from a screen-space pixel grid: rotation
  angles and motion are continuous (a baked sprite tilts as a rigid chunky
  image), animation stays 60 fps, upscaling is NEAREST at real screen pixels
- THE WHOLE WORLD follows it (`?pixel=N`): the level rasterizes into a
  world-anchored art-res pixel group and that finished image is what the
  camera moves / sways / rotates — rigid-pixel-image rotation at the
  COMPOSITE quad ("Before" mode, like the props / portraits), with a bleed
  margin so tilting never exposes void; NOT rotation inside the group
  (per-frame re-rasterization = edge crawl)
- OFF-VIBE, never do: screen-space quantization of motion / rotation /
  camera; BLURRY downscale (rasterize at art res + NEAREST upscale instead)
- THE ALIASING IS PART OF THE ART DIRECTION (the HM2 look) — DECIDED, do not
  re-litigate (the experiments: docs/HISTORY.md): tilted geometry and rotated
  pixel images keep hard, stair-stepped, all-or-nothing edges. NO
  ANTIALIASING of any kind: no MSAA (`antialias: false` stays), no
  sampling-side smoothing, no FXAA. Smoothness lives ONLY in MOTION: the
  `smooth` composite flag means sub-pixel PLACEMENT (no origin snap), never
  soft sampling
- ALL on-screen text is UPPERCASE, enforced STRUCTURALLY:
  `Graphics::draw_text` ASCII-uppercases every string at the single
  rendering boundary, so write strings (code literals, level-JSON dialogue /
  objectives / captions, editor labels) in whatever case reads best at the
  source; non-ASCII glyphs (`·`, `→`, …) pass through. Consequently the e2e
  text-arena probes see uppercase: match `'HEALTH:'`, never `'Health:'`

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
  samples input (the app passes `player_firing: bool`, `cursor: Vec2`) and
  never advances a timer (the kill flash / spark expiry count down in
  `GameState::render_world`, the app-side wrapper). Immediate-mode UI (a
  button = draw + click test in one call: menus, `?viz`, `editor_ui`) is
  legitimately APP code
- WHY: it is the test boundary — render-layer code verifies in ~1 s under
  `cargo test`, app-layer code needs a ~50 s browser round trip. Keep the
  app layer thin
- CHARACTERS: Rust computes poses (numbers), GLSL evaluates rigs, JS only
  ferries — NEVER add character animation to JS
  (`no_js_pose_logic_is_left`, `no_js_boss_animation_is_left`). Robots:
  `pose_plan` in src/render/pose.rs is THE ONE pose implementation (kick /
  stomp timing derived from `FinisherKind::impacts()`); the `ROBOT` and
  `PORTRAIT` ops carry its 11 scalars + flags, web/robot-core.js REQUIRES
  `opts.plan`. Boss: `boss_spheres` in src/render/shoggoth.rs places every
  sphere and runs the wander behaviour; it crosses as a run of `SPHERE` ops
  closed by `SHOGGOTH x y sizePx maskAt`, and web/shoggoth-core.js only
  draws that list. KEEP the boss's f32-rounded matrix maths and truncated
  literals (`6.283`, not `TAU`): bit-exact parity with its golden record
  depends on them. tests/fixtures/{pose_plan,shoggoth_spheres}.txt are
  FROZEN golden records: a deliberate pose change updates the rows in the
  same commit. The tool pages get poses from the wasm
  (`tools/engine-pose.js` -> `src/wasm_api.rs`; loading the wasm does not
  start the game). Detail: docs/ARCHITECTURE.md "Characters"
- EVERY FRAME DRAWS A SCREEN: a screen switch (`self.screen = …`, a modal
  flag) takes effect AFTER the current screen is drawn — record the intent,
  draw, then switch. Never `switch; return;` before drawing (a one-frame
  flash of neither screen), and never re-dispatch to the new screen in the
  same frame (it would see the same key press). Pinned by
  `tests/e2e/specs/menu-transitions.spec.js`

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
  `src/graphics/stream.rs` reads a frame back (`walk`, `check`, `Affine`);
  `tests/render_stream.rs` runs the REAL `render_world` + `render_hud` on
  every floor and every prop at every px. A new draw path gets a stream
  test there FIRST (~1 s vs ~50 s in a browser)
- THE OPCODE TABLE (27 ops) exists twice, PINNED by `cargo test`
  (`src/graphics/stream.rs`): Rust (`mod op` in graphics.rs + `OP_ARGS` in
  stream.rs; ops 21-23 in `src/static_geo.rs`) and JS (`web/ops.js`:
  `["NAME", args]` rows, row index = opcode). A new opcode = a Rust const +
  arity, an `ops.js` row, a `case N: // NAME` in renderer.js: the tests fail
  until all three agree
- MIRRORED ACROSS THE BOUNDARY — edit both or neither (pinned where noted):
  the POSTFX kind table (`Graphics::postfx` doc <-> web/renderer/postfx.js +
  renderer/shaders.js + the `?viz` EFFECTS list in src/app/viz/effects.rs);
  the DRIVE scene geometry (`src/drive.rs` tunables <-> `DRIVE_FS` + the palm
  placement in web/renderer/backgrounds.js `drawDrive`) and its integer hash
  (`hash01` <-> `driveHash`), PINNED (`the_js_mirror_matches`); the robot
  index tables (src/render/robots.rs <-> renderer.js `ROBOT_COLORS` /
  `ROBOT_WEAPONS`); the pose scalar ORDER + flag bits (src/render/pose.rs
  <-> `POSE_SCALARS` / `planFromScalars`, PINNED); the rig's ROTATION
  ORDER per joint chain inside web/robot-core.js (`leg()` /
  `arm()`, the CPU rig = the reference <-> `rigVS`, the GPU rig;
  `tests/e2e/render/rig-parity.js`); `PIX_DEPTH` (stream.rs <->
  renderer.js); the 20-float SPHERE layout (src/render/shoggoth.rs <->
  `instVS` in web/shoggoth-core.js, PINNED)
- PIXEL-ART GROUPS (`pixel_begin` / `pixel_end`, ops 15/16): never average or
  point-sample a hi-res image — RASTERIZE AT THE ART RESOLUTION, upscale
  NEAREST. Groups nest <= 4 deep, <= 1024 texels a side (beyond =
  pass-through). A camera-sized group must be a WHOLE number of texels (the
  composite's v flip is anchored at the integer row count — a fractional
  height makes content swim; `composite-coherence.js`). Actors draw AFTER
  the `?pixel=N` world group closes: never re-quantize a moving sprite onto
  the world grid
- THE STATIC GEOMETRY CACHE (`Graphics::static_layer`, ops 21-23; floor tiles
  + walls only): content must be SOLID primitives (no text / sprites),
  recorded UNCULLED (`Level::full_bounds`) and FRAME-INVARIANT — anything
  that varies per frame (kill-flash tint, debug overlays) must BYPASS it and
  draw plainly. One key live; `load_floor` bumps it
- THE BACKDROP's occlusion rect (`src/backdrop_clip.rs`, op 24) must stay
  CONSERVATIVE (only what the floor is guaranteed to cover); `backdrop-clip.js`
  requires clipped == full, pixel for pixel
- CHARACTER DRAWS: robots render LIVE every frame through web/robot-core.js
  as ONE instanced batch (the GPU rig; the CPU rig is the reference); the
  boss through web/shoggoth-core.js as TWO instanced draws (body depth-ON,
  then the mask depth-OFF, in order — never merge them; the per-sphere path
  `pipe.instanced = false` is the reference, `shoggoth-parity.js`);
  portraits, guns and heads are baked ONCE into a persistent NEAREST atlas
  and drawn as rigid pixel sprites rotated in 2D
- PERF RULES, each one measured on the 2018 MacBook Air (numbers + method in
  docs/RENDERING.md): `antialias: false` ALWAYS; never add a texture fetch to
  the batch fragment shader (it taxes every fragment of every layer); a
  full-screen layer is the unit of GPU cost (~1-2 ms there) — prefer
  computing at ART resolution into a tiny target + ONE upscaled quad (DRIVE
  / BACKDROP economics); batch flushes ORPHAN the buffer (`bufferData`,
  never `bufferSubData` into a live store); the context is `alpha: true` on
  Apple; NEVER leave a DOM element over the game canvas during play (it
  ~doubled GPU cost in measurements — and voids any measurement taken that
  way); `initRenderer`'s BATCH CORE stays ONE closure (decision + evidence:
  docs/ARCHITECTURE.md) — only subsystems that own their state are
  factories (`web/renderer/{text,backgrounds,postfx}.js`), and a change
  there is proven by diffing the `FP` hashes (docs/TESTING.md). MEASURE with
  `?gpuprobe` BEFORE optimizing a layer; `?perf` + **P** = the CPU trace
  (viewer: tools/perf.html)

## Code map (short — the detailed tour is docs/CODEMAP.md; tools + editor: docs/TOOLS.md)
- `src/lib.rs` is just the module list. SIM: `ecs/`, `components/`,
  `systems/` (incl. `passive.rs` bystanders, `head.rs`, `finisher.rs`),
  `scenario.rs`, `game.rs`, `sim.rs` (the SHARED tick `GameSystems::step` +
  the headless `Simulation`), `pathfinding.rs`, `collision.rs`. RENDER:
  `render.rs` + `render/{world,hud,robots,pose,shoggoth,comms,dialogue,floor_props,title}.rs`,
  `level.rs`, `camera.rs`, `props.rs` + `props/` (by FAMILY:
  `layers/{datacenter,outdoor,lobby}.rs` + `draw/…`), `drive.rs`,
  `ending.rs`, `sparks.rs`; `hud_ammo.rs` / `hud_msg.rs` are HUD STATE
  machines. APP (wasm-only): `app.rs` + `app/{game_loop,game_input,
  game_events,world_render,menus,viz,viz/*,url,perf}.rs` (`update_game` =
  a spine of named phases: their ORDER is the behaviour), `editor_ui.rs`,
  `input.rs`. AUDIO: `audio/` (music is CODE: one song = one Rust file in
  `audio/songs/` — built with the authoring API `audio/compose.rs` (it
  covers the whole format), or `const` literals of it: `audio/songs.rs` +
  the instrument types in `audio/voice.rs`; docs/MUSIC_CODE.md. A baked
  note is DRY: pan, drive, echo / hall sends, duck and sweep are live
  per-lane channels.
  The music level is ONE constant, `MUSIC_GAIN` — never put a
  `DynamicsCompressorNode` on the music path, its automatic make-up gain
  doubled the level. The WebAudio engine = `audio/engine.rs` +
  `audio/engine/*`: it ships on wasm only but NEVER names `web_sys`
  directly — every Web Audio type goes through
  `audio/engine/webaudio.rs` (`web_sys` re-exports on wasm, a RECORDING MOCK
  under `cargo test`), so the voice builders run natively and
  `audio/engine/tests.rs` checks the graphs they build; a new `web_sys` call
  = the same method on the mock, the native build fails until it exists)
- `web/` = the hand-written JS runtime (plain ES modules, no build step in
  dev): `renderer.js` + `renderer/{shaders,text,backgrounds,postfx}.js`,
  `ops.js`, `robot-core.js`, `shoggoth-core.js`, `gpu-probe.js`. Root
  `open_miami.js` / `_bg.wasm` are GENERATED (gitignored). `make bundle`
  (`bun build`) = DEPLOY ONLY (.github/workflows/wasm-build.yml)
- GENERATED RUST — never hand-edit: `src/levels_data.rs` from `levels/*.json`
  (`make gen-levels`; format docs/SCENARIO_FORMAT.md; level INDEX = position
  in `levels/index.json`, `?floor=N` takes the floor ID) and
  `src/props_data.rs` from `props/props.json` (`make gen-props`;
  docs/PROPS_FORMAT.md). `make check-levels` / `check-props` are in `verify`.
  Editor SAVE -> then run `make gen-levels` / `make gen-props`
- FILES THAT TOOLS PARSE (moving / renaming breaks a generator or a test):
  `PROP_NAMES` in `src/props.rs` (tools/gen_props.py), `title_glyph` in
  `src/render/title.rs` (tools/gen_title.py -> index.html's loading SVG),
  and, read by cargo tests: `POSE_SCALARS` / `planFromScalars` in
  `web/robot-core.js`, the `TABLE` rows of `web/ops.js`, `SPHERE_FLOATS` /
  the `instVS` attributes of `web/shoggoth-core.js`, the `case N: // NAME`
  labels of `web/renderer.js`, and the DRIVE literals (`float horizon = h *`,
  `const SPEED =`, … — the exact prefixes are in `src/drive.rs`'s test) of
  `DRIVE_FS` / `drawDrive` / `driveHash`
- NEW PROPS are APPENDED (ids are persisted in props/props.json): a name in
  `PROP_NAMES`, a table entry + a draw fn in its family's two files, a
  dispatcher arm in `props/draw.rs`
- TOOLS (docs/TOOLS.md): `/?viz` (SPRITES / MUSICS / LEVELS / EFFECTS),
  `/?floor=N[&pixel=N][&debug][&noise=0][&precompute=0]` (ALL flags:
  docs/URL_PARAMS.md), `serve.py` (dev server :8080, no-store, the editor
  write API guarded by `X-Editor-Token`), `/docs`, `/render-tests/<name>`
  (renderer-only harness), `tools/` (the `?viz` panels, generators, perf
  viewer). DEBUG MODE: only with `?debug` in the URL; **I** toggles the
  overlays (vision cones, inflated walls, pathfinding), and with them on
  **K** purges the floor, **B** cracks the boss's mask, **G** skips the
  active tutorial gate

## Verification Requirements
- ALWAYS run `make verify` before declaring any task complete or saying
  "we're done". ALL checks must pass; if one fails, fix it and re-run
- `make verify` = `CORE_CHECKS` in the Makefile; CI
  (`.github/workflows/ci.yml`) runs exactly these targets, one job each:
  `check-fmt` (rustfmt), `check-clippy` (native + wasm32, `-D warnings`),
  `check-test` (all tests incl. doc tests), `check-build` (release),
  `check-wasm-build` (wasm32 compile check), `check-levels`, `check-props`
  (generated data current)
- `[profile.release]` in Cargo.toml is `lto` + `codegen-units = 1` +
  `panic = "abort"` + `strip` — release only; `cargo test` keeps unwinding
- THE BROWSER SUITES (what each covers: docs/TESTING.md) are excluded from
  `make verify`; `make verify-all` = `verify` + both, and
  `.github/workflows/e2e-tests.yml` runs them (one matrix job each). Run
  them after any change to `web/`, to `src/app/`, or to what a stream
  carries:
  - `make check-e2e` — the Playwright specs (`tests/e2e/specs`)
  - `make check-render` — the standalone pixel-acceptance scripts
    (`tests/e2e/render/`), in parallel against a `serve.py` the target
    starts on `RENDER_PORT` and kills; logs in
    `tests/e2e/test-results/render-*.log`
- ALWAYS run them through the Makefile: it wires up `make e2e-prep`
  (`make build-wasm` — wasm32 target + `wasm-bindgen-cli` pinned to
  Cargo.lock's `wasm-bindgen` version —, `bun install`, the Chromium
  install), `ulimit -c 0` (no GB-sized Chromium core dumps) and the
  timeouts that keep a run from hanging: 60 s per Playwright TEST,
  `E2E_TIMEOUT` (180 s) for the run, `RENDER_TIMEOUT` (180 s) per render
  script. The toolchain is **Bun** (`bun install` / `bunx playwright …`),
  not npm/node

## Artifact Server
- An artifact server is available at `$ARTIFACTER_API_URL`
- Use PUT requests to upload files to any route - the files will become available via GET requests
- Authentication requires `$ARTIFACTER_API_KEY` header
- This enables fast iteration by uploading wasm and HTML files for immediate testing
