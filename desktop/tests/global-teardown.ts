import { FullConfig } from '@playwright/test';

async function globalTeardown(config: FullConfig) {
  console.log('🧹 Starting global test teardown...');
  
  // Clean up test data if needed
  const fs = require('fs');
  const path = require('path');
  
  const testDataDir = path.join(__dirname, '../test-data');
  
  // Only clean up in CI to avoid removing useful test data during development
  if (process.env.CI && fs.existsSync(testDataDir)) {
    try {
      fs.rmSync(testDataDir, { recursive: true, force: true });
      console.log('Cleaned up test data directory');
    } catch (error) {
      console.warn('Failed to clean up test data:', error);
    }
  }
  
  console.log('✅ Global teardown completed');
}

export default globalTeardown; 