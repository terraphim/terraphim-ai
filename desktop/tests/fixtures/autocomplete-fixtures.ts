/**
 * Test fixtures and mock data for Novel Editor Autocomplete tests
 * These fixtures provide consistent test data across different test scenarios
 */

import { AutocompleteSuggestion, AutocompleteTestConfig } from '../e2e/helpers/autocomplete-helpers';

/**
 * Expected suggestions for common test queries
 */
export const EXPECTED_SUGGESTIONS: Record<string, AutocompleteSuggestion[]> = {
  'terraphim': [
    { text: 'terraphim-graph', snippet: 'Knowledge graph implementation', score: 0.95 },
    { text: 'terraphim-automata', snippet: 'Text matching and autocomplete', score: 0.90 },
    { text: 'terraphim-service', snippet: 'Main service layer with AI integration', score: 0.88 },
    { text: 'terraphim-types', snippet: 'Shared type definitions', score: 0.85 },
    { text: 'terraphim-config', snippet: 'Configuration management', score: 0.82 },
  ],

  'graph': [
    { text: 'knowledge-graph', snippet: 'Graph-based knowledge representation', score: 0.92 },
    { text: 'terraphim-graph', snippet: 'Knowledge graph implementation', score: 0.90 },
    { text: 'role-graph', snippet: 'Role-based graph traversal', score: 0.87 },
    { text: 'rolegraph', snippet: 'Per-role knowledge graph', score: 0.85 },
  ],

  'search': [
    { text: 'search-functionality', snippet: 'Document search capabilities', score: 0.93 },
    { text: 'role-based-search', snippet: 'Personalized search by role', score: 0.90 },
    { text: 'fuzzy-search', snippet: 'Approximate string matching', score: 0.87 },
    { text: 'semantic-search', snippet: 'Meaning-based search', score: 0.85 },
  ],

  'role': [
    { text: 'role-based-search', snippet: 'Personalized search by role', score: 0.94 },
    { text: 'role-graph', snippet: 'Role-based graph traversal', score: 0.91 },
    { text: 'rolegraph', snippet: 'Per-role knowledge graph', score: 0.88 },
    { text: 'role-config', snippet: 'Role-specific configuration', score: 0.85 },
  ],

  'auto': [
    { text: 'automata', snippet: 'Finite state automata for text matching', score: 0.95 },
    { text: 'autocomplete', snippet: 'Suggestion system', score: 0.92 },
    { text: 'automatic', snippet: 'Automated processes', score: 0.85 },
  ],
};

/**
 * Mock suggestions for offline testing
 */
export const MOCK_SUGGESTIONS: AutocompleteSuggestion[] = [
  { text: 'terraphim-graph', snippet: 'Knowledge graph implementation', score: 0.95 },
  { text: 'terraphim-automata', snippet: 'Text matching and autocomplete', score: 0.90 },
  { text: 'terraphim-service', snippet: 'Main service layer', score: 0.88 },
  { text: 'knowledge-graph', snippet: 'Graph-based knowledge representation', score: 0.85 },
  { text: 'role-based-search', snippet: 'Personalized search by role', score: 0.82 },
  { text: 'haystack-integration', snippet: 'Data source integration', score: 0.80 },
  { text: 'atomic-server', snippet: 'Atomic Data protocol support', score: 0.78 },
  { text: 'mcp-protocol', snippet: 'Model Context Protocol', score: 0.75 },
];

/**
 * Test configurations for different scenarios
 */
export const TEST_CONFIGS: Record<string, AutocompleteTestConfig> = {
  default: {
    trigger: '/',
    minLength: 1,
    maxSuggestions: 8,
    debounceDelay: 300,
    mcpServerPort: 8001,
  },

  fast: {
    trigger: '/',
    minLength: 1,
    maxSuggestions: 5,
    debounceDelay: 100,
    mcpServerPort: 8001,
  },

  minimal: {
    trigger: '/',
    minLength: 2,
    maxSuggestions: 3,
    debounceDelay: 500,
    mcpServerPort: 8001,
  },

  comprehensive: {
    trigger: '/',
    minLength: 1,
    maxSuggestions: 12,
    debounceDelay: 200,
    mcpServerPort: 8001,
  },

  alternative_trigger: {
    trigger: '@',
    minLength: 1,
    maxSuggestions: 8,
    debounceDelay: 300,
    mcpServerPort: 8001,
  },
};

/**
 * Test queries for different scenarios
 */
export const TEST_QUERIES = {
  // Basic functionality tests
  basic: ['terraphim', 'graph', 'search', 'role'],

  // Edge case tests
  edgeCases: [
    '', // Empty query
    'a', // Single character
    'xyz', // No matches expected
    'terraphim-very-long-term-that-should-not-exist', // Long query
    '123', // Numeric
    '!@#', // Special characters
  ],

  // Performance tests
  performance: ['te', 'ter', 'terra', 'terrap', 'terraph', 'terraphi', 'terraphim'],

  // Common programming terms
  programming: [
    'function', 'class', 'interface', 'type', 'async', 'await',
    'promise', 'callback', 'event', 'handler', 'service', 'component'
  ],

  // Domain-specific terms
  domainSpecific: [
    'knowledge', 'semantic', 'ontology', 'taxonomy', 'classification',
    'indexing', 'ranking', 'scoring', 'relevance', 'precision', 'recall'
  ],
};

/**
 * Expected error scenarios and messages
 */
export const ERROR_SCENARIOS = {
  serverUnavailable: {
    query: 'terraphim',
    expectedError: 'MCP server not responding',
    expectedFallback: 'Using mock autocomplete',
    expectedSuggestions: 0,
  },

  networkTimeout: {
    query: 'graph',
    expectedError: 'Request timeout',
    expectedFallback: 'Connection failed',
    expectedSuggestions: 0,
  },

  invalidResponse: {
    query: 'invalid',
    expectedError: 'Invalid response format',
    expectedFallback: 'Parsing error',
    expectedSuggestions: 0,
  },

  emptyResults: {
    query: 'nonexistent-term-xyz123',
    expectedError: null,
    expectedFallback: null,
    expectedSuggestions: 0,
  },
};

/**
 * Visual regression test scenarios
 */
export const VISUAL_TEST_SCENARIOS = [
  {
    name: 'basic-dropdown',
    query: 'terraphim',
    description: 'Basic autocomplete dropdown appearance',
    expectedSuggestions: 5,
  },
  {
    name: 'long-suggestions',
    query: 'knowledge',
    description: 'Dropdown with long suggestion text',
    expectedSuggestions: 4,
  },
  {
    name: 'many-suggestions',
    query: 'te',
    description: 'Dropdown with maximum suggestions',
    expectedSuggestions: 8,
  },
  {
    name: 'no-suggestions',
    query: 'xyz123',
    description: 'Empty state when no suggestions found',
    expectedSuggestions: 0,
  },
];

/**
 * Keyboard navigation test scenarios
 */
export const KEYBOARD_TEST_SCENARIOS = [
  {
    name: 'arrow-down-navigation',
    steps: [
      { action: 'type', data: '/terraphim' },
      { action: 'wait', data: 500 },
      { action: 'keypress', data: 'ArrowDown' },
      { action: 'keypress', data: 'ArrowDown' },
      { action: 'verify-selection', data: 1 }, // Second item selected
    ]
  },
  {
    name: 'arrow-up-navigation',
    steps: [
      { action: 'type', data: '/graph' },
      { action: 'wait', data: 500 },
      { action: 'keypress', data: 'ArrowDown' },
      { action: 'keypress', data: 'ArrowDown' },
      { action: 'keypress', data: 'ArrowUp' },
      { action: 'verify-selection', data: 0 }, // Back to first item
    ]
  },
  {
    name: 'tab-selection',
    steps: [
      { action: 'type', data: '/search' },
      { action: 'wait', data: 500 },
      { action: 'keypress', data: 'ArrowDown' },
      { action: 'keypress', data: 'Tab' },
      { action: 'verify-insertion', data: 'search-functionality' },
    ]
  },
  {
    name: 'enter-selection',
    steps: [
      { action: 'type', data: '/role' },
      { action: 'wait', data: 500 },
      { action: 'keypress', data: 'Enter' },
      { action: 'verify-insertion', data: 'role-based-search' },
    ]
  },
  {
    name: 'escape-cancellation',
    steps: [
      { action: 'type', data: '/auto' },
      { action: 'wait', data: 500 },
      { action: 'keypress', data: 'Escape' },
      { action: 'verify-dropdown-closed', data: true },
    ]
  },
];

/**
 * Performance benchmarks
 */
export const PERFORMANCE_BENCHMARKS = {
  responseTime: {
    excellent: 100, // ms
    good: 300,
    acceptable: 500,
    poor: 1000,
  },

  debounceEffectiveness: {
    minDelay: 200, // ms
    maxDelay: 600,
    optimalDelay: 300,
  },

  suggestionCounts: {
    minimal: 3,
    standard: 8,
    maximum: 12,
  },
};

/**
 * Mock MCP server responses for testing
 */
export const MOCK_MCP_RESPONSES = {
  successfulAutocomplete: {
    jsonrpc: '2.0',
    id: 1,
    result: {
      content: [
        { type: 'text', text: 'Found 5 suggestions' },
        { type: 'text', text: '• terraphim-graph' },
        { type: 'text', text: '• terraphim-automata' },
        { type: 'text', text: '• terraphim-service' },
        { type: 'text', text: '• knowledge-graph' },
        { type: 'text', text: '• role-based-search' },
      ]
    }
  },

  autocompleteWithSnippets: {
    jsonrpc: '2.0',
    id: 1,
    result: {
      content: [
        { type: 'text', text: 'Found 3 suggestions with snippets' },
        { type: 'text', text: 'terraphim-graph — Knowledge graph implementation' },
        { type: 'text', text: 'terraphim-automata — Text matching and autocomplete' },
        { type: 'text', text: 'knowledge-graph — Graph-based knowledge representation' },
      ]
    }
  },

  emptyResponse: {
    jsonrpc: '2.0',
    id: 1,
    result: {
      content: [
        { type: 'text', text: 'Found 0 suggestions' }
      ]
    }
  },

  errorResponse: {
    jsonrpc: '2.0',
    id: 1,
    error: {
      code: -1,
      message: 'Internal server error',
      data: 'Failed to process autocomplete request'
    }
  },

  toolsList: {
    jsonrpc: '2.0',
    id: 1,
    result: {
      tools: [
        {
          name: 'autocomplete_terms',
          description: 'Autocomplete terms using FST prefix + fuzzy fallback'
        },
        {
          name: 'autocomplete_with_snippets',
          description: 'Autocomplete and return short snippets from matching documents'
        }
      ]
    }
  },
};

/**
 * Test data for different user roles
 */
export const ROLE_SPECIFIC_DATA = {
  'Terraphim Engineer': {
    expectedTerms: ['terraphim', 'graph', 'service', 'automata', 'config'],
    commonQueries: ['terraphim', 'knowledge-graph', 'role-based'],
    specificSuggestions: ['terraphim-graph', 'terraphim-automata', 'terraphim-service']
  },

  'Default': {
    expectedTerms: ['knowledge', 'search', 'graph', 'document'],
    commonQueries: ['knowledge', 'search', 'document'],
    specificSuggestions: ['knowledge-graph', 'document-search', 'semantic-search']
  },

  'Research Assistant': {
    expectedTerms: ['research', 'analysis', 'data', 'insight'],
    commonQueries: ['research', 'analysis', 'methodology'],
    specificSuggestions: ['research-methodology', 'data-analysis', 'insight-extraction']
  },
};

/**
 * Accessibility test cases
 */
export const ACCESSIBILITY_TESTS = [
  {
    name: 'keyboard-only-navigation',
    description: 'Test complete flow using only keyboard',
    requirements: ['tab-navigation', 'arrow-keys', 'enter-selection', 'escape-cancel']
  },
  {
    name: 'screen-reader-compatibility',
    description: 'Test screen reader announcements',
    requirements: ['aria-labels', 'role-attributes', 'live-regions']
  },
  {
    name: 'high-contrast-mode',
    description: 'Test visibility in high contrast mode',
    requirements: ['contrast-ratios', 'border-visibility', 'focus-indicators']
  },
];

/**
 * Helper function to get expected suggestions for a query
 */
export function getExpectedSuggestions(query: string): AutocompleteSuggestion[] {
  const lowerQuery = query.toLowerCase();

  // Check exact matches first
  if (EXPECTED_SUGGESTIONS[lowerQuery]) {
    return EXPECTED_SUGGESTIONS[lowerQuery];
  }

  // Check partial matches
  for (const [key, suggestions] of Object.entries(EXPECTED_SUGGESTIONS)) {
    if (key.includes(lowerQuery) || lowerQuery.includes(key)) {
      return suggestions;
    }
  }

  // Return empty array for no matches
  return [];
}

/**
 * Helper function to validate suggestion structure
 */
export function validateSuggestionStructure(suggestion: any): boolean {
  return (
    typeof suggestion === 'object' &&
    typeof suggestion.text === 'string' &&
    suggestion.text.length > 0 &&
    (suggestion.snippet === undefined || typeof suggestion.snippet === 'string') &&
    (suggestion.score === undefined || (typeof suggestion.score === 'number' && suggestion.score >= 0 && suggestion.score <= 1))
  );
}

/**
 * Helper function to create test scenario variations
 */
export function createTestScenarios(baseQuery: string): Array<{query: string, description: string}> {
  return [
    { query: baseQuery, description: `Complete term: ${baseQuery}` },
    { query: baseQuery.slice(0, -1), description: `Partial term: ${baseQuery.slice(0, -1)}` },
    { query: baseQuery.slice(0, 2), description: `Short prefix: ${baseQuery.slice(0, 2)}` },
    { query: baseQuery.toUpperCase(), description: `Uppercase: ${baseQuery.toUpperCase()}` },
    { query: baseQuery.charAt(0).toUpperCase() + baseQuery.slice(1), description: `Title case: ${baseQuery.charAt(0).toUpperCase() + baseQuery.slice(1)}` },
  ];
}