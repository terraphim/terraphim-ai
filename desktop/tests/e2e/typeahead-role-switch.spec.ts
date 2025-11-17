import { expect, test } from '@playwright/test';

const FRONTEND_URL = 'http://localhost:5173';

test.describe('Typeahead role switch and KG autocomplete', () => {
	test.beforeEach(async ({ page }) => {
		await page.goto(FRONTEND_URL);
		await page.waitForSelector('input[type="search"]', { timeout: 30000 });
	});

	test('switching to KG-enabled role renders KG input and shows suggestions', async ({ page }) => {
		// Intercept autocomplete endpoint and return mock suggestions
		await page.route('**/autocomplete/**', async (route) => {
			await route.fulfill({
				status: 200,
				contentType: 'application/json',
				body: JSON.stringify({
					status: 'success',
					suggestions: [
						{ term: 'terraphim-graph' },
						{ term: 'graph embeddings' },
						{ term: 'knowledge graph' },
					],
				}),
			});
		});

		// Switch role to a KG-enabled role (Engineer in test configs)
		const roleSelect = page.locator('select').first();
		await expect(roleSelect).toBeVisible();
		await roleSelect.selectOption('Engineer');

		// KG input should render
		const kgInput = page.locator('[data-testid="kg-search-input"]');
		await expect(kgInput).toBeVisible();

		// Type to trigger suggestions
		await kgInput.fill('gr');

		// Verify suggestions list appears with mocked data
		const suggestions = page.locator(
			'[data-testid="kg-autocomplete-list"] [data-testid="kg-autocomplete-item"]'
		);
		await expect(suggestions.first()).toBeVisible();
		await expect(suggestions).toHaveCount(3);
	});
});
