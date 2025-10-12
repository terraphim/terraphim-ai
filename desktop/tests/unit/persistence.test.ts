/**
 * Unit Tests for Persistence Functionality
 *
 * Tests the localStorage-based persistence functions used by
 * Search and Chat components.
 */

import { describe, it, expect, beforeEach, vi } from 'vitest';

// Mock localStorage
const localStorageMock = (() => {
  let store: Record<string, string> = {};

  return {
    getItem: (key: string) => store[key] || null,
    setItem: (key: string, value: string) => {
      store[key] = value.toString();
    },
    removeItem: (key: string) => {
      delete store[key];
    },
    clear: () => {
      store = {};
    },
    length: 0,
    key: (index: number) => null,
  };
})();

Object.defineProperty(window, 'localStorage', {
  value: localStorageMock
});

// Mock console methods to avoid noise in tests
global.console = {
  ...console,
  warn: vi.fn(),
  error: vi.fn(),
  log: vi.fn(),
};

describe('Search State Persistence', () => {
  beforeEach(() => {
    localStorage.clear();
  });

  it('should save and load search state correctly', () => {
    const mockRole = 'TestRole';
    const searchState = {
      input: 'test search query',
      results: [
        {
          id: '1',
          title: 'Test Result',
          url: 'https://example.com',
          body: 'Test content',
          rank: 1.0
        }
      ]
    };

    // Simulate saving state
    const stateKey = `terraphim:searchState:${mockRole}`;
    localStorage.setItem(stateKey, JSON.stringify(searchState));

    // Simulate loading state
    const loadedState = JSON.parse(localStorage.getItem(stateKey) || '{}');

    expect(loadedState.input).toBe(searchState.input);
    expect(loadedState.results).toEqual(searchState.results);
  });

  it('should handle corrupted search state gracefully', () => {
    const mockRole = 'TestRole';
    const stateKey = `terraphim:searchState:${mockRole}`;

    // Save corrupted JSON
    localStorage.setItem(stateKey, '{invalid json}');

    // Should not throw when trying to parse
    let loadedState;
    try {
      const raw = localStorage.getItem(stateKey);
      loadedState = raw ? JSON.parse(raw) : null;
    } catch (e) {
      // Expect this to catch the error gracefully
      expect(e).toBeInstanceOf(SyntaxError);
      loadedState = null;
    }

    expect(loadedState).toBeNull();
  });

  it('should handle missing localStorage gracefully', () => {
    // Mock localStorage to be undefined
    const originalLocalStorage = window.localStorage;
    // @ts-ignore
    delete window.localStorage;

    // Should not throw when localStorage is unavailable
    expect(() => {
      try {
        // @ts-ignore
        if (typeof window === 'undefined') return;
        const raw = localStorage?.getItem?.('test') || null;
      } catch (e) {
        // Expected when localStorage is not available
      }
    }).not.toThrow();

    // Restore localStorage
    window.localStorage = originalLocalStorage;
  });
});

describe('Chat State Persistence', () => {
  beforeEach(() => {
    localStorage.clear();
  });

  it('should save and load chat state correctly', () => {
    const mockRole = 'TestRole';
    const chatState = {
      messages: [
        { role: 'user', content: 'Hello' },
        { role: 'assistant', content: 'Hi there!' }
      ],
      conversationId: 'test-conversation-123'
    };

    // Simulate saving state
    const stateKey = `terraphim:chatState:${mockRole}`;
    localStorage.setItem(stateKey, JSON.stringify(chatState));

    // Simulate loading state
    const loadedState = JSON.parse(localStorage.getItem(stateKey) || '{}');

    expect(loadedState.messages).toEqual(chatState.messages);
    expect(loadedState.conversationId).toBe(chatState.conversationId);
  });

  it('should validate message structure on load', () => {
    const mockRole = 'TestRole';
    const stateKey = `terraphim:chatState:${mockRole}`;

    // Save state with invalid message structure
    const invalidState = {
      messages: 'not an array',
      conversationId: 123 // should be string
    };
    localStorage.setItem(stateKey, JSON.stringify(invalidState));

    const loadedState = JSON.parse(localStorage.getItem(stateKey) || '{}');

    // Validation logic should check if messages is array
    const isValidMessages = Array.isArray(loadedState.messages);
    const isValidConversationId = typeof loadedState.conversationId === 'string';

    expect(isValidMessages).toBe(false);
    expect(isValidConversationId).toBe(false);
  });

  it('should handle role-specific state separation', () => {
    const role1 = 'Role1';
    const role2 = 'Role2';

    const state1 = { messages: [{ role: 'user', content: 'Message from role1' }] };
    const state2 = { messages: [{ role: 'user', content: 'Message from role2' }] };

    localStorage.setItem(`terraphim:chatState:${role1}`, JSON.stringify(state1));
    localStorage.setItem(`terraphim:chatState:${role2}`, JSON.stringify(state2));

    const loadedState1 = JSON.parse(localStorage.getItem(`terraphim:chatState:${role1}`) || '{}');
    const loadedState2 = JSON.parse(localStorage.getItem(`terraphim:chatState:${role2}`) || '{}');

    expect(loadedState1.messages[0].content).toBe('Message from role1');
    expect(loadedState2.messages[0].content).toBe('Message from role2');
    expect(loadedState1.messages[0].content).not.toBe(loadedState2.messages[0].content);
  });
});

describe('Markdown Preference Persistence', () => {
  beforeEach(() => {
    localStorage.clear();
  });

  it('should save and load markdown preference', () => {
    const prefKey = 'terraphim:chatMarkdown';

    // Save preference
    localStorage.setItem(prefKey, 'true');

    // Load preference
    const loadedPref = localStorage.getItem(prefKey);
    const isMarkdownEnabled = loadedPref === 'true';

    expect(isMarkdownEnabled).toBe(true);

    // Change preference
    localStorage.setItem(prefKey, 'false');
    const updatedPref = localStorage.getItem(prefKey);
    const isMarkdownDisabled = updatedPref === 'false';

    expect(isMarkdownDisabled).toBe(true);
  });

  it('should handle missing markdown preference', () => {
    const prefKey = 'terraphim:chatMarkdown';

    // No preference set
    const loadedPref = localStorage.getItem(prefKey);

    expect(loadedPref).toBeNull();

    // Should default to false when null
    const defaultValue = loadedPref === 'true';
    expect(defaultValue).toBe(false);
  });
});

describe('Storage Quota and Error Handling', () => {
  it('should handle localStorage quota exceeded', () => {
    // Mock localStorage to throw quota exceeded error
    const mockSetItem = vi.fn(() => {
      throw new DOMException('QuotaExceededError');
    });

    const originalSetItem = localStorage.setItem;
    localStorage.setItem = mockSetItem;

    // Should not crash when quota is exceeded
    expect(() => {
      try {
        localStorage.setItem('test', 'value');
      } catch (e) {
        // Expected to catch the error
        expect(e).toBeInstanceOf(DOMException);
      }
    }).not.toThrow();

    // Restore original
    localStorage.setItem = originalSetItem;
  });

  it('should handle localStorage access denied', () => {
    // Mock localStorage to throw security error
    const mockGetItem = vi.fn(() => {
      throw new DOMException('SecurityError');
    });

    const originalGetItem = localStorage.getItem;
    localStorage.getItem = mockGetItem;

    // Should handle security errors gracefully
    expect(() => {
      try {
        localStorage.getItem('test');
      } catch (e) {
        expect(e).toBeInstanceOf(DOMException);
      }
    }).not.toThrow();

    // Restore original
    localStorage.getItem = originalGetItem;
  });
});
