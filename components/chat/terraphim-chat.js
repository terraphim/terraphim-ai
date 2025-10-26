import { TerraphimElement } from '../base/terraphim-element.js';
import { ChatAPIClient } from './chat-api-client.js';
import './terraphim-chat-header.js';
import './terraphim-chat-messages.js';
import './terraphim-chat-input.js';

/**
 * TerraphimChat Component
 *
 * Main chat interface that orchestrates all chat sub-components.
 * Manages message state, API communication, and user interactions.
 *
 * @fires message-sent - Dispatched when a message is sent {text, role}
 * @fires message-received - Dispatched when a response is received {text, role}
 * @fires error - Dispatched when an error occurs {error, message}
 *
 * @example
 * ```html
 * <terraphim-chat
 *   api-endpoint="http://localhost:8000"
 *   use-tauri
 *   header-title="Terraphim AI"
 *   header-subtitle="Your Knowledge Assistant"
 *   render-markdown
 *   virtual-scrolling>
 * </terraphim-chat>
 * ```
 */
export class TerraphimChat extends TerraphimElement {
  static get properties() {
    return {
      // API Configuration
      apiEndpoint: { type: String, default: 'http://localhost:8000' },
      useTauri: { type: Boolean, reflect: true },

      // Header Configuration
      headerTitle: { type: String, default: 'Chat' },
      headerSubtitle: { type: String, default: '' },
      showHeaderControls: { type: Boolean, reflect: true, default: true },
      showClearButton: { type: Boolean, reflect: true, default: true },
      showSettingsButton: { type: Boolean, reflect: true },

      // Display Options
      renderMarkdown: { type: Boolean, reflect: true, default: true },
      virtualScrolling: { type: Boolean, reflect: true, default: true },

      // Input Configuration
      inputPlaceholder: { type: String, default: 'Type your message...' },
      inputDisabled: { type: Boolean, reflect: true },

      // Session Management
      sessionId: { type: String },

      // State
      messages: { type: Array, default: () => [] },
      sending: { type: Boolean, reflect: true },
      loading: { type: Boolean, reflect: true },
    };
  }

  constructor() {
    super();
    this.attachShadow({ mode: 'open' });
    this._apiClient = null;
    this._messagesContainer = null;
    this._chatInput = null;
    this._componentsSetup = false;
  }

  connectedCallback() {
    super.connectedCallback();

    // Initialize API client
    this._apiClient = new ChatAPIClient(this.useTauri, this.apiEndpoint);

    // Listen to API events
    this._apiClient.addEventListener('api-success', this._handleAPISuccess.bind(this));
    this._apiClient.addEventListener('api-error', this._handleAPIError.bind(this));

    // Setup component references after render completes
    // Use setTimeout to ensure shadow DOM is populated
    setTimeout(() => {
      this._setupComponentRefs();
    }, 0);
  }

  disconnectedCallback() {
    super.disconnectedCallback();
    if (this._apiClient) {
      this._apiClient.removeEventListener('api-success', this._handleAPISuccess.bind(this));
      this._apiClient.removeEventListener('api-error', this._handleAPIError.bind(this));
    }
  }

  styles() {
    return `
      :host {
        display: flex;
        flex-direction: column;
        height: 100%;
        background: var(--bg-page);
        position: relative;
      }

      .chat-container {
        display: flex;
        flex-direction: column;
        height: 100%;
        min-height: 0;
      }

      terraphim-chat-header {
        flex-shrink: 0;
      }

      terraphim-chat-messages {
        flex: 1;
        min-height: 0;
      }

      terraphim-chat-input {
        flex-shrink: 0;
      }

      /* Loading overlay */
      .loading-overlay {
        position: absolute;
        top: 0;
        left: 0;
        right: 0;
        bottom: 0;
        background: rgba(0, 0, 0, 0.2);
        display: flex;
        align-items: center;
        justify-content: center;
        z-index: 100;
      }

      .loading-spinner {
        width: 48px;
        height: 48px;
        border: 4px solid var(--border-primary);
        border-top-color: var(--color-primary);
        border-radius: 50%;
        animation: spin 0.8s linear infinite;
      }

      @keyframes spin {
        to { transform: rotate(360deg); }
      }

      .hidden {
        display: none;
      }
    `;
  }

  render() {
    const html = `
      <style>${this.styles()}</style>
      <div class="chat-container">
        ${this.showHeaderControls ? `
          <terraphim-chat-header
            title="${this._escapeHTML(this.headerTitle)}"
            subtitle="${this._escapeHTML(this.headerSubtitle)}"
            ${this.showClearButton ? 'show-clear-button' : ''}
            ${this.showSettingsButton ? 'show-settings-button' : ''}>
          </terraphim-chat-header>
        ` : ''}

        <terraphim-chat-messages
          id="chatMessages"
          ${this.renderMarkdown ? 'render-markdown' : ''}
          ${this.virtualScrolling ? 'virtual-scrolling' : ''}
          ${this.sending ? 'sending' : ''}
          ${this.loading ? 'loading' : ''}>
        </terraphim-chat-messages>

        <terraphim-chat-input
          id="chatInput"
          placeholder="${this._escapeHTML(this.inputPlaceholder)}"
          ${this.inputDisabled || this.sending ? 'disabled' : ''}>
        </terraphim-chat-input>

        <div class="loading-overlay ${this.loading ? '' : 'hidden'}">
          <div class="loading-spinner"></div>
        </div>
      </div>
    `;

    this.setHTML(this.shadowRoot, html);
  }

  _setupComponentRefs() {
    const shadowRoot = this.shadowRoot;
    if (!shadowRoot) {
      // Shadow root not ready yet, try again
      setTimeout(() => this._setupComponentRefs(), 50);
      return;
    }

    this._messagesContainer = shadowRoot.querySelector('#chatMessages');
    this._chatInput = shadowRoot.querySelector('#chatInput');
    const header = shadowRoot.querySelector('terraphim-chat-header');

    if (!this._messagesContainer || !this._chatInput) {
      // Elements not rendered yet, try again
      setTimeout(() => this._setupComponentRefs(), 50);
      return;
    }

    // Mark as set up
    this._componentsSetup = true;

    // Update messages display now that container is ready
    if (this._messagesContainer) {
      this._updateMessagesDisplay();
    }

    if (this._chatInput) {
      this._chatInput.addEventListener('message-submit', this._handleMessageSubmit.bind(this));
    }

    if (header) {
      header.addEventListener('clear-clicked', this._handleClearMessages.bind(this));
      header.addEventListener('settings-clicked', this._handleSettings.bind(this));
    }
  }

  _updateMessagesDisplay() {
    if (!this._componentsSetup) {
      // Components not ready yet, will update when setup completes
      return;
    }
    if (this._messagesContainer && this.messages) {
      this._messagesContainer.messages = [...this.messages];
    }
  }

  async _handleMessageSubmit(e) {
    const text = e.detail.text;

    // Add user message
    this.addMessage({
      role: 'user',
      content: text,
      timestamp: new Date().toISOString()
    });

    // Dispatch sent event
    this.dispatchEvent(new CustomEvent('message-sent', {
      detail: { text, role: 'user' },
      bubbles: true,
      composed: true
    }));

    // Set sending state
    this.sending = true;

    try {
      // Send to API
      const response = await this._apiClient.sendMessage(text, this.sessionId);

      // Add assistant response
      this.addMessage({
        role: 'assistant',
        content: response.content || response.message || 'No response',
        timestamp: new Date().toISOString()
      });

      // Dispatch received event
      this.dispatchEvent(new CustomEvent('message-received', {
        detail: { text: response.content || response.message, role: 'assistant' },
        bubbles: true,
        composed: true
      }));

    } catch (error) {
      console.error('Error sending message:', error);

      // Add error message
      this.addMessage({
        role: 'system',
        content: `Error: ${error.message}`,
        timestamp: new Date().toISOString()
      });

      // Dispatch error event
      this.dispatchEvent(new CustomEvent('error', {
        detail: { error, message: error.message },
        bubbles: true,
        composed: true
      }));

    } finally {
      this.sending = false;
    }
  }

  _handleClearMessages() {
    this.clearMessages();
  }

  _handleSettings() {
    this.dispatchEvent(new CustomEvent('settings-clicked', {
      bubbles: true,
      composed: true
    }));
  }

  _handleAPISuccess(e) {
    console.log('API Success:', e.detail);
  }

  _handleAPIError(e) {
    console.error('API Error:', e.detail);
    this.dispatchEvent(new CustomEvent('error', {
      detail: e.detail,
      bubbles: true,
      composed: true
    }));
  }

  // Public API Methods

  /**
   * Add a message to the chat
   * @param {Object} message - Message object {role, content, timestamp, ...}
   */
  addMessage(message) {
    this.messages = [...this.messages, message];
    this._updateMessagesDisplay();
  }

  /**
   * Clear all messages
   */
  clearMessages() {
    this.messages = [];
    this._updateMessagesDisplay();
  }

  /**
   * Set the session ID for API calls
   * @param {string} sessionId - Session identifier
   */
  setSession(sessionId) {
    this.sessionId = sessionId;
  }

  /**
   * Get current messages
   * @returns {Array} Current message list
   */
  getMessages() {
    return [...this.messages];
  }

  /**
   * Load messages (useful for restoring conversation)
   * @param {Array} messages - Array of message objects
   */
  loadMessages(messages) {
    this.messages = [...messages];
    this._updateMessagesDisplay();
  }

  /**
   * Send a message programmatically
   * @param {string} text - Message text
   */
  async sendMessage(text) {
    if (this._chatInput) {
      this._chatInput.value = text;
      await this._handleMessageSubmit({ detail: { text } });
    }
  }

  _escapeHTML(str) {
    const div = document.createElement('div');
    div.textContent = str;
    return div.innerHTML;
  }
}

customElements.define('terraphim-chat', TerraphimChat);
