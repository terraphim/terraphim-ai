/**
 * Helper functions for Context Management UI tests
 *
 * This module provides reusable helper functions for testing context management
 * functionality, including conversation operations, context manipulation, and
 * UI interaction utilities.
 */

import { Page, expect, Locator } from '@playwright/test';
import type { Document } from '../../src/lib/types';

export interface ConversationData {
  title: string;
  role: string;
  id?: string;
}

export interface ContextData {
  contextType: 'document' | 'search_result' | 'user_input' | 'system' | 'external';
  title: string;
  content: string;
  source?: string;
  metadata?: Record<string, string>;
}

export interface MessageData {
  role: 'user' | 'assistant' | 'system';
  content: string;
}

/**
 * Context Management Helper Class
 */
export class ContextTestHelpers {
  constructor(private page: Page) {}

  /**
   * Navigation helpers
   */
  async navigateToConversations(): Promise<void> {
    await this.page.click('[data-testid="conversations-tab"]');
    await this.page.waitForLoadState('networkidle');
  }

  async navigateToChat(): Promise<void> {
    await this.page.click('[data-testid="chat-tab"]');
    await this.page.waitForLoadState('networkidle');
  }

  async navigateToSearch(): Promise<void> {
    await this.page.click('[data-testid="search-tab"]');
    await this.page.waitForLoadState('networkidle');
  }

  /**
   * Wait for application to be ready
   */
  async waitForApplicationReady(): Promise<void> {
    // Wait for main UI elements to load
    await expect(this.page.locator('[data-testid="app-container"]')).toBeVisible({ timeout: 30000 });
    await expect(this.page.locator('[data-testid="sidebar"]')).toBeVisible();
    await expect(this.page.locator('[data-testid="main-content"]')).toBeVisible();

    // Wait for any initial loading to complete
    await this.page.waitForLoadState('networkidle');
    await this.page.waitForTimeout(1000); // Brief stabilization delay
  }

  /**
   * Clear application state
   */
  async clearApplicationState(): Promise<void> {
    // Clear any existing conversations, contexts, etc.
    try {
      await this.page.evaluate(() => {
        // Clear localStorage
        localStorage.clear();

        // Clear sessionStorage
        sessionStorage.clear();

        // Reset any global application state if available
        if ((window as any).clearAppState) {
          (window as any).clearAppState();
        }
      });

      // Reload page to ensure clean state
      await this.page.reload();
      await this.waitForApplicationReady();
    } catch (error) {
      console.warn('Could not clear application state:', error);
    }
  }

  /**
   * Select an existing conversation
   */
  async selectConversation(conversationId: string): Promise<void> {
    await this.page.click(`[data-testid="conversation-${conversationId}"]`);
    await this.page.waitForLoadState('networkidle');

    // Verify conversation selected
    await expect(this.page.locator('[data-testid="current-conversation"]')).toBeVisible();
  }

  /**
   * Delete a conversation
   */
  async deleteConversation(conversationId: string): Promise<void> {
    // Right-click on conversation to open context menu
    await this.page.click(`[data-testid="conversation-${conversationId}"]`, { button: 'right' });

    // Click delete option
    await this.page.click('[data-testid="delete-conversation"]');

    // Confirm deletion
    await this.page.click('[data-testid="confirm-delete"]');

    // Wait for deletion to complete
    await expect(this.page.locator(`[data-testid="conversation-${conversationId}"]`)).not.toBeVisible();
    await this.page.waitForLoadState('networkidle');
  }

  /**
   * Conversation management helpers
   */
  async createConversation(data: ConversationData): Promise<string> {
    await this.navigateToConversations();

    // Click new conversation button
    await this.page.click('[data-testid="new-conversation-button"]');

    // Fill conversation details
    await this.page.fill('[data-testid="conversation-title-input"]', data.title);
    await this.page.selectOption('[data-testid="conversation-role-select"]', data.role);

    // Create conversation
    await this.page.click('[data-testid="create-conversation-confirm"]');

    // Wait for creation and get ID from URL or element
    await this.page.waitForSelector('[data-testid="active-conversation-title"]');

    // Return the conversation ID (this may need adjustment based on actual implementation)
    const conversationId = await this.page.getAttribute('[data-testid="conversation-details"]', 'data-conversation-id');

    if (!conversationId) {
      throw new Error('Failed to get conversation ID after creation');
    }

    return conversationId;
  }

  async openConversation(conversationTitle: string): Promise<void> {
    await this.navigateToConversations();
    await this.page.click(`[data-testid="conversation-item"]:has-text("${conversationTitle}")`);
    await this.page.waitForSelector('[data-testid="conversation-details"]');
  }

  async getConversationList(): Promise<ConversationData[]> {
    await this.navigateToConversations();
    await this.page.waitForSelector('[data-testid="conversation-list"]');

    const conversations: ConversationData[] = [];
    const conversationItems = this.page.locator('[data-testid="conversation-item"]');

    const count = await conversationItems.count();

    for (let i = 0; i < count; i++) {
      const item = conversationItems.nth(i);
      const title = await item.locator('[data-testid="conversation-title"]').textContent();
      const role = await item.locator('[data-testid="conversation-role"]').textContent();
      const id = await item.getAttribute('data-conversation-id');

      if (title && role) {
        conversations.push({
          title: title.trim(),
          role: role.trim(),
          id: id || undefined
        });
      }
    }

    return conversations;
  }

  async deleteConversation(conversationTitle: string): Promise<void> {
    await this.openConversation(conversationTitle);

    // Open conversation menu
    await this.page.click('[data-testid="conversation-menu-button"]');
    await this.page.click('[data-testid="delete-conversation-button"]');

    // Confirm deletion
    await this.page.click('[data-testid="confirm-delete-conversation"]');

    // Wait for redirect to conversations list
    await this.page.waitForSelector('[data-testid="conversation-list"]');
  }

  /**
   * Message management helpers
   */
  async addMessage(data: MessageData): Promise<void> {
    // Fill message content
    await this.page.fill('[data-testid="message-input"]', data.content);

    // Select role if not default user
    if (data.role !== 'user') {
      await this.page.selectOption('[data-testid="message-role-select"]', data.role);
    }

    // Send message
    await this.page.click('[data-testid="send-message-button"]');

    // Wait for message to appear
    await this.page.waitForSelector(`[data-testid="message-item"]:has-text("${data.content}")`);
  }

  async getMessages(): Promise<MessageData[]> {
    const messages: MessageData[] = [];
    const messageItems = this.page.locator('[data-testid="message-item"]');

    const count = await messageItems.count();

    for (let i = 0; i < count; i++) {
      const item = messageItems.nth(i);
      const role = await item.getAttribute('data-role') as 'user' | 'assistant' | 'system';
      const content = await item.locator('[data-testid="message-content"]').textContent();

      if (role && content) {
        messages.push({
          role,
          content: content.trim()
        });
      }
    }

    return messages;
  }

  async waitForAssistantResponse(timeoutMs: number = 30000): Promise<string> {
    // Wait for thinking indicator to appear and disappear
    await this.page.waitForSelector('[data-testid="assistant-thinking"]', { timeout: 5000 });
    await this.page.waitForSelector('[data-testid="assistant-thinking"]', {
      state: 'detached',
      timeout: timeoutMs
    });

    // Get the latest assistant message
    const assistantMessages = this.page.locator('[data-testid="message-item"][data-role="assistant"]');
    const lastMessage = assistantMessages.last();

    const content = await lastMessage.locator('[data-testid="message-content"]').textContent();

    if (!content) {
      throw new Error('No assistant response received');
    }

    return content.trim();
  }

  /**
   * Context management helpers
   */
  async addManualContext(data: ContextData): Promise<void> {
    // Open add context dialog
    await this.page.click('[data-testid="add-manual-context-button"]');

    // Fill context details
    await this.page.selectOption('[data-testid="context-type-select"]', data.type);
    await this.page.fill('[data-testid="context-title-input"]', data.title);
    await this.page.fill('[data-testid="context-content-textarea"]', data.content);

    // Add metadata if provided
    if (data.metadata) {
      for (const [key, value] of Object.entries(data.metadata)) {
        await this.page.click('[data-testid="add-metadata-button"]');

        // Fill the last metadata key-value pair
        const metadataItems = this.page.locator('[data-testid="metadata-item"]');
        const lastItem = metadataItems.last();

        await lastItem.locator('[data-testid="metadata-key-input"]').fill(key);
        await lastItem.locator('[data-testid="metadata-value-input"]').fill(value);
      }
    }

    // Save context
    await this.page.click('[data-testid="save-context-button"]');

    // Wait for context to appear in the list
    await this.page.waitForSelector(`[data-testid="context-item"]:has-text("${data.title}")`);
  }

  async addSearchContext(query: string, documents: Document[], limit?: number): Promise<void> {
    // Navigate to search first
    await this.navigateToSearch();

    // Perform search (mock or real depending on test setup)
    await this.page.fill('[data-testid="search-input"]', query);
    await this.page.click('[data-testid="search-button"]');

    // Wait for results
    await this.page.waitForSelector('[data-testid="search-results"]');

    // Select documents (limit to specified number)
    const documentsToSelect = limit ? documents.slice(0, limit) : documents;

    for (const doc of documentsToSelect) {
      // Find and select the document checkbox
      const docItem = this.page.locator(`[data-testid="search-result-item"]:has-text("${doc.title}")`);
      await docItem.locator('[data-testid="search-result-checkbox"]').check();
    }

    // Add selected results as context
    await this.page.click('[data-testid="add-to-context-button"]');

    // Select target conversation (assumes we're in a conversation context)
    const activeConversationTitle = await this.page.locator('[data-testid="active-conversation-title"]').textContent();
    if (activeConversationTitle) {
      await this.page.selectOption('[data-testid="target-conversation-select"]', activeConversationTitle);
    }

    await this.page.click('[data-testid="confirm-add-context"]');

    // Navigate back to conversation to see the added context
    await this.navigateToConversations();
    if (activeConversationTitle) {
      await this.openConversation(activeConversationTitle);
    }
  }

  async getContextItems(): Promise<ContextData[]> {
    const contexts: ContextData[] = [];
    const contextItems = this.page.locator('[data-testid="context-item"]');

    const count = await contextItems.count();

    for (let i = 0; i < count; i++) {
      const item = contextItems.nth(i);
      const type = await item.getAttribute('data-context-type') as ContextData['type'];
      const title = await item.locator('[data-testid="context-title"]').textContent();
      const content = await item.locator('[data-testid="context-content"]').textContent();

      if (type && title && content) {
        // Get metadata if available
        const metadata: Record<string, string> = {};
        const metadataItems = item.locator('[data-testid="context-metadata-item"]');
        const metadataCount = await metadataItems.count();

        for (let j = 0; j < metadataCount; j++) {
          const metadataItem = metadataItems.nth(j);
          const key = await metadataItem.getAttribute('data-key');
          const value = await metadataItem.getAttribute('data-value');

          if (key && value) {
            metadata[key] = value;
          }
        }

        contexts.push({
          type,
          title: title.trim(),
          content: content.trim(),
          metadata: Object.keys(metadata).length > 0 ? metadata : undefined
        });
      }
    }

    return contexts;
  }

  async editContext(originalTitle: string, newData: Partial<ContextData>): Promise<void> {
    const contextItem = this.page.locator(`[data-testid="context-item"]:has-text("${originalTitle}")`);

    // Hover to show edit button
    await contextItem.hover();
    await contextItem.locator('[data-testid="edit-context-button"]').click();

    // Update fields
    if (newData.title) {
      await this.page.fill('[data-testid="context-title-input"]', newData.title);
    }

    if (newData.content) {
      await this.page.fill('[data-testid="context-content-textarea"]', newData.content);
    }

    if (newData.type) {
      await this.page.selectOption('[data-testid="context-type-select"]', newData.type);
    }

    // Update metadata if provided
    if (newData.metadata) {
      // Clear existing metadata first
      const removeButtons = this.page.locator('[data-testid="remove-metadata-button"]');
      const removeCount = await removeButtons.count();

      for (let i = 0; i < removeCount; i++) {
        await removeButtons.first().click();
      }

      // Add new metadata
      for (const [key, value] of Object.entries(newData.metadata)) {
        await this.page.click('[data-testid="add-metadata-button"]');

        const metadataItems = this.page.locator('[data-testid="metadata-item"]');
        const lastItem = metadataItems.last();

        await lastItem.locator('[data-testid="metadata-key-input"]').fill(key);
        await lastItem.locator('[data-testid="metadata-value-input"]').fill(value);
      }
    }

    // Save changes
    await this.page.click('[data-testid="save-context-button"]');

    // Wait for updated context to appear
    const expectedTitle = newData.title || originalTitle;
    await this.page.waitForSelector(`[data-testid="context-item"]:has-text("${expectedTitle}")`);
  }

  async removeContext(contextTitle: string): Promise<void> {
    const contextItem = this.page.locator(`[data-testid="context-item"]:has-text("${contextTitle}")`);

    // Hover to show remove button
    await contextItem.hover();
    await contextItem.locator('[data-testid="remove-context-button"]').click();

    // Confirm removal
    await this.page.click('[data-testid="confirm-remove-context"]');

    // Wait for context to be removed
    await this.page.waitForSelector(`[data-testid="context-item"]:has-text("${contextTitle}")`, {
      state: 'detached'
    });
  }

  /**
   * Validation and assertion helpers
   */
  async assertConversationExists(title: string): Promise<void> {
    await this.navigateToConversations();
    await expect(this.page.locator(`[data-testid="conversation-item"]:has-text("${title}")`)).toBeVisible();
  }

  async assertMessageExists(content: string, role?: MessageData['role']): Promise<void> {
    const messageSelector = role
      ? `[data-testid="message-item"][data-role="${role}"]:has-text("${content}")`
      : `[data-testid="message-item"]:has-text("${content}")`;

    await expect(this.page.locator(messageSelector)).toBeVisible();
  }

  async assertContextExists(title: string, type?: ContextData['type']): Promise<void> {
    const contextSelector = type
      ? `[data-testid="context-item"][data-context-type="${type}"]:has-text("${title}")`
      : `[data-testid="context-item"]:has-text("${title}")`;

    await expect(this.page.locator(contextSelector)).toBeVisible();
  }

  async assertContextCount(expectedCount: number): Promise<void> {
    await expect(this.page.locator('[data-testid="context-item"]')).toHaveCount(expectedCount);
  }

  async assertMessageCount(expectedCount: number): Promise<void> {
    await expect(this.page.locator('[data-testid="message-item"]')).toHaveCount(expectedCount);
  }

  async assertConversationTitle(expectedTitle: string): Promise<void> {
    await expect(this.page.locator('[data-testid="active-conversation-title"]')).toHaveText(expectedTitle);
  }

  async assertConversationRole(expectedRole: string): Promise<void> {
    await expect(this.page.locator('[data-testid="active-conversation-role"]')).toContainText(expectedRole);
  }

  /**
   * Performance and interaction helpers
   */
  async measureContextLoadTime(): Promise<number> {
    const startTime = Date.now();
    await this.page.waitForSelector('[data-testid="context-item"]');
    return Date.now() - startTime;
  }

  async measureMessageSendTime(): Promise<number> {
    const startTime = Date.now();
    await this.page.click('[data-testid="send-message-button"]');
    await this.page.waitForSelector('[data-testid="message-item"]', { timeout: 30000 });
    return Date.now() - startTime;
  }

  async simulateNetworkDelay(delayMs: number): Promise<void> {
    await this.page.route('**/*', async (route) => {
      await new Promise(resolve => setTimeout(resolve, delayMs));
      await route.continue();
    });
  }

  /**
   * Accessibility helpers
   */
  async checkKeyboardNavigation(): Promise<void> {
    // Test tab navigation through context management UI
    await this.page.keyboard.press('Tab');
    const focused = await this.page.evaluate(() => document.activeElement?.getAttribute('data-testid'));
    expect(focused).toBeTruthy();
  }

  async checkAriaLabels(): Promise<void> {
    // Verify important elements have proper ARIA labels
    const importantSelectors = [
      '[data-testid="conversation-messages"]',
      '[data-testid="conversation-context"]',
      '[data-testid="add-manual-context-button"]',
      '[data-testid="send-message-button"]'
    ];

    for (const selector of importantSelectors) {
      const element = this.page.locator(selector);
      if (await element.count() > 0) {
        const ariaLabel = await element.getAttribute('aria-label');
        const hasAriaLabelledBy = await element.getAttribute('aria-labelledby');

        expect(ariaLabel || hasAriaLabelledBy).toBeTruthy();
      }
    }
  }

  /**
   * Mock and test data helpers
   */
  async mockTauriInvoke(): Promise<void> {
    await this.page.addInitScript(() => {
      // @ts-ignore
      window.__TAURI__ = {
        invoke: async (command: string, args: any) => {
          // Mock Tauri invoke responses for testing
          switch (command) {
            case 'create_conversation':
              return {
                status: 'Success',
                conversation_id: `mock-conv-${Date.now()}`,
                error: null
              };
            case 'add_context_to_conversation':
              return { status: 'Success', error: null };
            case 'add_message_to_conversation':
              return {
                status: 'Success',
                message_id: `mock-msg-${Date.now()}`,
                error: null
              };
            case 'get_conversation':
              return {
                status: 'Success',
                conversation: {
                  id: args.conversation_id,
                  title: 'Mock Conversation',
                  role: { name: 'Engineer' },
                  messages: [],
                  global_context: []
                },
                error: null
              };
            case 'list_conversations':
              return {
                status: 'Success',
                conversations: [],
                error: null
              };
            default:
              return { status: 'Error', error: `Unknown command: ${command}` };
          }
        }
      };
    });
  }
}

/**
 * Factory function to create context test helpers
 */
export function createContextHelpers(page: Page): ContextTestHelpers {
  return new ContextTestHelpers(page);
}
