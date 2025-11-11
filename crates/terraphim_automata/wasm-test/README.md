# Terraphim Automata WASM Test

This directory contains WASM bindings and tests for `terraphim_automata`, demonstrating browser-compatible autocomplete functionality.

## Prerequisites

Install `wasm-pack`:
```bash
cargo install wasm-pack
```

## Building

Build the WASM module:
```bash
wasm-pack build --target web --out-dir pkg
```

Build for Node.js:
```bash
wasm-pack build --target nodejs --out-dir pkg-node
```

Build optimized release:
```bash
wasm-pack build --release --target web --out-dir pkg
```

## Testing

Run WASM tests in headless browser:
```bash
wasm-pack test --headless --firefox
```

Or with Chrome:
```bash
wasm-pack test --headless --chrome
```

## Usage Example

```html
<!DOCTYPE html>
<html>
<head>
    <meta charset="utf-8">
    <title>Terraphim Autocomplete Demo</title>
</head>
<body>
    <h1>Terraphim Autocomplete</h1>
    <input type="text" id="search" placeholder="Type to search...">
    <div id="results"></div>

    <script type="module">
        import init, {
            build_index_from_json,
            autocomplete,
            version
        } from './pkg/terraphim_automata_wasm_test.js';

        await init();
        console.log(version());

        const thesaurus = {
            "name": "Engineering",
            "data": {
                "project management": {
                    "id": 1,
                    "nterm": "project management",
                    "url": "https://example.com/pm"
                },
                "project constraints": {
                    "id": 2,
                    "nterm": "project constraints",
                    "url": "https://example.com/constraints"
                }
            }
        };

        const index = build_index_from_json(JSON.stringify(thesaurus));

        document.getElementById('search').addEventListener('input', (e) => {
            const query = e.target.value;
            if (query.length > 0) {
                const results = JSON.parse(autocomplete(index, query, 10));
                document.getElementById('results').innerHTML = results
                    .map(r => `<div>${r.term} (${r.score})</div>`)
                    .join('');
            }
        });
    </script>
</body>
</html>
```

## API Reference

### `init()`
Initialize the WASM module. Must be called before using other functions.

### `version(): string`
Returns the version information.

### `build_index_from_json(json_str: string): Uint8Array`
Build an autocomplete index from a JSON thesaurus string.

**Parameters:**
- `json_str`: JSON string in thesaurus format

**Returns:** Serialized index as Uint8Array

### `autocomplete(index: Uint8Array, query: string, max_results: number): string`
Search the autocomplete index.

**Parameters:**
- `index`: Serialized autocomplete index
- `query`: Search query string
- `max_results`: Maximum number of results

**Returns:** JSON string array of results

## File Size

Optimized WASM bundle size (gzipped):
- Development: ~500KB
- Release: ~200KB (estimated)

## Browser Compatibility

Requires:
- WebAssembly support
- ES6 modules
- Crypto.getRandomValues() for UUID generation

Supported browsers:
- Chrome 57+
- Firefox 52+
- Safari 11+
- Edge 16+
