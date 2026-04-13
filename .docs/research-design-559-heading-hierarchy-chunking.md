# Research & Design: #559 Heading Hierarchy and Educational Content Chunking

**Status**: Approved
**Date**: 2026-04-13
**Refs**: Gitea #559

---

## Phase 1: Research

### Essential Questions Check

| Question | Answer | Evidence |
|----------|--------|----------|
| Energizing? | Yes | Directly enables Odilo DLT revenue pipeline |
| Leverages strengths? | Yes | Extends existing markdown parser + ULID infrastructure |
| Meets real need? | Yes | Odilo DLT needs automated content ingest, textbook is 38K lines |

**Proceed**: Yes (3/3)

### Problem Statement

The Odilo DLT needs to process raw educational content (textbooks, articles) into chunked, tagged, skill-mapped objects for the Alma search system. The existing `terraphim-markdown-parser` provides block-level parsing with stable ULIDs but has no structural awareness -- no heading hierarchy, no section classification, no chunking.

### Current State Analysis

#### Existing Implementation (`terraphim-markdown-parser` v1.0.0)

The crate provides:
- `extract_first_heading()` -- H1 heading extraction via mdast AST
- `normalize_markdown()` -- parse into `NormalizedMarkdown { markdown, blocks: Vec<Block> }` with stable ULIDs
- `blocks_to_documents()` -- convert blocks into `Document` structs (`source_id#block_id`)
- `Block` struct with `id: Ulid`, `kind: BlockKind { Paragraph, ListItem }`, `span: Range<usize>`
- `<!-- terraphim:block-id:ULID -->` anchors for persistent cross-referencing
- Uses `markdown` crate (mdast-based AST) for parsing

#### Code Locations

| Component | Location | Purpose |
|-----------|----------|---------|
| Parser core | `crates/terraphim-markdown-parser/src/lib.rs` | All parsing, block ID, normalization |
| CLI binary | `crates/terraphim-markdown-parser/src/main.rs` | Stdin/file normalize + print |
| Scratchpad | `crates/terraphim-markdown-parser/src/scratchpad.rs` | pulldown-cmark experiment (unused) |
| Document type | `crates/terraphim_types/src/lib.rs:473` | `Document` struct used by `blocks_to_documents()` |

#### Key Constraint: No Downstream Consumers

`cargo tree --invert` shows nothing depends on `terraphim-markdown-parser` yet. We can extend the API freely without breaking changes.

#### Dependencies

| Crate | Version | Notes |
|-------|---------|-------|
| `markdown` | 1.0.0-alpha.21 | mdast AST parser, already handles all heading levels |
| `terraphim_types` | 1.15.0 | `Document`, `DocumentType` |
| `ulid` | 1.0.0 | Stable block IDs |
| `thiserror` | workspace | Error handling |

#### Critical Finding: `markdown` crate already parses all headings

The existing `extract_first_heading()` function uses `markdown::to_mdast()` which produces `Node::Heading` nodes with `depth: u8` (1-6). The current code only matches `depth == 1`. All heading nodes are already in the AST -- we just need to extract them.

### Vital Few (Essentialism)

| # | Essential Constraint | Why Vital |
|---|---------------------|-----------|
| 1 | Stable ULID inheritance | Enables re-ingest without losing entity extraction history |
| 2 | Configurable section patterns | Not hardcoded to one textbook |
| 3 | Heading tree with block attachment | Foundation for all downstream chunking |

#### Eliminated from Scope (5/25 Rule)

| Eliminated Item | Why Eliminated |
|----------------|----------------|
| YAML frontmatter parsing (#3 in issue) | Low value: most textbooks lack frontmatter. Can add later if needed. |
| Cross-reference detection (#4 in issue) | Regex pass is a separate concern. Block in future issue. |
| Token counting (exact wordpiece) | Approximate char/4 or whitespace-based is sufficient. Exact tokenization depends on model. |
| Overlap generation (2-sentence) | Can be computed at chunk consumption time, not at parse time. |
| DLT/pipeline integration | This is a library crate. Pipeline orchestration is out of scope. |

### Risks and Unknowns

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| "Power of Selling" has non-standard heading structure | Medium | High | Spike: parse first 500 lines to verify H1/H2 patterns |
| Very large file (38K lines) memory usage | Low | Medium | Stream parse or batch; mdast is already in-memory so this is fine |
| Section pattern matching too aggressive | Low | Low | Default to `Main`, only classify if pattern matches |

### Assumptions

| Assumption | Basis | Risk if Wrong |
|------------|-------|---------------|
| Chapters are H1 or H2 | Issue says "14 chapters" | May need configurable chapter heading level |
| Blocks attach to last preceding heading | Standard markdown semantics | Nested structures may complicate |
| `markdown` crate handles GFM extensions | Already using `ParseOptions::gfm()` | Verified in existing code |

---

## Phase 2: Design

### 5/25 Rule -- Top 5 In Scope

1. **Heading hierarchy detection** -- Build `HeadingNode` tree from mdast
2. **Section type classification** -- Configurable pattern matching
3. **Education-aware chunking** -- `ContentChunk` production from heading tree
4. **Block attachment to headings** -- Assign blocks to parent heading nodes
5. **`ContentChunk` type with stable ULIDs** -- Composite IDs from block ULIDs

### Avoid At All Cost

- YAML frontmatter parsing (separate concern)
- Cross-reference / link detection (separate concern)
- Exact tokenizer for token counts (model-dependent)
- Pipeline orchestration / DLT integration (different crate)
- Vector embedding generation (downstream consumer)

### Simplicity Check

**What if this could be easy?**

The simplest design: one new function `chunk_by_headings()` that takes `NormalizedMarkdown` + config, walks the existing AST to build heading nodes, attaches blocks, classifies sections, and returns `Vec<ContentChunk>`. No new traits, no new modules beyond what's needed for types.

**Senior Engineer Test**: This is a pure function transforming one data structure into another. No state, no IO, no async. It's already simple.

### Architecture

```
Markdown text
    |
    v
normalize_markdown()          [existing]
    |
    v
NormalizedMarkdown { markdown, blocks: Vec<Block> }
    |
    v
build_heading_tree()          [NEW - Step 2]
    |
    v
HeadingTree { roots: Vec<HeadingNode> }
    |
    v
classify_sections()           [NEW - Step 3, uses SectionConfig]
    |
    v
HeadingTree (with section_type filled)
    |
    v
chunk_by_headings()           [NEW - Step 4]
    |
    v
Vec<ContentChunk>
```

### Key Design Decisions

| Decision | Rationale | Alternatives Rejected |
|----------|-----------|----------------------|
| Store AST in `NormalizedMarkdown` | Avoids double-parse; `NormalizedMarkdown` can be modified | Re-parsing AST (wasteful, already parsed once) |
| `SectionConfig` as plain struct, not trait | YAGNI; pattern matching is simple | SectionClassifier trait (over-engineering) |
| `ContentChunk` owns its text (String) not borrows | Downstream consumers need owned data | Borrowing from NormalizedMarkdown (lifetimes complexity) |
| New modules `heading.rs` + `chunk.rs` | Keep lib.rs focused on existing block/normalize concerns | Everything in lib.rs (too large) |
| All heading levels captured (H1-H6) | Different textbooks use different conventions | Only H1/H2 (too restrictive) |

### File Changes

#### New Files

| File | Purpose |
|------|---------|
| `src/heading.rs` | `HeadingNode`, `HeadingTree`, `build_heading_tree()`, `SectionType`, `SectionConfig`, `classify_sections()` |
| `src/chunk.rs` | `ContentChunk`, `chunk_by_headings()` |
| `examples/textbook_chunks.rs` | Full pipeline demonstration on sample textbook content |
| `tests/chunking_integration.rs` | Integration tests for heading tree + chunking |

#### Modified Files

| File | Changes |
|------|---------|
| `src/lib.rs` | Add `pub mod chunk; pub mod heading;`, re-export key types, add `ast` field to `NormalizedMarkdown` |
| `Cargo.toml` | No changes needed (all deps already present) |

#### Deleted Files

| File | Reason |
|------|--------|
| `src/scratchpad.rs` | Unused pulldown-cmark experiment, replaced by examples/ |

### Public API Design

```rust
// === src/heading.rs ===

/// Section type classification based on heading patterns.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SectionType {
    Main,
    Sidebar(String),
    Career,
    Assessment,
}

/// Pattern rule for section classification.
#[derive(Debug, Clone)]
pub struct SectionPattern {
    pub pattern: String,
    pub section_type: SectionType,
}

/// Configuration for section classification.
#[derive(Debug, Clone)]
pub struct SectionConfig {
    pub rules: Vec<SectionPattern>,
}

impl SectionConfig {
    pub fn textbook_default() -> Self { ... }
}

/// A node in the heading hierarchy tree.
#[derive(Debug, Clone)]
pub struct HeadingNode {
    pub level: u8,
    pub title: String,
    pub section_type: SectionType,
    pub blocks: Vec<Ulid>,
    pub children: Vec<HeadingNode>,
    pub byte_range: Range<usize>,
    pub heading_number: Option<HeadingNumber>,
}

/// Parsed heading number (e.g., "3.2.1").
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HeadingNumber {
    pub components: Vec<u8>,
}

/// The full heading tree.
#[derive(Debug, Clone)]
pub struct HeadingTree {
    pub roots: Vec<HeadingNode>,
}

/// Build a heading tree from normalized markdown.
///
/// Walks the mdast AST to extract heading nodes, attaches blocks from
/// the normalized markdown by byte-range overlap, and assigns heading
/// numbers based on position.
pub fn build_heading_tree(
    normalized: &NormalizedMarkdown,
) -> Result<HeadingTree, MarkdownParserError> { ... }

/// Classify sections in the heading tree using pattern rules.
pub fn classify_sections(
    tree: &mut HeadingTree,
    config: &SectionConfig,
) { ... }

// === src/chunk.rs ===

/// An education-aware content chunk.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContentChunk {
    pub chunk_id: String,
    pub content_id: String,
    pub block_ids: Vec<Ulid>,
    pub chapter_number: Option<u8>,
    pub section_path: String,
    pub chunk_type: SectionType,
    pub text: String,
}

/// Produce content chunks from a classified heading tree.
///
/// Each heading node with content becomes one chunk. Sidebar, career,
/// and assessment sections are chunked as complete units. Main sections
/// are chunked per subsection.
pub fn chunk_by_headings(
    content_id: &str,
    tree: &HeadingTree,
    normalized: &NormalizedMarkdown,
) -> Vec<ContentChunk> { ... }
```

### Error Types

No new error variants needed. `MarkdownParserError::Markdown` covers AST parse failures. Block attachment failures use existing `MissingOrInvalidBlockId`.

### Test Strategy

#### Unit Tests

| Test | Location | Purpose |
|------|----------|---------|
| `test_build_heading_tree_simple` | `heading.rs` | 3-level heading hierarchy |
| `test_build_heading_tree_attaches_blocks` | `heading.rs` | Blocks assigned to correct parent |
| `test_classify_sections_default_config` | `heading.rs` | Power Selling/Selling U/Assessment patterns |
| `test_classify_sections_no_match_is_main` | `heading.rs` | Unknown headings default to Main |
| `test_chunk_by_headings_single_chapter` | `chunk.rs` | One H1 with paragraphs |
| `test_chunk_by_headings_nested` | `chunk.rs` | H1 > H2 > H3 hierarchy |
| `test_chunk_preserves_block_ulids` | `chunk.rs` | ULIDs from normalized markdown survive |
| `test_chunk_composite_ids` | `chunk.rs` | chunk_id = content_id + heading_path + first_block_ulid |
| `test_section_config_custom_patterns` | `heading.rs` | Non-default pattern rules work |

#### Integration Tests

| Test | Location | Purpose |
|------|----------|---------|
| `test_full_pipeline_normalize_tree_chunk` | `tests/chunking_integration.rs` | End-to-end: raw markdown -> chunks |
| `test_blocks_to_documents_compat` | `tests/chunking_integration.rs` | Existing `blocks_to_documents()` still works |

#### Property Tests

```rust
proptest! {
    #[test]
    fn heading_tree_never_panics(input: String) {
        let normalized = normalize_markdown(&input).ok();
        if let Some(n) = normalized {
            let _ = build_heading_tree(&n);
        }
    }
}
```

### Implementation Steps

#### Step 1: Types (`src/heading.rs` top half)
**Files:** `src/heading.rs` (new), `src/lib.rs` (add mod)
**Description:** Define `SectionType`, `SectionPattern`, `SectionConfig`, `HeadingNumber`, `HeadingNode`, `HeadingTree`
**Tests:** `SectionConfig::textbook_default()` has expected rules, type construction
**Estimated:** 1h

#### Step 2: Heading tree builder (`src/heading.rs` bottom half)
**Files:** `src/heading.rs`
**Description:** Implement `build_heading_tree()`. Use the stored AST from `NormalizedMarkdown`, extract all heading nodes (H1-H6), build tree, attach blocks by byte-range overlap.
**Tests:** `test_build_heading_tree_simple`, `test_build_heading_tree_attaches_blocks`
**Dependencies:** Step 1
**Estimated:** 2h (simpler since AST is already available)

#### Step 3: Section classification (`src/heading.rs`)
**Files:** `src/heading.rs`
**Description:** Implement `classify_sections()` with pattern matching. Walk tree, match heading titles against config rules.
**Tests:** `test_classify_sections_default_config`, `test_classify_sections_no_match_is_main`
**Dependencies:** Step 2
**Estimated:** 1h

#### Step 4: Chunking (`src/chunk.rs`)
**Files:** `src/chunk.rs` (new), `src/lib.rs` (add mod)
**Description:** Define `ContentChunk`, implement `chunk_by_headings()`. Flatten heading tree into chunks with composite IDs and text.
**Tests:** `test_chunk_by_headings_single_chapter`, `test_chunk_preserves_block_ulids`, `test_chunk_composite_ids`
**Dependencies:** Step 3
**Estimated:** 2h

#### Step 5: Integration tests
**Files:** `tests/chunking_integration.rs` (new)
**Description:** End-to-end test with synthetic multi-chapter markdown. Verify existing API compatibility.
**Tests:** `test_full_pipeline_normalize_tree_chunk`, `test_blocks_to_documents_compat`
**Dependencies:** Step 4
**Estimated:** 1h

**Total estimated effort: 7h**

### Rollback Plan

If issues discovered:
1. `heading.rs` and `chunk.rs` are new modules -- removal is clean
2. `lib.rs` changes are additive (`pub mod`) -- revert is trivial
3. No existing API changes -- zero regression risk

### New Dependencies

None. All required crates (`markdown`, `ulid`, `terraphim_types`, `serde`, `thiserror`) are already in the dependency graph.

### Performance Considerations

| Metric | Target | Measurement |
|--------|--------|-------------|
| Parse + tree + chunk 38K-line file | < 2s | Integration test benchmark |
| Memory for heading tree | < 50MB | Proportional to content size |

The AST is stored in `NormalizedMarkdown` by `normalize_markdown()`, so `build_heading_tree()` does zero parsing -- it walks the already-built AST.

---

## Open Questions (RESOLVED)

1. **Chapter heading level**: ALL levels (H1-H6). The tree captures every heading, not just H1/H2.
2. **Token count approximation**: `text.split_whitespace().count()` (simple, good enough).
3. **`scratchpad.rs`**: DELETE. Replace with `examples/` and integration tests.

## Design Revisions from Review

### NormalizedMarkdown CAN be modified
Store the mdast AST in `NormalizedMarkdown` so `build_heading_tree()` avoids a second parse. This changes `NormalizedMarkdown` from:

```rust
pub struct NormalizedMarkdown {
    pub markdown: String,
    pub blocks: Vec<Block>,
}
```

to:

```rust
pub struct NormalizedMarkdown {
    pub markdown: String,
    pub blocks: Vec<Block>,
    pub ast: Option<Node>,
}
```

The `ast` field is `Option<Node>` -- populated by `normalize_markdown()`, skipped by lower-level functions. This eliminates the "re-parse AST" concern in Step 2.

### scratchpad.rs -> deleted, replaced by examples/

The scratchpad contained an unused `pulldown-cmark` experiment. Replace with:
- `examples/textbook_chunks.rs` -- demonstrates full pipeline on sample content
- Existing integration tests cover the rest
