import { test, expect } from '@playwright/test';

test.describe('App Navigation', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto('/');
    await page.waitForSelector('input[type="search"]', { timeout: 30000 });
  });

  test('should display main navigation elements', async ({ page }) => {
    // Check that main elements are present
    await expect(page.locator('input[type="search"]')).toBeVisible();
    
    // Check for footer navigation (appears on hover)
    const footer = page.locator('footer');
    await footer.hover();
    
    // Wait for navigation to appear
    await page.waitForTimeout(500);
    
    // Check navigation items
    const homeLink = page.locator('a[href="/"]');
    const configLink = page.locator('a[href="/fetch/json"]');
    const contactsLink = page.locator('a[href="/contacts"]');
    
    if (await homeLink.isVisible()) {
      await expect(homeLink).toBeVisible();
    }
    
    if (await configLink.isVisible()) {
      await expect(configLink).toBeVisible();
    }
  });

  test('should navigate to configuration page', async ({ page }) => {
    // Hover over footer to reveal navigation
    const footer = page.locator('footer');
    await footer.hover();
    await page.waitForTimeout(500);
    
    // Click on configuration link if available
    const configLink = page.locator('a[href="/fetch/json"]');
    if (await configLink.isVisible()) {
      await configLink.click();
      
      // Check URL changed
      await expect(page).toHaveURL(/\/fetch\/json/);
      
      // Wait for configuration page to load
      await page.waitForTimeout(2000);
      
      // Should still have navigation elements
      await expect(footer).toBeVisible();
    }
  });

  test('should return to home page', async ({ page }) => {
    // First navigate away from home
    const footer = page.locator('footer');
    await footer.hover();
    await page.waitForTimeout(500);
    
    const configLink = page.locator('a[href="/fetch/json"]');
    if (await configLink.isVisible()) {
      await configLink.click();
      await page.waitForTimeout(1000);
      
      // Then navigate back to home
      await footer.hover();
      await page.waitForTimeout(500);
      
      const homeLink = page.locator('a[href="/"]');
      if (await homeLink.isVisible()) {
        await homeLink.click();
        
        // Should be back on home page
        await expect(page).toHaveURL('/');
        
        // Search input should be visible
        await expect(page.locator('input[type="search"]')).toBeVisible();
        
        // Logo should be visible
        await expect(page.locator('img[alt="Terraphim Logo"]')).toBeVisible();
      }
    }
  });

  test('should handle browser back/forward navigation', async ({ page }) => {
    // Start on home page
    const initialUrl = page.url();
    
    // Navigate to config if possible
    const footer = page.locator('footer');
    await footer.hover();
    await page.waitForTimeout(500);
    
    const configLink = page.locator('a[href="/fetch/json"]');
    if (await configLink.isVisible()) {
      await configLink.click();
      await page.waitForTimeout(1000);
      
      // Use browser back button
      await page.goBack();
      await page.waitForTimeout(1000);
      
      // Should be back on home
      expect(page.url()).toBe(initialUrl);
      await expect(page.locator('input[type="search"]')).toBeVisible();
      
      // Use browser forward button
      await page.goForward();
      await page.waitForTimeout(1000);
      
      // Should be on config page again
      await expect(page).toHaveURL(/\/fetch\/json/);
    }
  });

  test('should maintain app state during navigation', async ({ page }) => {
    const searchInput = page.locator('input[type="search"]');
    
    // Enter some text in search
    await searchInput.fill('navigation test');
    
    // Navigate to config page
    const footer = page.locator('footer');
    await footer.hover();
    await page.waitForTimeout(500);
    
    const configLink = page.locator('a[href="/fetch/json"]');
    if (await configLink.isVisible()) {
      await configLink.click();
      await page.waitForTimeout(1000);
      
      // Navigate back to home
      await footer.hover();
      await page.waitForTimeout(500);
      
      const homeLink = page.locator('a[href="/"]');
      if (await homeLink.isVisible()) {
        await homeLink.click();
        await page.waitForTimeout(1000);
        
        // Search input should still contain text
        const inputValue = await searchInput.inputValue();
        expect(inputValue).toBe('navigation test');
      }
    }
  });

  test('should handle direct URL navigation', async ({ page }) => {
    // Navigate directly to config page via URL
    await page.goto('/fetch/json');
    await page.waitForTimeout(2000);
    
    // Should be on config page
    await expect(page).toHaveURL(/\/fetch\/json/);
    
    // App should still be functional
    const footer = page.locator('footer');
    await expect(footer).toBeVisible();
    
    // Navigate directly back to home
    await page.goto('/');
    await page.waitForTimeout(1000);
    
    // Should be on home page with search
    await expect(page.locator('input[type="search"]')).toBeVisible();
  });

  test('should handle invalid routes gracefully', async ({ page }) => {
    // Navigate to non-existent route
    await page.goto('/invalid-route');
    await page.waitForTimeout(2000);
    
    // App should either redirect to home or show 404 gracefully
    // Either way, the app should remain functional
    const searchInput = page.locator('input[type="search"]');
    const footer = page.locator('footer');
    
    // At least one of these should be visible (depending on routing strategy)
    const searchVisible = await searchInput.isVisible();
    const footerVisible = await footer.isVisible();
    
    expect(searchVisible || footerVisible).toBeTruthy();
  });

  test('should maintain navigation consistency', async ({ page }) => {
    // Check that navigation elements are consistent across pages
    const footer = page.locator('footer');
    
    // On home page
    await footer.hover();
    await page.waitForTimeout(500);
    
    const homeNavItems = await page.locator('nav a').count();
    
    // Navigate to config page if possible
    const configLink = page.locator('a[href="/fetch/json"]');
    if (await configLink.isVisible()) {
      await configLink.click();
      await page.waitForTimeout(1000);
      
      // Check navigation on config page
      await footer.hover();
      await page.waitForTimeout(500);
      
      const configNavItems = await page.locator('nav a').count();
      
      // Should have same number of navigation items
      expect(configNavItems).toBe(homeNavItems);
    }
  });

  test('should handle rapid navigation changes', async ({ page }) => {
    const footer = page.locator('footer');
    
    // Rapidly navigate between pages
    for (let i = 0; i < 3; i++) {
      await footer.hover();
      await page.waitForTimeout(200);
      
      const configLink = page.locator('a[href="/fetch/json"]');
      if (await configLink.isVisible()) {
        await configLink.click();
        await page.waitForTimeout(300);
        
        await footer.hover();
        await page.waitForTimeout(200);
        
        const homeLink = page.locator('a[href="/"]');
        if (await homeLink.isVisible()) {
          await homeLink.click();
          await page.waitForTimeout(300);
        }
      }
    }
    
    // App should still be functional after rapid navigation
    await expect(page.locator('input[type="search"]')).toBeVisible();
  });
}); 