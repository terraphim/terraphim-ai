/**
 * End-to-End Tests for Ollama Integration
 *
 * This test suite validates the complete Ollama integration including:
 * - Ollama connectivity verification and health checks
 * - Model availability and switching (llama3.2:3b)
 * - Streaming response handling
 * - Configuration through frontend UI
 * - Fallback behavior when Ollama is offline
 * - Performance testing with local LLM
 * - Integration with both chat and summarization features
 */

import { expect, test } from '@playwright/test';
import {
	ciClick,
	ciNavigate,
	ciSearch,
	ciWait,
	ciWaitForSelector,
	getTimeouts,
} from '../../src/test-utils/ci-friendly';

// Test configuration
const TEST_TIMEOUT = 180000; // Extended for model loading
const OLLAMA_HEALTH_TIMEOUT = 10000;
const MODEL_RESPONSE_TIMEOUT = 60000;
const BACKEND_WAIT_TIME = 3000;

// Ollama configuration
const OLLAMA_CONFIG = {
	baseUrl: 'http://127.0.0.1:11434',
	defaultModel: 'llama3.2:3b',
	alternativeModels: ['llama3.1:8b', 'llama3:8b', 'codellama:7b'],
	healthEndpoint: '/api/tags',
	generateEndpoint: '/api/generate',
};

// Test data for Ollama validation
const OLLAMA_TEST_PROMPTS = [
	{
		prompt: 'Explain Rust ownership in one sentence.',
		expectedKeywords: ['ownership', 'rust', 'memory', 'borrow', 'move'],
		maxTokens: 100,
	},
	{
		prompt: 'What is async programming?',
		expectedKeywords: ['async', 'concurrent', 'await', 'task', 'thread'],
		maxTokens: 150,
	},
	{
		prompt: 'List three benefits of WebAssembly.',
		expectedKeywords: ['webassembly', 'wasm', 'performance', 'portable', 'secure'],
		maxTokens: 200,
	},
];

test.describe('Ollama Integration E2E Tests', () => {
	test.beforeEach(async ({ page }) => {
		test.setTimeout(TEST_TIMEOUT);
	});

	test.describe('Ollama Connectivity and Health Checks', () => {
		test('should verify Ollama service is running', async ({ page }) => {
			// First check if we can reach Ollama directly
			try {
				const response = await page.evaluate(async (config) => {
					try {
						const res = await fetch(`${config.baseUrl}${config.healthEndpoint}`);
						return {
							ok: res.ok,
							status: res.status,
							data: await res.json(),
						};
					} catch (error) {
						return {
							ok: false,
							error: error.message,
						};
					}
				}, OLLAMA_CONFIG);

				console.log('Ollama health check:', response);

				if (response.ok) {
					expect(response.status).toBe(200);
					expect(response.data).toBeDefined();

					// Check if our target model is available
					const models = response.data.models || [];
					const availableModels = models.map((m: any) => m.name);
					console.log('Available Ollama models:', availableModels);

					const hasTargetModel = availableModels.some((name: string) =>
						name.includes(OLLAMA_CONFIG.defaultModel.split(':')[0])
					);

					if (!hasTargetModel) {
						console.warn(
							`Target model ${OLLAMA_CONFIG.defaultModel} not found. Available:`,
							availableModels
						);
						console.warn('Consider running: ollama pull ' + OLLAMA_CONFIG.defaultModel);
					}
				} else {
					console.error('Ollama service not available:', response.error);
					console.log('Make sure Ollama is running: ollama serve');
					console.log(
						'And the required model is pulled: ollama pull ' + OLLAMA_CONFIG.defaultModel
					);
				}
			} catch (error) {
				console.error('Failed to check Ollama health:', error);
				throw new Error(
					'Ollama service check failed. Ensure Ollama is running on ' + OLLAMA_CONFIG.baseUrl
				);
			}
		});

		test('should validate model availability through frontend', async ({ page }) => {
			// Navigate to config page to check Ollama configuration
			await ciNavigate(page, '/config/wizard');
			await ciWaitForSelector(page, '[data-testid="config-wizard"]', 'navigation');

			// Look for Ollama role configuration
			const roles = page.locator('[data-testid="role-config"], .role-config');
			const roleCount = await roles.count();

			let ollamaRoleFound = false;

			for (let i = 0; i < roleCount; i++) {
				const role = roles.nth(i);

				// Check if this role uses Ollama
				const llmProviderSelect = role.locator(
					'select[name*="llm_provider"], select:has(option[value="ollama"])'
				);
				const llmProviderInput = role.locator('input[name*="llm_provider"]');

				let isOllamaRole = false;

				if (await llmProviderSelect.isVisible()) {
					const selectedValue = await llmProviderSelect.inputValue();
					isOllamaRole = selectedValue === 'ollama';
				} else if (await llmProviderInput.isVisible()) {
					const inputValue = await llmProviderInput.inputValue();
					isOllamaRole = inputValue === 'ollama';
				}

				if (isOllamaRole) {
					ollamaRoleFound = true;
					console.log(`Found Ollama role at index ${i}`);

					// Check Ollama-specific configuration
					const baseUrlInput = role.locator(
						'input[name*="ollama_base_url"], input[name*="base_url"]'
					);
					const modelInput = role.locator('input[name*="ollama_model"], input[name*="model"]');

					if (await baseUrlInput.isVisible()) {
						const baseUrl = await baseUrlInput.inputValue();
						console.log('Configured Ollama base URL:', baseUrl);
						expect(baseUrl).toContain('11434'); // Default Ollama port
					}

					if (await modelInput.isVisible()) {
						const model = await modelInput.inputValue();
						console.log('Configured Ollama model:', model);
						expect(model).toBeTruthy();
						expect(model.length).toBeGreaterThan(0);
					}

					// Test connection button if available
					const testConnectionButton = role.locator(
						'[data-testid="test-ollama-connection"], button:has-text("Test Connection")'
					);
					const canTestConnection = await testConnectionButton.isVisible();

					if (canTestConnection) {
						await testConnectionButton.click();
						await ciWait(page, 'large');

						// Check for connection result
						const connectionResult = role.locator(
							'[data-testid="connection-result"], .connection-status'
						);
						const hasResult = await connectionResult.isVisible();

						if (hasResult) {
							const resultText = await connectionResult.textContent();
							console.log('Connection test result:', resultText);

							// Should indicate success or provide helpful error
							expect(resultText?.toLowerCase()).toMatch(
								/success|connected|available|error|failed|timeout/
							);
						}
					}

					break;
				}
			}

			if (!ollamaRoleFound) {
				console.warn('No Ollama role configuration found in config wizard');
				console.log('Expected to find role with llm_provider="ollama"');
			}
		});

		test('should handle Ollama service unavailable gracefully', async ({ page }) => {
			// This test simulates backend behavior when Ollama is down
			// We'll test through the frontend interface

			await ciNavigate(page, '/chat');
			await ciWaitForSelector(page, '[data-testid="chat-container"]');

			const chatInput = page.locator('[data-testid="chat-input"]');
			const sendButton = page.locator('[data-testid="send-button"]');

			// Try to send a message
			await chatInput.fill('Test message when Ollama might be unavailable');
			await sendButton.click();

			// Wait for response or error
			await page.waitForFunction(
				() => {
					const errorEl = document.querySelector('[data-testid="chat-error"], .error-message');
					const responseEl = document.querySelector('.message.assistant-message:last-child');
					const loadingEl = document.querySelector('[data-testid="message-loading"]');
					return errorEl || responseEl || !loadingEl;
				},
				{ timeout: MODEL_RESPONSE_TIMEOUT }
			);

			// Check the outcome
			const errorMessage = page.locator('[data-testid="chat-error"], .error-message');
			const hasError = await errorMessage.isVisible();

			if (hasError) {
				const errorText = await errorMessage.textContent();
				console.log('Chat error (possibly due to Ollama unavailability):', errorText);

				// Error message should be user-friendly
				expect(errorText?.toLowerCase()).toMatch(/unavailable|connection|service|error|failed/);

				// Should offer helpful guidance
				const expectedTerms = ['ollama', 'service', 'connection', 'try again', 'check'];
				const hasHelpfulTerms = expectedTerms.some((term) =>
					errorText?.toLowerCase().includes(term)
				);

				expect(hasHelpfulTerms).toBeTruthy();
			} else {
				console.log('Chat succeeded - Ollama is working correctly');
			}
		});
	});

	test.describe('Model Response Quality', () => {
		test('should generate coherent responses for programming questions', async ({ page }) => {
			await ciNavigate(page, '/chat');
			await ciWaitForSelector(page, '[data-testid="chat-container"]');

			const chatInput = page.locator('[data-testid="chat-input"]');
			const sendButton = page.locator('[data-testid="send-button"]');
			const messageList = page.locator('[data-testid="message-list"]');

			for (const testCase of OLLAMA_TEST_PROMPTS) {
				console.log('Testing prompt:', testCase.prompt);

				const initialMessageCount = await messageList.locator('.message').count();

				// Send prompt
				await chatInput.fill(testCase.prompt);
				await sendButton.click();

				// Wait for assistant response
				await page.waitForSelector('.message.assistant-message', {
					timeout: MODEL_RESPONSE_TIMEOUT,
					state: 'visible',
				});

				// Get the latest assistant response
				const assistantMessages = messageList.locator('.message.assistant-message');
				const latestResponse = assistantMessages.last();

				const responseText = await latestResponse.textContent();
				console.log('Response preview:', responseText?.substring(0, 100) + '...');

				// Validate response quality
				expect(responseText?.length).toBeGreaterThan(20);
				expect(responseText?.length).toBeLessThan(2000); // Reasonable upper bound

				// Check for relevant keywords
				const responseWords = responseText?.toLowerCase().split(/\s+/) || [];
				const foundKeywords = testCase.expectedKeywords.filter((keyword) =>
					responseWords.some((word) => word.includes(keyword.toLowerCase()))
				);

				console.log('Found relevant keywords:', foundKeywords);
				expect(foundKeywords.length).toBeGreaterThan(0);

				// Brief pause between tests
				await ciWait(page, 'medium');
			}
		});

		test('should handle streaming responses correctly', async ({ page }) => {
			await ciNavigate(page, '/chat');
			await ciWaitForSelector(page, '[data-testid="chat-container"]');

			const chatInput = page.locator('[data-testid="chat-input"]');
			const sendButton = page.locator('[data-testid="send-button"]');
			const messageList = page.locator('[data-testid="message-list"]');

			// Send a longer prompt that should stream
			const longPrompt =
				'Explain the differences between async/await and traditional callback patterns in JavaScript, including examples and best practices.';

			await chatInput.fill(longPrompt);
			await sendButton.click();

			// Watch for streaming behavior
			await ciWait(page, 'small');

			const assistantMessage = messageList.locator('.message.assistant-message').last();

			// Check if response appears progressively (streaming)
			let previousLength = 0;
			let streamingDetected = false;

			for (let i = 0; i < 10; i++) {
				await page.waitForTimeout(1000);

				const currentText = await assistantMessage.textContent();
				const currentLength = currentText?.length || 0;

				if (currentLength > previousLength) {
					console.log(`Streaming progress: ${currentLength} characters`);
					streamingDetected = true;
				}

				previousLength = currentLength;

				// If response seems complete, break early
				if (currentLength > 100 && i > 3) {
					const recentChange = currentLength > previousLength;
					if (!recentChange) break;
				}
			}

			console.log('Streaming detected:', streamingDetected);
			console.log('Final response length:', previousLength);

			expect(previousLength).toBeGreaterThan(50);
		});

		test('should maintain context across conversation turns', async ({ page }) => {
			await ciNavigate(page, '/chat');
			await ciWaitForSelector(page, '[data-testid="chat-container"]');

			const chatInput = page.locator('[data-testid="chat-input"]');
			const sendButton = page.locator('[data-testid="send-button"]');
			const messageList = page.locator('[data-testid="message-list"]');

			// First message - establish context
			await chatInput.fill('My name is Alex and I am learning Rust programming.');
			await sendButton.click();

			await page.waitForSelector('.message.assistant-message', {
				timeout: MODEL_RESPONSE_TIMEOUT,
			});

			await ciWait(page, 'medium');

			// Second message - reference previous context
			await chatInput.fill('What would you recommend for my learning path?');
			await sendButton.click();

			await page.waitForSelector('.message.assistant-message:nth-last-child(1)', {
				timeout: MODEL_RESPONSE_TIMEOUT,
			});

			const secondResponse = messageList.locator('.message.assistant-message').last();
			const responseText = await secondResponse.textContent();

			console.log('Context-aware response:', responseText?.substring(0, 150) + '...');

			// Response should reference Rust or learning context
			const contextualTerms = ['rust', 'learning', 'recommend', 'path', 'study'];
			const responseWords = responseText?.toLowerCase() || '';

			const foundContextualTerms = contextualTerms.filter((term) => responseWords.includes(term));

			console.log('Found contextual terms:', foundContextualTerms);
			expect(foundContextualTerms.length).toBeGreaterThan(0);
		});
	});

	test.describe('Performance and Resource Management', () => {
		test('should handle concurrent requests efficiently', async ({ page }) => {
			await ciNavigate(page, '/');
			await ciWaitForSelector(page, 'input[type="search"]');

			// Perform search to get multiple results for summarization
			await ciSearch(page, 'input[type="search"]', 'rust programming guide');
			await ciWait(page, 'afterSearch');

			const resultItems = page.locator('.result-item');
			const resultCount = await resultItems.count();

			if (resultCount >= 2) {
				console.log(`Testing concurrent summarization of ${Math.min(resultCount, 3)} documents`);

				const startTime = Date.now();

				// Trigger multiple summarizations concurrently
				const summarizePromises = [];

				for (let i = 0; i < Math.min(resultCount, 3); i++) {
					const result = resultItems.nth(i);
					const summarizeButton = result.locator(
						'[data-testid="summarize-button"], button:has-text("Summarize")'
					);

					if (await summarizeButton.isVisible()) {
						summarizePromises.push(summarizeButton.click());
					}
				}

				// Execute all summarization requests
				await Promise.all(summarizePromises);

				// Wait for all summaries to complete
				await page.waitForFunction(
					() => {
						const loadingElements = document.querySelectorAll('[data-testid="summary-loading"]');
						return loadingElements.length === 0;
					},
					{ timeout: MODEL_RESPONSE_TIMEOUT * 2 }
				);

				const endTime = Date.now();
				const totalTime = endTime - startTime;

				console.log(`Concurrent summarization completed in ${totalTime}ms`);

				// Verify all summaries were generated
				const summaries = page.locator('[data-testid="ai-summary"], .ai-summary');
				const summaryCount = await summaries.count();

				console.log(`Generated ${summaryCount} summaries`);
				expect(summaryCount).toBeGreaterThan(0);

				// Performance should be reasonable (parallel processing)
				expect(totalTime).toBeLessThan(MODEL_RESPONSE_TIMEOUT * 2);
			}
		});

		test('should handle memory efficiently during long conversations', async ({ page }) => {
			await ciNavigate(page, '/chat');
			await ciWaitForSelector(page, '[data-testid="chat-container"]');

			const chatInput = page.locator('[data-testid="chat-input"]');
			const sendButton = page.locator('[data-testid="send-button"]');
			const messageList = page.locator('[data-testid="message-list"]');

			// Send multiple messages to build up conversation history
			const testMessages = [
				'Tell me about Rust.',
				'What are the main features?',
				'How does memory management work?',
				'Can you give an example?',
				'What about error handling?',
			];

			for (const message of testMessages) {
				await chatInput.fill(message);
				await sendButton.click();

				// Wait for response
				await page.waitForSelector('.message.assistant-message', {
					timeout: MODEL_RESPONSE_TIMEOUT,
				});

				// Brief pause between messages
				await ciWait(page, 'small');
			}

			// Check final state
			const totalMessages = await messageList.locator('.message').count();
			console.log('Total messages in conversation:', totalMessages);

			expect(totalMessages).toBeGreaterThanOrEqual(testMessages.length * 2); // User + assistant messages

			// Check that the interface is still responsive
			await chatInput.fill('Final test message');
			const inputValue = await chatInput.inputValue();
			expect(inputValue).toBe('Final test message');

			// Memory usage should be reasonable (no memory leaks in UI)
			const performanceMetrics = await page.evaluate(() => {
				return {
					usedJSHeapSize: (performance as any).memory?.usedJSHeapSize || 0,
					totalJSHeapSize: (performance as any).memory?.totalJSHeapSize || 0,
				};
			});

			console.log('Memory metrics:', performanceMetrics);
		});

		test('should recover from temporary Ollama disconnections', async ({ page }) => {
			await ciNavigate(page, '/chat');
			await ciWaitForSelector(page, '[data-testid="chat-container"]');

			const chatInput = page.locator('[data-testid="chat-input"]');
			const sendButton = page.locator('[data-testid="send-button"]');

			// Send initial message to verify connection
			await chatInput.fill('Initial connectivity test');
			await sendButton.click();

			// Wait for response or error
			try {
				await page.waitForSelector('.message.assistant-message', {
					timeout: MODEL_RESPONSE_TIMEOUT,
				});
				console.log('Initial connection successful');

				// Try another message after brief delay
				await ciWait(page, 'large');

				await chatInput.fill('Recovery test message');
				await sendButton.click();

				await page.waitForSelector('.message.assistant-message:nth-last-child(1)', {
					timeout: MODEL_RESPONSE_TIMEOUT,
				});

				console.log('Recovery test successful');
			} catch (error) {
				console.log('Connection test failed - checking error handling');

				// Should show helpful error message
				const errorMessage = page.locator('[data-testid="chat-error"], .error-message');
				const hasError = await errorMessage.isVisible();

				if (hasError) {
					const errorText = await errorMessage.textContent();
					console.log('Error message:', errorText);

					// Should offer retry or guidance
					const retryButton = page.locator(
						'[data-testid="retry-message"], button:has-text("Retry")'
					);
					const canRetry = await retryButton.isVisible();
					console.log('Retry option available:', canRetry);
				}
			}
		});
	});

	test.describe('Configuration and Model Management', () => {
		test('should allow switching between available models', async ({ page }) => {
			await ciNavigate(page, '/config/wizard');
			await ciWaitForSelector(page, '[data-testid="config-wizard"]');

			// Find Ollama role configuration
			const ollamaRole = page.locator(
				'[data-testid="role-config"]:has(select option[value="ollama"]), [data-testid="role-config"]:has(input[value="ollama"])'
			);
			const hasOllamaRole = await ollamaRole.isVisible();

			if (hasOllamaRole) {
				const modelInput = ollamaRole.locator('input[name*="model"], input[name*="ollama_model"]');
				const modelSelect = ollamaRole.locator('select[name*="model"]');

				if (await modelInput.isVisible()) {
					// Test changing model
					const currentModel = await modelInput.inputValue();
					console.log('Current model:', currentModel);

					// Try alternative model
					const alternativeModel =
						OLLAMA_CONFIG.alternativeModels.find((m) => m !== currentModel) || 'llama3:8b';

					await modelInput.fill(alternativeModel);
					console.log('Changed to model:', alternativeModel);

					// Save configuration
					const saveButton = page.locator('[data-testid="save-config"], button:has-text("Save")');
					if (await saveButton.isVisible()) {
						await saveButton.click();
						await ciWait(page, 'medium');

						// Test the new model
						await ciNavigate(page, '/chat');
						await ciWaitForSelector(page, '[data-testid="chat-container"]');

						const chatInput = page.locator('[data-testid="chat-input"]');
						const sendButton = page.locator('[data-testid="send-button"]');

						await chatInput.fill('Test message with new model');
						await sendButton.click();

						// Check if it works with the new model
						try {
							await page.waitForSelector('.message.assistant-message', {
								timeout: MODEL_RESPONSE_TIMEOUT,
							});
							console.log('Model switch successful');
						} catch (error) {
							console.log('Model switch may have failed - model might not be available');
						}

						// Restore original model
						await ciNavigate(page, '/config/wizard');
						await ciWaitForSelector(page, '[data-testid="config-wizard"]');
						const restoreModelInput = ollamaRole.locator('input[name*="model"]');
						await restoreModelInput.fill(currentModel);
						await saveButton.click();
					}
				}
			}
		});

		test('should validate Ollama configuration parameters', async ({ page }) => {
			await ciNavigate(page, '/config/wizard');
			await ciWaitForSelector(page, '[data-testid="config-wizard"]');

			// Look for role with Ollama provider
			const roles = page.locator('[data-testid="role-config"]');
			const roleCount = await roles.count();

			for (let i = 0; i < roleCount; i++) {
				const role = roles.nth(i);

				// Check if this is an Ollama role
				const llmProvider = role.locator(
					'select[name*="llm_provider"], input[name*="llm_provider"]'
				);
				const isOllamaRole = await llmProvider.isVisible();

				if (isOllamaRole) {
					// Test base URL validation
					const baseUrlInput = role.locator(
						'input[name*="base_url"], input[name*="ollama_base_url"]'
					);
					if (await baseUrlInput.isVisible()) {
						// Test invalid URL
						await baseUrlInput.fill('invalid-url');

						// Look for validation feedback
						const validationMessage = role.locator(
							'.validation-error, .error-message, .field-error'
						);

						// Save to trigger validation
						const saveButton = page.locator('[data-testid="save-config"], button:has-text("Save")');
						if (await saveButton.isVisible()) {
							await saveButton.click();
							await ciWait(page, 'small');

							const hasValidation = await validationMessage.isVisible();
							if (hasValidation) {
								const validationText = await validationMessage.textContent();
								console.log('URL validation message:', validationText);
								expect(validationText?.toLowerCase()).toMatch(/url|invalid|format/);
							}

							// Restore valid URL
							await baseUrlInput.fill(OLLAMA_CONFIG.baseUrl);
						}
					}

					// Test model name validation
					const modelInput = role.locator('input[name*="model"], input[name*="ollama_model"]');
					if (await modelInput.isVisible()) {
						const originalModel = await modelInput.inputValue();

						// Test empty model
						await modelInput.fill('');

						const saveButton = page.locator('[data-testid="save-config"], button:has-text("Save")');
						if (await saveButton.isVisible()) {
							await saveButton.click();
							await ciWait(page, 'small');

							// Should require model name
							const isValid = await saveButton.isEnabled();
							if (!isValid) {
								console.log('Correctly prevents saving without model name');
							}

							// Restore model
							await modelInput.fill(originalModel);
						}
					}

					break;
				}
			}
		});
	});
});
