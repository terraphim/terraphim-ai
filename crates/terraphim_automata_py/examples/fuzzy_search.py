#!/usr/bin/env python3
"""
Fuzzy search example using terraphim_automata

This example demonstrates:
1. Handling typos with Jaro-Winkler similarity
2. Handling typos with Levenshtein distance
3. Comparing different fuzzy search approaches
"""

from terraphim_automata import build_index

THESAURUS = """{
    "name": "Technology Terms",
    "data": {
        "kubernetes": {"id": 1, "nterm": "kubernetes", "url": "https://kubernetes.io"},
        "python": {"id": 2, "nterm": "python", "url": "https://python.org"},
        "javascript": {"id": 3, "nterm": "javascript", "url": "https://javascript.com"},
        "typescript": {"id": 4, "nterm": "typescript", "url": "https://typescriptlang.org"},
        "postgresql": {"id": 5, "nterm": "postgresql", "url": "https://postgresql.org"},
        "elasticsearch": {"id": 6, "nterm": "elasticsearch", "url": "https://elastic.co"},
        "tensorflow": {"id": 7, "nterm": "tensorflow", "url": "https://tensorflow.org"},
        "pytorch": {"id": 8, "nterm": "pytorch", "url": "https://pytorch.org"}
    }
}"""


def main():
    """Run the fuzzy search example"""
    print("=" * 60)
    print("Fuzzy Search Example")
    print("=" * 60)

    # Build index
    index = build_index(THESAURUS)
    print(f"\nIndex: {index.name} ({len(index)} terms)")

    # Test cases with typos
    test_cases = [
        ("kubernets", "kubernetes"),  # Missing last letter
        ("pythn", "python"),  # Missing vowel
        ("javascrpt", "javascript"),  # Missing i
        ("typscript", "typescript"),  # Missing e
        ("postgre", "postgresql"),  # Abbreviated
        ("elasticsrch", "elasticsearch"),  # Missing ea
        ("tensorflw", "tensorflow"),  # Missing o
        ("pytorh", "pytorch"),  # Missing c
    ]

    print("\n1. Jaro-Winkler Fuzzy Search")
    print("-" * 60)
    print("   (Good for typos at the start of words)")

    for query, expected in test_cases:
        results = index.fuzzy_search(query, threshold=0.8, max_results=3)
        found = any(r.term == expected for r in results)
        status = "✓" if found else "✗"
        print(f"   {status} Query: '{query}' → Expected: '{expected}'")
        if results:
            print(f"      Results: {[f'{r.term} ({r.score:.2f})' for r in results[:3]]}")
        else:
            print("      No results found")

    print("\n2. Levenshtein Fuzzy Search")
    print("-" * 60)
    print("   (Good for general typos)")

    for query, expected in test_cases:
        results = index.fuzzy_search_levenshtein(query, max_distance=2, max_results=3)
        found = any(r.term == expected for r in results)
        status = "✓" if found else "✗"
        print(f"   {status} Query: '{query}' → Expected: '{expected}'")
        if results:
            print(f"      Results: {[r.term for r in results[:3]]}")
        else:
            print("      No results found")

    # Threshold comparison
    print("\n3. Threshold Comparison (Jaro-Winkler)")
    print("-" * 60)
    query = "kubernets"
    for threshold in [0.95, 0.9, 0.85, 0.8, 0.7]:
        results = index.fuzzy_search(query, threshold=threshold, max_results=5)
        print(f"   Threshold {threshold:.2f}: {len(results)} result(s)")
        if results:
            print(f"      → {[f'{r.term} ({r.score:.2f})' for r in results]}")

    # Distance comparison
    print("\n4. Distance Comparison (Levenshtein)")
    print("-" * 60)
    query = "pythn"
    for distance in [1, 2, 3, 4]:
        results = index.fuzzy_search_levenshtein(query, max_distance=distance, max_results=5)
        print(f"   Max distance {distance}: {len(results)} result(s)")
        if results:
            print(f"      → {[r.term for r in results]}")

    # Real-world scenario
    print("\n5. Real-World Typo Scenarios")
    print("-" * 60)
    typos = [
        "kuberentes",  # Transposition
        "pyton",  # Missing letter
        "javasript",  # Missing letter
        "typescrpt",  # Missing letter
    ]

    for typo in typos:
        jw_results = index.fuzzy_search(typo, threshold=0.8, max_results=1)
        lev_results = index.fuzzy_search_levenshtein(typo, max_distance=2, max_results=1)

        print(f"\n   Typo: '{typo}'")
        print(f"      Jaro-Winkler: {jw_results[0].term if jw_results else 'No match'}")
        print(f"      Levenshtein:  {lev_results[0].term if lev_results else 'No match'}")

    print("\n" + "=" * 60)
    print("Fuzzy search example completed!")
    print("=" * 60)


if __name__ == "__main__":
    main()
