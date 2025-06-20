import { chromium, FullConfig } from '@playwright/test';

async function globalSetup(config: FullConfig) {
  console.log('🚀 Starting global test setup...');
  
  // In CI environment, we might need to build the app first
  if (process.env.CI) {
    console.log('Building Tauri app for CI...');
    // Add any CI-specific setup here
  }
  
  // Setup test data directory
  const fs = require('fs');
  const path = require('path');
  
  const testDataDir = path.join(__dirname, '../test-data');
  if (!fs.existsSync(testDataDir)) {
    fs.mkdirSync(testDataDir, { recursive: true });
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
        path: path.join(testDataDir, 'test-documents'),
        role: 'test_role'
      }
    ],
    global_shortcut: 'Ctrl+Alt+T',
    theme: 'default',
    server_url: 'http://localhost:3000'
  };
  
  fs.writeFileSync(
    path.join(testDataDir, 'test-config.json'),
    JSON.stringify(testConfig, null, 2)
  );
  
  // Create test documents
  const docsDir = path.join(testDataDir, 'test-documents');
  if (!fs.existsSync(docsDir)) {
    fs.mkdirSync(docsDir, { recursive: true });
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
    fs.writeFileSync(path.join(docsDir, doc.name), doc.content);
  });
  
  console.log('✅ Global setup completed');
}

export default globalSetup; 