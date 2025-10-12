import { test, expect } from '@playwright/test';
import fs from 'fs';
import path from 'path';

test.describe('Atomic Server Haystack File Tests', () => {
  test('should configure and test atomic haystack using file config', async () => {
    console.log('🔧 Starting atomic haystack configuration and test using file...');

    // Step 1: Read the working configuration file
    console.log('📝 Step 1: Reading working configuration file...');
    const configPath = path.join(process.cwd(), 'atomic-debug-fixed-config.json');
    const config = JSON.parse(fs.readFileSync(configPath, 'utf8'));

    console.log('🔍 Config secret starts with:', config.roles["Atomic Debug Fixed"].haystacks[0]?.atomic_server_secret?.substring(0, 50) + '...');

    // Step 2: Configure the server
    console.log('📝 Step 2: Configuring Terraphim server...');
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

    // Step 3: Wait for configuration to be applied
    console.log('⏳ Step 3: Waiting for configuration to be applied...');
    await new Promise(resolve => setTimeout(() => resolve(undefined), 5000));

    // Step 4: Verify configuration was applied
    console.log('🔍 Step 4: Verifying configuration was applied...');
    const configResponse = await fetch('http://localhost:8000/config');
    const currentConfig = await configResponse.json();
    console.log('📊 Current config roles:', Object.keys(currentConfig.config.roles || {}));

    // Step 5: Test search functionality
    console.log('🔍 Step 5: Testing search functionality...');

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
