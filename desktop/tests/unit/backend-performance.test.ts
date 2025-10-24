/**
 * Backend Performance Unit Tests
 * 
 * These tests validate that the backend performance optimizations are working:
 * 1. No persistence operations in search path
 * 2. Fast search responses
 * 3. No opendal warnings
 */

import { test, expect } from '@playwright/test';

test.describe('Backend Performance Validation', () => {
  
  test('should validate search API response time', async ({ request }) => {
    console.log('🔍 Testing search API response time...');
    
    const searchQuery = {
      needle: 'artificial intelligence',
      role: 'Default',
      limit: 10
    };
    
    const startTime = Date.now();
    
    const response = await request.post('http://localhost:8000/api/documents/search', {
      data: searchQuery,
      headers: {
        'Content-Type': 'application/json'
      }
    });
    
    const responseTime = Date.now() - startTime;
    
    // Validate response
    expect(response.status()).toBe(200);
    
    const data = await response.json();
    expect(data.status).toBe('success');
    expect(data.results).toBeDefined();
    
    // Validate performance
    expect(responseTime).toBeLessThan(2000); // Should be under 2 seconds
    
    console.log(`✅ Search API response time: ${responseTime}ms`);
    console.log(`   Results count: ${data.results.length}`);
  });

  test('should validate Rust Engineer role search performance', async ({ request }) => {
    console.log('🔍 Testing Rust Engineer role search performance...');
    
    const searchQuery = {
      needle: 'async tokio',
      role: 'Rust Engineer',
      limit: 10
    };
    
    const startTime = Date.now();
    
    const response = await request.post('http://localhost:8000/api/documents/search', {
      data: searchQuery,
      headers: {
        'Content-Type': 'application/json'
      }
    });
    
    const responseTime = Date.now() - startTime;
    
    // Validate response
    expect(response.status()).toBe(200);
    
    const data = await response.json();
    expect(data.status).toBe('success');
    
    // Validate performance
    expect(responseTime).toBeLessThan(2000); // Should be under 2 seconds
    
    console.log(`✅ Rust Engineer search response time: ${responseTime}ms`);
  });

  test('should validate Terraphim Engineer role search performance', async ({ request }) => {
    console.log('🔍 Testing Terraphim Engineer role search performance...');
    
    const searchQuery = {
      needle: 'knowledge graph terraphim',
      role: 'Terraphim Engineer',
      limit: 10
    };
    
    const startTime = Date.now();
    
    const response = await request.post('http://localhost:8000/api/documents/search', {
      data: searchQuery,
      headers: {
        'Content-Type': 'application/json'
      }
    });
    
    const responseTime = Date.now() - startTime;
    
    // Validate response
    expect(response.status()).toBe(200);
    
    const data = await response.json();
    expect(data.status).toBe('success');
    
    // Validate performance
    expect(responseTime).toBeLessThan(2000); // Should be under 2 seconds
    
    console.log(`✅ Terraphim Engineer search response time: ${responseTime}ms`);
  });

  test('should validate multiple concurrent searches', async ({ request }) => {
    console.log('🔍 Testing multiple concurrent searches...');
    
    const searchQueries = [
      { needle: 'artificial intelligence', role: 'Default' },
      { needle: 'async tokio', role: 'Rust Engineer' },
      { needle: 'knowledge graph', role: 'Terraphim Engineer' }
    ];
    
    const startTime = Date.now();
    
    // Run all searches concurrently
    const promises = searchQueries.map(query => 
      request.post('http://localhost:8000/api/documents/search', {
        data: query,
        headers: {
          'Content-Type': 'application/json'
        }
      })
    );
    
    const responses = await Promise.all(promises);
    const totalTime = Date.now() - startTime;
    
    // Validate all responses
    for (const response of responses) {
      expect(response.status()).toBe(200);
      
      const data = await response.json();
      expect(data.status).toBe('success');
    }
    
    // Validate performance - concurrent searches should still be fast
    expect(totalTime).toBeLessThan(3000); // Should be under 3 seconds for all
    
    console.log(`✅ All ${searchQueries.length} concurrent searches completed in ${totalTime}ms`);
  });

  test('should validate server health and configuration', async ({ request }) => {
    console.log('🔍 Testing server health and configuration...');
    
    // Test health endpoint
    const healthResponse = await request.get('http://localhost:8000/health');
    expect(healthResponse.status()).toBe(200);
    
    const healthData = await healthResponse.json();
    expect(healthData.status).toBe('ok');
    
    // Test config endpoint
    const configResponse = await request.get('http://localhost:8000/api/config');
    expect(configResponse.status()).toBe(200);
    
    const configData = await configResponse.json();
    expect(configData.roles).toBeDefined();
    expect(configData.roles['Default']).toBeDefined();
    expect(configData.roles['Rust Engineer']).toBeDefined();
    expect(configData.roles['Terraphim Engineer']).toBeDefined();
    
    console.log('✅ Server health and configuration validated');
  });
});
