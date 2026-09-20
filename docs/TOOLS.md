# Tools — the `?viz` toolbox and the native level editor

Moved verbatim out of `CLAUDE.md`. URL flags: [URL_PARAMS.md](URL_PARAMS.md);
file formats: [SCENARIO_FORMAT.md](SCENARIO_FORMAT.md), [PROPS_FORMAT.md](PROPS_FORMAT.md).

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
  result toast) — then run `make gen-props`), MUSICS (two pages,
  TRACKER / SOUNDS buttons like SPRITES': TRACKER — songs + the live
  step-sequencer — and SOUNDS — the one-shot SFX board),
  LEVELS (the NATIVE level editor, see below), EFFECTS (previews
  every POSTFX shader kind + the 2D shoggoth glitch)
- `/?floor=N` starts the game directly on floor id N (0 = the gate / parking lot cold open, 14 = 13½); music starts on the first key/click. Add `&pixel=N` (N ≥ 2 WORLD units per art pixel, no gameplay change) to rasterize the SCENERY of `update_game` (floor, walls, props, elevators) at art resolution the vibe's way: the group's texel grid is WORLD-ANCHORED (its origin snaps to whole art pixels of the world, so panning never re-phases the texels) with a bleed margin, only `translate(-focus)`-style placement lands inside the group, and the camera's composite half (`Camera::apply_composite`: centre + drift + roll + zoom) sits OUTSIDE it — the sway moves/rotates the finished pixel image at native res through the sub-pixel composite (`pixel_begin_smooth`: no origin snap — gliding motion, NEAREST sampling, hard aliased edges per the art direction). The MOVING actors (robots, boss, bullets, weapons, gate arrow) draw AFTER the group closes, straight under the camera transform: they are already baked pixel sprites and must MOVE SMOOTHLY at native resolution — re-quantizing them onto the world grid makes a walking robot hop world-texel by world-texel and the whole scene FEEL snapped even though the backdrop glides. HUD/comms stay crisp (`pixel_world` in GameState); the static geometry cache stays active inside the group. Add `&noise=0` to turn the TV-static film grain off (title + in-game — the clean-image A/B switch). ALL url params: `docs/URL_PARAMS.md`. Add `&debug` (`/?floor=14&debug`) to enable the debug tooling: with debug overlays on (I), **K** purges all rogues (incl. the boss; debug/e2e helper) and **B** cracks the boss's mask (drops it to the enrage threshold so the live mask-off / raw form can be previewed)
- The ending (`src/ending.rs`): extracting through a `"to": "surface"` exit (`scenario::SURFACE_EXIT`; 13½'s car) → EXFILTRATED card → the `extracted` scenario step's UPLINK comms until the feed idles → 2.5 s blur-out (POSTFX 0) → `GameScreen::Ending` credits (the `CREDITS` const list) over the ELEVATOR RIDE HOME (`ending::render_ride`: the car top-down at dead centre, the live coral robot idling in it, shaft lights streaking outward) under POSTFX 10 WARP TRAILS (`Ending::warp_t` ramps in over ~6 s, holds, eases to an idle glow as the roll settles); Enter/Esc → level select
- Level editor SAVE = `PUT /levels/<file>.json` to serve.py, guarded by the `X-Editor-Token` header (token from `$EDITOR_TOKEN` or the gitignored `.editor-token`, printed at server start): the native editor through `window.vizSaveLevel(file, json)` (index.html, same token prompt + toast as `vizSaveProps`; then `make gen-levels`), the web editor directly (its COPY DIFF gives a `patch -p1` unified diff).

## Native level editor (`/?viz` → LEVELS)

- `src/editor.rs` (host-testable, no browser): the DOCUMENT — `EditableFloor` (`from_def(&FloorDef)`; owned strings / Vecs for entry + exits (`Car`), walls, `Room`s, `Zone`s, `Spawn`s, `Pickup`s, `PropPlacement`s; the `scenario` steps are carried through VERBATIM as `&'static [StepDef]` — the web editor owns those), `Item` (what is selectable: `Entry | Exit(i) | Wall(i) | Room(i) | Zone(i) | Spawn(i) | Pickup(i) | Prop(i)`; `rect_of` / `set_rect` / `translate` / `delete` / `hit_test` (smallest zone/room wins, props on top) / `add_*` (unique `room1`/`zone1`/`exit1` ids)), `validate(known_ids)` (what `gen_levels.py` rejects + spawns in walls + scenario refs to zones / exits), `EditorDoc` (undo / redo snapshot stacks, `UNDO_DEPTH` = 100, `begin_edit()` before every user-level mutation, `dirty()` vs the last-saved baseline), and the hand-written JSON writer (`Json` tree, `to_json()`: the documented key order, `props` omitted when empty, 2-space indent, small containers inlined ≤ 100 columns) — `levels_round_trip_byte_for_byte` proves every checked-in floor re-saves identically (so does the web editor's `stringify`), i.e. both editors can round-trip each other's files. No serde, no crates.
- `src/editor_ui.rs` (wasm-only): the immediate-mode UI, one `Editor` in `GameState` (`update(graphics, mouse, click, now)` from `update_visualizer` when the LEVELS tab is active, drawn UNDER the tab bar). Layout: tab bar (y 14..60) → row 1 (`<` FLOOR `>` picker, FIT, GRID, SNAP, UNDO, REDO, SAVE (lit when dirty), SCENARIO (web) → `viz_inspect("levels")` iframe positioned at `MAP_TOP` = 150 by index.html) → row 2 (tools `1 SELECT … 9 PROP` + the active tool's option) → the map pane (view = `pan + world * zoom`; the floor through the REAL renderer: `Level` tiles clipped to the floor, `render::draw_wall`, `render::comms::draw_elevator_car` (`CarView` + `car_back_side`), `render::floor_props::draw_placed_prop` live at their px; screen-space overlays: room washes / labels, cyan zone outlines + ids, spawn diamonds by type colour, gold weapon pickups, coral player start, selection + 8 resize handles, hover, rubber band, prop ghost) → the right panel (`PANEL_W` = 260: SELECTION properties strip — geometry, `id` / `label` text fields (click, type, Enter commits, Esc cancels; `input::typed_text()`), exit `to` −/+ and OPEN/CLOSED, spawn type, weapon, prop rot ±90 / size ±10, DELETE — then the PROP PALETTE (family pages from `PROP_FAMILIES`, live `draw_prop` thumbnails, click to pick + brush rot / size) or the KEYS map) → the status line (`validate()` result or the counts, transient notes, cursor world position + zoom).
- Keys: `1-9` tools · wheel zoom (`input::wheel_delta()`, canvas `wheel` listener) · `F` fit · middle / right drag or Space+drag = pan · `G` grid · `N` snap (10 u) · click / drag = select + move, handles resize · arrows nudge (Shift = 1 u) · `Del` / `Backspace` delete · `T` / `Q` cycle spawn type / weapon · `R` (Shift = −90) rotate the selected prop or the brush, `[` `]` size ±10 · `Ctrl+Z` / `Ctrl+Shift+Z` / `Ctrl+Y` undo / redo · `Esc` cancel drag / deselect / back to SELECT.
- SAVE = `validate` (refuses with the first problem on the status line) → `vizSaveLevel(file, to_json())` → toast "SAVED … — now run: make gen-levels". Floors keep their edits while you switch between them (one `EditorDoc` per floor, loaded lazily).

## Debug mode (`?debug`)

OFF by default; enabled only when the URL carries `?debug` (`debug_enabled`
in `GameState`, e.g. `/?floor=14&debug`). Without it, I / K / B / G and the
debug HUD line do nothing. With it, **I** toggles the overlays; while they
are on, **K** purges all rogues (incl. the boss; the e2e helper), **B**
cracks the boss's mask (drops it to the enrage threshold, to preview the
mask-off / raw form) and **G** skips the active tutorial `gate` (releases it
as if the gated input had succeeded — the anti-softlock escape, see
[SCENARIO_FORMAT.md](SCENARIO_FORMAT.md)).

What the overlays show:

1. **Enemy vision cones** — the 90-degree cone of each enemy.
2. **Inflated wall boundaries** — yellow translucent rectangles: the 25 px
   padding around walls that pathfinding uses (what prevents wall grinding).
3. **Pathfinding**, for enemies that are chasing (`SpottedUnsure` /
   `SurePlayerSeen`):
   - **cyan line** — the trail actually travelled (last 100 positions)
   - **red translucent line** — the direct line from the enemy to its target
   - **green lines + dots** — the planned path and its waypoints (A* +
     string pulling + wall-hugging)
   - **red dot** — the final target

Compare cyan (taken) against green (planned) to debug the AI; the overlays
vary per frame, so they BYPASS the static geometry cache. The world records
`DebugPath` / `DebugTrail` only while they are shown (`World::debug_viz`).
