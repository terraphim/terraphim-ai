/**
 * Global setup for Context Management UI tests
 * 
 * This file handles the initialization required for context management tests,
 * including backend services, test data preparation, and environment validation.
 */

import { chromium, FullConfig } from '@playwright/test';
import { exec } from 'child_process';
import { promisify } from 'util';
import path from 'path';
import fs from 'fs/promises';

const execAsync = promisify(exec);

interface TestEnvironment {
  frontendReady: boolean;
  backendReady: boolean;
  mcpServerReady: boolean;
}

/**
 * Validate required system dependencies for context management tests
 */
async function validateSystemRequirements(): Promise<void> {
  console.log('🔍 Validating system requirements...');

  const requirements = [
    { command: 'node --version', name: 'Node.js' },
    { command: 'cargo --version', name: 'Rust/Cargo' },
    { command: 'git --version', name: 'Git' },
  ];

  for (const req of requirements) {
    try {
      const { stdout } = await execAsync(req.command);
      console.log(`✅ ${req.name}: ${stdout.trim()}`);
    } catch (error) {
      console.error(`❌ ${req.name} not found or not working properly`);
      throw new Error(`Missing required dependency: ${req.name}`);
    }
  }
}

/**
 * Setup test directories and clean previous test artifacts
 */
async function setupTestDirectories(): Promise<void> {
  console.log('📁 Setting up test directories...');

  const directories = [
    'test-results',
    'test-results/context-artifacts',
    'test-results/context-report',
    'test-results/screenshots',
    'test-results/videos',
    'test-results/traces',
  ];

  for (const dir of directories) {
    try {
      await fs.mkdir(dir, { recursive: true });
      console.log(`✅ Created directory: ${dir}`);
    } catch (error) {
      console.warn(`⚠️ Directory ${dir} already exists or couldn't be created`);
    }
  }

  // Clean previous test artifacts
  try {
    await fs.rm('test-results/context-artifacts', { recursive: true, force: true });
    await fs.mkdir('test-results/context-artifacts', { recursive: true });
    console.log('🧹 Cleaned previous test artifacts');
  } catch (error) {
    console.warn('⚠️ Could not clean previous artifacts');
  }
}

/**
 * Build required components for context management tests
 */
async function buildComponents(): Promise<void> {
  console.log('🔨 Building required components...');

  const currentDir = process.cwd();
  
  try {
    // Build the main Terraphim server (backend API)
    console.log('🏗️ Building Terraphim server...');
    process.chdir(path.resolve(__dirname, '../..'));
    
    const { stdout, stderr } = await execAsync('cargo build --bin terraphim_server --release');
    if (stderr && !stderr.includes('warning')) {
      console.error('Backend build errors:', stderr);
    }
    console.log('✅ Backend server built successfully');

    // Build MCP server for context integration
    console.log('🏗️ Building MCP server...');
    process.chdir(path.resolve(__dirname, '../../crates/terraphim_mcp_server'));
    
    const mcpBuild = await execAsync('cargo build --release');
    if (mcpBuild.stderr && !mcpBuild.stderr.includes('warning')) {
      console.error('MCP server build errors:', mcpBuild.stderr);
    }
    console.log('✅ MCP server built successfully');

  } catch (error) {
    console.error('❌ Build failed:', error);
    throw new Error('Component build failed');
  } finally {
    process.chdir(currentDir);
  }
}

/**
 * Start and validate backend services
 */
async function startBackendServices(): Promise<TestEnvironment> {
  console.log('🚀 Starting backend services...');

  const environment: TestEnvironment = {
    frontendReady: false,
    backendReady: false,
    mcpServerReady: false,
  };

  const BACKEND_PORT = process.env.BACKEND_PORT || '8000';
  const MCP_SERVER_PORT = process.env.MCP_SERVER_PORT || '8001';
  const MAX_RETRIES = 30;
  const RETRY_INTERVAL = 2000;

  // Helper function to check if a service is ready
  const checkServiceHealth = async (url: string, serviceName: string): Promise<boolean> => {
    try {
      const response = await fetch(url);
      const isHealthy = response.ok;
      if (isHealthy) {
        console.log(`✅ ${serviceName} is healthy at ${url}`);
      }
      return isHealthy;
    } catch (error) {
      return false;
    }
  };

  // Start backend server (if not already running)
  console.log(`🚀 Starting backend server on port ${BACKEND_PORT}...`);
  try {
    const currentDir = process.cwd();
    process.chdir(path.resolve(__dirname, '../..'));
    
    const backendProcess = exec(`./target/release/terraphim_server --port ${BACKEND_PORT}`, {
      env: { 
        ...process.env, 
        RUST_LOG: 'info',
        TERRAPHIM_TEST_MODE: 'true'
      }
    });

    backendProcess.stdout?.on('data', (data) => {
      if (process.env.DEBUG) console.log(`Backend: ${data}`);
    });

    backendProcess.stderr?.on('data', (data) => {
      if (process.env.DEBUG) console.error(`Backend Error: ${data}`);
    });

    process.chdir(currentDir);
  } catch (error) {
    console.warn('⚠️ Backend server might already be running or failed to start');
  }

  // Start MCP server (if not already running)  
  console.log(`🚀 Starting MCP server on port ${MCP_SERVER_PORT}...`);
  try {
    const mcpProcess = exec(`./target/release/terraphim_mcp_server --sse --bind 127.0.0.1:${MCP_SERVER_PORT}`, {
      cwd: path.resolve(__dirname, '../../crates/terraphim_mcp_server'),
      env: { 
        ...process.env, 
        RUST_LOG: 'info'
      }
    });

    mcpProcess.stdout?.on('data', (data) => {
      if (process.env.DEBUG) console.log(`MCP: ${data}`);
    });

    mcpProcess.stderr?.on('data', (data) => {
      if (process.env.DEBUG) console.error(`MCP Error: ${data}`);
    });
  } catch (error) {
    console.warn('⚠️ MCP server might already be running or failed to start');
  }

  // Wait for services to be ready
  console.log('⏳ Waiting for backend services to be ready...');
  
  for (let attempt = 1; attempt <= MAX_RETRIES; attempt++) {
    console.log(`🔄 Health check attempt ${attempt}/${MAX_RETRIES}`);

    // Check backend server health
    if (!environment.backendReady) {
      environment.backendReady = await checkServiceHealth(
        `http://localhost:${BACKEND_PORT}/health`,
        'Backend server'
      );
    }

    // Check MCP server health
    if (!environment.mcpServerReady) {
      environment.mcpServerReady = await checkServiceHealth(
        `http://localhost:${MCP_SERVER_PORT}/message?sessionId=health-check`,
        'MCP server'
      );
    }

    // If both services are ready, break
    if (environment.backendReady && environment.mcpServerReady) {
      console.log('✅ All backend services are ready!');
      break;
    }

    // Wait before next attempt
    if (attempt < MAX_RETRIES) {
      console.log(`⏳ Waiting ${RETRY_INTERVAL}ms before next health check...`);
      await new Promise(resolve => setTimeout(resolve, RETRY_INTERVAL));
    }
  }

  if (!environment.backendReady) {
    throw new Error(`Backend server failed to start on port ${BACKEND_PORT} after ${MAX_RETRIES} attempts`);
  }

  if (!environment.mcpServerReady) {
    console.warn(`⚠️ MCP server not ready on port ${MCP_SERVER_PORT}, some tests may fail`);
  }

  return environment;
}

/**
 * Perform functional tests of context management API
 */
async function performFunctionalTests(environment: TestEnvironment): Promise<void> {
  if (!environment.backendReady) {
    console.warn('⚠️ Skipping functional tests - backend not ready');
    return;
  }

  console.log('🧪 Performing functional tests...');

  const BACKEND_PORT = process.env.BACKEND_PORT || '8000';
  const baseUrl = `http://localhost:${BACKEND_PORT}`;

  try {
    // Test conversation creation
    console.log('🧪 Testing conversation creation...');
    const createResponse = await fetch(`${baseUrl}/conversations`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        title: 'Test Setup Conversation',
        role: 'Engineer'
      })
    });

    if (!createResponse.ok) {
      throw new Error(`Conversation creation failed: ${createResponse.status}`);
    }

    const createData = await createResponse.json();
    if (createData.status !== 'success') {
      throw new Error(`Conversation creation returned error: ${createData.error}`);
    }

    const conversationId = createData.conversation_id;
    console.log(`✅ Conversation created with ID: ${conversationId}`);

    // Test context addition
    console.log('🧪 Testing context addition...');
    const contextResponse = await fetch(`${baseUrl}/conversations/${conversationId}/context`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        context_type: 'system',
        title: 'Setup Test Context',
        content: 'This is a test context item created during setup validation',
        metadata: { source: 'setup_test' }
      })
    });

    if (!contextResponse.ok) {
      throw new Error(`Context addition failed: ${contextResponse.status}`);
    }

    const contextData = await contextResponse.json();
    if (contextData.status !== 'success') {
      throw new Error(`Context addition returned error: ${contextData.error}`);
    }

    console.log('✅ Context added successfully');

    // Test conversation retrieval
    console.log('🧪 Testing conversation retrieval...');
    const getResponse = await fetch(`${baseUrl}/conversations/${conversationId}`);
    
    if (!getResponse.ok) {
      throw new Error(`Conversation retrieval failed: ${getResponse.status}`);
    }

    const getData = await getResponse.json();
    if (getData.status !== 'success' || !getData.conversation) {
      throw new Error(`Conversation retrieval returned error: ${getData.error}`);
    }

    if (getData.conversation.global_context.length !== 1) {
      throw new Error('Context was not properly added to conversation');
    }

    console.log('✅ Conversation with context retrieved successfully');
    console.log('✅ All functional tests passed');

  } catch (error) {
    console.error('❌ Functional test failed:', error);
    throw new Error('Functional validation failed');
  }
}

/**
 * Setup test data for context management tests
 */
async function setupTestData(): Promise<void> {
  console.log('📊 Setting up test data...');

  // Create test configuration files
  const testConfig = {
    conversations: [
      {
        title: 'Sample Rust Programming Discussion',
        role: 'Engineer',
        messages: [
          { role: 'user', content: 'How do I handle async errors in Rust?' },
          { role: 'assistant', content: 'You can use Result types with async functions...' }
        ],
        context: [
          {
            type: 'document',
            title: 'Rust Async Error Handling',
            content: 'Best practices for error handling in async Rust code...'
          }
        ]
      }
    ],
    searchResults: [
      {
        id: 'rust-async-guide',
        title: 'Complete Rust Async Programming Guide',
        body: 'Comprehensive guide covering async/await, futures, and tokio runtime...',
        url: 'https://example.com/rust-async-guide',
        rank: 95
      }
    ]
  };

  try {
    await fs.writeFile(
      'test-results/test-data.json', 
      JSON.stringify(testConfig, null, 2)
    );
    console.log('✅ Test data configuration written');
  } catch (error) {
    console.warn('⚠️ Could not write test data configuration');
  }
}

/**
 * Main global setup function
 */
async function globalSetup(config: FullConfig): Promise<void> {
  console.log('🚀 Starting Context Management UI Test Global Setup');
  console.log(`📋 Running ${config.projects.length} test project(s)`);

  try {
    // Validate system requirements
    await validateSystemRequirements();

    // Setup test directories
    await setupTestDirectories();

    // Build required components
    await buildComponents();

    // Start backend services and validate they're working
    const environment = await startBackendServices();

    // Perform functional tests
    await performFunctionalTests(environment);

    // Setup test data
    await setupTestData();

    // Save environment state for tests
    const environmentState = {
      ...environment,
      timestamp: new Date().toISOString(),
      config: {
        backendPort: process.env.BACKEND_PORT || '8000',
        mcpServerPort: process.env.MCP_SERVER_PORT || '8001',
        frontendPort: process.env.FRONTEND_PORT || '5173',
      }
    };

    await fs.writeFile(
      'test-results/environment.json',
      JSON.stringify(environmentState, null, 2)
    );

    console.log('✅ Global setup completed successfully');
    console.log(`📊 Environment state saved to test-results/environment.json`);

  } catch (error) {
    console.error('❌ Global setup failed:', error);
    throw error;
  }
}

export default globalSetup;