// Headless acceptance test for TEXT (opcode 6) and the GLYPH ATLAS — a
// standalone script like composite-coherence.js, not a Playwright spec:
//
//   cd tests/e2e && bun render/text-glyphs.js [baseURL=http://localhost:8098]
//
// Renderer-only (the /render-tests harness, see lib.js; the page loads the
// real VT323). Text is lazily rasterized, one glyph at a time, into a 1024 px
// atlas at 48 px and drawn as scaled quads. No golden image of a font —
// what is asserted is the CONTRACT of `TEXT idx x y size r g b a`:
//   1) (x, y) is the LEFT end of the BASELINE: capitals sit on y, start at x,
//      and nothing is drawn outside the line's box;
//   2) `size` scales the text linearly (2x the size = 2x the box);
//   3) VT323 is monospace and the pen advances by whole glyphs: a marker
//      after N characters lands N advances to the right, whatever the glyphs;
//   4) colour and alpha are applied to the glyph's coverage (red text has no
//      green or blue; alpha 0.5 over black is half as bright);
//   5) text follows the transform stack (rotated a quarter turn, the box
//      becomes tall);
//   6) the text ARENA is indexed: the same stream with another arena draws
//      the other strings, and an out-of-range index draws nothing;
//   7) THE ATLAS SURVIVES ITS OWN RESET: after ~700 distinct glyphs the atlas
//      fills and is rebuilt from scratch — text drawn after the reset is
//      pixel-identical to the same text drawn before, including when the
//      reset happens IN THE MIDDLE of a frame that already drew text.
const { openHarness, ops } = require('./lib');

const BASE = process.argv[2] || 'http://localhost:8098';
const W = 640, H = 360;
const SEP = '\u001f'; // the text arena separator (Graphics::TEXT_SEP)
const BLACK = ops.CLEAR(0, 0, 0);

(async () => {
  const t = await openHarness(BASE);
  const draw = (name, strings, cmds) => t.shot(name, [...BLACK, ...cmds], { text: strings.join(SEP), w: W, h: H });
  const fontOk = await t.page.evaluate(() => document.fonts.check("48px 'GameFont'"));
  t.report(fontOk, 'the harness loaded VT323 (GameFont)');

  // 1) anchor: left end of the baseline.
  const X = 100, Y = 200, SIZE = 48;
  await draw('caps', ['HEALTH'], ops.TEXT(0, X, Y, SIZE, 1, 1, 1));
  const caps = await t.stats('caps');
  const [bx0, by0, bx1, by1] = caps.bbox;
  t.report(bx0 >= X - 1 && bx0 <= X + 4, `text starts at x (leftmost lit column ${bx0}, x = ${X})`);
  t.report(by1 <= Y && by1 >= Y - 3, `capitals sit on the baseline (lowest lit row ${by1}, y = ${Y})`);
  t.report(by0 >= Y - SIZE && by0 <= Y - SIZE * 0.4, `capitals are ${Y - by0} px tall at size ${SIZE}`);
  const advance = (bx1 - bx0) / 5.6; // six glyphs: 5 advances + most of one
  const outside = await t.stats('caps', [0, 0, W, Y - SIZE - 2]);
  t.report(outside.lit === 0, `nothing drawn above the line box (${outside.lit} lit px)`);

  // 2) size scales linearly.
  await draw('caps-2x', ['HEALTH'], ops.TEXT(0, X, Y, SIZE * 2, 1, 1, 1));
  const big = (await t.stats('caps-2x')).bbox;
  const sw = (big[2] - big[0]) / (bx1 - bx0), sh = (big[3] - big[1]) / (by1 - by0);
  t.report(Math.abs(sw - 2) < 0.08 && Math.abs(sh - 2) < 0.08, `2x the size = 2x the box (${sw.toFixed(3)} x ${sh.toFixed(3)})`);

  // 3) monospace pen: a '|' after 8 glyphs lands at the same column whether
  //    they are wide or narrow.
  const bars = [];
  for (const [i, s] of ['MMMMMMMM|', 'iiiiiiii|', '        |', 'W.W.W.W.|'].entries()) {
    await draw(`mono-${i}`, [s], ops.TEXT(0, 40, Y, SIZE, 1, 1, 1));
    const st = await t.stats(`mono-${i}`);
    bars.push(st.bbox[2]); // the bar is the rightmost ink
  }
  t.report(new Set(bars).size === 1, `the pen advances by whole monospace glyphs (bar at x = ${[...new Set(bars)].join(' / ')})`);
  const measured = (bars[0] - 40) / 8.5;
  t.report(Math.abs(measured - advance) < 2.5, `advance ~${measured.toFixed(1)} px at size ${SIZE} (from the word's box: ${advance.toFixed(1)})`);

  // 4) colour + alpha.
  await draw('red', ['HEALTH'], ops.TEXT(0, X, Y, SIZE, 1, 0, 0));
  const red = await t.stats('red');
  t.report(red.mean[0] > 1 && red.mean[1] === 0 && red.mean[2] === 0, `red text is red only (mean rgb ${red.mean.map((v) => v.toFixed(2))})`);
  await draw('half', ['HEALTH'], ops.TEXT(0, X, Y, SIZE, 1, 1, 1, 0.5));
  const half = await t.stats('half'), full = await t.stats('caps');
  const ratio = half.mean[0] / full.mean[0];
  t.report(Math.abs(ratio - 0.5) < 0.03, `alpha 0.5 over black is half as bright (${ratio.toFixed(3)})`);

  // 5) the transform stack.
  await draw('turned', ['HEALTH'], [...ops.SAVE(), ...ops.TRANSLATE(320, 60), ...ops.ROTATE(Math.PI / 2),
    ...ops.TEXT(0, 0, 0, SIZE, 1, 1, 1), ...ops.RESTORE()]);
  const tb = (await t.stats('turned')).bbox;
  t.report(tb[3] - tb[1] > (tb[2] - tb[0]) * 2.5 && Math.abs((tb[3] - tb[1]) - (bx1 - bx0)) <= 2,
    `rotated a quarter turn, the box is tall (${tb[2] - tb[0]} x ${tb[3] - tb[1]}; upright ${bx1 - bx0} x ${by1 - by0})`);

  // 6) the arena.
  const two = [...ops.TEXT(0, 40, 120, 32, 1, 1, 1), ...ops.TEXT(1, 40, 240, 32, 1, 1, 1)];
  await draw('arena-ab', ['AMMO 12', 'FLOOR 3'], two);
  await draw('arena-ba', ['FLOOR 3', 'AMMO 12'], two);
  await draw('arena-aa', ['AMMO 12', 'AMMO 12'], two);
  const top = [0, 80, W, 130], bottom = [0, 200, W, 250];
  const swapped = await t.diff('arena-ab', 'arena-ba', 0, top), kept = await t.diff('arena-ab', 'arena-aa', 0, top);
  const kept2 = await t.diff('arena-ba', 'arena-aa', 0, bottom);
  t.report(swapped.differing > 50 && kept.differing === 0 && kept2.differing === 0, `TEXT draws arena[idx] (swap changes ${swapped.differing} px of line 1, same string = 0)`);
  await draw('arena-oob', ['ONLY ONE'], ops.TEXT(5, 40, 120, 32, 1, 1, 1));
  t.report((await t.stats('arena-oob')).lit === 0, 'an out-of-range text index draws nothing');

  // WIDE FALLBACK GLYPHS, on every machine. What VT323 lacks falls back to
  // the box's fonts: real ones on a CI runner (wide CJK cells full of ink),
  // NONE on a bare dev box (every fallback glyph is then a VT323-wide tofu,
  // all atlas generations share one grid, and the reset bug below cannot
  // show — which is how it was first measured away). So the flood's code
  // points are MADE wide here: `measureText` reports 19 + 7k px and
  // `fillText` inks the whole advance.
  await t.page.evaluate(() => {
    const P = CanvasRenderingContext2D.prototype, measure = P.measureText, fill = P.fillText;
    const wide = (str) => { const cp = str.codePointAt(0); return cp >= 0x4e00 ? 19 + (cp % 5) * 7 : 0; };
    P.measureText = function (str) { const w = wide(str); return w ? { width: w } : measure.call(this, str); };
    P.fillText = function (str, x, y) {
      const w = wide(str);
      if (w) this.fillRect(x, y - 40, w, 52); // ink across the whole ADVANCE; the cell's padding stays clear, as with a real glyph
      else fill.call(this, str, x, y);
    };
  });

  // 7) the atlas reset. `before` = a HUD-like frame on a fresh-ish atlas.
  const hud = ['HEALTH: 100', 'AMMO 12/12', 'PURGE THE FLOOR — 3 LEFT', 'EXIT'];
  // Sizes on both sides of the atlas's 48 px: MAGNIFIED text (the last line)
  // is the one whose bilinear footprint reaches past its cell's edge, i.e.
  // the one that would pick up whatever lies next to the cell in the atlas.
  const hudCmds = hud.flatMap((_, i) => ops.TEXT(i, 30, 60 + i * 70, i === 3 ? 120 : 40, 1, 0.8, 0.3));
  await draw('hud-before', hud, hudCmds);
  // Flood the atlas with distinct glyphs, 60 a frame, until it has been
  // rebuilt at least once (capacity: 1024 / ~24 px cells x 16 rows ~ 670).
  let cp = 0x4e00; // CJK: never in VT323 — the wide fallback glyphs patched in above
  for (let f = 0; f < 16; f++) {
    let s = '';
    for (let k = 0; k < 60; k++) s += String.fromCodePoint(cp++);
    await draw(`flood-${f}`, [s], ops.TEXT(0, 0, 100, 8, 1, 1, 1));
  }
  await draw('hud-after', hud, hudCmds);
  const after = await t.diff('hud-before', 'hud-after', 1);
  t.report(after.differing === 0, `text after an atlas reset == the same text before (${after.differing} px differ by more than 1, max delta ${after.max})`);
  // The hard case: the reset lands MID-FRAME, between two texts of the same
  // frame. One string of 800 distinct glyphs cannot fit the atlas (~670), so
  // drawing it forces at least one reset whatever the atlas held before; the
  // HUD is drawn before AND after it. (Reference = the HUD drawn twice on a
  // quiet atlas: blending the soft glyph edges twice is brighter than once.)
  await draw('hud-twice', hud, [...hudCmds, ...hudCmds]);
  let spill = '';
  for (let k = 0; k < 800; k++) spill += String.fromCodePoint(cp++);
  await draw('hud-midframe', [...hud, spill], [...hudCmds, ...ops.TEXT(4, 0, 350, 2, 0, 0, 0), ...hudCmds]);
  const mid = await t.diff('hud-twice', 'hud-midframe', 1, [0, 0, W, 300]);
  t.report(mid.differing === 0, `a reset in the middle of a frame leaves that frame's text intact (${mid.differing} px differ by more than 1, max delta ${mid.max})`);
  await draw('hud-settled', hud, hudCmds);
  const settled = await t.diff('hud-before', 'hud-settled', 1);
  t.report(settled.differing === 0, `...and the next frame is clean too (${settled.differing} px differ by more than 1, max delta ${settled.max})`);

  if (process.env.DUMP) for (const n of ['hud-before', 'hud-after', 'hud-midframe']) await t.dump(n, `${process.env.DUMP}/${n}.png`);
  await t.finish();
})();
