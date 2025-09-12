// Test script to verify operator behavior
const { parseSearchInput, buildSearchQuery } = require('../../src/lib/Search/searchUtils.ts');

// Test cases for operator detection and behavior
const testCases = [
  {
    input: "graph AND performance",
    expectedOperator: "AND",
    expectedTerms: ["graph", "performance"]
  },
  {
    input: "graph OR performance", 
    expectedOperator: "OR",
    expectedTerms: ["graph", "performance"]
  },
  {
    input: "graph and performance",
    expectedOperator: "AND", 
    expectedTerms: ["graph", "performance"]
  },
  {
    input: "graph or performance",
    expectedOperator: "OR",
    expectedTerms: ["graph", "performance"]
  },
  {
    input: "graph AND performance AND issues",
    expectedOperator: "AND",
    expectedTerms: ["graph", "performance", "issues"]
  },
  {
    input: "graph OR performance OR issues", 
    expectedOperator: "OR",
    expectedTerms: ["graph", "performance", "issues"]
  },
  {
    input: "graph AND performance OR issues",
    expectedOperator: "AND", // Should use first operator found
    expectedTerms: ["graph", "performance", "issues"]
  },
  {
    input: "graph OR performance AND issues",
    expectedOperator: "OR", // Should use first operator found  
    expectedTerms: ["graph", "performance", "issues"]
  }
];

console.log("Testing operator detection and parsing...\n");

testCases.forEach((testCase, index) => {
  console.log(`Test ${index + 1}: "${testCase.input}"`);
  
  try {
    const parsed = parseSearchInput(testCase.input);
    console.log(`  Parsed operator: ${parsed.operator}`);
    console.log(`  Parsed terms: [${parsed.terms.join(', ')}]`);
    console.log(`  Has operator: ${parsed.hasOperator}`);
    
    const searchQuery = buildSearchQuery(parsed, "test-role");
    console.log(`  Search query operator: ${searchQuery.operator}`);
    console.log(`  Search query terms: [${searchQuery.search_terms?.join(', ') || 'N/A'}]`);
    
    // Check if results match expectations
    const operatorMatch = parsed.operator === testCase.expectedOperator;
    const termsMatch = JSON.stringify(parsed.terms) === JSON.stringify(testCase.expectedTerms);
    
    console.log(`  ✅ Operator match: ${operatorMatch}`);
    console.log(`  ✅ Terms match: ${termsMatch}`);
    console.log(`  ✅ Overall: ${operatorMatch && termsMatch ? 'PASS' : 'FAIL'}`);
    
  } catch (error) {
    console.log(`  ❌ Error: ${error.message}`);
  }
  
  console.log('');
});

// Test UI operator controls vs parsed operators
console.log("Testing UI operator controls vs parsed operators...\n");

const uiOperatorTests = [
  {
    input: "graph performance",
    uiOperator: "and",
    expectedBehavior: "Should use UI operator (AND) instead of parsing"
  },
  {
    input: "graph performance", 
    uiOperator: "or",
    expectedBehavior: "Should use UI operator (OR) instead of parsing"
  },
  {
    input: "graph AND performance",
    uiOperator: "or", 
    expectedBehavior: "Should use UI operator (OR) and override parsed AND"
  },
  {
    input: "graph OR performance",
    uiOperator: "and",
    expectedBehavior: "Should use UI operator (AND) and override parsed OR"
  }
];

uiOperatorTests.forEach((test, index) => {
  console.log(`UI Test ${index + 1}: "${test.input}" with UI operator "${test.uiOperator}"`);
  console.log(`  Expected: ${test.expectedBehavior}`);
  
  // Simulate the UI operator logic from Search.svelte
  const inputText = test.input.trim();
  const selectedOperator = test.uiOperator;
  
  if (selectedOperator !== 'none') {
    const terms = inputText.split(/\s+/).filter(term => term.length > 0);
    if (terms.length > 1) {
      const fakeParser = {
        hasOperator: true,
        operator: (selectedOperator === 'and' ? 'AND' : 'OR'),
        terms: terms,
        originalQuery: inputText,
      };
      const searchQuery = buildSearchQuery(fakeParser, "test-role");
      console.log(`  Result operator: ${searchQuery.operator}`);
      console.log(`  Result terms: [${searchQuery.search_terms?.join(', ') || 'N/A'}]`);
    }
  }
  
  console.log('');
});
