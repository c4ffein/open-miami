const { test, expect } = require('@playwright/test');
const { collectErrors, installFrameProbe, waitForFrames, waitForFrameTexts, tap } = require('./helpers');

// The two places sound is switched from: the SETTINGS modal (SOUND, and the
// MUSIC level row) and the `?viz` toolbox, whose tab bar mirrors the SOUND
// toggle — a saved SOUND OFF used to silence every play button of `?viz` with
// no hint on screen. Both write the same `om.*` localStorage keys.
const setting = (page, name) => page.evaluate((k) => localStorage.getItem(`om.${k}`), name);
const has = (s) => (texts) => texts.includes(s);

/** Centre of the SOUND button: right-aligned in the `?viz` tab bar. */
const soundButton = (page) => ({ x: page.viewportSize().width - 20 - 79, y: 37 });

async function openViz(page) {
  await installFrameProbe(page);
  // Every AudioContext the page creates, so its state can be read back.
  await page.addInitScript(() => {
    const Real = window.AudioContext;
    window.__audioCtxs = [];
    window.AudioContext = function (...a) {
      const c = new Real(...a);
      window.__audioCtxs.push(c);
      return c;
    };
    window.AudioContext.prototype = Real.prototype;
  });
  await page.goto('/?viz&precompute=0');
  await page.locator('canvas#glcanvas').waitFor({ state: 'visible', timeout: 10000 });
  await waitForFrames(page, 10);
}
const audioStates = (page) => page.evaluate(() => window.__audioCtxs.map((c) => c.state));

test.describe('Open Miami - sound settings', () => {
  // ONE page load, and off the SPRITES tab at once: that tab draws every
  // character live, which under software rendering starves the specs running
  // in parallel (the floor-1 lift run went from 25 s to > 60 s).
  test('?viz shows a saved SOUND OFF on every tab, and toggles it', async ({ page }) => {
    const errors = collectErrors(page);
    await page.addInitScript(() => {
      if (localStorage.getItem('om.sound') === null) localStorage.setItem('om.sound', 'off');
    });
    await openViz(page);
    for (const tab of [1, 2, 3]) {
      await page.mouse.click(20 + tab * 168 + 79, 37); // MUSICS / LEVELS / EFFECTS
      await waitForFrames(page, 3);
      await waitForFrameTexts(page, has('SOUND: OFF'), { what: `the saved SOUND: OFF on tab ${tab}` });
    }
    await page.mouse.click(20 + 168 + 79, 37); // back to MUSICS (a light tab)
    expect(await audioStates(page)).toEqual(['suspended']);

    const at = soundButton(page);
    await page.mouse.click(at.x, at.y);
    await waitForFrameTexts(page, has('SOUND: ON'), { what: 'SOUND: ON after a click' });
    expect(await setting(page, 'sound')).toBe('on');
    // The click is a user gesture: switching sound on unlocks the context.
    await expect.poll(() => audioStates(page)).toEqual(['running']);

    await page.mouse.click(at.x, at.y);
    await waitForFrameTexts(page, has('SOUND: OFF'), { what: 'SOUND: OFF after a second click' });
    expect(await setting(page, 'sound')).toBe('off');
    await expect.poll(() => audioStates(page)).toEqual(['suspended']);
    expect(errors).toEqual([]);
  });

  test('SETTINGS has a MUSIC level that cycles and persists', async ({ page }) => {
    const errors = collectErrors(page);
    await installFrameProbe(page);
    const openSettings = async () => {
      await page.goto('/?precompute=0');
      await page.locator('canvas#glcanvas').waitFor({ state: 'visible', timeout: 10000 });
      await waitForFrames(page, 10);
      await page.locator('canvas#glcanvas').focus();
      await tap(page, 'ArrowDown'); // PLAY -> SETTINGS
      await tap(page, 'Enter');
      return waitForFrameTexts(page, has('MUSIC'), { what: 'the MUSIC row' });
    };
    let texts = await openSettings();
    expect(texts).toEqual(expect.arrayContaining(['SOUND', 'MUSIC', '100%', 'FPS CAP']));

    await tap(page, 'ArrowDown'); // SOUND -> MUSIC
    await tap(page, 'Enter');
    await waitForFrameTexts(page, has('75%'), { what: 'MUSIC 75%' });
    expect(await setting(page, 'music')).toBe('75');
    for (const pct of ['50%', '25%', '0%', '100%']) {
      await tap(page, 'Enter');
      await waitForFrameTexts(page, has(pct), { what: `MUSIC ${pct}` });
    }
    await tap(page, 'Enter'); // leave it at 75 %
    await waitForFrameTexts(page, has('75%'), { what: 'MUSIC back at 75%' });
    // ArrowUp from the first row wraps to the last one: three rows now.
    await tap(page, 'ArrowUp'); // MUSIC -> SOUND
    await tap(page, 'ArrowUp'); // SOUND -> FPS CAP
    await tap(page, 'ArrowDown'); // FPS CAP -> SOUND
    await tap(page, 'ArrowDown'); // SOUND -> MUSIC: Enter must still act on MUSIC
    await tap(page, 'Enter');
    await waitForFrameTexts(page, has('50%'), { what: 'the wrapped selection acting on MUSIC' });

    texts = await openSettings(); // a fresh page load reads the saved level
    expect(texts).toContain('50%');
    expect(errors).toEqual([]);
  });
});
