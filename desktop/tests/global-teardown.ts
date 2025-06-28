import { FullConfig } from '@playwright/test';
import { rmSync, existsSync } from 'fs';
import { join } from 'path';
import { fileURLToPath } from 'url';

const __filename = fileURLToPath(import.meta.url);
const __dirname = join(__filename, '..');

async function globalTeardown(config: FullConfig) {
  console.log('🧹 Starting global test teardown...');
  
  // Clean up test data if needed
  const testDataDir = join(__dirname, '../test-data');
  
  // Only clean up in CI to avoid removing useful test data during development
  if (process.env.CI && existsSync(testDataDir)) {
    try {
      rmSync(testDataDir, { recursive: true, force: true });
      console.log('Cleaned up test data directory');
    } catch (error) {
      console.warn('Failed to clean up test data:', error);
    }
  }
  
  console.log('✅ Global teardown completed');
}

export default globalTeardown; 