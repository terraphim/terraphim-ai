/**
 * Playwright configuration specifically for Novel Editor Autocomplete tests
 * This config extends the base configuration with autocomplete-specific settings
 */

import { defineConfig, devices } from '@playwright/test';
import baseConfig from '../playwright.config';

/**
 * Extended configuration for autocomplete testing
 */
export default defineConfig({
	...baseConfig,

	// Test files specifically for autocomplete
	testDir: './tests/e2e',
	testMatch: '**/novel-autocomplete.spec.ts',

	// Autocomplete tests require more time for service startup
	globalTimeout: process.env.CI ? 15 * 60 * 1000 : 10 * 60 * 1000, // 15min/10min
	timeout: process.env.CI ? 120 * 1000 : 60 * 1000, // 2min/1min per test

	// Expect settings for autocomplete
	expect: {
		// Longer timeout for autocomplete responses
		timeout: process.env.CI ? 30 * 1000 : 15 * 1000,
		// Visual comparison settings
		threshold: 0.2, // Allow some variation in screenshots
		animations: 'disabled', // Disable animations for consistent screenshots
	},

	// Reporter configuration for autocomplete tests
	reporter: process.env.CI
		? [
				['github'],
				[
					'html',
					{
						outputFolder: 'test-results/autocomplete-report',
						open: 'never',
					},
				],
				[
					'json',
					{
						outputFile: 'test-results/autocomplete-results.json',
					},
				],
				[
					'junit',
					{
						outputFile: 'test-results/autocomplete-junit.xml',
					},
				],
			]
		: [
				[
					'html',
					{
						outputFolder: 'test-results/autocomplete-report',
						open: 'on-failure',
					},
				],
				['list', { printSteps: true }],
			],

	// Use settings with autocomplete-specific configurations
	use: {
		...baseConfig.use,

		// Extended timeouts for autocomplete operations
		actionTimeout: process.env.CI ? 60 * 1000 : 30 * 1000,
		navigationTimeout: process.env.CI ? 90 * 1000 : 60 * 1000,

		// Screenshot settings for autocomplete UI
		screenshot: {
			mode: 'only-on-failure',
			fullPage: true,
		},

		// Video recording for debugging autocomplete interactions
		video: {
			mode: process.env.CI ? 'retain-on-failure' : 'off',
			size: { width: 1280, height: 720 },
		},

		// Trace collection for autocomplete debugging
		trace: process.env.CI ? 'retain-on-failure' : 'on-first-retry',

		// Custom user agent for autocomplete tests
		userAgent: 'Playwright Autocomplete Tests',

		// Viewport optimized for editor testing
		viewport: { width: 1440, height: 900 },

		// Ignore HTTPS errors (for local testing)
		ignoreHTTPSErrors: true,

		// Accept downloads (for test artifacts)
		acceptDownloads: true,

		// Extra HTTP headers for autocomplete tests
		extraHTTPHeaders: {
			'X-Test-Suite': 'Novel-Autocomplete',
			'X-Test-Environment': process.env.CI ? 'CI' : 'Local',
		},
	},

	// Global setup for autocomplete tests
	globalSetup: require.resolve('./global-setup-autocomplete.ts'),
	globalTeardown: require.resolve('./global-teardown-autocomplete.ts'),

	// Projects for different autocomplete test scenarios
	projects: [
		// Desktop Chrome - Primary autocomplete testing
		{
			name: 'autocomplete-chrome',
			use: {
				...devices['Desktop Chrome'],
				// Chrome-specific settings for autocomplete
				launchOptions: {
					args: [
						'--disable-web-security', // For MCP server CORS
						'--disable-features=VizDisplayCompositor', // Better for CI
						'--disable-backgrounding-occluded-windows',
						'--disable-renderer-backgrounding',
						'--disable-background-timer-throttling',
						'--disable-ipc-flooding-protection',
					],
				},
			},
		},

		// Desktop Firefox - Cross-browser autocomplete testing
		{
			name: 'autocomplete-firefox',
			use: {
				...devices['Desktop Firefox'],
				// Firefox-specific settings
				launchOptions: {
					firefoxUserPrefs: {
						'dom.webnotifications.enabled': false,
						'media.navigator.permission.disabled': true,
					},
				},
			},
			dependencies: ['autocomplete-chrome'], // Run after Chrome tests
		},

		// Desktop Safari - WebKit testing (macOS only)
		{
			name: 'autocomplete-safari',
			use: { ...devices['Desktop Safari'] },
			dependencies: ['autocomplete-chrome'],
			// Skip Safari on CI unless explicitly requested
			...(process.env.CI && !process.env.TEST_SAFARI
				? {
						testIgnore: '**/*',
					}
				: {}),
		},

		// Mobile Chrome - Touch interaction testing
		{
			name: 'autocomplete-mobile-chrome',
			use: {
				...devices['Pixel 5'],
				// Mobile-specific autocomplete settings
				hasTouch: true,
				isMobile: true,
			},
			dependencies: ['autocomplete-chrome'],
			// Skip mobile tests by default unless requested
			...(process.env.TEST_MOBILE
				? {}
				: {
						testIgnore: '**/*',
					}),
		},

		// Performance testing project
		{
			name: 'autocomplete-performance',
			use: {
				...devices['Desktop Chrome'],
				// Performance testing specific settings
				launchOptions: {
					args: [
						'--no-sandbox',
						'--disable-setuid-sandbox',
						'--disable-dev-shm-usage',
						'--memory-pressure-off',
					],
				},
			},
			testMatch: '**/novel-autocomplete.spec.ts',
			grep: /@performance/,
			dependencies: ['autocomplete-chrome'],
		},

		// Visual regression testing project
		{
			name: 'autocomplete-visual',
			use: {
				...devices['Desktop Chrome'],
				// Consistent settings for visual testing
				deviceScaleFactor: 1,
				viewport: { width: 1280, height: 720 },
			},
			testMatch: '**/novel-autocomplete.spec.ts',
			grep: /@visual/,
			dependencies: ['autocomplete-chrome'],
			expect: {
				// Stricter visual comparison settings
				threshold: 0.1,
				animations: 'disabled',
			},
		},
	],

	// Web server for autocomplete tests
	webServer: {
		command: process.env.CI
			? 'yarn run dev --host' // Expose to all interfaces in CI
			: 'yarn run dev',
		port: 5173,
		reuseExistingServer: !process.env.CI,
		timeout: 120 * 1000, // 2 minutes for server startup
		// Environment variables for the web server
		env: {
			VITE_MCP_SERVER_URL: process.env.MCP_SERVER_URL || 'http://localhost:8001',
			VITE_TEST_MODE: 'true',
			VITE_AUTOCOMPLETE_DEBUG: process.env.DEBUG ? 'true' : 'false',
		},
	},

	// Parallel execution settings for autocomplete tests
	fullyParallel: false, // Autocomplete tests may interfere with each other
	workers: process.env.CI ? 1 : 2, // Limited workers for MCP server

	// Retry configuration
	retries: process.env.CI ? 3 : 1,

	// Output directory for autocomplete test artifacts
	outputDir: 'test-results/autocomplete-artifacts',

	// Metadata for test reporting
	metadata: {
		testSuite: 'Novel Editor Autocomplete',
		version: process.env.npm_package_version || '1.0.0',
		environment: process.env.CI ? 'CI' : 'Local',
		mcpServerPort: process.env.MCP_SERVER_PORT || '8001',
		timestamp: new Date().toISOString(),
	},
});
