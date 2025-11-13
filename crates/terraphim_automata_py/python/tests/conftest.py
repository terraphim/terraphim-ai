"""Pytest configuration and shared fixtures"""

import json

import pytest


@pytest.fixture
def sample_thesaurus_json():
    """Sample thesaurus JSON for testing"""
    return """{
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
            },
            "neural network": {
                "id": 4,
                "nterm": "neural network",
                "url": "https://example.com/nn"
            },
            "natural language processing": {
                "id": 5,
                "nterm": "natural language processing",
                "url": "https://example.com/nlp"
            }
        }
    }"""


@pytest.fixture
def sample_text():
    """Sample text for testing"""
    return """
    Machine learning is a subset of artificial intelligence that enables
    systems to learn and improve from experience without being explicitly
    programmed.

    Deep learning is a type of machine learning based on artificial neural
    networks. Neural network architectures have multiple layers that can
    learn complex patterns.

    Natural language processing is another important area of artificial
    intelligence that deals with human language.
    """


@pytest.fixture
def empty_thesaurus_json():
    """Empty thesaurus for testing edge cases"""
    return '{"name": "Empty", "data": {}}'


@pytest.fixture
def large_thesaurus_json():
    """Generate a large thesaurus for performance testing"""
    data = {}
    for i in range(1000):
        term = f"term {i} with multiple words"
        data[term] = {"id": i + 1, "nterm": f"normalized_{i}", "url": f"https://example.com/{i}"}

    return json.dumps({"name": "Large", "data": data})
