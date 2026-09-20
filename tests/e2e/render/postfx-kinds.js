// Headless acceptance test for the POSTFX KINDS (opcode 14) — a standalone
// script like composite-coherence.js, not a Playwright spec:
//
//   cd tests/e2e && bun render/postfx-kinds.js [baseURL=http://localhost:8098]
//
// Renderer-only (the /render-tests harness, see lib.js): one hand-built scene
// is rendered through every kind of the table in `Graphics::postfx`'s doc
// (src/graphics.rs), with the post shaders' clock frozen. No golden images —
// what is asserted is what the table PROMISES:
//   1) t = 0 is the identity for every single-pass kind (0-9, 11) and for
//      TV STATIC (13): the frame through the scene target + the post shader
//      comes out as the frame drawn directly;
//   2) at t = 1 every kind really does something, and no two kinds produce
//      the same image (a kind routed to the wrong shader branch would);
//   3) the same stream at the same clock = the same pixels, bit for bit
//      (what makes the printed `FP` hashes diffable across a refactor);
//   4) the clock and the colour are wired where the table says they are;
//   5) MODAL STATIC (12): the panel passes through untouched, framed by a
//      6-px white then a 6-px black ring, and `t` IS the static's coverage;
//   6) TV STATIC (13): opacity scales with t;
//   7) WARP TRAILS (10): the feedback accumulates frame over frame, and a
//      frame without kind 10 CLEARS it;
//   8) an unknown kind is a no-op ("Any other kind is a no-op").
const { openHarness, ops } = require('./lib');

const BASE = process.argv[2] || 'http://localhost:8098';
const W = 640, H = 360;
const NAMES = ['BLUR-OUT', 'SYNTHWAVE CRT', 'VHS TAPE', 'DRUNK SWAY', 'CRT TUBE', 'ACID TRIP',
  'DATAMOSH', 'NEON BLOOM', 'PIXEL MOSAIC', 'TUNNEL RUSH', 'WARP TRAILS', 'UI GREY',
  'MODAL STATIC', 'TV STATIC'];
const SINGLE_PASS = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 11];

// A scene with what the effects feed on: a checker (detail everywhere, so a
// blur / shift / pixelation is visible), saturated and bright blocks (bloom,
// hue, channel split), thin lines, a white disc.
function scene() {
  const s = [...ops.CLEAR(0.05, 0.05, 0.08)];
  for (let ty = 0; ty < H / 40; ty++) for (let tx = 0; tx < W / 40; tx++) {
    const d = (tx + ty) % 2 === 0;
    s.push(...ops.RECT(tx * 40, ty * 40, 40, 40, d ? 0.16 : 0.30, d ? 0.14 : 0.24, d ? 0.22 : 0.36));
  }
  s.push(...ops.RECT(60, 60, 120, 90, 1, 0.2, 0.6));
  s.push(...ops.RECT(250, 40, 140, 70, 0.1, 0.9, 1.0));
  s.push(...ops.RECT(440, 200, 130, 110, 1, 1, 1));
  s.push(...ops.RECT(90, 230, 160, 60, 0.95, 0.85, 0.1));
  s.push(...ops.LINE(20, 180, 620, 190, 2, 1, 1, 1));
  s.push(...ops.LINE(320, 10, 300, 350, 1, 0.3, 1, 0.4));
  s.push(...ops.CIRCLE(330, 180, 26, 1, 1, 1));
  return s;
}

(async () => {
  const t = await openHarness(BASE);
  const base = scene();
  const fx = (kind, tt, r = 0, g = 0, b = 0) => [...base, ...ops.POSTFX(kind, tt, r, g, b)];

  await t.shot('base', base, { w: W, h: H });

  // 1) t = 0 is the identity.
  for (const k of [...SINGLE_PASS, 13]) {
    await t.shot(`k${k}-t0`, fx(k, 0, 0.9, 0.2, 0.6), { w: W, h: H });
    const d = await t.diff(`k${k}-t0`, 'base', 2);
    t.report(d.differing === 0, `kind ${k} ${NAMES[k]}: t=0 is the identity (max delta ${d.max}, ${d.differing} px over 2)`);
  }

  // 2) t = 1 does something, and every kind is its own image.
  for (const k of SINGLE_PASS) {
    await t.shot(`k${k}-t1`, fx(k, 1, 0.9, 0.2, 0.6), { w: W, h: H });
    const d = await t.diff(`k${k}-t1`, 'base', 8);
    t.report(d.share > 0.2, `kind ${k} ${NAMES[k]}: t=1 changes the frame (${(d.share * 100).toFixed(0)}% of px, mean delta ${d.mean.toFixed(1)})`);
  }
  let closest = { share: 1, a: -1, b: -1 };
  for (const a of SINGLE_PASS) for (const b of SINGLE_PASS) {
    if (a >= b) continue;
    const d = await t.diff(`k${a}-t1`, `k${b}-t1`, 8);
    if (d.share < closest.share) closest = { share: d.share, a, b };
  }
  t.report(closest.share > 0.1, `no two kinds render the same image (closest pair: ${closest.a} / ${closest.b}, ${(closest.share * 100).toFixed(0)}% of px differ)`);

  // 3) deterministic.
  let unstable = [];
  for (const k of SINGLE_PASS) {
    await t.shot(`k${k}-t1-again`, fx(k, 1, 0.9, 0.2, 0.6), { w: W, h: H });
    const d = await t.diff(`k${k}-t1`, `k${k}-t1-again`, 0);
    if (d.differing) unstable.push(k);
  }
  t.report(unstable.length === 0, `same stream + same clock = same pixels, every kind${unstable.length ? ' — NOT: ' + unstable : ''}`);

  // 4) the clock (every kind but the static PIXEL MOSAIC animates) and the
  //    colour (the kinds whose doc names it) are wired.
  for (const k of SINGLE_PASS) {
    await t.shot(`k${k}-later`, fx(k, 1, 0.9, 0.2, 0.6), { w: W, h: H, ms: 4370 });
    const d = await t.diff(`k${k}-t1`, `k${k}-later`, 0);
    if (k === 8) t.report(d.differing === 0, `kind 8 ${NAMES[8]}: does not depend on the clock`);
    else t.report(d.share > 0.01, `kind ${k} ${NAMES[k]}: animates with the clock (${(d.share * 100).toFixed(0)}% of px)`);
  }
  for (const k of [0, 1, 7, 11]) {
    await t.shot(`k${k}-cyan`, fx(k, 1, 0.1, 0.9, 1.0), { w: W, h: H });
    const d = await t.diff(`k${k}-t1`, `k${k}-cyan`, 4);
    t.report(d.share > 0.05, `kind ${k} ${NAMES[k]}: the colour tints it (${(d.share * 100).toFixed(0)}% of px)`);
  }

  // 5) MODAL STATIC over a flat mid-grey: non-static px stay grey (the blur
  //    and the desaturation of a flat grey are that grey), static cells are
  //    0.42 * grey + 0.58 * {0,1} — so coverage can be COUNTED.
  const grey = [...ops.CLEAR(0.5, 0.5, 0.5)];
  const hx = 0.25, hy = 0.25; // panel = the centred half of the screen (160..480, 90..270)
  const panel = [W * (0.5 - hx), H * (0.5 - hy), W * (0.5 + hx), H * (0.5 + hy)];
  await t.shot('grey', grey, { w: W, h: H });
  for (const tt of [0, 0.5, 1]) {
    await t.shot(`k12-t${tt}`, [...grey, ...ops.POSTFX(12, tt, hx, hy, 0)], { w: W, h: H });
  }
  const inside = await t.diff('k12-t1', 'grey', 1, panel);
  t.report(inside.differing === 0, `kind 12 ${NAMES[12]}: the panel passes through untouched (${inside.differing} px differ)`);
  const white = await t.pixel('k12-t1', panel[0] - 3, H / 2), black = await t.pixel('k12-t1', panel[0] - 9, H / 2);
  t.report(white.every((v) => v === 255) && black.every((v) => v === 0),
    `kind 12: framed by a white then a black 6-px ring (${white} / ${black})`);
  for (const [tt, lo, hi] of [[0, 0, 0], [0.5, 0.45, 0.55], [1, 1, 1]]) {
    const out = await t.diff(`k12-t${tt}`, 'grey', 30, [0, 0, W, 60]); // a strip well outside the rings
    t.report(out.share >= lo && out.share <= hi, `kind 12: t=${tt} -> ${(out.share * 100).toFixed(1)}% static coverage (want ${Math.round(lo * 100)}-${Math.round(hi * 100)}%)`);
  }

  // 6) TV STATIC: opacity scales with t.
  const means = [];
  for (const tt of [0.25, 0.5, 1]) {
    await t.shot(`k13-t${tt}`, fx(13, tt), { w: W, h: H });
    means.push((await t.diff(`k13-t${tt}`, 'base', 0)).mean);
  }
  t.report(means[0] > 1 && means[1] > means[0] * 1.6 && means[2] > means[1] * 1.6,
    `kind 13 ${NAMES[13]}: opacity scales with t (mean delta ${means.map((m) => m.toFixed(1)).join(' < ')})`);

  // 7) WARP TRAILS: a feedback accumulator.
  const warp = fx(10, 0.8, 1, 0.4, 0.8);
  await t.shot('warp-1', warp, { w: W, h: H });
  for (let i = 2; i <= 12; i++) await t.shot(`warp-${i}`, warp, { w: W, h: H, ms: 1000 + i * 16 });
  const grown = await t.diff('warp-12', 'warp-1', 6);
  t.report(grown.share > 0.02, `kind 10 ${NAMES[10]}: the trails accumulate over frames (${(grown.share * 100).toFixed(1)}% of px changed by frame 12)`);
  await t.shot('warp-break', base, { w: W, h: H }); // a frame WITHOUT kind 10...
  await t.shot('warp-restart', warp, { w: W, h: H }); // ...clears the accumulator
  const restart = await t.diff('warp-restart', 'warp-1', 0);
  t.report(restart.differing === 0, `kind 10: a frame without it clears the accumulator (restart vs first frame: ${restart.differing} px differ)`);

  // 8) unknown kinds.
  for (const k of [14, 99, -1]) {
    await t.shot(`k${k}`, fx(k, 1, 0.3, 0.3, 0), { w: W, h: H });
    const d = await t.diff(`k${k}`, 'base', 2);
    t.report(d.differing === 0, `unknown kind ${k} is a no-op (${d.differing} px differ)`);
  }

  await t.finish();
})();
