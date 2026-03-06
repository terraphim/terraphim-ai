// Test operator parsing behavior
import { readFileSync } from 'fs';
import { fileURLToPath } from 'url';
import { dirname, join } from 'path';

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);

// Simple test of the parsing logic
function parseSearchInput(inputText) {
  const trimmedInput = inputText.trim();

  if (!trimmedInput) {
    return {
      hasOperator: false,
      operator: null,
      terms: [inputText],
      originalQuery: inputText,
    };
  }

  // First, check for capitalized operators (priority)
  const capitalizedAndRegex = /\b(AND)\b/;
  const capitalizedOrRegex = /\b(OR)\b/;

  // Then check for lowercase operators (fallback)
  const lowercaseAndRegex = /\b(and)\b/i;
  const lowercaseOrRegex = /\b(or)\b/i;

  const hasCapitalizedAnd = capitalizedAndRegex.test(trimmedInput);
  const hasCapitalizedOr = capitalizedOrRegex.test(trimmedInput);
  const hasLowercaseAnd = lowercaseAndRegex.test(trimmedInput) && !hasCapitalizedAnd;
  const hasLowercaseOr = lowercaseOrRegex.test(trimmedInput) && !hasCapitalizedOr;

  // Handle capitalized AND operator (highest priority)
  if (hasCapitalizedAnd && !hasCapitalizedOr) {
    const terms = trimmedInput.split(capitalizedAndRegex)
      .filter((part, index) => index % 2 === 0)
      .map(term => term.trim())
      .filter(term => term.length > 0);

    return {
      hasOperator: true,
      operator: 'AND',
      terms,
      originalQuery: inputText,
    };
  }

  // Handle capitalized OR operator (second priority)
  if (hasCapitalizedOr && !hasCapitalizedAnd) {
    const terms = trimmedInput.split(capitalizedOrRegex)
      .filter((part, index) => index % 2 === 0)
      .map(term => term.trim())
      .filter(term => term.length > 0);

    return {
      hasOperator: true,
      operator: 'OR',
      terms,
      originalQuery: inputText,
    };
  }

  // Handle mixed capitalized operators - use the first one found
  if (hasCapitalizedAnd && hasCapitalizedOr) {
    const andIndex = trimmedInput.indexOf(' AND ');
    const orIndex = trimmedInput.indexOf(' OR ');

    if (andIndex !== -1 && (orIndex === -1 || andIndex < orIndex)) {
      // Use AND as the operator, but split on both AND and OR to get all terms
      const terms = trimmedInput.split(/\s+(?:AND|OR)\s+/i)
        .map(term => term.trim())
        .filter(term => term.length > 0);

      return {
        hasOperator: true,
        operator: 'AND',
        terms,
        originalQuery: inputText,
      };
    } else if (orIndex !== -1) {
      // Use OR as the operator, but split on both AND and OR to get all terms
      const terms = trimmedInput.split(/\s+(?:AND|OR)\s+/i)
        .map(term => term.trim())
        .filter(term => term.length > 0);

      return {
        hasOperator: true,
        operator: 'OR',
        terms,
        originalQuery: inputText,
      };
    }
  }

  // Fallback to lowercase operators if no capitalized ones found
  if (hasLowercaseAnd && !hasLowercaseOr) {
    const terms = trimmedInput.split(lowercaseAndRegex)
      .filter((part, index) => index % 2 === 0)
      .map(term => term.trim())
      .filter(term => term.length > 0);

    return {
      hasOperator: true,
      operator: 'AND',
      terms,
      originalQuery: inputText,
    };
  }

  // Handle lowercase OR operator
  if (hasLowercaseOr && !hasLowercaseAnd) {
    const terms = trimmedInput.split(lowercaseOrRegex)
      .filter((part, index) => index % 2 === 0)
      .map(term => term.trim())
      .filter(term => term.length > 0);

    return {
      hasOperator: true,
      operator: 'OR',
      terms,
      originalQuery: inputText,
    };
  }

  // Handle mixed lowercase operators - use the first one found
  if (hasLowercaseAnd && hasLowercaseOr) {
    const andIndex = trimmedInput.toLowerCase().indexOf(' and ');
    const orIndex = trimmedInput.toLowerCase().indexOf(' or ');

    if (andIndex !== -1 && (orIndex === -1 || andIndex < orIndex)) {
      // Use AND as the operator, but split on both and and or to get all terms
      const terms = trimmedInput.split(/\s+(?:and|or)\s+/i)
        .map(term => term.trim())
        .filter(term => term.length > 0);

      return {
        hasOperator: true,
        operator: 'AND',
        terms,
        originalQuery: inputText,
      };
    } else if (orIndex !== -1) {
      // Use OR as the operator, but split on both and and or to get all terms
      const terms = trimmedInput.split(/\s+(?:and|or)\s+/i)
        .map(term => term.trim())
        .filter(term => term.length > 0);

      return {
        hasOperator: true,
        operator: 'OR',
        terms,
        originalQuery: inputText,
      };
    }
  }

  // No operators found
  return {
    hasOperator: false,
    operator: null,
    terms: [trimmedInput],
    originalQuery: inputText,
  };
}

function buildSearchQuery(parsed, role) {
  if (parsed.hasOperator && parsed.terms.length > 1) {
    const validTerms = parsed.terms.filter(term => term.trim().length > 0);

    if (validTerms.length > 1) {
      return {
        search_term: validTerms[0],
        search_terms: validTerms,
        operator: parsed.operator?.toLowerCase(),
        skip: 0,
        limit: 50,
        role: role || null,
      };
    }
  }

  const singleTerm = parsed.terms[0]?.trim() || '';
  return {
    search_term: singleTerm,
    search_terms: undefined,
    operator: undefined,
    skip: 0,
    limit: 50,
    role: role || null,
  };
}

// Test cases
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

let allPassed = true;

testCases.forEach((testCase, index) => {
  console.log(`Test ${index + 1}: "${testCase.input}"`);

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

  if (!(operatorMatch && termsMatch)) {
    allPassed = false;
  }

  console.log('');
});

console.log(`\nOverall result: ${allPassed ? 'ALL TESTS PASSED' : 'SOME TESTS FAILED'}`);

// Test UI operator controls vs parsed operators
console.log("\nTesting UI operator controls vs parsed operators...\n");

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

  // Simulate the improved UI operator logic from Search.svelte
  const inputText = test.input.trim();
  const selectedOperator = test.uiOperator;

  if (selectedOperator !== 'none') {
    // First parse the input to remove any text operators and get clean terms
    const parsed = parseSearchInput(inputText);

    // If parsing found operators, use those terms; otherwise split on spaces
    const terms = parsed.hasOperator ? parsed.terms : inputText.split(/\s+/).filter(term => term.length > 0);

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
