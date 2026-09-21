const { defineConfig, devices } = require('@playwright/test');

// Run through `make check-e2e` (repo root): it builds the wasm, installs the
// browser and wraps the whole run in a wall-clock `timeout` (E2E_TIMEOUT,
// 180 s), so specs must stay short (a full floor-1 playthrough is ~15 s, a
// per-floor smoke test ~3 s).
module.exports = defineConfig({
  testDir: './specs',
  fullyParallel: true,
  forbidOnly: !!process.env.CI,
  retries: process.env.CI ? 1 : 0,
  // TWO local workers, not the default (half the cores = 4 here): the pages
  // render in SOFTWARE, so the box is CPU-bound and more workers only stretch
  // every test. Measured, 22 tests on the 8-core dev box: the RUN takes the
  // same ~125 s at 2, 3 and 4 workers, but the longest test (the floor-1 lift
  // run, 24 s alone) takes 34 s at 2 workers, 53 s at 3 and 57 s at 4 — it
  // crossed the 60 s limit the day two tests were added.
  workers: process.env.CI ? 1 : 2,
  reporter: process.env.CI ? [['list'], ['html', { open: 'never' }]] : 'html',
  timeout: 60000, // 60 s per test (the Makefile / CLAUDE.md promise)
  globalTimeout: 10 * 60 * 1000, // whole run: 10 min (retries + serial CI worker)

  use: {
    baseURL: 'http://localhost:8000',
    trace: 'on-first-retry',
    screenshot: 'only-on-failure',
    video: 'retain-on-failure',
  },

  projects: [
    {
      name: 'chromium',
      use: {
        ...devices['Desktop Chrome'],
        launchOptions: {
          // All rendering is WebGL, so headless Chromium must have a GL
          // backend: ANGLE on SwiftShader (software). Needed on a GPU-less
          // local box; harmless on CI's ubuntu runners.
          // Do NOT add `--single-process`: with it, the second page a worker
          // opens after a WebGL page hangs ("Test timeout ... while setting up
          // page"), which is exactly what happens on the serial CI worker.
          args: [
            '--disable-dev-shm-usage',
            '--disable-blink-features=AutomationControlled',
            '--no-sandbox',
            '--disable-setuid-sandbox',
            '--enable-unsafe-swiftshader',
            '--use-gl=angle',
            '--use-angle=swiftshader',
          ],
        },
      },
    },
  ],

  webServer: {
    command: 'python3 -m http.server 8000',
    url: 'http://localhost:8000',
    reuseExistingServer: !process.env.CI,
    cwd: '../..',
  },
});
