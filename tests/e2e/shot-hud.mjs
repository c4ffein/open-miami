// Ad-hoc HUD eyeball harness (not a spec): loads a floor with a frozen
// clock, steps it to chosen times and screenshots the chromatic HUD.
// Usage: bun tests/e2e/shot-hud.mjs
import playwright from './node_modules/playwright/index.mjs';
import { mkdirSync } from 'fs';

const OUT = process.env.OUT_DIR || 'shots';
mkdirSync(OUT, { recursive: true });

const browser = await playwright.chromium.launch({
  args: [
    '--no-sandbox',
    '--disable-setuid-sandbox',
    '--enable-unsafe-swiftshader',
    '--use-gl=angle',
    '--use-angle=swiftshader',
    '--disable-dev-shm-usage',
  ],
});
const page = await browser.newPage({ viewport: { width: 1280, height: 800 } });
page.on('pageerror', (e) => console.log('pageerror:', e.message));
page.on('console', (m) => {
  if (m.type() === 'error') console.log('console.error:', m.text());
});

await page.addInitScript(() => {
  try {
    localStorage.setItem('om.fps_cap', '0');
  } catch (e) {}
});

await page.goto('http://localhost:8102/?floor=2&debug');
await page.waitForSelector('#loading', { state: 'hidden', timeout: 30000 });
await page.waitForTimeout(500);
// Focus + unlock audio/game with a key press.
await page.keyboard.down('s');
await page.waitForTimeout(120);
await page.keyboard.up('s');
await page.waitForTimeout(400);

// Freeze the clock AFTER load: from here each rAF renders at __fakeT.
await page.evaluate(() => {
  window.__realNow = performance.now.bind(performance);
  window.__fakeT = window.__realNow();
  performance.now = () => window.__fakeT;
});
const step = async (ms) => {
  await page.evaluate((d) => {
    window.__fakeT += d;
  }, ms);
  await page.waitForTimeout(80); // a few rAFs render at the frozen time
};

// Let the frozen clock settle a couple frames.
await step(16);
await step(16);

// 1/2: the ammo box + HUD at two chroma steps ~60ms apart.
// Floor 2 spawns armed? If NO GUN, the box still shows... it slides out.
await page.screenshot({ path: `${OUT}/hud-chroma-step-a.png` });
await step(60);
await page.screenshot({ path: `${OUT}/hud-chroma-step-b.png` });

// 3: roller resting with the floor's opening message (it rests 4s; we are
// well inside it at frozen time). Crop the top-right.
await page.screenshot({
  path: `${OUT}/hud-roller-resting.png`,
  clip: { x: 780, y: 0, width: 500, height: 140 },
});

// 4: roller mid-roll: advance past the 4s rest so it rolls away, catch it.
// Small steps — the sim clamps dt to 0.1 s, one big jump would be eaten.
for (let i = 0; i < 50; i++) await step(80); // rest expires (4 s game time)
await step(60);
await step(60); // ~0.12s into the 0.35s roll
await page.screenshot({
  path: `${OUT}/hud-roller-midroll.png`,
  clip: { x: 780, y: 0, width: 500, height: 140 },
});
for (let i = 0; i < 6; i++) await step(80); // fully hidden
await page.screenshot({
  path: `${OUT}/hud-roller-gone.png`,
  clip: { x: 780, y: 0, width: 500, height: 200 },
});

// 5: the rogues counter crop (always visible, top-right under the roller).
await page.screenshot({
  path: `${OUT}/hud-rogues-counter.png`,
  clip: { x: 950, y: 40, width: 330, height: 100 },
});

// 6: ammo box crop at two steps for the zoom check.
await page.screenshot({
  path: `${OUT}/hud-ammo-crop-a.png`,
  clip: { x: 0, y: 640, width: 360, height: 160 },
});
await step(60);
await page.screenshot({
  path: `${OUT}/hud-ammo-crop-b.png`,
  clip: { x: 0, y: 640, width: 360, height: 160 },
});

// Dump the frame texts seen (sanity).
const texts = await page.evaluate(() => {
  return document.querySelector('canvas#glcanvas') ? 'canvas ok' : 'no canvas';
});
console.log(texts);

await browser.close();
console.log('done ->', OUT);
