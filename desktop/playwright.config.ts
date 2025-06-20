import { defineConfig, devices } from '@playwright/test';

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
    ? [['html'], ['github']] 
    : [['html'], ['list']],
  /* Shared settings for all the projects below. See https://playwright.dev/docs/api/class-testoptions. */
  use: {
    /* Base URL to use in actions like `await page.goto('/')`. */
    baseURL: 'http://localhost:1420',
    /* Collect trace when retrying the failed test. See https://playwright.dev/docs/trace-viewer */
    trace: 'on-first-retry',
    /* Take screenshot on failure */
    screenshot: 'only-on-failure',
    /* Record video on failure */
    video: 'retain-on-failure',
  },

  projects: [
    {
      name: 'Desktop App Tests',
      testDir: './tests/e2e',
      use: { 
        ...devices['Desktop Chrome'],
        // Custom settings for desktop app testing
        viewport: { width: 1280, height: 720 },
        // Extend timeout for Tauri app startup
        actionTimeout: 30000,
        navigationTimeout: 30000,
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
    command: 'yarn tauri dev',
    url: 'http://localhost:1420',
    timeout: 120 * 1000,
    reuseExistingServer: !process.env.CI,
  },
  
  /* Global test timeout */
  timeout: 60000,
  
  /* Global setup and teardown */
  globalSetup: require.resolve('./tests/global-setup.ts'),
  globalTeardown: require.resolve('./tests/global-teardown.ts'),
  
  /* Configure folders */
  outputDir: 'test-results/',
  
  /* Expect configuration */
  expect: {
    /* Maximum time expect() should wait for the condition to be met */
    timeout: 10000,
    /* Threshold for visual comparisons */
    threshold: 0.2,
  },
}); 