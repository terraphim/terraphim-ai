"""Benchmarks for autocomplete functionality"""

import json

import pytest

from terraphim_automata import build_index


def generate_thesaurus(num_terms: int) -> str:
    """Generate a thesaurus with specified number of terms"""
    data = {}
    for i in range(num_terms):
        term = f"term {i} with some words"
        data[term] = {
            "id": i + 1,
            "nterm": f"normalized_{i}",
            "url": f"https://example.com/{i}",
        }

    return json.dumps({"name": "Benchmark", "data": data})


@pytest.fixture(scope="module")
def small_index():
    """Small index with 100 terms"""
    return build_index(generate_thesaurus(100))


@pytest.fixture(scope="module")
def medium_index():
    """Medium index with 1,000 terms"""
    return build_index(generate_thesaurus(1000))


@pytest.fixture(scope="module")
def large_index():
    """Large index with 10,000 terms"""
    return build_index(generate_thesaurus(10000))


class TestIndexBuilding:
    """Benchmark index building"""

    def test_build_small_index(self, benchmark):
        """Benchmark building small index (100 terms)"""
        thesaurus = generate_thesaurus(100)
        result = benchmark(build_index, thesaurus)
        assert len(result) == 100

    def test_build_medium_index(self, benchmark):
        """Benchmark building medium index (1,000 terms)"""
        thesaurus = generate_thesaurus(1000)
        result = benchmark(build_index, thesaurus)
        assert len(result) == 1000

    def test_build_large_index(self, benchmark):
        """Benchmark building large index (10,000 terms)"""
        thesaurus = generate_thesaurus(10000)
        result = benchmark(build_index, thesaurus)
        assert len(result) == 10000


class TestPrefixSearch:
    """Benchmark prefix search"""

    def test_search_small_index(self, benchmark, small_index):
        """Benchmark searching in small index"""
        result = benchmark(small_index.search, "term", max_results=10)
        assert len(result) <= 10

    def test_search_medium_index(self, benchmark, medium_index):
        """Benchmark searching in medium index"""
        result = benchmark(medium_index.search, "term", max_results=10)
        assert len(result) <= 10

    def test_search_large_index(self, benchmark, large_index):
        """Benchmark searching in large index"""
        result = benchmark(large_index.search, "term", max_results=10)
        assert len(result) <= 10

    def test_search_with_many_results(self, benchmark, large_index):
        """Benchmark searching with many results"""
        result = benchmark(large_index.search, "term", max_results=100)
        assert len(result) <= 100

    def test_search_no_results(self, benchmark, large_index):
        """Benchmark searching with no results"""
        result = benchmark(large_index.search, "xyz123")
        assert len(result) == 0


class TestFuzzySearch:
    """Benchmark fuzzy search"""

    def test_fuzzy_search_jaro_winkler_small(self, benchmark, small_index):
        """Benchmark Jaro-Winkler fuzzy search on small index"""
        result = benchmark(small_index.fuzzy_search, "tem", threshold=0.8, max_results=10)
        assert isinstance(result, list)

    def test_fuzzy_search_jaro_winkler_medium(self, benchmark, medium_index):
        """Benchmark Jaro-Winkler fuzzy search on medium index"""
        result = benchmark(medium_index.fuzzy_search, "tem", threshold=0.8, max_results=10)
        assert isinstance(result, list)

    def test_fuzzy_search_jaro_winkler_large(self, benchmark, large_index):
        """Benchmark Jaro-Winkler fuzzy search on large index"""
        result = benchmark(large_index.fuzzy_search, "tem", threshold=0.8, max_results=10)
        assert isinstance(result, list)

    def test_fuzzy_search_levenshtein_small(self, benchmark, small_index):
        """Benchmark Levenshtein fuzzy search on small index"""
        result = benchmark(
            small_index.fuzzy_search_levenshtein, "tem", max_distance=2, max_results=10
        )
        assert isinstance(result, list)

    def test_fuzzy_search_levenshtein_medium(self, benchmark, medium_index):
        """Benchmark Levenshtein fuzzy search on medium index"""
        result = benchmark(
            medium_index.fuzzy_search_levenshtein, "tem", max_distance=2, max_results=10
        )
        assert isinstance(result, list)

    def test_fuzzy_search_levenshtein_large(self, benchmark, large_index):
        """Benchmark Levenshtein fuzzy search on large index"""
        result = benchmark(
            large_index.fuzzy_search_levenshtein, "tem", max_distance=2, max_results=10
        )
        assert isinstance(result, list)

    def test_fuzzy_search_high_threshold(self, benchmark, large_index):
        """Benchmark fuzzy search with high threshold (fewer results)"""
        result = benchmark(large_index.fuzzy_search, "tem", threshold=0.95, max_results=10)
        assert isinstance(result, list)

    def test_fuzzy_search_low_threshold(self, benchmark, large_index):
        """Benchmark fuzzy search with low threshold (more results)"""
        result = benchmark(large_index.fuzzy_search, "tem", threshold=0.5, max_results=10)
        assert isinstance(result, list)


class TestSearchPatterns:
    """Benchmark different search patterns"""

    def test_short_prefix(self, benchmark, large_index):
        """Benchmark searching with short prefix (2 chars)"""
        result = benchmark(large_index.search, "te", max_results=10)
        assert isinstance(result, list)

    def test_medium_prefix(self, benchmark, large_index):
        """Benchmark searching with medium prefix (5 chars)"""
        result = benchmark(large_index.search, "term ", max_results=10)
        assert isinstance(result, list)

    def test_long_prefix(self, benchmark, large_index):
        """Benchmark searching with long prefix (10+ chars)"""
        result = benchmark(large_index.search, "term 1 wit", max_results=10)
        assert isinstance(result, list)

    def test_exact_match(self, benchmark, large_index):
        """Benchmark searching for exact match"""
        result = benchmark(large_index.search, "term 50 with some words", max_results=10)
        assert isinstance(result, list)


if __name__ == "__main__":
    pytest.main([__file__, "-v", "--benchmark-only"])
