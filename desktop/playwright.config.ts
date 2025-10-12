import { defineConfig, devices } from '@playwright/test';
import { fileURLToPath } from 'url';
import { dirname, join } from 'path';

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);

/**
 * @see https://playwright.dev/docs/test-configuration
 */
export default defineConfig({
  testDir: './tests/e2e',
  /* Run tests in files in parallel - disabled in CI for stability */
  fullyParallel: !process.env.CI,
  /* Fail the build on CI if you accidentally left test.only in the source code. */
  forbidOnly: !!process.env.CI,
  /* Retry on CI only */
  retries: process.env.CI ? 3 : 0,
  /* Opt out of parallel tests on CI. */
  workers: process.env.CI ? 1 : undefined,
  /* Reporter to use. See https://playwright.dev/docs/test-reporters */
  reporter: process.env.CI
    ? [['github'], ['html'], ['json', { outputFile: 'test-results/results.json' }]]
    : [['html'], ['list']],
  /* Shared settings for all the projects below. See https://playwright.dev/docs/api/class-testoptions. */
  use: {
    /* Base URL to use in actions like `await page.goto('/')`. */
    baseURL: 'http://localhost:5173',
    /* Collect trace when retrying the failed test. See https://playwright.dev/docs/trace-viewer */
    trace: process.env.CI ? 'retain-on-failure' : 'on-first-retry',
    /* Take screenshot on failure */
    screenshot: 'only-on-failure',
    /* Record video on failure */
    video: process.env.CI ? 'retain-on-failure' : 'off',
    /* CI-specific settings */
    ...(process.env.CI && {
      // Headless mode for CI
      headless: true,
      // Slower actions for CI stability
      actionTimeout: 60000,
      navigationTimeout: 60000,
      // Additional CI-friendly browser args
      launchOptions: {
        args: [
          '--disable-animations',
          '--disable-background-timer-throttling',
          '--disable-backgrounding-occluded-windows',
          '--disable-renderer-backgrounding',
          '--no-sandbox',
          '--disable-setuid-sandbox',
          '--disable-dev-shm-usage',
        ],
      },
    }),
  },

  projects: [
    {
      name: 'Desktop App Tests',
      testDir: './tests/e2e',
      use: {
        ...devices['Desktop Chrome'],
        // Custom settings for desktop app testing
        viewport: { width: 1280, height: 720 },
        // Extend timeout for app startup
        actionTimeout: process.env.CI ? 60000 : 30000,
        navigationTimeout: process.env.CI ? 60000 : 30000,
        // Consistent locale for CI
        locale: 'en-US',
        timezoneId: 'UTC',
      },
    },

    {
      name: 'Visual Regression Tests',
      testDir: './tests/visual',
      use: {
        ...devices['Desktop Chrome'],
        viewport: { width: 1280, height: 720 },
        actionTimeout: process.env.CI ? 60000 : 30000,
        navigationTimeout: process.env.CI ? 60000 : 30000,
      },
    },
  ],

  /* Run your local dev server before starting the tests */
  webServer: process.env.CI ? undefined : {
    command: 'yarn run dev',
    url: 'http://localhost:5173',
    timeout: 180 * 1000,
    reuseExistingServer: !process.env.CI,
  },

  /* Global test timeout - increased for CI */
  timeout: process.env.CI ? 120000 : 60000,

  /* Global setup and teardown */
  globalSetup: join(__dirname, 'tests/global-setup.ts'),
  globalTeardown: join(__dirname, 'tests/global-teardown.ts'),

  /* Configure folders */
  outputDir: 'test-results/',

  /* Expect configuration */
  expect: {
    /* Maximum time expect() should wait for the condition to be met */
    timeout: process.env.CI ? 20000 : 10000,
    /* Threshold for visual comparisons */
    threshold: 0.2,
  },

  /* CI-specific configurations */
  ...(process.env.CI && {
    // Preserve test artifacts in CI
    preserveOutput: 'always',
    // Report slow tests
    reportSlowTests: {
      max: 5,
      threshold: 30000,
    },
  }),
});
