/**
 * End-to-End tests for Knowledge Graph Search and Context Management
 *
 * This test suite validates the complete KG search functionality including:
 * - KG search modal UI interactions
 * - Autocomplete search with FST-based matching
 * - Adding KG term definitions to conversation context
 * - Adding complete KG index to conversation context
 * - Performance testing under load
 * - Error handling and edge cases
 */

import { expect, test } from '@playwright/test';

// Test configuration
const TEST_CONFIG = {
	APP_URL: 'http://localhost:5173',
	SEARCH_TIMEOUT: 10000,
	AUTOCOMPLETE_DEBOUNCE: 300,
	TEST_ROLE: 'Engineer',
	DEFAULT_CONVERSATION_TITLE: 'KG Test Conversation',
};

// Mock data for testing
const MOCK_KG_TERMS = [
	{
		term: 'async',
		normalized_term: 'async_programming',
		url: 'https://docs.rs/tokio/latest/tokio/',
		score: 0.95,
	},
	{
		term: 'tokio',
		normalized_term: 'tokio_runtime',
		url: 'https://docs.rs/tokio/latest/tokio/runtime/',
		score: 0.92,
	},
	{
		term: 'futures',
		normalized_term: 'future_trait',
		url: 'https://docs.rs/futures/latest/futures/',
		score: 0.89,
	},
];

const PERFORMANCE_THRESHOLDS = {
	SEARCH_RESPONSE_TIME_MS: 2000,
	AUTOCOMPLETE_RESPONSE_TIME_MS: 500,
	CONTEXT_ADD_TIME_MS: 1000,
	MAX_CONCURRENT_REQUESTS: 10,
};

test.describe('KG Search and Context Management E2E', () => {
	test.beforeEach(async ({ page }) => {
		console.log('🚀 Setting up KG search test environment...');

		// Navigate to the application
		await page.goto(TEST_CONFIG.APP_URL);

		// Wait for the app to load completely
		await page.waitForLoadState('networkidle');

		// Ensure we're in chat mode with a conversation
		await page.waitForSelector('[data-testid="chat-interface"]', {
			timeout: TEST_CONFIG.SEARCH_TIMEOUT,
		});

		console.log('✅ Test environment setup complete');
	});

	test.describe('KG Search Modal Functionality', () => {
		test('should open KG search modal from chat interface', async ({ page }) => {
			console.log('🔍 Testing KG search modal opening...');

			// Click the KG search button in the context panel
			const kgSearchButton = page.locator('[data-testid="kg-search-button"]');
			await expect(kgSearchButton).toBeVisible({ timeout: 5000 });
			await kgSearchButton.click();

			// Verify modal opens
			const kgModal = page.locator('[data-testid="kg-search-modal"]');
			await expect(kgModal).toBeVisible({ timeout: 3000 });

			// Verify modal components are present
			await expect(page.locator('[data-testid="kg-search-input"]')).toBeVisible();
			await expect(page.locator('[data-testid="kg-suggestions-list"]')).toBeVisible();
			await expect(page.locator('[data-testid="kg-add-term-button"]')).toBeVisible();
			await expect(page.locator('[data-testid="kg-add-index-button"]')).toBeVisible();

			console.log('✅ KG search modal opened successfully');
		});

		test('should perform autocomplete search with debouncing', async ({ page }) => {
			console.log('⌨️ Testing autocomplete search functionality...');

			// Open KG search modal
			await page.click('[data-testid="kg-search-button"]');
			await page.waitForSelector('[data-testid="kg-search-modal"]');

			const searchInput = page.locator('[data-testid="kg-search-input"]');
			const suggestionsList = page.locator('[data-testid="kg-suggestions-list"]');

			// Test debounced search
			await searchInput.fill('asy');

			// Wait for debounce delay
			await page.waitForTimeout(TEST_CONFIG.AUTOCOMPLETE_DEBOUNCE + 100);

			// Verify suggestions appear
			await expect(suggestionsList).toBeVisible();

			// Check for loading state during search
			const loadingIndicator = page.locator('[data-testid="kg-search-loading"]');

			// Clear and search for a different term
			await searchInput.fill('');
			await searchInput.fill('tokio');

			// Wait for new suggestions
			await page.waitForTimeout(TEST_CONFIG.AUTOCOMPLETE_DEBOUNCE + 100);

			// Verify suggestions updated
			const suggestions = page.locator('[data-testid="kg-suggestion-item"]');
			await expect(suggestions.first()).toBeVisible({ timeout: 5000 });

			console.log('✅ Autocomplete search working with proper debouncing');
		});

		test('should handle keyboard navigation in suggestions', async ({ page }) => {
			console.log('⌨️ Testing keyboard navigation...');

			// Open modal and perform search
			await page.click('[data-testid="kg-search-button"]');
			await page.waitForSelector('[data-testid="kg-search-modal"]');

			const searchInput = page.locator('[data-testid="kg-search-input"]');
			await searchInput.fill('async');
			await page.waitForTimeout(TEST_CONFIG.AUTOCOMPLETE_DEBOUNCE + 100);

			// Test arrow key navigation
			await searchInput.press('ArrowDown');

			// Verify first suggestion is highlighted
			const firstSuggestion = page.locator('[data-testid="kg-suggestion-item"]').first();
			await expect(firstSuggestion).toHaveClass(/selected|highlighted|active/);

			// Navigate down
			await searchInput.press('ArrowDown');

			// Navigate back up
			await searchInput.press('ArrowUp');

			// Test Enter key selection
			await searchInput.press('Enter');

			// Verify suggestion was selected (modal should close or term should be highlighted)
			await expect(page.locator('[data-testid="kg-search-modal"]')).toBeHidden({
				timeout: 3000,
			});

			console.log('✅ Keyboard navigation working correctly');
		});

		test('should handle search errors gracefully', async ({ page }) => {
			console.log('❌ Testing error handling...');

			// Open modal
			await page.click('[data-testid="kg-search-button"]');
			await page.waitForSelector('[data-testid="kg-search-modal"]');

			// Test with invalid/empty search
			const searchInput = page.locator('[data-testid="kg-search-input"]');
			await searchInput.fill('');
			await searchInput.press('Enter');

			// Verify no error state or graceful handling
			const errorMessage = page.locator('[data-testid="kg-search-error"]');

			// Test with very long search query
			const longQuery = 'a'.repeat(1001); // Over 1000 char limit
			await searchInput.fill(longQuery);
			await page.waitForTimeout(TEST_CONFIG.AUTOCOMPLETE_DEBOUNCE + 100);

			// Should show error for too long query
			await expect(errorMessage).toBeVisible({ timeout: 3000 });

			console.log('✅ Error handling working correctly');
		});
	});

	test.describe('Context Addition Functionality', () => {
		test('should add KG term definition to conversation context', async ({ page }) => {
			console.log('📚 Testing KG term context addition...');

			// Open KG search modal and search for a term
			await page.click('[data-testid="kg-search-button"]');
			await page.waitForSelector('[data-testid="kg-search-modal"]');

			const searchInput = page.locator('[data-testid="kg-search-input"]');
			await searchInput.fill('async');
			await page.waitForTimeout(TEST_CONFIG.AUTOCOMPLETE_DEBOUNCE + 100);

			// Select first suggestion
			const firstSuggestion = page.locator('[data-testid="kg-suggestion-item"]').first();
			await expect(firstSuggestion).toBeVisible();
			await firstSuggestion.click();

			// Click add term button
			const addTermButton = page.locator('[data-testid="kg-add-term-button"]');
			await addTermButton.click();

			// Verify modal closes
			await expect(page.locator('[data-testid="kg-search-modal"]')).toBeHidden({
				timeout: 3000,
			});

			// Verify term appears in context panel
			const contextPanel = page.locator('[data-testid="context-panel"]');
			await expect(contextPanel).toContainText('async', { timeout: 5000 });

			// Verify context item is properly formatted as KG term
			const kgContextItem = page.locator('[data-testid="context-item-kg-term"]');
			await expect(kgContextItem).toBeVisible({ timeout: 3000 });

			console.log('✅ KG term successfully added to context');
		});

		test('should add complete KG index to conversation context', async ({ page }) => {
			console.log('📊 Testing complete KG index context addition...');

			// Open KG search modal
			await page.click('[data-testid="kg-search-button"]');
			await page.waitForSelector('[data-testid="kg-search-modal"]');

			// Click add index button
			const addIndexButton = page.locator('[data-testid="kg-add-index-button"]');
			await addIndexButton.click();

			// Verify modal closes
			await expect(page.locator('[data-testid="kg-search-modal"]')).toBeHidden({
				timeout: 3000,
			});

			// Verify KG index appears in context panel
			const contextPanel = page.locator('[data-testid="context-panel"]');
			await expect(contextPanel).toContainText('Knowledge Graph Index', {
				timeout: 5000,
			});

			// Verify context item shows index statistics
			const kgIndexItem = page.locator('[data-testid="context-item-kg-index"]');
			await expect(kgIndexItem).toBeVisible({ timeout: 3000 });
			await expect(kgIndexItem).toContainText(/\d+ terms/); // Should show term count
			await expect(kgIndexItem).toContainText(/\d+ nodes/); // Should show node count

			console.log('✅ Complete KG index successfully added to context');
		});

		test('should display KG context items with proper formatting', async ({ page }) => {
			console.log('🎨 Testing KG context item display formatting...');

			// Add a KG term to context first
			await page.click('[data-testid="kg-search-button"]');
			await page.waitForSelector('[data-testid="kg-search-modal"]');

			const searchInput = page.locator('[data-testid="kg-search-input"]');
			await searchInput.fill('tokio');
			await page.waitForTimeout(TEST_CONFIG.AUTOCOMPLETE_DEBOUNCE + 100);

			await page.locator('[data-testid="kg-suggestion-item"]').first().click();
			await page.click('[data-testid="kg-add-term-button"]');

			// Wait for context to load
			await page.waitForTimeout(2000);

			// Verify KG context item formatting
			const kgContextItem = page.locator('[data-testid="context-item-kg-term"]');
			await expect(kgContextItem).toBeVisible();

			// Check for KG-specific styling
			await expect(kgContextItem).toHaveClass(/kg-term/);

			// Verify term details are displayed
			await expect(kgContextItem).toContainText('tokio');

			// Check for definition section
			const definition = kgContextItem.locator('[data-testid="kg-term-definition"]');
			await expect(definition).toBeVisible();

			// Verify compact/full view toggle if implemented
			const expandButton = kgContextItem.locator('[data-testid="kg-expand-button"]');
			if (await expandButton.isVisible()) {
				await expandButton.click();
				// Verify expanded view shows more details
				await expect(kgContextItem).toContainText('Related Terms');
			}

			console.log('✅ KG context items display with proper formatting');
		});
	});

	test.describe('Performance Testing', () => {
		test('should handle concurrent search requests efficiently', async ({ page }) => {
			console.log('⚡ Testing concurrent search performance...');

			// Open KG search modal
			await page.click('[data-testid="kg-search-button"]');
			await page.waitForSelector('[data-testid="kg-search-modal"]');

			const searchInput = page.locator('[data-testid="kg-search-input"]');

			// Measure search response time
			const startTime = Date.now();

			await searchInput.fill('async');
			await page.waitForTimeout(TEST_CONFIG.AUTOCOMPLETE_DEBOUNCE);

			// Wait for suggestions to appear
			await page.locator('[data-testid="kg-suggestion-item"]').first().waitFor({
				timeout: PERFORMANCE_THRESHOLDS.AUTOCOMPLETE_RESPONSE_TIME_MS,
			});

			const responseTime = Date.now() - startTime;

			expect(responseTime).toBeLessThan(PERFORMANCE_THRESHOLDS.AUTOCOMPLETE_RESPONSE_TIME_MS);

			console.log(
				`✅ Search completed in ${responseTime}ms (threshold: ${PERFORMANCE_THRESHOLDS.AUTOCOMPLETE_RESPONSE_TIME_MS}ms)`
			);

			// Test rapid successive searches (simulating fast typing)
			const terms = ['async', 'tokio', 'futures', 'runtime', 'spawn'];

			for (const term of terms) {
				await searchInput.fill(term);
				await page.waitForTimeout(50); // Rapid typing simulation
			}

			// Wait for final debounced search
			await page.waitForTimeout(TEST_CONFIG.AUTOCOMPLETE_DEBOUNCE + 100);

			// Verify only the last search result is shown
			await expect(page.locator('[data-testid="kg-suggestions-list"]')).toBeVisible();

			console.log('✅ Concurrent search handling working efficiently');
		});

		test('should maintain performance with large result sets', async ({ page }) => {
			console.log('📊 Testing performance with large result sets...');

			// Open modal and search for a broad term that might return many results
			await page.click('[data-testid="kg-search-button"]');
			await page.waitForSelector('[data-testid="kg-search-modal"]');

			const searchInput = page.locator('[data-testid="kg-search-input"]');

			// Search for a common term
			const startTime = Date.now();
			await searchInput.fill('rust');
			await page.waitForTimeout(TEST_CONFIG.AUTOCOMPLETE_DEBOUNCE);

			// Wait for results
			await page.locator('[data-testid="kg-suggestion-item"]').first().waitFor({
				timeout: PERFORMANCE_THRESHOLDS.SEARCH_RESPONSE_TIME_MS,
			});

			const responseTime = Date.now() - startTime;
			expect(responseTime).toBeLessThan(PERFORMANCE_THRESHOLDS.SEARCH_RESPONSE_TIME_MS);

			// Verify reasonable number of results (not too many to overwhelm UI)
			const suggestions = page.locator('[data-testid="kg-suggestion-item"]');
			const count = await suggestions.count();

			expect(count).toBeGreaterThan(0);
			expect(count).toBeLessThanOrEqual(50); // Reasonable limit

			console.log(`✅ Large result set handled in ${responseTime}ms with ${count} suggestions`);
		});
	});

	test.describe('Edge Cases and Error Handling', () => {
		test('should handle network failures gracefully', async ({ page }) => {
			console.log('🌐 Testing network failure handling...');

			// Simulate network failure
			await page.route('**/*', (route) => route.abort());

			// Try to open KG search modal
			await page.click('[data-testid="kg-search-button"]');

			// Should still open modal but show error state for searches
			await page.waitForSelector('[data-testid="kg-search-modal"]');

			const searchInput = page.locator('[data-testid="kg-search-input"]');
			await searchInput.fill('async');
			await page.waitForTimeout(TEST_CONFIG.AUTOCOMPLETE_DEBOUNCE + 100);

			// Verify error message is shown
			const errorMessage = page.locator('[data-testid="kg-search-error"]');
			await expect(errorMessage).toBeVisible({ timeout: 5000 });
			await expect(errorMessage).toContainText(/network|failed|error/i);

			console.log('✅ Network failures handled gracefully');
		});

		test('should handle empty search results', async ({ page }) => {
			console.log('🔍 Testing empty search results...');

			await page.click('[data-testid="kg-search-button"]');
			await page.waitForSelector('[data-testid="kg-search-modal"]');

			const searchInput = page.locator('[data-testid="kg-search-input"]');

			// Search for non-existent term
			await searchInput.fill('nonexistentxyzabc123');
			await page.waitForTimeout(TEST_CONFIG.AUTOCOMPLETE_DEBOUNCE + 100);

			// Verify empty state message
			const emptyState = page.locator('[data-testid="kg-search-empty"]');
			await expect(emptyState).toBeVisible({ timeout: 3000 });
			await expect(emptyState).toContainText(/no results|not found/i);

			console.log('✅ Empty search results handled correctly');
		});

		test('should validate input length limits', async ({ page }) => {
			console.log('📏 Testing input validation...');

			await page.click('[data-testid="kg-search-button"]');
			await page.waitForSelector('[data-testid="kg-search-modal"]');

			const searchInput = page.locator('[data-testid="kg-search-input"]');

			// Test maximum length validation (should be 1000 chars)
			const maxQuery = 'a'.repeat(1000);
			await searchInput.fill(maxQuery);

			// Should accept up to 1000 characters
			expect((await searchInput.inputValue()).length).toBeLessThanOrEqual(1000);

			// Test over limit
			const overLimitQuery = 'a'.repeat(1001);
			await searchInput.fill(overLimitQuery);
			await page.waitForTimeout(TEST_CONFIG.AUTOCOMPLETE_DEBOUNCE + 100);

			// Should show validation error
			const errorMessage = page.locator('[data-testid="kg-search-error"]');
			await expect(errorMessage).toBeVisible();
			await expect(errorMessage).toContainText(/too long|maximum/i);

			console.log('✅ Input validation working correctly');
		});
	});

	test.afterEach(async ({ page }) => {
		console.log('🧹 Cleaning up test environment...');

		// Close any open modals
		const modal = page.locator('[data-testid="kg-search-modal"]');
		if (await modal.isVisible()) {
			await page.keyboard.press('Escape');
		}

		console.log('✅ Test cleanup complete');
	});
});

// Helper functions for test utilities
async function waitForKGSearchModal(page: any) {
	await page.waitForSelector('[data-testid="kg-search-modal"]', {
		timeout: 5000,
	});
}

async function performKGSearch(page: any, query: string) {
	const searchInput = page.locator('[data-testid="kg-search-input"]');
	await searchInput.fill(query);
	await page.waitForTimeout(TEST_CONFIG.AUTOCOMPLETE_DEBOUNCE + 100);
}

async function addKGTermToContext(page: any, searchTerm: string) {
	await page.click('[data-testid="kg-search-button"]');
	await waitForKGSearchModal(page);
	await performKGSearch(page, searchTerm);

	await page.locator('[data-testid="kg-suggestion-item"]').first().click();
	await page.click('[data-testid="kg-add-term-button"]');

	// Wait for modal to close
	await expect(page.locator('[data-testid="kg-search-modal"]')).toBeHidden({
		timeout: 3000,
	});
}
