/**
 * @fileoverview Search Components - Complete Phase 2.1 Export
 * Central export point for all search infrastructure components
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

// Web Components - All Phases
export { TerraphimTermChips } from './terraphim-term-chips.js';
export { TerraphimSearchInput } from './terraphim-search-input.js';
export { TerraphimResultItem } from './terraphim-result-item.js';
export { TerraphimSearchResults } from './terraphim-search-results.js';
export { TerraphimSearch } from './terraphim-search.js';

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
  import('./terraphim-search-input.js');
  import('./terraphim-result-item.js');
  import('./terraphim-search-results.js');
  import('./terraphim-search.js');
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

/**
 * Phase 2 Implementation Status
 * @constant
 */
export const PHASE2_STATUS = {
  completed: true,
  version: '1.0.0',
  components: [
    'terraphim-search-input.js'
  ],
  date: '2025-10-24'
};

/**
 * Phase 3 Implementation Status
 * @constant
 */
export const PHASE3_STATUS = {
  completed: true,
  version: '1.0.0',
  components: [
    'terraphim-result-item.js',
    'terraphim-search-results.js'
  ],
  date: '2025-10-24'
};

/**
 * Phase 4 Implementation Status
 * @constant
 */
export const PHASE4_STATUS = {
  completed: true,
  version: '1.0.0',
  components: [
    'terraphim-search.js'
  ],
  date: '2025-10-24'
};

/**
 * Complete Phase 2.1 Implementation Status
 * @constant
 */
export const PHASE21_STATUS = {
  completed: true,
  version: '1.0.0',
  totalComponents: 8,
  phases: {
    phase1: PHASE1_STATUS,
    phase2: PHASE2_STATUS,
    phase3: PHASE3_STATUS,
    phase4: PHASE4_STATUS
  },
  features: [
    'Search input with autocomplete',
    'Term chips with operators',
    'Result items with SSE streaming',
    'Search results container',
    'Main search orchestrator',
    'State persistence',
    'Event coordination',
    'Role-based context'
  ],
  date: '2025-10-24'
};
