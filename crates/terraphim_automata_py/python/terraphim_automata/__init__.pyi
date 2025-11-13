"""Type stubs for terraphim_automata"""

from typing import List, Optional, Tuple

class AutocompleteResult:
    """Result from autocomplete search"""

    term: str
    normalized_term: str
    id: int
    url: Optional[str]
    score: float

    def __repr__(self) -> str: ...
    def __str__(self) -> str: ...

class Matched:
    """Matched term in text"""

    term: str
    normalized_term: str
    id: int
    url: Optional[str]
    pos: Optional[Tuple[int, int]]

    def __repr__(self) -> str: ...
    def __str__(self) -> str: ...

class AutocompleteIndex:
    """Autocomplete index for fast prefix searches"""

    @property
    def name(self) -> str:
        """Get the name of the autocomplete index"""
        ...

    def __len__(self) -> int:
        """Get the number of terms in the index"""
        ...

    def search(self, prefix: str, max_results: int = 10) -> List[AutocompleteResult]:
        """
        Search for terms matching the prefix

        Args:
            prefix: The search prefix
            max_results: Maximum number of results to return (default: 10)

        Returns:
            List of AutocompleteResult objects

        Note:
            Case sensitivity is determined when the index is built
        """
        ...

    def fuzzy_search(
        self, query: str, threshold: float = 0.8, max_results: int = 10
    ) -> List[AutocompleteResult]:
        """
        Fuzzy search using Jaro-Winkler similarity

        Args:
            query: The search query
            threshold: Similarity threshold (0.0 to 1.0, default: 0.8)
            max_results: Maximum number of results (default: 10)

        Returns:
            List of AutocompleteResult objects sorted by relevance
        """
        ...

    def fuzzy_search_levenshtein(
        self, query: str, max_distance: int = 2, max_results: int = 10
    ) -> List[AutocompleteResult]:
        """
        Fuzzy search using Levenshtein distance

        Args:
            query: The search query
            max_distance: Maximum edit distance (default: 2)
            max_results: Maximum number of results (default: 10)

        Returns:
            List of AutocompleteResult objects sorted by relevance
        """
        ...

    def __repr__(self) -> str: ...
    def __str__(self) -> str: ...

def load_thesaurus(json_str: str) -> Tuple[str, int]:
    """
    Load thesaurus from JSON string

    Args:
        json_str: JSON string containing thesaurus data

    Returns:
        Tuple of (thesaurus_name, number_of_terms)

    Example:
        >>> json_str = '{"name": "test", "data": {"term1": {"id": 1, "nterm": "normalized", "url": "https://example.com"}}}'
        >>> name, count = load_thesaurus(json_str)
    """
    ...

def build_index(json_str: str, case_sensitive: bool = False) -> AutocompleteIndex:
    """
    Build autocomplete index from thesaurus JSON

    Args:
        json_str: JSON string containing thesaurus data
        case_sensitive: Whether the index should be case-sensitive (default: False)

    Returns:
        AutocompleteIndex object

    Example:
        >>> json_str = '{"name": "test", "data": {"term1": {"id": 1, "nterm": "normalized", "url": "https://example.com"}}}'
        >>> index = build_index(json_str)
        >>> results = index.search("ter")
    """
    ...

def find_all_matches(
    text: str, json_str: str, return_positions: bool = True
) -> List[Matched]:
    """
    Find all matches of thesaurus terms in text

    Args:
        text: The text to search in
        json_str: JSON string containing thesaurus data
        return_positions: Whether to return match positions (default: True)

    Returns:
        List of Matched objects

    Example:
        >>> text = "This is a test document with some terms"
        >>> json_str = '{"name": "test", "data": {"test": {"id": 1, "nterm": "test", "url": "https://example.com"}}}'
        >>> matches = find_all_matches(text, json_str)
    """
    ...

def replace_with_links(text: str, json_str: str, link_type: str) -> str:
    """
    Replace all thesaurus term matches with links

    Args:
        text: The text to process
        json_str: JSON string containing thesaurus data
        link_type: Type of links to create ('wiki', 'html', 'markdown', 'plain')

    Returns:
        String with replaced links

    Example:
        >>> text = "This is a test"
        >>> json_str = '{"name": "test", "data": {"test": {"id": 1, "nterm": "test", "url": "https://example.com"}}}'
        >>> result = replace_with_links(text, json_str, "markdown")
    """
    ...

def extract_paragraphs(
    text: str, json_str: str, include_term: bool = True
) -> List[Tuple[str, str]]:
    """
    Extract paragraphs starting at matched terms

    Args:
        text: The text to process
        json_str: JSON string containing thesaurus data
        include_term: Whether to include the matched term in the paragraph (default: True)

    Returns:
        List of tuples (term, paragraph_text)

    Example:
        >>> text = "Paragraph one.\\n\\nParagraph two with term.\\n\\nParagraph three."
        >>> json_str = '{"name": "test", "data": {"term": {"id": 1, "nterm": "term", "url": ""}}}'
        >>> paragraphs = extract_paragraphs(text, json_str)
    """
    ...

__version__: str
