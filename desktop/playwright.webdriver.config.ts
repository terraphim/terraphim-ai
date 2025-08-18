import { defineConfig, devices } from '@playwright/test';
import { fileURLToPath } from 'url';
import { dirname, join } from 'path';

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);

/**
 * Playwright configuration for WebDriver-based Tauri tests
 * 
 * This configuration is specifically designed for testing Tauri applications
 * using the official Tauri WebDriver support.
 */

export default defineConfig({
  testDir: './tests/webdriver',
  /* Run tests in files in parallel - disabled for WebDriver tests */
  fullyParallel: false,
  /* Fail the build on CI if you accidentally left test.only in the source code. */
  forbidOnly: !!process.env.CI,
  /* Retry on CI only */
  retries: process.env.CI ? 3 : 0,
  /* Single worker for WebDriver tests */
  workers: 1,
  /* Reporter to use. See https://playwright.dev/docs/test-reporters */
  reporter: process.env.CI 
    ? [['github'], ['html'], ['json', { outputFile: 'test-results/webdriver-results.json' }]] 
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
      name: 'Tauri WebDriver Tests',
      testDir: './tests/webdriver',
      use: { 
        ...devices['Desktop Chrome'],
        // Custom settings for Tauri WebDriver testing
        viewport: { width: 1280, height: 720 },
        // Extend timeout for Tauri app startup
        actionTimeout: process.env.CI ? 60000 : 30000,
        navigationTimeout: process.env.CI ? 60000 : 30000,
        // Consistent locale for CI
        locale: 'en-US',
        timezoneId: 'UTC',
        // WebDriver-specific settings
        launchOptions: {
          args: [
            '--no-sandbox',
            '--disable-dev-shm-usage',
            '--disable-gpu',
            '--disable-web-security',
            '--allow-running-insecure-content',
            '--disable-features=VizDisplayCompositor',
          ],
        },
      },
    },
  ],

  /* Run your local dev server before starting the tests */
  webServer: process.env.CI ? undefined : {
    command: 'yarn tauri dev --port 5173',
    url: 'http://localhost:5173',
    timeout: 180 * 1000,
    reuseExistingServer: !process.env.CI,
  },
  
  /* Global test timeout - increased for WebDriver tests */
  timeout: process.env.CI ? 120000 : 60000,
  
  /* Global setup and teardown */
  globalSetup: join(__dirname, 'tests/webdriver/setup.ts'),
  globalTeardown: join(__dirname, 'tests/webdriver/teardown.ts'),
  
  /* Configure folders */
  outputDir: 'test-results/webdriver/',
  
  /* Expect configuration */
  expect: {
    /* Maximum time expect() should wait for the condition to be met */
    timeout: process.env.CI ? 20000 : 10000,
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