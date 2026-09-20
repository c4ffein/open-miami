const { test, expect } = require('@playwright/test');
const { collectErrors, installFrameProbe, waitForFrames, loadFloor, tap } = require('./helpers');

// Switching between a menu and a sub-menu must never show a frame of NEITHER:
// every frame of the title and of every modal ends on a POSTFX (13 = the TV
// static, 12 = the modal static that blurs what is behind the panel). A
// transition that switches screens and returns WITHOUT drawing ships a frame
// with no POSTFX at all — the bare backdrop, or (from PAUSED) the raw world
// with no modal and no static: a one-frame flash. So: drive the menus and
// require that NOT ONE frame of the whole sequence lacks its POSTFX.
const bareFrames = (page) =>
  page.evaluate(() => window.__om.log.map((f, i) => ({ i, ...f })).filter((f) => f.postfx === -1));
const mark = (page) => page.evaluate(() => { window.__om.log.length = 0; });

test.describe('Open Miami - menu transitions', () => {
  test('title <-> SETTINGS / ABOUT never shows a bare frame', async ({ page }) => {
    const errors = collectErrors(page);
    await installFrameProbe(page);
    await page.goto('/?precompute=0');
    await page.locator('canvas#glcanvas').waitFor({ state: 'visible', timeout: 10000 });
    await waitForFrames(page, 10);
    await page.locator('canvas#glcanvas').focus();
    await mark(page);

    await tap(page, 'ArrowDown'); // PLAY -> SETTINGS
    await tap(page, 'Enter'); //      open SETTINGS
    await waitForFrames(page, 5);
    await tap(page, 'Escape'); //     back to the title
    await waitForFrames(page, 5);
    await tap(page, 'ArrowDown'); // SETTINGS -> ABOUT
    await tap(page, 'Enter'); //      open ABOUT
    await waitForFrames(page, 5);
    await tap(page, 'Escape'); //     back to the title
    await waitForFrames(page, 5);

    expect(await bareFrames(page)).toEqual([]);
    expect(errors).toEqual([]);
  });

  test('in-game PAUSED <-> SETTINGS <-> game never shows a bare frame', async ({ page }) => {
    const errors = collectErrors(page);
    await loadFloor(page, 2);
    await mark(page);

    await tap(page, 'Escape'); //     pause
    await waitForFrames(page, 5);
    await tap(page, 'ArrowDown'); // CONTINUE -> SETTINGS
    await tap(page, 'Enter'); //      the stacked SETTINGS modal
    await waitForFrames(page, 5);
    await tap(page, 'Escape'); //     pop SETTINGS -> PAUSED
    await waitForFrames(page, 5);
    await tap(page, 'Escape'); //     resume
    await waitForFrames(page, 5);

    expect(await bareFrames(page)).toEqual([]);
    expect(errors).toEqual([]);
  });
});
