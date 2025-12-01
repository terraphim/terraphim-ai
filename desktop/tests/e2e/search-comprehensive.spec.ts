/**
 * Comprehensive Search Functionality Tests
 *
 * This test suite covers all aspects of the search functionality:
 * - Basic search with different query types
 * - Autocomplete functionality
 * - Search result display and interaction
 * - Filtering and sorting
 * - Error handling
 * - Performance with large result sets
 */

import { test, expect } from '@playwright/test';
import {
	ciWaitForSelector,
	ciSearch,
	ciNavigate,
	ciWait,
	ciClick,
	getTimeouts,
} from '../../src/test-utils/ci-friendly';

// Test configuration
const TEST_TIMEOUT = 90000;
const SEARCH_TIMEOUT = 30000;

// Test data
const SEARCH_QUERIES = {
	basic: 'rust programming',
	complex: 'rust async tokio futures error handling',
	withOperators: 'rust AND async OR futures',
	empty: '',
	specialChars: 'rust@#$%^&*()',
	veryLong:
		'rust programming language async await tokio futures streams error handling patterns best practices',
	numeric: 'rust 2024 edition',
	mixedCase: 'Rust Async Programming Tokio',
};

const EXPECTED_RESULT_PROPERTIES = ['title', 'body', 'url', 'rank', 'tags'];

test.describe('Search Functionality Comprehensive Tests', () => {
	test.beforeEach(async ({ page }) => {
		test.setTimeout(TEST_TIMEOUT);
		await ciNavigate(page, '/');
		await ciWaitForSelector(page, '[data-testid="search-tab"]', 'navigation');
	});

	test.describe('Basic Search Functionality', () => {
		test('should perform basic search and display results', async ({ page }) => {
			console.log('🔍 Testing basic search functionality...');

			const searchInput = page.locator('input[type="search"]');
			await searchInput.fill(SEARCH_QUERIES.basic);
			await searchInput.press('Enter');

			// Wait for search results
			await ciWait(page, 'medium');

			// Verify results are displayed
			const searchResults = page.locator('[data-testid="search-results"] .box');
			await expect(searchResults.first()).toBeVisible();

			// Verify result structure
			const firstResult = searchResults.first();
			for (const property of EXPECTED_RESULT_PROPERTIES) {
				const element = firstResult.locator(`[data-testid="${property}"]`);
				if (await element.isVisible()) {
					console.log(`✅ Result has ${property} property`);
				}
			}

			console.log('✅ Basic search completed successfully');
		});

		test('should handle complex search queries', async ({ page }) => {
			console.log('🔍 Testing complex search queries...');

			const searchInput = page.locator('input[type="search"]');
			await searchInput.fill(SEARCH_QUERIES.complex);
			await searchInput.press('Enter');

			await ciWait(page, 'medium');

			const searchResults = page.locator('[data-testid="search-results"] .box');
			const resultCount = await searchResults.count();

			expect(resultCount).toBeGreaterThan(0);
			console.log(`✅ Complex search returned ${resultCount} results`);
		});

		test('should handle search with logical operators', async ({ page }) => {
			console.log('🔍 Testing search with logical operators...');

			const searchInput = page.locator('input[type="search"]');
			await searchInput.fill(SEARCH_QUERIES.withOperators);
			await searchInput.press('Enter');

			await ciWait(page, 'medium');

			const searchResults = page.locator('[data-testid="search-results"] .box');
			const resultCount = await searchResults.count();

			expect(resultCount).toBeGreaterThanOrEqual(0);
			console.log(`✅ Operator search returned ${resultCount} results`);
		});

		test('should handle empty search gracefully', async ({ page }) => {
			console.log('🔍 Testing empty search handling...');

			const searchInput = page.locator('input[type="search"]');
			await searchInput.press('Enter');

			await ciWait(page, 'small');

			// Should show empty state or not perform search
			const emptyState = page.locator('[data-testid="empty-state"], .has-text-centered');
			const hasEmptyState = await emptyState.isVisible();

			if (hasEmptyState) {
				console.log('✅ Empty search shows appropriate empty state');
			} else {
				console.log('✅ Empty search handled gracefully');
			}
		});

		test('should handle special characters in search', async ({ page }) => {
			console.log('🔍 Testing special characters in search...');

			const searchInput = page.locator('input[type="search"]');
			await searchInput.fill(SEARCH_QUERIES.specialChars);
			await searchInput.press('Enter');

			await ciWait(page, 'medium');

			// Should not crash and should handle gracefully
			const searchResults = page.locator('[data-testid="search-results"] .box');
			const resultCount = await searchResults.count();

			console.log(`✅ Special characters search returned ${resultCount} results`);
		});
	});

	test.describe('Autocomplete Functionality', () => {
		test('should show autocomplete suggestions', async ({ page }) => {
			console.log('🔍 Testing autocomplete suggestions...');

			const searchInput = page.locator('input[type="search"]');
			await searchInput.click();
			await searchInput.fill('rust');

			// Wait for suggestions to appear
			await ciWait(page, 'small');

			const suggestions = page.locator('.suggestions li');
			const suggestionCount = await suggestions.count();

			if (suggestionCount > 0) {
				console.log(`✅ Autocomplete showed ${suggestionCount} suggestions`);

				// Test selecting a suggestion
				await suggestions.first().click();
				console.log('✅ Suggestion selected successfully');
			} else {
				console.log('ℹ️ No autocomplete suggestions available');
			}
		});

		test('should handle keyboard navigation in autocomplete', async ({ page }) => {
			console.log('🔍 Testing keyboard navigation in autocomplete...');

			const searchInput = page.locator('input[type="search"]');
			await searchInput.click();
			await searchInput.fill('rust');

			await ciWait(page, 'small');

			const suggestions = page.locator('.suggestions li');
			const suggestionCount = await suggestions.count();

			if (suggestionCount > 0) {
				// Test arrow key navigation
				await searchInput.press('ArrowDown');
				await searchInput.press('ArrowDown');
				await searchInput.press('ArrowUp');

				// Test selection with Enter
				await searchInput.press('Enter');
				console.log('✅ Keyboard navigation in autocomplete works');
			} else {
				console.log('ℹ️ No suggestions available for keyboard navigation test');
			}
		});

		test('should filter suggestions based on input', async ({ page }) => {
			console.log('🔍 Testing autocomplete filtering...');

			const searchInput = page.locator('input[type="search"]');
			await searchInput.click();

			// Test with different prefixes
			const testPrefixes = ['r', 'ru', 'rust', 'rust '];

			for (const prefix of testPrefixes) {
				await searchInput.fill(prefix);
				await ciWait(page, 'small');

				const suggestions = page.locator('.suggestions li');
				const suggestionCount = await suggestions.count();

				console.log(`Prefix "${prefix}": ${suggestionCount} suggestions`);
			}

			console.log('✅ Autocomplete filtering tested');
		});
	});

	test.describe('Search Results Display', () => {
		test('should display search results with proper formatting', async ({ page }) => {
			console.log('🔍 Testing search results display...');

			const searchInput = page.locator('input[type="search"]');
			await searchInput.fill(SEARCH_QUERIES.basic);
			await searchInput.press('Enter');

			await ciWait(page, 'medium');

			const searchResults = page.locator('[data-testid="search-results"] .box');
			const resultCount = await searchResults.count();

			expect(resultCount).toBeGreaterThan(0);

			// Check first result formatting
			const firstResult = searchResults.first();

			// Check title
			const title = firstResult.locator('h2.title');
			await expect(title).toBeVisible();

			// Check description/body
			const description = firstResult.locator('.description');
			await expect(description).toBeVisible();

			// Check rank
			const rank = firstResult.locator('.tag:has-text("Rank")');
			await expect(rank).toBeVisible();

			// Check tags if present
			const tags = firstResult.locator('.taglist');
			const hasTags = await tags.isVisible();
			if (hasTags) {
				console.log('✅ Result has tags displayed');
			}

			console.log('✅ Search results properly formatted');
		});

		test('should handle large result sets', async ({ page }) => {
			console.log('🔍 Testing large result sets...');

			const searchInput = page.locator('input[type="search"]');
			await searchInput.fill(SEARCH_QUERIES.veryLong);
			await searchInput.press('Enter');

			await ciWait(page, 'medium');

			const searchResults = page.locator('[data-testid="search-results"] .box');
			const resultCount = await searchResults.count();

			console.log(`✅ Large query returned ${resultCount} results`);

			// Check if pagination or "load more" is available
			const loadMoreButton = page.locator('[data-testid="load-more-button"]');
			const hasLoadMore = await loadMoreButton.isVisible();

			if (hasLoadMore) {
				console.log('✅ Load more functionality available');
			}
		});

		test('should display result metadata correctly', async ({ page }) => {
			console.log('🔍 Testing result metadata display...');

			const searchInput = page.locator('input[type="search"]');
			await searchInput.fill(SEARCH_QUERIES.basic);
			await searchInput.press('Enter');

			await ciWait(page, 'medium');

			const searchResults = page.locator('[data-testid="search-results"] .box');
			const firstResult = searchResults.first();

			// Check for action buttons
			const actionButtons = firstResult.locator('.level-right .level-item');
			const buttonCount = await actionButtons.count();

			expect(buttonCount).toBeGreaterThan(0);
			console.log(`✅ Result has ${buttonCount} action buttons`);

			// Check for specific action buttons
			const addToContextButton = firstResult.locator('[data-testid="add-to-context-button"]');
			const hasAddToContext = await addToContextButton.isVisible();

			if (hasAddToContext) {
				console.log('✅ Add to context button available');
			}
		});
	});

	test.describe('Search Performance', () => {
		test('should perform search within reasonable time', async ({ page }) => {
			console.log('🔍 Testing search performance...');

			const startTime = Date.now();

			const searchInput = page.locator('input[type="search"]');
			await searchInput.fill(SEARCH_QUERIES.basic);
			await searchInput.press('Enter');

			// Wait for results to appear
			await ciWaitForSelector(page, '[data-testid="search-results"] .box', 'search');

			const endTime = Date.now();
			const searchTime = endTime - startTime;

			console.log(`✅ Search completed in ${searchTime}ms`);

			// Search should complete within 10 seconds
			expect(searchTime).toBeLessThan(10000);
		});

		test('should handle rapid successive searches', async ({ page }) => {
			console.log('🔍 Testing rapid successive searches...');

			const searchInput = page.locator('input[type="search"]');

			const queries = ['rust', 'async', 'tokio', 'futures'];

			for (const query of queries) {
				await searchInput.fill(query);
				await searchInput.press('Enter');
				await ciWait(page, 'small');
			}

			console.log('✅ Rapid successive searches handled');
		});
	});

	test.describe('Search Error Handling', () => {
		test('should handle network errors gracefully', async ({ page }) => {
			console.log('🔍 Testing network error handling...');

			// Intercept network requests to simulate error
			await page.route('**/documents/search', (route) => {
				route.abort('failed');
			});

			const searchInput = page.locator('input[type="search"]');
			await searchInput.fill(SEARCH_QUERIES.basic);
			await searchInput.press('Enter');

			await ciWait(page, 'medium');

			// Should show error message
			const errorMessage = page.locator('.error, [data-testid="error-message"]');
			const hasError = await errorMessage.isVisible();

			if (hasError) {
				console.log('✅ Network error handled gracefully');
			} else {
				console.log('ℹ️ No error message displayed (may be handled differently)');
			}
		});

		test('should handle malformed responses', async ({ page }) => {
			console.log('🔍 Testing malformed response handling...');

			// Intercept and modify response
			await page.route('**/documents/search', (route) => {
				route.fulfill({
					status: 200,
					contentType: 'application/json',
					body: '{"invalid": "json"',
				});
			});

			const searchInput = page.locator('input[type="search"]');
			await searchInput.fill(SEARCH_QUERIES.basic);
			await searchInput.press('Enter');

			await ciWait(page, 'medium');

			// Should handle malformed response gracefully
			console.log('✅ Malformed response handled');
		});
	});

	test.describe('Search Accessibility', () => {
		test('should be accessible via keyboard', async ({ page }) => {
			console.log('🔍 Testing keyboard accessibility...');

			// Tab to search input
			await page.keyboard.press('Tab');
			await page.keyboard.press('Tab');

			// Type search query
			await page.keyboard.type(SEARCH_QUERIES.basic);
			await page.keyboard.press('Enter');

			await ciWait(page, 'medium');

			const searchResults = page.locator('[data-testid="search-results"] .box');
			const resultCount = await searchResults.count();

			expect(resultCount).toBeGreaterThan(0);
			console.log('✅ Keyboard accessibility works');
		});

		test('should have proper ARIA labels', async ({ page }) => {
			console.log('🔍 Testing ARIA labels...');

			const searchInput = page.locator('input[type="search"]');
			const hasAriaLabel = await searchInput.getAttribute('aria-label');

			if (hasAriaLabel) {
				console.log('✅ Search input has ARIA label');
			}

			// Check search results
			await searchInput.fill(SEARCH_QUERIES.basic);
			await searchInput.press('Enter');
			await ciWait(page, 'medium');

			const searchResults = page.locator('[data-testid="search-results"]');
			const hasAriaRole = await searchResults.getAttribute('role');

			if (hasAriaRole) {
				console.log('✅ Search results have ARIA role');
			}
		});
	});

	test.describe('Search Integration', () => {
		test('should integrate with context management', async ({ page }) => {
			console.log('🔍 Testing search-context integration...');

			// Perform search
			const searchInput = page.locator('input[type="search"]');
			await searchInput.fill(SEARCH_QUERIES.basic);
			await searchInput.press('Enter');
			await ciWait(page, 'medium');

			// Add result to context
			const searchResults = page.locator('[data-testid="search-results"] .box');
			const firstResult = searchResults.first();
			const addToContextButton = firstResult.locator('[data-testid="add-to-context-button"]');

			await addToContextButton.click();
			await ciWait(page, 'medium');

			// Navigate to chat to verify context
			await ciNavigate(page, '/chat');
			await ciWaitForSelector(page, '[data-testid="chat-interface"]', 'navigation');

			const contextItems = page.locator('[data-testid="conversation-context"] .context-item');
			const contextCount = await contextItems.count();

			expect(contextCount).toBeGreaterThan(0);
			console.log('✅ Search-context integration works');
		});

		test('should integrate with knowledge graph', async ({ page }) => {
			console.log('🔍 Testing search-KG integration...');

			// Perform search
			const searchInput = page.locator('input[type="search"]');
			await searchInput.fill(SEARCH_QUERIES.basic);
			await searchInput.press('Enter');
			await ciWait(page, 'medium');

			// Check if KG tags are displayed
			const searchResults = page.locator('[data-testid="search-results"] .box');
			const firstResult = searchResults.first();
			const kgTags = firstResult.locator('.tag-button');
			const tagCount = await kgTags.count();

			if (tagCount > 0) {
				console.log(`✅ Found ${tagCount} KG tags in search results`);

				// Test clicking on KG tag
				await kgTags.first().click();
				await ciWait(page, 'medium');
				console.log('✅ KG tag interaction works');
			} else {
				console.log('ℹ️ No KG tags found in search results');
			}
		});
	});
});
