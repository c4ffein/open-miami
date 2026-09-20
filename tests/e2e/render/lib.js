// Shared by the RENDERER-ONLY pixel tests (postfx-kinds.js, text-glyphs.js,
// drive-backdrop.js): they drive web/renderer.js directly through the
// /render-tests harness page (`window.__rt`, no game, no wasm) with hand-built
// command streams, and assert PROPERTIES of the pixels — no golden images
// (SwiftShader's exact bytes are not a contract; what each subsystem promises
// is).
//
// Everything that makes a frame non-reproducible is pinned before the
// renderer loads: `performance.now` (the post shaders' clock) is a settable
// fake, and `Math.random` (the pre-rolled static sheet) is a seeded PRNG. So
// "the same stream twice = the same pixels" is itself a testable property —
// and the per-case hashes each script prints (`FP <case> <hash>`) can be
// diffed across a refactor of the renderer: identical = pixel-identical.
const { chromium } = require('playwright');
const { OP } = require('../ops'); // the ONE opcode table (web/ops.js)

const ARGS = ['--no-sandbox', '--disable-setuid-sandbox', '--enable-unsafe-swiftshader',
  '--use-gl=angle', '--use-angle=swiftshader', '--disable-dev-shm-usage'];

// Stream builders (names + arities = web/ops.js).
const ops = {
  CLEAR: (r, g, b) => [OP.CLEAR, r, g, b, 1],
  RECT: (x, y, w, h, r, g, b, a = 1) => [OP.RECT, x, y, w, h, r, g, b, a],
  LINE: (x1, y1, x2, y2, t, r, g, b, a = 1) => [OP.LINE, x1, y1, x2, y2, t, r, g, b, a],
  CIRCLE: (x, y, rad, r, g, b, a = 1) => [OP.CIRCLE, x, y, rad, r, g, b, a],
  TEXT: (idx, x, y, size, r, g, b, a = 1) => [OP.TEXT, idx, x, y, size, r, g, b, a],
  SAVE: () => [OP.SAVE],
  RESTORE: () => [OP.RESTORE],
  TRANSLATE: (x, y) => [OP.TRANSLATE, x, y],
  ROTATE: (a) => [OP.ROTATE, a],
  POSTFX: (kind, t, r = 0, g = 0, b = 0) => [OP.POSTFX, kind, t, r, g, b],
  DRIVE: (w, h, t, glitch, split, px, dim, offs = [0, 0, 0, 0, 0, 0, 0, 0, 0]) =>
    [OP.DRIVE, w, h, t, glitch, split, px, dim, ...offs],
  BACKDROP: (w, h, t, px, ex = 0, ey = 0, ew = 0, eh = 0) => [OP.BACKDROP, w, h, t, px, ex, ey, ew, eh],
};

// Runs IN THE PAGE (serialized by addInitScript): the fakes + the pixel tools.
function pageSide(seed) {
  window.__fakeT = 1000;
  performance.now = () => window.__fakeT;
  let s = seed >>> 0;
  Math.random = () => { // mulberry32
    s = (s + 0x6D2B79F5) >>> 0;
    let t = s;
    t = Math.imul(t ^ (t >>> 15), t | 1);
    t ^= t + Math.imul(t ^ (t >>> 7), t | 61);
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  };
  window.__grainOffset = [0, 0]; // renderer.js: a fixed TV-static roll
  window.__shots = new Map();
  const c2 = document.createElement('canvas');
  const g2 = c2.getContext('2d', { willReadFrequently: true });
  // Render one stream at fake time `ms`, keep its pixels under `name`.
  window.__shot = (name, cmds, text, ms, w, h) => {
    const { frameRender, canvas } = window.__rt;
    if (canvas.width !== w || canvas.height !== h) {
      canvas.width = w; canvas.height = h; canvas.dataset.dpr = '1';
    }
    window.__fakeT = ms;
    frameRender(new Float32Array(cmds), text || '');
    c2.width = w; c2.height = h;
    g2.clearRect(0, 0, w, h);
    g2.drawImage(canvas, 0, 0); // same task as the draw: the buffer is intact
    const px = g2.getImageData(0, 0, w, h).data;
    window.__shots.set(name, { w, h, px });
    let hash = 0x811c9dc5;
    for (let i = 0; i < px.length; i++) hash = Math.imul(hash ^ px[i], 0x01000193) >>> 0;
    const gl = canvas.getContext('webgl');
    return { hash: hash.toString(16).padStart(8, '0'), glError: gl.getError() };
  };
  // Compare two shots (optionally inside a rect): how many pixels differ by
  // more than `tol` on any channel, the worst channel delta, the mean |delta|.
  window.__diff = (a, b, tol = 0, rect = null) => {
    const A = window.__shots.get(a), B = window.__shots.get(b);
    const [x0, y0, x1, y1] = rect || [0, 0, A.w, A.h];
    let n = 0, max = 0, sum = 0, count = 0;
    for (let y = y0; y < y1; y++) for (let x = x0; x < x1; x++) {
      const i = (y * A.w + x) * 4;
      let d = 0;
      for (let k = 0; k < 3; k++) { const e = Math.abs(A.px[i + k] - B.px[i + k]); if (e > d) d = e; sum += e; }
      if (d > tol) n++;
      if (d > max) max = d;
      count++;
    }
    return { differing: n, share: n / count, max, mean: sum / (count * 3) };
  };
  // Stats of one shot inside a rect: mean rgb, distinct colours, the bounding
  // box of the pixels whose brightest channel exceeds `lit`.
  window.__stats = (name, rect = null, lit = 40) => {
    const S = window.__shots.get(name);
    const [x0, y0, x1, y1] = rect || [0, 0, S.w, S.h];
    const mean = [0, 0, 0], colours = new Set();
    let bx0 = 1e9, by0 = 1e9, bx1 = -1, by1 = -1, litN = 0, count = 0;
    for (let y = y0; y < y1; y++) for (let x = x0; x < x1; x++) {
      const i = (y * S.w + x) * 4, r = S.px[i], g = S.px[i + 1], b = S.px[i + 2];
      mean[0] += r; mean[1] += g; mean[2] += b;
      colours.add((r << 16) | (g << 8) | b);
      if (Math.max(r, g, b) > lit) {
        litN++;
        if (x < bx0) bx0 = x; if (x > bx1) bx1 = x; if (y < by0) by0 = y; if (y > by1) by1 = y;
      }
      count++;
    }
    return { mean: mean.map((v) => v / count), colours: colours.size, lit: litN,
      bbox: litN ? [bx0, by0, bx1, by1] : null };
  };
  window.__pixel = (name, x, y) => {
    const S = window.__shots.get(name), i = (y * S.w + x) * 4;
    return [S.px[i], S.px[i + 1], S.px[i + 2]];
  };
  // Is the shot constant over every aligned `cell` x `cell` block (an
  // art-resolution image upscaled NEAREST)? Returns the share of blocks that
  // are NOT uniform.
  window.__blockiness = (name, cell) => {
    const S = window.__shots.get(name);
    let bad = 0, total = 0;
    for (let by = 0; by + cell <= S.h; by += cell) for (let bx = 0; bx + cell <= S.w; bx += cell) {
      const i0 = (by * S.w + bx) * 4;
      let uniform = true;
      for (let y = 0; y < cell && uniform; y++) for (let x = 0; x < cell; x++) {
        const i = ((by + y) * S.w + bx + x) * 4;
        if (S.px[i] !== S.px[i0] || S.px[i + 1] !== S.px[i0 + 1] || S.px[i + 2] !== S.px[i0 + 2]) { uniform = false; break; }
      }
      if (!uniform) bad++;
      total++;
    }
    return bad / total;
  };
}

async function openHarness(base, { seed = 1 } = {}) {
  const browser = await chromium.launch({ args: ARGS });
  const page = await browser.newPage({ viewport: { width: 800, height: 600 }, deviceScaleFactor: 1 });
  await page.addInitScript(pageSide, seed);
  const errors = [];
  page.on('console', (m) => { if (m.type() === 'error') errors.push(m.text()); });
  page.on('pageerror', (e) => errors.push(String(e)));
  await page.goto(base + '/render-tests'); // no test selected: no rAF loop
  await page.waitForFunction(() => window.__rt, null, { timeout: 30000 });

  let failures = 0;
  const glErrors = [];
  const t = {
    page, errors,
    report(ok, msg) { console.log((ok ? 'PASS ' : 'FAIL ') + msg); if (!ok) failures++; },
    // Render `cmds` (flat array) at fake time `ms`; prints the case's hash.
    async shot(name, cmds, { text = '', ms = 1000, w = 640, h = 360 } = {}) {
      const r = await page.evaluate(([n, c, tx, m, W, H]) => window.__shot(n, c, tx, m, W, H),
        [name, cmds, text, ms, w, h]);
      if (r.glError) glErrors.push(`${name}: 0x${r.glError.toString(16)}`);
      console.log(`FP ${name} ${r.hash}`);
      return r.hash;
    },
    diff: (a, b, tol = 0, rect = null) => page.evaluate(([x, y, z, r]) => window.__diff(x, y, z, r), [a, b, tol, rect]),
    stats: (name, rect = null, lit = 40) => page.evaluate(([n, r, l]) => window.__stats(n, r, l), [name, rect, lit]),
    pixel: (name, x, y) => page.evaluate(([n, a, b]) => window.__pixel(n, a, b), [name, x, y]),
    // Debugging aid: write a shot to a PNG (`await t.dump('name', '/tmp/x.png')`).
    async dump(name, file) {
      const b64 = await page.evaluate((n) => {
        const S = window.__shots.get(n), c = document.createElement('canvas');
        c.width = S.w; c.height = S.h;
        c.getContext('2d').putImageData(new ImageData(new Uint8ClampedArray(S.px), S.w, S.h), 0, 0);
        return c.toDataURL('image/png').split(',')[1];
      }, name);
      require('fs').writeFileSync(file, Buffer.from(b64, 'base64'));
    },
    blockiness: (name, cell) => page.evaluate(([n, c]) => window.__blockiness(n, c), [name, cell]),
    async finish() {
      t.report(errors.length === 0, `no console / page errors${errors.length ? ': ' + errors.slice(0, 3).join(' | ') : ''}`);
      t.report(glErrors.length === 0, `no GL errors${glErrors.length ? ': ' + glErrors.slice(0, 5).join(', ') : ''}`);
      await browser.close();
      console.log(failures ? `${failures} FAILED` : 'ALL PASS');
      process.exit(failures ? 1 : 0);
    },
  };
  return t;
}

module.exports = { openHarness, ops };
