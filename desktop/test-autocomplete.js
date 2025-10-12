#!/usr/bin/env node

/**
 * Test script to demonstrate autocomplete functionality with the MCP server
 */

const BASE_URL = 'http://localhost:8001';
const SESSION_ID = `test-${Date.now()}`;

async function testMCPEndpoint() {
  console.log('🧪 Testing MCP Server Autocomplete Functionality');
  console.log('================================================');
  console.log(`Server: ${BASE_URL}`);
  console.log(`Session: ${SESSION_ID}`);
  console.log('');

  try {
    // Test 1: List available tools
    console.log('1️⃣ Testing tools/list...');
    const toolsResponse = await fetch(`${BASE_URL}/message?sessionId=${SESSION_ID}`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        jsonrpc: '2.0',
        id: 1,
        method: 'tools/list',
        params: {}
      })
    });

    if (toolsResponse.ok) {
      const tools = await toolsResponse.json();
      console.log('✅ Tools list response received');
      console.log('Available tools:', tools.result?.tools?.map(t => t.name) || 'None');
    } else {
      console.log('❌ Tools list failed:', toolsResponse.status, toolsResponse.statusText);
    }
    console.log('');

    // Test 2: Build autocomplete index
    console.log('2️⃣ Testing build_autocomplete_index...');
    const buildResponse = await fetch(`${BASE_URL}/message?sessionId=${SESSION_ID}`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        jsonrpc: '2.0',
        id: 2,
        method: 'tools/call',
        params: {
          name: 'build_autocomplete_index',
          arguments: {}
        }
      })
    });

    if (buildResponse.ok) {
      const buildResult = await buildResponse.json();
      console.log('✅ Build index response received');
      console.log('Result:', buildResult);
    } else {
      console.log('❌ Build index failed:', buildResponse.status, buildResponse.statusText);
    }
    console.log('');

    // Test 3: Test autocomplete with snippets
    console.log('3️⃣ Testing autocomplete_with_snippets...');
    const autocompleteResponse = await fetch(`${BASE_URL}/message?sessionId=${SESSION_ID}`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        jsonrpc: '2.0',
        id: 3,
        method: 'tools/call',
        params: {
          name: 'autocomplete_with_snippets',
          arguments: {
            query: 'terraphim',
            limit: 5
          }
        }
      })
    });

    if (autocompleteResponse.ok) {
      const autocompleteResult = await autocompleteResponse.json();
      console.log('✅ Autocomplete response received');
      console.log('Result:', autocompleteResult);

      if (autocompleteResult.result?.content) {
        console.log('\n📝 Autocomplete Suggestions:');
        autocompleteResult.result.content.forEach((item, index) => {
          if (item.type === 'text') {
            console.log(`  ${index + 1}. ${item.text}`);
          }
        });
      }
    } else {
      console.log('❌ Autocomplete failed:', autocompleteResponse.status, autocompleteResponse.statusText);
    }
    console.log('');

    // Test 4: Test basic autocomplete
    console.log('4️⃣ Testing autocomplete_terms...');
    const basicResponse = await fetch(`${BASE_URL}/message?sessionId=${SESSION_ID}`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        jsonrpc: '2.0',
        id: 4,
        method: 'tools/call',
        params: {
          name: 'autocomplete_terms',
          arguments: {
            query: 'graph',
            limit: 3
          }
        }
      })
    });

    if (basicResponse.ok) {
      const basicResult = await basicResponse.json();
      console.log('✅ Basic autocomplete response received');
      console.log('Result:', basicResult);
    } else {
      console.log('❌ Basic autocomplete failed:', basicResponse.status, basicResponse.statusText);
    }

  } catch (error) {
    console.error('❌ Test failed with error:', error.message);
  }
}

// Run the test
testMCPEndpoint().then(() => {
  console.log('\n🏁 Test completed');
  process.exit(0);
}).catch((error) => {
  console.error('💥 Test crashed:', error);
  process.exit(1);
});
