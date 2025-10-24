// This is a new test file
import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { render, screen, fireEvent, waitFor } from '@testing-library/svelte/svelte5';
import '@testing-library/jest-dom';

// Test utilities for real API testing
import { startTestServer, stopTestServer } from '../../../__test-utils__/testServer';
import { createTestConfig } from '../../../__test-utils__/testConfig';

import ContextManagementHarness from './ContextManagementHarness.svelte';

describe('Context Management with Test Harness', () => {
  let testServer: any;
  let serverUrl: string;

  beforeEach(async () => {
    try {
      testServer = await startTestServer(createTestConfig());
      serverUrl = testServer.address();
      (global as any).CONFIG = { ServerURL: serverUrl };
      (global as any).__IS_TAURI__ = false;
    } catch (error) {
      console.error('Test server setup failed:', error);
      throw error;
    }
  }, 20000);

  afterEach(async () => {
    if (testServer) {
      await stopTestServer(testServer);
    }
  }, 20000);

  it('should add context via the harness', async () => {
    render(ContextManagementHarness);

    // Check initial state immediately after render
    // Note: onMount is async, so this will check before getConversations completes
    expect(screen.getByTestId('conversation-count')).toHaveTextContent('0');
    expect(screen.getByTestId('context-count')).toHaveTextContent('0');

    // Wait for initial conversations to load
    await waitFor(() => {
      expect(screen.getByTestId('conversation-count')).toHaveTextContent('1');
    });
    
    // After load, context for first conversation should also be loaded (initially 0)
    expect(screen.getByTestId('context-count')).toHaveTextContent('0');

    // Click the "Add Context" button
    const addButton = screen.getByRole('button', { name: /Add Context/i });
    await fireEvent.click(addButton);

    // Wait for the context count to update
    await waitFor(() => {
      expect(screen.getByTestId('context-count')).toHaveTextContent('1');
    });
  });
});
