import { TerraphimElement } from '../base/terraphim-element.js';

/**
 * TerraphimSessionManager Component
 *
 * Manages chat sessions with save/load/delete capabilities.
 *
 * @fires session-create - When a new session is created
 * @fires session-select - When a session is selected {session, id}
 * @fires session-delete - When a session is deleted {session, id}
 * @fires session-rename - When a session is renamed {session, id, newName}
 *
 * @example
 * ```html
 * <terraphim-session-manager
 *   current-session-id="123"
 *   show-create-button>
 * </terraphim-session-manager>
 * ```
 */
export class TerraphimSessionManager extends TerraphimElement {
  static get properties() {
    return {
      sessions: { type: Array, default: () => [] },
      currentSessionId: { type: String },
      showCreateButton: { type: Boolean, reflect: true, default: true },
      compact: { type: Boolean, reflect: true },
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
        background: var(--bg-secondary);
        padding: var(--spacing-sm);
      }

      .session-header {
        display: flex;
        justify-content: space-between;
        align-items: center;
        margin-bottom: var(--spacing-sm);
      }

      .session-title {
        font-size: var(--font-size-sm);
        font-weight: var(--font-weight-semibold);
        color: var(--text-secondary);
        text-transform: uppercase;
        letter-spacing: 0.05em;
        margin: 0;
      }

      .create-session-btn {
        padding: var(--spacing-xs) var(--spacing-sm);
        background: var(--color-primary);
        color: var(--color-primary-contrast);
        border: none;
        border-radius: var(--border-radius-md);
        cursor: pointer;
        font-size: var(--font-size-xs);
        font-weight: var(--font-weight-medium);
        transition: var(--transition-base);
        display: flex;
        align-items: center;
        gap: var(--spacing-xs);
      }

      .create-session-btn:hover {
        background: var(--color-primary-dark);
      }

      .create-session-btn svg {
        width: 14px;
        height: 14px;
        fill: currentColor;
      }

      .session-list {
        display: flex;
        flex-direction: column;
        gap: var(--spacing-xs);
      }

      .session-item {
        display: flex;
        align-items: center;
        gap: var(--spacing-sm);
        padding: var(--spacing-sm);
        background: var(--bg-elevated);
        border: 1px solid var(--border-primary);
        border-radius: var(--border-radius-md);
        cursor: pointer;
        transition: var(--transition-base);
      }

      .session-item:hover {
        background: var(--bg-hover);
        border-color: var(--border-hover);
      }

      .session-item.active {
        background: var(--color-primary);
        color: var(--color-primary-contrast);
        border-color: var(--color-primary);
      }

      .session-icon {
        flex-shrink: 0;
        width: 24px;
        height: 24px;
        display: flex;
        align-items: center;
        justify-content: center;
        opacity: 0.7;
      }

      .session-icon svg {
        width: 18px;
        height: 18px;
        fill: currentColor;
      }

      .session-item.active .session-icon {
        opacity: 1;
      }

      .session-content {
        flex: 1;
        min-width: 0;
      }

      .session-name {
        font-size: var(--font-size-sm);
        font-weight: var(--font-weight-medium);
        margin: 0 0 var(--spacing-xs) 0;
        overflow: hidden;
        text-overflow: ellipsis;
        white-space: nowrap;
      }

      .session-meta {
        font-size: var(--font-size-xs);
        opacity: 0.8;
        margin: 0;
      }

      .session-actions {
        display: flex;
        gap: var(--spacing-xs);
        opacity: 0;
        transition: opacity var(--transition-fast);
      }

      .session-item:hover .session-actions {
        opacity: 1;
      }

      .session-item.active .session-actions {
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

      .session-item.active .action-btn:hover {
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
        width: 48px;
        height: 48px;
        margin-bottom: var(--spacing-md);
        opacity: 0.5;
      }

      .empty-state-text {
        font-size: var(--font-size-sm);
        margin: 0;
      }

      :host([compact]) .session-meta {
        display: none;
      }

      :host([compact]) .session-item {
        padding: var(--spacing-xs) var(--spacing-sm);
      }
    `;
  }

  render() {
    const html = `
      <style>${this.styles()}</style>
      <div class="session-header">
        <h3 class="session-title">Sessions</h3>
        ${this.showCreateButton ? `
          <button class="create-session-btn" id="createBtn">
            <svg viewBox="0 0 24 24" xmlns="http://www.w3.org/2000/svg">
              <path d="M19,13H13V19H11V13H5V11H11V5H13V11H19V13Z"/>
            </svg>
            New
          </button>
        ` : ''}
      </div>
      <div class="session-list">
        ${this.sessions.length > 0
          ? this.sessions.map((session, index) => this._renderSession(session, index)).join('')
          : `
            <div class="empty-state">
              <svg viewBox="0 0 24 24" xmlns="http://www.w3.org/2000/svg">
                <path d="M12,2A2,2 0 0,1 14,4C14,4.74 13.6,5.39 13,5.73V7H14A7,7 0 0,1 21,14H22A1,1 0 0,1 23,15V18A1,1 0 0,1 22,19H21V20A2,2 0 0,1 19,22H5A2,2 0 0,1 3,20V19H2A1,1 0 0,1 1,18V15A1,1 0 0,1 2,14H3A7,7 0 0,1 10,7H11V5.73C10.4,5.39 10,4.74 10,4A2,2 0 0,1 12,2M7.5,13A0.5,0.5 0 0,0 7,13.5A0.5,0.5 0 0,0 7.5,14A0.5,0.5 0 0,0 8,13.5A0.5,0.5 0 0,0 7.5,13M16.5,13A0.5,0.5 0 0,0 16,13.5A0.5,0.5 0 0,0 16.5,14A0.5,0.5 0 0,0 17,13.5A0.5,0.5 0 0,0 16.5,13Z"/>
              </svg>
              <p class="empty-state-text">No sessions yet.<br>Create one to get started!</p>
            </div>
          `
        }
      </div>
    `;

    this.setHTML(this.shadowRoot, html);

    // Attach event listeners
    const createBtn = this.$('#createBtn');
    if (createBtn) {
      createBtn.addEventListener('click', this._handleCreate.bind(this));
    }

    // Attach listeners to session items
    this.sessions.forEach((session, index) => {
      const item = this.$(`#session-${index}`);
      const deleteBtn = this.$(`#delete-${index}`);
      const renameBtn = this.$(`#rename-${index}`);

      if (item) {
        item.addEventListener('click', (e) => {
          if (!deleteBtn?.contains(e.target) && !renameBtn?.contains(e.target)) {
            this._handleSelect(session, index);
          }
        });
      }

      if (deleteBtn) {
        deleteBtn.addEventListener('click', (e) => {
          e.stopPropagation();
          this._handleDelete(session, index);
        });
      }

      if (renameBtn) {
        renameBtn.addEventListener('click', (e) => {
          e.stopPropagation();
          this._handleRename(session, index);
        });
      }
    });
  }

  _renderSession(session, index) {
    const isActive = session.id === this.currentSessionId;
    const messageCount = session.messageCount || 0;
    const lastUpdated = session.lastUpdated ? this._formatDate(session.lastUpdated) : 'Just now';

    return `
      <div class="session-item ${isActive ? 'active' : ''}" id="session-${index}">
        <div class="session-icon">
          <svg viewBox="0 0 24 24" xmlns="http://www.w3.org/2000/svg">
            <path d="M12,2A2,2 0 0,1 14,4C14,4.74 13.6,5.39 13,5.73V7H14A7,7 0 0,1 21,14H22A1,1 0 0,1 23,15V18A1,1 0 0,1 22,19H21V20A2,2 0 0,1 19,22H5A2,2 0 0,1 3,20V19H2A1,1 0 0,1 1,18V15A1,1 0 0,1 2,14H3A7,7 0 0,1 10,7H11V5.73C10.4,5.39 10,4.74 10,4A2,2 0 0,1 12,2M7.5,13A0.5,0.5 0 0,0 7,13.5A0.5,0.5 0 0,0 7.5,14A0.5,0.5 0 0,0 8,13.5A0.5,0.5 0 0,0 7.5,13M16.5,13A0.5,0.5 0 0,0 16,13.5A0.5,0.5 0 0,0 16.5,14A0.5,0.5 0 0,0 17,13.5A0.5,0.5 0 0,0 16.5,13Z"/>
          </svg>
        </div>
        <div class="session-content">
          <p class="session-name">${this._escapeHTML(session.name || 'Unnamed Session')}</p>
          <p class="session-meta">${messageCount} messages • ${lastUpdated}</p>
        </div>
        <div class="session-actions">
          <button class="action-btn" id="rename-${index}" title="Rename">
            <svg viewBox="0 0 24 24" xmlns="http://www.w3.org/2000/svg">
              <path d="M20.71,7.04C21.1,6.65 21.1,6 20.71,5.63L18.37,3.29C18,2.9 17.35,2.9 16.96,3.29L15.12,5.12L18.87,8.87M3,17.25V21H6.75L17.81,9.93L14.06,6.18L3,17.25Z"/>
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

  _formatDate(dateStr) {
    const date = new Date(dateStr);
    const now = new Date();
    const diff = now - date;
    const days = Math.floor(diff / (1000 * 60 * 60 * 24));

    if (days === 0) return 'Today';
    if (days === 1) return 'Yesterday';
    if (days < 7) return `${days} days ago`;
    return date.toLocaleDateString();
  }

  _handleCreate() {
    this.dispatchEvent(new CustomEvent('session-create', {
      bubbles: true,
      composed: true
    }));
  }

  _handleSelect(session, index) {
    this.dispatchEvent(new CustomEvent('session-select', {
      detail: { session, index, id: session.id },
      bubbles: true,
      composed: true
    }));
  }

  _handleDelete(session, index) {
    if (confirm(`Delete session "${session.name}"?`)) {
      this.dispatchEvent(new CustomEvent('session-delete', {
        detail: { session, index, id: session.id },
        bubbles: true,
        composed: true
      }));
    }
  }

  _handleRename(session, index) {
    const newName = prompt('Enter new session name:', session.name);
    if (newName && newName.trim()) {
      this.dispatchEvent(new CustomEvent('session-rename', {
        detail: { session, index, id: session.id, newName: newName.trim() },
        bubbles: true,
        composed: true
      }));
    }
  }

  // Public API
  createSession(name, id = null) {
    const session = {
      id: id || Date.now().toString(),
      name: name || 'New Session',
      messageCount: 0,
      lastUpdated: new Date().toISOString(),
      createdAt: new Date().toISOString()
    };
    this.sessions = [...this.sessions, session];
    return session;
  }

  deleteSession(id) {
    this.sessions = this.sessions.filter(s => s.id !== id);
  }

  renameSession(id, newName) {
    this.sessions = this.sessions.map(s =>
      s.id === id ? { ...s, name: newName, lastUpdated: new Date().toISOString() } : s
    );
  }

  updateSession(id, updates) {
    this.sessions = this.sessions.map(s =>
      s.id === id ? { ...s, ...updates, lastUpdated: new Date().toISOString() } : s
    );
  }

  _escapeHTML(str) {
    const div = document.createElement('div');
    div.textContent = str;
    return div.innerHTML;
  }
}

customElements.define('terraphim-session-manager', TerraphimSessionManager);
