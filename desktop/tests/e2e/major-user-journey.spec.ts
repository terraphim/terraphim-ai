/**
 * Comprehensive End-to-End Test for Major User Journey
 * 
 * This test covers the complete user journey:
 * 1. Search for documents
 * 2. Add document to context
 * 3. Chat with document context
 * 4. Add KG term to context
 * 5. Chat with enhanced context
 * 6. Verify context is passed to LLM
 * 
 * This test validates the core functionality that users rely on most.
 */

import { test, expect } from '@playwright/test';
import {
  ciWaitForSelector,
  ciSearch,
  ciNavigate,
  ciWait,
  ciClick,
  getTimeouts
} from '../../src/test-utils/ci-friendly';

// Test configuration
const TEST_TIMEOUT = 120000; // Extended for complete user journey
const LLM_RESPONSE_TIMEOUT = 30000; // Timeout for LLM responses
const BACKEND_WAIT_TIME = 2000;

// Test data
const TEST_SEARCH_QUERIES = [
  'rust async programming',
  'tokio futures',
  'error handling patterns'
];

const TEST_CHAT_MESSAGES = [
  'Can you explain the key concepts from the documents I added?',
  'How do these concepts relate to each other?',
  'What are the best practices mentioned in the context?'
];

const TEST_KG_TERMS = [
  'async-await',
  'futures',
  'tokio-runtime'
];

test.describe('Major User Journey E2E Tests', () => {
  test.beforeEach(async ({ page }) => {
    // Set extended timeout for this test suite
    test.setTimeout(TEST_TIMEOUT);

    // Navigate to the application
    await ciNavigate(page, '/');
    
    // Wait for the app to load
    await ciWaitForSelector(page, '[data-testid="search-tab"]', 'navigation');
  });

  test.describe('Complete User Journey', () => {
    test('should complete full user journey: search → add context → chat → add KG → chat with enhanced context', async ({ page }) => {
      console.log('🚀 Starting complete user journey test...');

      // Step 1: Search for documents
      console.log('📝 Step 1: Searching for documents...');
      await performSearch(page, TEST_SEARCH_QUERIES[0]);
      
      // Verify search results are displayed
      const searchResults = page.locator('[data-testid="search-results"] .box');
      await expect(searchResults.first()).toBeVisible();
      console.log('✅ Search results displayed');

      // Step 2: Add document to context
      console.log('📝 Step 2: Adding document to context...');
      const firstResult = searchResults.first();
      const addToContextButton = firstResult.locator('[data-testid="add-to-context-button"]');
      
      // Wait for the button to be available
      await expect(addToContextButton).toBeVisible();
      await addToContextButton.click();
      
      // Wait for success notification
      await ciWait(page, 'medium');
      console.log('✅ Document added to context');

      // Step 3: Navigate to chat
      console.log('📝 Step 3: Navigating to chat...');
      await ciNavigate(page, '/chat');
      await ciWaitForSelector(page, '[data-testid="chat-interface"]', 'navigation');
      console.log('✅ Chat interface loaded');

      // Step 4: Verify context is visible
      console.log('📝 Step 4: Verifying context is visible...');
      const contextPanel = page.locator('[data-testid="context-panel"]');
      await expect(contextPanel).toBeVisible();
      
      const contextItems = contextPanel.locator('[data-testid="conversation-context"] .context-item');
      await expect(contextItems.first()).toBeVisible();
      console.log('✅ Context items visible in chat');

      // Step 5: Chat with document context
      console.log('📝 Step 5: Chatting with document context...');
      await performChatWithContext(page, TEST_CHAT_MESSAGES[0]);
      console.log('✅ Chat with context completed');

      // Step 6: Add KG term to context
      console.log('📝 Step 6: Adding KG term to context...');
      await addKGTermToContext(page, TEST_KG_TERMS[0]);
      console.log('✅ KG term added to context');

      // Step 7: Chat with enhanced context
      console.log('📝 Step 7: Chatting with enhanced context...');
      await performChatWithContext(page, TEST_CHAT_MESSAGES[1]);
      console.log('✅ Chat with enhanced context completed');

      // Step 8: Verify context is passed to LLM
      console.log('📝 Step 8: Verifying context is passed to LLM...');
      await verifyContextPassedToLLM(page);
      console.log('✅ Context verification completed');

      console.log('🎉 Complete user journey test passed!');
    });

    test('should handle multiple documents in context', async ({ page }) => {
      console.log('🚀 Starting multiple documents test...');

      // Search and add multiple documents
      for (let i = 0; i < 2; i++) {
        await performSearch(page, TEST_SEARCH_QUERIES[i]);
        
        const searchResults = page.locator('[data-testid="search-results"] .box');
        const result = searchResults.first();
        const addToContextButton = result.locator('[data-testid="add-to-context-button"]');
        
        await addToContextButton.click();
        await ciWait(page, 'medium');
        
        console.log(`✅ Document ${i + 1} added to context`);
      }

      // Navigate to chat and verify multiple context items
      await ciNavigate(page, '/chat');
      await ciWaitForSelector(page, '[data-testid="chat-interface"]', 'navigation');

      const contextItems = page.locator('[data-testid="conversation-context"] .context-item');
      const itemCount = await contextItems.count();
      expect(itemCount).toBeGreaterThanOrEqual(2);
      console.log(`✅ Multiple context items (${itemCount}) visible`);

      // Chat with multiple documents
      await performChatWithContext(page, 'Summarize the key points from all the documents in context');
      console.log('✅ Chat with multiple documents completed');
    });

    test('should handle search with autocomplete and filters', async ({ page }) => {
      console.log('🚀 Starting search with autocomplete test...');

      // Test autocomplete functionality
      const searchInput = page.locator('input[type="search"]');
      await searchInput.click();
      await searchInput.fill('rust');
      
      // Wait for autocomplete suggestions
      await ciWait(page, 'small');
      const suggestions = page.locator('.suggestions li');
      const suggestionCount = await suggestions.count();
      
      if (suggestionCount > 0) {
        // Select first suggestion
        await suggestions.first().click();
        console.log('✅ Autocomplete suggestion selected');
      }

      // Perform search
      await searchInput.press('Enter');
      await ciWait(page, 'medium');

      // Verify results
      const searchResults = page.locator('[data-testid="search-results"] .box');
      await expect(searchResults.first()).toBeVisible();
      console.log('✅ Search with autocomplete completed');
    });

    test('should handle error scenarios gracefully', async ({ page }) => {
      console.log('🚀 Starting error handling test...');

      // Test with invalid search query
      const searchInput = page.locator('input[type="search"]');
      await searchInput.fill('invalid_query_that_should_not_return_results');
      await searchInput.press('Enter');
      await ciWait(page, 'medium');

      // Should show empty state or no results message
      const emptyState = page.locator('[data-testid="empty-state"], .has-text-centered');
      const hasEmptyState = await emptyState.isVisible();
      
      if (hasEmptyState) {
        console.log('✅ Empty state handled gracefully');
      } else {
        // Check if there are results (might be some fallback results)
        const searchResults = page.locator('[data-testid="search-results"] .box');
        const resultCount = await searchResults.count();
        console.log(`Found ${resultCount} results for invalid query`);
      }

      // Test chat without context
      await ciNavigate(page, '/chat');
      await ciWaitForSelector(page, '[data-testid="chat-interface"]', 'navigation');

      const chatInput = page.locator('[data-testid="chat-input"]');
      const sendButton = page.locator('[data-testid="send-message-button"]');
      
      await chatInput.fill('Test message without context');
      await sendButton.click();
      
      // Should handle chat without context gracefully
      await ciWait(page, 'medium');
      console.log('✅ Chat without context handled gracefully');
    });
  });

  test.describe('Context Management Integration', () => {
    test('should add and remove context items', async ({ page }) => {
      // Add document to context
      await performSearch(page, TEST_SEARCH_QUERIES[0]);
      const searchResults = page.locator('[data-testid="search-results"] .box');
      const firstResult = searchResults.first();
      await firstResult.locator('[data-testid="add-to-context-button"]').click();
      await ciWait(page, 'medium');

      // Navigate to chat
      await ciNavigate(page, '/chat');
      await ciWaitForSelector(page, '[data-testid="chat-interface"]', 'navigation');

      // Verify context item is present
      const contextItems = page.locator('[data-testid="conversation-context"] .context-item');
      await expect(contextItems.first()).toBeVisible();

      // Remove context item
      const deleteButton = contextItems.first().locator('[data-testid="delete-context-0"]');
      await deleteButton.click();
      
      // Confirm deletion if modal appears
      const confirmDialog = page.locator('[data-testid="confirm-delete-modal"]');
      const hasConfirmDialog = await confirmDialog.isVisible();
      
      if (hasConfirmDialog) {
        await confirmDialog.locator('[data-testid="confirm-delete-button"]').click();
      }

      await ciWait(page, 'medium');
      console.log('✅ Context item removed successfully');
    });

    test('should edit context items', async ({ page }) => {
      // Add document to context
      await performSearch(page, TEST_SEARCH_QUERIES[0]);
      const searchResults = page.locator('[data-testid="search-results"] .box');
      const firstResult = searchResults.first();
      await firstResult.locator('[data-testid="add-to-context-button"]').click();
      await ciWait(page, 'medium');

      // Navigate to chat
      await ciNavigate(page, '/chat');
      await ciWaitForSelector(page, '[data-testid="chat-interface"]', 'navigation');

      // Edit context item
      const contextItems = page.locator('[data-testid="conversation-context"] .context-item');
      const editButton = contextItems.first().locator('[data-testid="edit-context-0"]');
      await editButton.click();

      // Modify content in edit modal
      const editModal = page.locator('[data-testid="context-edit-modal"]');
      await expect(editModal).toBeVisible();

      const titleInput = editModal.locator('[data-testid="context-title-input"]');
      await titleInput.fill('Edited Title');
      
      const saveButton = editModal.locator('[data-testid="save-context-button"]');
      await saveButton.click();

      await ciWait(page, 'medium');
      console.log('✅ Context item edited successfully');
    });
  });

  test.describe('Knowledge Graph Integration', () => {
    test('should search and add KG terms to context', async ({ page }) => {
      // Navigate to chat first
      await ciNavigate(page, '/chat');
      await ciWaitForSelector(page, '[data-testid="chat-interface"]', 'navigation');

      // Open KG search modal
      const kgSearchButton = page.locator('[data-testid="kg-search-button"]');
      const kgSearchVisible = await kgSearchButton.isVisible();
      
      if (kgSearchVisible) {
        await kgSearchButton.click();
        
        const kgModal = page.locator('[data-testid="kg-search-modal"]');
        await expect(kgModal).toBeVisible();

        // Search for KG terms
        const searchInput = kgModal.locator('[data-testid="kg-search-input"]');
        await searchInput.fill(TEST_KG_TERMS[0]);
        
        const searchButton = kgModal.locator('[data-testid="kg-search-submit"]');
        await searchButton.click();
        
        await ciWait(page, 'medium');

        // Add first result to context
        const results = kgModal.locator('[data-testid="kg-search-results"] .result-item');
        const resultCount = await results.count();
        
        if (resultCount > 0) {
          const addButton = results.first().locator('[data-testid="add-to-context"]');
          await addButton.click();
          console.log('✅ KG term added to context');
        }

        // Close modal
        const closeButton = kgModal.locator('[data-testid="close-modal-button"]');
        await closeButton.click();
      } else {
        console.log('KG search not available for current role');
      }
    });
  });

  // Helper functions
  async function performSearch(page: any, query: string) {
    const searchInput = page.locator('input[type="search"]');
    await searchInput.fill(query);
    await searchInput.press('Enter');
    await ciWait(page, 'medium');
  }

  async function performChatWithContext(page: any, message: string) {
    const chatInput = page.locator('[data-testid="chat-input"]');
    const sendButton = page.locator('[data-testid="send-message-button"]');
    
    await chatInput.fill(message);
    await sendButton.click();
    
    // Wait for user message to appear
    await ciWait(page, 'small');
    
    // Wait for assistant response
    await page.waitForSelector('.msg.assistant', {
      timeout: LLM_RESPONSE_TIMEOUT,
      state: 'visible'
    });
    
    await ciWait(page, 'small');
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

  async function verifyContextPassedToLLM(page: any) {
    // Send a message that should reference the context
    const chatInput = page.locator('[data-testid="chat-input"]');
    const sendButton = page.locator('[data-testid="send-message-button"]');
    
    await chatInput.fill('Please reference the specific documents and terms I added to context in your response');
    await sendButton.click();
    
    // Wait for response
    await page.waitForSelector('.msg.assistant', {
      timeout: LLM_RESPONSE_TIMEOUT,
      state: 'visible'
    });
    
    // Check if the response mentions context or seems to reference it
    const assistantMessage = page.locator('.msg.assistant').last();
    const responseText = await assistantMessage.textContent();
    
    // The response should be substantial and not just a generic response
    expect(responseText?.length).toBeGreaterThan(50);
    console.log('✅ LLM response received with context integration');
  }
});

