/**
 * Playwright helper functions for Novel Editor Autocomplete testing
 * These utilities provide reusable functions for testing autocomplete functionality
 */

import { Page, Locator, expect } from '@playwright/test';
import { ciWaitForSelector, ciWait, ciClick, getTimeouts, getWaitTimes } from '../../../src/test-utils/ci-friendly';

export interface AutocompleteSuggestion {
  text: string;
  snippet?: string;
  score?: number;
  type?: string;
}

export interface AutocompleteTestConfig {
  trigger: string;
  minLength: number;
  maxSuggestions: number;
  debounceDelay: number;
  mcpServerPort: number;
}

export const DEFAULT_AUTOCOMPLETE_CONFIG: AutocompleteTestConfig = {
  trigger: '/',
  minLength: 1,
  maxSuggestions: 8,
  debounceDelay: 300,
  mcpServerPort: 8001,
};

/**
 * Check if MCP server is available for testing
 */
export async function checkMCPServerHealth(port: number = 8001): Promise<boolean> {
  try {
    const response = await fetch(`http://localhost:${port}/message?sessionId=playwright-test`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        jsonrpc: '2.0',
        id: 1,
        method: 'tools/list',
        params: {}
      }),
      signal: AbortSignal.timeout(5000)
    });

    return response.ok;
  } catch (error) {
    console.warn(`MCP server health check failed on port ${port}:`, error);
    return false;
  }
}

/**
 * Wait for MCP server to be ready with retry logic
 */
export async function waitForMCPServer(port: number = 8001, maxAttempts: number = 30): Promise<boolean> {
  for (let attempt = 1; attempt <= maxAttempts; attempt++) {
    if (await checkMCPServerHealth(port)) {
      console.log(`MCP server ready on port ${port} (attempt ${attempt})`);
      return true;
    }

    console.log(`Waiting for MCP server... (attempt ${attempt}/${maxAttempts})`);
    await new Promise(resolve => setTimeout(resolve, 2000));
  }

  console.error(`MCP server failed to start after ${maxAttempts} attempts`);
  return false;
}

/**
 * Navigate to a page with editor and wait for it to be ready
 */
export async function navigateToEditor(page: Page, editorPath: string = '/editor'): Promise<void> {
  await page.goto(editorPath);

  // Wait for the editor to be present
  await ciWaitForSelector(page, '[data-testid="novel-editor"]', 'navigation');

  // Wait for the editor to be interactive
  await page.waitForFunction(() => {
    const editor = document.querySelector('[data-testid="novel-editor"]');
    return editor && !editor.classList.contains('loading');
  }, { timeout: getTimeouts().navigation });

  console.log('Novel editor is ready');
}

/**
 * Get the Novel editor element
 */
export async function getNovelEditor(page: Page): Promise<Locator> {
  const editor = await ciWaitForSelector(page, '[data-testid="novel-editor"]');
  return page.locator('[data-testid="novel-editor"]');
}

/**
 * Focus the Novel editor and position cursor
 */
export async function focusEditor(page: Page, position: 'start' | 'end' = 'end'): Promise<void> {
  const editor = await getNovelEditor(page);
  await editor.click();

  // Position cursor
  if (position === 'end') {
    await page.keyboard.press('End');
  } else {
    await page.keyboard.press('Home');
  }

  await ciWait(page, 'small');
}

/**
 * Type text in the Novel editor with autocomplete trigger
 */
export async function typeInEditor(
  page: Page,
  text: string,
  config: AutocompleteTestConfig = DEFAULT_AUTOCOMPLETE_CONFIG
): Promise<void> {
  await focusEditor(page);

  // Type the trigger character
  await page.keyboard.type(config.trigger);

  // Wait for autocomplete to be ready
  await ciWait(page, 'small');

  // Type the search text
  await page.keyboard.type(text);

  console.log(`Typed "${config.trigger}${text}" in editor`);
}

/**
 * Wait for autocomplete dropdown to appear
 */
export async function waitForAutocompleteDropdown(
  page: Page,
  timeout: number = getTimeouts().medium
): Promise<Locator> {
  const dropdown = page.locator('.terraphim-suggestion-dropdown');

  await expect(dropdown).toBeVisible({ timeout });

  // Wait for suggestions to load
  await page.waitForFunction(() => {
    const dropdown = document.querySelector('.terraphim-suggestion-dropdown');
    const items = dropdown?.querySelectorAll('.terraphim-suggestion-item:not(.terraphim-suggestion-empty)');
    return items && items.length > 0;
  }, { timeout });

  console.log('Autocomplete dropdown appeared with suggestions');
  return dropdown;
}

/**
 * Get all autocomplete suggestions from the dropdown
 */
export async function getAutocompleteSuggestions(page: Page): Promise<AutocompleteSuggestion[]> {
  const dropdown = page.locator('.terraphim-suggestion-dropdown');
  await expect(dropdown).toBeVisible();

  const suggestions: AutocompleteSuggestion[] = [];

  const items = dropdown.locator('.terraphim-suggestion-item:not(.terraphim-suggestion-empty)');
  const count = await items.count();

  for (let i = 0; i < count; i++) {
    const item = items.nth(i);

    const text = await item.locator('.terraphim-suggestion-text').textContent() || '';
    const snippet = await item.locator('.terraphim-suggestion-snippet').textContent() || undefined;
    const scoreElement = item.locator('.terraphim-suggestion-score');
    const scoreText = await scoreElement.textContent();
    const score = scoreText ? parseFloat(scoreText.replace('%', '')) / 100 : undefined;

    suggestions.push({ text, snippet, score });
  }

  console.log(`Found ${suggestions.length} autocomplete suggestions`);
  return suggestions;
}

/**
 * Get the currently selected suggestion index
 */
export async function getSelectedSuggestionIndex(page: Page): Promise<number> {
  const dropdown = page.locator('.terraphim-suggestion-dropdown');
  const selectedItem = dropdown.locator('.terraphim-suggestion-selected');
  const allItems = dropdown.locator('.terraphim-suggestion-item:not(.terraphim-suggestion-empty)');

  const count = await allItems.count();
  for (let i = 0; i < count; i++) {
    if (await allItems.nth(i).locator('.terraphim-suggestion-selected').isVisible()) {
      return i;
    }
  }

  return 0; // Default to first item
}

/**
 * Navigate autocomplete suggestions with keyboard
 */
export async function navigateAutocompleteSuggestions(
  page: Page,
  direction: 'up' | 'down',
  steps: number = 1
): Promise<void> {
  const key = direction === 'up' ? 'ArrowUp' : 'ArrowDown';

  for (let i = 0; i < steps; i++) {
    await page.keyboard.press(key);
    await ciWait(page, 'tiny');
  }

  console.log(`Navigated ${direction} ${steps} step(s) in autocomplete`);
}

/**
 * Select current autocomplete suggestion
 */
export async function selectAutocompleteSuggestion(
  page: Page,
  method: 'tab' | 'enter' | 'click' = 'tab',
  index?: number
): Promise<void> {
  if (method === 'click' && index !== undefined) {
    const dropdown = page.locator('.terraphim-suggestion-dropdown');
    const items = dropdown.locator('.terraphim-suggestion-item:not(.terraphim-suggestion-empty)');
    await items.nth(index).click();
  } else {
    const key = method === 'tab' ? 'Tab' : 'Enter';
    await page.keyboard.press(key);
  }

  // Wait for suggestion to be inserted and dropdown to close
  await page.waitForFunction(() => {
    const dropdown = document.querySelector('.terraphim-suggestion-dropdown');
    return !dropdown || !dropdown.isConnected;
  }, { timeout: getTimeouts().short });

  console.log(`Selected autocomplete suggestion via ${method}`);
}

/**
 * Cancel autocomplete dropdown
 */
export async function cancelAutocomplete(page: Page): Promise<void> {
  await page.keyboard.press('Escape');

  // Wait for dropdown to close
  await page.waitForFunction(() => {
    const dropdown = document.querySelector('.terraphim-suggestion-dropdown');
    return !dropdown || !dropdown.isConnected;
  }, { timeout: getTimeouts().short });

  console.log('Cancelled autocomplete');
}

/**
 * Test autocomplete end-to-end flow
 */
export async function testAutocompleteFlow(
  page: Page,
  query: string,
  config: AutocompleteTestConfig = DEFAULT_AUTOCOMPLETE_CONFIG
): Promise<AutocompleteSuggestion[]> {
  console.log(`Testing autocomplete flow with query: "${query}"`);

  // Type the query
  await typeInEditor(page, query, config);

  // Wait for debounce delay
  await page.waitForTimeout(config.debounceDelay + 100);

  // Wait for dropdown
  await waitForAutocompleteDropdown(page);

  // Get suggestions
  const suggestions = await getAutocompleteSuggestions(page);

  // Cancel to clean up
  await cancelAutocomplete(page);

  return suggestions;
}

/**
 * Measure autocomplete response time
 */
export async function measureAutocompleteResponseTime(
  page: Page,
  query: string,
  config: AutocompleteTestConfig = DEFAULT_AUTOCOMPLETE_CONFIG
): Promise<number> {
  console.log(`Measuring autocomplete response time for: "${query}"`);

  await focusEditor(page);

  // Start timing
  const startTime = Date.now();

  // Type trigger and query
  await page.keyboard.type(config.trigger + query);

  // Wait for dropdown to appear
  await waitForAutocompleteDropdown(page);

  // End timing
  const endTime = Date.now();
  const responseTime = endTime - startTime;

  // Clean up
  await cancelAutocomplete(page);

  console.log(`Autocomplete response time: ${responseTime}ms`);
  return responseTime;
}

/**
 * Check autocomplete status panel
 */
export async function getAutocompleteStatus(page: Page): Promise<string> {
  try {
    const statusElement = await page.waitForSelector('[data-testid="autocomplete-status"]', {
      timeout: getTimeouts().short
    });
    const status = await statusElement.textContent() || 'Unknown';
    console.log(`Autocomplete status: ${status}`);
    return status;
  } catch (error) {
    console.warn('Could not get autocomplete status:', error);
    return 'Status unavailable';
  }
}

/**
 * Click autocomplete test button in UI
 */
export async function clickAutocompleteTestButton(page: Page): Promise<void> {
  const testButton = await ciWaitForSelector(page, '[data-testid="autocomplete-test-button"]');
  await testButton.click();

  // Wait for test to complete
  await ciWait(page, 'medium');

  console.log('Clicked autocomplete test button');
}

/**
 * Click autocomplete rebuild button in UI
 */
export async function clickAutocompleteRebuildButton(page: Page): Promise<void> {
  const rebuildButton = await ciWaitForSelector(page, '[data-testid="autocomplete-rebuild-button"]');
  await rebuildButton.click();

  // Wait for rebuild to complete
  await ciWait(page, 'large');

  console.log('Clicked autocomplete rebuild button');
}

/**
 * Verify autocomplete configuration in UI
 */
export async function verifyAutocompleteConfig(
  page: Page,
  expectedConfig: Partial<AutocompleteTestConfig>
): Promise<void> {
  // Check trigger character
  if (expectedConfig.trigger) {
    const triggerText = await page.textContent('[data-testid="autocomplete-trigger"]');
    expect(triggerText).toContain(expectedConfig.trigger);
  }

  // Check max suggestions
  if (expectedConfig.maxSuggestions) {
    const maxSuggestionsText = await page.textContent('[data-testid="autocomplete-max-results"]');
    expect(maxSuggestionsText).toContain(expectedConfig.maxSuggestions.toString());
  }

  console.log('Verified autocomplete configuration');
}

/**
 * Wait for specific autocomplete status
 */
export async function waitForAutocompleteStatus(
  page: Page,
  expectedStatus: string,
  timeout: number = getTimeouts().medium
): Promise<void> {
  await page.waitForFunction(
    (status) => {
      const statusElement = document.querySelector('[data-testid="autocomplete-status"]');
      return statusElement && statusElement.textContent && statusElement.textContent.includes(status);
    },
    expectedStatus,
    { timeout }
  );

  console.log(`Autocomplete status reached: ${expectedStatus}`);
}

/**
 * Simulate network conditions for autocomplete testing
 */
export async function simulateNetworkConditions(
  page: Page,
  conditions: 'offline' | 'slow' | 'fast' | 'default' = 'default'
): Promise<void> {
  const context = page.context();

  switch (conditions) {
    case 'offline':
      await context.setOffline(true);
      break;
    case 'slow':
      await context.setOffline(false);
      await context.route('**/*', route => {
        setTimeout(() => route.continue(), 2000);
      });
      break;
    case 'fast':
      await context.setOffline(false);
      await context.unroute('**/*');
      break;
    case 'default':
      await context.setOffline(false);
      await context.unroute('**/*');
      break;
  }

  console.log(`Network conditions set to: ${conditions}`);
}

/**
 * Take screenshot of autocomplete dropdown for visual testing
 */
export async function screenshotAutocompleteDropdown(
  page: Page,
  filename: string
): Promise<void> {
  const dropdown = page.locator('.terraphim-suggestion-dropdown');
  await expect(dropdown).toBeVisible();

  await dropdown.screenshot({ path: `test-results/screenshots/${filename}` });
  console.log(`Screenshot saved: ${filename}`);
}
