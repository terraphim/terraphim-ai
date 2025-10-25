/**
 * @fileoverview TerraphimSearchResults - Container for search result items
 * Manages result display, SSE streaming for summarization, loading/empty/error states
 * Provides progressive result updates and virtual scrolling support
 */

import { TerraphimElement } from '../base/terraphim-element.js';
import { SearchAPI } from './search-api.js';
import './terraphim-result-item.js';

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
 * TerraphimSearchResults - Container for displaying search results
 *
 * @element terraphim-search-results
 *
 * @fires {CustomEvent} results-loaded - When all results are loaded
 * @fires {CustomEvent} result-clicked - Bubbled from result-item
 * @fires {CustomEvent} load-more - When scroll reaches bottom
 * @fires {CustomEvent} sse-connected - When SSE connection is established
 * @fires {CustomEvent} sse-disconnected - When SSE connection is closed
 * @fires {CustomEvent} sse-error - When SSE connection fails
 *
 * @attr {boolean} loading - Loading state
 * @attr {string} query - Current search query
 * @attr {string} role - Role name for SSE context
 * @attr {string} error - Error message if any
 *
 * @example
 * <terraphim-search-results
 *   query="rust async"
 *   role="engineer"
 * ></terraphim-search-results>
 *
 * <script>
 *   const results = document.querySelector('terraphim-search-results');
 *   results.setResults([
 *     { id: '1', title: 'Document 1', description: '...', url: '...' }
 *   ]);
 *
 *   results.addEventListener('result-clicked', (e) => {
 *     console.log('Result clicked:', e.detail.result);
 *   });
 * </script>
 */
export class TerraphimSearchResults extends TerraphimElement {
  static get observedAttributes() {
    return ['loading', 'query', 'role', 'error'];
  }

  static get properties() {
    return {
      loading: { type: Boolean, reflect: true, default: false },
      query: { type: String, reflect: true, default: '' },
      role: { type: String, reflect: true, default: '' },
      error: { type: String, reflect: true, default: '' },
    };
  }

  constructor() {
    super();

    /**
     * Array of result documents
     * @type {SearchResultDocument[]}
     */
    this._results = [];

    /**
     * SearchAPI instance
     * @type {SearchAPI}
     */
    this._api = new SearchAPI();

    /**
     * SSE stream stop function
     * @type {Function|null}
     */
    this._stopSSE = null;

    /**
     * Intersection observer for lazy loading
     * @type {IntersectionObserver|null}
     */
    this._intersectionObserver = null;

    /**
     * SSE connection status
     * @type {string}
     */
    this._sseStatus = 'disconnected'; // 'disconnected', 'connecting', 'connected', 'error'

    this.attachShadow({ mode: 'open' });
  }

  /**
   * Get results array
   * @returns {SearchResultDocument[]}
   */
  get results() {
    return this._results;
  }

  /**
   * Set results array and trigger re-render
   * @param {SearchResultDocument[]} results - Array of result documents
   */
  setResults(results) {
    if (!Array.isArray(results)) {
      console.warn('setResults expects an array');
      return;
    }

    this._results = results;
    this.loading = false;
    this.error = '';
    this.requestUpdate();

    this.emit('results-loaded', { count: results.length, results });
  }

  /**
   * Add a single result (for streaming)
   * @param {SearchResultDocument} result - Result document to add
   */
  addResult(result) {
    this._results = [...this._results, result];
    this.requestUpdate();
  }

  /**
   * Clear all results
   */
  clearResults() {
    this._results = [];
    this.error = '';
    this.requestUpdate();
  }

  /**
   * Start SSE summarization stream
   * @param {string} query - Search query
   * @param {string} role - Role name
   */
  startSummarization(query, role) {
    // Stop any existing stream
    this.stopSummarization();

    this._sseStatus = 'connecting';
    this.requestUpdate();

    console.log('Starting summarization stream for query:', query, 'role:', role);

    // Start SSE stream
    this._stopSSE = this._api.startSummarizationStream(
      (taskId, summary) => {
        console.log('SSE update received:', { taskId, summary: summary.substring(0, 50) + '...' });
        this._handleSummaryUpdate(taskId, summary);
      },
      (error) => {
        console.error('SSE error:', error);
        this._sseStatus = 'error';
        this.requestUpdate();
        this.emit('sse-error', { error });
      }
    );

    this._sseStatus = 'connected';
    this.requestUpdate();
    this.emit('sse-connected');
  }

  /**
   * Stop SSE summarization stream
   */
  stopSummarization() {
    if (this._stopSSE) {
      console.log('Stopping summarization stream');
      this._stopSSE();
      this._stopSSE = null;
      this._sseStatus = 'disconnected';
      this.requestUpdate();
      this.emit('sse-disconnected');
    }
  }

  /**
   * Handle SSE summary update
   * @private
   * @param {string} taskId - Task ID
   * @param {string} summary - Summary text
   */
  _handleSummaryUpdate(taskId, summary) {
    // Find result items and update them
    // In real implementation, we'd match by document ID or task ID
    // For now, we'll update the first result without a summary
    const resultItems = this.$$('terraphim-result-item');

    // Simple strategy: update results without summaries in order
    for (const item of resultItems) {
      if (item.summarizationStatus === 'pending' || item.summarizationStatus === 'in-progress') {
        item.updateSummary(summary);
        break;
      }
    }

    // Also update the internal results array
    const index = this._results.findIndex(r => !r.summarization);
    if (index !== -1) {
      this._results[index] = {
        ...this._results[index],
        summarization: summary
      };
    }
  }

  /**
   * Lifecycle hook: setup after connection
   */
  onConnected() {
    // Set up intersection observer for lazy loading
    this._setupIntersectionObserver();
  }

  /**
   * Lifecycle hook: cleanup before disconnection
   */
  onDisconnected() {
    // Stop SSE stream
    this.stopSummarization();

    // Cleanup API
    this._api.cleanup();

    // Cleanup intersection observer
    if (this._intersectionObserver) {
      this._intersectionObserver.disconnect();
      this._intersectionObserver = null;
    }
  }

  /**
   * Set up intersection observer for load-more functionality
   * @private
   */
  _setupIntersectionObserver() {
    if (typeof IntersectionObserver === 'undefined') return;

    this._intersectionObserver = new IntersectionObserver(
      (entries) => {
        entries.forEach(entry => {
          if (entry.isIntersecting) {
            this.emit('load-more');
          }
        });
      },
      {
        root: null,
        rootMargin: '100px',
        threshold: 0.1
      }
    );
  }

  /**
   * Render the component
   */
  render() {
    this.setHTML(this.shadowRoot, `
      <style>
        :host {
          display: block;
          width: 100%;
        }

        .results-container {
          width: 100%;
          max-width: 1200px;
          margin: 0 auto;
        }

        /* Loading state */
        .loading-state {
          display: ${this.loading ? 'flex' : 'none'};
          flex-direction: column;
          align-items: center;
          justify-content: center;
          padding: 3rem 1rem;
          gap: 1rem;
        }

        .loading-spinner {
          width: 3rem;
          height: 3rem;
          border: 4px solid #f3f3f3;
          border-top: 4px solid #3273dc;
          border-radius: 50%;
          animation: spin 1s linear infinite;
        }

        @keyframes spin {
          0% { transform: rotate(0deg); }
          100% { transform: rotate(360deg); }
        }

        .loading-text {
          color: #666;
          font-size: 1rem;
        }

        /* Skeleton loading */
        .skeleton-container {
          display: ${this.loading ? 'block' : 'none'};
        }

        .skeleton-card {
          background: white;
          border: 1px solid #dbdbdb;
          border-radius: 6px;
          padding: 1.5rem;
          margin-bottom: 1rem;
        }

        .skeleton-line {
          height: 1rem;
          background: linear-gradient(90deg, #f0f0f0 25%, #e0e0e0 50%, #f0f0f0 75%);
          background-size: 200% 100%;
          animation: loading 1.5s ease-in-out infinite;
          border-radius: 4px;
          margin-bottom: 0.75rem;
        }

        .skeleton-line.title {
          width: 70%;
          height: 1.5rem;
        }

        .skeleton-line.subtitle {
          width: 40%;
          height: 0.875rem;
        }

        .skeleton-line.text {
          width: 100%;
        }

        .skeleton-line.short {
          width: 60%;
        }

        @keyframes loading {
          0% { background-position: 200% 0; }
          100% { background-position: -200% 0; }
        }

        /* Error state */
        .error-state {
          display: ${this.error && !this.loading ? 'flex' : 'none'};
          flex-direction: column;
          align-items: center;
          justify-content: center;
          padding: 3rem 1rem;
          gap: 1rem;
          background: #ffeded;
          border: 1px solid #ff6b6b;
          border-radius: 6px;
          margin: 1rem 0;
        }

        .error-icon {
          width: 3rem;
          height: 3rem;
          color: #ff6b6b;
        }

        .error-title {
          color: #d63031;
          font-size: 1.25rem;
          font-weight: 600;
          margin: 0;
        }

        .error-message {
          color: #666;
          text-align: center;
          max-width: 500px;
        }

        .retry-btn {
          padding: 0.75rem 1.5rem;
          background: #3273dc;
          color: white;
          border: none;
          border-radius: 4px;
          font-size: 1rem;
          font-weight: 600;
          cursor: pointer;
          transition: background-color 0.2s ease;
        }

        .retry-btn:hover {
          background: #2366d1;
        }

        /* Empty state */
        .empty-state {
          display: ${!this.loading && !this.error && this._results.length === 0 ? 'flex' : 'none'};
          flex-direction: column;
          align-items: center;
          justify-content: center;
          padding: 4rem 1rem;
          gap: 1.5rem;
        }

        .empty-icon {
          width: 5rem;
          height: 5rem;
          opacity: 0.3;
        }

        .empty-title {
          color: #666;
          font-size: 1.5rem;
          font-weight: 600;
          margin: 0;
        }

        .empty-message {
          color: #999;
          text-align: center;
          max-width: 500px;
          line-height: 1.6;
        }

        /* Results grid */
        .results-grid {
          display: ${!this.loading && !this.error && this._results.length > 0 ? 'block' : 'none'};
        }

        /* SSE Status indicator */
        .sse-status {
          display: flex;
          align-items: center;
          gap: 0.5rem;
          padding: 0.5rem 1rem;
          background: #f8f9fa;
          border-radius: 4px;
          margin-bottom: 1rem;
          font-size: 0.875rem;
          color: #666;
        }

        .sse-dot {
          width: 8px;
          height: 8px;
          border-radius: 50%;
          background-color: #dbdbdb;
        }

        .sse-dot.connected {
          background-color: #48c774;
        }

        .sse-dot.connecting {
          background-color: #ffdd57;
          animation: pulse 1.5s ease-in-out infinite;
        }

        .sse-dot.error {
          background-color: #ff6b6b;
        }

        @keyframes pulse {
          0%, 100% { opacity: 1; }
          50% { opacity: 0.5; }
        }

        /* Load more sentinel */
        .load-more-sentinel {
          height: 1px;
          margin: 2rem 0;
        }

        /* Result count */
        .results-header {
          display: ${!this.loading && !this.error && this._results.length > 0 ? 'flex' : 'none'};
          justify-content: space-between;
          align-items: center;
          margin-bottom: 1rem;
          padding: 0.75rem 0;
        }

        .result-count {
          color: #666;
          font-size: 0.875rem;
        }

        .result-count strong {
          color: #363636;
          font-weight: 600;
        }
      </style>

      <div class="results-container">
        <!-- Loading State -->
        <div class="loading-state">
          <div class="loading-spinner"></div>
          <div class="loading-text">Searching...</div>
        </div>

        <!-- Skeleton Loading -->
        <div class="skeleton-container">
          ${this._renderSkeletons(3)}
        </div>

        <!-- Error State -->
        <div class="error-state">
          <svg class="error-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <circle cx="12" cy="12" r="10"/>
            <line x1="12" y1="8" x2="12" y2="12"/>
            <line x1="12" y1="16" x2="12.01" y2="16"/>
          </svg>
          <h2 class="error-title">Search Error</h2>
          <p class="error-message">${this._escapeHtml(this.error)}</p>
          <button class="retry-btn">Retry Search</button>
        </div>

        <!-- Empty State -->
        <div class="empty-state">
          <svg class="empty-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <circle cx="11" cy="11" r="8"/>
            <path d="M21 21l-4.35-4.35"/>
          </svg>
          <h2 class="empty-title">No Results Found</h2>
          <p class="empty-message">
            ${this.query
              ? `No results found for "${this._escapeHtml(this.query)}". Try different search terms or broaden your query.`
              : 'Enter a search query to find documents.'}
          </p>
        </div>

        <!-- Results Grid -->
        <div class="results-grid">
          <!-- SSE Status -->
          ${this._renderSSEStatus()}

          <!-- Results Header -->
          <div class="results-header">
            <div class="result-count">
              Found <strong>${this._results.length}</strong> result${this._results.length === 1 ? '' : 's'}
              ${this.query ? ` for "<strong>${this._escapeHtml(this.query)}</strong>"` : ''}
            </div>
          </div>

          <!-- Result Items -->
          ${this._renderResults()}

          <!-- Load More Sentinel -->
          <div class="load-more-sentinel"></div>
        </div>
      </div>
    `);

    this._attachEventListeners();
  }

  /**
   * Render SSE status indicator
   * @private
   * @returns {string}
   */
  _renderSSEStatus() {
    if (this._sseStatus === 'disconnected') return '';

    const statusText = {
      'connecting': 'Connecting to summarization service...',
      'connected': 'Real-time summarization active',
      'error': 'Summarization service unavailable'
    }[this._sseStatus] || '';

    return `
      <div class="sse-status">
        <span class="sse-dot ${this._sseStatus}"></span>
        <span>${statusText}</span>
      </div>
    `;
  }

  /**
   * Render skeleton loading cards
   * @private
   * @param {number} count - Number of skeletons
   * @returns {string}
   */
  _renderSkeletons(count) {
    const skeleton = `
      <div class="skeleton-card">
        <div class="skeleton-line title"></div>
        <div class="skeleton-line subtitle"></div>
        <div class="skeleton-line text"></div>
        <div class="skeleton-line text"></div>
        <div class="skeleton-line short"></div>
      </div>
    `;

    return Array(count).fill(skeleton).join('');
  }

  /**
   * Render result items
   * @private
   * @returns {string}
   */
  _renderResults() {
    if (this._results.length === 0) return '';

    return this._results.map(result => `
      <terraphim-result-item data-result-id="${this._escapeHtml(result.id)}"></terraphim-result-item>
    `).join('');
  }

  /**
   * Attach event listeners to rendered elements
   * @private
   */
  _attachEventListeners() {
    // Set result data on each result-item element
    this._results.forEach((result, index) => {
      const resultItem = this.$$('terraphim-result-item')[index];
      if (resultItem) {
        resultItem.result = result;
        resultItem.query = this.query;

        // Bubble events from result items
        resultItem.addEventListener('result-clicked', (e) => {
          this.emit('result-clicked', e.detail);
        });

        resultItem.addEventListener('copy-url', (e) => {
          this.emit('copy-url', e.detail);
        });

        resultItem.addEventListener('view-details', (e) => {
          this.emit('view-details', e.detail);
        });

        resultItem.addEventListener('tag-clicked', (e) => {
          this.emit('tag-clicked', e.detail);
        });

        resultItem.addEventListener('request-summary', (e) => {
          // Mark as in-progress
          resultItem.setSummarizationStatus('in-progress');
          this.emit('request-summary', e.detail);
        });
      }
    });

    // Retry button
    const retryBtn = this.$('.retry-btn');
    if (retryBtn) {
      retryBtn.addEventListener('click', () => {
        this.error = '';
        this.emit('retry-search');
      });
    }

    // Observe load-more sentinel
    const sentinel = this.$('.load-more-sentinel');
    if (sentinel && this._intersectionObserver) {
      this._intersectionObserver.observe(sentinel);
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
if (!customElements.get('terraphim-search-results')) {
  customElements.define('terraphim-search-results', TerraphimSearchResults);
}
