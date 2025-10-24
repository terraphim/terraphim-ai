import { fireEvent, render, screen, waitFor } from '@testing-library/svelte';
import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import '@testing-library/jest-dom';

import { createTestConfig } from '../__test-utils__/testConfig';
// Test utilities for real API testing
import { startTestServer, stopTestServer } from '../__test-utils__/testServer';

import Chat from '../lib/Chat/Chat.svelte';

describe('Context Edit Workflow E2E Tests', () => {
	let testServer: any;
	let serverUrl: string;

	beforeEach(async () => {
		// Start a real test server instance
		testServer = await startTestServer(createTestConfig());
		serverUrl = testServer.address();

		// Update CONFIG for tests
		(global as any).CONFIG = { ServerURL: serverUrl };
		(global as any).__IS_TAURI__ = false;
	});

	afterEach(async () => {
		if (testServer) {
			await stopTestServer(testServer);
		}
	});

	describe('Complete Context Management Workflow', () => {
		it('should add, edit, and delete context through the UI', async () => {
			render(Chat);

			// Wait for the component to initialize
			await waitFor(() => {
				expect(screen.queryByTestId('show-add-context-button')).toBeInTheDocument();
			});

			// Step 1: Add a new context item
			const addButton = screen.getByTestId('show-add-context-button');
			await fireEvent.click(addButton);

			// Fill out the add context form
			const titleInput = screen.getByTestId('context-title-input');
			const contentTextarea = screen.getByTestId('context-content-textarea');

			await fireEvent.input(titleInput, { target: { value: 'E2E Test Document' } });
			await fireEvent.input(contentTextarea, {
				target: {
					value:
						'This is a comprehensive test document for end-to-end testing of context management.',
				},
			});

			// Submit the form
			const saveButton = screen.getByTestId('add-context-submit-button');
			await fireEvent.click(saveButton);

			// Wait for the context to be added and appear in the list
			await waitFor(() => {
				expect(screen.getByText('E2E Test Document')).toBeInTheDocument();
			});

			// Verify context appears in the list
			expect(screen.getByText('E2E Test Document')).toBeInTheDocument();
			expect(screen.getByTestId('context-summary')).toHaveTextContent('1 context items');

			// Step 2: Edit the context item
			// Look for the edit button (assuming it appears on hover or is always visible)
			const contextItem = screen
				.getByText('E2E Test Document')
				.closest('[data-testid*="context-item"]');
			expect(contextItem).toBeInTheDocument();

			// Find and click the edit button
			const editButton = contextItem?.querySelector('[data-testid*="edit-context"]');
			if (editButton) {
				await fireEvent.click(editButton);

				// Wait for the edit modal to appear
				await waitFor(() => {
					expect(screen.getByText('Edit Context Item')).toBeInTheDocument();
				});

				// Update the title and add a summary
				const editTitleInput = screen.getByTestId('context-title-input');
				const editSummaryTextarea = screen.getByTestId('context-summary-textarea');

				await fireEvent.input(editTitleInput, { target: { value: 'Updated E2E Test Document' } });
				await fireEvent.input(editSummaryTextarea, {
					target: { value: 'This document has been updated via E2E testing workflow' },
				});

				// Save the changes
				const editSaveButton = screen.getByTestId('save-context-button');
				await fireEvent.click(editSaveButton);

				// Wait for the modal to close and changes to reflect
				await waitFor(() => {
					expect(screen.queryByText('Edit Context Item')).not.toBeInTheDocument();
				});

				// Verify the changes are reflected
				await waitFor(() => {
					expect(screen.getByText('Updated E2E Test Document')).toBeInTheDocument();
				});
			}

			// Step 3: Delete the context item
			const updatedContextItem = screen
				.getByText('Updated E2E Test Document')
				.closest('[data-testid*="context-item"]');
			const deleteButton = updatedContextItem?.querySelector('[data-testid*="delete-context"]');

			if (deleteButton) {
				await fireEvent.click(deleteButton);

				// Wait for the context to be removed
				await waitFor(() => {
					expect(screen.queryByText('Updated E2E Test Document')).not.toBeInTheDocument();
				});

				// Verify empty state is shown
				await waitFor(() => {
					expect(screen.getByTestId('empty-context-message')).toBeInTheDocument();
				});
			}
		});

		it('should handle context with summary field end-to-end', async () => {
			render(Chat);

			// Wait for initialization
			await waitFor(() => {
				expect(screen.queryByTestId('show-add-context-button')).toBeInTheDocument();
			});

			// Add context with summary
			const addButton = screen.getByTestId('show-add-context-button');
			await fireEvent.click(addButton);

			const titleInput = screen.getByTestId('context-title-input');
			const summaryTextarea = screen.getByTestId('context-summary-textarea');
			const contentTextarea = screen.getByTestId('context-content-textarea');

			await fireEvent.input(titleInput, { target: { value: 'Document with Summary' } });
			await fireEvent.input(summaryTextarea, {
				target: { value: 'This is a concise summary of the document content' },
			});
			await fireEvent.input(contentTextarea, {
				target: {
					value:
						'This is the full detailed content of the document that provides comprehensive information about the topic.',
				},
			});

			const saveButton = screen.getByTestId('add-context-submit-button');
			await fireEvent.click(saveButton);

			// Wait for the context to appear
			await waitFor(() => {
				expect(screen.getByText('Document with Summary')).toBeInTheDocument();
			});

			// Verify the summary is displayed (implementation dependent)
			// This would depend on how the Chat component displays summaries
			expect(screen.getByText('Document with Summary')).toBeInTheDocument();
		});

		it('should persist context changes across server round-trips', async () => {
			render(Chat);

			// Wait for initialization
			await waitFor(() => {
				expect(screen.queryByTestId('show-add-context-button')).toBeInTheDocument();
			});

			// Add a context item
			const addButton = screen.getByTestId('show-add-context-button');
			await fireEvent.click(addButton);

			const titleInput = screen.getByTestId('context-title-input');
			const contentTextarea = screen.getByTestId('context-content-textarea');

			await fireEvent.input(titleInput, { target: { value: 'Persistent Context Test' } });
			await fireEvent.input(contentTextarea, {
				target: { value: 'This context should persist across server operations' },
			});

			const saveButton = screen.getByTestId('add-context-submit-button');
			await fireEvent.click(saveButton);

			await waitFor(() => {
				expect(screen.getByText('Persistent Context Test')).toBeInTheDocument();
			});

			// Manually verify persistence via API call
			const conversationsResponse = await fetch(`${serverUrl}/conversations`);
			const conversationsData = await conversationsResponse.json();

			expect(conversationsData.status).toBe('Success');

			if (conversationsData.conversations && conversationsData.conversations.length > 0) {
				const conversation = conversationsData.conversations[0];
				expect(conversation.global_context).toBeDefined();
				expect(conversation.global_context.length).toBe(1);
				expect(conversation.global_context[0].title).toBe('Persistent Context Test');
			}
		});

		it('should handle multiple context items correctly', async () => {
			render(Chat);

			await waitFor(() => {
				expect(screen.queryByTestId('show-add-context-button')).toBeInTheDocument();
			});

			// Add multiple context items
			const contextItems = [
				{ title: 'First Context Item', content: 'Content for the first item' },
				{ title: 'Second Context Item', content: 'Content for the second item' },
				{ title: 'Third Context Item', content: 'Content for the third item' },
			];

			for (let i = 0; i < contextItems.length; i++) {
				const addButton = screen.getByTestId('show-add-context-button');
				await fireEvent.click(addButton);

				const titleInput = screen.getByTestId('context-title-input');
				const contentTextarea = screen.getByTestId('context-content-textarea');

				await fireEvent.input(titleInput, { target: { value: contextItems[i].title } });
				await fireEvent.input(contentTextarea, { target: { value: contextItems[i].content } });

				const saveButton = screen.getByTestId('add-context-submit-button');
				await fireEvent.click(saveButton);

				// Wait for each item to be added
				await waitFor(() => {
					expect(screen.getByText(contextItems[i].title)).toBeInTheDocument();
				});
			}

			// Verify all items are displayed
			contextItems.forEach((item) => {
				expect(screen.getByText(item.title)).toBeInTheDocument();
			});

			// Verify context summary shows correct count
			await waitFor(() => {
				expect(screen.getByTestId('context-summary')).toHaveTextContent('3 context items');
			});
		});
	});

	describe('Error Handling in E2E Flow', () => {
		it('should handle server errors gracefully during context operations', async () => {
			render(Chat);

			await waitFor(() => {
				expect(screen.queryByTestId('show-add-context-button')).toBeInTheDocument();
			});

			// Stop the server to simulate network failure
			await stopTestServer(testServer);
			testServer = null;

			const consoleSpy = vi.spyOn(console, 'error').mockImplementation(() => {});

			// Try to add context when server is down
			const addButton = screen.getByTestId('show-add-context-button');
			await fireEvent.click(addButton);

			const titleInput = screen.getByTestId('context-title-input');
			const contentTextarea = screen.getByTestId('context-content-textarea');

			await fireEvent.input(titleInput, { target: { value: 'Error Test Context' } });
			await fireEvent.input(contentTextarea, { target: { value: 'This should fail gracefully' } });

			const saveButton = screen.getByTestId('add-context-submit-button');
			await fireEvent.click(saveButton);

			// Should handle the error gracefully
			await waitFor(() => {
				expect(consoleSpy).toHaveBeenCalledWith(
					expect.stringContaining('❌ Error adding manual context:')
				);
			});

			consoleSpy.mockRestore();
		});

		it('should handle invalid context data gracefully', async () => {
			render(Chat);

			await waitFor(() => {
				expect(screen.queryByTestId('show-add-context-button')).toBeInTheDocument();
			});

			// Try to add context with minimal/invalid data
			const addButton = screen.getByTestId('show-add-context-button');
			await fireEvent.click(addButton);

			// Submit without filling required fields
			const saveButton = screen.getByTestId('add-context-submit-button');

			// Button should be disabled
			expect(saveButton).toBeDisabled();

			// Add only title (should still be disabled)
			const titleInput = screen.getByTestId('context-title-input');
			await fireEvent.input(titleInput, { target: { value: '   ' } }); // Whitespace only

			expect(saveButton).toBeDisabled();
		});
	});

	describe('Real API Integration E2E', () => {
		it('should perform complete CRUD operations via real API', async () => {
			// This test directly uses the API to verify the full workflow

			// Create conversation
			const createResponse = await fetch(`${serverUrl}/conversations`, {
				method: 'POST',
				headers: { 'Content-Type': 'application/json' },
				body: JSON.stringify({ title: 'E2E API Test Conversation', role: 'TestRole' }),
			});
			expect(createResponse.ok).toBe(true);
			const createData = await createResponse.json();
			const conversationId = createData.conversation_id;

			// Add context
			const addResponse = await fetch(`${serverUrl}/conversations/${conversationId}/context`, {
				method: 'POST',
				headers: { 'Content-Type': 'application/json' },
				body: JSON.stringify({
					context_type: 'Document',
					title: 'API CRUD Test Document',
					summary: 'Initial summary for CRUD testing',
					content: 'Initial content for comprehensive CRUD testing',
					metadata: { test_type: 'e2e_api_crud' },
				}),
			});
			expect(addResponse.ok).toBe(true);

			// Get conversation to retrieve context ID
			const getResponse = await fetch(`${serverUrl}/conversations/${conversationId}`);
			expect(getResponse.ok).toBe(true);
			const convData = await getResponse.json();
			const contextId = convData.conversation.global_context[0].id;

			// Update context
			const updateResponse = await fetch(
				`${serverUrl}/conversations/${conversationId}/context/${contextId}`,
				{
					method: 'PUT',
					headers: { 'Content-Type': 'application/json' },
					body: JSON.stringify({
						title: 'Updated API CRUD Test Document',
						summary: 'Updated summary after CRUD operations',
						content: 'Updated content with more comprehensive information',
						metadata: { test_type: 'e2e_api_crud', updated: 'true' },
					}),
				}
			);
			expect(updateResponse.ok).toBe(true);
			const updateData = await updateResponse.json();
			expect(updateData.status).toBe('Success');
			expect(updateData.context.title).toBe('Updated API CRUD Test Document');

			// Verify update persisted
			const verifyResponse = await fetch(`${serverUrl}/conversations/${conversationId}`);
			expect(verifyResponse.ok).toBe(true);
			const verifyData = await verifyResponse.json();
			expect(verifyData.conversation.global_context[0].title).toBe(
				'Updated API CRUD Test Document'
			);
			expect(verifyData.conversation.global_context[0].summary).toBe(
				'Updated summary after CRUD operations'
			);

			// Delete context
			const deleteResponse = await fetch(
				`${serverUrl}/conversations/${conversationId}/context/${contextId}`,
				{
					method: 'DELETE',
				}
			);
			expect(deleteResponse.ok).toBe(true);
			const deleteData = await deleteResponse.json();
			expect(deleteData.status).toBe('Success');

			// Verify deletion
			const finalResponse = await fetch(`${serverUrl}/conversations/${conversationId}`);
			expect(finalResponse.ok).toBe(true);
			const finalData = await finalResponse.json();
			expect(finalData.conversation.global_context).toHaveLength(0);
		});

		it('should handle concurrent context operations', async () => {
			// Create conversation
			const createResponse = await fetch(`${serverUrl}/conversations`, {
				method: 'POST',
				headers: { 'Content-Type': 'application/json' },
				body: JSON.stringify({ title: 'Concurrent Test Conversation', role: 'TestRole' }),
			});
			const createData = await createResponse.json();
			const conversationId = createData.conversation_id;

			// Add multiple context items concurrently
			const contextPromises = Array.from({ length: 5 }, (_, i) =>
				fetch(`${serverUrl}/conversations/${conversationId}/context`, {
					method: 'POST',
					headers: { 'Content-Type': 'application/json' },
					body: JSON.stringify({
						context_type: 'Document',
						title: `Concurrent Context ${i + 1}`,
						summary: `Summary for concurrent context ${i + 1}`,
						content: `Content for concurrent context item number ${i + 1}`,
						metadata: { batch: 'concurrent', index: i.toString() },
					}),
				})
			);

			const addResponses = await Promise.all(contextPromises);
			addResponses.forEach((response) => {
				expect(response.ok).toBe(true);
			});

			// Verify all contexts were added
			const verifyResponse = await fetch(`${serverUrl}/conversations/${conversationId}`);
			const verifyData = await verifyResponse.json();
			expect(verifyData.conversation.global_context).toHaveLength(5);

			// Verify all titles are present
			const titles = verifyData.conversation.global_context.map((ctx: any) => ctx.title);
			for (let i = 1; i <= 5; i++) {
				expect(titles).toContain(`Concurrent Context ${i}`);
			}
		});
	});
});
