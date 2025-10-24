export interface ParsedSearchInput {
	hasOperator: boolean;
	operator: 'AND' | 'OR' | null;
	terms: string[];
	originalQuery: string;
}

export function parseSearchInput(inputText: string): ParsedSearchInput {
<<<<<<< HEAD
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
      .map(term => term.trim());

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
      .map(term => term.trim());

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
        .map(term => term.trim());

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
        .map(term => term.trim());

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
      .map(term => term.trim());

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
      .map(term => term.trim());

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
        .map(term => term.trim());

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
        .map(term => term.trim());

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
=======
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
		const terms = trimmedInput
			.split(capitalizedAndRegex)
			.filter((_part, index) => index % 2 === 0) // Take only non-operator parts
			.map((term) => term.trim())
			.filter((term) => term.length > 0); // Remove empty terms

		return {
			hasOperator: true,
			operator: 'AND',
			terms,
			originalQuery: inputText,
		};
	}

	// Handle capitalized OR operator (second priority)
	if (hasCapitalizedOr && !hasCapitalizedAnd) {
		const terms = trimmedInput
			.split(capitalizedOrRegex)
			.filter((_part, index) => index % 2 === 0) // Take only non-operator parts
			.map((term) => term.trim())
			.filter((term) => term.length > 0); // Remove empty terms

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
			const terms = trimmedInput
				.split(/\s+(?:AND|OR)\s+/i)
				.map((term) => term.trim())
				.filter((term) => term.length > 0); // Remove empty terms

			return {
				hasOperator: true,
				operator: 'AND',
				terms,
				originalQuery: inputText,
			};
		} else if (orIndex !== -1) {
			// Use OR as the operator, but split on both AND and OR to get all terms
			const terms = trimmedInput
				.split(/\s+(?:AND|OR)\s+/i)
				.map((term) => term.trim())
				.filter((term) => term.length > 0); // Remove empty terms

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
		const terms = trimmedInput
			.split(lowercaseAndRegex)
			.filter((_part, index) => index % 2 === 0) // Take only non-operator parts
			.map((term) => term.trim())
			.filter((term) => term.length > 0); // Remove empty terms

		return {
			hasOperator: true,
			operator: 'AND',
			terms,
			originalQuery: inputText,
		};
	}

	// Handle lowercase OR operator
	if (hasLowercaseOr && !hasLowercaseAnd) {
		const terms = trimmedInput
			.split(lowercaseOrRegex)
			.filter((_part, index) => index % 2 === 0) // Take only non-operator parts
			.map((term) => term.trim())
			.filter((term) => term.length > 0); // Remove empty terms

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
			const terms = trimmedInput
				.split(/\s+(?:and|or)\s+/i)
				.map((term) => term.trim())
				.filter((term) => term.length > 0); // Remove empty terms

			return {
				hasOperator: true,
				operator: 'AND',
				terms,
				originalQuery: inputText,
			};
		} else if (orIndex !== -1) {
			// Use OR as the operator, but split on both and and or to get all terms
			const terms = trimmedInput
				.split(/\s+(?:and|or)\s+/i)
				.map((term) => term.trim())
				.filter((term) => term.length > 0); // Remove empty terms

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
>>>>>>> origin/main
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
		const validTerms = parsed.terms.filter((term) => term.trim().length > 0);

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

// Suggestion types for autocomplete
export interface AutocompleteSuggestion {
	term: string;
	type: 'term' | 'operator';
	description?: string;
}

/**
 * Get autocomplete suggestions with logical operator support
 */
<<<<<<< HEAD
export async function getSuggestions(query: string, role: string): Promise<AutocompleteSuggestion[]> {
  const parsed = parseSearchInput(query);
  
  // Extract the term to autocomplete
  let searchTerm = '';
  if (parsed.hasOperator && parsed.terms.length > 1) {
    // Get the last term for autocomplete
    searchTerm = parsed.terms[parsed.terms.length - 1].trim();
  } else {
    searchTerm = query.trim();
  }
  
  if (!searchTerm || searchTerm.length < 1) {
    return [];
  }
  
  // Try to use Tauri invoke first (for testing)
  try {
    const { invoke } = await import('@tauri-apps/api/tauri');
    const suggestions = await invoke('get_suggestions', { query: searchTerm, role });
    
    // Add operator suggestions for complete terms (length >= 3)
    if (query.length >= 3 && !query.includes(' AND ') && !query.includes(' OR ')) {
      const lastTerm = parsed.terms[parsed.terms.length - 1];
      if (lastTerm && lastTerm.length >= 3) {
        suggestions.push(
          { term: `${lastTerm} AND `, type: 'operator', description: `Search for documents containing ${lastTerm} AND another term` },
          { term: `${lastTerm} OR `, type: 'operator', description: `Search for documents containing ${lastTerm} OR another term` }
        );
      }
    }
    
    // Limit suggestions to 10 items
    return (suggestions as AutocompleteSuggestion[]).slice(0, 10);
  } catch (error) {
    // Fall back to fetch
  }
  
  // Call backend service for suggestions
  try {
    const response = await fetch(`/api/suggestions?q=${encodeURIComponent(searchTerm)}&role=${role}`);
    if (!response.ok) return [];
    return await response.json();
  } catch {
    // Fallback to mock suggestions for testing
    const suggestions: AutocompleteSuggestion[] = [];
    
    if (searchTerm.toLowerCase().includes('rust')) {
      suggestions.push(
        { term: 'rust', type: 'term', description: 'Rust programming language' },
        { term: 'rust-lang', type: 'term', description: 'Rust language documentation' }
      );
    }
    
    if (searchTerm.toLowerCase().includes('async')) {
      suggestions.push(
        { term: 'async', type: 'term', description: 'Asynchronous programming' },
        { term: 'await', type: 'term', description: 'Async/await syntax' }
      );
    }
    
    if (searchTerm.toLowerCase().includes('api')) {
      suggestions.push(
        { term: 'api', type: 'term', description: 'Application Programming Interface' }
      );
    }
    
    if (searchTerm.toLowerCase().includes('algorithm')) {
      suggestions.push(
        { term: 'algorithm', type: 'term', description: 'Algorithm implementation' }
      );
    }
    
    // Add operator suggestions for complete terms (length >= 3)
    if (query.length >= 3 && !query.includes(' AND ') && !query.includes(' OR ')) {
      const lastTerm = parsed.terms[parsed.terms.length - 1];
      if (lastTerm && lastTerm.length >= 3) {
        suggestions.push(
          { term: `${lastTerm} AND `, type: 'operator', description: `Search for documents containing ${lastTerm} AND another term` },
          { term: `${lastTerm} OR `, type: 'operator', description: `Search for documents containing ${lastTerm} OR another term` }
        );
      }
    }
    
    // Limit suggestions to 10 items
    return suggestions.slice(0, 10);
  }
=======
export async function getSuggestions(
	query: string,
	role: string
): Promise<AutocompleteSuggestion[]> {
	const { invoke } = await import('@tauri-apps/api/tauri');
	const suggestions: AutocompleteSuggestion[] = [];

	// Parse the input to see if it contains operators
	const parsed = parseSearchInput(query);

	try {
		// Call the Tauri backend for autocomplete suggestions
		const backendSuggestions = await invoke('get_autocomplete_suggestions', {
			query: query.trim(),
			role: role
		});

		// Handle undefined or null responses
		if (Array.isArray(backendSuggestions)) {
			suggestions.push(...backendSuggestions);
		}
	} catch (error) {
		// Fallback to empty suggestions if backend call fails
		console.warn('Failed to get autocomplete suggestions:', error);
	}

	// Add operator suggestions for complete terms (length >= 3)
	// Only add operator suggestions if there's no operator in the query
	if (query.length >= 3 && !parsed.hasOperator) {
		const lastTerm = parsed.terms[parsed.terms.length - 1];
		if (lastTerm && lastTerm.length >= 3) {
			suggestions.push(
				{
					term: `${lastTerm} AND `,
					type: 'operator',
					description: `Search for documents containing ${lastTerm} AND another term`,
				},
				{
					term: `${lastTerm} OR `,
					type: 'operator',
					description: `Search for documents containing ${lastTerm} OR another term`,
				}
			);
		}
	}

	// Limit suggestions to reasonable number for UX (max 10)
	return suggestions.slice(0, 10);
>>>>>>> origin/main
}
