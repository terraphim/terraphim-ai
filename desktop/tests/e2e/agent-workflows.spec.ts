import { test, expect } from '@playwright/test';

/**
 * Basic E2E tests for Terraphim AI Application
 */

test.describe('Terraphim AI Basic Tests', () => {
	test('should load application without errors', async ({ page }) => {
		await page.goto('/');
		await page.waitForLoadState('networkidle');

		// Check page loaded successfully
		await expect(page.locator('body')).toBeVisible();

		// Check no critical console errors
		const errors: string[] = [];
		page.on('console', (msg) => {
			if (msg.type() === 'error') {
				errors.push(msg.text());
			}
		});

		// Wait for any initial console messages
		await page.waitForTimeout(2000);

		// Filter out expected WebSocket connection messages during startup
		const criticalErrors = errors.filter(
			(error) =>
				!error.includes('WebSocket connection') &&
				!error.includes('Failed to connect') &&
				!error.includes('Connection refused') &&
				!error.includes('NetConnection') &&
				!error.includes('favicon')
		);

		expect(criticalErrors).toHaveLength(0);
	});
});
