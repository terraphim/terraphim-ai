import { TerraphimElement } from '../base/terraphim-element.js';

/**
 * TerraphimArticleModal Component
 *
 * Modal for displaying full article content with metadata and actions.
 *
 * @fires article-save - When article is saved to knowledge graph
 * @fires article-copy - When article content is copied
 * @fires modal-close - When modal is closed
 *
 * @example
 * ```html
 * <terraphim-article-modal
 *   title="Article Title"
 *   author="Author Name"
 *   render-markdown>
 * </terraphim-article-modal>
 * ```
 */
export class TerraphimArticleModal extends TerraphimElement {
  static get properties() {
    return {
      isOpen: { type: Boolean, default: false },
      title: { type: String, default: 'Article' },
      author: { type: String, default: '' },
      date: { type: String, default: '' },
      url: { type: String, default: '' },
      tags: { type: Array, default: () => [] },
      content: { type: String, default: '' },
      renderMarkdown: { type: Boolean, default: false },
      showActions: { type: Boolean, reflect: true, default: true },
    };
  }

  constructor() {
    super();
    this.attachShadow({ mode: 'open' });
  }

  styles() {
    return `
      :host {
        display: none;
        position: fixed;
        top: 0;
        left: 0;
        width: 100%;
        height: 100%;
        z-index: 1000;
      }

      :host([is-open]) {
        display: flex;
        align-items: center;
        justify-content: center;
      }

      .modal-backdrop {
        position: absolute;
        top: 0;
        left: 0;
        width: 100%;
        height: 100%;
        background: rgba(0, 0, 0, 0.5);
        animation: fadeIn 0.2s ease-out;
      }

      .modal-container {
        position: relative;
        background: var(--bg-elevated);
        border-radius: var(--border-radius-lg);
        box-shadow: 0 20px 60px rgba(0, 0, 0, 0.3);
        width: 90%;
        max-width: 900px;
        max-height: 90vh;
        display: flex;
        flex-direction: column;
        animation: slideIn 0.3s ease-out;
        z-index: 1001;
      }

      .modal-header {
        display: flex;
        justify-content: space-between;
        align-items: flex-start;
        padding: var(--spacing-lg);
        border-bottom: 1px solid var(--border-primary);
      }

      .header-content {
        flex: 1;
        min-width: 0;
      }

      .modal-title {
        font-size: var(--font-size-2xl);
        font-weight: var(--font-weight-bold);
        margin: 0 0 var(--spacing-sm) 0;
        color: var(--text-primary);
      }

      .article-meta {
        display: flex;
        flex-wrap: wrap;
        gap: var(--spacing-md);
        font-size: var(--font-size-sm);
        color: var(--text-secondary);
        margin-bottom: var(--spacing-sm);
      }

      .meta-item {
        display: flex;
        align-items: center;
        gap: var(--spacing-xs);
      }

      .meta-item svg {
        width: 16px;
        height: 16px;
        fill: currentColor;
        opacity: 0.7;
      }

      .article-tags {
        display: flex;
        flex-wrap: wrap;
        gap: var(--spacing-xs);
        margin-top: var(--spacing-xs);
      }

      .tag {
        padding: var(--spacing-xs) var(--spacing-sm);
        background: var(--bg-hover);
        border: 1px solid var(--border-primary);
        border-radius: var(--border-radius-sm);
        font-size: var(--font-size-xs);
        color: var(--text-secondary);
      }

      .header-actions {
        display: flex;
        gap: var(--spacing-xs);
        margin-left: var(--spacing-md);
      }

      .action-button {
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

      .action-button:hover {
        background: var(--bg-hover);
        color: var(--text-primary);
        border-color: var(--border-hover);
      }

      .action-button svg {
        width: 20px;
        height: 20px;
        fill: currentColor;
      }

      .close-button {
        background: transparent;
        border: none;
        padding: var(--spacing-xs);
        cursor: pointer;
        border-radius: var(--border-radius-sm);
        color: var(--text-secondary);
        transition: var(--transition-base);
      }

      .close-button:hover {
        background: var(--bg-hover);
        color: var(--text-primary);
      }

      .close-button svg {
        width: 24px;
        height: 24px;
        fill: currentColor;
      }

      .modal-body {
        flex: 1;
        overflow-y: auto;
        padding: var(--spacing-xl);
        color: var(--text-primary);
        line-height: 1.6;
      }

      .article-content {
        max-width: 720px;
        margin: 0 auto;
      }

      .article-content pre {
        background: var(--bg-secondary);
        padding: var(--spacing-md);
        border-radius: var(--border-radius-md);
        overflow-x: auto;
      }

      .article-content code {
        background: var(--bg-secondary);
        padding: var(--spacing-xs);
        border-radius: var(--border-radius-sm);
        font-family: var(--font-family-mono);
        font-size: 0.9em;
      }

      .article-content pre code {
        background: transparent;
        padding: 0;
      }

      .article-content img {
        max-width: 100%;
        height: auto;
        border-radius: var(--border-radius-md);
      }

      .article-content h1,
      .article-content h2,
      .article-content h3 {
        margin-top: var(--spacing-lg);
        margin-bottom: var(--spacing-md);
      }

      .article-content p {
        margin-bottom: var(--spacing-md);
      }

      .article-content a {
        color: var(--color-primary);
        text-decoration: none;
      }

      .article-content a:hover {
        text-decoration: underline;
      }

      .modal-footer {
        display: flex;
        justify-content: space-between;
        align-items: center;
        padding: var(--spacing-lg);
        border-top: 1px solid var(--border-primary);
        background: var(--bg-secondary);
      }

      .footer-info {
        font-size: var(--font-size-xs);
        color: var(--text-tertiary);
      }

      .footer-actions {
        display: flex;
        gap: var(--spacing-sm);
      }

      .modal-button {
        padding: var(--spacing-sm) var(--spacing-lg);
        border: none;
        border-radius: var(--border-radius-md);
        font-family: var(--font-family-sans);
        font-size: var(--font-size-sm);
        font-weight: var(--font-weight-medium);
        cursor: pointer;
        transition: var(--transition-base);
        display: flex;
        align-items: center;
        gap: var(--spacing-xs);
      }

      .modal-button.secondary {
        background: var(--bg-elevated);
        color: var(--text-primary);
        border: 1px solid var(--border-primary);
      }

      .modal-button.secondary:hover {
        background: var(--bg-hover);
      }

      .modal-button.primary {
        background: var(--color-primary);
        color: var(--color-primary-contrast);
      }

      .modal-button.primary:hover {
        background: var(--color-primary-dark);
      }

      .modal-button svg {
        width: 16px;
        height: 16px;
        fill: currentColor;
      }

      @keyframes fadeIn {
        from { opacity: 0; }
        to { opacity: 1; }
      }

      @keyframes slideIn {
        from {
          opacity: 0;
          transform: translateY(-20px);
        }
        to {
          opacity: 1;
          transform: translateY(0);
        }
      }

      /* Reading mode enhancements */
      .modal-body::-webkit-scrollbar {
        width: 8px;
      }

      .modal-body::-webkit-scrollbar-track {
        background: var(--bg-secondary);
      }

      .modal-body::-webkit-scrollbar-thumb {
        background: var(--border-primary);
        border-radius: var(--border-radius-sm);
      }

      .modal-body::-webkit-scrollbar-thumb:hover {
        background: var(--border-hover);
      }
    `;
  }

  render() {
    const wordCount = this.content ? this.content.split(/\s+/).length : 0;
    const readTime = Math.ceil(wordCount / 200);

    const html = `
      <style>${this.styles()}</style>
      <div class="modal-backdrop" id="backdrop"></div>
      <div class="modal-container" role="dialog" aria-modal="true" aria-labelledby="modal-title">
        <div class="modal-header">
          <div class="header-content">
            <h2 class="modal-title" id="modal-title">${this._escapeHTML(this.title)}</h2>
            <div class="article-meta">
              ${this.author ? `
                <div class="meta-item">
                  <svg viewBox="0 0 24 24" xmlns="http://www.w3.org/2000/svg">
                    <path d="M12,4A4,4 0 0,1 16,8A4,4 0 0,1 12,12A4,4 0 0,1 8,8A4,4 0 0,1 12,4M12,14C16.42,14 20,15.79 20,18V20H4V18C4,15.79 7.58,14 12,14Z"/>
                  </svg>
                  ${this._escapeHTML(this.author)}
                </div>
              ` : ''}
              ${this.date ? `
                <div class="meta-item">
                  <svg viewBox="0 0 24 24" xmlns="http://www.w3.org/2000/svg">
                    <path d="M19,19H5V8H19M16,1V3H8V1H6V3H5C3.89,3 3,3.89 3,5V19A2,2 0 0,0 5,21H19A2,2 0 0,0 21,19V5C21,3.89 20.1,3 19,3H18V1M17,12H12V17H17V12Z"/>
                  </svg>
                  ${this._escapeHTML(this.date)}
                </div>
              ` : ''}
              ${wordCount > 0 ? `
                <div class="meta-item">
                  <svg viewBox="0 0 24 24" xmlns="http://www.w3.org/2000/svg">
                    <path d="M12,20A8,8 0 0,0 20,12A8,8 0 0,0 12,4A8,8 0 0,0 4,12A8,8 0 0,0 12,20M12,2A10,10 0 0,1 22,12A10,10 0 0,1 12,22C6.47,22 2,17.5 2,12A10,10 0 0,1 12,2M12.5,7V12.25L17,14.92L16.25,16.15L11,13V7H12.5Z"/>
                  </svg>
                  ${readTime} min read
                </div>
              ` : ''}
            </div>
            ${this.tags.length > 0 ? `
              <div class="article-tags">
                ${this.tags.map(tag => `<span class="tag">${this._escapeHTML(tag)}</span>`).join('')}
              </div>
            ` : ''}
          </div>
          ${this.showActions ? `
            <div class="header-actions">
              <button class="action-button" id="copyBtn" title="Copy content">
                <svg viewBox="0 0 24 24" xmlns="http://www.w3.org/2000/svg">
                  <path d="M19,21H8V7H19M19,5H8A2,2 0 0,0 6,7V21A2,2 0 0,0 8,23H19A2,2 0 0,0 21,21V7A2,2 0 0,0 19,5M16,1H4A2,2 0 0,0 2,3V17H4V3H16V1Z"/>
                </svg>
              </button>
              <button class="action-button" id="saveBtn" title="Save to knowledge graph">
                <svg viewBox="0 0 24 24" xmlns="http://www.w3.org/2000/svg">
                  <path d="M15,9H5V5H15M12,19A3,3 0 0,1 9,16A3,3 0 0,1 12,13A3,3 0 0,1 15,16A3,3 0 0,1 12,19M17,3H5C3.89,3 3,3.9 3,5V19A2,2 0 0,0 5,21H19A2,2 0 0,0 21,19V7L17,3Z"/>
                </svg>
              </button>
            </div>
          ` : ''}
          <button class="close-button" id="closeBtn" aria-label="Close modal">
            <svg viewBox="0 0 24 24" xmlns="http://www.w3.org/2000/svg">
              <path d="M19,6.41L17.59,5L12,10.59L6.41,5L5,6.41L10.59,12L5,17.59L6.41,19L12,13.41L17.59,19L19,17.59L13.41,12L19,6.41Z"/>
            </svg>
          </button>
        </div>
        <div class="modal-body">
          <div class="article-content" id="articleContent">
            ${this.renderMarkdown ? this._renderMarkdown(this.content) : this._escapeHTML(this.content)}
          </div>
        </div>
        ${this.url ? `
          <div class="modal-footer">
            <div class="footer-info">
              Source: <a href="${this._escapeHTML(this.url)}" target="_blank" rel="noopener noreferrer">${this._escapeHTML(this.url)}</a>
            </div>
          </div>
        ` : ''}
      </div>
    `;

    this.setHTML(this.shadowRoot, html);

    // Attach event listeners
    const backdrop = this.$('#backdrop');
    const closeBtn = this.$('#closeBtn');
    const copyBtn = this.$('#copyBtn');
    const saveBtn = this.$('#saveBtn');

    if (backdrop) {
      backdrop.addEventListener('click', () => this.close());
    }

    if (closeBtn) {
      closeBtn.addEventListener('click', () => this.close());
    }

    if (copyBtn) {
      copyBtn.addEventListener('click', () => this._handleCopy());
    }

    if (saveBtn) {
      saveBtn.addEventListener('click', () => this._handleSave());
    }

    // Escape key handler
    document.addEventListener('keydown', this._handleEscape);
  }

  disconnectedCallback() {
    super.disconnectedCallback();
    document.removeEventListener('keydown', this._handleEscape);
  }

  _handleEscape = (e) => {
    if (e.key === 'Escape' && this.isOpen) {
      this.close();
    }
  };

  _handleCopy() {
    navigator.clipboard.writeText(this.content).then(() => {
      this.dispatchEvent(new CustomEvent('article-copy', {
        detail: { content: this.content },
        bubbles: true,
        composed: true
      }));
      // Visual feedback could be added here
      const copyBtn = this.$('#copyBtn');
      if (copyBtn) {
        const originalTitle = copyBtn.getAttribute('title');
        copyBtn.setAttribute('title', 'Copied!');
        setTimeout(() => {
          copyBtn.setAttribute('title', originalTitle);
        }, 2000);
      }
    });
  }

  _handleSave() {
    this.dispatchEvent(new CustomEvent('article-save', {
      detail: {
        title: this.title,
        author: this.author,
        date: this.date,
        url: this.url,
        tags: this.tags,
        content: this.content
      },
      bubbles: true,
      composed: true
    }));
  }

  _renderMarkdown(text) {
    // Simple markdown rendering (basic support)
    // For production, consider using a proper markdown library
    return text
      .replace(/^### (.+)$/gm, '<h3>$1</h3>')
      .replace(/^## (.+)$/gm, '<h2>$1</h2>')
      .replace(/^# (.+)$/gm, '<h1>$1</h1>')
      .replace(/\*\*(.+?)\*\*/g, '<strong>$1</strong>')
      .replace(/\*(.+?)\*/g, '<em>$1</em>')
      .replace(/`(.+?)`/g, '<code>$1</code>')
      .replace(/\n\n/g, '</p><p>')
      .replace(/^(.+)$/gm, '<p>$1</p>')
      .replace(/<p><h([1-6])>/g, '<h$1>')
      .replace(/<\/h([1-6])><\/p>/g, '</h$1>');
  }

  // Public API
  open(options = {}) {
    if (options.title) this.title = options.title;
    if (options.author) this.author = options.author;
    if (options.date) this.date = options.date;
    if (options.url) this.url = options.url;
    if (options.tags) this.tags = options.tags;
    if (options.content) this.content = options.content;
    if (options.renderMarkdown !== undefined) this.renderMarkdown = options.renderMarkdown;
    this.isOpen = true;
  }

  close() {
    this.isOpen = false;
    this.dispatchEvent(new CustomEvent('modal-close', {
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

customElements.define('terraphim-article-modal', TerraphimArticleModal);
