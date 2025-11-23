"""Tests for thesaurus loading functionality"""

import pytest
from terraphim_automata import build_index, load_thesaurus


class TestLoadThesaurus:
    """Test load_thesaurus function"""

    def test_load_simple_thesaurus(self):
        """Test loading a simple thesaurus"""
        json_str = """{
            "name": "Test",
            "data": {
                "term1": {
                    "id": 1,
                    "nterm": "normalized1",
                    "url": "https://example.com/1"
                },
                "term2": {
                    "id": 2,
                    "nterm": "normalized2",
                    "url": "https://example.com/2"
                }
            }
        }"""

        name, count = load_thesaurus(json_str)
        assert name == "Test"
        assert count == 2

    def test_load_empty_thesaurus(self):
        """Test loading an empty thesaurus"""
        json_str = '{"name": "Empty", "data": {}}'

        name, count = load_thesaurus(json_str)
        assert name == "Empty"
        assert count == 0

    def test_load_large_thesaurus(self):
        """Test loading a larger thesaurus"""
        # Generate a thesaurus with many terms
        data = {
            f"term{i}": {"id": i, "nterm": f"normalized{i}", "url": f"https://example.com/{i}"}
            for i in range(1, 101)
        }
        import json

        json_str = json.dumps({"name": "Large", "data": data})

        name, count = load_thesaurus(json_str)
        assert name == "Large"
        assert count == 100

    def test_load_thesaurus_with_optional_fields(self):
        """Test loading thesaurus with optional fields"""
        json_str = """{
            "name": "Test",
            "data": {
                "term1": {
                    "id": 1,
                    "nterm": "normalized1"
                }
            }
        }"""

        name, count = load_thesaurus(json_str)
        assert name == "Test"
        assert count == 1

    def test_load_invalid_json(self):
        """Test loading invalid JSON"""
        with pytest.raises(ValueError) as excinfo:
            load_thesaurus("{invalid json}")
        assert "Failed to load thesaurus" in str(excinfo.value)

    def test_load_malformed_thesaurus(self):
        """Test loading malformed thesaurus structure"""
        # Missing required fields
        json_str = '{"name": "Test"}'

        with pytest.raises(ValueError):
            load_thesaurus(json_str)


class TestBuildIndex:
    """Test build_index function"""

    def test_build_simple_index(self):
        """Test building a simple index"""
        json_str = """{
            "name": "Test",
            "data": {
                "term1": {"id": 1, "nterm": "normalized1", "url": "https://example.com/1"}
            }
        }"""

        index = build_index(json_str)
        assert index is not None
        assert index.name == "Test"
        assert len(index) == 1

    def test_build_case_sensitive_index(self):
        """Test building a case-sensitive index"""
        json_str = """{
            "name": "Test",
            "data": {
                "Test": {"id": 1, "nterm": "test", "url": "https://example.com/1"}
            }
        }"""

        index = build_index(json_str, case_sensitive=True)
        results = index.search("Test")
        assert len(results) > 0

        results = index.search("test")
        assert len(results) == 0  # Case-sensitive, won't match

    def test_build_case_insensitive_index(self):
        """Test building a case-insensitive index (default)"""
        json_str = """{
            "name": "Test",
            "data": {
                "Test": {"id": 1, "nterm": "test", "url": "https://example.com/1"}
            }
        }"""

        index = build_index(json_str, case_sensitive=False)
        results_upper = index.search("Test")
        results_lower = index.search("test")
        assert len(results_upper) > 0
        assert len(results_lower) > 0

    def test_build_index_preserves_data(self):
        """Test that index preserves all thesaurus data"""
        json_str = """{
            "name": "Engineering",
            "data": {
                "machine learning": {
                    "id": 42,
                    "nterm": "ml",
                    "url": "https://example.com/ml"
                }
            }
        }"""

        index = build_index(json_str)
        results = index.search("machine")

        assert len(results) > 0
        result = results[0]
        assert result.id == 42
        assert result.normalized_term == "ml"
        assert result.url == "https://example.com/ml"

    def test_build_index_with_special_characters(self):
        """Test building index with special characters in terms"""
        json_str = """{
            "name": "Test",
            "data": {
                "C++": {"id": 1, "nterm": "cpp", "url": "https://example.com/cpp"},
                "C#": {"id": 2, "nterm": "csharp", "url": "https://example.com/csharp"},
                "node.js": {"id": 3, "nterm": "nodejs", "url": "https://example.com/nodejs"}
            }
        }"""

        index = build_index(json_str)
        assert len(index) == 3

        # Test searching for terms with special characters
        results = index.search("C")
        assert len(results) > 0

    def test_build_index_with_unicode(self):
        """Test building index with Unicode characters"""
        json_str = """{
            "name": "Test",
            "data": {
                "café": {"id": 1, "nterm": "cafe", "url": "https://example.com/cafe"},
                "naïve": {"id": 2, "nterm": "naive", "url": "https://example.com/naive"}
            }
        }"""

        index = build_index(json_str)
        assert len(index) == 2

        results = index.search("caf")
        assert len(results) > 0


class TestIntegration:
    """Integration tests combining multiple functions"""

    def test_load_and_build_workflow(self):
        """Test the typical workflow: load thesaurus, build index, search"""
        json_str = """{
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
                }
            }
        }"""

        # Load thesaurus
        name, count = load_thesaurus(json_str)
        assert name == "Engineering"
        assert count == 2

        # Build index
        index = build_index(json_str)
        assert index.name == name
        assert len(index) == count

        # Search
        results = index.search("learn")
        assert len(results) == 2

    def test_real_world_thesaurus(self):
        """Test with a realistic thesaurus structure"""
        json_str = """{
            "name": "Software Engineering",
            "data": {
                "continuous integration": {
                    "id": 1,
                    "nterm": "continuous integration",
                    "url": "https://example.com/ci"
                },
                "continuous deployment": {
                    "id": 2,
                    "nterm": "continuous deployment",
                    "url": "https://example.com/cd"
                },
                "test driven development": {
                    "id": 3,
                    "nterm": "test driven development",
                    "url": "https://example.com/tdd"
                },
                "behavior driven development": {
                    "id": 4,
                    "nterm": "behavior driven development",
                    "url": "https://example.com/bdd"
                },
                "domain driven design": {
                    "id": 5,
                    "nterm": "domain driven design",
                    "url": "https://example.com/ddd"
                }
            }
        }"""

        index = build_index(json_str)

        # Test prefix search
        results = index.search("continuous")
        assert len(results) == 2

        # Test suffix search
        results = index.search("driven")
        assert len(results) == 3

        # Test fuzzy search
        results = index.fuzzy_search("continuos", threshold=0.85)
        assert len(results) > 0


if __name__ == "__main__":
    pytest.main([__file__, "-v"])
