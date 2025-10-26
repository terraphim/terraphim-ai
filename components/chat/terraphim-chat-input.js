/**
 * @fileoverview TerraphimChatInput - Message input component with auto-resize and keyboard shortcuts
 * Provides textarea with Enter to send, Shift+Enter for newline, and loading state
 */

import { TerraphimElement } from '../base/terraphim-element.js';

/**
 * TerraphimChatInput - Message input component
 *
 * @element terraphim-chat-input
 *
 * @attr {boolean} disabled - Disable input and send button
 * @attr {string} placeholder - Placeholder text
 * @attr {string} value - Current input value
 *
 * @fires message-submit - When Enter is pressed or send button is clicked
 * @fires input-change - When input value changes
 *
 * @example
 * <terraphim-chat-input
 *   placeholder="Type your message..."
 *   disabled>
 * </terraphim-chat-input>
 */
export class TerraphimChatInput extends TerraphimElement {
  static get observedAttributes() {
    return ['disabled', 'placeholder', 'value'];
  }

  static get properties() {
    return {
      disabled: { type: Boolean, reflect: true },
      placeholder: { type: String, default: 'Type your message and press Enter...' },
      value: { type: String, default: '' }
    };
  }

  constructor() {
    super();
    this.attachShadow({ mode: 'open' });

    /**
     * Reference to textarea element
     * @private
     */
    this._textarea = null;

    /**
     * Min height for textarea
     * @private
     */
    this._minHeight = 48; // 3rem in px

    /**
     * Max height for textarea
     * @private
     */
    this._maxHeight = 128; // 8rem in px
  }

  onConnected() {
    this._textarea = this.$('#messageInput');
    this._adjustHeight();
  }

  /**
   * Handle keyboard input
   * @private
   * @param {KeyboardEvent} event
   */
  _handleKeydown(event) {
    if ((event.key === 'Enter' || event.key === 'Return') && !event.shiftKey) {
      event.preventDefault();
      this._handleSubmit();
    }
  }

  /**
   * Handle input changes
   * @private
   * @param {InputEvent} event
   */
  _handleInput(event) {
    const newValue = event.target.value;
    this.value = newValue;
    this._adjustHeight();

    this.emit('input-change', { value: newValue });
  }

  /**
   * Handle submit (Enter key or send button)
   * @private
   */
  _handleSubmit() {
    const text = this.value.trim();

    if (text && !this.disabled) {
      this.emit('message-submit', { text });
      this.clear();
      this.focus();
    }
  }

  /**
   * Auto-adjust textarea height based on content
   * @private
   */
  _adjustHeight() {
    if (!this._textarea) return;

    // Reset height to measure scrollHeight correctly
    this._textarea.style.height = 'auto';

    // Calculate new height
    const newHeight = Math.min(
      Math.max(this._textarea.scrollHeight, this._minHeight),
      this._maxHeight
    );

    this._textarea.style.height = `${newHeight}px`;
  }

  /**
   * Clear the input
   */
  clear() {
    this.value = '';
    if (this._textarea) {
      this._textarea.value = '';
      this._adjustHeight();
    }
  }

  /**
   * Focus the input
   */
  focus() {
    if (this._textarea) {
      this._textarea.focus();
    }
  }

  /**
   * Get current input value
   * @returns {string}
   */
  getValue() {
    return this.value;
  }

  /**
   * Set input value programmatically
   * @param {string} value
   */
  setValue(value) {
    this.value = value;
    if (this._textarea) {
      this._textarea.value = value;
      this._adjustHeight();
    }
  }

  /**
   * Render the component
   */
  render() {
    const styles = `
      <style>
        :host {
          display: block;
          padding: var(--spacing-md, 1rem);
          background: var(--bg-elevated, #ffffff);
          border-top: 1px solid var(--border-primary, #dbdbdb);
        }

        .input-container {
          display: flex;
          gap: var(--spacing-sm, 0.5rem);
          align-items: flex-end;
        }

        .input-wrapper {
          flex: 1;
          position: relative;
        }

        textarea {
          width: 100%;
          min-height: var(--input-min-height, 3rem);
          max-height: var(--input-max-height, 8rem);
          padding: var(--input-padding, var(--spacing-md, 1rem));
          border: 1px solid var(--input-border, var(--border-primary, #dbdbdb));
          border-radius: var(--input-border-radius, var(--border-radius-lg, 6px));
          background: var(--input-bg, var(--bg-page, #ffffff));
          color: var(--text-primary, #363636);
          font-family: var(--font-family-sans);
          font-size: var(--font-size-base, 1rem);
          line-height: var(--line-height-normal, 1.5);
          resize: none;
          transition: var(--transition-base);
          box-sizing: border-box;
          overflow-y: auto;
        }

        textarea::placeholder {
          color: var(--text-tertiary, #7a7a7a);
          opacity: 0.8;
        }

        textarea:focus {
          outline: none;
          border-color: var(--input-focus-border, var(--border-focus, #3273dc));
          box-shadow: var(--shadow-focus, 0 0 0 3px rgba(50, 115, 220, 0.25));
        }

        textarea:disabled {
          opacity: 0.6;
          cursor: not-allowed;
          background: var(--bg-secondary, #fafafa);
        }

        .send-button {
          padding: var(--spacing-md, 1rem);
          background: var(--color-primary, #3273dc);
          color: var(--color-primary-contrast, #ffffff);
          border: none;
          border-radius: var(--border-radius-lg, 6px);
          cursor: pointer;
          transition: var(--transition-base);
          display: flex;
          align-items: center;
          justify-content: center;
          min-width: 3rem;
          height: 3rem;
          flex-shrink: 0;
        }

        .send-button:hover:not(:disabled) {
          background: var(--color-primary-dark, #2366d1);
          transform: translateY(-1px);
          box-shadow: var(--shadow-md, 0 2px 4px rgba(0, 0, 0, 0.1));
        }

        .send-button:active:not(:disabled) {
          transform: translateY(0);
        }

        .send-button:disabled {
          opacity: 0.5;
          cursor: not-allowed;
          background: var(--color-primary, #3273dc);
        }

        .send-button:focus-visible {
          outline: 2px solid var(--border-focus, #3273dc);
          outline-offset: 2px;
        }

        .icon {
          width: 1.25rem;
          height: 1.25rem;
          fill: currentColor;
        }

        .loading {
          animation: spin 1s linear infinite;
        }

        @keyframes spin {
          from {
            transform: rotate(0deg);
          }
          to {
            transform: rotate(360deg);
          }
        }

        .hint {
          position: absolute;
          bottom: -1.25rem;
          left: 0;
          font-size: var(--font-size-xs, 0.75rem);
          color: var(--text-tertiary, #7a7a7a);
          opacity: 0;
          transition: opacity var(--transition-fast);
        }

        .input-wrapper:focus-within .hint {
          opacity: 1;
        }

        /* Character counter (optional feature) */
        .char-counter {
          position: absolute;
          bottom: var(--spacing-xs, 0.25rem);
          right: var(--spacing-xs, 0.25rem);
          font-size: var(--font-size-xs, 0.75rem);
          color: var(--text-tertiary, #7a7a7a);
          background: var(--bg-elevated, #ffffff);
          padding: 0.125rem 0.25rem;
          border-radius: var(--border-radius-sm, 2px);
          opacity: 0;
          transition: opacity var(--transition-fast);
        }

        .input-wrapper:focus-within .char-counter {
          opacity: 1;
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

    const sendIcon = this.disabled
      ? `<svg class="icon loading" viewBox="0 0 24 24" fill="none" stroke="currentColor">
           <circle cx="12" cy="12" r="10" stroke-width="3" stroke-dasharray="31.415 31.415" />
         </svg>`
      : `<svg class="icon" fill="currentColor" viewBox="0 0 20 20">
           <path d="M10.894 2.553a1 1 0 00-1.788 0l-7 14a1 1 0 001.169 1.409l5-1.429A1 1 0 009 15.571V11a1 1 0 112 0v4.571a1 1 0 00.725.962l5 1.428a1 1 0 001.17-1.408l-7-14z"></path>
         </svg>`;

    this.setHTML(this.shadowRoot, `
      ${styles}
      <div class="input-container">
        <div class="input-wrapper">
          <textarea
            id="messageInput"
            placeholder="${this.placeholder}"
            rows="1"
            autocomplete="off"
            autocorrect="off"
            autocapitalize="off"
            spellcheck="true"
            aria-label="Chat message input"
            aria-describedby="chat-hint"
            ${this.disabled ? 'disabled' : ''}
          >${this.value}</textarea>
          <div id="chat-hint" class="hint sr-only">
            Press Enter to send, Shift+Enter for new line
          </div>
        </div>

        <button
          class="send-button"
          id="sendButton"
          type="button"
          aria-label="Send message"
          ${this.disabled ? 'disabled' : ''}
        >
          ${sendIcon}
        </button>
      </div>
    `);

    // Attach event listeners
    this._textarea = this.$('#messageInput');
    const sendButton = this.$('#sendButton');

    if (this._textarea) {
      this._textarea.addEventListener('keydown', (e) => this._handleKeydown(e));
      this._textarea.addEventListener('input', (e) => this._handleInput(e));
      this._adjustHeight();
    }

    if (sendButton) {
      sendButton.addEventListener('click', () => this._handleSubmit());
    }
  }
}

// Register the custom element
customElements.define('terraphim-chat-input', TerraphimChatInput);
