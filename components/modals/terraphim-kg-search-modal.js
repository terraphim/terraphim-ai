import { TerraphimElement } from '../base/terraphim-element.js';

/**
 * TerraphimKgSearchModal Component
 *
 * Modal for searching the knowledge graph and viewing relationships.
 *
 * @fires kg-search - When search is performed {query, filters}
 * @fires result-select - When a result is selected {result}
 * @fires result-add-context - When result is added to context {result}
 * @fires modal-close - When modal is closed
 *
 * @example
 * ```html
 * <terraphim-kg-search-modal
 *   show-filters
 *   show-graph-view>
 * </terraphim-kg-search-modal>
 * ```
 */
export class TerraphimKgSearchModal extends TerraphimElement {
  static get properties() {
    return {
      isOpen: { type: Boolean, default: false },
      query: { type: String, default: '' },
      results: { type: Array, default: () => [] },
      selectedResult: { type: Object, default: null },
      showFilters: { type: Boolean, reflect: true, default: true },
      showGraphView: { type: Boolean, reflect: true, default: true },
      filterType: { type: String, default: 'all' },
      sortBy: { type: String, default: 'relevance' }, // 'relevance', 'date', 'title'
      isSearching: { type: Boolean, default: false },
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
        width: 95%;
        max-width: 1200px;
        height: 85vh;
        display: flex;
        flex-direction: column;
        animation: slideIn 0.3s ease-out;
        z-index: 1001;
      }

      .modal-header {
        display: flex;
        justify-content: space-between;
        align-items: center;
        padding: var(--spacing-lg);
        border-bottom: 1px solid var(--border-primary);
      }

      .header-content {
        flex: 1;
      }

      .modal-title {
        font-size: var(--font-size-xl);
        font-weight: var(--font-weight-bold);
        margin: 0 0 var(--spacing-sm) 0;
        color: var(--text-primary);
      }

      .search-container {
        display: flex;
        gap: var(--spacing-sm);
        margin-top: var(--spacing-sm);
      }

      .search-input-wrapper {
        flex: 1;
        position: relative;
      }

      .search-input {
        width: 100%;
        padding: var(--spacing-sm) var(--spacing-md);
        padding-left: var(--spacing-xl);
        background: var(--bg-secondary);
        border: 2px solid var(--border-primary);
        border-radius: var(--border-radius-md);
        font-family: var(--font-family-sans);
        font-size: var(--font-size-base);
        color: var(--text-primary);
        transition: var(--transition-base);
      }

      .search-input:focus {
        outline: none;
        border-color: var(--color-primary);
      }

      .search-icon {
        position: absolute;
        left: var(--spacing-sm);
        top: 50%;
        transform: translateY(-50%);
        width: 20px;
        height: 20px;
        fill: var(--text-tertiary);
      }

      .search-btn {
        padding: var(--spacing-sm) var(--spacing-lg);
        background: var(--color-primary);
        color: var(--color-primary-contrast);
        border: none;
        border-radius: var(--border-radius-md);
        cursor: pointer;
        font-size: var(--font-size-sm);
        font-weight: var(--font-weight-semibold);
        transition: var(--transition-base);
        white-space: nowrap;
      }

      .search-btn:hover {
        background: var(--color-primary-dark);
      }

      .search-btn:disabled {
        opacity: 0.5;
        cursor: not-allowed;
      }

      .close-button {
        background: transparent;
        border: none;
        padding: var(--spacing-xs);
        cursor: pointer;
        border-radius: var(--border-radius-sm);
        color: var(--text-secondary);
        transition: var(--transition-base);
        margin-left: var(--spacing-md);
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
        display: flex;
        min-height: 0;
      }

      .sidebar {
        width: 250px;
        border-right: 1px solid var(--border-primary);
        padding: var(--spacing-md);
        overflow-y: auto;
      }

      .filter-section {
        margin-bottom: var(--spacing-lg);
      }

      .filter-title {
        font-size: var(--font-size-sm);
        font-weight: var(--font-weight-semibold);
        color: var(--text-primary);
        margin-bottom: var(--spacing-sm);
        text-transform: uppercase;
        letter-spacing: 0.05em;
      }

      .filter-option {
        display: flex;
        align-items: center;
        gap: var(--spacing-sm);
        padding: var(--spacing-xs) 0;
        cursor: pointer;
        color: var(--text-secondary);
        transition: var(--transition-fast);
      }

      .filter-option:hover {
        color: var(--text-primary);
      }

      .filter-option input[type="radio"] {
        cursor: pointer;
      }

      .results-container {
        flex: 1;
        display: flex;
        flex-direction: column;
        min-width: 0;
      }

      .results-header {
        display: flex;
        justify-content: space-between;
        align-items: center;
        padding: var(--spacing-md);
        border-bottom: 1px solid var(--border-primary);
      }

      .results-count {
        font-size: var(--font-size-sm);
        color: var(--text-secondary);
      }

      .sort-select {
        padding: var(--spacing-xs) var(--spacing-sm);
        background: var(--bg-secondary);
        border: 1px solid var(--border-primary);
        border-radius: var(--border-radius-sm);
        font-size: var(--font-size-xs);
        color: var(--text-primary);
      }

      .results-list {
        flex: 1;
        overflow-y: auto;
        padding: var(--spacing-md);
      }

      .result-item {
        padding: var(--spacing-md);
        margin-bottom: var(--spacing-sm);
        background: var(--bg-secondary);
        border: 1px solid var(--border-primary);
        border-radius: var(--border-radius-md);
        cursor: pointer;
        transition: var(--transition-base);
      }

      .result-item:hover {
        background: var(--bg-hover);
        border-color: var(--border-hover);
      }

      .result-item.selected {
        border-color: var(--color-primary);
        background: var(--bg-hover);
      }

      .result-header {
        display: flex;
        justify-content: space-between;
        align-items: flex-start;
        margin-bottom: var(--spacing-xs);
      }

      .result-title {
        font-size: var(--font-size-base);
        font-weight: var(--font-weight-semibold);
        color: var(--text-primary);
        margin: 0;
      }

      .result-score {
        padding: var(--spacing-xs) var(--spacing-sm);
        background: var(--color-primary);
        color: var(--color-primary-contrast);
        border-radius: var(--border-radius-sm);
        font-size: var(--font-size-xs);
        font-weight: var(--font-weight-bold);
      }

      .result-description {
        font-size: var(--font-size-sm);
        color: var(--text-secondary);
        margin: var(--spacing-xs) 0;
        line-height: 1.5;
      }

      .result-meta {
        display: flex;
        gap: var(--spacing-md);
        font-size: var(--font-size-xs);
        color: var(--text-tertiary);
      }

      .result-actions {
        display: flex;
        gap: var(--spacing-xs);
        margin-top: var(--spacing-sm);
      }

      .result-action-btn {
        padding: var(--spacing-xs) var(--spacing-sm);
        background: transparent;
        border: 1px solid var(--border-primary);
        border-radius: var(--border-radius-sm);
        cursor: pointer;
        font-size: var(--font-size-xs);
        color: var(--text-secondary);
        transition: var(--transition-fast);
      }

      .result-action-btn:hover {
        background: var(--color-primary);
        color: var(--color-primary-contrast);
        border-color: var(--color-primary);
      }

      .empty-state {
        text-align: center;
        padding: var(--spacing-2xl);
        color: var(--text-tertiary);
      }

      .empty-state svg {
        width: 64px;
        height: 64px;
        margin-bottom: var(--spacing-md);
        opacity: 0.3;
      }

      .empty-state-title {
        font-size: var(--font-size-lg);
        font-weight: var(--font-weight-semibold);
        margin: 0 0 var(--spacing-xs) 0;
        color: var(--text-secondary);
      }

      .empty-state-text {
        font-size: var(--font-size-sm);
        margin: 0;
      }

      .loading {
        text-align: center;
        padding: var(--spacing-xl);
        color: var(--text-secondary);
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

      @keyframes spin {
        to { transform: rotate(360deg); }
      }

      .spinner {
        display: inline-block;
        width: 24px;
        height: 24px;
        border: 3px solid var(--border-primary);
        border-top-color: var(--color-primary);
        border-radius: 50%;
        animation: spin 0.6s linear infinite;
      }
    `;
  }

  render() {
    const html = `
      <style>${this.styles()}</style>
      <div class="modal-backdrop" id="backdrop"></div>
      <div class="modal-container" role="dialog" aria-modal="true" aria-labelledby="modal-title">
        <div class="modal-header">
          <div class="header-content">
            <h2 class="modal-title" id="modal-title">Knowledge Graph Search</h2>
            <div class="search-container">
              <div class="search-input-wrapper">
                <svg class="search-icon" viewBox="0 0 24 24" xmlns="http://www.w3.org/2000/svg">
                  <path d="M9.5,3A6.5,6.5 0 0,1 16,9.5C16,11.11 15.41,12.59 14.44,13.73L14.71,14H15.5L20.5,19L19,20.5L14,15.5V14.71L13.73,14.44C12.59,15.41 11.11,16 9.5,16A6.5,6.5 0 0,1 3,9.5A6.5,6.5 0 0,1 9.5,3M9.5,5C7,5 5,7 5,9.5C5,12 7,14 9.5,14C12,14 14,12 14,9.5C14,7 12,5 9.5,5Z"/>
                </svg>
                <input
                  type="text"
                  class="search-input"
                  id="searchInput"
                  placeholder="Search knowledge graph..."
                  value="${this._escapeHTML(this.query)}">
              </div>
              <button class="search-btn" id="searchBtn" ${this.isSearching ? 'disabled' : ''}>
                ${this.isSearching ? 'Searching...' : 'Search'}
              </button>
            </div>
          </div>
          <button class="close-button" id="closeBtn" aria-label="Close modal">
            <svg viewBox="0 0 24 24" xmlns="http://www.w3.org/2000/svg">
              <path d="M19,6.41L17.59,5L12,10.59L6.41,5L5,6.41L10.59,12L5,17.59L6.41,19L12,13.41L17.59,19L19,17.59L13.41,12L19,6.41Z"/>
            </svg>
          </button>
        </div>
        <div class="modal-body">
          ${this.showFilters ? `
            <aside class="sidebar">
              <div class="filter-section">
                <h3 class="filter-title">Type</h3>
                <label class="filter-option">
                  <input type="radio" name="filterType" value="all" ${this.filterType === 'all' ? 'checked' : ''}>
                  <span>All Types</span>
                </label>
                <label class="filter-option">
                  <input type="radio" name="filterType" value="document" ${this.filterType === 'document' ? 'checked' : ''}>
                  <span>Documents</span>
                </label>
                <label class="filter-option">
                  <input type="radio" name="filterType" value="article" ${this.filterType === 'article' ? 'checked' : ''}>
                  <span>Articles</span>
                </label>
                <label class="filter-option">
                  <input type="radio" name="filterType" value="note" ${this.filterType === 'note' ? 'checked' : ''}>
                  <span>Notes</span>
                </label>
                <label class="filter-option">
                  <input type="radio" name="filterType" value="reference" ${this.filterType === 'reference' ? 'checked' : ''}>
                  <span>References</span>
                </label>
              </div>
            </aside>
          ` : ''}
          <div class="results-container">
            <div class="results-header">
              <span class="results-count">
                ${this.results.length} result${this.results.length !== 1 ? 's' : ''}
              </span>
              <select class="sort-select" id="sortSelect">
                <option value="relevance" ${this.sortBy === 'relevance' ? 'selected' : ''}>Sort by Relevance</option>
                <option value="date" ${this.sortBy === 'date' ? 'selected' : ''}>Sort by Date</option>
                <option value="title" ${this.sortBy === 'title' ? 'selected' : ''}>Sort by Title</option>
              </select>
            </div>
            <div class="results-list">
              ${this.isSearching ? `
                <div class="loading">
                  <div class="spinner"></div>
                  <p>Searching knowledge graph...</p>
                </div>
              ` : this.results.length > 0 ? this._renderResults() : this._renderEmptyState()}
            </div>
          </div>
        </div>
      </div>
    `;

    this.setHTML(this.shadowRoot, html);

    // Attach event listeners
    const backdrop = this.$('#backdrop');
    const closeBtn = this.$('#closeBtn');
    const searchBtn = this.$('#searchBtn');
    const searchInput = this.$('#searchInput');
    const sortSelect = this.$('#sortSelect');
    const filterRadios = this.$$('input[name="filterType"]');

    if (backdrop) backdrop.addEventListener('click', () => this.close());
    if (closeBtn) closeBtn.addEventListener('click', () => this.close());
    if (searchBtn) searchBtn.addEventListener('click', () => this._performSearch());
    if (searchInput) {
      searchInput.addEventListener('input', (e) => {
        this.query = e.target.value;
      });
      searchInput.addEventListener('keypress', (e) => {
        if (e.key === 'Enter') {
          this._performSearch();
        }
      });
    }
    if (sortSelect) {
      sortSelect.addEventListener('change', (e) => {
        this.sortBy = e.target.value;
        this._sortResults();
      });
    }
    filterRadios.forEach(radio => {
      radio.addEventListener('change', (e) => {
        this.filterType = e.target.value;
        this._filterResults();
      });
    });

    // Result action listeners
    this.$$('.result-item').forEach((item, index) => {
      item.addEventListener('click', () => this._selectResult(index));
    });
    this.$$('.result-action-add').forEach((btn, index) => {
      btn.addEventListener('click', (e) => {
        e.stopPropagation();
        this._addToContext(index);
      });
    });
  }

  _renderResults() {
    return this.results.map((result, index) => `
      <div class="result-item ${this.selectedResult === result ? 'selected' : ''}" data-index="${index}">
        <div class="result-header">
          <h4 class="result-title">${this._escapeHTML(result.title || 'Untitled')}</h4>
          ${result.score !== undefined ? `<span class="result-score">${Math.round(result.score * 100)}%</span>` : ''}
        </div>
        <p class="result-description">${this._escapeHTML(result.description || result.body || 'No description')}</p>
        <div class="result-meta">
          ${result.type ? `<span>Type: ${this._escapeHTML(result.type)}</span>` : ''}
          ${result.date ? `<span>Date: ${this._escapeHTML(result.date)}</span>` : ''}
          ${result.tags ? `<span>Tags: ${result.tags.slice(0, 3).join(', ')}</span>` : ''}
        </div>
        <div class="result-actions">
          <button class="result-action-btn result-action-add">+ Add to Context</button>
          <button class="result-action-btn">View Details</button>
        </div>
      </div>
    `).join('');
  }

  _renderEmptyState() {
    return `
      <div class="empty-state">
        <svg viewBox="0 0 24 24" xmlns="http://www.w3.org/2000/svg" fill="currentColor">
          <path d="M9.5,3A6.5,6.5 0 0,1 16,9.5C16,11.11 15.41,12.59 14.44,13.73L14.71,14H15.5L20.5,19L19,20.5L14,15.5V14.71L13.73,14.44C12.59,15.41 11.11,16 9.5,16A6.5,6.5 0 0,1 3,9.5A6.5,6.5 0 0,1 9.5,3M9.5,5C7,5 5,7 5,9.5C5,12 7,14 9.5,14C12,14 14,12 14,9.5C14,7 12,5 9.5,5Z"/>
        </svg>
        <h3 class="empty-state-title">${this.query ? 'No results found' : 'Start searching'}</h3>
        <p class="empty-state-text">${this.query ? 'Try different keywords or adjust filters' : 'Enter a search query to explore the knowledge graph'}</p>
      </div>
    `;
  }

  _performSearch() {
    if (!this.query.trim()) return;

    this.isSearching = true;
    this.dispatchEvent(new CustomEvent('kg-search', {
      detail: {
        query: this.query,
        filters: { type: this.filterType }
      },
      bubbles: true,
      composed: true
    }));

    // Simulate search delay (in real app, this would be async)
    setTimeout(() => {
      this.isSearching = false;
    }, 500);
  }

  _selectResult(index) {
    this.selectedResult = this.results[index];
    this.dispatchEvent(new CustomEvent('result-select', {
      detail: { result: this.selectedResult },
      bubbles: true,
      composed: true
    }));
  }

  _addToContext(index) {
    const result = this.results[index];
    this.dispatchEvent(new CustomEvent('result-add-context', {
      detail: { result },
      bubbles: true,
      composed: true
    }));
  }

  _sortResults() {
    // Sorting would be implemented here
    this.render();
  }

  _filterResults() {
    // Filtering would be implemented here
    this.render();
  }

  // Public API
  open(options = {}) {
    if (options.query) this.query = options.query;
    if (options.results) this.results = options.results;
    this.isOpen = true;
  }

  close() {
    this.isOpen = false;
    this.dispatchEvent(new CustomEvent('modal-close', {
      bubbles: true,
      composed: true
    }));
  }

  setResults(results) {
    this.results = results;
    this.isSearching = false;
  }

  _escapeHTML(str) {
    const div = document.createElement('div');
    div.textContent = str;
    return div.innerHTML;
  }
}

customElements.define('terraphim-kg-search-modal', TerraphimKgSearchModal);
