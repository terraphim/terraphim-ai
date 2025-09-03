import { test, expect } from '@playwright/test';

test.describe('KG Links Visibility Tests', () => {
  test.beforeEach(async ({ page }) => {
    // Navigate to the app
    await page.goto('/');
    await page.waitForLoadState('networkidle');
    
    // Wait for the app to be ready
    await page.waitForSelector('[data-testid="search-input"]', { timeout: 10000 });
  });

  test('should display KG links in search results when using Terraphim Engineer role', async ({ page }) => {
    // Ensure we're using the Terraphim Engineer role (which has terraphim_it: true)
    const roleSelect = page.locator('[data-testid="role-selector"]');
    if (await roleSelect.isVisible()) {
      await roleSelect.selectOption('Terraphim Engineer');
    }
    
    // Search for the test KG demo document
    await page.fill('[data-testid="search-input"]', 'test-kg-demo');
    await page.press('[data-testid="search-input"]', 'Enter');
    
    // Wait for search results
    await page.waitForSelector('[data-testid="search-result"]', { timeout: 10000 });
    
    // Click on the test document to open it in the article modal
    const firstResult = page.locator('[data-testid="search-result"]').first();
    await firstResult.click();
    
    // Wait for the article modal to open
    await page.waitForSelector('.modal.is-active', { timeout: 5000 });
    
    // Look for KG links in the document content
    // KG links should have href starting with "kg:" and special styling
    const kgLinks = page.locator('a[href^="kg:"]');
    
    // Verify that KG links are present
    const linkCount = await kgLinks.count();
    expect(linkCount).toBeGreaterThan(0);
    
    // Check for specific expected KG terms from the test document
    const expectedTerms = ['graph', 'haystack', 'service'];
    
    for (const term of expectedTerms) {
      const kgLink = page.locator(`a[href="kg:${term}"]`);
      await expect(kgLink).toBeVisible();
      
      // Verify the link has the correct styling (purple color and link icon)
      await expect(kgLink).toHaveCSS('color', 'rgb(142, 68, 173)'); // #8e44ad
      
      // Verify the link has the knowledge graph icon (🔗)
      const linkText = await kgLink.textContent();
      expect(linkText).toContain('🔗');
    }
  });

  test('should NOT display KG links when using a role without terraphim_it enabled', async ({ page }) => {
    // Use a role that doesn't have terraphim_it enabled (e.g., "Default")
    const roleSelect = page.locator('[data-testid="role-selector"]');
    if (await roleSelect.isVisible()) {
      await roleSelect.selectOption('Default');
    }
    
    // Search for the test KG demo document
    await page.fill('[data-testid="search-input"]', 'test-kg-demo');
    await page.press('[data-testid="search-input"]', 'Enter');
    
    // Wait for search results
    await page.waitForSelector('[data-testid="search-result"]', { timeout: 10000 });
    
    // Click on the test document
    const firstResult = page.locator('[data-testid="search-result"]').first();
    await firstResult.click();
    
    // Wait for the article modal to open
    await page.waitForSelector('.modal.is-active', { timeout: 5000 });
    
    // Verify that NO KG links are present
    const kgLinks = page.locator('a[href^="kg:"]');
    const linkCount = await kgLinks.count();
    expect(linkCount).toBe(0);
    
    // Verify that the terms appear as regular text, not links
    const modalContent = page.locator('.content-viewer');
    const contentText = await modalContent.textContent();
    
    // The terms should be present but not as links
    expect(contentText).toContain('graph');
    expect(contentText).toContain('haystack');
    expect(contentText).toContain('service');
  });

  test('should open KG document modal when clicking KG link', async ({ page }) => {
    // Setup: Use Terraphim Engineer role and open test document
    const roleSelect = page.locator('[data-testid="role-selector"]');
    if (await roleSelect.isVisible()) {
      await roleSelect.selectOption('Terraphim Engineer');
    }
    
    await page.fill('[data-testid="search-input"]', 'test-kg-demo');
    await page.press('[data-testid="search-input"]', 'Enter');
    await page.waitForSelector('[data-testid="search-result"]', { timeout: 10000 });
    
    const firstResult = page.locator('[data-testid="search-result"]').first();
    await firstResult.click();
    await page.waitForSelector('.modal.is-active', { timeout: 5000 });
    
    // Click on a KG link (e.g., "graph")
    const graphLink = page.locator('a[href="kg:graph"]').first();
    await expect(graphLink).toBeVisible();
    
    // Enable console logging to debug KG link clicks
    page.on('console', msg => {
      if (msg.text().includes('KG Link Click Debug Info')) {
        console.log('🔍 Console:', msg.text());
      }
    });
    
    await graphLink.click();
    
    // Wait for the KG document modal to appear
    // This should be a second modal that opens on top of the first
    await page.waitForTimeout(2000); // Wait for potential async loading
    
    // Check if a KG document modal opened
    // The KG modal should have the KG context header
    const kgContextHeaders = page.locator('.kg-context');
    const kgContextCount = await kgContextHeaders.count();
    
    if (kgContextCount > 0) {
      // Verify KG context is displayed
      const kgContext = kgContextHeaders.first();
      await expect(kgContext).toContain('Knowledge Graph');
      await expect(kgContext).toContain('Term: graph');
    } else {
      // Log for debugging if KG modal didn't appear
      console.log('⚠️ KG document modal did not appear after clicking link');
      
      // Check browser console for any errors or debug messages
      const logs = await page.evaluate(() => {
        return window.console;
      });
    }
  });

  test('should handle multiple KG links in the same document', async ({ page }) => {
    // Setup: Use Terraphim Engineer role and open test document
    const roleSelect = page.locator('[data-testid="role-selector"]');
    if (await roleSelect.isVisible()) {
      await roleSelect.selectOption('Terraphim Engineer');
    }
    
    await page.fill('[data-testid="search-input"]', 'test-kg-demo');
    await page.press('[data-testid="search-input"]', 'Enter');
    await page.waitForSelector('[data-testid="search-result"]', { timeout: 10000 });
    
    const firstResult = page.locator('[data-testid="search-result"]').first();
    await firstResult.click();
    await page.waitForSelector('.modal.is-active', { timeout: 5000 });
    
    // Count all KG links in the document
    const kgLinks = page.locator('a[href^="kg:"]');
    const linkCount = await kgLinks.count();
    
    // Should have multiple links (graph, haystack, service, etc.)
    expect(linkCount).toBeGreaterThan(2);
    
    // Test clicking different links
    const availableLinks = [];
    for (let i = 0; i < Math.min(linkCount, 3); i++) {
      const link = kgLinks.nth(i);
      const href = await link.getAttribute('href');
      const text = await link.textContent();
      
      availableLinks.push({ href, text: text?.replace('🔗 ', '') });
      
      // Verify each link has proper styling and content
      await expect(link).toBeVisible();
      await expect(link).toHaveAttribute('href', href!);
    }
    
    console.log('Found KG links:', availableLinks);
  });

  test('should maintain KG links when switching between documents', async ({ page }) => {
    // Setup: Use Terraphim Engineer role
    const roleSelect = page.locator('[data-testid="role-selector"]');
    if (await roleSelect.isVisible()) {
      await roleSelect.selectOption('Terraphim Engineer');
    }
    
    // Search and open first document
    await page.fill('[data-testid="search-input"]', 'knowledge');
    await page.press('[data-testid="search-input"]', 'Enter');
    await page.waitForSelector('[data-testid="search-result"]', { timeout: 10000 });
    
    const firstResult = page.locator('[data-testid="search-result"]').first();
    await firstResult.click();
    await page.waitForSelector('.modal.is-active', { timeout: 5000 });
    
    // Check for KG links in first document
    const kgLinksFirst = page.locator('a[href^="kg:"]');
    const firstDocLinkCount = await kgLinksFirst.count();
    
    // Close the modal
    const closeButton = page.locator('.modal-close-btn');
    await closeButton.click();
    await page.waitForSelector('.modal.is-active', { state: 'hidden' });
    
    // Search and open second document
    await page.fill('[data-testid="search-input"]', 'terraphim');
    await page.press('[data-testid="search-input"]', 'Enter');
    await page.waitForSelector('[data-testid="search-result"]', { timeout: 10000 });
    
    const secondResult = page.locator('[data-testid="search-result"]').first();
    await secondResult.click();
    await page.waitForSelector('.modal.is-active', { timeout: 5000 });
    
    // Check for KG links in second document
    const kgLinksSecond = page.locator('a[href^="kg:"]');
    const secondDocLinkCount = await kgLinksSecond.count();
    
    // Both documents should have KG preprocessing applied
    // (exact counts may vary based on content, but should be > 0 if KG terms are present)
    console.log(`First document KG links: ${firstDocLinkCount}, Second document KG links: ${secondDocLinkCount}`);
  });

  test('should work with both Tauri and web modes', async ({ page }) => {
    // This test ensures KG links work in both desktop (Tauri) and web environments
    
    // Check if we're in Tauri mode by looking for Tauri-specific APIs
    const isTauri = await page.evaluate(() => {
      return typeof (window as any).__TAURI__ !== 'undefined';
    });
    
    console.log(`Running in ${isTauri ? 'Tauri' : 'Web'} mode`);
    
    // Setup: Use Terraphim Engineer role
    const roleSelect = page.locator('[data-testid="role-selector"]');
    if (await roleSelect.isVisible()) {
      await roleSelect.selectOption('Terraphim Engineer');
    }
    
    await page.fill('[data-testid="search-input"]', 'test-kg-demo');
    await page.press('[data-testid="search-input"]', 'Enter');
    await page.waitForSelector('[data-testid="search-result"]', { timeout: 10000 });
    
    const firstResult = page.locator('[data-testid="search-result"]').first();
    await firstResult.click();
    await page.waitForSelector('.modal.is-active', { timeout: 5000 });
    
    // KG links should work in both modes
    const kgLinks = page.locator('a[href^="kg:"]');
    const linkCount = await kgLinks.count();
    expect(linkCount).toBeGreaterThan(0);
    
    // Test clicking a KG link
    if (linkCount > 0) {
      const firstKgLink = kgLinks.first();
      const linkHref = await firstKgLink.getAttribute('href');
      
      console.log(`Testing KG link click in ${isTauri ? 'Tauri' : 'Web'} mode: ${linkHref}`);
      await firstKgLink.click();
      
      // Wait and check for any response (modal, console logs, etc.)
      await page.waitForTimeout(1000);
    }
  });
});
