# @terraphim/autocomplete

Fast autocomplete and knowledge graph functionality for Terraphim AI with native Node.js and WebAssembly support.

## Features

- 🚀 **High Performance**: Native Rust bindings with N-API for maximum speed
- 🔍 **Smart Autocomplete**: Prefix-based and fuzzy search with Jaro-Winkler similarity
- 🧠 **Knowledge Graph**: Graph-based semantic search and term connectivity
- 🌐 **Cross-Platform**: Support for Linux, macOS (Intel/Apple Silicon), and Windows
- 📦 **TypeScript**: Full TypeScript definitions included
- 🎯 **Easy to Use**: Simple API for rapid integration

## Installation

```bash
npm install @terraphim/autocomplete
```

## Quick Start

### Basic Autocomplete

```javascript
const { build_autocomplete_index_from_json, autocomplete } = require('@terraphim/autocomplete');

// Build an index from a thesaurus
const thesaurus = {
  name: "Engineering",
  data: {
    "machine learning": {
      id: 1,
      nterm: "machine learning",
      url: "https://example.com/ml"
    },
    "deep learning": {
      id: 2,
      nterm: "deep learning",
      url: "https://example.com/dl"
    },
    "neural networks": {
      id: 3,
      nterm: "neural networks",
      url: "https://example.com/nn"
    }
  }
};

// Create autocomplete index
const indexBytes = build_autocomplete_index_from_json(JSON.stringify(thesaurus));

// Search for completions
const results = autocomplete(indexBytes, "machine", 10);
console.log(results);
// Output:
// [
//   {
//     term: "machine learning",
//     normalized_term: "machine learning",
//     id: 1,
//     url: "https://example.com/ml",
//     score: 1.0
//   }
// ]
```

### Fuzzy Search

```javascript
const { fuzzy_autocomplete_search } = require('@terraphim/autocomplete');

// Fuzzy search with typos or partial matches
const fuzzyResults = fuzzy_autocomplete_search(
  indexBytes,
  "machin",  // Note the typo
  0.8,       // Similarity threshold (0.0-1.0)
  10         // Max results
);
console.log(fuzzyResults);
```

### TypeScript Usage

```typescript
import {
  build_autocomplete_index_from_json,
  autocomplete,
  fuzzy_autocomplete_search,
  AutocompleteResult
} from '@terraphim/autocomplete';

interface ThesaurusData {
  name: string;
  data: Record<string, {
    id: number;
    nterm: string;
    url: string;
  }>;
}

const thesaurus: ThesaurusData = {
  name: "Engineering",
  data: {
    "machine learning": {
      id: 1,
      nterm: "machine learning",
      url: "https://example.com/ml"
    }
  }
};

const indexBytes = build_autocomplete_index_from_json(JSON.stringify(thesaurus));
const results: AutocompleteResult[] = autocomplete(indexBytes, "machine", 10);
```

## API Reference

### Core Functions

#### `build_autocomplete_index_from_json(thesaurusJson: string): Uint8Array`

Builds an optimized autocomplete index from a JSON thesaurus.

- **Parameters:**
  - `thesaurusJson`: JSON string containing thesaurus data
- **Returns:** Serialized index as bytes for efficient searching
- **Throws:** Error if thesaurus JSON is invalid

#### `autocomplete(indexBytes: Uint8Array, query: string, maxResults?: number): AutocompleteResult[]`

Performs prefix-based autocomplete search.

- **Parameters:**
  - `indexBytes`: Serialized autocomplete index
  - `query`: Search query string
  - `maxResults`: Maximum number of results (default: all)
- **Returns:** Array of autocomplete results sorted by relevance

#### `fuzzy_autocomplete_search(indexBytes: Uint8Array, query: string, threshold?: number, maxResults?: number): AutocompleteResult[]`

Performs fuzzy search using Jaro-Winkler similarity algorithm.

- **Parameters:**
  - `indexBytes`: Serialized autocomplete index
  - `query`: Search query string
  - `threshold`: Similarity threshold 0.0-1.0 (default: 0.8)
  - `maxResults`: Maximum number of results (default: all)
- **Returns:** Array of autocomplete results sorted by similarity

### Types

#### `AutocompleteResult`

```typescript
interface AutocompleteResult {
  term: string;           // Original term
  normalized_term: string; // Normalized term for matching
  id: number;            // Unique identifier
  url: string;           // Associated URL
  score: number;         // Relevance score (0.0-1.0)
}
```

### Knowledge Graph Functions

#### `are_terms_connected(terms: string[]): boolean`

Checks if all terms are connected in the knowledge graph.

- **Parameters:**
  - `terms`: Array of term strings to check
- **Returns:** `true` if terms are connected, `false` otherwise

#### `build_role_graph_from_json(graphJson: string): Uint8Array`

Builds a knowledge graph from JSON data.

- **Parameters:**
  - `graphJson`: JSON string containing graph data
- **Returns:** Serialized graph data

### Utility Functions

#### `version(): string`

Returns the package version information.

## Thesaurus Format

The thesaurus should follow this JSON structure:

```json
{
  "name": "Thesaurus Name",
  "data": {
    "term name": {
      "id": 1,
      "nterm": "normalized term",
      "url": "https://example.com/resource"
    }
  }
}
```

### Required Fields

- `id`: Unique numeric identifier
- `nterm`: Normalized term string (used for matching)
- `url`: URL associated with the term

## Performance

- **Index Building**: O(n) where n is the number of terms
- **Search**: O(log n) for prefix search
- **Memory**: ~10-50 bytes per term (depending on term length)
- **Startup**: <100ms to load and deserialize typical thesauri

## Browser Support

This package is designed for Node.js environments. For browser usage, consider using the WebAssembly version directly from the main Terraphim AI repository.

## Examples

### React Component

```jsx
import React, { useState, useEffect } from 'react';
import { build_autocomplete_index_from_json, autocomplete } from '@terraphim/autocomplete';

function AutocompleteInput() {
  const [index, setIndex] = useState(null);
  const [suggestions, setSuggestions] = useState([]);

  useEffect(() => {
    // Load and build index
    const thesaurus = loadThesaurus(); // Your thesaurus loading logic
    const indexBytes = build_autocomplete_index_from_json(JSON.stringify(thesaurus));
    setIndex(indexBytes);
  }, []);

  const handleInput = (query) => {
    if (index && query.length > 2) {
      const results = autocomplete(index, query, 5);
      setSuggestions(results);
    } else {
      setSuggestions([]);
    }
  };

  return (
    <div>
      <input
        type="text"
        onChange={(e) => handleInput(e.target.value)}
        placeholder="Search..."
      />
      <ul>
        {suggestions.map((result) => (
          <li key={result.id}>
            <a href={result.url}>{result.term}</a>
          </li>
        ))}
      </ul>
    </div>
  );
}
```

### Express.js API

```javascript
const express = require('express');
const { build_autocomplete_index_from_json, autocomplete } = require('@terraphim/autocomplete');

const app = express();
let index = null;

// Load index on startup
const thesaurus = require('./engineering-thesaurus.json');
index = build_autocomplete_index_from_json(JSON.stringify(thesaurus));

app.get('/autocomplete', (req, res) => {
  const { q, limit = 10 } = req.query;

  if (!q || q.length < 2) {
    return res.json([]);
  }

  try {
    const results = autocomplete(index, q, parseInt(limit));
    res.json(results);
  } catch (error) {
    res.status(500).json({ error: error.message });
  }
});

app.listen(3000, () => {
  console.log('Autocomplete API running on port 3000');
});
```

## Development

```bash
# Install dependencies
npm install

# Build native module
npm run build

# Run tests
npm test

# Build for all platforms
npm run universal
```

## License

MIT © Terraphim Contributors

## Contributing

Contributions are welcome! Please read the [contributing guidelines](https://github.com/terraphim/terraphim-ai/blob/main/CONTRIBUTING.md) and submit pull requests to the main repository.

## Support

- 📖 [Documentation](https://docs.terraphim.ai)
- 🐛 [Issue Tracker](https://github.com/terraphim/terraphim-ai/issues)
- 💬 [Discussions](https://github.com/terraphim/terraphim-ai/discussions)