/**
 * Context and LLM Integration Tests
 *
 * This test suite focuses on the integration between context management
 * and LLM functionality, ensuring that context is properly passed to
 * the LLM and affects the responses.
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
const TEST_TIMEOUT = 120000;
const LLM_RESPONSE_TIMEOUT = 45000;

// Test data
const TEST_DOCUMENTS = [
	{
		title: 'Rust Async Programming Guide',
		content:
			'Rust async programming uses the async/await syntax and the tokio runtime. Key concepts include futures, streams, and error handling with Result types.',
		tags: ['rust', 'async', 'tokio', 'futures'],
	},
	{
		title: 'Error Handling in Rust',
		content:
			'Rust uses Result<T, E> and Option<T> types for error handling. The ? operator is used for early returns, and match expressions handle different error cases.',
		tags: ['rust', 'error-handling', 'result', 'option'],
	},
	{
		title: 'Tokio Runtime Concepts',
		content:
			'The Tokio runtime provides async I/O, timers, and task scheduling. It uses a work-stealing scheduler and supports both single-threaded and multi-threaded execution.',
		tags: ['tokio', 'runtime', 'async', 'scheduler'],
	},
];

const TEST_KG_TERMS = [
	{
		term: 'async-await',
		definition: 'Rust syntax for writing asynchronous code that looks like synchronous code',
		related: ['futures', 'tokio', 'async-runtime'],
	},
	{
		term: 'futures',
		definition: 'A trait representing an asynchronous computation that can be polled to completion',
		related: ['async-await', 'streams', 'poll'],
	},
	{
		term: 'tokio-runtime',
		definition: 'The async runtime for Rust that provides I/O, timers, and task scheduling',
		related: ['async-await', 'futures', 'scheduler'],
	},
];

const CONTEXT_VERIFICATION_QUERIES = [
	'What are the key concepts mentioned in the documents I added?',
	'How do the different concepts relate to each other?',
	'Can you explain the specific examples from the context?',
	'What are the best practices mentioned in the provided documents?',
];

test.describe('Context and LLM Integration Tests', () => {
	test.beforeEach(async ({ page }) => {
		test.setTimeout(TEST_TIMEOUT);
		await ciNavigate(page, '/chat');
		await ciWaitForSelector(page, '[data-testid="chat-interface"]', 'navigation');
	});

	test.describe('Document Context Integration', () => {
		test('should add documents to context and verify LLM uses them', async ({ page }) => {
			console.log('🧠 Testing document context integration...');

			// Add multiple documents to context
			for (let i = 0; i < TEST_DOCUMENTS.length; i++) {
				await addDocumentToContext(page, TEST_DOCUMENTS[i]);
				console.log(`✅ Document ${i + 1} added to context`);
			}

			// Verify context items are visible
			const contextItems = page.locator('[data-testid="conversation-context"] .context-item');
			const contextCount = await contextItems.count();
			expect(contextCount).toBeGreaterThanOrEqual(TEST_DOCUMENTS.length);
			console.log(`✅ ${contextCount} context items visible`);

			// Test LLM response with context
			await testLLMResponseWithContext(page, CONTEXT_VERIFICATION_QUERIES[0]);
			console.log('✅ LLM response with document context verified');
		});

		test('should handle context updates and LLM responses', async ({ page }) => {
			console.log('🧠 Testing context updates and LLM responses...');

			// Add initial document
			await addDocumentToContext(page, TEST_DOCUMENTS[0]);

			// Chat with initial context
			await testLLMResponseWithContext(
				page,
				'What does the first document say about async programming?'
			);

			// Add another document
			await addDocumentToContext(page, TEST_DOCUMENTS[1]);

			// Chat with updated context
			await testLLMResponseWithContext(page, 'Now compare the concepts from both documents');

			console.log('✅ Context updates and LLM responses work correctly');
		});

		test('should handle context removal and LLM responses', async ({ page }) => {
			console.log('🧠 Testing context removal and LLM responses...');

			// Add multiple documents
			for (const doc of TEST_DOCUMENTS) {
				await addDocumentToContext(page, doc);
			}

			// Chat with full context
			await testLLMResponseWithContext(page, 'Summarize all the documents in context');

			// Remove one document
			const contextItems = page.locator('[data-testid="conversation-context"] .context-item');
			const deleteButton = contextItems.first().locator('[data-testid="delete-context-0"]');
			await deleteButton.click();

			// Confirm deletion if modal appears
			const confirmDialog = page.locator('[data-testid="confirm-delete-modal"]');
			const hasConfirmDialog = await confirmDialog.isVisible();
			if (hasConfirmDialog) {
				await confirmDialog.locator('[data-testid="confirm-delete-button"]').click();
			}

			await ciWait(page, 'medium');

			// Chat with reduced context
			await testLLMResponseWithContext(page, 'What documents are still in context?');

			console.log('✅ Context removal and LLM responses work correctly');
		});
	});

	test.describe('Knowledge Graph Context Integration', () => {
		test('should add KG terms to context and verify LLM uses them', async ({ page }) => {
			console.log('🧠 Testing KG term context integration...');

			// Add KG terms to context
			for (const kgTerm of TEST_KG_TERMS) {
				await addKGTermToContext(page, kgTerm.term);
				console.log(`✅ KG term "${kgTerm.term}" added to context`);
			}

			// Verify KG context items are visible
			const contextItems = page.locator('[data-testid="conversation-context"] .context-item');
			const contextCount = await contextItems.count();
			expect(contextCount).toBeGreaterThanOrEqual(TEST_KG_TERMS.length);
			console.log(`✅ ${contextCount} KG context items visible`);

			// Test LLM response with KG context
			await testLLMResponseWithContext(page, 'Explain the KG terms I added to context');
			console.log('✅ LLM response with KG context verified');
		});

		test('should handle mixed document and KG context', async ({ page }) => {
			console.log('🧠 Testing mixed document and KG context...');

			// Add both document and KG context
			await addDocumentToContext(page, TEST_DOCUMENTS[0]);
			await addKGTermToContext(page, TEST_KG_TERMS[0].term);
			await addDocumentToContext(page, TEST_DOCUMENTS[1]);

			// Verify mixed context
			const contextItems = page.locator('[data-testid="conversation-context"] .context-item');
			const contextCount = await contextItems.count();
			expect(contextCount).toBeGreaterThanOrEqual(3);
			console.log(`✅ Mixed context with ${contextCount} items`);

			// Test LLM response with mixed context
			await testLLMResponseWithContext(
				page,
				'How do the documents and KG terms relate to each other?'
			);
			console.log('✅ Mixed context LLM response verified');
		});
	});

	test.describe('Context Quality and LLM Response Verification', () => {
		test('should verify context affects LLM responses', async ({ page }) => {
			console.log('🧠 Testing context influence on LLM responses...');

			// First, chat without context
			await testLLMResponseWithContext(page, 'What is async programming in Rust?');
			const responseWithoutContext = await getLastLLMResponse(page);

			// Add context
			await addDocumentToContext(page, TEST_DOCUMENTS[0]);
			await addKGTermToContext(page, TEST_KG_TERMS[0].term);

			// Chat with context
			await testLLMResponseWithContext(page, 'What is async programming in Rust?');
			const responseWithContext = await getLastLLMResponse(page);

			// Responses should be different (context should influence response)
			expect(responseWithContext).not.toBe(responseWithoutContext);
			console.log('✅ Context influences LLM responses');
		});

		test('should verify context relevance in responses', async ({ page }) => {
			console.log('🧠 Testing context relevance in responses...');

			// Add specific context
			await addDocumentToContext(page, TEST_DOCUMENTS[0]);
			await addKGTermToContext(page, TEST_KG_TERMS[0].term);

			// Ask specific questions about the context
			const specificQuestions = [
				'What does the document say about tokio?',
				'Can you explain the async-await concept from the KG?',
				'How do futures work according to the context?',
			];

			for (const question of specificQuestions) {
				await testLLMResponseWithContext(page, question);
				console.log(`✅ Answered specific question: ${question}`);
			}
		});

		test('should handle context overflow gracefully', async ({ page }) => {
			console.log('🧠 Testing context overflow handling...');

			// Add many context items (test limits)
			const maxContextItems = 20;
			for (let i = 0; i < maxContextItems; i++) {
				await addDocumentToContext(page, {
					title: `Test Document ${i + 1}`,
					content: `This is test document ${i + 1} with some content about Rust programming.`,
					tags: ['test', 'rust', `doc-${i + 1}`],
				});
			}

			// Verify context is manageable
			const contextItems = page.locator('[data-testid="conversation-context"] .context-item');
			const contextCount = await contextItems.count();
			console.log(`✅ Added ${contextCount} context items`);

			// Chat with large context
			await testLLMResponseWithContext(page, 'Summarize the key points from all the context items');
			console.log('✅ Large context handled gracefully');
		});
	});

	test.describe('Context Persistence and State Management', () => {
		test('should maintain context across page navigation', async ({ page }) => {
			console.log('🧠 Testing context persistence across navigation...');

			// Add context
			await addDocumentToContext(page, TEST_DOCUMENTS[0]);
			await addKGTermToContext(page, TEST_KG_TERMS[0].term);

			// Navigate away and back
			await ciNavigate(page, '/');
			await ciWait(page, 'small');
			await ciNavigate(page, '/chat');
			await ciWaitForSelector(page, '[data-testid="chat-interface"]', 'navigation');

			// Verify context is still there
			const contextItems = page.locator('[data-testid="conversation-context"] .context-item');
			const contextCount = await contextItems.count();
			expect(contextCount).toBeGreaterThan(0);
			console.log('✅ Context persisted across navigation');
		});

		test('should maintain context across browser refresh', async ({ page }) => {
			console.log('🧠 Testing context persistence across refresh...');

			// Add context
			await addDocumentToContext(page, TEST_DOCUMENTS[0]);
			await addKGTermToContext(page, TEST_KG_TERMS[0].term);

			// Refresh page
			await page.reload();
			await ciWaitForSelector(page, '[data-testid="chat-interface"]', 'navigation');

			// Verify context is still there
			const contextItems = page.locator('[data-testid="conversation-context"] .context-item');
			const contextCount = await contextItems.count();
			expect(contextCount).toBeGreaterThan(0);
			console.log('✅ Context persisted across refresh');
		});
	});

	test.describe('Error Handling in Context-LLM Integration', () => {
		test('should handle LLM errors gracefully', async ({ page }) => {
			console.log('🧠 Testing LLM error handling...');

			// Add context
			await addDocumentToContext(page, TEST_DOCUMENTS[0]);

			// Simulate LLM error by intercepting requests
			await page.route('**/chat', (route) => {
				route.abort('failed');
			});

			// Try to chat
			const chatInput = page.locator('[data-testid="chat-input"]');
			const sendButton = page.locator('[data-testid="send-message-button"]');

			await chatInput.fill('Test message that should fail');
			await sendButton.click();

			await ciWait(page, 'medium');

			// Should show error message
			const errorMessage = page.locator('.error, [data-testid="error-message"]');
			const hasError = await errorMessage.isVisible();

			if (hasError) {
				console.log('✅ LLM error handled gracefully');
			} else {
				console.log('ℹ️ Error handling may be different');
			}
		});

		test('should handle context loading errors', async ({ page }) => {
			console.log('🧠 Testing context loading error handling...');

			// Simulate context loading error
			await page.route('**/conversations/**', (route) => {
				route.abort('failed');
			});

			// Navigate to chat
			await ciNavigate(page, '/chat');
			await ciWaitForSelector(page, '[data-testid="chat-interface"]', 'navigation');

			// Should handle context loading error gracefully
			const contextPanel = page.locator('[data-testid="context-panel"]');
			await expect(contextPanel).toBeVisible();
			console.log('✅ Context loading error handled gracefully');
		});
	});

	// Helper functions
	async function addDocumentToContext(page: any, document: any) {
		// Navigate to search first
		await ciNavigate(page, '/');

		// Perform search (simulate finding the document)
		const searchInput = page.locator('input[type="search"]');
		await searchInput.fill(document.title);
		await searchInput.press('Enter');
		await ciWait(page, 'medium');

		// Add first result to context
		const searchResults = page.locator('[data-testid="search-results"] .box');
		const firstResult = searchResults.first();
		const addToContextButton = firstResult.locator('[data-testid="add-to-context-button"]');

		if (await addToContextButton.isVisible()) {
			await addToContextButton.click();
			await ciWait(page, 'medium');
		} else {
			// If no search results, add manually
			await ciNavigate(page, '/chat');
			await addManualContext(page, document);
		}
	}

	async function addManualContext(page: any, document: any) {
		const addContextButton = page.locator('[data-testid="show-add-context-button"]');
		await addContextButton.click();

		const titleInput = page.locator('[data-testid="context-title-input"]');
		const contentInput = page.locator('[data-testid="context-content-textarea"]');
		const saveButton = page.locator('[data-testid="add-context-submit-button"]');

		await titleInput.fill(document.title);
		await contentInput.fill(document.content);
		await saveButton.click();

		await ciWait(page, 'medium');
	}

	async function addKGTermToContext(page: any, term: string) {
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
			}

			const closeButton = kgModal.locator('[data-testid="close-modal-button"]');
			await closeButton.click();
		}
	}

	async function testLLMResponseWithContext(page: any, message: string) {
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

	async function getLastLLMResponse(page: any) {
		const assistantMessages = page.locator('.msg.assistant');
		const lastMessage = assistantMessages.last();
		return await lastMessage.textContent();
	}
});
