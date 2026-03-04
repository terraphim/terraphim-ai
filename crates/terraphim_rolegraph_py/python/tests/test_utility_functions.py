"""Tests for free functions: magic_pair, magic_unpair, split_paragraphs."""

from terraphim_rolegraph import magic_pair, magic_unpair, split_paragraphs


class TestMagicPair:
    def test_basic(self):
        result = magic_pair(3, 5)
        assert isinstance(result, int)

    def test_roundtrip(self):
        for x in range(10):
            for y in range(10):
                z = magic_pair(x, y)
                a, b = magic_unpair(z)
                assert (a, b) == (x, y), f"Roundtrip failed for ({x}, {y})"

    def test_bijective(self):
        seen = set()
        for x in range(20):
            for y in range(20):
                z = magic_pair(x, y)
                assert z not in seen, f"Collision at ({x}, {y})"
                seen.add(z)


class TestMagicUnpair:
    def test_basic(self):
        x, y = magic_unpair(0)
        assert isinstance(x, int)
        assert isinstance(y, int)

    def test_identity_pair(self):
        z = magic_pair(0, 0)
        assert magic_unpair(z) == (0, 0)


class TestSplitParagraphs:
    def test_basic(self):
        parts = split_paragraphs("Hello world. How are you?")
        assert isinstance(parts, list)
        assert len(parts) > 0

    def test_empty_string(self):
        parts = split_paragraphs("")
        assert parts == []

    def test_single_sentence(self):
        parts = split_paragraphs("Just one sentence.")
        assert len(parts) >= 1

    def test_multiple_paragraphs(self):
        text = "First paragraph.\n\nSecond paragraph.\n\nThird paragraph."
        parts = split_paragraphs(text)
        assert len(parts) >= 2
