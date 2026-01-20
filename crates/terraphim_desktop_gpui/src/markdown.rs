//! Shared markdown rendering utilities
//!
//! Provides reusable markdown parsing and rendering functionality
//! that can be used across chat messages, article modal, and editor preview.
//!
//! Uses pulldown-cmark for parsing and GPUI for rendering.

use gpui::*;
use pulldown_cmark::{Event, Parser, Tag, TagEnd};
use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};

/// Markdown element types that can be rendered
#[derive(Clone, Debug)]
pub enum MarkdownElement {
    /// Heading element (h1-h6)
    Heading { level: usize, content: String },
    /// Paragraph text
    Paragraph(String),
    /// Code block with syntax highlighting
    CodeBlock { language: String, content: String },
    /// List item
    ListItem { level: usize, content: String },
    /// Blockquote
    Blockquote(String),
}

/// Cache for parsed markdown elements
/// Key: String content hash, Value: Parsed markdown elements
type MarkdownCache = Arc<Mutex<HashMap<String, Vec<MarkdownElement>>>>;

/// Get or create the global markdown cache
fn markdown_cache() -> &'static MarkdownCache {
    static CACHE: OnceLock<MarkdownCache> = OnceLock::new();
    CACHE.get_or_init(|| Arc::new(Mutex::new(HashMap::new())))
}

/// Helper function to generate cache key from content
fn cache_key(content: &str) -> String {
    // Use the content itself as key (simple but effective)
    // For large documents, consider using a hash instead
    content.to_string()
}

/// Parse markdown text into a vector of MarkdownElement
/// Uses a global cache to avoid re-parsing the same content
pub fn parse_markdown(content: &str) -> Vec<MarkdownElement> {
    let key = cache_key(content);

    // Check cache first
    if let Ok(cache) = markdown_cache().try_lock() {
        if let Some(cached) = cache.get(&key) {
            return cached.clone();
        }
    }

    // Parse the markdown
    let parser = Parser::new(content);
    let mut elements = Vec::new();
    let mut current_text = String::new();
    let mut code_block = None;
    let mut list_level: usize = 0;

    for event in parser {
        match event {
            Event::Start(Tag::Heading { level, .. }) => {
                if !current_text.is_empty() {
                    elements.push(MarkdownElement::Paragraph(current_text.clone()));
                    current_text.clear();
                }
            }
            Event::End(TagEnd::Heading(level)) => {
                if !current_text.is_empty() {
                    // Convert HeadingLevel to usize (H1 = 1, H2 = 2, etc.)
                    let level_num = match level {
                        pulldown_cmark::HeadingLevel::H1 => 1,
                        pulldown_cmark::HeadingLevel::H2 => 2,
                        pulldown_cmark::HeadingLevel::H3 => 3,
                        pulldown_cmark::HeadingLevel::H4 => 4,
                        pulldown_cmark::HeadingLevel::H5 => 5,
                        pulldown_cmark::HeadingLevel::H6 => 6,
                    };
                    elements.push(MarkdownElement::Heading {
                        level: level_num,
                        content: current_text.clone(),
                    });
                    current_text.clear();
                }
            }
            Event::Start(Tag::CodeBlock(kind)) => {
                let language = match kind {
                    pulldown_cmark::CodeBlockKind::Fenced(fence) => {
                        if fence.is_empty() {
                            "text".to_string()
                        } else {
                            fence.to_string()
                        }
                    }
                    _ => "text".to_string(),
                };
                code_block = Some(language);
            }
            Event::End(TagEnd::CodeBlock) => {
                if let Some(lang) = code_block.take() {
                    if !current_text.is_empty() {
                        elements.push(MarkdownElement::CodeBlock {
                            language: lang,
                            content: current_text.clone(),
                        });
                        current_text.clear();
                    }
                }
            }
            Event::Start(Tag::List(_)) => list_level += 1,
            Event::End(TagEnd::List(_)) => list_level = list_level.saturating_sub(1),
            Event::Start(Tag::Item) => {
                if !current_text.is_empty() && list_level == 0 {
                    elements.push(MarkdownElement::Paragraph(current_text.clone()));
                    current_text.clear();
                }
            }
            Event::End(TagEnd::Item) => {
                if !current_text.is_empty() {
                    elements.push(MarkdownElement::ListItem {
                        level: list_level,
                        content: current_text.clone(),
                    });
                    current_text.clear();
                }
            }
            Event::Text(text) => {
                current_text.push_str(&text);
            }
            Event::Code(code) => {
                current_text.push('`');
                current_text.push_str(&code);
                current_text.push('`');
            }
            Event::Start(Tag::Strong) => current_text.push_str("**"),
            Event::End(TagEnd::Strong) => current_text.push_str("**"),
            Event::Start(Tag::Emphasis) => current_text.push('*'),
            Event::End(TagEnd::Emphasis) => current_text.push('*'),
            Event::SoftBreak | Event::HardBreak => current_text.push('\n'),
            _ => {}
        }
    }

    if !current_text.is_empty() {
        elements.push(MarkdownElement::Paragraph(current_text));
    }

    // Store in cache for future use
    if let Ok(mut cache) = markdown_cache().try_lock() {
        cache.insert(key, elements.clone());
    }

    elements
}

/// Render markdown elements into GPUI elements
pub fn render_markdown_elements(elements: Vec<MarkdownElement>) -> impl IntoElement {
    div()
        .flex()
        .flex_col()
        .gap_2()
        .children(elements.into_iter().map(|element| {
            match element {
                MarkdownElement::Heading { level, content } => {
                    let font_size = match level {
                        1 => 30.0,
                        2 => 26.0,
                        3 => 22.0,
                        4 => 20.0,
                        5 => 18.0,
                        _ => 16.0,
                    };
                    div()
                        .text_size(px(font_size))
                        .font_weight(FontWeight::BOLD)
                        .text_color(rgb(0x1a1a1a))
                        .mt_4()
                        .mb_2()
                        .child(content.clone())
                        .into_any_element()
                }
                MarkdownElement::Paragraph(text) => div()
                    .text_size(px(14.0))
                    .text_color(rgb(0x333333))
                    .line_height(px(22.0))
                    .mb_4()
                    .child(text.clone())
                    .into_any_element(),
                MarkdownElement::CodeBlock {
                    language: _,
                    content,
                } => div()
                    .bg(rgb(0xf8f9fa))
                    .border_1()
                    .border_color(rgb(0xe0e0e0))
                    .rounded_md()
                    .p_4()
                    .mb_4()
                    .child(
                        div()
                            .font_family("Monospace")
                            .text_size(px(13.0))
                            .text_color(rgb(0x24292e))
                            .child(content.clone()),
                    )
                    .into_any_element(),
                MarkdownElement::ListItem { level: _, content } => div()
                    .flex()
                    .items_start()
                    .mb_2()
                    .child(format!("• {}", content))
                    .into_any_element(),
                MarkdownElement::Blockquote(text) => div()
                    .border_l_4()
                    .border_color(rgb(0x6a737d))
                    .pl_4()
                    .mb_4()
                    .child(
                        div()
                            .text_size(px(14.0))
                            .text_color(rgb(0x6a737d))
                            .italic()
                            .child(text.clone()),
                    )
                    .into_any_element(),
            }
        }))
}

/// Parse and render markdown text in one step
pub fn render_markdown(text: &str) -> impl IntoElement {
    let elements = parse_markdown(text);
    render_markdown_elements(elements)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_markdown_heading() {
        let markdown = "# Hello World";
        let elements = parse_markdown(markdown);
        assert_eq!(elements.len(), 1);
        match &elements[0] {
            MarkdownElement::Heading { level, content } => {
                assert_eq!(*level, 1);
                assert_eq!(content, "Hello World");
            }
            _ => panic!("Expected Heading element"),
        }
    }

    #[test]
    fn test_parse_markdown_paragraph() {
        let markdown = "This is a paragraph";
        let elements = parse_markdown(markdown);
        assert_eq!(elements.len(), 1);
        match &elements[0] {
            MarkdownElement::Paragraph(text) => {
                assert_eq!(text, "This is a paragraph");
            }
            _ => panic!("Expected Paragraph element"),
        }
    }

    #[test]
    fn test_parse_markdown_code_block() {
        let markdown = "```rust\nfn main() {}\n```";
        let elements = parse_markdown(markdown);
        assert_eq!(elements.len(), 1);
        match &elements[0] {
            MarkdownElement::CodeBlock { language, content } => {
                assert_eq!(language, "rust");
                assert!(content.contains("fn main()"));
            }
            _ => panic!("Expected CodeBlock element"),
        }
    }

    #[test]
    fn test_parse_markdown_mixed() {
        let markdown = "# Title\n\nSome text\n\n```rust\ncode\n```";
        let elements = parse_markdown(markdown);
        assert!(elements.len() >= 3);
    }
}
