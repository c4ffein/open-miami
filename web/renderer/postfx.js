/* =========================================================================
   POSTFX (opcode 14) — the offscreen scene target, the single-pass post
   shader (kinds 0-9, 11, 12) and the WARP TRAILS feedback accumulator (kind
   10): one of renderer.js's self-contained subsystems, as a factory. (Kind
   13 TV STATIC is NOT here: it is a blended quad / a fold of the batch
   shader, so it lives with the batch in renderer.js.)

   The kind table is documented on `Graphics::postfx` (src/graphics.rs) —
   MIRRORED, keep in sync. Pixel test: tests/e2e/render/postfx-kinds.js.

   From the core it takes the shared full-screen quad buffer and `handBack()`
   — after a pass that drew to the canvas, the core re-installs the canvas as
   the batch target and re-binds the batch state.
   ========================================================================= */

import { OP, OP_ARGS } from "../ops.js";
import { POST_VS, POST_FS, WARP_FS } from "./shaders.js";

const OP_POSTFX = OP.POSTFX;

export function createPostfx({ gl, compile, makeTexture, loc, quadVbo, handBack }) {
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

  // The POSTFX request of the current frame (kind, t, r, g, b) or null.
  const postfx = { kind: 0, t: 0, r: 0, g: 0, b: 0 };

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
    gl.bindBuffer(gl.ARRAY_BUFFER, quadVbo);
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
    handBack(); // the canvas is the batch target again
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
    gl.bindBuffer(gl.ARRAY_BUFFER, quadVbo);
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
    handBack(); // the canvas is the batch target again
  }

  // The end of a frame: present the scene target through the post shader
  // (or the warp pass), when the frame was routed through it.
  function present(active, w, h) {
    const warpFrame = active && (postfx.kind | 0) === 10;
    if (warpFrame) runWarpPass(w, h);
    else if (active) runPostPass(w, h);
    if (!warpFrame) warpLive = false; // next warp frame starts from a clean accumulator
  }

  return {
    request: postfx,
    scan: scanPostfx,
    sawBackdrop: () => scanSawBackdrop,
    ensureSceneTarget,
    sceneFbo,
    present,
  };
}
