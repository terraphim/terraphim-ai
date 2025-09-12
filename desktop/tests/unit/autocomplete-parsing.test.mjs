// Test autocomplete suggestion application and parsing
import { parseSearchInput, buildSearchQuery } from '../../src/lib/Search/searchUtils.ts';

// Simulate the applySuggestion logic from Search.svelte
function applySuggestion(input, suggestion) {
  if (suggestion === 'AND' || suggestion === 'OR') {
    return input + ` ${suggestion} `;
  } else {
    // It's a term suggestion - replace the current partial term
    const words = input.trim().split(/\s+/);
    const lastWord = words[words.length - 1];
    
    // If the last word is a partial match for the suggestion, replace it
    if (suggestion.toLowerCase().startsWith(lastWord.toLowerCase())) {
      // Replace the last word with the full suggestion
      words[words.length - 1] = suggestion;
      return words.join(' ');
    } else {
      // If no partial match, add as new term
      return input + ` ${suggestion}`;
    }
  }
}

// Test cases for autocomplete suggestion application
const testCases = [
  {
    name: "Replace partial term with full suggestion",
    input: "graph AND per",
    suggestion: "performance problems",
    expectedOutput: "graph AND performance problems",
    expectedTerms: ["graph", "performance problems"]
  },
  {
    name: "Replace partial term with exact match",
    input: "graph AND per",
    suggestion: "performance",
    expectedOutput: "graph AND performance",
    expectedTerms: ["graph", "performance"]
  },
  {
    name: "Add new term when no partial match",
    input: "graph AND performance",
    suggestion: "issues",
    expectedOutput: "graph AND performance issues",
    expectedTerms: ["graph", "performance issues"]
  },
  {
    name: "Handle operator suggestion",
    input: "graph performance",
    suggestion: "AND",
    expectedOutput: "graph performance AND ",
    expectedTerms: ["graph performance"]
  },
  {
    name: "Replace partial term in complex query",
    input: "graph AND per AND issues",
    suggestion: "performance",
    expectedOutput: "graph AND performance AND issues",
    expectedTerms: ["graph", "performance", "issues"]
  },
  {
    name: "Replace partial term with multi-word suggestion",
    input: "graph AND per",
    suggestion: "performance optimization",
    expectedOutput: "graph AND performance optimization",
    expectedTerms: ["graph", "performance optimization"]
  }
];

console.log("Testing autocomplete suggestion application...\n");

let allPassed = true;

testCases.forEach((testCase, index) => {
  console.log(`Test ${index + 1}: ${testCase.name}`);
  console.log(`  Input: "${testCase.input}"`);
  console.log(`  Suggestion: "${testCase.suggestion}"`);
  
  const result = applySuggestion(testCase.input, testCase.suggestion);
  console.log(`  Result: "${result}"`);
  console.log(`  Expected: "${testCase.expectedOutput}"`);
  
  const outputMatch = result === testCase.expectedOutput;
  console.log(`  ✅ Output match: ${outputMatch}`);
  
  if (outputMatch) {
    // Test parsing of the result
    const parsed = parseSearchInput(result);
    console.log(`  Parsed terms: [${parsed.terms.join(', ')}]`);
    console.log(`  Expected terms: [${testCase.expectedTerms.join(', ')}]`);
    
    const termsMatch = JSON.stringify(parsed.terms) === JSON.stringify(testCase.expectedTerms);
    console.log(`  ✅ Terms match: ${termsMatch}`);
    
    if (!termsMatch) {
      allPassed = false;
    }
  } else {
    allPassed = false;
  }
  
  console.log('');
});

console.log(`Overall result: ${allPassed ? 'ALL TESTS PASSED' : 'SOME TESTS FAILED'}`);

// Test the specific case mentioned by the user
console.log("\nTesting specific user case: 'graph AND per' + 'performance problems'");
const userInput = "graph AND per";
const userSuggestion = "performance problems";
const userResult = applySuggestion(userInput, userSuggestion);
const userParsed = parseSearchInput(userResult);

console.log(`Input: "${userInput}"`);
console.log(`Suggestion: "${userSuggestion}"`);
console.log(`Result: "${userResult}"`);
console.log(`Parsed terms: [${userParsed.terms.join(', ')}]`);
console.log(`Expected: ["graph", "performance problems"]`);

const userTestPassed = JSON.stringify(userParsed.terms) === JSON.stringify(["graph", "performance problems"]);
console.log(`✅ User case test: ${userTestPassed ? 'PASS' : 'FAIL'}`);
