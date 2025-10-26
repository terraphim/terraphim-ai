/**
 * @fileoverview ChatAPIClient - Dual-mode communication layer for Terraphim Chat
 * Supports both Tauri IPC (desktop) and HTTP (web) transparently with automatic retry and error handling
 */

/**
 * ChatAPIClient provides a unified interface for communicating with the Terraphim backend
 * Automatically detects environment (Tauri vs Web) and routes commands appropriately
 *
 * Features:
 * - Dual-mode communication (Tauri IPC + HTTP fallback)
 * - Exponential backoff retry mechanism
 * - Event system for API responses
 * - Command routing table for HTTP endpoints
 * - Automatic error handling and recovery
 *
 * @class
 * @extends EventTarget
 *
 * @example
 * const client = new ChatAPIClient(false, 'http://localhost:8000');
 * const response = await client.sendMessage('engineer', messages, conversationId);
 */
export class ChatAPIClient extends EventTarget {
  /**
   * Create a new ChatAPIClient instance
   * @param {boolean} isTauri - Whether running in Tauri desktop mode
   * @param {string} [serverUrl='http://localhost:8000'] - Backend server URL
   * @param {Object} [options={}] - Additional options
   * @param {number} [options.maxRetries=3] - Maximum retry attempts
   * @param {number} [options.retryDelay=1000] - Initial retry delay in ms
   * @param {number} [options.timeout=30000] - Request timeout in ms
   */
  constructor(isTauri = false, serverUrl = 'http://localhost:8000', options = {}) {
    super();

    this.isTauri = isTauri;
    this.serverUrl = serverUrl;
    this.options = {
      maxRetries: 3,
      retryDelay: 1000,
      timeout: 30000,
      ...options
    };

    /**
     * Command routing table for HTTP endpoints
     * Maps Tauri command names to HTTP endpoints
     * @private
     */
    this._routes = {
      'chat': {
        endpoint: '/chat',
        method: 'POST'
      },
      'create_conversation': {
        endpoint: '/conversations',
        method: 'POST'
      },
      'get_conversation': {
        endpoint: (params) => `/conversations/${params.conversationId}`,
        method: 'GET'
      },
      'list_conversations': {
        endpoint: '/conversations',
        method: 'GET'
      },
      'add_context_to_conversation': {
        endpoint: (params) => `/conversations/${params.conversationId}/context`,
        method: 'POST'
      },
      'update_context': {
        endpoint: (params) => `/conversations/${params.conversationId}/context/${params.contextId}`,
        method: 'PUT'
      },
      'delete_context': {
        endpoint: (params) => `/conversations/${params.conversationId}/context/${params.contextId}`,
        method: 'DELETE'
      },
      'find_documents_for_kg_term': {
        endpoint: (params) => `/roles/${encodeURIComponent(params.roleName)}/kg_search?term=${encodeURIComponent(params.term)}`,
        method: 'GET'
      },
      'get_persistent_conversation': {
        endpoint: (params) => `/conversations/persistent/${params.conversationId}`,
        method: 'GET'
      },
      'create_persistent_conversation': {
        endpoint: '/conversations/persistent',
        method: 'POST'
      },
      'update_persistent_conversation': {
        endpoint: (params) => `/conversations/persistent/${params.conversationId}`,
        method: 'PUT'
      }
    };
  }

  /**
   * Execute a command with automatic routing and retry logic
   * @param {string} command - Command name
   * @param {Object} params - Command parameters
   * @param {number} [retryCount=0] - Current retry attempt
   * @returns {Promise<any>} Command response
   */
  async execute(command, params = {}, retryCount = 0) {
    try {
      let response;

      if (this.isTauri) {
        response = await this._invokeTauri(command, params);
      } else {
        response = await this._invokeHTTP(command, params);
      }

      // Emit success event
      this.dispatchEvent(new CustomEvent('api-success', {
        detail: { command, params, response }
      }));

      return response;
    } catch (error) {
      // Emit error event
      this.dispatchEvent(new CustomEvent('api-error', {
        detail: { command, params, error, retryCount }
      }));

      // Retry with exponential backoff
      if (retryCount < this.options.maxRetries) {
        const delay = this.options.retryDelay * Math.pow(2, retryCount);
        await this._sleep(delay);
        return this.execute(command, params, retryCount + 1);
      }

      // Max retries exceeded
      throw error;
    }
  }

  /**
   * Invoke command via Tauri IPC
   * @private
   * @param {string} command - Command name
   * @param {Object} params - Command parameters
   * @returns {Promise<any>} Command response
   */
  async _invokeTauri(command, params) {
    // Dynamic import to avoid loading Tauri in web mode
    try {
      const { invoke } = await import('@tauri-apps/api/tauri');
      return await invoke(command, params);
    } catch (error) {
      throw new Error(`Tauri IPC error: ${error.message}`);
    }
  }

  /**
   * Invoke command via HTTP
   * @private
   * @param {string} command - Command name
   * @param {Object} params - Command parameters
   * @returns {Promise<any>} Command response
   */
  async _invokeHTTP(command, params) {
    const route = this._routes[command];
    if (!route) {
      throw new Error(`Unknown command: ${command}`);
    }

    const endpoint = typeof route.endpoint === 'function'
      ? route.endpoint(params)
      : route.endpoint;

    const method = route.method;
    const url = `${this.serverUrl}${endpoint}`;

    // Create abort controller for timeout
    const controller = new AbortController();
    const timeoutId = setTimeout(() => controller.abort(), this.options.timeout);

    try {
      const response = await fetch(url, {
        method,
        headers: {
          'Content-Type': 'application/json'
        },
        body: method !== 'GET' ? JSON.stringify(params) : undefined,
        signal: controller.signal
      });

      clearTimeout(timeoutId);

      if (!response.ok) {
        const errorText = await response.text().catch(() => response.statusText);
        throw new Error(`HTTP ${response.status}: ${errorText}`);
      }

      return await response.json();
    } catch (error) {
      clearTimeout(timeoutId);

      if (error.name === 'AbortError') {
        throw new Error(`Request timeout after ${this.options.timeout}ms`);
      }

      throw error;
    }
  }

  /**
   * Sleep for specified duration
   * @private
   * @param {number} ms - Duration in milliseconds
   * @returns {Promise<void>}
   */
  _sleep(ms) {
    return new Promise(resolve => setTimeout(resolve, ms));
  }

  // ============================================
  // HIGH-LEVEL API METHODS
  // ============================================

  /**
   * Send a chat message
   * @param {string} role - Role name
   * @param {Array<Object>} messages - Message history
   * @param {string} [conversationId=null] - Optional conversation ID
   * @returns {Promise<Object>} Chat response
   *
   * @example
   * const response = await client.sendMessage('engineer', [
   *   { role: 'user', content: 'Hello!' }
   * ], 'conv-123');
   */
  async sendMessage(role, messages, conversationId = null) {
    const payload = { role, messages };
    if (conversationId) {
      payload.conversation_id = conversationId;
    }

    return await this.execute('chat', { request: payload });
  }

  /**
   * Create a new conversation
   * @param {string} title - Conversation title
   * @param {string} role - Role name
   * @returns {Promise<Object>} Created conversation
   */
  async createConversation(title, role) {
    return await this.execute('create_conversation', { title, role });
  }

  /**
   * Get conversation by ID
   * @param {string} conversationId - Conversation ID
   * @returns {Promise<Object>} Conversation data
   */
  async getConversation(conversationId) {
    return await this.execute('get_conversation', { conversationId });
  }

  /**
   * List all conversations
   * @returns {Promise<Array<Object>>} Array of conversations
   */
  async getConversations() {
    return await this.execute('list_conversations', {});
  }

  /**
   * Delete a conversation
   * @param {string} conversationId - Conversation ID
   * @returns {Promise<Object>} Delete response
   */
  async deleteConversation(conversationId) {
    // Assuming there's a delete endpoint (not in blueprint, but needed)
    return await this.execute('delete_conversation', { conversationId });
  }

  /**
   * Add context to conversation
   * @param {string} conversationId - Conversation ID
   * @param {Object} context - Context data
   * @param {string} context.title - Context title
   * @param {string} context.content - Context content
   * @param {string} context.context_type - Context type
   * @returns {Promise<Object>} Add context response
   */
  async addContext(conversationId, context) {
    return await this.execute('add_context_to_conversation', {
      conversationId,
      context
    });
  }

  /**
   * Update context item
   * @param {string} conversationId - Conversation ID
   * @param {string} contextId - Context item ID
   * @param {Object} request - Update request
   * @returns {Promise<Object>} Update response
   */
  async updateContext(conversationId, contextId, request) {
    return await this.execute('update_context', {
      conversationId,
      contextId,
      request
    });
  }

  /**
   * Delete context item
   * @param {string} conversationId - Conversation ID
   * @param {string} contextId - Context item ID
   * @returns {Promise<Object>} Delete response
   */
  async deleteContext(conversationId, contextId) {
    return await this.execute('delete_context', {
      conversationId,
      contextId
    });
  }

  /**
   * Find KG documents for a term
   * @param {string} roleName - Role name
   * @param {string} term - Search term
   * @returns {Promise<Array<Object>>} Array of documents
   */
  async findKGDocuments(roleName, term) {
    return await this.execute('find_documents_for_kg_term', {
      roleName,
      term
    });
  }

  /**
   * Get persistent conversation
   * @param {string} conversationId - Conversation ID
   * @returns {Promise<Object>} Persistent conversation
   */
  async getPersistentConversation(conversationId) {
    return await this.execute('get_persistent_conversation', { conversationId });
  }

  /**
   * Create persistent conversation
   * @param {Object} conversation - Conversation data
   * @returns {Promise<Object>} Created persistent conversation
   */
  async createPersistentConversation(conversation) {
    return await this.execute('create_persistent_conversation', conversation);
  }

  /**
   * Update persistent conversation
   * @param {string} conversationId - Conversation ID
   * @param {Object} conversation - Updated conversation data
   * @returns {Promise<Object>} Update response
   */
  async updatePersistentConversation(conversationId, conversation) {
    return await this.execute('update_persistent_conversation', {
      conversationId,
      ...conversation
    });
  }

  /**
   * Summarize content using LLM
   * @param {string} role - Role name
   * @param {string} content - Content to summarize
   * @returns {Promise<Object>} Summarize response
   */
  async summarize(role, content) {
    // Blueprint section 4.1 mentions this but no specific endpoint
    // Assuming a summarize endpoint exists
    return await this.execute('summarize', { role, content });
  }
}
