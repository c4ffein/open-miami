// Headless acceptance test for the two FULL-SHADER BACKGROUNDS — DRIVE
// (opcode 20, the synthwave drive behind the menus) and BACKDROP (opcode 24,
// the neon-wave void around a floor). A standalone script like
// composite-coherence.js, not a Playwright spec:
//
//   cd tests/e2e && bun render/drive-backdrop.js [baseURL=http://localhost:8098]
//
// Renderer-only (the /render-tests harness, see lib.js). Both are computed
// at ART RESOLUTION into a tiny target and drawn as ONE NEAREST-upscaled quad
// (the perf rule in CLAUDE.md). No golden image of the scenery — what is
// asserted is the CONTRACT of the two ops:
//   DRIVE  w h t glitch split px dim o0..o8
//   1) the art grid: the image is constant over every aligned px x px block
//      (rasterized at art resolution, upscaled NEAREST — never blurred);
//   2) it is a scene (a sky over a road, split at the horizon, the sun in
//      the sky's centre), and `t` moves it;
//   3) same arguments = same pixels;
//   4) `dim` mixes toward the menu's near-black, inside the shader: dim = 1
//      is that flat colour, dim = 0.5 is halfway;
//   5) `o[band]` tears ONE of the nine horizontal bands sideways: that band
//      is the untorn image shifted, the slice it vacated shows the void, and
//      the eight other bands do not change;
//   6) `split` ghosts the channels from 0.75 px up, and not below;
//   7) the quad lands at the current transform's origin, and only there.
//   BACKDROP  w h t px ex ey ew eh
//   8) the art grid, again; 9) it stays DIM (a void, not a light show) and
//   drifts slowly with `t`; 10) the occlusion rect is not drawn, the rest is
//   pixel-identical to the full quad (backdrop-clip.js proves the same on
//   live game frames).
const { openHarness, ops } = require('./lib');

const BASE = process.argv[2] || 'http://localhost:8098';
const W = 640, H = 360, PX = 4;
const CLEAR = ops.CLEAR(0.5, 0, 0.5); // a colour neither background produces
const CLEAR_RGB = [128, 0, 128];

(async () => {
  const t = await openHarness(BASE);
  const drive = (name, { time = 12.3, glitch = 0, split = 0, px = PX, dim = 0, offs, pre = [] } = {}) =>
    t.shot(name, [...CLEAR, ...pre, ...ops.DRIVE(W, H, time, glitch, split, px, dim, offs)], { w: W, h: H });
  const horizon = Math.round(H * 0.44);

  // ---- DRIVE ----
  await drive('drive');
  t.report((await t.blockiness('drive', PX)) === 0, `DRIVE: constant over every ${PX}x${PX} block (art resolution, NEAREST upscale)`);
  await drive('drive-px8', { px: 8 });
  t.report((await t.blockiness('drive-px8', 8)) === 0 && (await t.blockiness('drive', 8)) > 0.05,
    'DRIVE: px sets the art grid (px=8 is 8x8-uniform, px=4 is not)');

  const sky = await t.stats('drive', [0, 0, W, horizon]), road = await t.stats('drive', [0, horizon, W, H]);
  const sun = await t.stats('drive', [W / 2 - 40, horizon - 70, W / 2 + 40, horizon - 10]);
  const corner = await t.stats('drive', [0, 0, 80, 60]);
  const lum = (m) => 0.299 * m[0] + 0.587 * m[1] + 0.114 * m[2];
  t.report(sky.colours > 8 && road.colours > 8, `DRIVE: a sky (${sky.colours} colours) over a road (${road.colours} colours)`);
  t.report(lum(sun.mean) > lum(corner.mean) * 2, `DRIVE: the sun sits over the horizon's centre (luma ${lum(sun.mean).toFixed(0)} vs sky corner ${lum(corner.mean).toFixed(0)})`);
  await drive('drive-later', { time: 12.8 });
  const moved = await t.diff('drive', 'drive-later', 0, [0, horizon, W, H]);
  t.report(moved.share > 0.05, `DRIVE: t moves the road (${(moved.share * 100).toFixed(0)}% of its px in 0.5 s)`);
  await drive('drive-again');
  t.report((await t.diff('drive', 'drive-again', 0)).differing === 0, 'DRIVE: same arguments = same pixels');

  await drive('drive-dim1', { dim: 1 });
  const d1 = await t.stats('drive-dim1');
  t.report(d1.colours === 1 && d1.mean[0] <= 6 && d1.mean[2] <= 11, `DRIVE: dim = 1 is the flat menu black (${d1.mean.map((v) => v.toFixed(0))})`);
  await drive('drive-dim05', { dim: 0.5 });
  const full = await t.stats('drive'), half = await t.stats('drive-dim05');
  const want = full.mean.map((v, i) => (v + d1.mean[i]) / 2);
  t.report(half.mean.every((v, i) => Math.abs(v - want[i]) < 1.5), `DRIVE: dim = 0.5 is halfway (mean ${half.mean.map((v) => v.toFixed(1))}, want ${want.map((v) => v.toFixed(1))})`);

  // The tear. Band 3 of 9 = rows [120, 160) at H = 360; shift = 40 CSS px.
  const BAND = 3, SHIFT = 40, bandH = H / 9;
  const offs = [0, 0, 0, 0, 0, 0, 0, 0, 0];
  offs[BAND] = SHIFT;
  await drive('drive-torn', { offs });
  const y0 = BAND * bandH, y1 = (BAND + 1) * bandH;
  const above = await t.diff('drive', 'drive-torn', 0, [0, 0, W, y0]), below = await t.diff('drive', 'drive-torn', 0, [0, y1, W, H]);
  t.report(above.differing === 0 && below.differing === 0, `DRIVE: a torn band leaves the 8 others alone (${above.differing + below.differing} px differ outside it)`);
  const shifted = await t.page.evaluate(([y0, y1, W, SHIFT, PX]) => {
    const A = window.__shots.get('drive'), B = window.__shots.get('drive-torn');
    let bad = 0, voidBad = 0;
    for (let y = y0; y < y1; y++) for (let x = 0; x < W; x++) {
      const i = (y * W + x) * 4;
      if (x < SHIFT - PX) { // the vacated slice: the backing void (0.01, 0.0, 0.03)
        if (B.px[i] > 4 || B.px[i + 1] > 1 || B.px[i + 2] > 9) voidBad++;
      } else if (x < SHIFT) {
        // The art column whose CENTRE lands half a cell outside the scene:
        // DRIVE_FS's void test is `p.x < -0.5 * uPx`, so it still samples.
      } else {
        const j = (y * W + x - SHIFT) * 4;
        if (A.px[j] !== B.px[i] || A.px[j + 1] !== B.px[i + 1] || A.px[j + 2] !== B.px[i + 2]) bad++;
      }
    }
    return { bad, voidBad };
  }, [y0, y1, W, SHIFT, PX]);
  t.report(shifted.bad === 0, `DRIVE: the torn band IS the image shifted ${SHIFT} px (${shifted.bad} px are not)`);
  t.report(shifted.voidBad === 0, `DRIVE: the slice it vacated shows the void (${shifted.voidBad} px do not)`);

  await drive('drive-split', { split: 6 });
  await drive('drive-split-tiny', { split: 0.5 });
  const sp = await t.diff('drive', 'drive-split', 4), tiny = await t.diff('drive', 'drive-split-tiny', 0);
  t.report(sp.share > 0.03 && tiny.differing === 0, `DRIVE: the channel split ghosts from 0.75 px up (6 px: ${(sp.share * 100).toFixed(0)}% of px; 0.5 px: ${tiny.differing})`);
  await drive('drive-glitch', { glitch: 1 });
  t.report((await t.blockiness('drive-glitch', PX)) === 0, 'DRIVE: a glitched frame stays on the art grid');

  // Placement: a 320x180 drive at translate(160, 88).
  const TX = 160, TY = 88, SW = 320, SH = 180;
  await t.shot('drive-placed', [...CLEAR, ...ops.SAVE(), ...ops.TRANSLATE(TX, TY), ...ops.DRIVE(SW, SH, 12.3, 0, 0, PX, 0), ...ops.RESTORE()], { w: W, h: H });
  await t.shot('drive-small', [...CLEAR, ...ops.DRIVE(SW, SH, 12.3, 0, 0, PX, 0)], { w: W, h: H });
  const placed = await t.page.evaluate(([TX, TY, SW, SH, W, H, C]) => {
    const A = window.__shots.get('drive-small'), B = window.__shots.get('drive-placed');
    let bad = 0, spill = 0;
    for (let y = 0; y < H; y++) for (let x = 0; x < W; x++) {
      const i = (y * W + x) * 4;
      const inside = x >= TX && x < TX + SW && y >= TY && y < TY + SH;
      if (inside) {
        const j = ((y - TY) * W + x - TX) * 4;
        if (A.px[j] !== B.px[i] || A.px[j + 1] !== B.px[i + 1] || A.px[j + 2] !== B.px[i + 2]) bad++;
      } else if (B.px[i] !== C[0] || B.px[i + 1] !== C[1] || B.px[i + 2] !== C[2]) spill++;
    }
    return { bad, spill };
  }, [TX, TY, SW, SH, W, H, CLEAR_RGB]);
  t.report(placed.bad === 0 && placed.spill === 0, `DRIVE: lands at the transform's origin, and only there (${placed.bad} px misplaced, ${placed.spill} px spilled)`);

  // ---- BACKDROP ----
  const backdrop = (name, time, e = [0, 0, 0, 0], px = PX) =>
    t.shot(name, [...CLEAR, ...ops.BACKDROP(W, H, time, px, ...e)], { w: W, h: H });
  await backdrop('void', 30);
  t.report((await t.blockiness('void', PX)) === 0, `BACKDROP: constant over every ${PX}x${PX} block`);
  const v = await t.stats('void', null, 110);
  t.report(v.colours > 16 && lum(v.mean) < 45 && v.lit === 0, `BACKDROP: a DIM field (${v.colours} colours, mean luma ${lum(v.mean).toFixed(1)}, ${v.lit} px over 110)`);
  await backdrop('void-later', 32);
  const drift = await t.diff('void', 'void-later', 0);
  t.report(drift.share > 0.2 && drift.mean < 8, `BACKDROP: drifts slowly (2 s: ${(drift.share * 100).toFixed(0)}% of px, mean delta ${drift.mean.toFixed(1)})`);
  await backdrop('void-again', 30);
  t.report((await t.diff('void', 'void-again', 0)).differing === 0, 'BACKDROP: same arguments = same pixels');
  const E = [200, 100, 240, 160];
  await backdrop('void-clipped', 30, E);
  const hole = await t.stats('void-clipped', [E[0] + 4, E[1] + 4, E[0] + E[2] - 4, E[1] + E[3] - 4]);
  t.report(hole.colours === 1 && hole.mean[0] === CLEAR_RGB[0], 'BACKDROP: the occlusion rect is not drawn');
  const ring = await t.page.evaluate(([E, W, H]) => {
    const A = window.__shots.get('void'), B = window.__shots.get('void-clipped');
    let bad = 0;
    for (let y = 0; y < H; y++) for (let x = 0; x < W; x++) {
      if (x >= E[0] && x < E[0] + E[2] && y >= E[1] && y < E[1] + E[3]) continue;
      const i = (y * W + x) * 4;
      if (A.px[i] !== B.px[i] || A.px[i + 1] !== B.px[i + 1] || A.px[i + 2] !== B.px[i + 2]) bad++;
    }
    return bad;
  }, [E, W, H]);
  t.report(ring === 0, `BACKDROP: outside the rect, clipped == full (${ring} px differ)`);

  if (process.env.DUMP) for (const n of ['drive', 'drive-torn', 'drive-glitch', 'void']) await t.dump(n, `${process.env.DUMP}/${n}.png`);
  await t.finish();
})();
