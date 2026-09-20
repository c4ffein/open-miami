# Rendering — the detailed reference

The full opcode-by-opcode description of the renderer, with the WHY and the
measurements behind each decision. `CLAUDE.md` carries the RULES distilled
from this page; this page is where they come from. Companion pages:
[ARCHITECTURE.md](ARCHITECTURE.md) (where code lives), [PIPELINE.md](PIPELINE.md)
(one frame, as a diagram). "renderer.js" / "robot-core.js" are in `web/`.

Moved verbatim out of `CLAUDE.md`; keep it current when the renderer changes.

## Rendering architecture

- The Rust/wasm engine owns the simulation only; **all rendering is WebGL in JS**
- Each frame, `Graphics` (src/graphics.rs) records a flat f32 command stream
  (rects, circles, lines, arcs, text, transforms, robots, the shoggoth) and
  hands it to `window.frameRender` once per frame — a single zero-copy
  wasm->JS crossing
- `renderer.js` owns the canvas/GPU: one batched triangle pipeline, VT323 text
  via a lazily-built glyph atlas, robots rendered LIVE every frame through the
  robot-core.js 3D->2D pipeline (`createRobotPipeline(gl)` on the same GL
  context, into a per-frame scratch tile atlas — continuous animation time, no
  cache/quantization — as ONE BATCH per flush: `batchBegin` / `batchDraw` /
  `batchEnd` draw every queued robot into its own 128-texel tile of
  one shared 1024² pass-1 target (one clear) — through the GPU RIG, see the
  next bullet — and run ONE tile-aware inked
  post draw over all the tiles, written AT BLOCK RESOLUTION — `ROBOT_ART` =
  ceil(128 / 3) = 43 texels per robot, one per pixelate block — into the
  NEAREST-sampled robot atlas (the same image a 1:1 tile gives; the quad
  covers 128/3 of those texels)); the boss the same way through shoggoth-core.js
  (`createShoggothPipeline(gl)`, a bigger 256px scratch tile, opcode SHOGGOTH
  = 13: `x y sizePx heading reveal time`)
- THE ROBOT SKELETON EXISTS TWICE in robot-core.js, on purpose. The GPU RIG
  (`rigVS`, what the game's batches run): the joint hierarchy is evaluated
  IN THE VERTEX SHADER from 16 per-INSTANCE floats (tile, facing, palette
  index + `posePlan()`'s scalars — the pose LOGIC stays in JS), the mesh
  (`buildRigMesh`) holds every box any robot can show — body, bare-hand
  barrel, ALL THREE held weapons — pre-placed in its joint's frame and
  tagged with a visibility class, and the boxes a robot does not show
  (other weapons, the barrel, a severed head) COLLAPSE to one off-clip point
  (degenerate triangles), so one fixed-size mesh serves every loadout and
  the whole batch is ONE `bufferData` + ONE `drawArraysInstancedANGLE`
  (`ANGLE_instanced_arrays`; tiles are placed by a clip-space offset and
  clipped by a fragment `discard` on the tile-local NDC = the old per-tile
  scissor, per pixel). The CPU RIG (`_renderRobot`: JS M4 chains → a 27-mat4
  uniform palette, one draw per robot) serves every single-sprite render
  (inspector / orbit cameras, the portrait bake) and is the REFERENCE; a
  batch falls back to it per robot without the extension or for
  `opts.orbit` / `opts.halfV`, and `?rig=cpu` forces it (the A/B switch for
  `?perf` traces). The GEOMETRY lives once (`RIG` pivots + `RIG_BOXES`, read
  by both); what is mirrored is the ORDER OF ROTATIONS per joint chain
  (`leg()` / `arm()` vs `rigVS` main) — edit both or neither.
  `tests/e2e/render/rig-parity.js` (in `make check-render`) renders every pose x
  weapon x a spread of times / palettes / facings through both rigs via
  `tools/rig-parity.html` (also the human-eye page: CPU | GPU | DIFF;
  `?bench=N` times both rigs' submit cost) and asserts the art-res atlases
  match to a few edge texels per tile (float32 shader trig vs float64 JS)
- shoggoth-core.js extends robot-core's exported `SpritePipeline` (shared
  pass-1 target + inked post pass + `M4`); the 2D-primitive
  `Graphics::draw_shoggoth` is only the `?viz` gallery / level-map thumbnail
- SIZING: the canvas backing buffer is CSS size x devicePixelRatio
  (`Graphics::sync_size`, polled ~1/s by the game loop — window resizes,
  browser zoom and monitor-DPR changes are picked up live); the wasm records
  every frame in CSS-pixel coordinates and publishes the ratio as `data-dpr`
  on the canvas, renderer.js keeps `uRes` in CSS px while the viewport is the
  physical buffer, so primitives rasterize at real screen pixels (no browser
  rescale/blur on HiDPI). The camera derives its zoom from the viewport
  (`REF_VIEW_W/H`, `ZOOM_SCALE_MIN/MAX` in src/camera.rs: ~constant visible
  area whatever the window size/aspect, clamped for legibility)
- Opcode 14 = `POSTFX kind t r g b`: when present anywhere in a frame,
  renderer.js renders the whole frame into an offscreen scene FBO and draws it
  through a full-screen post shader. Kinds 0-9 (table mirrored in renderer.js
  and `Graphics::postfx`): 0 blur-out/dissolve toward the colour, 1 synthwave
  CRT, 2 VHS tape, 3 drunk sway, 4 CRT tube (barrel + grille), 5 acid trip
  (hue cycling), 6 datamosh glitch, 7 neon bloom, 8 pixel mosaic, 9 tunnel
  rush, 10 warp trails (FEEDBACK: a persistent ping-pong accumulator in
  renderer.js, pulled toward the centre + faded each frame and re-fed the
  scene's bright saturated pixels = radial long-exposure light trails; the
  accumulator is cleared whenever the previous frame did not use kind 10),
  11 UI grey (the modal wash), 12 modal static (colour.rg = a centred
  panel's half extents: inside passes through, outside blurred + buried
  under `t` coverage of hard 6-px static), 13 TV static (the frame
  untouched + the same 6-px static grain over every cell at opacity `t`,
  no wash — the title screen runs it at 0.075 for a faint dead-channel
  shimmer; the one kind that is NOT a post pass: drawn as a single
  alpha-blended quad of a pre-rolled noise texture at the end of the
  frame, never routing the frame through the scene FBO. `?grain=fold` =
  an EXPERIMENT kept opt-in: the grain FOLDED INTO THE BATCH FRAGMENT
  SHADER — blending the noise texel over a colour is the affine map
  `g(c) = c(1-k) + n*k`, which commutes with alpha blending, so graining
  every fragment as it lands on the canvas (noise by `gl_FragCoord`; the
  premultiplied form for a pixel group's composite; off inside groups;
  only when the frame opens with a BACKDROP and no post pass follows)
  gives the quad's pixels without the quad's layer —
  `tests/e2e/render/grain-fold.js` (in `make check-render`) proves the pixels on
  three live frames. It is NOT the default: it is a TRADE, measured with
  `?gpuprobe=headroom` on the 2018 MacBook Air (4.12 Mpx) — the quad's layer
  goes (game frame 8.6 -> 6.8 ms GPU) but the second texture fetch makes
  every batch fragment ~47% dearer (layer 1.44 -> 2.11 ms), so the frame
  takes FEWER extra layers (5.6 -> 4.7): break-even ~2 full-screen layers
  of batch fill. (A first verdict of "clear loss" was taken with the probe
  panel over the canvas and is void.) LESSON: a fetch added to the batch
  shader taxes every fragment of every layer; without the flag the shader
  compiles without the grain code);
  the `?viz` EFFECTS tab previews them all. Only the last POSTFX of a
  frame applies
- Opcodes 15/16 = PIXEL-ART GROUPS: `PIX_BEGIN px w h smooth` …
  `PIX_END x y` (`Graphics::pixel_begin` / `pixel_end`; `smooth` = 1 via
  `pixel_begin_smooth`). The principle: never average or
  point-sample a hi-res image — RASTERIZE AT THE ART RESOLUTION and upscale
  NEAREST. BEGIN flushes, redirects the batch into a `ceil(w/px) x ceil(h/px)`
  texel region of a 1024² NEAREST scratch FBO (cleared transparent) and
  installs the transform `scale(1/px)` so group-local `0..w x 0..h` maps to
  texels, drawn with hard coverage (no MSAA, no smoothing); inside a group
  line/outline thickness is clamped ≥ 1 texel and circle radius ≥ 0.5 texel.
  `px` is in the caller's CURRENT local units (open a group under a scale /
  rotation and the art pixels scale / rotate with the object). END flushes,
  restores the outer target + transform (unbalanced saves are discarded) and
  draws the group as a `(w, h)` quad at `(x, y)` in the outer transform
  (a rotation in force at BEGIN rotates the finished pixel image), origin
  snapped to whole pixels of the target it lands in. With `smooth` = 1 the
  composite skips the origin snap (for a pixel image that moves / rotates
  continuously — the world group): the quad places SUB-PIXEL so the
  motion glides, while sampling stays plain NEAREST — hard aliased texel
  edges, per the art direction (## Design). The composite's v flip is
  anchored at the INTEGER texel row count (a fractional `h/px` would
  shift sampling by the ceil remainder, which changes as a camera-sized
  group resizes — content would swim row by row while panning).
  `tests/e2e/render/composite-coherence.js` (standalone bun script, like
  props-stability) asserts the composite numerically at DPR 1 and 2:
  edge position matches the analytic expectation (incl. FRACTIONAL group
  sizes — the v-flip regression), slope matches the requested angle,
  texel interiors stay pure and rigid, and the smooth flag's sub-pixel
  placement tracks fractional motion while the snapped one quantizes it;
  `/render-tests/<name>` (serve.py route to render-tests.html) is the
  human-eye version of the same scenes. Groups NEST up to 4
  deep (`PIX_DEPTH`): each depth owns its own scratch texture + FBO
  (lazily created), an inner END composites into the enclosing group's
  texels (premultiplied), whose grid it snaps to. Groups over 1024 texels
  per side or past the depth cap fall back to pass-through (their END is a
  no-op). Robots / the boss can be drawn inside a group (they get quantized
  twice: their tile px, then the group px). Inside a group renderer.js
  applies the PIXEL-ART RULE at rasterization time: axis-aligned rects get a
  whole-texel size (rounded once, min 1) + whole-texel origin, circles of
  radius ≤ 2 texels a half-texel radius + grid-snapped centre, lines a
  whole-texel thickness + texel-centre endpoints (a moving shape keeps one
  stamp and hops texel by texel); circles are always tessellated in target
  space so a circle under a rotating transform (fan well / hub) is
  frame-stable. `tests/e2e/render/props-stability.js` (standalone bun script) is the
  headless acceptance test for this on the PROPS page (rotating layers of
  DATACENTER, OUTDOOR and LOBBY props: only their boxes may differ between
  frozen clocks)
- Opcodes 17/18 = PIXEL SPRITES at ART resolution, upscaled by their quads —
  never smoothed. 17 PORTRAIT `colorIdx x y sizePx time mode` (screen space,
  `Graphics::draw_robot_portrait`): the dialogue portrait — BAKED ONCE per
  (colorIdx, mode) through robot-core (fixed 3/4 camera, frozen neutral idle
  frame, 64-texel art) into a PERSISTENT NEAREST cache atlas in renderer.js
  (512², 64px tiles, Map keyed colorIdx*2+mode — NOT the per-frame scratch
  atlas), then drawn every frame as that rigid pixel image on a quad that
  gently ROCKS in 2D (~±5° at 1.5 rad/s of `time`, phase also offset by draw
  position — the Hotline-Miami portrait look; `time` only drives the rock);
  `mode` 0 = the full-body bust (slightly-elevated camera), 1 = HEADSHOT
  (camera pushed in and raised to head height — near-eye-level, head +
  shoulders fill the tile; the dialogue frame's borderless face);
  render/dialogue.rs draws the JRPG letterbox (a ~52 px black bar at the
  top, a ~170 px one at the bottom carrying the name + typewriter line
  from the left edge) plus a RIGHT-SIDE FACE SLAB between the bars — a
  dark translucent panel with a diagonal-cut left border (accent edge
  lines; narrow at the top, wide where it meets the bottom bar) carrying
  the BIG live headshot (mode 1 for robot speakers; SWARM = three small
  headshots out of phase down the diagonal, CORRUPTOR = the live
  shoggoth, UPLINK = its glyph). 18 GUNPICKUP
  `weaponIdx x y angle sizePx` (world space, `Graphics::draw_gun_pickup`,
  weaponIdx 0 bar/1 pistol/2 machinegun/3 shotgun = robot-core's
  `GROUND_WEAPON_MODELS`): a weapon lying flat as its 3D model
  (`RobotPipeline.renderGun`, top-down, laid on its side), BAKED ONCE per
  weaponIdx at angle 0 (32-texel art, `GUN_ART`) into the same persistent
  cache atlas as the portraits (negative Map keys) and drawn as that rigid
  pixel sprite on a quad rotated in 2D by `angle` — equivalent to spinning
  the model, since the top-down ortho camera only sees up-facing normals;
  render.rs draws pickups with a stable position-hashed
  resting angle and thrown weapons with their spin. Unarmed robots
  (weapon = fist) get a RELAXED pose variant in robot-core's `posePlan`
  (arms hanging loose w/ splay + elbow bend, easy walk swing); combat poses
  and armed robots are unchanged
- Opcode 19 = `PIX_BLIT sx sy sw sh x y` (`Graphics::pixel_blit`): re-draw
  the rect `(sx, sy)..(sx+sw, sy+sh)` — in the group's local units — of the
  LAST-closed pixel group as a `(sw, sh)` quad at `(x, y)` in the current
  transform (NEAREST, origin snapped like PIX_END). The group's texels
  persist until the next PIX_BEGIN, so a scene rasterized once can be
  re-placed many times for one textured quad each. (No in-game caller right
  now: drive.rs's tear bands used it until the drive went full-shader)
- Opcode 20 = `DRIVE w h t glitch split px dim o0..o8` (`Graphics::drive`):
  the synthwave drive backdrop (title screen, `?viz` MUSICS preview) as one
  full-shader pass AT ART RESOLUTION — renderer.js's DRIVE_FS computes
  every art pixel (sky bands, cut-band sun, stars, digital rain, road
  rows, palms, tear bands, red/cyan channel split, neon debris)
  shadertoy-style into a tiny `ceil(w/px) x ceil(h/px)` NEAREST target
  (~84K fragment evaluations whatever the canvas/DPR; the quantization
  comes free), then draws it as ONE upscaled textured quad — so
  fill-rate-poor GPUs pay a texture fetch per screen pixel instead of
  stacked full-screen layers. src/drive.rs stays the
  source of truth for the deterministic glitch schedules (unit-tested
  natively) and ships them as the op args; palm slots / debris blocks are
  placed per frame in renderer.js (same integer hash as Rust's `hash01`)
  and handed to the shader as uniforms; the scene geometry constants are
  MIRRORED between drive.rs's tunables and the shader — edit both or
  neither. The canvas context is created with `antialias: false`
  (ALWAYS — aliasing is the art direction, see ## Design): sprites/text/groups
  are texture quads, and a multisampled
  default framebuffer ~4x-es the bandwidth of every full-screen layer
- Opcodes 21/22/23 = the STATIC GEOMETRY CACHE: `STATIC_BEGIN key` …
  `STATIC_END` / `STATIC_REF key` (`Graphics::static_layer(key, content)`;
  op values + framing host-tested in `src/static_geo.rs`). Frame-invariant
  world geometry — the floor TILES + WALLS only — is tessellated ONCE by
  renderer.js into a persistent VBO under `key`, in WORLD coordinates (the
  transform in force at the BEGIN is the camera and is excluded: BEGIN swaps
  the CPU transform for identity, END restores it), uploaded with one
  STATIC_DRAW bufferData and drawn that frame; every later frame the wasm
  emits just `STATIC_REF key` (2 floats — `static_layer` skips its closure
  entirely) and the renderer draws the cached VBO with its then-current CPU
  transform (the camera: pan/zoom/sway) applied IN THE VERTEX SHADER via the
  batch program's `uXA`/`uXB` affine uniforms (identity for all dynamic
  draws — one shader). ONE key live at a time: a new key evicts (deletes)
  the old buffer; `load_floor` bumps the key (`floor_static_key`), a
  checkpoint restore keeps it (same floor = same tiles/walls; a death
  restart re-records — always correct). Sections must be SOLID primitives
  only (everything samples the white texture — no text/sprites), recorded
  UNCULLED (`Level::full_bounds`: the cache must hold for every camera
  position, the GPU clips), and frame-invariant: `update_game`'s world path
  BYPASSES the cache (plain per-frame draws, no static ops) during the
  kill flash (per-frame floor tint) and with debug overlays on (I: walls
  interleave their inflated-boundary outlines); the cached VBO survives
  bypass frames. The cache DOES work inside the `?pixel=N` world group
  (punt lifted): the world-space VBO draws into the group's texels through
  the same vertex-shader affine (there the CPU transform is the group's
  world->texel mapping — still affine), one buffer serving both modes. Props, elevators, actors, HUD stay dynamic
  (draw-order safe), and the editor / `?viz` map never records sections
- Opcode 24 = `BACKDROP w h t px` (`Graphics::backdrop`): the NEON-WAVE
  VOID behind/outside the level — slow interference waves (periods 10 s+)
  of heavily-darkened hot pink / cyan / violet over near-black, peak
  brightness below every floor tone in src/palette.rs (a void, not a light
  show). DRIVE economics: renderer.js's BACKDROP_FS computes every ART
  pixel (`px` ~6 CSS px) into its own tiny `ceil(w/px) x ceil(h/px)`
  NEAREST target, then ONE upscaled opaque quad at the current transform's
  origin. Drawn FIRST in `render_world`, full-screen in SCREEN space
  (before the camera / the `?pixel=N` scenery group — both modes, and the
  kill-flash bypass frames too); the floor tiles (clipped to the floor's
  rect by `Level::set_size`) + walls paint over it, so it only shows
  outside the level. Normal frame content under POSTFX (it lands in the
  scene FBO like everything else). OCCLUSION: the op is
  `BACKDROP w h t px ex ey ew eh` — `e*` = the screen rect the floor is
  GUARANTEED to cover (`src/backdrop_clip.rs`, pure + host-tested: the
  floor rect under `screen = centre + R(roll)(world - focus) zoom` is a
  slightly rotated rectangle; the rect between the innermost x of its left /
  right edges and the innermost y of its top / bottom edges lies inside it;
  inset 2 px + 2 art texels in the `?pixel=N` world, clamped to the screen;
  `Camera::floor_occlusion`), and renderer.js draws the void only AROUND it:
  the SAME full-screen quad up to four times under a SCISSOR in whole
  physical px (re-cut strips interpolate their own UVs and flip NEAREST at
  texel boundaries — the test caught it), nothing at all when the floor
  fills the screen. Not shading a fragment cannot cost anything (unlike the
  grain fold): up to a full layer saved, 1.8 ms on the 2018 MacBook Air.
  `tests/e2e/render/backdrop-clip.js` (in `make check-render`) renders live
  frames clipped and full and requires them pixel-IDENTICAL (corner of a
  floor, `?pixel=3` / `6`, floor 0, DPR 2; several sway phases each);
  `?backdrop=full` = the A/B switch
- Opcode 25 = `HEAD colorIdx x y angle sizePx` (`Graphics::draw_head`): a
  DETACHED ROBOT HEAD lying face-up on the floor — the KICK finisher's
  trophy. Baked ONCE per colour through `RobotPipeline.renderHead` (the head
  + visor cubes only, true top-down, tipped ~0.4 rad so the visor band and
  crown both read, 16-texel art `HEAD_ART`) into the same persistent cache
  atlas as portraits/guns (Map keys `-10 - colorIdx`) and drawn as that
  rigid pixel sprite on a quad ROTATED in 2D by `angle` — the physics' live
  spin glides at native res, actor-layer (after the `?pixel=N` group
  closes). The sim side is `src/systems/head.rs` (host-tested): the KICK
  finisher's impact decapitates its victim — the corpse gets `Headless`
  (rendered as robot-core pose `downed_headless`, head cubes collapsed) and
  a `DetachedHead` launches along the kick (deterministic jitter/spin from
  a hash seed), mirrors the thrown-weapon/knockback physics (friction
  slide to rest, damped wall bounces, sub-stepped vs walls), then persists
  as a corpse detail under a `MAX_HEADS` oldest-first ring cap; render/robots.rs
  draws an oil splat + drip trail at the detach point. FINISHER VARIETY:
  `FinisherSystem::kind_for(weapon, seed)` picks per victim by a
  deterministic hash — unarmed = POUND / STOMP (two-hit quick stomp) /
  KICK, bar or empty gun = OVERHEAD / KICK, loaded gun always EXECUTE; the
  player poses `kick` / `stomp` in robot-core's `posePlan` run on the
  finisher's own timer (choreographed to `FinisherKind::impacts`)
- THE OPCODE TABLE exists twice, pinned together: Rust (`mod op` in
  src/graphics.rs + `OP_ARGS` in `src/graphics/stream.rs`; ops 21-23's
  values in `src/static_geo.rs`) and JS (`web/ops.js` — read by
  renderer.js, gpu-probe.js, and parsed from disk by the CommonJS
  tests/e2e/specs/helpers.js). ENFORCED by `cargo test`
  (`src/graphics/stream.rs`): `web_ops_js_matches_the_rust_table` (every
  row's name, position = value, arity), `renderer_js_dispatch_matches_the_table`
  (every `case N: // NAME` of the renderer's switch, and every opcode has a
  case) and `every_draw_method_emits_its_declared_arity`. A new opcode = a
  Rust const + arity, a `web/ops.js` row, a renderer `case` — the tests
  fail until all three agree
- `Graphics` is a RECORDER with two surfaces: the browser one (canvas
  sizing + `flush` -> `window.frameRender`, wasm-only) and the HEADLESS one
  (`Graphics::new_headless(w, h)` + `take_frame()`, native). Every draw
  method is plain Rust, so everything that only records — `camera`,
  `level`, `render` + `render/*`, the `draw` submodules of `props` /
  `drive` / `ending`, `sparks::render_sparks` — builds and is TESTED
  natively. Only `input`, `editor_ui`, `audio/engine` and the `app`
  module are `cfg(target_arch = "wasm32")`: do not gate a module just
  because it takes a `&Graphics`. `graphics::stream` reads a frame back
  (`walk`, `check` = the structural validator: arity, finite floats,
  balanced SAVE/RESTORE + pixel groups ≤ `PIX_DEPTH`, static sections
  framed + solid-only, TEXT indices; `Affine` / `final_transform` = the
  mirror of renderer.js's `tTranslate` / `tScale` / `tRotate`).
  `tests/render_stream.rs` runs the REAL `render::world::render_world`
  over EVERY floor (cached frame, `REF` frame, debug + kill-flash bypass
  frames, `?pixel=2|3|6`: one smooth whole-texel group holding the static
  floor, robots outside it) and every prop at every px; src/camera.rs's
  tests check `screen_to_world` against the transform `apply()` records. A
  new draw path gets a stream test there first — ~1 s, vs ~50 s for a
  browser round trip

## Perf tracing (`?perf`)
- Opt-in per-frame trace across engine / boundary / renderer: add `perf` to
  the URL (e.g. `/?floor=2&debug&perf`), play a bit, press **P** — the last
  300 frames are logged to the console as one JSON blob (and copied to the
  clipboard, best-effort). Paste (or drag-drop) it into `tools/perf.html`
  for a stacked per-frame chart (with the vsync GAP band + 16.7/33.3 ms
  guides), a click-to-open single-frame flame timeline, and avg/p95/max
  summaries. `window.__perfDump()` returns the same JSON string
- Pieces: the collector `window.__perf` (index.html plain script:
  `perfSpan`/`perfCount`/`perfFrameStart`/`perfFrameEnd`, all no-ops without
  the flag), the wasm `perf` module in src/app/perf.rs (drop-guard spans `sim`,
  `scenario`, `record`, `flush`; the Rust-side `enabled()` guard means a
  disabled run never crosses the boundary), and renderer.js sub-spans
  (`walk`, `sprites`, `submit`, `postfx` — they nest inside `flush` on the
  timeline) + counters (`cmds`, `draws` via a gl.drawArrays shim installed
  only when tracing, `fbos` = render-target switches via a gl.bindFramebuffer
  shim likewise, `robots`). Skipped FPS-cap frames never open a frame

## GPU probe (`?gpuprobe`)
- `?perf` only times the CPU. When the CPU spans are ~1 ms and the frame
  `gap` still sits at ~30 ms the machine is GPU-BOUND (the 2018 MacBook Air's
  UHD 617 at 2880 px wide is), and the frame loop is its own GPU timer: the
  browser paces `requestAnimationFrame` at the GPU's finish rate. `?gpuprobe`
  (`tools/gpu-probe.js`, wrapped around `frameRender` by renderer.js only
  when the flag is present) runs a KNOCKOUT experiment on that: ~2 s per
  configuration, 2 rounds, it REWRITES the command stream (walking it with
  the renderer's own `OP_ARGS`) to strip one class of work — BACKDROP, POSTFX
  13, all POSTFX, `STATIC_REF`, RECTs covering ≥ 25% of the screen, ROBOT /
  SHOGGOTH, TEXT, everything but CLEAR — and reports
  `period(baseline) − period(knockout)` per class, the baseline re-measured
  between rounds (thermal drift) and "clear only" = the floor cost of
  presenting the canvas at all. STRESS rows go past the vsync ceiling a
  knockout hits: `+1 backdrop` / `+1 static geo` / `+1` / `+3 blend rect`
  draw a layer TWICE (marginal cost), and `clear +6` / `clear +10 rect`
  give the header's `per full-screen layer` (slope) and `FIXED cost of
  presenting the canvas` (intercept). `STATIC_BEGIN..END` recordings always
  pass through whole (recorded once per floor). Result: on-screen table,
  console, clipboard, `window.__gpuProbe`. Measure with it BEFORE optimizing
  a layer. `?gpuprobe=fixed` = the QUICK probe (~10-15 s): just the fixed
  presentation cost + the per-layer cost (CLEAR + N invisible rects, N
  doubled until the period passes 24 ms so fast GPUs are measurable too,
  panel hidden while measuring) — for A/B-ing `?ctx=` flags across machines.
  `?gpuprobe=curve` = the whole ladder (CLEAR + N rects, slope per rung);
  `?gpuprobe=headroom` = the ladder ON TOP OF THE REAL GAME FRAME: how many
  extra full-screen layers the scene takes before leaving the vsync floor +
  the frame's actual GPU time from the fit past the knee.
  OBSERVER EFFECT (found by the curve mode): an HTML ELEMENT OVER THE CANVAS
  changes how the browser presents it — on the MacBook Air the full probe's
  visible result panel roughly DOUBLED everything it measured (bare clear
  ~4.5 -> 9.6 ms, layer 1.06 -> 1.8 ms). Every mode now hides its panel
  while measuring (progress in the tab title). Absolute ms figures recorded
  before that (the "9.6 ms fixed cost", "1.8 ms per layer" quoted here) are
  the WITH-OVERLAY regime: the relative A/B verdicts stand, the absolute
  numbers are ~2x pessimistic for the real game, which has nothing over its
  canvas. COROLLARY for the game itself: never leave a DOM element on top of
  the game canvas during play
- WHAT IT FOUND on that MacBook Air (30 -> 60 fps, none of it fill-rate
  tuning): (1) the batch uploaded every flush with `bufferSubData` at offset
  0 into one 2 MiB store — a write into a buffer the previous draw still
  reads = a stall per flush, ~20 per frame, 31 -> 19 ms; flushes now ORPHAN
  (`bufferData` of exactly the filled vertices; `?vbo=sub` = the old path).
  (2) the context's `alpha: false` cost 3.6 ms of FIXED presentation time
  per frame (13.3 -> 9.7 ms; an alpha-less buffer is emulated over Apple's
  always-alpha IOSurfaces): on Apple platforms the context is `alpha: true`
  and the blend keeps canvas alpha at 1 (`CTX_ALPHA` / `pixBlend`; `?ctx=`
  A/B flags in docs/URL_PARAMS.md). Reference numbers there: 1.8 ms per
  full-screen layer at 2880x1046, opaque or blended alike; the floor cache
  is ONE layer deep; `?pixel=N` is cost-neutral (its composite quad is the
  layer it saves); text and robots are ~free once flushes do not stall

