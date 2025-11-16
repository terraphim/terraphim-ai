"""
Terraphim Automata - Fast autocomplete and text processing for knowledge graphs

This package provides Python bindings for the Terraphim Automata Rust library,
offering high-performance text matching, autocomplete, and knowledge graph operations.

Example:
    >>> from terraphim_automata import build_index
    >>>
    >>> # Create a thesaurus
    >>> thesaurus_json = '''
    ... {
    ...     "name": "Engineering",
    ...     "data": {
    ...         "machine learning": {
    ...             "id": 1,
    ...             "nterm": "machine learning",
    ...             "url": "https://example.com/ml"
    ...         },
    ...         "deep learning": {
    ...             "id": 2,
    ...             "nterm": "deep learning",
    ...             "url": "https://example.com/dl"
    ...         }
    ...     }
    ... }
    ... '''
    >>>
    >>> # Build autocomplete index
    >>> index = build_index(thesaurus_json)
    >>>
    >>> # Search for completions
    >>> results = index.search("mach")
    >>> for result in results:
    ...     print(f"{result.term} (score: {result.score})")
"""

from terraphim_automata.terraphim_automata import (
    AutocompleteIndex,
    AutocompleteResult,
    Matched,
    build_index,
    extract_paragraphs,
    find_all_matches,
    load_thesaurus,
    replace_with_links,
)

__version__ = "1.0.0"

__all__ = [
    "AutocompleteIndex",
    "AutocompleteResult",
    "Matched",
    "build_index",
    "extract_paragraphs",
    "find_all_matches",
    "load_thesaurus",
    "replace_with_links",
]
