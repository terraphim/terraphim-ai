import { test, expect } from '@playwright/test';

const ATOMIC_SERVER_URL = 'http://localhost:9883/';
const TERRAPHIM_SERVER_URL = 'http://localhost:8000';

test.describe('Atomic Lib Integration', () => {
	test('should connect to atomic server using atomic-lib', async ({ page }) => {
		console.log('🔍 Testing atomic server connectivity with atomic-lib...');

		// Navigate to a test page
		await page.goto('data:text/html,<html><body><div id="test"></div></body></html>');

		// Test atomic-lib import and basic functionality
		const result = await page.evaluate(async (atomicServerUrl) => {
			try {
				// Import atomic-lib dynamically
				const atomicLib = await import('@tomic/lib');

				console.log('✅ atomic-lib imported successfully');
				console.log('Available exports:', Object.keys(atomicLib));

				// Test if we can create a basic store
				if (atomicLib.Store) {
					const store = new atomicLib.Store({
						serverUrl: atomicServerUrl,
					});

					console.log('✅ Store created successfully');

					return {
						success: true,
						availableExports: Object.keys(atomicLib),
						message: 'Atomic lib import and store creation successful',
					};
				} else {
					return {
						success: false,
						error: 'Store class not found',
						availableExports: Object.keys(atomicLib),
						message: 'Store class not available',
					};
				}
			} catch (error) {
				console.error('❌ Atomic lib error:', error);
				return {
					success: false,
					error: error.message,
					message: 'Atomic lib import failed',
				};
			}
		}, ATOMIC_SERVER_URL);

		console.log('📊 Atomic lib test result:', result);
		expect(result.success).toBeTruthy();
		expect(result.availableExports).toBeTruthy();
	});

	test('should test atomic server connectivity', async ({ page }) => {
		console.log('🔍 Testing atomic server connectivity...');

		// Test direct HTTP connection to atomic server
		const response = await page.request.get(ATOMIC_SERVER_URL);
		console.log('📊 Atomic server response status:', response.status());

		expect(response.ok()).toBeTruthy();
		console.log('✅ Atomic server is accessible');
	});
});
