/**
 * @fileoverview TerraphimSearch - Main search orchestrator component
 * Coordinates all search sub-components: input, chips, and results
 * Manages search workflow, state persistence, and SSE streaming
 */

import { TerraphimElement } from '../base/terraphim-element.js';
import { TerraphimState } from '../base/terraphim-state.js';
import { SearchAPI } from './search-api.js';
import { parseSearchInput } from './search-utils.js';
import './terraphim-search-input.js';
import './terraphim-term-chips.js';
import './terraphim-search-results.js';

/**
 * @typedef {Object} SearchState
 * @property {string} query - Current search query
 * @property {string[]} terms - Selected search terms
 * @property {string} operator - Logical operator ('AND' or 'OR')
 * @property {string} role - Current role
 * @property {number} timestamp - Last update timestamp
 */

/**
 * TerraphimSearch - Main search orchestrator
 * Composes search input, term chips, and results into a unified interface
 *
 * @element terraphim-search
 *
 * @fires {CustomEvent} search-started - When search begins
 * @fires {CustomEvent} search-completed - When search finishes successfully
 * @fires {CustomEvent} search-failed - When search fails
 * @fires {CustomEvent} role-changed - When role changes
 * @fires {CustomEvent} state-updated - When internal state changes
 *
 * @attr {string} role - Current role name
 * @attr {boolean} loading - Loading state
 * @attr {string} query - Current search query
 * @attr {string} operator - Current operator ('AND' or 'OR')
 * @attr {boolean} auto-search - Auto-execute search when terms change
 *
 * @example
 * <terraphim-search
 *   role="engineer"
 *   auto-search
 * ></terraphim-search>
 *
 * <script>
 *   const search = document.querySelector('terraphim-search');
 *
 *   search.addEventListener('search-completed', (e) => {
 *     console.log('Search completed:', e.detail.results);
 *   });
 *
 *   // Programmatic search
 *   search.executeSearch('rust async', { limit: 10 });
 * </script>
 */
export class TerraphimSearch extends TerraphimElement {
  static get observedAttributes() {
    return ['role', 'loading', 'query', 'operator', 'auto-search'];
  }

  static get properties() {
    return {
      role: { type: String, reflect: true, default: '' },
      loading: { type: Boolean, reflect: true, default: false },
      query: { type: String, reflect: true, default: '' },
      operator: { type: String, reflect: true, default: 'AND' },
      autoSearch: { type: Boolean, reflect: true, default: false },
    };
  }

  constructor() {
    super();

    /**
     * Selected search terms
     * @type {Array<{value: string, isFromKG: boolean}>}
     */
    this._terms = [];

    /**
     * Current search results
     * @type {Array}
     */
    this._results = [];

    /**
     * Error message if any
     * @type {string}
     */
    this._error = '';

    /**
     * SearchAPI instance
     * @type {SearchAPI}
     */
    this._api = new SearchAPI();

    /**
     * TerraphimState instance for global state management
     * @type {TerraphimState}
     */
    this._state = null;

    /**
     * Search history
     * @type {Array<SearchState>}
     */
    this._searchHistory = [];

    /**
     * Maximum history size
     * @type {number}
     */
    this._maxHistorySize = 50;

    /**
     * Flag to prevent circular updates between input and chips
     * @type {boolean}
     */
    this._isUpdatingFromChips = false;

    /**
     * Parse timeout for debouncing input parsing
     * @type {number|null}
     */
    this._parseTimeout = null;

    /**
     * State unsubscribe functions
     * @type {Function[]}
     */
    this._stateUnsubscribers = [];

    this.attachShadow({ mode: 'open' });
  }

  /**
   * Get current search terms
   * @returns {Array<{value: string, isFromKG: boolean}>}
   */
  get terms() {
    return this._terms;
  }

  /**
   * Set search terms
   * @param {Array<{value: string, isFromKG: boolean}>} value
   */
  set terms(value) {
    if (!Array.isArray(value)) return;
    this._terms = value;
    this.requestUpdate();
  }

  /**
   * Get current results
   * @returns {Array}
   */
  get results() {
    return this._results;
  }

  /**
   * Get current error
   * @returns {string}
   */
  get error() {
    return this._error;
  }

  /**
   * Lifecycle hook: setup after connection
   */
  onConnected() {
    // Initialize state management
    this._initializeState();

    // Load persisted search state
    this._loadSearchState();

    // Load search history
    this._loadSearchHistory();
  }

  /**
   * Lifecycle hook: cleanup before disconnection
   */
  onDisconnected() {
    // Clear parse timeout
    if (this._parseTimeout) {
      clearTimeout(this._parseTimeout);
      this._parseTimeout = null;
    }

    // Cleanup API connections
    this._api.cleanup();

    // Unsubscribe from state
    this._stateUnsubscribers.forEach(unsubscribe => unsubscribe());
    this._stateUnsubscribers = [];
  }

  /**
   * Initialize state management
   * @private
   */
  _initializeState() {
    // Try to get global state, or create local state
    if (typeof window !== 'undefined' && window.__TERRAPHIM_STATE__) {
      this._state = window.__TERRAPHIM_STATE__;
    } else {
      this._state = new TerraphimState({}, {
        persist: true,
        storagePrefix: 'terraphim',
        debug: false
      });
    }

    // Subscribe to role changes
    const roleUnsub = this._state.subscribe('role', (newRole) => {
      if (newRole && newRole !== this.role) {
        this.role = newRole;
        this.emit('role-changed', { role: newRole });
      }
    }, { immediate: true });

    this._stateUnsubscribers.push(roleUnsub);
  }

  /**
   * Execute search programmatically
   * @param {string} query - Search query
   * @param {Object} [options] - Search options
   * @param {number} [options.skip] - Results to skip
   * @param {number} [options.limit] - Maximum results
   * @returns {Promise<void>}
   *
   * @example
   * await search.executeSearch('rust async', { limit: 20 });
   */
  async executeSearch(query, options = {}) {
    if (!query || query.trim().length === 0) {
      console.warn('Cannot execute search with empty query');
      return;
    }

    this.query = query;
    this.loading = true;
    this._error = '';

    // Parse query to extract terms
    this._parseAndUpdateChips(query);

    // Emit search-started event
    this.emit('search-started', {
      query,
      terms: this._terms,
      operator: this.operator,
      role: this.role
    });

    // Update UI to show loading state
    this.requestUpdate();

    const startTime = Date.now();

    try {
      // Execute search via API
      const result = await this._api.search(query, {
        role: this.role,
        skip: options.skip,
        limit: options.limit
      });

      if (result.status === 'success') {
        this._results = result.results || [];
        this._error = '';

        const duration = Date.now() - startTime;
        console.log(`Search completed in ${duration}ms: ${this._results.length} results`);

        // Update results component
        const resultsComponent = this.$('terraphim-search-results');
        if (resultsComponent) {
          resultsComponent.setResults(this._results);

          // Start SSE streaming for summarization
          resultsComponent.startSummarization(query, this.role);
        }

        // Save search state
        this._saveSearchState();

        // Add to search history
        this._addToSearchHistory(query, this._terms, this.operator);

        // Emit search-completed event
        this.emit('search-completed', {
          query,
          results: this._results,
          duration,
          count: this._results.length
        });
      } else {
        throw new Error(result.error || 'Search failed with unknown error');
      }
    } catch (error) {
      console.error('Search failed:', error);
      this._error = error.message || 'Search failed';

      // Update results component with error
      const resultsComponent = this.$('terraphim-search-results');
      if (resultsComponent) {
        resultsComponent.error = this._error;
        resultsComponent.clearResults();
      }

      // Emit search-failed event
      this.emit('search-failed', {
        query,
        error: this._error
      });
    } finally {
      this.loading = false;
      this.requestUpdate();
    }
  }

  /**
   * Clear current search and results
   */
  clearSearch() {
    this.query = '';
    this._terms = [];
    this._results = [];
    this._error = '';

    // Clear input component
    const inputComponent = this.$('terraphim-search-input');
    if (inputComponent) {
      inputComponent.clear();
    }

    // Clear chips component
    const chipsComponent = this.$('terraphim-term-chips');
    if (chipsComponent) {
      chipsComponent.clearAll();
    }

    // Clear results component
    const resultsComponent = this.$('terraphim-search-results');
    if (resultsComponent) {
      resultsComponent.clearResults();
      resultsComponent.stopSummarization();
    }

    // Clear persisted state
    this._clearSearchState();

    this.requestUpdate();
  }

  /**
   * Set role and update all sub-components
   * @param {string} role - New role name
   */
  setRole(role) {
    this.role = role;

    // Update state
    if (this._state) {
      this._state.set('role', role);
    }

    // Update sub-components
    const inputComponent = this.$('terraphim-search-input');
    if (inputComponent) {
      inputComponent.role = role;
    }

    const resultsComponent = this.$('terraphim-search-results');
    if (resultsComponent) {
      resultsComponent.role = role;
    }

    this.emit('role-changed', { role });
    this.requestUpdate();
  }

  /**
   * Get current state snapshot
   * @returns {SearchState}
   */
  getState() {
    return {
      query: this.query,
      terms: this._terms.map(t => t.value),
      operator: this.operator,
      role: this.role,
      timestamp: Date.now()
    };
  }

  /**
   * Restore state from snapshot
   * @param {SearchState} state - State to restore
   */
  setState(state) {
    if (!state) return;

    this.query = state.query || '';
    this.operator = state.operator || 'AND';
    this.role = state.role || '';

    if (Array.isArray(state.terms)) {
      this._terms = state.terms.map(term => ({
        value: term,
        isFromKG: false // TODO: detect from thesaurus
      }));
    }

    this.requestUpdate();
  }

  /**
   * Parse search input and update term chips
   * @private
   * @param {string} input - Search input text
   */
  _parseAndUpdateChips(input) {
    const parsed = parseSearchInput(input);

    if (parsed.hasOperator && parsed.terms.length > 1) {
      // Update terms and operator
      this._terms = parsed.terms.map(term => ({
        value: term,
        isFromKG: false // TODO: check against thesaurus
      }));

      this.operator = parsed.operator;

      // Update chips component
      const chipsComponent = this.$('terraphim-term-chips');
      if (chipsComponent) {
        chipsComponent.terms = this._terms;
        chipsComponent.operator = this.operator;
      }
    } else if (parsed.terms.length === 1 && this._terms.length > 0) {
      // Single term - clear chips
      this._terms = [];
      this.operator = 'AND';

      const chipsComponent = this.$('terraphim-term-chips');
      if (chipsComponent) {
        chipsComponent.clearAll();
      }
    } else if (parsed.terms.length === 0) {
      // Empty input - clear everything
      this._terms = [];
      this.operator = 'AND';

      const chipsComponent = this.$('terraphim-term-chips');
      if (chipsComponent) {
        chipsComponent.clearAll();
      }
    }
  }

  /**
   * Update input from selected terms
   * @private
   */
  _updateInputFromTerms() {
    this._isUpdatingFromChips = true;

    let newQuery = '';

    if (this._terms.length === 0) {
      newQuery = '';
    } else if (this._terms.length === 1) {
      newQuery = this._terms[0].value;
    } else {
      newQuery = this._terms.map(t => t.value).join(` ${this.operator} `);
    }

    this.query = newQuery;

    // Update input component
    const inputComponent = this.$('terraphim-search-input');
    if (inputComponent) {
      inputComponent.setValue(newQuery);
    }

    // Reset flag after a delay
    setTimeout(() => {
      this._isUpdatingFromChips = false;
    }, 10);
  }

  /**
   * Save search state to localStorage
   * @private
   */
  _saveSearchState() {
    try {
      const key = this._getSearchStateKey();
      const state = {
        query: this.query,
        terms: this._terms,
        operator: this.operator,
        results: this._results.slice(0, 10), // Save first 10 results only
        timestamp: Date.now()
      };

      localStorage.setItem(key, JSON.stringify(state));
    } catch (error) {
      console.warn('Failed to save search state:', error);
    }
  }

  /**
   * Load search state from localStorage
   * @private
   */
  _loadSearchState() {
    try {
      const key = this._getSearchStateKey();
      const raw = localStorage.getItem(key);

      if (!raw) return;

      const state = JSON.parse(raw);

      if (state.query) {
        this.query = state.query;

        const inputComponent = this.$('terraphim-search-input');
        if (inputComponent) {
          inputComponent.setValue(state.query);
        }
      }

      if (Array.isArray(state.terms)) {
        this._terms = state.terms;

        const chipsComponent = this.$('terraphim-term-chips');
        if (chipsComponent) {
          chipsComponent.terms = state.terms;
        }
      }

      if (state.operator) {
        this.operator = state.operator;

        const chipsComponent = this.$('terraphim-term-chips');
        if (chipsComponent) {
          chipsComponent.operator = state.operator;
        }
      }

      if (Array.isArray(state.results)) {
        this._results = state.results;

        const resultsComponent = this.$('terraphim-search-results');
        if (resultsComponent) {
          resultsComponent.setResults(state.results);
        }
      }
    } catch (error) {
      console.warn('Failed to load search state:', error);
    }
  }

  /**
   * Clear search state from localStorage
   * @private
   */
  _clearSearchState() {
    try {
      const key = this._getSearchStateKey();
      localStorage.removeItem(key);
    } catch (error) {
      console.warn('Failed to clear search state:', error);
    }
  }

  /**
   * Get search state key for localStorage
   * @private
   * @returns {string}
   */
  _getSearchStateKey() {
    return `terraphim:searchState:${this.role || 'default'}`;
  }

  /**
   * Add to search history
   * @private
   * @param {string} query - Search query
   * @param {Array} terms - Search terms
   * @param {string} operator - Logical operator
   */
  _addToSearchHistory(query, terms, operator) {
    const entry = {
      query,
      terms: terms.map(t => t.value),
      operator,
      timestamp: Date.now()
    };

    // Remove duplicate entries
    this._searchHistory = this._searchHistory.filter(
      h => h.query !== query
    );

    // Add to beginning of history
    this._searchHistory.unshift(entry);

    // Limit history size
    if (this._searchHistory.length > this._maxHistorySize) {
      this._searchHistory = this._searchHistory.slice(0, this._maxHistorySize);
    }

    // Save to localStorage
    this._saveSearchHistory();
  }

  /**
   * Save search history to localStorage
   * @private
   */
  _saveSearchHistory() {
    try {
      const key = 'terraphim:searchHistory';
      localStorage.setItem(key, JSON.stringify(this._searchHistory));
    } catch (error) {
      console.warn('Failed to save search history:', error);
    }
  }

  /**
   * Load search history from localStorage
   * @private
   */
  _loadSearchHistory() {
    try {
      const key = 'terraphim:searchHistory';
      const raw = localStorage.getItem(key);

      if (raw) {
        this._searchHistory = JSON.parse(raw);
      }
    } catch (error) {
      console.warn('Failed to load search history:', error);
    }
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
          max-width: 1200px;
          margin: 0 auto;
        }

        .search-container {
          display: flex;
          flex-direction: column;
          gap: 1rem;
          padding: 1rem;
        }

        .search-header {
          display: flex;
          flex-direction: column;
          gap: 0.5rem;
        }

        .search-input-section {
          width: 100%;
        }

        .search-chips-section {
          width: 100%;
          min-height: ${this._terms.length > 0 ? 'auto' : '0'};
          transition: min-height 0.2s ease;
        }

        .search-results-section {
          width: 100%;
          margin-top: 1rem;
        }

        /* Loading overlay */
        .loading-overlay {
          display: ${this.loading ? 'flex' : 'none'};
          position: fixed;
          top: 0;
          left: 0;
          right: 0;
          bottom: 0;
          background: rgba(255, 255, 255, 0.8);
          align-items: center;
          justify-content: center;
          z-index: 9999;
          backdrop-filter: blur(2px);
        }

        .loading-spinner {
          width: 4rem;
          height: 4rem;
          border: 4px solid #f3f3f3;
          border-top: 4px solid #3273dc;
          border-radius: 50%;
          animation: spin 1s linear infinite;
        }

        @keyframes spin {
          0% { transform: rotate(0deg); }
          100% { transform: rotate(360deg); }
        }

        /* Responsive design */
        @media (max-width: 768px) {
          .search-container {
            padding: 0.5rem;
          }
        }

        @media (max-width: 480px) {
          .search-container {
            padding: 0.25rem;
          }
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

      <div class="search-container">
        <!-- Search Header -->
        <div class="search-header" role="search" aria-label="Search interface">
          <!-- Search Input -->
          <div class="search-input-section">
            <terraphim-search-input
              placeholder="Search over Knowledge graph for ${this._escapeHtml(this.role || 'default')}"
              role="${this._escapeHtml(this.role)}"
              value="${this._escapeHtml(this.query)}"
            ></terraphim-search-input>
          </div>

          <!-- Term Chips -->
          <div class="search-chips-section">
            <terraphim-term-chips
              operator="${this.operator}"
            ></terraphim-term-chips>
          </div>
        </div>

        <!-- Search Results -->
        <div class="search-results-section">
          <terraphim-search-results
            query="${this._escapeHtml(this.query)}"
            role="${this._escapeHtml(this.role)}"
            ?loading="${this.loading}"
            error="${this._escapeHtml(this._error)}"
          ></terraphim-search-results>
        </div>

        <!-- Loading Overlay -->
        <div class="loading-overlay" aria-live="polite" aria-busy="${this.loading}">
          <div class="loading-spinner"></div>
          <span class="sr-only">Searching...</span>
        </div>
      </div>
    `);

    this._attachEventListeners();
  }

  /**
   * Attach event listeners to sub-components
   * @private
   */
  _attachEventListeners() {
    // Search Input Component
    const inputComponent = this.$('terraphim-search-input');
    if (inputComponent) {
      // Handle search submission
      inputComponent.addEventListener('search-submit', (e) => {
        const { query, parsed } = e.detail;
        this.executeSearch(query);
      });

      // Handle term selection from autocomplete
      inputComponent.addEventListener('term-added', (e) => {
        const { term, isFromKG } = e.detail;

        // Check if term already exists
        if (!this._terms.some(t => t.value === term)) {
          this._terms = [...this._terms, { value: term, isFromKG }];

          // Update chips component
          const chipsComponent = this.$('terraphim-term-chips');
          if (chipsComponent) {
            chipsComponent.addTerm(term, isFromKG);
          }

          this._updateInputFromTerms();
        }
      });

      // Handle input changes with debounced parsing
      inputComponent.addEventListener('input-changed', (e) => {
        const { value } = e.detail;
        this.query = value;

        // Only parse if input contains operators
        if (!this._isUpdatingFromChips &&
            (value.includes(' AND ') || value.includes(' OR ') ||
             value.includes(' and ') || value.includes(' or '))) {

          // Clear previous timeout
          if (this._parseTimeout) {
            clearTimeout(this._parseTimeout);
          }

          // Debounce parsing (300ms)
          this._parseTimeout = setTimeout(() => {
            this._parseAndUpdateChips(value);
            this._parseTimeout = null;
          }, 300);
        }
      });
    }

    // Term Chips Component
    const chipsComponent = this.$('terraphim-term-chips');
    if (chipsComponent) {
      // Set initial terms
      chipsComponent.terms = this._terms;
      chipsComponent.operator = this.operator;

      // Handle term removal
      chipsComponent.addEventListener('term-removed', (e) => {
        const { term } = e.detail;
        this._terms = this._terms.filter(t => t.value !== term);
        this._updateInputFromTerms();

        // Auto-search if enabled
        if (this.autoSearch && this._terms.length > 0) {
          this.executeSearch(this.query);
        }
      });

      // Handle operator change
      chipsComponent.addEventListener('operator-changed', (e) => {
        const { operator } = e.detail;
        this.operator = operator;
        this._updateInputFromTerms();

        // Auto-search if enabled
        if (this.autoSearch && this._terms.length > 1) {
          this.executeSearch(this.query);
        }
      });

      // Handle clear all
      chipsComponent.addEventListener('clear-all', () => {
        this._terms = [];
        this.operator = 'AND';
        this.query = '';

        const inputComponent = this.$('terraphim-search-input');
        if (inputComponent) {
          inputComponent.clear();
        }
      });
    }

    // Search Results Component
    const resultsComponent = this.$('terraphim-search-results');
    if (resultsComponent) {
      // Bubble result events
      resultsComponent.addEventListener('result-clicked', (e) => {
        this.emit('result-clicked', e.detail);
      });

      resultsComponent.addEventListener('retry-search', () => {
        if (this.query) {
          this.executeSearch(this.query);
        }
      });

      resultsComponent.addEventListener('load-more', () => {
        this.emit('load-more', { query: this.query, currentCount: this._results.length });
      });

      // SSE events
      resultsComponent.addEventListener('sse-connected', () => {
        console.log('SSE summarization stream connected');
      });

      resultsComponent.addEventListener('sse-disconnected', () => {
        console.log('SSE summarization stream disconnected');
      });

      resultsComponent.addEventListener('sse-error', (e) => {
        console.error('SSE summarization error:', e.detail.error);
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
if (!customElements.get('terraphim-search')) {
  customElements.define('terraphim-search', TerraphimSearch);
}
