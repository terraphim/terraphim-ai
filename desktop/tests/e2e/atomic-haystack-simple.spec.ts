import { test, expect } from '@playwright/test';
import fs from 'fs';
import path from 'path';

const ATOMIC_SERVER_URL = process.env.ATOMIC_SERVER_URL || 'http://localhost:9883';
const ATOMIC_SERVER_SECRET = process.env.ATOMIC_SERVER_SECRET;

test.describe('Atomic Server Haystack Simple Tests', () => {
  test.beforeAll(async () => {
    // Ensure we have the required environment variables
    if (!ATOMIC_SERVER_SECRET) {
      throw new Error('ATOMIC_SERVER_SECRET environment variable is not set');
    }
    console.log('✅ Environment variables loaded');
    console.log('🔍 Secret starts with:', ATOMIC_SERVER_SECRET.substring(0, 50) + '...');
  });

  test('should configure and test atomic haystack sequentially', async () => {
    console.log('🔧 Starting atomic haystack configuration and test...');

    // Step 1: Configure the server
    console.log('📝 Step 1: Configuring Terraphim server...');

    const config = {
      id: "Server",
      global_shortcut: "Ctrl+Shift+F",
      roles: {
        "Atomic Debug Fixed": {
          shortname: "AtomicDebugFixed",
          name: "Atomic Debug Fixed",
          relevance_function: "title-scorer",
          terraphim_it: false,
          theme: "spacelab",
          kg: null,
          haystacks: [{
            location: "http://localhost:9883/",
            service: "Atomic",
            read_only: true,
            atomic_server_secret: ATOMIC_SERVER_SECRET
          }],
          extra: {}
        }
      },
      default_role: "Atomic Debug Fixed",
      selected_role: "Atomic Debug Fixed"
    };

    console.log('🔍 Config secret starts with:', config.roles["Atomic Debug Fixed"].haystacks[0]?.atomic_server_secret?.substring(0, 50) + '...');

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
      console.log('✅ Successfully configured Terraphim server');
    }

    // Step 2: Wait for configuration to be applied
    console.log('⏳ Step 2: Waiting for configuration to be applied...');
    await new Promise(resolve => setTimeout(() => resolve(undefined), 5000));

    // Step 3: Verify configuration was applied
    console.log('🔍 Step 3: Verifying configuration was applied...');
    const configResponse = await fetch('http://localhost:8000/config');
    const currentConfig = await configResponse.json();
    console.log('📊 Current config roles:', Object.keys(currentConfig.config.roles || {}));

    // Step 4: Test search functionality
    console.log('🔍 Step 4: Testing search functionality...');

    const searchResponse = await fetch('http://localhost:8000/documents/search', {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
      },
      body: JSON.stringify({
        search_term: 'field',
        role: 'Atomic Debug Fixed',
        limit: 10
      })
    });

    console.log('📊 Search response status:', searchResponse.status);

    if (searchResponse.status !== 200) {
      const errorText = await searchResponse.text();
      console.log('❌ Search failed with error:', errorText);
      throw new Error(`Search failed with status ${searchResponse.status}: ${errorText}`);
    }

    expect(searchResponse.status).toBe(200);

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
    console.log('🎉 All atomic haystack tests passed!');
  });
});
