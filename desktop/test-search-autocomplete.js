#!/usr/bin/env node

/**
 * Test script to verify search autocomplete functionality with the main Terraphim server
 */

const BASE_URL = 'http://localhost:8000';

async function testSearchAutocomplete() {
  console.log('🔍 Testing Search Autocomplete Functionality');
  console.log('============================================');
  console.log(`Server: ${BASE_URL}`);
  console.log('');

  try {
    // Test 1: Check if server is running
    console.log('1️⃣ Testing server health...');
    const healthResponse = await fetch(`${BASE_URL}/health`);

    if (healthResponse.ok) {
      const health = await healthResponse.text();
      console.log('✅ Server is running:', health);
    } else {
      console.log('❌ Server health check failed:', healthResponse.status, healthResponse.statusText);
      return;
    }
    console.log('');

    // Test 2: Test autocomplete endpoint
    console.log('2️⃣ Testing FST autocomplete endpoint...');
    const role = 'Terraphim Engineer';
    const query = 'terraphim';

    const autocompleteResponse = await fetch(`${BASE_URL}/autocomplete/${encodeURIComponent(role)}/${encodeURIComponent(query)}`);

    if (autocompleteResponse.ok) {
      const autocompleteResult = await autocompleteResponse.json();
      console.log('✅ Autocomplete response received');
      console.log('Result:', JSON.stringify(autocompleteResult, null, 2));

      if (autocompleteResult.status === 'success' && autocompleteResult.suggestions) {
        console.log('\n📝 Autocomplete Suggestions:');
        autocompleteResult.suggestions.forEach((suggestion, index) => {
          console.log(`  ${index + 1}. ${suggestion.term} (score: ${suggestion.score})`);
        });
      }
    } else {
      console.log('❌ Autocomplete failed:', autocompleteResponse.status, autocompleteResponse.statusText);
      console.log('Response text:', await autocompleteResponse.text());
    }
    console.log('');

    // Test 3: Test with another query
    console.log('3️⃣ Testing autocomplete with "graph"...');
    const query2 = 'graph';

    const autocompleteResponse2 = await fetch(`${BASE_URL}/autocomplete/${encodeURIComponent(role)}/${encodeURIComponent(query2)}`);

    if (autocompleteResponse2.ok) {
      const autocompleteResult2 = await autocompleteResponse2.json();
      console.log('✅ Second autocomplete response received');
      console.log('Result:', JSON.stringify(autocompleteResult2, null, 2));
    } else {
      console.log('❌ Second autocomplete failed:', autocompleteResponse2.status, autocompleteResponse2.statusText);
    }

  } catch (error) {
    console.error('❌ Test failed with error:', error.message);
  }
}

// Run the test
testSearchAutocomplete().then(() => {
  console.log('\n🏁 Test completed');
  process.exit(0);
}).catch((error) => {
  console.error('💥 Test crashed:', error);
  process.exit(1);
});
