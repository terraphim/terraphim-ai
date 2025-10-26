import { TerraphimElement } from '../base/terraphim-element.js';

/**
 * TerraphimConversationList Component
 *
 * Displays a list of recent conversations with search and filtering.
 *
 * @fires conversation-select - When a conversation is selected {conversation, id}
 * @fires conversation-delete - When a conversation is deleted {conversation, id}
 * @fires conversation-archive - When a conversation is archived {conversation, id}
 *
 * @example
 * ```html
 * <terraphim-conversation-list
 *   show-search
 *   current-conversation-id="123">
 * </terraphim-conversation-list>
 * ```
 */
export class TerraphimConversationList extends TerraphimElement {
  static get properties() {
    return {
      conversations: { type: Array, default: () => [] },
      currentConversationId: { type: String },
      showSearch: { type: Boolean, reflect: true, default: true },
      searchQuery: { type: String, default: '' },
      sortBy: { type: String, default: 'recent' }, // 'recent', 'alphabetical'
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
        height: 100%;
        background: var(--bg-secondary);
      }

      .search-container {
        padding: var(--spacing-sm);
        border-bottom: 1px solid var(--border-primary);
      }

      .search-input {
        width: 100%;
        padding: var(--spacing-sm);
        background: var(--bg-elevated);
        border: 1px solid var(--border-primary);
        border-radius: var(--border-radius-md);
        font-family: var(--font-family-sans);
        font-size: var(--font-size-sm);
        color: var(--text-primary);
        transition: var(--transition-base);
      }

      .search-input:focus {
        outline: none;
        border-color: var(--color-primary);
      }

      .search-input::placeholder {
        color: var(--text-tertiary);
      }

      .conversation-list {
        flex: 1;
        overflow-y: auto;
        padding: var(--spacing-xs);
      }

      .conversation-item {
        display: flex;
        align-items: flex-start;
        gap: var(--spacing-sm);
        padding: var(--spacing-sm);
        margin-bottom: var(--spacing-xs);
        background: var(--bg-elevated);
        border: 1px solid var(--border-primary);
        border-radius: var(--border-radius-md);
        cursor: pointer;
        transition: var(--transition-base);
      }

      .conversation-item:hover {
        background: var(--bg-hover);
        border-color: var(--border-hover);
      }

      .conversation-item.active {
        background: var(--color-primary);
        color: var(--color-primary-contrast);
        border-color: var(--color-primary);
      }

      .conversation-avatar {
        flex-shrink: 0;
        width: 40px;
        height: 40px;
        border-radius: 50%;
        background: linear-gradient(135deg, var(--color-primary), var(--color-primary-dark));
        display: flex;
        align-items: center;
        justify-content: center;
        font-weight: var(--font-weight-bold);
        font-size: var(--font-size-sm);
        color: var(--color-primary-contrast);
      }

      .conversation-item.active .conversation-avatar {
        background: rgba(255, 255, 255, 0.2);
      }

      .conversation-content {
        flex: 1;
        min-width: 0;
      }

      .conversation-header {
        display: flex;
        justify-content: space-between;
        align-items: baseline;
        margin-bottom: var(--spacing-xs);
      }

      .conversation-title {
        font-size: var(--font-size-sm);
        font-weight: var(--font-weight-semibold);
        margin: 0;
        overflow: hidden;
        text-overflow: ellipsis;
        white-space: nowrap;
      }

      .conversation-time {
        font-size: var(--font-size-xs);
        opacity: 0.7;
        flex-shrink: 0;
        margin-left: var(--spacing-sm);
      }

      .conversation-preview {
        font-size: var(--font-size-xs);
        opacity: 0.8;
        margin: 0;
        overflow: hidden;
        text-overflow: ellipsis;
        white-space: nowrap;
      }

      .conversation-actions {
        display: flex;
        gap: var(--spacing-xs);
        opacity: 0;
        transition: opacity var(--transition-fast);
      }

      .conversation-item:hover .conversation-actions {
        opacity: 1;
      }

      .action-btn {
        padding: var(--spacing-xs);
        background: transparent;
        border: none;
        border-radius: var(--border-radius-sm);
        cursor: pointer;
        transition: var(--transition-fast);
        display: flex;
        align-items: center;
        justify-content: center;
      }

      .action-btn:hover {
        background: rgba(0, 0, 0, 0.1);
      }

      .conversation-item.active .action-btn:hover {
        background: rgba(255, 255, 255, 0.2);
      }

      .action-btn svg {
        width: 14px;
        height: 14px;
        fill: currentColor;
      }

      .empty-state {
        text-align: center;
        padding: var(--spacing-xl);
        color: var(--text-tertiary);
      }

      .empty-state svg {
        width: 64px;
        height: 64px;
        margin-bottom: var(--spacing-md);
        opacity: 0.3;
      }

      .empty-state-title {
        font-size: var(--font-size-base);
        font-weight: var(--font-weight-semibold);
        margin: 0 0 var(--spacing-xs) 0;
      }

      .empty-state-text {
        font-size: var(--font-size-sm);
        margin: 0;
      }
    `;
  }

  render() {
    const filteredConversations = this._filterConversations();

    const html = `
      <style>${this.styles()}</style>
      ${this.showSearch ? `
        <div class="search-container">
          <input
            type="text"
            class="search-input"
            id="searchInput"
            placeholder="Search conversations..."
            value="${this._escapeHTML(this.searchQuery)}">
        </div>
      ` : ''}
      <div class="conversation-list">
        ${filteredConversations.length > 0
          ? filteredConversations.map((conv, index) => this._renderConversation(conv, index)).join('')
          : this._renderEmptyState()
        }
      </div>
    `;

    this.setHTML(this.shadowRoot, html);

    // Attach event listeners
    const searchInput = this.$('#searchInput');
    if (searchInput) {
      searchInput.addEventListener('input', (e) => {
        this.searchQuery = e.target.value;
      });
    }

    // Attach listeners to conversation items
    filteredConversations.forEach((conv, index) => {
      const item = this.$(`#conv-${index}`);
      const deleteBtn = this.$(`#delete-${index}`);
      const archiveBtn = this.$(`#archive-${index}`);

      if (item) {
        item.addEventListener('click', (e) => {
          if (!deleteBtn?.contains(e.target) && !archiveBtn?.contains(e.target)) {
            this._handleSelect(conv, index);
          }
        });
      }

      if (deleteBtn) {
        deleteBtn.addEventListener('click', (e) => {
          e.stopPropagation();
          this._handleDelete(conv, index);
        });
      }

      if (archiveBtn) {
        archiveBtn.addEventListener('click', (e) => {
          e.stopPropagation();
          this._handleArchive(conv, index);
        });
      }
    });
  }

  _renderConversation(conv, index) {
    const isActive = conv.id === this.currentConversationId;
    const initials = this._getInitials(conv.title || 'Chat');
    const time = this._formatTime(conv.lastMessageAt || conv.createdAt);
    const preview = conv.lastMessage || 'No messages yet';

    return `
      <div class="conversation-item ${isActive ? 'active' : ''}" id="conv-${index}">
        <div class="conversation-avatar">${initials}</div>
        <div class="conversation-content">
          <div class="conversation-header">
            <h4 class="conversation-title">${this._escapeHTML(conv.title || 'Untitled Conversation')}</h4>
            <span class="conversation-time">${time}</span>
          </div>
          <p class="conversation-preview">${this._escapeHTML(preview)}</p>
        </div>
        <div class="conversation-actions">
          <button class="action-btn" id="archive-${index}" title="Archive">
            <svg viewBox="0 0 24 24" xmlns="http://www.w3.org/2000/svg">
              <path d="M3,3H21V7H3V3M4,8H20V21H4V8M9.5,11A0.5,0.5 0 0,0 9,11.5V13H15V11.5A0.5,0.5 0 0,0 14.5,11H9.5Z"/>
            </svg>
          </button>
          <button class="action-btn" id="delete-${index}" title="Delete">
            <svg viewBox="0 0 24 24" xmlns="http://www.w3.org/2000/svg">
              <path d="M19,4H15.5L14.5,3H9.5L8.5,4H5V6H19M6,19A2,2 0 0,0 8,21H16A2,2 0 0,0 18,19V7H6V19Z"/>
            </svg>
          </button>
        </div>
      </div>
    `;
  }

  _renderEmptyState() {
    return `
      <div class="empty-state">
        <svg viewBox="0 0 24 24" xmlns="http://www.w3.org/2000/svg">
          <path d="M20,2H4A2,2 0 0,0 2,4V22L6,18H20A2,2 0 0,0 22,16V4A2,2 0 0,0 20,2M6,9H18V11H6M14,14H6V12H14M18,8H6V6H18"/>
        </svg>
        <h3 class="empty-state-title">${this.searchQuery ? 'No matches found' : 'No conversations'}</h3>
        <p class="empty-state-text">${this.searchQuery ? 'Try a different search term' : 'Start a new conversation to get started'}</p>
      </div>
    `;
  }

  _filterConversations() {
    let filtered = [...this.conversations];

    // Apply search filter
    if (this.searchQuery.trim()) {
      const query = this.searchQuery.toLowerCase();
      filtered = filtered.filter(conv =>
        (conv.title && conv.title.toLowerCase().includes(query)) ||
        (conv.lastMessage && conv.lastMessage.toLowerCase().includes(query))
      );
    }

    // Apply sorting
    if (this.sortBy === 'recent') {
      filtered.sort((a, b) => {
        const dateA = new Date(a.lastMessageAt || a.createdAt);
        const dateB = new Date(b.lastMessageAt || b.createdAt);
        return dateB - dateA;
      });
    } else if (this.sortBy === 'alphabetical') {
      filtered.sort((a, b) => (a.title || '').localeCompare(b.title || ''));
    }

    return filtered;
  }

  _getInitials(text) {
    const words = text.trim().split(/\s+/);
    if (words.length >= 2) {
      return (words[0][0] + words[1][0]).toUpperCase();
    }
    return text.substring(0, 2).toUpperCase();
  }

  _formatTime(dateStr) {
    const date = new Date(dateStr);
    const now = new Date();
    const diff = now - date;
    const minutes = Math.floor(diff / (1000 * 60));
    const hours = Math.floor(diff / (1000 * 60 * 60));
    const days = Math.floor(diff / (1000 * 60 * 60 * 24));

    if (minutes < 1) return 'Just now';
    if (minutes < 60) return `${minutes}m`;
    if (hours < 24) return `${hours}h`;
    if (days < 7) return `${days}d`;
    return date.toLocaleDateString([], { month: 'short', day: 'numeric' });
  }

  _handleSelect(conv, index) {
    this.dispatchEvent(new CustomEvent('conversation-select', {
      detail: { conversation: conv, index, id: conv.id },
      bubbles: true,
      composed: true
    }));
  }

  _handleDelete(conv, index) {
    if (confirm(`Delete conversation "${conv.title}"?`)) {
      this.dispatchEvent(new CustomEvent('conversation-delete', {
        detail: { conversation: conv, index, id: conv.id },
        bubbles: true,
        composed: true
      }));
    }
  }

  _handleArchive(conv, index) {
    this.dispatchEvent(new CustomEvent('conversation-archive', {
      detail: { conversation: conv, index, id: conv.id },
      bubbles: true,
      composed: true
    }));
  }

  // Public API
  addConversation(conversation) {
    this.conversations = [conversation, ...this.conversations];
  }

  deleteConversation(id) {
    this.conversations = this.conversations.filter(c => c.id !== id);
  }

  updateConversation(id, updates) {
    this.conversations = this.conversations.map(c =>
      c.id === id ? { ...c, ...updates } : c
    );
  }

  _escapeHTML(str) {
    const div = document.createElement('div');
    div.textContent = str;
    return div.innerHTML;
  }
}

customElements.define('terraphim-conversation-list', TerraphimConversationList);
