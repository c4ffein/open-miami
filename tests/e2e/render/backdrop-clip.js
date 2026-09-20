// Headless acceptance test for the BACKDROP OCCLUSION — a standalone script
// like composite-coherence.js, not a Playwright spec:
//
//   cd tests/e2e && bun render/backdrop-clip.js [baseURL=http://localhost:8098]
//
// The void backdrop (opcode 24) is an opaque full-screen quad drawn first;
// the floor then paints over most of it. The wasm now sends, with the op, the
// screen rect the floor is guaranteed to cover (src/backdrop_clip.rs — the
// floor's rotated on-screen image, inset) and renderer.js draws the backdrop
// as strips AROUND it: the hidden fragments are never shaded. That is only
// legal if the picture does not change, so this script captures live game
// frames, freezes the game, and renders each captured stream twice —
//     CLIPPED  the stream as recorded (exclusion rect in the op),
//     FULL     the same stream with the rect zeroed (the whole quad) —
// and asserts the two canvases are IDENTICAL, pixel for pixel (not "close":
// every excluded pixel must be overdrawn by opaque floor, so nothing may
// differ at all). Scenes: the floor's corner (where the exclusion's edges run
// through the screen), the `?pixel=N` world (the floor's edge is quantized to
// art texels and composited sub-pixel — the inset must absorb that), floor 0
// (outdoor surface + dialogue), each sampled at several moments so the sway's
// roll / drift take different values. It also checks the exclusion is really
// in use (a rect that never appears would pass trivially).
const { chromium } = require('playwright');

const BASE = process.argv[2] || 'http://localhost:8098';
const SCENES = [
  ['floor corner', '/?floor=5&debug&noise=0'],
  ['pixel world', '/?floor=5&debug&noise=0&pixel=3'],
  ['pixel world, coarse', '/?floor=2&debug&noise=0&pixel=6'],
  ['floor 0', '/?floor=0&debug&noise=0'],
  ['floor corner @ DPR 2', '/?floor=5&debug&noise=0', 2], // the scissor works in PHYSICAL px
];
const SAMPLES = 4;

(async () => {
  const browser = await chromium.launch({
    args: ['--no-sandbox', '--disable-setuid-sandbox', '--enable-unsafe-swiftshader', '--use-gl=angle', '--use-angle=swiftshader', '--disable-dev-shm-usage'],
  });
  let failures = 0;
  const report = (ok, msg) => { console.log((ok ? 'PASS ' : 'FAIL ') + msg); if (!ok) failures++; };

  for (const [name, url, dpr] of SCENES) {
    const page = await browser.newPage({ viewport: { width: 1280, height: 720 }, deviceScaleFactor: dpr || 1 });
    await page.addInitScript(() => {
      window.__fakeT = null;
      const orig = performance.now.bind(performance);
      performance.now = () => (window.__fakeT == null ? orig() : window.__fakeT);
      try { localStorage.setItem('om.fps_cap', '120'); } catch (e) {}
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

    let worst = 0, excludedShare = 0, withRect = 0;
    for (let k = 0; k < SAMPLES; k++) {
      await page.evaluate(() => { window.__fakeT = null; });
      await page.waitForTimeout(k === 0 ? 1200 : 900); // let the sway move on
      await page.evaluate(() => { window.__fakeT = performance.now(); });
      await page.waitForTimeout(150);
      const r = await page.evaluate(() => {
        const canvas = document.getElementById('glcanvas');
        const W = canvas.width, H = canvas.height;
        const c2 = document.createElement('canvas'); c2.width = W; c2.height = H;
        const g2 = c2.getContext('2d', { willReadFrequently: true });
        const { cmds, text } = window.__lastFrame;
        const OP_ARGS = [4, 8, 9, 7, 9, 9, 8, 0, 0, 2, 1, 8, 2, 6, 5, 4, 2, 6, 5, 6, 16, 1, 0, 1, 8, 5];
        const full = new Float32Array(cmds);
        let rect = null;
        for (let i = 0; i < cmds.length;) {
          const op = cmds[i];
          if (op === 24) { rect = [cmds[i + 5], cmds[i + 6], cmds[i + 7], cmds[i + 8], cmds[i + 1], cmds[i + 2]]; full[i + 7] = 0; full[i + 8] = 0; }
          i += 1 + OP_ARGS[op];
        }
        const shot = (stream) => {
          window.__realFrameRender(stream, text);
          g2.clearRect(0, 0, W, H); g2.drawImage(canvas, 0, 0);
          return g2.getImageData(0, 0, W, H).data;
        };
        const a = shot(cmds), b = shot(full);
        let differing = 0, first = null;
        for (let p = 0; p < a.length; p += 4) {
          if (a[p] !== b[p] || a[p + 1] !== b[p + 1] || a[p + 2] !== b[p + 2]) {
            differing++;
            if (!first) first = [(p / 4) % W, Math.floor(p / 4 / W)];
          }
        }
        return { rect, differing, first };
      });
      if (!r.rect) { report(false, `${name}: the frame carries a BACKDROP op`); break; }
      const [ex, ey, ew, eh, w, h] = r.rect;
      if (ew > 0 && eh > 0) { withRect++; excludedShare += (ew * eh) / (w * h); }
      if (r.differing > worst) worst = r.differing;
      if (r.differing) console.log(`   sample ${k}: ${r.differing} differing px, first at ${r.first}, rect [${[ex, ey, ew, eh].map((v) => v.toFixed(1))}]`);
    }
    console.log(`${name}: exclusion on ${withRect}/${SAMPLES} samples, mean ${(100 * excludedShare / Math.max(1, withRect)).toFixed(0)}% of the screen not drawn`);
    report(errors.length === 0, `${name}: no console / page errors${errors.length ? ': ' + errors.join(' | ') : ''}`);
    report(withRect === SAMPLES, `${name}: the exclusion rect is in use (${withRect}/${SAMPLES})`);
    report(worst === 0, `${name}: clipped == full backdrop, pixel for pixel (worst sample: ${worst} differing px)`);
    await page.close();
  }

  await browser.close();
  if (failures) { console.log(`${failures} FAILED`); process.exit(1); }
  console.log('ALL PASS');
})().catch((e) => { console.error(e); process.exit(1); });
