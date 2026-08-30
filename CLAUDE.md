## Development Constraints
- NEVER add any additional dependency

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

## Rendering Architecture
- The Rust/wasm engine owns the simulation only; **all rendering is WebGL in JS**
- Each frame, `Graphics` (src/graphics.rs) records a flat f32 command stream
  (rects, circles, lines, arcs, text, transforms, robots, the shoggoth) and
  hands it to `window.frameRender` once per frame — a single zero-copy
  wasm->JS crossing
- `renderer.js` owns the canvas/GPU: one batched triangle pipeline, VT323 text
  via a lazily-built glyph atlas, robots rendered LIVE every frame through the
  robot-core.js 3D->2D pipeline (`createRobotPipeline(gl)` on the same GL
  context, into a per-frame scratch tile atlas — continuous animation time, no
  cache/quantization); the boss the same way through shoggoth-core.js
  (`createShoggothPipeline(gl)`, a bigger 256px scratch tile, opcode SHOGGOTH
  = 13: `x y sizePx heading reveal time`)
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
  alpha-blended quad of a pre-rolled noise texture, never routing the
  frame through the scene FBO);
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
  `tests/e2e/composite-coherence.js` (standalone bun script, like
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
  frame-stable. `tests/e2e/props-stability.js` (standalone bun script) is the
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
  render_dialogue.rs draws the JRPG letterbox (a ~52 px black bar at the
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
  weaponIdx at angle 0 (24-texel art, `GUN_ART`) into the same persistent
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
- The command opcode tables in src/graphics.rs (`mod op`), renderer.js
  (incl. its `OP_ARGS` arity table used by the POSTFX pre-scan) and
  tests/e2e/specs/helpers.js (`OP_ARGS`) must stay in sync (ops 21-23's
  values live in `src/static_geo.rs`)

## Repo layout (post-`proto/`)
- Root: `index.html`, `renderer.js`, `robot-core.js` (the 3D->2D robot pipeline, imported by renderer.js at runtime), `shoggoth-core.js` (the boss pipeline, built on robot-core), `serve.py` (dev server, no-store + level-editor write API + the `/render-tests/<name>` route), `docs.html` (the `/docs` page: the RENDERING PIPELINE map — hand-built HTML/CSS mirroring the mermaid source in `docs/PIPELINE.md`, which GitHub renders — plus the persistent-vs-per-frame table, the cost model and links to every doc; serve.py routes `/docs` to it, `/docs/*.md` stay real files), `render-tests.html` (RENDER TESTS: a renderer-only harness — no wasm, no game — that drives `initRenderer`/`frameRender` with hand-built command streams so the smooth pixel-group composite can be eyeballed in isolation on any GPU; tests `square` (rocking black square), `sway` (the exact game sway over a checker/walls scene), `split` (smooth vs hard composite side by side); tweak via `?px=&amp=&period=&smooth=&zoom=`)
- `tools/`: the `?viz` panels — `inspector.html` (character inspector: `?kind=robot&color=…` / `?kind=shoggoth&phase=masked|enraged`, `&embed=1` for the SPRITES tab; 3D orbit + 2D top-down views), `levels.html` + `levels-editor*.js` (level + scenario editor, LEVELS tab) — and `gen_levels.py`, `gen_props.py`
- `levels/`: `floor_00.json` (the ground-level cold open: gate / parking lot, passive crowd), `floor_01..13.json`, `floor_13h.json`, `index.json` — the floors' single source of truth (format: `docs/SCENARIO_FORMAT.md`). Level *index* = position in `index.json` (sorted by id: index 0 = floor 0); `?floor=N` takes the floor **id** A floor may carry `"props": [{ "kind", "x", "y", "rot" (deg, cw), "size" (world units, default 100) }]` = placed set dressing (`kind` = a `PROP_NAMES` snake_case id, validated by `gen_levels.py` → `FloorDef.props: &[PropPlacement]`); DECORATION ONLY — drawn in-game by `src/floor_props.rs` (`render_floor_props`, called in `update_game` after the walls and before the actors, inside the `?pixel=N` world group), no collision
- `src/levels_data.rs` is GENERATED from `levels/*.json` by `make gen-levels`; `make check-levels` validates + checks it is current. Never hand-edit it.
- Cold-open engine bits (floor 0): `src/systems/passive.rs` — passive civilians (`"type": "passive"` spawns → `AIState::Passive`, brief in `AI.passive: PassiveAI`; the AI system delegates to `passive::update_passive`; `alert_passives` / any damage flips them hostile); scenario actions `alert` / `hold` / `look_at` (`scenario.rs` `AlertTarget` / `HoldDef` / `LookAtDef`; `ScenarioState::hold_active/hold_caption/look_at`; lib.rs `update_game` skips player input + `stop_player` while held, `render_hold_caption`, `Camera::set_cinematic`); `FloorDef.surface` (`src/level.rs` renders checker|asphalt|marble|concrete|grating); `ElevatorKind` lift|door|gate on entry/exits (`render_comms.rs` `draw_doorway` / `draw_gateway`); `"to": "surface"` → `scenario::SURFACE_EXIT` (floor id 0 is real now)
- `src/props.rs`: the PROP library, 60 props in three FAMILIES (`PROP_FAMILIES` = contiguous id ranges: DATACENTER 0–23 the server-floor set, OUTDOOR 24–41 the gate / parking lot for the planned floor 00 — cars, charge pad, main gate with its swing arm, guard booth, bollards, planter, lamp post, road decals, drone pad, scooter rack, drain, holo billboard, dumpster —, LOBBY 42–59 the welcome hall — reception desk, turnstiles, scanner arch, benches, plant, lobby holo, directory totem, vending, coffee corner, charge lockers, floor logo, call panel, velvet rope, extinguisher, credit kiosk, holo clock, welcome mat; `family_range` / `prop_family`; new props are APPENDED, ids are persisted in props/props.json) drawn imperatively from primitives as LAYERS (`PROP_LAYERS`: `LayerDef { name, pivot, bounds, rot: LayerRot::{None, Static(deg), Spin{hz}, Sway{deg,hz}, Anim(fn)}, pixel: PixelMode::{Before, After} }`; `draw_prop_layer(g, kind, layer, t)` draws one layer in its own frame; `draw_prop_ex(g, kind, center, size, t, px, &PropDrawOpts{visible, modes})` is the driver — per layer `translate(pivot)` then, with `px >= 2` (design units of the 100-box), a pixel group per layer either BEFORE its rotation (rotate, then group: the pixel image turns as a whole) or AFTER (group in the parent frame, rotate inside: re-rasterized on the parent grid); `px <= 1` = plain drawing, identical to the pre-layer look; `draw_prop` = `draw_prop_ex` with the saved settings — what a floor renderer should call). Nothing draws props on floors yet
- `props/props.json` = the SAVED per-prop `px` + per-layer before/after (format: `docs/PROPS_FORMAT.md`), written by the `?viz` PROPS page SAVE (`PUT /props/props.json`, serve.py, same token as levels) and compiled by `make gen-props` into `src/props_data.rs` (`PROP_SETTINGS`; GENERATED — never hand-edit); `make check-props` (in `make verify`) validates + checks it is current. `tools/gen_props.py` reads `PROP_NAMES` from props.rs for the order / kind ids (`snake_case` of the display name); layer names are checked by a props.rs unit test

## `?viz` toolbox (one entry point)
- `/?viz` tabs: SPRITES (two pages: CHARACTERS — click one → 3D/2D inspector
  iframe — and PROPS — the animated prop library from
  `src/props.rs`, wasm-drawn grid + big preview, one page per FAMILY
  (DATACENTER / OUTDOOR / LOBBY buttons right of SAVE; the 4-column grid is
  sized by the largest family, switching pages selects that family's first
  prop): PIXEL − / + edits the
  SELECTED prop's art-pixel size (1 = off … 10, design units; every tile
  draws at its own prop's px; tiles + preview are drawn at
  `props::snap_size` = an integer texel→device-pixel magnification), GRID overlays the prop's art grid on the
  preview, the LAYERS list under the preview has per layer an eye (hide in
  the preview), S (solo) and a BEFORE / AFTER pixel-mode toggle, SAVE PUTs
  `props/props.json` via `window.vizSaveProps` (index.html; token prompt +
  result toast) — then run `make gen-props`), MUSICS (tracker + SFX),
  LEVELS (the NATIVE level editor, see below), EFFECTS (previews
  every POSTFX shader kind + the 2D shoggoth glitch)
- `/?floor=N` starts the game directly on floor id N (0 = the gate / parking lot cold open, 14 = 13½); music starts on the first key/click. Add `&pixel=N` (N ≥ 2 WORLD units per art pixel, no gameplay change) to rasterize the SCENERY of `update_game` (floor, walls, props, elevators) at art resolution the vibe's way: the group's texel grid is WORLD-ANCHORED (its origin snaps to whole art pixels of the world, so panning never re-phases the texels) with a bleed margin, only `translate(-focus)`-style placement lands inside the group, and the camera's composite half (`Camera::apply_composite`: centre + drift + roll + zoom) sits OUTSIDE it — the sway moves/rotates the finished pixel image at native res through the sub-pixel composite (`pixel_begin_smooth`: no origin snap — gliding motion, NEAREST sampling, hard aliased edges per the art direction). The MOVING actors (robots, boss, bullets, weapons, gate arrow) draw AFTER the group closes, straight under the camera transform: they are already baked pixel sprites and must MOVE SMOOTHLY at native resolution — re-quantizing them onto the world grid makes a walking robot hop world-texel by world-texel and the whole scene FEEL snapped even though the backdrop glides. HUD/comms stay crisp (`pixel_world` in GameState); the static geometry cache stays active inside the group. Add `&noise=0` to turn the TV-static film grain off (title + in-game — the clean-image A/B switch). ALL url params: `docs/URL_PARAMS.md`. Add `&debug` (`/?floor=14&debug`) to enable the debug tooling: with debug overlays on (I), **K** purges all rogues (incl. the boss; debug/e2e helper) and **B** cracks the boss's mask (drops it to the enrage threshold so the live mask-off / raw form can be previewed)
- The ending (`src/ending.rs`): extracting through a `"to": "surface"` exit (`scenario::SURFACE_EXIT`; 13½'s car) → EXFILTRATED card → the `extracted` scenario step's UPLINK comms until the feed idles → 2.5 s blur-out (POSTFX 0) → `GameScreen::Ending` credits (the `CREDITS` const list) over the ELEVATOR RIDE HOME (`ending::render_ride`: the car top-down at dead centre, the live coral robot idling in it, shaft lights streaking outward) under POSTFX 10 WARP TRAILS (`Ending::warp_t` ramps in over ~6 s, holds, eases to an idle glow as the roll settles); Enter/Esc → level select
- Level editor SAVE = `PUT /levels/<file>.json` to serve.py, guarded by the `X-Editor-Token` header (token from `$EDITOR_TOKEN` or the gitignored `.editor-token`, printed at server start): the native editor through `window.vizSaveLevel(file, json)` (index.html, same token prompt + toast as `vizSaveProps`; then `make gen-levels`), the web editor directly (its COPY DIFF gives a `patch -p1` unified diff).

## Native level editor (`/?viz` → LEVELS)
- `src/editor.rs` (host-testable, no browser): the DOCUMENT — `EditableFloor` (`from_def(&FloorDef)`; owned strings / Vecs for entry + exits (`Car`), walls, `Room`s, `Zone`s, `Spawn`s, `Pickup`s, `PropPlacement`s; the `scenario` steps are carried through VERBATIM as `&'static [StepDef]` — the web editor owns those), `Item` (what is selectable: `Entry | Exit(i) | Wall(i) | Room(i) | Zone(i) | Spawn(i) | Pickup(i) | Prop(i)`; `rect_of` / `set_rect` / `translate` / `delete` / `hit_test` (smallest zone/room wins, props on top) / `add_*` (unique `room1`/`zone1`/`exit1` ids)), `validate(known_ids)` (what `gen_levels.py` rejects + spawns in walls + scenario refs to zones / exits), `EditorDoc` (undo / redo snapshot stacks, `UNDO_DEPTH` = 100, `begin_edit()` before every user-level mutation, `dirty()` vs the last-saved baseline), and the hand-written JSON writer (`Json` tree, `to_json()`: the documented key order, `props` omitted when empty, 2-space indent, small containers inlined ≤ 100 columns) — `levels_round_trip_byte_for_byte` proves every checked-in floor re-saves identically (so does the web editor's `stringify`), i.e. both editors can round-trip each other's files. No serde, no crates.
- `src/editor_ui.rs` (wasm-only): the immediate-mode UI, one `Editor` in `GameState` (`update(graphics, mouse, click, now)` from `update_visualizer` when the LEVELS tab is active, drawn UNDER the tab bar). Layout: tab bar (y 14..60) → row 1 (`<` FLOOR `>` picker, FIT, GRID, SNAP, UNDO, REDO, SAVE (lit when dirty), SCENARIO (web) → `viz_inspect("levels")` iframe positioned at `MAP_TOP` = 150 by index.html) → row 2 (tools `1 SELECT … 9 PROP` + the active tool's option) → the map pane (view = `pan + world * zoom`; the floor through the REAL renderer: `Level` tiles clipped to the floor, `render::draw_wall`, `render_comms::draw_elevator_car` (`CarView` + `car_back_side`), `floor_props::draw_placed_prop` live at their px; screen-space overlays: room washes / labels, cyan zone outlines + ids, spawn diamonds by type colour, gold weapon pickups, coral player start, selection + 8 resize handles, hover, rubber band, prop ghost) → the right panel (`PANEL_W` = 260: SELECTION properties strip — geometry, `id` / `label` text fields (click, type, Enter commits, Esc cancels; `input::typed_text()`), exit `to` −/+ and OPEN/CLOSED, spawn type, weapon, prop rot ±90 / size ±10, DELETE — then the PROP PALETTE (family pages from `PROP_FAMILIES`, live `draw_prop` thumbnails, click to pick + brush rot / size) or the KEYS map) → the status line (`validate()` result or the counts, transient notes, cursor world position + zoom).
- Keys: `1-9` tools · wheel zoom (`input::wheel_delta()`, canvas `wheel` listener) · `F` fit · middle / right drag or Space+drag = pan · `G` grid · `N` snap (10 u) · click / drag = select + move, handles resize · arrows nudge (Shift = 1 u) · `Del` / `Backspace` delete · `T` / `Q` cycle spawn type / weapon · `R` (Shift = −90) rotate the selected prop or the brush, `[` `]` size ±10 · `Ctrl+Z` / `Ctrl+Shift+Z` / `Ctrl+Y` undo / redo · `Esc` cancel drag / deselect / back to SELECT.
- SAVE = `validate` (refuses with the first problem on the status line) → `vizSaveLevel(file, to_json())` → toast "SAVED … — now run: make gen-levels". Floors keep their edits while you switch between them (one `EditorDoc` per floor, loaded lazily).

## Verification Requirements
- ALWAYS run `make verify` before declaring any task complete or saying "we're done"
- The `make verify` command runs core CI pipeline checks locally:
  - Code formatting (rustfmt) - `make check-fmt`
  - Linting (clippy) - `make check-clippy`
  - Test suite (all tests including doc tests) - `make check-test`
  - Release build - `make check-build`
- ALL checks must pass before completing a task
- If any check fails, fix the issues and re-run `make verify`

### Note on E2E Tests
- E2E tests (`make check-e2e`) require wasm-bindgen-cli build tool to be installed
- wasm-bindgen and web-sys are already in Cargo.toml as dependencies (no new dependencies needed)
- E2E tests are excluded from `make verify` but can be run separately with `make verify-all`
- The `make check-e2e` target will automatically install wasm-bindgen-cli if not present
- E2E tests require the wasm32-unknown-unknown Rust target and Playwright dependencies

#### **E2E Test Timeout Enforcement**
- Prefer running E2E tests via `make check-e2e` — it wires up the toolchain and the timeout for you
- The e2e toolchain runs on **Bun** (`bun install` / `bunx playwright ...`), not npm/node
- Both the Makefile and the Playwright config enforce a 60-second timeout so a run cannot hang indefinitely

## Perf Tracing (`?perf`)
- Opt-in per-frame trace across engine / boundary / renderer: add `perf` to
  the URL (e.g. `/?floor=2&debug&perf`), play a bit, press **P** — the last
  300 frames are logged to the console as one JSON blob (and copied to the
  clipboard, best-effort). Paste (or drag-drop) it into `tools/perf.html`
  for a stacked per-frame chart (with the vsync GAP band + 16.7/33.3 ms
  guides), a click-to-open single-frame flame timeline, and avg/p95/max
  summaries. `window.__perfDump()` returns the same JSON string
- Pieces: the collector `window.__perf` (index.html plain script:
  `perfSpan`/`perfCount`/`perfFrameStart`/`perfFrameEnd`, all no-ops without
  the flag), the wasm `perf` module in src/lib.rs (drop-guard spans `sim`,
  `scenario`, `record`, `flush`; the Rust-side `enabled()` guard means a
  disabled run never crosses the boundary), and renderer.js sub-spans
  (`walk`, `sprites`, `submit`, `postfx` — they nest inside `flush` on the
  timeline) + counters (`cmds`, `draws` via a gl.drawArrays shim installed
  only when tracing, `robots`). Skipped FPS-cap frames never open a frame

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
