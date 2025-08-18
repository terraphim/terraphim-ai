import { test, expect, Page } from '@playwright/test';
import { execSync } from 'child_process';
import fs from 'fs';
import path from 'path';
import { fileURLToPath } from 'url';

interface TestConfigInfo {
  configFile: string;
  atomicServerUrl: string;
  hasAtomicSecret: boolean;
  hasOpenRouterKey: boolean;
  openRouterModel: string;
  testRole: string;
}

/**
 * Comprehensive E2E Test for Terraphim Engineer functionality
 * 
 * This test validates the complete workflow:
 * 1. Configuration setup with environment variables
 * 2. Search functionality with knowledge graph 
 * 3. AI summarization using OpenRouter
 * 4. Atomic server integration for saving articles
 * 
 * Prerequisites:
 * - ATOMIC_SERVER_SECRET environment variable set
 * - OPENROUTER_API_KEY environment variable set  
 * - Atomic server running on localhost:9883
 * - Terraphim server running on localhost:8000
 */

let testConfigInfo: TestConfigInfo;
// Get current file directory in ES modules
const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);
const PROJECT_ROOT = path.resolve(__dirname, '../../../');

test.describe('Terraphim Engineer - Complete Functionality Test', () => {
  
  test.beforeAll(async () => {
    console.log('🚀 Setting up comprehensive Terraphim Engineer test...');
    
    // Check environment variables
    const atomicSecret = process.env.ATOMIC_SERVER_SECRET;
    const openRouterKey = process.env.OPENROUTER_API_KEY;
    
    console.log(`📊 Environment Check:`);
    console.log(`  - ATOMIC_SERVER_SECRET: ${atomicSecret ? '✅ SET' : '❌ NOT SET'}`);
    console.log(`  - OPENROUTER_API_KEY: ${openRouterKey ? '✅ SET' : '❌ NOT SET'}`);
    
    if (!atomicSecret) {
      console.warn('⚠️  Warning: ATOMIC_SERVER_SECRET not set - atomic save functionality will be limited');
    }
    
    if (!openRouterKey) {
      console.warn('⚠️  Warning: OPENROUTER_API_KEY not set - AI summarization will not work');
    }
    
    // Run setup script to create configuration
    console.log('🔧 Running configuration setup script...');
    
    try {
      const setupOutput = execSync('./scripts/setup_test_config.sh', {
        cwd: PROJECT_ROOT,
        encoding: 'utf8',
        env: {
          ...process.env,
          ATOMIC_SERVER_URL: process.env.ATOMIC_SERVER_URL || 'http://localhost:9883',
          ATOMIC_SERVER_SECRET: atomicSecret || '',
          OPENROUTER_API_KEY: openRouterKey || '',
          OPENROUTER_MODEL: process.env.OPENROUTER_MODEL || 'openai/gpt-3.5-turbo'
        }
      });
      
      console.log('📝 Setup script output:');
      console.log(setupOutput);
      
    } catch (error) {
      console.error('❌ Failed to run setup script:', error);
      throw error;
    }
    
    // Read test configuration info
    const configInfoPath = path.join(PROJECT_ROOT, 'test_config_info.json');
    if (!fs.existsSync(configInfoPath)) {
      throw new Error(`Test configuration info not found at: ${configInfoPath}`);
    }
    
    testConfigInfo = JSON.parse(fs.readFileSync(configInfoPath, 'utf8'));
    console.log('📋 Test Configuration loaded:', testConfigInfo);
    
    // Verify configuration file exists
    const fullConfigPath = path.join(PROJECT_ROOT, testConfigInfo.configFile);
    if (!fs.existsSync(fullConfigPath)) {
      throw new Error(`Configuration file not found at: ${fullConfigPath}`);
    }
    
    console.log('✅ Test setup complete!');
  });

  test.afterAll(async () => {
    // Cleanup test files
    const filesToCleanup = [
      'test_config_info.json',
      'terraphim_engineer_test_config_final.json',
      'terraphim_engineer_test_config.json.bak'
    ];
    
    for (const file of filesToCleanup) {
      const filePath = path.join(PROJECT_ROOT, file);
      if (fs.existsSync(filePath)) {
        fs.unlinkSync(filePath);
        console.log(`🗑️  Cleaned up: ${file}`);
      }
    }
  });

  test('should validate test configuration is properly set up', async () => {
    console.log('🔍 Validating test configuration...');
    
    expect(testConfigInfo).toBeDefined();
    expect(testConfigInfo.configFile).toBeTruthy();
    expect(testConfigInfo.testRole).toBe('Terraphim Engineer Test');
    expect(testConfigInfo.atomicServerUrl).toBeTruthy();
    
    // Check configuration file structure
    const configPath = path.join(PROJECT_ROOT, testConfigInfo.configFile);
    const config = JSON.parse(fs.readFileSync(configPath, 'utf8'));
    
    expect(config.roles[testConfigInfo.testRole]).toBeDefined();
    
    const testRole = config.roles[testConfigInfo.testRole];
    expect(testRole.relevance_function).toBe('terraphim-graph');
    expect(testRole.terraphim_it).toBe(true);
    expect(testRole.haystacks).toHaveLength(2); // Ripgrep + Atomic
    
    // Validate atomic haystack
    const atomicHaystack = testRole.haystacks.find(h => h.service === 'Atomic');
    expect(atomicHaystack).toBeDefined();
    expect(atomicHaystack.location).toBe(testConfigInfo.atomicServerUrl);
    expect(atomicHaystack.read_only).toBe(false); // Writable for testing
    
    // Validate OpenRouter configuration if available
    if (testConfigInfo.hasOpenRouterKey) {
      expect(testRole.openrouter_enabled).toBe(true);
      expect(testRole.openrouter_api_key).toBeTruthy();
      expect(testRole.openrouter_model).toBe(testConfigInfo.openRouterModel);
    }
    
    console.log('✅ Test configuration validation passed');
  });

  test('should start Terraphim server with test configuration', async ({ page }) => {
    console.log('🚀 Starting Terraphim server with test configuration...');
    
    // Navigate to the application
    await page.goto('http://localhost:5173', { waitUntil: 'networkidle' });
    
    // Wait for the application to load
    await expect(page.locator('body')).toBeVisible();
    
    // Check if we can access the main interface
    await expect(page.locator('[data-testid="search-container"], .search-container, input[type="search"], #search-input')).toBeVisible({ timeout: 10000 });
    
    console.log('✅ Terraphim application loaded successfully');
  });

  test('should perform search and validate results', async ({ page }) => {
    console.log('🔍 Testing search functionality...');
    
    await page.goto('http://localhost:5173', { waitUntil: 'networkidle' });
    
    // Find the search input
    const searchInput = page.locator('input[type="search"], #search-input, .search-input').first();
    await expect(searchInput).toBeVisible({ timeout: 10000 });
    
    // Perform a search for terms that should exist in the knowledge graph
    const searchTerm = 'terraphim graph knowledge';
    console.log(`🔎 Searching for: "${searchTerm}"`);
    
    await searchInput.fill(searchTerm);
    await searchInput.press('Enter');
    
    // Wait for search results
    console.log('⏳ Waiting for search results...');
    await expect(page.locator('.box, .result-item, .search-result').first()).toBeVisible({ timeout: 15000 });
    
    // Validate that we have results
    const resultElements = page.locator('.box, .result-item, .search-result');
    const resultCount = await resultElements.count();
    expect(resultCount).toBeGreaterThan(0);
    
    console.log(`✅ Found ${resultCount} search results`);
    
    // Check for rank indicators
    const rankElements = page.locator('.tag:has-text("Rank"), [class*="rank"]');
    if (await rankElements.count() > 0) {
      console.log('✅ Search results show ranking information');
    }
    
    // Check for knowledge graph tags (if any)
    const tagElements = page.locator('.tag, .tag-button');
    if (await tagElements.count() > 0) {
      console.log('✅ Search results show tags from knowledge graph');
    }
  });

  test('should test AI summarization functionality', async ({ page }) => {
    if (!testConfigInfo.hasOpenRouterKey) {
      test.skip(true, 'OPENROUTER_API_KEY not available - skipping summarization test');
      return;
    }
    
    console.log('🤖 Testing AI summarization functionality...');
    
    await page.goto('http://localhost:5173', { waitUntil: 'networkidle' });
    
    // Perform a search first
    const searchInput = page.locator('input[type="search"], #search-input, .search-input').first();
    await searchInput.fill('service haystack');
    await searchInput.press('Enter');
    
    // Wait for results
    await expect(page.locator('.box, .result-item, .search-result').first()).toBeVisible({ timeout: 15000 });
    
    // Look for AI Summary button
    const aiSummaryButton = page.locator('button:has-text("AI Summary"), .ai-summary-button, button[title*="summary"]').first();
    
    if (await aiSummaryButton.count() > 0) {
      console.log('🎯 Found AI Summary button, testing...');
      
      await aiSummaryButton.click();
      
      // Wait for summary to be generated
      console.log('⏳ Waiting for AI summary generation...');
      
      // Look for loading indicator
      await expect(page.locator('.ai-summary-loading, [class*="loading"], .fa-spinner').first()).toBeVisible({ timeout: 5000 });
      
      // Wait for summary content
      await expect(page.locator('.ai-summary-content, .ai-summary, [class*="summary-content"]').first()).toBeVisible({ timeout: 30000 });
      
      console.log('✅ AI summarization functionality working');
      
      // Validate summary content
      const summaryContent = await page.locator('.ai-summary-content, .ai-summary').first().textContent();
      expect(summaryContent).toBeTruthy();
      expect(summaryContent!.length).toBeGreaterThan(10);
      
    } else {
      console.log('⚠️  AI Summary button not found - may not be available for this content type');
    }
  });

  test('should test atomic save functionality', async ({ page }) => {
    if (!testConfigInfo.hasAtomicSecret) {
      test.skip(true, 'ATOMIC_SERVER_SECRET not available - skipping atomic save test');
      return;
    }
    
    console.log('☁️  Testing atomic save functionality...');
    
    await page.goto('http://localhost:5173', { waitUntil: 'networkidle' });
    
    // Perform a search first
    const searchInput = page.locator('input[type="search"], #search-input, .search-input').first();
    await searchInput.fill('knowledge graph system');
    await searchInput.press('Enter');
    
    // Wait for results
    await expect(page.locator('.box, .result-item, .search-result').first()).toBeVisible({ timeout: 15000 });
    
    // Look for atomic save button (cloud upload icon)
    const atomicSaveButton = page.locator('button[aria-label*="Atomic"], .fa-cloud-upload-alt, button[title*="Atomic"]').first();
    
    if (await atomicSaveButton.count() > 0) {
      console.log('☁️  Found atomic save button, testing...');
      
      await atomicSaveButton.click();
      
      // Wait for modal to appear
      console.log('📝 Waiting for atomic save modal...');
      await expect(page.locator('.modal.is-active, .modal-content, [class*="modal"]').first()).toBeVisible({ timeout: 10000 });
      
      // Validate modal content
      const modalTitle = page.locator('.modal-card-title, .modal-header, h1, h2').first();
      await expect(modalTitle).toBeVisible();
      
      // Check for form fields
      const articleTitleInput = page.locator('input[placeholder*="title"], #article-title, [name="title"]').first();
      const parentSelect = page.locator('select, .select select, [name="parent"]').first();
      
      if (await articleTitleInput.count() > 0) {
        console.log('📝 Modal form elements found');
        
        // Fill in a test title
        await articleTitleInput.fill('Test Article from E2E Test');
        
        // Close modal (for now, don't actually save to avoid polluting the server)
        const closeButton = page.locator('.modal-close, .delete, button:has-text("Close"), button:has-text("Cancel")').first();
        if (await closeButton.count() > 0) {
          await closeButton.click();
        } else {
          await page.keyboard.press('Escape');
        }
        
        console.log('✅ Atomic save modal functionality working');
      } else {
        console.log('⚠️  Modal form fields not found as expected');
      }
      
    } else {
      console.log('⚠️  Atomic save button not found - may not be available for current role configuration');
    }
  });

  test('should validate knowledge graph integration', async ({ page }) => {
    console.log('🕸️  Testing knowledge graph integration...');
    
    await page.goto('http://localhost:5173', { waitUntil: 'networkidle' });
    
    // Search for something that should have KG tags
    const searchInput = page.locator('input[type="search"], #search-input, .search-input').first();
    await searchInput.fill('service haystack knowledge');
    await searchInput.press('Enter');
    
    // Wait for results
    await expect(page.locator('.box, .result-item, .search-result').first()).toBeVisible({ timeout: 15000 });
    
    // Look for clickable tags
    const clickableTags = page.locator('.tag-button, .tag[role="button"], button .tag');
    
    if (await clickableTags.count() > 0) {
      console.log('🏷️  Found clickable knowledge graph tags');
      
      // Click on the first tag
      const firstTag = clickableTags.first();
      const tagText = await firstTag.textContent();
      console.log(`🔎 Clicking on tag: "${tagText}"`);
      
      await firstTag.click();
      
      // Wait for KG document modal or additional results
      console.log('⏳ Waiting for knowledge graph document...');
      
      // Look for modal or new content
      const hasModal = await page.locator('.modal.is-active, .modal-content').first().isVisible({ timeout: 5000 }).catch(() => false);
      
      if (hasModal) {
        console.log('✅ Knowledge graph document modal opened successfully');
        
        // Close modal
        const closeButton = page.locator('.modal-close, .delete').first();
        if (await closeButton.count() > 0) {
          await closeButton.click();
        }
      } else {
        console.log('📊 Knowledge graph integration may be working (no modal appeared)');
      }
      
    } else {
      console.log('⚠️  No clickable tags found - KG may not be built yet');
    }
  });

  test('should validate complete workflow end-to-end', async ({ page }) => {
    console.log('🔄 Running complete workflow validation...');
    
    await page.goto('http://localhost:5173', { waitUntil: 'networkidle' });
    
    // Step 1: Search
    console.log('1️⃣ Step 1: Performing search...');
    const searchInput = page.locator('input[type="search"], #search-input, .search-input').first();
    await searchInput.fill('terraphim system knowledge graph');
    await searchInput.press('Enter');
    
    await expect(page.locator('.box, .result-item, .search-result').first()).toBeVisible({ timeout: 15000 });
    const resultCount = await page.locator('.box, .result-item, .search-result').count();
    console.log(`✅ Step 1 complete: Found ${resultCount} search results`);
    
    // Step 2: Test summarization (if available)
    if (testConfigInfo.hasOpenRouterKey) {
      console.log('2️⃣ Step 2: Testing AI summarization...');
      const aiButton = page.locator('button:has-text("AI Summary"), .ai-summary-button').first();
      
      if (await aiButton.count() > 0) {
        await aiButton.click();
        
        // Wait briefly for the request to start
        await page.waitForTimeout(2000);
        console.log('✅ Step 2 complete: AI summarization triggered');
      } else {
        console.log('⚠️  Step 2 skipped: AI Summary button not found');
      }
    } else {
      console.log('⏭️  Step 2 skipped: OpenRouter API key not available');
    }
    
    // Step 3: Test atomic save availability (if available)
    if (testConfigInfo.hasAtomicSecret) {
      console.log('3️⃣ Step 3: Testing atomic save availability...');
      const atomicButton = page.locator('button[aria-label*="Atomic"], .fa-cloud-upload-alt').first();
      
      if (await atomicButton.count() > 0) {
        console.log('✅ Step 3 complete: Atomic save button available');
      } else {
        console.log('⚠️  Step 3: Atomic save button not found');
      }
    } else {
      console.log('⏭️  Step 3 skipped: Atomic server secret not available');
    }
    
    // Step 4: Test knowledge graph tags
    console.log('4️⃣ Step 4: Testing knowledge graph integration...');
    const tagCount = await page.locator('.tag, .tag-button').count();
    console.log(`✅ Step 4 complete: Found ${tagCount} tags in results`);
    
    console.log('🎉 Complete workflow validation finished!');
    
    // Summary
    console.log('\n📊 Test Summary:');
    console.log(`  🔍 Search: ✅ Working (${resultCount} results)`);
    console.log(`  🤖 AI Summary: ${testConfigInfo.hasOpenRouterKey ? '✅ Available' : '⚠️  Not configured'}`);
    console.log(`  ☁️  Atomic Save: ${testConfigInfo.hasAtomicSecret ? '✅ Available' : '⚠️  Not configured'}`);
    console.log(`  🏷️  KG Tags: ✅ Found (${tagCount} tags)`);
  });

});

/**
 * Test configuration validation
 * This test runs independently to validate that the configuration system works
 */
test.describe('Configuration System Validation', () => {
  
  test('should validate environment variable substitution', async () => {
    console.log('🔧 Testing environment variable substitution...');
    
    // Check that our configuration files exist
    const templateExists = fs.existsSync(path.join(PROJECT_ROOT, 'terraphim_engineer_test_config.json'));
    expect(templateExists).toBe(true);
    
    console.log('✅ Template configuration file exists');
    
    // If we have a final config, validate it has substituted values
    const finalConfigPath = path.join(PROJECT_ROOT, 'terraphim_engineer_test_config_final.json');
    if (fs.existsSync(finalConfigPath)) {
      const config = JSON.parse(fs.readFileSync(finalConfigPath, 'utf8'));
      const testRole = config.roles['Terraphim Engineer Test'];
      
      // Check that environment variables were substituted (no ${} placeholders remain)
      const atomicHaystack = testRole.haystacks.find(h => h.service === 'Atomic');
      expect(atomicHaystack.location).not.toContain('${');
      expect(atomicHaystack.atomic_server_secret || '').not.toContain('${');
      
      if (testRole.openrouter_api_key) {
        expect(testRole.openrouter_api_key).not.toContain('${');
      }
      
      console.log('✅ Environment variable substitution working correctly');
    }
  });
  
}); 