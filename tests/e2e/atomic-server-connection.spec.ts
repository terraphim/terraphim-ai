import { test, expect } from '@playwright/test';
import fs from 'fs';

// Load atomic server credentials from environment variables
const ATOMIC_SERVER_URL = process.env.ATOMIC_SERVER_URL || "http://localhost:9883/";
const ATOMIC_SERVER_SECRET = process.env.ATOMIC_SERVER_SECRET;

// Validate required environment variables
if (!ATOMIC_SERVER_SECRET) {
  throw new Error('ATOMIC_SERVER_SECRET environment variable is required for atomic server tests');
}

class AtomicServerManager {
  private baseUrl: string;
  private secret: string;

  constructor(baseUrl: string = ATOMIC_SERVER_URL, secret: string = ATOMIC_SERVER_SECRET!) {
    this.baseUrl = baseUrl.endsWith('/') ? baseUrl.slice(0, -1) : baseUrl;
    this.secret = secret;
  }

  async isReady(): Promise<boolean> {
    try {
      const response = await fetch(this.baseUrl);
      return response.status < 500;
    } catch {
      return false;
    }
  }

  async testConnection(): Promise<boolean> {
    console.log(`🔗 Testing connection to ${this.baseUrl}...`);
    const isConnected = await this.isReady();
    
    if (isConnected) {
      console.log('✅ Atomic server connection successful');
    } else {
      console.log('❌ Atomic server connection failed');
    }
    
    return isConnected;
  }
}

// Configure tests to use reasonable timeouts
test.setTimeout(30000);

test.describe('Atomic Server Connection Tests', () => {
  let atomicServerManager: AtomicServerManager;

  test.beforeAll(async () => {
    atomicServerManager = new AtomicServerManager();
  });

  test('should successfully connect to atomic server', async () => {
    const isConnected = await atomicServerManager.testConnection();
    expect(isConnected).toBe(true);
  });

  test('should validate environment variables are loaded', async () => {
    expect(ATOMIC_SERVER_URL).toBeTruthy();
    expect(ATOMIC_SERVER_SECRET).toBeTruthy();
    expect(ATOMIC_SERVER_URL).toMatch(/^https?:\/\//);
    expect(ATOMIC_SERVER_SECRET.length).toBeGreaterThan(50);
  });

  test('should handle invalid URL gracefully', async () => {
    const invalidManager = new AtomicServerManager('http://localhost:99999', ATOMIC_SERVER_SECRET);
    const isConnected = await invalidManager.isReady();
    expect(isConnected).toBe(false);
  });
});

test.describe('Atomic Server Configuration Tests', () => {
  test('should create and validate configuration structure', async () => {
    const config = {
      roles: {
        'Test Role': {
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
    
    expect(config.roles['Test Role']).toBeDefined();
    expect(config.roles['Test Role'].haystacks).toHaveLength(1);
    expect(config.roles['Test Role'].haystacks[0].service).toBe('Atomic');
    expect(config.roles['Test Role'].haystacks[0].location).toBe(ATOMIC_SERVER_URL);
  });

  test('should write and read configuration files', async () => {
    const config = {
      roles: {
        'File Test Role': {
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
    
    const configPath = 'test-atomic-config.json';
    
    // Write config
    fs.writeFileSync(configPath, JSON.stringify(config, null, 2));
    expect(fs.existsSync(configPath)).toBe(true);
    
    // Read and validate
    const readConfig = JSON.parse(fs.readFileSync(configPath, 'utf8'));
    expect(readConfig).toEqual(config);
    
    // Cleanup
    fs.unlinkSync(configPath);
  });
}); 