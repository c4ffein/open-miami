# URL parameters

Every query flag the project understands, by page. Flags combine freely
(`/?floor=2&pixel=3&noise=0&debug`).

## The game (`/`)

| Param | Values | What it does |
| --- | --- | --- |
| `floor=N` | floor **id** 0–14 | Start directly on that floor (0 = the gate / parking-lot cold open, 1–13 the tower, 14 = 13½). Music starts on the first key/click. |
| `pixel=N` | N ≥ 2, world units per art pixel | PIXELATED SCENERY: floor, walls, props and elevators rasterize at art resolution on a world-anchored grid and the finished image glides/sways under the camera through the sub-pixel composite (hard aliased pixel edges — the art direction; smoothness is in the motion only). Actors (robots, boss, bullets, weapons) and the HUD stay native-smooth baked sprites — the Hotline-Miami layering. Off when absent. |
| `noise=0` | `0`, `false`, `off` | Turn the TV-static film grain OFF (title screen and in-game alike). Default on. The clean-image A/B switch for judging the pixelated world. |
| `debug` | flag | Enable the debug tooling: **I** toggles the overlays (vision cones, inflated wall boundaries, pathfinding); with overlays on, **K** purges all rogues, **B** cracks the boss's mask, **G** skips the active tutorial gate. Off-limits without the flag. |
| `perf` | flag | Per-frame perf tracing across engine / boundary / renderer. Play, then press **P**: the last 300 frames dump as one JSON blob (console + clipboard) — paste or drag it into `tools/perf.html`. |
| `ending` | flag | Jump straight to the credits ride (dev shortcut, same spirit as `floor=N`). |
| `viz` | flag | The asset toolbox instead of the game: SPRITES / MUSICS / LEVELS / EFFECTS tabs. |

## Docs (`/docs`)

The rendering-pipeline page (`docs.html`): the frame's journey from the wasm
command stream through renderer.js to the framebuffer, the persistent-vs-
per-frame table and the cost model. Its mermaid source is
`docs/PIPELINE.md` (GitHub renders it as a diagram).

## Render tests (`/render-tests/<name>`)

`render-tests.html`: a renderer-only harness (no wasm, no game, no font) that
drives `initRenderer`/`frameRender` with hand-built command streams — the
sub-pixel pixel-group composite in isolation, for eyeballing on any GPU. The
dev server routes `/render-tests/<name>`; on static hosting use
`render-tests.html?t=<name>`. `/render-tests/` lists the tests.

| Test | Scene |
| --- | --- |
| `square` | A black square gently rocking through the sub-pixel composite — the minimal pixelated world. Hard stair-stepped edges (the art direction), rigid chunky interior, gliding motion. |
| `sway` | The exact in-game camera sway (0.35° roll @ 0.11 Hz + 2.5 px drift) over a checker/walls scene at game art scale. What `?pixel=N` does to the scenery, minus the game. |
| `split` | The same rocking scene twice: left through the sub-pixel composite (what the game uses — gliding motion), right through the snapped one (motion quantized). Edges hard in both; the difference is the motion. |

Tweaks (all optional): `px=` art pixel size · `amp=` rock amplitude in
degrees · `period=` rock period in seconds · `smooth=0|1` composite kind ·
`zoom=` outer scale.

## Character inspector (`tools/inspector.html`)

`kind=robot` (default) or `kind=shoggoth` · `color=<palette name>` ·
`pose=<pose>` · `weapon=<weapon>` · `phase=masked|enraged` (shoggoth) ·
`tess=<level>` (shoggoth sphere tessellation) · `px=1..12` (art pixel size) ·
`embed=1` (panel layout for the `?viz` SPRITES iframe).

## Web level editor (`tools/levels.html`)

`floor=<id>` (select a floor, kept in sync as you switch) · `dir=samples`
(edit `levels/samples/` instead of `levels/`) · `embed` (iframe layout for
the `?viz` LEVELS tab).
