import { test, expect } from '@playwright/test';
import fs from 'fs';
import path from 'path';
import { fileURLToPath } from 'url';

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);

const ATOMIC_SERVER_URL = "http://localhost:9883/";
const ATOMIC_SERVER_SECRET = process.env.ATOMIC_SERVER_SECRET;

test.describe('Atomic Server Haystack Working Tests', () => {
  let configPath: string;

  test.beforeAll(async () => {
    if (!ATOMIC_SERVER_SECRET) {
      throw new Error('ATOMIC_SERVER_SECRET environment variable is not set');
    }

    // Create a working test configuration for atomic server
    configPath = path.join(__dirname, 'atomic-working-config.json');
    
    const config = {
      id: "Server",
      global_shortcut: "Ctrl+Shift+A",
      roles: {
        'Atomic Debug Fixed': {
          shortname: "AtomicDebugFixed",
          name: "Atomic Debug Fixed",
          relevance_function: "title-scorer",
          terraphim_it: false,
          theme: "spacelab",
          kg: null,
          haystacks: [
            {
              location: ATOMIC_SERVER_URL,
              service: "Atomic",
              read_only: true,
              atomic_server_secret: ATOMIC_SERVER_SECRET
            }
          ],
          extra: {}
        }
      },
      default_role: "Atomic Debug Fixed",
      selected_role: "Atomic Debug Fixed"
    };

    fs.writeFileSync(configPath, JSON.stringify(config, null, 2));
    console.log('🔧 Created atomic working configuration');
  });

  test.afterAll(async () => {
    // Cleanup
    if (fs.existsSync(configPath)) {
      fs.unlinkSync(configPath);
    }
  });

  test('should configure Terraphim server with atomic haystack', async () => {
    console.log('🔧 Configuring Terraphim server with atomic haystack...');
    
    const config = JSON.parse(fs.readFileSync(configPath, 'utf8'));
    
    const updateResponse = await fetch('http://localhost:8000/config', {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
      },
      body: JSON.stringify(config)
    });
    
    console.log('📊 Config update response status:', updateResponse.status);
    expect(updateResponse.status).toBe(200);
    
    if (updateResponse.status === 200) {
      console.log('✅ Successfully configured Terraphim server with atomic haystack');
    }
  });

  test('should search atomic server haystack through Terraphim', async () => {
    console.log('🔍 Testing atomic server haystack search...');
    
    // Wait for configuration to be applied
    await new Promise(resolve => setTimeout(() => resolve(undefined), 2000));
    
    const searchResponse = await fetch('http://localhost:8000/documents/search', {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
      },
              body: JSON.stringify({
          search_term: 'form',
          role: 'Atomic Debug Fixed',
          limit: 10
        })
    });
    
    console.log('📊 Search response status:', searchResponse.status);
    expect(searchResponse.status).toBe(200);
    
    if (searchResponse.status === 200) {
      const searchResults = await searchResponse.json();
      console.log('✅ Atomic haystack search successful!');
      console.log(`📊 Found ${searchResults.results?.length || 0} results`);
      
      // Verify we got results
      expect(searchResults.results).toBeDefined();
      expect(Array.isArray(searchResults.results)).toBe(true);
      expect(searchResults.results.length).toBeGreaterThan(0);
      
      // Verify result structure
      const firstResult = searchResults.results[0];
      expect(firstResult).toHaveProperty('id');
      expect(firstResult).toHaveProperty('title');
      expect(firstResult).toHaveProperty('body');
      expect(firstResult).toHaveProperty('url');
      
      console.log('✅ Search results have proper structure');
    }
  });

  test('should search atomic server haystack with different terms', async () => {
    console.log('🔍 Testing atomic server haystack with different search terms...');
    
    const searchTerms = ['field', 'input', 'search', 'test'];
    
    for (const term of searchTerms) {
      console.log(`🔍 Searching for: "${term}"`);
      
      const searchResponse = await fetch('http://localhost:8000/documents/search', {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
        },
        body: JSON.stringify({
          search_term: term,
          role: 'Atomic Debug Fixed',
          limit: 5
        })
      });
      
      expect(searchResponse.status).toBe(200);
      
      if (searchResponse.status === 200) {
        const searchResults = await searchResponse.json();
        console.log(`📊 Found ${searchResults.results?.length || 0} results for "${term}"`);
        
        // Verify we got results structure
        expect(searchResults.results).toBeDefined();
        expect(Array.isArray(searchResults.results)).toBe(true);
        
        if (searchResults.results.length > 0) {
          console.log(`✅ Search for "${term}" returned relevant results`);
        }
      }
    }
  });

  test('should verify atomic server connectivity', async ({ page }) => {
    console.log('🔍 Verifying atomic server connectivity...');
    
    // Test atomic server root endpoint
    const rootResponse = await page.request.get(ATOMIC_SERVER_URL, {
      headers: { 'Accept': 'application/json' }
    });
    
    expect(rootResponse.ok()).toBeTruthy();
    console.log('✅ Atomic server root endpoint accessible');
    
    // Test haystack endpoint
    const haystackResponse = await page.request.get(`${ATOMIC_SERVER_URL}haystack`, {
      headers: { 'Accept': 'application/json' }
    });
    
    expect(haystackResponse.ok()).toBeTruthy();
    console.log('✅ Atomic server haystack endpoint accessible');
    
    // Test search endpoint
    const searchResponse = await page.request.get(`${ATOMIC_SERVER_URL}search?q=test`, {
      headers: { 'Accept': 'application/json' }
    });
    
    expect(searchResponse.ok()).toBeTruthy();
    console.log('✅ Atomic server search endpoint accessible');
  });

  test('should verify atomic server agent authentication', async ({ page }) => {
    console.log('🔍 Verifying atomic server agent authentication...');
    
    if (!ATOMIC_SERVER_SECRET) {
      throw new Error('ATOMIC_SERVER_SECRET environment variable is not set');
    }
    
    // Extract agent subject from secret
    const agentSubject = await page.evaluate((secret) => {
      try {
        const decoded = atob(secret);
        const json = JSON.parse(decoded);
        return json.subject;
      } catch (error) {
        throw new Error(`Failed to extract agent subject: ${error}`);
      }
    }, ATOMIC_SERVER_SECRET);
    
    // Test agent endpoint
    const agentResponse = await page.request.get(agentSubject, {
      headers: { 'Accept': 'application/json' }
    });
    
    expect(agentResponse.ok()).toBeTruthy();
    console.log('✅ Atomic server agent authentication working');
    
    if (agentResponse.ok()) {
      const agentData = await agentResponse.json();
      console.log('📊 Agent name:', agentData.name);
      console.log('📊 Agent public key:', agentData['public-key']);
    }
  });

  test('should test atomic server haystack performance', async () => {
    console.log('🔍 Testing atomic server haystack performance...');
    
    const startTime = Date.now();
    
    const searchResponse = await fetch('http://localhost:8000/documents/search', {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
      },
              body: JSON.stringify({
          search_term: 'test',
          role: 'Atomic Debug Fixed',
          limit: 20
        })
    });
    
    const endTime = Date.now();
    const responseTime = endTime - startTime;
    
    console.log(`📊 Search response time: ${responseTime}ms`);
    
    expect(searchResponse.status).toBe(200);
    expect(responseTime).toBeLessThan(5000); // Should complete within 5 seconds
    
    if (searchResponse.status === 200) {
      const searchResults = await searchResponse.json();
      console.log(`✅ Performance test passed: ${searchResults.results?.length || 0} results in ${responseTime}ms`);
    }
  });
}); 