# Research: rlmgrep vs terraphim_rlm — Evolving build-runner-llm with Hybrid Search + RLM

**Date:** 2026-05-18  
**Context:** Existing build-runner-llm is deployed on bigbox (Epic #1423). This research compares rlmgrep's approach with Terraphim's infrastructure and proposes specific improvements to evolve the current bash-script implementation into a hybrid search-first system with RLM fallback and KG curation.

---

## 1. Executive Summary

The existing `build-runner-llm` (`scripts/build-runner-llm.sh`) successfully implements a **KG-first, deterministic build runner** that:
- Auto-detects CI config (GitHub Actions, BUILD.md, Cargo.toml, Makefile, Earthfile, package.json)
- Transforms commands via `terraphim-agent replace --role DevOpsRunner` (cargo → rch, npm → bun)
- Validates against whitelist, tracks costs, posts Gitea status
- Captures learnings on failure

**Cost:** $0.0001/build (zero LLM calls on hot path). **Latency:** 0.1s for transformation.

However, it has a critical limitation: **it is purely deterministic**. When the KG misses a pattern, it falls back to hardcoded commands — not to intelligent reasoning. It does not leverage:
- Hybrid search over code/documentation/external haystacks
- `terraphim_rlm` crate for complex reasoning
- RLM as a KG curator (writing new concepts back)
- Structured output signatures (like rlmgrep's `--signature-json`)

This document analyses rlmgrep's architecture, maps it against Terraphim's existing infrastructure, and proposes a **v5 architecture** that evolves build-runner-llm from a bash script into a Rust-based hybrid search + RLM system.

---

## 2. rlmgrep Deep Analysis

### 2.1 What rlmgrep Does

rlmgrep is a Python CLI that uses DSPy RLM to search codebases with natural language:

```text
User Query: "Where is retry/backoff configured?"
    |
    v
[File Discovery]  →  collect_candidates()  →  ripgrep-style walking
    |
    v
[Ingestion]  →  load_files()  →  Convert PDF/office/image/audio to text
    |
    v
[Context Building]  →  directory: {path: full_text}  +  ASCII file_map
    |
    v
[DSPy RLM]  →  dspy.RLM(signature, interpreter=Deno/Python)
    |            Iterates: read → reason → output matches
    v
[Output]  →  Vec<Match{path, line}>  +  optional narrative answer
```

### 2.2 rlmgrep's Critical Flaw: Brute-Force Context Loading

rlmgrep loads **entire file contents** into the LLM context:

```python
# rlmgrep/rlm.py
directory = {k: v.text for k, v in files.items()}  # ALL files, ALL text
file_map = build_file_map(sorted(files.keys()))     # ASCII tree

# Sent to LLM:
# directory: dict = dspy.InputField(desc="Mapping from relative path to full file text")
# file_map: str = dspy.InputField(desc="ASCII tree of directory keys")
```

**Why this fails at scale:**
- 5,000 files × 100 lines = 500K lines in context
- GPT-4o context: 128K tokens
- Math: **does not work**. rlmgrep aborts at 5,000 files.
- Cost: **$0.01-0.05 per query** (expensive for CI)
- No learning between queries — each query is independent

### 2.3 What rlmgrep Gets Right

| Feature | Implementation | Value |
|---------|---------------|-------|
| **Structured signatures** | DSPy `Signature` with typed output fields (`Match`, `answer`) | Agents consume typed output |
| **Custom signatures** | `--signature-json 'summary: str, findings: list[dict[str,str]]'` | Flexible structured output |
| **Multi-modal ingestion** | MarkItDown for PDF/office/image/audio | Search any file type |
| **Sidecar caching** | `.filename.pdf.md` cached conversions | Avoid re-conversion |
| **Sub-model recursion** | Cheap `sub_lm` for recursive calls | Cost optimisation |
| **Grep-compatible output** | rg-style headings with line numbers | Human + machine readable |

---

## 3. Current terraphim_rlm State

### 3.1 Architecture (Mature Infrastructure, Stubbed LLM)

```text
TerraphimRlm
├── SessionManager          ✅ Mature (create/destroy/extend, context vars)
├── QueryLoop               ✅ Mature (iteration/recursion budgets, cancellation)
├── Executor                ✅ Mature (Firecracker/Docker/local, code + bash)
├── BudgetTracker           ✅ Mature (token + time + recursion budgets)
├── LlmBridge               ❌ STUB (returns "[LLM Bridge stub] Query: ...")
└── CommandParser           ✅ Mature (FINAL, RUN, CODE, SNAPSHOT, ROLLBACK, QUERY_LLM)
```

### 3.2 The LLM Bridge is the Critical Blocker

`crates/terraphim_rlm/src/llm_bridge.rs` line 192-201:

```rust
// TODO: Actually call the LLM service
// For now, return a stub response
let response_text = format!("[LLM Bridge stub] Query: {}...", ...);
```

**Without a real LLM client, terraphim_rlm cannot participate in build-runner-llm.**

### 3.3 What terraphim_rlm Has That rlmgrep Lacks

| Feature | terraphim_rlm | rlmgrep |
|---------|--------------|---------|
| **True sandboxing** | Firecracker VMs (hardware isolation) | Deno interpreter |
| **Budget discipline** | Token + time + recursion (hard limits) | max_iterations only |
| **Snapshots** | VM state capture/rollback | None |
| **Cancellation** | Async cancellation channels | None |
| **Structured concurrency** | tokio::spawn with scoped tasks | Synchronous |
| **MCP integration** | Exposed as tools to Claude/Cursor | None |

---

## 4. Current build-runner-llm Assessment

### 4.1 What Works Today

```text
Git Push → Webhook → ADF Orchestrator → build-runner-llm.sh
                                              │
                                              ├──► detect_and_extract()
                                              │      ├──► GitHub Actions (yq parse)
                                              │      ├──► BUILD.md (bash code blocks)
                                              │      ├──► Cargo.toml (default Rust)
                                              │      ├──► Makefile (make)
                                              │      ├──► Earthfile (RUN lines)
                                              │      └──► package.json (bun defaults)
                                              │
                                              ├──► transform_command()
                                              │      └──► terraphim-agent replace --role DevOpsRunner
                                              │            ├──► cargo build → rch exec -- cargo build
                                              │            ├──► cargo test → rch exec -- cargo test
                                              │            ├──► npm install → bun install
                                              │            └──► (etc.)
                                              │
                                              ├──► validate_command() (whitelist)
                                              ├──► execute_command()
                                              ├──► POST_STATUS (Gitea API)
                                              └──► send_cost_metrics (Quickwit)
```

### 4.2 What's Missing

| Gap | Impact | Proposed Solution |
|-----|--------|-------------------|
| **No hybrid search** | Cannot search code/docs/external to resolve "why did this fail?" | Integrate `HybridSearcher` |
| **No RLM fallback** | KG miss → hardcoded fallback, not intelligent reasoning | `ContextualRlm` with retrieved context |
| **No structured output** | Output is plain text, not agent-consumable | `RlmSignature` trait |
| **No KG curation** | Failed builds don't enrich the KG | `KgCurationRlm` extracts concepts |
| **No sufficiency judge** | Cannot decide "is this answer good enough?" | Two-tier `SufficiencyJudge` |
| **LLM bridge stub** | terraphim_rlm cannot call LLMs | Implement real LLM clients |
| **No code haystack** | Cannot search project source for failure analysis | Integrate `terraphim_file_search` |
| **No doc haystack** | Cannot query BUILD.md, AGENTS.md, README | Integrate `haystack_grepapp` |

---

## 5. Proposed v5 Architecture: Hybrid Search + RLM

### 5.1 Core Philosophy

> **Search first, RLM last. Fast deterministic search (Aho-Corasick, ripgrep, KG graph queries) handles 80-90% of queries. RLM is invoked only for synthesis, evaluation, or concept discovery — and writes back to the KG to improve future searches.**

This is the **inverse of rlmgrep's "RLM first, brute force"** approach and an **evolution of build-runner-llm's "KG only"** approach.

### 5.2 Architecture Diagram

```text
┌─────────────────────────────────────────────────────────────────────────────┐
│                         BUILD-RUNNER-LLM v5                                 │
│                    (Rust binary, replaces bash script)                       │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  ┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐         │
│  │   Detection     │───►│  Hybrid Search  │───►│  Sufficiency    │         │
│  │   Phase         │    │  Phase          │    │  Judge          │         │
│  │                 │    │                 │    │                 │         │
│  │ • GitHub Actions│    │ • Code search   │    │ • Heuristic     │         │
│  │ • BUILD.md      │    │   (fff + ripgrep│    │   (coverage,    │         │
│  │ • Cargo.toml    │    │    + KgPathScorer│   │    confidence)  │         │
│  │ • Makefile      │    │ • Doc search    │    │ • LLM Judge     │         │
│  │ • Earthfile     │    │   (JMAP,        │    │   (evaluation)  │         │
│  │ • package.json  │    │    grep.app)    │    │                 │         │
│  │                 │    │ • KG query      │    │                 │         │
│  │                 │    │   (RoleGraph)   │    │                 │         │
│  │                 │    │ • External      │    │                 │         │
│  │                 │    │   (haystacks)   │    │                 │         │
│  └─────────────────┘    └─────────────────┘    └────────┬────────┘         │
│                                                         │                  │
│                              ┌──────────────────────────┼──────────┐       │
│                              │                          │          │       │
│                              ▼                          ▼          ▼       │
│                        ┌─────────┐               ┌─────────────┐  ┌─────┐  │
│                        │Sufficient│               │NeedsSynthesis│  │Fail │  │
│                        │         │               │              │  │     │  │
│                        │ Return  │               │ RLM Fallback │  │Error│  │
│                        │ results │               │              │  │     │  │
│                        └─────────┘               └──────┬───────┘  └─────┘  │
│                                                         │                  │
│                              ┌──────────────────────────┘                  │
│                              ▼                                             │
│                        ┌─────────────────┐                                 │
│                        │  ContextualRlm  │                                 │
│                        │                 │                                 │
│                        │ • Retrieved     │                                 │
│                        │   chunks        │                                 │
│                        │ • KG concepts   │                                 │
│                        │ • Source map    │                                 │
│                        │ • Query         │                                 │
│                        └────────┬────────┘                                 │
│                                 │                                          │
│                                 ▼                                          │
│                        ┌─────────────────┐                                 │
│                        │  Evaluation     │                                 │
│                        │  (Judge)        │                                 │
│                        │                 │                                 │
│                        │ Compare RLM     │                                 │
│                        │ output vs       │                                 │
│                        │ search results  │                                 │
│                        └────────┬────────┘                                 │
│                                 │                                          │
│                                 ▼                                          │
│                        ┌─────────────────┐                                 │
│                        │  KG Curation    │                                 │
│                        │  (RLM Role)     │                                 │
│                        │                 │                                 │
│                        │ Extract new     │                                 │
│                        │ concepts from   │                                 │
│                        │ query+answer    │                                 │
│                        │ → Write to      │                                 │
│                        │   RoleGraph     │                                 │
│                        └─────────────────┘                                 │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

### 5.3 Component Specifications

#### 5.3.1 Hybrid Search Orchestrator (`HybridSearcher`)

**Purpose:** Parallel search across all haystacks, retrieve only top-K relevant chunks.

```rust
pub struct HybridSearcher {
    code_search: Arc<dyn HaystackProvider>,      // fff_search + ripgrep + KgPathScorer
    doc_search: Arc<dyn HaystackProvider>,       // haystack_jmap, haystack_grepapp
    kg_query: Arc<RoleGraphSync>,                // RoleGraph Aho-Corasick + TF-IDF fallback
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
- rlmgrep: Loads ALL files into LLM context (O(corpus_size))
- HybridSearcher: Retrieves top-K chunks from indexed haystacks (O(K))

#### 5.3.2 Sufficiency Judge (`SufficiencyJudge`)

**Purpose:** Decide if search results are sufficient or if RLM fallback is needed.

```rust
pub enum Sufficiency {
    Sufficient,           // Return search results directly
    NeedsSynthesis,       // RLM synthesis with retrieved context
    NeedsExpansion,       // Expand search (broader terms, more haystacks)
    Insufficient,         // Minimal context — RLM cold start
}

pub struct SufficiencyJudge {
    heuristic: HeuristicJudge,   // Fast: coverage, confidence, diversity, recency
    llm: Option<LlmJudge>,       // Slow: LLM evaluates "can we answer from context?"
}

impl SufficiencyJudge {
    pub fn evaluate(&self, query: &str, results: &HybridResults) -> Sufficiency {
        // Tier 1: Heuristic (no LLM tokens)
        let heuristic_score = self.heuristic.score(query, results);
        match heuristic_score {
            Score::High => return Sufficiency::Sufficient,
            Score::Low => return Sufficiency::Insufficient,
            Score::Medium => {
                // Tier 2: LLM Judge (only when uncertain)
                self.llm.as_ref()
                    .map(|j| j.evaluate(query, results))
                    .unwrap_or(Sufficiency::NeedsSynthesis)
            }
        }
    }
}
```

**Cost optimisation:** Tier 1 is free (no LLM). Tier 2 costs ~$0.001 but only runs on 10-20% of queries.

#### 5.3.3 Contextual RLM (`ContextualRlm`)

**Purpose:** RLM fallback with pre-retrieved context (not full corpus).

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

When RLM is invoked, it receives:
1. **Retrieved chunks** (top-K passages, truncated to fit context window)
2. **KG concept map** (relevant nodes/edges with ranks)
3. **Source metadata** (for citation tracking)
4. **Original query**

This is the **opposite of rlmgrep**: instead of "here are all files, find the answer", it's "here are the most relevant passages, synthesise the answer".

#### 5.3.4 Structured Output Signatures (`RlmSignature`)

**Purpose:** Typed RLM outputs, ported from rlmgrep's DSPy signatures.

```rust
pub trait RlmSignature: Send + Sync {
    type Output: serde::Serialize + serde::de::DeserializeOwned;
    fn instructions(&self) -> String;
    fn parse(&self, raw: &str) -> Result<Self::Output, SignatureError>;
}

// Search result signature (rlmgrep parity)
pub struct SearchResultSignature;
impl RlmSignature for SearchResultSignature {
    type Output = Vec<Match>;
    // ...
}

// Concept extraction signature (KG curation)
pub struct ConceptExtractionSignature;
impl RlmSignature for ConceptExtractionSignature {
    type Output = Vec<NewConcept>;
    // ...
}

// Answer with citations
pub struct AnswerSignature;
impl RlmSignature for AnswerSignature {
    type Output = AnswerWithCitations;
    // ...
}
```

#### 5.3.5 KG Curation Agent (`KgCurationRlm`)

**Purpose:** RLM extracts new concepts from interactions and writes them to the RoleGraph.

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
            "Given this query and answer, extract new concepts for the knowledge graph:\n\
             Query: {}\n\
             Answer: {}\n\
             Extract: concept name, synonyms, relationships.",
            query, rlm_answer
        );
        
        let concepts: Vec<NewConcept> = self
            .rlm
            .query_with_signature::<ConceptExtractionSignature>(&prompt)
            .await?;
        
        // Write to RoleGraph
        let mut graph = self.rolegraph.lock().await;
        for concept in &concepts {
            graph.add_concept(concept)?;
        }
        
        // Rebuild automata so new concepts are immediately searchable
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

#### 5.3.6 Real LLM Bridge Implementation

**The critical missing piece.** Replace the stub with real LLM clients:

```rust
#[async_trait]
pub trait LlmClient: Send + Sync {
    async fn complete(&self, request: CompletionRequest) 
        -> Result<CompletionResponse, LlmError>;
}

pub struct OpenAiClient { /* ... */ }
pub struct AnthropicClient { /* ... */ }
pub struct OllamaClient { /* ... */ }  // Local, zero cost

pub struct LlmBridge {
    config: LlmBridgeConfig,
    session_manager: Arc<SessionManager>,
    client: Arc<dyn LlmClient>,  // NEW: real client
    budget_trackers: DashMap<SessionId, Arc<BudgetTracker>>,
}
```

**Cost control:** Use local models (Ollama) by default for RLM; cloud models only for complex reasoning.

---

## 6. Integration with Existing build-runner-llm

### 6.1 Evolution Path (Not Replacement)

```
Current: build-runner-llm.sh (bash, terraphim-agent CLI)
    │
    ▼
Phase 1: Hybrid search integration
    │        • Add code/doc search to detect phase
    │        • Search BUILD.md, AGENTS.md, README for context
    │
    ▼
Phase 2: RLM fallback for unknown failures
    │        • When command fails and KG has no answer,
    │          search code/docs and ask RLM "why did this fail?"
    │
    ▼
Phase 3: KG curation
    │        • RLM extracts concepts from failure analysis
    │        • Writes to DevOpsRunner role KG
    │
    ▼
Phase 4: Rust binary replacement
             • Replace bash script with Rust binary
             • Integrate terraphim_rlm crate directly
             • Structured output via RlmSignature
```

### 6.2 Concrete Use Cases

#### Use Case 1: Build Failure Analysis (Currently: Manual)

**Current state:**
```bash
$ cargo test --workspace
   Compiling terraphim_rlm v0.1.0
error[E0433]: failed to resolve: use of undeclared crate `tokio`
  --> crates/terraphim_rlm/src/lib.rs:42:5
# build-runner-llm: FAIL, posts Gitea status, captures learning
```

**With hybrid search + RLM:**
```
1. Command fails: cargo test --workspace
2. Hybrid search: "tokio undeclared crate terraphim_rlm"
   → Code haystack: finds Cargo.toml (no tokio in dependencies)
   → Doc haystack: finds BUILD.md (mentions feature flags)
   → KG query: finds "tokio" concept in RoleGraph
3. RLM context:
   - Retrieved: Cargo.toml dependencies section
   - Retrieved: BUILD.md feature flag documentation
   - Retrieved: RoleGraph "tokio" node connections
4. RLM synthesis:
   "Error: tokio is not in Cargo.toml dependencies for terraphim_rlm.
    Fix: Add tokio = { version = "1", features = ["full"] } to [dependencies].
    Or: Enable the "async" feature flag mentioned in BUILD.md."
5. KG curation:
   - New concept: "missing dependency error"
   - New edge: "tokio" → "feature flags"
   - Future builds: KG recognises pattern, suggests fix without RLM
```

#### Use Case 2: New Project Type Detection (Currently: Hardcoded)

**Current state:**
```bash
# build-runner-llm.sh detects project type by file existence:
if [ -f "Cargo.toml" ]; then echo "cargo fmt; cargo clippy; cargo build; cargo test"
elif [ -f "package.json" ]; then echo "bun install; bun run build; bun test"
# ... etc
```

**With hybrid search + RLM:**
```
1. New project: terraphim-ai-python (PyTorch bindings)
2. Detection: No Cargo.toml, no package.json, but has pyproject.toml
3. Hybrid search over code:
   → "pyproject.toml" found
   → "[build-system]" section found
   → "maturin" build tool mentioned
4. RLM fallback:
   "This is a Python project using Maturin for Rust bindings.
    Build steps: maturin develop, pytest, mypy"
5. KG curation:
   - New concept: "maturin" → synonyms: pyo3, rust-python
   - New concept: "pyproject.toml" → build tool
   - Future: Any project with pyproject.toml gets correct build steps
```

#### Use Case 3: Structured Build Report (Currently: Plain Text)

**Current state:**
```bash
[INFO] Build completed successfully
[INFO] Cost report: $0.0001 total (KG: 4 lookups, LLM: 0 calls)
```

**With structured signatures:**
```json
{
  "build_id": "abc123",
  "status": "success",
  "steps": [
    { "name": "format", "command": "cargo fmt", "duration_ms": 2000, "status": "pass" },
    { "name": "lint", "command": "rch exec -- cargo clippy", "duration_ms": 45000, "status": "pass" },
    { "name": "compile", "command": "rch exec -- cargo build", "duration_ms": 120000, "status": "pass" },
    { "name": "test", "command": "rch exec -- cargo test", "duration_ms": 300000, "status": "pass" }
  ],
  "cost": { "kg_lookups": 4, "llm_calls": 0, "total_cents": 0.0001 },
  "concepts_learned": 0
}
```

---

## 7. Implementation Roadmap

### Phase 1: LLM Bridge Implementation (2 days)

**Goal:** Make terraphim_rlm actually call LLMs.

1. **Implement `LlmClient` trait**
   - `OpenAiClient`: OpenAI API (GPT-4o, GPT-5)
   - `AnthropicClient`: Anthropic API (Claude 3.5/4)
   - `OllamaClient`: Local models (zero cost)

2. **Update `LlmBridge`**
   - Replace stub with real client calls
   - Add retry/backoff logic
   - Add structured output mode (JSON schema)

3. **Configuration**
   - `RlmConfig` adds `llm_provider`, `api_key`, `model`
   - Support `OLLAMA_URL` for local deployment

### Phase 2: Hybrid Search Integration (2 days)

**Goal:** Enable search across code, docs, and KG.

1. **Create `HybridSearcher`**
   - Parallel search across haystacks
   - Result fusion and KG re-ranking

2. **Integrate with build-runner**
   - Search BUILD.md, AGENTS.md, README for build context
   - Search code for failure patterns
   - Query DevOpsRunner role KG

3. **Add to MCP server**
   - New tool: `hybrid_search(query, haystacks)`

### Phase 3: Sufficiency Judge + Contextual RLM (2 days)

**Goal:** Decide when to invoke RLM and what context to provide.

1. **Implement `SufficiencyJudge`**
   - Heuristic tier: coverage, confidence, diversity
   - LLM tier: evaluation prompt

2. **Implement `ContextualRlm`**
   - Build RLM context from retrieved chunks
   - Context window management (truncate low-relevance)

3. **Integration test**
   - Query with sufficient KG results → no LLM call
   - Query with insufficient results → RLM invoked with context

### Phase 4: Structured Signatures (1 day)

**Goal:** Typed RLM outputs (rlmgrep parity).

1. **Define `RlmSignature` trait**
2. **Implement signatures:**
   - `SearchResultSignature` → `Vec<Match>`
   - `AnswerSignature` → `AnswerWithCitations`
   - `ConceptExtractionSignature` → `Vec<NewConcept>`

### Phase 5: KG Curation (2 days)

**Goal:** RLM writes new concepts back to KG.

1. **Implement `KgCurationRlm`**
   - Extract concepts from RLM interactions
   - Write to RoleGraph with proper edges
   - Rebuild Aho-Corasick automata

2. **Add to build-runner**
   - On build failure: search → RLM analysis → KG curation
   - On new project detection: RLM extraction → KG population

### Phase 6: Replace Bash Script (2 days)

**Goal:** Deploy Rust-based build-runner-llm.

1. **Create `crates/build_runner_llm`**
   - Rust binary that orchestrates all components
   - Maintains compatibility with existing ADF integration

2. **Update deployment**
   - Replace `scripts/build-runner-llm.sh`
   - Update `terraphim.toml` agent config

**Total: 11 days**

---

## 8. Cost Analysis

| Phase | KG Lookups | LLM Calls | Cost | Latency | Frequency |
|-------|-----------|-----------|------|---------|-----------|
| **Hot path** (KG hit) | 2-4 | 0 | $0.0001 | 0.1s | 80% |
| **Search expansion** (KG miss) | 4-8 | 0 | $0.0002 | 0.5s | 15% |
| **RLM synthesis** (complex) | 4-8 | 1 | $0.005 | 5s | 4% |
| **KG curation** (new concept) | 4-8 | 2 | $0.01 | 10s | 1% |
| **Average over 100 builds** | — | — | **$0.0007** | **0.5s** | — |

**Comparison:**

| Approach | Cost/Build | Latency | Adaptability |
|----------|-----------|---------|--------------|
| Deterministic (old) | $0.00 | 3min | 0% |
| rlmgrep-style | $0.01-0.05 | 15-30s | 100% |
| Current build-runner-llm | $0.0001 | 0.1s | KG-only |
| **Proposed v5** | **$0.0007** | **0.5s** | **100%** |

---

## 9. Comparison Matrix

| Dimension | rlmgrep | Current build-runner-llm | Proposed v5 |
|-----------|---------|-------------------------|-------------|
| **Search strategy** | RLM brute-force | KG deterministic | Hybrid search + RLM fallback |
| **Token complexity** | O(corpus_size) | O(1) | O(retrieved_chunks) |
| **Indexing** | None | RoleGraph (Aho-Corasick) | RoleGraph + file frecency + haystacks |
| **Multi-modal** | MarkItDown PDF/office/image/audio | None | Port MarkItDown + sidecar cache |
| **Sandbox** | Deno interpreter | None (bash) | Firecracker VM / Docker |
| **Structured output** | DSPy Signatures | None | `RlmSignature` trait |
| **KG integration** | None | Read-only (DevOpsRunner) | Bidirectional: read + write |
| **Learning** | None per-query | `terraphim-agent learn` | Concepts extracted and indexed |
| **Budget control** | max_iterations | Cost thresholds | Token + time + recursion |
| **Citation** | Manual verification | None | Automatic source attribution |
| **MCP tools** | None | `terraphim-agent` CLI | Full MCP server integration |
| **Build focus** | Code search | Build execution | Build execution + failure analysis |

---

## 10. Conclusion

The existing `build-runner-llm` proves that **KG-first deterministic execution is viable and cheap**. rlmgrep proves that **LLM-powered search is powerful but expensive**. The opportunity is to combine both:

1. **Keep the KG-first hot path** — it's $0.0001/build and 0.1s
2. **Add hybrid search** — when KG misses, search code/docs/external before invoking RLM
3. **Add RLM fallback** — for complex reasoning, not brute-force search
4. **Add KG curation** — RLM writes new concepts back, making KG smarter over time
5. **Implement the LLM bridge** — terraphim_rlm's stub is the critical blocker

This creates a **virtuous cycle** where the system gets cheaper and faster over time:
- Day 1: 80% KG hits, 20% RLM fallback
- Day 30: 95% KG hits, 5% RLM fallback
- Day 90: 99% KG hits, 1% RLM fallback (new project types only)

The build-runner-llm evolves from a **script that runs builds** to an **intelligent system that understands builds**.

---

## References

- `scripts/build-runner-llm.sh` — Current implementation
- `.docs/ARCHITECTURE-build-runner-llm.md` — Architecture overview
- `.docs/design-build-runner-llm-v4-leverage-existing.md` — KG-first design
- `.docs/design-build-runner-llm-v4.1-devops-runner-role.md` — DevOpsRunner role
- `crates/terraphim_rlm/src/llm_bridge.rs` — Stubbed LLM bridge (line 192)
- `crates/terraphim_rolegraph/src/lib.rs` — RoleGraph implementation
- `crates/haystack_core/src/lib.rs` — HaystackProvider trait
- `crates/terraphim_file_search/src/kg_scorer.rs` — KgPathScorer
- https://github.com/halfprice06/rlmgrep — rlmgrep repository
