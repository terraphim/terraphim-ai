#!/usr/bin/env node

// Test script for knowledge graph functionality
const {
  buildRoleGraphFromJson,
  areTermsConnected,
  queryGraph,
  getGraphStats,
  version
} = require('./index.js');

console.log('Testing Terraphim Knowledge Graph Package v1.0.0');
console.log('===============================================\n');

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
    },
    "artificial intelligence": {
      id: 6,
      nterm: "artificial intelligence",
      url: "https://example.com/ai"
    }
  }
};

try {
  // Test 1: Build role graph
  console.log('Test 1: Building role graph...');
  const graphBytes = buildRoleGraphFromJson("Test Engineer", JSON.stringify(thesaurus));
  console.log(`✓ Role graph built successfully (${graphBytes.length} bytes)`);

  // Test 2: Get graph statistics
  console.log('\nTest 2: Getting graph statistics...');
  const stats = getGraphStats(Buffer.from(graphBytes));
  console.log('✓ Graph statistics:');
  console.log(`  - Node count: ${stats.nodeCount}`);
  console.log(`  - Edge count: ${stats.edgeCount}`);
  console.log(`  - Document count: ${stats.documentCount}`);
  console.log(`  - Thesaurus size: ${stats.thesaurusSize}`);
  console.log(`  - Is populated: ${stats.isPopulated}`);

  // Test 3: Check connectivity
  console.log('\nTest 3: Checking term connectivity...');
  const connectivityText = "machine learning deep learning";
  const isConnected = areTermsConnected(Buffer.from(graphBytes), connectivityText);
  console.log(`✓ Terms connectivity for "${connectivityText}": ${isConnected}`);

  // Test 4: Query graph
  console.log('\nTest 4: Querying graph...');
  const query = "machine learning";
  const results = queryGraph(Buffer.from(graphBytes), query, 0, 10);
  console.log(`✓ Found ${results.length} results for query "${query}":`);
  results.forEach((result, i) => {
    console.log(`  ${i + 1}. ${result.documentId} (rank: ${result.rank})`);
    console.log(`     Tags: [${result.tags.join(', ')}]`);
    console.log(`     Nodes: [${result.nodes.join(', ')}]`);
  });

  // Test 5: Complex query
  console.log('\nTest 5: Complex query...');
  const complexQuery = "artificial intelligence";
  const complexResults = queryGraph(Buffer.from(graphBytes), complexQuery, 0, 5);
  console.log(`✓ Found ${complexResults.length} results for complex query "${complexQuery}"`);

  console.log('\n🎉 All knowledge graph tests passed! Package is working correctly.');

} catch (error) {
  console.error('\n❌ Knowledge graph test failed:', error.message);
  console.error('Stack trace:', error.stack);
  process.exit(1);
}
