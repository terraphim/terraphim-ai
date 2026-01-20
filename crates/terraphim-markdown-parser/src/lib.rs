use std::collections::HashSet;
use std::ops::Range;
use std::str::FromStr;

use markdown::ParseOptions;
use markdown::mdast::Node;
use terraphim_types::Document;
use thiserror::Error;
use ulid::Ulid;

pub const TERRAPHIM_BLOCK_ID_PREFIX: &str = "terraphim:block-id:";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlockKind {
    Paragraph,
    ListItem,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Block {
    pub id: Ulid,
    pub kind: BlockKind,

    /// Byte span of the block in the markdown buffer.
    ///
    /// For paragraphs, this includes the block-id comment line plus the paragraph content.
    /// For list items, this includes the full list item (including nested content).
    pub span: Range<usize>,

    /// Byte span of the block-id anchor.
    ///
    /// For paragraphs, this is the full comment line (including any leading quote/indent prefix).
    /// For list items, this is the inline HTML comment inside the list item’s first line.
    pub id_span: Range<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NormalizedMarkdown {
    pub markdown: String,
    pub blocks: Vec<Block>,
}

#[derive(Debug, Error)]
pub enum MarkdownParserError {
    #[error("failed to parse markdown: {0}")]
    Markdown(String),

    #[error("missing or invalid terraphim block id for {0:?} at byte offset {1}")]
    MissingOrInvalidBlockId(BlockKind, usize),
}

impl From<markdown::message::Message> for MarkdownParserError {
    fn from(value: markdown::message::Message) -> Self {
        Self::Markdown(format!("{value:?}"))
    }
}

#[derive(Debug, Clone)]
struct Edit {
    range: Range<usize>,
    replacement: String,
}

impl Edit {
    fn insert(at: usize, text: String) -> Self {
        Self {
            range: at..at,
            replacement: text,
        }
    }
}

/// Ensure every list item and paragraph has a stable Terraphim block id.
///
/// Canonical forms:
/// - Paragraph: `<!-- terraphim:block-id:<ULID> -->` on its own line immediately before the paragraph
/// - List item: inline after the marker (and optional task checkbox), e.g. `- <!-- terraphim:block-id:<ULID> --> text`
pub fn ensure_terraphim_block_ids(markdown: &str) -> Result<String, MarkdownParserError> {
    let ast = markdown::to_mdast(markdown, &ParseOptions::gfm())?;
    let mut edits: Vec<Edit> = Vec::new();
    ensure_block_ids_in_children(&ast, markdown, &mut edits, ParentKind::Other);

    if edits.is_empty() {
        return Ok(markdown.to_string());
    }

    // Apply edits from the end of the buffer to the beginning so byte offsets stay valid.
    edits.sort_by(|a, b| b.range.start.cmp(&a.range.start));
    let mut out = markdown.to_string();
    for edit in edits {
        out.replace_range(edit.range, &edit.replacement);
    }
    Ok(out)
}

/// Normalize markdown into canonical Terraphim form and return the extracted blocks.
pub fn normalize_markdown(markdown: &str) -> Result<NormalizedMarkdown, MarkdownParserError> {
    let normalized = ensure_terraphim_block_ids(markdown)?;
    let blocks = extract_blocks(&normalized)?;
    Ok(NormalizedMarkdown {
        markdown: normalized,
        blocks,
    })
}

/// Convert extracted blocks into Terraphim `Document`s so downstream graph tooling can be reused.
pub fn blocks_to_documents(source_id: &str, normalized: &NormalizedMarkdown) -> Vec<Document> {
    normalized
        .blocks
        .iter()
        .map(|block| {
            let block_id = block.id.to_string();
            let id = format!("{source_id}#{block_id}");
            let body = strip_terraphim_block_id_comments(&normalized.markdown[block.span.clone()])
                .trim()
                .to_string();
            let title = first_nonempty_line(&body).unwrap_or_else(|| "Untitled".to_string());
            Document {
                id,
                url: source_id.to_string(),
                title,
                body,
                description: None,
                summarization: None,
                stub: None,
                tags: None,
                rank: None,
                source_haystack: None,
            }
        })
        .collect()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ParentKind {
    ListItem,
    Other,
}

fn ensure_block_ids_in_children(
    node: &Node,
    source: &str,
    edits: &mut Vec<Edit>,
    parent: ParentKind,
) {
    match node {
        Node::Root(root) => {
            ensure_block_ids_in_list(&root.children, source, edits, ParentKind::Other)
        }
        Node::Blockquote(bq) => ensure_block_ids_in_list(&bq.children, source, edits, parent),
        Node::List(list) => ensure_block_ids_in_list(&list.children, source, edits, parent),
        Node::ListItem(li) => {
            if let Some(pos) = node.position() {
                ensure_list_item_inline_id(source, pos.start.offset, edits);
            }
            ensure_block_ids_in_list(&li.children, source, edits, ParentKind::ListItem);
        }
        _ => {
            if let Some(children) = children(node) {
                ensure_block_ids_in_list(children, source, edits, parent);
            }
        }
    }
}

fn ensure_block_ids_in_list(
    children: &[Node],
    source: &str,
    edits: &mut Vec<Edit>,
    parent: ParentKind,
) {
    let mut first_direct_paragraph_in_list_item = false;

    for (idx, child) in children.iter().enumerate() {
        match child {
            Node::ListItem(_) => ensure_block_ids_in_children(child, source, edits, parent),
            Node::Paragraph(_) => {
                // The first direct paragraph of a list item is considered owned by the list item’s
                // inline block id, so we do not insert a separate comment line for it.
                if parent == ParentKind::ListItem && !first_direct_paragraph_in_list_item {
                    first_direct_paragraph_in_list_item = true;
                } else if let Some(pos) = child.position() {
                    let has_prev_block_id = idx
                        .checked_sub(1)
                        .and_then(|prev| parse_block_id_from_html_node(&children[prev]))
                        .is_some();
                    if !has_prev_block_id {
                        edits.push(insert_paragraph_id_comment(source, pos.start.offset));
                    }
                }
            }
            _ => ensure_block_ids_in_children(child, source, edits, parent),
        }
    }
}

fn insert_paragraph_id_comment(source: &str, paragraph_start: usize) -> Edit {
    let (line_start, prefix) = line_prefix_at(source, paragraph_start);
    let id = Ulid::new();
    Edit::insert(
        line_start,
        format!("{prefix}<!-- terraphim:block-id:{id} -->\n"),
    )
}

fn ensure_list_item_inline_id(source: &str, list_item_start: usize, edits: &mut Vec<Edit>) {
    let (line_start, line_end) = line_bounds_at(source, list_item_start);
    let line = &source[line_start..line_end];

    if let Some((comment_start, comment_end, parsed)) = find_inline_block_id_comment(line) {
        if parsed.is_some() {
            return;
        }

        // Replace invalid block id comment with a fresh one.
        let replacement = format!("<!-- terraphim:block-id:{} -->", Ulid::new());
        edits.push(Edit {
            range: (line_start + comment_start)..(line_start + comment_end),
            replacement,
        });
        return;
    }

    // No existing comment on the first line; insert it after the list marker and optional checkbox.
    if let Some(insert_at) = list_item_inline_insert_point(source, list_item_start) {
        let trailing_space = match source.as_bytes().get(insert_at) {
            None | Some(b'\n') | Some(b'\r') => "",
            _ => " ",
        };
        edits.push(Edit::insert(
            insert_at,
            format!(
                "<!-- terraphim:block-id:{} -->{trailing_space}",
                Ulid::new()
            ),
        ));
    }
}

fn list_item_inline_insert_point(source: &str, list_item_start: usize) -> Option<usize> {
    let bytes = source.as_bytes();
    let mut i = list_item_start;

    // Skip indentation and blockquote markers on this line (e.g. "> " prefixes).
    // We only do a shallow pass to handle common cases like "> - item".
    loop {
        while i < bytes.len() && (bytes[i] == b' ' || bytes[i] == b'\t') {
            i += 1;
        }
        if bytes.get(i..i + 2) == Some(b"> ") {
            i += 2;
            continue;
        }
        break;
    }

    // Unordered list marker
    if matches!(bytes.get(i), Some(b'-' | b'*' | b'+')) {
        i += 1;
        if matches!(bytes.get(i), Some(b' ' | b'\t')) {
            i += 1;
        } else {
            return None;
        }
    } else if matches!(bytes.get(i), Some(b'0'..=b'9')) {
        // Ordered list marker: digits + '.' or ')' + whitespace
        while matches!(bytes.get(i), Some(b'0'..=b'9')) {
            i += 1;
        }
        if matches!(bytes.get(i), Some(b'.' | b')')) {
            i += 1;
        } else {
            return None;
        }
        if matches!(bytes.get(i), Some(b' ' | b'\t')) {
            i += 1;
        } else {
            return None;
        }
    } else {
        return None;
    }

    // Optional task list checkbox: [ ] / [x] / [X]
    if bytes.get(i) == Some(&b'[')
        && matches!(bytes.get(i + 1), Some(b' ' | b'x' | b'X'))
        && bytes.get(i + 2) == Some(&b']')
        && matches!(bytes.get(i + 3), Some(b' ' | b'\t'))
    {
        i += 4;
    }

    Some(i)
}

fn extract_blocks(markdown: &str) -> Result<Vec<Block>, MarkdownParserError> {
    let ast = markdown::to_mdast(markdown, &ParseOptions::gfm())?;
    let mut blocks = Vec::new();
    extract_blocks_from_children(&ast, markdown, &mut blocks, ParentKind::Other)?;

    // Validate uniqueness: ids should be stable and non-duplicated.
    let mut seen = HashSet::new();
    for b in &blocks {
        let id = b.id.to_string();
        if !seen.insert(id) {
            // If duplicates exist, it is safer to surface an error rather than silently re-ID.
            return Err(MarkdownParserError::MissingOrInvalidBlockId(
                b.kind,
                b.span.start,
            ));
        }
    }

    Ok(blocks)
}

fn extract_blocks_from_children(
    node: &Node,
    source: &str,
    blocks: &mut Vec<Block>,
    parent: ParentKind,
) -> Result<(), MarkdownParserError> {
    match node {
        Node::Root(root) => {
            extract_blocks_from_list(&root.children, source, blocks, ParentKind::Other)?;
        }
        Node::Blockquote(bq) => {
            extract_blocks_from_list(&bq.children, source, blocks, parent)?;
        }
        Node::List(list) => {
            extract_blocks_from_list(&list.children, source, blocks, parent)?;
        }
        Node::ListItem(li) => {
            let Some(pos) = node.position() else {
                return Ok(());
            };

            let Some((id, id_span)) = extract_list_item_id(source, pos.start.offset) else {
                return Err(MarkdownParserError::MissingOrInvalidBlockId(
                    BlockKind::ListItem,
                    pos.start.offset,
                ));
            };
            let start = line_bounds_at(source, pos.start.offset).0;
            let end = pos.end.offset;
            blocks.push(Block {
                id,
                kind: BlockKind::ListItem,
                span: start..end,
                id_span,
            });
            extract_blocks_from_list(&li.children, source, blocks, ParentKind::ListItem)?;
        }
        _ => {
            if let Some(children) = children(node) {
                extract_blocks_from_list(children, source, blocks, parent)?;
            }
        }
    }
    Ok(())
}

fn extract_blocks_from_list(
    children: &[Node],
    source: &str,
    blocks: &mut Vec<Block>,
    parent: ParentKind,
) -> Result<(), MarkdownParserError> {
    let mut first_direct_paragraph_in_list_item = false;

    for (idx, child) in children.iter().enumerate() {
        match child {
            Node::ListItem(_) => extract_blocks_from_children(child, source, blocks, parent)?,
            Node::Paragraph(_) => {
                if parent == ParentKind::ListItem && !first_direct_paragraph_in_list_item {
                    first_direct_paragraph_in_list_item = true;
                    continue;
                }

                let Some(pos) = child.position() else {
                    continue;
                };

                let Some((id, anchor_span)) = idx
                    .checked_sub(1)
                    .and_then(|prev| {
                        parse_block_id_from_html_node_with_span(source, &children[prev])
                    })
                    .and_then(|(id, span)| id.map(|id| (id, span)))
                else {
                    return Err(MarkdownParserError::MissingOrInvalidBlockId(
                        BlockKind::Paragraph,
                        pos.start.offset,
                    ));
                };

                blocks.push(Block {
                    id,
                    kind: BlockKind::Paragraph,
                    span: anchor_span.start..pos.end.offset,
                    id_span: anchor_span,
                })
            }
            _ => extract_blocks_from_children(child, source, blocks, parent)?,
        }
    }

    Ok(())
}

fn extract_list_item_id(source: &str, list_item_start: usize) -> Option<(Ulid, Range<usize>)> {
    let (line_start, line_end) = line_bounds_at(source, list_item_start);
    let line = &source[line_start..line_end];
    let (comment_start, comment_end, parsed) = find_inline_block_id_comment(line)?;
    let id = parsed?;
    Some((id, (line_start + comment_start)..(line_start + comment_end)))
}

fn parse_block_id_from_html_node(node: &Node) -> Option<Ulid> {
    match node {
        Node::Html(val) => parse_block_id_comment(&val.value),
        _ => None,
    }
}

fn parse_block_id_from_html_node_with_span(
    source: &str,
    node: &Node,
) -> Option<(Option<Ulid>, Range<usize>)> {
    let Node::Html(val) = node else { return None };
    let id = parse_block_id_comment(&val.value);

    let Some(pos) = node.position() else {
        return Some((id, 0..0));
    };

    let (line_start, line_end) = line_bounds_at(source, pos.start.offset);
    Some((id, line_start..line_end))
}

fn parse_block_id_comment(raw_html: &str) -> Option<Ulid> {
    let html = raw_html.trim();
    let inner = html
        .strip_prefix("<!--")
        .and_then(|s| s.strip_suffix("-->"))?;
    let inner = inner.trim();
    let id_str = inner.strip_prefix(TERRAPHIM_BLOCK_ID_PREFIX)?;
    Ulid::from_str(id_str.trim()).ok()
}

fn find_inline_block_id_comment(line: &str) -> Option<(usize, usize, Option<Ulid>)> {
    let start = line.find("<!--")?;
    let marker = line[start..].find(TERRAPHIM_BLOCK_ID_PREFIX)? + start;
    let end = line[marker..].find("-->")? + marker + 3;

    let comment_start = start;
    let comment_end = end;
    let comment = &line[comment_start..comment_end];
    Some((comment_start, comment_end, parse_block_id_comment(comment)))
}

fn line_bounds_at(source: &str, offset: usize) -> (usize, usize) {
    let line_start = source[..offset].rfind('\n').map(|i| i + 1).unwrap_or(0);
    let line_end = source[offset..]
        .find('\n')
        .map(|i| offset + i)
        .unwrap_or_else(|| source.len());
    (line_start, line_end)
}

fn line_prefix_at(source: &str, offset: usize) -> (usize, String) {
    let (line_start, _line_end) = line_bounds_at(source, offset);
    let prefix = &source[line_start..offset];
    (line_start, prefix.to_string())
}

fn children(node: &Node) -> Option<&Vec<Node>> {
    match node {
        Node::Root(root) => Some(&root.children),
        Node::Blockquote(bq) => Some(&bq.children),
        Node::List(list) => Some(&list.children),
        Node::ListItem(li) => Some(&li.children),
        Node::Paragraph(p) => Some(&p.children),
        Node::Heading(h) => Some(&h.children),
        _ => None,
    }
}

fn strip_terraphim_block_id_comments(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for line in text.lines() {
        let mut remaining = line;
        let mut cleaned = String::new();
        loop {
            let Some((start, end, _)) = find_inline_block_id_comment(remaining) else {
                cleaned.push_str(remaining);
                break;
            };
            cleaned.push_str(&remaining[..start]);
            remaining = &remaining[end..];
        }

        if cleaned.trim().is_empty() {
            continue;
        }

        out.push_str(cleaned.trim_end());
        out.push('\n')
    }
    out
}

fn first_nonempty_line(text: &str) -> Option<String> {
    text.lines()
        .map(|l| l.trim())
        .find(|l| !l.is_empty())
        .map(|l| l.chars().take(80).collect::<String>())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn count_block_ids(s: &str) -> usize {
        s.lines()
            .filter(|l| l.contains("<!-- terraphim:block-id:"))
            .count()
    }

    #[test]
    fn inserts_paragraph_ids() {
        let input = "Hello world\n\nSecond paragraph\n";
        let out = ensure_terraphim_block_ids(input).unwrap();
        // 2 paragraphs => 2 id comment lines
        assert_eq!(count_block_ids(&out), 2);
        assert!(out.contains("Hello world"));
        assert!(out.contains("Second paragraph"));
    }

    #[test]
    fn inserts_list_item_inline_ids() {
        let input = "- first\n- second\n";
        let out = ensure_terraphim_block_ids(input).unwrap();
        assert_eq!(count_block_ids(&out), 2);
        assert!(out.contains("- <!-- terraphim:block-id:"));
    }

    #[test]
    fn normalize_returns_blocks() {
        let input = "- item\n\nPara\n";
        let normalized = normalize_markdown(input).unwrap();
        assert!(normalized.blocks.len() >= 2);
    }
}
