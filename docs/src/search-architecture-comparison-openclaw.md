# Terraphim vs OpenClaw: Search Architecture Comparison

This document compares the search and memory retrieval architectures of Terraphim AI and OpenClaw, two agent runtime systems with fundamentally different design philosophies. The comparison is based on OpenClaw's `docs/concepts/memory.md` (commit `7f7d49ae`) and the Terraphim codebase as of March 2026.

## Executive Summary

Both systems solve the same problem -- retrieving relevant context for AI agents from accumulated knowledge -- but make opposite trade-offs:

- **Terraphim**: Graph-native, offline-first, deterministic. Uses explicit knowledge graph structure (Aho-Corasick + BM25 + symbolic embeddings) with ~15-20 MB RAM, no LLM inference at query time.
- **OpenClaw**: Embedding-native, hybrid retrieval. Uses dense vector embeddings (neural) + BM25 keyword search with configurable embedding providers (local GGUF or cloud APIs).

Neither approach is universally superior. The right choice depends on operational constraints (offline vs online), resource budget (RAM vs GPU/API costs), and the nature of queries (exact-match vs paraphrase-heavy).

## Architecture Comparison

### Search Pipeline

| Stage | Terraphim | OpenClaw |
|-------|-----------|----------|
| **Lexical extraction** | Aho-Corasick automaton (O(n), multi-pattern, exact + synonym matching) | BM25 via SQLite FTS5 (standard inverted index) |
| **Statistical ranking** | BM25 / BM25F / BM25+ / TFIDF / Jaccard / QueryRatio (6 pluggable scorers) | BM25 only (FTS5 rank function) |
| **Graph traversal** | RoleGraph: node.rank + edge.rank + doc.rank (co-occurrence graph) | None |
| **Semantic similarity** | Symbolic embeddings (Jaccard + IS-A path distance, feature-gated) | Dense vector embeddings (cosine similarity via neural models) |
| **Hybrid scoring** | Graph rank (70%) + TF-IDF (30%) implemented | Vector (70%) + BM25 (30%) configurable |
| **String similarity** | Levenshtein, Jaro, Jaro-Winkler (post-hoc re-ranking) | None |
| **Path connectivity** | DFS-based coherence check (are query terms connected?) | None |

### Embedding Approach

| Dimension | Terraphim Symbolic Embeddings | OpenClaw Dense Embeddings |
|-----------|------------------------------|--------------------------|
| **Representation** | Ancestor/descendant sets from IS-A hierarchy | Float vectors (768-1536 dimensions) |
| **Similarity function** | 70% Jaccard (set overlap) + 30% path distance | Cosine similarity |
| **Inference required** | No (computed from explicit graph structure) | Yes (embedding model must encode each chunk) |
| **Model dependency** | None | embeddinggemma-300M GGUF (~0.6 GB) or cloud API |
| **Online updates** | Instant (recompute sets on graph change) | Re-embedding required for changed chunks |
| **Paraphrase handling** | Limited to thesaurus synonyms | Strong (neural models capture semantic similarity) |
| **Explainability** | High (can inspect ancestor/descendant overlap) | Low (opaque vector geometry) |

### Memory and Resource Footprint

| Resource | Terraphim | OpenClaw |
|----------|-----------|----------|
| **RAM (search index)** | ~15-20 MB (graph + automata) | ~0.6 GB (local embedding model) + SQLite index |
| **Disk** | Thesaurus JSON + optional binary cache | SQLite database + embedding cache (50,000 entries default) |
| **GPU** | Not required | Not required (GGUF runs on CPU) but benefits from Metal/CUDA |
| **Network** | Never (fully offline) | Optional (cloud embedding APIs: OpenAI, Gemini, Voyage) |
| **Query latency** | < 10 ms typical | Variable (local embedding: 50-200 ms; cloud: 200-500 ms) |

### Document Processing

| Feature | Terraphim | OpenClaw |
|---------|-----------|----------|
| **Chunking** | Full document indexing (no chunking) | ~400 token chunks with 80-token overlap |
| **Document types** | Any text (Markdown, plaintext, structured) | Markdown files only (`MEMORY.md` + `memory/*.md`) |
| **Field weighting** | BM25F: title (2.0), body (1.0), description (1.5), tags (0.5) | No field weighting (chunks are unstructured) |
| **Multi-source** | 10+ haystack types (local files, Atomic, Google Docs, Jira, Discourse, email, etc.) | Single workspace directory + optional extra paths |
| **Index refresh** | On document insertion (immediate) | File watcher with 1.5s debounce |

### Session and Memory Management

| Feature | Terraphim | OpenClaw |
|---------|-----------|----------|
| **Session search** | `terraphim-agent sessions search` via Tantivy FTS | `memory_search` tool via vector + BM25 hybrid |
| **Session indexing** | Manual batch import (`sessions import`) | Delta-based auto-indexing (100 KB / 50 messages threshold) |
| **Session sources** | Claude Code, Cursor, Aider, OpenCode, Codex | OpenClaw sessions only |
| **Pre-compaction flush** | Not implemented | Automatic "silent agentic turn" before context compaction |
| **Memory tiering** | KG concepts (permanent) + session logs (searchable) + learnings (queryable) | `MEMORY.md` (curated) + `memory/YYYY-MM-DD.md` (daily) + session JSONL |
| **Learning capture** | `terraphim-agent learn hook` (auto-captures failed commands) | No equivalent (manual writes to memory files) |

## What Terraphim Has That OpenClaw Lacks

1. **Knowledge graph structure** -- Explicit nodes, edges, and co-occurrence relationships encode domain knowledge. OpenClaw has no graph; it relies on embedding geometry to approximate semantic relationships.

2. **Six pluggable scorers** -- BM25, BM25F (multi-field), BM25+, TFIDF, Jaccard, QueryRatio. OpenClaw uses only FTS5 BM25.

3. **Role-based graphs** -- Separate knowledge graphs per user persona (engineer, researcher, etc.). OpenClaw has per-agent memory but no role-specific search tuning.

4. **Path connectivity checking** -- Can verify whether matched query terms form a connected subgraph, validating query coherence. No OpenClaw equivalent.

5. **Multi-source haystack federation** -- Searches across 10+ data sources simultaneously (local files, Atomic Server, Google Docs, Jira, Confluence, Discourse, email, etc.). OpenClaw searches only its workspace directory.

6. **Automatic learning capture** -- PostToolUse hook captures failed commands and corrections. OpenClaw relies on the model writing memories explicitly.

7. **Thesaurus-driven synonym expansion** -- Synonyms are pre-loaded into the Aho-Corasick automaton, so "Rust" matches "Rust programming language" without neural inference. OpenClaw relies on embedding similarity for this.

## What OpenClaw Has That Terraphim Lacks

1. **Dense vector embeddings** -- Neural models capture paraphrase similarity ("Mac Studio gateway host" matches "the machine running the gateway"). Terraphim's symbolic embeddings are limited to IS-A hierarchy relationships.

2. **Pre-compaction memory flush** -- Before context window compaction, OpenClaw triggers a silent turn prompting the model to persist durable memories. Terraphim has no equivalent; ADF agents lose unsaved context on session end.

3. **Delta-based auto-indexing** -- Session transcripts are automatically indexed when 100 KB or 50 messages accumulate. Terraphim's `sessions import` is manual.

4. **Chunked indexing** -- ~400 token chunks with 80-token overlap improve precision for long documents. Terraphim indexes whole documents.

5. **Embedding provider abstraction** -- Configurable local (GGUF), OpenAI, Gemini, or Voyage embedding providers with automatic fallback. Terraphim has no embedding provider layer.

6. **QMD experimental backend** -- Local-first search sidecar combining BM25 + vectors + reranking via Bun + node-llama-cpp.

7. **Embedding cache** -- 50,000-entry cache avoids re-embedding unchanged text. Not applicable to Terraphim's non-neural approach.

## Potential Cross-Pollination

### From OpenClaw to Terraphim

| Feature | Effort | Value | Notes |
|---------|--------|-------|-------|
| Pre-compaction flush | Low | High | Add a "persist insights" hook before agent session end in ADF |
| Delta-based session indexing | Medium | Medium | Automate `terraphim-agent sessions import` with file watcher |
| Chunked indexing | Medium | Low | Graph-based ranking already handles relevance; chunking is a minor optimisation |
| Dense vector layer (optional) | High | Medium | See [Cleora integration roadmap](./scorers/graph-embedding-analysis.md) for the planned hybrid approach |

### From Terraphim to OpenClaw

| Feature | Value | Notes |
|---------|-------|-------|
| Multi-source federation | High | OpenClaw is workspace-only; federated search across Jira, Confluence, email, etc. would expand its usefulness significantly |
| Role-based graphs | Medium | Different agent personas could have different search tuning |
| Pluggable scorer selection | Medium | BM25F field weighting and BM25+ tuning would improve OpenClaw's FTS5-only ranking |
| Path connectivity validation | Low | Niche but useful for query coherence checking |
| Learning capture hooks | High | Automatic failure capture is more reliable than model-initiated memory writes |

## Architectural Trade-off Summary

```
                 Terraphim                          OpenClaw
                 ---------                          --------
Offline-first <------------------------------> Cloud-optional
Deterministic <------------------------------> Probabilistic
Graph-native  <------------------------------> Embedding-native
Explicit knowledge <-------------------------> Latent knowledge
Low resource  <------------------------------> Higher resource
Exact + synonym <---------------------------> Paraphrase recall
Multi-source  <------------------------------> Single-workspace
```

Both are valid architectures. Terraphim optimises for transparency, offline operation, and low resource usage. OpenClaw optimises for paraphrase recall and ease of configuration. The planned Cleora hybrid integration (see [graph-embedding-analysis](./scorers/graph-embedding-analysis.md)) would bring optional dense vector capabilities to Terraphim while preserving its core properties.
