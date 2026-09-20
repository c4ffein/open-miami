/* =========================================================================
   OPEN MIAMI — WebGL renderer.

   The Rust/wasm engine owns the simulation and describes each frame as a
   flat Float32Array command stream (plus a \x1f-separated text arena),
   handed over once per frame through window.frameRender. This module owns
   the canvas and the GPU: it executes the stream with a single batched
   triangle pipeline.

   Command opcodes — mirror of `mod op` in src/graphics.rs. Keep in sync.
     0 CLEAR      r g b a
     1 RECT       x y w h  r g b a
     2 RECT_LINES x y w h thickness  r g b a
     3 CIRCLE     x y radius  r g b a
     4 LINE       x1 y1 x2 y2 thickness  r g b a
     5 ARC        x y radius a0 a1  r g b a          (filled pie slice)
     6 TEXT       textIdx x y size  r g b a          (left / baseline)
     7 SAVE
     8 RESTORE
     9 TRANSLATE  x y
    10 ROTATE     angle
    11 ROBOT      colorIdx weaponIdx flags x y angle sizePx + 11 pose scalars
    12 SCALE      sx sy
    13 SHOGGOTH   x y sizePx heading reveal time
    14 POSTFX     kind t r g b                        (full-screen post pass)
    15 PIX_BEGIN  px w h smooth                       (open a pixel-art group)
    16 PIX_END    x y                                 (close it, draw at x y)
    17 PORTRAIT   colorIdx x y sizePx time mode       (dialogue portrait: baked-once
                                                      pixel-art face, rocked in 2D
                                                      by `time`; mode 0 = bust,
                                                      1 = headshot)
    18 GUNPICKUP  weaponIdx x y angle sizePx          (weapon lying on the floor:
                                                      baked-once pixel-art sprite
                                                      of its 3D model, quad spun
                                                      in 2D by `angle`)
    19 PIX_BLIT   sx sy sw sh x y                     (re-draw a rect of the
                                                      LAST-closed pixel group
                                                      at x y — its texels
                                                      persist until the next
                                                      PIX_BEGIN)
    20 DRIVE      w h t glitch split px dim o0..o8    (the synthwave drive
                                                      backdrop: every pixel
                                                      computed in one opaque
                                                      full-shader pass; Rust
                                                      ships the tested tear /
                                                      split schedules)
    21 STATIC_BEGIN key                               (static geometry cache:
                                                      tessellate the section
                                                      once into a persistent
                                                      world-space VBO under
                                                      `key`, and draw it)
    22 STATIC_END                                     (close the recording)
    23 STATIC_REF  key                                (draw the cached VBO
                                                      with the current CPU
                                                      transform applied in
                                                      the vertex shader)
    24 BACKDROP   w h t px ex ey ew eh                (the neon-wave void; e* =
                                                      the rect the floor will
                                                      cover, NOT drawn: strips
                                                      around it — ew <= 0: none;
                                                      behind/outside the
                                                      level: art-res shader
                                                      pass + one upscaled
                                                      quad, like DRIVE)
    25 HEAD       colorIdx x y angle sizePx           (detached robot head on
                                                      the floor: baked-once
                                                      pixel-art sprite, quad
                                                      spun in 2D by `angle`)

   Everything is drawn as vertex-colored, textured triangles in one
   interleaved dynamic buffer (a 1x1 white texture stands in for solid
   geometry), so a frame typically costs a handful of draw calls: the batch
   only breaks when the bound texture changes (solids -> robot atlas ->
   solids -> glyph atlas -> shoggoth atlas).

   Text: VT323 ("GameFont") glyphs are rasterized lazily into a glyph-atlas
   texture via a scratch 2D canvas, then drawn as quads like everything
   else.

   Robots: true live 3D->2D, every frame, at continuous animation time. Each
   ROBOT command reserves a tile in a per-frame scratch atlas and queues a
   robot-core render. The queued robots run as ONE BATCH inside this same GL
   context right before the batch that samples them is drawn (robot-core's
   batchBegin / batchDraw / batchEnd): pass 1 draws every robot's lit boxes
   into its own tile of a single 1024² scene target (one clear) — through
   robot-core's GPU RIG: the skeleton runs in the vertex shader from a dozen
   per-instance pose scalars, so the whole batch is ONE instanced draw
   (`?rig=cpu` = the old CPU rig, JS pose matrices + one draw per robot) — and
   pass 2 is ONE tile-aware edge-ink / posterize / pixelate draw over all the
   tiles into the atlas, AT BLOCK RESOLUTION (ROBOT_ART = ceil(128 / 3) = 43
   NEAREST texels per robot — one per pixelate block, the exact image a 1:1
   post pass gives, without the 9 redundant copies of each block). So N robots
   cost one instanced scene draw + one post draw + N textured quads, with no tile
   cache, no quantization of the animation and no CPU readback.

   Shoggoth (the boss): the same mechanism through shoggoth-core.js — a SHOGGOTH
   command reserves a bigger tile in its own scratch atlas and queues a live
   render of the mass / mask / tentacles at (heading, reveal, time); the tile is
   drawn as an axis-aligned quad through the transform stack (its facing is
   baked into the render itself, not a quad rotation).

   POSTFX: when a frame's stream contains opcode 14 (found by a cheap pre-scan
   over the opcode table), the whole frame is rendered into an offscreen scene
   framebuffer instead of the canvas, then drawn through a full-screen post
   shader. The kinds are a menu of Hotline-Miami-flavoured looks:
     0 BLUR-OUT      growing multi-tap blur + dissolve toward the colour (the ending)
     1 SYNTHWAVE CRT scanlines, chromatic split, vignette, grain (the credits)
     2 VHS TAPE      tracking band, per-line jitter, chroma bleed, dropouts
     3 DRUNK SWAY    slow rotation/zoom breathing, wavy warp, ghosting, hue drift
     4 CRT TUBE      barrel distortion, aperture grille, hard scanlines, flicker
     5 ACID TRIP     radial hue cycling, oversaturation, posterize, liquid warp
     6 DATAMOSH      slice/block displacement glitch, channel swaps, noise blocks
     7 NEON BLOOM    bright-pass glow, shadow tint toward the colour
     8 PIXEL MOSAIC  chunky pixelation + dithered posterize
     9 TUNNEL RUSH   radial zoom blur toward the centre (adrenaline)
    10 WARP TRAILS   FEEDBACK: a persistent ping-pong accumulator is pulled
                     toward the centre each frame (so its content streams
                     OUTWARD), faded, and re-fed the scene's bright saturated
                     pixels — long-exposure radial light trails (the ending
                     elevator ride); the colour tints the trail decay. The
                     accumulator is cleared whenever the previous frame did
                     not use kind 10, so the effect always starts clean.
   All kinds share the args `kind t r g b` (t = 0..1 strength, rgb = the
   effect's colour where it uses one). Only the last POSTFX of a frame applies.

   PIXEL-ART GROUPS (opcodes 15/16): the clean way to pixelate primitive-drawn
   content is not to average or point-sample a hi-res image but to RASTERIZE
   AT THE ART RESOLUTION and upscale nearest. PIX_BEGIN `px w h` flushes,
   points the batch at a scratch framebuffer (a region of ceil(w/px) x
   ceil(h/px) texels of a 1024x1024 NEAREST-filtered texture, cleared
   transparent) and installs the transform scale(1/px), so the group's local
   0..w x 0..h maps onto those texels; everything until PIX_END is drawn there
   with hard coverage (FBOs carry no MSAA; the batch shader has no smoothing),
   so a shape's edge either owns a texel or it does not — every art pixel is a
   full pixel and the grid is anchored to the object. Inside a group line /
   outline thickness is clamped to >= 1 texel and circle radius to >= 0.5
   texel so hairlines survive. PIX_END `x y` flushes, restores the outer
   target + transform, and draws the group texture as a (w, h) quad at (x, y)
   in the outer transform (through it: a rotation in force at PIX_BEGIN
   rotates the finished pixel image), origin snapped to whole pixels of the
   target it lands in. Groups NEST up to PIX_DEPTH (4) deep: each depth owns
   its own scratch texture + FBO, an inner PIX_END composites its texels into
   the enclosing group's target (premultiplied, NEAREST), whose grid the
   origin snaps to. A group over the 1024-texel cap or beyond the depth cap
   is drawn pass-through (no pixelation; its PIX_END is a no-op). Robots /
   the boss can be drawn inside a group (their tiles composite into the
   group's texels). Inside a group primitives obey the PIXEL-ART RULE at
   rasterization time (see solidRect / circle / line): axis-aligned rects
   get a whole-texel size (rounded once, min 1) and a whole-texel origin,
   circles of radius <= 2 texels a half-texel radius and a grid-snapped
   centre (texel centre for odd diameters, corner for even), lines a
   whole-texel thickness and texel-centre endpoints — so a moving shape keeps
   one constant stamp and hops texel by texel. Circles are always tessellated
   in target space (fixed polygon phase), so a circle under a rotating
   transform (a fan's well / hub) never changes its rasterization.
   ========================================================================= */

import { createRobotPipeline, planFromScalars, POSE_SCALARS } from "./robot-core.js";
import { wrapGpuProbe } from "./gpu-probe.js";
import { createShoggothPipeline } from "./shoggoth-core.js";
import { OP, OP_ARGS, TEXT_SEP } from "./ops.js";
import {
  VS, FS, POST_VS, POST_FS, WARP_FS, DRIVE_VS, DRIVE_FS, BACKDROP_FS,
} from "./renderer/shaders.js";

const OP_POSTFX = OP.POSTFX;

/* ---- robot tables (indices mirror src/graphics.rs draw_robot; there is no
   pose table: the engine sends the pose as NUMBERS, src/render/pose.rs) ---- */
const ROBOT_COLORS = ["coral", "red", "violet", "magenta"];
const ROBOT_WEAPONS = ["fist", "pistol", "machinegun", "shotgun"];
const ROBOT_TILE = 128; // per-robot pass-1 scene resolution (texels)
const ROBOT_PX = 3; // robot-core pixelation block size at this tile size
// Atlas tile side: one NEAREST texel per pixelate block (the post pass writes
// the batch at block resolution; the quad shows ROBOT_TILE / ROBOT_PX = 42.67
// of them — the last block is a partial one, exactly as at 1:1).
const ROBOT_ART = Math.ceil(ROBOT_TILE / ROBOT_PX); // 43
const ROBOT_COLS = 8; // batch layout: 8x8 = 64 robots per batch; more just flush early
const ROBOT_ATLAS_SIZE = 512; // >= ROBOT_COLS * ROBOT_ART (344) texels

/* ---- shoggoth (boss) scratch tiles ---- */
const SHOG_TILE = 384; // the boss is large (and drawn ~1:1 at the camera zoom)
const SHOG_PX = 4; // shoggoth-core pixelation block size at this tile size
const SHOG_ATLAS_SIZE = 768; // 2x2 = 4 bosses per batch (one is the norm)

/* ---- pixel-sprite tiles (PORTRAIT + GUNPICKUP) ----
   Like the robot atlas, these tiles are NEAREST-filtered and rendered AT THE
   ART RESOLUTION, then upscaled by the quad that samples them — true pixel
   art, never smoothed. One 64px tile per sprite; a ground gun uses a
   GUN_ART-texel corner of its tile. */
const FX_TILE = 64; // tile side = the portrait's art resolution (texels)
const GUN_ART = 32; // ground-gun art resolution (texels) within a tile
// (32 divides ROBOT_TILE exactly: renderGun's pixelate block is a clean
//  128/32 = 4, and the detailed gun silhouettes get room to read)
const HEAD_ART = 16; // detached-head art resolution (texels) within a tile
/* ---- pixel-sprite cache (PORTRAIT + GUNPICKUP) ----
   Classic Hotline-Miami-style portraits: each (colorIdx, mode) face is
   rendered through the 3D pipeline exactly once — FIXED camera at the base
   yaw, frozen pose — into a persistent NEAREST atlas, then drawn every frame
   as that baked image on a quad that gently ROCKS in 2D around its centre
   (the finished pixel art tilts as a rigid sprite, chunky pixels and all).
   GROUND GUNS share the atlas: each weaponIdx is rendered once at angle 0
   (the true top-down camera makes spinning the flat model and rotating its
   baked sprite equivalent) and the quad is spun in 2D by the opcode angle. */
const PORTRAIT_ATLAS_SIZE = 512; // 8x8 64px tiles; 8 portraits + 4 guns used
const PORTRAIT_BAKE_TIME = 0.35; // frozen clock for the bake: a neutral idle frame
const PORTRAIT_ROCK_AMP = 5 * (Math.PI / 180); // rocking amplitude (~5 deg)
const PORTRAIT_ROCK_W = 1.5; // rocking angular speed (rad/s of `time`)
const PORTRAIT_YAW = 0.6; // 3/4 base yaw (rad)
const PORTRAIT_PITCH = 0.55; // slightly-elevated 3/4 camera (bust, mode 0)
// HEADSHOT (mode 1): the camera pushed in and raised to head height so the
// face fills most of the tile — near-eye-level.
const HEADSHOT_YAW = 0.22; // near-frontal base yaw: the visor stays toward the viewer
const HEADSHOT_PITCH = 0.12; // eye level, barely above: the face, not the head's top
const HEADSHOT_HALFV = 0.52; // ortho half-extent: head + a hint of shoulders
const HEADSHOT_CENTER = [0, 1.86, 0]; // orbit focus at head height (head y=1.95)
const PORTRAIT_HALFV = 1.55; // bust ortho half-extent (whole robot)
const PORTRAIT_CENTER = [0, 0.95, 0]; // bust orbit focus (robot-core default)

/* ---- glyph atlas config ------------------------------------------------- */
const GLYPH_FS = 48; // rasterization font size; quads scale from this
const GLYPH_PAD = 2; // padding inside each glyph cell
const GLYPH_ATLAS_SIZE = 1024;

/* ---- pixel-art group scratch target ---- */
const PIX_MAX = 1024; // texels per side of a scratch texture (= the group cap)
const PIX_DEPTH = 4; // max nesting depth of pixel-art groups (one scratch target each)

export function initRenderer(canvas) {
  // THE CANVAS PRESENTATION PATH has a fixed per-frame GPU cost of its own —
  // what the browser + OS spend getting this buffer to the glass, bare clear
  // included — and it depends on how the context is created. Measured with
  // `?gpuprobe` ("FIXED cost" line) on a 2018 MacBook Air (Chrome, ANGLE
  // Metal, UHD 617, 2880x1046): alpha:false = 13.3 ms, alpha:true = 9.7 ms —
  // 3.6 ms, two full-screen layers' worth, the difference between 52 and a
  // locked 60 fps. (Apple's IOSurface-backed drawing buffers always carry an
  // alpha channel; an alpha-less WebGL buffer has to be emulated on top.)
  // So: on Apple platforms the context HAS alpha and the blend keeps it at 1
  // (`pixBlend`; every full pass writes alpha 1), i.e. the canvas is still
  // opaque over the page. Elsewhere alpha:false stays — unmeasured there, and
  // an opaque buffer is what lets a compositor skip blending the canvas.
  // `?ctx=` flips the choices for A/B probing: `alpha` / `opaque` force the
  // buffer kind, `sync` = no desynchronized hint, `lowpower` = no
  // high-performance GPU request.
  const ctxFlags = (typeof location !== "undefined"
    && new URLSearchParams(location.search).get("ctx") || "").split(",");
  const applePlatform = typeof navigator !== "undefined"
    && /Mac|iPhone|iPad|iPod/.test(navigator.platform || navigator.userAgent || "");
  const CTX_ALPHA = ctxFlags.includes("alpha") || (applePlatform && !ctxFlags.includes("opaque"));
  const gl = canvas.getContext("webgl", {
    // Opaque canvas where that is the cheap kind (see CTX_ALPHA above): the
    // game paints every pixel every frame, so the compositor can scan it
    // out directly instead of alpha-blending the buffer over the page.
    alpha: CTX_ALPHA,
    // NO MSAA, ON PURPOSE — the ALIASING is part of the art direction
    // (CLAUDE.md ## Design): tilted geometry stair-stepping under the
    // camera sway is the Hotline-Miami-2 look. (An `?aa=1` MSAA experiment
    // existed briefly: it also cost 4x bandwidth per full-screen layer and
    // dropped the 2018 MacBook to 30 fps. Do not re-add antialiasing.)
    antialias: false,
    // No depth/stencil on the default framebuffer either: the 2D batch
    // paints in submission order and never depth-tests (DEPTH_TEST stays
    // disabled), so the default `depth: true` would allocate a full-screen
    // physical-resolution buffer that is never read or written. The robot
    // pipeline's small render target keeps its own depth attachment — that
    // one is real 3D and needs it.
    depth: false,
    stencil: false,
    premultipliedAlpha: true,
    preserveDrawingBuffer: false,
    // Dual-GPU laptops: without this Chrome may hand WebGL the INTEGRATED
    // GPU and the game crawls at 30 fps on machines that could do 120.
    powerPreference: ctxFlags.includes("lowpower") ? "default" : "high-performance",
    // Low-latency canvas: where supported (Chrome + a compositor overlay
    // path) the swap bypasses the compositor queue, saving up to one vsync
    // of input->photon latency. Ignored by other browsers; if a platform
    // ever shows tearing or a black canvas, delete this line.
    desynchronized: !ctxFlags.includes("sync"),
  });
  if (!gl) {
    throw new Error("WebGL is not available; the game cannot render.");
  }

  /* ---- perf tracing (?perf; collector = window.__perf in index.html) ----
     PERF is null on a normal run: every check below is a single falsy test
     and the gl.drawArrays shim is only installed when tracing is on, so
     disabled runs keep the raw function and pay nothing. */
  const PERF = (typeof window !== "undefined" && window.__perf && window.__perf.enabled)
    ? window.__perf : null;
  if (PERF) {
    // Which GPU actually runs the game (dual-GPU laptops!) — lands in the
    // dump's meta, next to the canvas size.
    const dbgInfo = gl.getExtension("WEBGL_debug_renderer_info");
    PERF.gpu = String(gl.getParameter(dbgInfo ? dbgInfo.UNMASKED_RENDERER_WEBGL : gl.RENDERER));
    // Count every draw call on this context — the batch pipeline, the
    // robot/shoggoth sprite pipelines and the post passes all share `gl`.
    const rawDrawArrays = gl.drawArrays.bind(gl);
    gl.drawArrays = function (mode, first, count) {
      PERF._draws++;
      return rawDrawArrays(mode, first, count);
    };
    // (the robots' GPU rig draws its whole batch through the instancing
    // extension — getExtension hands every caller the same object)
    const instExt = gl.getExtension("ANGLE_instanced_arrays");
    if (instExt) {
      const rawInstanced = instExt.drawArraysInstancedANGLE.bind(instExt);
      instExt.drawArraysInstancedANGLE = function (mode, first, count, n) {
        PERF._draws++;
        return rawInstanced(mode, first, count, n);
      };
    }
    // ... and every render-target switch (the `fbos` counter: sprite passes,
    // pixel groups, post passes).
    PERF._fbos = 0;
    const rawBindFramebuffer = gl.bindFramebuffer.bind(gl);
    gl.bindFramebuffer = function (target, fbo) {
      PERF._fbos++;
      return rawBindFramebuffer(target, fbo);
    };
  }
  // Per-frame accumulators (renderQueuedSprites runs a variable number of
  // times per frame, once per flush that has queued sprites).
  let perfSpriteMs = 0;   // total ms spent in sprite passes this frame
  let perfSpriteT0 = 0;   // first sprite pass start (span anchor)
  let perfRobotN = 0;     // robots + bosses rendered live this frame

  /* ---- program ---- */
  function compile(type, src) {
    const s = gl.createShader(type);
    gl.shaderSource(s, src);
    gl.compileShader(s);
    if (!gl.getShaderParameter(s, gl.COMPILE_STATUS)) {
      throw new Error("Shader compile failed: " + gl.getShaderInfoLog(s));
    }
    return s;
  }
  const prog = gl.createProgram();
  gl.attachShader(prog, compile(gl.VERTEX_SHADER, VS));
  // (`?grain=fold`: the TV static folded into this shader — see FS)
  const GRAIN_FOLD = typeof location !== "undefined"
    && new URLSearchParams(location.search).get("grain") === "fold";
  gl.attachShader(prog, compile(gl.FRAGMENT_SHADER, (GRAIN_FOLD ? "#define GRAIN\n" : "") + FS));
  gl.linkProgram(prog);
  if (!gl.getProgramParameter(prog, gl.LINK_STATUS)) {
    throw new Error("Program link failed: " + gl.getProgramInfoLog(prog));
  }
  gl.useProgram(prog);
  const loc = {
    aPos: gl.getAttribLocation(prog, "aPos"),
    aUv: gl.getAttribLocation(prog, "aUv"),
    aColor: gl.getAttribLocation(prog, "aColor"),
    uRes: gl.getUniformLocation(prog, "uRes"),
    uTex: gl.getUniformLocation(prog, "uTex"),
    uXA: gl.getUniformLocation(prog, "uXA"),
    uXB: gl.getUniformLocation(prog, "uXB"),
    uGrain: gl.getUniformLocation(prog, "uGrain"),
    uGrainT: gl.getUniformLocation(prog, "uGrainT"),
    uGrainPre: gl.getUniformLocation(prog, "uGrainPre"),
    uGrainK: gl.getUniformLocation(prog, "uGrainK"),
  };
  if (GRAIN_FOLD) {
    gl.uniform1i(loc.uGrain, 2); // texture unit 2: the static sheet, bound per folded frame
    gl.uniform1f(loc.uGrainT, 0);
    gl.uniform1f(loc.uGrainPre, 0);
  }
  // Identity: dynamic draws are already CPU-transformed. Only the static
  // geometry cache draw (drawStatic) ever changes these, and it resets them.
  gl.uniform3f(loc.uXA, 1, 0, 0);
  gl.uniform3f(loc.uXB, 0, 1, 0);

  gl.enable(gl.BLEND);
  gl.blendFunc(gl.SRC_ALPHA, gl.ONE_MINUS_SRC_ALPHA);
  gl.disable(gl.DEPTH_TEST);
  gl.pixelStorei(gl.UNPACK_PREMULTIPLY_ALPHA_WEBGL, false);
  gl.pixelStorei(gl.UNPACK_FLIP_Y_WEBGL, false);

  /* ---- interleaved dynamic vertex buffer: x y u v r g b a ----
     Every flush hands the GPU only the vertices it filled, as a fresh
     `bufferData` (buffer ORPHANING), never a `bufferSubData` at offset 0
     into one big preallocated store: a sub-data write into a buffer the
     previous draw may still be reading forces the driver to either wait
     for that draw or copy the store (the whole 2 MiB, on drivers that keep
     a shadow copy) — per flush, ~20 flushes a frame. On a 2018 MacBook Air
     (Chrome, ANGLE Metal, UHD 617) `?gpuprobe` showed the frame cost
     tracking the NUMBER OF FLUSHES rather than the pixels, with "clear
     only" at vsync: a per-flush stall. Orphaning lets the driver hand out
     a right-sized pooled buffer and the draws overlap. `?vbo=sub` keeps
     the old sub-data path for the A/B. */
  const FLOATS_PER_VERT = 8;
  const MAX_VERTS = 65536;
  const verts = new Float32Array(MAX_VERTS * FLOATS_PER_VERT);
  let vCount = 0;
  const vbo = gl.createBuffer();
  gl.bindBuffer(gl.ARRAY_BUFFER, vbo);
  const vboSubData = typeof location !== "undefined"
    && new URLSearchParams(location.search).get("vbo") === "sub";
  gl.bufferData(gl.ARRAY_BUFFER, vboSubData ? verts.byteLength : 6 * FLOATS_PER_VERT * 4, gl.DYNAMIC_DRAW);
  const STRIDE = FLOATS_PER_VERT * 4;
  gl.enableVertexAttribArray(loc.aPos);
  gl.vertexAttribPointer(loc.aPos, 2, gl.FLOAT, false, STRIDE, 0);
  gl.enableVertexAttribArray(loc.aUv);
  gl.vertexAttribPointer(loc.aUv, 2, gl.FLOAT, false, STRIDE, 8);
  gl.enableVertexAttribArray(loc.aColor);
  gl.vertexAttribPointer(loc.aColor, 4, gl.FLOAT, false, STRIDE, 16);

  /* ---- textures ---- */
  function makeTexture(size, nearest) {
    const t = gl.createTexture();
    gl.bindTexture(gl.TEXTURE_2D, t);
    const filter = nearest ? gl.NEAREST : gl.LINEAR;
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MIN_FILTER, filter);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MAG_FILTER, filter);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_S, gl.CLAMP_TO_EDGE);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_T, gl.CLAMP_TO_EDGE);
    if (size) {
      gl.texImage2D(gl.TEXTURE_2D, 0, gl.RGBA, size, size, 0, gl.RGBA, gl.UNSIGNED_BYTE, null);
    }
    return t;
  }

  // 1x1 white: solid geometry samples this so one program draws everything.
  const whiteTex = makeTexture();
  gl.bindTexture(gl.TEXTURE_2D, whiteTex);
  gl.texImage2D(
    gl.TEXTURE_2D, 0, gl.RGBA, 1, 1, 0, gl.RGBA, gl.UNSIGNED_BYTE,
    new Uint8Array([255, 255, 255, 255])
  );

  const glyphTex = makeTexture(GLYPH_ATLAS_SIZE);

  // ---- TV static (POSTFX kind 13): a pre-rolled noise sheet ----
  // One texel = one 6-physical-px static cell: rgb = a hard black/white
  // roll, alpha = that cell's own strength (0.4..1, so the film sparkles).
  // Drawn as ONE alpha-blended full-screen quad (NEAREST, REPEAT, a random
  // whole-texel UV offset per frame = fresh static) — the same look the
  // MODAL STATIC's noise cells have, WITHOUT routing the frame through the
  // scene FBO + a post pass, which costs two full-screen memory touches a
  // bandwidth-starved GPU can feel.
  const STATIC_SIZE = 512;
  const staticTex = makeTexture(undefined, true);
  {
    gl.bindTexture(gl.TEXTURE_2D, staticTex);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_S, gl.REPEAT);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_T, gl.REPEAT);
    const noise = new Uint8Array(STATIC_SIZE * STATIC_SIZE * 4);
    for (let i = 0; i < STATIC_SIZE * STATIC_SIZE; i++) {
      const bw = Math.random() < 0.5 ? 0 : 255;
      noise[i * 4] = bw;
      noise[i * 4 + 1] = bw;
      noise[i * 4 + 2] = bw;
      noise[i * 4 + 3] = Math.round((0.4 + 0.6 * Math.random()) * 255);
    }
    gl.texImage2D(
      gl.TEXTURE_2D, 0, gl.RGBA, STATIC_SIZE, STATIC_SIZE, 0, gl.RGBA,
      gl.UNSIGNED_BYTE, noise
    );
    gl.bindTexture(gl.TEXTURE_2D, null);
  }

  /* ---- robot scratch atlas: the render target the batched post pass fills ---- */
  // Tiles are handed out per frame in stream order and recycled after every
  // flush (once the quads that sample them have been drawn), so the atlas
  // only ever needs to hold the robots of one batch. NEAREST, per the art
  // direction: no sampling-side smoothing anywhere (the rotated quad keeps
  // hard block edges).
  const robotTex = makeTexture(ROBOT_ATLAS_SIZE, true);
  const robotCols = ROBOT_COLS;
  const robotSlots = robotCols * robotCols;
  const robotFbo = gl.createFramebuffer();
  gl.bindFramebuffer(gl.FRAMEBUFFER, robotFbo);
  gl.framebufferTexture2D(gl.FRAMEBUFFER, gl.COLOR_ATTACHMENT0, gl.TEXTURE_2D, robotTex, 0);
  if (gl.checkFramebufferStatus(gl.FRAMEBUFFER) !== gl.FRAMEBUFFER_COMPLETE) {
    throw new Error("Robot atlas framebuffer is incomplete; the game cannot render.");
  }
  gl.bindFramebuffer(gl.FRAMEBUFFER, null);
  // `?rig=cpu` = the A/B switch back to the CPU rig (JS pose matrices, one
  // draw per robot); default = robot-core's GPU rig (the skeleton in the
  // vertex shader, the whole batch in ONE instanced draw).
  const robotGpuRig = !(typeof location !== "undefined"
    && new URLSearchParams(location.search).get("rig") === "cpu");
  const robotPipe = createRobotPipeline(gl, { rt: ROBOT_TILE, gpuRig: robotGpuRig });
  // Robots queued for the current batch, ROBOT_Q floats per slot: colorIdx,
  // weaponIdx, flags, then the 11 pose scalars exactly as the engine sent
  // them (src/render/pose.rs computes the pose; nothing here animates) —
  // rendered into their tiles by flush() right before the draw.
  const ROBOT_Q = 3 + POSE_SCALARS.length;
  const robotQueue = new Float32Array(robotSlots * ROBOT_Q);
  let robotUsed = 0;
  // Reused per render so the per-frame robot path never allocates.
  const robotPlan = {
    bob: 0, lean: 0, zback: 0, recoil: 0, legA: 0, legB: 0, armLp: 0, armRp: 0,
    armRaise: 0, armOut: 0, elbow: 0, shoot: false, headless: false,
  };
  const robotOpts = { color: "coral", weapon: "fist", facingDeg: 0, plan: robotPlan };
  // The batch lays its tiles out from the atlas origin (robotCols per row).
  const robotTarget = { fbo: robotFbo, x: 0, y: 0 };

  /* ---- shoggoth scratch atlas: same scheme, bigger tiles, its own pipeline ---- */
  const shogTex = makeTexture(SHOG_ATLAS_SIZE);
  const shogCols = Math.floor(SHOG_ATLAS_SIZE / SHOG_TILE);
  const shogSlots = shogCols * shogCols;
  const shogFbo = gl.createFramebuffer();
  gl.bindFramebuffer(gl.FRAMEBUFFER, shogFbo);
  gl.framebufferTexture2D(gl.FRAMEBUFFER, gl.COLOR_ATTACHMENT0, gl.TEXTURE_2D, shogTex, 0);
  if (gl.checkFramebufferStatus(gl.FRAMEBUFFER) !== gl.FRAMEBUFFER_COMPLETE) {
    throw new Error("Shoggoth atlas framebuffer is incomplete; the game cannot render.");
  }
  gl.bindFramebuffer(gl.FRAMEBUFFER, null);
  const shogPipe = createShoggothPipeline(gl, { rt: SHOG_TILE });
  // Bosses queued for the current batch: (heading, reveal, time) per slot.
  const shogQueue = new Float32Array(shogSlots * 3);
  let shogUsed = 0;
  const shogOpts = {
    reveal: 0, time: 0, heading: 0, wander: false, px: SHOG_PX, transparent: true,
  };
  const shogTarget = { fbo: shogFbo, x: 0, y: 0, w: SHOG_TILE, h: SHOG_TILE };

  /* ---- pixel-sprite cache: baked-once sprites, rotated in 2D ----
     PERSISTENT — never recycled per frame, unlike every scratch atlas above.
     Each (colorIdx, mode) portrait is rendered through robot-core exactly
     once (lazily, on first use), with a FIXED camera (the base yaw, no sway)
     and a frozen clock (a neutral idle frame); every subsequent frame just
     draws the cached tile as a rotated quad. Ground guns (GUNPICKUP) share
     the atlas — one bake per weaponIdx at angle 0, negative Map keys — and
     spin as rotated quads the same way. ~zero per-frame cost. */
  const portraitTex = makeTexture(PORTRAIT_ATLAS_SIZE, true);
  const portraitCols = Math.floor(PORTRAIT_ATLAS_SIZE / FX_TILE);
  const portraitFbo = gl.createFramebuffer();
  gl.bindFramebuffer(gl.FRAMEBUFFER, portraitFbo);
  gl.framebufferTexture2D(gl.FRAMEBUFFER, gl.COLOR_ATTACHMENT0, gl.TEXTURE_2D, portraitTex, 0);
  if (gl.checkFramebufferStatus(gl.FRAMEBUFFER) !== gl.FRAMEBUFFER_COMPLETE) {
    throw new Error("Portrait cache framebuffer is incomplete; the game cannot render.");
  }
  gl.bindFramebuffer(gl.FRAMEBUFFER, null);
  // colorIdx * 2 + mode -> baked slot; ground guns use key -1 - weaponIdx.
  const portraitCache = new Map();
  const portraitOpts = {
    pose: "idle", color: "coral", weapon: "fist", time: 0, facingDeg: 0,
    // rt/FX_TILE post blocks: one output texel per block = 64-texel art
    px: ROBOT_TILE / FX_TILE, transparent: true,
    orbit: {
      yaw: PORTRAIT_YAW, pitch: PORTRAIT_PITCH, halfV: PORTRAIT_HALFV,
      center: PORTRAIT_CENTER,
    },
  };
  const portraitTarget = { fbo: portraitFbo, x: 0, y: 0, w: FX_TILE, h: FX_TILE };
  // Slot of the (colorIdx, mode) portrait, baking it on first use. The bake
  // is a mid-stream 3D render: flush what is pending, keep the pipelines'
  // attrib state disjoint (see renderQueuedSprites), rebind the batch after.
  function portraitSlotFor(colorIdx, mode) {
    const key = colorIdx * 2 + mode;
    let slot = portraitCache.get(key);
    if (slot !== undefined) return slot;
    slot = portraitCache.size;
    flush();
    gl.disableVertexAttribArray(loc.aPos);
    gl.disableVertexAttribArray(loc.aUv);
    gl.disableVertexAttribArray(loc.aColor);
    const headshot = mode > 0;
    portraitOpts.color = ROBOT_COLORS[colorIdx] || ROBOT_COLORS[0];
    portraitOpts.time = PORTRAIT_BAKE_TIME;
    portraitOpts.orbit.yaw = headshot ? HEADSHOT_YAW : PORTRAIT_YAW;
    portraitOpts.orbit.pitch = headshot ? HEADSHOT_PITCH : PORTRAIT_PITCH;
    portraitOpts.orbit.halfV = headshot ? HEADSHOT_HALFV : PORTRAIT_HALFV;
    portraitOpts.orbit.center = headshot ? HEADSHOT_CENTER : PORTRAIT_CENTER;
    portraitTarget.x = (slot % portraitCols) * FX_TILE;
    portraitTarget.y = Math.floor(slot / portraitCols) * FX_TILE;
    robotPipe.render(portraitOpts, portraitTarget);
    gl.bindTexture(gl.TEXTURE_2D, null);
    bindBatchState();
    portraitCache.set(key, slot);
    return slot;
  }
  const gunBakeOpts = {
    weaponIdx: 0, angle: 0, px: ROBOT_TILE / GUN_ART, transparent: true,
  };
  const gunBakeTarget = { fbo: portraitFbo, x: 0, y: 0, w: GUN_ART, h: GUN_ART };
  // Slot of the ground-gun sprite for `weaponIdx`, baked on first use: one
  // renderGun at ANGLE 0 into a GUN_ART-texel corner of a cache tile. The
  // camera is a true top-down ortho (robot-core topDownVP) and the model
  // lies flat, so every visible face's normal points straight up: spinning
  // the model about the vertical axis and rotating the baked sprite in 2D
  // are equivalent (shading is yaw-invariant). Same mid-stream bake
  // discipline as portraitSlotFor above.
  function gunSlotFor(weaponIdx) {
    const key = -1 - weaponIdx; // negative keys: guns; >= 0: portraits
    let slot = portraitCache.get(key);
    if (slot !== undefined) return slot;
    slot = portraitCache.size;
    flush();
    gl.disableVertexAttribArray(loc.aPos);
    gl.disableVertexAttribArray(loc.aUv);
    gl.disableVertexAttribArray(loc.aColor);
    gunBakeOpts.weaponIdx = weaponIdx;
    gunBakeTarget.x = (slot % portraitCols) * FX_TILE;
    gunBakeTarget.y = Math.floor(slot / portraitCols) * FX_TILE;
    robotPipe.renderGun(gunBakeOpts, gunBakeTarget);
    gl.bindTexture(gl.TEXTURE_2D, null);
    bindBatchState();
    portraitCache.set(key, slot);
    return slot;
  }
  const headBakeOpts = {
    color: "coral", px: ROBOT_TILE / HEAD_ART, transparent: true,
  };
  const headBakeTarget = { fbo: portraitFbo, x: 0, y: 0, w: HEAD_ART, h: HEAD_ART };
  // Slot of the detached-head sprite for `colorIdx`, baked on first use: one
  // renderHead (the head + visor cubes, face-up, true top-down) at angle 0
  // into a HEAD_ART-texel corner of a cache tile. Spinning the baked sprite
  // in 2D is equivalent to spinning the model (see gunSlotFor). Keys
  // -10 - colorIdx keep clear of the guns' -1..-4.
  function headSlotFor(colorIdx) {
    const key = -10 - colorIdx;
    let slot = portraitCache.get(key);
    if (slot !== undefined) return slot;
    slot = portraitCache.size;
    flush();
    gl.disableVertexAttribArray(loc.aPos);
    gl.disableVertexAttribArray(loc.aUv);
    gl.disableVertexAttribArray(loc.aColor);
    headBakeOpts.color = ROBOT_COLORS[colorIdx] || ROBOT_COLORS[0];
    headBakeTarget.x = (slot % portraitCols) * FX_TILE;
    headBakeTarget.y = Math.floor(slot / portraitCols) * FX_TILE;
    robotPipe.renderHead(headBakeOpts, headBakeTarget);
    gl.bindTexture(gl.TEXTURE_2D, null);
    bindBatchState();
    portraitCache.set(key, slot);
    return slot;
  }

  /* ---- POSTFX: offscreen scene target + the full-screen post program ---- */
  const postProg = gl.createProgram();
  gl.attachShader(postProg, compile(gl.VERTEX_SHADER, POST_VS));
  gl.attachShader(postProg, compile(gl.FRAGMENT_SHADER, POST_FS));
  gl.linkProgram(postProg);
  if (!gl.getProgramParameter(postProg, gl.LINK_STATUS)) {
    throw new Error("Post program link failed: " + gl.getProgramInfoLog(postProg));
  }
  const postLoc = {
    aPos: gl.getAttribLocation(postProg, "aPos"),
    uScene: gl.getUniformLocation(postProg, "uScene"),
    uRes: gl.getUniformLocation(postProg, "uRes"),
    uKind: gl.getUniformLocation(postProg, "uKind"),
    uT: gl.getUniformLocation(postProg, "uT"),
    uColor: gl.getUniformLocation(postProg, "uColor"),
    uTime: gl.getUniformLocation(postProg, "uTime"),
  };
  const postVbo = gl.createBuffer();
  gl.bindBuffer(gl.ARRAY_BUFFER, postVbo);
  gl.bufferData(gl.ARRAY_BUFFER, new Float32Array([-1, -1, 1, -1, 1, 1, -1, -1, 1, 1, -1, 1]), gl.STATIC_DRAW);
  const sceneTex = makeTexture();
  const sceneFbo = gl.createFramebuffer();
  let sceneW = 0, sceneH = 0;
  // (Re)allocate the scene target to the canvas size (lazily, on first use /
  // resize) — the FBO is only touched on frames that carry a POSTFX.
  function ensureSceneTarget(w, h) {
    if (sceneW === w && sceneH === h) return;
    gl.bindTexture(gl.TEXTURE_2D, sceneTex);
    gl.texImage2D(gl.TEXTURE_2D, 0, gl.RGBA, w, h, 0, gl.RGBA, gl.UNSIGNED_BYTE, null);
    gl.bindTexture(gl.TEXTURE_2D, null);
    gl.bindFramebuffer(gl.FRAMEBUFFER, sceneFbo);
    gl.framebufferTexture2D(gl.FRAMEBUFFER, gl.COLOR_ATTACHMENT0, gl.TEXTURE_2D, sceneTex, 0);
    if (gl.checkFramebufferStatus(gl.FRAMEBUFFER) !== gl.FRAMEBUFFER_COMPLETE) {
      throw new Error("Scene framebuffer is incomplete; the post pass cannot render.");
    }
    gl.bindFramebuffer(gl.FRAMEBUFFER, null);
    sceneW = w;
    sceneH = h;
  }
  // The framebuffer the batch draws into: null (the canvas) normally, the
  // scene FBO on frames that end in a post pass, the pixel-group scratch
  // target inside a PIX_BEGIN/PIX_END group — plus the target's size:
  // batchW/H is the coordinate space (the vertex shader's uRes), batchVW/VH
  // the pixel size of the target (the viewport). They differ only on the
  // canvas / scene targets, where the wasm records in CSS pixels but the
  // backing buffer is sized to physical device pixels (CSS x data-dpr, see
  // Graphics::sync_size) — the viewport mapping does the upscale, so every
  // primitive lands on real screen pixels with no browser rescale. Group
  // scratch targets are 1:1 (texels).
  let batchFbo = null;
  let batchW = 1, batchH = 1;
  let batchVW = 1, batchVH = 1;
  // This frame's canvas sizes: logical (CSS px — what the command stream is
  // recorded in) and physical (the backing buffer).
  let frameW = 1, frameH = 1;
  let framePW = 1, framePH = 1;
  // The POSTFX request of the current frame (kind, t, r, g, b) or null.
  const postfx = { kind: 0, t: 0, r: 0, g: 0, b: 0 };
  let postfxActive = false;

  // Walk the stream by the opcode table (no execution) and pick up the LAST
  // POSTFX, if any — it must be known before the first draw so the whole
  // frame lands in the scene target.
  let scanSawBackdrop = false;
  function scanPostfx(cmds) {
    scanSawBackdrop = false;
    let i = 0;
    const n = cmds.length;
    let found = false;
    while (i < n) {
      const op = cmds[i++];
      const args = OP_ARGS[op];
      if (args === undefined) break; // corrupt stream: frameRender reports it
      if (op === 24) scanSawBackdrop = true; // BACKDROP: the frame has an opaque base layer
      if (op === OP_POSTFX) {
        postfx.kind = cmds[i] | 0;
        postfx.t = cmds[i + 1];
        postfx.r = cmds[i + 2];
        postfx.g = cmds[i + 3];
        postfx.b = cmds[i + 4];
        found = true;
      }
      i += args;
    }
    return found;
  }

  // Draw the scene target to the canvas through the post shader, then hand
  // the GL state back to the batch pipeline.
  function runPostPass(w, h) {
    gl.bindFramebuffer(gl.FRAMEBUFFER, null);
    gl.viewport(0, 0, w, h);
    gl.disable(gl.BLEND);
    gl.useProgram(postProg);
    gl.disableVertexAttribArray(loc.aUv);
    gl.disableVertexAttribArray(loc.aColor);
    gl.bindBuffer(gl.ARRAY_BUFFER, postVbo);
    gl.enableVertexAttribArray(postLoc.aPos);
    gl.vertexAttribPointer(postLoc.aPos, 2, gl.FLOAT, false, 0, 0);
    gl.activeTexture(gl.TEXTURE0);
    gl.bindTexture(gl.TEXTURE_2D, sceneTex);
    gl.uniform1i(postLoc.uScene, 0);
    gl.uniform2f(postLoc.uRes, w, h);
    gl.uniform1f(postLoc.uKind, postfx.kind);
    gl.uniform1f(postLoc.uT, postfx.t);
    gl.uniform3f(postLoc.uColor, postfx.r, postfx.g, postfx.b);
    gl.uniform1f(postLoc.uTime, (performance.now() % 100000) / 1000);
    gl.drawArrays(gl.TRIANGLES, 0, 6);
    gl.bindTexture(gl.TEXTURE_2D, null);
    if (postLoc.aPos !== loc.aPos) gl.disableVertexAttribArray(postLoc.aPos);
    batchFbo = null;
    batchW = frameW;
    batchH = frameH;
    batchVW = framePW;
    batchVH = framePH;
    bindBatchState();
    gl.uniform1i(loc.uTex, 0);
  }

  /* ---- POSTFX kind 10 (WARP TRAILS): ping-pong feedback accumulator ---- */
  const warpProg = gl.createProgram();
  gl.attachShader(warpProg, compile(gl.VERTEX_SHADER, POST_VS));
  gl.attachShader(warpProg, compile(gl.FRAGMENT_SHADER, WARP_FS));
  gl.linkProgram(warpProg);
  if (!gl.getProgramParameter(warpProg, gl.LINK_STATUS)) {
    throw new Error("Warp program link failed: " + gl.getProgramInfoLog(warpProg));
  }
  const warpLoc = {
    aPos: gl.getAttribLocation(warpProg, "aPos"),
    uScene: gl.getUniformLocation(warpProg, "uScene"),
    uPrev: gl.getUniformLocation(warpProg, "uPrev"),
    uRes: gl.getUniformLocation(warpProg, "uRes"),
    uT: gl.getUniformLocation(warpProg, "uT"),
    uColor: gl.getUniformLocation(warpProg, "uColor"),
    uMode: gl.getUniformLocation(warpProg, "uMode"),
  };
  // Two canvas-sized LINEAR accumulators (the sub-texel pull needs bilinear
  // sampling), created lazily on the first warp frame, reallocated on resize.
  const warpTex = [null, null];
  const warpFbo = [null, null];
  let warpW = 0, warpH = 0;
  let warpRead = 0; // index of the accumulator holding last frame's trails
  let warpLive = false; // did the PREVIOUS frame run the warp pass?
  function ensureWarpTargets(w, h) {
    if (warpW === w && warpH === h) return;
    for (let i = 0; i < 2; i++) {
      if (!warpTex[i]) {
        warpTex[i] = makeTexture();
        warpFbo[i] = gl.createFramebuffer();
      }
      gl.bindTexture(gl.TEXTURE_2D, warpTex[i]);
      gl.texImage2D(gl.TEXTURE_2D, 0, gl.RGBA, w, h, 0, gl.RGBA, gl.UNSIGNED_BYTE, null);
      gl.bindFramebuffer(gl.FRAMEBUFFER, warpFbo[i]);
      gl.framebufferTexture2D(gl.FRAMEBUFFER, gl.COLOR_ATTACHMENT0, gl.TEXTURE_2D, warpTex[i], 0);
      if (gl.checkFramebufferStatus(gl.FRAMEBUFFER) !== gl.FRAMEBUFFER_COMPLETE) {
        throw new Error("Warp framebuffer is incomplete; the trails cannot render.");
      }
    }
    gl.bindTexture(gl.TEXTURE_2D, null);
    gl.bindFramebuffer(gl.FRAMEBUFFER, null);
    warpW = w;
    warpH = h;
    warpLive = false; // fresh (or resized) buffers hold garbage: clear first
  }
  function clearWarpAccum() {
    for (let i = 0; i < 2; i++) {
      gl.bindFramebuffer(gl.FRAMEBUFFER, warpFbo[i]);
      gl.viewport(0, 0, warpW, warpH);
      gl.clearColor(0, 0, 0, 1);
      gl.clear(gl.COLOR_BUFFER_BIT);
    }
    gl.bindFramebuffer(gl.FRAMEBUFFER, null);
  }
  // The kind-10 replacement for runPostPass: pass A folds last frame's
  // trails (pulled toward the centre = streaming outward) + the scene's
  // bright pixels into the write accumulator, pass B presents scene+trails
  // to the canvas, then read/write swap. State handed back to the batch
  // pipeline exactly like runPostPass.
  function runWarpPass(w, h) {
    ensureWarpTargets(w, h);
    if (!warpLive) clearWarpAccum(); // the effect was off last frame: start clean
    const write = 1 - warpRead;
    gl.disable(gl.BLEND);
    gl.useProgram(warpProg);
    gl.disableVertexAttribArray(loc.aUv);
    gl.disableVertexAttribArray(loc.aColor);
    gl.bindBuffer(gl.ARRAY_BUFFER, postVbo);
    gl.enableVertexAttribArray(warpLoc.aPos);
    gl.vertexAttribPointer(warpLoc.aPos, 2, gl.FLOAT, false, 0, 0);
    gl.uniform1i(warpLoc.uScene, 0);
    gl.uniform1i(warpLoc.uPrev, 1);
    gl.uniform2f(warpLoc.uRes, w, h);
    gl.uniform1f(warpLoc.uT, postfx.t);
    gl.uniform3f(warpLoc.uColor, postfx.r, postfx.g, postfx.b);
    gl.activeTexture(gl.TEXTURE1);
    gl.bindTexture(gl.TEXTURE_2D, warpTex[warpRead]);
    gl.activeTexture(gl.TEXTURE0);
    gl.bindTexture(gl.TEXTURE_2D, sceneTex);
    // Pass A: combine into the write accumulator.
    gl.bindFramebuffer(gl.FRAMEBUFFER, warpFbo[write]);
    gl.viewport(0, 0, w, h);
    gl.uniform1f(warpLoc.uMode, 0);
    gl.drawArrays(gl.TRIANGLES, 0, 6);
    // Pass B: present the scene + the fresh trails on the canvas.
    gl.activeTexture(gl.TEXTURE1);
    gl.bindTexture(gl.TEXTURE_2D, warpTex[write]);
    gl.bindFramebuffer(gl.FRAMEBUFFER, null);
    gl.viewport(0, 0, w, h);
    gl.uniform1f(warpLoc.uMode, 1);
    gl.drawArrays(gl.TRIANGLES, 0, 6);
    gl.bindTexture(gl.TEXTURE_2D, null);
    gl.activeTexture(gl.TEXTURE0);
    gl.bindTexture(gl.TEXTURE_2D, null);
    if (warpLoc.aPos !== loc.aPos) gl.disableVertexAttribArray(warpLoc.aPos);
    warpRead = write;
    warpLive = true;
    batchFbo = null;
    batchW = frameW;
    batchH = frameH;
    batchVW = framePW;
    batchVH = framePH;
    bindBatchState();
    gl.uniform1i(loc.uTex, 0);
  }

  /* ---- DRIVE (opcode 20): the synthwave backdrop as ONE shader pass ----
     Every pixel of the scene — banded dusk sky, cut-band sun, stars, digital
     rain, the road rushing at the camera, palm silhouettes, tear bands,
     red/cyan channel split, neon debris — is COMPUTED in the fragment
     shader, shadertoy-style, AT ART RESOLUTION: the shader runs once per
     art pixel into a tiny NEAREST target (the quantization for free), and
     the finished image lands as one upscaled textured quad. No pixel-group
     re-records, no stacked blended layers, and the scene math costs ~84K
     fragment evaluations however large the canvas or DPR. The wasm
     side (src/drive.rs) stays the source of truth for the deterministic
     glitch schedules and ships them as op args; palm slots and debris
     blocks are placed here per frame (same integer hash as Rust's
     `hash01`) and handed over as uniforms so the per-pixel loop stays
     cheap. The scene geometry constants mirror src/drive.rs's tunables. */
  const driveProg = gl.createProgram();
  gl.attachShader(driveProg, compile(gl.VERTEX_SHADER, DRIVE_VS));
  gl.attachShader(driveProg, compile(gl.FRAGMENT_SHADER, DRIVE_FS));
  gl.linkProgram(driveProg);
  if (!gl.getProgramParameter(driveProg, gl.LINK_STATUS)) {
    throw new Error("Drive program link failed: " + gl.getProgramInfoLog(driveProg));
  }
  const driveLoc = {
    aPos: gl.getAttribLocation(driveProg, "aPos"),
    uSize: gl.getUniformLocation(driveProg, "uSize"),
    uTexH: gl.getUniformLocation(driveProg, "uTexH"),
    uT: gl.getUniformLocation(driveProg, "uT"),
    uGlitch: gl.getUniformLocation(driveProg, "uGlitch"),
    uSplit: gl.getUniformLocation(driveProg, "uSplit"),
    uPx: gl.getUniformLocation(driveProg, "uPx"),
    uDim: gl.getUniformLocation(driveProg, "uDim"),
    uOffs: gl.getUniformLocation(driveProg, "uOffs[0]"),
    uSunSeed: gl.getUniformLocation(driveProg, "uSunSeed"),
    uPalmA: gl.getUniformLocation(driveProg, "uPalmA[0]"),
    uPalmB: gl.getUniformLocation(driveProg, "uPalmB[0]"),
    uDebris: gl.getUniformLocation(driveProg, "uDebris[0]"),
    uDebrisC: gl.getUniformLocation(driveProg, "uDebrisC[0]"),
  };
  // The drive's ART-RESOLUTION render target (ceil(w/px) x ceil(h/px)
  // texels, NEAREST): the shader runs once per art pixel, the result is
  // upscaled by a single textured quad — so the per-pixel scene math costs
  // ~84K fragment evaluations instead of millions, whatever the canvas /
  // DPR. Reallocated when the rect or art-pixel size changes.
  let driveTex = null, driveFbo = null, driveTW = 0, driveTH = 0;
  function ensureDriveTarget(tw, th) {
    if (driveTW === tw && driveTH === th) return;
    if (!driveTex) {
      driveTex = gl.createTexture();
      driveFbo = gl.createFramebuffer();
      gl.bindTexture(gl.TEXTURE_2D, driveTex);
      gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MIN_FILTER, gl.NEAREST);
      gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MAG_FILTER, gl.NEAREST);
      gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_S, gl.CLAMP_TO_EDGE);
      gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_T, gl.CLAMP_TO_EDGE);
    } else {
      gl.bindTexture(gl.TEXTURE_2D, driveTex);
    }
    gl.texImage2D(gl.TEXTURE_2D, 0, gl.RGBA, tw, th, 0, gl.RGBA, gl.UNSIGNED_BYTE, null);
    gl.bindTexture(gl.TEXTURE_2D, null);
    gl.bindFramebuffer(gl.FRAMEBUFFER, driveFbo);
    gl.framebufferTexture2D(gl.FRAMEBUFFER, gl.COLOR_ATTACHMENT0, gl.TEXTURE_2D, driveTex, 0);
    if (gl.checkFramebufferStatus(gl.FRAMEBUFFER) !== gl.FRAMEBUFFER_COMPLETE) {
      throw new Error("Drive framebuffer is incomplete; the backdrop cannot render.");
    }
    gl.bindFramebuffer(gl.FRAMEBUFFER, null);
    driveTW = tw;
    driveTH = th;
  }
  const drivePalmA = new Float32Array(24 * 4);
  const drivePalmB = new Float32Array(24 * 4);
  const driveDebris = new Float32Array(7 * 4);
  const driveDebrisC = new Float32Array(7 * 4);
  // Exact port of src/drive.rs `hash01` (u32 wrapping arithmetic), so palm
  // stutter / debris scheduling stay bit-identical to the primitive era.
  function driveHash(a, b) {
    let x = (Math.imul(a >>> 0, 374761393) + Math.imul(b >>> 0, 668265263)) >>> 0;
    x = Math.imul(x ^ (x >>> 13), 1274126177) >>> 0;
    return ((x ^ (x >>> 16)) & 0xffffff) / 0xffffff;
  }
  function drawDrive(w, h, t, glitch, split, px, dim, offs, offsBase) {
    flush();
    // Palm slots (mirrors the old `scene` palm loop, far to near): the
    // per-slot placement runs once here; the shader only does bbox tests
    // and, inside a palm's box, the trunk / frond segment distances.
    const horizon = h * 0.44, ppu = w * 0.14;
    const SPEED = 13.0, SPACING = 6.5, PX = 4.6, PH = 3.4, ZFAR = 36.0;
    drivePalmA.fill(0);
    drivePalmB.fill(0);
    let pi = 0;
    for (let i = 11; i >= 0; i--) {
      for (const side of [-1, 1]) {
        const slot = pi++;
        const phase = side > 0 ? 0.5 : 0.0;
        const travelled = (t * SPEED) / SPACING + phase;
        const pid = ((Math.floor(travelled) + i) * 2 + (side > 0 ? 1 : 0)) >>> 0;
        // Stutter: on hashed ~130ms buckets a palm freezes on the bucket's
        // start time, then snaps forward.
        const bkt = Math.floor(t / 0.13);
        const te = driveHash(pid, (505 + bkt) >>> 0) < glitch * 0.4 ? bkt * 0.13 : t;
        const trav = (te * SPEED) / SPACING + phase;
        const off = trav - Math.floor(trav);
        const z = (i + 1 - off) * SPACING;
        if (z < 1.05 || z > ZFAR) continue;
        const s = 1 / z;
        const yb = horizon + (h - horizon) / z;
        const xb = w * 0.5 + side * PX * ppu * s * (1 + 0.12 * driveHash(pid, 61));
        const ht = PH * ppu * s * (0.8 + 0.4 * driveHash(pid, 62));
        if (ht < 3) continue;
        const fog = Math.pow(z / ZFAR, 1.3);
        const lean = -side * 0.10 + (driveHash(pid, 63) - 0.5) * 0.24;
        const sway = Math.sin(t * 1.1 + pid) * 0.05;
        const o = slot * 4;
        drivePalmA[o] = xb; drivePalmA[o + 1] = yb; drivePalmA[o + 2] = ht; drivePalmA[o + 3] = lean;
        drivePalmB[o] = fog; drivePalmB[o + 1] = pid % 1024; drivePalmB[o + 2] = sway; drivePalmB[o + 3] = 1;
      }
    }
    // Debris blocks: on hashed ~100ms buckets a handful of neon rects flash.
    driveDebris.fill(0);
    const db = Math.floor(t / 0.10);
    if (glitch > 0 && driveHash(db, 611) < glitch * 0.5) {
      const n = 2 + Math.floor(driveHash(db, 612) * 5);
      for (let i = 0; i < n; i++) {
        const kind = Math.floor(driveHash(db, 780 + i) * 3);
        const c = kind === 0 ? [0.2, 0.95, 1.0] : kind === 1 ? [1.0, 0.25, 0.85] : [0.95, 0.95, 1.0];
        const o = i * 4;
        driveDebris[o] = driveHash(db, 700 + i) * w;
        driveDebris[o + 1] = driveHash(db, 720 + i) * h;
        // At least one art pixel each way, so quantized sampling can't miss.
        driveDebris[o + 2] = Math.max(4 + driveHash(db, 740 + i) * 50, px);
        driveDebris[o + 3] = Math.max(2 + driveHash(db, 760 + i) * 8, px);
        driveDebrisC[o] = c[0]; driveDebrisC[o + 1] = c[1]; driveDebrisC[o + 2] = c[2];
        driveDebrisC[o + 3] = 0.25 + 0.35 * driveHash(db, 790 + i);
      }
    }
    // Sun-band glitch bucket (the shader hashes per slice off this seed).
    const sb = Math.floor(t / 0.12);
    const sunSeed = driveHash(sb, 399) < glitch * 0.3 ? (sb % 997) + 1 : 0;
    // PASS 1: the scene, one fragment per art pixel, into the tiny target.
    const tw = Math.ceil(w / px), th = Math.ceil(h / px);
    ensureDriveTarget(tw, th);
    gl.useProgram(driveProg);
    gl.disableVertexAttribArray(loc.aUv);
    gl.disableVertexAttribArray(loc.aColor);
    gl.bindBuffer(gl.ARRAY_BUFFER, postVbo);
    gl.enableVertexAttribArray(driveLoc.aPos);
    gl.vertexAttribPointer(driveLoc.aPos, 2, gl.FLOAT, false, 0, 0);
    gl.uniform2f(driveLoc.uSize, w, h);
    gl.uniform1f(driveLoc.uTexH, th);
    gl.uniform1f(driveLoc.uT, t);
    gl.uniform1f(driveLoc.uGlitch, glitch);
    gl.uniform1f(driveLoc.uSplit, split);
    gl.uniform1f(driveLoc.uPx, px);
    gl.uniform1f(driveLoc.uDim, dim);
    gl.uniform1fv(driveLoc.uOffs, offs.subarray(offsBase, offsBase + 9));
    gl.uniform1f(driveLoc.uSunSeed, sunSeed);
    gl.uniform4fv(driveLoc.uPalmA, drivePalmA);
    gl.uniform4fv(driveLoc.uPalmB, drivePalmB);
    gl.uniform4fv(driveLoc.uDebris, driveDebris);
    gl.uniform4fv(driveLoc.uDebrisC, driveDebrisC);
    gl.bindFramebuffer(gl.FRAMEBUFFER, driveFbo);
    gl.viewport(0, 0, tw, th);
    gl.disable(gl.BLEND); // the backdrop is opaque
    gl.drawArrays(gl.TRIANGLES, 0, 6);
    if (driveLoc.aPos !== loc.aPos) gl.disableVertexAttribArray(driveLoc.aPos);
    bindBatchState(); // restores target, program, blend, attribs, buffers
    // PASS 2: the finished art-pixel image as ONE NEAREST-upscaled quad at
    // the current transform's origin (texel row 0 is the scene's bottom).
    setTexture(driveTex);
    quad(0, 0, w, h, 0, 1, 1, 0, 1, 1, 1, 1);
  }

  /* ---- BACKDROP (opcode 24): the neon-wave void, one shader pass ----
     What shows OUTSIDE the level's floor bounds: 2-3 slow overlapping
     sine-field interference waves in heavily-darkened hot pink / cyan /
     violet over near-black. DRIVE economics — the shader runs once per ART
     pixel (px ~6 CSS px) into a tiny NEAREST target, then ONE upscaled
     opaque quad at the current transform's origin. Normal frame content:
     when POSTFX is active it lands in the scene FBO like everything else.
     Deliberately dim — the play area must dominate; peak brightness stays
     below every floor base tone in src/palette.rs (a void, not a light
     show). Periods 10 s+ (angular speeds <= ~0.5 rad/s). */
  const backdropProg = gl.createProgram();
  gl.attachShader(backdropProg, compile(gl.VERTEX_SHADER, DRIVE_VS));
  gl.attachShader(backdropProg, compile(gl.FRAGMENT_SHADER, BACKDROP_FS));
  gl.linkProgram(backdropProg);
  if (!gl.getProgramParameter(backdropProg, gl.LINK_STATUS)) {
    throw new Error("Backdrop program link failed: " + gl.getProgramInfoLog(backdropProg));
  }
  const backdropLoc = {
    aPos: gl.getAttribLocation(backdropProg, "aPos"),
    uSize: gl.getUniformLocation(backdropProg, "uSize"),
    uTexH: gl.getUniformLocation(backdropProg, "uTexH"),
    uT: gl.getUniformLocation(backdropProg, "uT"),
    uPx: gl.getUniformLocation(backdropProg, "uPx"),
  };
  // The backdrop's ART-RESOLUTION render target (ceil(w/px) x ceil(h/px)
  // texels, NEAREST) — its own texture: the drive's target may be live in
  // the same frame (`?viz` previews).
  let backdropTex = null, backdropFbo = null, backdropTW = 0, backdropTH = 0;
  function ensureBackdropTarget(tw, th) {
    if (backdropTW === tw && backdropTH === th) return;
    if (!backdropTex) {
      backdropTex = gl.createTexture();
      backdropFbo = gl.createFramebuffer();
      gl.bindTexture(gl.TEXTURE_2D, backdropTex);
      gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MIN_FILTER, gl.NEAREST);
      gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MAG_FILTER, gl.NEAREST);
      gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_S, gl.CLAMP_TO_EDGE);
      gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_T, gl.CLAMP_TO_EDGE);
    } else {
      gl.bindTexture(gl.TEXTURE_2D, backdropTex);
    }
    gl.texImage2D(gl.TEXTURE_2D, 0, gl.RGBA, tw, th, 0, gl.RGBA, gl.UNSIGNED_BYTE, null);
    gl.bindTexture(gl.TEXTURE_2D, null);
    gl.bindFramebuffer(gl.FRAMEBUFFER, backdropFbo);
    gl.framebufferTexture2D(gl.FRAMEBUFFER, gl.COLOR_ATTACHMENT0, gl.TEXTURE_2D, backdropTex, 0);
    if (gl.checkFramebufferStatus(gl.FRAMEBUFFER) !== gl.FRAMEBUFFER_COMPLETE) {
      throw new Error("Backdrop framebuffer is incomplete; the game cannot render.");
    }
    gl.bindFramebuffer(gl.FRAMEBUFFER, null);
    backdropTW = tw;
    backdropTH = th;
  }
  // `?backdrop=full` = ignore the exclusion rect (the A/B for `?gpuprobe`).
  const BACKDROP_FULL = typeof location !== "undefined"
    && new URLSearchParams(location.search).get("backdrop") === "full";
  function drawBackdrop(w, h, t, px, ex, ey, ew, eh) {
    // The floor covers the whole screen (mid-level, the common case): there
    // is no void to see — skip the wave pass and the quad altogether.
    if (!BACKDROP_FULL && ew > 0 && eh > 0 && ex <= 0 && ey <= 0 && ex + ew >= w && ey + eh >= h) return;
    flush();
    // PASS 1: the waves, one fragment per art pixel, into the tiny target.
    const tw = Math.ceil(w / px), th = Math.ceil(h / px);
    ensureBackdropTarget(tw, th);
    gl.useProgram(backdropProg);
    gl.disableVertexAttribArray(loc.aUv);
    gl.disableVertexAttribArray(loc.aColor);
    gl.bindBuffer(gl.ARRAY_BUFFER, postVbo);
    gl.enableVertexAttribArray(backdropLoc.aPos);
    gl.vertexAttribPointer(backdropLoc.aPos, 2, gl.FLOAT, false, 0, 0);
    gl.uniform2f(backdropLoc.uSize, w, h);
    gl.uniform1f(backdropLoc.uTexH, th);
    gl.uniform1f(backdropLoc.uT, t);
    gl.uniform1f(backdropLoc.uPx, px);
    gl.bindFramebuffer(gl.FRAMEBUFFER, backdropFbo);
    gl.viewport(0, 0, tw, th);
    gl.disable(gl.BLEND); // the backdrop is opaque
    gl.drawArrays(gl.TRIANGLES, 0, 6);
    if (backdropLoc.aPos !== loc.aPos) gl.disableVertexAttribArray(backdropLoc.aPos);
    bindBatchState(); // restores target, program, blend, attribs, buffers
    // PASS 2: the finished art-pixel image as ONE NEAREST-upscaled quad at
    // the current transform's origin (texel row 0 is the scene's bottom).
    // Drawn right away, UNBLENDED: an opaque full-screen quad through the
    // blending batch would still read the whole destination (see drawStatic).
    setTexture(backdropTex);
    const identity = m[0] === 1 && m[1] === 0 && m[2] === 0 && m[3] === 1 && m[4] === 0 && m[5] === 0;
    gl.disable(gl.BLEND);
    if (ew > 0 && eh > 0 && !BACKDROP_FULL && identity && !pix) {
      // OCCLUSION: (ex, ey, ew, eh) is a rect that opaque content drawn later
      // (the floor) is guaranteed to cover — src/backdrop_clip.rs. Draw the
      // void only AROUND it: the SAME full-screen quad four times under a
      // SCISSOR (above / below / left / right of the rect, whole physical
      // pixels, the rect rounded INWARD). Same triangles = bit-identical
      // texel choice for every surviving pixel (re-cut strips interpolate
      // their own UVs and flip NEAREST at texel boundaries —
      // tests/e2e/render/backdrop-clip.js caught exactly that), and fragments the
      // floor would paint over are never shaded at all.
      const sx = batchVW / batchW, sy = batchVH / batchH; // CSS px -> physical
      const x0 = Math.max(0, Math.ceil(ex * sx)), x1 = Math.min(batchVW, Math.floor((ex + ew) * sx));
      const y0 = Math.max(0, Math.ceil(ey * sy)), y1 = Math.min(batchVH, Math.floor((ey + eh) * sy));
      gl.enable(gl.SCISSOR_TEST);
      const strip = (px0, py0, px1, py1) => { // top-down physical px -> GL's bottom-up scissor
        if (px1 <= px0 || py1 <= py0) return;
        gl.scissor(px0, batchVH - py1, px1 - px0, py1 - py0);
        quad(0, 0, w, h, 0, 1, 1, 0, 1, 1, 1, 1);
        flush();
      };
      strip(0, 0, batchVW, y0);         // above
      strip(0, y1, batchVW, batchVH);   // below
      strip(0, y0, x0, y1);             // left
      strip(x1, y0, batchVW, y1);       // right
      gl.disable(gl.SCISSOR_TEST);
    } else {
      quad(0, 0, w, h, 0, 1, 1, 0, 1, 1, 1, 1);
      flush();
    }
    gl.enable(gl.BLEND);
  }

  /* ---- pixel-art groups: a NEAREST scratch target per nesting depth ---- */
  // Groups nest (depth <= PIX_DEPTH): each depth owns its own 1024x1024
  // scratch texture + FBO (an inner group's texture is sampled while the
  // outer group's texture is the render target, so they cannot share one),
  // created lazily — the plain game only ever touches depth 0.
  const pixTargets = [];
  function pixTarget(depth) {
    let t = pixTargets[depth];
    if (t) return t;
    const tex = gl.createTexture();
    gl.bindTexture(gl.TEXTURE_2D, tex);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MIN_FILTER, gl.NEAREST);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MAG_FILTER, gl.NEAREST);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_S, gl.CLAMP_TO_EDGE);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_T, gl.CLAMP_TO_EDGE);
    gl.texImage2D(gl.TEXTURE_2D, 0, gl.RGBA, PIX_MAX, PIX_MAX, 0, gl.RGBA, gl.UNSIGNED_BYTE, null);
    gl.bindTexture(gl.TEXTURE_2D, null);
    const fbo = gl.createFramebuffer();
    gl.bindFramebuffer(gl.FRAMEBUFFER, fbo);
    gl.framebufferTexture2D(gl.FRAMEBUFFER, gl.COLOR_ATTACHMENT0, gl.TEXTURE_2D, tex, 0);
    if (gl.checkFramebufferStatus(gl.FRAMEBUFFER) !== gl.FRAMEBUFFER_COMPLETE) {
      throw new Error("Pixel-group framebuffer is incomplete; the game cannot render.");
    }
    gl.bindFramebuffer(gl.FRAMEBUFFER, null);
    t = { tex, fbo };
    pixTargets[depth] = t;
    return t;
  }
  pixTarget(0);
  // The open groups, innermost last. A real entry holds the group's size and
  // the state its PIX_END restores (`outer*`: the enclosing target — the
  // canvas / scene FBO or an outer group's texture — and transform); a
  // PIX_BEGIN that fell back to pass-through (too big, too deep) pushes the
  // SKIP marker so its PIX_END is skipped too. `pix` is the innermost open
  // group or null.
  const PIX_SKIP = { skip: true };
  const pixStack = [];
  let pix = null;
  let pixDepth = 0; // number of REAL open groups
  // The most recently CLOSED real group — its texels persist in the scratch
  // texture until the next pixBegin, which is what PIX_BLIT re-draws from.
  let lastPix = null;
  // Size of one texel of the open group in current LOCAL units (the min
  // thickness / min diameter clamps). 1/sqrt(|det m|) = local units per texel.
  // During STATIC recording m is swapped to identity (the VBO records world
  // coordinates), which would report 1 world unit — but the section still
  // rasterizes into the open group's texels, so the real texel size comes
  // from the transform captured at STATIC_BEGIN (the group's world->texel
  // map). Without this, sub-texel features (the walls' 2-unit border) record
  // thinner than a texel and pop in/out per wall with the grid phase.
  function pixTexelLocal() {
    const mm = staticRec && pix ? staticRec.camM : m;
    const det = mm[0] * mm[3] - mm[1] * mm[2];
    const s = Math.sqrt(Math.abs(det));
    return s > 1e-9 ? 1 / s : 1;
  }
  function pixBegin(px, w, h, smooth) {
    px = Math.max(1, px || 1);
    const tw = Math.ceil(w / px), th = Math.ceil(h / px);
    if (pixDepth >= PIX_DEPTH || !(tw > 0 && th > 0) || tw > PIX_MAX || th > PIX_MAX) {
      pixStack.push(PIX_SKIP);
      return;
    }
    lastPix = null; // this begin may clear the texels a PIX_BLIT would sample
    flush();
    const tgt = pixTarget(pixDepth);
    const g = {
      px, w, h, tw, th, smooth: !!smooth, tex: tgt.tex, fbo: tgt.fbo,
      outer: pix, outerM: m, outerStack: stack.length,
      outerFbo: batchFbo, outerW: batchW, outerH: batchH,
      outerVW: batchVW, outerVH: batchVH,
    };
    pixStack.push(g);
    pix = g;
    pixDepth++;
    m = [1 / px, 0, 0, 1 / px, 0, 0];
    batchFbo = g.fbo;
    batchW = tw;
    batchH = th;
    batchVW = tw;
    batchVH = th;
    bindBatchState();
    // Clear just the region this group uses (scissored; clears ignore the viewport).
    gl.enable(gl.SCISSOR_TEST);
    gl.scissor(0, 0, tw, th);
    gl.clearColor(0, 0, 0, 0);
    gl.clear(gl.COLOR_BUFFER_BIT);
    gl.disable(gl.SCISSOR_TEST);
  }
  function pixEnd(x, y) {
    const g = pixStack.pop();
    if (!g || g.skip) return;
    lastPix = g; // the texels stay valid for PIX_BLIT until the next pixBegin
    flush(); // the group's content, into its scratch region
    pix = g.outer;
    pixDepth--;
    m = g.outerM;
    stack.length = g.outerStack; // balance away any unmatched SAVEs inside
    batchFbo = g.outerFbo;
    batchW = g.outerW;
    batchH = g.outerH;
    batchVW = g.outerVW;
    batchVH = g.outerVH;
    bindBatchState();
    // The group texels are premultiplied (drawn with straight-alpha colour
    // over transparent black, coverage accumulated), so composite them with
    // (ONE, 1-a) — into the canvas or into an outer group's texels alike.
    gl.blendFunc(gl.ONE, gl.ONE_MINUS_SRC_ALPHA);
    setGrain(1); // the composite's texels are premultiplied
    setTexture(g.tex);
    // Snap the on-screen origin to whole pixels of the CURRENT target's
    // coordinate space (CSS pixels on the canvas/scene, the outer group's
    // texels inside a group) so the art pixels do not shimmer as the object
    // drifts by fractions of a pixel. SMOOTH groups (`smooth` = 1 at BEGIN)
    // skip the snap: a composite that MOVES continuously (the `?pixel=N`
    // world under the camera sway) places sub-pixel so its motion never
    // quantizes. Sampling stays NEAREST either way — hard, aliased texel
    // edges are the art direction (CLAUDE.md ## Design), never smoothed.
    let dx = 0, dy = 0;
    if (!g.smooth) {
      const sx = m[0] * x + m[2] * y + m[4];
      const sy = m[1] * x + m[3] * y + m[5];
      dx = Math.round(sx) - sx;
      dy = Math.round(sy) - sy;
    }
    m[4] += dx;
    m[5] += dy;
    // Row 0 of the region is the group's bottom (GL's bottom-up window
    // coordinates through the same VS), so v is flipped like the sprite
    // tiles. The flip is anchored at the INTEGER row count `th` (the
    // viewport the content rasterized in), NOT at the fractional g.h/g.px:
    // with a fractional group height the content's top row sits at texel
    // row th, its bottom at th - h/px — anchoring v at 0 would shift every
    // sample up by the ceil remainder (NEAREST rounds that to a whole-row
    // shift, and the remainder CHANGES as a camera-sized group resizes, so
    // all horizontal content would swim row by row while the camera pans).
    const u1 = g.w / g.px / PIX_MAX;
    const v0 = g.th / PIX_MAX;
    const v1 = (g.th - g.h / g.px) / PIX_MAX;
    quad(x, y, g.w, g.h, 0, v0, u1, v1, 1, 1, 1, 1);
    m[4] -= dx;
    m[5] -= dy;
    flush();
    pixBlend();
  }
  // PIX_BLIT: re-draw the rect (sx, sy)..(sx+sw, sy+sh) — in the last-closed
  // group's LOCAL units — of that group's scratch texels as a (sw, sh) quad
  // at (x, y) in the current transform. This is what makes "rasterize once,
  // place many times" possible (drive.rs's tear bands): each extra placement
  // costs one textured quad instead of a re-record of the group's content.
  // A no-op when there is no valid source (pass-through group, or a pixBegin
  // has run since — its clear may have invalidated the texels).
  function pixBlit(sx, sy, sw, sh, x, y) {
    const g = lastPix;
    if (!g || !(sw > 0 && sh > 0)) return;
    flush();
    // Same composite + snap as the PIX_END draw.
    gl.blendFunc(gl.ONE, gl.ONE_MINUS_SRC_ALPHA);
    setGrain(1); // the composite's texels are premultiplied
    setTexture(g.tex);
    const tx = m[0] * x + m[2] * y + m[4];
    const ty = m[1] * x + m[3] * y + m[5];
    const dx = Math.round(tx) - tx, dy = Math.round(ty) - ty;
    m[4] += dx;
    m[5] += dy;
    // v flipped like PIX_END: local y = 0 is the group's TOP texel row,
    // anchored at the INTEGER row count g.th (see the pixEnd comment).
    const u0 = sx / g.px / PIX_MAX;
    const u1 = (sx + sw) / g.px / PIX_MAX;
    const v0 = (g.th - sy / g.px) / PIX_MAX;
    const v1 = (g.th - (sy + sh) / g.px) / PIX_MAX;
    quad(x, y, sw, sh, u0, v0, u1, v1, 1, 1, 1, 1);
    m[4] -= dx;
    m[5] -= dy;
    flush();
    pixBlend();
  }
  // The folded static (see FS): on for draws that land on the CANVAS of a
  // folded frame, off inside pixel groups; `pre` = 1 for a group's
  // premultiplied composite quad. Callers flush first (uniforms are per draw).
  let grainT = 0, grainU0 = 0, grainV0 = 0;
  function setGrain(pre) {
    if (!GRAIN_FOLD) return; // (the uniforms do not exist in the plain shader)
    gl.uniform1f(loc.uGrainT, pix ? 0 : grainT);
    gl.uniform1f(loc.uGrainPre, pre);
  }
  // The batch blend for the current target: straight alpha onto the canvas /
  // scene; into a transparent group target straight alpha for colour but
  // accumulated coverage (a = sa + da * (1 - sa)) so the texels come out
  // premultiplied and PIX_END can composite them correctly.
  function pixBlend() {
    // (CTX_ALPHA: the canvas has an alpha channel — the same separate
    // blend keeps it at 1, i.e. opaque over the page, whatever is drawn.)
    if (pix || CTX_ALPHA) {
      gl.blendFuncSeparate(gl.SRC_ALPHA, gl.ONE_MINUS_SRC_ALPHA, gl.ONE, gl.ONE_MINUS_SRC_ALPHA);
    } else {
      gl.blendFunc(gl.SRC_ALPHA, gl.ONE_MINUS_SRC_ALPHA);
    }
    setGrain(0); // (straight-alpha draws; on only when the target is the canvas)
  }

  // Re-establish everything the batched pipeline relies on. The robot passes
  // rebind program/buffers/attribs/framebuffer/viewport/blend/depth, so this
  // runs after them (and it is cheap enough to be defensive about it).
  function bindBatchState() {
    gl.bindFramebuffer(gl.FRAMEBUFFER, batchFbo);
    gl.viewport(0, 0, batchVW, batchVH);
    gl.useProgram(prog);
    gl.uniform2f(loc.uRes, batchW, batchH);
    gl.disable(gl.DEPTH_TEST);
    gl.disable(gl.CULL_FACE);
    gl.disable(gl.SCISSOR_TEST);
    gl.enable(gl.BLEND);
    pixBlend();
    gl.bindBuffer(gl.ARRAY_BUFFER, vbo);
    gl.enableVertexAttribArray(loc.aPos);
    gl.vertexAttribPointer(loc.aPos, 2, gl.FLOAT, false, STRIDE, 0);
    gl.enableVertexAttribArray(loc.aUv);
    gl.vertexAttribPointer(loc.aUv, 2, gl.FLOAT, false, STRIDE, 8);
    gl.enableVertexAttribArray(loc.aColor);
    gl.vertexAttribPointer(loc.aColor, 4, gl.FLOAT, false, STRIDE, 16);
    gl.activeTexture(gl.TEXTURE0);
  }

  // Run the queued robot / shoggoth renders into their atlas tiles. Leaves the
  // batch state rebound (and TEXTURE0 unbound — flush binds what it needs).
  function renderQueuedSprites() {
    let perfT = 0;
    if (PERF) {
      perfT = performance.now();
      if (perfSpriteT0 === 0) perfSpriteT0 = perfT;
      perfRobotN += robotUsed + shogUsed;
    }
    // Our attrib arrays would otherwise stay enabled (pointing at the batch
    // VBO) while the sprite programs draw; keep the pipelines disjoint.
    gl.disableVertexAttribArray(loc.aPos);
    gl.disableVertexAttribArray(loc.aUv);
    gl.disableVertexAttribArray(loc.aColor);
    // Robots: ONE batch — every robot into its own tile viewport of the
    // pipeline's shared scene target, then one post draw over all of them
    // into the atlas at block resolution (tile i at column i % robotCols,
    // row floor(i / robotCols), ROBOT_ART texels each — what drawRobot samples).
    if (robotUsed > 0) {
      robotPipe.batchBegin(robotCols, robotUsed);
      for (let i = 0; i < robotUsed; i++) {
        const q = i * ROBOT_Q, Q = robotQueue;
        robotOpts.color = ROBOT_COLORS[Q[q] | 0] || ROBOT_COLORS[0];
        robotOpts.weapon = ROBOT_WEAPONS[Q[q + 1] | 0] || ROBOT_WEAPONS[0];
        planFromScalars(robotPlan, Q, q + 3, Q[q + 2] | 0);
        robotPipe.batchDraw(i, robotOpts);
      }
      robotPipe.batchEnd(robotTarget, robotCols, robotUsed, ROBOT_PX, true);
    }
    for (let i = 0; i < shogUsed; i++) {
      const q = i * 3;
      shogOpts.heading = shogQueue[q];
      shogOpts.reveal = shogQueue[q + 1];
      shogOpts.time = shogQueue[q + 2];
      shogTarget.x = (i % shogCols) * SHOG_TILE;
      shogTarget.y = Math.floor(i / shogCols) * SHOG_TILE;
      shogPipe.render(shogOpts, shogTarget);
    }
    // The pipelines sampled their own scene texture on TEXTURE0; drop it so an
    // atlas is never both bound for sampling and attached to a framebuffer.
    gl.bindTexture(gl.TEXTURE_2D, null);
    bindBatchState();
    if (PERF) perfSpriteMs += performance.now() - perfT;
  }

  // The pipeline's constructor left its own buffers bound: put ours back.
  bindBatchState();

  let boundTex = null;
  function flush() {
    // before the batch that samples them
    if (robotUsed > 0 || shogUsed > 0) renderQueuedSprites();
    if (vCount === 0) {
      robotUsed = 0;
      shogUsed = 0;
      return;
    }
    gl.bindTexture(gl.TEXTURE_2D, boundTex || whiteTex);
    gl.bindBuffer(gl.ARRAY_BUFFER, vbo);
    if (vboSubData) gl.bufferSubData(gl.ARRAY_BUFFER, 0, verts.subarray(0, vCount * FLOATS_PER_VERT));
    else gl.bufferData(gl.ARRAY_BUFFER, verts.subarray(0, vCount * FLOATS_PER_VERT), gl.DYNAMIC_DRAW);
    gl.drawArrays(gl.TRIANGLES, 0, vCount);
    vCount = 0;
    robotUsed = 0; // the quads sampling this batch's tiles are submitted: recycle
    shogUsed = 0;
  }

  function setTexture(tex) {
    if (boundTex !== tex) {
      flush();
      boundTex = tex;
    }
  }

  /* ---- transform stack (canvas-style: translate/rotate only) ---- */
  // Row form [a, b, c, d, e, f]: x' = a*x + c*y + e ; y' = b*x + d*y + f
  let m = [1, 0, 0, 1, 0, 0];
  const stack = [];
  function tSave() {
    stack.push(m.slice());
  }
  function tRestore() {
    if (stack.length) m = stack.pop();
  }
  function tTranslate(x, y) {
    m[4] += m[0] * x + m[2] * y;
    m[5] += m[1] * x + m[3] * y;
  }
  function tScale(sx, sy) {
    m[0] *= sx; m[1] *= sx;
    m[2] *= sy; m[3] *= sy;
  }
  function tRotate(angle) {
    const c = Math.cos(angle), s = Math.sin(angle);
    const a0 = m[0], b0 = m[1], c0 = m[2], d0 = m[3];
    m[0] = a0 * c + c0 * s;
    m[1] = b0 * c + d0 * s;
    m[2] = -a0 * s + c0 * c;
    m[3] = -b0 * s + d0 * c;
  }

  /* ---- STATIC GEOMETRY CACHE (opcodes 21/22/23) ----
     Frame-invariant world geometry (the floor tiles + walls) baked ONCE into
     a persistent VBO and re-drawn every later frame for a 2-float STATIC_REF
     — instead of ~2000 floats re-recorded and re-tessellated per frame (the
     bulk of the `walk` span). STATIC_BEGIN `key` flushes and starts routing
     every tessellated vertex into a growable side buffer, with the transform
     in force at the BEGIN (the camera) REPLACED by identity — so the
     vertices come out in WORLD coordinates, while the section's own
     save/translate/... still apply. STATIC_END uploads the buffer to a
     persistent VBO under `key` (gl.bufferData STATIC_DRAW, once; a
     different key's old buffer is deleted — one live key, the current
     floor), restores the camera transform and draws the cache. STATIC_REF
     `key` just draws it: the CPU-side transform at that point (the camera,
     which moves/zooms/sways every frame) is handed to the batch vertex
     shader as the uXA/uXB affine uniform, applied on the GPU — identical
     math to the CPU path, so the cache tracks the camera exactly; dynamic
     draws keep uXA/uXB at identity. The section must be SOLID geometry only
     (everything samples whiteTex): text or sprites inside would bake with
     the wrong texture. Works inside a pixel group too (the `?pixel=N`
     world): there `m` is the group's world->texel mapping — still affine —
     and the batch target is the group's scratch region, so the same VBO
     draws into the group's texels; one buffer serves both modes. */
  let staticCache = null; // { key, vbo, count } — the one cached section
  let staticRec = null; // { key, camM } while recording BEGIN..END
  let staticVerts = new Float32Array(4096 * FLOATS_PER_VERT); // grows
  let staticCount = 0;
  let staticOpaque = true; // every recorded vertex had alpha 1 (see drawStatic)
  let staticWarned = false;
  function staticVert(x, y, u, v, r, g, b, a) {
    if ((staticCount + 1) * FLOATS_PER_VERT > staticVerts.length) {
      const grown = new Float32Array(staticVerts.length * 2);
      grown.set(staticVerts);
      staticVerts = grown;
    }
    const o = staticCount * FLOATS_PER_VERT;
    staticVerts[o] = x;
    staticVerts[o + 1] = y;
    staticVerts[o + 2] = u;
    staticVerts[o + 3] = v;
    staticVerts[o + 4] = r;
    staticVerts[o + 5] = g;
    staticVerts[o + 6] = b;
    staticVerts[o + 7] = a;
    if (a < 1) staticOpaque = false;
    staticCount++;
  }
  function staticBegin(key) {
    flush(); // everything recorded before the section draws first (order)
    staticRec = { key, camM: m };
    m = [1, 0, 0, 1, 0, 0]; // record in world coordinates
    staticCount = 0;
    staticOpaque = true;
  }
  function staticEnd() {
    if (!staticRec) return;
    const { key, camM } = staticRec;
    staticRec = null;
    m = camM; // the camera transform is live again
    if (staticCache && staticCache.key !== key) {
      gl.deleteBuffer(staticCache.vbo); // a new key evicts the old floor
      staticCache = null;
    }
    if (!staticCache) staticCache = { key, vbo: gl.createBuffer(), count: 0 };
    gl.bindBuffer(gl.ARRAY_BUFFER, staticCache.vbo);
    gl.bufferData(
      gl.ARRAY_BUFFER,
      staticVerts.subarray(0, staticCount * FLOATS_PER_VERT),
      gl.STATIC_DRAW
    );
    gl.bindBuffer(gl.ARRAY_BUFFER, vbo);
    staticCache.key = key;
    staticCache.count = staticCount;
    staticCache.opaque = staticOpaque;
    if (PERF) PERF.staticOpaque = staticOpaque;
    drawStatic(key); // the build frame draws it too
  }
  function drawStatic(key) {
    if (!staticCache || staticCache.key !== key || staticCache.count === 0) {
      if (!staticWarned) {
        staticWarned = true;
        console.error("frameRender: STATIC_REF for uncached key", key);
      }
      return;
    }
    flush(); // pending dynamic geometry first (draw order)
    // The camera: the CPU-side transform at this point, applied in the VS.
    gl.uniform3f(loc.uXA, m[0], m[2], m[4]);
    gl.uniform3f(loc.uXB, m[1], m[3], m[5]);
    gl.bindTexture(gl.TEXTURE_2D, whiteTex); // the section is solid geometry
    gl.bindBuffer(gl.ARRAY_BUFFER, staticCache.vbo);
    gl.vertexAttribPointer(loc.aPos, 2, gl.FLOAT, false, STRIDE, 0);
    gl.vertexAttribPointer(loc.aUv, 2, gl.FLOAT, false, STRIDE, 8);
    gl.vertexAttribPointer(loc.aColor, 4, gl.FLOAT, false, STRIDE, 16);
    // FILL-RATE: the floor + walls cover most of the screen, several tiles
    // deep. With every vertex opaque the blend is a no-op that still makes
    // the GPU READ the destination for every fragment — on a bandwidth-bound
    // integrated GPU that is half the cost of the layer. Same pixels without.
    if (staticCache.opaque) gl.disable(gl.BLEND);
    gl.drawArrays(gl.TRIANGLES, 0, staticCache.count);
    if (staticCache.opaque) gl.enable(gl.BLEND);
    // Hand the state back to the dynamic batch: identity + the stream VBO.
    gl.uniform3f(loc.uXA, 1, 0, 0);
    gl.uniform3f(loc.uXB, 0, 1, 0);
    gl.bindBuffer(gl.ARRAY_BUFFER, vbo);
    gl.vertexAttribPointer(loc.aPos, 2, gl.FLOAT, false, STRIDE, 0);
    gl.vertexAttribPointer(loc.aUv, 2, gl.FLOAT, false, STRIDE, 8);
    gl.vertexAttribPointer(loc.aColor, 4, gl.FLOAT, false, STRIDE, 16);
  }

  function vert(x, y, u, v, r, g, b, a) {
    if (staticRec) {
      // Recording a static section: world-space vertex into the side buffer
      // (m is identity + the section's own local transforms).
      staticVert(
        m[0] * x + m[2] * y + m[4],
        m[1] * x + m[3] * y + m[5],
        u, v, r, g, b, a
      );
      return;
    }
    if (vCount >= MAX_VERTS) flush(); // order-safe: same texture, same state
    const o = vCount * FLOATS_PER_VERT;
    verts[o] = m[0] * x + m[2] * y + m[4];
    verts[o + 1] = m[1] * x + m[3] * y + m[5];
    verts[o + 2] = u;
    verts[o + 3] = v;
    verts[o + 4] = r;
    verts[o + 5] = g;
    verts[o + 6] = b;
    verts[o + 7] = a;
    vCount++;
  }

  // Textured axis-aligned quad in *local* space (goes through the transform).
  function quad(x, y, w, h, u0, v0, u1, v1, r, g, b, a) {
    vert(x, y, u0, v0, r, g, b, a);
    vert(x + w, y, u1, v0, r, g, b, a);
    vert(x + w, y + h, u1, v1, r, g, b, a);
    vert(x, y, u0, v0, r, g, b, a);
    vert(x + w, y + h, u1, v1, r, g, b, a);
    vert(x, y + h, u0, v1, r, g, b, a);
  }

  // Vertex already in TARGET space (device pixels, or the open group's
  // texels): bypasses the transform.
  function vertRaw(tx, ty, u, v, r, g, b, a) {
    if (staticRec) {
      // During static recording target space IS world space (m = identity +
      // the section's own transforms; circles tessellate through here).
      staticVert(tx, ty, u, v, r, g, b, a);
      return;
    }
    if (vCount >= MAX_VERTS) flush();
    const o = vCount * FLOATS_PER_VERT;
    verts[o] = tx;
    verts[o + 1] = ty;
    verts[o + 2] = u;
    verts[o + 3] = v;
    verts[o + 4] = r;
    verts[o + 5] = g;
    verts[o + 6] = b;
    verts[o + 7] = a;
    vCount++;
  }
  function quadRaw(x0, y0, x1, y1, r, g, b, a) {
    vertRaw(x0, y0, 0.5, 0.5, r, g, b, a);
    vertRaw(x1, y0, 0.5, 0.5, r, g, b, a);
    vertRaw(x1, y1, 0.5, 0.5, r, g, b, a);
    vertRaw(x0, y0, 0.5, 0.5, r, g, b, a);
    vertRaw(x1, y1, 0.5, 0.5, r, g, b, a);
    vertRaw(x0, y1, 0.5, 0.5, r, g, b, a);
  }
  // Uniform scale of the transform (local unit -> target pixels), or 0 when
  // the axes are not (near) equal length (non-uniform scale: no snapping).
  function uniformScale() {
    const sx = Math.hypot(m[0], m[1]), sy = Math.hypot(m[2], m[3]);
    return Math.abs(sx - sy) <= 1e-3 * (sx + sy) ? sx : 0;
  }
  const AXIS_ALIGNED = () => Math.abs(m[1]) < 1e-6 && Math.abs(m[2]) < 1e-6;

  // ---- THE PIXEL-ART RULE INSIDE GROUPS ----
  // Inside a pixel-art group primitives are snapped to the texel grid at
  // rasterization time so a moving / animated shape keeps ONE constant stamp
  // and hops texel by texel instead of deforming with the grid phase:
  //   rects  (axis-aligned) size rounded ONCE to whole texels (min 1), then
  //          the origin rounded to whole texels;
  //   circles of radius <= 2 texels: radius rounded to a half-texel, centre
  //          snapped to a texel centre (odd diameter) / corner (even);
  //          bigger circles stay continuous (fans, wells);
  //   lines  thickness rounded to whole texels (min 1), endpoints snapped to
  //          texel centres.
  // Circles are always tessellated in TARGET space (polygon phase fixed to
  // the target axes, segment count from the on-target radius) so a circle
  // drawn under a rotating transform never changes its rasterization: the
  // well / hub of a spinning fan is frame-stable, only the blades move.

  function solidRect(x, y, w, h, r, g, b, a) {
    setTexture(whiteTex);
    if (pix && AXIS_ALIGNED()) {
      let x0 = m[0] * x + m[4], y0 = m[3] * y + m[5];
      let x1 = m[0] * (x + w) + m[4], y1 = m[3] * (y + h) + m[5];
      if (x1 < x0) { const t = x0; x0 = x1; x1 = t; }
      if (y1 < y0) { const t = y0; y0 = y1; y1 = t; }
      const ws = Math.max(1, Math.round(x1 - x0)), hs = Math.max(1, Math.round(y1 - y0));
      x0 = Math.round(x0);
      y0 = Math.round(y0);
      quadRaw(x0, y0, x0 + ws, y0 + hs, r, g, b, a);
      return;
    }
    quad(x, y, w, h, 0.5, 0.5, 0.5, 0.5, r, g, b, a);
  }

  // Stroke centered on the rect edges, matching canvas strokeRect.
  function rectLines(x, y, w, h, t, r, g, b, a) {
    if (pix) t = Math.max(t, pixTexelLocal()); // >= 1 texel
    const ht = t / 2;
    solidRect(x - ht, y - ht, w + t, t, r, g, b, a); // top
    solidRect(x - ht, y + h - ht, w + t, t, r, g, b, a); // bottom
    solidRect(x - ht, y + ht, t, h - t, r, g, b, a); // left
    solidRect(x + w - ht, y + ht, t, h - t, r, g, b, a); // right
  }

  function circle(x, y, radius, r, g, b, a) {
    setTexture(whiteTex);
    const sc = uniformScale();
    if (sc > 0) {
      // Target-space tessellation (rotation-invariant).
      let tx = m[0] * x + m[2] * y + m[4], ty = m[1] * x + m[3] * y + m[5];
      let rt = radius * sc;
      if (pix) {
        rt = Math.max(rt, 0.5); // >= 1 texel across
        if (rt <= 2) {
          const d = Math.max(1, Math.round(2 * rt)); // diameter in whole texels
          rt = d / 2;
          if (d & 1) {
            tx = Math.floor(tx) + 0.5;
            ty = Math.floor(ty) + 0.5;
          } else {
            tx = Math.round(tx);
            ty = Math.round(ty);
          }
        }
      }
      const segs = Math.max(12, Math.min(96, Math.ceil(rt)));
      for (let i = 0; i < segs; i++) {
        const a0 = (i / segs) * Math.PI * 2;
        const a1 = ((i + 1) / segs) * Math.PI * 2;
        vertRaw(tx, ty, 0.5, 0.5, r, g, b, a);
        vertRaw(tx + Math.cos(a0) * rt, ty + Math.sin(a0) * rt, 0.5, 0.5, r, g, b, a);
        vertRaw(tx + Math.cos(a1) * rt, ty + Math.sin(a1) * rt, 0.5, 0.5, r, g, b, a);
      }
      return;
    }
    if (pix) radius = Math.max(radius, 0.5 * pixTexelLocal()); // >= 1 texel across
    const segs = Math.max(12, Math.min(48, Math.ceil(radius)));
    for (let i = 0; i < segs; i++) {
      const a0 = (i / segs) * Math.PI * 2;
      const a1 = ((i + 1) / segs) * Math.PI * 2;
      vert(x, y, 0.5, 0.5, r, g, b, a);
      vert(x + Math.cos(a0) * radius, y + Math.sin(a0) * radius, 0.5, 0.5, r, g, b, a);
      vert(x + Math.cos(a1) * radius, y + Math.sin(a1) * radius, 0.5, 0.5, r, g, b, a);
    }
  }

  // Filled pie slice from a0 to a1 (canvas arc + close + fill semantics).
  function arcPie(x, y, radius, a0, a1, r, g, b, a) {
    setTexture(whiteTex);
    let span = a1 - a0;
    if (span < 0) span += Math.PI * 2;
    const segs = Math.max(4, Math.ceil((span / (Math.PI * 2)) * 48));
    for (let i = 0; i < segs; i++) {
      const s0 = a0 + (span * i) / segs;
      const s1 = a0 + (span * (i + 1)) / segs;
      vert(x, y, 0.5, 0.5, r, g, b, a);
      vert(x + Math.cos(s0) * radius, y + Math.sin(s0) * radius, 0.5, 0.5, r, g, b, a);
      vert(x + Math.cos(s1) * radius, y + Math.sin(s1) * radius, 0.5, 0.5, r, g, b, a);
    }
  }

  // Butt-capped line segment as a quad (canvas default lineCap).
  function line(x1, y1, x2, y2, t, r, g, b, a) {
    setTexture(whiteTex);
    const sc = pix ? uniformScale() : 0;
    if (sc > 0) {
      // In a group: endpoints to texel centres, whole-texel thickness, in
      // target space.
      const ax = Math.floor(m[0] * x1 + m[2] * y1 + m[4]) + 0.5;
      const ay = Math.floor(m[1] * x1 + m[3] * y1 + m[5]) + 0.5;
      const bx = Math.floor(m[0] * x2 + m[2] * y2 + m[4]) + 0.5;
      const by = Math.floor(m[1] * x2 + m[3] * y2 + m[5]) + 0.5;
      const tt = Math.max(1, Math.round(t * sc));
      const dx = bx - ax, dy = by - ay;
      const len = Math.hypot(dx, dy);
      if (len < 1e-6) {
        // Degenerate after snapping: one texel-sized dot.
        const h = tt / 2;
        quadRaw(ax - h, ay - h, ax + h, ay + h, r, g, b, a);
        return;
      }
      const nx = (-dy / len) * (tt / 2), ny = (dx / len) * (tt / 2);
      vertRaw(ax + nx, ay + ny, 0.5, 0.5, r, g, b, a);
      vertRaw(bx + nx, by + ny, 0.5, 0.5, r, g, b, a);
      vertRaw(bx - nx, by - ny, 0.5, 0.5, r, g, b, a);
      vertRaw(ax + nx, ay + ny, 0.5, 0.5, r, g, b, a);
      vertRaw(bx - nx, by - ny, 0.5, 0.5, r, g, b, a);
      vertRaw(ax - nx, ay - ny, 0.5, 0.5, r, g, b, a);
      return;
    }
    if (pix) t = Math.max(t, pixTexelLocal()); // >= 1 texel
    const dx = x2 - x1, dy = y2 - y1;
    const len = Math.hypot(dx, dy);
    if (len < 1e-6) return;
    const nx = (-dy / len) * (t / 2);
    const ny = (dx / len) * (t / 2);
    vert(x1 + nx, y1 + ny, 0.5, 0.5, r, g, b, a);
    vert(x2 + nx, y2 + ny, 0.5, 0.5, r, g, b, a);
    vert(x2 - nx, y2 - ny, 0.5, 0.5, r, g, b, a);
    vert(x1 + nx, y1 + ny, 0.5, 0.5, r, g, b, a);
    vert(x2 - nx, y2 - ny, 0.5, 0.5, r, g, b, a);
    vert(x1 - nx, y1 - ny, 0.5, 0.5, r, g, b, a);
  }

  /* ---- glyph atlas: lazy VT323 rasterization ---- */
  const glyphs = new Map(); // char -> {u0,v0,u1,v1,w,h,advance}
  const glyphCellH = Math.ceil(GLYPH_FS * 1.3);
  const glyphBaseline = GLYPH_FS; // baseline offset from cell top
  let glyphPenX = 0;
  let glyphPenY = 0;
  const scratch = document.createElement("canvas");
  const scratchCtx = scratch.getContext("2d", { willReadFrequently: false });

  function bakeGlyph(ch) {
    scratchCtx.font = `${GLYPH_FS}px 'GameFont', monospace`;
    const advance = scratchCtx.measureText(ch).width;
    const cellW = Math.ceil(advance) + GLYPH_PAD * 2;
    if (glyphPenX + cellW > GLYPH_ATLAS_SIZE) {
      glyphPenX = 0;
      glyphPenY += glyphCellH;
    }
    if (glyphPenY + glyphCellH > GLYPH_ATLAS_SIZE) {
      // Atlas full (would need hundreds of distinct glyphs) — reset it.
      glyphs.clear();
      glyphPenX = 0;
      glyphPenY = 0;
    }
    scratch.width = cellW;
    scratch.height = glyphCellH;
    scratchCtx.clearRect(0, 0, cellW, glyphCellH);
    scratchCtx.font = `${GLYPH_FS}px 'GameFont', monospace`;
    scratchCtx.fillStyle = "#ffffff";
    scratchCtx.textBaseline = "alphabetic";
    scratchCtx.fillText(ch, GLYPH_PAD, glyphBaseline);
    flush(); // texture upload must not reorder past pending quads
    gl.bindTexture(gl.TEXTURE_2D, glyphTex);
    gl.texSubImage2D(gl.TEXTURE_2D, 0, glyphPenX, glyphPenY, gl.RGBA, gl.UNSIGNED_BYTE, scratch);
    const info = {
      u0: glyphPenX / GLYPH_ATLAS_SIZE,
      v0: glyphPenY / GLYPH_ATLAS_SIZE,
      u1: (glyphPenX + cellW) / GLYPH_ATLAS_SIZE,
      v1: (glyphPenY + glyphCellH) / GLYPH_ATLAS_SIZE,
      w: cellW,
      h: glyphCellH,
      advance,
    };
    glyphs.set(ch, info);
    glyphPenX += cellW;
    return info;
  }

  function drawText(text, x, y, size, r, g, b, a) {
    const s = size / GLYPH_FS;
    let pen = x;
    for (const ch of text) {
      if (ch === " ") {
        let info = glyphs.get(" ");
        if (!info) info = bakeGlyph(" ");
        pen += info.advance * s;
        continue;
      }
      let info = glyphs.get(ch);
      if (!info) info = bakeGlyph(ch);
      setTexture(glyphTex);
      quad(
        pen - GLYPH_PAD * s,
        y - glyphBaseline * s,
        info.w * s,
        info.h * s,
        info.u0, info.v0, info.u1, info.v1,
        r, g, b, a
      );
      pen += info.advance * s;
    }
  }

  /* ---- robots: queue a live render into a scratch tile, draw it as a quad ---- */
  // Facing is applied as quad rotation (the tile is rendered facing "up"), so
  // the robot goes through the transform stack like every other quad. Args at
  // cmds[a..]: colorIdx weaponIdx flags x y angle sizePx + the 11 pose scalars.
  function drawRobot(cmds, a) {
    const x = cmds[a + 3], y = cmds[a + 4], angle = cmds[a + 5], sizePx = cmds[a + 6];
    setTexture(robotTex);
    // Need a free tile AND room for the whole quad in this batch: a flush
    // recycles tiles, so the six verts of one robot must never straddle one.
    if (robotUsed >= robotSlots || vCount + 6 > MAX_VERTS) flush();
    const slot = robotUsed++;
    const q = slot * ROBOT_Q;
    robotQueue[q] = cmds[a];         // colorIdx
    robotQueue[q + 1] = cmds[a + 1]; // weaponIdx
    robotQueue[q + 2] = cmds[a + 2]; // flags
    for (let k = 0; k < POSE_SCALARS.length; k++) robotQueue[q + 3 + k] = cmds[a + 7 + k];
    // The tile holds one atlas texel per pixelate block; the quad covers the
    // tile's 128 scene texels = 128 / 3 blocks (the last one partial), inset
    // by half a scene texel on each side (against neighbor-tile bleed — the
    // same mapping as sampling a 1:1 tile, so the on-screen size is unchanged).
    const span = ROBOT_TILE / ROBOT_PX;
    const inset = 0.5 / ROBOT_PX;
    const tx = (slot % robotCols) * ROBOT_ART;
    const ty = Math.floor(slot / robotCols) * ROBOT_ART;
    // Pass 2 draws with GL's bottom-up viewport, so the tile's first row is
    // the robot's bottom: flip v so the quad reads it top-down like the canvas.
    const u0 = (tx + inset) / ROBOT_ATLAS_SIZE;
    const v0 = (ty + span - inset) / ROBOT_ATLAS_SIZE;
    const u1 = (tx + span - inset) / ROBOT_ATLAS_SIZE;
    const v1 = (ty + inset) / ROBOT_ATLAS_SIZE;
    const h = sizePx / 2;
    const c = Math.cos(angle), s = Math.sin(angle);
    // Rotated quad corners in local space (rotation about the robot's
    // center), then through the transform stack in vert().
    const ex = h * c, ey = h * s; // half-extent along the rotated x axis
    const fx = -h * s, fy = h * c; // half-extent along the rotated y axis
    const x0 = x - ex - fx, y0 = y - ey - fy; // top-left
    const x1 = x + ex - fx, y1 = y + ey - fy; // top-right
    const x2 = x + ex + fx, y2 = y + ey + fy; // bottom-right
    const x3 = x - ex + fx, y3 = y - ey + fy; // bottom-left
    vert(x0, y0, u0, v0, 1, 1, 1, 1);
    vert(x1, y1, u1, v0, 1, 1, 1, 1);
    vert(x2, y2, u1, v1, 1, 1, 1, 1);
    vert(x0, y0, u0, v0, 1, 1, 1, 1);
    vert(x2, y2, u1, v1, 1, 1, 1, 1);
    vert(x3, y3, u0, v1, 1, 1, 1, 1);
  }

  /* ---- shoggoth: queue a live boss render into a scratch tile, draw it as a quad ---- */
  // Axis-aligned quad of sizePx centered on (x, y), through the transform
  // stack. `heading` (radians, screen convention: 0 = +x, PI/2 = +y/down) is
  // what the mask leans toward; `reveal` 0..1 is the mask-off progress (0 =
  // masked, 1 = raw form); `time` is the engine's continuous clock.
  function drawShoggoth(x, y, sizePx, heading, reveal, time) {
    setTexture(shogTex);
    if (shogUsed >= shogSlots || vCount + 6 > MAX_VERTS) flush();
    const slot = shogUsed++;
    const q = slot * 3;
    shogQueue[q] = heading;
    shogQueue[q + 1] = reveal;
    shogQueue[q + 2] = time;
    const inset = 0.5;
    const tx = (slot % shogCols) * SHOG_TILE;
    const ty = Math.floor(slot / shogCols) * SHOG_TILE;
    // v flipped: pass 2 renders bottom-up (see drawRobot)
    const u0 = (tx + inset) / SHOG_ATLAS_SIZE;
    const v0 = (ty + SHOG_TILE - inset) / SHOG_ATLAS_SIZE;
    const u1 = (tx + SHOG_TILE - inset) / SHOG_ATLAS_SIZE;
    const v1 = (ty + inset) / SHOG_ATLAS_SIZE;
    const h = sizePx / 2;
    quad(x - h, y - h, sizePx, sizePx, u0, v0, u1, v1, 1, 1, 1, 1);
  }

  // Dialogue portrait: the baked (colorIdx, mode) face from the persistent
  // portrait cache — a fixed-camera, frozen-pose 64-texel render made once —
  // NEAREST-upscaled to sizePx on a quad that gently ROCKS around its centre
  // (`time` only drives the 2D tilt: the classic Hotline-Miami portrait).
  // mode 0 = bust (slightly-elevated full-body camera), mode 1 = headshot
  // (pushed in / raised to head height: the face fills the tile). Screen
  // space (through the transform stack, like everything).
  function drawPortrait(colorIdx, x, y, sizePx, time, mode) {
    const ci = ROBOT_COLORS[colorIdx | 0] ? colorIdx | 0 : 0;
    const slot = portraitSlotFor(ci, mode > 0.5 ? 1 : 0); // bakes on first use
    setTexture(portraitTex);
    if (vCount + 6 > MAX_VERTS) flush();
    const tx = (slot % portraitCols) * FX_TILE;
    const ty = Math.floor(slot / portraitCols) * FX_TILE;
    // v flipped: pass 2 renders bottom-up (see drawRobot)
    const u0 = tx / PORTRAIT_ATLAS_SIZE;
    const v0 = (ty + FX_TILE) / PORTRAIT_ATLAS_SIZE;
    const u1 = (tx + FX_TILE) / PORTRAIT_ATLAS_SIZE;
    const v1 = ty / PORTRAIT_ATLAS_SIZE;
    // The rock: the finished pixel image tilts as a rigid sprite (rotated
    // QUAD corners, NEAREST — chunky pixels and all). Phase-shifted by the
    // draw position so side-by-side heads (SWARM) never rock in unison, on
    // top of the per-head `time` offsets render_dialogue already passes.
    const rock =
      Math.sin(time * PORTRAIT_ROCK_W + x * 0.013 + y * 0.007) * PORTRAIT_ROCK_AMP;
    const h = sizePx / 2;
    const c = Math.cos(rock), s = Math.sin(rock);
    const ex = h * c, ey = h * s; // half-extent along the rotated x axis
    const fx = -h * s, fy = h * c; // half-extent along the rotated y axis
    vert(x - ex - fx, y - ey - fy, u0, v0, 1, 1, 1, 1);
    vert(x + ex - fx, y + ey - fy, u1, v0, 1, 1, 1, 1);
    vert(x + ex + fx, y + ey + fy, u1, v1, 1, 1, 1, 1);
    vert(x - ex - fx, y - ey - fy, u0, v0, 1, 1, 1, 1);
    vert(x + ex + fx, y + ey + fy, u1, v1, 1, 1, 1, 1);
    vert(x - ex + fx, y - ey + fy, u0, v1, 1, 1, 1, 1);
  }

  // Weapon lying on the ground: its 3D model top-down at GUN_ART texels,
  // baked ONCE per weaponIdx at angle 0 into the persistent pixel-sprite
  // cache, then NEAREST-upscaled to sizePx on a quad ROTATED in 2D by
  // `angle` (radians, screen convention: positive = clockwise) around its
  // centre — for the true top-down ortho camera the two are equivalent (see
  // gunSlotFor). World space (through the transform stack).
  function drawGunPickup(weaponIdx, x, y, angle, sizePx) {
    // renderGun falls back to the bar model for out-of-range indices; clamp
    // the same way so the cache stays bounded to the 4 real weapons.
    const wi = weaponIdx >= 0 && weaponIdx < 4 ? weaponIdx | 0 : 0;
    const slot = gunSlotFor(wi); // bakes on first use
    setTexture(portraitTex);
    if (vCount + 6 > MAX_VERTS) flush();
    const tx = (slot % portraitCols) * FX_TILE;
    const ty = Math.floor(slot / portraitCols) * FX_TILE;
    // v flipped: pass 2 renders bottom-up (see drawRobot)
    const u0 = tx / PORTRAIT_ATLAS_SIZE;
    const v0 = (ty + GUN_ART) / PORTRAIT_ATLAS_SIZE;
    const u1 = (tx + GUN_ART) / PORTRAIT_ATLAS_SIZE;
    const v1 = ty / PORTRAIT_ATLAS_SIZE;
    const h = sizePx / 2;
    const c = Math.cos(angle), s = Math.sin(angle);
    const ex = h * c, ey = h * s; // half-extent along the rotated x axis
    const fx = -h * s, fy = h * c; // half-extent along the rotated y axis
    vert(x - ex - fx, y - ey - fy, u0, v0, 1, 1, 1, 1);
    vert(x + ex - fx, y + ey - fy, u1, v0, 1, 1, 1, 1);
    vert(x + ex + fx, y + ey + fy, u1, v1, 1, 1, 1, 1);
    vert(x - ex - fx, y - ey - fy, u0, v0, 1, 1, 1, 1);
    vert(x + ex + fx, y + ey + fy, u1, v1, 1, 1, 1, 1);
    vert(x - ex + fx, y - ey + fy, u0, v1, 1, 1, 1, 1);
  }

  // Detached robot head on the floor (the KICK finisher's trophy): the head
  // + visor cubes face-up at HEAD_ART texels, baked ONCE per colorIdx into
  // the persistent pixel-sprite cache, then NEAREST-upscaled to sizePx on a
  // quad ROTATED in 2D by `angle` around its centre — the physics' live
  // spin glides at native resolution while the pixels stay chunky. World
  // space (through the transform stack).
  function drawHead(colorIdx, x, y, angle, sizePx) {
    const ci = ROBOT_COLORS[colorIdx | 0] ? colorIdx | 0 : 0;
    const slot = headSlotFor(ci); // bakes on first use
    setTexture(portraitTex);
    if (vCount + 6 > MAX_VERTS) flush();
    const tx = (slot % portraitCols) * FX_TILE;
    const ty = Math.floor(slot / portraitCols) * FX_TILE;
    // v flipped: pass 2 renders bottom-up (see drawRobot)
    const u0 = tx / PORTRAIT_ATLAS_SIZE;
    const v0 = (ty + HEAD_ART) / PORTRAIT_ATLAS_SIZE;
    const u1 = (tx + HEAD_ART) / PORTRAIT_ATLAS_SIZE;
    const v1 = ty / PORTRAIT_ATLAS_SIZE;
    const h = sizePx / 2;
    const c = Math.cos(angle), s = Math.sin(angle);
    const ex = h * c, ey = h * s; // half-extent along the rotated x axis
    const fx = -h * s, fy = h * c; // half-extent along the rotated y axis
    vert(x - ex - fx, y - ey - fy, u0, v0, 1, 1, 1, 1);
    vert(x + ex - fx, y + ey - fy, u1, v0, 1, 1, 1, 1);
    vert(x + ex + fx, y + ey + fy, u1, v1, 1, 1, 1, 1);
    vert(x - ex - fx, y - ey - fy, u0, v0, 1, 1, 1, 1);
    vert(x + ex + fx, y + ey + fy, u1, v1, 1, 1, 1, 1);
    vert(x - ex + fx, y - ey + fy, u0, v1, 1, 1, 1, 1);
  }

  /* ---- frame execution ---- */
  function frameRender(cmds, textArena) {
    // Perf (?perf): the `walk` span covers the opcode loop + batch building
    // (including the intermediate flushes it triggers); `sprites` is the
    // accumulated live robot/boss passes, `submit` the final upload + draw,
    // `postfx` the post pass. All nest inside the wasm side's `flush` span.
    let perfT0 = 0, perfD0 = 0, perfF0 = 0;
    if (PERF) {
      perfT0 = performance.now();
      perfD0 = PERF._draws;
      perfF0 = PERF._fbos;
      perfSpriteMs = 0;
      perfSpriteT0 = 0;
      perfRobotN = 0;
    }
    // Backing buffer = CSS size x devicePixelRatio (Graphics::sync_size, which
    // publishes the ratio as data-dpr); the stream's coordinates are CSS px.
    const pw = canvas.width, ph = canvas.height;
    const dpr = parseFloat(canvas.dataset.dpr) || 1;
    const w = pw / dpr, h = ph / dpr;
    frameW = w;
    frameH = h;
    framePW = pw;
    framePH = ph;
    // A POSTFX anywhere in the frame routes the whole frame through the
    // offscreen scene target (decided up front, before the first draw) —
    // EXCEPT kind 13 (TV STATIC), which needs nothing from the scene and is
    // drawn as a plain blended noise quad at the end of the frame instead.
    postfxActive = scanPostfx(cmds);
    let staticOverlay = 0;
    if (postfxActive && (postfx.kind | 0) === 13) {
      staticOverlay = postfx.t;
      postfxActive = false;
    }
    // FOLD the static into the batch shader (see FS) when its conditions
    // hold; otherwise it stays the end-of-frame quad. One random whole-texel
    // offset per frame either way (REPEAT wrapping), one texel per 6 px.
    grainT = 0;
    if (staticOverlay > 0) {
      const off = (typeof window !== "undefined" && window.__grainOffset) || null; // (tests: a fixed roll)
      grainU0 = off ? off[0] : Math.floor(Math.random() * STATIC_SIZE) / STATIC_SIZE;
      grainV0 = off ? off[1] : Math.floor(Math.random() * STATIC_SIZE) / STATIC_SIZE;
      const fold = GRAIN_FOLD && scanSawBackdrop
        && !(typeof window !== "undefined" && window.__grainFold === false);
      if (fold) {
        grainT = staticOverlay;
        staticOverlay = 0; // no quad
      }
    }
    if (postfxActive) ensureSceneTarget(pw, ph);
    batchFbo = postfxActive ? sceneFbo : null;
    batchW = w;
    batchH = h;
    batchVW = pw;
    batchVH = ph;
    // A group left open by a truncated stream must not leak into this frame.
    pix = null;
    pixDepth = 0;
    pixStack.length = 0;
    lastPix = null; // a PIX_BLIT never samples a previous frame's texels
    staticRec = null; // an unterminated static recording never leaks either
    bindBatchState();
    gl.uniform1i(loc.uTex, 0);
    if (grainT > 0) {
      const gs = 1 / (6 * STATIC_SIZE);
      gl.uniform4f(loc.uGrainK, gs, -gs, grainU0, grainV0 + ph * gs);
      gl.activeTexture(gl.TEXTURE2);
      gl.bindTexture(gl.TEXTURE_2D, staticTex);
      gl.activeTexture(gl.TEXTURE0);
    }
    setGrain(0);

    const texts = textArena.length ? textArena.split(TEXT_SEP) : [];
    m = [1, 0, 0, 1, 0, 0];
    stack.length = 0;
    boundTex = null;
    vCount = 0;
    robotUsed = 0;
    shogUsed = 0;

    let i = 0;
    const n = cmds.length;
    while (i < n) {
      const op = cmds[i++];
      switch (op) {
        case 0: { // CLEAR
          flush();
          gl.clearColor(cmds[i], cmds[i + 1], cmds[i + 2], 1.0);
          if (pix) {
            // Only the open group's region of the scratch texture.
            gl.enable(gl.SCISSOR_TEST);
            gl.scissor(0, 0, pix.tw, pix.th);
            gl.clear(gl.COLOR_BUFFER_BIT);
            gl.disable(gl.SCISSOR_TEST);
          } else {
            gl.clear(gl.COLOR_BUFFER_BIT);
          }
          i += 4;
          break;
        }
        case 1: // RECT
          solidRect(cmds[i], cmds[i + 1], cmds[i + 2], cmds[i + 3],
            cmds[i + 4], cmds[i + 5], cmds[i + 6], cmds[i + 7]);
          i += 8;
          break;
        case 2: // RECT_LINES
          rectLines(cmds[i], cmds[i + 1], cmds[i + 2], cmds[i + 3], cmds[i + 4],
            cmds[i + 5], cmds[i + 6], cmds[i + 7], cmds[i + 8]);
          i += 9;
          break;
        case 3: // CIRCLE
          circle(cmds[i], cmds[i + 1], cmds[i + 2],
            cmds[i + 3], cmds[i + 4], cmds[i + 5], cmds[i + 6]);
          i += 7;
          break;
        case 4: // LINE
          line(cmds[i], cmds[i + 1], cmds[i + 2], cmds[i + 3], cmds[i + 4],
            cmds[i + 5], cmds[i + 6], cmds[i + 7], cmds[i + 8]);
          i += 9;
          break;
        case 5: // ARC
          arcPie(cmds[i], cmds[i + 1], cmds[i + 2], cmds[i + 3], cmds[i + 4],
            cmds[i + 5], cmds[i + 6], cmds[i + 7], cmds[i + 8]);
          i += 9;
          break;
        case 6: { // TEXT
          const text = texts[cmds[i] | 0] ?? "";
          drawText(text, cmds[i + 1], cmds[i + 2], cmds[i + 3],
            cmds[i + 4], cmds[i + 5], cmds[i + 6], cmds[i + 7]);
          i += 8;
          break;
        }
        case 7: // SAVE
          tSave();
          break;
        case 8: // RESTORE
          tRestore();
          break;
        case 9: // TRANSLATE
          tTranslate(cmds[i], cmds[i + 1]);
          i += 2;
          break;
        case 10: // ROTATE
          tRotate(cmds[i]);
          i += 1;
          break;
        case 11: // ROBOT
          drawRobot(cmds, i);
          i += 18;
          break;
        case 12: // SCALE
          tScale(cmds[i], cmds[i + 1]);
          i += 2;
          break;
        case 13: // SHOGGOTH
          drawShoggoth(cmds[i], cmds[i + 1], cmds[i + 2], cmds[i + 3], cmds[i + 4],
            cmds[i + 5]);
          i += 6;
          break;
        case 14: // POSTFX (already picked up by the pre-scan)
          i += 5;
          break;
        case 15: // PIX_BEGIN
          pixBegin(cmds[i], cmds[i + 1], cmds[i + 2], cmds[i + 3] !== 0);
          i += 4;
          break;
        case 16: // PIX_END
          pixEnd(cmds[i], cmds[i + 1]);
          i += 2;
          break;
        case 17: // PORTRAIT
          drawPortrait(cmds[i], cmds[i + 1], cmds[i + 2], cmds[i + 3], cmds[i + 4], cmds[i + 5]);
          i += 6;
          break;
        case 18: // GUN_PICKUP
          drawGunPickup(cmds[i], cmds[i + 1], cmds[i + 2], cmds[i + 3], cmds[i + 4]);
          i += 5;
          break;
        case 19: // PIX_BLIT
          pixBlit(cmds[i], cmds[i + 1], cmds[i + 2], cmds[i + 3], cmds[i + 4], cmds[i + 5]);
          i += 6;
          break;
        case 20: // DRIVE (w h t glitch split px dim o0..o8)
          drawDrive(cmds[i], cmds[i + 1], cmds[i + 2], cmds[i + 3], cmds[i + 4],
            cmds[i + 5], cmds[i + 6], cmds, i + 7);
          i += 16;
          break;
        case 21: // STATIC_BEGIN (key)
          staticBegin(cmds[i]);
          i += 1;
          break;
        case 22: // STATIC_END
          staticEnd();
          break;
        case 23: // STATIC_REF (key)
          drawStatic(cmds[i]);
          i += 1;
          break;
        case 24: // BACKDROP (w h t px ex ey ew eh)
          drawBackdrop(cmds[i], cmds[i + 1], cmds[i + 2], cmds[i + 3],
            cmds[i + 4], cmds[i + 5], cmds[i + 6], cmds[i + 7]);
          i += 8;
          break;
        case 25: // HEAD
          drawHead(cmds[i], cmds[i + 1], cmds[i + 2], cmds[i + 3], cmds[i + 4]);
          i += 5;
          break;
        default:
          // Unknown opcode: the stream is corrupt; stop rather than
          // misinterpret the remaining floats.
          console.error("frameRender: unknown opcode", op, "at", i - 1);
          i = n;
          break;
      }
    }
    while (pixStack.length) pixEnd(0, 0); // unterminated groups: close them where they are
    const perfTSubmit = PERF ? performance.now() : 0;
    if (PERF) window.perfSpan("walk", perfT0, perfTSubmit - perfT0);
    flush();
    // TV STATIC (kind 13): one alpha-blended quad of the pre-rolled noise
    // sheet over the finished frame — one texel per 6 physical px, a fresh
    // random whole-texel offset each frame (REPEAT wrapping).
    if (staticOverlay > 0) {
      const savedM = m;
      m = [1, 0, 0, 1, 0, 0];
      const u0 = grainU0, v0 = grainV0;
      setTexture(staticTex);
      quad(
        0, 0, frameW, frameH,
        u0, v0, u0 + pw / 6 / STATIC_SIZE, v0 + ph / 6 / STATIC_SIZE,
        1, 1, 1, staticOverlay
      );
      flush();
      m = savedM;
    }
    if (grainT > 0) { grainT = 0; setGrain(0); }
    const perfTPost = PERF ? performance.now() : 0;
    if (PERF) window.perfSpan("submit", perfTSubmit, perfTPost - perfTSubmit);
    // The post passes work on the final pixels: physical resolution.
    const warpFrame = postfxActive && (postfx.kind | 0) === 10;
    if (warpFrame) runWarpPass(pw, ph);
    else if (postfxActive) runPostPass(pw, ph);
    if (!warpFrame) warpLive = false; // next warp frame starts from a clean accumulator
    if (PERF) {
      if (postfxActive) window.perfSpan("postfx", perfTPost, performance.now() - perfTPost);
      if (perfSpriteMs > 0) window.perfSpan("sprites", perfSpriteT0, perfSpriteMs);
      window.perfCount("cmds", cmds.length);
      window.perfCount("draws", PERF._draws - perfD0);
      window.perfCount("fbos", PERF._fbos - perfF0);
      window.perfCount("robots", perfRobotN);
    }
  }

  // `?gpuprobe`: the GPU knockout experiment (gpu-probe.js) — strips one
  // class of work at a time from the stream and reads the GPU cost off the
  // frame period. Off = the raw function, zero overhead.
  if (typeof location !== "undefined" && new URLSearchParams(location.search).has("gpuprobe")) {
    const dbg = gl.getExtension("WEBGL_debug_renderer_info");
    const gpuName = String(gl.getParameter(dbg ? dbg.UNMASKED_RENDERER_WEBGL : gl.RENDERER));
    return wrapGpuProbe(frameRender, canvas, OP_ARGS, gpuName,
      new URLSearchParams(location.search).get("gpuprobe"));
  }
  return frameRender;
}
