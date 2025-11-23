"""Benchmarks for text matching and replacement functionality"""

import json

import pytest
from terraphim_automata import extract_paragraphs, find_all_matches, replace_with_links


def generate_thesaurus(num_terms: int) -> str:
    """Generate a thesaurus with specified number of terms"""
    data = {}
    terms = [
        "machine learning",
        "deep learning",
        "artificial intelligence",
        "neural network",
        "data science",
        "natural language processing",
        "computer vision",
        "reinforcement learning",
        "supervised learning",
        "unsupervised learning",
    ]

    for i in range(num_terms):
        term = terms[i % len(terms)] if i < len(terms) else f"term_{i}"
        data[term if i < len(terms) else f"{term}_{i}"] = {
            "id": i + 1,
            "nterm": f"normalized_{i}",
            "url": f"https://example.com/{i}",
        }

    return json.dumps({"name": "Benchmark", "data": data})


def generate_text(num_paragraphs: int, terms_per_paragraph: int) -> str:
    """Generate text with multiple paragraphs containing terms"""
    terms = [
        "machine learning",
        "deep learning",
        "artificial intelligence",
        "neural network",
        "data science",
    ]

    paragraphs = []
    for i in range(num_paragraphs):
        paragraph = f"This is paragraph {i}. "
        for j in range(terms_per_paragraph):
            term = terms[j % len(terms)]
            paragraph += f"We discuss {term} in detail. "
        paragraph += "This is the end of the paragraph."
        paragraphs.append(paragraph)

    return "\n\n".join(paragraphs)


@pytest.fixture(scope="module")
def small_thesaurus():
    """Small thesaurus with 10 terms"""
    return generate_thesaurus(10)


@pytest.fixture(scope="module")
def medium_thesaurus():
    """Medium thesaurus with 100 terms"""
    return generate_thesaurus(100)


@pytest.fixture(scope="module")
def small_text():
    """Small text with 10 paragraphs"""
    return generate_text(10, 3)


@pytest.fixture(scope="module")
def medium_text():
    """Medium text with 100 paragraphs"""
    return generate_text(100, 3)


@pytest.fixture(scope="module")
def large_text():
    """Large text with 1000 paragraphs"""
    return generate_text(1000, 3)


class TestFindMatches:
    """Benchmark find_all_matches function"""

    def test_find_matches_small_text(self, benchmark, small_text, small_thesaurus):
        """Benchmark finding matches in small text"""
        result = benchmark(find_all_matches, small_text, small_thesaurus, True)
        assert isinstance(result, list)

    def test_find_matches_medium_text(self, benchmark, medium_text, small_thesaurus):
        """Benchmark finding matches in medium text"""
        result = benchmark(find_all_matches, medium_text, small_thesaurus, True)
        assert isinstance(result, list)

    def test_find_matches_large_text(self, benchmark, large_text, small_thesaurus):
        """Benchmark finding matches in large text"""
        result = benchmark(find_all_matches, large_text, small_thesaurus, True)
        assert isinstance(result, list)

    def test_find_matches_with_positions(self, benchmark, medium_text, small_thesaurus):
        """Benchmark finding matches with positions"""
        result = benchmark(find_all_matches, medium_text, small_thesaurus, True)
        assert isinstance(result, list)

    def test_find_matches_without_positions(self, benchmark, medium_text, small_thesaurus):
        """Benchmark finding matches without positions"""
        result = benchmark(find_all_matches, medium_text, small_thesaurus, False)
        assert isinstance(result, list)

    def test_find_matches_many_terms(self, benchmark, medium_text, medium_thesaurus):
        """Benchmark finding matches with many terms in thesaurus"""
        result = benchmark(find_all_matches, medium_text, medium_thesaurus, True)
        assert isinstance(result, list)


class TestReplaceWithLinks:
    """Benchmark replace_with_links function"""

    def test_replace_markdown_small(self, benchmark, small_text, small_thesaurus):
        """Benchmark markdown replacement in small text"""
        result = benchmark(replace_with_links, small_text, small_thesaurus, "markdown")
        assert isinstance(result, str)

    def test_replace_markdown_medium(self, benchmark, medium_text, small_thesaurus):
        """Benchmark markdown replacement in medium text"""
        result = benchmark(replace_with_links, medium_text, small_thesaurus, "markdown")
        assert isinstance(result, str)

    def test_replace_markdown_large(self, benchmark, large_text, small_thesaurus):
        """Benchmark markdown replacement in large text"""
        result = benchmark(replace_with_links, large_text, small_thesaurus, "markdown")
        assert isinstance(result, str)

    def test_replace_html_medium(self, benchmark, medium_text, small_thesaurus):
        """Benchmark HTML replacement in medium text"""
        result = benchmark(replace_with_links, medium_text, small_thesaurus, "html")
        assert isinstance(result, str)

    def test_replace_wiki_medium(self, benchmark, medium_text, small_thesaurus):
        """Benchmark wiki-style replacement in medium text"""
        result = benchmark(replace_with_links, medium_text, small_thesaurus, "wiki")
        assert isinstance(result, str)

    def test_replace_plain_medium(self, benchmark, medium_text, small_thesaurus):
        """Benchmark plain text replacement in medium text"""
        result = benchmark(replace_with_links, medium_text, small_thesaurus, "plain")
        assert isinstance(result, str)

    def test_replace_many_terms(self, benchmark, medium_text, medium_thesaurus):
        """Benchmark replacement with many terms in thesaurus"""
        result = benchmark(replace_with_links, medium_text, medium_thesaurus, "markdown")
        assert isinstance(result, str)


class TestExtractParagraphs:
    """Benchmark extract_paragraphs function"""

    def test_extract_paragraphs_small(self, benchmark, small_text, small_thesaurus):
        """Benchmark extracting paragraphs from small text"""
        result = benchmark(extract_paragraphs, small_text, small_thesaurus)
        assert isinstance(result, list)

    def test_extract_paragraphs_medium(self, benchmark, medium_text, small_thesaurus):
        """Benchmark extracting paragraphs from medium text"""
        result = benchmark(extract_paragraphs, medium_text, small_thesaurus)
        assert isinstance(result, list)

    def test_extract_paragraphs_large(self, benchmark, large_text, small_thesaurus):
        """Benchmark extracting paragraphs from large text"""
        result = benchmark(extract_paragraphs, large_text, small_thesaurus)
        assert isinstance(result, list)

    def test_extract_paragraphs_many_terms(self, benchmark, medium_text, medium_thesaurus):
        """Benchmark extracting paragraphs with many terms in thesaurus"""
        result = benchmark(extract_paragraphs, medium_text, medium_thesaurus)
        assert isinstance(result, list)


class TestComplexScenarios:
    """Benchmark complex real-world scenarios"""

    def test_full_workflow_small(self, benchmark, small_text, small_thesaurus):
        """Benchmark full workflow: find, replace, extract on small text"""

        def workflow():
            matches = find_all_matches(small_text, small_thesaurus, True)
            replaced = replace_with_links(small_text, small_thesaurus, "markdown")
            paragraphs = extract_paragraphs(small_text, small_thesaurus)
            return matches, replaced, paragraphs

        result = benchmark(workflow)
        assert len(result) == 3

    def test_full_workflow_medium(self, benchmark, medium_text, small_thesaurus):
        """Benchmark full workflow: find, replace, extract on medium text"""

        def workflow():
            matches = find_all_matches(medium_text, small_thesaurus, True)
            replaced = replace_with_links(medium_text, small_thesaurus, "markdown")
            paragraphs = extract_paragraphs(medium_text, small_thesaurus)
            return matches, replaced, paragraphs

        result = benchmark(workflow)
        assert len(result) == 3

    def test_repeated_matching(self, benchmark, medium_text, small_thesaurus):
        """Benchmark repeated matching operations"""

        def repeated_matches():
            results = []
            for _ in range(10):
                results.append(find_all_matches(medium_text, small_thesaurus, True))
            return results

        result = benchmark(repeated_matches)
        assert len(result) == 10


if __name__ == "__main__":
    pytest.main([__file__, "-v", "--benchmark-only"])
