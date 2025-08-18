/**
 * CI-friendly test utilities for Playwright tests
 * Use these utilities to make tests more stable in CI environments
 */

/**
 * Get CI-friendly timeout values
 */
export function getTimeouts() {
  const isCI = Boolean(process?.env?.CI);
  
  return {
    // Base timeouts
    short: isCI ? 10000 : 5000,        // 10s/5s for quick actions
    medium: isCI ? 30000 : 15000,      // 30s/15s for normal actions
    long: isCI ? 60000 : 30000,        // 60s/30s for slow actions
    navigation: isCI ? 60000 : 30000,  // 60s/30s for page navigation
    
    // Test-specific timeouts
    search: isCI ? 20000 : 10000,      // Search operation timeout
    hover: isCI ? 5000 : 2000,         // Hover action timeout
    animation: isCI ? 2000 : 1000,     // Animation completion timeout
    serverStart: isCI ? 180000 : 120000, // Server startup timeout
  };
}

/**
 * Get CI-friendly wait times for explicit waits
 */
export function getWaitTimes() {
  const isCI = Boolean(process?.env?.CI);
  
  return {
    // Small delays
    tiny: isCI ? 500 : 200,           // 500ms/200ms
    small: isCI ? 1000 : 500,         // 1s/500ms
    medium: isCI ? 2000 : 1000,       // 2s/1s
    large: isCI ? 5000 : 2000,        // 5s/2s
    
    // Specific action delays
    afterClick: isCI ? 1000 : 500,    // After clicking elements
    afterHover: isCI ? 1000 : 300,    // After hovering elements
    afterSearch: isCI ? 3000 : 1500,  // After search operations
    afterNavigation: isCI ? 2000 : 1000, // After page navigation
  };
}

/**
 * Check if running in CI environment
 */
export function isCI(): boolean {
  return Boolean(process?.env?.CI);
}

/**
 * CI-friendly page.waitForTimeout wrapper
 * Automatically adjusts timeout based on CI environment
 */
export async function ciWait(page: any, type: keyof ReturnType<typeof getWaitTimes>) {
  const waitTimes = getWaitTimes();
  const timeout = waitTimes[type];
  await page.waitForTimeout(timeout);
}

/**
 * CI-friendly selector waiting with automatic timeout adjustment
 */
export async function ciWaitForSelector(
  page: any, 
  selector: string, 
  timeoutType: keyof ReturnType<typeof getTimeouts> = 'medium'
) {
  const timeouts = getTimeouts();
  const timeout = timeouts[timeoutType];
  
  return await page.waitForSelector(selector, { 
    timeout,
    // More lenient in CI
    state: isCI() ? 'attached' : 'visible'
  });
}

/**
 * CI-friendly hover action with proper delays
 */
export async function ciHover(page: any, selector: string) {
  const element = await ciWaitForSelector(page, selector);
  await element.hover();
  
  // Add delay for hover effects to complete
  const waitTimes = getWaitTimes();
  await page.waitForTimeout(waitTimes.afterHover);
  
  return element;
}

/**
 * CI-friendly click action with proper delays
 */
export async function ciClick(page: any, selector: string) {
  const element = await ciWaitForSelector(page, selector);
  await element.click();
  
  // Add delay for click effects to complete
  const waitTimes = getWaitTimes();
  await page.waitForTimeout(waitTimes.afterClick);
  
  return element;
}

/**
 * CI-friendly search operation
 */
export async function ciSearch(page: any, searchSelector: string, query: string) {
  const searchInput = await ciWaitForSelector(page, searchSelector);
  
  // Clear existing content first
  await searchInput.clear();
  await ciWait(page, 'tiny');
  
  // Fill the search query
  await searchInput.fill(query);
  await ciWait(page, 'small');
  
  // Press Enter to search
  await searchInput.press('Enter');
  
  // Wait for search to complete
  await ciWait(page, 'afterSearch');
  
  return searchInput;
}

/**
 * CI-friendly navigation with proper waiting
 */
export async function ciNavigate(page: any, url: string) {
  await page.goto(url, {
    timeout: getTimeouts().navigation,
    waitUntil: isCI() ? 'domcontentloaded' : 'load'
  });
  
  // Additional wait for page to stabilize
  await ciWait(page, 'afterNavigation');
}

/**
 * CI-friendly expectation helper with retry logic
 */
export async function ciExpect(page: any, assertion: () => Promise<void>, maxRetries = 3) {
  const waitTimes = getWaitTimes();
  
  for (let i = 0; i < maxRetries; i++) {
    try {
      await assertion();
      return; // Success
    } catch (error) {
      if (i === maxRetries - 1) {
        throw error; // Last attempt failed
      }
      
      // Wait before retry
      await page.waitForTimeout(waitTimes.medium);
    }
  }
}

/**
 * CI configuration for test setup
 */
export function getCIConfig() {
  return {
    headless: isCI(),
    slowMo: isCI() ? 100 : 0,
    timeout: getTimeouts().long,
    actionTimeout: getTimeouts().medium,
    navigationTimeout: getTimeouts().navigation,
    
    // Browser launch options for CI
    launchOptions: isCI() ? {
      args: [
        '--disable-animations',
        '--disable-background-timer-throttling',
        '--disable-backgrounding-occluded-windows',
        '--disable-renderer-backgrounding',
        '--no-sandbox',
        '--disable-setuid-sandbox',
        '--disable-dev-shm-usage',
      ],
    } : {},
  };
} 