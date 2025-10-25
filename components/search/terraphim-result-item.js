/**
 * @fileoverview TerraphimResultItem - Individual search result display component
 * Displays a single search result with title, description, URL, tags, and AI summarization
 * Supports real-time SSE summarization updates, term highlighting, and action buttons
 */

import { TerraphimElement } from '../base/terraphim-element.js';

/**
 * @typedef {Object} SearchResultDocument
 * @property {string} id - Document ID
 * @property {string} url - Document URL
 * @property {string} title - Document title
 * @property {string} body - Document content
 * @property {string} [description] - Document description
 * @property {string[]} [tags] - Document tags
 * @property {number} [rank] - Relevance score
 * @property {string} [summarization] - AI-generated summary
 */

/**
 * TerraphimResultItem - Display a single search result as a card
 *
 * @element terraphim-result-item
 *
 * @fires {CustomEvent} result-clicked - When result card is clicked
 * @fires {CustomEvent} copy-url - When copy URL button is clicked
 * @fires {CustomEvent} view-details - When view details button is clicked
 * @fires {CustomEvent} tag-clicked - When a tag is clicked
 *
 * @attr {boolean} expanded - Whether description is expanded
 * @attr {string} summarization-status - Status: 'none', 'pending', 'in-progress', 'complete'
 *
 * @example
 * <terraphim-result-item></terraphim-result-item>
 *
 * <script>
 *   const item = document.querySelector('terraphim-result-item');
 *   item.result = {
 *     id: 'doc-1',
 *     title: 'Rust Async Programming',
 *     description: 'Guide to async programming in Rust',
 *     url: 'https://example.com/rust-async',
 *     tags: ['rust', 'async', 'programming'],
 *     rank: 95
 *   };
 *   item.query = 'rust async';
 * </script>
 */
export class TerraphimResultItem extends TerraphimElement {
  static get observedAttributes() {
    return ['expanded', 'summarization-status'];
  }

  static get properties() {
    return {
      expanded: { type: Boolean, reflect: true, default: false },
      summarizationStatus: { type: String, reflect: true, default: 'none' },
    };
  }

  constructor() {
    super();

    /**
     * Result document object
     * @type {SearchResultDocument|null}
     */
    this._result = null;

    /**
     * Search query for term highlighting
     * @type {string}
     */
    this._query = '';

    /**
     * Current summary text (from SSE)
     * @type {string}
     */
    this._summary = '';

    /**
     * Description character limit before truncation
     * @type {number}
     */
    this._descriptionLimit = 200;

    this.attachShadow({ mode: 'open' });
  }

  /**
   * Get result object
   * @returns {SearchResultDocument|null}
   */
  get result() {
    return this._result;
  }

  /**
   * Set result object and trigger re-render
   * @param {SearchResultDocument} value
   */
  set result(value) {
    this._result = value;

    // Initialize summarization status based on result
    if (value && value.summarization) {
      this.summarizationStatus = 'complete';
      this._summary = value.summarization;
    } else {
      this.summarizationStatus = 'none';
      this._summary = '';
    }

    this.requestUpdate();
  }

  /**
   * Get search query
   * @returns {string}
   */
  get query() {
    return this._query;
  }

  /**
   * Set search query and trigger re-render
   * @param {string} value
   */
  set query(value) {
    this._query = value;
    this.requestUpdate();
  }

  /**
   * Get current summary
   * @returns {string}
   */
  get summary() {
    return this._summary;
  }

  /**
   * Update summary text (called from SSE stream)
   * @param {string} summary - New summary text
   */
  updateSummary(summary) {
    this._summary = summary;
    this.summarizationStatus = 'complete';
    this.requestUpdate();
  }

  /**
   * Set summarization status
   * @param {string} status - Status: 'none', 'pending', 'in-progress', 'complete'
   */
  setSummarizationStatus(status) {
    this.summarizationStatus = status;
    this.requestUpdate();
  }

  /**
   * Highlight search terms in text
   * @param {string[]} terms - Terms to highlight
   */
  highlightTerms(terms) {
    // Store terms for highlighting during render
    this._highlightTerms = terms;
    this.requestUpdate();
  }

  /**
   * Toggle expanded state
   */
  toggleExpanded() {
    this.expanded = !this.expanded;
  }

  /**
   * Copy URL to clipboard
   */
  async copyUrl() {
    if (!this._result || !this._result.url) return;

    try {
      await navigator.clipboard.writeText(this._result.url);
      this.emit('copy-url', { url: this._result.url, success: true });

      // Show temporary feedback
      this._showCopyFeedback();
    } catch (error) {
      console.error('Failed to copy URL:', error);
      this.emit('copy-url', { url: this._result.url, success: false, error });
    }
  }

  /**
   * Show temporary copy feedback
   * @private
   */
  _showCopyFeedback() {
    const copyBtn = this.$('.copy-btn');
    if (!copyBtn) return;

    copyBtn.textContent = 'Copied!';
    copyBtn.classList.add('success');

    setTimeout(() => {
      copyBtn.textContent = 'Copy URL';
      copyBtn.classList.remove('success');
    }, 2000);
  }

  /**
   * Render the component
   */
  render() {
    if (!this._result) {
      this.setHTML(this.shadowRoot, '<div class="empty">No result to display</div>');
      return;
    }

    const { title, description, url, tags, rank, body } = this._result;
    const hasDescription = description && description.trim().length > 0;
    const isTruncated = hasDescription && description.length > this._descriptionLimit;
    const displayDescription = hasDescription
      ? (this.expanded || !isTruncated
          ? description
          : description.slice(0, this._descriptionLimit) + '...')
      : 'No description available';

    this.setHTML(this.shadowRoot, `
      <style>
        :host {
          display: block;
          width: 100%;
        }

        .result-card {
          background: white;
          border: 1px solid #dbdbdb;
          border-radius: 6px;
          padding: 1.5rem;
          margin-bottom: 1rem;
          transition: box-shadow 0.2s ease, transform 0.2s ease;
          cursor: pointer;
        }

        .result-card:hover {
          box-shadow: 0 2px 8px rgba(10, 10, 10, 0.1);
          transform: translateY(-1px);
        }

        .result-header {
          display: flex;
          justify-content: space-between;
          align-items: flex-start;
          gap: 1rem;
          margin-bottom: 0.75rem;
        }

        .title-section {
          flex: 1;
        }

        .result-title {
          font-size: 1.3rem;
          font-weight: 600;
          color: #363636;
          margin: 0 0 0.5rem 0;
          line-height: 1.3;
          cursor: pointer;
          transition: color 0.2s ease;
        }

        .result-title:hover {
          color: #3273dc;
          text-decoration: underline;
        }

        .metadata {
          display: flex;
          flex-wrap: wrap;
          gap: 0.5rem;
          align-items: center;
          margin-bottom: 0.75rem;
        }

        .rank-badge {
          display: inline-flex;
          align-items: center;
          padding: 0.25rem 0.75rem;
          background: #f5f5f5;
          border: 1px solid #dbdbdb;
          border-radius: 12px;
          font-size: 0.75rem;
          font-weight: 600;
          color: #666;
        }

        .url-link {
          font-size: 0.875rem;
          color: #3273dc;
          text-decoration: none;
          max-width: 400px;
          overflow: hidden;
          text-overflow: ellipsis;
          white-space: nowrap;
        }

        .url-link:hover {
          text-decoration: underline;
        }

        .tags-container {
          display: flex;
          flex-wrap: wrap;
          gap: 0.375rem;
          margin-bottom: 0.75rem;
        }

        .tag {
          display: inline-flex;
          align-items: center;
          padding: 0.25rem 0.625rem;
          background: #3273dc;
          color: white;
          border-radius: 12px;
          font-size: 0.75rem;
          font-weight: 500;
          cursor: pointer;
          transition: background-color 0.2s ease, transform 0.15s ease;
        }

        .tag:hover {
          background: #2366d1;
          transform: scale(1.05);
        }

        .description-section {
          margin-bottom: 0.75rem;
        }

        .description-label {
          font-weight: 600;
          color: #666;
          font-size: 0.875rem;
          margin-bottom: 0.25rem;
        }

        .description-content {
          color: #4a4a4a;
          line-height: 1.6;
          font-size: 0.95rem;
        }

        .description-content.no-description {
          color: #999;
          font-style: italic;
        }

        .highlight {
          background-color: #ffeb3b;
          padding: 0.1rem 0.2rem;
          border-radius: 2px;
          font-weight: 600;
        }

        .expand-btn {
          display: ${isTruncated ? 'inline-block' : 'none'};
          background: none;
          border: none;
          color: #3273dc;
          cursor: pointer;
          font-size: 0.875rem;
          font-weight: 600;
          padding: 0.25rem 0;
          margin-top: 0.25rem;
        }

        .expand-btn:hover {
          text-decoration: underline;
        }

        /* Summarization Section */
        .summary-section {
          margin-top: 0.75rem;
        }

        .summary-trigger {
          display: ${this.summarizationStatus === 'none' ? 'inline-flex' : 'none'};
          align-items: center;
          gap: 0.5rem;
          padding: 0.5rem 1rem;
          background: #f0f8ff;
          border: 1px solid #3273dc;
          border-radius: 4px;
          color: #3273dc;
          font-size: 0.875rem;
          font-weight: 600;
          cursor: pointer;
          transition: background-color 0.2s ease;
        }

        .summary-trigger:hover {
          background: #e3f2fd;
        }

        .summary-loading {
          display: ${this.summarizationStatus === 'pending' || this.summarizationStatus === 'in-progress' ? 'flex' : 'none'};
          align-items: center;
          gap: 0.5rem;
          padding: 0.75rem;
          background: #f8f9fa;
          border-left: 4px solid #3273dc;
          border-radius: 4px;
          color: #3273dc;
          font-size: 0.875rem;
        }

        .spinner {
          width: 1rem;
          height: 1rem;
          border: 2px solid #f3f3f3;
          border-top: 2px solid #3273dc;
          border-radius: 50%;
          animation: spin 0.8s linear infinite;
        }

        @keyframes spin {
          0% { transform: rotate(0deg); }
          100% { transform: rotate(360deg); }
        }

        .summary-display {
          display: ${this.summarizationStatus === 'complete' && this._summary ? 'block' : 'none'};
          margin-top: 0.75rem;
          padding: 0.75rem;
          background: #f8f9fa;
          border-left: 4px solid #3273dc;
          border-radius: 4px;
        }

        .summary-header {
          display: flex;
          align-items: center;
          gap: 0.5rem;
          margin-bottom: 0.5rem;
          font-size: 0.875rem;
          font-weight: 600;
          color: #3273dc;
        }

        .summary-content {
          color: #4a4a4a;
          line-height: 1.5;
          font-size: 0.9rem;
        }

        /* Actions */
        .actions {
          display: flex;
          gap: 0.75rem;
          margin-top: 1rem;
          padding-top: 1rem;
          border-top: 1px solid #f5f5f5;
        }

        .action-btn {
          display: inline-flex;
          align-items: center;
          gap: 0.375rem;
          padding: 0.5rem 1rem;
          background: white;
          border: 1px solid #dbdbdb;
          border-radius: 4px;
          color: #4a4a4a;
          font-size: 0.875rem;
          cursor: pointer;
          transition: all 0.2s ease;
        }

        .action-btn:hover {
          background: #f5f5f5;
          border-color: #b5b5b5;
        }

        .action-btn.primary {
          background: #3273dc;
          color: white;
          border-color: #3273dc;
        }

        .action-btn.primary:hover {
          background: #2366d1;
          border-color: #2366d1;
        }

        .action-btn.success {
          background: #48c774;
          color: white;
          border-color: #48c774;
        }

        .action-btn-icon {
          width: 1rem;
          height: 1rem;
        }

        .empty {
          padding: 1rem;
          text-align: center;
          color: #999;
        }

        /* Status indicator */
        .status-indicator {
          display: flex;
          align-items: center;
          gap: 0.5rem;
          font-size: 0.75rem;
          color: #999;
        }

        .status-dot {
          width: 8px;
          height: 8px;
          border-radius: 50%;
          background-color: #dbdbdb;
        }

        .status-dot.pending {
          background-color: #ffdd57;
          animation: pulse 1.5s ease-in-out infinite;
        }

        .status-dot.in-progress {
          background-color: #3273dc;
          animation: pulse 1.5s ease-in-out infinite;
        }

        .status-dot.complete {
          background-color: #48c774;
        }

        @keyframes pulse {
          0%, 100% { opacity: 1; }
          50% { opacity: 0.5; }
        }
      </style>

      <div class="result-card">
        <div class="result-header">
          <div class="title-section">
            <h2 class="result-title">${this._escapeHtml(title)}</h2>

            <div class="metadata">
              ${rank !== undefined ? `
                <span class="rank-badge">Rank ${rank}</span>
              ` : ''}

              ${url ? `
                <a href="${this._escapeHtml(url)}" class="url-link" target="_blank" rel="noopener noreferrer">${this._escapeHtml(url)}</a>
              ` : ''}
            </div>

            ${tags && tags.length > 0 ? `
              <div class="tags-container">
                ${tags.map(tag => `
                  <span class="tag" data-tag="${this._escapeHtml(tag)}">${this._escapeHtml(tag)}</span>
                `).join('')}
              </div>
            ` : ''}
          </div>

          ${this._renderStatusIndicator()}
        </div>

        <div class="description-section">
          <div class="description-label">Description:</div>
          <div class="description-content ${!hasDescription ? 'no-description' : ''}">
            ${this._highlightText(displayDescription)}
          </div>
          ${isTruncated ? `
            <button class="expand-btn">
              ${this.expanded ? 'Show less' : 'Show more'}
            </button>
          ` : ''}
        </div>

        ${this._renderSummarization()}

        <div class="actions">
          <button class="action-btn primary view-details-btn">
            <svg class="action-btn-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
              <path d="M1 12s4-8 11-8 11 8 11 8-4 8-11 8-11-8-11-8z"/>
              <circle cx="12" cy="12" r="3"/>
            </svg>
            View Details
          </button>

          ${url ? `
            <button class="action-btn copy-btn">
              <svg class="action-btn-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                <rect x="9" y="9" width="13" height="13" rx="2" ry="2"/>
                <path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"/>
              </svg>
              Copy URL
            </button>
          ` : ''}
        </div>
      </div>
    `);

    this._attachEventListeners();
  }

  /**
   * Render status indicator
   * @private
   * @returns {string}
   */
  _renderStatusIndicator() {
    if (this.summarizationStatus === 'none') return '';

    const statusText = {
      'pending': 'Summary pending',
      'in-progress': 'Generating summary...',
      'complete': 'Summary ready'
    }[this.summarizationStatus] || '';

    return `
      <div class="status-indicator">
        <span class="status-dot ${this.summarizationStatus}"></span>
        <span>${statusText}</span>
      </div>
    `;
  }

  /**
   * Render summarization section
   * @private
   * @returns {string}
   */
  _renderSummarization() {
    return `
      <div class="summary-section">
        <button class="summary-trigger">
          <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <path d="M21 15a2 2 0 0 1-2 2H7l-4 4V5a2 2 0 0 1 2-2h14a2 2 0 0 1 2 2z"/>
          </svg>
          Generate AI Summary
        </button>

        <div class="summary-loading">
          <div class="spinner"></div>
          <span>Generating AI summary...</span>
        </div>

        <div class="summary-display">
          <div class="summary-header">
            <svg width="16" height="16" viewBox="0 0 24 24" fill="currentColor">
              <path d="M9.5 2a1 1 0 0 1 1-1h3a1 1 0 0 1 1 1v1H18a2 2 0 0 1 2 2v14a2 2 0 0 1-2 2H6a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h3.5V2zm1 1v1h3V3h-3z"/>
            </svg>
            AI Summary
          </div>
          <div class="summary-content">
            ${this._escapeHtml(this._summary)}
          </div>
        </div>
      </div>
    `;
  }

  /**
   * Highlight search terms in text
   * @private
   * @param {string} text - Text to highlight
   * @returns {string} HTML with highlighted terms
   */
  _highlightText(text) {
    if (!this._query || this._query.trim().length === 0) {
      return this._escapeHtml(text);
    }

    // Extract terms from query (split by AND/OR)
    const terms = this._query
      .split(/\s+(?:AND|OR)\s+/i)
      .map(t => t.trim())
      .filter(t => t.length > 0);

    let result = this._escapeHtml(text);

    // Highlight each term
    terms.forEach(term => {
      const regex = new RegExp(`(${this._escapeRegex(term)})`, 'gi');
      result = result.replace(regex, '<span class="highlight">$1</span>');
    });

    return result;
  }

  /**
   * Escape regex special characters
   * @private
   * @param {string} str - String to escape
   * @returns {string} Escaped string
   */
  _escapeRegex(str) {
    return str.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
  }

  /**
   * Attach event listeners to rendered elements
   * @private
   */
  _attachEventListeners() {
    // Title click
    const title = this.$('.result-title');
    if (title) {
      title.addEventListener('click', (e) => {
        e.stopPropagation();
        this.emit('result-clicked', { result: this._result });
      });
    }

    // Card click
    const card = this.$('.result-card');
    if (card) {
      card.addEventListener('click', () => {
        this.emit('result-clicked', { result: this._result });
      });
    }

    // Tag clicks
    this.$$('.tag').forEach(tag => {
      tag.addEventListener('click', (e) => {
        e.stopPropagation();
        const tagValue = tag.getAttribute('data-tag');
        this.emit('tag-clicked', { tag: tagValue, result: this._result });
      });
    });

    // Expand/collapse description
    const expandBtn = this.$('.expand-btn');
    if (expandBtn) {
      expandBtn.addEventListener('click', (e) => {
        e.stopPropagation();
        this.toggleExpanded();
      });
    }

    // Summary trigger button
    const summaryTrigger = this.$('.summary-trigger');
    if (summaryTrigger) {
      summaryTrigger.addEventListener('click', (e) => {
        e.stopPropagation();
        this.setSummarizationStatus('pending');
        this.emit('request-summary', { result: this._result });
      });
    }

    // View details button
    const viewDetailsBtn = this.$('.view-details-btn');
    if (viewDetailsBtn) {
      viewDetailsBtn.addEventListener('click', (e) => {
        e.stopPropagation();
        this.emit('view-details', { result: this._result });
      });
    }

    // Copy URL button
    const copyBtn = this.$('.copy-btn');
    if (copyBtn) {
      copyBtn.addEventListener('click', (e) => {
        e.stopPropagation();
        this.copyUrl();
      });
    }
  }

  /**
   * Escape HTML special characters
   * @private
   * @param {string} str - String to escape
   * @returns {string} Escaped string
   */
  _escapeHtml(str) {
    if (!str) return '';
    const div = document.createElement('div');
    div.textContent = str;
    return div.innerHTML;
  }
}

// Register the custom element
if (!customElements.get('terraphim-result-item')) {
  customElements.define('terraphim-result-item', TerraphimResultItem);
}
