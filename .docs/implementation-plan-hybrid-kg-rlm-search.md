# Implementation Plan: Hybrid KG-First RLM-Assisted Search

**Status**: Draft
**Research Doc**: `.docs/research-hybrid-kg-rlm-search.md`
**Author**: OpenCode
**Date**: 2026-05-18
**Estimated Effort**: 4-6 days

## Overview

### Summary

Add a hybrid search workflow that reuses Terraphim's existing KG, haystack and relevance infrastructure, then invokes RLM only when deterministic search confidence is low or the caller requests synthesis. RLM may propose new KG concepts, but concept promotion is judge-gated or reviewable.

### Approach

Create a thin orchestrator over existing components rather than introducing a separate `rlmgrep` clone. The orchestrator will call existing search, calculate confidence, optionally call RLM with bounded evidence, validate output with a judge, and return a stable structured result.

### Scope

**In Scope:**

- Shared hybrid search request/result types.
- Service-level orchestrator that reuses `TerraphimService::search` and current haystack/ranking behaviour.
- Confidence scoring over existing document evidence.
- Optional RLM fallback using top-K retrieved evidence.
- Judge-gated KG concept proposal output.
- CLI/MCP JSON output suitable for agents.
- Tests covering deterministic path, fallback trigger, no-LLM degraded path and concept proposal gating.

**Out of Scope:**

- Direct dependency on Python `rlmgrep` or DSPy.
- New vector database or embedding store.
- Replacing existing relevance functions.
- Automatic unconditional KG writes.
- Full non-text conversion parity with `rlmgrep`.

**Avoid At All Cost:**

- Duplicating haystack dispatch logic outside `terraphim_middleware`.
- Duplicating relevance/ranking logic outside `terraphim_service` and existing score modules.
- Letting RLM become the default retriever.
- Allowing ungrounded RLM concepts into the KG.

## Architecture

### Component Diagram

```text
HybridSearchRequest
  -> HybridSearchOrchestrator
      -> TerraphimService::search / search_haystacks
      -> ConfidenceScorer
      -> if confident: HybridSearchResult
      -> if not confident: RlmFallback
          -> top-K evidence only
          -> Judge
          -> ConceptProposalBuilder
      -> HybridSearchResult
```

### Data Flow

```text
query + role + options
  -> existing role KG and haystack retrieval
  -> existing ranking and KG preprocessing
  -> confidence assessment
  -> optional RLM synthesis and concept mining
  -> judge validation
  -> structured result with evidence and proposed concepts
```

### Key Design Decisions

| Decision | Rationale | Alternatives Rejected |
|----------|-----------|----------------------|
| Orchestrator over existing search | Minimises duplication and preserves current behaviour. | Direct `rlmgrep` port; separate search stack. |
| RLM fallback only | Controls cost and keeps KG/haystack search as source of truth. | RLM-first retrieval. |
| Top-K evidence to RLM | Bounds cost and improves grounding. | Whole repository/context loading. |
| Proposed KG concepts first | Prevents KG pollution. | Direct automatic writes. |

### Eliminated Options

| Option Rejected | Why Rejected | Risk of Including |
|-----------------|--------------|-------------------|
| Python/DSPy dependency | Adds runtime and duplicates existing Rust services. | Operational complexity and divergent output semantics. |
| New `ServiceType::Rlmgrep` | Makes RLM a haystack rather than an orchestrated fallback. | Expensive and confusing layering. |
| Embeddings/vector DB | Not required for first increment. | Broad dependency and migration burden. |
| Rewriting `TerraphimService::search` | Existing method is already large. | Regression risk. |

### Simplicity Check

The simplest design is an additive orchestrator module that calls current search, calculates confidence and conditionally calls RLM. It does not replace haystacks, relevance functions, rolegraph, persistence or CLI configuration.

**Nothing Speculative Checklist:**

- [x] No features the user did not request.
- [x] No new abstraction unless it prevents duplication.
- [x] No new datastore.
- [x] No unconditional KG mutation.
- [x] No direct dependency on `rlmgrep`.

## File Changes

### New Files

| File | Purpose |
|------|---------|
| `crates/terraphim_types/src/hybrid_search.rs` | Shared request/result, evidence, confidence and concept proposal types. |
| `crates/terraphim_service/src/hybrid_search.rs` | Orchestrator, confidence scoring and fallback decision logic. |
| `crates/terraphim_service/tests/hybrid_search_test.rs` | Integration tests over existing service configuration and real local haystacks. |

### Modified Files

| File | Changes |
|------|---------|
| `crates/terraphim_types/src/lib.rs` | Re-export hybrid search types. |
| `crates/terraphim_service/src/lib.rs` | Add `pub mod hybrid_search` and expose service entry point without expanding existing `search` internals. |
| `crates/terraphim_rlm/src/llm_bridge.rs` | Replace or wrap stubbed query implementation with existing LLM routing, if this issue includes fallback execution. |
| `crates/terraphim_mcp_server/src/lib.rs` | Add MCP tool or extend search tool with hybrid mode. |
| `crates/terraphim_agent/src/main.rs` | Add CLI command or flag for hybrid search JSON output. |

### Deleted Files

None.

## API Design

### Public Types

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HybridSearchRequest {
    pub query: SearchQuery,
    pub mode: HybridSearchMode,
    pub max_evidence: usize,
    pub min_confidence: f32,
    pub allow_rlm_fallback: bool,
    pub allow_concept_proposals: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum HybridSearchMode {
    SearchOnly,
    Answer,
    AnswerWithConceptMining,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HybridSearchResult {
    pub answer: Option<String>,
    pub evidence: Vec<SearchEvidence>,
    pub confidence: SearchConfidence,
    pub fallback_used: Option<RlmFallbackReason>,
    pub concept_proposals: Vec<KgConceptProposal>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchEvidence {
    pub document_id: String,
    pub title: String,
    pub url: String,
    pub source_haystack: Option<String>,
    pub snippet: Option<String>,
    pub line: Option<u64>,
    pub rank: Option<u64>,
    pub score: Option<f32>,
    pub concepts_matched: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchConfidence {
    pub score: f32,
    pub enough_results: bool,
    pub enough_kg_coverage: bool,
    pub top_result_separation: Option<f32>,
    pub requires_fallback: bool,
    pub reasons: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KgConceptProposal {
    pub term: String,
    pub synonyms: Vec<String>,
    pub evidence_urls: Vec<String>,
    pub confidence: f32,
    pub status: ConceptProposalStatus,
}
```

### Public Functions

```rust
impl TerraphimService {
    pub async fn hybrid_search(
        &mut self,
        request: &HybridSearchRequest,
    ) -> Result<HybridSearchResult>;
}
```

### Error Types

Prefer existing `ServiceError`. Add variants only if needed:

```rust
#[error("hybrid search fallback failed: {0}")]
HybridFallback(String),
```

## Test Strategy

### Unit Tests

| Test | Location | Purpose |
|------|----------|---------|
| `confidence_high_for_ranked_documents` | `hybrid_search.rs` | Does not invoke RLM when evidence is strong. |
| `confidence_low_for_empty_documents` | `hybrid_search.rs` | Triggers fallback decision. |
| `concept_proposals_default_to_proposed` | `hybrid_search.rs` | Prevents direct accepted KG writes. |
| `evidence_preserves_source_haystack` | `hybrid_search.rs` | Keeps grounding metadata. |

### Integration Tests

| Test | Location | Purpose |
|------|----------|---------|
| `hybrid_search_reuses_existing_haystack_results` | `crates/terraphim_service/tests/hybrid_search_test.rs` | Confirms retrieval comes from configured haystacks. |
| `hybrid_search_degrades_without_rlm` | `crates/terraphim_service/tests/hybrid_search_test.rs` | Returns deterministic evidence with fallback disabled/unavailable. |
| `hybrid_search_concept_mining_is_gated` | `crates/terraphim_service/tests/hybrid_search_test.rs` | Ensures concepts are proposed, not written unconditionally. |

### No Mocks

Tests should use temporary local haystacks and real service configuration. Do not mock haystack indexers or RLM; use deterministic fallback-disabled tests first, then add feature-gated integration once the real LLM bridge is wired.

## Implementation Steps

### Step 1: Shared Types

**Files:** `crates/terraphim_types/src/hybrid_search.rs`, `crates/terraphim_types/src/lib.rs`
**Description:** Add serialisable request/result/evidence/confidence/concept proposal types.
**Tests:** Type construction and serialisation tests.
**Estimated:** 0.5 day

### Step 2: Confidence Scorer

**Files:** `crates/terraphim_service/src/hybrid_search.rs`
**Description:** Add pure functions that score existing `Document` evidence without invoking RLM.
**Tests:** Unit tests for high-confidence, low-confidence and edge cases.
**Dependencies:** Step 1
**Estimated:** 0.5 day

### Step 3: Orchestrator Over Existing Search

**Files:** `crates/terraphim_service/src/hybrid_search.rs`, `crates/terraphim_service/src/lib.rs`
**Description:** Implement `TerraphimService::hybrid_search` by calling existing `search` and converting documents to evidence.
**Tests:** Integration tests with a real temporary haystack.
**Dependencies:** Step 2
**Estimated:** 1 day

### Step 4: RLM Fallback Adapter

**Files:** `crates/terraphim_service/src/hybrid_search.rs`, `crates/terraphim_rlm/src/llm_bridge.rs`
**Description:** Add bounded top-K evidence prompt path and wire real LLM routing if required for fallback.
**Tests:** Feature-gated integration or fallback-disabled deterministic test until provider credentials are available.
**Dependencies:** Step 3
**Estimated:** 1-2 days

### Step 5: Judge and Concept Proposals

**Files:** `crates/terraphim_service/src/hybrid_search.rs`
**Description:** Parse RLM concept mining output into `KgConceptProposal`, mark as `Proposed`, and include evidence references.
**Tests:** Validate ungrounded proposals are rejected or marked low confidence.
**Dependencies:** Step 4
**Estimated:** 1 day

### Step 6: CLI/MCP Exposure

**Files:** `crates/terraphim_agent/src/main.rs`, `crates/terraphim_mcp_server/src/lib.rs`
**Description:** Add agent-facing command/tool returning stable JSON.
**Tests:** CLI/MCP integration tests if existing harness supports them.
**Dependencies:** Step 5
**Estimated:** 1 day

### Step 7: Documentation and Telemetry

**Files:** `.docs/summary.md`, relevant command docs
**Description:** Document KG-first behaviour, fallback semantics and concept proposal safety.
**Tests:** Docs build/check if applicable.
**Dependencies:** Step 6
**Estimated:** 0.5 day

## Rollback Plan

If issues are discovered:

1. Disable CLI/MCP hybrid entry point.
2. Keep existing `TerraphimService::search` unchanged and continue using current search endpoints.
3. Ignore proposed concept outputs; no accepted KG mutations should need rollback.

Feature flag candidate: `hybrid-rlm-search`.

## Dependencies

### New Dependencies

None expected.

### Dependency Updates

None expected.

## Performance Considerations

### Expected Performance

| Metric | Target | Measurement |
|--------|--------|-------------|
| Hot-path LLM calls | 0 | Telemetry field `fallback_used == None`. |
| Evidence cap | Default top 10 | Request/result metadata. |
| RLM fallback cost | Explicit and counted | Existing RLM budget/cost tracking. |
| Search latency | Close to existing search latency when fallback unused | Integration benchmark or timed test. |

## Open Items

| Item | Status | Owner |
|------|--------|-------|
| Decide exact concept proposal persistence location | Pending | Maintainers |
| Decide whether to expose as new MCP tool or mode on existing search | Pending | Maintainers |
| Confirm real LLM bridge routing path | Pending | Implementer |

## Approval

- [ ] Technical review complete
- [ ] Test strategy approved
- [ ] Performance targets agreed
- [ ] Human approval received
