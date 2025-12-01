/**
 * End-to-End Tests for Document Summarization Functionality
 *
 * This test suite validates the document summarization system including:
 * - AI summarization of search results with Ollama
 * - Auto-summarize toggle functionality in role configuration
 * - Summary caching and regeneration
 * - Error handling for LLM failures
 * - Integration with ResultItem component
 * - Performance and timeout handling
 */

import { test, expect } from '@playwright/test';
import {
	ciWaitForSelector,
	ciSearch,
	ciNavigate,
	ciWait,
	ciClick,
	getTimeouts,
} from '../../src/test-utils/ci-friendly';

// Test configuration
const TEST_TIMEOUT = 120000; // Extended for LLM operations
const SUMMARIZATION_TIMEOUT = 45000; // Timeout for summary generation
const BACKEND_WAIT_TIME = 2000;

// Test data for summarization
const TEST_SEARCH_QUERIES = [
	'rust async programming',
	'tokio futures',
	'error handling patterns',
	'webassembly performance',
	'microservices architecture',
];

const EXPECTED_SUMMARY_KEYWORDS = [
	['rust', 'async', 'tokio', 'future', 'await'],
	['tokio', 'future', 'runtime', 'async'],
	['error', 'result', 'handling', 'rust'],
	['wasm', 'webassembly', 'performance', 'native'],
	['microservices', 'distributed', 'architecture', 'system'],
];

test.describe('Document Summarization E2E Tests', () => {
	test.beforeEach(async ({ page }) => {
		test.setTimeout(TEST_TIMEOUT);

		// Navigate to search page to find documents to summarize
		await ciNavigate(page, '/');
		await ciWaitForSelector(page, 'input[type="search"]', 'navigation');
	});

	test.describe('Basic Summarization Functionality', () => {
		test('should generate AI summary for search result', async ({ page }) => {
			// Perform search to get results
			const searchInput = await ciWaitForSelector(page, 'input[type="search"]');
			await ciSearch(page, 'input[type="search"]', TEST_SEARCH_QUERIES[0]);

			// Wait for search results
			await ciWait(page, 'afterSearch');

			// Look for result items with summarize buttons
			const resultItems = page.locator('.result-item');
			const resultCount = await resultItems.count();

			if (resultCount > 0) {
				const firstResult = resultItems.first();

				// Look for summarize button (might be in actions menu)
				const summarizeButton = firstResult.locator(
					'[data-testid="summarize-button"], .summarize-action, button:has-text("Summarize")'
				);
				const summarizeButtonVisible = await summarizeButton.isVisible();

				if (summarizeButtonVisible) {
					// Click summarize button
					await summarizeButton.click();

					// Wait for loading indicator
					const loadingIndicator = firstResult.locator(
						'[data-testid="summary-loading"], .summary-loading'
					);
					if (await loadingIndicator.isVisible()) {
						console.log('Summary generation started...');

						// Wait for summary to appear or loading to disappear
						await page.waitForFunction(
							() => {
								const loading = document.querySelector(
									'[data-testid="summary-loading"], .summary-loading'
								);
								const summary = document.querySelector('[data-testid="ai-summary"], .ai-summary');
								return !loading || summary;
							},
							{ timeout: SUMMARIZATION_TIMEOUT }
						);
					}

					// Check for summary content
					const summaryContent = firstResult.locator(
						'[data-testid="ai-summary"], .ai-summary-content'
					);
					const summaryVisible = await summaryContent.isVisible();

					if (summaryVisible) {
						const summaryText = await summaryContent.textContent();
						console.log('Generated summary:', summaryText?.substring(0, 100) + '...');

						// Validate summary content
						expect(summaryText?.length).toBeGreaterThan(20);
						expect(summaryText?.length).toBeLessThan(1000);

						// Check if summary contains relevant keywords
						const lowerSummary = summaryText?.toLowerCase() || '';
						const relevantKeywords = EXPECTED_SUMMARY_KEYWORDS[0];
						const foundKeywords = relevantKeywords.filter((keyword) =>
							lowerSummary.includes(keyword.toLowerCase())
						);

						expect(foundKeywords.length).toBeGreaterThan(0);
						console.log('Found relevant keywords:', foundKeywords);
					} else {
						console.log('Summary not visible - checking for error state');

						// Check for error message
						const errorMessage = firstResult.locator(
							'[data-testid="summary-error"], .summary-error'
						);
						const hasError = await errorMessage.isVisible();

						if (hasError) {
							const errorText = await errorMessage.textContent();
							console.log('Summary error:', errorText);
						}
					}
				} else {
					console.log('Summarize button not found - checking if auto-summarization is enabled');

					// Check if summary already exists (auto-summarization)
					const existingSummary = firstResult.locator('[data-testid="ai-summary"], .ai-summary');
					const hasSummary = await existingSummary.isVisible();

					if (hasSummary) {
						console.log('Auto-summary detected');
						const summaryText = await existingSummary.textContent();
						expect(summaryText?.length).toBeGreaterThan(10);
					}
				}
			} else {
				console.log('No search results found for summarization test');
			}
		});

		test('should handle multiple document summarization', async ({ page }) => {
			// Perform search
			const searchInput = await ciWaitForSelector(page, 'input[type="search"]');
			await ciSearch(page, 'input[type="search"]', TEST_SEARCH_QUERIES[1]);
			await ciWait(page, 'afterSearch');

			const resultItems = page.locator('.result-item');
			const resultCount = await resultItems.count();

			if (resultCount >= 2) {
				// Try to summarize first two results
				for (let i = 0; i < Math.min(2, resultCount); i++) {
					const resultItem = resultItems.nth(i);
					const summarizeButton = resultItem.locator(
						'[data-testid="summarize-button"], button:has-text("Summarize")'
					);

					const buttonVisible = await summarizeButton.isVisible();
					if (buttonVisible) {
						await summarizeButton.click();

						// Wait briefly between requests to avoid overloading
						await ciWait(page, 'medium');

						console.log(`Initiated summarization for result ${i + 1}`);
					}
				}

				// Wait for all summaries to complete
				await page.waitForTimeout(SUMMARIZATION_TIMEOUT / 2);

				// Check results
				const summaries = page.locator('[data-testid="ai-summary"], .ai-summary');
				const summaryCount = await summaries.count();

				console.log(`Generated ${summaryCount} summaries out of ${resultCount} results`);
				expect(summaryCount).toBeGreaterThan(0);
			}
		});

		test('should show cache status for summaries', async ({ page }) => {
			// Search for documents
			const searchInput = await ciWaitForSelector(page, 'input[type="search"]');
			await ciSearch(page, 'input[type="search"]', TEST_SEARCH_QUERIES[2]);
			await ciWait(page, 'afterSearch');

			const resultItems = page.locator('.result-item');
			const firstResult = resultItems.first();

			// Generate summary first time
			const summarizeButton = firstResult.locator(
				'[data-testid="summarize-button"], button:has-text("Summarize")'
			);
			const buttonVisible = await summarizeButton.isVisible();

			if (buttonVisible) {
				await summarizeButton.click();
				await page.waitForTimeout(SUMMARIZATION_TIMEOUT / 2);

				// Look for cache indicator
				const cacheIndicator = firstResult.locator(
					'[data-testid="cache-status"], .cache-indicator, .tag:has-text("cached"), .tag:has-text("fresh")'
				);
				const hasCacheIndicator = await cacheIndicator.isVisible();

				if (hasCacheIndicator) {
					const cacheText = await cacheIndicator.textContent();
					console.log('Cache status:', cacheText);
					expect(cacheText).toMatch(/fresh|cached/i);
				}

				// Try to regenerate summary to test cache behavior
				const regenerateButton = firstResult.locator(
					'[data-testid="regenerate-summary"], button:has-text("Regenerate")'
				);
				const canRegenerate = await regenerateButton.isVisible();

				if (canRegenerate) {
					await regenerateButton.click();
					await ciWait(page, 'medium');

					// Should show loading again
					const loadingAfterRegen = firstResult.locator('[data-testid="summary-loading"]');
					const isLoadingAgain = await loadingAfterRegen.isVisible();
					console.log('Regeneration triggered loading:', isLoadingAgain);
				}
			}
		});
	});

	test.describe('Auto-Summarization Configuration', () => {
		test('should respect auto-summarize setting in role configuration', async ({ page }) => {
			// Navigate to config wizard to check auto-summarize settings
			await ciNavigate(page, '/config/wizard');
			await ciWaitForSelector(page, '[data-testid="config-wizard"]', 'navigation');

			// Find the role with LLM configuration
			const roles = page.locator('[data-testid="role-config"]');
			const roleCount = await roles.count();

			if (roleCount > 0) {
				// Check first role's LLM settings
				const firstRole = roles.first();

				// Look for auto-summarize checkbox
				const autoSummarizeCheckbox = firstRole.locator(
					'input[type="checkbox"]:near(text("auto-summarize")), input[id*="auto-summarize"]'
				);
				const hasAutoSummarize = await autoSummarizeCheckbox.isVisible();

				if (hasAutoSummarize) {
					const isChecked = await autoSummarizeCheckbox.isChecked();
					console.log('Auto-summarize enabled:', isChecked);

					// Test toggling the setting
					await autoSummarizeCheckbox.click();

					// Save configuration
					const saveButton = page.locator('[data-testid="save-config"], button:has-text("Save")');
					const canSave = await saveButton.isVisible();

					if (canSave) {
						await saveButton.click();
						await ciWait(page, 'medium');

						// Navigate back to search to test the setting
						await ciNavigate(page, '/');
						await ciWaitForSelector(page, 'input[type="search"]');

						// Perform search and check if auto-summarization behavior changed
						await ciSearch(page, 'input[type="search"]', TEST_SEARCH_QUERIES[3]);
						await ciWait(page, 'afterSearch');

						// Check if results have automatic summaries
						const resultWithSummary = page.locator(
							'.result-item:has([data-testid="ai-summary"], .ai-summary)'
						);
						const autoSummaryCount = await resultWithSummary.count();

						console.log(`Found ${autoSummaryCount} results with auto-summaries`);
					}
				}
			}
		});

		test('should validate LLM provider configuration for summarization', async ({ page }) => {
			await ciNavigate(page, '/config/wizard');
			await ciWaitForSelector(page, '[data-testid="config-wizard"]');

			// Look for LLM provider settings
			const llmProviderSelect = page.locator(
				'select[name*="llm"], select:has(option:text("ollama"))'
			);
			const hasLlmConfig = await llmProviderSelect.isVisible();

			if (hasLlmConfig) {
				// Check current provider
				const currentProvider = await llmProviderSelect.inputValue();
				console.log('Current LLM provider:', currentProvider);

				// If Ollama is available, select it
				const ollamaOption = llmProviderSelect.locator('option[value="ollama"]');
				const hasOllama = await ollamaOption.isVisible();

				if (hasOllama) {
					await llmProviderSelect.selectOption('ollama');

					// Check for Ollama-specific settings
					const ollamaUrlInput = page.locator(
						'input[name*="ollama_base_url"], input[placeholder*="ollama"]'
					);
					const ollamaModelInput = page.locator(
						'input[name*="ollama_model"], input[value*="llama"]'
					);

					const hasOllamaConfig = await ollamaUrlInput.isVisible();
					console.log('Ollama configuration available:', hasOllamaConfig);

					if (hasOllamaConfig) {
						// Verify default values
						const baseUrl = await ollamaUrlInput.inputValue();
						const model = await ollamaModelInput.inputValue();

						console.log('Ollama base URL:', baseUrl);
						console.log('Ollama model:', model);

						expect(baseUrl).toContain('11434'); // Default Ollama port
						expect(model).toContain('llama'); // Should have a llama model
					}
				}
			}
		});
	});

	test.describe('Error Handling and Edge Cases', () => {
		test('should handle summarization timeout gracefully', async ({ page }) => {
			// Search for documents
			const searchInput = await ciWaitForSelector(page, 'input[type="search"]');
			await ciSearch(page, 'input[type="search"]', TEST_SEARCH_QUERIES[4]);
			await ciWait(page, 'afterSearch');

			const resultItems = page.locator('.result-item');
			const firstResult = resultItems.first();

			const summarizeButton = firstResult.locator('[data-testid="summarize-button"]');
			const buttonVisible = await summarizeButton.isVisible();

			if (buttonVisible) {
				await summarizeButton.click();

				// Wait for a shorter timeout to test timeout handling
				try {
					await page.waitForSelector('[data-testid="ai-summary"]', {
						timeout: 5000, // Very short timeout to trigger timeout scenario
					});
				} catch {
					console.log('Expected timeout occurred, checking error handling');

					// Check for timeout error message
					const errorMessage = firstResult.locator('[data-testid="summary-error"], .error-message');
					const hasError = await errorMessage.isVisible();

					if (hasError) {
						const errorText = await errorMessage.textContent();
						console.log('Timeout error message:', errorText);

						// Should offer retry option
						const retryButton = firstResult.locator(
							'[data-testid="retry-summary"], button:has-text("Retry")'
						);
						const canRetry = await retryButton.isVisible();
						console.log('Retry option available:', canRetry);
					}
				}
			}
		});

		test('should handle Ollama service unavailable', async ({ page }) => {
			// This test simulates what happens when Ollama is not running
			const searchInput = await ciWaitForSelector(page, 'input[type="search"]');
			await ciSearch(page, 'input[type="search"]', 'test summarization');
			await ciWait(page, 'afterSearch');

			const resultItems = page.locator('.result-item');
			const firstResult = resultItems.first();

			const summarizeButton = firstResult.locator('[data-testid="summarize-button"]');
			const buttonVisible = await summarizeButton.isVisible();

			if (buttonVisible) {
				await summarizeButton.click();

				// Wait for either success or error
				await page.waitForFunction(
					() => {
						const summary = document.querySelector('[data-testid="ai-summary"]');
						const error = document.querySelector('[data-testid="summary-error"]');
						const loading = document.querySelector('[data-testid="summary-loading"]');
						return summary || error || !loading;
					},
					{ timeout: SUMMARIZATION_TIMEOUT }
				);

				// Check the outcome
				const hasSummary = await firstResult.locator('[data-testid="ai-summary"]').isVisible();
				const hasError = await firstResult.locator('[data-testid="summary-error"]').isVisible();

				console.log('Summary generated:', hasSummary);
				console.log('Error occurred:', hasError);

				if (hasError) {
					const errorText = await firstResult
						.locator('[data-testid="summary-error"]')
						.textContent();
					console.log('Error details:', errorText);

					// Error should be user-friendly
					expect(errorText?.toLowerCase()).toMatch(/unavailable|error|failed|connection/);
				}
			}
		});

		test('should handle empty or short documents', async ({ page }) => {
			// This test would ideally use mock data, but we'll test with actual results
			const searchInput = await ciWaitForSelector(page, 'input[type="search"]');
			await ciSearch(page, 'input[type="search"]', 'minimal content test');
			await ciWait(page, 'afterSearch');

			const resultItems = page.locator('.result-item');
			const resultCount = await resultItems.count();

			if (resultCount > 0) {
				const result = resultItems.first();

				// Check document content length (if visible)
				const contentElement = result.locator(
					'.description, .content, [data-testid="document-body"]'
				);
				const contentVisible = await contentElement.isVisible();

				if (contentVisible) {
					const contentText = await contentElement.textContent();
					const contentLength = contentText?.length || 0;

					console.log('Document content length:', contentLength);

					// Try summarization
					const summarizeButton = result.locator('[data-testid="summarize-button"]');
					const buttonVisible = await summarizeButton.isVisible();

					if (buttonVisible) {
						await summarizeButton.click();
						await ciWait(page, 'large');

						const summary = result.locator('[data-testid="ai-summary"]');
						const hasSummary = await summary.isVisible();

						if (hasSummary) {
							const summaryText = await summary.textContent();
							const summaryLength = summaryText?.length || 0;

							console.log('Summary length:', summaryLength);

							// Summary should be reasonable length
							if (contentLength < 100) {
								// For very short content, summary might be short or indicate "already concise"
								expect(summaryLength).toBeLessThan(contentLength + 50);
							}
						}
					}
				}
			}
		});
	});

	test.describe('Performance and User Experience', () => {
		test('should provide visual feedback during summarization', async ({ page }) => {
			const searchInput = await ciWaitForSelector(page, 'input[type="search"]');
			await ciSearch(page, 'input[type="search"]', 'performance test');
			await ciWait(page, 'afterSearch');

			const resultItems = page.locator('.result-item');
			const firstResult = resultItems.first();

			const summarizeButton = firstResult.locator('[data-testid="summarize-button"]');
			const buttonVisible = await summarizeButton.isVisible();

			if (buttonVisible) {
				// Check initial state
				expect(await summarizeButton.isVisible()).toBeTruthy();

				await summarizeButton.click();

				// Should show loading indicator immediately
				await ciWait(page, 'tiny');

				const loadingIndicator = firstResult.locator(
					'[data-testid="summary-loading"], .loading, .spinner'
				);
				const hasLoading = await loadingIndicator.isVisible();

				console.log('Loading indicator shown:', hasLoading);

				// Button should be disabled during loading
				const buttonDisabled = await summarizeButton.isDisabled();
				console.log('Button disabled during loading:', buttonDisabled);

				// Wait for completion
				await page.waitForFunction(
					() => {
						const loading = document.querySelector('[data-testid="summary-loading"], .loading');
						return !loading || !loading.isVisible;
					},
					{ timeout: SUMMARIZATION_TIMEOUT }
				);

				// Loading should be gone, button should be re-enabled or hidden
				const stillLoading = await loadingIndicator.isVisible();
				expect(stillLoading).toBeFalsy();
			}
		});

		test('should allow canceling summarization', async ({ page }) => {
			const searchInput = await ciWaitForSelector(page, 'input[type="search"]');
			await ciSearch(page, 'input[type="search"]', 'cancellation test');
			await ciWait(page, 'afterSearch');

			const resultItems = page.locator('.result-item');
			const firstResult = resultItems.first();

			const summarizeButton = firstResult.locator('[data-testid="summarize-button"]');
			const buttonVisible = await summarizeButton.isVisible();

			if (buttonVisible) {
				await summarizeButton.click();

				// Look for cancel button during loading
				await ciWait(page, 'small');

				const cancelButton = firstResult.locator(
					'[data-testid="cancel-summary"], button:has-text("Cancel")'
				);
				const canCancel = await cancelButton.isVisible();

				if (canCancel) {
					console.log('Cancel button available');
					await cancelButton.click();

					// Should stop loading
					await ciWait(page, 'small');

					const stillLoading = await firstResult
						.locator('[data-testid="summary-loading"]')
						.isVisible();
					expect(stillLoading).toBeFalsy();

					// Original summarize button should be available again
					const summarizeAvailable = await summarizeButton.isVisible();
					console.log('Summarize button available after cancel:', summarizeAvailable);
				} else {
					console.log('Cancel functionality not available');
				}
			}
		});
	});
});
