import { TerraphimElement } from '../base/terraphim-element.js';

/**
 * TerraphimChatContextPanel Component
 *
 * Displays context items (documents, files, URLs) that provide additional context to the chat.
 *
 * @fires context-item-add - When add context button is clicked
 * @fires context-item-remove - When a context item is removed {id, item}
 * @fires context-item-click - When a context item is clicked {id, item}
 *
 * @example
 * ```html
 * <terraphim-chat-context-panel
 *   title="Context"
 *   show-add-button>
 * </terraphim-chat-context-panel>
 * ```
 */
export class TerraphimChatContextPanel extends TerraphimElement {
  static get properties() {
    return {
      title: { type: String, default: 'Context' },
      contextItems: { type: Array, default: () => [] },
      showAddButton: { type: Boolean, reflect: true, default: true },
      collapsed: { type: Boolean, reflect: true },
      maxItems: { type: Number, default: 10 },
    };
  }

  constructor() {
    super();
    this.attachShadow({ mode: 'open' });
  }

  styles() {
    return `
      :host {
        display: flex;
        flex-direction: column;
        background: var(--bg-secondary);
        border-left: 1px solid var(--border-primary);
        height: 100%;
      }

      .panel-header {
        display: flex;
        justify-content: space-between;
        align-items: center;
        padding: var(--spacing-md);
        border-bottom: 1px solid var(--border-primary);
        background: var(--bg-elevated);
      }

      .panel-title {
        font-size: var(--font-size-base);
        font-weight: var(--font-weight-semibold);
        color: var(--text-primary);
        margin: 0;
      }

      .header-controls {
        display: flex;
        gap: var(--spacing-xs);
      }

      .icon-button {
        padding: var(--spacing-xs);
        background: transparent;
        border: 1px solid var(--border-primary);
        border-radius: var(--border-radius-sm);
        cursor: pointer;
        color: var(--text-secondary);
        transition: var(--transition-base);
        display: flex;
        align-items: center;
        justify-content: center;
      }

      .icon-button:hover {
        background: var(--bg-hover);
        border-color: var(--border-hover);
        color: var(--text-primary);
      }

      .icon-button svg {
        width: 16px;
        height: 16px;
        fill: currentColor;
      }

      .panel-content {
        flex: 1;
        overflow-y: auto;
        padding: var(--spacing-sm);
      }

      .context-list {
        display: flex;
        flex-direction: column;
        gap: var(--spacing-xs);
      }

      .context-item {
        display: flex;
        align-items: flex-start;
        gap: var(--spacing-sm);
        padding: var(--spacing-sm);
        background: var(--bg-elevated);
        border: 1px solid var(--border-primary);
        border-radius: var(--border-radius-md);
        cursor: pointer;
        transition: var(--transition-base);
      }

      .context-item:hover {
        background: var(--bg-hover);
        border-color: var(--border-hover);
      }

      .context-item-icon {
        flex-shrink: 0;
        width: 32px;
        height: 32px;
        display: flex;
        align-items: center;
        justify-content: center;
        background: var(--color-primary);
        color: var(--color-primary-contrast);
        border-radius: var(--border-radius-sm);
        font-size: var(--font-size-sm);
        font-weight: var(--font-weight-medium);
      }

      .context-item-icon svg {
        width: 16px;
        height: 16px;
        fill: currentColor;
      }

      .context-item-content {
        flex: 1;
        min-width: 0;
      }

      .context-item-title {
        font-size: var(--font-size-sm);
        font-weight: var(--font-weight-medium);
        color: var(--text-primary);
        margin: 0 0 var(--spacing-xs) 0;
        overflow: hidden;
        text-overflow: ellipsis;
        white-space: nowrap;
      }

      .context-item-subtitle {
        font-size: var(--font-size-xs);
        color: var(--text-tertiary);
        margin: 0;
        overflow: hidden;
        text-overflow: ellipsis;
        white-space: nowrap;
      }

      .context-item-remove {
        flex-shrink: 0;
        padding: var(--spacing-xs);
        background: transparent;
        border: none;
        color: var(--text-tertiary);
        cursor: pointer;
        transition: var(--transition-base);
        border-radius: var(--border-radius-sm);
      }

      .context-item-remove:hover {
        background: var(--color-danger);
        color: var(--color-danger-contrast);
      }

      .context-item-remove svg {
        width: 14px;
        height: 14px;
        fill: currentColor;
      }

      .empty-state {
        display: flex;
        flex-direction: column;
        align-items: center;
        justify-content: center;
        padding: var(--spacing-xl);
        text-align: center;
        color: var(--text-tertiary);
      }

      .empty-state svg {
        width: 48px;
        height: 48px;
        margin-bottom: var(--spacing-md);
        opacity: 0.5;
      }

      .empty-state-text {
        font-size: var(--font-size-sm);
        margin: 0;
      }

      :host([collapsed]) .panel-content {
        display: none;
      }
    `;
  }

  render() {
    const html = `
      <style>${this.styles()}</style>
      <div class="panel-header">
        <h3 class="panel-title">${this._escapeHTML(this.title)}</h3>
        <div class="header-controls">
          ${this.showAddButton ? `
            <button class="icon-button" id="addBtn" title="Add context">
              <svg viewBox="0 0 24 24" xmlns="http://www.w3.org/2000/svg">
                <path d="M19,13H13V19H11V13H5V11H11V5H13V11H19V13Z"/>
              </svg>
            </button>
          ` : ''}
          <button class="icon-button" id="toggleBtn" title="${this.collapsed ? 'Expand' : 'Collapse'}">
            <svg viewBox="0 0 24 24" xmlns="http://www.w3.org/2000/svg">
              ${this.collapsed
                ? '<path d="M7.41,15.41L12,10.83L16.59,15.41L18,14L12,8L6,14L7.41,15.41Z"/>'
                : '<path d="M7.41,8.58L12,13.17L16.59,8.58L18,10L12,16L6,10L7.41,8.58Z"/>'
              }
            </svg>
          </button>
        </div>
      </div>
      <div class="panel-content">
        ${this.contextItems.length > 0 ? `
          <div class="context-list">
            ${this.contextItems.map((item, index) => this._renderContextItem(item, index)).join('')}
          </div>
        ` : `
          <div class="empty-state">
            <svg viewBox="0 0 24 24" xmlns="http://www.w3.org/2000/svg">
              <path d="M13,9H18.5L13,3.5V9M6,2H14L20,8V20A2,2 0 0,1 18,22H6C4.89,22 4,21.1 4,20V4C4,2.89 4.89,2 6,2M15,18V16H6V18H15M18,14V12H6V14H18Z"/>
            </svg>
            <p class="empty-state-text">No context items added yet.<br>Click the + button to add context.</p>
          </div>
        `}
      </div>
    `;

    this.setHTML(this.shadowRoot, html);

    // Attach event listeners
    const addBtn = this.$('#addBtn');
    const toggleBtn = this.$('#toggleBtn');

    if (addBtn) {
      addBtn.addEventListener('click', this._handleAddContext.bind(this));
    }
    if (toggleBtn) {
      toggleBtn.addEventListener('click', this._handleToggle.bind(this));
    }

    // Attach listeners to context items
    this.contextItems.forEach((item, index) => {
      const itemEl = this.$(`#context-item-${index}`);
      const removeBtn = this.$(`#remove-btn-${index}`);

      if (itemEl) {
        itemEl.addEventListener('click', (e) => {
          if (e.target !== removeBtn && !removeBtn?.contains(e.target)) {
            this._handleItemClick(item, index);
          }
        });
      }

      if (removeBtn) {
        removeBtn.addEventListener('click', (e) => {
          e.stopPropagation();
          this._handleItemRemove(item, index);
        });
      }
    });
  }

  _renderContextItem(item, index) {
    const icon = this._getIconForType(item.type);
    const truncatedPath = item.path ? this._truncatePath(item.path) : item.type;

    return `
      <div class="context-item" id="context-item-${index}">
        <div class="context-item-icon">
          ${icon}
        </div>
        <div class="context-item-content">
          <p class="context-item-title">${this._escapeHTML(item.title || item.name || 'Untitled')}</p>
          <p class="context-item-subtitle">${this._escapeHTML(truncatedPath)}</p>
        </div>
        <button class="context-item-remove" id="remove-btn-${index}" title="Remove">
          <svg viewBox="0 0 24 24" xmlns="http://www.w3.org/2000/svg">
            <path d="M19,6.41L17.59,5L12,10.59L6.41,5L5,6.41L10.59,12L5,17.59L6.41,19L12,13.41L17.59,19L19,17.59L13.41,12L19,6.41Z"/>
          </svg>
        </button>
      </div>
    `;
  }

  _getIconForType(type) {
    const icons = {
      file: '<svg viewBox="0 0 24 24" xmlns="http://www.w3.org/2000/svg"><path d="M13,9H18.5L13,3.5V9M6,2H14L20,8V20A2,2 0 0,1 18,22H6C4.89,22 4,21.1 4,20V4C4,2.89 4.89,2 6,2Z"/></svg>',
      url: '<svg viewBox="0 0 24 24" xmlns="http://www.w3.org/2000/svg"><path d="M16.36,14C16.44,13.34 16.5,12.68 16.5,12C16.5,11.32 16.44,10.66 16.36,10H19.74C19.9,10.64 20,11.31 20,12C20,12.69 19.9,13.36 19.74,14M14.59,19.56C15.19,18.45 15.65,17.25 15.97,16H18.92C17.96,17.65 16.43,18.93 14.59,19.56M14.34,14H9.66C9.56,13.34 9.5,12.68 9.5,12C9.5,11.32 9.56,10.65 9.66,10H14.34C14.43,10.65 14.5,11.32 14.5,12C14.5,12.68 14.43,13.34 14.34,14M12,19.96C11.17,18.76 10.5,17.43 10.09,16H13.91C13.5,17.43 12.83,18.76 12,19.96M8,8H5.08C6.03,6.34 7.57,5.06 9.4,4.44C8.8,5.55 8.35,6.75 8,8M5.08,16H8C8.35,17.25 8.8,18.45 9.4,19.56C7.57,18.93 6.03,17.65 5.08,16M4.26,14C4.1,13.36 4,12.69 4,12C4,11.31 4.1,10.64 4.26,10H7.64C7.56,10.66 7.5,11.32 7.5,12C7.5,12.68 7.56,13.34 7.64,14M12,4.03C12.83,5.23 13.5,6.57 13.91,8H10.09C10.5,6.57 11.17,5.23 12,4.03M18.92,8H15.97C15.65,6.75 15.19,5.55 14.59,4.44C16.43,5.07 17.96,6.34 18.92,8M12,2C6.47,2 2,6.5 2,12A10,10 0 0,0 12,22A10,10 0 0,0 22,12A10,10 0 0,0 12,2Z"/></svg>',
      folder: '<svg viewBox="0 0 24 24" xmlns="http://www.w3.org/2000/svg"><path d="M10,4H4C2.89,4 2,4.89 2,6V18A2,2 0 0,0 4,20H20A2,2 0 0,0 22,18V8C22,6.89 21.1,6 20,6H12L10,4Z"/></svg>',
      document: '<svg viewBox="0 0 24 24" xmlns="http://www.w3.org/2000/svg"><path d="M6,2A2,2 0 0,0 4,4V20A2,2 0 0,0 6,22H18A2,2 0 0,0 20,20V8L14,2H6M6,4H13V9H18V20H6V4M8,12V14H16V12H8M8,16V18H13V16H8Z"/></svg>',
    };
    return icons[type] || icons.file;
  }

  _truncatePath(path, maxLength = 30) {
    if (path.length <= maxLength) return path;
    const start = path.substring(0, 15);
    const end = path.substring(path.length - 12);
    return `${start}...${end}`;
  }

  _handleAddContext() {
    this.dispatchEvent(new CustomEvent('context-item-add', {
      bubbles: true,
      composed: true
    }));
  }

  _handleToggle() {
    this.collapsed = !this.collapsed;
  }

  _handleItemClick(item, index) {
    this.dispatchEvent(new CustomEvent('context-item-click', {
      detail: { item, index, id: item.id },
      bubbles: true,
      composed: true
    }));
  }

  _handleItemRemove(item, index) {
    this.dispatchEvent(new CustomEvent('context-item-remove', {
      detail: { item, index, id: item.id },
      bubbles: true,
      composed: true
    }));
  }

  // Public API
  addContextItem(item) {
    if (this.contextItems.length >= this.maxItems) {
      console.warn(`Maximum context items (${this.maxItems}) reached`);
      return false;
    }
    this.contextItems = [...this.contextItems, { ...item, id: item.id || Date.now() }];
    return true;
  }

  removeContextItem(id) {
    this.contextItems = this.contextItems.filter(item => item.id !== id);
  }

  clearContextItems() {
    this.contextItems = [];
  }

  _escapeHTML(str) {
    const div = document.createElement('div');
    div.textContent = str;
    return div.innerHTML;
  }
}

customElements.define('terraphim-chat-context-panel', TerraphimChatContextPanel);
