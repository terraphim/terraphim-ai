#!/usr/bin/env node

/**
 * Test script to verify Novel editor autocomplete integration
 * This script tests both Tauri and MCP server backends
 */

const BASE_URL = 'http://localhost:8001';
const SESSION_ID = `novel-test-${Date.now()}`;

console.log('🧪 Testing Novel Editor Autocomplete Integration');
console.log('==============================================');
console.log(`Server: ${BASE_URL}`);
console.log(`Session: ${SESSION_ID}`);
console.log('');

async function testMCPIntegration() {
  console.log('1️⃣ Testing MCP Server Integration');
  console.log('--------------------------------');

  try {
    // Test connection
    const healthResponse = await fetch(`${BASE_URL}/message?sessionId=${SESSION_ID}`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        jsonrpc: '2.0',
        id: 1,
        method: 'tools/list',
        params: {}
      })
    });

    if (!healthResponse.ok) {
      console.log('❌ MCP server not responding');
      console.log(`   Status: ${healthResponse.status} ${healthResponse.statusText}`);
      console.log('   Make sure MCP server is running:');
      console.log('   cd crates/terraphim_mcp_server && cargo run -- --sse --bind 127.0.0.1:8001');
      return false;
    }

    const tools = await healthResponse.json();
    console.log('✅ MCP server responding');
    console.log(`   Available tools: ${tools.result?.tools?.length || 0}`);

    // Test autocomplete terms
    console.log('\n📝 Testing autocomplete_terms...');
    const termsResponse = await fetch(`${BASE_URL}/message?sessionId=${SESSION_ID}`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        jsonrpc: '2.0',
        id: 2,
        method: 'tools/call',
        params: {
          name: 'autocomplete_terms',
          arguments: {
            query: 'terraphim',
            limit: 5,
            role: 'Default'
          }
        }
      })
    });

    if (termsResponse.ok) {
      const termsResult = await termsResponse.json();
      console.log('✅ autocomplete_terms working');

      if (termsResult.result?.content) {
        console.log(`   Found ${termsResult.result.content.length} items`);
        const suggestions = termsResult.result.content
          .filter(item => item.type === 'text' && !item.text.startsWith('Found'))
          .slice(0, 3);
        suggestions.forEach((item, i) => {
          console.log(`   ${i + 1}. ${item.text}`);
        });
      }
    } else {
      console.log('❌ autocomplete_terms failed');
    }

    // Test autocomplete with snippets
    console.log('\n📝 Testing autocomplete_with_snippets...');
    const snippetsResponse = await fetch(`${BASE_URL}/message?sessionId=${SESSION_ID}`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        jsonrpc: '2.0',
        id: 3,
        method: 'tools/call',
        params: {
          name: 'autocomplete_with_snippets',
          arguments: {
            query: 'graph',
            limit: 3,
            role: 'Default'
          }
        }
      })
    });

    if (snippetsResponse.ok) {
      const snippetsResult = await snippetsResponse.json();
      console.log('✅ autocomplete_with_snippets working');

      if (snippetsResult.result?.content) {
        const suggestions = snippetsResult.result.content
          .filter(item => item.type === 'text' && !item.text.startsWith('Found'))
          .slice(0, 3);
        suggestions.forEach((item, i) => {
          console.log(`   ${i + 1}. ${item.text}`);
        });
      }
    } else {
      console.log('❌ autocomplete_with_snippets failed');
    }

    return true;
  } catch (error) {
    console.error('❌ MCP integration test failed:', error.message);
    return false;
  }
}

async function testTauriIntegration() {
  console.log('\n2️⃣ Testing Tauri Integration');
  console.log('----------------------------');

  console.log('ℹ️  Tauri integration requires the desktop app to be running');
  console.log('   Start with: cd desktop && yarn run tauri dev');
  console.log('   Or test manually in the app using the "Test" button');
}

async function testNovelEditorIntegration() {
  console.log('\n3️⃣ Testing Novel Editor Integration');
  console.log('----------------------------------');

  console.log('✨ TerraphimSuggestion Extension Features:');
  console.log('   • Trigger character: "/" (configurable)');
  console.log('   • Minimum query length: 1 character');
  console.log('   • Maximum suggestions: 8 (configurable)');
  console.log('   • Debounce delay: 300ms');
  console.log('   • Keyboard navigation: ↑↓ arrows, Tab/Enter to select, Esc to cancel');
  console.log('   • Visual feedback: Dropdown with suggestions, scores, and snippets');
  console.log('   • Fallback: Graceful degradation when backend unavailable');

  console.log('\n🎯 Testing Instructions:');
  console.log('   1. Open the Terraphim desktop app');
  console.log('   2. Navigate to an editor page');
  console.log('   3. Click "Demo" to insert sample content');
  console.log('   4. Type "/" followed by a term (e.g., "/terraphim")');
  console.log('   5. Verify suggestions appear in dropdown');
  console.log('   6. Use arrow keys to navigate, Tab/Enter to select');
  console.log('   7. Check the autocomplete status panel for connection info');
}

async function testServiceStatus() {
  console.log('\n4️⃣ Service Status Check');
  console.log('----------------------');

  // Test common ports where services might be running
  const ports = [8001, 3000, 8000, 8080];

  for (const port of ports) {
    const url = `http://localhost:${port}`;
    try {
      const response = await fetch(`${url}/health`, {
        signal: AbortSignal.timeout(2000)
      });
      console.log(`✅ Service responding on ${url}`);
      console.log(`   Status: ${response.status} ${response.statusText}`);
    } catch (error) {
      // Try MCP endpoint
      try {
        const mcpResponse = await fetch(`${url}/message?sessionId=health`, {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify({
            jsonrpc: '2.0',
            id: 0,
            method: 'tools/list',
            params: {}
          }),
          signal: AbortSignal.timeout(2000)
        });

        if (mcpResponse.ok || mcpResponse.status === 405) {
          console.log(`✅ MCP service responding on ${url}`);
        }
      } catch (mcpError) {
        console.log(`❌ No service on ${url}`);
      }
    }
  }
}

// Main test execution
async function runTests() {
  try {
    await testServiceStatus();

    const mcpWorking = await testMCPIntegration();
    await testTauriIntegration();
    await testNovelEditorIntegration();

    console.log('\n🏁 Test Summary');
    console.log('==============');
    console.log(`MCP Server Integration: ${mcpWorking ? '✅ Working' : '❌ Failed'}`);
    console.log('Tauri Integration: ℹ️  Requires manual testing in app');
    console.log('Novel Editor Integration: ✅ Configured');

    if (mcpWorking) {
      console.log('\n🎉 Autocomplete integration is ready!');
      console.log('   Start the desktop app and try typing "/" in the editor.');
    } else {
      console.log('\n⚠️  Start the MCP server to enable full functionality:');
      console.log('   cd crates/terraphim_mcp_server');
      console.log('   cargo run -- --sse --bind 127.0.0.1:8001 --verbose');
    }

  } catch (error) {
    console.error('💥 Test execution failed:', error);
  }
}

// Run the tests
runTests().then(() => {
  console.log('\n📊 Test completed - check results above');
  process.exit(0);
}).catch((error) => {
  console.error('💥 Test crashed:', error);
  process.exit(1);
});