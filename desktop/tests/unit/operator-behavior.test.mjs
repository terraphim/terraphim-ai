/**
 * Test for AND/OR operator behavior in search
 * This test verifies that:
 * 1. AND and OR operators change search behavior
 * 2. Operators are parsed correctly
 * 3. Search queries are built properly with operators
 */

import { describe, it, expect } from 'vitest';
import { parseSearchInput, buildSearchQuery } from '../../src/lib/Search/searchUtils.js';

describe('Search Operator Behavior', () => {
  describe('parseSearchInput', () => {
    it('should parse AND operator correctly', () => {
      const result = parseSearchInput('rust AND javascript');
      expect(result.hasOperator).toBe(true);
      expect(result.operator).toBe('AND');
      expect(result.terms).toEqual(['rust', 'javascript']);
    });

    it('should parse OR operator correctly', () => {
      const result = parseSearchInput('python OR typescript');
      expect(result.hasOperator).toBe(true);
      expect(result.operator).toBe('OR');
      expect(result.terms).toEqual(['python', 'typescript']);
    });

    it('should parse lowercase and operator correctly', () => {
      const result = parseSearchInput('rust and javascript');
      expect(result.hasOperator).toBe(true);
      expect(result.operator).toBe('AND');
      expect(result.terms).toEqual(['rust', 'javascript']);
    });

    it('should parse lowercase or operator correctly', () => {
      const result = parseSearchInput('python or typescript');
      expect(result.hasOperator).toBe(true);
      expect(result.operator).toBe('OR');
      expect(result.terms).toEqual(['python', 'typescript']);
    });

    it('should prioritize capitalized operators', () => {
      const result = parseSearchInput('rust AND javascript or python');
      expect(result.hasOperator).toBe(true);
      expect(result.operator).toBe('AND');
      expect(result.terms).toEqual(['rust', 'javascript or python']);
    });

    it('should handle single term without operators', () => {
      const result = parseSearchInput('rust');
      expect(result.hasOperator).toBe(false);
      expect(result.operator).toBe(null);
      expect(result.terms).toEqual(['rust']);
    });

    it('should handle multiple terms with AND', () => {
      const result = parseSearchInput('rust AND javascript AND python');
      expect(result.hasOperator).toBe(true);
      expect(result.operator).toBe('AND');
      expect(result.terms).toEqual(['rust', 'javascript', 'python']);
    });

    it('should handle multiple terms with OR', () => {
      const result = parseSearchInput('rust OR javascript OR python');
      expect(result.hasOperator).toBe(true);
      expect(result.operator).toBe('OR');
      expect(result.terms).toEqual(['rust', 'javascript', 'python']);
    });
  });

  describe('buildSearchQuery', () => {
    it('should build AND query correctly', () => {
      const parsed = parseSearchInput('rust AND javascript');
      const query = buildSearchQuery(parsed, 'Engineer');

      expect(query.search_term).toBe('rust');
      expect(query.search_terms).toEqual(['rust', 'javascript']);
      expect(query.operator).toBe('and');
      expect(query.role).toBe('Engineer');
    });

    it('should build OR query correctly', () => {
      const parsed = parseSearchInput('python OR typescript');
      const query = buildSearchQuery(parsed, 'Engineer');

      expect(query.search_term).toBe('python');
      expect(query.search_terms).toEqual(['python', 'typescript']);
      expect(query.operator).toBe('or');
      expect(query.role).toBe('Engineer');
    });

    it('should build single term query correctly', () => {
      const parsed = parseSearchInput('rust');
      const query = buildSearchQuery(parsed, 'Engineer');

      expect(query.search_term).toBe('rust');
      expect(query.search_terms).toBeUndefined();
      expect(query.operator).toBeUndefined();
      expect(query.role).toBe('Engineer');
    });

    it('should handle empty terms', () => {
      const parsed = parseSearchInput('rust AND   AND javascript');
      const query = buildSearchQuery(parsed, 'Engineer');

      expect(query.search_term).toBe('rust');
      expect(query.search_terms).toEqual(['rust', 'javascript']);
      expect(query.operator).toBe('and');
    });
  });

  describe('Operator precedence and parsing', () => {
    it('should prioritize capitalized AND over lowercase or', () => {
      const result = parseSearchInput('term1 and term2 AND term3 or term4');
      expect(result.operator).toBe('AND');
      expect(result.terms.length).toBeGreaterThan(1);
    });

    it('should prioritize capitalized OR over lowercase and', () => {
      const result = parseSearchInput('term1 and term2 OR term3 and term4');
      expect(result.operator).toBe('OR');
      expect(result.terms.length).toBeGreaterThan(1);
    });

    it('should handle mixed case operators correctly', () => {
      const result = parseSearchInput('rust AND javascript OR python');
      expect(result.hasOperator).toBe(true);
      expect(result.operator).toBe('AND');
    });
  });
});
