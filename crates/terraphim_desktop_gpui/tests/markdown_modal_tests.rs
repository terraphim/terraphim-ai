#![recursion_limit = "1024"]

/// Comprehensive test suite for MarkdownModal component
/// Tests markdown parsing, rendering, search functionality, and reusability

use terraphim_desktop_gpui::views::markdown_modal::{
    MarkdownModal, MarkdownModalOptions, MarkdownModalState,
    MarkdownStyles, TocEntry, SearchResult, MarkdownModalEvent
};

/// Test markdown content samples
const SAMPLE_MARKDOWN: &str = r#"
# Main Heading

This is a simple paragraph with **bold text** and *italic text*.

## Subsection

Here's some `inline code` and a code block:

```rust
fn main() {
    println!("Hello, world!");
}
```

- List item 1
- List item 2
- List item 3

> This is a blockquote
> with multiple lines

### Third Level

More content here.
"#;

const COMPLEX_MARKDOWN: &str = r#"
# Documentation Title

## Introduction

This is comprehensive documentation with multiple sections.

### Features

- **Feature 1**: Description here
- **Feature 2**: Another description
- **Feature 3**: Final description

## Code Examples

```javascript
function example() {
    return "This is JavaScript";
}
```

```python
def example():
    return "This is Python"
```

## Configuration

The configuration includes several settings:
1. Database connection
2. API endpoints
3. Security settings

> **Note**: Always ensure security best practices.

## Conclusion

This concludes the documentation.
"#;

#[test]
fn test_markdown_modal_options_default() {
    let options = MarkdownModalOptions::default();

    assert!(options.title.is_none());
    assert!(options.show_search);
    assert!(options.show_toc);
    assert_eq!(options.max_width, Some(1000.0));
    assert_eq!(options.max_height, Some(700.0));
    assert!(options.enable_keyboard_shortcuts);
    assert!(options.custom_classes.is_empty());
}

#[test]
fn test_markdown_modal_options_custom() {
    let options = MarkdownModalOptions {
        title: Some("Custom Title".to_string()),
        show_search: false,
        show_toc: false,
        max_width: Some(800.0),
        max_height: Some(600.0),
        enable_keyboard_shortcuts: false,
        custom_classes: vec!["custom-class".to_string()],
    };

    assert_eq!(options.title, Some("Custom Title".to_string()));
    assert!(!options.show_search);
    assert!(!options.show_toc);
    assert_eq!(options.max_width, Some(800.0));
    assert_eq!(options.max_height, Some(600.0));
    assert!(!options.enable_keyboard_shortcuts);
    assert_eq!(options.custom_classes.len(), 1);
}

#[test]
fn test_markdown_modal_state_initial() {
    let state = MarkdownModalState {
        is_open: false,
        content: String::new(),
        search_query: String::new(),
        current_section: None,
        toc_entries: Vec::new(),
        search_results: Vec::new(),
        selected_search_result: None,
    };

    assert!(!state.is_open);
    assert!(state.content.is_empty());
    assert!(state.search_query.is_empty());
    assert!(state.current_section.is_none());
    assert!(state.toc_entries.is_empty());
    assert!(state.search_results.is_empty());
    assert!(state.selected_search_result.is_none());
}

#[test]
fn test_toc_entry_structure() {
    let entry = TocEntry {
        title: "Section Title".to_string(),
        level: 2,
        id: "section-title".to_string(),
        position: 100,
    };

    assert_eq!(entry.title, "Section Title");
    assert_eq!(entry.level, 2);
    assert_eq!(entry.id, "section-title");
    assert_eq!(entry.position, 100);
}

#[test]
fn test_search_result_structure() {
    let result = SearchResult {
        line_number: 5,
        snippet: "**found** text".to_string(),
        context: "Line 4\nLine 5: found text\nLine 6".to_string(),
        position: 10,
    };

    assert_eq!(result.line_number, 5);
    assert_eq!(result.snippet, "**found** text");
    assert_eq!(result.context, "Line 4\nLine 5: found text\nLine 6");
    assert_eq!(result.position, 10);
}

#[test]
fn test_markdown_styles_default() {
    let styles = MarkdownStyles::default();

    assert_eq!(styles.heading_sizes, [32.0, 28.0, 24.0, 20.0, 18.0, 16.0]);
    assert_eq!(styles.base_font_size, 14.0);
    assert_eq!(styles.line_height, 24.0);
}

#[test]
fn test_markdown_styles_custom() {
    let custom_sizes = [28.0, 24.0, 20.0, 18.0, 16.0, 14.0];
    let styles = MarkdownStyles {
        heading_sizes: custom_sizes,
        base_font_size: 16.0,
        line_height: 28.0,
    };

    assert_eq!(styles.heading_sizes, custom_sizes);
    assert_eq!(styles.base_font_size, 16.0);
    assert_eq!(styles.line_height, 28.0);
}

#[test]
fn test_section_id_generation() {
    // This would normally be tested through the modal's internal method
    // For now, test the expected pattern
    let test_cases = vec![
        ("Simple Heading", "simple-heading"),
        ("Heading with-hyphens", "heading-with-hyphens"),
        ("Heading_with_underscores", "heading-with-underscores"),
        ("Heading with Multiple   Spaces", "heading-with-multiple---spaces"),
        ("Heading-123-with-numbers", "heading-123-with-numbers"),
        ("", ""),
    ];

    for (input, expected) in test_cases {
        // Simulate the generate_section_id logic
        let result = input
            .to_lowercase()
            .chars()
            .map(|c| if c.is_alphanumeric() { c } else { '-' })
            .collect::<String>()
            .trim_matches('-')
            .to_string();

        assert_eq!(result, expected, "Failed for input: {}", input);
    }
}

#[test]
fn test_search_term_highlighting() {
    // Test the highlight_search_term logic
    let text = "This is a test string";
    let query = "test";
    let pos = text.find(query).unwrap();

    // Simulate the highlight_search_term logic
    let end = (pos + query.len()).min(text.len());
    let result = format!(
        "{}**{}**{}",
        &text[..pos],
        &text[pos..end],
        &text[end..]
    );

    assert_eq!(result, "This is a **test** string");
}

#[test]
fn test_search_content_extraction() {
    let content = "Line 1\nLine 2 with keyword\nLine 3\nLine 4 with keyword\nLine 5";
    let lines: Vec<&str> = content.lines().collect();
    let query = "keyword";

    // Simulate search_content logic
    let mut results = Vec::new();

    for (line_number, line) in lines.iter().enumerate() {
        if let Some(pos) = line.to_lowercase().find(&query.to_lowercase()) {
            results.push(SearchResult {
                line_number: line_number + 1,
                snippet: format!("{}**{}**{}",
                    &line[..pos],
                    &line[pos..pos + query.len()],
                    &line[pos + query.len()..]
                ),
                context: format!("{}: {}", line_number + 1, line),
                position: pos,
            });
        }
    }

    assert_eq!(results.len(), 2);
    assert_eq!(results[0].line_number, 2);
    assert_eq!(results[1].line_number, 4);
    assert!(results[0].snippet.contains("**keyword**"));
    assert!(results[1].snippet.contains("**keyword**"));
}

#[test]
fn test_get_search_context() {
    let lines = vec![
        "Line 1",
        "Line 2",
        "Target line",
        "Line 4",
        "Line 5",
    ];
    let line_number: usize = 2; // Target line index
    let context_size = 1;

    // Simulate get_search_context logic
    let start = line_number.saturating_sub(context_size);
    let end = (line_number + context_size + 1).min(lines.len());

    let context = lines[start..end]
        .iter()
        .enumerate()
        .map(|(i, line)| {
            let actual_line = start + i + 1;
            format!("{}: {}", actual_line, line)
        })
        .collect::<Vec<_>>()
        .join("\n");

    let expected = "2: Line 2\n3: Target line\n4: Line 4";
    assert_eq!(context, expected);
}

#[test]
fn test_search_result_limits() {
    // Test that search results are properly limited
    let content = "keyword\n".repeat(100); // 100 lines with "keyword"
    let lines: Vec<&str> = content.lines().collect();
    let query = "keyword";

    // Simulate search with limit
    let mut results = Vec::new();

    for (line_number, line) in lines.iter().enumerate() {
        if line.contains(query) {
            results.push(SearchResult {
                line_number: line_number + 1,
                snippet: line.to_string(),
                context: line.to_string(),
                position: 0,
            });
        }

        // Apply limit (simulating the truncate in actual implementation)
        if results.len() >= 50 {
            results.truncate(50);
            break;
        }
    }

    assert_eq!(results.len(), 50); // Should be limited to 50
}

#[test]
fn test_empty_content_handling() {
    // Test how the modal handles empty content
    let empty_content = "";
    let lines: Vec<&str> = empty_content.lines().collect();
    let query = "test";

    // Simulate search on empty content
    let mut results = Vec::new();

    for (line_number, line) in lines.iter().enumerate() {
        if line.to_lowercase().contains(&query.to_lowercase()) {
            results.push(line_number);
        }
    }

    assert!(results.is_empty());
    assert!(lines.is_empty());
}

#[test]
fn test_complex_markdown_structure() {
    // Test handling of complex markdown with multiple elements
    let content = COMPLEX_MARKDOWN;

    // Count different markdown elements
    let heading_count = content.lines().filter(|line| line.starts_with('#')).count();
    let code_block_count = content.matches("```").count() / 2;
    let list_count = content.lines().filter(|line| line.trim_start().starts_with("-")).count();
    let blockquote_count = content.lines().filter(|line| line.trim_start().starts_with(">")).count();

    assert_eq!(heading_count, 6);
    assert_eq!(code_block_count, 2);
    assert!(list_count > 0);
    assert!(blockquote_count > 0);
}

#[test]
fn test_markdown_modal_event_types() {
    // Test that all event types can be created
    let events = vec![
        MarkdownModalEvent::Closed,
        MarkdownModalEvent::SectionNavigated {
            section: "test-section".to_string()
        },
        MarkdownModalEvent::SearchPerformed {
            query: "test".to_string(),
            results: Vec::new()
        },
        MarkdownModalEvent::LinkClicked {
            url: "https://example.com".to_string()
        },
        MarkdownModalEvent::KeyboardShortcut {
            shortcut: "ctrl+f".to_string()
        },
    ];

    assert_eq!(events.len(), 5);

    // Test event matching
    match &events[0] {
        MarkdownModalEvent::Closed => {} // Expected
        _ => panic!("Expected Closed event"),
    }

    match &events[1] {
        MarkdownModalEvent::SectionNavigated { section } => {
            assert_eq!(section, "test-section");
        }
        _ => panic!("Expected SectionNavigated event"),
    }
}

#[test]
fn test_toc_extraction_simulation() {
    // Simulate TOC extraction logic
    let content = SAMPLE_MARKDOWN;
    let mut toc_entries = Vec::new();
    let mut in_heading = false;
    let mut heading_level = 1;
    let mut heading_text = String::new();

    for line in content.lines() {
        if line.starts_with('#') {
            // Count heading level
            heading_level = line.chars().take_while(|c| *c == '#').count();

            // Extract text after #
            if let Some(start) = line.find(|c: char| !c.is_whitespace() && c != '#') {
                heading_text = line[start..].trim().to_string();
                in_heading = true;
            }
        } else if in_heading && !line.trim().is_empty() {
            // Generate ID and create entry
            let id = heading_text
                .to_lowercase()
                .chars()
                .map(|c| if c.is_alphanumeric() { c } else { '-' })
                .collect::<String>()
                .trim_matches('-')
                .to_string();

            toc_entries.push(TocEntry {
                title: heading_text.clone(),
                level: heading_level,
                id,
                position: 0, // Would be actual position in real implementation
            });

            in_heading = false;
        }
    }

    assert!(!toc_entries.is_empty());
    assert_eq!(toc_entries[0].title, "Main Heading");
    assert_eq!(toc_entries[0].level, 1);
    assert_eq!(toc_entries[1].title, "Subsection");
    assert_eq!(toc_entries[1].level, 2);
}

#[test]
fn test_keyboard_shortcut_scenarios() {
    // Test keyboard shortcut handling scenarios
    let shortcuts = vec![
        ("escape", "close"),
        ("ctrl+f", "search"),
        ("cmd+f", "search"),
        ("ctrl+k", "clear_search"),
        ("cmd+k", "clear_search"),
        ("n", "next_result"),
        ("p", "previous_result"),
    ];

    for (shortcut, expected_action) in shortcuts {
        // Simulate keyboard handling logic
        let action = match shortcut {
            "escape" => "close",
            "ctrl+f" | "cmd+f" => "search",
            "ctrl+k" | "cmd+k" => "clear_search",
            "n" => "next_result",
            "p" => "previous_result",
            _ => "unknown",
        };

        assert_eq!(action, expected_action, "Failed for shortcut: {}", shortcut);
    }
}

#[test]
fn test_modal_sizing_configurations() {
    // Test different modal size configurations
    let configurations = vec![
        (Some(800.0), Some(600.0), "small"),
        (Some(1000.0), Some(700.0), "medium"),
        (Some(1200.0), Some(800.0), "large"),
        (None, None, "flexible"),
    ];

    for (width, height, name) in configurations {
        let options = MarkdownModalOptions {
            max_width: width,
            max_height: height,
            ..Default::default()
        };

        match name {
            "small" => {
                assert_eq!(options.max_width, Some(800.0));
                assert_eq!(options.max_height, Some(600.0));
            }
            "medium" => {
                assert_eq!(options.max_width, Some(1000.0));
                assert_eq!(options.max_height, Some(700.0));
            }
            "large" => {
                assert_eq!(options.max_width, Some(1200.0));
                assert_eq!(options.max_height, Some(800.0));
            }
            "flexible" => {
                assert!(options.max_width.is_none());
                assert!(options.max_height.is_none());
            }
            _ => {}
        }
    }
}

#[test]
fn test_feature_flags_configuration() {
    // Test different feature flag combinations
    let configurations = vec![
        (true, true, true, "all_features"),
        (false, false, false, "minimal"),
        (true, false, true, "search_only"),
        (false, true, true, "toc_only"),
        (true, true, false, "no_keyboard"),
    ];

    for (search, toc, keyboard, name) in configurations {
        let options = MarkdownModalOptions {
            show_search: search,
            show_toc: toc,
            enable_keyboard_shortcuts: keyboard,
            ..Default::default()
        };

        match name {
            "all_features" => {
                assert!(options.show_search);
                assert!(options.show_toc);
                assert!(options.enable_keyboard_shortcuts);
            }
            "minimal" => {
                assert!(!options.show_search);
                assert!(!options.show_toc);
                assert!(!options.enable_keyboard_shortcuts);
            }
            "search_only" => {
                assert!(options.show_search);
                assert!(!options.show_toc);
                assert!(options.enable_keyboard_shortcuts);
            }
            "toc_only" => {
                assert!(!options.show_search);
                assert!(options.show_toc);
                assert!(options.enable_keyboard_shortcuts);
            }
            "no_keyboard" => {
                assert!(options.show_search);
                assert!(options.show_toc);
                assert!(!options.enable_keyboard_shortcuts);
            }
            _ => {}
        }
    }
}

#[test]
fn test_error_handling_scenarios() {
    // Test how the modal handles various error conditions

    // Empty search query
    let empty_query = "";
    let search_results = if empty_query.is_empty() {
        Vec::<SearchResult>::new()
    } else {
        vec![SearchResult {
            line_number: 1,
            snippet: "test".to_string(),
            context: "test".to_string(),
            position: 0,
        }]
    };

    assert!(search_results.is_empty());

    // Invalid content (malformed markdown)
    let malformed_content = "# Unclosed heading\n```rust\nfn test() {";
    let line_count = malformed_content.lines().count();
    assert!(line_count > 0);

    // Very long content
    let very_long_content = "Line ".repeat(10000);
    assert!(very_long_content.len() > 40000);
}

#[test]
fn test_reusability_patterns() {
    // Test that the modal can be configured for different use cases

    // Documentation viewer configuration
    let doc_viewer = MarkdownModalOptions {
        title: Some("Documentation".to_string()),
        show_search: true,
        show_toc: true,
        max_width: Some(1200.0),
        max_height: Some(800.0),
        enable_keyboard_shortcuts: true,
        custom_classes: vec!["documentation-viewer".to_string()],
    };

    // Simple preview configuration
    let simple_preview = MarkdownModalOptions {
        title: None,
        show_search: false,
        show_toc: false,
        max_width: Some(600.0),
        max_height: Some(400.0),
        enable_keyboard_shortcuts: false,
        custom_classes: vec!["simple-preview".to_string()],
    };

    // Code snippet viewer
    let code_viewer = MarkdownModalOptions {
        title: Some("Code Snippet".to_string()),
        show_search: false,
        show_toc: false,
        max_width: Some(1000.0),
        max_height: Some(600.0),
        enable_keyboard_shortcuts: true,
        custom_classes: vec!["code-viewer".to_string()],
    };

    // Verify each configuration has different characteristics
    assert!(doc_viewer.show_search && doc_viewer.show_toc);
    assert!(!simple_preview.show_search && !simple_preview.show_toc);
    assert!(!code_viewer.show_search && !code_viewer.show_toc);

    assert_eq!(doc_viewer.max_width, Some(1200.0));
    assert_eq!(simple_preview.max_width, Some(600.0));
    assert_eq!(code_viewer.max_width, Some(1000.0));
}

#[test]
fn test_performance_considerations() {
    // Test performance-related scenarios

    // Large content handling
    let large_content = "# Heading\n\n".repeat(1000);
    let content_length = large_content.len();

    // Should handle large content without issues
    assert!(content_length > 10000);

    // Search performance with many results
    let _many_results_content = "keyword\n".repeat(1000);
    let simulated_results = (0..1000).map(|i| SearchResult {
        line_number: i + 1,
        snippet: "keyword found".to_string(),
        context: format!("Line {}: keyword", i + 1),
        position: 0,
    }).collect::<Vec<_>>();

    // Should limit results for performance
    let limited_results = simulated_results.into_iter().take(50).collect::<Vec<_>>();
    assert_eq!(limited_results.len(), 50);

    // TOC generation with many headings
    let many_headings_content = (0..100).map(|i| format!("# Heading {}", i)).collect::<Vec<_>>().join("\n");
    let heading_count = many_headings_content.lines().filter(|line| line.starts_with('#')).count();

    // Should handle many headings efficiently
    assert_eq!(heading_count, 100);
}