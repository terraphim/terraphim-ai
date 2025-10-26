import { TerraphimElement } from '../base/terraphim-element.js';

/**
 * TerraphimChatHeader Component
 *
 * Displays the chat header with title, subtitle, and optional control buttons.
 *
 * @fires clear-clicked - Dispatched when clear button is clicked
 * @fires settings-clicked - Dispatched when settings button is clicked
 *
 * @example
 * ```html
 * <terraphim-chat-header
 *   title="Terraphim AI Assistant"
 *   subtitle="Ask me anything about your knowledge base"
 *   show-clear-button
 *   show-settings-button>
 * </terraphim-chat-header>
 * ```
 */
export class TerraphimChatHeader extends TerraphimElement {
  static get properties() {
    return {
      title: { type: String, default: 'Chat' },
      subtitle: { type: String, default: '' },
      showClearButton: { type: Boolean, reflect: true },
      showSettingsButton: { type: Boolean, reflect: true },
      showMinimizeButton: { type: Boolean, reflect: true },
    };
  }

  constructor() {
    super();
    this.attachShadow({ mode: 'open' });
  }

  styles() {
    return `
      :host {
        display: block;
        background: var(--bg-elevated);
        border-bottom: 1px solid var(--border-primary);
      }

      .header-container {
        display: flex;
        justify-content: space-between;
        align-items: center;
        padding: var(--spacing-md);
        gap: var(--spacing-md);
      }

      .header-content {
        flex: 1;
        min-width: 0;
      }

      .header-title {
        font-size: var(--font-size-lg);
        font-weight: var(--font-weight-semibold);
        color: var(--text-primary);
        margin: 0;
        overflow: hidden;
        text-overflow: ellipsis;
        white-space: nowrap;
      }

      .header-subtitle {
        font-size: var(--font-size-sm);
        color: var(--text-tertiary);
        margin: var(--spacing-xs) 0 0 0;
        overflow: hidden;
        text-overflow: ellipsis;
        white-space: nowrap;
      }

      .header-controls {
        display: flex;
        gap: var(--spacing-xs);
        flex-shrink: 0;
      }

      .header-button {
        padding: var(--spacing-xs) var(--spacing-sm);
        background: var(--bg-secondary);
        color: var(--text-primary);
        border: 1px solid var(--border-primary);
        border-radius: var(--border-radius-md);
        cursor: pointer;
        font-family: var(--font-family-sans);
        font-size: var(--font-size-sm);
        transition: var(--transition-base);
        display: flex;
        align-items: center;
        gap: var(--spacing-xs);
      }

      .header-button:hover {
        background: var(--bg-hover);
        border-color: var(--border-hover);
      }

      .header-button:active {
        transform: scale(0.98);
      }

      .header-button svg {
        width: 16px;
        height: 16px;
        fill: currentColor;
      }

      .header-button.danger:hover {
        background: var(--color-danger);
        border-color: var(--color-danger);
        color: var(--color-danger-contrast);
      }

      /* Responsive adjustments */
      @media (max-width: 640px) {
        .header-button span {
          display: none;
        }

        .header-button {
          padding: var(--spacing-xs);
        }
      }
    `;
  }

  render() {
    const html = `
      <style>${this.styles()}</style>
      <div class="header-container">
        <div class="header-content">
          <h2 class="header-title">${this._escapeHTML(this.title)}</h2>
          ${this.subtitle ? `<p class="header-subtitle">${this._escapeHTML(this.subtitle)}</p>` : ''}
        </div>
        <div class="header-controls">
          <slot name="controls-before"></slot>

          ${this.showSettingsButton ? `
            <button class="header-button" id="settingsBtn" title="Settings">
              <svg viewBox="0 0 24 24" xmlns="http://www.w3.org/2000/svg">
                <path d="M12 15.5A3.5 3.5 0 0 1 8.5 12 3.5 3.5 0 0 1 12 8.5a3.5 3.5 0 0 1 3.5 3.5 3.5 3.5 0 0 1-3.5 3.5m7.43-2.53c.04-.32.07-.64.07-.97 0-.33-.03-.66-.07-1l2.11-1.63c.19-.15.24-.42.12-.64l-2-3.46c-.12-.22-.39-.31-.61-.22l-2.49 1c-.52-.39-1.06-.73-1.69-.98l-.37-2.65A.506.506 0 0 0 14 2h-4c-.25 0-.46.18-.5.42l-.37 2.65c-.63.25-1.17.59-1.69.98l-2.49-1c-.22-.09-.49 0-.61.22l-2 3.46c-.13.22-.07.49.12.64L4.57 11c-.04.34-.07.67-.07 1 0 .33.03.65.07.97l-2.11 1.66c-.19.15-.25.42-.12.64l2 3.46c.12.22.39.3.61.22l2.49-1.01c.52.4 1.06.74 1.69.99l.37 2.65c.04.24.25.42.5.42h4c.25 0 .46-.18.5-.42l.37-2.65c.63-.26 1.17-.59 1.69-.99l2.49 1.01c.22.08.49 0 .61-.22l2-3.46c.12-.22.07-.49-.12-.64l-2.11-1.66z"/>
              </svg>
              <span>Settings</span>
            </button>
          ` : ''}

          ${this.showClearButton ? `
            <button class="header-button danger" id="clearBtn" title="Clear Chat">
              <svg viewBox="0 0 24 24" xmlns="http://www.w3.org/2000/svg">
                <path d="M19,4H15.5L14.5,3H9.5L8.5,4H5V6H19M6,19A2,2 0 0,0 8,21H16A2,2 0 0,0 18,19V7H6V19Z"/>
              </svg>
              <span>Clear</span>
            </button>
          ` : ''}

          ${this.showMinimizeButton ? `
            <button class="header-button" id="minimizeBtn" title="Minimize">
              <svg viewBox="0 0 24 24" xmlns="http://www.w3.org/2000/svg">
                <path d="M19,13H5V11H19V13Z"/>
              </svg>
            </button>
          ` : ''}

          <slot name="controls-after"></slot>
        </div>
      </div>
    `;

    this.setHTML(this.shadowRoot, html);

    // Attach event listeners
    const clearBtn = this.$('#clearBtn');
    const settingsBtn = this.$('#settingsBtn');
    const minimizeBtn = this.$('#minimizeBtn');

    if (clearBtn) {
      clearBtn.addEventListener('click', this._handleClear.bind(this));
    }
    if (settingsBtn) {
      settingsBtn.addEventListener('click', this._handleSettings.bind(this));
    }
    if (minimizeBtn) {
      minimizeBtn.addEventListener('click', this._handleMinimize.bind(this));
    }
  }

  _handleClear(e) {
    e.preventDefault();
    this.dispatchEvent(new CustomEvent('clear-clicked', {
      bubbles: true,
      composed: true
    }));
  }

  _handleSettings(e) {
    e.preventDefault();
    this.dispatchEvent(new CustomEvent('settings-clicked', {
      bubbles: true,
      composed: true
    }));
  }

  _handleMinimize(e) {
    e.preventDefault();
    this.dispatchEvent(new CustomEvent('minimize-clicked', {
      bubbles: true,
      composed: true
    }));
  }

  _escapeHTML(str) {
    const div = document.createElement('div');
    div.textContent = str;
    return div.innerHTML;
  }
}

customElements.define('terraphim-chat-header', TerraphimChatHeader);
