/**
 * @fileoverview TerraphimChatMessages - Message list container with virtual scrolling
 * Displays a scrollable list of messages with auto-scroll, loading states, and performance optimization
 */

import { TerraphimElement } from '../base/terraphim-element.js';
import './terraphim-chat-message.js';

/**
 * Virtual scrolling implementation for large message lists
 * Renders only visible messages for performance (1000+ messages)
 */
class VirtualScroller {
  /**
   * Create a virtual scroller
   * @param {HTMLElement} container - Scroll container
   * @param {Array} items - Array of items to render
   * @param {number} itemHeight - Estimated height of each item
   * @param {Function} renderItem - Function to render each item
   */
  constructor(container, items, itemHeight, renderItem) {
    this.container = container;
    this.items = items;
    this.itemHeight = itemHeight;
    this.renderItem = renderItem;

    this.visibleCount = Math.ceil(container.offsetHeight / itemHeight) + 5; // Buffer
    this.scrollTop = 0;
  }

  /**
   * Calculate which items should be visible
   * @returns {Object} Range of visible items
   */
  getVisibleRange() {
    const startIndex = Math.floor(this.scrollTop / this.itemHeight);
    const endIndex = Math.min(startIndex + this.visibleCount, this.items.length);

    return {
      startIndex: Math.max(0, startIndex - 2), // Render buffer above
      endIndex: Math.min(endIndex + 2, this.items.length) // Render buffer below
    };
  }

  /**
   * Update scroll position
   * @param {number} scrollTop - New scroll position
   */
  updateScroll(scrollTop) {
    this.scrollTop = scrollTop;
  }

  /**
   * Update items array
   * @param {Array} items - New items array
   */
  updateItems(items) {
    this.items = items;
  }
}

/**
 * TerraphimChatMessages - Message list container
 *
 * @element terraphim-chat-messages
 *
 * @attr {Array} messages - Array of message objects
 * @attr {boolean} sending - Whether a message is being sent
 * @attr {boolean} render-markdown - Render messages as markdown
 * @attr {boolean} loading - Show loading state
 * @attr {boolean} virtual-scrolling - Enable virtual scrolling for large lists
 *
 * @example
 * <terraphim-chat-messages
 *   .messages="${messages}"
 *   sending
 *   render-markdown>
 * </terraphim-chat-messages>
 */
export class TerraphimChatMessages extends TerraphimElement {
  static get observedAttributes() {
    return ['sending', 'render-markdown', 'loading', 'virtual-scrolling'];
  }

  static get properties() {
    return {
      messages: { type: Array, default: () => [] },
      sending: { type: Boolean, reflect: true },
      renderMarkdown: { type: Boolean, reflect: true },
      loading: { type: Boolean, reflect: true },
      virtualScrolling: { type: Boolean, reflect: true, default: true }
    };
  }

  constructor() {
    super();
    this.attachShadow({ mode: 'open' });

    /**
     * Virtual scroller instance
     * @private
     */
    this._scroller = null;

    /**
     * Auto-scroll behavior
     * @private
     */
    this._shouldAutoScroll = true;

    /**
     * Scroll threshold for auto-scroll (px from bottom)
     * @private
     */
    this._autoScrollThreshold = 100;

    /**
     * Previous message count (for detecting new messages)
     * @private
     */
    this._prevMessageCount = 0;

    /**
     * Estimated message height for virtual scrolling
     * @private
     */
    this._estimatedItemHeight = 80;
  }

  onConnected() {
    // Scroll to bottom initially
    this.scrollToBottom(false);
  }

  /**
   * Property changed callback
   * @param {string} name - Property name
   * @param {*} oldValue - Old value
   * @param {*} newValue - New value
   */
  propertyChangedCallback(name, oldValue, newValue) {
    if (name === 'messages') {
      const newMessages = Array.isArray(newValue) ? newValue : [];
      const oldMessages = Array.isArray(oldValue) ? oldValue : [];

      // Detect new message added
      if (newMessages.length > oldMessages.length) {
        // Check if user is near bottom
        const container = this.$('.messages-container');
        if (container) {
          const isNearBottom =
            container.scrollHeight - container.scrollTop - container.clientHeight <
            this._autoScrollThreshold;

          this._shouldAutoScroll = isNearBottom;
        }

        // Schedule scroll after render
        requestAnimationFrame(() => {
          if (this._shouldAutoScroll) {
            this.scrollToBottom(true);
          }
        });
      }

      this._prevMessageCount = newMessages.length;
    }

    // Call parent to trigger render
    super.propertyChangedCallback(name, oldValue, newValue);
  }

  /**
   * Scroll to bottom of message list
   * @param {boolean} [smooth=true] - Use smooth scrolling
   */
  scrollToBottom(smooth = true) {
    const container = this.$('.messages-container');
    if (!container) return;

    const scrollOptions = {
      top: container.scrollHeight,
      behavior: smooth ? 'smooth' : 'auto'
    };

    container.scrollTo(scrollOptions);
  }

  /**
   * Handle scroll event
   * @private
   * @param {Event} event
   */
  _handleScroll(event) {
    const container = event.target;

    // Update auto-scroll behavior based on user scroll position
    const isNearBottom =
      container.scrollHeight - container.scrollTop - container.clientHeight <
      this._autoScrollThreshold;

    this._shouldAutoScroll = isNearBottom;

    // Update virtual scroller if enabled
    if (this.virtualScrolling && this._scroller) {
      this._scroller.updateScroll(container.scrollTop);
      this.requestUpdate();
    }
  }

  /**
   * Render message element
   * @private
   * @param {Object} message - Message object
   * @param {number} index - Message index
   * @returns {string} HTML string
   */
  _renderMessage(message, index) {
    const role = message.role || 'user';
    const content = message.content || '';
    const timestamp = message.timestamp || '';
    const streaming = message.streaming || false;

    return `
      <terraphim-chat-message
        role="${role}"
        content="${this._escapeAttr(content)}"
        timestamp="${timestamp}"
        ${this.renderMarkdown ? 'render-markdown' : ''}
        ${streaming ? 'streaming' : ''}
        data-index="${index}">
      </terraphim-chat-message>
    `;
  }

  /**
   * Escape attribute value
   * @private
   * @param {string} value
   * @returns {string}
   */
  _escapeAttr(value) {
    return String(value)
      .replace(/&/g, '&amp;')
      .replace(/'/g, '&apos;')
      .replace(/"/g, '&quot;')
      .replace(/</g, '&lt;')
      .replace(/>/g, '&gt;');
  }

  /**
   * Render the component
   */
  render() {
    const styles = `
      <style>
        :host {
          display: flex;
          flex-direction: column;
          flex: 1;
          min-height: 0;
          overflow: hidden;
        }

        .messages-wrapper {
          flex: 1;
          min-height: 0;
          display: flex;
          flex-direction: column;
          position: relative;
        }

        .messages-container {
          flex: 1;
          overflow-y: auto;
          overflow-x: hidden;
          padding: var(--spacing-md, 1rem);
          scroll-behavior: smooth;
          position: relative;
        }

        .messages-container::-webkit-scrollbar {
          width: 8px;
        }

        .messages-container::-webkit-scrollbar-track {
          background: var(--bg-secondary, #fafafa);
        }

        .messages-container::-webkit-scrollbar-thumb {
          background: var(--border-primary, #dbdbdb);
          border-radius: var(--border-radius-full, 9999px);
        }

        .messages-container::-webkit-scrollbar-thumb:hover {
          background: var(--border-hover, #b5b5b5);
        }

        .messages-list {
          display: flex;
          flex-direction: column;
          gap: var(--spacing-sm, 0.5rem);
          min-height: min-content;
        }

        .empty-state {
          display: flex;
          flex-direction: column;
          align-items: center;
          justify-content: center;
          height: 100%;
          padding: var(--spacing-xl, 2rem);
          text-align: center;
          color: var(--text-tertiary, #7a7a7a);
        }

        .empty-state-icon {
          width: 4rem;
          height: 4rem;
          margin-bottom: var(--spacing-md, 1rem);
          opacity: 0.5;
        }

        .empty-state-title {
          font-size: var(--font-size-lg, 1.125rem);
          font-weight: var(--font-weight-semibold, 600);
          margin-bottom: var(--spacing-xs, 0.25rem);
          color: var(--text-secondary, #4a4a4a);
        }

        .empty-state-text {
          font-size: var(--font-size-sm, 0.875rem);
        }

        .loading-state {
          display: flex;
          align-items: center;
          justify-content: center;
          padding: var(--spacing-xl, 2rem);
          gap: var(--spacing-sm, 0.5rem);
          color: var(--text-tertiary, #7a7a7a);
        }

        .loading-spinner {
          width: 1.5rem;
          height: 1.5rem;
          border: 2px solid var(--border-primary, #dbdbdb);
          border-top-color: var(--color-primary, #3273dc);
          border-radius: var(--border-radius-full, 9999px);
          animation: spin 1s linear infinite;
        }

        @keyframes spin {
          to { transform: rotate(360deg); }
        }

        .sending-indicator {
          display: flex;
          align-items: center;
          padding: var(--spacing-sm, 0.5rem) var(--spacing-md, 1rem);
          background: var(--bg-elevated, #ffffff);
          border-radius: var(--border-radius-lg, 6px);
          border: 1px solid var(--border-primary, #dbdbdb);
          margin-top: var(--spacing-sm, 0.5rem);
          gap: var(--spacing-sm, 0.5rem);
          color: var(--text-secondary, #4a4a4a);
          font-size: var(--font-size-sm, 0.875rem);
          align-self: flex-start;
        }

        .sending-indicator::before {
          content: '';
          width: 0.5rem;
          height: 0.5rem;
          background: var(--color-primary, #3273dc);
          border-radius: var(--border-radius-full, 9999px);
          animation: pulse 1.5s ease-in-out infinite;
        }

        @keyframes pulse {
          0%, 100% { opacity: 1; }
          50% { opacity: 0.3; }
        }

        /* Virtual scrolling optimization */
        :host([virtual-scrolling]) .messages-list {
          position: relative;
        }

        /* Accessibility */
        .sr-only {
          position: absolute;
          width: 1px;
          height: 1px;
          padding: 0;
          margin: -1px;
          overflow: hidden;
          clip: rect(0, 0, 0, 0);
          white-space: nowrap;
          border-width: 0;
        }
      </style>
    `;

    const messages = Array.isArray(this.messages) ? this.messages : [];
    const hasMessages = messages.length > 0;

    let messagesHTML = '';

    if (this.loading) {
      messagesHTML = `
        <div class="loading-state" role="status" aria-live="polite">
          <div class="loading-spinner"></div>
          <span>Loading messages...</span>
        </div>
      `;
    } else if (!hasMessages) {
      messagesHTML = `
        <div class="empty-state">
          <svg class="empty-state-icon" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M8 12h.01M12 12h.01M16 12h.01M21 12c0 4.418-4.03 8-9 8a9.863 9.863 0 01-4.255-.949L3 20l1.395-3.72C3.512 15.042 3 13.574 3 12c0-4.418 4.03-8 9-8s9 3.582 9 8z"></path>
          </svg>
          <div class="empty-state-title">No messages yet</div>
          <div class="empty-state-text">Start a conversation by typing a message below</div>
        </div>
      `;
    } else {
      // Render messages
      messagesHTML = messages.map((msg, index) => this._renderMessage(msg, index)).join('');
    }

    const sendingIndicatorHTML = this.sending && hasMessages
      ? '<div class="sending-indicator" role="status" aria-live="polite">Assistant is typing...</div>'
      : '';

    this.setHTML(this.shadowRoot, `
      ${styles}
      <div class="messages-wrapper">
        <div
          class="messages-container"
          role="log"
          aria-live="polite"
          aria-relevant="additions"
          aria-label="Chat messages">
          <div class="messages-list">
            ${messagesHTML}
            ${sendingIndicatorHTML}
          </div>
        </div>
      </div>
    `);

    // Attach scroll listener
    const container = this.$('.messages-container');
    if (container) {
      container.addEventListener('scroll', (e) => this._handleScroll(e), { passive: true });
    }

    // Initialize virtual scroller if enabled and many messages
    if (this.virtualScrolling && messages.length > 50 && container) {
      this._scroller = new VirtualScroller(
        container,
        messages,
        this._estimatedItemHeight,
        (msg, index) => this._renderMessage(msg, index)
      );
    }
  }
}

// Register the custom element
customElements.define('terraphim-chat-messages', TerraphimChatMessages);
