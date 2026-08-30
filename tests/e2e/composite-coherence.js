// Headless coherence test for the pixel-group composite (the `?pixel=N`
// world path) — not part of the Playwright spec run, a standalone script
// like props-stability.js:
//
//   cd tests/e2e && bun composite-coherence.js [baseURL=http://localhost:8098]
//
// It freezes the game loop (frozen clock + the FPS cap's skip), then drives
// window.frameRender directly with a synthetic stream: a black square inside
// a pixel group under a slight rotation — the minimal version of the swaying
// pixelated world. Sampling is NEAREST on purpose (the ALIASING is the art
// direction — CLAUDE.md ## Design — so nothing here asserts smoothing).
// Numeric assertions on the raw canvas RGBA:
//   1) ABSOLUTE placement: the edge sits where the analytic transform says,
//      including for FRACTIONAL group sizes (w/px not an integer — the
//      camera-sized world group; regression for the composite's v flip,
//      which must anchor at the integer texel row count or content swims);
//   2) the measured slope matches the requested angle, and the hard-pixel
//      edge stays within rasterization bounds of the ideal line;
//   3) texel interiors stay PURE (no blur bleed: black 0, white 255) and
//      RIGID under sub-pixel motion;
//   4) the `smooth` flag = SUB-PIXEL PLACEMENT: a 0.37-px translation moves
//      the mean edge position by ~0.37 px (gliding motion), while the
//      snapped composite moves it ~0 (quantized) — motion is where the
//      smoothness lives, never in the sampling.
// Run at deviceScaleFactor 1 and 2 (the Retina case).
const { chromium } = require('playwright');

const BASE = process.argv[2] || 'http://localhost:8098';

(async () => {
  const browser = await chromium.launch({
    args: ['--no-sandbox', '--disable-setuid-sandbox', '--enable-unsafe-swiftshader', '--use-gl=angle', '--use-angle=swiftshader', '--disable-dev-shm-usage'],
  });
  let failures = 0;
  const report = (ok, msg) => { console.log((ok ? 'PASS ' : 'FAIL ') + msg); if (!ok) failures++; };

  for (const dpr of [1, 2]) {
    const page = await browser.newPage({ viewport: { width: 1280, height: 800 }, deviceScaleFactor: dpr });
    await page.addInitScript(() => {
      window.__fakeT = null;
      const orig = performance.now.bind(performance);
      performance.now = () => (window.__fakeT == null ? orig() : window.__fakeT);
      // Keep the FPS cap ON: with the clock frozen it skips every engine
      // frame, so manual frameRender calls own the canvas.
      try { localStorage.setItem('om.fps_cap', '120'); } catch (e) {}
    });
    const errors = [];
    page.on('console', (m) => { if (m.type() === 'error') errors.push(m.text()); });
    page.on('pageerror', (e) => errors.push(String(e)));
    await page.goto(BASE + '/?floor=2&debug');
    await page.waitForFunction(() => document.getElementById('loading').style.display === 'none', null, { timeout: 30000 });
    await page.waitForTimeout(300);
    await page.evaluate(() => { window.__fakeT = 5000; });
    await page.waitForTimeout(200);

    const out = await page.evaluate(async (dpr) => {
      const canvas = document.getElementById('glcanvas');
      const CW = canvas.width, CH = canvas.height;
      const c2 = document.createElement('canvas');
      c2.width = CW; c2.height = CH;
      const g2 = c2.getContext('2d', { willReadFrequently: true });
      const SQ = { x: 120, y: 120, w: 240, h: 240 };
      function frame(a, px, smooth, cx, cy, W) {
        // ops: CLEAR, SAVE, TRANSLATE, ROTATE, PIX_BEGIN(smooth), white bg
        // rect, black square rect, PIX_END centered, RESTORE
        const s = [
          0, 0.85, 0.85, 0.85, 1,
          7, 9, cx, cy, 10, a,
          15, px, W, W, smooth,
          1, 0, 0, W, W, 1, 1, 1, 1,
          1, SQ.x, SQ.y, SQ.w, SQ.h, 0, 0, 0, 1,
          16, -W / 2, -W / 2, 8,
        ];
        window.frameRender(new Float32Array(s), '');
        g2.clearRect(0, 0, CW, CH);
        g2.drawImage(canvas, 0, 0);
        return g2.getImageData(0, 0, CW, CH).data;
      }
      const lum = (d, x, y) => { const o = (y * CW + x) * 4; return (d[o] + d[o + 1] + d[o + 2]) / 3; };
      function edgeY(d, x, y0, y1) {
        for (let y = y0; y < y1; y++) {
          const a = lum(d, x, y), b = lum(d, x, y + 1);
          if (a >= 128 && b < 128) return y + (a - 128) / (a - b);
        }
        return NaN;
      }
      const cx = 640, cy = 400;
      const res = [];
      for (const cfg of [
        { a: 0.0061, px: 8, W: 480, smooth: 1 }, // the game's max sway roll
        { a: 0.0061, px: 8, W: 480, smooth: 0 }, // hard composite, contrast baseline
        { a: 0.035, px: 3, W: 480, smooth: 1 },  // 2 deg at game-like art scale
        // FRACTIONAL group sizes (W/px not an integer): the game's world group
        // is camera-sized, so ceil(h/px) != h/px — the composite's v flip must
        // anchor at the integer row count or every horizontal edge lands
        // displaced by the ceil remainder (and swims as the group resizes).
        { a: 0.0061, px: 8, W: 481, smooth: 1 }, // remainder 0.875 texel
        { a: 0.0061, px: 3, W: 481, smooth: 1 }, // remainder 0.67 texel
      ]) {
        const d = frame(cfg.a, cfg.px, cfg.smooth, cx, cy, cfg.W);
        // The square's top edge: group-local y = SQ.y, quad at (-W/2, -W/2).
        const eyOff = SQ.y - cfg.W / 2;
        // Per-column edge positions vs the UNSHIFTED analytic expectation
        // (measure(d2) reuses the same baseline so the mean difference IS
        // the rendered motion of the edge).
        const measure = (img) => {
          const xs = [], ys = [], offs = [];
          for (let u = -110; u <= 110; u += 1) {
            const ex = (cx + Math.cos(cfg.a) * u - Math.sin(cfg.a) * eyOff) * dpr;
            const eyc = (cy + Math.sin(cfg.a) * u + Math.cos(cfg.a) * eyOff) * dpr;
            const x = Math.round(ex);
            const y = edgeY(img, x, Math.round(eyc) - 30, Math.round(eyc) + 30);
            if (!isNaN(y)) { xs.push(x); ys.push(y); offs.push(y - eyc); }
          }
          return { xs, ys, offs };
        };
        const M = measure(d);
        const n = M.xs.length;
        const mx = M.xs.reduce((a, b) => a + b, 0) / n, my = M.ys.reduce((a, b) => a + b, 0) / n;
        let sxy = 0, sxx = 0;
        for (let i = 0; i < n; i++) { sxy += (M.xs[i] - mx) * (M.ys[i] - my); sxx += (M.xs[i] - mx) ** 2; }
        const slope = sxy / sxx;
        let maxR = 0;
        for (let i = 0; i < n; i++) maxR = Math.max(maxR, Math.abs(M.ys[i] - (my + slope * (M.xs[i] - mx))));
        // ABSOLUTE placement: the rendered edge vs the analytic expectation.
        // Catches sampling-offset bugs a straightness fit cannot see.
        const meanOff = M.offs.reduce((a, b) => a + b, 0) / n;
        const inBlack = lum(d, Math.round(cx * dpr), Math.round(cy * dpr));
        const inWhite = lum(d, Math.round((cx - 200) * dpr), Math.round(cy * dpr));
        // sub-pixel vertical shift: interiors must not change; the MEAN edge
        // position must glide (smooth) or hold (snapped)
        const d2 = frame(cfg.a, cfg.px, cfg.smooth, cx, cy + 0.37, cfg.W);
        let interiorDiff = 0;
        for (let k = 0; k < 400; k++) {
          const x = Math.round((cx - 60 + (k % 20) * 6) * dpr), y = Math.round((cy - 60 + Math.floor(k / 20) * 6) * dpr);
          if (Math.abs(lum(d2, x, y) - lum(d, x, y)) > 2) interiorDiff++;
        }
        const M2 = measure(d2);
        const meanMove = M2.offs.reduce((a, b) => a + b, 0) / M2.offs.length - meanOff;
        res.push({ cfg, n, slope, maxR, meanOff, inBlack, inWhite, interiorDiff, meanMove });
      }
      return res;
    }, dpr);

    for (const r of out) {
      const tag = `dpr${dpr} a=${r.cfg.a} px=${r.cfg.px} W=${r.cfg.W} smooth=${r.cfg.smooth}`;
      report(r.n > 180, `${tag}: edge found on ${r.n}/221 columns`);
      report(Math.abs(r.slope - Math.tan(r.cfg.a)) < 0.002,
        `${tag}: slope ${r.slope.toFixed(5)} ~ ${Math.tan(r.cfg.a).toFixed(5)}`);
      report(Math.abs(r.meanOff) < 1.5,
        `${tag}: edge sits ${r.meanOff.toFixed(2)} px from the analytic position (< 1.5)`);
      report(r.maxR < 0.75,
        `${tag}: hard-pixel edge stays within ${r.maxR.toFixed(3)} px of the ideal line (< 0.75)`);
      if (r.cfg.smooth) {
        report(Math.abs(r.meanMove - 0.37 * dpr) < 0.4,
          `${tag}: sub-pixel shift moved the mean edge ${r.meanMove.toFixed(3)} px ~ ${(0.37 * dpr).toFixed(2)} (gliding placement)`);
      } else {
        report(Math.abs(r.meanMove) < 0.4,
          `${tag}: snapped composite held the edge (moved ${r.meanMove.toFixed(3)} px ~ 0)`);
      }
      report(r.inBlack < 3 && r.inWhite > 252,
        `${tag}: interiors pure (black ${r.inBlack}, white ${r.inWhite})`);
      report(r.interiorDiff === 0, `${tag}: interior texels rigid under sub-pixel motion (${r.interiorDiff} changed)`);
    }
    console.log(`dpr${dpr} console errors:`, JSON.stringify(errors));
    failures += errors.length;
    await page.close();
  }
  await browser.close();
  console.log(failures === 0 ? 'ALL PASS' : `${failures} FAILURES`);
  process.exit(failures === 0 ? 0 : 1);
})();
