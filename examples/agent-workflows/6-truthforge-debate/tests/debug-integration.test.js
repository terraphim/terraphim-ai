/**
 * Debug Mode Integration Tests
 * Tests for LLM request/response debug logging functionality
 */

const { test, expect, beforeAll, afterAll, describe } = require('@playwright/test');

describe('TruthForge Debug Mode Integration', () => {
  let page, browser;

  beforeAll(async ({browser: b}) => {
    browser = b;
    page = await browser.newPage();

    // Enable console logging
    page.on('console', msg => {
      if (msg.type() === 'error') {
        console.error('Browser error:', msg.text());
      }
    });

    // Navigate to test page
    await page.goto('http://localhost:8081/6-truthforge-debate/test-debug-mode.html');
    await page.waitForLoadState('networkidle');
  });

  afterAll(async () => {
    if (page) await page.close();
  });

  test('should load debug panel component', async () => {
    const panel = await page.$('#debug-panel-container');
    expect(panel).toBeTruthy();

    const panelContent = await page.$('.debug-panel');
    expect(panelContent).toBeTruthy();
  });

  test('should initialize all components without errors', async () => {
    const testResults = await page.$('#test-results');
    expect(testResults).toBeTruthy();

    const checks = await page.$$('#test-results li');
    expect(checks.length).toBeGreaterThan(4);
  });

  test('should display debug panel when visible', async () => {
    const container = await page.$('#debug-panel-container');
    const display = await container.evaluate(el => getComputedStyle(el).display);
    expect(display).toBe('block');
  });

  test('should add debug entries when test button clicked', async () => {
    await page.click('#test-btn');

    // Wait for entries to appear
    await page.waitForSelector('.debug-entry', { timeout: 5000 });

    const entries = await page.$$('.debug-entry');
    expect(entries.length).toBeGreaterThanOrEqual(1);
  });

  test('should show request and response entries', async () => {
    await page.click('#test-btn');
    await page.waitForSelector('.debug-entry', { timeout: 5000 });
    await page.waitForTimeout(1500); // Wait for response

    const requestEntries = await page.$$('.debug-entry.request');
    const responseEntries = await page.$$('.debug-entry.response');

    expect(requestEntries.length).toBeGreaterThanOrEqual(1);
    expect(responseEntries.length).toBeGreaterThanOrEqual(1);
  });

  test('should sanitize sensitive data in prompts', async () => {
    await page.click('#test-btn');
    await page.waitForSelector('.debug-entry', { timeout: 5000 });

    // Expand first entry
    await page.click('.debug-entry-header');

    // Check that email is redacted
    const content = await page.$eval('.debug-entry-body', el => el.textContent);
    expect(content).toContain('[EMAIL]');
    expect(content).not.toContain('user@example.com');
  });

  test('should toggle panel visibility', async () => {
    const toggleBtn = await page.$('#toggle-debug');
    expect(toggleBtn).toBeTruthy();

    await toggleBtn.click();
    let display = await page.$eval('#debug-panel-container', el => getComputedStyle(el).display);
    expect(display).toBe('none');

    await toggleBtn.click();
    display = await page.$eval('#debug-panel-container', el => getComputedStyle(el).display);
    expect(display).toBe('block');
  });

  test('should clear entries when clear button clicked', async () => {
    // Add some entries first
    await page.click('#test-btn');
    await page.waitForSelector('.debug-entry', { timeout: 5000 });

    // Accept confirmation dialog
    page.on('dialog', dialog => dialog.accept());

    // Expand panel and click clear
    if (!await page.$('.debug-content.expanded')) {
      await page.click('.debug-header');
    }
    await page.click('#debug-clear');

    // Wait a bit for clear to process
    await page.waitForTimeout(500);

    // Check entries are gone
    const entries = await page.$$('.debug-entry');
    expect(entries.length).toBe(0);
  });
});

describe('Debug Mode Settings Integration', () => {
  test('should have debug checkbox in settings modal', async () => {
    const { page } = await browser.newPage();
    await page.goto('http://localhost:8081/6-truthforge-debate/');

    // Wait for settings to load
    await page.waitForSelector('#settings-toggle', { timeout: 10000 });

    // Open settings
    await page.click('#settings-toggle');
    await page.waitForSelector('.settings-modal', { timeout: 5000 });

    // Check for debug checkbox
    const debugCheckbox = await page.$('#enable-debug-mode');
    expect(debugCheckbox).toBeTruthy();

    const label = await page.$('label[for="enable-debug-mode"]');
    expect(label).toBeTruthy();

    await page.close();
  });
});
