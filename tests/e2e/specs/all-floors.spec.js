const { test, expect } = require('@playwright/test');
const { collectErrors, installFrameProbe, waitForFrames } = require('./helpers');

// EVERY floor, in the real browser: it boots, keeps rendering real frames,
// and nothing errors. One short test per floor so they run in parallel
// locally and a failure names its floor.
//
// What this adds over the native suites: `tests/playthrough_gate.rs` plays
// every floor headlessly and `tests/render_stream.rs` validates every
// floor's recorded command stream — but only a browser runs the JS side: the
// renderer walking that stream, the GLSL compiling, robot-core / shoggoth-core
// drawing, a prop or floor surface the WebGL path chokes on. Floors come from
// levels/index.json, so a new floor is covered the day it is added.
//
// Deliberately NOT asserted: that the player robot is on screen. Floors open
// differently — 0 / 6 / 7 / 13 on a cinematic with the player off-camera
// (culled), 14 on the boss intro card before any world is drawn — so "what
// is visible at frame N" is not an invariant. What IS: frames keep coming,
// every one decodes, each draws something, and the page logs no error.
const FLOORS = require('../../../levels/index.json').floors;
// Past the first frames (static cache recorded, sprites baked). Kept small:
// headless Chromium renders through SOFTWARE GL (SwiftShader) at ~5-15 fps.
const FRAMES = 20;

test.describe('Open Miami - every floor renders', () => {
  for (const { id, name } of FLOORS) {
    test(`floor ${id} (${name}) boots and renders without errors`, async ({ page }) => {
      const errors = collectErrors(page);
      await installFrameProbe(page);
      // precompute=0: do not sit in the audio pre-render gate (docs/URL_PARAMS.md).
      await page.goto(`/?floor=${id}&debug&precompute=0`);
      await page.locator('canvas#glcanvas').waitFor({ state: 'visible', timeout: 10000 });
      await waitForFrames(page, FRAMES);

      const om = await page.evaluate(() => ({
        frames: window.__om.frames,
        cmds: window.__om.cmds,
        malformed: window.__om.malformed,
      }));
      expect(om.frames).toBeGreaterThanOrEqual(FRAMES);
      // The renderer's walk never desynced on any of those frames...
      expect(om.malformed).toBeNull();
      // ...a real frame is being drawn (not an empty / CLEAR-only stream)...
      expect(om.cmds).toBeGreaterThan(20);
      // ...and WebGL is required: no console / page error of any kind.
      expect(errors).toEqual([]);
    });
  }
});
