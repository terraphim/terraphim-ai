#!/usr/bin/env python3
"""
Basic autocomplete example using terraphim_automata

This example demonstrates:
1. Building an autocomplete index from a thesaurus
2. Performing prefix searches
3. Accessing result metadata
"""

from terraphim_automata import build_index

# Define a thesaurus with programming concepts
PROGRAMMING_THESAURUS = """{
    "name": "Programming Concepts",
    "data": {
        "machine learning": {
            "id": 1,
            "nterm": "machine learning",
            "url": "https://en.wikipedia.org/wiki/Machine_learning"
        },
        "deep learning": {
            "id": 2,
            "nterm": "deep learning",
            "url": "https://en.wikipedia.org/wiki/Deep_learning"
        },
        "reinforcement learning": {
            "id": 3,
            "nterm": "reinforcement learning",
            "url": "https://en.wikipedia.org/wiki/Reinforcement_learning"
        },
        "neural network": {
            "id": 4,
            "nterm": "neural network",
            "url": "https://en.wikipedia.org/wiki/Neural_network"
        },
        "natural language processing": {
            "id": 5,
            "nterm": "natural language processing",
            "url": "https://en.wikipedia.org/wiki/Natural_language_processing"
        },
        "computer vision": {
            "id": 6,
            "nterm": "computer vision",
            "url": "https://en.wikipedia.org/wiki/Computer_vision"
        },
        "data science": {
            "id": 7,
            "nterm": "data science",
            "url": "https://en.wikipedia.org/wiki/Data_science"
        },
        "artificial intelligence": {
            "id": 8,
            "nterm": "artificial intelligence",
            "url": "https://en.wikipedia.org/wiki/Artificial_intelligence"
        }
    }
}"""


def main():
    """Run the basic autocomplete example"""
    print("=" * 60)
    print("Basic Autocomplete Example")
    print("=" * 60)

    # Build the index
    print("\n1. Building autocomplete index...")
    index = build_index(PROGRAMMING_THESAURUS)
    print(f"   ✓ Index created: {index.name}")
    print(f"   ✓ Number of terms: {len(index)}")

    # Example searches
    queries = ["mach", "learn", "neural", "data", "artif"]

    print("\n2. Performing prefix searches:")
    print("-" * 60)
    for query in queries:
        results = index.search(query, max_results=5)
        print(f"\n   Query: '{query}'")
        print(f"   Found {len(results)} result(s):")
        for i, result in enumerate(results, 1):
            print(f"     {i}. {result.term}")
            print(f"        • ID: {result.id}")
            print(f"        • Normalized: {result.normalized_term}")
            print(f"        • Score: {result.score:.2f}")
            print(f"        • URL: {result.url}")

    # Case sensitivity example
    print("\n3. Case sensitivity test:")
    print("-" * 60)
    query = "MACHINE"
    results = index.search(query, case_sensitive=False)
    print(f"   Query: '{query}' (case-insensitive)")
    print(f"   Results: {[r.term for r in results]}")

    # Max results example
    print("\n4. Limiting results:")
    print("-" * 60)
    for max_results in [1, 3, 5]:
        results = index.search("", max_results=max_results)
        print(f"   Max results: {max_results} → Got {len(results)} result(s)")

    print("\n" + "=" * 60)
    print("Example completed successfully!")
    print("=" * 60)


if __name__ == "__main__":
    main()
