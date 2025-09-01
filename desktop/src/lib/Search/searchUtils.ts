export interface ParsedSearchInput {
  hasOperator: boolean;
  operator: 'AND' | 'OR' | null;
  terms: string[];
  originalQuery: string;
}

export function parseSearchInput(inputText: string): ParsedSearchInput {
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

  // Check for AND operator (case insensitive, word boundaries)
  const andRegex = /\b(and)\b/i;
  const orRegex = /\b(or)\b/i;

  const hasAnd = andRegex.test(trimmedInput);
  const hasOr = orRegex.test(trimmedInput);

  // Handle AND operator
  if (hasAnd && !hasOr) {
    const parts = trimmedInput.split(andRegex);
    const terms = [];
    for (let i = 0; i < parts.length; i += 2) { // Skip the 'and' parts
      const term = parts[i]?.trim() || '';
      terms.push(term);
    }
    return {
      hasOperator: true,
      operator: 'AND',
      terms,
      originalQuery: inputText,
    };
  }

  // Handle OR operator
  if (hasOr && !hasAnd) {
    const parts = trimmedInput.split(orRegex);
    const terms = [];
    for (let i = 0; i < parts.length; i += 2) { // Skip the 'or' parts
      const term = parts[i]?.trim() || '';
      terms.push(term);
    }
    return {
      hasOperator: true,
      operator: 'OR',
      terms,
      originalQuery: inputText,
    };
  }

  // Handle mixed operators - use the first one found
  if (hasAnd && hasOr) {
    const andIndex = trimmedInput.toLowerCase().indexOf('and');
    const orIndex = trimmedInput.toLowerCase().indexOf('or');

    if (andIndex < orIndex) {
      const parts = trimmedInput.split(andRegex);
      const terms = [];
      for (let i = 0; i < parts.length; i += 2) {
        const term = parts[i]?.trim() || '';
        terms.push(term);
      }
      return {
        hasOperator: true,
        operator: 'AND',
        terms,
        originalQuery: inputText,
      };
    } else {
      const parts = trimmedInput.split(orRegex);
      const terms = [];
      for (let i = 0; i < parts.length; i += 2) {
        const term = parts[i]?.trim() || '';
        terms.push(term);
      }
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

export interface SearchQuery {
  search_term: string;
  search_terms?: string[];
  operator?: 'and' | 'or' | null;
  skip?: number;
  limit?: number;
  role?: string | null;
}

export function buildSearchQuery(parsed: ParsedSearchInput, role?: string): SearchQuery {
  if (parsed.hasOperator && parsed.terms.length > 1) {
    return {
      search_term: parsed.terms[0],
      search_terms: parsed.terms,
      operator: parsed.operator?.toLowerCase() as 'and' | 'or',
      skip: 0,
      limit: 50,
      role: role || null,
    };
  }

  return {
    search_term: parsed.terms[0] || '',
    search_terms: undefined,
    operator: undefined,
    skip: 0,
    limit: 50,
    role: role || null,
  };
}
