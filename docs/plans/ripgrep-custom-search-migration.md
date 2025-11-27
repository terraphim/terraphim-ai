# Migration Plan: Ripgrep to Custom Search Implementation

## Executive Summary

This plan outlines migrating from external `rg` process spawning to an embedded Rust search implementation, inspired by [Zed's project search optimization](https://zed.dev/blog/nerd-sniped-project-search). The goal is improved latency (especially time-to-first-result), better integration with Terraphim's async architecture, and elimination of external process overhead.

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

## Proposed Architecture

### Design Principles (from Zed)

1. **Task Prioritization**: Use `select_biased!` to ensure match extraction > buffer loading > file scanning
2. **Pipeline Parallelism**: Multiple async tasks connected via channels
3. **Early Exit**: Report first match ASAP, complete full scan later
4. **Buffer Awareness**: Prefer already-loaded content over disk reads

### Three-Stage Pipeline

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                        Custom Search Architecture                           │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  Stage 1: File Scanning          Stage 2: Buffer Loading    Stage 3: Match │
│  ─────────────────────           ──────────────────────     Extraction     │
│                                                              ─────────────  │
│  ┌─────────────────┐            ┌─────────────────┐        ┌─────────────┐ │
│  │  WalkBuilder    │            │  Buffer Cache   │        │ Full Match  │ │
│  │  (ignore crate) │───────────▶│  Check          │───────▶│ Extraction  │ │
│  │                 │  files w/  │                 │ loaded │             │ │
│  │  - .gitignore   │  matches   │  Memory-mapped  │ buffers│ - All ranges│ │
│  │  - file types   │            │  or async read  │        │ - Context   │ │
│  │  - glob filters │            │                 │        │ - Line nums │ │
│  └─────────────────┘            └─────────────────┘        └─────────────┘ │
│          │                              │                        │         │
│          │ Bounded Channel              │ Bounded Channel        │         │
│          ▼                              ▼                        ▼         │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │                    select_biased! Task Scheduler                     │   │
│  │                                                                      │   │
│  │   Priority 1: Extract matches from loaded buffers (immediate UI)    │   │
│  │   Priority 2: Load buffers for files with confirmed matches         │   │
│  │   Priority 3: Scan new file paths for potential matches             │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                                    │                                       │
│                                    ▼                                       │
│                          ┌─────────────────┐                               │
│                          │  Result Stream  │                               │
│                          │  (incremental)  │                               │
│                          └─────────────────┘                               │
└─────────────────────────────────────────────────────────────────────────────┘
```

## Implementation Plan

### Phase 1: New Crate Setup

**Create**: `crates/terraphim_search/`

**Dependencies**:
```toml
[dependencies]
# File traversal (from ripgrep)
ignore = "0.4"

# Regex matching (from ripgrep)
grep-regex = "0.1"
grep-searcher = "0.1"
grep-matcher = "0.1"

# Or use standalone regex crate
regex = "1"

# Async runtime
tokio = { version = "1", features = ["fs", "sync", "rt-multi-thread"] }

# Channels
async-channel = "2"

# Memory mapping (optional optimization)
memmap2 = "0.9"

# Existing terraphim deps
terraphim_types = { path = "../terraphim_types" }
```

**Module Structure**:
```
terraphim_search/
├── src/
│   ├── lib.rs              # Public API
│   ├── scanner.rs          # Stage 1: File scanning with ignore
│   ├── loader.rs           # Stage 2: Buffer loading/caching
│   ├── matcher.rs          # Stage 3: Match extraction
│   ├── scheduler.rs        # select_biased! orchestration
│   ├── config.rs           # Search configuration
│   └── results.rs          # Result types and streaming
├── benches/
│   └── search_benchmark.rs # Performance comparison
└── tests/
    ├── scanner_tests.rs
    ├── integration_tests.rs
    └── regression_tests.rs # Compare with ripgrep output
```

### Phase 2: Core Components

#### 2.1 Scanner (File Traversal)

```rust
// scanner.rs
use ignore::{WalkBuilder, WalkState};
use tokio::sync::mpsc;

pub struct Scanner {
    root: PathBuf,
    config: ScanConfig,
}

pub struct ScanConfig {
    pub hidden: bool,           // Include hidden files
    pub git_ignore: bool,       // Respect .gitignore
    pub file_types: Vec<String>,// e.g., ["md", "rs"]
    pub globs: Vec<String>,     // Custom glob patterns
    pub max_depth: Option<usize>,
}

impl Scanner {
    /// Parallel directory walk with early termination support
    pub fn scan_parallel(
        &self,
        pattern: &Regex,
        tx: mpsc::Sender<ScanResult>,
    ) -> JoinHandle<()> {
        let walker = WalkBuilder::new(&self.root)
            .hidden(!self.config.hidden)
            .git_ignore(self.config.git_ignore)
            .threads(num_cpus::get())
            .build_parallel();

        walker.run(|| {
            Box::new(|entry| {
                if let Ok(entry) = entry {
                    if entry.file_type().map(|t| t.is_file()).unwrap_or(false) {
                        // Quick first-match check (don't extract all matches yet)
                        if has_match(&entry.path(), pattern) {
                            tx.blocking_send(ScanResult::FileWithMatch(entry.path()));
                        }
                    }
                }
                WalkState::Continue
            })
        });
    }
}
```

#### 2.2 Buffer Loader

```rust
// loader.rs
use memmap2::Mmap;
use std::sync::Arc;
use dashmap::DashMap;

pub struct BufferCache {
    /// Already-loaded document content
    loaded: DashMap<PathBuf, Arc<String>>,
    /// Memory-mapped files for large content
    mapped: DashMap<PathBuf, Arc<Mmap>>,
}

impl BufferCache {
    /// Load file content, preferring cache then mmap then async read
    pub async fn load(&self, path: &Path) -> Result<BufferContent> {
        // Check if already loaded (from Terraphim document index)
        if let Some(content) = self.loaded.get(path) {
            return Ok(BufferContent::Cached(content.clone()));
        }

        let metadata = tokio::fs::metadata(path).await?;

        // Use mmap for large files (>1MB)
        if metadata.len() > 1_000_000 {
            let file = std::fs::File::open(path)?;
            let mmap = unsafe { Mmap::map(&file)? };
            let arc = Arc::new(mmap);
            self.mapped.insert(path.to_path_buf(), arc.clone());
            return Ok(BufferContent::Mapped(arc));
        }

        // Async read for smaller files
        let content = tokio::fs::read_to_string(path).await?;
        let arc = Arc::new(content);
        self.loaded.insert(path.to_path_buf(), arc.clone());
        Ok(BufferContent::Loaded(arc))
    }
}
```

#### 2.3 Match Extractor

```rust
// matcher.rs
use grep_regex::RegexMatcher;
use grep_searcher::{Searcher, Sink, SinkMatch};

pub struct MatchExtractor {
    matcher: RegexMatcher,
    context_lines: usize,
}

#[derive(Debug, Clone)]
pub struct Match {
    pub path: PathBuf,
    pub line_number: u64,
    pub line_content: String,
    pub byte_offset: u64,
    pub match_ranges: Vec<Range<usize>>,
    pub context_before: Vec<String>,
    pub context_after: Vec<String>,
}

impl MatchExtractor {
    pub fn extract_all(&self, path: &Path, content: &[u8]) -> Vec<Match> {
        let mut matches = Vec::new();
        let mut searcher = Searcher::new();

        searcher.search_slice(
            &self.matcher,
            content,
            MatchSink { matches: &mut matches, path },
        ).ok();

        matches
    }
}
```

#### 2.4 Scheduler (The Key Innovation)

```rust
// scheduler.rs
use tokio::select;

pub struct SearchScheduler {
    /// Files confirmed to have matches, waiting for full extraction
    pending_extraction: mpsc::Receiver<PathBuf>,
    /// Files needing content load
    pending_load: mpsc::Receiver<PathBuf>,
    /// New paths from scanner
    pending_scan: mpsc::Receiver<PathBuf>,
    /// Result output
    result_tx: mpsc::Sender<SearchResult>,
}

impl SearchScheduler {
    pub async fn run(mut self, buffer_cache: Arc<BufferCache>, extractor: Arc<MatchExtractor>) {
        loop {
            // CRITICAL: Priority-based task selection (Zed's key insight)
            tokio::select! {
                biased;  // Process in order of priority

                // Priority 1: Extract matches from already-loaded buffers
                // These give immediate UI feedback
                Some(path) = self.pending_extraction.recv() => {
                    if let Ok(content) = buffer_cache.get(&path) {
                        let matches = extractor.extract_all(&path, content.as_bytes());
                        for m in matches {
                            self.result_tx.send(SearchResult::Match(m)).await.ok();
                        }
                    }
                }

                // Priority 2: Load content for files with confirmed matches
                Some(path) = self.pending_load.recv() => {
                    if let Ok(_) = buffer_cache.load(&path).await {
                        // Queue for extraction
                        self.extraction_tx.send(path).await.ok();
                    }
                }

                // Priority 3: Scan new paths (lowest priority)
                Some(path) = self.pending_scan.recv() => {
                    // Quick check for match, queue for loading if found
                    if quick_match_check(&path) {
                        self.load_tx.send(path).await.ok();
                    }
                }

                else => break, // All channels closed
            }
        }
    }
}
```

### Phase 3: Integration with Terraphim

#### 3.1 New IndexMiddleware Implementation

```rust
// In crates/terraphim_middleware/src/indexer/native_search.rs

use terraphim_search::{SearchEngine, SearchConfig};

pub struct NativeSearchIndexer {
    engine: Arc<SearchEngine>,
    buffer_cache: Arc<BufferCache>,
}

impl IndexMiddleware for NativeSearchIndexer {
    async fn index(&self, needle: &str, haystack: &Haystack) -> Result<Index> {
        let config = SearchConfig::from_haystack(haystack);

        // Stream results as they come in
        let mut rx = self.engine.search(needle, &config).await;
        let mut index = Index::default();

        while let Some(result) = rx.recv().await {
            match result {
                SearchResult::Match(m) => {
                    let doc = self.match_to_document(m, haystack).await?;
                    index.insert(doc.id.clone(), doc);
                }
                SearchResult::Complete(stats) => {
                    log::debug!("Search complete: {:?}", stats);
                    break;
                }
            }
        }

        Ok(index)
    }
}
```

#### 3.2 Configuration Bridge

```rust
// config.rs
impl SearchConfig {
    pub fn from_haystack(haystack: &Haystack) -> Self {
        let extra = haystack.get_extra_parameters();

        Self {
            root: PathBuf::from(&haystack.location),
            file_types: extra.get("type")
                .map(|t| vec![t.clone()])
                .unwrap_or_else(|| vec!["md".to_string()]),
            globs: extra.get("glob")
                .map(|g| vec![g.clone()])
                .unwrap_or_default(),
            case_sensitive: extra.get("case_sensitive")
                .map(|v| v == "true")
                .unwrap_or(false),
            context_lines: extra.get("context")
                .and_then(|c| c.parse().ok())
                .unwrap_or(3),
            max_results: extra.get("max_count")
                .and_then(|c| c.parse().ok()),
            tags: extra.get("tag")
                .map(|t| t.split(',').map(String::from).collect())
                .unwrap_or_default(),
        }
    }
}
```

#### 3.3 ServiceType Extension

```rust
// In crates/terraphim_config/src/lib.rs

pub enum ServiceType {
    Ripgrep,      // Keep for backwards compatibility
    NativeSearch, // New embedded search
    Atomic,
    QueryRs,
    // ...
}
```

### Phase 4: Feature Parity Checklist

| Feature | Ripgrep | Custom | Notes |
|---------|---------|--------|-------|
| Case-insensitive search | ✅ | 🔲 | Default behavior |
| Regex support | ✅ | 🔲 | Via grep-regex |
| File type filtering | ✅ | 🔲 | Via ignore crate |
| Glob patterns | ✅ | 🔲 | Via ignore crate |
| Context lines | ✅ | 🔲 | Configurable |
| .gitignore respect | ✅ | 🔲 | Via ignore crate |
| Binary file detection | ✅ | 🔲 | Via grep-searcher |
| UTF-16 transcoding | ✅ | 🔲 | Via grep-searcher |
| Tag filtering (AND logic) | ✅ | 🔲 | Custom impl |
| Max count per file | ✅ | 🔲 | Early termination |
| Line numbers | ✅ | 🔲 | Built-in |
| JSON output | ✅ | N/A | Direct Rust types |
| Security validation | ✅ | 🔲 | Simplified (no CLI) |
| Result caching | ✅ | 🔲 | Integrate with BufferCache |

### Phase 5: Performance Benchmarks

#### Benchmark Criteria

```rust
// benches/search_benchmark.rs

#[bench]
fn bench_first_match_latency(b: &mut Bencher) {
    // Time from search start to first result
}

#[bench]
fn bench_total_throughput(b: &mut Bencher) {
    // Time to complete full search
}

#[bench]
fn bench_memory_usage(b: &mut Bencher) {
    // Peak memory during search
}

#[bench]
fn bench_vs_ripgrep_latency(b: &mut Bencher) {
    // Direct comparison with current impl
}
```

#### Target Metrics (based on Zed's results)

| Metric | Current (ripgrep) | Target | Zed's Achievement |
|--------|-------------------|--------|-------------------|
| First match latency | ~100-500ms | <50ms | 16.8s → 32ms (rust repo) |
| Total throughput | Baseline | ±10% | Slightly slower acceptable |
| Memory overhead | Low (external) | <100MB | - |
| Process spawn | ~10-50ms | 0ms | Eliminated |

### Phase 6: Migration Strategy

#### 6.1 Parallel Implementation

1. Keep existing `RipgrepIndexer` unchanged
2. Add `NativeSearchIndexer` as new option
3. Add `ServiceType::NativeSearch` to config
4. Run both in parallel for validation

#### 6.2 Feature Flag Rollout

```toml
# In Cargo.toml
[features]
default = ["ripgrep-search"]
ripgrep-search = []
native-search = ["terraphim_search"]
```

#### 6.3 Validation Testing

```rust
#[test]
fn test_native_vs_ripgrep_parity() {
    let query = "test";
    let haystack = test_haystack();

    let ripgrep_results = RipgrepIndexer::default().index(query, &haystack).await;
    let native_results = NativeSearchIndexer::new().index(query, &haystack).await;

    // Results should match (order may differ)
    assert_eq!(
        ripgrep_results.keys().collect::<HashSet<_>>(),
        native_results.keys().collect::<HashSet<_>>()
    );
}
```

#### 6.4 Deprecation Timeline

1. **v1.1**: Add `NativeSearch` as opt-in
2. **v1.2**: Make `NativeSearch` default, `Ripgrep` opt-out
3. **v2.0**: Remove `Ripgrep` support

## Risk Mitigation

### Known Challenges

1. **Regex compatibility**: Ensure grep-regex handles all patterns ripgrep accepts
2. **Unicode handling**: Test with non-ASCII filenames and content
3. **Large file handling**: Memory mapping edge cases
4. **Symlink handling**: Match ripgrep's symlink behavior

### Fallback Strategy

```rust
impl IndexMiddleware for HybridSearchIndexer {
    async fn index(&self, needle: &str, haystack: &Haystack) -> Result<Index> {
        // Try native search first
        match self.native.index(needle, haystack).await {
            Ok(index) => Ok(index),
            Err(e) => {
                log::warn!("Native search failed, falling back to ripgrep: {}", e);
                self.ripgrep.index(needle, haystack).await
            }
        }
    }
}
```

## Estimated Effort

| Phase | Components | Complexity |
|-------|------------|------------|
| Phase 1 | Crate setup, dependencies | Low |
| Phase 2 | Scanner, Loader, Matcher, Scheduler | High |
| Phase 3 | Integration, Config bridge | Medium |
| Phase 4 | Feature parity | Medium |
| Phase 5 | Benchmarks | Low |
| Phase 6 | Migration, Testing | Medium |

## References

- [Zed Blog: Nerd-Sniped by Project Search](https://zed.dev/blog/nerd-sniped-project-search)
- [ignore crate documentation](https://docs.rs/ignore)
- [grep crate documentation](https://docs.rs/grep)
- [ripgrep source code](https://github.com/BurntSushi/ripgrep)
- Current implementation: `crates/terraphim_middleware/src/command/ripgrep.rs`

## Appendix: Key Code from Zed

### select_biased! Pattern

```rust
// From Zed's implementation
loop {
    select_biased! {
        // Highest priority: extract matches from loaded buffers
        result = find_all_matches_rx.recv() => { ... }

        // Medium priority: confirm matches in candidate files
        result = find_first_match_rx.recv() => { ... }

        // Lowest priority: scan new paths
        result = scan_paths_rx.recv() => { ... }
    }
}
```

This prioritization ensures that:
1. UI shows results as soon as matches are confirmed
2. Known-match files get loaded before scanning continues
3. Scanning doesn't starve match extraction
