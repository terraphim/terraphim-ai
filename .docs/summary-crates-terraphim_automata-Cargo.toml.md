# Summary: crates/terraphim_automata/Cargo.toml

## Purpose
Text processing automata crate - provides Aho-Corasick search, FST autocomplete, and knowledge graph term matching.

## Key Details

### Core Functionality
- Aho-Corasick automata for fast term matching
- FST (Finite State Transducer) for autocomplete
- Knowledge graph term extraction and linking
- WASM bindings for browser/Node.js use

### Dependencies
- `aho-corasick` (1.0) - Fast multi-pattern matching
- `fst` (0.4) - Finite state transducers
- `bincode` (1.3) - Binary serialization
- `daachorse` (optional) - Double-array Aho-Corasick
- `zstd` (optional) - Compression
- `wasm-bindgen` (optional) - WASM bindings
- `tsify` (optional) - TypeScript type generation

### Features
- **default**: None (minimal)
- **remote-loading**: Async remote thesaurus loading (`tokio`, `reqwest`)
- **tokio-runtime**: Tokio runtime support
- **typescript**: TypeScript type generation
- **wasm**: Full WASM support
- **medical**: Medical domain extensions (`daachorse`, `zstd`)

### WASM Target
- Special `getrandom` dependency with `wasm_js` feature for WASM32 target

### Benchmarks
- `autocomplete_bench` - Criterion benchmark for autocomplete performance

### Dev Dependencies
- `criterion` (0.8) - Benchmarking framework
- `tempfile` - Temporary files

### Multi-Language Distribution
- Rust crate (crates.io)
- Node.js package via WASM (`terraphim_automata/wasm-test`)
- Python bindings via PyO3 (`terraphim_automata_py`)
