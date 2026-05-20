# Research: rlmgrep vs Terraphim — Proposal for Intelligent Hybrid Grep

**Date:** 2026-05-18  
**Goal:** Design a Terraphim-powered intelligent grep that surpasses rlmgrep by leveraging hybrid search (KG + documentation + code + external haystacks) with RLM fallback and KG curation.

---

## 1. Executive Summary

`rlmgrep` demonstrates that natural-language code search is valuable but implements it inefficiently: it loads **all files into LLM context** and relies on brute-force reasoning. This is O(corpus_size) in tokens and expensive.

Terraphim already possesses superior infrastructure:
- **Fast search**: `fff_search` (frecency-ranked), `ripgrep`, `RoleGraph` (Aho-Corasick O(n))
- **Relevance scoring**: `KgPathScorer` boosts by KG concept matches
- **Multi-haystack**: `HaystackProvider` trait for code, docs, JMAP, external sources
- **Structured execution**: `terraphim_rlm` with Firecracker/Docker sandboxing
- **Knowledge graph**: `RoleGraph` with hot-reload thesaurus and learning indexer

The opportunity is to create **`terraphim-grep`** — an intelligent grep that:
1. **Searches first** using hybrid KG + file + haystack search (deterministic, fast, cheap)
2. **Falls back to RLM** only when search is insufficient (complex reasoning, synthesis)
3. **Uses RLM to curate the KG** — extracting new concepts from interactions to improve future searches
4. **Outputs structured results** with citations (like rlmgrep's `--signature-json` but better)

This is the **inverse of rlmgrep**: search-first, RLM-last, continuously learning.

---

## 2. rlmgrep Analysis

### 2.1 Architecture

```text
User Query: "Where is retry/backoff configured?"
    |
    v
[File Discovery]  →  collect_candidates()  →  Walk filesystem
    |
    v
[Ingestion]  →  load_files()  →  Convert PDF/office/image/audio to text
    |
    v
[Context Build]  →  directory: {path: full_text}  +  ASCII file_map
    |
    v
[DSPy RLM]  →  LLM reasons over ALL files, outputs Match{path, line}
    |
    v
[Verify]  →  Drop hallucinated line numbers
    |
    v
[Output]  →  rg-style headings with line numbers
```

### 2.2 Critical Flaw: Brute-Force Context Loading

```python
# rlmgrep loads ENTIRE file contents into the LLM
directory = {k: v.text for k, v in files.items()}  # ALL files, ALL text

# Limits: aborts at 5,000 files
# Cost: $0.01-0.05 per query
# No learning between queries
```

**Why this fails:**
- Large repos (5K+ files) exceed context windows
- Every query re-reads the filesystem (no index)
- No relevance pre-filtering
- Expensive: LLM processes full corpus for every query

### 2.3 What rlmgrep Gets Right

| Feature | Value |
|---------|-------|
| **Natural language queries** | No regex crafting needed |
| **Structured signatures** | `--signature-json` for agent-consumable output |
| **Multi-modal ingestion** | PDFs, images, audio via MarkItDown |
| **Sidecar caching** | `.filename.pdf.md` avoids re-conversion |
| **Grep-compatible output** | rg-style headings with line numbers |
| **Context lines** | `-C`, `-A`, `-B` for surrounding context |

---

## 3. Terraphim Infrastructure Assessment

### 3.1 Existing Search Capabilities

| Component | Capability | Relevance |
|-----------|-----------|-----------|
| `RoleGraph` | Aho-Corasick O(n) concept detection; TF-IDF fallback; graph ranking | **Core** — fast concept matching |
| `KgPathScorer` | Boosts file search by KG concept matches in paths | **Core** — relevance ranking |
| `fff_search` | Fast frecency-ranked file finder; ripgrep integration | **Core** — file discovery |
| `HaystackProvider` | Uniform async search over heterogeneous backends | **Core** — multi-source search |
| `terraphim_automata` | FST autocomplete; Jaro-Winkler fuzzy search | **UX** — query suggestions |
| `terraphim_mcp_server` | Exposes search as MCP tools | **Integration** |
| `learning_indexer` | Indexes SharedLearnings into RoleGraph | **Core** — RLM writes back |
| `terraphim_rlm` | Sandboxed execution, query loop, budgets | **Core** — RLM fallback |

### 3.2 Critical Gap: LLM Bridge is a Stub

`crates/terraphim_rlm/src/llm_bridge.rs` lines 192-201:

```rust
// TODO: Actually call the LLM service
// For now, return a stub response
let response_text = format!("[LLM Bridge stub] Query: {}...", ...);
```

**Without real LLM integration, terraphim_rlm cannot participate in intelligent grep.**

### 3.3 What Terraphim Has That rlmgrep Lacks

| Feature | Terraphim | rlmgrep |
|---------|----------|---------|
| **Pre-indexed search** | RoleGraph (Aho-Corasick) + file frecency | None (re-reads every query) |
| **Multi-haystack** | Code + docs + external via HaystackProvider | Local files only |
| **Relevance scoring** | KG path scoring + graph edge weights | None (LLM sees all files) |
| **Sandboxing** | Firecracker VMs | Deno interpreter |
| **Budget control** | Token + time + recursion | max_iterations only |
| **Snapshots** | VM state capture/rollback | None |
| **Learning** | KG curation from RLM interactions | None per-query |
| **MCP integration** | Claude/Cursor tools | None |

---

## 4. Proposed Architecture: `terraphim-grep`

### 4.1 Philosophy

> **Search first, RLM last. Fast deterministic search handles 80-90% of queries. RLM is invoked only for synthesis, evaluation, or concept discovery — and writes back to the KG to improve future searches.**

This is the **inverse of rlmgrep's "RLM first, brute force"** approach.

### 4.2 System Architecture

```text
┌─────────────────────────────────────────────────────────────────────────────┐
│                         terraphim-grep                                       │
│              Intelligent Hybrid Search with RLM Fallback                     │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  User Query: "Where is retry/backoff configured and what are the defaults?" │
│                                                                             │
│                              │                                              │
│                              ▼                                              │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │                    PHASE 1: HYBRID SEARCH                           │   │
│  │           (Deterministic, fast, zero LLM tokens)                   │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                              │                                              │
│        ┌─────────────────────┼─────────────────────┐                       │
│        │                     │                     │                       │
│        ▼                     ▼                     ▼                       │
│  ┌───────────┐        ┌───────────┐        ┌───────────┐                  │
│  │  Code     │        │  Docs     │        │  KG Query │                  │
│  │  Search   │        │  Search   │        │           │                  │
│  │           │        │           │        │           │                  │
│  │fff_search │        │haystack_  │        │RoleGraph  │                  │
│  │+ ripgrep  │        │jmap       │        │Aho-       │                  │
│  │+ KgPath   │        │haystack_  │        │Corasick   │                  │
│  │Scorer     │        │grepapp    │        │+ TF-IDF   │                  │
│  │           │        │           │        │fallback   │                  │
│  └─────┬─────┘        └─────┬─────┘        └─────┬─────┘                  │
│        │                     │                     │                       │
│        └─────────────────────┼─────────────────────┘                       │
│                              │                                              │
│                              ▼                                              │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │                 PHASE 2: RESULT FUSION & RANKING                    │   │
│  │         (Merge + deduplicate + re-rank by KG edge weights)         │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                              │                                              │
│                              ▼                                              │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │                 PHASE 3: SUFFICIENCY JUDGE                          │   │
│  │  "Do these results answer the query?"                               │   │
│  │                                                                     │   │
│  │  Tier 1: Heuristic (free)                                           │   │
│  │    - Coverage: all query terms found?                               │   │
│  │    - Confidence: KG matches exceed threshold?                       │   │
│  │    - Diversity: results from multiple haystacks?                    │   │
│  │                                                                     │   │
│  │  Tier 2: LLM Judge ($0.001, 10% of queries)                         │   │
│  │    - "Given these results, can we answer the query?"                │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                              │                                              │
│              ┌───────────────┼───────────────┐                             │
│              │               │               │                             │
│              ▼               ▼               ▼                             │
│        ┌─────────┐    ┌──────────┐    ┌──────────┐                        │
│        │Sufficient│    │NeedsSynth│    │NeedsMore │                        │
│        │          │    │          │    │          │                        │
│        │Return    │    │RLM w/    │    │Expand    │                        │
│        │results   │    │context   │    │search    │                        │
│        └──────────┘    └────┬─────┘    └──────────┘                        │
│                             │                                             │
│                             ▼                                             │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │                 PHASE 4: RLM SYNTHESIS (if needed)                  │   │
│  │                                                                     │   │
│  │  Input (NOT full corpus!):                                          │   │
│  │    - Top-K retrieved chunks (relevant passages only)               │   │
│  │    - KG concept map (relevant nodes/edges)                         │   │
│  │    - Source metadata (for citations)                               │   │
│  │    - Original query                                                │   │
│  │                                                                     │   │
│  │  Output:                                                            │   │
│  │    - Synthesised answer                                             │   │
│  │    - Citations with file:line references                            │   │
│  │    - Confidence score                                               │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                             │                                             │
│                             ▼                                             │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │                 PHASE 5: KG CURATION                                │   │
│  │                                                                     │   │
│  │  RLM extracts new concepts from query + answer:                     │   │
│  │    - New concept: "retry configuration"                             │   │
│  │    - Synonyms: "backoff", "retry policy", "exponential backoff"     │   │
│  │    - Relationships: "retry" → "tokio::time" → "Duration"            │   │
│  │                                                                     │   │
│  │  Writes to RoleGraph → rebuilds automata                            │   │
│  │  Future queries for "retry" are now faster                          │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

### 4.3 Component Specifications

#### 4.3.1 Hybrid Search Orchestrator

```rust
pub struct HybridSearcher {
    code_search: Arc<dyn HaystackProvider>,      // fff + ripgrep + KgPathScorer
    doc_search: Arc<dyn HaystackProvider>,       // JMAP, grep.app, etc.
    kg_query: Arc<RoleGraphSync>,                // RoleGraph
}

impl HybridSearcher {
    pub async fn search(&self, query: &SearchQuery) -> HybridResults {
        // Parallel search across all haystacks
        let (code, docs, kg) = tokio::join!(
            self.code_search.search(query),
            self.doc_search.search(query),
            self.kg_query.search(query),
        );
        
        // Fuse: merge, deduplicate, re-rank by KG edge weights
        self.fuse_and_rank(code, docs, kg)
    }
}
```

**Key difference from rlmgrep:**
- rlmgrep: O(corpus_size) tokens — loads ALL files into LLM
- HybridSearcher: O(K) tokens — retrieves only top-K relevant chunks

#### 4.3.2 Sufficiency Judge

```rust
pub enum Sufficiency {
    Sufficient,           // Return search results directly
    NeedsSynthesis,       // RLM synthesis with retrieved context
    NeedsExpansion,       // Expand search (broader terms, more haystacks)
    Insufficient,         // Minimal context — RLM cold start
}

pub struct SufficiencyJudge {
    heuristic: HeuristicJudge,   // Fast: coverage, confidence, diversity
    llm: Option<LlmJudge>,       // Slow: LLM evaluation (only when uncertain)
}
```

**Cost optimisation:** Tier 1 is free. Tier 2 costs ~$0.001 but only runs on 10-20% of queries.

#### 4.3.3 Contextual RLM

When search is insufficient, RLM receives pre-filtered context:

```rust
pub struct RlmContext {
    pub query: String,
    pub retrieved_chunks: Vec<RetrievedChunk>,
    pub kg_concepts: Vec<KgConcept>,
    pub source_map: HashMap<String, DocumentMetadata>,
}

pub struct RetrievedChunk {
    pub content: String,
    pub source: String,       // file path or doc ID
    pub line_start: Option<usize>,
    pub line_end: Option<usize>,
    pub relevance_score: f64,
    pub haystack: String,     // "code", "docs", "external"
}
```

**Opposite of rlmgrep:** Instead of "here are all files, find the answer", it's "here are the most relevant passages, synthesise the answer".

#### 4.3.4 Structured Output (rlmgrep Parity + Enhancement)

```rust
pub trait RlmSignature: Send + Sync {
    type Output: serde::Serialize + serde::de::DeserializeOwned;
    fn instructions(&self) -> String;
    fn parse(&self, raw: &str) -> Result<Self::Output, SignatureError>;
}

// Search result signature (rlmgrep parity)
pub struct SearchResultSignature;
impl RlmSignature for SearchResultSignature {
    type Output = Vec<Match>;  // { path, line, context }
}

// Answer with citations
pub struct AnswerSignature;
impl RlmSignature for AnswerSignature {
    type Output = AnswerWithCitations;
    // { answer: String, citations: Vec<Citation>, confidence: f64 }
}

// Concept extraction (KG curation)
pub struct ConceptExtractionSignature;
impl RlmSignature for ConceptExtractionSignature {
    type Output = Vec<NewConcept>;
}
```

#### 4.3.5 KG Curation Agent

```rust
pub struct KgCurationRlm {
    rlm: Arc<TerraphimRlm>,
    rolegraph: Arc<Mutex<RoleGraph>>,
}

impl KgCurationRlm {
    pub async fn extract_and_index(
        &self,
        query: &str,
        rlm_answer: &str,
    ) -> Result<Vec<NewConcept>> {
        let prompt = format!(
            "Extract new concepts for the knowledge graph:\n\
             Query: {}\n\
             Answer: {}\n\
             Extract: concept name, synonyms, relationships.",
            query, rlm_answer
        );
        
        let concepts = self.rlm
            .query_with_signature::<ConceptExtractionSignature>(&prompt)
            .await?;
        
        // Write to RoleGraph and rebuild automata
        let mut graph = self.rolegraph.lock().await;
        for concept in &concepts {
            graph.add_concept(concept)?;
        }
        graph.rebuild_automata()?;
        
        Ok(concepts)
    }
}
```

**Virtuous cycle:**
1. Query → search finds results via existing KG
2. Insufficient → RLM synthesises answer
3. RLM extracts new concepts → writes to KG
4. Future queries benefit from enriched KG
5. Over time, RLM is needed less and less

---

## 5. CLI Design

### 5.1 Interface (rlmgrep-compatible + Terraphim extensions)

```bash
# Basic search (hybrid search only, no LLM)
terraphim-grep "Where is retry/backoff configured?" .

# With narrative answer (RLM synthesis if needed)
terraphim-grep --answer "What does this repo do and where are the entry points?" .

# Custom structured output (rlmgrep parity)
terraphim-grep --signature-json 'summary: str, findings: list[dict[str,str]]' \
    "Audit auth and summarize issues" .

# Context lines (rlmgrep parity)
terraphim-grep -C 2 "Where is retry/backoff configured?" .

# Restrict to code only
terraphim-grep --haystack code "How do we parse JWTs?" .

# Include documentation
terraphim-grep --haystack code,docs "What is the architecture?" .

# Force RLM (skip search)
terraphim-grep --rlm "Explain the build system" .

# Verbose (show search + RLM iterations)
terraphim-grep -v "Find race conditions" .
```

### 5.2 Output Format

**Search-only mode (sufficient results):**
```
./crates/terraphim_rlm/src/retry.rs
42:    pub fn with_backoff(self, backoff: ExponentialBackoff) -> Self {
43:        self.backoff = Some(backoff);
44:        self

./crates/terraphim_rlm/src/config.rs
89:    pub retry_policy: RetryPolicy,
90:    pub max_retries: u32,
```

**RLM synthesis mode (insufficient search):**
```
===== Answer =====
Retry/backoff is configured in two places:

1. crates/terraphim_rlm/src/retry.rs:42 — ExponentialBackoff is set via 
   `with_backoff()` builder method. Defaults to 3 retries with 100ms base delay.

2. crates/terraphim_rlm/src/config.rs:89 — RetryPolicy enum defines 
   Fixed, Exponential, and Custom strategies.

===== Matches =====
./crates/terraphim_rlm/src/retry.rs
42:    pub fn with_backoff(self, backoff: ExponentialBackoff) -> Self {
...

./crates/terraphim_rlm/src/config.rs
89:    pub retry_policy: RetryPolicy,
...
```

---

## 6. Implementation Roadmap

### Phase 1: LLM Bridge (2 days)
**Critical blocker.** Replace stub with real LLM clients:

```rust
#[async_trait]
pub trait LlmClient: Send + Sync {
    async fn complete(&self, request: CompletionRequest) 
        -> Result<CompletionResponse, LlmError>;
}

pub struct OpenAiClient;
pub struct AnthropicClient;
pub struct OllamaClient;  // Local, zero cost
```

### Phase 2: Hybrid Search (2 days)
Create `HybridSearcher` that parallelises code + docs + KG search.

### Phase 3: Sufficiency Judge (1 day)
Two-tier judge: heuristic (free) + LLM (uncertain cases only).

### Phase 4: Contextual RLM (2 days)
Build RLM context from retrieved chunks + KG concepts. Context window management.

### Phase 5: Structured Signatures (1 day)
`RlmSignature` trait + implementations for Match, Answer, Concept extraction.

### Phase 6: KG Curation (2 days)
RLM extracts concepts from interactions, writes to RoleGraph, rebuilds automata.

### Phase 7: CLI + Integration (2 days)
`terraphim-grep` binary with rlmgrep-compatible interface.

**Total: 12 days**

---

## 7. Cost Analysis

| Phase | KG Lookups | LLM Calls | Cost | Latency | Frequency |
|-------|-----------|-----------|------|---------|-----------|
| **Search-only** | 2-4 | 0 | $0.0001 | 0.1s | 80% |
| **Search expansion** | 4-8 | 0 | $0.0002 | 0.5s | 15% |
| **RLM synthesis** | 4-8 | 1 | $0.005 | 5s | 4% |
| **KG curation** | 4-8 | 2 | $0.01 | 10s | 1% |
| **Average** | — | — | **$0.001** | **0.5s** | — |

**vs rlmgrep:**

| Metric | rlmgrep | terraphim-grep |
|--------|---------|----------------|
| Cost/query | $0.01-0.05 | $0.001 (20-50x cheaper) |
| Latency | 15-30s | 0.1-5s (3-300x faster) |
| Corpus size | Limited by context window | Unlimited (search index) |
| Learning | None per-query | KG enriches over time |
| Multi-haystack | Local files only | Code + docs + external |

---

## 8. Comparison Matrix

| Dimension | rlmgrep | terraphim-grep (proposed) |
|-----------|---------|---------------------------|
| **Search strategy** | RLM brute-force | Hybrid search first, RLM fallback |
| **Token complexity** | O(corpus_size) | O(retrieved_chunks) |
| **Indexing** | None | RoleGraph + file frecency |
| **Multi-modal** | MarkItDown PDF/office/image/audio | Port MarkItDown + sidecar cache |
| **Sandbox** | Deno interpreter | Firecracker VM / Docker |
| **Structured output** | DSPy Signatures | `RlmSignature` trait |
| **KG integration** | None | Bidirectional: read + write |
| **Learning** | None | Concepts extracted and indexed |
| **Budget control** | max_iterations | Token + time + recursion |
| **Citation** | Manual verification | Automatic source attribution |
| **MCP tools** | None | Full MCP server integration |

---

## 9. Conclusion

rlmgrep proves that LLM-powered code search is valuable but implements it inefficiently. Terraphim has all the pieces for a superior architecture:

- **Fast search**: `fff_search`, `ripgrep`, `RoleGraph` Aho-Corasick
- **Relevance ranking**: `KgPathScorer`, graph edge weights
- **Sandboxed execution**: `terraphim_rlm` with Firecracker/Docker
- **Structured APIs**: MCP server, typed search results
- **Extensible KG**: `RoleGraph` with hot-reload thesaurus

The missing pieces are:
1. Real LLM bridge (replace stub)
2. Hybrid search orchestrator
3. Sufficiency judge
4. Structured RLM signatures
5. KG curation feedback loop

These can be built incrementally on top of existing crates, creating `terraphim-grep` that is faster, cheaper, and more accurate than rlmgrep while continuously improving its knowledge graph.

**The virtuous cycle:**
- Day 1: 80% search-only, 20% RLM fallback
- Day 30: 95% search-only, 5% RLM fallback
- Day 90: 99% search-only, 1% RLM (novel queries only)

Over time, the system becomes **cheaper and faster** — the opposite of rlmpgrep, which costs the same for every query.

---

## References

- https://github.com/halfprice06/rlmgrep — rlmgrep repository
- `crates/terraphim_rlm/src/llm_bridge.rs` — Stubbed LLM bridge (line 192)
- `crates/terraphim_rolegraph/src/lib.rs` — RoleGraph implementation
- `crates/haystack_core/src/lib.rs` — HaystackProvider trait
- `crates/terraphim_file_search/src/kg_scorer.rs` — KgPathScorer
- `crates/terraphim_mcp_server/src/lib.rs` — MCP server with search tools
- `.docs/ARCHITECTURE-build-runner-llm.md` — Existing KG-first architecture
