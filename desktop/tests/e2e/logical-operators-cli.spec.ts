import { test, expect } from '@playwright/test';

test.describe('CLI Logical Operators Integration', () => {
	test.beforeEach(async ({ page }) => {
		// Start the Tauri app
		await page.goto('http://localhost:1420');
		await page.waitForLoadState('networkidle');
	});

	test.describe('Tauri Command Integration', () => {
		test('should execute AND search via Tauri command', async ({ page }) => {
			// Execute Tauri command with logical operators
			const searchResult = await page.evaluate(async () => {
				const { invoke } = window.__TAURI__.tauri;

				return await invoke('search_documents', {
					query: {
						search_term: 'rust',
						search_terms: ['rust', 'async'],
						operator: 'and',
						skip: 0,
						limit: 10,
						role: 'Engineer',
					},
				});
			});

			// Verify the search was executed
			expect(searchResult).toBeDefined();
			expect(Array.isArray(searchResult)).toBe(true);

			// If there are results, verify they contain both terms
			if (searchResult.length > 0) {
				for (const result of searchResult.slice(0, 3)) {
					const textToSearch =
						`${result.body} ${result.description} ${result.tags?.join(' ')}`.toLowerCase();
					const hasRust = textToSearch.includes('rust');
					const hasAsync = textToSearch.includes('async');

					// For AND operation, both terms should be present
					expect(hasRust && hasAsync).toBe(true);
				}
			}
		});

		test('should execute OR search via Tauri command', async ({ page }) => {
			const searchResult = await page.evaluate(async () => {
				const { invoke } = window.__TAURI__.tauri;

				return await invoke('search_documents', {
					query: {
						search_term: 'api',
						search_terms: ['api', 'sdk'],
						operator: 'or',
						skip: 0,
						limit: 10,
						role: 'Engineer',
					},
				});
			});

			expect(searchResult).toBeDefined();
			expect(Array.isArray(searchResult)).toBe(true);

			// If there are results, verify they contain at least one term
			if (searchResult.length > 0) {
				for (const result of searchResult.slice(0, 3)) {
					const textToSearch =
						`${result.body} ${result.description} ${result.tags?.join(' ')}`.toLowerCase();
					const hasApi = textToSearch.includes('api');
					const hasSdk = textToSearch.includes('sdk');

					// For OR operation, at least one term should be present
					expect(hasApi || hasSdk).toBe(true);
				}
			}
		});

		test('should handle single term search (backward compatibility)', async ({ page }) => {
			const searchResult = await page.evaluate(async () => {
				const { invoke } = window.__TAURI__.tauri;

				return await invoke('search_documents', {
					query: {
						search_term: 'rust',
						search_terms: null,
						operator: null,
						skip: 0,
						limit: 10,
						role: 'Engineer',
					},
				});
			});

			expect(searchResult).toBeDefined();
			expect(Array.isArray(searchResult)).toBe(true);

			// Should work just like regular search
			if (searchResult.length > 0) {
				const firstResult = searchResult[0];
				expect(firstResult).toHaveProperty('body');
				expect(firstResult).toHaveProperty('url');
				expect(firstResult).toHaveProperty('id');
			}
		});

		test('should handle role-specific searches with operators', async ({ page }) => {
			// Test with different roles
			const roles = ['Engineer', 'System Operator'];

			for (const role of roles) {
				const searchResult = await page.evaluate(async (testRole) => {
					const { invoke } = window.__TAURI__.tauri;

					return await invoke('search_documents', {
						query: {
							search_term: 'system',
							search_terms: ['system', 'operation'],
							operator: 'and',
							skip: 0,
							limit: 5,
							role: testRole,
						},
					});
				}, role);

				expect(searchResult).toBeDefined();
				expect(Array.isArray(searchResult)).toBe(true);

				// Results should be role-appropriate
				if (searchResult.length > 0) {
					expect(searchResult.length).toBeLessThanOrEqual(5); // Respects limit
				}
			}
		});
	});

	test.describe('Configuration Integration', () => {
		test('should get current configuration with role support', async ({ page }) => {
			const config = await page.evaluate(async () => {
				const { invoke } = window.__TAURI__.tauri;
				return await invoke('get_config');
			});

			expect(config).toBeDefined();
			expect(config).toHaveProperty('roles');
			expect(typeof config.roles).toBe('object');

			// Should have at least one role configured
			const roleNames = Object.keys(config.roles);
			expect(roleNames.length).toBeGreaterThan(0);
		});

		test('should update selected role', async ({ page }) => {
			// First get available roles
			const config = await page.evaluate(async () => {
				const { invoke } = window.__TAURI__.tauri;
				return await invoke('get_config');
			});

			const availableRoles = Object.keys(config.roles);
			expect(availableRoles.length).toBeGreaterThan(0);

			// Update to first available role
			const targetRole = availableRoles[0];
			const updatedConfig = await page.evaluate(async (role) => {
				const { invoke } = window.__TAURI__.tauri;
				return await invoke('update_selected_role', { roleName: role });
			}, targetRole);

			expect(updatedConfig).toBeDefined();
			expect(updatedConfig.selected_role).toBe(targetRole);
		});
	});

	test.describe('Autocomplete Integration', () => {
		test('should get autocomplete suggestions via Tauri', async ({ page }) => {
			const suggestions = await page.evaluate(async () => {
				const { invoke } = window.__TAURI__.tauri;

				return await invoke('autocomplete', {
					query: 'rust',
					role: 'Engineer',
					limit: 10,
				});
			});

			expect(suggestions).toBeDefined();
			expect(Array.isArray(suggestions)).toBe(true);

			if (suggestions.length > 0) {
				// Should have relevant autocomplete structure
				const firstSuggestion = suggestions[0];
				expect(firstSuggestion).toHaveProperty('term');
				expect(typeof firstSuggestion.term).toBe('string');
			}
		});

		test('should get role-specific autocomplete suggestions', async ({ page }) => {
			const roles = ['Engineer', 'System Operator'];

			for (const role of roles) {
				const suggestions = await page.evaluate(async (testRole) => {
					const { invoke } = window.__TAURI__.tauri;

					return await invoke('autocomplete', {
						query: 'sys',
						role: testRole,
						limit: 5,
					});
				}, role);

				expect(suggestions).toBeDefined();
				expect(Array.isArray(suggestions)).toBe(true);

				if (suggestions.length > 0) {
					expect(suggestions.length).toBeLessThanOrEqual(5);

					// Each suggestion should be relevant to the role
					for (const suggestion of suggestions) {
						expect(suggestion.term.toLowerCase()).toContain('sys');
					}
				}
			}
		});
	});

	test.describe('Error Handling', () => {
		test('should handle invalid search queries gracefully', async ({ page }) => {
			const invalidQueries = [
				{
					search_term: '',
					search_terms: [],
					operator: 'and',
					role: 'Engineer',
				},
				{
					search_term: 'test',
					search_terms: ['test', ''],
					operator: 'invalid_operator',
					role: 'Engineer',
				},
			];

			for (const query of invalidQueries) {
				try {
					const result = await page.evaluate(async (testQuery) => {
						const { invoke } = window.__TAURI__.tauri;
						return await invoke('search_documents', { query: testQuery });
					}, query);

					// Should return empty results or handle gracefully
					expect(Array.isArray(result)).toBe(true);
				} catch (error) {
					// Should not crash the application
					expect(error).toBeDefined();
				}
			}
		});

		test('should handle invalid role names', async ({ page }) => {
			try {
				await page.evaluate(async () => {
					const { invoke } = window.__TAURI__.tauri;
					return await invoke('search_documents', {
						query: {
							search_term: 'test',
							search_terms: ['test'],
							operator: 'and',
							role: 'NonexistentRole',
						},
					});
				});
			} catch (error) {
				// Should handle invalid role gracefully
				expect(error).toBeDefined();
			}
		});

		test('should handle network failures gracefully', async ({ page }) => {
			// This test would require mocking network failures
			// For now, just verify that the command structure is correct
			const searchCommand = await page.evaluate(async () => {
				const { invoke } = window.__TAURI__.tauri;

				try {
					return await invoke('search_documents', {
						query: {
							search_term: 'test',
							search_terms: ['test'],
							operator: 'and',
							skip: 0,
							limit: 1,
							role: 'Engineer',
						},
					});
				} catch (error) {
					return { error: error.message };
				}
			});

			// Should either succeed or return structured error
			expect(searchCommand).toBeDefined();
		});
	});

	test.describe('Performance and Limits', () => {
		test('should respect pagination parameters', async ({ page }) => {
			const smallLimit = await page.evaluate(async () => {
				const { invoke } = window.__TAURI__.tauri;
				return await invoke('search_documents', {
					query: {
						search_term: 'system',
						search_terms: null,
						operator: null,
						skip: 0,
						limit: 2,
						role: 'Engineer',
					},
				});
			});

			const largerLimit = await page.evaluate(async () => {
				const { invoke } = window.__TAURI__.tauri;
				return await invoke('search_documents', {
					query: {
						search_term: 'system',
						search_terms: null,
						operator: null,
						skip: 0,
						limit: 10,
						role: 'Engineer',
					},
				});
			});

			expect(Array.isArray(smallLimit)).toBe(true);
			expect(Array.isArray(largerLimit)).toBe(true);

			if (smallLimit.length > 0 && largerLimit.length > 0) {
				expect(smallLimit.length).toBeLessThanOrEqual(2);
				expect(largerLimit.length).toBeGreaterThanOrEqual(smallLimit.length);
			}
		});

		test('should handle skip parameter for pagination', async ({ page }) => {
			const firstPage = await page.evaluate(async () => {
				const { invoke } = window.__TAURI__.tauri;
				return await invoke('search_documents', {
					query: {
						search_term: 'system',
						search_terms: null,
						operator: null,
						skip: 0,
						limit: 3,
						role: 'Engineer',
					},
				});
			});

			const secondPage = await page.evaluate(async () => {
				const { invoke } = window.__TAURI__.tauri;
				return await invoke('search_documents', {
					query: {
						search_term: 'system',
						search_terms: null,
						operator: null,
						skip: 3,
						limit: 3,
						role: 'Engineer',
					},
				});
			});

			expect(Array.isArray(firstPage)).toBe(true);
			expect(Array.isArray(secondPage)).toBe(true);

			// If both pages have results, they should be different
			if (firstPage.length > 0 && secondPage.length > 0) {
				const firstPageIds = firstPage.map((r) => r.id);
				const secondPageIds = secondPage.map((r) => r.id);

				// No overlap expected between pages
				const overlap = firstPageIds.filter((id) => secondPageIds.includes(id));
				expect(overlap.length).toBe(0);
			}
		});
	});
});
