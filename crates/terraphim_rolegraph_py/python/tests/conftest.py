"""Pytest configuration and shared fixtures for terraphim_rolegraph tests."""

import json

import pytest

from terraphim_rolegraph import Document, RoleGraph


@pytest.fixture
def sample_thesaurus_json():
    """Thesaurus with 6 AI/ML terms for testing."""
    return json.dumps(
        {
            "name": "Engineering",
            "data": {
                "machine learning": {
                    "id": 1,
                    "nterm": "machine learning",
                    "url": "https://example.com/ml",
                },
                "deep learning": {
                    "id": 2,
                    "nterm": "deep learning",
                    "url": "https://example.com/dl",
                },
                "neural network": {
                    "id": 3,
                    "nterm": "neural network",
                    "url": "https://example.com/nn",
                },
                "artificial intelligence": {
                    "id": 4,
                    "nterm": "artificial intelligence",
                    "url": "https://example.com/ai",
                },
                "natural language processing": {
                    "id": 5,
                    "nterm": "natural language processing",
                    "url": "https://example.com/nlp",
                },
                "computer vision": {
                    "id": 6,
                    "nterm": "computer vision",
                    "url": "https://example.com/cv",
                },
            },
        }
    )


@pytest.fixture
def sample_documents():
    """Three documents with overlapping thesaurus terms to create graph edges.

    Doc1: machine learning, deep learning, neural network
    Doc2: deep learning, artificial intelligence, natural language processing
    Doc3: machine learning, computer vision, artificial intelligence
    """
    return [
        Document(
            id="doc1",
            url="https://example.com/doc1",
            title="ML Foundations",
            body=(
                "machine learning is a subset of artificial intelligence. "
                "deep learning uses neural network architectures for pattern recognition."
            ),
        ),
        Document(
            id="doc2",
            url="https://example.com/doc2",
            title="AI and NLP",
            body=(
                "deep learning powers modern natural language processing systems. "
                "artificial intelligence has transformed how we process text."
            ),
        ),
        Document(
            id="doc3",
            url="https://example.com/doc3",
            title="Vision and Learning",
            body=(
                "machine learning and computer vision enable image recognition. "
                "artificial intelligence drives autonomous vehicles."
            ),
        ),
    ]


@pytest.fixture
def populated_rolegraph(sample_thesaurus_json, sample_documents):
    """A RoleGraph with all sample documents inserted."""
    rg = RoleGraph("engineer", sample_thesaurus_json)
    for doc in sample_documents:
        rg.insert_document(doc.id, doc)
    return rg
