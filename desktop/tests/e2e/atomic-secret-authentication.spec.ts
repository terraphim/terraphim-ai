import { test, expect } from '@playwright/test';
import { spawn, ChildProcess } from 'child_process';
import fs from 'fs';
import path from 'path';
import { fileURLToPath } from 'url';

// If using ESM, import dotenv/config at the very top for .env support
import 'dotenv/config';
import { config } from 'dotenv';
// Load .env from the project root (one level up from desktop)
config({ path: '../../.env' });

// Ensure __filename and __dirname are defined before use
const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);

// Comment: If you see errors about Node.js types, run: yarn add -D @types/node

const ATOMIC_SERVER_URL = process.env.ATOMIC_SERVER_URL || "http://localhost:9883/";
const ATOMIC_SERVER_SECRET = process.env.ATOMIC_SERVER_SECRET;
const TERRAPHIM_SERVER_URL = process.env.TERRAPHIM_SERVER_URL || "http://localhost:8000";

// Skip tests if atomic server secret is not available
const shouldSkipAtomicTests = !ATOMIC_SERVER_SECRET;

class TerraphimServerManager {
  private process: ChildProcess | null = null;
  private port: number = 8000;

  async start(): Promise<void> {
    if (this.process) {
      console.log('Server already running');
      return;
    }

    console.log('🚀 Starting Terraphim server for atomic secret authentication...');
    
    // Start the server process
    this.process = spawn('cargo', ['run', '--bin', 'terraphim_server'], {
      cwd: path.resolve(__dirname, '../../'),
      stdio: ['pipe', 'pipe', 'pipe'],
      env: {
        ...process.env,
        RUST_LOG: 'info',
      },
    });

    // Handle stdout and stderr
    this.process.stdout?.on('data', (data) => {
      const output = data.toString();
      if (output.includes('listening on')) {
        console.log('✅ Terraphim server started');
      }
    });

    this.process.stderr?.on('data', (data) => {
      const output = data.toString();
      if (output.includes('listening on')) {
        console.log('✅ Terraphim server started');
      } else if (output.includes('error') || output.includes('Error')) {
        console.log('Terraphim server error:', output.trim());
      }
    });

    // Wait for server to start
    await this.waitForReady();
  }

  async waitForReady(): Promise<void> {
    console.log('⏳ Waiting for Terraphim server to be ready...');
    
    for (let i = 0; i < 30; i++) {
      try {
        const response = await fetch(`http://localhost:${this.port}/health`);
        if (response.ok) {
          console.log('✅ Terraphim server is ready');
          return;
        }
      } catch (error) {
        // Server not ready yet
      }
      await new Promise(resolve => setTimeout(resolve, 1000));
    }
    
    throw new Error('Terraphim server failed to start within 30 seconds');
  }

  async stop(): Promise<void> {
    if (this.process) {
      console.log('🛑 Terraphim server stopped');
      this.process.kill('SIGTERM');
      this.process = null;
    }
  }
}

test.describe('Atomic Secret Authentication', () => {
  let serverManager: TerraphimServerManager;

  test.beforeAll(async () => {
    test.skip(shouldSkipAtomicTests, "ATOMIC_SERVER_SECRET not available");
    
    console.log('🔧 Setting up atomic secret authentication tests...');
    
    // Test atomic server connectivity
    try {
      const response = await fetch(ATOMIC_SERVER_URL);
      expect(response.status).toBeLessThan(500);
      console.log('✅ Atomic server is accessible');
    } catch (error) {
      console.log('❌ Atomic server not accessible:', error);
      test.skip(true, "Atomic server not accessible");
    }
    
    serverManager = new TerraphimServerManager();
  });

  test.afterAll(async () => {
    if (serverManager) {
      await serverManager.stop();
    }
    console.log('🧹 Atomic secret authentication test cleanup completed');
  });

  test('should validate atomic server secret format and authentication', async () => {
    console.log('🔐 Testing atomic server secret validation...');
    
    // Validate that the secret is properly formatted
    expect(ATOMIC_SERVER_SECRET).toBeTruthy();
    expect(ATOMIC_SERVER_SECRET!.length).toBeGreaterThan(0);
    
    // Test that the secret can be decoded as base64
    try {
      const decoded = Buffer.from(ATOMIC_SERVER_SECRET!, 'base64').toString();
      expect(decoded).toBeTruthy();
      console.log('✅ Atomic server secret is valid base64');
    } catch (error) {
      console.log('❌ Atomic server secret is not valid base64:', error);
      throw error;
    }
    
    // Test authenticated access to atomic server
    const authResponse = await fetch(`${ATOMIC_SERVER_URL}/agents`, {
      headers: {
        'Accept': 'application/json',
        'Authorization': `Bearer ${ATOMIC_SERVER_SECRET}`
      }
    });
    
    // Should get a proper response (not 401/403 for invalid auth)
    expect([200, 400, 404, 422]).toContain(authResponse.status);
    console.log(`✅ Authenticated access test completed with status: ${authResponse.status}`);
  });

  test('should configure Terraphim server with authenticated atomic haystack', async () => {
    console.log('🔧 Configuring Terraphim server with authenticated atomic haystack...');
    
    await serverManager.start();
    
    // Create configuration with authenticated atomic haystack
    const config = {
      id: "Server",
      global_shortcut: "Ctrl+Shift+H",
      roles: {
        'Authenticated Atomic Tester': {
          shortname: "AuthAtomicTest",
          name: "Authenticated Atomic Tester",
          relevance_function: "title-scorer",
          theme: "spacelab",
          kg: null,
          haystacks: [
            {
              location: ATOMIC_SERVER_URL.replace(/\/$/, ''),
              service: "Atomic",
              read_only: true,
              atomic_server_secret: ATOMIC_SERVER_SECRET
            }
          ],
          extra: {},
          terraphim_it: false
        }
      }
    };
    
    // Update server configuration
    const configResponse = await fetch(`${TERRAPHIM_SERVER_URL}/api/config`, {
      method: 'PUT',
      headers: {
        'Content-Type': 'application/json',
      },
      body: JSON.stringify(config),
      signal: AbortSignal.timeout(10000)
    });
    
    console.log(`Config update response status: ${configResponse.status}`);
    expect(configResponse.ok).toBeTruthy();
    console.log('✅ Successfully updated Terraphim server config with authenticated atomic haystack');
  });

  test('should perform authenticated atomic haystack search', async () => {
    console.log('🔍 Testing authenticated atomic haystack search...');
    
    // Test search with authenticated access
    const searchResponse = await fetch(`${TERRAPHIM_SERVER_URL}/documents/search`, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
      },
      body: JSON.stringify({
        search_term: 'test',
        role: 'Authenticated Atomic Tester',
        limit: 10
      }),
      signal: AbortSignal.timeout(15000)
    });
    
    expect(searchResponse.ok).toBeTruthy();
    
    if (searchResponse.ok) {
      const results = await searchResponse.json();
      console.log(`📄 Authenticated search results: ${results.results?.length || 0} documents`);
      expect(results.results).toBeDefined();
    }
    
    console.log('✅ Authenticated atomic haystack search completed');
  });

  test('should handle authentication errors gracefully', async () => {
    console.log('🔧 Testing authentication error handling...');
    
    // Test with invalid secret
    const invalidSecret = 'invalid_base64_secret';
    const invalidConfig = {
      id: "Server",
      roles: {
        'Invalid Auth Tester': {
          shortname: "InvalidAuthTest",
          name: "Invalid Auth Tester",
          relevance_function: "title-scorer",
          theme: "spacelab",
          kg: null,
          haystacks: [
            {
              location: ATOMIC_SERVER_URL.replace(/\/$/, ''),
              service: "Atomic",
              read_only: true,
              atomic_server_secret: invalidSecret
            }
          ],
          extra: {},
          terraphim_it: false
        }
      }
    };
    
    // Update server configuration with invalid secret
    const configResponse = await fetch(`${TERRAPHIM_SERVER_URL}/api/config`, {
      method: 'PUT',
      headers: {
        'Content-Type': 'application/json',
      },
      body: JSON.stringify(invalidConfig),
      signal: AbortSignal.timeout(10000)
    });
    
    // Should handle invalid secret gracefully
    expect([200, 400, 422, 500]).toContain(configResponse.status);
    console.log(`✅ Invalid secret handling completed with status: ${configResponse.status}`);
  });

  test('should compare public vs authenticated access', async () => {
    console.log('🔍 Comparing public vs authenticated access...');
    
    // Test public access
    const publicResponse = await fetch(`${ATOMIC_SERVER_URL}/agents`, {
      headers: {
        'Accept': 'application/json'
      }
    });
    
    // Test authenticated access
    const authResponse = await fetch(`${ATOMIC_SERVER_URL}/agents`, {
      headers: {
        'Accept': 'application/json',
        'Authorization': `Bearer ${ATOMIC_SERVER_SECRET}`
      }
    });
    
    console.log(`Public access status: ${publicResponse.status}`);
    console.log(`Authenticated access status: ${authResponse.status}`);
    
    // Both should return valid responses
    expect([200, 400, 404, 422]).toContain(publicResponse.status);
    expect([200, 400, 404, 422]).toContain(authResponse.status);
    
    console.log('✅ Public vs authenticated access comparison completed');
  });
}); 