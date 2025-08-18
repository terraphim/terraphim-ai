import { chromium, FullConfig } from '@playwright/test';
import { writeFileSync, existsSync, mkdirSync, readFileSync } from 'fs';
import { join } from 'path';
import { fileURLToPath } from 'url';

const __filename = fileURLToPath(import.meta.url);
const __dirname = join(__filename, '..');

// Load .env file from project root  
function loadEnvFile() {
  const envPath = join(__dirname, '../../.env');
  if (existsSync(envPath)) {
    const envContent = readFileSync(envPath, 'utf8');
    envContent.split('\n').forEach(line => {
      const [key, value] = line.split('=');
      if (key && value) {
        process.env[key.trim()] = value.trim().replace(/^"(.*)"$/, '$1');
      }
    });
    console.log('✅ Loaded .env file for tests');
  } else {
    console.log('⚠️ No .env file found at', envPath);
  }
}

async function globalSetup(config: FullConfig) {
  console.log('🚀 Starting global test setup...');
  
  // Load environment variables from .env file
  loadEnvFile();
  
  // In CI environment, we might need to build the app first
  if (process.env.CI) {
    console.log('Building Tauri app for CI...');
    // Add any CI-specific setup here
  }
  
  // Setup test data directory
  const testDataDir = join(__dirname, '../test-data');
  if (!existsSync(testDataDir)) {
    mkdirSync(testDataDir, { recursive: true });
    console.log('Created test data directory');
  }
  
  // Create test fixtures
  const testConfig = {
    knowledge_graphs: [
      {
        id: 'test-kg',
        name: 'Test Knowledge Graph',
        description: 'Test data for E2E tests',
        publish: false,
        path: join(testDataDir, 'test-documents'),
        role: 'test_role'
      }
    ],
    global_shortcut: 'Ctrl+Alt+T',
    theme: 'default',
    server_url: 'http://localhost:3000'
  };
  
  writeFileSync(
    join(testDataDir, 'test-config.json'),
    JSON.stringify(testConfig, null, 2)
  );
  
  // Create test documents
  const docsDir = join(testDataDir, 'test-documents');
  if (!existsSync(docsDir)) {
    mkdirSync(docsDir, { recursive: true });
  }
  
  const testDocuments = [
    {
      name: 'test-doc-1.md',
      content: `# Test Document 1
      
This is a test document for artificial intelligence testing.
It contains information about machine learning and neural networks.
      `
    },
    {
      name: 'test-doc-2.md',
      content: `# Test Document 2
      
This document covers advanced topics in data science.
Including topics like deep learning and computer vision.
      `
    }
  ];
  
  testDocuments.forEach(doc => {
    writeFileSync(join(docsDir, doc.name), doc.content);
  });
  
  console.log('✅ Global setup completed');
}

export default globalSetup; 