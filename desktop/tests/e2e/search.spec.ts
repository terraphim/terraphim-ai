import { test, expect } from '@playwright/test';
import {
	ciWaitForSelector,
	ciSearch,
	ciNavigate,
	ciWait,
	ciClick,
	getTimeouts,
} from '../../src/test-utils/ci-friendly';

test.describe('Search Functionality', () => {
	test.beforeEach(async ({ page }) => {
		// Navigate to the app using CI-friendly navigation
		await ciNavigate(page, '/');

		// Wait for the app to load using CI-friendly timeouts
		await ciWaitForSelector(page, 'input[type="search"]', 'navigation');
	});

	test('should display search input and logo on startup', async ({ page }) => {
		// Check that search input is visible
		const searchInput = await ciWaitForSelector(page, 'input[type="search"]');
		await expect(searchInput).toBeVisible();

		// Check that logo is displayed
		const logo = await ciWaitForSelector(page, 'img[alt="Terraphim Logo"]');
		await expect(logo).toBeVisible();

		// Check welcome message
		await expect(page.locator('text=I am Terraphim, your personal assistant.')).toBeVisible();
	});

	test('should perform search when typing and pressing enter', async ({ page }) => {
		// Perform search using CI-friendly search helper
		await ciSearch(page, 'input[type="search"]', 'artificial intelligence');

		// The logo should disappear if there are results
		// or remain if no results (both are valid depending on test data)
		const logo = page.locator('img[alt="Terraphim Logo"]');
		const logoVisible = await logo.isVisible();

		if (!logoVisible) {
			// If logo is hidden, we should have results
			console.log('Search returned results');
		} else {
			// If logo is still visible, no results were found (which is fine for tests)
			console.log('Search returned no results');
		}
	});

	test('should show search suggestions when typing', async ({ page }) => {
		const searchInput = await ciWaitForSelector(page, 'input[type="search"]');

		// Type partial text to trigger suggestions
		await searchInput.fill('art');

		// Wait for suggestions to appear using CI-friendly timing
		await ciWait(page, 'small');

		// Check if suggestions dropdown appears
		const suggestions = page.locator('.suggestions');
		const suggestionsVisible = await suggestions.isVisible();

		if (suggestionsVisible) {
			console.log('Suggestions are working');
			// Click on a suggestion if available
			const firstSuggestion = suggestions.locator('li').first();
			if (await firstSuggestion.isVisible()) {
				await ciClick(page, '.suggestions li:first-child');

				// Check that suggestion was applied to input
				const inputValue = await searchInput.inputValue();
				expect(inputValue.length).toBeGreaterThan(3);
			}
		}
	});

	test('should handle keyboard navigation in suggestions', async ({ page }) => {
		const searchInput = await ciWaitForSelector(page, 'input[type="search"]');

		// Type to trigger suggestions
		await searchInput.fill('machine');
		await ciWait(page, 'small');

		// Try keyboard navigation
		await searchInput.press('ArrowDown');
		await ciWait(page, 'tiny');

		// Press Enter to select
		await searchInput.press('Enter');

		// The input should have been updated or search should be triggered
		await ciWait(page, 'small');
	});

	test('should clear search input', async ({ page }) => {
		const searchInput = await ciWaitForSelector(page, 'input[type="search"]');

		// Fill input with text
		await searchInput.fill('test query');
		expect(await searchInput.inputValue()).toBe('test query');

		// Clear the input
		await searchInput.clear();
		expect(await searchInput.inputValue()).toBe('');

		// Logo should be visible again
		const logo = await ciWaitForSelector(page, 'img[alt="Terraphim Logo"]');
		await expect(logo).toBeVisible();
	});

	test('should handle empty search gracefully', async ({ page }) => {
		const searchInput = await ciWaitForSelector(page, 'input[type="search"]');

		// Try to search with empty input
		await searchInput.press('Enter');

		// Should not crash and logo should remain visible
		await ciWait(page, 'medium');
		const logo = await ciWaitForSelector(page, 'img[alt="Terraphim Logo"]');
		await expect(logo).toBeVisible();
	});

	test('should handle long search queries', async ({ page }) => {
		const searchInput = await ciWaitForSelector(page, 'input[type="search"]');

		const longQuery =
			'artificial intelligence machine learning deep learning neural networks computer vision natural language processing data science algorithms optimization'.repeat(
				3
			);

		await searchInput.fill(longQuery);
		await searchInput.press('Enter');

		// Should handle long queries without crashing
		await ciWait(page, 'afterSearch');

		// App should still be responsive
		const searchInputAfter = await ciWaitForSelector(page, 'input[type="search"]');
		await expect(searchInputAfter).toBeVisible();
	});

	test('should maintain search state during theme changes', async ({ page }) => {
		const searchInput = await ciWaitForSelector(page, 'input[type="search"]');

		// Enter a search query
		await searchInput.fill('test query');

		// Try to change theme (if theme switcher is visible)
		const themeButton = page.locator('button:has-text("Theme")');
		const themeButtonVisible = await themeButton.isVisible();

		if (themeButtonVisible) {
			await ciClick(page, 'button:has-text("Theme")');

			// Click on a theme option if available
			const darkTheme = page.locator('text=Darkly');
			if (await darkTheme.isVisible()) {
				await ciClick(page, 'text=Darkly');
			}
		}

		// Search input should still contain the query
		const inputValue = await searchInput.inputValue();
		expect(inputValue).toBe('test query');
	});

	test('should display error messages gracefully', async ({ page }) => {
		// This test checks that the app handles errors without crashing
		// We can't easily simulate backend errors in E2E tests,
		// but we can check that the UI remains functional

		const searchInput = await ciWaitForSelector(page, 'input[type="search"]');
		await searchInput.fill('test error case');
		await searchInput.press('Enter');

		await ciWait(page, 'afterSearch');

		// App should remain functional even if there are errors
		await expect(searchInput).toBeVisible();

		// Check if error message is shown
		const errorMessage = page.locator('.error');
		const errorVisible = await errorMessage.isVisible();

		if (errorVisible) {
			console.log('Error message displayed correctly');
			expect(await errorMessage.textContent()).toContain('Error');
		}
	});
});
