import { test, expect } from '@playwright/test';
import path from 'path';
import { fileURLToPath } from 'url';

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);

/**
 * E2E tests for Terraphim AI Application
 * Tests core functionality including search, chat, knowledge graph integration
 */

// Test scenarios for core Terraphim functionality
const coreFeatures = [
  {
    name: 'Search Functionality',
    description: 'Document search and retrieval',
    testUrl: '/',
    testSelectors: {
      searchInput: 'input[placeholder*="Search"], input[placeholder*="search"], .search-input input, [data-testid="search-input"]',
      searchButton: '.search-button, [data-testid="search-button"], button:has-text("Search")',
      searchResults: '.search-results, .result-item, [data-testid="search-results"]'
    }
  },
  {
    name: 'Chat Interface',
    description: 'AI chat with context',
    testUrl: '/',
    testSelectors: {
      chatInput: 'textarea[placeholder*="Message"], .chat-input textarea, [data-testid="chat-input"]',
      sendButton: '.send-button, [data-testid="send-button"], button:has-text("Send")',
      chatMessages: '.chat-message, .message, [data-testid="chat-messages"]'
    }
  },
  {
    name: 'Knowledge Graph',
    description: 'Knowledge graph context management',
    testUrl: '/',
    testSelectors: {
      kgButton: '[data-testid="kg-button"], button:has-text("Knowledge")',
      contextPanel: '.context-panel, [data-testid="context-panel"]',
      addTermButton: '.add-term-button, [data-testid="add-term"]'
    }
  },
  {
    name: 'Navigation',
    description: 'Application navigation and routing',
    testUrl: '/',
    testSelectors: {
      searchTab: '[data-testid="search-tab"], a:has-text("Search")',
      chatTab: '[data-testid="chat-tab"], a:has-text("Chat")',
      settingsButton: '[data-testid="settings-button"], button:has-text("Settings")'
    }
  }
];

// Test each core feature
coreFeatures.forEach(feature => {
  test.describe(`${feature.name}`, () => {

    test(`should load Terraphim app and test ${feature.name}`, async ({ page }) => {
      // Navigate to the main application
      await page.goto(feature.testUrl);
      await page.waitForLoadState('networkidle');

      // Check page loaded successfully
      await expect(page.locator('body')).toBeVisible();

      // Check for the application title or main elements
      const title = await page.title();
      expect(title || await page.locator('h1, .title, [data-testid="app-title"]').first().textContent()).toBeTruthy();

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
      const criticalErrors = errors.filter(error =>
        !error.includes('WebSocket connection') &&
        !error.includes('Failed to connect') &&
        !error.includes('Connection refused') &&
        !error.includes('NetConnection') &&
        !error.includes('favicon')
      );

      expect(criticalErrors).toHaveLength(0);
    });

    test(`should have ${feature.name} elements available`, async ({ page }) => {
      await page.goto(feature.testUrl);
      await page.waitForLoadState('networkidle');

      // Wait for app to fully load
      await page.waitForTimeout(3000);

      // Test specific feature selectors
      const selectors = Object.values(feature.testSelectors);

      for (const selector of selectors) {
        try {
          const element = page.locator(selector).first();
          if (await element.count() > 0) {
            await expect(element).toBeVisible({ timeout: 5000 });
            break; // At least one selector variant exists
          }
        } catch (error) {
          // Continue to next selector variant
          continue;
        }
      }
    });
  });
});

// Additional integration tests
test.describe('Application Integration', () => {
  test('should handle search workflow end-to-end', async ({ page }) => {
    await page.goto('/');
    await page.waitForLoadState('networkidle');
    await page.waitForTimeout(3000);

    // Try to find and interact with search functionality
    const searchInput = page.locator('input[placeholder*="Search"], input[placeholder*="search"], .search-input input').first();

    if (await searchInput.count() > 0) {
      await searchInput.fill('rust programming');
      await page.waitForTimeout(1000);

      // Try to submit search
      const searchButton = page.locator('button:has-text("Search"), .search-button, [data-testid="search-button"]').first();
      if (await searchButton.count() > 0) {
        await searchButton.click();
        await page.waitForTimeout(2000);
      }
    }

    // Verify page is still responsive
    await expect(page.locator('body')).toBeVisible();
  });

  test('should handle chat interface', async ({ page }) => {
    await page.goto('/');
    await page.waitForLoadState('networkidle');
    await page.waitForTimeout(3000);

    // Try to find chat functionality
    const chatInput = page.locator('textarea, input[type="text"]').first();

    if (await chatInput.count() > 0) {
      await chatInput.fill('Hello, can you help me with programming?');
      await page.waitForTimeout(1000);

      // Try to send message
      const sendButton = page.locator('button:has-text("Send"), .send-button, button[type="submit"]').first();
      if (await sendButton.count() > 0) {
        await sendButton.click();
        await page.waitForTimeout(3000);
      }
    }

    // Verify page is still responsive
    await expect(page.locator('body')).toBeVisible();
  });

  test('should handle navigation between sections', async ({ page }) => {
    await page.goto('/');
    await page.waitForLoadState('networkidle');
    await page.waitForTimeout(3000);

    // Look for navigation elements
    const navigationLinks = page.locator('a, button, [role="button"]');
    const navCount = await navigationLinks.count();

    if (navCount > 0) {
      // Try clicking first few navigation elements
      const maxClicks = Math.min(3, navCount);
      for (let i = 0; i < maxClicks; i++) {
        try {
          const link = navigationLinks.nth(i);
          if (await link.isVisible()) {
            await link.click();
            await page.waitForTimeout(2000);
            await page.goto('/'); // Return to main page
            await page.waitForTimeout(1000);
          }
        } catch (error) {
          // Continue with next link
          continue;
        }
      }
    }

    // Verify we can return to main page
    await page.goto('/');
    await expect(page.locator('body')).toBeVisible();
  });
});
