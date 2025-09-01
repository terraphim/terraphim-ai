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
    const terms = trimmedInput.split(andRegex)
      .filter((part, index) => index % 2 === 0) // Take only non-operator parts
      .map(term => term.trim())
      .filter(term => term.length > 0); // Remove empty terms

    return {
      hasOperator: true,
      operator: 'AND',
      terms,
      originalQuery: inputText,
    };
  }

  // Handle OR operator
  if (hasOr && !hasAnd) {
    const terms = trimmedInput.split(orRegex)
      .filter((part, index) => index % 2 === 0) // Take only non-operator parts
      .map(term => term.trim())
      .filter(term => term.length > 0); // Remove empty terms

    return {
      hasOperator: true,
      operator: 'OR',
      terms,
      originalQuery: inputText,
    };
  }

  // Handle mixed operators - use the first one found
  if (hasAnd && hasOr) {
    const andIndex = trimmedInput.toLowerCase().indexOf(' and ');
    const orIndex = trimmedInput.toLowerCase().indexOf(' or ');

    if (andIndex !== -1 && (orIndex === -1 || andIndex < orIndex)) {
      const terms = trimmedInput.split(andRegex)
        .filter((part, index) => index % 2 === 0) // Take only non-operator parts
        .map(term => term.trim())
        .filter(term => term.length > 0); // Remove empty terms

      return {
        hasOperator: true,
        operator: 'AND',
        terms,
        originalQuery: inputText,
      };
    } else if (orIndex !== -1) {
      const terms = trimmedInput.split(orRegex)
        .filter((part, index) => index % 2 === 0) // Take only non-operator parts
        .map(term => term.trim())
        .filter(term => term.length > 0); // Remove empty terms

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
    // Filter out empty terms
    const validTerms = parsed.terms.filter(term => term.trim().length > 0);

    if (validTerms.length > 1) {
      return {
        search_term: validTerms[0],
        search_terms: validTerms,
        operator: parsed.operator?.toLowerCase() as 'and' | 'or',
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
    search_terms: undefined,
    operator: undefined,
    skip: 0,
    limit: 50,
    role: role || null,
  };
}
