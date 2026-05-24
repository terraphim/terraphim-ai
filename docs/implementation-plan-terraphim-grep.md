# Implementation Plan: terraphim-grep — Intelligent Hybrid Grep

**Status**: Draft
**Research Doc**: `docs/research-terraphim-grep.md`
**Gitea Issue**: #1743
**Author**: Agent
**Date**: 2026-05-23
**Estimated Effort**: 12 days (as per research document)

## Overview

### Summary
Implement terraphim-grep: an intelligent grep tool that uses hybrid search (FFF + ripgrep + KG) for fast deterministic results, falling back to RLM only when needed, and learning by writing new concepts back to the knowledge graph.

### Approach
Build a new crate `terraphim_grep` that orchestrates existing components (RoleGraph, KgPathScorer, HaystackProvider, LlmClient) with new logic for sufficiency judgment and KG curation.

### Scope

**In Scope:**
- Create `terraphim_grep` crate with hybrid search orchestration
- Implement `SufficiencyJudge` with heuristic and LLM tiers
- Implement `RlmSignature` trait for structured outputs
- Implement KG curation loop (RLM → RoleGraph)
- Extend `terraphim_cli` with `grep` subcommand
- Maintain rlmgrep-compatible interface

**Out of Scope:**
- Firecracker VM integration (Docker sufficient for CLI)
- MCP server integration (future phase)
- Multi-modal ingestion (PDF/images)
- Streaming output

**Avoid At All Cost** (from 5/25 analysis):
- Loading ALL files into LLM context (rlmgrep's mistake)
- Premature optimisation of sufficiency thresholds
- Building our own LLM client instead of reusing terraphim_service

## Architecture

### Component Diagram

```
terraphim_grep
├── HybridSearcher (parallel search across 3 haystacks)
│   ├── CodeSearch (fff + ripgrep + KgPathScorer)
│   ├── DocSearch (HaystackProvider)
│   └── KgSearch (RoleGraph Aho-Corasick)
│
├── SufficiencyJudge (tiered evaluation)
│   ├── HeuristicJudge (coverage, confidence, diversity)
│   └── LlmJudge (fallback for uncertain cases)
│
├── RlmExecutor (RLM with pre-retrieved context)
│   ├── RlmContext (retrieved chunks + KG concepts)
│   └── RlmSignature implementations
│
└── KgCuration (RLM → KG feedback loop)
    └── ConceptExtractionSignature
```

### Data Flow

```
User Query
    │
    ▼
┌─────────────────┐
│ HybridSearcher  │  ← parallel tokio::join!
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│ SufficiencyJudge│
└────────┬────────┘
         │
    ┌────┴────┐
    ▼         ▼
┌───────┐  ┌───────┐
│Suffic.│  │Insuffic│
│Return │  │RLM ctx │
└───────┘  └────┬────┘
                │
                ▼
        ┌───────────────┐
        │ RlmExecutor   │
        └───────┬───────┘
                │
                ▼
        ┌───────────────┐
        │ KgCuration    │  (async, non-blocking)
        └───────────────┘
```

### Key Design Decisions

| Decision | Rationale | Alternatives Rejected |
|----------|-----------|----------------------|
| New `terraphim_grep` crate | Separation of concerns; doesn't pollute existing crates | Extend terraphim_cli directly (too coupled) |
| Reuse `terraphim_service::llm::LlmClient` | Already implemented; has chat_completion | Building custom HTTP client ( reinventing wheel) |
| Heuristic first, LLM second | Cost optimisation: 80-90% queries are free | LLM judge for all (expensive) |
| Async KG updates | Non-blocking; doesn't slow response | Synchronous updates (blocks user) |

### Eliminated Options (Essentialism)

| Option Rejected | Why Rejected | Risk of Including |
|-----------------|--------------|-------------------|
| Firecracker VMs | Overkill for CLI tool; Docker sufficient | Complexity, maintenance burden |
| Real-time KG updates | Could cause lock contention | Performance degradation |
| Custom LLM client | terraphim_service already has one | Duplicated code, diverging interfaces |

### Simplicity Check

**What if this could be easy?**
- Start with just hybrid search (no RLM) to prove architecture
- Add RLM fallback only when search returns empty
- KG curation as a background task

## File Changes

### New Crate Structure

```
crates/terraphim_grep/
├── Cargo.toml
└── src/
    ├── lib.rs                    # Module root + exports
    ├── error.rs                 # TerraphimGrepError
    ├── hybrid_searcher.rs        # HybridSearcher + ResultFusion
    ├── sufficiency_judge.rs      # SufficiencyJudge + HeuristicJudge
    ├── rlm_context.rs           # RlmContext building
    ├── signatures.rs            # RlmSignature trait + impls
    ├── kg_curation.rs           # KgCurationRlm
    └── cli.rs                   # CLI argument parsing (optional)
```

### New Files

| File | Purpose |
|------|---------|
| `crates/terraphim_grep/Cargo.toml` | Crate manifest |
| `crates/terraphim_grep/src/lib.rs` | Module root, public exports |
| `crates/terraphim_grep/src/error.rs` | Error types |
| `crates/terraphim_grep/src/hybrid_searcher.rs` | Parallel search orchestration |
| `crates/terraphim_grep/src/sufficiency_judge.rs` | Tiered sufficiency evaluation |
| `crates/terraphim_grep/src/rlm_context.rs` | RLM context construction |
| `crates/terraphim_grep/src/signatures.rs` | RlmSignature trait + implementations |
| `crates/terraphim_grep/src/kg_curation.rs` | KG curation feedback loop |

### Modified Files

| File | Changes |
|------|---------|
| `Cargo.toml` | Add `crates/terraphim_grep` to workspace members |
| `crates/terraphim_cli/src/main.rs` | Add `Grep` subcommand |

### Deleted Files
(None)

## API Design

### Public Types

```rust
// crates/terraphim_grep/src/lib.rs

pub struct TerraphimGrep {
    hybrid_searcher: Arc<HybridSearcher>,
    sufficiency_judge: Arc<SufficiencyJudge>,
    rlm_executor: Arc<RlmExecutor>,
    kg_curation: Arc<KgCurationRlm>,
}

impl TerraphimGrep {
    pub async fn search(&self, query: &str, options: GrepOptions) -> GrepResult;
}

// crates/terraphim_grep/src/error.rs

#[derive(Debug, thiserror::Error)]
pub enum TerraphimGrepError {
    #[error("search failed: {0}")]
    SearchFailed(String),

    #[error("LLM not configured: {0}")]
    LlmNotConfigured(String),

    #[error("insufficient results: {0}")]
    InsufficientResults(String),

    #[error("KG curation failed: {0}")]
    KgCurationFailed(String),
}

// crates/terraphim_grep/src/hybrid_searcher.rs

#[derive(Debug, Clone)]
pub struct GrepOptions {
    pub haystack: Haystack,        // Code, Docs, or All
    pub context_lines: usize,      // -C N
    pub max_results: usize,        // -n N
    pub force_rlm: bool,           // --rlm flag
    pub include_answer: bool,      // --answer flag
}

pub struct HybridResults {
    pub code_results: Vec<RetrievedChunk>,
    pub doc_results: Vec<RetrievedChunk>,
    pub kg_concepts: Vec<KgConcept>,
}

pub struct RetrievedChunk {
    pub content: String,
    pub source: String,
    pub line_start: Option<usize>,
    pub line_end: Option<usize>,
    pub relevance_score: f64,
    pub haystack: &'static str,
}

// crates/terraphim_grep/src/sufficiency_judge.rs

pub enum Sufficiency {
    Sufficient(Vec<RetrievedChunk>),
    NeedsSynthesis(Vec<RetrievedChunk>),
    NeedsExpansion(Vec<RetrievedChunk>),
    Insufficient(Vec<RetrievedChunk>),
}

pub struct HeuristicThresholds {
    pub min_coverage: f64,        // Query terms found (0.0-1.0)
    pub min_kg_confidence: f64,   // KG concept match score
    pub min_diversity: usize,     // Results from N different haystacks
}

impl Default for HeuristicThresholds {
    fn default() -> Self {
        Self {
            min_coverage: 0.7,
            min_kg_confidence: 0.5,
            min_diversity: 2,
        }
    }
}

// crates/terraphim_grep/src/signatures.rs

pub trait RlmSignature: Send + Sync {
    type Output: serde::Serialize + serde::de::DeserializeOwned;

    fn instructions(&self) -> String;
    fn parse(&self, raw: &str) -> Result<Self::Output, TerraphimGrepError>;
}

pub struct SearchResultSignature;
impl RlmSignature for SearchResultSignature {
    type Output = Vec<Match>;
}

pub struct AnswerSignature;
impl RlmSignature for AnswerSignature {
    type Output = AnswerWithCitations;
}

pub struct ConceptExtractionSignature;
impl RlmSignature for ConceptExtractionSignature {
    type Output = Vec<NewConcept>;
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Match {
    pub path: String,
    pub line: usize,
    pub line_end: Option<usize>,
    pub context: Vec<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AnswerWithCitations {
    pub answer: String,
    pub citations: Vec<Citation>,
    pub confidence: f64,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Citation {
    pub source: String,
    pub line: Option<usize>,
    pub excerpt: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct NewConcept {
    pub name: String,
    pub synonyms: Vec<String>,
    pub relationships: Vec<String>,
}
```

### Public Functions

```rust
// crates/terraphim_grep/src/lib.rs

/// Create a new TerraphimGrep instance
pub async fn new(config: GrepConfig) -> Result<Arc<Self>, TerraphimGrepError>;

/// Execute a grep query
pub async fn search(
    &self,
    query: &str,
    options: GrepOptions,
) -> Result<GrepResult, TerraphimGrepError>;

/// Get search result statistics
pub fn stats(&self) -> GrepStats;
```

### Error Types

```rust
// crates/terraphim_grep/src/error.rs

#[derive(Debug, thiserror::Error)]
pub enum TerraphimGrepError {
    #[error("search failed: {0}")]
    SearchFailed(String),

    #[error("LLM not configured: {0}")]
    LlmNotConfigured(#[from] terraphim_service::ServiceError),

    #[error("insufficient results: {0}")]
    InsufficientResults(String),

    #[error("KG curation failed: {0}")]
    KgCurationFailed(#[from] terraphim_rolegraph::Error),

    #[error("RLM execution failed: {0}")]
    RlmFailed(String),

    #[error("timeout after {0:?}")]
    Timeout(Duration),
}
```

## Test Strategy

### Unit Tests

| Test | Location | Purpose |
|------|----------|---------|
| `test_hybrid_search_parallel` | `hybrid_searcher.rs` | Verify parallel execution |
| `test_fusion_deduplication` | `hybrid_searcher.rs` | Verify result merging |
| `test_heuristic_coverage` | `sufficiency_judge.rs` | Coverage calculation |
| `test_heuristic_diversity` | `sufficiency_judge.rs` | Diversity calculation |
| `test_signature_parse_search` | `signatures.rs` | SearchResult parsing |
| `test_signature_parse_answer` | `signatures.rs` | AnswerWithCitations parsing |
| `test_concept_extraction` | `signatures.rs` | NewConcept parsing |
| `test_kg_curation_new_concepts` | `kg_curation.rs` | Concept added to graph |

### Integration Tests

| Test | Location | Purpose |
|------|----------|---------|
| `test_grep_search_only` | `tests/terraphim_grep.rs` | Full search-only flow |
| `test_grep_with_rlm_fallback` | `tests/terraphim_grep.rs` | RLM fallback triggered |
| `test_grep_answer_mode` | `tests/terraphim_grep.rs` | --answer flag |
| `test_grep_context_lines` | `tests/terraphim_grep.rs` | -C flag |
| `test_grep_haystack_filter` | `tests/terraphim_grep.rs` | --haystack flag |

### Property Tests

```rust
proptest! {
    #[test]
    fn test_heuristic_never_panics(thresholds: HeuristicThresholds, results: HybridResults) {
        let judge = SufficiencyJudge::new(thresholds);
        let _ = judge.judge(&results);
    }

    #[test]
    fn test_signature_parse_never_panics(raw: String, signature: TestSignature) {
        let _ = signature.parse(&raw);
    }
}
```

## Implementation Steps

### Step 1: Create crate structure + error types
**Files:** `crates/terraphim_grep/Cargo.toml`, `src/lib.rs`, `src/error.rs`
**Description:** Create new crate with module structure and error types
**Tests:** Unit tests for error display
**Estimated:** 2 hours

```rust
// Key code to write
#[derive(Debug, thiserror::Error)]
pub enum TerraphimGrepError {
    #[error("search failed: {0}")]
    SearchFailed(String),
    #[error("LLM not configured: {0}")]
    LlmNotConfigured(String),
    // ...
}
```

### Step 2: Implement HybridSearcher with parallel search
**Files:** `src/hybrid_searcher.rs`
**Description:** Implement parallel search across code, docs, KG haystacks
**Tests:** Unit tests for parallel execution, fusion, deduplication
**Dependencies:** Step 1
**Estimated:** 4 hours

```rust
pub struct HybridSearcher {
    code_search: Arc<dyn HaystackProvider>,
    doc_search: Arc<dyn HaystackProvider>,
    kg_query: Arc<RoleGraph>,
    path_scorer: Arc<KgPathScorer>,
}

impl HybridSearcher {
    pub async fn search(&self, query: &str) -> Result<HybridResults, TerraphimGrepError> {
        let (code, docs, kg) = tokio::join!(
            self.code_search.search(query),
            self.doc_search.search(query),
            self.kg_query.search(query),
        );
        // Fuse and rank
        self.fuse_and_rank(code?, docs?, kg?)
    }
}
```

### Step 3: Implement SufficiencyJudge with heuristic tiers
**Files:** `src/sufficiency_judge.rs`
**Description:** Implement heuristic-based sufficiency evaluation
**Tests:** Unit tests for coverage, confidence, diversity calculations
**Dependencies:** Step 2
**Estimated:** 3 hours

```rust
pub struct HeuristicJudge {
    thresholds: HeuristicThresholds,
}

impl HeuristicJudge {
    fn judge(&self, results: &HybridResults) -> Sufficiency {
        let coverage = self.calculate_coverage(results);
        let confidence = self.calculate_kg_confidence(results);
        let diversity = self.calculate_diversity(results);

        if coverage >= self.thresholds.min_coverage
            && confidence >= self.thresholds.min_kg_confidence
            && diversity >= self.thresholds.min_diversity
        {
            Sufficiency::Sufficient(results.to_chunks())
        } else if coverage > 0.3 {
            Sufficiency::NeedsSynthesis(results.to_chunks())
        } else {
            Sufficiency::Insufficient(results.to_chunks())
        }
    }
}
```

### Step 4: Implement RlmSignature trait and implementations
**Files:** `src/signatures.rs`
**Description:** Define trait and implement SearchResult, Answer, ConceptExtraction signatures
**Tests:** Unit tests for parsing
**Dependencies:** Step 1
**Estimated:** 2 hours

### Step 5: Implement RLM context building
**Files:** `src/rlm_context.rs`
**Description:** Build RLM context from retrieved chunks + KG concepts
**Tests:** Unit tests for context construction
**Dependencies:** Step 2, Step 4
**Estimated:** 2 hours

### Step 6: Implement KG curation feedback loop
**Files:** `src/kg_curation.rs`
**Description:** RLM extracts concepts → writes to RoleGraph → rebuilds automata
**Tests:** Unit tests for concept extraction and graph updates
**Dependencies:** Step 4, Step 5
**Estimated:** 3 hours

### Step 7: Wire TerraphimGrep together in lib.rs
**Files:** `src/lib.rs`
**Description:** Integrate all components; add search() method
**Tests:** Integration tests
**Dependencies:** Steps 1-6
**Estimated:** 2 hours

### Step 8: Add Grep subcommand to terraphim_cli
**Files:** `crates/terraphim_cli/src/main.rs`
**Description:** Add CLI interface compatible with rlmgrep
**Tests:** CLI integration tests
**Dependencies:** Step 7
**Estimated:** 3 hours

### Step 9: Integration tests + documentation
**Files:** `tests/terraphim_grep.rs`, README
**Description:** Full integration tests + user documentation
**Tests:** Integration tests pass
**Dependencies:** Step 8
**Estimated:** 2 hours

## Rollback Plan

If issues discovered:
1. Disable RLM fallback via `--rlm=false` flag
2. Revert to pure search-only mode
3. KG curation can be disabled via config flag

Feature flag: `TERRAPHIM_GREP_RLM_ENABLED=false`

## Migration (if applicable)

No database migrations needed - this is a new tool.

## Dependencies

### New Dependencies
| Crate | Version | Justification |
|-------|---------|---------------|
| (none) | - | All deps come from existing terraphim crates |

### Dependency Updates
(None - reusing existing crates)

## Software Release Definition (SRD)

Not applicable for internal tool.

## Performance Considerations

### Expected Performance
| Metric | Target | Measurement |
|--------|--------|-------------|
| Search-only latency | < 500ms | Benchmark |
| Search + RLM latency | < 5s | Benchmark |
| Memory (search-only) | < 10MB | Profiling |
| Memory (with RLM) | < 100MB | Profiling |

### Benchmarks to Add
```rust
#[bench]
fn bench_hybrid_search_1000_files(b: &mut Bencher) {
    let grep = TerraphimGrep::new_test();
    b.iter(|| grep.search("retry configuration", GrepOptions::default()));
}

#[bench]
fn bench_sufficiency_judge_heuristic(b: &mut Bencher) {
    let results = generate_test_results(100);
    let judge = SufficiencyJudge::default();
    b.iter(|| judge.judge(&results));
}
```

## Open Items

| Item | Status | Owner |
|------|--------|-------|
| Sufficiency threshold tuning | Pending | Need empirical data |
| LLM client configuration UX | Pending | How to configure API key? |
| KG curation rate limiting | Pending | Every query vs batched? |

## Approval

- [ ] Technical review complete
- [ ] Test strategy approved
- [ ] Performance targets agreed
- [ ] Human approval received
