/**
 * End-to-End Workflow Integration Tests
 * 
 * These tests validate complete workflows through the entire context management system,
 * from frontend UI interactions through Tauri commands to backend API calls.
 * Tests ensure all layers work together correctly for real-world usage scenarios.
 */

import { test, expect } from '@playwright/test';
import { ContextTestHelpers } from '../helpers/context-helpers';
import type { Page } from '@playwright/test';

// Test configuration
const TEST_TIMEOUT = 60000;
const BACKEND_WAIT_TIME = 2000;
const UI_INTERACTION_DELAY = 500;

interface WorkflowTestData {
  conversations: Array<{
    title: string;
    role: string;
    messages: Array<{
      content: string;
      role: 'user' | 'assistant';
    }>;
    contexts: Array<{
      title: string;
      content: string;
      type: 'document' | 'search_result' | 'user_input' | 'system' | 'external';
      source?: string;
    }>;
  }>;
}

// Test data for comprehensive workflow testing
const WORKFLOW_TEST_DATA: WorkflowTestData = {
  conversations: [
    {
      title: 'Rust Async Programming Research',
      role: 'Engineer',
      messages: [
        { content: 'How do I handle errors in async Rust code?', role: 'user' },
        { content: 'You can use Result types with async functions...', role: 'assistant' }
      ],
      contexts: [
        {
          title: 'Async Rust Guide',
          content: 'Comprehensive guide to async programming patterns in Rust, including error handling strategies and best practices.',
          type: 'document',
          source: 'https://docs.rs/tokio'
        },
        {
          title: 'Error Handling Examples',
          content: 'Code examples showing proper error propagation in async contexts using ? operator and Result types.',
          type: 'search_result',
          source: 'stackoverflow_search'
        }
      ]
    },
    {
      title: 'WebAssembly Performance Analysis',
      role: 'Analyst',
      messages: [
        { content: 'What are the performance implications of WASM?', role: 'user' },
        { content: 'WASM provides near-native performance...', role: 'assistant' }
      ],
      contexts: [
        {
          title: 'WASM Benchmarks',
          content: 'Performance benchmarks comparing WASM execution to native code across different workloads.',
          type: 'document',
          source: 'research_paper'
        }
      ]
    }
  ]
};

test.describe('End-to-End Context Management Workflows', () => {
  let helpers: ContextTestHelpers;

  test.beforeEach(async ({ page }) => {
    helpers = new ContextTestHelpers(page);
    
    // Navigate to application and wait for initialization
    await page.goto('/');
    await helpers.waitForApplicationReady();
    
    // Clear any existing state
    await helpers.clearApplicationState();
  });

  test.afterEach(async ({ page }) => {
    // Clean up test data after each test
    await helpers.clearApplicationState();
  });

  test('Complete workflow: Create conversation with context and messages', async ({ page }) => {
    test.setTimeout(TEST_TIMEOUT);

    const conversationData = WORKFLOW_TEST_DATA.conversations[0];
    
    // Step 1: Create conversation
    await test.step('Create new conversation', async () => {
      const conversationId = await helpers.createConversation({
        title: conversationData.title,
        role: conversationData.role
      });
      
      expect(conversationId).toBeDefined();
      expect(conversationId.length).toBeGreaterThan(0);
      
      // Verify conversation appears in sidebar
      await expect(page.locator(`[data-testid="conversation-${conversationId}"]`)).toBeVisible();
      await expect(page.locator(`[data-testid="conversation-title-${conversationId}"]`))
        .toHaveText(conversationData.title);
    });
    
    // Step 2: Add context items
    await test.step('Add context items to conversation', async () => {
      for (const [index, context] of conversationData.contexts.entries()) {
        await helpers.addManualContext({
          title: context.title,
          content: context.content,
          contextType: context.type,
          source: context.source
        });
        
        // Verify context appears in context panel
        await expect(page.locator(`[data-testid="context-item-${index}"]`)).toBeVisible();
        await expect(page.locator(`[data-testid="context-title-${index}"]`))
          .toHaveText(context.title);
        
        // Verify context type indicator
        await expect(page.locator(`[data-testid="context-type-${index}"]`))
          .toHaveText(context.type.replace('_', ' '));
      }
      
      // Verify total context count
      const contextCount = await page.locator('[data-testid^="context-item-"]').count();
      expect(contextCount).toBe(conversationData.contexts.length);
    });
    
    // Step 3: Add messages
    await test.step('Add messages to conversation', async () => {
      for (const [index, message] of conversationData.messages.entries()) {
        await helpers.addMessage({
          content: message.content,
          role: message.role
        });
        
        // Verify message appears in chat
        await expect(page.locator(`[data-testid="message-${index}"]`)).toBeVisible();
        await expect(page.locator(`[data-testid="message-content-${index}"]`))
          .toContainText(message.content);
        
        // Verify message role styling
        if (message.role === 'user') {
          await expect(page.locator(`[data-testid="message-${index}"]`))
            .toHaveClass(/user-message/);
        } else {
          await expect(page.locator(`[data-testid="message-${index}"]`))
            .toHaveClass(/assistant-message/);
        }
      }
      
      // Verify total message count
      const messageCount = await page.locator('[data-testid^="message-"]').count();
      expect(messageCount).toBe(conversationData.messages.length);
    });
    
    // Step 4: Verify complete conversation state
    await test.step('Verify complete conversation state', async () => {
      // Check conversation metadata
      const conversationTitle = await page.locator('[data-testid="current-conversation-title"]').textContent();
      expect(conversationTitle).toBe(conversationData.title);
      
      const conversationRole = await page.locator('[data-testid="current-conversation-role"]').textContent();
      expect(conversationRole).toBe(conversationData.role);
      
      // Check context panel summary
      const contextSummary = await page.locator('[data-testid="context-summary"]').textContent();
      expect(contextSummary).toContain(`${conversationData.contexts.length} context items`);
      
      // Check message thread summary  
      const messagesSummary = await page.locator('[data-testid="messages-summary"]').textContent();
      expect(messagesSummary).toContain(`${conversationData.messages.length} messages`);
    });
  });

  test('Multi-conversation workflow with context switching', async ({ page }) => {
    test.setTimeout(TEST_TIMEOUT * 2);
    
    const conversationIds: string[] = [];
    
    // Step 1: Create multiple conversations
    await test.step('Create multiple conversations', async () => {
      for (const [index, conversationData] of WORKFLOW_TEST_DATA.conversations.entries()) {
        const conversationId = await helpers.createConversation({
          title: conversationData.title,
          role: conversationData.role
        });
        
        conversationIds.push(conversationId);
        
        // Add initial context and message
        await helpers.addManualContext({
          title: conversationData.contexts[0].title,
          content: conversationData.contexts[0].content,
          contextType: conversationData.contexts[0].type
        });
        
        await helpers.addMessage({
          content: conversationData.messages[0].content,
          role: conversationData.messages[0].role
        });
      }
      
      // Verify all conversations exist in sidebar
      for (const conversationId of conversationIds) {
        await expect(page.locator(`[data-testid="conversation-${conversationId}"]`)).toBeVisible();
      }
    });
    
    // Step 2: Switch between conversations and verify state isolation
    await test.step('Test conversation switching and state isolation', async () => {
      // Switch to first conversation
      await helpers.selectConversation(conversationIds[0]);
      await page.waitForTimeout(UI_INTERACTION_DELAY);
      
      let currentTitle = await page.locator('[data-testid="current-conversation-title"]').textContent();
      expect(currentTitle).toBe(WORKFLOW_TEST_DATA.conversations[0].title);
      
      let messageCount = await page.locator('[data-testid^="message-"]').count();
      expect(messageCount).toBe(1);
      
      let contextCount = await page.locator('[data-testid^="context-item-"]').count();
      expect(contextCount).toBe(1);
      
      // Switch to second conversation
      await helpers.selectConversation(conversationIds[1]);
      await page.waitForTimeout(UI_INTERACTION_DELAY);
      
      currentTitle = await page.locator('[data-testid="current-conversation-title"]').textContent();
      expect(currentTitle).toBe(WORKFLOW_TEST_DATA.conversations[1].title);
      
      messageCount = await page.locator('[data-testid^="message-"]').count();
      expect(messageCount).toBe(1);
      
      contextCount = await page.locator('[data-testid^="context-item-"]').count();
      expect(contextCount).toBe(1);
      
      // Verify each conversation has different content
      const firstConversationContent = await page.locator('[data-testid="message-content-0"]').textContent();
      expect(firstConversationContent).toBe(WORKFLOW_TEST_DATA.conversations[1].messages[0].content);
    });
    
    // Step 3: Modify one conversation and verify others unchanged
    await test.step('Test conversation isolation during modifications', async () => {
      // Stay in second conversation, add more content
      await helpers.addMessage({
        content: 'Additional message for testing isolation',
        role: 'user'
      });
      
      await helpers.addManualContext({
        title: 'Isolation Test Context',
        content: 'This context should only appear in the second conversation',
        contextType: 'user_input'
      });
      
      // Verify second conversation has new content
      let messageCount = await page.locator('[data-testid^="message-"]').count();
      expect(messageCount).toBe(2);
      
      let contextCount = await page.locator('[data-testid^="context-item-"]').count();
      expect(contextCount).toBe(2);
      
      // Switch back to first conversation
      await helpers.selectConversation(conversationIds[0]);
      await page.waitForTimeout(UI_INTERACTION_DELAY);
      
      // Verify first conversation unchanged
      messageCount = await page.locator('[data-testid^="message-"]').count();
      expect(messageCount).toBe(1);
      
      contextCount = await page.locator('[data-testid^="context-item-"]').count();
      expect(contextCount).toBe(1);
      
      const firstMessage = await page.locator('[data-testid="message-content-0"]').textContent();
      expect(firstMessage).toBe(WORKFLOW_TEST_DATA.conversations[0].messages[0].content);
    });
  });

  test('Search integration workflow', async ({ page }) => {
    test.setTimeout(TEST_TIMEOUT);
    
    await test.step('Create conversation for search integration', async () => {
      const conversationId = await helpers.createConversation({
        title: 'Search Integration Test',
        role: 'Engineer'
      });
      
      expect(conversationId).toBeDefined();
    });
    
    await test.step('Perform search and add results to context', async () => {
      // Open search panel
      await page.click('[data-testid="search-panel-toggle"]');
      await expect(page.locator('[data-testid="search-panel"]')).toBeVisible();
      
      // Enter search query
      await page.fill('[data-testid="search-input"]', 'rust async programming');
      await page.click('[data-testid="search-execute"]');
      
      // Wait for search results
      await expect(page.locator('[data-testid="search-results"]')).toBeVisible({ timeout: 10000 });
      
      // Verify search results appear
      const resultCount = await page.locator('[data-testid^="search-result-"]').count();
      expect(resultCount).toBeGreaterThan(0);
      
      // Select first search result for context
      await page.click('[data-testid="search-result-0"] [data-testid="add-to-context"]');
      
      // Verify result added to context
      await expect(page.locator('[data-testid="context-item-0"]')).toBeVisible();
      const contextType = await page.locator('[data-testid="context-type-0"]').textContent();
      expect(contextType).toBe('search result');
    });
    
    await test.step('Verify search result context integration', async () => {
      // Check that search result context has proper metadata
      await page.click('[data-testid="context-item-0"] [data-testid="context-details-toggle"]');
      
      const contextDetails = page.locator('[data-testid="context-details-0"]');
      await expect(contextDetails).toBeVisible();
      
      // Verify search metadata is preserved
      await expect(contextDetails.locator('[data-testid="context-source"]'))
        .toContainText('search');
      
      await expect(contextDetails.locator('[data-testid="context-rank"]'))
        .toBeVisible();
      
      // Verify search result can be used in conversation
      await helpers.addMessage({
        content: 'Based on the search results, explain async error handling',
        role: 'user'
      });
      
      const messageCount = await page.locator('[data-testid^="message-"]').count();
      expect(messageCount).toBe(1);
    });
  });

  test('Context management limits and edge cases', async ({ page }) => {
    test.setTimeout(TEST_TIMEOUT);
    
    await test.step('Test context item limits', async () => {
      const conversationId = await helpers.createConversation({
        title: 'Context Limits Test',
        role: 'Engineer'
      });
      
      // Add contexts up to the limit (assuming max is 20 based on test config)
      const maxContextItems = 22; // Try to exceed limit
      
      for (let i = 0; i < maxContextItems; i++) {
        await helpers.addManualContext({
          title: `Context Item ${i + 1}`,
          content: `This is test content for context item number ${i + 1}. It contains meaningful information for testing purposes.`,
          contextType: 'user_input'
        });
        
        // Check if we've hit the limit
        const contextCount = await page.locator('[data-testid^="context-item-"]').count();
        
        if (contextCount < i + 1) {
          // We've hit the limit - verify warning message
          await expect(page.locator('[data-testid="context-limit-warning"]')).toBeVisible();
          break;
        }
      }
      
      // Verify final context count doesn't exceed limit
      const finalContextCount = await page.locator('[data-testid^="context-item-"]').count();
      expect(finalContextCount).toBeLessThanOrEqual(20);
    });
    
    await test.step('Test context length limits', async () => {
      // Try to add a very large context item
      const largeContent = 'A'.repeat(60000); // 60KB content
      
      await helpers.addManualContext({
        title: 'Large Content Test',
        content: largeContent,
        contextType: 'document'
      });
      
      // Check if content was truncated or warning shown
      const contextItem = page.locator('[data-testid="context-item-0"]');
      await expect(contextItem).toBeVisible();
      
      // Check for truncation indicator or warning
      const hasTruncationWarning = await page.locator('[data-testid="content-truncated-warning"]').isVisible();
      const hasLengthWarning = await page.locator('[data-testid="context-length-warning"]').isVisible();
      
      expect(hasTruncationWarning || hasLengthWarning).toBe(true);
    });
    
    await test.step('Test context removal and cleanup', async () => {
      // Add a few context items
      for (let i = 0; i < 3; i++) {
        await helpers.addManualContext({
          title: `Removable Context ${i + 1}`,
          content: `Content for context item ${i + 1}`,
          contextType: 'user_input'
        });
      }
      
      let contextCount = await page.locator('[data-testid^="context-item-"]').count();
      expect(contextCount).toBe(3);
      
      // Remove middle context item
      await page.click('[data-testid="context-item-1"] [data-testid="remove-context"]');
      
      // Confirm removal
      await page.click('[data-testid="confirm-remove-context"]');
      
      // Verify context removed and others remained
      contextCount = await page.locator('[data-testid^="context-item-"]').count();
      expect(contextCount).toBe(2);
      
      // Verify remaining contexts are still accessible
      await expect(page.locator('[data-testid="context-title-0"]'))
        .toHaveText('Removable Context 1');
      await expect(page.locator('[data-testid="context-title-1"]'))
        .toHaveText('Removable Context 3');
    });
  });

  test('Error handling and recovery workflow', async ({ page }) => {
    test.setTimeout(TEST_TIMEOUT);
    
    await test.step('Test network error handling', async () => {
      // Create conversation normally
      const conversationId = await helpers.createConversation({
        title: 'Error Handling Test',
        role: 'Engineer'
      });
      
      // Simulate network issue by intercepting API calls
      await page.route('**/conversations/*/context', route => {
        route.fulfill({
          status: 500,
          contentType: 'application/json',
          body: JSON.stringify({ error: 'Internal Server Error' })
        });
      });
      
      // Try to add context - should fail gracefully
      await helpers.addManualContext({
        title: 'Network Error Test',
        content: 'This should fail due to network error',
        contextType: 'user_input'
      });
      
      // Verify error message displayed
      await expect(page.locator('[data-testid="context-error-message"]')).toBeVisible();
      await expect(page.locator('[data-testid="context-error-message"]'))
        .toContainText('Failed to add context');
      
      // Verify conversation state unchanged
      const contextCount = await page.locator('[data-testid^="context-item-"]').count();
      expect(contextCount).toBe(0);
    });
    
    await test.step('Test error recovery', async () => {
      // Remove network error simulation
      await page.unroute('**/conversations/*/context');
      
      // Try adding context again - should work
      await page.click('[data-testid="retry-add-context"]');
      
      await expect(page.locator('[data-testid="context-item-0"]')).toBeVisible();
      const contextCount = await page.locator('[data-testid^="context-item-"]').count();
      expect(contextCount).toBe(1);
      
      // Verify error message cleared
      await expect(page.locator('[data-testid="context-error-message"]')).not.toBeVisible();
    });
    
    await test.step('Test invalid data handling', async () => {
      // Try to create conversation with empty title
      await page.click('[data-testid="new-conversation-button"]');
      await page.fill('[data-testid="conversation-title-input"]', '');
      await page.click('[data-testid="create-conversation-confirm"]');
      
      // Verify validation error
      await expect(page.locator('[data-testid="title-validation-error"]')).toBeVisible();
      await expect(page.locator('[data-testid="title-validation-error"]'))
        .toContainText('Title is required');
      
      // Verify conversation not created
      await page.click('[data-testid="cancel-conversation-creation"]');
      const conversationCount = await page.locator('[data-testid^="conversation-"]').count();
      expect(conversationCount).toBe(1); // Only the existing test conversation
    });
  });

  test('Accessibility and keyboard navigation workflow', async ({ page }) => {
    test.setTimeout(TEST_TIMEOUT);
    
    await test.step('Test keyboard navigation for conversation creation', async () => {
      // Focus new conversation button with Tab
      await page.keyboard.press('Tab');
      await expect(page.locator('[data-testid="new-conversation-button"]')).toBeFocused();
      
      // Activate with Enter
      await page.keyboard.press('Enter');
      await expect(page.locator('[data-testid="conversation-creation-modal"]')).toBeVisible();
      
      // Navigate form with Tab
      await page.keyboard.press('Tab');
      await expect(page.locator('[data-testid="conversation-title-input"]')).toBeFocused();
      
      await page.keyboard.type('Keyboard Navigation Test');
      
      await page.keyboard.press('Tab');
      await expect(page.locator('[data-testid="conversation-role-select"]')).toBeFocused();
      
      await page.keyboard.press('ArrowDown');
      await page.keyboard.press('Enter'); // Select role
      
      await page.keyboard.press('Tab');
      await page.keyboard.press('Enter'); // Create conversation
      
      // Verify conversation created
      await expect(page.locator('[data-testid="current-conversation-title"]'))
        .toHaveText('Keyboard Navigation Test');
    });
    
    await test.step('Test screen reader accessibility', async () => {
      // Check for proper ARIA labels
      await expect(page.locator('[data-testid="conversations-list"]'))
        .toHaveAttribute('aria-label', 'Conversations list');
      
      await expect(page.locator('[data-testid="context-panel"]'))
        .toHaveAttribute('aria-label', 'Context panel');
      
      await expect(page.locator('[data-testid="message-input"]'))
        .toHaveAttribute('aria-label', 'Message input');
      
      // Check for proper heading hierarchy
      const headings = page.locator('h1, h2, h3, h4, h5, h6');
      const headingLevels = await headings.evaluateAll(elements => 
        elements.map(el => parseInt(el.tagName.substring(1)))
      );
      
      // Verify no heading level skips
      for (let i = 1; i < headingLevels.length; i++) {
        expect(headingLevels[i] - headingLevels[i-1]).toBeLessThanOrEqual(1);
      }
    });
    
    await test.step('Test high contrast mode compatibility', async () => {
      // Enable high contrast mode simulation
      await page.emulateMedia({ forcedColors: 'active' });
      
      // Verify critical UI elements remain visible
      await expect(page.locator('[data-testid="new-conversation-button"]')).toBeVisible();
      await expect(page.locator('[data-testid="message-input"]')).toBeVisible();
      await expect(page.locator('[data-testid="send-message-button"]')).toBeVisible();
      
      // Check contrast ratios for text elements
      const titleElement = page.locator('[data-testid="current-conversation-title"]');
      await expect(titleElement).toBeVisible();
      
      // Reset media simulation
      await page.emulateMedia({ forcedColors: null });
    });
  });

  test('Performance workflow with large datasets', async ({ page }) => {
    test.setTimeout(TEST_TIMEOUT * 2);
    
    await test.step('Test performance with many conversations', async () => {
      // Create multiple conversations quickly
      const conversationPromises = [];
      for (let i = 0; i < 10; i++) {
        conversationPromises.push(
          helpers.createConversation({
            title: `Performance Test Conversation ${i + 1}`,
            role: 'Engineer'
          })
        );
      }
      
      const conversationIds = await Promise.all(conversationPromises);
      expect(conversationIds).toHaveLength(10);
      
      // Measure sidebar rendering time
      const startTime = Date.now();
      await expect(page.locator('[data-testid^="conversation-"]').nth(9)).toBeVisible();
      const renderTime = Date.now() - startTime;
      
      expect(renderTime).toBeLessThan(2000); // Should render within 2 seconds
    });
    
    await test.step('Test performance with large context', async () => {
      const conversationId = await helpers.createConversation({
        title: 'Large Context Test',
        role: 'Engineer'
      });
      
      // Add many context items
      const contextPromises = [];
      for (let i = 0; i < 15; i++) {
        contextPromises.push(
          helpers.addManualContext({
            title: `Performance Context ${i + 1}`,
            content: `Large content block ${i + 1} `.repeat(100), // ~2KB each
            contextType: 'document'
          })
        );
      }
      
      await Promise.all(contextPromises);
      
      // Verify context panel remains responsive
      const contextPanel = page.locator('[data-testid="context-panel"]');
      await expect(contextPanel).toBeVisible();
      
      // Test scrolling performance
      await contextPanel.scrollIntoViewIfNeeded();
      
      // Verify all context items rendered
      const contextCount = await page.locator('[data-testid^="context-item-"]').count();
      expect(contextCount).toBe(15);
    });
  });
});

// Helper function to wait for application stability
async function waitForApplicationStability(page: Page): Promise<void> {
  // Wait for any pending network requests to complete
  await page.waitForLoadState('networkidle');
  
  // Wait for main UI elements to be stable
  await expect(page.locator('[data-testid="app-container"]')).toBeVisible();
  await expect(page.locator('[data-testid="sidebar"]')).toBeVisible();
  await expect(page.locator('[data-testid="main-content"]')).toBeVisible();
  
  // Small delay for any final UI updates
  await page.waitForTimeout(UI_INTERACTION_DELAY);
}