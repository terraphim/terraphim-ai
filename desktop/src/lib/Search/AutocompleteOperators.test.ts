import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

// Mock the Tauri API
const mockInvoke = vi.fn();
vi.mock('@tauri-apps/api/tauri', () => ({
	invoke: mockInvoke,
}));

// Mock the autocomplete service
vi.mock('../services/novelAutocompleteService', () => ({
	novelAutocompleteService: {
		getSuggestions: vi.fn(),
		setRole: vi.fn(),
		buildAutocompleteIndex: vi.fn(),
	},
}));

import { getSuggestions } from './searchUtils';

describe('Autocomplete with Logical Operators', () => {
	beforeEach(() => {
		vi.clearAllMocks();
	});

	afterEach(() => {
		vi.resetAllMocks();
	});

	describe('Operator Suggestions', () => {
		it('should suggest AND operator after completing a term', async () => {
			// Mock successful term suggestions
			mockInvoke.mockResolvedValueOnce([
				{ term: 'rust', type: 'term', description: 'Rust programming language' },
				{ term: 'rust-lang', type: 'term', description: 'Rust language documentation' },
			]);

			const suggestions = await getSuggestions('rust', 'Engineer');

			expect(suggestions).toEqual([
				{ term: 'rust', type: 'term', description: 'Rust programming language' },
				{ term: 'rust-lang', type: 'term', description: 'Rust language documentation' },
				{
					term: 'rust AND ',
					type: 'operator',
					description: 'Search for documents containing rust AND another term',
				},
				{
					term: 'rust OR ',
					type: 'operator',
					description: 'Search for documents containing rust OR another term',
				},
			]);
		});

		it('should not suggest operators for very short queries', async () => {
			mockInvoke.mockResolvedValueOnce([
				{ term: 'rust', type: 'term', description: 'Rust programming language' },
			]);

			const suggestions = await getSuggestions('r', 'Engineer');

			// Should only return term suggestions, no operators for single character
			expect(suggestions.every((s) => s.type === 'term')).toBe(true);
			expect(suggestions.some((s) => s.type === 'operator')).toBe(false);
		});

		it('should suggest appropriate second terms after operator', async () => {
			mockInvoke.mockResolvedValueOnce([
				{ term: 'async', type: 'term', description: 'Asynchronous programming' },
				{ term: 'await', type: 'term', description: 'Async/await syntax' },
				{ term: 'actix', type: 'term', description: 'Actix web framework' },
			]);

			const suggestions = await getSuggestions('rust AND a', 'Engineer');

			// Should return terms that start with 'a' and are relevant as second terms
			expect(suggestions).toEqual([
				{ term: 'async', type: 'term', description: 'Asynchronous programming' },
				{ term: 'await', type: 'term', description: 'Async/await syntax' },
				{ term: 'actix', type: 'term', description: 'Actix web framework' },
			]);
		});

		it('should handle OR operator in suggestions', async () => {
			mockInvoke.mockResolvedValueOnce([
				{ term: 'sdk', type: 'term', description: 'Software Development Kit' },
				{ term: 'service', type: 'term', description: 'Web service' },
			]);

			const suggestions = await getSuggestions('api OR s', 'Engineer');

			expect(suggestions).toEqual([
				{ term: 'sdk', type: 'term', description: 'Software Development Kit' },
				{ term: 'service', type: 'term', description: 'Web service' },
			]);
		});

		it('should not suggest operators within existing operator queries', async () => {
			mockInvoke.mockResolvedValueOnce([
				{ term: 'async', type: 'term', description: 'Asynchronous programming' },
				{ term: 'await', type: 'term', description: 'Async/await syntax' },
			]);

			const suggestions = await getSuggestions('rust AND async', 'Engineer');

			// Should not add more operator suggestions to completed operator query
			const operatorSuggestions = suggestions.filter((s) => s.type === 'operator');
			expect(operatorSuggestions).toHaveLength(0);
		});

		it('should handle mixed case in operator detection', async () => {
			mockInvoke.mockResolvedValueOnce([
				{ term: 'async', type: 'term', description: 'Asynchronous programming' },
			]);

			const suggestions1 = await getSuggestions('rust and a', 'Engineer');
			const suggestions2 = await getSuggestions('rust And a', 'Engineer');
			const suggestions3 = await getSuggestions('rust AND a', 'Engineer');

			// All should be treated the same way
			expect(suggestions1).toEqual(suggestions2);
			expect(suggestions2).toEqual(suggestions3);
		});

		it('should handle whitespace in operator queries', async () => {
			mockInvoke.mockResolvedValueOnce([
				{ term: 'async', type: 'term', description: 'Asynchronous programming' },
			]);

			const suggestions = await getSuggestions('  rust   AND   a', 'Engineer');

			expect(suggestions).toEqual([
				{ term: 'async', type: 'term', description: 'Asynchronous programming' },
			]);
		});
	});

	describe('Suggestion Ranking', () => {
		it('should prioritize exact matches in operator queries', async () => {
			mockInvoke.mockResolvedValueOnce([
				{ term: 'async', type: 'term', description: 'Asynchronous programming' },
				{ term: 'await', type: 'term', description: 'Async/await syntax' },
				{ term: 'api', type: 'term', description: 'Application Programming Interface' },
				{ term: 'algorithm', type: 'term', description: 'Algorithm implementation' },
			]);

			const suggestions = await getSuggestions('rust AND a', 'Engineer');

			// Should return all 'a' matches, likely with some ranking preference
			expect(suggestions.length).toBeGreaterThan(0);
			expect(suggestions.every((s) => s.term.toLowerCase().startsWith('a'))).toBe(true);
		});

		it('should limit number of suggestions appropriately', async () => {
			// Mock many suggestions
			const manyTerms = Array.from({ length: 20 }, (_, i) => ({
				term: `async${i}`,
				type: 'term',
				description: `Async term ${i}`,
			}));

			mockInvoke.mockResolvedValueOnce(manyTerms);

			const suggestions = await getSuggestions('rust AND async', 'Engineer');

			// Should limit to reasonable number (typically 10 or fewer for UX)
			expect(suggestions.length).toBeLessThanOrEqual(10);
		});
	});

	describe('Error Handling', () => {
		it('should handle service errors gracefully', async () => {
			mockInvoke.mockRejectedValueOnce(new Error('Service unavailable'));

			const suggestions = await getSuggestions('rust AND async', 'Engineer');

			// Should return empty array or fallback suggestions
			expect(Array.isArray(suggestions)).toBe(true);
		});

		it('should handle malformed operator queries', async () => {
			mockInvoke.mockResolvedValueOnce([]);

			const suggestions1 = await getSuggestions('AND AND AND', 'Engineer');
			const suggestions2 = await getSuggestions('rust AND AND', 'Engineer');

			expect(Array.isArray(suggestions1)).toBe(true);
			expect(Array.isArray(suggestions2)).toBe(true);
		});

		it('should handle empty responses from service', async () => {
			mockInvoke.mockResolvedValueOnce([]);

			const suggestions = await getSuggestions('rust AND nonexistent', 'Engineer');

			expect(suggestions).toEqual([]);
		});
	});

	describe('Context-Aware Suggestions', () => {
		it('should consider role context in operator suggestions', async () => {
			// Different roles might have different term suggestions
			mockInvoke
				.mockResolvedValueOnce([
					{ term: 'async', type: 'term', description: 'Async programming' },
					{ term: 'actix', type: 'term', description: 'Actix framework' },
				])
				.mockResolvedValueOnce([
					{ term: 'admin', type: 'term', description: 'System administration' },
					{ term: 'ansible', type: 'term', description: 'Ansible automation' },
				]);

			const engineerSuggestions = await getSuggestions('rust AND a', 'Engineer');
			const operatorSuggestions = await getSuggestions('rust AND a', 'System Operator');

			// Should get different suggestions based on role
			expect(engineerSuggestions).not.toEqual(operatorSuggestions);
		});

		it('should handle role-specific thesaurus in operator context', async () => {
			mockInvoke.mockResolvedValueOnce([
				{ term: 'deployment', type: 'term', description: 'Software deployment' },
				{ term: 'docker', type: 'term', description: 'Container technology' },
			]);

			const suggestions = await getSuggestions('infrastructure AND d', 'System Operator');

			expect(suggestions.some((s) => s.term === 'deployment')).toBe(true);
			expect(suggestions.some((s) => s.term === 'docker')).toBe(true);
		});
	});

	describe('Performance', () => {
		it('should not make redundant service calls for operator parsing', async () => {
			mockInvoke.mockResolvedValueOnce([
				{ term: 'async', type: 'term', description: 'Async programming' },
			]);

			await getSuggestions('rust AND a', 'Engineer');

			// Should only make one call to the service
			expect(mockInvoke).toHaveBeenCalledTimes(1);
		});

		it('should handle rapid typing in operator queries', async () => {
			mockInvoke
				.mockResolvedValueOnce([{ term: 'a', type: 'term', description: 'A' }])
				.mockResolvedValueOnce([{ term: 'as', type: 'term', description: 'As' }])
				.mockResolvedValueOnce([{ term: 'async', type: 'term', description: 'Async' }]);

			// Simulate rapid typing
			const promises = [
				getSuggestions('rust AND a', 'Engineer'),
				getSuggestions('rust AND as', 'Engineer'),
				getSuggestions('rust AND async', 'Engineer'),
			];

			const results = await Promise.all(promises);

			expect(results).toHaveLength(3);
			expect(results.every((r) => Array.isArray(r))).toBe(true);
		});
	});
});
