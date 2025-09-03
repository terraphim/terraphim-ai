import { test, expect } from '@playwright/test';
import {
  ciWaitForSelector,
  ciNavigate,
  ciWait,
  ciClick,
  ciHover,
  getTimeouts
} from '../../src/test-utils/ci-friendly';

test.describe('App Navigation', () => {
  test.beforeEach(async ({ page }) => {
    // Navigate to the app using CI-friendly navigation
    await ciNavigate(page, '/');

    // Wait for the app to load
    await ciWaitForSelector(page, 'input[type="search"]', 'navigation');
  });

  test('should display home page correctly', async ({ page }) => {
    // Check main elements are present
    await expect(page.locator('input[type="search"]')).toBeVisible();
    await expect(page.locator('img[alt="Terraphim Logo"]')).toBeVisible();
    await expect(page.locator('text=I am Terraphim, your personal assistant.')).toBeVisible();
  });

  test('should show navigation menu on footer hover', async ({ page }) => {
    // Use CI-friendly hover action
    const footer = await ciHover(page, 'footer');

    // Navigation should become visible
    const nav = await ciWaitForSelector(page, 'nav', 'medium');
    await expect(nav).toBeVisible();

    // Check for navigation links
    const links = await nav.locator('a').count();
    expect(links).toBeGreaterThan(0);
  });

  test('should navigate to config wizard', async ({ page }) => {
    // Hover over footer to reveal navigation
    await ciHover(page, 'footer');

    // Click on wizard link using CI-friendly click
    await ciClick(page, 'a[href="/config/wizard"]');

    // Verify navigation to wizard
    await expect(page).toHaveURL('/config/wizard');
    await ciWaitForSelector(page, 'h3:has-text("Configuration Wizard")', 'navigation');
  });

  test('should navigate to graph visualization', async ({ page }) => {
    // Hover over footer to reveal navigation
    await ciHover(page, 'footer');

    // Click on graph link
    await ciClick(page, 'a[href="/graph"]');

    // Verify navigation to graph
    await expect(page).toHaveURL('/graph');
    await ciWaitForSelector(page, 'svg', 'navigation');
  });

  test('should navigate to JSON editor', async ({ page }) => {
    // Hover over footer to reveal navigation
    await ciHover(page, 'footer');

    // Click on JSON editor link
    await ciClick(page, 'a[href="/config/json"]');

    // Verify navigation to JSON editor
    await expect(page).toHaveURL('/config/json');
    await ciWaitForSelector(page, 'textarea, .json-editor', 'navigation');
  });

  test('should return to home from any screen', async ({ page }) => {
    // Navigate to wizard first
    await ciHover(page, 'footer');
    await ciClick(page, 'a[href="/config/wizard"]');
    await ciWait(page, 'afterNavigation');

    // Navigate back to home
    await ciHover(page, 'footer');
    await ciClick(page, 'a[href="/"]');

    // Verify back on home page
    await expect(page).toHaveURL('/');
    await ciWaitForSelector(page, 'input[type="search"]', 'navigation');
    await expect(page.locator('img[alt="Terraphim Logo"]')).toBeVisible();
  });

  test('should maintain navigation consistency across screens', async ({ page }) => {
    // Check navigation on home page
    await ciHover(page, 'footer');

    const homeNavItems = await page.locator('nav a').count();
    expect(homeNavItems).toBeGreaterThan(0);

    // Navigate to wizard and check navigation
    await ciClick(page, 'a[href="/config/wizard"]');
    await ciWait(page, 'afterNavigation');

    await ciHover(page, 'footer');

    const wizardNavItems = await page.locator('nav a').count();
    expect(wizardNavItems).toBeGreaterThan(0);

    // Navigate to graph and check navigation
    await ciClick(page, 'a[href="/graph"]');
    await ciWait(page, 'afterNavigation');

    await ciHover(page, 'footer');

    const graphNavItems = await page.locator('nav a').count();
    expect(graphNavItems).toBeGreaterThan(0);
  });

  test('should handle navigation errors gracefully', async ({ page }) => {
    // Try to navigate to non-existent page
    await ciNavigate(page, '/non-existent-page');

    // Should handle gracefully (either 404 page or redirect to home)
    // Wait for page to stabilize
    await ciWait(page, 'afterNavigation');

    // App should still be functional - either show 404 or redirect to home
    const hasSearch = await page.locator('input[type="search"]').isVisible();
    const has404 = await page.locator('text=404, text=Not Found').isVisible();

    // Either should be true
    expect(hasSearch || has404).toBeTruthy();
  });

  test('should maintain app state during navigation', async ({ page }) => {
    // Enter a search query on home page
    const searchInput = await ciWaitForSelector(page, 'input[type="search"]');
    await searchInput.fill('test query');

    // Navigate to wizard
    await ciHover(page, 'footer');
    await ciClick(page, 'a[href="/config/wizard"]');
    await ciWait(page, 'afterNavigation');

    // Navigate back to home
    await ciHover(page, 'footer');
    await ciClick(page, 'a[href="/"]');

    // Search input should be cleared (fresh state)
    const searchInputAfter = await ciWaitForSelector(page, 'input[type="search"]');
    const inputValue = await searchInputAfter.inputValue();
    expect(inputValue).toBe('');
  });

  test('should load pages within acceptable time limits', async ({ page }) => {
    const timeouts = getTimeouts();
    const startTime = Date.now();

    // Navigate to wizard
    await ciHover(page, 'footer');
    await ciClick(page, 'a[href="/config/wizard"]');
    await ciWaitForSelector(page, 'h3:has-text("Configuration Wizard")', 'navigation');

    const wizardLoadTime = Date.now() - startTime;
    expect(wizardLoadTime).toBeLessThan(timeouts.navigation);

    // Navigate to graph
    const graphStartTime = Date.now();
    await ciHover(page, 'footer');
    await ciClick(page, 'a[href="/graph"]');
    await ciWaitForSelector(page, 'svg', 'navigation');

    const graphLoadTime = Date.now() - graphStartTime;
    expect(graphLoadTime).toBeLessThan(timeouts.navigation);
  });
});
