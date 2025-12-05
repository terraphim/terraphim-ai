"""Tests for text matching and replacement functionality"""

import pytest
from terraphim_automata import extract_paragraphs, find_all_matches, replace_with_links

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
        "artificial intelligence": {
            "id": 3,
            "nterm": "artificial intelligence",
            "url": "https://example.com/ai"
        }
    }
}"""


class TestFindMatches:
    """Test find_all_matches function"""

    def test_find_single_match(self):
        """Test finding a single match"""
        text = "I am studying machine learning."
        matches = find_all_matches(text, SAMPLE_THESAURUS)

        assert len(matches) == 1
        assert matches[0].term == "machine learning"
        assert matches[0].id == 1
        assert matches[0].url == "https://example.com/ml"

    def test_find_multiple_matches(self):
        """Test finding multiple matches"""
        text = "Machine learning and deep learning are part of artificial intelligence."
        matches = find_all_matches(text, SAMPLE_THESAURUS)

        assert len(matches) == 3
        terms = [m.term for m in matches]
        assert "machine learning" in terms
        assert "deep learning" in terms
        assert "artificial intelligence" in terms

    def test_find_matches_case_insensitive(self):
        """Test that matching is case-insensitive"""
        text = "MACHINE LEARNING and Machine Learning are the same."
        matches = find_all_matches(text, SAMPLE_THESAURUS)

        assert len(matches) == 2
        assert all(m.term == "machine learning" for m in matches)

    def test_find_matches_with_positions(self):
        """Test finding matches with positions"""
        text = "I study machine learning daily."
        matches = find_all_matches(text, SAMPLE_THESAURUS, return_positions=True)

        assert len(matches) == 1
        assert matches[0].pos is not None
        start, end = matches[0].pos
        assert text[start:end].lower() == "machine learning"

    def test_find_matches_without_positions(self):
        """Test finding matches without positions"""
        text = "I study machine learning daily."
        matches = find_all_matches(text, SAMPLE_THESAURUS, return_positions=False)

        assert len(matches) == 1
        assert matches[0].pos is None

    def test_find_no_matches(self):
        """Test text with no matches"""
        text = "This text has no matching terms."
        matches = find_all_matches(text, SAMPLE_THESAURUS)

        assert len(matches) == 0

    def test_match_attributes(self):
        """Test that matches have all expected attributes"""
        text = "I study machine learning."
        matches = find_all_matches(text, SAMPLE_THESAURUS)

        assert len(matches) == 1
        match = matches[0]
        assert hasattr(match, "term")
        assert hasattr(match, "normalized_term")
        assert hasattr(match, "id")
        assert hasattr(match, "url")
        assert hasattr(match, "pos")

    def test_match_repr(self):
        """Test match __repr__ method"""
        text = "I study machine learning."
        matches = find_all_matches(text, SAMPLE_THESAURUS)

        assert len(matches) == 1
        repr_str = repr(matches[0])
        assert "Matched" in repr_str
        assert "machine learning" in repr_str


class TestReplaceWithLinks:
    """Test replace_with_links function"""

    def test_replace_markdown_links(self):
        """Test replacing with markdown links"""
        text = "I study machine learning and deep learning."
        result = replace_with_links(text, SAMPLE_THESAURUS, "markdown")

        assert "[machine learning](https://example.com/ml)" in result
        assert "[deep learning](https://example.com/dl)" in result

    def test_replace_html_links(self):
        """Test replacing with HTML links"""
        text = "I study machine learning."
        result = replace_with_links(text, SAMPLE_THESAURUS, "html")

        assert '<a href="https://example.com/ml">machine learning</a>' in result

    def test_replace_wiki_links(self):
        """Test replacing with wiki-style links"""
        text = "I study machine learning."
        result = replace_with_links(text, SAMPLE_THESAURUS, "wiki")

        assert "[[machine learning]]" in result

    def test_replace_plain_text(self):
        """Test replacing with plain normalized text"""
        text = "I study machine learning."
        result = replace_with_links(text, SAMPLE_THESAURUS, "plain")

        # Should still have the term but normalized
        assert "machine learning" in result

    def test_replace_case_insensitive(self):
        """Test that replacement is case-insensitive"""
        text = "I study MACHINE LEARNING."
        result = replace_with_links(text, SAMPLE_THESAURUS, "markdown")

        assert "[machine learning](https://example.com/ml)" in result

    def test_replace_multiple_terms(self):
        """Test replacing multiple terms"""
        text = "Machine learning and deep learning are parts of artificial intelligence."
        result = replace_with_links(text, SAMPLE_THESAURUS, "markdown")

        assert "[machine learning](https://example.com/ml)" in result
        assert "[deep learning](https://example.com/dl)" in result
        assert "[artificial intelligence](https://example.com/ai)" in result

    def test_replace_no_matches(self):
        """Test text with no matches"""
        text = "This text has no matching terms."
        result = replace_with_links(text, SAMPLE_THESAURUS, "markdown")

        # Text should remain unchanged
        assert result == text

    def test_replace_invalid_link_type(self):
        """Test with invalid link type"""
        text = "I study machine learning."
        with pytest.raises(ValueError) as excinfo:
            replace_with_links(text, SAMPLE_THESAURUS, "invalid")

        assert "Invalid link type" in str(excinfo.value)


class TestExtractParagraphs:
    """Test extract_paragraphs function"""

    def test_extract_single_paragraph(self):
        """Test extracting a single paragraph"""
        text = """This is the first paragraph.

This paragraph contains machine learning which is interesting.

This is the third paragraph."""

        paragraphs = extract_paragraphs(text, SAMPLE_THESAURUS)

        assert len(paragraphs) > 0
        # Find the paragraph with machine learning
        ml_paragraph = next((p for p in paragraphs if p[0] == "machine learning"), None)
        assert ml_paragraph is not None
        assert "machine learning" in ml_paragraph[1]

    def test_extract_multiple_paragraphs(self):
        """Test extracting multiple paragraphs with different terms"""
        text = """Paragraph about machine learning.

Another paragraph about deep learning.

Final paragraph about artificial intelligence."""

        paragraphs = extract_paragraphs(text, SAMPLE_THESAURUS)

        assert len(paragraphs) >= 3
        terms = [p[0] for p in paragraphs]
        assert "machine learning" in terms
        assert "deep learning" in terms
        assert "artificial intelligence" in terms

    def test_extract_paragraph_content(self):
        """Test that extracted paragraphs contain correct content"""
        text = """First paragraph.

This paragraph is about machine learning and its applications.
It continues on multiple lines.

Third paragraph."""

        paragraphs = extract_paragraphs(text, SAMPLE_THESAURUS)

        ml_paragraph = next((p for p in paragraphs if p[0] == "machine learning"), None)
        assert ml_paragraph is not None
        # Paragraph should start from the matched term
        assert ml_paragraph[1].startswith("machine learning")
        assert "applications" in ml_paragraph[1]

    def test_extract_no_paragraphs(self):
        """Test text with no matching terms"""
        text = """Paragraph one.

Paragraph two.

Paragraph three."""

        paragraphs = extract_paragraphs(text, SAMPLE_THESAURUS)

        assert len(paragraphs) == 0


class TestErrorHandling:
    """Test error handling"""

    def test_invalid_thesaurus_json(self):
        """Test with invalid JSON"""
        text = "Some text"

        # PyO3 may raise either ValueError or RuntimeError depending on the error path
        with pytest.raises((ValueError, RuntimeError)):
            find_all_matches(text, "{invalid json}")

        with pytest.raises((ValueError, RuntimeError)):
            replace_with_links(text, "{invalid json}", "markdown")

        with pytest.raises((ValueError, RuntimeError)):
            extract_paragraphs(text, "{invalid json}")


if __name__ == "__main__":
    pytest.main([__file__, "-v"])
