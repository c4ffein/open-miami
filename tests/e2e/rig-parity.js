// Headless acceptance test for the robots' GPU RIG (robot-core.js) — a
// standalone script like composite-coherence.js, not a Playwright spec:
//
//   cd tests/e2e && bun rig-parity.js [baseURL=http://localhost:8098]
//
// The game draws its robots through the GPU rig: the skeleton runs in the
// vertex shader from a dozen per-instance pose scalars and the whole batch
// is ONE instanced draw. The CPU rig (JS builds a 27-mat4 palette per robot,
// one draw each) stays in robot-core as the single-sprite path AND as the
// reference: tools/rig-parity.html renders every pose x weapon x a spread of
// animation times / palettes / facings through both, in full 64-tile
// batches, and compares the art-resolution atlas texel by texel. Asserted:
//   1) the GPU rig is actually live (instancing available, no GL errors);
//   2) every tile holds a robot (no collapsed / off-tile geometry);
//   3) parity: the two rigs differ by at most a few edge texels per tile
//      (float32 shader trig vs float64 JS trig can flip a texel whose
//      centre sits on a box edge; anything structural — a wrong joint, a
//      weapon that failed to collapse, a spill into the neighbour tile —
//      moves dozens);
//   4) the CPU reference itself is unchanged (pinned checksum, SwiftShader
//      only — a different rasterizer legitimately hashes differently).
// The page is also the human-eye version: /tools/rig-parity.html (add
// ?bench=200 for the CPU submit cost of both rigs on the machine at hand).
const { chromium } = require('playwright');

const BASE = process.argv[2] || 'http://localhost:8098';
// Per-tile / total budget of differing texels (a tile is 43 x 43 = 1849).
const MAX_TILE_DIFF = 6;
const MAX_TOTAL_DIFF_PER_CASE = 0.5;
// FNV of the CPU-rig atlases under SwiftShader (headless Chromium). Update
// ONLY for an intended change of the robot art / rig.
const CPU_GOLDEN = process.env.RIG_GOLDEN || '';

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
  await page.goto(BASE + '/tools/rig-parity.html');
  await page.waitForFunction(() => window.rigParity && window.rigParity.done, null, { timeout: 60000 });
  const r = await page.evaluate(() => window.rigParity);

  console.log(`cases ${r.cases} · differing texels ${r.totalDiff} · worst tile ${r.maxTileDiff} · max channel delta ${r.maxChannelDiff}`);
  console.log(`cpu hash ${r.cpuHash} · gpu hash ${r.gpuHash}`);
  const worst = r.tiles.slice().sort((a, b) => b.diff - a.diff).slice(0, 5);
  for (const t of worst) console.log(`   ${String(t.diff).padStart(4)}  ${t.name}`);

  report(errors.length === 0, `no console / page errors${errors.length ? ': ' + errors.join(' | ') : ''}`);
  report(r.glErrors === 0, `no GL errors (${r.glErrors})`);
  report(r.gpuRig && r.instanced, 'the GPU rig is live (ANGLE_instanced_arrays)');
  report(r.cases >= 160, `covers every pose x weapon x time (${r.cases} cases)`);
  report(r.emptyTiles === 0, `every tile holds a robot (${r.emptyTiles} empty)`);
  report(r.maxTileDiff <= MAX_TILE_DIFF, `worst tile differs by ${r.maxTileDiff} texels (<= ${MAX_TILE_DIFF})`);
  report(r.totalDiff <= r.cases * MAX_TOTAL_DIFF_PER_CASE,
    `total differing texels ${r.totalDiff} (<= ${r.cases * MAX_TOTAL_DIFF_PER_CASE})`);
  if (CPU_GOLDEN) report(r.cpuHash === CPU_GOLDEN, `CPU reference checksum ${r.cpuHash} == pinned ${CPU_GOLDEN}`);

  await browser.close();
  if (failures) { console.log(`${failures} FAILED`); process.exit(1); }
  console.log('ALL PASS');
})().catch((e) => { console.error(e); process.exit(1); });
