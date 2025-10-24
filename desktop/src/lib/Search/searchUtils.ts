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
}
