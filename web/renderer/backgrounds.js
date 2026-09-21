/* =========================================================================
   BACKGROUNDS — DRIVE (opcode 20, the synthwave drive) and BACKDROP (opcode
   24, the neon-wave void): two of renderer.js's self-contained subsystems, as
   a factory. Same economics for both (the perf rule in CLAUDE.md): the scene
   is COMPUTED at ART RESOLUTION, one fragment per art pixel, into a tiny
   NEAREST target, then drawn as ONE upscaled quad through the batch.

   The factory owns its programs + targets; from the batch core it takes
   `flush` / `bindBatchState` / `setTexture` / `quad`, the shared full-screen
   quad buffer, and `batchView()` — the live state drawBackdrop's occlusion
   path reads (identity transform? open pixel group? target size).

   MIRRORED in src/drive.rs (PINNED by its `the_js_mirror_matches`, which
   parses THIS file): the scene geometry literals of `drawDrive` and
   `driveHash` = `hash01`. Pixel test: tests/e2e/render/drive-backdrop.js
   (+ backdrop-clip.js on live frames).
   ========================================================================= */

import { DRIVE_VS, DRIVE_FS, BACKDROP_FS } from "./shaders.js";

export function createBackgrounds({
  gl, compile, loc, quadVbo, flush, bindBatchState, setTexture, quad, batchView,
}) {
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
    gl.bindBuffer(gl.ARRAY_BUFFER, quadVbo);
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
    gl.bindBuffer(gl.ARRAY_BUFFER, quadVbo);
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
    const view = batchView(); // the core's live state: transform, open group, target size
    gl.disable(gl.BLEND);
    if (ew > 0 && eh > 0 && !BACKDROP_FULL && view.identity && !view.inGroup) {
      // OCCLUSION: (ex, ey, ew, eh) is a rect that opaque content drawn later
      // (the floor) is guaranteed to cover — src/backdrop_clip.rs. Draw the
      // void only AROUND it: the SAME full-screen quad four times under a
      // SCISSOR (above / below / left / right of the rect, whole physical
      // pixels, the rect rounded INWARD). Same triangles = bit-identical
      // texel choice for every surviving pixel (re-cut strips interpolate
      // their own UVs and flip NEAREST at texel boundaries —
      // tests/e2e/render/backdrop-clip.js caught exactly that), and fragments the
      // floor would paint over are never shaded at all.
      const batchVW = view.vw, batchVH = view.vh;
      const sx = batchVW / view.w, sy = batchVH / view.h; // CSS px -> physical
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

  return { drawDrive, drawBackdrop };
}
