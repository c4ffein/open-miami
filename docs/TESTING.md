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
  the pose scalar order + flag bits vs `web/robot-core.js`
  (`src/render/pose.rs`), the 20-float sphere layout vs `web/shoggoth-core.js`
  (`src/render/shoggoth.rs`). And two GOLDEN RECORDS, captured from JS
  animation code before it was deleted, which the Rust ports must keep
  matching: `tests/fixtures/pose_plan.txt` (robots) and
  `tests/fixtures/shoggoth_spheres.txt` (the boss).

The simulation tick is SHARED, not duplicated: the browser loop and
`Simulation` both call `sim::GameSystems::step`, so what these tests play is
what ships.

Conventions: fixed `dt` (deterministic), no wall-clock time, no randomness
without a seed; a new draw path gets a `render_stream` test before a browser
one.

**The audio engine** (`src/audio/engine/tests.rs`): the real WebAudio voice
builders run natively against the recording mock of
`src/audio/engine/webaudio.rs`, and the node graph of every SFX bake / note
voice / live one-shot is checked for the defects Web Audio reports by
THROWING — which the engine swallows by design: exponential ramps to <= 0,
unstopped oscillators, orphan nodes, events in the past, a sound longer than
its bake length. Mutation-tested (5 recipe breaks, 5 caught). A new sound is
covered the day it is added to `SFX_KINDS` / a song. The MUSIC path has its
own graph tests (mutation-tested 10 / 10): the lane channels are wired in
order (panner → drive → ducker → bus → lowpass → soft-clip, echo loop + hall
returning into the ducker, no compressor on the music path), a song change
re-points them (`apply_voices`), wide voices bake STEREO and a tie holds its
peak, `compose`-built songs make the centre-pan law up, the live SKETCH of an
unbaked note is bounded (one oscillator per partial), and the scheduler —
driven a frame at a time against the mock clock through a whole play-through
of every song — is never in the clock's past (humanize included) and ducks
exactly on the kicks of ducked sections; a COMPUTED voice's note bakes at
once (no offline context, nothing in flight, the buffer the length
`key_seconds` says) and sketches like any other. The computed voices
themselves (`src/audio/dsp.rs`) are tested as signals: every string rings
at its pitch (autocorrelation, ±1.5 %), decays but still rings at half a
second, ends silent, stays finite; the violin swells and holds; a glide
arrives at its target; a strum staggers; the filter envelope darkens; the
sub adds weight; the same note bakes the same samples; a big note renders
under 150 ms even in debug (~20 ms release).

**Refactoring the AI?** `tests/ai_fingerprint.rs` is an `#[ignore]`d tool, not
a check: it hashes every enemy's state on every tick of every floor. Run it
before and after (`cargo test --test ai_fingerprint -- --ignored --nocapture
| grep ^FP`) and diff — identical output = behaviour unchanged, bit for bit.

## Browser (`tests/e2e/`, on Bun — see its README)

Headless Chromium on software GL (SwiftShader, ~5–15 fps): correct pixels,
slow frames. Always run through the Makefile — it builds the wasm, installs
the browser, sets the library path and the timeouts.

- **`make check-render`** — nine standalone scripts, each comparing PIXELS, in
  TWO parallel waves (`RENDER_LIVE` then `RENDER_ONLY` in the Makefile: the
  live-game ones wait on fixed sleeps, so their parallel width stays at the
  six a 4-core CI runner is green with). Two families:
  - on LIVE GAME frames: the pixel-group composite (`composite-coherence`),
    prop pixel-art stability (`props-stability`), the robots' GPU rig vs the
    CPU reference (`rig-parity`), the folded TV static (`grain-fold`), the
    floor-occluded backdrop (`backdrop-clip`), the boss's instanced path vs
    its per-sphere reference (`shoggoth-parity`);
  - RENDERER-ONLY, no game and no wasm (`render/lib.js` drives
    `web/renderer.js` through the `/render-tests` harness page with
    hand-built streams, the clock and `Math.random` pinned; ~3-6 s each):
    every POSTFX kind against what its table promises (`postfx-kinds`: t = 0
    is the identity, each kind is its own image, clock + colour wired, the
    modal static's coverage IS `t`, the warp accumulator accumulates and
    clears, unknown kinds are no-ops), TEXT + the glyph atlas
    (`text-glyphs`: baseline / size / monospace pen / colour / transform /
    arena, and text after an ATLAS RESET == text before), and the two
    full-shader backgrounds (`drive-backdrop`: the art grid, `dim`, one torn
    band = the image shifted, split threshold, placement, the occlusion
    rect). They assert PROPERTIES, never golden images; each was
    mutation-tested (7 renderer breaks, 7 caught). They also print one
    `FP <case> <hash>` line per rendered case into their log: REFACTORING
    `web/renderer.js`? diff those lines before / after — identical =
    pixel-identical on 120+ cases.

  These are the safety net for any change to `web/`.
- **`make check-e2e`** — Playwright: floor 1 loads and draws its HUD, the
  player purges the floor and rides the lift to floor 2 (the real boot path,
  audio pre-render gate included); `menu-transitions.spec.js`: driving the
  title and pause menus, no frame of the sequence lacks its POSTFX (a
  screen switch that returns without drawing = a one-frame flash);
  `sound-settings.spec.js`: `?viz` shows a saved SOUND OFF on every tab and
  its toggle flips `om.sound` + the `AudioContext` (suspended <-> running),
  the SETTINGS MUSIC row cycles 100 → 75 → … → 0 → 100, persists `om.music`
  and is read back on the next load; and
  `all-floors.spec.js`: EVERY floor of
  `levels/index.json` boots, keeps rendering well-formed frames and logs no
  error (with `?precompute=0`, ~3 s a floor). TWO local workers
  (`playwright.config.js`), measured: the pages render in software, so the
  run takes the same ~125 s at 2, 3 or 4 workers while the longest test
  (the floor-1 lift run, 24 s alone) takes 34 / 53 / 57 s — more workers
  only push single tests toward the 60 s limit. A page that sits on the
  title screen or on `?viz` SPRITES is the expensive kind: leave them fast.

Timeouts, so a run cannot hang: Playwright caps each TEST at 60 s, the
Makefile caps the run at `E2E_TIMEOUT` (180 s — measured: the 3 gameplay
specs ~32 s + `all-floors.spec.js` ~42 s serially; it skips the audio
pre-render gate with `?precompute=0`, the gameplay specs keep the real boot
path) and each render script at `RENDER_TIMEOUT` (180 s — measured:
`composite-coherence` ~7 s, `props-stability` ~37 s (it waits on rendered
ENGINE FRAMES, never on sleeps — the pattern for any new live-game script:
tap `window.frameRender`, count, `waitForFunction`),
`rig-parity` ~5 s, `grain-fold` ~15 s, `backdrop-clip` ~30 s,
`shoggoth-parity` ~20 s, the three renderer-only ones ~3-6 s; the whole
target ~85 s). The render scripts run against a `serve.py` the
target starts on `RENDER_PORT` (a free ephemeral port by default) and kills;
logs in `tests/e2e/test-results/render-*.log`.

**CI has fonts, the dev box may have none** (`fc-list | wc -l` = 0 there): any
glyph VT323 lacks then falls back to a VT323-wide tofu locally and to a real,
wider glyph on CI — text pixels differ, and a text bug can hide (it did:
docs/HISTORY.md). To run a suite with CI-like fonts without root: `apt-get
download fonts-dejavu-core fonts-wqy-microhei` (with the `-o Dir::State=…`
options of tests/e2e/setup-browser-deps.sh), `dpkg -x` them somewhere, write a
fonts.conf whose `<dir>` points there, and run `FONTCONFIG_FILE=<that file>
make check-render`. On a CI failure, the `FAIL` lines are the run's `::error`
annotations and the full logs are in its `check-render-test-results` artifact.

What only the browser can catch: GLSL that does not compile, the JS renderer
mis-walking a stream, robot-core / shoggoth-core failing, a prop or surface
the WebGL path chokes on. What it should NOT be used for: gameplay rules —
the headless simulation proves those thousands of times faster.

## Not covered (known)

How the audio SOUNDS, and the engine's async bake plumbing (the node graphs
themselves are host-tested: `src/audio/engine/tests.rs`);
`src/app/` (input, menus, `?viz`) is only exercised by the Playwright specs.
