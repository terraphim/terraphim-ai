/**
 * Frontend integration tests for Context Management UI
 *
 * This test suite validates the frontend components for conversation and context management.
 * These tests serve both as validation and as specification for the required UI components.
 *
 * Test coverage includes:
 * - Conversation creation and management
 * - Message addition and display
 * - Context attachment from search results
 * - Manual context addition
 * - Context visualization and editing
 * - Integration with Tauri backend commands
 */

import { test, expect } from '@playwright/test';

// Test fixtures
const TEST_CONVERSATIONS = [
	{
		title: 'Rust Programming Help',
		role: 'Engineer',
		initialMessage: 'I need help with Rust async programming',
		searchQuery: 'rust async tokio',
		expectedContext: ['tokio', 'futures', 'async'],
	},
	{
		title: 'System Design Discussion',
		role: 'System Operator',
		initialMessage: 'How do I design a distributed system?',
		searchQuery: 'distributed systems architecture',
		expectedContext: ['microservices', 'scalability', 'load balancing'],
	},
];

const MOCK_SEARCH_RESULTS = [
	{
		id: 'doc-1',
		title: 'Rust Async Programming Guide',
		body: 'Complete guide to asynchronous programming in Rust using tokio, futures, and async/await.',
		url: 'https://example.com/rust-async',
		rank: 95,
	},
	{
		id: 'doc-2',
		title: 'Tokio Runtime Concepts',
		body: 'Understanding the Tokio runtime, task scheduling, and concurrent execution patterns.',
		url: 'https://example.com/tokio-runtime',
		rank: 88,
	},
	{
		id: 'doc-3',
		title: 'Distributed Systems Architecture',
		body: 'Principles of designing scalable distributed systems with microservices.',
		url: 'https://example.com/distributed-systems',
		rank: 92,
	},
];

test.describe('Context Management UI', () => {
	test.beforeEach(async ({ page }) => {
		// Navigate to the application
		await page.goto('http://localhost:5173');

		// Wait for the app to load
		await page.waitForLoadState('networkidle');
	});

	test.describe('Conversation Management', () => {
		test('should create new conversation', async ({ page }) => {
			// Look for the conversations or chat section
			// This assumes there will be a conversations UI component
			await page.click('[data-testid="new-conversation-button"]');

			// Fill in conversation details
			await page.fill('[data-testid="conversation-title-input"]', 'Test Conversation');
			await page.selectOption('[data-testid="conversation-role-select"]', 'Engineer');

			// Create the conversation
			await page.click('[data-testid="create-conversation-confirm"]');

			// Verify conversation appears in the list
			await expect(page.locator('[data-testid="conversation-list"]')).toContainText(
				'Test Conversation'
			);

			// Verify conversation details
			await expect(page.locator('[data-testid="active-conversation-title"]')).toContainText(
				'Test Conversation'
			);
			await expect(page.locator('[data-testid="active-conversation-role"]')).toContainText(
				'Engineer'
			);
		});

		test('should list existing conversations', async ({ page }) => {
			// Navigate to conversations list
			await page.click('[data-testid="conversations-tab"]');

			// Wait for conversations to load
			await page.waitForSelector('[data-testid="conversation-list"]');

			// Check if conversations are displayed with required information
			const conversationItems = page.locator('[data-testid="conversation-item"]');
			await expect(conversationItems.first()).toBeVisible();

			// Verify conversation item contains title, role, and timestamp
			const firstConversation = conversationItems.first();
			await expect(firstConversation.locator('[data-testid="conversation-title"]')).toBeVisible();
			await expect(firstConversation.locator('[data-testid="conversation-role"]')).toBeVisible();
			await expect(firstConversation.locator('[data-testid="conversation-updated"]')).toBeVisible();
		});

		test('should open and display conversation details', async ({ page }) => {
			// Create a conversation first
			await page.click('[data-testid="new-conversation-button"]');
			await page.fill('[data-testid="conversation-title-input"]', 'Detailed Conversation');
			await page.selectOption('[data-testid="conversation-role-select"]', 'Engineer');
			await page.click('[data-testid="create-conversation-confirm"]');

			// Open the conversation
			await page.click('[data-testid="conversation-item"]:has-text("Detailed Conversation")');

			// Verify conversation view shows details
			await expect(page.locator('[data-testid="conversation-details"]')).toBeVisible();
			await expect(page.locator('[data-testid="conversation-messages"]')).toBeVisible();
			await expect(page.locator('[data-testid="conversation-context"]')).toBeVisible();
			await expect(page.locator('[data-testid="message-input"]')).toBeVisible();
		});
	});

	test.describe('Message Management', () => {
		test.beforeEach(async ({ page }) => {
			// Create a conversation for message tests
			await page.click('[data-testid="new-conversation-button"]');
			await page.fill('[data-testid="conversation-title-input"]', 'Message Test Conversation');
			await page.selectOption('[data-testid="conversation-role-select"]', 'Engineer');
			await page.click('[data-testid="create-conversation-confirm"]');
		});

		test('should add user message to conversation', async ({ page }) => {
			const testMessage = 'This is a test user message';

			// Type and send message
			await page.fill('[data-testid="message-input"]', testMessage);
			await page.click('[data-testid="send-message-button"]');

			// Verify message appears in conversation
			const messagesList = page.locator('[data-testid="conversation-messages"]');
			await expect(messagesList).toContainText(testMessage);

			// Verify message has correct role indicator
			const userMessage = page.locator(
				'[data-testid="message-item"]:has-text("' + testMessage + '")'
			);
			await expect(userMessage).toHaveAttribute('data-role', 'user');
		});

		test('should support different message roles', async ({ page }) => {
			const messages = [
				{ content: 'User message', role: 'user' },
				{ content: 'System message', role: 'system' },
				{ content: 'Assistant message', role: 'assistant' },
			];

			for (const message of messages) {
				// Add message with specific role
				await page.fill('[data-testid="message-input"]', message.content);
				await page.selectOption('[data-testid="message-role-select"]', message.role);
				await page.click('[data-testid="send-message-button"]');

				// Verify message appears with correct role
				const messageItem = page.locator(
					`[data-testid="message-item"]:has-text("${message.content}")`
				);
				await expect(messageItem).toHaveAttribute('data-role', message.role);
			}
		});

		test('should display messages in chronological order', async ({ page }) => {
			const messages = ['First message', 'Second message', 'Third message'];

			// Add messages sequentially
			for (const message of messages) {
				await page.fill('[data-testid="message-input"]', message);
				await page.click('[data-testid="send-message-button"]');
				await page.waitForTimeout(100); // Small delay to ensure order
			}

			// Verify messages appear in order
			const messageItems = page.locator('[data-testid="message-item"]');
			for (let i = 0; i < messages.length; i++) {
				await expect(messageItems.nth(i)).toContainText(messages[i]);
			}
		});
	});

	test.describe('Context Management', () => {
		test.beforeEach(async ({ page }) => {
			// Create a conversation for context tests
			await page.click('[data-testid="new-conversation-button"]');
			await page.fill('[data-testid="conversation-title-input"]', 'Context Test Conversation');
			await page.selectOption('[data-testid="conversation-role-select"]', 'Engineer');
			await page.click('[data-testid="create-conversation-confirm"]');
		});

		test('should add search results as context', async ({ page }) => {
			// Navigate to search or trigger search results
			await page.click('[data-testid="search-tab"]');
			await page.fill('[data-testid="search-input"]', 'rust async programming');
			await page.click('[data-testid="search-button"]');

			// Wait for search results
			await page.waitForSelector('[data-testid="search-results"]');

			// Select documents to add as context
			await page.check('[data-testid="search-result-checkbox"]:first-of-type');
			await page.check('[data-testid="search-result-checkbox"]:nth-of-type(2)');

			// Add selected results as context
			await page.click('[data-testid="add-to-context-button"]');
			await page.selectOption(
				'[data-testid="target-conversation-select"]',
				'Context Test Conversation'
			);
			await page.click('[data-testid="confirm-add-context"]');

			// Navigate back to conversation
			await page.click('[data-testid="conversations-tab"]');
			await page.click('[data-testid="conversation-item"]:has-text("Context Test Conversation")');

			// Verify context appears
			const contextSection = page.locator('[data-testid="conversation-context"]');
			await expect(contextSection).toBeVisible();
			await expect(contextSection.locator('[data-testid="context-item"]')).toHaveCount(2);
		});

		test('should add manual context', async ({ page }) => {
			// Navigate to conversation
			await page.click('[data-testid="conversation-item"]:has-text("Context Test Conversation")');

			// Add manual context
			await page.click('[data-testid="add-manual-context-button"]');

			// Fill in context details
			await page.selectOption('[data-testid="context-type-select"]', 'user_input');
			await page.fill('[data-testid="context-title-input"]', 'User Background');
			await page.fill(
				'[data-testid="context-content-textarea"]',
				'I am a beginner Rust programmer learning async concepts.'
			);

			// Add metadata
			await page.click('[data-testid="add-metadata-button"]');
			await page.fill('[data-testid="metadata-key-input"]:first-of-type', 'skill_level');
			await page.fill('[data-testid="metadata-value-input"]:first-of-type', 'beginner');

			// Save context
			await page.click('[data-testid="save-context-button"]');

			// Verify context appears
			const contextItem = page.locator('[data-testid="context-item"]:has-text("User Background")');
			await expect(contextItem).toBeVisible();
			await expect(contextItem).toContainText('beginner');
		});

		test('should display different context types with appropriate styling', async ({ page }) => {
			const contextTypes = [
				{ type: 'document', title: 'Test Document', icon: 'file' },
				{ type: 'search_result', title: 'Search Results', icon: 'search' },
				{ type: 'user_input', title: 'User Input', icon: 'user' },
				{ type: 'system', title: 'System Context', icon: 'cog' },
				{ type: 'external', title: 'External Source', icon: 'external-link' },
			];

			// Navigate to conversation
			await page.click('[data-testid="conversation-item"]:has-text("Context Test Conversation")');

			for (const context of contextTypes) {
				// Add context of each type
				await page.click('[data-testid="add-manual-context-button"]');
				await page.selectOption('[data-testid="context-type-select"]', context.type);
				await page.fill('[data-testid="context-title-input"]', context.title);
				await page.fill('[data-testid="context-content-textarea"]', `Content for ${context.title}`);
				await page.click('[data-testid="save-context-button"]');

				// Verify context item has appropriate styling
				const contextItem = page.locator(
					`[data-testid="context-item"]:has-text("${context.title}")`
				);
				await expect(contextItem).toHaveAttribute('data-context-type', context.type);
				await expect(contextItem.locator(`[data-testid="context-icon"]`)).toHaveClass(
					new RegExp(context.icon)
				);
			}
		});

		test('should edit existing context', async ({ page }) => {
			// Navigate to conversation and add context
			await page.click('[data-testid="conversation-item"]:has-text("Context Test Conversation")');
			await page.click('[data-testid="add-manual-context-button"]');
			await page.selectOption('[data-testid="context-type-select"]', 'document');
			await page.fill('[data-testid="context-title-input"]', 'Original Title');
			await page.fill('[data-testid="context-content-textarea"]', 'Original content');
			await page.click('[data-testid="save-context-button"]');

			// Edit the context
			const contextItem = page.locator('[data-testid="context-item"]:has-text("Original Title")');
			await contextItem.hover();
			await contextItem.locator('[data-testid="edit-context-button"]').click();

			// Update content
			await page.fill('[data-testid="context-title-input"]', 'Updated Title');
			await page.fill('[data-testid="context-content-textarea"]', 'Updated content');
			await page.click('[data-testid="save-context-button"]');

			// Verify changes
			await expect(
				page.locator('[data-testid="context-item"]:has-text("Updated Title")')
			).toBeVisible();
			await expect(page.locator('[data-testid="context-item"]')).toContainText('Updated content');
		});

		test('should remove context items', async ({ page }) => {
			// Navigate to conversation and add context
			await page.click('[data-testid="conversation-item"]:has-text("Context Test Conversation")');
			await page.click('[data-testid="add-manual-context-button"]');
			await page.fill('[data-testid="context-title-input"]', 'Context to Remove');
			await page.fill('[data-testid="context-content-textarea"]', 'This context will be removed');
			await page.click('[data-testid="save-context-button"]');

			// Verify context exists
			const contextItem = page.locator(
				'[data-testid="context-item"]:has-text("Context to Remove")'
			);
			await expect(contextItem).toBeVisible();

			// Remove context
			await contextItem.hover();
			await contextItem.locator('[data-testid="remove-context-button"]').click();
			await page.click('[data-testid="confirm-remove-context"]');

			// Verify context is removed
			await expect(contextItem).not.toBeVisible();
		});
	});

	test.describe('Context Integration with Chat', () => {
		test('should show context information in chat interface', async ({ page }) => {
			// Create conversation with context
			await page.click('[data-testid="new-conversation-button"]');
			await page.fill('[data-testid="conversation-title-input"]', 'Chat Context Test');
			await page.selectOption('[data-testid="conversation-role-select"]', 'Engineer');
			await page.click('[data-testid="create-conversation-confirm"]');

			// Add context
			await page.click('[data-testid="add-manual-context-button"]');
			await page.fill('[data-testid="context-title-input"]', 'Relevant Documentation');
			await page.fill(
				'[data-testid="context-content-textarea"]',
				'Important information for the AI assistant'
			);
			await page.click('[data-testid="save-context-button"]');

			// Start chat or send message
			await page.fill(
				'[data-testid="message-input"]',
				'Please help me based on the provided context'
			);
			await page.click('[data-testid="send-message-button"]');

			// Verify context is visible or referenced in the chat
			await expect(page.locator('[data-testid="active-context-indicator"]')).toBeVisible();
			await expect(page.locator('[data-testid="context-summary"]')).toContainText('1 context item');
		});

		test('should highlight relevant context during conversation', async ({ page }) => {
			// Setup conversation with multiple context items
			await page.click('[data-testid="new-conversation-button"]');
			await page.fill('[data-testid="conversation-title-input"]', 'Context Highlighting Test');
			await page.click('[data-testid="create-conversation-confirm"]');

			const contexts = [
				{ title: 'Rust Basics', content: 'Information about Rust fundamentals', relevant: true },
				{ title: 'Python Guide', content: 'Python programming concepts', relevant: false },
				{
					title: 'Async Programming',
					content: 'Asynchronous programming patterns',
					relevant: true,
				},
			];

			// Add multiple context items
			for (const context of contexts) {
				await page.click('[data-testid="add-manual-context-button"]');
				await page.fill('[data-testid="context-title-input"]', context.title);
				await page.fill('[data-testid="context-content-textarea"]', context.content);
				await page.click('[data-testid="save-context-button"]');
			}

			// Send a message about Rust async programming
			await page.fill('[data-testid="message-input"]', 'How do I use async/await in Rust?');
			await page.click('[data-testid="send-message-button"]');

			// Verify relevant context items are highlighted
			const rustContext = page.locator('[data-testid="context-item"]:has-text("Rust Basics")');
			const asyncContext = page.locator(
				'[data-testid="context-item"]:has-text("Async Programming")'
			);
			const pythonContext = page.locator('[data-testid="context-item"]:has-text("Python Guide")');

			await expect(rustContext).toHaveClass(/highlighted|active|relevant/);
			await expect(asyncContext).toHaveClass(/highlighted|active|relevant/);
			await expect(pythonContext).not.toHaveClass(/highlighted|active|relevant/);
		});
	});

	test.describe('Context Limits and Validation', () => {
		test('should enforce context limits', async ({ page }) => {
			// Create conversation
			await page.click('[data-testid="new-conversation-button"]');
			await page.fill('[data-testid="conversation-title-input"]', 'Context Limits Test');
			await page.click('[data-testid="create-conversation-confirm"]');

			// Try to add many context items (testing backend limits)
			const maxContextItems = 50; // Based on backend default config

			for (let i = 0; i < maxContextItems + 5; i++) {
				await page.click('[data-testid="add-manual-context-button"]');
				await page.fill('[data-testid="context-title-input"]', `Context Item ${i + 1}`);
				await page.fill('[data-testid="context-content-textarea"]', `Content for item ${i + 1}`);

				const saveButton = page.locator('[data-testid="save-context-button"]');
				await saveButton.click();

				// After reaching the limit, expect error message
				if (i >= maxContextItems) {
					await expect(page.locator('[data-testid="context-limit-error"]')).toBeVisible();
					await page.click('[data-testid="cancel-context-button"]'); // Cancel the dialog
					break;
				}
			}

			// Verify we have the maximum number of context items
			const contextItems = page.locator('[data-testid="context-item"]');
			await expect(contextItems).toHaveCount(maxContextItems);
		});

		test('should validate context input', async ({ page }) => {
			// Create conversation
			await page.click('[data-testid="new-conversation-button"]');
			await page.fill('[data-testid="conversation-title-input"]', 'Context Validation Test');
			await page.click('[data-testid="create-conversation-confirm"]');

			// Try to add context with empty required fields
			await page.click('[data-testid="add-manual-context-button"]');

			// Try to save without title
			await page.click('[data-testid="save-context-button"]');
			await expect(page.locator('[data-testid="title-required-error"]')).toBeVisible();

			// Add title but no content
			await page.fill('[data-testid="context-title-input"]', 'Valid Title');
			await page.click('[data-testid="save-context-button"]');
			await expect(page.locator('[data-testid="content-required-error"]')).toBeVisible();

			// Add valid content
			await page.fill('[data-testid="context-content-textarea"]', 'Valid content');
			await page.click('[data-testid="save-context-button"]');

			// Should succeed now
			await expect(
				page.locator('[data-testid="context-item"]:has-text("Valid Title")')
			).toBeVisible();
		});
	});

	test.describe('Context Export and Import', () => {
		test('should export conversation with context', async ({ page }) => {
			// Create conversation with context and messages
			await page.click('[data-testid="new-conversation-button"]');
			await page.fill('[data-testid="conversation-title-input"]', 'Export Test Conversation');
			await page.click('[data-testid="create-conversation-confirm"]');

			// Add context
			await page.click('[data-testid="add-manual-context-button"]');
			await page.fill('[data-testid="context-title-input"]', 'Export Context');
			await page.fill('[data-testid="context-content-textarea"]', 'Context for export test');
			await page.click('[data-testid="save-context-button"]');

			// Add message
			await page.fill('[data-testid="message-input"]', 'Test message for export');
			await page.click('[data-testid="send-message-button"]');

			// Export conversation
			await page.click('[data-testid="conversation-menu-button"]');
			await page.click('[data-testid="export-conversation-button"]');

			// Wait for download (this might need to be adjusted based on actual implementation)
			const [download] = await Promise.all([
				page.waitForEvent('download'),
				page.click('[data-testid="confirm-export-button"]'),
			]);

			// Verify download occurred
			expect(download.suggestedFilename()).toMatch(/export.*\.json$/);
		});
	});

	test.describe('Accessibility and Keyboard Navigation', () => {
		test('should support keyboard navigation for context management', async ({ page }) => {
			// Create conversation
			await page.click('[data-testid="new-conversation-button"]');
			await page.fill('[data-testid="conversation-title-input"]', 'Keyboard Navigation Test');
			await page.click('[data-testid="create-conversation-confirm"]');

			// Use Tab to navigate to add context button
			await page.keyboard.press('Tab');
			// Continue tabbing until we reach the add context button
			// This assumes proper tab order implementation
			await page.keyboard.press('Enter'); // Activate add context

			// Navigate through the form with keyboard
			await page.keyboard.type('Keyboard Context');
			await page.keyboard.press('Tab');
			await page.keyboard.press('ArrowDown'); // Select context type
			await page.keyboard.press('Tab');
			await page.keyboard.type('Context added via keyboard navigation');

			// Save with Enter
			await page.keyboard.press('Tab');
			await page.keyboard.press('Enter');

			// Verify context was added
			await expect(
				page.locator('[data-testid="context-item"]:has-text("Keyboard Context")')
			).toBeVisible();
		});

		test('should have proper ARIA labels and roles', async ({ page }) => {
			// Create conversation
			await page.click('[data-testid="new-conversation-button"]');
			await page.fill('[data-testid="conversation-title-input"]', 'Accessibility Test');
			await page.click('[data-testid="create-conversation-confirm"]');

			// Check ARIA attributes on key elements
			await expect(page.locator('[data-testid="conversation-messages"]')).toHaveAttribute(
				'role',
				'log'
			);
			await expect(page.locator('[data-testid="conversation-context"]')).toHaveAttribute(
				'role',
				'region'
			);
			await expect(page.locator('[data-testid="conversation-context"]')).toHaveAttribute(
				'aria-label',
				/context/i
			);

			// Check buttons have proper labels
			await expect(page.locator('[data-testid="add-manual-context-button"]')).toHaveAttribute(
				'aria-label'
			);
			await expect(page.locator('[data-testid="send-message-button"]')).toHaveAttribute(
				'aria-label'
			);
		});
	});
});

test.describe('Context Management Integration Tests', () => {
	test('should integrate context with backend Tauri commands', async ({ page }) => {
		// This test verifies the frontend properly calls Tauri commands
		// and handles responses for context management

		// Mock Tauri invoke responses
		await page.addInitScript(() => {
			// @ts-ignore
			window.__TAURI__ = {
				invoke: async (command: string, args: any) => {
					switch (command) {
						case 'create_conversation':
							return { status: 'Success', conversation_id: 'test-conv-id', error: null };
						case 'add_context_to_conversation':
							return { status: 'Success', error: null };
						case 'get_conversation':
							return {
								status: 'Success',
								conversation: {
									id: 'test-conv-id',
									title: args.title || 'Test Conversation',
									role: { name: 'Engineer' },
									messages: [],
									global_context: [],
								},
							};
						default:
							return { status: 'Error', error: 'Unknown command' };
					}
				},
			};
		});

		// Create conversation and verify backend integration
		await page.click('[data-testid="new-conversation-button"]');
		await page.fill('[data-testid="conversation-title-input"]', 'Backend Integration Test');
		await page.click('[data-testid="create-conversation-confirm"]');

		// Add context and verify it calls the backend
		await page.click('[data-testid="add-manual-context-button"]');
		await page.fill('[data-testid="context-title-input"]', 'Backend Test Context');
		await page.fill('[data-testid="context-content-textarea"]', 'Testing backend integration');
		await page.click('[data-testid="save-context-button"]');

		// Verify the UI reflects successful backend operations
		await expect(
			page.locator('[data-testid="context-item"]:has-text("Backend Test Context")')
		).toBeVisible();
	});
});
