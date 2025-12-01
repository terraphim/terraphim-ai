import { fireEvent, render, screen, waitFor } from '@testing-library/svelte/svelte5';
import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import '@testing-library/jest-dom';
import { Modal } from 'svelma';

import { createTestConfig } from '../../../__test-utils__/testConfig';
// Test utilities for real API testing
import { startTestServer, stopTestServer } from '../../../__test-utils__/testServer';
import type { ContextItem } from '../Chat.svelte';
import ContextEditModal from '../ContextEditModal.svelte';

describe('ContextEditModal Integration Tests', () => {
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

	const createTestContextItem = (): ContextItem => ({
		id: 'test-context-1',
		context_type: 'Document',
		title: 'Test Document',
		summary: 'This is a test summary for the document',
		content: 'This is the full content of the test document with detailed information.',
		metadata: {
			source: 'test',
			document_id: 'test-doc-123',
		},
		created_at: '2024-01-01T12:00:00Z',
		relevance_score: 0.95,
	});

	describe('Modal Rendering and UI', () => {
		it('should render edit modal with context data', () => {
			const testContext = createTestContextItem();

			render(ContextEditModal, {
				props: {
					active: true,
					context: testContext,
					mode: 'edit',
				},
			});

			// Check modal title
			expect(screen.getByText('Edit Context Item')).toBeInTheDocument();

			// Check form fields are populated
			expect(screen.getByTestId('context-title-input')).toHaveValue('Test Document');
			expect(screen.getByTestId('context-summary-textarea')).toHaveValue(
				'This is a test summary for the document'
			);
			expect(screen.getByTestId('context-content-textarea')).toHaveValue(
				'This is the full content of the test document with detailed information.'
			);
			expect(screen.getByTestId('context-type-select')).toHaveValue('Document');
		});

		it('should render create modal for new context', () => {
			render(ContextEditModal, {
				props: {
					active: true,
					context: null,
					mode: 'create',
				},
			});

			expect(screen.getByText('Add Context Item')).toBeInTheDocument();

			// Check form fields are empty/default
			expect(screen.getByTestId('context-title-input')).toHaveValue('');
			expect(screen.getByTestId('context-summary-textarea')).toHaveValue('');
			expect(screen.getByTestId('context-content-textarea')).toHaveValue('');
			expect(screen.getByTestId('context-type-select')).toHaveValue('UserInput');
		});

		it('should not render when inactive', () => {
			render(ContextEditModal, {
				props: {
					active: false,
					context: null,
					mode: 'edit',
				},
			});

			expect(screen.queryByText('Edit Context Item')).not.toBeInTheDocument();
			expect(screen.queryByText('Add Context Item')).not.toBeInTheDocument();
		});
	});

	describe('Form Validation', () => {
		it('should validate required fields', () => {
			render(ContextEditModal, {
				props: {
					active: true,
					context: null,
					mode: 'create',
				},
			});

			// Save button should be disabled initially
			const saveButton = screen.getByTestId('save-context-button');
			expect(saveButton).toBeDisabled();

			// Add only title
			const titleInput = screen.getByTestId('context-title-input');
			fireEvent.input(titleInput, { target: { value: 'Test Title' } });
			expect(saveButton).toBeDisabled();

			// Add content - should enable save
			const contentTextarea = screen.getByTestId('context-content-textarea');
			fireEvent.input(contentTextarea, { target: { value: 'Test content' } });
			expect(saveButton).not.toBeDisabled();
		});

		it('should show validation messages for empty required fields', async () => {
			render(ContextEditModal, {
				props: {
					active: true,
					context: null,
					mode: 'create',
				},
			});

			// Clear title and check validation message
			const titleInput = screen.getByTestId('context-title-input');
			fireEvent.input(titleInput, { target: { value: '' } });

			expect(screen.getByText('Title is required')).toBeInTheDocument();

			// Clear content and check validation message
			const contentTextarea = screen.getByTestId('context-content-textarea');
			fireEvent.input(contentTextarea, { target: { value: '' } });

			expect(screen.getByText('Content is required')).toBeInTheDocument();
		});

		it('should validate summary character limit', async () => {
			render(ContextEditModal, {
				props: {
					active: true,
					context: null,
					mode: 'create',
				},
			});

			const summaryTextarea = screen.getByTestId('context-summary-textarea');
			const testSummary = 'a'.repeat(250);

			fireEvent.input(summaryTextarea, { target: { value: testSummary } });

			expect(screen.getByText('250/500 characters')).toBeInTheDocument();
		});
	});

	describe('Form Interactions', () => {
		it('should handle context type selection', async () => {
			const testContext = createTestContextItem();

			render(ContextEditModal, {
				props: {
					active: true,
					context: testContext,
					mode: 'edit',
				},
			});

			const typeSelect = screen.getByTestId('context-type-select');
			fireEvent.change(typeSelect, { target: { value: 'UserInput' } });

			expect(typeSelect).toHaveValue('UserInput');
		});

		it('should handle summary editing', async () => {
			const testContext = createTestContextItem();

			render(ContextEditModal, {
				props: {
					active: true,
					context: testContext,
					mode: 'edit',
				},
			});

			const summaryTextarea = screen.getByTestId('context-summary-textarea');
			const newSummary = 'Updated summary with new information';

			fireEvent.input(summaryTextarea, { target: { value: newSummary } });

			expect(summaryTextarea).toHaveValue(newSummary);
		});

		it('should handle metadata editing', async () => {
			const testContext = createTestContextItem();

			render(ContextEditModal, {
				props: {
					active: true,
					context: testContext,
					mode: 'edit',
				},
			});

			// Open advanced options
			const detailsElement = screen.getByText('Advanced Options').parentElement;
			fireEvent.click(detailsElement);

			// Check existing metadata is shown
			await waitFor(() => {
				expect(screen.getByDisplayValue('source')).toBeInTheDocument();
				expect(screen.getByDisplayValue('test')).toBeInTheDocument();
			});

			// Add new metadata
			const addMetadataButton = screen.getByText('Add Metadata');
			fireEvent.click(addMetadataButton);

			// Wait for the new metadata field to appear
			await waitFor(() => {
				const metadataKeys = screen.getAllByPlaceholderText('Key');
				expect(metadataKeys.length).toBeGreaterThanOrEqual(3);
			});
		});
	});

	describe('Event Dispatching', () => {
		it('should handle form submission in edit mode', async () => {
			const testContext = createTestContextItem();
			let updateEventFired = false;
			let updateEventData: any = null;

			const { component } = render(ContextEditModal, {
				props: {
					active: true,
					context: testContext,
					mode: 'edit',
				},
			});

			// Listen for the update event
			component.$on('update', (event) => {
				updateEventFired = true;
				updateEventData = event.detail;
			});

			// Modify title
			const titleInput = screen.getByTestId('context-title-input');
			fireEvent.input(titleInput, { target: { value: 'Updated Title' } });

			// Save changes
			const saveButton = screen.getByTestId('save-context-button');
			fireEvent.click(saveButton);

			expect(updateEventFired).toBe(true);
			expect(updateEventData).toMatchObject({
				id: 'test-context-1',
				title: 'Updated Title',
				summary: 'This is a test summary for the document',
			});
		});

		it('should handle form submission in create mode', async () => {
			let createEventFired = false;
			let createEventData: any = null;

			const { component } = render(ContextEditModal, {
				props: {
					active: true,
					context: null,
					mode: 'create',
				},
			});

			// Listen for the create event
			component.$on('create', (event) => {
				createEventFired = true;
				createEventData = event.detail;
			});

			// Fill form
			const titleInput = screen.getByTestId('context-title-input');
			const summaryTextarea = screen.getByTestId('context-summary-textarea');
			const contentTextarea = screen.getByTestId('context-content-textarea');

			fireEvent.input(titleInput, { target: { value: 'New Context Item' } });
			fireEvent.input(summaryTextarea, { target: { value: 'New summary' } });
			fireEvent.input(contentTextarea, { target: { value: 'New content' } });

			// Save
			const saveButton = screen.getByTestId('save-context-button');
			fireEvent.click(saveButton);

			expect(createEventFired).toBe(true);
			expect(createEventData).toMatchObject({
				title: 'New Context Item',
				summary: 'New summary',
				content: 'New content',
				context_type: 'UserInput',
			});
		});

		it('should handle delete button click', async () => {
			const testContext = createTestContextItem();
			let deleteEventFired = false;
			let deleteEventData: any = null;

			const { component } = render(ContextEditModal, {
				props: {
					active: true,
					context: testContext,
					mode: 'edit',
				},
			});

			// Listen for the delete event
			component.$on('delete', (event) => {
				deleteEventFired = true;
				deleteEventData = event.detail;
			});

			const deleteButton = screen.getByTestId('delete-context-button');
			fireEvent.click(deleteButton);

			expect(deleteEventFired).toBe(true);
			expect(deleteEventData).toBe('test-context-1');
		});

		it('should handle modal close', async () => {
			const testContext = createTestContextItem();
			let closeEventFired = false;

			const { component } = render(ContextEditModal, {
				props: {
					active: true,
					context: testContext,
					mode: 'edit',
				},
			});

			// Listen for the close event
			component.$on('close', () => {
				closeEventFired = true;
			});

			const cancelButton = screen.getByTestId('cancel-context-button');
			fireEvent.click(cancelButton);

			expect(closeEventFired).toBe(true);
		});
	});

	describe('Keyboard Shortcuts', () => {
		it('should close modal on Escape key', async () => {
			const testContext = createTestContextItem();
			let closeEventFired = false;

			const { component } = render(ContextEditModal, {
				props: {
					active: true,
					context: testContext,
					mode: 'edit',
				},
			});

			component.$on('close', () => {
				closeEventFired = true;
			});

			// Simulate Escape key
			fireEvent.keyDown(window, { key: 'Escape' });

			expect(closeEventFired).toBe(true);
		});

		it('should save on Ctrl+Enter', async () => {
			const testContext = createTestContextItem();
			let updateEventFired = false;
			let updateEventData: any = null;

			const { component } = render(ContextEditModal, {
				props: {
					active: true,
					context: testContext,
					mode: 'edit',
				},
			});

			component.$on('update', (event) => {
				updateEventFired = true;
				updateEventData = event.detail;
			});

			// Simulate Ctrl+Enter
			fireEvent.keyDown(window, { key: 'Enter', ctrlKey: true });

			expect(updateEventFired).toBe(true);
			expect(updateEventData).toMatchObject({
				id: 'test-context-1',
				title: 'Test Document',
			});
		});

		it('should save on Cmd+Enter (Mac)', async () => {
			const testContext = createTestContextItem();
			let updateEventFired = false;
			let updateEventData: any = null;

			const { component } = render(ContextEditModal, {
				props: {
					active: true,
					context: testContext,
					mode: 'edit',
				},
			});

			component.$on('update', (event) => {
				updateEventFired = true;
				updateEventData = event.detail;
			});

			// Simulate Cmd+Enter (Mac)
			fireEvent.keyDown(window, { key: 'Enter', metaKey: true });

			expect(updateEventFired).toBe(true);
			expect(updateEventData).toMatchObject({
				id: 'test-context-1',
				title: 'Test Document',
			});
		});
	});

	describe('Summary Field Integration', () => {
		it('should handle optional summary field properly', async () => {
			let createEventFired = false;
			let createEventData: any = null;

			const { component } = render(ContextEditModal, {
				props: {
					active: true,
					context: null,
					mode: 'create',
				},
			});

			component.$on('create', (event) => {
				createEventFired = true;
				createEventData = event.detail;
			});

			// Fill required fields but leave summary empty
			const titleInput = screen.getByTestId('context-title-input');
			const contentTextarea = screen.getByTestId('context-content-textarea');

			fireEvent.input(titleInput, { target: { value: 'Context without Summary' } });
			fireEvent.input(contentTextarea, { target: { value: 'Content without summary' } });

			const saveButton = screen.getByTestId('save-context-button');
			fireEvent.click(saveButton);

			expect(createEventFired).toBe(true);
			expect(createEventData).toMatchObject({
				title: 'Context without Summary',
				summary: '',
				content: 'Content without summary',
			});
		});

		it('should preserve existing summary when editing other fields', async () => {
			const testContext = createTestContextItem();
			let updateEventFired = false;
			let updateEventData: any = null;

			const { component } = render(ContextEditModal, {
				props: {
					active: true,
					context: testContext,
					mode: 'edit',
				},
			});

			component.$on('update', (event) => {
				updateEventFired = true;
				updateEventData = event.detail;
			});

			// Modify only the title
			const titleInput = screen.getByTestId('context-title-input');
			fireEvent.input(titleInput, { target: { value: 'Modified Title Only' } });

			const saveButton = screen.getByTestId('save-context-button');
			fireEvent.click(saveButton);

			expect(updateEventFired).toBe(true);
			expect(updateEventData).toMatchObject({
				title: 'Modified Title Only',
				summary: 'This is a test summary for the document', // Should remain unchanged
				content: 'This is the full content of the test document with detailed information.',
			});
		});

		it('should allow clearing summary field', async () => {
			const testContext = createTestContextItem();
			let updateEventFired = false;
			let updateEventData: any = null;

			const { component } = render(ContextEditModal, {
				props: {
					active: true,
					context: testContext,
					mode: 'edit',
				},
			});

			component.$on('update', (event) => {
				updateEventFired = true;
				updateEventData = event.detail;
			});

			// Clear the summary
			const summaryTextarea = screen.getByTestId('context-summary-textarea');
			fireEvent.input(summaryTextarea, { target: { value: '' } });

			const saveButton = screen.getByTestId('save-context-button');
			fireEvent.click(saveButton);

			expect(updateEventFired).toBe(true);
			expect(updateEventData).toMatchObject({
				summary: '',
			});
		});
	});

	describe('Real API Integration', () => {
		it('should work with real context data from server', async () => {
			// First create a conversation via the test server
			const createResponse = await fetch(`${serverUrl}/conversations`, {
				method: 'POST',
				headers: { 'Content-Type': 'application/json' },
				body: JSON.stringify({ title: 'Test Conversation', role: 'TestRole' }),
			});
			const createData = await createResponse.json();
			const conversationId = createData.conversation_id;

			// Add context via the API
			const contextResponse = await fetch(`${serverUrl}/conversations/${conversationId}/context`, {
				method: 'POST',
				headers: { 'Content-Type': 'application/json' },
				body: JSON.stringify({
					context_type: 'Document',
					title: 'API Integration Test',
					summary: 'This context was created via API for integration testing',
					content: 'Full content from API integration test',
					metadata: { test: 'api-integration' },
				}),
			});
			expect(contextResponse.ok).toBe(true);

			// Get the conversation to retrieve the context
			const convResponse = await fetch(`${serverUrl}/conversations/${conversationId}`);
			const convData = await convResponse.json();
			const contextItem = convData.conversation.global_context[0];

			// Render modal with real context data
			render(ContextEditModal, {
				props: {
					active: true,
					context: contextItem,
					mode: 'edit',
				},
			});

			// Verify the data is loaded correctly
			expect(screen.getByTestId('context-title-input')).toHaveValue('API Integration Test');
			expect(screen.getByTestId('context-summary-textarea')).toHaveValue(
				'This context was created via API for integration testing'
			);
			expect(screen.getByTestId('context-content-textarea')).toHaveValue(
				'Full content from API integration test'
			);
		});
	});

	describe('Error Handling', () => {
		it('should handle null context gracefully', () => {
			render(ContextEditModal, {
				props: {
					active: true,
					context: null,
					mode: 'edit', // This is an edge case - edit mode with null context
				},
			});

			// Should not crash and should show empty form
			expect(screen.queryByTestId('context-title-input')).not.toBeInTheDocument();
			expect(screen.queryByText('Edit Context Item')).toBeInTheDocument();
		});

		it('should handle invalid context data gracefully', () => {
			const invalidContext = {
				id: 'invalid',
				context_type: 'Invalid' as any,
				title: '',
				summary: null as any, // Invalid type
				content: '',
				metadata: null as any, // Invalid type
				created_at: 'invalid-date',
				relevance_score: 'invalid' as any, // Invalid type
			};

			expect(() => {
				render(ContextEditModal, {
					props: {
						active: true,
						context: invalidContext,
						mode: 'edit',
					},
				});
			}).not.toThrow();
		});
	});
});
