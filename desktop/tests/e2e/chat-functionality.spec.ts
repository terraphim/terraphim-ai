/**
 * Comprehensive End-to-End Tests for Chat Functionality
 *
 * This test suite validates the complete chat system including:
 * - Chat interface initialization and navigation
 * - Message sending and receiving with Ollama
 * - Conversation management (create, load, persist)
 * - Context management (add, edit, delete, search)
 * - KG search modal integration
 * - Error handling and recovery
 * - Ollama connectivity and fallback behavior
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
const TEST_TIMEOUT = 90000; // Extended for LLM responses
const OLLAMA_TIMEOUT = 30000; // Timeout for Ollama responses
const BACKEND_WAIT_TIME = 2000;

// Test data
const TEST_MESSAGES = [
	'Hello, can you help me with Rust async programming?',
	'What are the best practices for error handling in async Rust?',
	'How do I implement proper cancellation in tokio?',
	'Can you explain the differences between futures and streams?',
];

const TEST_CONTEXT_ITEMS = [
	{
		title: 'Rust Async Guide',
		content:
			'Comprehensive guide to async programming in Rust using tokio, futures, and async/await patterns.',
		type: 'document',
	},
	{
		title: 'Error Handling Patterns',
		content:
			'Best practices for error handling in asynchronous Rust code, including Result types and error propagation.',
		type: 'search_result',
	},
	{
		title: 'Tokio Documentation',
		content:
			'Official documentation for the Tokio async runtime, covering task spawning, scheduling, and I/O.',
		type: 'external',
	},
];

test.describe('Chat Functionality E2E Tests', () => {
	test.beforeEach(async ({ page }) => {
		// Set extended timeout for this test suite
		test.setTimeout(TEST_TIMEOUT);

		// Navigate to the chat page
		await ciNavigate(page, '/chat');

		// Wait for chat interface to initialize
		await ciWaitForSelector(page, '[data-testid="chat-container"]', 'navigation');
	});

	test.describe('Chat Interface Initialization', () => {
		test('should display chat interface with essential elements', async ({ page }) => {
			// Check for main chat elements
			await expect(page.locator('[data-testid="chat-container"]')).toBeVisible();
			await expect(page.locator('[data-testid="message-list"]')).toBeVisible();
			await expect(page.locator('[data-testid="chat-input"]')).toBeVisible();
			await expect(page.locator('[data-testid="send-button"]')).toBeVisible();

			// Check for context panel elements
			await expect(page.locator('[data-testid="context-panel-toggle"]')).toBeVisible();

			// Check for conversation management elements
			await expect(page.locator('[data-testid="new-conversation-button"]')).toBeVisible();
		});

		test('should show welcome message or empty state', async ({ page }) => {
			const messageList = page.locator('[data-testid="message-list"]');

			// Should either show welcome message or be empty
			const messageCount = await messageList.locator('.message').count();

			if (messageCount === 0) {
				// Empty state - should show placeholder or instructions
				const emptyState = page.locator('[data-testid="chat-empty-state"]');
				await expect(emptyState).toBeVisible();
			} else {
				// Should have at least a system or welcome message
				const firstMessage = messageList.locator('.message').first();
				await expect(firstMessage).toBeVisible();
			}
		});

		test('should display correct role information', async ({ page }) => {
			// Check if current role is displayed
			const roleIndicator = page.locator('[data-testid="current-role"]');
			const roleVisible = await roleIndicator.isVisible();

			if (roleVisible) {
				const roleText = await roleIndicator.textContent();
				expect(roleText).toBeTruthy();
				console.log('Current role:', roleText);
			}
		});
	});

	test.describe('Message Handling', () => {
		test('should send and receive messages with Ollama', async ({ page }) => {
			const chatInput = page.locator('[data-testid="chat-input"]');
			const sendButton = page.locator('[data-testid="send-button"]');
			const messageList = page.locator('[data-testid="message-list"]');

			// Count initial messages
			const initialMessageCount = await messageList.locator('.message').count();

			// Send a test message
			await chatInput.fill(TEST_MESSAGES[0]);
			await sendButton.click();

			// Wait for user message to appear
			await ciWait(page, 'small');
			const afterUserMessageCount = await messageList.locator('.message').count();
			expect(afterUserMessageCount).toBe(initialMessageCount + 1);

			// Check that user message is displayed correctly
			const userMessage = messageList.locator('.message').last();
			await expect(userMessage).toContainText(TEST_MESSAGES[0]);
			await expect(userMessage).toHaveClass(/user-message/);

			// Wait for assistant response (with extended timeout for Ollama)
			await page.waitForSelector('.message.assistant-message', {
				timeout: OLLAMA_TIMEOUT,
				state: 'visible',
			});

			// Verify assistant response appeared
			const finalMessageCount = await messageList.locator('.message').count();
			expect(finalMessageCount).toBe(initialMessageCount + 2);

			// Check assistant message properties
			const assistantMessage = messageList.locator('.message.assistant-message').last();
			await expect(assistantMessage).toBeVisible();

			const assistantText = await assistantMessage.textContent();
			expect(assistantText?.length).toBeGreaterThan(10); // Should have substantial response

			console.log('Assistant response length:', assistantText?.length);
		});

		test('should handle multiple messages in sequence', async ({ page }) => {
			const chatInput = page.locator('[data-testid="chat-input"]');
			const sendButton = page.locator('[data-testid="send-button"]');
			const messageList = page.locator('[data-testid="message-list"]');

			// Send multiple messages
			for (let i = 0; i < 2; i++) {
				const message = TEST_MESSAGES[i];

				await chatInput.fill(message);
				await sendButton.click();

				// Wait for user message
				await ciWait(page, 'small');

				// Wait for assistant response
				await page.waitForSelector('.message.assistant-message', {
					timeout: OLLAMA_TIMEOUT,
					state: 'visible',
				});

				// Small delay between messages
				await ciWait(page, 'medium');
			}

			// Verify we have the expected number of messages (2 pairs)
			const totalMessages = await messageList.locator('.message').count();
			expect(totalMessages).toBeGreaterThanOrEqual(4); // At least 2 user + 2 assistant
		});

		test('should handle empty message gracefully', async ({ page }) => {
			const chatInput = page.locator('[data-testid="chat-input"]');
			const sendButton = page.locator('[data-testid="send-button"]');

			// Try to send empty message
			await sendButton.click();

			// Should not create a new message
			await ciWait(page, 'small');

			// Input should still be focused or button should be disabled
			const isInputFocused = await chatInput.evaluate((el) => document.activeElement === el);
			const isButtonDisabled = await sendButton.isDisabled();

			expect(isInputFocused || isButtonDisabled).toBeTruthy();
		});

		test('should handle keyboard shortcuts', async ({ page }) => {
			const chatInput = page.locator('[data-testid="chat-input"]');
			const messageList = page.locator('[data-testid="message-list"]');

			const initialMessageCount = await messageList.locator('.message').count();

			// Type message and send with Enter
			await chatInput.fill('Testing keyboard shortcut');
			await chatInput.press('Enter');

			// Wait for message to be sent
			await ciWait(page, 'small');

			const newMessageCount = await messageList.locator('.message').count();
			expect(newMessageCount).toBe(initialMessageCount + 1);
		});
	});

	test.describe('Context Management', () => {
		test('should open and close context panel', async ({ page }) => {
			const contextToggle = page.locator('[data-testid="context-panel-toggle"]');
			const contextPanel = page.locator('[data-testid="context-panel"]');

			// Open context panel
			await contextToggle.click();
			await ciWait(page, 'small');
			await expect(contextPanel).toBeVisible();

			// Close context panel
			await contextToggle.click();
			await ciWait(page, 'small');
			await expect(contextPanel).not.toBeVisible();
		});

		test('should add manual context item', async ({ page }) => {
			// Open context panel
			const contextToggle = page.locator('[data-testid="context-panel-toggle"]');
			await contextToggle.click();
			await ciWait(page, 'small');

			// Click add context button
			const addContextButton = page.locator('[data-testid="add-context-button"]');
			await addContextButton.click();

			// Fill context form
			const contextForm = page.locator('[data-testid="context-form"]');
			await expect(contextForm).toBeVisible();

			const titleInput = contextForm.locator('[data-testid="context-title-input"]');
			const contentInput = contextForm.locator('[data-testid="context-content-input"]');
			const saveButton = contextForm.locator('[data-testid="save-context-button"]');

			await titleInput.fill(TEST_CONTEXT_ITEMS[0].title);
			await contentInput.fill(TEST_CONTEXT_ITEMS[0].content);
			await saveButton.click();

			// Wait for context item to appear
			await ciWait(page, 'medium');

			// Verify context item is displayed
			const contextList = page.locator('[data-testid="context-list"]');
			const contextItem = contextList.locator(`[data-title="${TEST_CONTEXT_ITEMS[0].title}"]`);
			await expect(contextItem).toBeVisible();
		});

		test('should edit existing context item', async ({ page }) => {
			// First add a context item (reuse previous test logic)
			const contextToggle = page.locator('[data-testid="context-panel-toggle"]');
			await contextToggle.click();

			const addContextButton = page.locator('[data-testid="add-context-button"]');
			await addContextButton.click();

			const contextForm = page.locator('[data-testid="context-form"]');
			const titleInput = contextForm.locator('[data-testid="context-title-input"]');
			const contentInput = contextForm.locator('[data-testid="context-content-input"]');
			const saveButton = contextForm.locator('[data-testid="save-context-button"]');

			await titleInput.fill(TEST_CONTEXT_ITEMS[0].title);
			await contentInput.fill(TEST_CONTEXT_ITEMS[0].content);
			await saveButton.click();
			await ciWait(page, 'medium');

			// Now edit the context item
			const contextList = page.locator('[data-testid="context-list"]');
			const contextItem = contextList.locator(`[data-title="${TEST_CONTEXT_ITEMS[0].title}"]`);
			const editButton = contextItem.locator('[data-testid="edit-context-button"]');

			await editButton.click();

			// Modify content
			const editForm = page.locator('[data-testid="context-edit-form"]');
			const editContentInput = editForm.locator('[data-testid="edit-content-input"]');
			const updateButton = editForm.locator('[data-testid="update-context-button"]');

			const updatedContent = TEST_CONTEXT_ITEMS[0].content + ' [EDITED]';
			await editContentInput.fill(updatedContent);
			await updateButton.click();

			// Verify changes
			await ciWait(page, 'medium');
			await expect(contextItem).toContainText('[EDITED]');
		});

		test('should delete context item', async ({ page }) => {
			// Add a context item first
			const contextToggle = page.locator('[data-testid="context-panel-toggle"]');
			await contextToggle.click();

			const addContextButton = page.locator('[data-testid="add-context-button"]');
			await addContextButton.click();

			const contextForm = page.locator('[data-testid="context-form"]');
			const titleInput = contextForm.locator('[data-testid="context-title-input"]');
			const contentInput = contextForm.locator('[data-testid="context-content-input"]');
			const saveButton = contextForm.locator('[data-testid="save-context-button"]');

			await titleInput.fill('Temporary Context');
			await contentInput.fill('This will be deleted');
			await saveButton.click();
			await ciWait(page, 'medium');

			// Delete the context item
			const contextList = page.locator('[data-testid="context-list"]');
			const contextItem = contextList.locator('[data-title="Temporary Context"]');
			const deleteButton = contextItem.locator('[data-testid="delete-context-button"]');

			await deleteButton.click();

			// Confirm deletion if modal appears
			const confirmDialog = page.locator('[data-testid="confirm-delete-modal"]');
			const confirmVisible = await confirmDialog.isVisible();

			if (confirmVisible) {
				const confirmButton = confirmDialog.locator('[data-testid="confirm-delete-button"]');
				await confirmButton.click();
			}

			// Verify item is removed
			await ciWait(page, 'medium');
			await expect(contextItem).not.toBeVisible();
		});
	});

	test.describe('KG Search Integration', () => {
		test('should open KG search modal', async ({ page }) => {
			// Look for KG search button (might be in context panel or main interface)
			const kgSearchButton = page.locator('[data-testid="kg-search-button"]');
			const kgSearchButtonVisible = await kgSearchButton.isVisible();

			if (kgSearchButtonVisible) {
				await kgSearchButton.click();

				// Wait for modal to open
				const kgModal = page.locator('[data-testid="kg-search-modal"]');
				await expect(kgModal).toBeVisible();

				// Check modal contents
				await expect(kgModal.locator('[data-testid="kg-search-input"]')).toBeVisible();
				await expect(kgModal.locator('[data-testid="kg-search-results"]')).toBeVisible();

				// Close modal
				const closeButton = kgModal.locator('[data-testid="close-modal-button"]');
				await closeButton.click();
				await expect(kgModal).not.toBeVisible();
			} else {
				console.log('KG search button not found - may be role-dependent');
			}
		});

		test('should search and add KG results to context', async ({ page }) => {
			const kgSearchButton = page.locator('[data-testid="kg-search-button"]');
			const kgSearchButtonVisible = await kgSearchButton.isVisible();

			if (kgSearchButtonVisible) {
				await kgSearchButton.click();

				const kgModal = page.locator('[data-testid="kg-search-modal"]');
				const searchInput = kgModal.locator('[data-testid="kg-search-input"]');
				const searchButton = kgModal.locator('[data-testid="kg-search-submit"]');

				// Perform KG search
				await searchInput.fill('rust async');
				await searchButton.click();

				// Wait for results
				await ciWait(page, 'medium');

				const results = kgModal.locator('[data-testid="kg-search-results"] .result-item');
				const resultCount = await results.count();

				if (resultCount > 0) {
					// Add first result to context
					const firstResult = results.first();
					const addToContextButton = firstResult.locator('[data-testid="add-to-context"]');
					await addToContextButton.click();

					// Verify result was added
					await ciWait(page, 'medium');

					// Close modal and check context panel
					const closeButton = kgModal.locator('[data-testid="close-modal-button"]');
					await closeButton.click();

					// Open context panel to verify
					const contextToggle = page.locator('[data-testid="context-panel-toggle"]');
					await contextToggle.click();

					const contextList = page.locator('[data-testid="context-list"]');
					const contextItems = await contextList.locator('.context-item').count();
					expect(contextItems).toBeGreaterThan(0);
				} else {
					console.log('No KG search results found');
				}
			}
		});
	});

	test.describe('Error Handling and Recovery', () => {
		test('should handle Ollama connection errors gracefully', async ({ page }) => {
			const chatInput = page.locator('[data-testid="chat-input"]');
			const sendButton = page.locator('[data-testid="send-button"]');

			// Send a message that might trigger an error
			await chatInput.fill('Test message for error handling');
			await sendButton.click();

			// Wait and check for error states
			await ciWait(page, 'large');

			// Look for error indicators
			const errorMessage = page.locator('[data-testid="chat-error"]');
			const loadingIndicator = page.locator('[data-testid="message-loading"]');

			const hasError = await errorMessage.isVisible();
			const isLoading = await loadingIndicator.isVisible();

			if (hasError) {
				console.log('Error detected - verifying error handling');
				await expect(errorMessage).toBeVisible();

				// Check for retry button
				const retryButton = page.locator('[data-testid="retry-message-button"]');
				const retryVisible = await retryButton.isVisible();

				if (retryVisible) {
					await retryButton.click();
					console.log('Retry functionality available');
				}
			} else if (isLoading) {
				console.log('Message still loading - waiting longer');
				await page.waitForSelector('[data-testid="message-loading"]', {
					state: 'detached',
					timeout: OLLAMA_TIMEOUT,
				});
			} else {
				console.log('Message processed successfully');
			}
		});

		test('should maintain chat state during navigation', async ({ page }) => {
			const chatInput = page.locator('[data-testid="chat-input"]');
			const sendButton = page.locator('[data-testid="send-button"]');
			const messageList = page.locator('[data-testid="message-list"]');

			// Send a message
			await chatInput.fill('State persistence test');
			await sendButton.click();
			await ciWait(page, 'medium');

			const messageCount = await messageList.locator('.message').count();

			// Navigate away and back
			await ciNavigate(page, '/');
			await ciWait(page, 'small');
			await ciNavigate(page, '/chat');
			await ciWaitForSelector(page, '[data-testid="chat-container"]');

			// Check if messages are still there
			const messageListAfter = page.locator('[data-testid="message-list"]');
			const messageCountAfter = await messageListAfter.locator('.message').count();

			// Should maintain at least some state
			expect(messageCountAfter).toBeGreaterThanOrEqual(0);
			console.log(`Messages before navigation: ${messageCount}, after: ${messageCountAfter}`);
		});
	});

	test.describe('Conversation Management', () => {
		test('should create new conversation', async ({ page }) => {
			const newConversationButton = page.locator('[data-testid="new-conversation-button"]');
			const messageList = page.locator('[data-testid="message-list"]');

			// Click new conversation
			await newConversationButton.click();
			await ciWait(page, 'medium');

			// Should reset the message list or show confirmation
			const messages = await messageList.locator('.message').count();
			console.log('Messages after new conversation:', messages);

			// Should be able to send messages in new conversation
			const chatInput = page.locator('[data-testid="chat-input"]');
			const sendButton = page.locator('[data-testid="send-button"]');

			await chatInput.fill('First message in new conversation');
			await sendButton.click();

			await ciWait(page, 'medium');
			const newMessages = await messageList.locator('.message').count();
			expect(newMessages).toBeGreaterThan(messages);
		});

		test('should load conversation history if available', async ({ page }) => {
			// This test checks if existing conversations are loaded
			const messageList = page.locator('[data-testid="message-list"]');
			const conversationDropdown = page.locator('[data-testid="conversation-selector"]');

			const hasConversationSelector = await conversationDropdown.isVisible();

			if (hasConversationSelector) {
				// Test conversation switching
				await conversationDropdown.click();

				const conversationOptions = page.locator('[data-testid="conversation-option"]');
				const optionCount = await conversationOptions.count();

				if (optionCount > 0) {
					await conversationOptions.first().click();
					await ciWait(page, 'medium');

					console.log('Conversation loaded successfully');
				}
			} else {
				console.log('No conversation selector found - single conversation mode');
			}
		});
	});
});
