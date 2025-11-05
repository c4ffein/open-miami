# End-to-End Tests for Open Miami

This directory contains automated browser-based tests that simulate actual gameplay.

## What These Tests Do

1. **Level Completion Test**: Simulates a player completing a level
   - Moves the player around the map
   - Shoots at enemy positions
   - Verifies the game continues running
   - Takes screenshots at each stage

2. **Death and Restart Test**: Tests player death handling
   - Lets enemies kill the player
   - Tests the restart functionality (R key)
   - Ensures no crashes occur

3. **Error Detection Test**: Checks for JavaScript errors
   - Monitors console for errors
   - Verifies clean game loading

## Running Tests Locally

### Prerequisites
```bash
cd tests/e2e
npm install
npx playwright install chromium
```

### Run Tests

From the repository root:

```bash
# Build WASM first
cargo build --release --target wasm32-unknown-unknown
cp target/wasm32-unknown-unknown/release/open_miami.wasm .

# Run tests
cd tests/e2e
npm test
```

### View Results

After running, check:
- `test-results/` - Screenshots from the test run
- `playwright-report/` - Detailed HTML report

### Run in Headed Mode (See the Browser)

```bash
npm run test:headed
```

### Debug Tests

```bash
npm run test:debug
```

## CI Integration

These tests run automatically on every push and pull request via GitHub Actions.

Results are uploaded as artifacts and commented on pull requests.

## Test Structure

```
tests/e2e/
├── package.json           # Dependencies
├── playwright.config.js   # Playwright configuration
├── specs/
│   └── level-completion.spec.js  # Test scenarios
├── test-results/          # Screenshots (generated)
└── playwright-report/     # HTML reports (generated)
```

## Adding New Tests

1. Create a new `.spec.js` file in `specs/`
2. Use Playwright's API to interact with the game
3. Take screenshots at key moments
4. Add assertions to verify behavior

Example:
```javascript
test('my new test', async ({ page }) => {
  await page.goto('/');
  await page.waitForSelector('canvas#glcanvas');
  const canvas = await page.locator('canvas#glcanvas');
  await canvas.click();

  // Your test logic here

  await page.screenshot({ path: 'test-results/my-test.png' });
});
```

## Tips

- Tests run in a real browser (Chromium)
- The game must be built to WASM before testing
- Screenshots help debug test failures
- Use `page.waitForTimeout()` sparingly - prefer waiting for specific elements
- The game runs at real speed, so timing matters
