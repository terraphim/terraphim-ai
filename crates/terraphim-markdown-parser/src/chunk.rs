use ulid::Ulid;

use crate::heading::{HeadingNode, HeadingTree, SectionType};
use crate::NormalizedMarkdown;

#[derive(Debug, Clone)]
pub struct ContentChunk {
    pub chunk_id: String,
    pub content_id: String,
    pub block_ids: Vec<Ulid>,
    pub chapter_number: Option<u8>,
    pub section_path: String,
    pub chunk_type: SectionType,
    pub text: String,
    pub token_count: u32,
}

struct ChunkState {
    chapter_counter: u8,
}

pub fn chunk_by_headings(
    content_id: &str,
    tree: &HeadingTree,
    normalized: &NormalizedMarkdown,
) -> Vec<ContentChunk> {
    let mut chunks = Vec::new();
    let mut state = ChunkState { chapter_counter: 0 };

    for root in &tree.roots {
        collect_chunks(
            root,
            content_id,
            normalized,
            &mut chunks,
            &mut state,
            &mut Vec::new(),
            0,
        );
    }

    chunks
}

fn collect_chunks(
    node: &HeadingNode,
    content_id: &str,
    normalized: &NormalizedMarkdown,
    chunks: &mut Vec<ContentChunk>,
    state: &mut ChunkState,
    path: &mut Vec<u8>,
    sibling_index: u8,
) {
    let is_chapter_root = path.is_empty();

    if is_chapter_root {
        state.chapter_counter += 1;
        path.push(state.chapter_counter);
    } else {
        path.push(sibling_index);
    }

    let section_path = format_path(path);

    if !node.blocks.is_empty() {
        let text = extract_block_text(&node.blocks, normalized);
        let token_count = text.split_whitespace().count() as u32;

        let chunk_id = format!(
            "{}#{}#{}",
            content_id,
            section_path,
            node.blocks
                .first()
                .map(|id| id.to_string())
                .unwrap_or_default()
        );

        chunks.push(ContentChunk {
            chunk_id,
            content_id: content_id.to_string(),
            block_ids: node.blocks.clone(),
            chapter_number: path.first().copied(),
            section_path: section_path.clone(),
            chunk_type: node.section_type.clone(),
            text,
            token_count,
        });
    }

    for (i, child) in node.children.iter().enumerate() {
        collect_chunks(
            child,
            content_id,
            normalized,
            chunks,
            state,
            path,
            i as u8 + 1,
        );
    }

    path.pop();
}

fn format_path(components: &[u8]) -> String {
    components
        .iter()
        .map(|c| c.to_string())
        .collect::<Vec<_>>()
        .join(".")
}

fn extract_block_text(block_ids: &[Ulid], normalized: &NormalizedMarkdown) -> String {
    let mut parts = Vec::new();
    for id in block_ids {
        if let Some(block) = normalized.blocks.iter().find(|b| b.id == *id) {
            let text =
                crate::strip_terraphim_block_id_comments(&normalized.markdown[block.span.clone()]);
            let trimmed = text.trim();
            if !trimmed.is_empty() {
                parts.push(trimmed.to_string());
            }
        }
    }
    parts.join("\n\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::heading::{build_heading_tree, classify_sections, SectionConfig};
    use crate::normalize_markdown;

    #[test]
    fn chunk_single_chapter() {
        let input = "# Chapter 1\n\nFirst paragraph.\n\nSecond paragraph.\n";
        let normalized = normalize_markdown(input).unwrap();
        let tree = build_heading_tree(&normalized).unwrap();
        let chunks = chunk_by_headings("test-doc", &tree, &normalized);

        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].content_id, "test-doc");
        assert_eq!(chunks[0].chapter_number, Some(1));
        assert_eq!(chunks[0].section_path, "1");
        assert_eq!(chunks[0].chunk_type, SectionType::Main);
        assert!(chunks[0].text.contains("First paragraph"));
        assert!(chunks[0].text.contains("Second paragraph"));
    }

    #[test]
    fn chunk_preserves_block_ulids() {
        let input = "# Chapter\n\nParagraph one\n\nParagraph two\n";
        let normalized = normalize_markdown(input).unwrap();
        let original_ids: Vec<Ulid> = normalized.blocks.iter().map(|b| b.id).collect();

        let tree = build_heading_tree(&normalized).unwrap();
        let chunks = chunk_by_headings("doc", &tree, &normalized);

        assert_eq!(chunks[0].block_ids.len(), 2);
        assert_eq!(chunks[0].block_ids, original_ids);
    }

    #[test]
    fn chunk_composite_ids() {
        let input = "# Chapter\n\nText\n";
        let normalized = normalize_markdown(input).unwrap();
        let tree = build_heading_tree(&normalized).unwrap();
        let chunks = chunk_by_headings("my-doc", &tree, &normalized);

        assert!(chunks[0].chunk_id.starts_with("my-doc#1#"));
    }

    #[test]
    fn chunk_nested_headings() {
        let input = "# Chapter\n\nIntro\n\n## Section A\n\nText A\n\n## Section B\n\nText B\n";
        let normalized = normalize_markdown(input).unwrap();
        let tree = build_heading_tree(&normalized).unwrap();
        let chunks = chunk_by_headings("book", &tree, &normalized);

        assert_eq!(chunks.len(), 3);
        assert_eq!(chunks[0].section_path, "1");
        assert_eq!(chunks[1].section_path, "1.1");
        assert_eq!(chunks[2].section_path, "1.2");
    }

    #[test]
    fn chunk_with_section_types() {
        let input =
            "# Chapter\n\nIntro\n\n## Power Selling: Tips\n\nTip\n\n## Selling U\n\nCareer\n";
        let normalized = normalize_markdown(input).unwrap();
        let mut tree = build_heading_tree(&normalized).unwrap();
        classify_sections(&mut tree, &SectionConfig::textbook_default());
        let chunks = chunk_by_headings("book", &tree, &normalized);

        assert_eq!(chunks[0].chunk_type, SectionType::Main);
        assert_eq!(
            chunks[1].chunk_type,
            SectionType::Sidebar("PowerSelling".to_string())
        );
        assert_eq!(chunks[2].chunk_type, SectionType::Career);
    }

    #[test]
    fn chunk_token_count() {
        let input = "# Chapter\n\nOne two three four five.\n";
        let normalized = normalize_markdown(input).unwrap();
        let tree = build_heading_tree(&normalized).unwrap();
        let chunks = chunk_by_headings("doc", &tree, &normalized);

        assert!(chunks[0].token_count > 0);
    }

    #[test]
    fn chunk_multiple_chapters() {
        let input = "# Chapter 1\n\nText 1\n\n# Chapter 2\n\nText 2\n\n# Chapter 3\n\nText 3\n";
        let normalized = normalize_markdown(input).unwrap();
        let tree = build_heading_tree(&normalized).unwrap();
        let chunks = chunk_by_headings("book", &tree, &normalized);

        assert_eq!(chunks.len(), 3);
        assert_eq!(chunks[0].chapter_number, Some(1));
        assert_eq!(chunks[1].chapter_number, Some(2));
        assert_eq!(chunks[2].chapter_number, Some(3));
    }
}
