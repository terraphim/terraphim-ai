import { test, expect } from '@playwright/test';

test.describe('RoleGraphVisualization Edit Functionality', () => {
	test.beforeEach(async ({ page }) => {
		await page.goto('/');
		// Wait for the main content to load
		await page.waitForSelector('.container.is-fluid');
	});

	test('double-clicking a node opens the modal in edit mode', async ({ page }) => {
		// 1. Click the button to navigate to the RoleGraphVisualization
		// Assuming there's a button/link with an ID or a specific text to navigate
		await page.click('a[href="#/rolegraph"]');

		// 2. Wait for the graph container and nodes to be rendered
		await page.waitForSelector('.graph-container svg .nodes circle');

		// 3. Get the first node and double-click it
		const firstNode = page.locator('.graph-container svg .nodes circle').first();
		await firstNode.dblclick();

		// 4. Assert that the ArticleModal is visible
		const modal = page.locator('.modal.is-active');
		await expect(modal).toBeVisible();

		// 5. Assert that the modal is in edit mode by checking for the "Save" button
		const saveButton = modal.locator('button.is-primary');
		await expect(saveButton).toBeVisible();
		await expect(saveButton).toHaveText('Save');

		// 6. Click the "Cancel" button to close the modal
		const cancelButton = modal.locator('button.is-light');
		await expect(cancelButton).toBeVisible();
		await cancelButton.click();

		// 7. Assert that the modal is no longer visible
		await expect(modal).not.toBeVisible();
	});
});
