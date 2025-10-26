/**
 * @fileoverview TerraphimChatMessage - Individual message bubble component
 * Displays a single chat message with markdown support, copy functionality, and streaming indicator
 */

import { TerraphimElement } from '../base/terraphim-element.js';

/**
 * Lightweight markdown renderer (no external dependencies)
 * Supports: headers, bold, italic, inline code, code blocks, links, lists
 */
class MarkdownRenderer {
  /**
   * Render markdown to HTML
   * @param {string} markdown - Markdown text
   * @returns {string} HTML string
   */
  static render(markdown) {
    let html = markdown;

    // Escape HTML first to prevent XSS
    html = this._escapeHTML(html);

    // Code blocks (must be first to protect from other replacements)
    html = html.replace(/```(\w+)?\n([\s\S]*?)```/g, (match, lang, code) => {
      return `<pre><code class="language-${lang || 'text'}">${code.trim()}</code></pre>`;
    });

    // Inline code
    html = html.replace(/`([^`]+)`/g, '<code>$1</code>');

    // Headers (h1, h2, h3)
    html = html.replace(/^### (.*$)/gim, '<h3>$1</h3>');
    html = html.replace(/^## (.*$)/gim, '<h2>$1</h2>');
    html = html.replace(/^# (.*$)/gim, '<h1>$1</h1>');

    // Bold
    html = html.replace(/\*\*(.+?)\*\*/g, '<strong>$1</strong>');

    // Italic
    html = html.replace(/\*(.+?)\*/g, '<em>$1</em>');

    // Links
    html = html.replace(/\[([^\]]+)\]\(([^)]+)\)/g, '<a href="$2" target="_blank" rel="noopener noreferrer">$1</a>');

    // Unordered lists
    html = html.replace(/^\* (.+)$/gim, '<li>$1</li>');
    html = html.replace(/(<li>.*<\/li>)/s, '<ul>$1</ul>');

    // Ordered lists
    html = html.replace(/^\d+\. (.+)$/gim, '<li>$1</li>');

    // Line breaks and paragraphs
    html = html.replace(/\n\n/g, '</p><p>');
    html = html.replace(/\n/g, '<br>');

    return `<div class="markdown-body">${html}</div>`;
  }

  /**
   * Escape HTML to prevent XSS
   * @private
   * @param {string} text - Text to escape
   * @returns {string} Escaped text
   */
  static _escapeHTML(text) {
    const div = document.createElement('div');
    div.textContent = text;
    return div.innerHTML;
  }
}

/**
 * TerraphimChatMessage - Individual message bubble component
 *
 * @element terraphim-chat-message
 *
 * @attr {string} role - Message role: 'user' | 'assistant' | 'system'
 * @attr {string} content - Message content
 * @attr {string} timestamp - ISO timestamp
 * @attr {boolean} render-markdown - Render content as markdown
 * @attr {boolean} streaming - Show streaming indicator
 *
 * @fires copy-message - When message is copied to clipboard
 * @fires save-message - When save as markdown is requested
 *
 * @example
 * <terraphim-chat-message
 *   role="assistant"
 *   content="Hello! How can I help?"
 *   timestamp="2025-10-26T10:30:00Z"
 *   render-markdown>
 * </terraphim-chat-message>
 */
export class TerraphimChatMessage extends TerraphimElement {
  static get observedAttributes() {
    return ['role', 'content', 'timestamp', 'render-markdown', 'streaming'];
  }

  static get properties() {
    return {
      role: { type: String, reflect: true, default: 'user' },
      content: { type: String, default: '' },
      timestamp: { type: String, default: '' },
      renderMarkdown: { type: Boolean, reflect: true },
      streaming: { type: Boolean, reflect: true }
    };
  }

  constructor() {
    super();
    this.attachShadow({ mode: 'open' });
  }

  /**
   * Copy message content to clipboard
   * @returns {Promise<void>}
   */
  async copyToClipboard() {
    try {
      await navigator.clipboard.writeText(this.content);
      this.emit('copy-message', { content: this.content });

      // Show temporary success feedback
      const copyBtn = this.$('.copy-btn');
      if (copyBtn) {
        const originalText = copyBtn.textContent;
        copyBtn.textContent = 'Copied!';
        setTimeout(() => {
          copyBtn.textContent = originalText;
        }, 2000);
      }
    } catch (error) {
      console.error('Failed to copy to clipboard:', error);
    }
  }

  /**
   * Save message as markdown file
   * @returns {Promise<void>}
   */
  async saveAsMarkdown() {
    const blob = new Blob([this.content], { type: 'text/markdown' });
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = `message-${Date.now()}.md`;
    a.click();
    URL.revokeObjectURL(url);

    this.emit('save-message', { content: this.content });
  }

  /**
   * Format timestamp for display
   * @private
   * @param {string} timestamp - ISO timestamp
   * @returns {string} Formatted time
   */
  _formatTimestamp(timestamp) {
    if (!timestamp) return '';

    try {
      const date = new Date(timestamp);
      return date.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' });
    } catch (error) {
      return '';
    }
  }

  /**
   * Render the component
   */
  render() {
    const styles = `
      <style>
        :host {
          display: flex;
          margin-bottom: var(--spacing-sm, 0.5rem);
          animation: slideIn 200ms ease-out;
        }

        @keyframes slideIn {
          from {
            opacity: 0;
            transform: translateY(10px);
          }
          to {
            opacity: 1;
            transform: translateY(0);
          }
        }

        :host([role="user"]) {
          justify-content: flex-end;
        }

        :host([role="assistant"]) {
          justify-content: flex-start;
        }

        :host([role="system"]) {
          justify-content: center;
        }

        .message-container {
          display: flex;
          flex-direction: column;
          max-width: var(--message-bubble-max-width, 70ch);
          width: 100%;
        }

        .bubble {
          padding: var(--message-bubble-padding, 0.75rem);
          border-radius: var(--message-bubble-border-radius, 12px);
          word-wrap: break-word;
          overflow-wrap: break-word;
          transition: var(--transition-base);
          position: relative;
        }

        :host([role="user"]) .bubble {
          background: var(--color-primary, #3273dc);
          color: var(--color-primary-contrast, #ffffff);
          border-bottom-right-radius: 4px;
        }

        :host([role="assistant"]) .bubble {
          background: var(--bg-elevated, #ffffff);
          color: var(--text-primary, #363636);
          border: 1px solid var(--border-primary, #dbdbdb);
          border-bottom-left-radius: 4px;
        }

        :host([role="system"]) .bubble {
          background: var(--bg-secondary, #fafafa);
          color: var(--text-secondary, #4a4a4a);
          text-align: center;
          font-size: var(--font-size-sm, 0.875rem);
          font-style: italic;
        }

        .content {
          white-space: pre-wrap;
          font-family: var(--font-family-sans);
          margin: 0;
          line-height: var(--line-height-normal, 1.5);
        }

        .markdown-body {
          font-family: var(--font-family-sans);
        }

        .markdown-body h1 {
          font-size: var(--font-size-2xl, 1.5rem);
          margin: var(--spacing-sm, 0.5rem) 0;
          font-weight: var(--font-weight-bold, 700);
        }

        .markdown-body h2 {
          font-size: var(--font-size-xl, 1.25rem);
          margin: var(--spacing-sm, 0.5rem) 0;
          font-weight: var(--font-weight-semibold, 600);
        }

        .markdown-body h3 {
          font-size: var(--font-size-lg, 1.125rem);
          margin: var(--spacing-sm, 0.5rem) 0;
          font-weight: var(--font-weight-semibold, 600);
        }

        .markdown-body pre {
          background: var(--bg-code, #f5f5f5);
          padding: var(--spacing-sm, 0.5rem);
          border-radius: var(--border-radius-md, 4px);
          overflow-x: auto;
          margin: var(--spacing-sm, 0.5rem) 0;
        }

        .markdown-body code {
          font-family: var(--font-family-mono);
          font-size: 0.9em;
          background: var(--bg-code, #f5f5f5);
          padding: 0.2em 0.4em;
          border-radius: var(--border-radius-sm, 2px);
        }

        .markdown-body pre code {
          background: none;
          padding: 0;
        }

        .markdown-body a {
          color: var(--text-link, #3273dc);
          text-decoration: underline;
        }

        .markdown-body a:hover {
          color: var(--text-link-hover, #2366d1);
        }

        .markdown-body ul,
        .markdown-body ol {
          margin: var(--spacing-sm, 0.5rem) 0;
          padding-left: var(--spacing-lg, 1.5rem);
        }

        .markdown-body li {
          margin: var(--spacing-xs, 0.25rem) 0;
        }

        .actions {
          display: flex;
          gap: var(--spacing-xs, 0.25rem);
          margin-top: var(--spacing-xs, 0.25rem);
          opacity: 0;
          transition: opacity var(--transition-fast);
        }

        .message-container:hover .actions,
        .actions:focus-within {
          opacity: 1;
        }

        .actions button {
          padding: var(--spacing-xs, 0.25rem) var(--spacing-sm, 0.5rem);
          border: 1px solid var(--border-primary, #dbdbdb);
          border-radius: var(--border-radius-sm, 2px);
          background: var(--bg-elevated, #ffffff);
          color: var(--text-secondary, #4a4a4a);
          cursor: pointer;
          font-size: var(--font-size-xs, 0.75rem);
          transition: var(--transition-fast);
          font-family: var(--font-family-sans);
        }

        .actions button:hover {
          background: var(--bg-hover, rgba(0, 0, 0, 0.05));
          border-color: var(--border-hover, #b5b5b5);
        }

        .actions button:focus-visible {
          outline: 2px solid var(--border-focus, #3273dc);
          outline-offset: 2px;
        }

        .timestamp {
          font-size: var(--font-size-xs, 0.75rem);
          color: var(--text-tertiary, #7a7a7a);
          margin-top: var(--spacing-xs, 0.25rem);
          opacity: 0.8;
        }

        :host([role="user"]) .timestamp {
          text-align: right;
        }

        .streaming-indicator {
          display: inline-block;
          margin-left: var(--spacing-xs, 0.25rem);
        }

        .streaming-indicator::after {
          content: '•••';
          animation: dots 1.5s infinite;
        }

        @keyframes dots {
          0%, 20% { content: '•'; }
          40% { content: '••'; }
          60%, 100% { content: '•••'; }
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

    const contentHTML = this.renderMarkdown
      ? MarkdownRenderer.render(this.content)
      : `<div class="content">${this._escapeHTML(this.content)}</div>`;

    const timestampDisplay = this._formatTimestamp(this.timestamp);
    const streamingIndicator = this.streaming
      ? '<span class="streaming-indicator" role="status" aria-label="Message streaming"></span>'
      : '';

    const showActions = this.role !== 'system';

    this.setHTML(this.shadowRoot, `
      ${styles}
      <div class="message-container">
        <div class="bubble">
          ${contentHTML}
          ${streamingIndicator}
        </div>
        ${timestampDisplay ? `<div class="timestamp">${timestampDisplay}</div>` : ''}
        ${showActions ? `
          <div class="actions">
            <button
              class="copy-btn"
              type="button"
              aria-label="Copy message to clipboard"
              title="Copy message">
              Copy
            </button>
            <button
              class="save-btn"
              type="button"
              aria-label="Save message as markdown"
              title="Save as .md">
              Save
            </button>
          </div>
        ` : ''}
      </div>
    `);

    // Attach event listeners
    if (showActions) {
      const copyBtn = this.$('.copy-btn');
      const saveBtn = this.$('.save-btn');

      if (copyBtn) {
        copyBtn.addEventListener('click', () => this.copyToClipboard());
      }

      if (saveBtn) {
        saveBtn.addEventListener('click', () => this.saveAsMarkdown());
      }
    }
  }

  /**
   * Escape HTML to prevent XSS
   * @private
   * @param {string} text - Text to escape
   * @returns {string} Escaped HTML
   */
  _escapeHTML(text) {
    const div = document.createElement('div');
    div.textContent = text;
    return div.innerHTML;
  }
}

// Register the custom element
customElements.define('terraphim-chat-message', TerraphimChatMessage);
