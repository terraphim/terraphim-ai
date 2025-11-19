# Terraphim Automata Python Bindings

[![PyPI version](https://badge.fury.io/py/terraphim-automata.svg)](https://badge.fury.io/py/terraphim-automata)
[![Python Support](https://img.shields.io/pypi/pyversions/terraphim-automata.svg)](https://pypi.org/project/terraphim-automata/)
[![License](https://img.shields.io/badge/License-Apache%202.0-blue.svg)](https://opensource.org/licenses/Apache-2.0)

Fast autocomplete and text processing library for knowledge graphs, powered by Rust.

## Features

- **⚡ Lightning Fast**: Built on Rust with Finite State Transducers (FST) and Aho-Corasick automata
- **🔍 Autocomplete**: Prefix-based search with sub-millisecond response times
- **🎯 Fuzzy Search**: Support for typos using Jaro-Winkler and Levenshtein distance
- **📝 Text Processing**: Find and replace terms with automatic linking
- **📄 Paragraph Extraction**: Extract relevant paragraphs based on term matches
- **🐍 Pythonic API**: Easy-to-use interface with type hints
- **🔒 Type Safe**: Full type stub support for IDEs and type checkers

## Installation

### From PyPI (Recommended)

```bash
pip install terraphim-automata
```

### From Source with uv

```bash
# Clone the repository
git clone https://github.com/terraphim/terraphim-ai.git
cd terraphim-ai/crates/terraphim_automata_py

# Install uv if you haven't already
pip install uv

# Build and install
uv pip install maturin
maturin develop
```

## Quick Start

### Building an Autocomplete Index

```python
from terraphim_automata import build_index

# Define a thesaurus with your terms
thesaurus_json = """{
    "name": "Engineering",
    "data": {
        "machine learning": {
            "id": 1,
            "nterm": "machine learning",
            "url": "https://example.com/ml"
        },
        "deep learning": {
            "id": 2,
            "nterm": "deep learning",
            "url": "https://example.com/dl"
        },
        "artificial intelligence": {
            "id": 3,
            "nterm": "artificial intelligence",
            "url": "https://example.com/ai"
        }
    }
}"""

# Build the index
index = build_index(thesaurus_json)

# Search for completions
results = index.search("mach")
for result in results:
    print(f"{result.term} (score: {result.score}, url: {result.url})")
```

### Fuzzy Search

```python
# Jaro-Winkler similarity (good for typos at the start)
results = index.fuzzy_search("machin lerning", threshold=0.8)

# Levenshtein distance (good for general typos)
results = index.fuzzy_search_levenshtein("machne", max_distance=2)
```

### Text Processing

```python
from terraphim_automata import find_all_matches, replace_with_links

text = "Machine learning and deep learning are subfields of artificial intelligence."

# Find all term matches
matches = find_all_matches(text, thesaurus_json)
for match in matches:
    print(f"Found '{match.term}' at position {match.pos}")

# Replace terms with markdown links
markdown = replace_with_links(text, thesaurus_json, "markdown")
print(markdown)
# Output: [machine learning](https://example.com/ml) and [deep learning](https://example.com/dl)
#         are subfields of [artificial intelligence](https://example.com/ai).

# Or HTML links
html = replace_with_links(text, thesaurus_json, "html")

# Or wiki-style links
wiki = replace_with_links(text, thesaurus_json, "wiki")
```

### Paragraph Extraction

```python
from terraphim_automata import extract_paragraphs

document = """
Introduction to AI.

Machine learning is a subset of artificial intelligence that focuses on
developing systems that can learn from data. It has applications in various
fields including computer vision and natural language processing.

Deep learning is a specialized form of machine learning.
"""

# Extract paragraphs containing matched terms
paragraphs = extract_paragraphs(document, thesaurus_json)
for term, paragraph in paragraphs:
    print(f"\nTerm: {term}")
    print(f"Paragraph: {paragraph[:100]}...")
```

## API Reference

### Classes

#### `AutocompleteIndex`

The main index class for fast prefix searches.

**Properties:**
- `name: str` - Name of the thesaurus
- `len() -> int` - Number of terms in the index

**Methods:**

##### `search(prefix: str, max_results: int = 10, case_sensitive: bool = False) -> List[AutocompleteResult]`

Search for terms matching the prefix.

**Parameters:**
- `prefix` - The search prefix
- `max_results` - Maximum number of results (default: 10)
- `case_sensitive` - Whether search is case-sensitive (default: False)

##### `fuzzy_search(query: str, threshold: float = 0.8, max_results: int = 10) -> List[AutocompleteResult]`

Fuzzy search using Jaro-Winkler similarity.

**Parameters:**
- `query` - The search query
- `threshold` - Similarity threshold 0.0-1.0 (default: 0.8)
- `max_results` - Maximum number of results (default: 10)

##### `fuzzy_search_levenshtein(query: str, max_distance: int = 2, max_results: int = 10) -> List[AutocompleteResult]`

Fuzzy search using Levenshtein distance.

**Parameters:**
- `query` - The search query
- `max_distance` - Maximum edit distance (default: 2)
- `max_results` - Maximum number of results (default: 10)

#### `AutocompleteResult`

Result from autocomplete search.

**Attributes:**
- `term: str` - The matched term
- `normalized_term: str` - Normalized form of the term
- `id: int` - Term ID from thesaurus
- `url: Optional[str]` - Associated URL
- `score: float` - Relevance score

#### `Matched`

A matched term found in text.

**Attributes:**
- `term: str` - The matched term
- `normalized_term: str` - Normalized form
- `id: int` - Term ID
- `url: Optional[str]` - Associated URL
- `pos: Optional[Tuple[int, int]]` - Match position (start, end)

### Functions

#### `build_index(json_str: str, case_sensitive: bool = False) -> AutocompleteIndex`

Build an autocomplete index from thesaurus JSON.

#### `load_thesaurus(json_str: str) -> Tuple[str, int]`

Load thesaurus and return (name, term_count).

#### `find_all_matches(text: str, json_str: str, return_positions: bool = True) -> List[Matched]`

Find all thesaurus term matches in text.

#### `replace_with_links(text: str, json_str: str, link_type: str) -> str`

Replace matched terms with links.

**Link types:**
- `"markdown"` - `[term](url)`
- `"html"` - `<a href="url">term</a>`
- `"wiki"` - `[[term]]`
- `"plain"` - `normalized_term`

#### `extract_paragraphs(text: str, json_str: str) -> List[Tuple[str, str]]`

Extract paragraphs containing matched terms.

## Thesaurus Format

The thesaurus JSON structure:

```json
{
    "name": "Thesaurus Name",
    "data": {
        "term to match": {
            "id": 1,
            "nterm": "normalized term",
            "url": "https://example.com/page"
        }
    }
}
```

**Fields:**
- `name` - Thesaurus name (required)
- `data` - Dictionary of terms (required)
  - Key: Term to match (case-insensitive)
  - `id`: Unique integer ID (required)
  - `nterm`: Normalized term form (required)
  - `url`: Associated URL (optional)

## Performance

Benchmarks on a modern laptop (Apple M1):

| Operation | Index Size | Time |
|-----------|------------|------|
| Build index | 10,000 terms | ~50ms |
| Prefix search | 10,000 terms | ~0.1ms |
| Fuzzy search | 10,000 terms | ~5ms |
| Find matches | 100KB text | ~2ms |
| Replace links | 100KB text | ~3ms |

Run benchmarks yourself:

```bash
cd crates/terraphim_automata_py
uv pip install pytest-benchmark
pytest python/benchmarks/ --benchmark-only
```

## Development

### Setup Development Environment

```bash
# Install uv
pip install uv

# Clone repository
git clone https://github.com/terraphim/terraphim-ai.git
cd terraphim-ai/crates/terraphim_automata_py

# Install development dependencies
uv pip install maturin pytest pytest-benchmark pytest-cov black ruff mypy

# Build in development mode
maturin develop

# Run tests
pytest python/tests/ -v

# Run benchmarks
pytest python/benchmarks/ --benchmark-only

# Format code
black python/
ruff check python/ --fix

# Type check
mypy python/terraphim_automata/
```

### Project Structure

```
crates/terraphim_automata_py/
├── src/
│   └── lib.rs              # Rust Python bindings (PyO3)
├── python/
│   ├── terraphim_automata/
│   │   ├── __init__.py     # Python package entry point
│   │   └── __init__.pyi    # Type stubs
│   ├── tests/              # Pytest test suite
│   │   ├── test_autocomplete.py
│   │   ├── test_matcher.py
│   │   └── test_thesaurus.py
│   └── benchmarks/         # Performance benchmarks
│       ├── benchmark_autocomplete.py
│       └── benchmark_matcher.py
├── Cargo.toml              # Rust dependencies
├── pyproject.toml          # Python package metadata
└── README.md               # This file
```

## Contributing

Contributions are welcome! Please:

1. Fork the repository
2. Create a feature branch
3. Add tests for new functionality
4. Ensure all tests pass: `pytest python/tests/ -v`
5. Format code: `black python/ && ruff check python/ --fix`
6. Submit a pull request

## License

Apache License 2.0 - See [LICENSE](../../LICENSE) for details.

## Links

- **Documentation**: https://docs.terraphim.ai
- **Repository**: https://github.com/terraphim/terraphim-ai
- **Issue Tracker**: https://github.com/terraphim/terraphim-ai/issues
- **PyPI**: https://pypi.org/project/terraphim-automata/
- **Terraphim AI**: https://terraphim.ai

## Related Projects

- [terraphim_automata](../terraphim_automata) - The Rust library this wraps
- [terraphim-ai](../..) - Full Terraphim AI system
