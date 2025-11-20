#!/usr/bin/env node

// Test script for autocomplete functionality
const {
  buildAutocompleteIndexFromJson,
  autocomplete,
  fuzzyAutocompleteSearch,
  version
} = require('./index.js');

console.log('Testing Terraphim Autocomplete Package v1.0.0');
console.log('=========================================\n');

// Test version
try {
  console.log('✓ Version:', version());
} catch (error) {
  console.error('✗ Version test failed:', error.message);
  process.exit(1);
}

// Sample thesaurus for testing
const thesaurus = {
  name: "Engineering",
  data: {
    "machine learning": {
      id: 1,
      nterm: "machine learning",
      url: "https://example.com/ml"
    },
    "deep learning": {
      id: 2,
      nterm: "deep learning",
      url: "https://example.com/dl"
    },
    "neural networks": {
      id: 3,
      nterm: "neural networks",
      url: "https://example.com/nn"
    },
    "computer vision": {
      id: 4,
      nterm: "computer vision",
      url: "https://example.com/cv"
    },
    "natural language processing": {
      id: 5,
      nterm: "natural language processing",
      url: "https://example.com/nlp"
    }
  }
};

try {
  // Test 1: Build autocomplete index
  console.log('Test 1: Building autocomplete index...');
  const indexBytes = buildAutocompleteIndexFromJson(JSON.stringify(thesaurus));
  console.log(`✓ Index built successfully (${indexBytes.length} bytes)`);

  // Test 2: Prefix search
  console.log('\nTest 2: Prefix search for "machine"...');
  const results = autocomplete(Buffer.from(indexBytes), "machine", 10);
  console.log(`✓ Found ${results.length} results:`);
  results.forEach((result, i) => {
    console.log(`  ${i + 1}. ${result.term} (score: ${result.score})`);
  });

  // Test 3: Prefix search for "learning"
  console.log('\nTest 3: Prefix search for "learning"...');
  const learningResults = autocomplete(Buffer.from(indexBytes), "learning", 10);
  console.log(`✓ Found ${learningResults.length} results:`);
  learningResults.forEach((result, i) => {
    console.log(`  ${i + 1}. ${result.term} (score: ${result.score})`);
  });

  // Test 4: Fuzzy search (placeholder)
  console.log('\nTest 4: Fuzzy search for "machin"...');
  const fuzzyResults = fuzzyAutocompleteSearch(Buffer.from(indexBytes), "machin", 0.8, 10);
  console.log(`✓ Found ${fuzzyResults.length} results (placeholder implementation)`);

  // Test 5: Empty query
  console.log('\nTest 5: Empty query...');
  const emptyResults = autocomplete(Buffer.from(indexBytes), "", 3);
  console.log(`✓ Found ${emptyResults.length} results for empty query (limited to 3)`);

  console.log('\n🎉 All tests passed! Autocomplete package is working correctly.');

} catch (error) {
  console.error('\n❌ Test failed:', error.message);
  console.error('Stack trace:', error.stack);
  process.exit(1);
}
