"""Tests for autocomplete functionality"""

import pytest

from terraphim_automata import build_index

# Sample thesaurus for testing
SAMPLE_THESAURUS = """{
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
        "reinforcement learning": {
            "id": 3,
            "nterm": "reinforcement learning",
            "url": "https://example.com/rl"
        },
        "natural language processing": {
            "id": 4,
            "nterm": "natural language processing",
            "url": "https://example.com/nlp"
        },
        "computer vision": {
            "id": 5,
            "nterm": "computer vision",
            "url": "https://example.com/cv"
        }
    }
}"""


@pytest.fixture
def index():
    """Create an autocomplete index for testing"""
    return build_index(SAMPLE_THESAURUS)


class TestAutocompleteIndex:
    """Test AutocompleteIndex class"""

    def test_index_creation(self, index):
        """Test that index is created successfully"""
        assert index is not None
        assert index.name == "Engineering"
        assert len(index) == 5

    def test_index_length(self, index):
        """Test __len__ method"""
        assert len(index) == 5

    def test_index_repr(self, index):
        """Test __repr__ method"""
        repr_str = repr(index)
        assert "AutocompleteIndex" in repr_str
        assert "Engineering" in repr_str
        assert "5" in repr_str

    def test_search_exact_prefix(self, index):
        """Test searching with exact prefix"""
        results = index.search("machine")
        assert len(results) > 0
        assert any(r.term == "machine learning" for r in results)

    def test_search_partial_prefix(self, index):
        """Test searching with partial prefix"""
        results = index.search("learn")
        assert len(results) >= 3  # machine learning, deep learning, reinforcement learning
        terms = [r.term for r in results]
        assert "machine learning" in terms
        assert "deep learning" in terms
        assert "reinforcement learning" in terms

    def test_search_case_insensitive(self, index):
        """Test case-insensitive search (default)"""
        results_lower = index.search("machine")
        results_upper = index.search("MACHINE")
        assert len(results_lower) == len(results_upper)
        assert results_lower[0].term == results_upper[0].term

    def test_search_case_sensitive(self):
        """Test case-sensitive search"""
        index = build_index(SAMPLE_THESAURUS, case_sensitive=True)
        results_lower = index.search("machine")
        results_upper = index.search("MACHINE")
        assert len(results_lower) > 0
        assert len(results_upper) == 0  # No uppercase terms in thesaurus

    def test_search_max_results(self, index):
        """Test max_results parameter"""
        results = index.search("", max_results=3)
        assert len(results) <= 3

    def test_search_no_results(self, index):
        """Test search with no matching results"""
        results = index.search("xyz123")
        assert len(results) == 0

    def test_search_empty_prefix(self, index):
        """Test search with empty prefix"""
        results = index.search("", max_results=10)
        # Empty prefix should return some results (all terms)
        assert len(results) > 0


class TestAutocompleteResult:
    """Test AutocompleteResult class"""

    def test_result_attributes(self, index):
        """Test that result has all expected attributes"""
        results = index.search("machine")
        assert len(results) > 0

        result = results[0]
        assert hasattr(result, "term")
        assert hasattr(result, "normalized_term")
        assert hasattr(result, "id")
        assert hasattr(result, "url")
        assert hasattr(result, "score")

    def test_result_values(self, index):
        """Test result attribute values"""
        results = index.search("machine learning")
        assert len(results) > 0

        result = next((r for r in results if r.term == "machine learning"), None)
        assert result is not None
        assert result.term == "machine learning"
        assert result.normalized_term == "machine learning"
        assert result.id == 1
        assert result.url == "https://example.com/ml"
        assert isinstance(result.score, float)

    def test_result_repr(self, index):
        """Test result __repr__ method"""
        results = index.search("machine")
        assert len(results) > 0

        result = results[0]
        repr_str = repr(result)
        assert "AutocompleteResult" in repr_str
        assert result.term in repr_str


class TestFuzzySearch:
    """Test fuzzy search functionality"""

    def test_fuzzy_search_jaro_winkler(self, index):
        """Test fuzzy search with Jaro-Winkler"""
        results = index.fuzzy_search("machin", threshold=0.8)
        assert len(results) > 0
        assert any("machine" in r.term for r in results)

    def test_fuzzy_search_threshold(self, index):
        """Test fuzzy search with different thresholds"""
        results_high = index.fuzzy_search("machin", threshold=0.95)
        results_low = index.fuzzy_search("machin", threshold=0.5)
        # Lower threshold should return more results
        assert len(results_low) >= len(results_high)

    def test_fuzzy_search_levenshtein(self, index):
        """Test fuzzy search with Levenshtein distance"""
        results = index.fuzzy_search_levenshtein("macine", max_distance=2)
        assert len(results) > 0
        assert any("machine" in r.term for r in results)

    def test_fuzzy_search_max_distance(self, index):
        """Test fuzzy search with different max distances"""
        results_small = index.fuzzy_search_levenshtein("macine", max_distance=1)
        results_large = index.fuzzy_search_levenshtein("macine", max_distance=3)
        # Larger distance should return more results
        assert len(results_large) >= len(results_small)

    def test_fuzzy_search_no_results(self, index):
        """Test fuzzy search with no matching results"""
        results = index.fuzzy_search("xyz123", threshold=0.8)
        assert len(results) == 0

    def test_fuzzy_search_max_results(self, index):
        """Test fuzzy search max_results parameter"""
        results = index.fuzzy_search("learn", threshold=0.5, max_results=2)
        assert len(results) <= 2


class TestErrorHandling:
    """Test error handling"""

    def test_invalid_json(self):
        """Test building index with invalid JSON"""
        with pytest.raises(ValueError) as excinfo:
            build_index("{invalid json}")
        assert "Failed to load thesaurus" in str(excinfo.value)

    def test_empty_json(self):
        """Test building index with empty/minimal JSON"""
        minimal = '{"name": "Empty", "data": {}}'
        index = build_index(minimal)
        assert len(index) == 0


if __name__ == "__main__":
    pytest.main([__file__, "-v"])
