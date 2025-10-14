/**
 * Tests for ripgrep tag validation and parameter translation
 *
 * This test suite validates that tags entered in ConfigWizard
 * translate into valid ripgrep parameters
 */

import { describe, expect, it } from 'vitest';

/**
 * Validate that a tag string produces valid ripgrep parameters
 */
function validateRipgrepTag(tag: string): {
	isValid: boolean;
	errors: string[];
	parameters: string[];
} {
	const errors: string[] = [];
	const parameters: string[] = [];

	if (!tag || tag.trim() === '') {
		return { isValid: true, errors: [], parameters: [] }; // Empty tag is valid (no filtering)
	}

	const trimmedTag = tag.trim();

	// Validate tag format
	if (!trimmedTag.startsWith('#')) {
		errors.push('Tag must start with # (e.g., "#rust")');
	}

	// Check for invalid characters that could break ripgrep
	const invalidChars = /["\\\n\r\t]/;
	if (invalidChars.test(trimmedTag)) {
		errors.push('Tag contains invalid characters (quotes, backslashes, or control characters)');
	} else {
		// Only check format if no invalid characters are present
		// Check for valid tag format (alphanumeric, dash, underscore after #)
		const validTagPattern = /^#[a-zA-Z0-9_-]+$/;
		if (!validTagPattern.test(trimmedTag)) {
			errors.push(
				'Tag must contain only letters, numbers, dash, or underscore after # (e.g., "#rust", "#test-data", "#web_dev")'
			);
		}
	}

	// If valid, generate expected ripgrep parameters
	if (errors.length === 0) {
		parameters.push('--all-match');
		parameters.push('-e');
		parameters.push(trimmedTag);
	}

	return {
		isValid: errors.length === 0,
		errors,
		parameters,
	};
}

/**
 * Validate multiple tags (comma or space separated)
 */
function validateRipgrepTags(tagsString: string): {
	isValid: boolean;
	errors: string[];
	parameters: string[];
} {
	if (!tagsString || tagsString.trim() === '') {
		return { isValid: true, errors: [], parameters: [] };
	}

	const allErrors: string[] = [];
	const allParameters: string[] = [];
	let _hasValidTags = false;

	// Split by comma or whitespace
	const tags = tagsString
		.split(/[,\s]+/)
		.map((t) => t.trim())
		.filter((t) => t.length > 0);

	for (const tag of tags) {
		const validation = validateRipgrepTag(tag);
		if (!validation.isValid) {
			allErrors.push(...validation.errors.map((e) => `Tag "${tag}": ${e}`));
		} else if (validation.parameters.length > 0) {
			// Only add --all-match once
			if (!allParameters.includes('--all-match')) {
				allParameters.push('--all-match');
			}
			// Add the -e and tag parameters
			allParameters.push('-e');
			allParameters.push(tag);
			_hasValidTags = true;
		}
	}

	return {
		isValid: allErrors.length === 0,
		errors: allErrors,
		parameters: allParameters,
	};
}

/**
 * Validate extra parameters map for ripgrep
 */
function validateRipgrepExtraParameters(extraParams: Record<string, string>): {
	isValid: boolean;
	errors: string[];
} {
	const errors: string[] = [];

	for (const [key, value] of Object.entries(extraParams)) {
		switch (key) {
			case 'tag': {
				const tagValidation = validateRipgrepTags(value);
				if (!tagValidation.isValid) {
					errors.push(...tagValidation.errors);
				}
				break;
			}

			case 'max_count': {
				const maxCount = parseInt(value, 10);
				if (Number.isNaN(maxCount) || maxCount < 1) {
					errors.push('max_count must be a positive integer');
				}
				break;
			}

			case 'context': {
				const context = parseInt(value, 10);
				if (Number.isNaN(context) || context < 0) {
					errors.push('context must be a non-negative integer');
				}
				break;
			}

			case 'glob':
				if (!value || value.trim() === '') {
					errors.push('glob pattern cannot be empty');
				}
				// Basic glob validation - check for dangerous patterns
				if (value.includes('..') && value.includes('/')) {
					errors.push('glob pattern should not traverse directories with ".."');
				}
				break;

			case 'type': {
				if (!value || value.trim() === '') {
					errors.push('type filter cannot be empty');
				}
				// Common file types that ripgrep supports
				const validTypes = [
					'md',
					'markdown',
					'rs',
					'rust',
					'js',
					'javascript',
					'ts',
					'typescript',
					'py',
					'python',
					'txt',
					'text',
				];
				if (!validTypes.includes(value.toLowerCase())) {
					errors.push(
						`type "${value}" may not be supported by ripgrep. Common types: ${validTypes.join(', ')}`
					);
				}
				break;
			}

			case 'case_sensitive':
				if (!['true', 'false'].includes(value.toLowerCase())) {
					errors.push('case_sensitive must be "true" or "false"');
				}
				break;

			default:
				// Unknown parameter - warn but don't fail
				errors.push(`Unknown parameter "${key}" - will be ignored by ripgrep`);
				break;
		}
	}

	return {
		isValid: errors.length === 0,
		errors,
	};
}

describe('Ripgrep Tag Validation', () => {
	describe('Single Tag Validation', () => {
		it('should accept valid tags', () => {
			const validTags = ['#rust', '#test', '#docs', '#web-dev', '#test_data', '#123'];

			for (const tag of validTags) {
				const result = validateRipgrepTag(tag);
				expect(result.isValid).toBe(true);
				expect(result.errors).toHaveLength(0);
				expect(result.parameters).toEqual(['--all-match', '-e', tag]);
			}
		});

		it('should reject tags without #', () => {
			const result = validateRipgrepTag('rust');
			expect(result.isValid).toBe(false);
			expect(result.errors).toContain('Tag must start with # (e.g., "#rust")');
		});

		it('should reject tags with invalid characters', () => {
			const invalidTags = ['#"quoted"', '#back\\slash', '#new\nline', '#tab\t'];

			for (const tag of invalidTags) {
				const result = validateRipgrepTag(tag);
				expect(result.isValid).toBe(false);
				expect(result.errors.some((e) => e.includes('contains invalid characters'))).toBe(true);
			}
		});

		it('should reject tags with invalid format', () => {
			const invalidTags = ['#space tag', '#special!char', '#@symbol', '#'];

			for (const tag of invalidTags) {
				const result = validateRipgrepTag(tag);
				expect(result.isValid).toBe(false);
				expect(result.errors.some((e) => e.includes('must contain only letters'))).toBe(true);
			}
		});

		it('should accept empty tags', () => {
			const result = validateRipgrepTag('');
			expect(result.isValid).toBe(true);
			expect(result.parameters).toHaveLength(0);
		});
	});

	describe('Multiple Tags Validation', () => {
		it('should handle comma-separated tags', () => {
			const result = validateRipgrepTags('#rust, #test, #docs');
			expect(result.isValid).toBe(true);
			expect(result.parameters).toEqual([
				'--all-match',
				'-e',
				'#rust',
				'-e',
				'#test',
				'-e',
				'#docs',
			]);
		});

		it('should handle space-separated tags', () => {
			const result = validateRipgrepTags('#rust #test #docs');
			expect(result.isValid).toBe(true);
			expect(result.parameters).toEqual([
				'--all-match',
				'-e',
				'#rust',
				'-e',
				'#test',
				'-e',
				'#docs',
			]);
		});

		it('should reject if any tag is invalid', () => {
			const result = validateRipgrepTags('#rust, invalid, #docs');
			expect(result.isValid).toBe(false);
			expect(result.errors.some((e) => e.includes('Tag "invalid"'))).toBe(true);
		});
	});

	describe('Extra Parameters Validation', () => {
		it('should validate all parameter types', () => {
			const validParams = {
				tag: '#rust',
				max_count: '10',
				context: '3',
				glob: '*.md',
				type: 'markdown',
				case_sensitive: 'true',
			};

			const result = validateRipgrepExtraParameters(validParams);
			expect(result.isValid).toBe(true);
			expect(result.errors).toHaveLength(0);
		});

		it('should reject invalid max_count', () => {
			const result = validateRipgrepExtraParameters({ max_count: 'invalid' });
			expect(result.isValid).toBe(false);
			expect(result.errors.some((e) => e.includes('max_count must be a positive integer'))).toBe(
				true
			);
		});

		it('should reject invalid context', () => {
			const result = validateRipgrepExtraParameters({ context: '-1' });
			expect(result.isValid).toBe(false);
			expect(result.errors.some((e) => e.includes('context must be a non-negative integer'))).toBe(
				true
			);
		});

		it('should warn about unknown parameters', () => {
			const result = validateRipgrepExtraParameters({ unknown_param: 'value' });
			expect(result.isValid).toBe(false);
			expect(result.errors.some((e) => e.includes('Unknown parameter "unknown_param"'))).toBe(true);
		});
	});
});

// Export functions for use in ConfigWizard
export { validateRipgrepTag, validateRipgrepTags, validateRipgrepExtraParameters };
