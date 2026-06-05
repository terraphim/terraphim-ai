use std::ops::Range;

use markdown::mdast::Node;
use serde::{Deserialize, Serialize};
use ulid::Ulid;

use crate::{MarkdownParserError, NormalizedMarkdown, children, collect_text_content};

/// Strategy used to match a heading title against a `SectionPattern`'s pattern string.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum MatchStrategy {
    /// Match when the heading title starts with the pattern.
    Prefix,
    /// Match when the heading title contains the pattern anywhere.
    Contains,
}

/// Describes the semantic role of a heading section in a document.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SectionType {
    /// The primary body content of the section.
    Main,
    /// A named sidebar or callout box, identified by its label.
    Sidebar(String),
    /// A career-focused section (e.g. "Selling U" textbook sections).
    Career,
    /// An assessment or review section (e.g. key takeaways, quizzes).
    Assessment,
}

impl std::fmt::Display for SectionType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SectionType::Main => write!(f, "Main"),
            SectionType::Sidebar(s) => write!(f, "Sidebar({s})"),
            SectionType::Career => write!(f, "Career"),
            SectionType::Assessment => write!(f, "Assessment"),
        }
    }
}

/// Represents a single rule that maps a heading title pattern to a [`SectionType`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SectionPattern {
    /// The string pattern to match against a heading title.
    pub pattern: String,
    /// The section type to assign when the pattern matches.
    pub section_type: SectionType,
    /// The strategy used to compare the heading title against `pattern`.
    pub match_strategy: MatchStrategy,
}

/// Represents an ordered collection of [`SectionPattern`] rules for classifying headings.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SectionConfig {
    /// The ordered list of pattern rules; first match wins.
    pub rules: Vec<SectionPattern>,
}

impl SectionConfig {
    /// Returns a default [`SectionConfig`] with common textbook section patterns pre-configured.
    pub fn textbook_default() -> Self {
        Self {
            rules: vec![
                SectionPattern {
                    pattern: "Power Selling".to_string(),
                    section_type: SectionType::Sidebar("PowerSelling".to_string()),
                    match_strategy: MatchStrategy::Prefix,
                },
                SectionPattern {
                    pattern: "Power Player".to_string(),
                    section_type: SectionType::Sidebar("PowerPlayer".to_string()),
                    match_strategy: MatchStrategy::Prefix,
                },
                SectionPattern {
                    pattern: "Power Point".to_string(),
                    section_type: SectionType::Sidebar("PowerPoint".to_string()),
                    match_strategy: MatchStrategy::Prefix,
                },
                // "Selling U" uses Contains because the phrase can appear
                // mid-title (e.g. "Chapter 3: Selling U -- Your Career").
                SectionPattern {
                    pattern: "Selling U".to_string(),
                    section_type: SectionType::Career,
                    match_strategy: MatchStrategy::Contains,
                },
                SectionPattern {
                    pattern: "Key Takeaways".to_string(),
                    section_type: SectionType::Assessment,
                    match_strategy: MatchStrategy::Prefix,
                },
                SectionPattern {
                    pattern: "Test Your Power Knowledge".to_string(),
                    section_type: SectionType::Assessment,
                    match_strategy: MatchStrategy::Prefix,
                },
            ],
        }
    }

    /// Classifies a heading `title` against the configured rules, returning the matched [`SectionType`].
    pub fn classify(&self, title: &str) -> SectionType {
        let title_trimmed = title.trim();
        for rule in &self.rules {
            let matched = match rule.match_strategy {
                MatchStrategy::Prefix => title_trimmed.starts_with(&rule.pattern),
                MatchStrategy::Contains => title_trimmed.contains(&rule.pattern),
            };
            if matched {
                return rule.section_type.clone();
            }
        }
        SectionType::Main
    }
}

impl Default for SectionConfig {
    fn default() -> Self {
        Self::textbook_default()
    }
}

/// Represents a single node in the heading hierarchy tree.
#[derive(Debug, Clone)]
pub struct HeadingNode {
    /// The heading depth level (1 for `#`, 2 for `##`, etc.).
    pub level: u8,
    /// The plain-text title extracted from the heading.
    pub title: String,
    /// The classified section type for this heading.
    pub section_type: SectionType,
    /// The ULIDs of blocks whose content falls directly under this heading.
    pub blocks: Vec<Ulid>,
    /// The child heading nodes nested beneath this heading.
    pub children: Vec<HeadingNode>,
    /// The byte range in the source covering this heading and its content.
    pub byte_range: Range<usize>,
}

/// Represents the full heading hierarchy extracted from a normalised markdown document.
#[derive(Debug, Clone)]
pub struct HeadingTree {
    /// The top-level heading nodes (depth-1 headings or document roots).
    pub roots: Vec<HeadingNode>,
}

/// Builds a [`HeadingTree`] from the AST embedded in a [`NormalizedMarkdown`] value.
pub fn build_heading_tree(
    normalized: &NormalizedMarkdown,
) -> Result<HeadingTree, MarkdownParserError> {
    let Some(ref ast) = normalized.ast else {
        return Ok(HeadingTree { roots: vec![] });
    };

    let headings = extract_headings(ast);
    let tree = build_tree_from_headings(&headings, normalized);
    Ok(tree)
}

/// Classifies every node in the heading tree in-place using the provided [`SectionConfig`].
pub fn classify_sections(tree: &mut HeadingTree, config: &SectionConfig) {
    for root in &mut tree.roots {
        classify_node(root, config);
    }
}

fn classify_node(node: &mut HeadingNode, config: &SectionConfig) {
    node.section_type = config.classify(&node.title);
    for child in &mut node.children {
        classify_node(child, config);
    }
}

pub struct RawHeading {
    level: u8,
    title: String,
    byte_start: usize,
    byte_end: usize,
}

fn extract_headings(node: &Node) -> Vec<RawHeading> {
    let mut result = Vec::new();
    collect_headings(node, &mut result);
    result
}

fn collect_headings(node: &Node, out: &mut Vec<RawHeading>) {
    if let Node::Heading(h) = node {
        let title = collect_text_content(&h.children);
        let (start, end) = if let Some(pos) = node.position() {
            (pos.start.offset, pos.end.offset)
        } else {
            (0, 0)
        };
        out.push(RawHeading {
            level: h.depth,
            title,
            byte_start: start,
            byte_end: end,
        });
        return;
    }

    if let Some(children) = children(node) {
        for child in children {
            collect_headings(child, out);
        }
    }
}

fn build_tree_from_headings(
    headings: &[RawHeading],
    normalized: &NormalizedMarkdown,
) -> HeadingTree {
    if headings.is_empty() {
        return HeadingTree { roots: vec![] };
    }

    let mut roots: Vec<HeadingNode> = Vec::new();
    let mut stack: Vec<HeadingNode> = Vec::new();

    for (i, heading) in headings.iter().enumerate() {
        let next_byte_start = headings
            .get(i + 1)
            .map(|h| h.byte_start)
            .unwrap_or(normalized.markdown.len());

        let blocks = blocks_in_range(&normalized.blocks, heading.byte_end, next_byte_start);

        let node = HeadingNode {
            level: heading.level,
            title: heading.title.clone(),
            section_type: SectionType::Main,
            blocks,
            children: Vec::new(),
            byte_range: heading.byte_start..next_byte_start,
        };

        while let Some(top) = stack.last() {
            if top.level < node.level {
                break;
            }
            let popped = stack.pop().unwrap();
            if let Some(parent) = stack.last_mut() {
                parent.children.push(popped);
            } else {
                roots.push(popped);
            }
        }

        stack.push(node);
    }

    while let Some(popped) = stack.pop() {
        if let Some(parent) = stack.last_mut() {
            parent.children.push(popped);
        } else {
            roots.push(popped);
        }
    }

    HeadingTree { roots }
}

fn blocks_in_range(blocks: &[crate::Block], start: usize, end: usize) -> Vec<Ulid> {
    blocks
        .iter()
        .filter(|b| b.span.start >= start && b.span.start < end)
        .map(|b| b.id)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::normalize_markdown;

    #[test]
    fn section_config_classify_sidebar() {
        let config = SectionConfig::textbook_default();
        assert_eq!(
            config.classify("Power Selling: The Art of Persuasion"),
            SectionType::Sidebar("PowerSelling".to_string())
        );
    }

    #[test]
    fn section_config_classify_career() {
        let config = SectionConfig::textbook_default();
        assert_eq!(
            config.classify("Selling U: Your Career"),
            SectionType::Career
        );
    }

    #[test]
    fn section_config_classify_assessment() {
        let config = SectionConfig::textbook_default();
        assert_eq!(
            config.classify("Key Takeaways from Chapter 3"),
            SectionType::Assessment
        );
    }

    #[test]
    fn section_config_classify_main_fallback() {
        let config = SectionConfig::textbook_default();
        assert_eq!(config.classify("Introduction to Sales"), SectionType::Main);
    }

    #[test]
    fn build_heading_tree_simple() {
        let input = "# Chapter 1\n\nIntro paragraph\n\n## Section 1.1\n\nSome text\n\n# Chapter 2\n\nMore text\n";
        let normalized = normalize_markdown(input).unwrap();
        let tree = build_heading_tree(&normalized).unwrap();

        assert_eq!(tree.roots.len(), 2);
        assert_eq!(tree.roots[0].title, "Chapter 1");
        assert_eq!(tree.roots[0].level, 1);
        assert_eq!(tree.roots[0].children.len(), 1);
        assert_eq!(tree.roots[0].children[0].title, "Section 1.1");
        assert_eq!(tree.roots[1].title, "Chapter 2");
    }

    #[test]
    fn build_heading_tree_attaches_blocks() {
        let input = "# Chapter\n\nParagraph one\n\nParagraph two\n";
        let normalized = normalize_markdown(input).unwrap();
        let tree = build_heading_tree(&normalized).unwrap();

        assert_eq!(tree.roots.len(), 1);
        assert_eq!(tree.roots[0].blocks.len(), 2);
    }

    #[test]
    fn build_heading_tree_all_levels() {
        let input = "# H1\n\n## H2\n\n### H3\n\n#### H4\n\n##### H5\n\n###### H6\n\nText\n";
        let normalized = normalize_markdown(input).unwrap();
        let tree = build_heading_tree(&normalized).unwrap();

        assert_eq!(tree.roots.len(), 1);
        assert_eq!(tree.roots[0].level, 1);
        assert_eq!(tree.roots[0].children.len(), 1);
        assert_eq!(tree.roots[0].children[0].level, 2);
        assert_eq!(tree.roots[0].children[0].children.len(), 1);
        assert_eq!(tree.roots[0].children[0].children[0].level, 3);
    }

    #[test]
    fn classify_sections_applies_config() {
        let input = "# Main Title\n\nText\n\n## Power Selling: Tips\n\nTip text\n\n## Selling U: Careers\n\nCareer text\n";
        let normalized = normalize_markdown(input).unwrap();
        let mut tree = build_heading_tree(&normalized).unwrap();
        classify_sections(&mut tree, &SectionConfig::textbook_default());

        assert_eq!(tree.roots[0].section_type, SectionType::Main);
        let ps = &tree.roots[0].children[0];
        assert_eq!(
            ps.section_type,
            SectionType::Sidebar("PowerSelling".to_string())
        );
        let su = &tree.roots[0].children[1];
        assert_eq!(su.section_type, SectionType::Career);
    }

    #[test]
    fn build_heading_tree_empty() {
        let input = "No headings here\n\nJust text\n";
        let normalized = normalize_markdown(input).unwrap();
        let tree = build_heading_tree(&normalized).unwrap();
        assert!(tree.roots.is_empty());
    }

    #[test]
    fn custom_section_config() {
        let config = SectionConfig {
            rules: vec![SectionPattern {
                pattern: "Experiment".to_string(),
                section_type: SectionType::Sidebar("Lab".to_string()),
                match_strategy: MatchStrategy::Prefix,
            }],
        };
        assert_eq!(
            config.classify("Experiment 3: Results"),
            SectionType::Sidebar("Lab".to_string())
        );
        assert_eq!(config.classify("Introduction"), SectionType::Main);
    }

    #[test]
    fn match_strategy_contains() {
        let config = SectionConfig {
            rules: vec![SectionPattern {
                pattern: "Selling U".to_string(),
                section_type: SectionType::Career,
                match_strategy: MatchStrategy::Contains,
            }],
        };
        // Contains matches mid-title
        assert_eq!(
            config.classify("Chapter 3: Selling U -- Your Career"),
            SectionType::Career,
        );
        // Contains also matches at the start
        assert_eq!(config.classify("Selling U: Careers"), SectionType::Career,);
        // No match when substring is absent
        assert_eq!(config.classify("Introduction"), SectionType::Main);
    }
}
