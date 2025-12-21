/**
 * Playwright configuration for Context Management UI tests
 *
 * This configuration is specifically tailored for testing the conversation and context
 * management functionality in Terraphim AI. It includes setup for the desktop application,
 * backend services, and appropriate test environments.
 */

import { defineConfig, devices } from '@playwright/test';
import path from 'path';

// Environment configuration
const CI = process.env.CI === 'true';
const DEBUG = process.env.DEBUG === 'true';
const TEST_MOBILE = process.env.TEST_MOBILE === 'true';
const TEST_SAFARI = process.env.TEST_SAFARI === 'true' || !CI;

// Server configuration
const FRONTEND_PORT = process.env.FRONTEND_PORT || '5173';
const BACKEND_PORT = process.env.BACKEND_PORT || '8000';
const MCP_SERVER_PORT = process.env.MCP_SERVER_PORT || '8001';

export default defineConfig({
	testDir: './e2e',
	testMatch: '**/context-management.spec.ts',

	// Global configuration
	timeout: CI ? 120_000 : 60_000, // 2 min in CI, 1 min locally
	globalTimeout: CI ? 900_000 : 600_000, // 15 min in CI, 10 min locally
	expect: {
		timeout: CI ? 30_000 : 15_000,
	},

	// Test execution settings
	fullyParallel: !CI, // Run in parallel locally, serial in CI for reliability
	forbidOnly: CI,
	retries: CI ? 2 : 1,
	workers: CI ? 1 : undefined,

	// Test artifacts
	use: {
		// Base URL for the frontend application
		baseURL: `http://localhost:${FRONTEND_PORT}`,

		// Global test configuration
		actionTimeout: CI ? 60_000 : 30_000,
		navigationTimeout: CI ? 60_000 : 30_000,

		// Capture screenshots and videos on failure
		screenshot: 'only-on-failure',
		video: 'retain-on-failure',
		trace: 'retain-on-failure',

		// Browser configuration
		ignoreHTTPSErrors: true,
		colorScheme: 'light',
	},

	// Test projects for different browsers and scenarios
	projects: [
		// Primary desktop testing
		{
			name: 'context-chrome',
			use: {
				...devices['Desktop Chrome'],
				// Disable web security for Tauri integration testing
				launchOptions: {
					args: [
						'--disable-web-security',
						'--disable-features=VizDisplayCompositor',
						'--disable-background-timer-throttling',
						'--disable-backgrounding-occluded-windows',
						'--disable-renderer-backgrounding',
					],
				},
			},
		},

		// Firefox cross-browser testing
		{
			name: 'context-firefox',
			use: {
				...devices['Desktop Firefox'],
				// Firefox specific configuration for Tauri
				launchOptions: {
					firefoxUserPrefs: {
						'dom.webnotifications.enabled': false,
						'permissions.default.desktop-notification': 2,
					},
				},
			},
		},

		// WebKit/Safari testing (macOS only)
		...(TEST_SAFARI
			? [
					{
						name: 'context-safari',
						use: {
							...devices['Desktop Safari'],
						},
					},
				]
			: []),

		// Mobile testing
		...(TEST_MOBILE
			? [
					{
						name: 'context-mobile-chrome',
						use: {
							...devices['Pixel 5'],
						},
					},
					{
						name: 'context-mobile-safari',
						use: {
							...devices['iPhone 12'],
						},
					},
				]
			: []),

		// Performance testing
		{
			name: 'context-performance',
			use: {
				...devices['Desktop Chrome'],
				launchOptions: {
					args: ['--enable-precise-memory-info', '--enable-memory-pressure-testing'],
				},
			},
			grep: /@performance/,
		},

		// Visual regression testing
		{
			name: 'context-visual',
			use: {
				...devices['Desktop Chrome'],
			},
			grep: /@visual/,
		},
	],

	// Global setup and teardown
	globalSetup: path.resolve(__dirname, 'global-setup-context.ts'),
	globalTeardown: path.resolve(__dirname, 'global-teardown-context.ts'),

	// Reporters
	reporter: [
		['list', { printSteps: DEBUG }],
		[
			'html',
			{
				outputFolder: 'test-results/context-report',
				open: DEBUG ? 'always' : 'never',
			},
		],
		...(CI
			? [
					['github'],
					['json', { outputFile: 'test-results/context-results.json' }],
					['junit', { outputFile: 'test-results/context-junit.xml' }],
				]
			: []),
	],

	// Output directory for test artifacts
	outputDir: 'test-results/context-artifacts',

	// Web server configuration for frontend
	webServer: [
		// Frontend development server
		{
			command: 'yarn dev',
			port: parseInt(FRONTEND_PORT),
			cwd: path.resolve(__dirname, '..'),
			timeout: 120_000,
			env: {
				NODE_ENV: 'test',
				VITE_BACKEND_URL: `http://localhost:${BACKEND_PORT}`,
				VITE_MCP_SERVER_URL: `http://localhost:${MCP_SERVER_PORT}`,
			},
			reuseExistingServer: !CI,
		},

		// Backend server (optional - may be started in global setup)
		{
			command: `cargo run --bin terraphim_server -- --port ${BACKEND_PORT}`,
			port: parseInt(BACKEND_PORT),
			cwd: path.resolve(__dirname, '../..'),
			timeout: 180_000,
			env: {
				RUST_LOG: DEBUG ? 'debug' : 'warn',
				TERRAPHIM_TEST_MODE: 'true',
			},
			reuseExistingServer: !CI,
		},
	],
});
