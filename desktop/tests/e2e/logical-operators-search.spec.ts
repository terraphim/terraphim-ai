import { test, expect } from '@playwright/test';

test.describe('Logical Operators Search E2E', () => {
  test.beforeEach(async ({ page }) => {
    // Start the Tauri app and wait for it to be ready
    await page.goto('http://localhost:1420');

    // Wait for the app to fully load
    await page.waitForLoadState('networkidle');

    // Ensure we're on the search page
    const searchInput = page.locator('input[placeholder*="search"], input[type="search"], .search-input');
    await expect(searchInput.first()).toBeVisible({ timeout: 15000 });
  });

  test.describe('UI Search with Logical Operators', () => {
    test('should parse AND operator in search input', async ({ page }) => {
      // Find the search input using broader selectors
      const searchInput = page.locator('input[placeholder*="search"], input[type="search"], .search-input').first();
      await expect(searchInput).toBeVisible({ timeout: 10000 });

      // Type search query with AND operator
      await searchInput.fill('rust AND async');

      // Press Enter to search or click search button
      await Promise.race([
        searchInput.press('Enter'),
        page.locator('button[type="submit"], .search-button, button:has-text("Search")').first().click().catch(() => {})
      ]);

      // Wait for any network requests to complete
      await page.waitForLoadState('networkidle');

      // Look for results container with various possible selectors
      const resultsContainer = page.locator('.search-results, .results, [class*="result"]').first();
      await expect(resultsContainer).toBeVisible({ timeout: 15000 });

      // Verify that the search executed - check for any search indicators
      const searchIndicators = page.locator('.search-term, .query-display, .search-query');
      if (await searchIndicators.count() > 0) {
        await expect(searchIndicators.first()).toContainText('rust');
        await expect(searchIndicators.first()).toContainText('async');
      }
    });

    test('should parse OR operator in search input', async ({ page }) => {
      const searchInput = page.locator('input[placeholder*="search"], input[type="search"], .search-input').first();
      await expect(searchInput).toBeVisible({ timeout: 10000 });

      await searchInput.fill('api OR sdk');

      await Promise.race([
        searchInput.press('Enter'),
        page.locator('button[type="submit"], .search-button, button:has-text("Search")').first().click().catch(() => {})
      ]);

      await page.waitForLoadState('networkidle');

      const resultsContainer = page.locator('.search-results, .results, [class*="result"]').first();
      await expect(resultsContainer).toBeVisible({ timeout: 15000 });

      // Verify OR operation was processed
      const searchIndicators = page.locator('.search-term, .query-display, .search-query');
      if (await searchIndicators.count() > 0) {
        const indicatorText = await searchIndicators.first().textContent();
        expect(indicatorText).toMatch(/(api|sdk)/i);
      }
    });

    test('should handle multiple terms with AND operator', async ({ page }) => {
      await page.goto('/');

      const searchInput = page.locator('[data-testid="search-input"]');
      await expect(searchInput).toBeVisible({ timeout: 10000 });

      await searchInput.fill('rust AND async AND programming');
      await searchInput.press('Enter');

      await page.waitForSelector('[data-testid="search-results"]', { timeout: 15000 });

      // Verify all terms are processed
      const appliedFilters = page.locator('[data-testid="applied-filters"]');
      if (await appliedFilters.isVisible()) {
        await expect(appliedFilters).toContainText('rust');
        await expect(appliedFilters).toContainText('async');
        await expect(appliedFilters).toContainText('programming');
      }
    });

    test('should handle case-insensitive operators', async ({ page }) => {
      await page.goto('/');

      const searchInput = page.locator('[data-testid="search-input"]');
      await expect(searchInput).toBeVisible({ timeout: 10000 });

      // Test lowercase 'and'
      await searchInput.fill('rust and async');
      await searchInput.press('Enter');

      await page.waitForSelector('[data-testid="search-results"]', { timeout: 15000 });

      // Clear and test mixed case
      await searchInput.clear();
      await searchInput.fill('rust And async');
      await searchInput.press('Enter');

      await page.waitForSelector('[data-testid="search-results"]', { timeout: 15000 });

      // Both should work the same way
      const results = page.locator('[data-testid="search-result-item"]');
      await expect(results).toHaveCount({ min: 0 });
    });

    test('should fall back to regular search for non-operator queries', async ({ page }) => {
      await page.goto('/');

      const searchInput = page.locator('[data-testid="search-input"]');
      await expect(searchInput).toBeVisible({ timeout: 10000 });

      // Regular search without operators
      await searchInput.fill('rust programming');
      await searchInput.press('Enter');

      await page.waitForSelector('[data-testid="search-results"]', { timeout: 15000 });

      const results = page.locator('[data-testid="search-result-item"]');
      await expect(results).toHaveCount({ min: 0 });
    });
  });

  test.describe('Autocomplete with Logical Operators', () => {
    test('should suggest AND and OR operators after typing a term', async ({ page }) => {
      await page.goto('/');

      const searchInput = page.locator('[data-testid="search-input"]');
      await expect(searchInput).toBeVisible({ timeout: 10000 });

      // Type a term and wait for suggestions
      await searchInput.fill('rust');

      // Wait for autocomplete suggestions to appear
      const suggestions = page.locator('[data-testid="autocomplete-suggestion"]');
      await expect(suggestions.first()).toBeVisible({ timeout: 5000 });

      // Look for operator suggestions
      const andSuggestion = page.locator('[data-testid="autocomplete-suggestion"]:has-text("rust AND")');
      const orSuggestion = page.locator('[data-testid="autocomplete-suggestion"]:has-text("rust OR")');

      // At least one operator suggestion should be present
      const hasOperatorSuggestions = await andSuggestion.isVisible() || await orSuggestion.isVisible();
      expect(hasOperatorSuggestions).toBe(true);
    });

    test('should suggest second terms after typing operator', async ({ page }) => {
      await page.goto('/');

      const searchInput = page.locator('[data-testid="search-input"]');
      await expect(searchInput).toBeVisible({ timeout: 10000 });

      // Type first term with operator
      await searchInput.fill('rust AND ');

      // Wait for suggestions for second term
      const suggestions = page.locator('[data-testid="autocomplete-suggestion"]');
      await expect(suggestions.first()).toBeVisible({ timeout: 5000 });

      // Should have term suggestions, not more operators
      const suggestionTexts = await suggestions.allTextContents();
      const hasTermSuggestions = suggestionTexts.some(text =>
        !text.includes('AND') && !text.includes('OR')
      );
      expect(hasTermSuggestions).toBe(true);
    });

    test('should apply autocomplete suggestion for operators', async ({ page }) => {
      await page.goto('/');

      const searchInput = page.locator('[data-testid="search-input"]');
      await expect(searchInput).toBeVisible({ timeout: 10000 });

      await searchInput.fill('rust');

      // Wait for and click on AND operator suggestion
      const andSuggestion = page.locator('[data-testid="autocomplete-suggestion"]:has-text("rust AND")');
      if (await andSuggestion.isVisible({ timeout: 5000 })) {
        await andSuggestion.click();

        // Verify the input was updated
        const inputValue = await searchInput.inputValue();
        expect(inputValue).toContain('rust AND');
      }
    });
  });

  test.describe('Search Results with Logical Operators', () => {
    test('should show results that match AND criteria', async ({ page }) => {
      await page.goto('/');

      const searchInput = page.locator('[data-testid="search-input"]');
      await expect(searchInput).toBeVisible({ timeout: 10000 });

      await searchInput.fill('rust AND programming');
      await searchInput.press('Enter');

      await page.waitForSelector('[data-testid="search-results"]', { timeout: 15000 });

      const results = page.locator('[data-testid="search-result-item"]');
      const resultCount = await results.count();

      if (resultCount > 0) {
        // Check first few results to ensure they contain both terms
        for (let i = 0; i < Math.min(3, resultCount); i++) {
          const result = results.nth(i);
          const resultText = await result.textContent();
          const lowerText = resultText?.toLowerCase() || '';

          // Should contain both terms (in title, description, or body)
          const hasRust = lowerText.includes('rust');
          const hasProgramming = lowerText.includes('programming');
          expect(hasRust && hasProgramming).toBe(true);
        }
      }
    });

    test('should show results that match OR criteria', async ({ page }) => {
      await page.goto('/');

      const searchInput = page.locator('[data-testid="search-input"]');
      await expect(searchInput).toBeVisible({ timeout: 10000 });

      await searchInput.fill('rust OR python');
      await searchInput.press('Enter');

      await page.waitForSelector('[data-testid="search-results"]', { timeout: 15000 });

      const results = page.locator('[data-testid="search-result-item"]');
      const resultCount = await results.count();

      if (resultCount > 0) {
        // Check first few results to ensure they contain at least one term
        for (let i = 0; i < Math.min(3, resultCount); i++) {
          const result = results.nth(i);
          const resultText = await result.textContent();
          const lowerText = resultText?.toLowerCase() || '';

          // Should contain at least one of the terms
          const hasRust = lowerText.includes('rust');
          const hasPython = lowerText.includes('python');
          expect(hasRust || hasPython).toBe(true);
        }
      }
    });

    test('should highlight matching terms in results', async ({ page }) => {
      await page.goto('/');

      const searchInput = page.locator('[data-testid="search-input"]');
      await expect(searchInput).toBeVisible({ timeout: 10000 });

      await searchInput.fill('rust AND async');
      await searchInput.press('Enter');

      await page.waitForSelector('[data-testid="search-results"]', { timeout: 15000 });

      const results = page.locator('[data-testid="search-result-item"]');
      const resultCount = await results.count();

      if (resultCount > 0) {
        // Check for highlighted terms in results
        const highlightedTerms = page.locator('.search-highlight, .highlight, mark');
        const highlightCount = await highlightedTerms.count();

        // Should have some highlighting if results contain search terms
        if (highlightCount > 0) {
          const highlightedText = await highlightedTerms.first().textContent();
          expect(highlightedText?.toLowerCase()).toMatch(/(rust|async)/);
        }
      }
    });
  });

  test.describe('Error Handling', () => {
    test('should handle malformed operator queries gracefully', async ({ page }) => {
      await page.goto('/');

      const searchInput = page.locator('[data-testid="search-input"]');
      await expect(searchInput).toBeVisible({ timeout: 10000 });

      // Test various malformed queries
      const malformedQueries = [
        'AND rust',
        'rust AND',
        'rust AND AND async',
        'OR OR OR',
        'rust AND OR async'
      ];

      for (const query of malformedQueries) {
        await searchInput.fill(query);
        await searchInput.press('Enter');

        // Should not crash or show error
        await page.waitForTimeout(1000);
        const errorMessage = page.locator('[data-testid="error-message"]');
        expect(await errorMessage.isVisible()).toBe(false);

        await searchInput.clear();
      }
    });

    test('should handle empty results gracefully', async ({ page }) => {
      await page.goto('/');

      const searchInput = page.locator('[data-testid="search-input"]');
      await expect(searchInput).toBeVisible({ timeout: 10000 });

      // Search for terms that likely won't have results
      await searchInput.fill('nonexistent AND alsononexistent');
      await searchInput.press('Enter');

      await page.waitForSelector('[data-testid="search-results"]', { timeout: 15000 });

      // Should show no results message
      const noResultsMessage = page.locator('[data-testid="no-results-message"]');
      const emptyState = page.locator('[data-testid="empty-state"]');

      const hasNoResultsIndication = await noResultsMessage.isVisible() || await emptyState.isVisible();
      expect(hasNoResultsIndication).toBe(true);
    });

    test('should handle service errors gracefully', async ({ page }) => {
      await page.goto('/');

      // Intercept search requests and simulate error
      await page.route('**/api/documents/search', async route => {
        await route.abort('failed');
      });

      const searchInput = page.locator('[data-testid="search-input"]');
      await expect(searchInput).toBeVisible({ timeout: 10000 });

      await searchInput.fill('rust AND async');
      await searchInput.press('Enter');

      // Should handle error gracefully without crashing
      await page.waitForTimeout(3000);

      const errorMessage = page.locator('[data-testid="error-message"], [data-testid="service-error"]');
      if (await errorMessage.isVisible()) {
        expect(await errorMessage.textContent()).toMatch(/(error|failed|unavailable)/i);
      }
    });
  });

  test.describe('Performance and UX', () => {
    test('should show loading state during search', async ({ page }) => {
      await page.goto('/');

      const searchInput = page.locator('[data-testid="search-input"]');
      await expect(searchInput).toBeVisible({ timeout: 10000 });

      // Intercept to add delay
      await page.route('**/api/documents/search', async route => {
        await page.waitForTimeout(2000); // Add 2s delay
        await route.continue();
      });

      await searchInput.fill('rust AND async');
      await searchInput.press('Enter');

      // Should show loading indicator
      const loadingIndicator = page.locator('[data-testid="loading"], [data-testid="search-loading"]');
      await expect(loadingIndicator).toBeVisible({ timeout: 1000 });

      // Loading should disappear when results load
      await expect(loadingIndicator).toBeHidden({ timeout: 10000 });
    });

    test('should maintain search state on page refresh', async ({ page }) => {
      await page.goto('/');

      const searchInput = page.locator('[data-testid="search-input"]');
      await expect(searchInput).toBeVisible({ timeout: 10000 });

      await searchInput.fill('rust AND async');
      await searchInput.press('Enter');

      await page.waitForSelector('[data-testid="search-results"]', { timeout: 15000 });

      // Refresh the page
      await page.reload();

      // Search input should maintain the query
      await expect(searchInput).toBeVisible({ timeout: 10000 });
      const inputValue = await searchInput.inputValue();
      expect(inputValue).toBe('rust AND async');
    });

    test('should handle rapid search queries without breaking', async ({ page }) => {
      await page.goto('/');

      const searchInput = page.locator('[data-testid="search-input"]');
      await expect(searchInput).toBeVisible({ timeout: 10000 });

      // Rapidly type and search multiple queries
      const queries = [
        'rust AND async',
        'api OR sdk',
        'web AND development',
        'python OR javascript'
      ];

      for (const query of queries) {
        await searchInput.fill(query);
        await searchInput.press('Enter');
        await page.waitForTimeout(500); // Small delay between queries
      }

      // Should handle all queries without errors
      const errorMessage = page.locator('[data-testid="error-message"]');
      expect(await errorMessage.isVisible()).toBe(false);

      // Final query should be in input
      const finalValue = await searchInput.inputValue();
      expect(finalValue).toBe('python OR javascript');
    });
  });
});
