/**
 * Playwright E2E tests for Novel Editor Autocomplete functionality
 * Tests the complete autocomplete integration including UI, backend services, and user interactions
 */

import { test, expect } from '@playwright/test';
import {
  checkMCPServerHealth,
  waitForMCPServer,
  navigateToEditor,
  getNovelEditor,
  typeInEditor,
  waitForAutocompleteDropdown,
  getAutocompleteSuggestions,
  navigateAutocompleteSuggestions,
  selectAutocompleteSuggestion,
  cancelAutocomplete,
  testAutocompleteFlow,
  measureAutocompleteResponseTime,
  getAutocompleteStatus,
  clickAutocompleteTestButton,
  clickAutocompleteRebuildButton,
  verifyAutocompleteConfig,
  waitForAutocompleteStatus,
  simulateNetworkConditions,
  screenshotAutocompleteDropdown,
  DEFAULT_AUTOCOMPLETE_CONFIG,
  type AutocompleteSuggestion,
  type AutocompleteTestConfig,
} from './helpers/autocomplete-helpers';

import {
  EXPECTED_SUGGESTIONS,
  MOCK_SUGGESTIONS,
  TEST_CONFIGS,
  TEST_QUERIES,
  ERROR_SCENARIOS,
  VISUAL_TEST_SCENARIOS,
  KEYBOARD_TEST_SCENARIOS,
  PERFORMANCE_BENCHMARKS,
  ROLE_SPECIFIC_DATA,
  getExpectedSuggestions,
  validateSuggestionStructure,
} from '../fixtures/autocomplete-fixtures';

import {
  ciWaitForSelector,
  ciWait,
  ciClick,
  getTimeouts,
  isCI,
} from '../../src/test-utils/ci-friendly';

// Test configuration
const TEST_CONFIG = TEST_CONFIGS.default;
const MCP_SERVER_PORT = process.env.MCP_SERVER_PORT ? parseInt(process.env.MCP_SERVER_PORT) : 8001;

test.describe('Novel Editor Autocomplete', () => {
  let mcpServerAvailable = false;

  test.beforeAll(async () => {
    // Check if MCP server is available for tests
    mcpServerAvailable = await checkMCPServerHealth(MCP_SERVER_PORT);

    if (mcpServerAvailable) {
      console.log(`✅ MCP server available on port ${MCP_SERVER_PORT}`);
    } else {
      console.log(`⚠️ MCP server not available on port ${MCP_SERVER_PORT} - some tests will use fallback behavior`);

      // Try to wait for server if in CI or explicit test mode
      if (isCI() || process.env.WAIT_FOR_MCP_SERVER) {
        console.log('Waiting for MCP server to become available...');
        mcpServerAvailable = await waitForMCPServer(MCP_SERVER_PORT, 30);
      }
    }
  });

  test.beforeEach(async ({ page }) => {
    // Navigate to editor page
    await navigateToEditor(page);

    // Wait for autocomplete system to initialize
    await ciWait(page, 'medium');
  });

  test.describe('Basic Functionality', () => {
    test('should display autocomplete trigger in editor', async ({ page }) => {
      const editor = await getNovelEditor(page);
      await expect(editor).toBeVisible();

      // Focus editor and type trigger character
      await editor.click();
      await page.keyboard.type(TEST_CONFIG.trigger);

      // Verify trigger character appears in editor
      const editorContent = await editor.textContent();
      expect(editorContent).toContain(TEST_CONFIG.trigger);
    });

    test('should show autocomplete dropdown when typing query', async ({ page }) => {
      test.skip(!mcpServerAvailable, 'MCP server not available');

      // Type a query that should trigger autocomplete
      await typeInEditor(page, 'terraphim', TEST_CONFIG);

      // Wait for debounce delay
      await page.waitForTimeout(TEST_CONFIG.debounceDelay + 100);

      // Wait for dropdown to appear
      const dropdown = await waitForAutocompleteDropdown(page);
      await expect(dropdown).toBeVisible();

      // Verify dropdown contains suggestions
      const suggestions = await getAutocompleteSuggestions(page);
      expect(suggestions.length).toBeGreaterThan(0);

      // Verify suggestion structure
      for (const suggestion of suggestions) {
        expect(validateSuggestionStructure(suggestion)).toBe(true);
      }
    });

    test('should respect minimum query length', async ({ page }) => {
      const config = TEST_CONFIGS.minimal; // minLength: 2

      // Type single character (should not trigger)
      await typeInEditor(page, 'a', config);
      await page.waitForTimeout(config.debounceDelay + 100);

      // Dropdown should not appear
      const dropdown = page.locator('.terraphim-suggestion-dropdown');
      await expect(dropdown).not.toBeVisible();

      // Type second character (should trigger)
      await page.keyboard.type('u');
      await page.waitForTimeout(config.debounceDelay + 100);

      // Now dropdown should appear (if server available)
      if (mcpServerAvailable) {
        await expect(dropdown).toBeVisible();
      }
    });

    test('should limit number of suggestions', async ({ page }) => {
      test.skip(!mcpServerAvailable, 'MCP server not available');

      const config = TEST_CONFIGS.minimal; // maxSuggestions: 3

      await typeInEditor(page, 'te', config);
      await page.waitForTimeout(config.debounceDelay + 100);
      await waitForAutocompleteDropdown(page);

      const suggestions = await getAutocompleteSuggestions(page);
      expect(suggestions.length).toBeLessThanOrEqual(config.maxSuggestions);
    });
  });

  test.describe('Keyboard Navigation', () => {
    test('should navigate suggestions with arrow keys', async ({ page }) => {
      test.skip(!mcpServerAvailable, 'MCP server not available');

      await typeInEditor(page, 'terraphim', TEST_CONFIG);
      await page.waitForTimeout(TEST_CONFIG.debounceDelay + 100);
      await waitForAutocompleteDropdown(page);

      // Navigate down
      await navigateAutocompleteSuggestions(page, 'down', 2);

      // Verify selection changed (would need to implement getSelectedSuggestionIndex)
      const dropdown = page.locator('.terraphim-suggestion-dropdown');
      const selectedItems = dropdown.locator('.terraphim-suggestion-selected');
      expect(await selectedItems.count()).toBe(1);

      // Navigate up
      await navigateAutocompleteSuggestions(page, 'up', 1);

      // Selection should have moved up
      expect(await selectedItems.count()).toBe(1);
    });

    test('should select suggestion with Tab key', async ({ page }) => {
      test.skip(!mcpServerAvailable, 'MCP server not available');

      await typeInEditor(page, 'graph', TEST_CONFIG);
      await page.waitForTimeout(TEST_CONFIG.debounceDelay + 100);
      await waitForAutocompleteDropdown(page);

      // Get suggestions to verify what should be selected
      const suggestions = await getAutocompleteSuggestions(page);
      expect(suggestions.length).toBeGreaterThan(0);

      // Select with Tab
      await selectAutocompleteSuggestion(page, 'tab');

      // Verify dropdown closed
      const dropdown = page.locator('.terraphim-suggestion-dropdown');
      await expect(dropdown).not.toBeVisible();

      // Verify text was inserted (would need to check editor content)
      const editor = await getNovelEditor(page);
      const content = await editor.textContent();
      expect(content).toContain(suggestions[0].text);
    });

    test('should select suggestion with Enter key', async ({ page }) => {
      test.skip(!mcpServerAvailable, 'MCP server not available');

      await typeInEditor(page, 'role', TEST_CONFIG);
      await page.waitForTimeout(TEST_CONFIG.debounceDelay + 100);
      await waitForAutocompleteDropdown(page);

      const suggestions = await getAutocompleteSuggestions(page);
      expect(suggestions.length).toBeGreaterThan(0);

      await selectAutocompleteSuggestion(page, 'enter');

      const dropdown = page.locator('.terraphim-suggestion-dropdown');
      await expect(dropdown).not.toBeVisible();
    });

    test('should cancel autocomplete with Escape key', async ({ page }) => {
      test.skip(!mcpServerAvailable, 'MCP server not available');

      await typeInEditor(page, 'search', TEST_CONFIG);
      await page.waitForTimeout(TEST_CONFIG.debounceDelay + 100);
      await waitForAutocompleteDropdown(page);

      await cancelAutocomplete(page);

      const dropdown = page.locator('.terraphim-suggestion-dropdown');
      await expect(dropdown).not.toBeVisible();
    });
  });

  test.describe('Service Integration', () => {
    test('should show proper status when MCP server available', async ({ page }) => {
      test.skip(!mcpServerAvailable, 'MCP server not available');

      const status = await getAutocompleteStatus(page);
      expect(status).toMatch(/Ready.*MCP server/i);
    });

    test('should show fallback status when MCP server unavailable', async ({ page }) => {
      test.skip(mcpServerAvailable, 'MCP server is available');

      const status = await getAutocompleteStatus(page);
      expect(status).toMatch(/server.*not.*responding|mock.*autocomplete/i);
    });

    test('should handle MCP server test button', async ({ page }) => {
      await clickAutocompleteTestButton(page);

      // Should show some result or status change
      await ciWait(page, 'medium');
      const status = await getAutocompleteStatus(page);
      expect(status).toBeDefined();
    });

    test('should handle rebuild index button', async ({ page }) => {
      await clickAutocompleteRebuildButton(page);

      // Should show rebuilding status
      await waitForAutocompleteStatus(page, 'Rebuilding', getTimeouts().short);
    });
  });

  test.describe('Different Query Types', () => {
    TEST_QUERIES.basic.forEach((query) => {
      test(`should handle query: "${query}"`, async ({ page }) => {
        test.skip(!mcpServerAvailable, 'MCP server not available');

        const suggestions = await testAutocompleteFlow(page, query, TEST_CONFIG);

        if (getExpectedSuggestions(query).length > 0) {
          expect(suggestions.length).toBeGreaterThan(0);

          // Verify suggestions are relevant
          const suggestionTexts = suggestions.map(s => s.text.toLowerCase());
          const hasRelevant = suggestionTexts.some(text =>
            text.includes(query.toLowerCase()) || query.toLowerCase().includes(text)
          );
          expect(hasRelevant).toBe(true);
        }
      });
    });

    test('should handle empty query gracefully', async ({ page }) => {
      // Type trigger but no query
      const editor = await getNovelEditor(page);
      await editor.click();
      await page.keyboard.type(TEST_CONFIG.trigger);

      await page.waitForTimeout(TEST_CONFIG.debounceDelay + 100);

      // Should not show dropdown for empty query
      const dropdown = page.locator('.terraphim-suggestion-dropdown');
      await expect(dropdown).not.toBeVisible();
    });

    test('should handle special characters in query', async ({ page }) => {
      const specialQueries = ['!@#', '123', 'query-with-dashes', 'query_with_underscores'];

      for (const query of specialQueries) {
        await typeInEditor(page, query, TEST_CONFIG);
        await page.waitForTimeout(TEST_CONFIG.debounceDelay + 100);

        // Should not crash - either show suggestions or gracefully handle
        // This is more about stability than specific behavior
        const dropdown = page.locator('.terraphim-suggestion-dropdown');
        const isVisible = await dropdown.isVisible();

        if (isVisible) {
          const suggestions = await getAutocompleteSuggestions(page);
          // If suggestions appear, they should be valid
          for (const suggestion of suggestions) {
            expect(validateSuggestionStructure(suggestion)).toBe(true);
          }
          await cancelAutocomplete(page);
        }

        // Clear editor for next query
        await editor.click();
        await page.keyboard.press('Control+a');
        await page.keyboard.press('Delete');
        await ciWait(page, 'small');
      }
    });
  });

  test.describe('Performance', () => {
    test('should respond within acceptable time limits', async ({ page }) => {
      test.skip(!mcpServerAvailable, 'MCP server not available');

      const query = 'terraphim';
      const responseTime = await measureAutocompleteResponseTime(page, query, TEST_CONFIG);

      // Check against performance benchmarks
      if (isCI()) {
        expect(responseTime).toBeLessThan(PERFORMANCE_BENCHMARKS.responseTime.poor);
      } else {
        expect(responseTime).toBeLessThan(PERFORMANCE_BENCHMARKS.responseTime.acceptable);
      }
    });

    test('should respect debounce delay', async ({ page }) => {
      test.skip(!mcpServerAvailable, 'MCP server not available');

      const config = TEST_CONFIGS.fast; // 100ms debounce

      // Start timing
      const startTime = Date.now();

      await typeInEditor(page, 'test', config);

      // Wait for dropdown to appear
      await waitForAutocompleteDropdown(page);
      const endTime = Date.now();

      // Should be at least the debounce delay
      const actualDelay = endTime - startTime;
      expect(actualDelay).toBeGreaterThanOrEqual(config.debounceDelay);
    });
  });

  test.describe('Error Handling', () => {
    test('should handle network offline condition', async ({ page }) => {
      // Simulate offline condition
      await simulateNetworkConditions(page, 'offline');

      await typeInEditor(page, 'terraphim', TEST_CONFIG);
      await page.waitForTimeout(TEST_CONFIG.debounceDelay + 2000); // Extra time for timeout

      // Should either show no dropdown or show error state
      const dropdown = page.locator('.terraphim-suggestion-dropdown');
      const isVisible = await dropdown.isVisible();

      if (isVisible) {
        // If dropdown shows, should indicate error or empty state
        const emptyState = dropdown.locator('.terraphim-suggestion-empty');
        await expect(emptyState).toBeVisible();
      }

      // Restore network
      await simulateNetworkConditions(page, 'default');
    });

    test('should handle slow network condition', async ({ page }) => {
      test.skip(!mcpServerAvailable, 'MCP server not available');

      // Simulate slow network
      await simulateNetworkConditions(page, 'slow');

      const query = 'graph';
      const responseTime = await measureAutocompleteResponseTime(page, query, TEST_CONFIG);

      // Should handle slow responses gracefully
      expect(responseTime).toBeGreaterThan(1000); // Should be slowed down
      expect(responseTime).toBeLessThan(10000); // But not hang indefinitely

      // Restore network
      await simulateNetworkConditions(page, 'default');
    });
  });

  test.describe('Visual Regression', () => {
    VISUAL_TEST_SCENARIOS.forEach((scenario) => {
      test(`visual test: ${scenario.name}`, async ({ page }) => {
        test.skip(!mcpServerAvailable && scenario.expectedSuggestions > 0, 'MCP server not available');

        await typeInEditor(page, scenario.query, TEST_CONFIG);
        await page.waitForTimeout(TEST_CONFIG.debounceDelay + 100);

        if (scenario.expectedSuggestions > 0) {
          await waitForAutocompleteDropdown(page);
          const suggestions = await getAutocompleteSuggestions(page);
          expect(suggestions.length).toBeGreaterThanOrEqual(1);

          // Take screenshot for visual comparison
          await screenshotAutocompleteDropdown(page, `${scenario.name}.png`);
        } else {
          // Verify no dropdown for empty results
          const dropdown = page.locator('.terraphim-suggestion-dropdown');
          await expect(dropdown).not.toBeVisible();
        }
      });
    });

    test('should maintain consistent styling across themes', async ({ page }) => {
      test.skip(!mcpServerAvailable, 'MCP server not available');

      // Test in light theme
      await typeInEditor(page, 'terraphim', TEST_CONFIG);
      await page.waitForTimeout(TEST_CONFIG.debounceDelay + 100);
      await waitForAutocompleteDropdown(page);
      await screenshotAutocompleteDropdown(page, 'light-theme.png');
      await cancelAutocomplete(page);

      // Switch to dark theme (if available)
      const themeToggle = page.locator('[data-testid="theme-toggle"]');
      if (await themeToggle.isVisible()) {
        await themeToggle.click();
        await ciWait(page, 'small');

        // Test in dark theme
        await typeInEditor(page, 'terraphim', TEST_CONFIG);
        await page.waitForTimeout(TEST_CONFIG.debounceDelay + 100);
        await waitForAutocompleteDropdown(page);
        await screenshotAutocompleteDropdown(page, 'dark-theme.png');
      }
    });
  });

  test.describe('Accessibility', () => {
    test('should be keyboard accessible', async ({ page }) => {
      test.skip(!mcpServerAvailable, 'MCP server not available');

      // Tab to editor
      await page.keyboard.press('Tab');
      const editor = await getNovelEditor(page);
      await expect(editor).toBeFocused();

      // Type query
      await page.keyboard.type('/terraphim');
      await page.waitForTimeout(TEST_CONFIG.debounceDelay + 100);
      await waitForAutocompleteDropdown(page);

      // Navigate with arrow keys
      await page.keyboard.press('ArrowDown');
      await page.keyboard.press('ArrowDown');

      // Select with Enter
      await page.keyboard.press('Enter');

      // Verify selection worked
      const dropdown = page.locator('.terraphim-suggestion-dropdown');
      await expect(dropdown).not.toBeVisible();
    });

    test('should have proper ARIA attributes', async ({ page }) => {
      test.skip(!mcpServerAvailable, 'MCP server not available');

      await typeInEditor(page, 'graph', TEST_CONFIG);
      await page.waitForTimeout(TEST_CONFIG.debounceDelay + 100);
      const dropdown = await waitForAutocompleteDropdown(page);

      // Check for proper ARIA roles and properties
      await expect(dropdown).toHaveAttribute('role', 'listbox');

      const suggestions = dropdown.locator('.terraphim-suggestion-item');
      const firstSuggestion = suggestions.first();
      await expect(firstSuggestion).toHaveAttribute('role', 'option');
    });
  });

  test.describe('Configuration', () => {
    test('should display current configuration in UI', async ({ page }) => {
      await verifyAutocompleteConfig(page, {
        trigger: TEST_CONFIG.trigger,
        maxSuggestions: TEST_CONFIG.maxSuggestions,
      });
    });

    test('should work with alternative trigger character', async ({ page }) => {
      test.skip(!mcpServerAvailable, 'MCP server not available');

      const config = TEST_CONFIGS.alternative_trigger; // Uses '@' trigger

      await typeInEditor(page, 'terraphim', config);
      await page.waitForTimeout(config.debounceDelay + 100);

      // Should work with '@' trigger
      await waitForAutocompleteDropdown(page);
      const suggestions = await getAutocompleteSuggestions(page);
      expect(suggestions.length).toBeGreaterThan(0);
    });
  });
});

test.describe('Role-based Autocomplete', () => {
  Object.entries(ROLE_SPECIFIC_DATA).forEach(([roleName, roleData]) => {
    test(`should provide role-specific suggestions for: ${roleName}`, async ({ page }) => {
      test.skip(!mcpServerAvailable, 'MCP server not available');

      // Switch to specific role (if role selector available)
      const roleSelector = page.locator('[data-testid="role-selector"]');
      if (await roleSelector.isVisible()) {
        await roleSelector.click();
        await page.locator(`text=${roleName}`).click();
        await ciWait(page, 'medium');
      }

      // Navigate to editor
      await navigateToEditor(page);

      // Test role-specific queries
      for (const query of roleData.commonQueries) {
        const suggestions = await testAutocompleteFlow(page, query, TEST_CONFIG);

        if (suggestions.length > 0) {
          // Verify some suggestions are role-specific
          const suggestionTexts = suggestions.map(s => s.text.toLowerCase());
          const hasRoleSpecific = roleData.specificSuggestions.some(specific =>
            suggestionTexts.some(text => text.includes(specific.toLowerCase()))
          );

          if (roleData.specificSuggestions.length > 0) {
            expect(hasRoleSpecific).toBe(true);
          }
        }
      }
    });
  });
});

// Cleanup after all tests
test.afterAll(async () => {
  console.log('Autocomplete tests completed');
});