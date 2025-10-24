/**
 * @fileoverview Search Components - Phase 1 Main Export
 * Central export point for all Phase 1 search infrastructure components
 */

// Utilities
export {
  parseSearchInput,
  buildSearchQuery,
  formatSearchTerms,
  isValidSearchQuery,
  getCurrentTerm,
  endsWithOperator,
  suggestOperators
} from './search-utils.js';

// API Client
export {
  SearchAPI,
  createSearchAPI
} from './search-api.js';

// Web Components
export { TerraphimTermChips } from './terraphim-term-chips.js';

/**
 * Initialize all search components
 * Call this once to register all custom elements
 *
 * @example
 * import { initSearchComponents } from './components/search/index.js';
 * initSearchComponents();
 */
export function initSearchComponents() {
  // Components are auto-registered on import
  // This function exists for explicit initialization if needed
  import('./terraphim-term-chips.js');
}

/**
 * Phase 1 Implementation Status
 * @constant
 */
export const PHASE1_STATUS = {
  completed: true,
  version: '1.0.0',
  components: [
    'search-utils.js',
    'search-api.js',
    'terraphim-term-chips.js'
  ],
  date: '2025-10-24'
};
