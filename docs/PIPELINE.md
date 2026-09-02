# The rendering pipeline

One frame, end to end. GitHub renders the mermaid blocks below as diagrams;
the same map, drawn by hand, lives at the dev server's `/docs` page
(docs.html — self-contained, no libraries, per the no-dependency rule).

## The big picture

The Rust/wasm engine owns the **simulation only**. Each frame it records
everything drawable into one flat `f32` command stream (CSS-pixel
coordinates) and hands it to JS **once** — a single zero-copy crossing.
`renderer.js` owns the canvas and the GPU: one batched triangle pipeline,
plus dedicated passes for the things a batch can't express.

```mermaid
flowchart TD
  RAF["requestAnimationFrame<br/>(the FPS cap may skip the whole frame)"] --> UPDATE
  subgraph WASM["wasm engine (Rust) — simulation"]
    UPDATE["update(): input, sim, AI, scenario"] --> REC["render_*(): record the frame<br/>Graphics = flat f32 command stream, CSS px"]
    REC --> OPS["ops: rect / line / circle / arc / text / transforms<br/>ROBOT · SHOGGOTH · PORTRAIT · GUN_PICKUP<br/>PIX_BEGIN/END/BLIT · STATIC_BEGIN/END/REF<br/>DRIVE · POSTFX"]
  end
  OPS ==>|"window.frameRender(cmds, texts)<br/>ONE zero-copy wasm→JS crossing per frame"| SCAN

  subgraph RJS["renderer.js (WebGL1) — owns the canvas + GPU"]
    SCAN["POSTFX pre-scan<br/>(kinds 0–12 route the frame into the scene FBO)"] --> WALK["opcode walk"]
    WALK --> BATCH["batched triangles<br/>(CPU transform stack, ONE shader,<br/>flushed on texture/target changes)"]
    WALK --> STATIC["STATIC geometry cache<br/>floor + walls tessellated ONCE into a persistent<br/>world-space VBO; later frames send a 2-float REF,<br/>the camera applies in the vertex shader (uXA/uXB)"]
    WALK --> PIX["PIXEL-ART GROUPS<br/>batch redirected into an art-res NEAREST scratch FBO;<br/>END composites the finished image as one quad —<br/>origin-snapped (props), or sub-pixel for the world<br/>(gliding motion; sampling stays NEAREST: aliasing is<br/>the art direction)"]
    WALK --> LIVE["robots / boss — LIVE 3D→2D every frame<br/>robot-core / shoggoth-core render into<br/>per-frame scratch tile atlases (NEAREST);<br/>robots as ONE batch: tiles of one pass-1 target,<br/>one post draw at block resolution"]
    WALK --> BAKE["portraits / ground guns — BAKED ONCE<br/>into a persistent NEAREST atlas,<br/>then drawn as rigid rocking / spinning quads"]
    WALK --> TXT["text — VT323 glyph atlas<br/>(lazily baked, all-caps at the draw boundary)"]
    WALK --> DRV["DRIVE backdrop — one full-shader pass<br/>at art resolution → one upscaled quad"]
  end

  LIVE --> BATCH
  BAKE --> BATCH
  TXT --> BATCH
  STATIC --> FB
  PIX --> FB
  DRV --> FB
  BATCH --> FB[("default framebuffer<br/>physical px = CSS × devicePixelRatio")]

  FB -->|"POSTFX 0–12 active"| SCENE[("scene FBO")] --> POST["full-screen post shader<br/>(blur-out, CRT, VHS, modal static, …;<br/>kind 10 = warp-trails feedback accumulator)"] --> FB
  FB --> GRAIN["TV static (POSTFX 13) — NOT a post pass:<br/>ONE alpha-blended quad over the finished frame,<br/>sampling a pre-rolled 512² noise texture at a random<br/>offset (cost = one full-screen read + write)"]
```

## Anatomy of an in-game frame

Draw order inside `update_game`, top of the list = drawn first (underneath):

```mermaid
flowchart TD
  A["clear"] --> B["WORLD SCENERY — floor tiles + walls<br/>(the STATIC cache VBO), placed props, elevators<br/>· with ?pixel=N: all of this rasterizes inside the<br/>world-anchored art-res group, then composites as one<br/>sub-pixel quad under the camera sway"]
  B --> C["ACTORS — corpses, then ground weapons,<br/>then standing robots + boss + bullets<br/>(baked/live pixel sprites on quads that move<br/>smoothly at native res — never world-grid-snapped)"]
  C --> D["HUD + comms — health, objective, dialogue<br/>letterbox, portraits (crisp, outside any group)"]
  D --> E["TV-static grain quad (?noise=0 removes it)"]
  E --> F["POSTFX full-screen pass, if any<br/>(pause modal wash, kill blur-out, warp trails…)"]
```

## What persists vs what is per-frame

| Persistent (built once, reused) | Per-frame |
| --- | --- |
| STATIC cache VBO (floor + walls of the current floor; evicted on floor change) | everything dynamic in the batch (actors, props' layers, HUD) |
| portrait / ground-gun atlas (baked per color / weapon on first use) | robot + boss tiles (LIVE 3D→2D, continuous animation time) |
| VT323 glyph atlas (grown lazily) | pixel-group scratch FBOs' contents (re-rasterized when used) |
| 512² TV-noise texture (rolled once at startup) | the noise quad's random UV offset |
| DRIVE / warp / scene FBO targets (allocated lazily, reused) | their contents |

## The cost model (why things are the way they are)

- **Bandwidth is the enemy** on fill-rate-poor GPUs at Retina: the
  framebuffer is physical-resolution, and every full-screen layer is a
  read (+ write) of ~4.5M pixels. MSAA (~4x that, per layer) was measured
  at 30 fps on the 2018 MacBook and removed; the TV grain (one blended
  quad = the cheapest possible full-screen overlay) still costs one
  read + write, which is visible on the FPS counter at cap 120.
- **The command stream is the CPU contract**: one crossing per frame,
  and the STATIC cache exists to shrink it (a floor's tiles + walls are
  ~2000 floats re-recorded per frame without it, 2 floats with it).
- **Art-res rasterization is a fill win**: a pixel group (or the DRIVE
  shader) computes at art resolution and pays native resolution only for
  one upscaled quad — the crunch and the savings come from the same place.
- **Aliasing is the art direction** (see CLAUDE.md ## Design): hard
  stair-stepped edges everywhere, smoothness only in motion (sub-pixel
  composite placement, continuous 60 fps animation).
