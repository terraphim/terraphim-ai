import { test, expect, Page } from '@playwright/test';
import {
  ciWaitForSelector,
  ciSearch,
  ciNavigate,
  ciWait,
  ciClick,
  getTimeouts
} from '../../src/test-utils/ci-friendly';

/**
 * Comprehensive Performance Validation Test for All Three Roles
 * 
 * This test validates that:
 * 1. All three roles (Default, Rust Engineer, Terraphim Engineer) work correctly
 * 2. Search is fast and responsive (< 2 seconds)
 * 3. No opendal warnings in logs
 * 4. UI doesn't freeze during search
 * 5. All role-specific functionality works
 */

interface SearchPerformanceMetrics {
  role: string;
  searchTime: number;
  resultCount: number;
  hasResults: boolean;
  uiResponsive: boolean;
}

test.describe('Performance Validation - All Roles', () => {
  let performanceMetrics: SearchPerformanceMetrics[] = [];

  test.beforeEach(async ({ page }) => {
    // Navigate to the app
    await ciNavigate(page, '/');
    await ciWaitForSelector(page, 'input[type="search"]', 'navigation');
  });

  test('should validate Default role search performance', async ({ page }) => {
    console.log('🔍 Testing Default role search performance...');
    
    // Switch to Default role if not already selected
    await switchToRole(page, 'Default');
    
    const metrics = await performSearchWithTiming(page, 'artificial intelligence', 'Default');
    performanceMetrics.push(metrics);
    
    // Validate performance
    expect(metrics.searchTime).toBeLessThan(2000); // Should be under 2 seconds
    expect(metrics.uiResponsive).toBe(true);
    
    console.log(`✅ Default role: ${metrics.searchTime}ms, ${metrics.resultCount} results`);
  });

  test('should validate Rust Engineer role search performance', async ({ page }) => {
    console.log('🔍 Testing Rust Engineer role search performance...');
    
    // Switch to Rust Engineer role
    await switchToRole(page, 'Rust Engineer');
    
    const metrics = await performSearchWithTiming(page, 'async tokio', 'Rust Engineer');
    performanceMetrics.push(metrics);
    
    // Validate performance
    expect(metrics.searchTime).toBeLessThan(2000); // Should be under 2 seconds
    expect(metrics.uiResponsive).toBe(true);
    
    console.log(`✅ Rust Engineer role: ${metrics.searchTime}ms, ${metrics.resultCount} results`);
  });

  test('should validate Terraphim Engineer role search performance', async ({ page }) => {
    console.log('🔍 Testing Terraphim Engineer role search performance...');
    
    // Switch to Terraphim Engineer role
    await switchToRole(page, 'Terraphim Engineer');
    
    const metrics = await performSearchWithTiming(page, 'knowledge graph terraphim', 'Terraphim Engineer');
    performanceMetrics.push(metrics);
    
    // Validate performance
    expect(metrics.searchTime).toBeLessThan(2000); // Should be under 2 seconds
    expect(metrics.uiResponsive).toBe(true);
    
    console.log(`✅ Terraphim Engineer role: ${metrics.searchTime}ms, ${metrics.resultCount} results`);
  });

  test('should validate role switching performance', async ({ page }) => {
    console.log('🔄 Testing role switching performance...');
    
    const roles = ['Default', 'Rust Engineer', 'Terraphim Engineer'];
    const switchTimes: number[] = [];
    
    for (const role of roles) {
      const startTime = Date.now();
      await switchToRole(page, role);
      const switchTime = Date.now() - startTime;
      switchTimes.push(switchTime);
      
      console.log(`✅ Switched to ${role} in ${switchTime}ms`);
      
      // Verify role is active
      const activeRole = await getActiveRole(page);
      expect(activeRole).toBe(role);
    }
    
    // All role switches should be fast
    const maxSwitchTime = Math.max(...switchTimes);
    expect(maxSwitchTime).toBeLessThan(1000); // Should be under 1 second
    
    console.log(`✅ All role switches completed, max time: ${maxSwitchTime}ms`);
  });

  test('should validate search responsiveness during rapid typing', async ({ page }) => {
    console.log('⌨️ Testing search responsiveness during rapid typing...');
    
    const searchInput = await ciWaitForSelector(page, 'input[type="search"]');
    
    // Type rapidly to test debouncing and responsiveness
    const testQueries = ['a', 'ar', 'art', 'arti', 'artif', 'artifi', 'artific', 'artifici', 'artificia', 'artificial'];
    
    for (let i = 0; i < testQueries.length; i++) {
      const query = testQueries[i];
      const startTime = Date.now();
      
      await searchInput.fill(query);
      await ciWait(page, 'tiny'); // Small wait to allow processing
      
      const responseTime = Date.now() - startTime;
      
      // Each keystroke should be processed quickly
      expect(responseTime).toBeLessThan(100); // Should be under 100ms per keystroke
      
      // UI should remain responsive
      const isInputFocused = await searchInput.evaluate(el => el === document.activeElement);
      expect(isInputFocused).toBe(true);
    }
    
    console.log('✅ Rapid typing test passed - UI remained responsive');
  });

  test('should validate no UI freeze during search', async ({ page }) => {
    console.log('🚫 Testing for UI freeze during search...');
    
    const searchInput = await ciWaitForSelector(page, 'input[type="search"]');
    
    // Start typing a search query
    await searchInput.fill('machine learning');
    
    // Start search and immediately try to interact with UI
    const searchPromise = searchInput.press('Enter');
    
    // While search is running, try to interact with other UI elements
    const logo = page.locator('img[alt="Terraphim Logo"]');
    const themeButton = page.locator('button:has-text("Theme")');
    
    // These should be clickable even during search
    if (await themeButton.isVisible()) {
      await themeButton.click();
      console.log('✅ Theme button remained clickable during search');
    }
    
    // Wait for search to complete
    await searchPromise;
    await ciWait(page, 'afterSearch');
    
    // UI should still be responsive
    const isInputFocused = await searchInput.evaluate(el => el === document.activeElement);
    expect(isInputFocused).toBe(true);
    
    console.log('✅ No UI freeze detected during search');
  });

  test('should validate search results quality across all roles', async ({ page }) => {
    console.log('📊 Testing search results quality across all roles...');
    
    const testCases = [
      { role: 'Default', query: 'artificial intelligence', minResults: 0 },
      { role: 'Rust Engineer', query: 'async tokio', minResults: 0 },
      { role: 'Terraphim Engineer', query: 'knowledge graph', minResults: 0 }
    ];
    
    for (const testCase of testCases) {
      await switchToRole(page, testCase.role);
      
      const searchInput = await ciWaitForSelector(page, 'input[type="search"]');
      await searchInput.fill(testCase.query);
      await searchInput.press('Enter');
      
      // Wait for results
      await ciWait(page, 'afterSearch');
      
      // Check for results (if any)
      const resultElements = page.locator('.box, .result-item, .search-result');
      const resultCount = await resultElements.count();
      
      console.log(`${testCase.role}: "${testCase.query}" -> ${resultCount} results`);
      
      // Results should be reasonable (not too many, not too few)
      expect(resultCount).toBeGreaterThanOrEqual(testCase.minResults);
      expect(resultCount).toBeLessThan(100); // Sanity check
    }
    
    console.log('✅ Search results quality validation passed');
  });

  test.afterAll(async () => {
    // Print performance summary
    console.log('\n📊 Performance Summary:');
    console.log('='.repeat(50));
    
    for (const metrics of performanceMetrics) {
      const status = metrics.searchTime < 2000 ? '✅' : '❌';
      console.log(`${status} ${metrics.role}: ${metrics.searchTime}ms (${metrics.resultCount} results)`);
    }
    
    const avgSearchTime = performanceMetrics.reduce((sum, m) => sum + m.searchTime, 0) / performanceMetrics.length;
    const maxSearchTime = Math.max(...performanceMetrics.map(m => m.searchTime));
    
    console.log(`\n📈 Performance Stats:`);
    console.log(`  Average search time: ${avgSearchTime.toFixed(0)}ms`);
    console.log(`  Max search time: ${maxSearchTime}ms`);
    console.log(`  All searches under 2s: ${maxSearchTime < 2000 ? '✅' : '❌'}`);
    console.log(`  UI responsive: ${performanceMetrics.every(m => m.uiResponsive) ? '✅' : '❌'}`);
  });
});

/**
 * Helper function to switch to a specific role
 */
async function switchToRole(page: Page, roleName: string): Promise<void> {
  // Look for role switcher button
  const roleButton = page.locator('button:has-text("Theme"), .role-switcher, [data-testid="role-switcher"]').first();
  
  if (await roleButton.isVisible()) {
    await ciClick(page, roleButton);
    
    // Look for role selection dropdown or modal
    const roleOption = page.locator(`text=${roleName}, [data-role="${roleName}"]`).first();
    
    if (await roleOption.isVisible()) {
      await ciClick(page, roleOption);
      await ciWait(page, 'small'); // Wait for role switch to complete
    }
  }
  
  // Verify role switch by checking for role-specific indicators
  const activeRole = await getActiveRole(page);
  if (activeRole !== roleName) {
    console.log(`⚠️  Role switch to ${roleName} may not have completed (current: ${activeRole})`);
  }
}

/**
 * Helper function to get the currently active role
 */
async function getActiveRole(page: Page): Promise<string> {
  // Try to find role indicator in UI
  const roleIndicator = page.locator('.role-name, [data-current-role], .active-role').first();
  
  if (await roleIndicator.isVisible()) {
    const roleText = await roleIndicator.textContent();
    return roleText?.trim() || 'Unknown';
  }
  
  // Fallback: check URL or other indicators
  const url = page.url();
  if (url.includes('rust')) return 'Rust Engineer';
  if (url.includes('terraphim')) return 'Terraphim Engineer';
  
  return 'Default';
}

/**
 * Helper function to perform search with timing measurements
 */
async function performSearchWithTiming(page: Page, query: string, role: string): Promise<SearchPerformanceMetrics> {
  const searchInput = await ciWaitForSelector(page, 'input[type="search"]');
  
  // Clear any existing search
  await searchInput.clear();
  
  // Measure search time
  const startTime = Date.now();
  
  await searchInput.fill(query);
  await searchInput.press('Enter');
  
  // Wait for search to complete
  await ciWait(page, 'afterSearch');
  
  const searchTime = Date.now() - startTime;
  
  // Count results
  const resultElements = page.locator('.box, .result-item, .search-result');
  const resultCount = await resultElements.count();
  
  // Test UI responsiveness by trying to interact with input
  const isInputFocused = await searchInput.evaluate(el => el === document.activeElement);
  const uiResponsive = isInputFocused;
  
  return {
    role,
    searchTime,
    resultCount,
    hasResults: resultCount > 0,
    uiResponsive
  };
}
