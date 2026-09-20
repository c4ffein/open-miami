// Headless acceptance test for the boss's INSTANCED sphere path
// (web/shoggoth-core.js) — a standalone script like rig-parity.js, not a
// Playwright spec:
//
//   cd tests/e2e && bun render/shoggoth-parity.js [baseURL=http://localhost:8098]
//
// The boss is a few dozen shaded spheres. The game draws them as TWO instanced
// draws (body depth-ON, mask depth-OFF); the original one-draw-per-sphere code
// stays as the REFERENCE. tools/shoggoth-parity.html renders the whole
// mask-off arc x a spread of clocks / headings / cameras through both, at the
// game's tile settings, and this script requires them to agree to a few edge
// pixels per frame (the reference multiplies VP * model in JS doubles; the
// instanced shader does it in float32). It is also the boss's FIRST pixel
// test: its animation was tuned by eye, and moving its sphere placement to
// Rust (docs/ARCHITECTURE.md roadmap) will be checked against this page.
const { chromium } = require('playwright');

const BASE = process.argv[2] || 'http://localhost:8098';
// A frame is 384 x 384 = 147,456 px. Budgets measured under SwiftShader
// (worst frame 0..a handful of silhouette-edge pixels), with headroom.
const MAX_FRAME_DIFF = 40;
const MAX_TOTAL_DIFF_PER_CASE = 6;

(async () => {
  const browser = await chromium.launch({
    args: ['--no-sandbox', '--disable-setuid-sandbox', '--enable-unsafe-swiftshader', '--use-gl=angle', '--use-angle=swiftshader', '--disable-dev-shm-usage'],
  });
  let failures = 0;
  const report = (ok, msg) => { console.log((ok ? 'PASS ' : 'FAIL ') + msg); if (!ok) failures++; };

  const page = await browser.newPage({ viewport: { width: 1280, height: 800 } });
  const errors = [];
  page.on('console', (m) => { if (m.type() === 'error') errors.push(m.text()); });
  page.on('pageerror', (e) => errors.push(String(e)));
  await page.goto(BASE + '/tools/shoggoth-parity.html');
  await page.waitForFunction(() => window.shoggothParity && window.shoggothParity.done, null, { timeout: 120000 });
  const r = await page.evaluate(() => window.shoggothParity);

  console.log(`cases ${r.cases} · differing px ${r.totalDiff} · worst frame ${r.maxTileDiff}`);
  report(errors.length === 0, `no console / page errors${errors.length ? ': ' + errors.join(' | ') : ''}`);
  report(r.glErrors === 0, `no GL errors (${r.glErrors})`);
  report(r.instanced, 'the instanced path is available (ANGLE_instanced_arrays)');
  report(r.cases >= 40, `covers the mask-off arc x clocks + orbit / wander / high tess (${r.cases} cases)`);
  report(r.emptyTiles === 0, `every frame actually drew the boss (${r.emptyTiles} near-empty)`);
  console.log(`scene draws per frame — reference: up to ${r.draws.refMax} (avg ${(r.draws.ref / r.cases).toFixed(1)}) · instanced: up to ${r.draws.instMax}`);
  report(r.draws.instMax <= 2, `the instanced path submits at most 2 scene draws a frame (${r.draws.instMax})`);
  report(r.draws.refMax > 10, `the reference really is the per-sphere path (${r.draws.refMax} draws)`);
  report(r.leakedAttribs === 0, `no instancing divisor left set on the context (${r.leakedAttribs})`);
  report(r.maxTileDiff <= MAX_FRAME_DIFF, `worst frame differs by ${r.maxTileDiff} px (<= ${MAX_FRAME_DIFF})`);
  report(r.totalDiff <= r.cases * MAX_TOTAL_DIFF_PER_CASE,
    `total differing px ${r.totalDiff} (<= ${r.cases * MAX_TOTAL_DIFF_PER_CASE})`);
  if (r.maxTileDiff > MAX_FRAME_DIFF) {
    for (const t of r.tiles.filter((t) => t.diff > MAX_FRAME_DIFF).slice(0, 8)) {
      console.log(`  reveal ${t.reveal} time ${t.time}: ${t.diff} px`);
    }
  }

  await browser.close();
  console.log(failures === 0 ? 'ALL PASS' : `${failures} FAILED`);
  process.exit(failures === 0 ? 0 : 1);
})();
