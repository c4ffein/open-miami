const { test, expect } = require('@playwright/test');
const {
  hold,
  collectErrors,
  loadFloor,
  lastFrameTexts,
  waitForFrameTexts,
  rogueCount,
  playerPos,
  purgeRogues,
  walkFloor1ToServiceLift,
  expectFloor2,
} = require('./helpers');

// Actually plays the game: `/?floor=1&debug` drops us straight onto floor 1
// (no title / level-select), the frame probe reads the WebGL HUD text, and the
// run ends by riding the SERVICE LIFT to floor 2. See helpers.js for the
// `?floor=N&debug` + held-keys conventions.
test.describe('Open Miami - Level Completion', () => {
  test('game loads on floor 1 without errors and draws the HUD', async ({ page }) => {
    const errors = collectErrors(page);

    await loadFloor(page, 1);

    // The in-game HUD, not the title/level-select screen: the top-right
    // "N ROGUES" chromatic counter (HEALTH:/ROGUES: rows are gone; the held
    // weapon lives in the sliding bottom-left ammo box). Floor 1 opens on a
    // passive lobby crowd: bystanders are not rogues until the scenario
    // alerts them, so the count starts at 0.
    const texts = await lastFrameTexts(page);
    expect(rogueCount(texts)).toBe(0);
    // Floor 1's opening directive rolls down top-right for ~4 s on load.
    await waitForFrameTexts(page, (t) => t.includes('PASS THE CHECKPOINT'), {
      what: "floor 1's PASS THE CHECKPOINT roller",
    });

    await page.screenshot({ path: 'test-results/01-floor1-loaded.png' });

    // WebGL is required: no error of any kind is tolerated.
    expect(errors).toEqual([]);
  });

  test('player moves, purges the rogues and takes the SERVICE LIFT to floor 2', async ({ page }) => {
    const errors = collectErrors(page);
    const canvas = await loadFloor(page, 1);

    // Debug overlay on (I), then K purges every rogue -> Rogues: 0 and the
    // all-dead scenario step opens the SERVICE LIFT. Done first so nothing
    // shoots at us while we walk.
    await purgeRogues(page);
    await page.screenshot({ path: 'test-results/02-rogues-purged.png' });

    // Move north a bit (the MAIN DOORS entry is bottom centre (500,750)): the player's world
    // position changes and, as the camera follows the player, so does the frame.
    const posBefore = await playerPos(page);
    const before = await canvas.screenshot();
    await hold(page, 'w', 400);
    const after = await canvas.screenshot();
    const posAfter = await playerPos(page);
    expect(posBefore).not.toBeNull();
    expect(posAfter).not.toBeNull();
    expect(posAfter.y).toBeLessThan(posBefore.y - 10);
    expect(after.equals(before)).toBe(false);
    await page.screenshot({ path: 'test-results/03-moved.png' });
    let texts = await lastFrameTexts(page);
    expect(rogueCount(texts)).toBe(0);

    // Through the turnstiles, west along the hall, then north into the open SERVICE LIFT (NW).
    await walkFloor1ToServiceLift(page);
    await page.waitForTimeout(1200); // dwell (0.6s) + the extraction card starts
    await page.screenshot({ path: 'test-results/04-extracting.png' });

    // The card ends -> floor 2 (COLD STORAGE, PURGE THE WARDENS roller) loads.
    texts = await expectFloor2(page);
    expect(rogueCount(texts)).toBeGreaterThan(0);
    await page.screenshot({ path: 'test-results/05-floor2-loaded.png' });

    expect(errors).toEqual([]);
  });
});
