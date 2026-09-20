// Headless acceptance test for the FOLDED TV STATIC — a standalone script
// like composite-coherence.js, not a Playwright spec:
//
//   cd tests/e2e && bun render/grain-fold.js [baseURL=http://localhost:8098]
//
// The film grain (POSTFX 13) used to be one more alpha-blended full-screen
// quad over the finished frame. With `?grain=fold` renderer.js FOLDS it into
// the batch fragment shader (opt-in: a trade — the quad's layer goes, but the
// extra fetch makes every batch fragment dearer; see FS in renderer.js): every fragment that lands on the canvas is grained as it
// is written, g(c) = c*(1-k) + n*k — an affine map, which commutes with
// alpha blending, so the final pixels are the quad's (see FS in renderer.js)
// and a whole full-screen layer of fill disappears.
//
// This script proves the "same pixels" half on REAL game frames: it captures
// the command stream of a live frame, freezes the game, and renders that one
// stream three ways with a fixed noise roll —
//     QUAD  (window.__grainFold = false: the old end-of-frame quad),
//     FOLD  (`?grain=fold`),
//     NONE  (the stream with its POSTFX op stripped: no grain at all)
// and asserts, over every pixel of the canvas:
//   1) FOLD == QUAD to within 8-bit rounding (the quad rounds the finished
//      colour once, the fold rounds at every blend): max channel delta <= 3,
//      and only a small share of pixels off by more than 1;
//   2) the grain is really there: FOLD differs from NONE on most pixels,
//      by about as much as QUAD does (a fold that silently turned itself
//      off would pass (1) against nothing... so compare against NONE).
// Scenes: plain gameplay, the `?pixel=N` world (the premultiplied group
// composite path), and floor 0's opening dialogue (translucent slab, text,
// portraits over the scene).
const { chromium } = require('playwright');

const BASE = process.argv[2] || 'http://localhost:8098';
const SCENES = [
  ['gameplay', '/?floor=5&debug&grain=fold'],
  ['pixel world', '/?floor=5&debug&pixel=3&grain=fold'],
  ['dialogue', '/?floor=0&debug&grain=fold'],
];

(async () => {
  const browser = await chromium.launch({
    args: ['--no-sandbox', '--disable-setuid-sandbox', '--enable-unsafe-swiftshader', '--use-gl=angle', '--use-angle=swiftshader', '--disable-dev-shm-usage'],
  });
  let failures = 0;
  const report = (ok, msg) => { console.log((ok ? 'PASS ' : 'FAIL ') + msg); if (!ok) failures++; };

  for (const [name, url] of SCENES) {
    const page = await browser.newPage({ viewport: { width: 1280, height: 720 }, deviceScaleFactor: 1 });
    await page.addInitScript(() => {
      window.__fakeT = null;
      const orig = performance.now.bind(performance);
      performance.now = () => (window.__fakeT == null ? orig() : window.__fakeT);
      try { localStorage.setItem('om.fps_cap', '120'); } catch (e) {}
      // Tap frameRender: keep a copy of the latest frame's stream.
      let real = null;
      Object.defineProperty(window, 'frameRender', {
        configurable: true,
        set(f) { real = f; window.__realFrameRender = f; },
        get() {
          return real && function (cmds, text) {
            window.__lastFrame = { cmds: new Float32Array(cmds), text };
            return real(cmds, text);
          };
        },
      });
    });
    const errors = [];
    page.on('console', (m) => { if (m.type() === 'error') errors.push(m.text()); });
    page.on('pageerror', (e) => errors.push(String(e)));
    await page.goto(BASE + url);
    await page.waitForFunction(() => document.getElementById('loading').style.display === 'none', null, { timeout: 30000 });
    await page.waitForTimeout(1200);
    await page.evaluate(() => { window.__fakeT = performance.now(); }); // freeze: the FPS cap now skips every engine frame
    await page.waitForTimeout(200);

    const r = await page.evaluate(() => {
      const canvas = document.getElementById('glcanvas');
      const W = canvas.width, H = canvas.height;
      const c2 = document.createElement('canvas'); c2.width = W; c2.height = H;
      const g2 = c2.getContext('2d', { willReadFrequently: true });
      const { cmds, text } = window.__lastFrame;
      const OP_ARGS = [4, 8, 9, 7, 9, 9, 8, 0, 0, 2, 1, 8, 2, 6, 5, 4, 2, 6, 5, 6, 16, 1, 0, 1, 8, 5];
      let hasStatic = false, hasBackdrop = false;
      const stripped = [];
      for (let i = 0; i < cmds.length;) {
        const op = cmds[i], n = OP_ARGS[op];
        if (op === 14 && (cmds[i + 1] | 0) === 13) hasStatic = true;
        if (op === 24) hasBackdrop = true;
        if (op !== 14) for (let k = 0; k <= n; k++) stripped.push(cmds[i + k]);
        i += 1 + n;
      }
      window.__grainOffset = [37 / 512, 211 / 512];
      function shot(stream, fold) {
        window.__grainFold = fold;
        window.__realFrameRender(stream, text);
        g2.clearRect(0, 0, W, H);
        g2.drawImage(canvas, 0, 0);
        return g2.getImageData(0, 0, W, H).data;
      }
      const quad = shot(cmds, false), fold = shot(cmds, true), none = shot(new Float32Array(stripped), true);
      function diff(a, b) {
        let max = 0, over1 = 0, any = 0, sum = 0;
        for (let p = 0; p < a.length; p += 4) {
          let m = 0;
          for (let c = 0; c < 3; c++) m = Math.max(m, Math.abs(a[p + c] - b[p + c]));
          if (m > max) max = m;
          if (m > 1) over1++;
          if (m > 0) any++;
          sum += m;
        }
        const px = a.length / 4;
        return { max, over1: over1 / px, any: any / px, mean: sum / px };
      }
      return { W, H, hasStatic, hasBackdrop, foldVsQuad: diff(fold, quad), foldVsNone: diff(fold, none), quadVsNone: diff(quad, none) };
    });

    const f = r.foldVsQuad, gN = r.foldVsNone, qN = r.quadVsNone;
    console.log(`${name}: ${r.W}x${r.H} · fold vs quad: max ${f.max}, >1 on ${(f.over1 * 100).toFixed(3)}% px, any on ${(f.any * 100).toFixed(1)}% ` +
      `· grain strength (mean delta vs no grain): fold ${gN.mean.toFixed(2)} / quad ${qN.mean.toFixed(2)}, touches ${(gN.any * 100).toFixed(0)}% px`);
    report(errors.length === 0, `${name}: no console / page errors${errors.length ? ': ' + errors.join(' | ') : ''}`);
    report(r.hasStatic && r.hasBackdrop, `${name}: the frame carries the TV static and a BACKDROP (the fold's precondition)`);
    report(f.max <= 3, `${name}: fold == quad to 8-bit rounding (max channel delta ${f.max} <= 3)`);
    report(f.over1 <= 0.02, `${name}: deltas above 1 on ${(f.over1 * 100).toFixed(3)}% of pixels (<= 2%)`);
    report(gN.any >= 0.5 && qN.mean > 0 && Math.abs(gN.mean - qN.mean) <= 0.25 * qN.mean,
      `${name}: the folded grain is there, as strong as the quad's (${gN.mean.toFixed(2)} vs ${qN.mean.toFixed(2)})`);
    await page.close();
  }

  await browser.close();
  if (failures) { console.log(`${failures} FAILED`); process.exit(1); }
  console.log('ALL PASS');
})().catch((e) => { console.error(e); process.exit(1); });
