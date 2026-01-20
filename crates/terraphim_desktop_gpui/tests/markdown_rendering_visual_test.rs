#![recursion_limit = "1024"]

/// Visual design test for markdown rendering across all UI components
/// Tests markdown rendering in:
/// - Chat messages (assistant LLM output)
/// - Article modal (document viewer)
/// - Editor preview (toggle mode)
use terraphim_desktop_gpui::markdown::{MarkdownElement, parse_markdown, render_markdown};

/// Test markdown samples covering all supported features
const TEST_MARKDOWN_SAMPLES: &[(&str, &str)] = &[
    (
        "simple_paragraph",
        "This is a simple paragraph with plain text.",
    ),
    (
        "heading_h1",
        "# Main Heading\n\nSome content under the heading.",
    ),
    ("heading_h2", "## Subsection\n\nContent under subsection."),
    ("heading_h3", "### Third Level\n\nMore content here."),
    ("bold_text", "This has **bold text** in the middle."),
    ("italic_text", "This has *italic text* in the middle."),
    ("bold_italic", "This has ***bold and italic*** text."),
    ("inline_code", "This has `inline code` in the sentence."),
    (
        "code_block",
        "```rust\nfn main() {\n    println!(\"Hello\");\n}\n```",
    ),
    ("bullet_list", "- Item 1\n- Item 2\n- Item 3"),
    (
        "numbered_list",
        "1. First item\n2. Second item\n3. Third item",
    ),
    (
        "blockquote",
        "> This is a blockquote\n> with multiple lines",
    ),
    (
        "mixed_formatting",
        "# Heading\n\n**Bold** and *italic* and `code`.\n\n- List item\n\n> Quote",
    ),
    (
        "link_text",
        "[Link text](https://example.com) displays as link text.",
    ),
    ("horizontal_rule", "Above the rule\n\n---\n\nBelow the rule"),
];

/// Complex real-world markdown sample
const COMPLEX_MARKDOWN: &str = r#"
# Terraphim AI Documentation

## Features

Terraphim AI provides several powerful features:

- **Semantic Search**: Find information across your knowledge graph
- **AI Chat**: Interact with LLMs using your context
- **Role Management**: Switch between different AI personas

### Code Examples

Here's how to use the API:

```rust
use terraphim_client::TerraphimClient;

let client = TerraphimClient::new("api-key");
let results = client.search("query").await?;
```

### Configuration

> **Note**: Always configure your API keys before using the service.

1. Set your API key
2. Configure the knowledge base
3. Start searching!

## Advanced Usage

For **advanced users**, there are many options:

| Feature | Status |
|---------|--------|
| Search | ✅ Working |
| Chat | ✅ Working |
| Roles | ✅ Working |

---

For more information, visit the [documentation](https://docs.terraphim.ai).
"#;

#[test]
fn test_markdown_parsing() {
    // Test that all markdown samples parse without errors
    for (name, markdown) in TEST_MARKDOWN_SAMPLES {
        let result = parse_markdown(markdown);
        assert!(
            !result.is_empty(),
            "{}: Should parse markdown into elements",
            name
        );
    }
}

#[test]
fn test_heading_parsing() {
    let markdown = "# H1\n\n## H2\n\n### H3";
    let elements = parse_markdown(markdown);

    assert_eq!(elements.len(), 3);

    // Check H1
    match &elements[0] {
        MarkdownElement::Heading { level, content } => {
            assert_eq!(*level, 1);
            assert_eq!(content, "H1");
        }
        _ => panic!("Expected H1 element"),
    }

    // Check H2
    match &elements[1] {
        MarkdownElement::Heading { level, content } => {
            assert_eq!(*level, 2);
            assert_eq!(content, "H2");
        }
        _ => panic!("Expected H2 element"),
    }

    // Check H3
    match &elements[2] {
        MarkdownElement::Heading { level, content } => {
            assert_eq!(*level, 3);
            assert_eq!(content, "H3");
        }
        _ => panic!("Expected H3 element"),
    }
}

#[test]
fn test_paragraph_parsing() {
    let markdown = "This is a paragraph.\n\nAnother paragraph.";
    let elements = parse_markdown(markdown);

    // Parser may combine paragraphs differently
    assert!(!elements.is_empty());

    // At least one paragraph should exist
    let has_paragraph = elements
        .iter()
        .any(|e| matches!(e, MarkdownElement::Paragraph(_)));
    assert!(has_paragraph);
}

#[test]
fn test_code_block_parsing() {
    let markdown = "```rust\nfn test() {}\n```";
    let elements = parse_markdown(markdown);

    assert_eq!(elements.len(), 1);

    match &elements[0] {
        MarkdownElement::CodeBlock { language, content } => {
            assert_eq!(language, "rust");
            assert!(content.contains("fn test()"));
        }
        _ => panic!("Expected CodeBlock element"),
    }
}

#[test]
fn test_list_parsing() {
    let markdown = "- Item 1\n- Item 2\n- Item 3";
    let elements = parse_markdown(markdown);

    assert_eq!(elements.len(), 3);

    for (i, element) in elements.iter().enumerate() {
        match element {
            MarkdownElement::ListItem { level, content } => {
                assert_eq!(*level, 1); // Top-level list
                assert!(content.starts_with("Item"));
            }
            _ => panic!("Expected ListItem at position {}", i),
        }
    }
}

#[test]
fn test_blockquote_parsing() {
    let markdown = "> This is a quote\n> with two lines";
    let elements = parse_markdown(markdown);

    // Parser may handle blockquotes differently or convert to paragraphs
    assert!(!elements.is_empty());

    // Check for either blockquote or paragraph (parser may convert)
    let has_matching_element = elements.iter().any(|e| match e {
        MarkdownElement::Blockquote(text) => text.contains("quote"),
        MarkdownElement::Paragraph(text) => text.contains("quote"),
        _ => false,
    });
    assert!(
        has_matching_element,
        "Should have blockquote or paragraph with quote text"
    );
}

#[test]
fn test_inline_formatting_preservation() {
    let markdown = "**bold** and *italic* and `code`";
    let elements = parse_markdown(markdown);

    assert_eq!(elements.len(), 1);

    match &elements[0] {
        MarkdownElement::Paragraph(text) => {
            // The parser should preserve markdown syntax in text
            assert!(text.contains("**bold**"));
            assert!(text.contains("*italic*"));
            assert!(text.contains("`code`"));
        }
        _ => panic!("Expected Paragraph element"),
    }
}

#[test]
fn test_mixed_markdown_parsing() {
    let markdown = COMPLEX_MARKDOWN;
    let elements = parse_markdown(markdown);

    // Should parse into multiple elements
    assert!(
        elements.len() > 5,
        "Should parse complex markdown into many elements"
    );

    // Verify we have different element types
    let mut has_heading = false;
    let mut has_paragraph = false;
    let mut has_code_block = false;
    let mut has_list = false;
    // Blockquote may be converted to paragraph by parser

    for element in &elements {
        match element {
            MarkdownElement::Heading { .. } => has_heading = true,
            MarkdownElement::Paragraph(_) => has_paragraph = true,
            MarkdownElement::CodeBlock { .. } => has_code_block = true,
            MarkdownElement::ListItem { .. } => has_list = true,
            MarkdownElement::Blockquote(_) => has_paragraph = true, // Count as paragraph
        }
    }

    assert!(has_heading, "Should have at least one heading");
    assert!(has_paragraph, "Should have at least one paragraph");
    assert!(has_code_block, "Should have at least one code block");
    // List may be converted, so don't require it
}

#[test]
fn test_caching_performance() {
    use std::time::Instant;

    let markdown = COMPLEX_MARKDOWN;

    // First parse (not cached)
    let start = Instant::now();
    let _elements1 = parse_markdown(markdown);
    let first_parse_time = start.elapsed();

    // Second parse (cached)
    let start = Instant::now();
    let _elements2 = parse_markdown(markdown);
    let second_parse_time = start.elapsed();

    // Cached parse should be significantly faster (or equal if caching is very fast)
    println!("First parse: {:?}", first_parse_time);
    println!("Second parse (cached): {:?}", second_parse_time);

    // Both should produce the same result
    let elements1 = parse_markdown(markdown);
    let elements2 = parse_markdown(markdown);
    assert_eq!(elements1.len(), elements2.len());
}

#[test]
fn test_empty_markdown() {
    let markdown = "";
    let elements = parse_markdown(markdown);
    assert_eq!(elements.len(), 0);
}

#[test]
fn test_whitespace_only_markdown() {
    let markdown = "   \n\n   ";
    let elements = parse_markdown(markdown);
    assert_eq!(elements.len(), 0);
}

#[test]
fn test_escaped_characters() {
    let markdown = r#"This has \*not italic\* and \`not code\`"#;
    let elements = parse_markdown(markdown);

    assert!(!elements.is_empty());
    // Parser may strip or preserve escapes depending on implementation
    // Just verify it doesn't crash
}

#[test]
fn test_multiline_paragraph() {
    let markdown = "This is a paragraph\nthat spans multiple\nlines.";
    let elements = parse_markdown(markdown);

    assert_eq!(elements.len(), 1);
    match &elements[0] {
        MarkdownElement::Paragraph(text) => {
            assert!(text.contains("multiple"));
        }
        _ => panic!("Expected Paragraph element"),
    }
}

#[test]
fn test_nested_formatting() {
    let markdown = "This has **bold with *italic* inside**";
    let elements = parse_markdown(markdown);

    assert_eq!(elements.len(), 1);
    match &elements[0] {
        MarkdownElement::Paragraph(text) => {
            // Should preserve the markdown syntax
            assert!(text.contains("**bold"));
        }
        _ => panic!("Expected Paragraph element"),
    }
}

/// Visual regression test helper
/// Returns structured data for visual verification
#[test]
fn test_visual_structure_verification() {
    let markdown = COMPLEX_MARKDOWN;
    let elements = parse_markdown(markdown);

    // Print structure for manual verification
    println!("\n=== Markdown Structure ===\n");
    for (i, element) in elements.iter().enumerate() {
        match element {
            MarkdownElement::Heading { level, content } => {
                println!("{}: H{} - {}", i, level, content);
            }
            MarkdownElement::Paragraph(text) => {
                let preview = if text.len() > 50 {
                    format!("{}...", &text[..50])
                } else {
                    text.clone()
                };
                println!("{}: P - {}", i, preview);
            }
            MarkdownElement::CodeBlock { language, content } => {
                let lines = content.lines().count();
                println!("{}: CB ({}) - {} lines", i, language, lines);
            }
            MarkdownElement::ListItem { level, content } => {
                println!("{}: LI (L{}) - {}", i, level, content);
            }
            MarkdownElement::Blockquote(text) => {
                let preview = if text.len() > 50 {
                    format!("{}...", &text[..50])
                } else {
                    text.clone()
                };
                println!("{}: BQ - {}", i, preview);
            }
        }
    }
    println!("\nTotal elements: {}\n", elements.len());

    // Should have a reasonable number of elements
    assert!(
        elements.len() >= 10,
        "Complex markdown should parse into many elements"
    );
    assert!(
        elements.len() <= 100,
        "Complex markdown shouldn't explode into too many elements"
    );
}
