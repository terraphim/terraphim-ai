import { test, expect } from '@playwright/test';

const TEST_CONFIG = {
	APP_URL: 'http://localhost:5173',
	SEARCH_TIMEOUT: 10000,
	TEST_ROLE: 'Engineer',
};

test.describe('KG Thesaurus JSON Content', () => {
	test.beforeEach(async ({ page }) => {
		await page.goto(TEST_CONFIG.APP_URL);
		await page.waitForLoadState('networkidle');
		// Wait for the page to be ready - look for any element that indicates the app is loaded
		await page.waitForSelector('body', { timeout: TEST_CONFIG.SEARCH_TIMEOUT });
		// Give the app a moment to initialize
		await page.waitForTimeout(2000);
	});

	test('should add full thesaurus JSON content to conversation context', async ({ page }) => {
		console.log('🧪 Testing thesaurus JSON content addition...');

		// Debug: Check what's on the page
		const pageContent = await page.textContent('body');
		console.log('Page content preview:', pageContent?.substring(0, 500));

		// Check if KG search button exists
		const kgButton = page.locator('[data-testid="kg-search-button"]');
		const isVisible = await kgButton.isVisible();
		console.log('KG search button visible:', isVisible);

		if (!isVisible) {
			// Try to find any button with "KG" text
			const kgTextButton = page.locator('button:has-text("KG")');
			const kgTextVisible = await kgTextButton.isVisible();
			console.log('KG text button visible:', kgTextVisible);
		}

		// Open KG search modal
		await page.click('[data-testid="kg-search-button"]');
		await page.waitForSelector('[data-testid="kg-search-modal"]');

		// Click add thesaurus button
		const addIndexButton = page.locator('[data-testid="kg-add-index-button"]');
		await expect(addIndexButton).toBeVisible();
		await addIndexButton.click();

		// Verify modal closes
		await expect(page.locator('[data-testid="kg-search-modal"]')).toBeHidden({ timeout: 3000 });

		// Wait for context to load
		await page.waitForTimeout(2000);

		// Verify context panel shows thesaurus content
		const contextPanel = page.locator('[data-testid="context-panel"]');
		await expect(contextPanel).toBeVisible({ timeout: 5000 });

		// Verify the context item contains JSON-like content
		const contextText = await contextPanel.textContent();
		expect(contextText).toContain('{');
		expect(contextText).toContain('}');
		expect(contextText).toContain('"');

		// Verify it contains thesaurus-related terms
		expect(contextText).toMatch(/definition|synonyms|related_terms|usage_examples|metadata/i);

		// Verify statistics are still shown
		expect(contextText).toMatch(/\d+ terms/);
		expect(contextText).toMatch(/\d+ nodes/);
		expect(contextText).toMatch(/\d+ edges/);

		console.log('✅ Thesaurus JSON content successfully added to context');
	});

	test('should display thesaurus data notification in context item', async ({ page }) => {
		console.log('🧪 Testing thesaurus data notification...');

		// Add thesaurus content
		await page.click('[data-testid="kg-search-button"]');
		await page.waitForSelector('[data-testid="kg-search-modal"]');
		await page.click('[data-testid="kg-add-index-button"]');
		await expect(page.locator('[data-testid="kg-search-modal"]')).toBeHidden({ timeout: 3000 });

		// Wait for context to load
		await page.waitForTimeout(2000);

		// Verify thesaurus data notification is shown
		const contextPanel = page.locator('[data-testid="context-panel"]');
		await expect(contextPanel).toContainText('Thesaurus Data', { timeout: 5000 });
		await expect(contextPanel).toContainText('complete thesaurus as JSON', { timeout: 5000 });
		await expect(contextPanel).toContainText('comprehensive AI understanding', { timeout: 5000 });

		console.log('✅ Thesaurus data notification displayed correctly');
	});
});
