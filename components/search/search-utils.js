/**
 * @fileoverview Search Utilities - Pure JavaScript search query parsing and formatting
 * Ported from desktop/src/lib/Search/searchUtils.ts
 * Handles AND/OR operator parsing, search query building, and term extraction
 */

/**
 * @typedef {Object} ParsedSearchInput
 * @property {boolean} hasOperator - Whether the input contains logical operators
 * @property {'AND'|'OR'|null} operator - The logical operator found (AND or OR)
 * @property {string[]} terms - Array of search terms extracted from input
 * @property {string} originalQuery - Original input string
 */

/**
 * Parse search input text to extract terms and operators
 * Supports both capitalized (AND, OR) and lowercase (and, or) operators
 * Handles mixed operators by using the first one found
 *
 * @param {string} inputText - Raw search input text
 * @returns {ParsedSearchInput} Parsed search structure
 *
 * @example
 * parseSearchInput('rust async')
 * // { hasOperator: false, operator: null, terms: ['rust async'], originalQuery: 'rust async' }
 *
 * parseSearchInput('rust AND async')
 * // { hasOperator: true, operator: 'AND', terms: ['rust', 'async'], originalQuery: 'rust AND async' }
 *
 * parseSearchInput('tokio OR actix')
 * // { hasOperator: true, operator: 'OR', terms: ['tokio', 'actix'], originalQuery: 'tokio OR actix' }
 */
export function parseSearchInput(inputText) {
  const trimmedInput = inputText.trim();

  // Handle empty or whitespace-only input
  if (!trimmedInput) {
    return {
      hasOperator: false,
      operator: null,
      terms: [inputText], // Return original input to preserve whitespace
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
      .filter((part, index) => index % 2 === 0) // Take only non-operator parts
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
      .filter((part, index) => index % 2 === 0) // Take only non-operator parts
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
      // Use AND as the operator, split only on AND
      const terms = trimmedInput.split(capitalizedAndRegex)
        .filter((part, index) => index % 2 === 0) // Take only non-operator parts
        .map(term => term.trim())
        .filter(term => term.length > 0);

      return {
        hasOperator: true,
        operator: 'AND',
        terms,
        originalQuery: inputText,
      };
    } else if (orIndex !== -1) {
      // Use OR as the operator, split only on OR
      const terms = trimmedInput.split(capitalizedOrRegex)
        .filter((part, index) => index % 2 === 0) // Take only non-operator parts
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
      .filter((part, index) => index % 2 === 0) // Take only non-operator parts
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
      .filter((part, index) => index % 2 === 0) // Take only non-operator parts
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
      // Use AND as the operator, split only on and
      const terms = trimmedInput.split(lowercaseAndRegex)
        .filter((part, index) => index % 2 === 0) // Take only non-operator parts
        .map(term => term.trim())
        .filter(term => term.length > 0);

      return {
        hasOperator: true,
        operator: 'AND',
        terms,
        originalQuery: inputText,
      };
    } else if (orIndex !== -1) {
      // Use OR as the operator, split only on or
      const terms = trimmedInput.split(lowercaseOrRegex)
        .filter((part, index) => index % 2 === 0) // Take only non-operator parts
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

/**
 * @typedef {Object} SearchQuery
 * @property {string} search_term - Primary search term
 * @property {string[]} [search_terms] - All search terms (for multi-term queries)
 * @property {'and'|'or'} [operator] - Logical operator (lowercase for API)
 * @property {number} [skip] - Number of results to skip (pagination)
 * @property {number} [limit] - Maximum number of results
 * @property {string|null} [role] - Role name for context-specific search
 */

/**
 * Build a search query object from parsed input
 * Converts parsed structure into API-compatible search query format
 *
 * @param {ParsedSearchInput} parsed - Parsed search input
 * @param {string} [role] - Optional role name for role-specific search
 * @returns {SearchQuery} Search query object for API
 *
 * @example
 * const parsed = parseSearchInput('rust AND async');
 * const query = buildSearchQuery(parsed, 'engineer');
 * // {
 * //   search_term: 'rust',
 * //   search_terms: ['rust', 'async'],
 * //   operator: 'and',
 * //   skip: 0,
 * //   limit: 50,
 * //   role: 'engineer'
 * // }
 */
export function buildSearchQuery(parsed, role) {
  if (parsed.hasOperator && parsed.terms.length > 1) {
    // Filter out empty terms
    const validTerms = parsed.terms.filter(term => term.trim().length > 0);

    if (validTerms.length > 1) {
      return {
        search_term: validTerms[0],
        search_terms: validTerms,
        operator: parsed.operator.toLowerCase(),
        skip: 0,
        limit: 50,
        role: role || null,
      };
    }
  }

  // Single term or no valid multi-term query
  const singleTerm = parsed.terms[0]?.trim() || '';
  return {
    search_term: singleTerm,
    skip: 0,
    limit: 50,
    role: role || null,
  };
}

/**
 * Format search terms for display
 * Converts array of terms and operator into readable string
 *
 * @param {string[]} terms - Array of search terms
 * @param {'AND'|'OR'} operator - Logical operator
 * @returns {string} Formatted search string
 *
 * @example
 * formatSearchTerms(['rust', 'async'], 'AND')
 * // 'rust AND async'
 */
export function formatSearchTerms(terms, operator) {
  if (!terms || terms.length === 0) return '';
  if (terms.length === 1) return terms[0];
  return terms.join(` ${operator} `);
}

/**
 * Validate search query
 * Checks if a search query is valid and ready for submission
 *
 * @param {string} query - Search query string
 * @returns {boolean} True if query is valid
 *
 * @example
 * isValidSearchQuery('rust')  // true
 * isValidSearchQuery('   ')   // false
 * isValidSearchQuery('')      // false
 */
export function isValidSearchQuery(query) {
  return typeof query === 'string' && query.trim().length > 0;
}

/**
 * Extract the current term being typed
 * Useful for autocomplete - gets the last incomplete term from input
 *
 * @param {string} input - Current search input
 * @param {number} [cursorPosition] - Optional cursor position
 * @returns {string} Current term being typed
 *
 * @example
 * getCurrentTerm('rust AND asy')
 * // 'asy'
 *
 * getCurrentTerm('tokio')
 * // 'tokio'
 */
export function getCurrentTerm(input, cursorPosition) {
  const textBeforeCursor = cursorPosition !== undefined
    ? input.slice(0, cursorPosition)
    : input;

  const words = textBeforeCursor.split(/\s+/);
  const lastWord = words[words.length - 1];

  // Check if last word is an operator
  if (lastWord.toLowerCase() === 'and' || lastWord.toLowerCase() === 'or') {
    return '';
  }

  return lastWord;
}

/**
 * Check if input ends with an operator
 * Used to determine if user should be prompted for next term
 *
 * @param {string} input - Search input string
 * @returns {boolean} True if input ends with AND or OR
 *
 * @example
 * endsWithOperator('rust AND ')  // true
 * endsWithOperator('rust AND')   // true
 * endsWithOperator('rust')       // false
 */
export function endsWithOperator(input) {
  const trimmed = input.trim();
  const lastWord = trimmed.split(/\s+/).pop().toUpperCase();
  return lastWord === 'AND' || lastWord === 'OR';
}

/**
 * Suggest operators for current query
 * Returns appropriate operators based on query state
 *
 * @param {string} input - Current search input
 * @returns {string[]} Array of suggested operators
 *
 * @example
 * suggestOperators('rust')
 * // ['AND', 'OR']
 *
 * suggestOperators('rust AND async')
 * // []
 */
export function suggestOperators(input) {
  const parsed = parseSearchInput(input);

  // If already has operator, don't suggest more
  if (parsed.hasOperator) return [];

  // If input is empty or too short, don't suggest
  if (input.trim().length < 2) return [];

  // Suggest both operators
  return ['AND', 'OR'];
}
