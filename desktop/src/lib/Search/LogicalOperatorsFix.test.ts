import { describe, expect, it } from 'vitest';
import { buildSearchQuery, parseSearchInput } from './searchUtils';

describe('Logical Operators Fix - Search Utils', () => {
	describe('parseSearchInput', () => {
		it('should parse AND operator without creating duplicate terms', () => {
			const result = parseSearchInput('rust AND async');

			expect(result.hasOperator).toBe(true);
			expect(result.operator).toBe('AND');
			expect(result.terms).toEqual(['rust', 'async']);
			expect(result.originalQuery).toBe('rust AND async');
		});

		it('should parse OR operator without creating duplicate terms', () => {
			const result = parseSearchInput('javascript OR python');

			expect(result.hasOperator).toBe(true);
			expect(result.operator).toBe('OR');
			expect(result.terms).toEqual(['javascript', 'python']);
			expect(result.originalQuery).toBe('javascript OR python');
		});

		it('should handle multiple terms with AND operator', () => {
			const result = parseSearchInput('rust AND async AND programming');

			expect(result.hasOperator).toBe(true);
			expect(result.operator).toBe('AND');
			expect(result.terms).toEqual(['rust', 'async', 'programming']);
			expect(result.originalQuery).toBe('rust AND async AND programming');
		});

		it('should handle single term without operator', () => {
			const result = parseSearchInput('rust');

			expect(result.hasOperator).toBe(false);
			expect(result.operator).toBe(null);
			expect(result.terms).toEqual(['rust']);
			expect(result.originalQuery).toBe('rust');
		});

		it('should handle empty/whitespace input', () => {
			const result = parseSearchInput('   ');

			expect(result.hasOperator).toBe(false);
			expect(result.operator).toBe(null);
			expect(result.terms).toEqual(['   ']); // Preserves whitespace as documented
			expect(result.originalQuery).toBe('   ');
		});
	});

	describe('buildSearchQuery', () => {
		it('should build query for AND operator with multiple terms', () => {
			const parsed = {
				hasOperator: true,
				operator: 'AND' as const,
				terms: ['rust', 'async'],
				originalQuery: 'rust AND async',
			};

			const result = buildSearchQuery(parsed, 'Engineer');

			expect(result.search_term).toBe('rust');
			expect(result.search_terms).toEqual(['rust', 'async']);
			expect(result.operator).toBe('and');
			expect(result.role).toBe('Engineer');
			expect(result.skip).toBe(0);
			expect(result.limit).toBe(50);
		});

		it('should build query for OR operator with multiple terms', () => {
			const parsed = {
				hasOperator: true,
				operator: 'OR' as const,
				terms: ['javascript', 'python'],
				originalQuery: 'javascript OR python',
			};

			const result = buildSearchQuery(parsed, 'Engineer');

			expect(result.search_term).toBe('javascript');
			expect(result.search_terms).toEqual(['javascript', 'python']);
			expect(result.operator).toBe('or');
			expect(result.role).toBe('Engineer');
		});

		it('should build single-term query without operator', () => {
			const parsed = {
				hasOperator: false,
				operator: null,
				terms: ['rust'],
				originalQuery: 'rust',
			};

			const result = buildSearchQuery(parsed, 'Engineer');

			expect(result.search_term).toBe('rust');
			expect(result.search_terms).toBeUndefined();
			expect(result.operator).toBeUndefined();
			expect(result.role).toBe('Engineer');
		});

		it('should filter empty terms and handle edge cases', () => {
			const parsed = {
				hasOperator: true,
				operator: 'AND' as const,
				terms: ['rust', '', 'async', '   '], // Contains empty and whitespace terms
				originalQuery: 'rust AND  AND async AND   ',
			};

			const result = buildSearchQuery(parsed, 'Engineer');

			expect(result.search_term).toBe('rust');
			expect(result.search_terms).toEqual(['rust', 'async']); // Empty terms filtered out
			expect(result.operator).toBe('and');
		});

		it('should handle invalid multi-term query by falling back to single term', () => {
			const parsed = {
				hasOperator: true,
				operator: 'AND' as const,
				terms: ['rust'], // Only one valid term
				originalQuery: 'rust AND   ',
			};

			const result = buildSearchQuery(parsed, 'Engineer');

			// Should fall back to single-term behavior
			expect(result.search_term).toBe('rust');
			expect(result.search_terms).toBeUndefined();
			expect(result.operator).toBeUndefined();
		});

		it('should handle role parameter correctly', () => {
			const parsed = {
				hasOperator: false,
				operator: null,
				terms: ['test'],
				originalQuery: 'test',
			};

			const result = buildSearchQuery(parsed, 'System Operator');
			expect(result.role).toBe('System Operator');

			const resultWithoutRole = buildSearchQuery(parsed);
			expect(resultWithoutRole.role).toBe(null);
		});
	});

	describe('Integration - Frontend to Backend Query Structure', () => {
		it('should create backend-compatible query structure for AND operator', () => {
			const parsed = parseSearchInput('rust AND async');
			const query = buildSearchQuery(parsed, 'Engineer');

			// This structure should work with the fixed backend get_all_terms() method
			expect(query).toMatchObject({
				search_term: 'rust',
				search_terms: ['rust', 'async'], // Contains all terms including first
				operator: 'and',
				role: 'Engineer',
			});

			// The backend get_all_terms() should return ['rust', 'async'] (no duplication)
		});

		it('should create backend-compatible query structure for OR operator', () => {
			const parsed = parseSearchInput('javascript OR python OR go');
			const query = buildSearchQuery(parsed, 'Engineer');

			expect(query).toMatchObject({
				search_term: 'javascript',
				search_terms: ['javascript', 'python', 'go'],
				operator: 'or',
				role: 'Engineer',
			});
		});

		it('should create backend-compatible single-term query', () => {
			const parsed = parseSearchInput('single-term');
			const query = buildSearchQuery(parsed, 'Engineer');

			expect(query).toMatchObject({
				search_term: 'single-term',
				search_terms: undefined, // No multiple terms
				operator: undefined, // No operator
				role: 'Engineer',
			});
		});
	});
});
