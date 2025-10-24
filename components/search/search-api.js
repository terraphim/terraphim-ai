/**
 * @fileoverview Search API Client - Handles communication with Terraphim backend
 * Supports both web mode (SSE) and Tauri mode (polling) for real-time updates
 * Provides search, autocomplete, and summarization streaming functionality
 */

import { parseSearchInput, buildSearchQuery } from './search-utils.js';

/**
 * @typedef {Object} SearchResult
 * @property {string} status - Response status
 * @property {Document[]} results - Array of search result documents
 */

/**
 * @typedef {Object} Document
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
 * @typedef {Object} AutocompleteSuggestion
 * @property {string} term - Suggested term
 * @property {number} [score] - Relevance score
 * @property {string} [description] - Optional description
 */

/**
 * SearchAPI - Main API client for Terraphim search operations
 * Automatically detects and adapts to web or Tauri environments
 */
export class SearchAPI {
  /**
   * Create a new SearchAPI instance
   * @param {Object} options - Configuration options
   * @param {string} [options.baseUrl] - Base URL for API endpoints (web mode)
   * @param {boolean} [options.isTauri] - Force Tauri mode (auto-detected if not set)
   * @param {Function} [options.onError] - Error callback function
   */
  constructor(options = {}) {
    this.baseUrl = options.baseUrl || this._detectBaseUrl();
    this.isTauri = options.isTauri !== undefined ? options.isTauri : this._detectTauri();
    this.onError = options.onError || console.error;

    /**
     * Active SSE connection for summarization streaming
     * @private
     */
    this._sseConnection = null;

    /**
     * Active polling interval for Tauri mode
     * @private
     */
    this._pollingInterval = null;

    /**
     * Abort controller for canceling requests
     * @private
     */
    this._abortController = null;
  }

  /**
   * Detect if running in Tauri environment
   * @private
   * @returns {boolean}
   */
  _detectTauri() {
    return typeof window !== 'undefined' && window.__TAURI__ !== undefined;
  }

  /**
   * Detect base URL from current location or default
   * @private
   * @returns {string}
   */
  _detectBaseUrl() {
    if (typeof window === 'undefined') return 'http://localhost:3000';

    // Check if there's a meta tag with API URL
    const metaTag = document.querySelector('meta[name="api-url"]');
    if (metaTag) {
      return metaTag.getAttribute('content');
    }

    // Default to current origin
    return window.location.origin;
  }

  /**
   * Perform a search query
   * @param {string} input - Search input text
   * @param {Object} options - Search options
   * @param {string} [options.role] - Role name for context-specific search
   * @param {number} [options.skip=0] - Number of results to skip
   * @param {number} [options.limit=50] - Maximum number of results
   * @returns {Promise<SearchResult>} Search results
   *
   * @example
   * const api = new SearchAPI();
   * const results = await api.search('rust async', { role: 'engineer' });
   */
  async search(input, options = {}) {
    const parsed = parseSearchInput(input);
    const searchQuery = buildSearchQuery(parsed, options.role);

    // Apply custom skip/limit if provided
    if (options.skip !== undefined) searchQuery.skip = options.skip;
    if (options.limit !== undefined) searchQuery.limit = options.limit;

    try {
      if (this.isTauri) {
        return await this._searchTauri(searchQuery);
      } else {
        return await this._searchWeb(searchQuery);
      }
    } catch (error) {
      this.onError('Search failed:', error);
      return { status: 'error', results: [], error: error.message };
    }
  }

  /**
   * Search using Tauri invoke
   * @private
   * @param {Object} searchQuery - Search query object
   * @returns {Promise<SearchResult>}
   */
  async _searchTauri(searchQuery) {
    const { invoke } = window.__TAURI__.tauri;
    return await invoke('search', { searchQuery });
  }

  /**
   * Search using web fetch API
   * @private
   * @param {Object} searchQuery - Search query object
   * @returns {Promise<SearchResult>}
   */
  async _searchWeb(searchQuery) {
    this._abortController = new AbortController();

    const response = await fetch(`${this.baseUrl}/documents/search`, {
      method: 'POST',
      headers: {
        'Accept': 'application/json',
        'Content-Type': 'application/json',
      },
      body: JSON.stringify(searchQuery),
      signal: this._abortController.signal,
    });

    if (!response.ok) {
      throw new Error(`HTTP error! Status: ${response.status}`);
    }

    return await response.json();
  }

  /**
   * Get autocomplete suggestions for a query
   * @param {string} query - Partial search query
   * @param {Object} options - Autocomplete options
   * @param {string} [options.role] - Role name for context-specific suggestions
   * @param {number} [options.limit=8] - Maximum number of suggestions
   * @returns {Promise<AutocompleteSuggestion[]>} Array of suggestions
   *
   * @example
   * const suggestions = await api.getAutocompleteSuggestions('rus', { role: 'engineer' });
   */
  async getAutocompleteSuggestions(query, options = {}) {
    const role = options.role || '';
    const limit = options.limit || 8;

    try {
      if (this.isTauri) {
        return await this._autocompleteTauri(query, role, limit);
      } else {
        return await this._autocompleteWeb(query, role, limit);
      }
    } catch (error) {
      this.onError('Autocomplete failed:', error);
      return [];
    }
  }

  /**
   * Autocomplete using Tauri invoke
   * @private
   * @param {string} query - Search query
   * @param {string} role - Role name
   * @param {number} limit - Maximum suggestions
   * @returns {Promise<AutocompleteSuggestion[]>}
   */
  async _autocompleteTauri(query, role, limit) {
    const { invoke } = window.__TAURI__.tauri;
    const response = await invoke('get_autocomplete_suggestions', {
      query,
      roleName: role,
      limit
    });

    if (response.status === 'success' && response.suggestions) {
      return response.suggestions;
    }

    return [];
  }

  /**
   * Autocomplete using web fetch API
   * @private
   * @param {string} query - Search query
   * @param {string} role - Role name
   * @param {number} limit - Maximum suggestions
   * @returns {Promise<AutocompleteSuggestion[]>}
   */
  async _autocompleteWeb(query, role, limit) {
    const url = `${this.baseUrl}/autocomplete/${encodeURIComponent(role)}/${encodeURIComponent(query)}?limit=${limit}`;
    const response = await fetch(url);

    if (!response.ok) {
      throw new Error(`HTTP error! Status: ${response.status}`);
    }

    const data = await response.json();
    if (data.status === 'success' && data.suggestions) {
      return data.suggestions;
    }

    return [];
  }

  /**
   * Start SSE connection for real-time summarization updates
   * @param {Function} onUpdate - Callback for summarization updates
   * @param {Function} [onError] - Error callback
   * @returns {Function} Stop streaming function
   *
   * @example
   * const stopStreaming = api.startSummarizationStream(
   *   (taskId, summary) => {
   *     console.log('Summary received:', summary);
   *   },
   *   (error) => {
   *     console.error('Streaming error:', error);
   *   }
   * );
   *
   * // Later, stop streaming
   * stopStreaming();
   */
  startSummarizationStream(onUpdate, onError) {
    if (this.isTauri) {
      // Tauri doesn't support SSE, use polling instead
      return this._startPolling(onUpdate);
    }

    return this._startSSE(onUpdate, onError);
  }

  /**
   * Start SSE connection (web mode)
   * @private
   * @param {Function} onUpdate - Update callback
   * @param {Function} [onError] - Error callback
   * @returns {Function} Stop function
   */
  _startSSE(onUpdate, onError) {
    // Close existing connection if any
    if (this._sseConnection) {
      this._sseConnection.close();
    }

    const sseUrl = `${this.baseUrl}/summarization/stream`;

    try {
      this._sseConnection = new EventSource(sseUrl);

      this._sseConnection.onopen = () => {
        console.log('SSE connection opened for summarization updates');
      };

      this._sseConnection.onmessage = (event) => {
        try {
          const data = JSON.parse(event.data);
          console.log('SSE received:', data);

          if (data.task_id && data.status === 'completed' && data.summary) {
            onUpdate(data.task_id, data.summary);
          }
        } catch (error) {
          console.warn('Failed to parse SSE message:', error);
        }
      };

      this._sseConnection.onerror = (error) => {
        console.warn('SSE connection error:', error);
        if (onError) onError(error);

        // Auto-reconnect after a delay
        setTimeout(() => {
          if (this._sseConnection) {
            this._startSSE(onUpdate, onError);
          }
        }, 5000);
      };

      // Return stop function
      return () => {
        if (this._sseConnection) {
          this._sseConnection.close();
          this._sseConnection = null;
        }
      };
    } catch (error) {
      console.error('Failed to create SSE connection:', error);
      if (onError) onError(error);
      return () => {}; // No-op stop function
    }
  }

  /**
   * Start polling for updates (Tauri mode)
   * @private
   * @param {Function} onUpdate - Update callback
   * @returns {Function} Stop function
   */
  _startPolling(onUpdate) {
    console.log('Starting polling for summary updates (Tauri mode)');

    // Poll every 2 seconds
    this._pollingInterval = setInterval(async () => {
      // Note: Actual polling implementation would need to track documents
      // and check for updates. This is a placeholder.
      // In practice, the search component would manage this.
    }, 2000);

    // Auto-stop after 30 seconds
    setTimeout(() => {
      this.stopPolling();
    }, 30000);

    // Return stop function
    return () => this.stopPolling();
  }

  /**
   * Stop polling (Tauri mode)
   */
  stopPolling() {
    if (this._pollingInterval) {
      clearInterval(this._pollingInterval);
      this._pollingInterval = null;
    }
  }

  /**
   * Cancel active search request
   */
  cancelSearch() {
    if (this._abortController) {
      this._abortController.abort();
      this._abortController = null;
    }
  }

  /**
   * Cleanup all connections and intervals
   */
  cleanup() {
    this.cancelSearch();
    this.stopPolling();

    if (this._sseConnection) {
      this._sseConnection.close();
      this._sseConnection = null;
    }
  }
}

/**
 * Create and configure a SearchAPI instance with defaults
 * @param {Object} options - Configuration options
 * @returns {SearchAPI} Configured API instance
 *
 * @example
 * const api = createSearchAPI({ baseUrl: 'http://localhost:3000' });
 */
export function createSearchAPI(options = {}) {
  return new SearchAPI(options);
}
