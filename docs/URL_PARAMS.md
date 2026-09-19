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
| `gpuprobe` | flag | GPU KNOCKOUT EXPERIMENT (`tools/gpu-probe.js`): for ~45 s the renderer strips one class of work at a time from the command stream (backdrop, TV static, postfx, cached floor + walls, big rects, robots, text, … down to "clear only") and reads each one's GPU cost off the frame PERIOD — on a GPU-bound machine the frame loop is its own GPU timer, which `?perf` (CPU spans only) cannot see. Stand still while it runs; the table lands on screen, in the console, on the clipboard and in `window.__gpuProbe`. `gpuprobe=fixed` = the QUICK probe (~10–15 s, nothing to stand still for): only the FIXED cost of presenting the canvas + the cost per full-screen layer, from CLEAR + N invisible rects with N ramped until the GPU is the limit — the one to A/B context flags across machines (`&ctx=alpha` vs `&ctx=opaque`). `gpuprobe=curve` = period vs load (~35 s): CLEAR + N rects over a ladder of N with the local slope per rung — a straight line above the vsync floor whose slope is the cost per layer and whose intercept is the bare-clear cost (N rungs on the vsync floor say nothing). `gpuprobe=headroom` = the same ladder ON TOP OF THE REAL GAME FRAME: how many extra full-screen layers the scene takes before it drops below the display rate, and the frame's actual GPU time from the fit past the knee. In every mode the result panel stays HIDDEN while measuring — an HTML element over the canvas measurably changes how the browser presents it. |
| `ctx=…` | comma list of `alpha`, `opaque`, `sync`, `lowpower` | A/B switches for how the WebGL context is created — the canvas PRESENTATION path, whose fixed per-frame GPU cost `?gpuprobe` reports. Default: an `alpha: true` buffer kept opaque by the blend on Apple platforms (measured 3.6 ms/frame cheaper than `alpha: false` on a 2018 MacBook Air), `alpha: false` elsewhere. `alpha` / `opaque` force the buffer kind, `sync` drops the `desynchronized` hint, `lowpower` drops the high-performance GPU request. |
| `backdrop=full` | `full` | Draw the whole void backdrop quad, ignoring the floor-occlusion rect the wasm sends with it (default: only the part the floor does not cover is drawn — pixel-identical, `tests/e2e/backdrop-clip.js`). The A/B for `?gpuprobe`. |
| `grain=fold` | `fold` | EXPERIMENT: fold the TV static into the batch fragment shader instead of drawing it as one alpha-blended full-screen quad at the end of the frame (same pixels to 8-bit rounding — `tests/e2e/grain-fold.js`). A trade, not a free win — measured on a 2018 MacBook Air: the game frame gets cheaper (8.6 -> 6.8 ms GPU) but every batch fragment gets ~47% dearer, so the frame tolerates fewer extra full-screen layers (5.6 -> 4.7). Off by default. |
| `vbo=sub` | `sub` | Batch vertex upload through `bufferSubData` into one preallocated 2 MiB store (the old path) instead of a right-sized `bufferData` per flush (orphaning, the default). The A/B for the per-flush stall `?gpuprobe` found on ANGLE Metal. |
| `rig=cpu` | `cpu` | Draw the robots through the CPU rig (JS pose matrices, one draw per robot) instead of the default GPU rig (skeleton in the vertex shader, the whole batch in one instanced draw). Same pixels — the A/B switch for `?perf` traces (`sprites` span, `draws` counter). |
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

## Rig parity (`tools/rig-parity.html`)

The robots' CPU rig vs GPU rig, every pose x weapon, side by side with a
texel diff (what `tests/e2e/rig-parity.js` asserts). `bench=N` also times N
batches of 64 robots through each rig (CPU submit cost on the machine at
hand).

## Character inspector (`tools/inspector.html`)

`kind=robot` (default) or `kind=shoggoth` · `color=<palette name>` ·
`pose=<pose>` · `weapon=<weapon>` · `phase=masked|enraged` (shoggoth) ·
`tess=<level>` (shoggoth sphere tessellation) · `px=1..12` (art pixel size) ·
`embed=1` (panel layout for the `?viz` SPRITES iframe).

## Web level editor (`tools/levels.html`)

`floor=<id>` (select a floor, kept in sync as you switch) · `dir=samples`
(edit `levels/samples/` instead of `levels/`) · `embed` (iframe layout for
the `?viz` LEVELS tab).
