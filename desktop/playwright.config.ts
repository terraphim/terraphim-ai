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
  /* Run tests in files in parallel */
  fullyParallel: true,
  /* Fail the build on CI if you accidentally left test.only in the source code. */
  forbidOnly: !!process.env.CI,
  /* Retry on CI only */
  retries: process.env.CI ? 2 : 0,
  /* Opt out of parallel tests on CI. */
  workers: process.env.CI ? 1 : undefined,
  /* Reporter to use. See https://playwright.dev/docs/test-reporters */
  reporter: process.env.CI 
    ? [['html'], ['github'], ['json', { outputFile: 'test-results/results.json' }]] 
    : [['html'], ['list']],
  /* Shared settings for all the projects below. See https://playwright.dev/docs/api/class-testoptions. */
  use: {
    /* Base URL to use in actions like `await page.goto('/')`. */
    baseURL: 'http://localhost:5173',
    /* Collect trace when retrying the failed test. See https://playwright.dev/docs/trace-viewer */
    trace: process.env.CI ? 'on-first-retry' : 'off',
    /* Take screenshot on failure */
    screenshot: 'only-on-failure',
    /* Record video on failure */
    video: process.env.CI ? 'retain-on-failure' : 'off',
    /* CI-specific settings */
    ...(process.env.CI && {
      // Headless mode for CI
      headless: true,
      // Slower actions for CI stability
      actionTimeout: 45000,
      navigationTimeout: 45000,
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
        actionTimeout: process.env.CI ? 45000 : 30000,
        navigationTimeout: process.env.CI ? 45000 : 30000,
      },
    },
    
    {
      name: 'Visual Regression Tests',
      testDir: './tests/visual',
      use: {
        ...devices['Desktop Chrome'],
        viewport: { width: 1280, height: 720 },
      },
    },
  ],

  /* Run your local dev server before starting the tests */
  webServer: process.env.CI ? undefined : {
    command: 'yarn run dev',
    url: 'http://localhost:5173',
    timeout: 180 * 1000, // Increased timeout for CI
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
    timeout: process.env.CI ? 15000 : 10000,
    /* Threshold for visual comparisons */
    threshold: 0.2,
  },
  
  /* CI-specific configurations */
  ...(process.env.CI && {
    // More verbose output in CI
    reporter: [['html'], ['github'], ['json', { outputFile: 'test-results/results.json' }]],
    // Preserve test artifacts in CI
    preserveOutput: 'always',
    // Report slow tests
    reportSlowTests: {
      max: 5,
      threshold: 15000,
    },
  }),
}); 