# Migration Plan: Ripgrep to Custom Search Implementation

## Executive Summary

This plan outlines migrating from external `rg` process spawning to an embedded Rust search implementation, inspired by [Zed's project search optimization](https://zed.dev/blog/nerd-sniped-project-search). The goal is improved latency (especially time-to-first-result), better integration with Terraphim's async architecture, and elimination of external process overhead.

**Key Strategy**: Leverage existing Terraphim crates (`terraphim_automata`, `terraphim_types`, `terraphim_rolegraph`) and established patterns to minimize new code and ensure consistency.

## Current State Analysis

### Existing Ripgrep Integration

**Location**: `crates/terraphim_middleware/src/command/ripgrep.rs`

**Current Architecture**:
```
SearchQuery → RipgrepIndexer → tokio::process::Command("rg")
    → JSON Lines stdout → json_decode() → Vec<Message> → Index
```

**Key Characteristics**:
- External process spawning via `tokio::process::Command`
- JSON Lines output parsing (`--json` flag)
- Default args: `--json --trim -C3 --ignore-case -tmarkdown`
- 64-entry LRU cache with composite key
- Security: needle validation, flag whitelist
- Extra parameters: tag, glob, type, max_count, context, case_sensitive

**Pain Points**:
1. Process spawn overhead (~10-50ms per search)
2. All results must complete before any are returned
3. No integration with Terraphim's buffer management
4. Limited control over search scheduling/prioritization
5. Cannot leverage already-loaded document content

---

## Existing Codebase Assets to Leverage

### 1. Pattern Matching (terraphim_automata)

**Location**: `crates/terraphim_automata/src/matcher.rs`

**Already Available**:
- `find_matches()` - Aho-Corasick multi-pattern matching with positions
- `replace_matches()` - Pattern replacement with link formatting
- `extract_paragraphs_from_automata()` - Context extraction from matched terms

```rust
// Existing: crates/terraphim_automata/src/matcher.rs:13-42
pub fn find_matches(
    text: &str,
    thesaurus: Thesaurus,
    return_positions: bool,
) -> Result<Vec<Matched>> {
    let ac = AhoCorasick::builder()
        .match_kind(MatchKind::LeftmostLongest)
        .ascii_case_insensitive(true)
        .build(patterns)?;
    // ... returns Vec<Matched> with term, normalized_term, positions
}
```

**Reuse Strategy**: Extend `find_matches()` to support regex patterns alongside Aho-Corasick for thesaurus-based matching.

### 2. FST-Based Autocomplete (terraphim_automata)

**Location**: `crates/terraphim_automata/src/autocomplete.rs`

**Already Available**:
- `AutocompleteIndex` - FST-backed prefix search
- `autocomplete_search()` - Fast prefix matching
- `fuzzy_autocomplete_search()` - Jaro-Winkler fuzzy matching
- Serialization/deserialization for caching

```rust
// Existing: crates/terraphim_automata/src/autocomplete.rs:168-230
pub fn autocomplete_search(
    index: &AutocompleteIndex,
    prefix: &str,
    limit: Option<usize>,
) -> Result<Vec<AutocompleteResult>>
```

**Reuse Strategy**: Use FST for fast file path filtering and term matching.

### 3. Core Types (terraphim_types)

**Location**: `crates/terraphim_types/src/lib.rs`

**Already Available**:
- `Document` - id, url, title, body, description, tags, rank, source_haystack
- `Index` - AHashMap<String, Document> with insert/extend/iter
- `SearchQuery` - search_term, role, skip, limit, LogicalOperator (AND/OR)
- `Thesaurus` - Dictionary mapping for normalized terms

```rust
// Existing: crates/terraphim_types/src/lib.rs
pub struct Document {
    pub id: String,
    pub url: String,
    pub title: String,
    pub body: String,
    pub description: Option<String>,
    pub tags: Option<Vec<String>>,
    pub rank: Option<u64>,
    pub source_haystack: Option<String>,
}

pub struct Index {
    inner: AHashMap<String, Document>,
}
```

**Reuse Strategy**: Use existing types directly - no new type definitions needed.

### 4. Caching Pattern

**Location**: `crates/terraphim_middleware/src/indexer/ripgrep.rs:19-24`

**Already Available**:
```rust
#[cached(
    result = true,
    size = 64,
    key = "String",
    convert = r#"{ format!("{}::{}::{:?}", haystack.location, needle, haystack.get_extra_parameters()) }"#
)]
async fn cached_ripgrep_index(needle: &str, haystack: &Haystack) -> Result<Index>
```

**Reuse Strategy**: Apply identical caching pattern to new search implementation.

### 5. Directory Traversal (walkdir)

**Location**: Already in dependencies
- `terraphim_server/Cargo.toml`: `walkdir = "2.4"`
- `terraphim_middleware/Cargo.toml`: `walkdir = "2.4.0"` (dev-dependencies)
- `terraphim_kg_linter/Cargo.toml`: `walkdir = "2.5"`

**Existing Usage**:
```rust
// terraphim_server/src/lib.rs:303
for entry in WalkDir::new(&haystack.location)
    .into_iter()
    .filter_map(|e| e.ok())
    .filter(|e| e.path().extension().is_some_and(|ext| ext == "md"))
```

**Reuse Strategy**: Use walkdir for basic traversal, add `ignore` crate only for .gitignore support.

### 6. Async Patterns

**Location**: Multiple files

**Concurrent API Calls** (`crates/terraphim_middleware/src/haystack/query_rs.rs:181-186`):
```rust
let (reddit_results, suggest_results, crates_results, docs_results) = tokio::join!(
    self.search_reddit_posts(needle),
    self.search_suggest_api(needle),
    self.search_crates_io(needle),
    self.search_docs_rs(needle),
);
```

**Queue-based Processing** (`crates/terraphim_service/src/summarization_queue.rs`):
- `tokio::sync::mpsc` for task queues
- Priority-based task management

**Reuse Strategy**: Follow established async patterns for consistency.

### 7. IndexMiddleware Trait

**Location**: `crates/terraphim_middleware/src/indexer/mod.rs:21-32`

```rust
pub trait IndexMiddleware {
    fn index(
        &self,
        needle: &str,
        haystack: &terraphim_config::Haystack,
    ) -> impl std::future::Future<Output = Result<Index>> + Send;
}
```

**Reuse Strategy**: Implement this trait for the new search engine.

### 8. Scoring System

**Location**: `crates/terraphim_service/src/score/scored.rs`

```rust
pub struct SearchResults<T>(Vec<Scored<T>>);
pub struct Scored<T> { score: f64, value: T }
```

**Reuse Strategy**: Use existing scoring infrastructure for result ranking.

---

## Existing Dependencies to Leverage

```toml
# Already in workspace - NO NEW DEPENDENCIES for core functionality
aho-corasick = "1.0.2"      # Multi-pattern matching (terraphim_automata)
fst = "0.4"                  # Finite state transducer (terraphim_automata)
ahash = "0.8"                # Fast hashing (terraphim_types, middleware)
cached = "0.56.0"            # Query caching (middleware, automata)
strsim = "0.11"              # String similarity (automata)
walkdir = "2.4"              # Directory traversal (server, linter)
tokio = "1"                  # Async runtime (full features)
serde = "1.0"                # Serialization
regex = "1"                  # Regex matching (implicit via aho-corasick)

# NEW - only for .gitignore support
ignore = "0.4"               # From ripgrep - .gitignore handling

# OPTIONAL - for performance optimization
memmap2 = "0.9"              # Memory-mapped files (large file optimization)
```

---

## Proposed Architecture

### Design Principles (from Zed)

1. **Task Prioritization**: Use `tokio::select!` with `biased` to ensure match extraction > buffer loading > file scanning
2. **Pipeline Parallelism**: Multiple async tasks connected via `tokio::sync::mpsc` channels
3. **Early Exit**: Report first match ASAP, complete full scan later
4. **Buffer Awareness**: Prefer already-loaded content (from `terraphim_persistence`)

### Architecture Leveraging Existing Code

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                    Custom Search Architecture (Terraphim-Native)            │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  Stage 1: File Scanning          Stage 2: Content Loading   Stage 3: Match │
│  ─────────────────────           ─────────────────────────  Extraction     │
│                                                              ─────────────  │
│  ┌─────────────────┐            ┌─────────────────┐        ┌─────────────┐ │
│  │  WalkDir +      │            │ terraphim_      │        │ terraphim_  │ │
│  │  ignore crate   │───────────▶│ persistence     │───────▶│ automata    │ │
│  │                 │  files w/  │ (existing docs) │ loaded │             │ │
│  │  - .gitignore   │  potential │                 │ content│ find_matches│ │
│  │  - file types   │  matches   │  + tokio::fs    │        │ + regex     │ │
│  │  - glob filters │            │  for new files  │        │ extraction  │ │
│  └─────────────────┘            └─────────────────┘        └─────────────┘ │
│          │                              │                        │         │
│          │ mpsc::channel                │ mpsc::channel          │         │
│          ▼                              ▼                        ▼         │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │             tokio::select! { biased; ... } Scheduler                │   │
│  │                                                                      │   │
│  │   Priority 1: Extract matches → SearchResults<Document>             │   │
│  │   Priority 2: Load content from persistence/disk                    │   │
│  │   Priority 3: Scan file paths                                       │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                                    │                                       │
│                                    ▼                                       │
│                          ┌─────────────────┐                               │
│                          │     Index       │  ← Existing type              │
│                          │  (AHashMap)     │                               │
│                          └─────────────────┘                               │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

## Implementation Plan

### Phase 1: Extend terraphim_automata (NOT new crate)

**Rationale**: Add regex search capability to existing crate rather than creating new crate.

**New Module**: `crates/terraphim_automata/src/regex_matcher.rs`

```rust
//! Regex-based text matching, complementing Aho-Corasick thesaurus matching.

use regex::{Regex, RegexBuilder};
use crate::Result;

/// Match result with position and context
#[derive(Debug, Clone)]
pub struct RegexMatch {
    pub line_number: usize,
    pub line_content: String,
    pub byte_offset: usize,
    pub match_start: usize,
    pub match_end: usize,
}

/// Find all regex matches in text with line numbers
pub fn find_regex_matches(
    text: &str,
    pattern: &str,
    case_insensitive: bool,
) -> Result<Vec<RegexMatch>> {
    let regex = RegexBuilder::new(pattern)
        .case_insensitive(case_insensitive)
        .build()?;

    let mut matches = Vec::new();
    let mut line_number = 1;
    let mut line_start = 0;

    for line in text.lines() {
        for mat in regex.find_iter(line) {
            matches.push(RegexMatch {
                line_number,
                line_content: line.to_string(),
                byte_offset: line_start + mat.start(),
                match_start: mat.start(),
                match_end: mat.end(),
            });
        }
        line_number += 1;
        line_start += line.len() + 1; // +1 for newline
    }

    Ok(matches)
}

/// Extract context lines around matches
pub fn extract_context(
    text: &str,
    match_line: usize,
    context_before: usize,
    context_after: usize,
) -> (Vec<String>, Vec<String>) {
    let lines: Vec<&str> = text.lines().collect();
    let start = match_line.saturating_sub(context_before + 1);
    let end = (match_line + context_after).min(lines.len());

    let before = lines[start..match_line.saturating_sub(1)]
        .iter()
        .map(|s| s.to_string())
        .collect();
    let after = lines[match_line..end]
        .iter()
        .map(|s| s.to_string())
        .collect();

    (before, after)
}
```

**Update Cargo.toml** (`crates/terraphim_automata/Cargo.toml`):
```toml
[dependencies]
# ... existing deps ...
regex = "1"  # Add for regex matching
```

### Phase 2: Add File Scanner Module

**New Module**: `crates/terraphim_automata/src/scanner.rs`

```rust
//! File scanning with gitignore support, leveraging existing walkdir usage patterns.

use ignore::{WalkBuilder, WalkState, DirEntry};
use std::path::{Path, PathBuf};
use tokio::sync::mpsc;

/// Configuration for file scanning (mirrors ripgrep extra_parameters)
#[derive(Debug, Clone, Default)]
pub struct ScanConfig {
    pub hidden: bool,
    pub git_ignore: bool,
    pub file_types: Vec<String>,  // e.g., ["md", "rs"]
    pub globs: Vec<String>,
    pub max_depth: Option<usize>,
}

impl ScanConfig {
    /// Create from Haystack extra_parameters (existing pattern)
    pub fn from_extra_parameters(params: &std::collections::HashMap<String, String>) -> Self {
        Self {
            hidden: params.get("hidden").map(|v| v == "true").unwrap_or(false),
            git_ignore: params.get("git_ignore").map(|v| v != "false").unwrap_or(true),
            file_types: params.get("type")
                .map(|t| t.split(',').map(String::from).collect())
                .unwrap_or_else(|| vec!["md".to_string()]),
            globs: params.get("glob")
                .map(|g| vec![g.clone()])
                .unwrap_or_default(),
            max_depth: params.get("max_depth").and_then(|d| d.parse().ok()),
        }
    }
}

/// Scan directory for files matching configuration
pub fn scan_files(root: &Path, config: &ScanConfig) -> Vec<PathBuf> {
    let mut builder = WalkBuilder::new(root);

    builder
        .hidden(!config.hidden)
        .git_ignore(config.git_ignore);

    if let Some(depth) = config.max_depth {
        builder.max_depth(Some(depth));
    }

    // Add file type filters
    for ft in &config.file_types {
        builder.types(
            ignore::types::TypesBuilder::new()
                .add_defaults()
                .select(ft)
                .build()
                .unwrap_or_default()
        );
    }

    builder.build()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().map(|t| t.is_file()).unwrap_or(false))
        .filter(|e| {
            if config.file_types.is_empty() {
                true
            } else {
                e.path().extension()
                    .and_then(|ext| ext.to_str())
                    .map(|ext| config.file_types.iter().any(|ft| ft == ext))
                    .unwrap_or(false)
            }
        })
        .map(|e| e.path().to_path_buf())
        .collect()
}

/// Parallel file scanner with channel output
pub fn scan_files_parallel(
    root: PathBuf,
    config: ScanConfig,
    tx: mpsc::UnboundedSender<PathBuf>,
) {
    let walker = WalkBuilder::new(&root)
        .hidden(!config.hidden)
        .git_ignore(config.git_ignore)
        .threads(num_cpus::get().min(8))
        .build_parallel();

    walker.run(|| {
        let tx = tx.clone();
        let file_types = config.file_types.clone();

        Box::new(move |entry: Result<DirEntry, ignore::Error>| {
            if let Ok(entry) = entry {
                if entry.file_type().map(|t| t.is_file()).unwrap_or(false) {
                    let dominated = file_types.is_empty() || entry.path()
                        .extension()
                        .and_then(|e| e.to_str())
                        .map(|e| file_types.iter().any(|ft| ft == e))
                        .unwrap_or(false);

                    if matches {
                        let _ = tx.send(entry.path().to_path_buf());
                    }
                }
            }
            WalkState::Continue
        })
    });
}
```

### Phase 3: Search Engine Combining Existing Components

**New Module**: `crates/terraphim_automata/src/search_engine.rs`

```rust
//! High-level search engine combining scanner, loader, and matcher.

use crate::{
    find_matches, Matched,
    regex_matcher::{find_regex_matches, RegexMatch, extract_context},
    scanner::{scan_files, ScanConfig},
    Result,
};
use terraphim_types::{Document, Index, Thesaurus};
use tokio::sync::mpsc;
use std::path::{Path, PathBuf};
use ahash::AHashSet;

/// Search result from the engine
#[derive(Debug, Clone)]
pub enum SearchResult {
    /// A document with matches
    Match(SearchMatch),
    /// Search completed
    Complete { files_scanned: usize, matches_found: usize },
    /// Progress update
    Progress { files_scanned: usize, total_files: usize },
}

#[derive(Debug, Clone)]
pub struct SearchMatch {
    pub path: PathBuf,
    pub matches: Vec<MatchDetail>,
    pub content: String,
}

#[derive(Debug, Clone)]
pub struct MatchDetail {
    pub line_number: usize,
    pub line_content: String,
    pub context_before: Vec<String>,
    pub context_after: Vec<String>,
}

/// Search configuration
#[derive(Debug, Clone)]
pub struct SearchConfig {
    pub root: PathBuf,
    pub scan_config: ScanConfig,
    pub case_insensitive: bool,
    pub context_lines: usize,
    pub max_results: Option<usize>,
    /// Use thesaurus-based matching (Aho-Corasick) instead of regex
    pub use_thesaurus: bool,
    pub thesaurus: Option<Thesaurus>,
}

/// Main search engine
pub struct SearchEngine;

impl SearchEngine {
    /// Search with streaming results (Zed-style priority scheduling)
    pub async fn search_streaming(
        pattern: &str,
        config: SearchConfig,
        result_tx: mpsc::Sender<SearchResult>,
    ) -> Result<()> {
        let files = scan_files(&config.root, &config.scan_config);
        let total_files = files.len();

        let (file_tx, mut file_rx) = mpsc::channel::<PathBuf>(100);
        let (content_tx, mut content_rx) = mpsc::channel::<(PathBuf, String)>(50);

        // Spawn file reader task
        let config_clone = config.clone();
        tokio::spawn(async move {
            for path in files {
                if let Ok(content) = tokio::fs::read_to_string(&path).await {
                    let _ = content_tx.send((path, content)).await;
                }
            }
        });

        // Process with priority scheduling
        let mut files_scanned = 0;
        let mut matches_found = 0;
        let pattern = pattern.to_string();

        loop {
            tokio::select! {
                biased;

                // Priority 1: Process loaded content
                Some((path, content)) = content_rx.recv() => {
                    files_scanned += 1;

                    let matches = if config.use_thesaurus {
                        if let Some(ref thesaurus) = config.thesaurus {
                            Self::find_thesaurus_matches(&content, thesaurus, config.context_lines)?
                        } else {
                            vec![]
                        }
                    } else {
                        Self::find_regex_matches_with_context(
                            &content, &pattern, config.case_insensitive, config.context_lines
                        )?
                    };

                    if !matches.is_empty() {
                        matches_found += matches.len();
                        let _ = result_tx.send(SearchResult::Match(SearchMatch {
                            path,
                            matches,
                            content,
                        })).await;

                        // Check max results
                        if let Some(max) = config.max_results {
                            if matches_found >= max {
                                break;
                            }
                        }
                    }

                    // Send progress
                    if files_scanned % 100 == 0 {
                        let _ = result_tx.send(SearchResult::Progress {
                            files_scanned,
                            total_files,
                        }).await;
                    }
                }

                else => break,
            }
        }

        let _ = result_tx.send(SearchResult::Complete {
            files_scanned,
            matches_found,
        }).await;

        Ok(())
    }

    /// Synchronous search returning Index (compatible with IndexMiddleware)
    pub async fn search(pattern: &str, config: SearchConfig) -> Result<Index> {
        let (tx, mut rx) = mpsc::channel(100);

        let pattern = pattern.to_string();
        tokio::spawn(async move {
            let _ = Self::search_streaming(&pattern, config, tx).await;
        });

        let mut index = Index::default();

        while let Some(result) = rx.recv().await {
            if let SearchResult::Match(m) = result {
                let doc = Document {
                    id: m.path.to_string_lossy().to_string(),
                    url: m.path.to_string_lossy().to_string(),
                    title: m.path.file_stem()
                        .and_then(|s| s.to_str())
                        .unwrap_or("untitled")
                        .to_string(),
                    body: m.content,
                    description: m.matches.first()
                        .map(|m| m.line_content.chars().take(200).collect()),
                    summarization: None,
                    stub: None,
                    tags: None,
                    rank: None,
                    source_haystack: Some(config.root.to_string_lossy().to_string()),
                };
                index.insert(doc.id.clone(), doc);
            }
        }

        Ok(index)
    }

    fn find_thesaurus_matches(
        content: &str,
        thesaurus: &Thesaurus,
        context_lines: usize,
    ) -> Result<Vec<MatchDetail>> {
        // Use existing find_matches from terraphim_automata
        let matches = find_matches(content, thesaurus.clone(), true)?;

        matches.into_iter().map(|m| {
            let (start, _end) = m.pos.unwrap_or((0, 0));
            let line_number = content[..start].matches('\n').count() + 1;
            let line_start = content[..start].rfind('\n').map(|i| i + 1).unwrap_or(0);
            let line_end = content[start..].find('\n').map(|i| start + i).unwrap_or(content.len());
            let line_content = content[line_start..line_end].to_string();

            let (context_before, context_after) = extract_context(
                content, line_number, context_lines, context_lines
            );

            Ok(MatchDetail {
                line_number,
                line_content,
                context_before,
                context_after,
            })
        }).collect()
    }

    fn find_regex_matches_with_context(
        content: &str,
        pattern: &str,
        case_insensitive: bool,
        context_lines: usize,
    ) -> Result<Vec<MatchDetail>> {
        let matches = find_regex_matches(content, pattern, case_insensitive)?;

        Ok(matches.into_iter().map(|m| {
            let (context_before, context_after) = extract_context(
                content, m.line_number, context_lines, context_lines
            );

            MatchDetail {
                line_number: m.line_number,
                line_content: m.line_content,
                context_before,
                context_after,
            }
        }).collect())
    }
}
```

### Phase 4: IndexMiddleware Implementation

**New File**: `crates/terraphim_middleware/src/indexer/native_search.rs`

```rust
//! Native search indexer using terraphim_automata's search engine.

use crate::{Error, Result};
use super::IndexMiddleware;
use terraphim_automata::search_engine::{SearchEngine, SearchConfig, ScanConfig};
use terraphim_config::Haystack;
use terraphim_types::Index;
use cached::proc_macro::cached;
use std::path::PathBuf;

/// Native search indexer - replaces RipgrepIndexer
#[derive(Default, Clone)]
pub struct NativeSearchIndexer;

#[cached(
    result = true,
    size = 64,
    key = "String",
    convert = r#"{ format!("native::{}::{}::{:?}", haystack.location, needle, haystack.get_extra_parameters()) }"#
)]
async fn cached_native_search(needle: &str, haystack: &Haystack) -> Result<Index> {
    let extra = haystack.get_extra_parameters();

    let config = SearchConfig {
        root: PathBuf::from(&haystack.location),
        scan_config: ScanConfig::from_extra_parameters(extra),
        case_insensitive: extra.get("case_sensitive")
            .map(|v| v != "true")
            .unwrap_or(true),
        context_lines: extra.get("context")
            .and_then(|c| c.parse().ok())
            .unwrap_or(3),
        max_results: extra.get("max_count")
            .and_then(|c| c.parse().ok()),
        use_thesaurus: false,  // Use regex by default
        thesaurus: None,
    };

    SearchEngine::search(needle, config).await
        .map_err(|e| Error::Other(e.to_string()))
}

impl IndexMiddleware for NativeSearchIndexer {
    async fn index(&self, needle: &str, haystack: &Haystack) -> Result<Index> {
        cached_native_search(needle, haystack).await
    }
}
```

### Phase 5: ServiceType Integration

**Update**: `crates/terraphim_config/src/lib.rs`

```rust
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum ServiceType {
    Ripgrep,       // Keep for backwards compatibility
    NativeSearch,  // New embedded search
    Atomic,
    QueryRs,
    ClickUp,
    Mcp,
    Perplexity,
    GrepApp,
}
```

**Update**: `crates/terraphim_middleware/src/indexer/mod.rs`

```rust
use native_search::NativeSearchIndexer;

pub async fn search_haystacks(...) -> Result<Index> {
    // ...
    let native_search = NativeSearchIndexer::default();

    for haystack in &role.haystacks {
        let index = match haystack.service {
            ServiceType::Ripgrep => ripgrep.index(needle, haystack).await?,
            ServiceType::NativeSearch => native_search.index(needle, haystack).await?,
            // ... other services
        };
        // ...
    }
}
```

---

## Feature Parity Checklist

| Feature | Ripgrep | Native | Implementation |
|---------|---------|--------|----------------|
| Case-insensitive | ✅ | ✅ | RegexBuilder::case_insensitive() |
| Regex support | ✅ | ✅ | regex crate |
| Thesaurus matching | ❌ | ✅ | Existing find_matches() |
| File type filtering | ✅ | ✅ | ignore crate types |
| Glob patterns | ✅ | ✅ | ignore crate |
| Context lines | ✅ | ✅ | extract_context() |
| .gitignore respect | ✅ | ✅ | ignore crate |
| Tag filtering (AND) | ✅ | ✅ | Multi-pattern matching |
| Max count | ✅ | ✅ | Early termination |
| Line numbers | ✅ | ✅ | Built-in |
| Result caching | ✅ | ✅ | cached macro (identical pattern) |
| Streaming results | ❌ | ✅ | mpsc channels |
| Priority scheduling | ❌ | ✅ | tokio::select! biased |

---

## Performance Benchmarks

**Add to**: `crates/terraphim_automata/benches/search_benchmark.rs`

```rust
use criterion::{criterion_group, criterion_main, Criterion, BenchmarkId};
use terraphim_automata::search_engine::{SearchEngine, SearchConfig, ScanConfig};
use std::path::PathBuf;

fn bench_native_vs_ripgrep(c: &mut Criterion) {
    let mut group = c.benchmark_group("search_comparison");

    let test_dir = PathBuf::from("../../docs/src");
    let patterns = vec!["test", "knowledge", "graph.*search"];

    for pattern in patterns {
        group.bench_with_input(
            BenchmarkId::new("native", pattern),
            pattern,
            |b, p| {
                b.to_async(tokio::runtime::Runtime::new().unwrap())
                    .iter(|| async {
                        let config = SearchConfig {
                            root: test_dir.clone(),
                            scan_config: ScanConfig::default(),
                            case_insensitive: true,
                            context_lines: 3,
                            max_results: None,
                            use_thesaurus: false,
                            thesaurus: None,
                        };
                        SearchEngine::search(p, config).await
                    })
            }
        );

        group.bench_with_input(
            BenchmarkId::new("ripgrep", pattern),
            pattern,
            |b, p| {
                b.to_async(tokio::runtime::Runtime::new().unwrap())
                    .iter(|| async {
                        let cmd = terraphim_middleware::command::RipgrepCommand::default();
                        cmd.run(p, &test_dir).await
                    })
            }
        );
    }

    group.finish();
}

criterion_group!(benches, bench_native_vs_ripgrep);
criterion_main!(benches);
```

---

## Migration Strategy

### Phase 1: Add Native Search (Non-Breaking)
1. Add `regex` to terraphim_automata dependencies
2. Add `ignore` to terraphim_automata dependencies
3. Implement new modules in terraphim_automata
4. Add `NativeSearchIndexer` to middleware
5. Add `ServiceType::NativeSearch`

### Phase 2: Validation
1. Create comparison tests (native vs ripgrep)
2. Run benchmarks
3. Test with real haystack configurations

### Phase 3: Default Switch
1. Change default `ServiceType` to `NativeSearch`
2. Update documentation
3. Deprecate `Ripgrep` service type

### Phase 4: Cleanup (Future)
1. Remove `RipgrepIndexer`
2. Remove ripgrep command module
3. Update all configs

---

## Files to Modify/Create

### New Files
- `crates/terraphim_automata/src/regex_matcher.rs`
- `crates/terraphim_automata/src/scanner.rs`
- `crates/terraphim_automata/src/search_engine.rs`
- `crates/terraphim_middleware/src/indexer/native_search.rs`
- `crates/terraphim_automata/benches/search_benchmark.rs`

### Modified Files
- `crates/terraphim_automata/Cargo.toml` - add regex, ignore
- `crates/terraphim_automata/src/lib.rs` - export new modules
- `crates/terraphim_config/src/lib.rs` - add ServiceType::NativeSearch
- `crates/terraphim_middleware/src/indexer/mod.rs` - add NativeSearchIndexer

---

## References

- [Zed Blog: Nerd-Sniped by Project Search](https://zed.dev/blog/nerd-sniped-project-search)
- [ignore crate documentation](https://docs.rs/ignore)
- [ripgrep source code](https://github.com/BurntSushi/ripgrep)
- Current implementation: `crates/terraphim_middleware/src/command/ripgrep.rs`
- Existing matcher: `crates/terraphim_automata/src/matcher.rs`
- Existing autocomplete: `crates/terraphim_automata/src/autocomplete.rs`
