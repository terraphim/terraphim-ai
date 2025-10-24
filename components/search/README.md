# Search Components - Phase 1 Implementation

Phase 1 of the Web Components migration for Terraphim AI Search functionality.

## Overview

This directory contains the foundation infrastructure for the search system, implemented using pure vanilla JavaScript Web Components with zero dependencies.

## Phase 1 Components (COMPLETED)

### 1. search-utils.js
Pure JavaScript utility functions for search query parsing and formatting.

**Features:**
- Parse search input to detect AND/OR operators
- Build API-compatible search query objects
- Format search terms for display
- Extract current term for autocomplete
- Validate search queries

**Key Functions:**
- `parseSearchInput(inputText)` - Parse search query with operators
- `buildSearchQuery(parsed, role)` - Build API query object
- `formatSearchTerms(terms, operator)` - Format for display
- `isValidSearchQuery(query)` - Validation
- `getCurrentTerm(input, cursorPosition)` - Extract current term
- `endsWithOperator(input)` - Check if ends with AND/OR
- `suggestOperators(input)` - Get operator suggestions

**Example:**
```javascript
import { parseSearchInput, buildSearchQuery } from './search-utils.js';

const parsed = parseSearchInput('rust AND async');
// {
//   hasOperator: true,
//   operator: 'AND',
//   terms: ['rust', 'async'],
//   originalQuery: 'rust AND async'
// }

const query = buildSearchQuery(parsed, 'engineer');
// {
//   search_term: 'rust',
//   search_terms: ['rust', 'async'],
//   operator: 'and',
//   skip: 0,
//   limit: 50,
//   role: 'engineer'
// }
```

### 2. search-api.js
API client for communicating with the Terraphim backend.

**Features:**
- Dual-mode support: Web (fetch + SSE) and Tauri (invoke)
- Search execution with operator support
- Autocomplete suggestions
- SSE streaming for real-time summarization updates
- Polling fallback for Tauri mode
- Request cancellation and cleanup

**Key Classes:**
- `SearchAPI` - Main API client
- `createSearchAPI(options)` - Factory function

**Example:**
```javascript
import { createSearchAPI } from './search-api.js';

const api = createSearchAPI({
  baseUrl: 'http://localhost:3000'
});

// Execute search
const results = await api.search('rust async', { role: 'engineer' });

// Get autocomplete suggestions
const suggestions = await api.getAutocompleteSuggestions('rus', {
  role: 'engineer',
  limit: 8
});

// Start summarization streaming
const stopStreaming = api.startSummarizationStream(
  (taskId, summary) => {
    console.log('Summary received:', summary);
  }
);

// Later, cleanup
api.cleanup();
```

### 3. terraphim-term-chips.js
Web Component for displaying search terms as interactive chips.

**Features:**
- Visual display of selected terms
- Knowledge graph term highlighting (blue chips)
- Remove individual terms
- Toggle AND/OR operator
- Clear all terms
- Full keyboard navigation
- ARIA accessibility attributes

**Custom Element:**
```html
<terraphim-term-chips operator="AND"></terraphim-term-chips>
```

**API:**
```javascript
const chips = document.querySelector('terraphim-term-chips');

// Set terms
chips.terms = [
  { value: 'rust', isFromKG: true },
  { value: 'async', isFromKG: true }
];

// Add term
chips.addTerm('tokio', false);

// Remove term
chips.removeTerm('rust');

// Toggle operator
chips.toggleOperator();

// Clear all
chips.clearAll();

// Listen to events
chips.addEventListener('term-removed', (e) => {
  console.log('Removed:', e.detail.term);
});

chips.addEventListener('operator-changed', (e) => {
  console.log('New operator:', e.detail.operator);
});

chips.addEventListener('clear-all', () => {
  console.log('All terms cleared');
});
```

## Testing

### Test Page
Open `search-phase1-example.html` in a browser to test all Phase 1 components:

```bash
# From project root
open components/search/search-phase1-example.html

# Or with a local server
python -m http.server 8000
# Then navigate to http://localhost:8000/components/search/search-phase1-example.html
```

### Test Sections
1. **Search Utils** - Test parsing and query building
2. **Term Chips** - Interactive chip manipulation
3. **Search API** - API communication (requires backend)
4. **Integration** - All components working together

## Architecture

### Design Principles
- **Zero Dependencies** - Pure vanilla JavaScript, no frameworks
- **Web Components** - Shadow DOM encapsulation
- **Dual Transport** - Web (SSE) and Tauri (invoke) support
- **Event-Driven** - CustomEvents for component communication
- **Accessibility** - Full ARIA support and keyboard navigation
- **Progressive Enhancement** - Graceful degradation when features unavailable

### Component Communication
```
┌─────────────────────────────────────────┐
│   terraphim-search (Phase 4)            │
│   Main orchestrator component           │
└───────────┬─────────────────────────────┘
            │
            ├──> terraphim-search-input (Phase 2)
            │    └──> search-api.js (autocomplete)
            │
            ├──> terraphim-term-chips (Phase 1)
            │    └──> search-utils.js (parsing)
            │
            └──> terraphim-search-results (Phase 3)
                 ├──> terraphim-result-item (Phase 3)
                 └──> search-api.js (SSE streaming)
```

### State Flow
```
User Input → parseSearchInput() → buildSearchQuery() → SearchAPI.search()
                                                              ↓
                                                         API Response
                                                              ↓
         Results Display ← SSE Streaming ← StartSummarizationStream
```

## File Structure
```
components/search/
├── README.md                      # This file
├── search-utils.js               # Utility functions (Phase 1)
├── search-api.js                 # API client (Phase 1)
├── terraphim-term-chips.js       # Term chips component (Phase 1)
├── search-phase1-example.html    # Test page (Phase 1)
│
├── terraphim-search-input.js     # TODO: Phase 2
├── terraphim-result-item.js      # TODO: Phase 3
├── terraphim-search-results.js   # TODO: Phase 3
└── terraphim-search.js           # TODO: Phase 4
```

## Integration with Existing Code

### From Svelte Implementation
This Phase 1 ports core functionality from:
- `desktop/src/lib/Search/searchUtils.ts` → `search-utils.js`
- `desktop/src/lib/Search/Search.svelte` (lines 270-404) → `search-api.js`
- `desktop/src/lib/Search/TermChip.svelte` → `terraphim-term-chips.js`

### API Compatibility
The search-api.js client is fully compatible with the existing Terraphim backend:
- `POST /documents/search` - Search execution
- `GET /autocomplete/{role}/{query}` - Autocomplete suggestions
- `GET /summarization/stream` - SSE streaming for summaries

### Tauri Commands
Compatible with existing Tauri commands:
- `search` - Execute search query
- `get_autocomplete_suggestions` - Get suggestions

## Next Steps - Phase 2

Phase 2 will implement:
1. **terraphim-search-input.js** - Search input with autocomplete
   - ARIA combobox pattern
   - Dropdown suggestions
   - Keyboard navigation (arrows, tab, enter)
   - Debounced autocomplete calls
   - Operator suggestions
   - Term chip creation

Expected completion: Week 2

## Development Guidelines

### Code Style
- Use JSDoc comments for all public methods
- Follow existing patterns from `components/base/`
- Include error handling and logging
- Add ARIA attributes for accessibility
- Use Shadow DOM for all components

### Event Naming Convention
- `kebab-case` for event names
- Include `detail` object with relevant data
- Set `bubbles: true, composed: true` for cross-boundary events

### Property/Attribute Pattern
- Use `observedAttributes` for attributes
- Use `properties` getter for type conversion
- Reflect important properties to attributes
- Provide both property and attribute APIs

## Browser Compatibility

**Minimum Requirements:**
- Custom Elements v1
- Shadow DOM v1
- ES6 Modules
- EventSource (for web mode SSE)
- Fetch API

**Supported Browsers:**
- Chrome/Edge 63+
- Firefox 63+
- Safari 12.1+
- All modern browsers (2018+)

## Known Limitations

1. **SSE in Tauri** - EventSource not available in Tauri, uses polling fallback
2. **Operator Precedence** - Only supports single operator per query (AND or OR, not mixed)
3. **Autocomplete** - Requires backend running for suggestions
4. **Term Validation** - No client-side term validation (delegated to backend)

## Performance Considerations

- **Parsing** - Cached regex patterns for operator detection
- **API Calls** - Request cancellation support via AbortController
- **Rendering** - Batch DOM updates with requestAnimationFrame
- **Events** - Debounced autocomplete to reduce API calls
- **Memory** - Automatic cleanup on component disconnect

## Contributing

When adding new components:
1. Follow the TerraphimElement base class pattern
2. Include comprehensive JSDoc comments
3. Add test cases to example HTML
4. Update this README with new component documentation
5. Ensure ARIA accessibility
6. Test in both web and Tauri modes (if applicable)

## License

Part of Terraphim AI project - see root LICENSE file.
