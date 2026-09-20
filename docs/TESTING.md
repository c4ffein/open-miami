# Testing

Four suites, cheapest first. The rule that shapes them: **put a test in the
cheapest suite that can see the bug** — which is why so much code is kept on
the native side of the wasm boundary (see [ARCHITECTURE.md](ARCHITECTURE.md)).

| Suite | Runs | Cost | Sees |
|---|---|---|---|
| Unit tests (`#[test]` next to the code, ~350) | `cargo test` | ~1 s after a change | one module's logic |
| Integration tests (`tests/*.rs`) | `cargo test` | ~1 s | the whole engine, headless |
| Render scripts (`tests/e2e/render/`) | `make check-render` | ~60 s | PIXELS, in a real browser |
| Playwright specs (`tests/e2e/specs/`) | `make check-e2e` | ~80 s | the real game, played |

`make verify` = fmt + clippy (native and wasm32) + all `cargo test` suites +
release build + wasm build + generated-data checks. `make verify-all` adds
the two browser suites. CI runs exactly these Makefile targets.

## Native (`cargo test`)

Everything that is not the browser: the ECS, every gameplay system, the
scenario engine, pathfinding, the level editor's document, the music
authoring API and sequencer — and, since `Graphics` records headlessly, all
the DRAW code too.

- **`tests/integration_tests.rs`** — systems working together on a `World`.
- **`tests/playthrough_gate.rs`** — a heuristic bot PLAYS every floor through
  the headless `Simulation`: no panic, no NaN position, every run reaches a
  terminal state, every enemy spawn is reachable, runs are deterministic.
- **`tests/thin_walls.rs`** — fast bullets / bodies never tunnel a thin wall.
- **`tests/render_stream.rs`** — records the REAL `render_world` on every
  floor (cached / referenced / debug / kill-flash frames, `?pixel=2|3|6`) and
  every prop at every art-pixel size, and validates the command stream the
  JS renderer would receive (`graphics::stream::check`).
- **`tests/perf_audit.rs`** — tick-cost benches (informational timings).
- **Cross-language pins** (unit tests that parse a JS file): the opcode table
  vs `web/ops.js` and the renderer's dispatch (`src/graphics/stream.rs`),
  `MASK_OFF_SECS` vs `web/shoggoth-core.js` (`src/systems/boss.rs`). And one
  FIXTURE pin: `tests/fixtures/pose_plan.txt`, generated from the JS
  `posePlan()` (`make gen-pose`, Bun), holds the Rust `pose_plan` bit-exact
  to it (`src/render/pose.rs`); `make check-pose` catches JS-side drift. A
  constant mirrored across the wasm boundary gets one of these.

The simulation tick is SHARED, not duplicated: the browser loop and
`Simulation` both call `sim::GameSystems::step`, so what these tests play is
what ships.

Conventions: fixed `dt` (deterministic), no wall-clock time, no randomness
without a seed; a new draw path gets a `render_stream` test before a browser
one.

## Browser (`tests/e2e/`, on Bun — see its README)

Headless Chromium on software GL (SwiftShader, ~5–15 fps): correct pixels,
slow frames. Always run through the Makefile — it builds the wasm, installs
the browser, sets the library path and the timeouts.

- **`make check-render`** — five standalone scripts, in parallel, each
  comparing PIXELS: the pixel-group composite (`composite-coherence`), prop
  pixel-art stability (`props-stability`), the robots' GPU rig vs the CPU
  reference (`rig-parity`), the folded TV static (`grain-fold`), the
  floor-occluded backdrop (`backdrop-clip`). These are the safety net for
  any change to `web/`.
- **`make check-e2e`** — Playwright: floor 1 loads and draws its HUD, the
  player purges the floor and rides the lift to floor 2 (the real boot path,
  audio pre-render gate included); `menu-transitions.spec.js`: driving the
  title and pause menus, no frame of the sequence lacks its POSTFX (a
  screen switch that returns without drawing = a one-frame flash); and
  `all-floors.spec.js`: EVERY floor of
  `levels/index.json` boots, keeps rendering well-formed frames and logs no
  error (with `?precompute=0`, ~3 s a floor).

What only the browser can catch: GLSL that does not compile, the JS renderer
mis-walking a stream, robot-core / shoggoth-core failing, a prop or surface
the WebGL path chokes on. What it should NOT be used for: gameplay rules —
the headless simulation proves those thousands of times faster.

## Not covered (known)

The WebAudio engine (`src/audio/engine*`) builds live node graphs and has no
host tests; postfx kinds, text and the drive backdrop have no pixel test;
`src/app/` (input, menus, `?viz`) is only exercised by the Playwright specs.
