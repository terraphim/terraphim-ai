/**
 * @fileoverview TerraphimCodeViewer - Code display with syntax highlighting
 * Shows code with line numbers and copy functionality
 */

import { TerraphimElement } from '../base/terraphim-element.js';

/**
 * TerraphimCodeViewer Component
 * Displays code with basic syntax highlighting and copy button
 */
export class TerraphimCodeViewer extends TerraphimElement {
  static get properties() {
    return {
      code: { type: String, default: '' },
      language: { type: String, default: 'javascript' },
      showLineNumbers: { type: Boolean, default: true },
      filename: { type: String, default: '' }
    };
  }

  constructor() {
    super();
    this.attachShadow({ mode: 'open' });
  }

  async handleCopy() {
    try {
      await navigator.clipboard.writeText(this.code);
      const btn = this.$('#copyBtn');
      if (btn) {
        const originalText = btn.textContent;
        btn.textContent = 'Copied!';
        setTimeout(() => {
          btn.textContent = originalText;
        }, 2000);
      }
    } catch (err) {
      console.error('Failed to copy code:', err);
    }
  }

  /**
   * Basic syntax highlighting for JavaScript
   * @param {string} code - Code to highlight
   * @returns {string} HTML with syntax highlighting
   */
  highlightCode(code) {
    if (!code) return '';

    let highlighted = code;

    // Escape HTML
    highlighted = highlighted
      .replace(/&/g, '&amp;')
      .replace(/</g, '&lt;')
      .replace(/>/g, '&gt;');

    // Keywords
    const keywords = [
      'class', 'const', 'let', 'var', 'function', 'return', 'if', 'else',
      'for', 'while', 'do', 'switch', 'case', 'break', 'continue', 'try',
      'catch', 'finally', 'throw', 'new', 'import', 'export', 'from',
      'default', 'extends', 'static', 'async', 'await', 'this', 'super'
    ];

    keywords.forEach(keyword => {
      const regex = new RegExp(`\\b(${keyword})\\b`, 'g');
      highlighted = highlighted.replace(regex, '<span class="keyword">$1</span>');
    });

    // Strings
    highlighted = highlighted.replace(/(['"`])((?:\\.|(?!\1).)*?)\1/g, '<span class="string">$1$2$1</span>');

    // Comments
    highlighted = highlighted.replace(/\/\/(.*?)$/gm, '<span class="comment">//$1</span>');
    highlighted = highlighted.replace(/\/\*([\s\S]*?)\*\//g, '<span class="comment">/*$1*/</span>');

    // Numbers
    highlighted = highlighted.replace(/\b(\d+)\b/g, '<span class="number">$1</span>');

    // Functions
    highlighted = highlighted.replace(/\b([a-zA-Z_$][a-zA-Z0-9_$]*)\s*\(/g, '<span class="function">$1</span>(');

    return highlighted;
  }

  render() {
    const lines = this.code.split('\n');
    const highlightedCode = this.highlightCode(this.code);
    const highlightedLines = highlightedCode.split('\n');

    this.setHTML(this.shadowRoot, `
      <style>
        :host {
          display: block;
          width: 100%;
        }

        .code-viewer {
          background: var(--color-bg-secondary, #f5f5f5);
          border: 1px solid var(--color-border, #e0e0e0);
          border-radius: 6px;
          overflow: hidden;
        }

        .code-header {
          display: flex;
          align-items: center;
          justify-content: space-between;
          padding: 8px 12px;
          background: var(--color-bg, #ffffff);
          border-bottom: 1px solid var(--color-border, #e0e0e0);
        }

        .code-filename {
          font-size: 12px;
          font-family: 'Consolas', 'Monaco', monospace;
          color: var(--color-text-secondary, #666);
        }

        .code-actions {
          display: flex;
          gap: 8px;
        }

        .copy-btn {
          padding: 4px 12px;
          background: transparent;
          border: 1px solid var(--color-border, #e0e0e0);
          border-radius: 4px;
          color: var(--color-text, #333);
          font-size: 12px;
          cursor: pointer;
          transition: all 0.2s;
        }

        .copy-btn:hover {
          background: var(--color-primary, #3498db);
          color: white;
          border-color: var(--color-primary, #3498db);
        }

        .code-container {
          display: flex;
          overflow-x: auto;
          max-height: 600px;
          overflow-y: auto;
        }

        .line-numbers {
          padding: 12px 8px;
          background: rgba(0, 0, 0, 0.05);
          color: var(--color-text-secondary, #999);
          font-family: 'Consolas', 'Monaco', monospace;
          font-size: 13px;
          line-height: 1.6;
          text-align: right;
          user-select: none;
          border-right: 1px solid var(--color-border, #e0e0e0);
          display: ${this.showLineNumbers ? 'block' : 'none'};
        }

        .line-number {
          display: block;
        }

        .code-content {
          flex: 1;
          padding: 12px;
          font-family: 'Consolas', 'Monaco', monospace;
          font-size: 13px;
          line-height: 1.6;
          color: var(--color-text, #333);
          overflow-x: auto;
        }

        .code-line {
          display: block;
          white-space: pre;
        }

        /* Syntax highlighting */
        .keyword {
          color: #0000ff;
          font-weight: bold;
        }

        .string {
          color: #a31515;
        }

        .comment {
          color: #008000;
          font-style: italic;
        }

        .number {
          color: #098658;
        }

        .function {
          color: #795e26;
        }

        /* Dark theme overrides */
        :host([theme="dark"]) .keyword {
          color: #569cd6;
        }

        :host([theme="dark"]) .string {
          color: #ce9178;
        }

        :host([theme="dark"]) .comment {
          color: #6a9955;
        }

        :host([theme="dark"]) .number {
          color: #b5cea8;
        }

        :host([theme="dark"]) .function {
          color: #dcdcaa;
        }
      </style>

      <div class="code-viewer">
        ${this.filename || this.language ? `
          <div class="code-header">
            <div class="code-filename">
              ${this.filename || `${this.language} code`}
            </div>
            <div class="code-actions">
              <button class="copy-btn" id="copyBtn">Copy</button>
            </div>
          </div>
        ` : ''}

        <div class="code-container">
          ${this.showLineNumbers ? `
            <div class="line-numbers">
              ${lines.map((_, i) => `
                <span class="line-number">${i + 1}</span>
              `).join('')}
            </div>
          ` : ''}

          <div class="code-content">
            ${highlightedLines.map(line => `
              <span class="code-line">${line || ' '}</span>
            `).join('')}
          </div>
        </div>
      </div>
    `);

    // Add event listener
    const copyBtn = this.$('#copyBtn');
    if (copyBtn) {
      copyBtn.addEventListener('click', () => this.handleCopy());
    }
  }
}

customElements.define('terraphim-code-viewer', TerraphimCodeViewer);
