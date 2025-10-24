/**
 * Search Functionality
 * Placeholder for Phase 2 - Will integrate Fuse.js
 */

class Search {
  constructor() {
    this.index = null;
    this.items = [];
    this.listeners = new Set();
  }

  /**
   * Initialize search index with component data
   * @param {Array} components - Array of component metadata
   */
  initialize(components) {
    this.items = components;
    // TODO Phase 2: Initialize Fuse.js with fuzzy search
    console.log('Search initialized with', components.length, 'items');
  }

  /**
   * Perform search query
   * @param {string} query - Search string
   * @returns {Array} Search results
   */
  search(query) {
    if (!query || query.length < 2) {
      return [];
    }

    // Simple substring search for Phase 1
    // TODO Phase 2: Replace with Fuse.js fuzzy search
    const lowerQuery = query.toLowerCase();

    const results = this.items.filter(item => {
      const searchText = [
        item.name,
        item.category,
        item.description,
        ...(item.tags || [])
      ].join(' ').toLowerCase();

      return searchText.includes(lowerQuery);
    });

    this.notifyListeners(query, results);
    return results;
  }

  /**
   * Add a search listener
   * @param {Function} listener - Callback function (query, results)
   */
  addListener(listener) {
    this.listeners.add(listener);
  }

  /**
   * Remove a search listener
   * @param {Function} listener - Callback function to remove
   */
  removeListener(listener) {
    this.listeners.delete(listener);
  }

  /**
   * Notify all listeners of search results
   * @param {string} query - Search query
   * @param {Array} results - Search results
   */
  notifyListeners(query, results) {
    this.listeners.forEach(listener => {
      try {
        listener(query, results);
      } catch (error) {
        console.error('Error in search listener:', error);
      }
    });
  }

  /**
   * Clear search results
   */
  clear() {
    this.notifyListeners('', []);
  }

  /**
   * Get all items in index
   * @returns {Array} All indexed items
   */
  getAllItems() {
    return this.items;
  }
}

// Export singleton instance
export const search = new Search();
