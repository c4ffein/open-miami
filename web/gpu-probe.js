/* GPU PROBE (`?gpuprobe`): which part of a frame does the GPU pay for?

   `?perf` times the CPU side only, and on a GPU-bound machine the CPU side is
   ~1 ms of a 30 ms frame. But a GPU-bound frame loop is its own GPU timer:
   the browser hands out the next animation frame at the pace the GPU finishes
   the previous one, so the mean frame PERIOD tracks the GPU frame time
   (floored at the display's vsync). The probe exploits that — a knockout
   experiment, no timer-query extension needed:

     it wraps `frameRender`, and for ~2 s per configuration REWRITES the
     command stream before the renderer sees it, stripping one class of work
     (the backdrop, the TV static, the cached floor + walls, big rects, the
     robots, text, ... down to "clear only"), and measures the frame period.
     period(baseline) - period(knockout) = what that class costs the GPU.

   Stand still in a representative spot and let it run (2 rounds, baseline
   re-measured between them so thermal drift shows). The result table lands
   on screen, in the console, on the clipboard (best effort) and in
   `window.__gpuProbe`. The game keeps simulating; only what is DRAWN changes.

   The stream is filtered with the renderer's own OP_ARGS arity table, so the
   probe needs no knowledge of the ops it does not touch. */

import { OP } from "./ops.js";

const {
  CLEAR: OP_CLEAR,
  RECT: OP_RECT,
  TEXT: OP_TEXT,
  SAVE: OP_SAVE,
  RESTORE: OP_RESTORE,
  SCALE: OP_SCALE,
  ROBOT: OP_ROBOT,
  SHOGGOTH: OP_SHOGGOTH,
  POSTFX: OP_POSTFX,
  PIX_BEGIN: OP_PIX_BEGIN,
  PIX_END: OP_PIX_END,
  STATIC_BEGIN: OP_STATIC_BEGIN,
  STATIC_END: OP_STATIC_END,
  STATIC_REF: OP_STATIC_REF,
  BACKDROP: OP_BACKDROP,
} = OP;
const POSTFX_TV_STATIC = 13;

const SETTLE_MS = (typeof window !== "undefined" && window.__gpuProbeFast) ? 50 : 500;   // ignored after each switch (pipeline drains, caches settle)
const MEASURE_MS = (typeof window !== "undefined" && window.__gpuProbeFast) ? 150 : 1700; // measured per configuration per round
const ROUNDS = 2;
const BIG_RECT = 0.25;   // a rect covering >= this share of the screen is "big"

// name, what it strips. `drop(op, cmds, i, ctx)` -> true = strip this op.
const CONFIGS = [
  { name: "baseline", note: "the full frame", drop: () => false },
  { name: "no backdrop", note: "op BACKDROP (the neon void, 1 opaque full-screen quad)",
    drop: (op) => op === OP_BACKDROP },
  { name: "no tv static", note: "POSTFX 13 (1 blended full-screen quad)",
    drop: (op, c, i) => op === OP_POSTFX && (c[i] | 0) === POSTFX_TV_STATIC },
  { name: "no postfx", note: "every POSTFX (scene FBO + post pass, when one is on)",
    drop: (op) => op === OP_POSTFX },
  { name: "no static geo", note: "STATIC_REF (the cached floor tiles + walls)",
    drop: (op) => op === OP_STATIC_REF },
  { name: "no big rects", note: "RECTs covering >= 25% of the screen (fades, washes, floor fills)",
    drop: (op, c, i, ctx) => op === OP_RECT && ctx.bigRect },
  { name: "no robots", note: "ROBOT + SHOGGOTH (sprite passes + their quads)",
    drop: (op) => op === OP_ROBOT || op === OP_SHOGGOTH },
  { name: "no text", note: "TEXT", drop: (op) => op === OP_TEXT },
  { name: "clear only", note: "nothing but the CLEAR: the floor cost of presenting this canvas",
    drop: (op) => op !== OP_CLEAR },
  // STRESS rows: a knockout cannot show more than (baseline - vsync) — once a
  // config reaches 16.7 ms the display is the limit, not the GPU. Drawing a
  // layer TWICE has no such ceiling: period(+1) - period(baseline) = what one
  // more of it costs, i.e. what removing / shrinking it is really worth.
  { name: "+1 backdrop", note: "BACKDROP drawn twice: the marginal cost of that layer",
    drop: () => false, dup: (op) => op === OP_BACKDROP },
  { name: "+1 static geo", note: "STATIC_REF drawn twice: floor + walls, overdraw included",
    drop: () => false, dup: (op) => op === OP_STATIC_REF },
  { name: "+1 blend rect", note: "one extra invisible blended full-screen RECT (~ the TV static quad)",
    drop: () => false, extraRect: 1 },
  { name: "+3 blend rect", note: "three of them: is the cost linear in full-screen layers?",
    drop: () => false, extraRect: 3 },
  // The FIXED cost: "clear only" hides under the vsync floor, so load it with
  // known layers until it surfaces. With c = the per-rect cost (the two rows'
  // difference / 4), period(clear + N) - N*c = what presenting this canvas
  // costs before the game draws anything (browser compositor + OS scaler
  // share the GPU with us).
  { name: "clear +6 rect", note: "CLEAR + 6 blended full-screen rects", drop: (op) => op !== OP_CLEAR, extraRect: 6 },
  { name: "clear +10 rect", note: "CLEAR + 10: slope = per-layer cost, intercept = the fixed cost",
    drop: (op) => op !== OP_CLEAR, extraRect: 10 },
];

/* `?gpuprobe=fixed` — the QUICK probe (~10 s): only the two numbers that
   describe a machine + context, not the game's layers: the FIXED cost of
   presenting this canvas and the cost PER full-screen layer. Draws nothing
   but the CLEAR + N invisible blended full-screen rects; N doubles until the
   GPU is clearly the limit (period >= 24 ms — a fast GPU needs hundreds),
   then N and 2N are measured: slope = per layer, intercept = fixed. Made for
   A/B-ing context flags across machines (`&ctx=alpha` vs `&ctx=opaque`). The
   panel stays HIDDEN while measuring (an element over the canvas can change
   how the browser presents it); progress is in the tab title. */
function wrapQuickProbe(frameRender, canvas, gpuName, filter, panel, mean) {
  const RAMP_SETTLE = 300, RAMP_MEASURE = 700, SETTLE = 500, MEASURE = 2000, N_MAX = 4096, GPU_BOUND_MS = 24;
  const clearPlus = (n) => ({ drop: (op) => op !== OP_CLEAR, extraRect: n });
  let phase = "wait", n = 8, cfg = null, t0 = 0, lastT = 0, periods = [], plan = [], got = {}, done = false;
  const startAt = performance.now() + 1500;
  const title = document.title;
  function begin(now, c, label) { cfg = c; t0 = now; periods = []; document.title = `PROBE ${label}`; }
  function finish(tooFast) {
    done = true; document.title = title; panel.style.display = "";
    const pN = mean(got[n] || []), p2N = mean(got[2 * n] || []);
    const perLayer = tooFast ? null : (p2N - pN) / n, fixed = tooFast ? null : pN - n * perLayer;
    const result = {
      mode: "fixed", gpu: gpuName, dpr: window.devicePixelRatio, url: location.search,
      canvas: { w: canvas.width, h: canvas.height, cssW: canvas.clientWidth, cssH: canvas.clientHeight },
      contextAlpha: !!(canvas.getContext("webgl") || { getContextAttributes: () => ({}) }).getContextAttributes().alpha,
      rects: tooFast ? null : [n, 2 * n], periodsMs: tooFast ? null : [+pN.toFixed(2), +p2N.toFixed(2)],
      perLayerMs: tooFast ? null : +perLayer.toFixed(3), fixedMs: tooFast ? null : +fixed.toFixed(2),
    };
    window.__gpuProbe = result;
    const lines = tooFast
      ? [`GPU PROBE (quick) — ${gpuName}`, `${N_MAX} full-screen layers still ran at the display rate:`,
         "this GPU is not the limit at this canvas size — nothing to measure, nothing to fix."]
      : [`GPU PROBE (quick) — ${gpuName}`,
         `canvas ${canvas.width}x${canvas.height} (dpr ${window.devicePixelRatio}) alpha:${result.contextAlpha} ${location.search}`,
         `FIXED cost of presenting the canvas: ${result.fixedMs} ms`,
         `per full-screen layer:               ${result.perLayerMs} ms`,
         `(clear + ${n} rects = ${result.periodsMs[0]} ms, clear + ${2 * n} rects = ${result.periodsMs[1]} ms)`];
    panel.textContent = lines.join("\n");
    console.log(lines.join("\n")); console.log(JSON.stringify(result));
    try { navigator.clipboard.writeText(lines.join("\n")).catch(() => {}); } catch (e) { /* no clipboard */ }
  }
  return function quickProbedFrameRender(cmds, textArena) {
    const now = performance.now();
    if (done || now < startAt) { lastT = now; return frameRender(cmds, textArena); }
    if (phase === "wait") { phase = "ramp"; panel.style.display = "none"; begin(now, clearPlus(n), `ramp ${n}`); }
    const dt = now - lastT; lastT = now;
    const settle = phase === "ramp" ? RAMP_SETTLE : SETTLE, measure = phase === "ramp" ? RAMP_MEASURE : MEASURE;
    if (now - t0 >= settle && dt < 250) periods.push(dt);
    if (now - t0 >= settle + measure) {
      if (phase === "ramp") {
        if (mean(periods) >= GPU_BOUND_MS || n >= N_MAX) {
          if (mean(periods) < GPU_BOUND_MS) { finish(true); return frameRender(cmds, textArena); }
          phase = "measure"; plan = [n, 2 * n, n, 2 * n];
        } else { n *= 2; }
        if (phase === "ramp") begin(now, clearPlus(n), `ramp ${n}`);
      } else {
        (got[cfg.extraRect] = got[cfg.extraRect] || []).push(...periods);
      }
      if (phase === "measure") {
        if (!plan.length) { finish(false); return frameRender(cmds, textArena); }
        const k = plan.shift(); begin(now, clearPlus(k), `measure ${k}`);
      }
    }
    return frameRender(filter(cmds, cfg, false), textArena);
  };
}

/* `?gpuprobe=curve` — period vs load (~35 s): CLEAR + N invisible full-screen
   rects for a ladder of N, with the local slope per rung. A GPU at one speed
   gives a straight line above the vsync floor (bare-clear cost + N * per-layer
   cost); rungs ON the floor say nothing. This mode is what exposed the
   probe's own OBSERVER EFFECT: on a 2018 MacBook Air the ladder is a clean
   1.06 ms / layer from a ~4.5 ms clear with the panel hidden, where the full
   probe — its result panel visible over the canvas at the time — had read
   1.8 ms / layer from 9.6 ms. Every mode now hides the panel while measuring;
   figures taken with an element over the canvas are not comparable. */
/* `?gpuprobe=headroom` — the same ladder on top of THE REAL GAME FRAME
   (nothing stripped): how many extra full-screen layers the frame takes
   before it leaves the vsync floor = the true GPU headroom of the scene you
   are standing in, in layers and (via the slope past the knee) in ms. */
function wrapCurveProbe(frameRender, canvas, gpuName, filter, panel, mean, headroom) {
  const LADDER = headroom ? [0, 1, 2, 3, 4, 5, 6, 8, 10, 12, 16, 20, 24]
    : [0, 2, 4, 6, 8, 10, 12, 16, 20, 24, 32, 48, 64];
  const SETTLE = 600, MEASURE = 2000;
  const startAt = performance.now() + 1500, title = document.title;
  let k = -1, t0 = 0, lastT = 0, periods = [], rows = [], done = false, cfg = null;
  function finish() {
    done = true; document.title = title; panel.style.display = "";
    const mpx = canvas.width * canvas.height / 1e6;
    const lines = [`GPU PROBE (${headroom ? "headroom: the game frame + N layers" : "curve: clear + N layers"}) — ${gpuName}`,
      `canvas ${canvas.width}x${canvas.height} = ${mpx.toFixed(2)} Mpx (dpr ${window.devicePixelRatio}) ${location.search}`,
      "", " rects   period   ms per extra layer (since the previous rung)   [per Mpx]"];
    rows.forEach((r, i) => {
      const prev = rows[i - 1];
      const slope = prev ? (r.period - prev.period) / (r.n - prev.n) : null;
      r.slopeMs = slope == null ? null : +slope.toFixed(3);
      lines.push(`${String(r.n).padStart(6)}${r.period.toFixed(2).padStart(9)}` +
        (slope == null ? "" : `${slope.toFixed(2).padStart(10)}${" ".repeat(40)}[${(slope / mpx).toFixed(3)}]`));
    });
    lines.push("", "(16.7 = the vsync floor: rungs sitting on it say nothing about the GPU)");
    // Past the knee the curve is a line: fit the rungs clearly above the
    // floor, and read off where that line crosses N = 0.
    const floorMs = rows[0].period, up = rows.filter((r) => r.period > floorMs * 1.15);
    if (up.length >= 2) {
      const a = up[0], b = up[up.length - 1], slope = (b.period - a.period) / (b.n - a.n), at0 = a.period - a.n * slope;
      lines.push(`fit past the knee: ${slope.toFixed(2)} ms per layer; the ${headroom ? "game frame" : "bare clear"} itself = ${at0.toFixed(1)} ms of GPU time` +
        (at0 < floorMs ? ` -> ${(floorMs - at0).toFixed(1)} ms (${((floorMs - at0) / slope).toFixed(1)} layers) of headroom under the ${floorMs.toFixed(1)} ms floor` : ""));
    }
    window.__gpuProbe = { mode: headroom ? "headroom" : "curve", gpu: gpuName, canvas: { w: canvas.width, h: canvas.height }, url: location.search, rows };
    panel.textContent = lines.join("\n");
    console.log(lines.join("\n"));
    try { navigator.clipboard.writeText(lines.join("\n")).catch(() => {}); } catch (e) { /* no clipboard */ }
  }
  return function curveProbedFrameRender(cmds, textArena) {
    const now = performance.now();
    if (done || now < startAt) { lastT = now; return frameRender(cmds, textArena); }
    const dt = now - lastT; lastT = now;
    if (k >= 0 && now - t0 >= SETTLE && dt < 250) periods.push(dt);
    if (k < 0 || now - t0 >= SETTLE + MEASURE) {
      if (k >= 0) rows.push({ n: LADDER[k], period: +mean(periods).toFixed(3) });
      k++;
      if (k >= LADDER.length) { finish(); return frameRender(cmds, textArena); }
      panel.style.display = "none";
      cfg = { drop: headroom ? () => false : (op) => op !== OP_CLEAR, extraRect: LADDER[k] };
      t0 = now; periods = []; document.title = `PROBE curve ${LADDER[k]}`;
    }
    return frameRender(filter(cmds, cfg, false), textArena);
  };
}

export function wrapGpuProbe(frameRender, canvas, opArgs, gpuName, mode) {
  const order = [];
  for (let r = 0; r < ROUNDS; r++) {
    for (let k = 0; k < CONFIGS.length; k++) order.push(k);
    order.push(0); // baseline again: drift check
  }
  const samples = CONFIGS.map(() => []);      // frame periods (ms) per config
  const baselineRuns = [];                    // mean period of each baseline pass, in order
  let step = -1, stepStart = 0, lastT = 0, done = false, runPeriods = [];
  let bigRectLayers = 0, bigRectFrames = 0;   // baseline only: screen-covering rect overdraw
  let out = new Float32Array(1 << 16);

  const pageTitle = document.title;
  const panel = document.createElement("pre");
  panel.style.cssText = "position:fixed;left:8px;top:8px;margin:0;padding:8px 10px;z-index:99;" +
    "font:12px/1.35 monospace;color:#9ff;background:rgba(0,0,0,.82);border:1px solid #299;" +
    "pointer-events:none;white-space:pre;max-height:95vh;overflow:hidden";
  document.body.appendChild(panel);
  panel.textContent = "GPU PROBE — get to a representative spot and STAND STILL; starting in 3 s…";
  const startAt = performance.now() + 3000;

  const mean = (a) => a.reduce((s, v) => s + v, 0) / (a.length || 1);
  const median = (a) => { const s = a.slice().sort((x, y) => x - y); return s.length ? s[s.length >> 1] : 0; };

  function filter(cmds, cfg, measureRects) {
    const need = cmds.length * 2 + 64 + (cfg.extraRect || 0) * 9;
    if (out.length < need) out = new Float32Array(need * 2);
    const frameArea = Math.max(1, canvas.clientWidth * canvas.clientHeight);
    const ctx = { bigRect: false };
    const scales = []; let area = 1, groups = 0, o = 0, i = 0, layers = 0, inStatic = false;
    const n = cmds.length;
    while (i < n) {
      const op = cmds[i], argc = opArgs[op];
      if (argc === undefined) break; // corrupt stream: hand the rest over untouched
      const a = i + 1;
      // just enough transform tracking to know a rect's SCREEN area
      if (op === OP_SAVE) scales.push(area);
      else if (op === OP_RESTORE) { if (scales.length) area = scales.pop(); }
      else if (op === OP_SCALE) area *= Math.abs(cmds[a] * cmds[a + 1]);
      else if (op === OP_PIX_BEGIN) groups++;
      else if (op === OP_PIX_END) groups = Math.max(0, groups - 1);
      ctx.bigRect = false;
      if (op === OP_RECT && groups === 0) {
        const share = Math.abs(cmds[a + 2] * cmds[a + 3]) * area / frameArea;
        ctx.bigRect = share >= BIG_RECT;
        if (measureRects && ctx.bigRect) layers += Math.min(1, share);
      }
      // A STATIC_BEGIN..END section is recorded ONCE per floor (the wasm then
      // only ever sends STATIC_REF): it always goes through whole, or a
      // knockout that happened to cover the record frame would lose the
      // floor for good.
      if (op === OP_STATIC_BEGIN) inStatic = true;
      const keep = inStatic || !cfg.drop(op, cmds, a, ctx);
      if (op === OP_STATIC_END) inStatic = false;
      if (keep) { out.set(cmds.subarray(i, a + argc), o); o += 1 + argc; }
      if (keep && !inStatic && cfg.dup && cfg.dup(op)) { out.set(cmds.subarray(i, a + argc), o); o += 1 + argc; }
      i = a + argc;
    }
    if (i < n) { out.set(cmds.subarray(i), o); o += n - i; }
    // (the stream ends with its transform stack unwound: screen space, CSS px)
    for (let k = 0; k < (cfg.extraRect || 0); k++) {
      out.set([OP_RECT, 0, 0, canvas.clientWidth, canvas.clientHeight, 0, 0, 0, 1 / 255], o);
      o += 9;
    }
    if (measureRects) { bigRectLayers += layers; bigRectFrames++; }
    return out.subarray(0, o);
  }

  function report() {
    const base = mean(baselineRuns);
    const rows = CONFIGS.map((c, k) => {
      const p = mean(samples[k]);
      return { config: c.name, periodMs: +p.toFixed(2), medianMs: +median(samples[k]).toFixed(2),
               fps: +(1000 / p).toFixed(1), savesMs: +(base - p).toFixed(2), frames: samples[k].length, note: c.note };
    });
    const result = {
      gpu: gpuName, dpr: window.devicePixelRatio,
      canvas: { w: canvas.width, h: canvas.height, cssW: canvas.clientWidth, cssH: canvas.clientHeight },
      url: location.search,
      baselineRunsMs: baselineRuns.map((v) => +v.toFixed(2)),
      bigRectLayersPerFrame: +(bigRectLayers / Math.max(1, bigRectFrames)).toFixed(2),
      rows,
    };
    const row = (n) => rows.find((r) => r.config === n);
    const c6 = row("clear +6 rect").periodMs, c10 = row("clear +10 rect").periodMs;
    result.perLayerMs = +((c10 - c6) / 4).toFixed(2);
    result.fixedMs = +(c6 - 6 * result.perLayerMs).toFixed(2);
    window.__gpuProbe = result;
    panel.style.display = ""; document.title = pageTitle;
    const pad = (s, n) => String(s).padEnd(n), lpad = (s, n) => String(s).padStart(n);
    const lines = [
      `GPU PROBE — ${gpuName}`,
      `canvas ${canvas.width}x${canvas.height} (dpr ${window.devicePixelRatio}) ${location.search}`,
      `baseline passes (ms, in order — drift check): ${result.baselineRunsMs.join("  ")}`,
      `big-rect overdraw in the baseline: ${result.bigRectLayersPerFrame} screens/frame`,
      "",
      `per full-screen layer: ${result.perLayerMs} ms · FIXED cost of presenting the canvas: ${result.fixedMs} ms` +
        ` (from the "clear +N" rows; budget = 16.7)`,
      `(saves < 0 on the "+N" stress rows = what the extra layer COSTS)`,
      `${pad("config", 15)}${lpad("period", 8)}${lpad("median", 8)}${lpad("fps", 7)}${lpad("saves", 8)}`,
      ...rows.map((r) => `${pad(r.config, 15)}${lpad(r.periodMs.toFixed(1), 8)}${lpad(r.medianMs.toFixed(1), 8)}` +
        `${lpad(r.fps.toFixed(0), 7)}${lpad(r.savesMs.toFixed(1), 8)}   ${r.note}`),
      "",
      "(a period near 16.7 = vsync floor: the GPU had headroom in that config)",
      "copied to the clipboard (best effort) — also window.__gpuProbe / the console",
    ];
    panel.textContent = lines.join("\n");
    const json = JSON.stringify(result);
    console.log(lines.join("\n")); console.log(json);
    try { navigator.clipboard.writeText(json).catch(() => {}); } catch (e) { /* no clipboard */ }
  }

  if (mode === "fixed") return wrapQuickProbe(frameRender, canvas, gpuName, filter, panel, mean);
  if (mode === "curve") return wrapCurveProbe(frameRender, canvas, gpuName, filter, panel, mean, false);
  if (mode === "headroom") return wrapCurveProbe(frameRender, canvas, gpuName, filter, panel, mean, true);
  return function probedFrameRender(cmds, textArena) {
    const now = performance.now();
    if (done || now < startAt) { lastT = now; return frameRender(cmds, textArena); }
    if (step < 0 || now - stepStart >= SETTLE_MS + MEASURE_MS) {
      if (step >= 0 && order[step] === 0) baselineRuns.push(mean(runPeriods));
      step++; stepStart = now; runPeriods = [];
      if (step >= order.length) { done = true; report(); return frameRender(cmds, textArena); }
      // The panel is HIDDEN while measuring: an HTML element over the canvas
      // changes how the browser presents it (measured on a 2018 MacBook Air:
      // a visible panel doubled the bare-clear cost, ~4.5 -> ~9.6 ms, and
      // inflated every layer 1.06 -> 1.8 ms). Progress goes to the tab title.
      panel.style.display = "none";
      document.title = `PROBE ${step + 1}/${order.length} ${CONFIGS[order[step]].name}`;
    } else if (now - stepStart >= SETTLE_MS && lastT > 0) {
      const dt = now - lastT;
      if (dt < 250) { samples[order[step]].push(dt); runPeriods.push(dt); } // (tab-switch stalls excluded)
    }
    lastT = now;
    const k = order[step];
    return frameRender(filter(cmds, CONFIGS[k], k === 0), textArena);
  };
}
