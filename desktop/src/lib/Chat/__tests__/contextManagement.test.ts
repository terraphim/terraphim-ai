import { fireEvent, render, screen, waitFor } from '@testing-library/svelte/svelte5';
import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import '@testing-library/jest-dom';

import { createTestConfig } from '../../../__test-utils__/testConfig';
// Test utilities for real API testing
import { startTestServer, stopTestServer } from '../../../__test-utils__/testServer';

import Chat from '../Chat.svelte';

describe('Context Management Integration Tests', () => {
	let testServer: any;
	let serverUrl: string;

	beforeEach(async () => {
		// Start a real test server instance
		testServer = await startTestServer(createTestConfig());
		serverUrl = testServer.address();

		// Update CONFIG for tests
		(global as any).CONFIG = { ServerURL: serverUrl };

		// Set test mode to use HTTP instead of Tauri
		(global as any).__IS_TAURI__ = false;
	});

	afterEach(async () => {
		if (testServer) {
			await stopTestServer(testServer);
		}
	});

	describe('Conversation Lifecycle', () => {
		it('should create and manage conversations through real API', async () => {
			render(Chat);

			// Wait for the component to initialize and make API calls
			await waitFor(async () => {
				const response = await fetch(`${serverUrl}/conversations`);
				expect(response.ok).toBe(true);
			});

			// Verify conversation creation happened
			const conversationsResponse = await fetch(`${serverUrl}/conversations`);
			const conversationsData = await conversationsResponse.json();
			expect(conversationsData.status).toBe('Success');
			expect(conversationsData.conversations).toBeDefined();
		});

		it('should handle conversation creation failures gracefully', async () => {
			// Stop the server to simulate network failure
			await stopTestServer(testServer);
			testServer = null;

			const consoleSpy = jest.spyOn(console, 'error').mockImplementation(() => {});

			render(Chat);

			// Should handle the error gracefully
			await waitFor(() => {
				expect(consoleSpy).toHaveBeenCalledWith(
					expect.stringContaining('❌ Error initializing conversation:')
				);
			});

			consoleSpy.mockRestore();
		});
	});

	describe('Context Addition Flow', () => {
		it('should add context through the UI form', async () => {
			render(Chat);

			// Wait for initialization
			await waitFor(() => {
				expect(screen.queryByTestId('show-add-context-button')).toBeInTheDocument();
			});

			// Click add context button
			const addButton = screen.getByTestId('show-add-context-button');
			await fireEvent.click(addButton);

			// Fill out the form
			const titleInput = screen.getByTestId('context-title-input');
			const contentTextarea = screen.getByTestId('context-content-textarea');

			await fireEvent.input(titleInput, { target: { value: 'Test Document Title' } });
			await fireEvent.input(contentTextarea, {
				target: { value: 'This is test document content for context management testing.' },
			});

			// Submit the form
			const saveButton = screen.getByTestId('add-context-submit-button');
			await fireEvent.click(saveButton);

			// Wait for the context to be added and form to close
			await waitFor(() => {
				expect(screen.queryByTestId('add-context-form')).not.toBeInTheDocument();
			});

			// Verify context appears in the list
			await waitFor(() => {
				expect(screen.getByText('Test Document Title')).toBeInTheDocument();
			});
		});

		it('should validate form inputs', async () => {
			render(Chat);

			await waitFor(() => {
				expect(screen.queryByTestId('show-add-context-button')).toBeInTheDocument();
			});

			const addButton = screen.getByTestId('show-add-context-button');
			await fireEvent.click(addButton);

			// Try to submit with empty form
			const saveButton = screen.getByTestId('add-context-submit-button');
			expect(saveButton).toBeDisabled();

			// Add only title
			const titleInput = screen.getByTestId('context-title-input');
			await fireEvent.input(titleInput, { target: { value: 'Only Title' } });
			expect(saveButton).toBeDisabled();

			// Add content to enable submit
			const contentTextarea = screen.getByTestId('context-content-textarea');
			await fireEvent.input(contentTextarea, { target: { value: 'Now with content' } });
			expect(saveButton).not.toBeDisabled();
		});
	});

	describe('Context Display', () => {
		it('should show empty state when no context exists', async () => {
			render(Chat);

			await waitFor(() => {
				expect(screen.getByTestId('empty-context-message')).toBeInTheDocument();
			});

			expect(screen.getByText('No context items yet')).toBeInTheDocument();
		});

		it('should display context items with proper metadata', async () => {
			render(Chat);

			// Add context through the API directly to test display
			await waitFor(async () => {
				// First get/create a conversation
				const conversationsResponse = await fetch(`${serverUrl}/conversations`);
				const conversationsData = await conversationsResponse.json();

				let conversationId: string | undefined;
				if (conversationsData.conversations && conversationsData.conversations.length > 0) {
					conversationId = conversationsData.conversations[0].id;
				} else {
					// Create conversation
					const createResponse = await fetch(`${serverUrl}/conversations`, {
						method: 'POST',
						headers: { 'Content-Type': 'application/json' },
						body: JSON.stringify({ title: 'Test Conversation', role: 'TestRole' }),
					});
					const createData = await createResponse.json();
					conversationId = createData.conversation_id;
				}

				// Add context
				await fetch(`${serverUrl}/conversations/${conversationId}/context`, {
					method: 'POST',
					headers: { 'Content-Type': 'application/json' },
					body: JSON.stringify({
						context_type: 'document',
						title: 'API Test Document',
						content: 'This document was added via API for display testing.',
						metadata: {
							source_type: 'test',
							document_id: 'test-doc-123',
						},
					}),
				});
			});

			// Refresh the component to load the new context
			const refreshButton = screen.getByTestId('refresh-context-button');
			await fireEvent.click(refreshButton);

			// Verify context displays
			await waitFor(() => {
				expect(screen.getByText('API Test Document')).toBeInTheDocument();
			});

			expect(screen.getByTestId('context-summary')).toHaveTextContent('1 context items');
		});
	});

	describe('Error Handling', () => {
		it('should handle API errors during context addition', async () => {
			render(Chat);

			await waitFor(() => {
				expect(screen.queryByTestId('show-add-context-button')).toBeInTheDocument();
			});

			// Stop server to simulate error
			await stopTestServer(testServer);
			testServer = null;

			const consoleSpy = jest.spyOn(console, 'error').mockImplementation(() => {});

			const addButton = screen.getByTestId('show-add-context-button');
			await fireEvent.click(addButton);

			const titleInput = screen.getByTestId('context-title-input');
			const contentTextarea = screen.getByTestId('context-content-textarea');

			await fireEvent.input(titleInput, { target: { value: 'Error Test' } });
			await fireEvent.input(contentTextarea, { target: { value: 'This should fail' } });

			const saveButton = screen.getByTestId('add-context-submit-button');
			await fireEvent.click(saveButton);

			await waitFor(() => {
				expect(consoleSpy).toHaveBeenCalledWith(
					expect.stringContaining('❌ Error adding manual context:')
				);
			});

			consoleSpy.mockRestore();
		});
	});

	describe('Real Data Flow', () => {
		it('should persist context across component re-renders', async () => {
			const { unmount } = render(Chat);

			// Add context
			await waitFor(() => {
				expect(screen.queryByTestId('show-add-context-button')).toBeInTheDocument();
			});

			const addButton = screen.getByTestId('show-add-context-button');
			await fireEvent.click(addButton);

			const titleInput = screen.getByTestId('context-title-input');
			const contentTextarea = screen.getByTestId('context-content-textarea');

			await fireEvent.input(titleInput, { target: { value: 'Persistent Context' } });
			await fireEvent.input(contentTextarea, {
				target: { value: 'This should persist across re-renders' },
			});

			const saveButton = screen.getByTestId('add-context-submit-button');
			await fireEvent.click(saveButton);

			await waitFor(() => {
				expect(screen.getByText('Persistent Context')).toBeInTheDocument();
			});

			// Unmount and re-mount component
			unmount();
			render(Chat);

			// Context should still be there
			await waitFor(() => {
				expect(screen.getByText('Persistent Context')).toBeInTheDocument();
			});
		});

		it('should handle multiple context items correctly', async () => {
			render(Chat);

			// Add multiple context items
			for (let i = 1; i <= 3; i++) {
				await waitFor(() => {
					expect(screen.queryByTestId('show-add-context-button')).toBeInTheDocument();
				});

				const addButton = screen.getByTestId('show-add-context-button');
				await fireEvent.click(addButton);

				const titleInput = screen.getByTestId('context-title-input');
				const contentTextarea = screen.getByTestId('context-content-textarea');

				await fireEvent.input(titleInput, { target: { value: `Context Item ${i}` } });
				await fireEvent.input(contentTextarea, { target: { value: `Content for item ${i}` } });

				const saveButton = screen.getByTestId('add-context-submit-button');
				await fireEvent.click(saveButton);

				await waitFor(() => {
					expect(screen.queryByTestId('add-context-form')).not.toBeInTheDocument();
				});
			}

			// Verify all items are displayed
			await waitFor(() => {
				expect(screen.getByTestId('context-summary')).toHaveTextContent('3 context items');
			});

			expect(screen.getByText('Context Item 1')).toBeInTheDocument();
			expect(screen.getByText('Context Item 2')).toBeInTheDocument();
			expect(screen.getByText('Context Item 3')).toBeInTheDocument();
		});
	});
});
