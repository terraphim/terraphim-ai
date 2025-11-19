#!/usr/bin/env python3
"""
Text processing example using terraphim_automata

This example demonstrates:
1. Finding term matches in text
2. Replacing terms with different link types
3. Extracting relevant paragraphs
"""

from terraphim_automata import extract_paragraphs, find_all_matches, replace_with_links

THESAURUS = """{
    "name": "Software Engineering",
    "data": {
        "continuous integration": {
            "id": 1,
            "nterm": "continuous integration",
            "url": "https://en.wikipedia.org/wiki/Continuous_integration"
        },
        "test driven development": {
            "id": 2,
            "nterm": "test driven development",
            "url": "https://en.wikipedia.org/wiki/Test-driven_development"
        },
        "agile methodology": {
            "id": 3,
            "nterm": "agile methodology",
            "url": "https://en.wikipedia.org/wiki/Agile_software_development"
        },
        "code review": {
            "id": 4,
            "nterm": "code review",
            "url": "https://en.wikipedia.org/wiki/Code_review"
        },
        "version control": {
            "id": 5,
            "nterm": "version control",
            "url": "https://en.wikipedia.org/wiki/Version_control"
        }
    }
}"""

SAMPLE_TEXT = """
Best Practices for Modern Software Development

In modern software development, continuous integration has become essential
for maintaining code quality. Teams that adopt continuous integration can
catch bugs earlier and deploy more frequently.

Test driven development is another important practice. By writing tests
first, developers can ensure their code meets requirements from the start.
Test driven development works particularly well with agile methodology.

Agile methodology emphasizes iterative development and close collaboration
with stakeholders. Regular code review sessions help maintain code quality
and share knowledge across the team.

Version control systems like Git are fundamental to collaborative development.
Proper version control practices enable teams to work together effectively
and maintain a clear history of changes.
"""


def main():
    """Run the text processing example"""
    print("=" * 70)
    print("Text Processing Example")
    print("=" * 70)

    # 1. Find all matches
    print("\n1. Finding Term Matches")
    print("-" * 70)
    matches = find_all_matches(SAMPLE_TEXT, THESAURUS, return_positions=True)
    print(f"   Found {len(matches)} matches:\n")

    for match in matches:
        if match.pos:
            start, end = match.pos
            context_start = max(0, start - 20)
            context_end = min(len(SAMPLE_TEXT), end + 20)
            context = SAMPLE_TEXT[context_start:context_end].replace("\n", " ")
            print(f"   • '{match.term}' (ID: {match.id})")
            print(f"     Position: {start}-{end}")
            print(f"     Context: ...{context}...")
            print()

    # 2. Replace with markdown links
    print("\n2. Markdown Links")
    print("-" * 70)
    markdown = replace_with_links(SAMPLE_TEXT, THESAURUS, "markdown")
    # Print first few lines
    lines = markdown.split("\n")[:8]
    for line in lines:
        if line.strip():
            print(f"   {line}")

    # 3. Replace with HTML links
    print("\n3. HTML Links")
    print("-" * 70)
    html = replace_with_links(SAMPLE_TEXT, THESAURUS, "html")
    # Print first sentence with HTML links
    first_sentence = html.split(".")[1].strip()
    print(f"   {first_sentence}...\n")

    # 4. Replace with wiki links
    print("\n4. Wiki-Style Links")
    print("-" * 70)
    wiki = replace_with_links(SAMPLE_TEXT, THESAURUS, "wiki")
    # Print first paragraph
    first_para = wiki.split("\n\n")[1]
    print(f"   {first_para}\n")

    # 5. Extract paragraphs
    print("\n5. Paragraph Extraction")
    print("-" * 70)
    paragraphs = extract_paragraphs(SAMPLE_TEXT, THESAURUS)
    print(f"   Extracted {len(paragraphs)} paragraphs:\n")

    for term, paragraph in paragraphs[:3]:  # Show first 3
        # Clean up paragraph for display
        clean_para = " ".join(paragraph.split())[:100]
        print(f"   Term: '{term}'")
        print(f"   Text: {clean_para}...")
        print()

    # 6. Demonstrate different use cases
    print("\n6. Use Case Examples")
    print("-" * 70)

    # Blog post transformation
    print("\n   A) Blog Post with Auto-Linking:")
    blog_snippet = """Continuous integration helps teams deploy faster."""
    blog_with_links = replace_with_links(blog_snippet, THESAURUS, "markdown")
    print(f"      Before: {blog_snippet}")
    print(f"      After:  {blog_with_links}")

    # Documentation generation
    print("\n   B) Documentation with Wiki Links:")
    doc_snippet = """Setup version control and enable code review."""
    doc_with_links = replace_with_links(doc_snippet, THESAURUS, "wiki")
    print(f"      Before: {doc_snippet}")
    print(f"      After:  {doc_with_links}")

    # Search result highlighting
    print("\n   C) Term Extraction for Search:")
    search_text = """Learn about test driven development and agile methodology."""
    search_matches = find_all_matches(search_text, THESAURUS, return_positions=False)
    extracted_terms = [m.term for m in search_matches]
    print(f"      Text: {search_text}")
    print(f"      Extracted terms: {extracted_terms}")

    print("\n" + "=" * 70)
    print("Text processing example completed!")
    print("=" * 70)


if __name__ == "__main__":
    main()
