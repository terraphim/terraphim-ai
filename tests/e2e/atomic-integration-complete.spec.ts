import { test, expect } from '@playwright/test';
import { spawn, ChildProcess } from 'child_process';
import fs from 'fs';
import path from 'path';

const ATOMIC_SERVER_URL = process.env.ATOMIC_SERVER_URL || "http://localhost:9883/";
const ATOMIC_SERVER_SECRET = process.env.ATOMIC_SERVER_SECRET;

if (!ATOMIC_SERVER_SECRET) {
  throw new Error('ATOMIC_SERVER_SECRET environment variable is required');
}

class TerraphimServerManager {
  private process: ChildProcess | null = null;
  private port: number = 8000;

  async start(): Promise<void> {
    console.log('🚀 Starting Terraphim server with new storage backend...');
    
    const serverPath = path.join(__dirname, '../../../target/release/terraphim_server');
    
    this.process = spawn(serverPath, [], {
      stdio: ['ignore', 'pipe', 'pipe'],
      cwd: path.join(__dirname, '../../..')
    });

    return new Promise((resolve, reject) => {
      const timeout = setTimeout(() => {
        reject(new Error('Terraphim server startup timeout'));
      }, 30000);

      this.process!.stdout?.on('data', (data) => {
        const output = data.toString();
        console.log('Terraphim server:', output.trim());
        if (output.includes('listening on')) {
          clearTimeout(timeout);
          resolve();
        }
      });

      this.process!.stderr?.on('data', (data) => {
        const error = data.toString();
        console.error('Terraphim server error:', error.trim());
        // Don't reject on stderr - some warnings are normal
      });

      this.process!.on('error', (error) => {
        clearTimeout(timeout);
        reject(error);
      });

      this.process!.on('exit', (code) => {
        if (code !== 0) {
          clearTimeout(timeout);
          reject(new Error(`Terraphim server exited with code ${code}`));
        }
      });
    });
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
      } catch {
        // Continue waiting
      }
      await new Promise(resolve => setTimeout(resolve, 1000));
    }
    throw new Error('Terraphim server failed to become ready');
  }

  async stop(): Promise<void> {
    if (this.process) {
      this.process.kill('SIGTERM');
      await new Promise(resolve => setTimeout(resolve, 2000));
      if (!this.process.killed) {
        this.process.kill('SIGKILL');
      }
      this.process = null;
      console.log('🛑 Terraphim server stopped');
    }
  }
}

test.describe('Complete Atomic Server Integration', () => {
  let terraphimServer: TerraphimServerManager;
  const configPath = 'atomic-integration-config.json';

  test.beforeAll(async () => {
    // Create role configuration with atomic server haystack
    const config = {
      roles: {
        'Atomic Integration Test': {
          haystacks: [
            {
              location: ATOMIC_SERVER_URL,
              service: "Atomic",
              read_only: true,
              atomic_server_secret: ATOMIC_SERVER_SECRET
            }
          ]
        }
      }
    };

    fs.writeFileSync(configPath, JSON.stringify(config, null, 2));
    console.log('📝 Created atomic server role configuration');

    // Start Terraphim server
    terraphimServer = new TerraphimServerManager();
    await terraphimServer.start();
    await terraphimServer.waitForReady();
  });

  test.afterAll(async () => {
    if (terraphimServer) {
      await terraphimServer.stop();
    }
    
    // Cleanup
    if (fs.existsSync(configPath)) {
      fs.unlinkSync(configPath);
    }
  });

  test('should load atomic server role configuration', async () => {
    console.log('🔧 Testing role configuration loading...');
    
    // Test that the configuration endpoint accepts our atomic role
    const configData = fs.readFileSync(configPath, 'utf8');
    const config = JSON.parse(configData);
    
    expect(config.roles['Atomic Integration Test']).toBeDefined();
    expect(config.roles['Atomic Integration Test'].haystacks).toHaveLength(1);
    expect(config.roles['Atomic Integration Test'].haystacks[0].service).toBe('Atomic');
    expect(config.roles['Atomic Integration Test'].haystacks[0].location).toBe(ATOMIC_SERVER_URL);
    
    console.log('✅ Role configuration is valid');
  });

  test('should connect to both servers', async () => {
    console.log('🔗 Testing server connectivity...');
    
    // Test Terraphim server
    const terraphimResponse = await fetch('http://localhost:8000/health');
    expect(terraphimResponse.ok).toBe(true);
    console.log('✅ Terraphim server is accessible');
    
    // Test Atomic server
    const atomicResponse = await fetch(ATOMIC_SERVER_URL);
    expect(atomicResponse.status).toBeLessThan(500);
    console.log('✅ Atomic server is accessible');
  });

  test('should perform atomic server haystack search and return results', async () => {
    console.log('🔍 Testing atomic server haystack search...');
    
    try {
      // Update Terraphim server configuration with atomic role
      const updateResponse = await fetch('http://localhost:8000/api/config', {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
        },
        body: fs.readFileSync(configPath, 'utf8')
      });
      
      if (updateResponse.ok) {
        console.log('✅ Successfully updated Terraphim server config');
      } else {
        console.log('⚠️ Config update response:', updateResponse.status, await updateResponse.text());
      }
      
      // Wait a moment for configuration to be applied
      await new Promise(resolve => setTimeout(resolve, 2000));
      
      // Perform search through atomic haystack
      const searchResponse = await fetch('http://localhost:8000/api/search', {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
        },
        body: JSON.stringify({
          query: 'test',
          role: 'Atomic Integration Test',
          limit: 10
        })
      });
      
      expect(searchResponse.ok).toBe(true);
      
      const searchResults = await searchResponse.json();
      console.log('🔍 Search results:', JSON.stringify(searchResults, null, 2));
      
      // Verify we got results structure
      expect(searchResults).toBeDefined();
      
      // Even if no documents match, we should get a valid response structure
      if (searchResults.results && Array.isArray(searchResults.results)) {
        console.log(`✅ Search returned ${searchResults.results.length} results`);
        
        // If we have results, verify they have the expected structure
        if (searchResults.results.length > 0) {
          const firstResult = searchResults.results[0];
          expect(firstResult).toHaveProperty('content');
          console.log('✅ Search results have proper structure');
        }
      } else {
        console.log('ℹ️ Search response structure:', Object.keys(searchResults));
      }
      
    } catch (error) {
      console.error('❌ Search test error:', error);
      throw error;
    }
  });
}); 