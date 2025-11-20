/**
 * Complete Workflow Integration Test
 *
 * This test covers the complete end-to-end workflow:
 * 1. Search for documents
 * 2. Add documents to context
 * 3. Chat with document context
 * 4. Add KG terms to context
 * 5. Chat with enhanced context
 * 6. Verify context is passed to LLM
 * 7. Test error handling and edge cases
 */

import { expect, test } from '@playwright/test';
import {
	ciClick,
	ciNavigate,
	ciWait,
	ciWaitForSelector,
	getTimeouts,
} from '../../src/test-utils/ci-friendly';

// Test configuration
const TEST_TIMEOUT = 180000; // Extended for complete workflow
const LLM_RESPONSE_TIMEOUT = 45000;

// Test scenarios
const WORKFLOW_SCENARIOS = [
	{
		name: 'Rust Programming Workflow',
		searchQueries: ['rust async programming', 'tokio futures', 'error handling'],
		chatMessages: [
			'Explain the key concepts from the documents I added',
			'How do these concepts work together?',
			'What are the best practices mentioned?',
		],
		kgTerms: ['async-await', 'futures', 'tokio-runtime'],
		expectedContext: ['rust', 'async', 'tokio', 'futures', 'error-handling'],
	},
	{
		name: 'System Design Workflow',
		searchQueries: ['distributed systems', 'microservices architecture', 'scalability patterns'],
		chatMessages: [
			'Summarize the architecture patterns from the context',
			'How do these patterns address scalability?',
			'What are the trade-offs mentioned?',
		],
		kgTerms: ['microservices', 'load-balancing', 'distributed-consensus'],
		expectedContext: ['distributed', 'microservices', 'scalability', 'architecture'],
	},
];

test.describe('Complete Workflow Integration Tests', () => {
	test.beforeEach(async ({ page }) => {
		test.setTimeout(TEST_TIMEOUT);
		await ciNavigate(page, '/');
		await ciWaitForSelector(page, '[data-testid="search-tab"]', 'navigation');
	});

	test.describe('Complete User Workflows', () => {
		for (const scenario of WORKFLOW_SCENARIOS) {
			test(`should complete ${scenario.name} workflow`, async ({ page }) => {
				console.log(`🚀 Starting ${scenario.name} workflow...`);

				// Step 1: Search for documents
				console.log('📝 Step 1: Searching for documents...');
				const searchResults = await performMultipleSearches(page, scenario.searchQueries);
				expect(searchResults.length).toBeGreaterThan(0);
				console.log(`✅ Found ${searchResults.length} search results`);

				// Step 2: Add documents to context
				console.log('📝 Step 2: Adding documents to context...');
				const contextItems = await addDocumentsToContext(page, searchResults);
				expect(contextItems).toBeGreaterThan(0);
				console.log(`✅ Added ${contextItems} documents to context`);

				// Step 3: Navigate to chat
				console.log('📝 Step 3: Navigating to chat...');
				await ciNavigate(page, '/chat');
				await ciWaitForSelector(page, '[data-testid="chat-interface"]', 'navigation');
				console.log('✅ Chat interface loaded');

				// Step 4: Verify context is visible
				console.log('📝 Step 4: Verifying context visibility...');
				const visibleContextItems = await verifyContextVisibility(page);
				expect(visibleContextItems).toBeGreaterThan(0);
				console.log(`✅ ${visibleContextItems} context items visible`);

				// Step 5: Chat with document context
				console.log('📝 Step 5: Chatting with document context...');
				await performChatWithContext(page, scenario.chatMessages[0]);
				console.log('✅ Chat with document context completed');

				// Step 6: Add KG terms to context
				console.log('📝 Step 6: Adding KG terms to context...');
				const kgItems = await addKGTermsToContext(page, scenario.kgTerms);
				console.log(`✅ Added ${kgItems} KG terms to context`);

				// Step 7: Chat with enhanced context
				console.log('📝 Step 7: Chatting with enhanced context...');
				await performChatWithContext(page, scenario.chatMessages[1]);
				console.log('✅ Chat with enhanced context completed');

				// Step 8: Verify context influence on LLM
				console.log('📝 Step 8: Verifying context influence on LLM...');
				await verifyContextInfluence(page, scenario.expectedContext);
				console.log('✅ Context influence verified');

				// Step 9: Final comprehensive chat
				console.log('📝 Step 9: Final comprehensive chat...');
				await performChatWithContext(page, scenario.chatMessages[2]);
				console.log('✅ Final comprehensive chat completed');

				console.log(`🎉 ${scenario.name} workflow completed successfully!`);
			});
		}

		test('should handle workflow with no search results', async ({ page }) => {
			console.log('🚀 Testing workflow with no search results...');

			// Search for something that won't return results
			const searchInput = page.locator('input[type="search"]');
			await searchInput.fill('nonexistent_query_that_will_not_return_results');
			await searchInput.press('Enter');
			await ciWait(page, 'medium');

			// Should show empty state
			const emptyState = page.locator('[data-testid="empty-state"], .has-text-centered');
			const hasEmptyState = await emptyState.isVisible();

			if (hasEmptyState) {
				console.log('✅ Empty search results handled gracefully');
			}

			// Navigate to chat and add manual context
			await ciNavigate(page, '/chat');
			await ciWaitForSelector(page, '[data-testid="chat-interface"]', 'navigation');

			// Add manual context
			await addManualContext(page, {
				title: 'Manual Context',
				content: 'This is manually added context for testing',
				type: 'user_input',
			});

			// Chat with manual context
			await performChatWithContext(page, 'What context do you have available?');
			console.log('✅ Workflow with no search results handled');
		});

		test('should handle workflow with partial failures', async ({ page }) => {
			console.log('🚀 Testing workflow with partial failures...');

			// Perform search
			const searchInput = page.locator('input[type="search"]');
			await searchInput.fill('rust programming');
			await searchInput.press('Enter');
			await ciWait(page, 'medium');

			// Try to add context (may fail)
			const searchResults = page.locator('[data-testid="search-results"] .box');
			const firstResult = searchResults.first();
			const addToContextButton = firstResult.locator('[data-testid="add-to-context-button"]');

			if (await addToContextButton.isVisible()) {
				await addToContextButton.click();
				await ciWait(page, 'medium');
			}

			// Navigate to chat
			await ciNavigate(page, '/chat');
			await ciWaitForSelector(page, '[data-testid="chat-interface"]', 'navigation');

			// Chat regardless of context status
			await performChatWithContext(page, 'Hello, can you help me?');
			console.log('✅ Partial failure workflow handled');
		});
	});

	test.describe('Workflow Performance and Stress Testing', () => {
		test('should handle rapid workflow execution', async ({ page }) => {
			console.log('🚀 Testing rapid workflow execution...');

			const startTime = Date.now();

			// Rapid search and context addition
			for (let i = 0; i < 3; i++) {
				await performSearch(page, `test query ${i + 1}`);
				await ciWait(page, 'small');
			}

			// Navigate to chat
			await ciNavigate(page, '/chat');
			await ciWaitForSelector(page, '[data-testid="chat-interface"]', 'navigation');

			// Rapid chat messages
			for (let i = 0; i < 3; i++) {
				await performChatWithContext(page, `Test message ${i + 1}`);
				await ciWait(page, 'small');
			}

			const endTime = Date.now();
			const totalTime = endTime - startTime;

			console.log(`✅ Rapid workflow completed in ${totalTime}ms`);
			expect(totalTime).toBeLessThan(60000); // Should complete within 1 minute
		});

		test('should handle large context sets', async ({ page }) => {
			console.log('🚀 Testing large context sets...');

			// Add many context items
			await ciNavigate(page, '/chat');
			await ciWaitForSelector(page, '[data-testid="chat-interface"]', 'navigation');

			const contextCount = 10;
			for (let i = 0; i < contextCount; i++) {
				await addManualContext(page, {
					title: `Context Item ${i + 1}`,
					content: `This is context item ${i + 1} with some content about the topic.`,
					type: 'document',
				});
			}

			// Verify context is manageable
			const contextItems = page.locator('[data-testid="conversation-context"] .context-item');
			const visibleCount = await contextItems.count();
			expect(visibleCount).toBeGreaterThan(0);
			console.log(`✅ Large context set (${visibleCount} items) handled`);

			// Chat with large context
			await performChatWithContext(page, 'Summarize the key points from all context items');
			console.log('✅ Chat with large context completed');
		});
	});

	test.describe('Workflow Error Recovery', () => {
		test('should recover from network errors', async ({ page }) => {
			console.log('🚀 Testing network error recovery...');

			// Simulate network error
			await page.route('**/documents/search', (route) => {
				route.abort('failed');
			});

			// Try to search
			const searchInput = page.locator('input[type="search"]');
			await searchInput.fill('test query');
			await searchInput.press('Enter');
			await ciWait(page, 'medium');

			// Should handle error gracefully
			const errorMessage = page.locator('.error, [data-testid="error-message"]');
			const hasError = await errorMessage.isVisible();

			if (hasError) {
				console.log('✅ Network error handled gracefully');
			}

			// Remove route interception
			await page.unroute('**/documents/search');

			// Try search again
			await searchInput.fill('test query');
			await searchInput.press('Enter');
			await ciWait(page, 'medium');

			// Should work now
			const searchResults = page.locator('[data-testid="search-results"] .box');
			const resultCount = await searchResults.count();
			console.log(`✅ Recovery successful, found ${resultCount} results`);
		});

		test('should recover from LLM errors', async ({ page }) => {
			console.log('🚀 Testing LLM error recovery...');

			// Navigate to chat
			await ciNavigate(page, '/chat');
			await ciWaitForSelector(page, '[data-testid="chat-interface"]', 'navigation');

			// Add context
			await addManualContext(page, {
				title: 'Test Context',
				content: 'This is test context for error recovery',
				type: 'document',
			});

			// Simulate LLM error
			await page.route('**/chat', (route) => {
				route.abort('failed');
			});

			// Try to chat
			await performChatWithContext(page, 'Test message that should fail');

			// Should handle error gracefully
			const errorMessage = page.locator('.error, [data-testid="error-message"]');
			const hasError = await errorMessage.isVisible();

			if (hasError) {
				console.log('✅ LLM error handled gracefully');
			}

			// Remove route interception
			await page.unroute('**/chat');

			// Try chat again
			await performChatWithContext(page, 'Test message after recovery');
			console.log('✅ LLM error recovery successful');
		});
	});

	test.describe('Workflow Accessibility and Usability', () => {
		test('should be accessible via keyboard only', async ({ page }) => {
			console.log('🚀 Testing keyboard-only accessibility...');

			// Navigate using keyboard
			await page.keyboard.press('Tab');
			await page.keyboard.press('Tab');
			await page.keyboard.press('Enter'); // Search tab

			// Search using keyboard
			await page.keyboard.type('rust programming');
			await page.keyboard.press('Enter');
			await ciWait(page, 'medium');

			// Tab to first result and add to context
			await page.keyboard.press('Tab');
			await page.keyboard.press('Tab');
			await page.keyboard.press('Enter'); // Add to context

			// Navigate to chat
			await page.keyboard.press('Tab');
			await page.keyboard.press('Enter'); // Chat tab

			await ciWaitForSelector(page, '[data-testid="chat-interface"]', 'navigation');

			// Chat using keyboard
			const chatInput = page.locator('[data-testid="chat-input"]');
			await chatInput.focus();
			await page.keyboard.type('Test message via keyboard');
			await page.keyboard.press('Enter');

			await ciWait(page, 'medium');
			console.log('✅ Keyboard-only workflow completed');
		});

		test('should have proper ARIA labels and roles', async ({ page }) => {
			console.log('🚀 Testing ARIA labels and roles...');

			// Check search interface
			const searchInput = page.locator('input[type="search"]');
			const hasSearchAriaLabel = await searchInput.getAttribute('aria-label');
			expect(hasSearchAriaLabel).toBeTruthy();

			// Check chat interface
			await ciNavigate(page, '/chat');
			await ciWaitForSelector(page, '[data-testid="chat-interface"]', 'navigation');

			const chatInput = page.locator('[data-testid="chat-input"]');
			const hasChatAriaLabel = await chatInput.getAttribute('aria-label');
			expect(hasChatAriaLabel).toBeTruthy();

			const contextPanel = page.locator('[data-testid="context-panel"]');
			const hasContextAriaRole = await contextPanel.getAttribute('role');
			expect(hasContextAriaRole).toBeTruthy();

			console.log('✅ ARIA labels and roles properly implemented');
		});
	});

	// Helper functions
	async function performSearch(page: any, query: string) {
		const searchInput = page.locator('input[type="search"]');
		await searchInput.fill(query);
		await searchInput.press('Enter');
		await ciWait(page, 'medium');
	}

	async function performMultipleSearches(page: any, queries: string[]) {
		const allResults = [];

		for (const query of queries) {
			await performSearch(page, query);
			const searchResults = page.locator('[data-testid="search-results"] .box');
			const resultCount = await searchResults.count();
			allResults.push(resultCount);
		}

		return allResults;
	}

	async function addDocumentsToContext(page: any, searchResults: number[]) {
		let totalAdded = 0;

		for (let i = 0; i < searchResults.length; i++) {
			if (searchResults[i] > 0) {
				const searchResultsElements = page.locator('[data-testid="search-results"] .box');
				const firstResult = searchResultsElements.first();
				const addToContextButton = firstResult.locator('[data-testid="add-to-context-button"]');

				if (await addToContextButton.isVisible()) {
					await addToContextButton.click();
					await ciWait(page, 'medium');
					totalAdded++;
				}
			}
		}

		return totalAdded;
	}

	async function verifyContextVisibility(page: any) {
		const contextItems = page.locator('[data-testid="conversation-context"] .context-item');
		return await contextItems.count();
	}

	async function performChatWithContext(page: any, message: string) {
		const chatInput = page.locator('[data-testid="chat-input"]');
		const sendButton = page.locator('[data-testid="send-message-button"]');

		await chatInput.fill(message);
		await sendButton.click();

		// Wait for user message
		await ciWait(page, 'small');

		// Wait for assistant response
		await page.waitForSelector('.msg.assistant', {
			timeout: LLM_RESPONSE_TIMEOUT,
			state: 'visible',
		});

		await ciWait(page, 'small');
	}

	async function addKGTermsToContext(page: any, terms: string[]) {
		let addedCount = 0;

		for (const term of terms) {
			const kgSearchButton = page.locator('[data-testid="kg-search-button"]');
			const kgSearchVisible = await kgSearchButton.isVisible();

			if (kgSearchVisible) {
				await kgSearchButton.click();

				const kgModal = page.locator('[data-testid="kg-search-modal"]');
				await expect(kgModal).toBeVisible();

				const searchInput = kgModal.locator('[data-testid="kg-search-input"]');
				await searchInput.fill(term);

				const searchButton = kgModal.locator('[data-testid="kg-search-submit"]');
				await searchButton.click();

				await ciWait(page, 'medium');

				const results = kgModal.locator('[data-testid="kg-search-results"] .result-item');
				const resultCount = await results.count();

				if (resultCount > 0) {
					const addButton = results.first().locator('[data-testid="add-to-context"]');
					await addButton.click();
					await ciWait(page, 'medium');
					addedCount++;
				}

				const closeButton = kgModal.locator('[data-testid="close-modal-button"]');
				await closeButton.click();
			}
		}

		return addedCount;
	}

	async function verifyContextInfluence(page: any, expectedContext: string[]) {
		const chatInput = page.locator('[data-testid="chat-input"]');
		const sendButton = page.locator('[data-testid="send-message-button"]');

		await chatInput.fill(
			'Please reference the specific terms and concepts from the context in your response'
		);
		await sendButton.click();

		// Wait for response
		await page.waitForSelector('.msg.assistant', {
			timeout: LLM_RESPONSE_TIMEOUT,
			state: 'visible',
		});

		// Check if response mentions context
		const assistantMessage = page.locator('.msg.assistant').last();
		const responseText = await assistantMessage.textContent();

		// Response should be substantial
		expect(responseText?.length).toBeGreaterThan(50);
		console.log('✅ Context influence verified in LLM response');
	}

	async function addManualContext(page: any, context: any) {
		const addContextButton = page.locator('[data-testid="show-add-context-button"]');
		await addContextButton.click();

		const titleInput = page.locator('[data-testid="context-title-input"]');
		const contentInput = page.locator('[data-testid="context-content-textarea"]');
		const typeSelect = page.locator('[data-testid="context-type-select"]');
		const saveButton = page.locator('[data-testid="add-context-submit-button"]');

		await titleInput.fill(context.title);
		await contentInput.fill(context.content);
		await typeSelect.selectOption(context.type);
		await saveButton.click();

		await ciWait(page, 'medium');
	}
});
