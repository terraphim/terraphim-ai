# Research Document: Hybrid KG-First RLM-Assisted Search

**Status**: Draft
**Author**: OpenCode
**Date**: 2026-05-18
**Reviewers**: Terraphim maintainers

## Executive Summary

Terraphim should implement a KG-first hybrid search pipeline inspired by `rlmgrep`, but not clone `rlmgrep`'s RLM-first corpus loading model. The essential improvement is to reuse Terraphim's existing role, knowledge graph, haystack, ranking and RLM infrastructure so deterministic search handles the hot path, while RLM is invoked only for low-confidence, ambiguous or synthesis-heavy cases and can propose new KG concepts.

## Essential Questions Check

| Question | Answer | Evidence |
|----------|--------|----------|
| Energising? | Yes | This strengthens Terraphim's core knowledge graph search capability and makes RLM an amplifier instead of a cost centre. |
| Leverages strengths? | Yes | Terraphim already has role-based KG, haystacks, TerraphimGraph ranking, BM25/TF-IDF scoring and RLM orchestration. |
| Meets real need? | Yes | Agents need semantic code/documentation search that can fall back to reasoning without duplicating search functionality or polluting the KG. |

**Proceed**: Yes - 3/3 YES.

## Problem Statement

### Description

`rlmgrep` demonstrates a useful agent-facing pattern: natural-language search over files with grep-like, line-grounded output and optional narrative answers. Terraphim needs equivalent or better behaviour across KG, documentation haystacks, code haystacks and external haystacks, but should avoid adding a second RLM-first search implementation that bypasses existing Terraphim search and ranking.

### Impact

Without this change, agents either use literal search tools directly or invoke RLM without the benefit of Terraphim's KG and haystack infrastructure. This increases cost, reduces grounding, and misses an opportunity to turn failed searches into durable KG learning.

### Success Criteria

- A single hybrid search workflow reuses existing Terraphim search primitives.
- KG/haystack search is always attempted before RLM fallback.
- RLM is used only when confidence is low, evidence conflicts, or synthesis is requested.
- Results include grounded evidence, source haystack, scores and matched KG concepts.
- RLM can propose new KG concepts, but automatic KG mutation is gated by judge confidence or review status.

## Current State Analysis

### Existing Implementation

Terraphim already has the main components needed for this feature:

- Role configuration stores relevance function, KG, haystacks and LLM settings.
- Middleware searches role haystacks across multiple backends.
- `TerraphimService::search` ranks results through existing relevance functions.
- `TerraphimGraph` search loads/builds thesaurus data, indexes haystack documents into rolegraph, applies lexical fallback and blends TF-IDF scoring.
- `terraphim_rlm` provides sessions, budgets, execution backends, MCP tools and KG command validation.
- `build-runner-llm` already establishes the desired KG-first, LLM-on-exception pattern.

### Code Locations

| Component | Location | Purpose |
|-----------|----------|---------|
| Role and haystack configuration | `crates/terraphim_config/src/lib.rs` | Defines `Role`, `Haystack`, `ServiceType`, relevance and LLM settings. |
| Haystack dispatch | `crates/terraphim_middleware/src/indexer/mod.rs` | Searches configured haystacks through existing service-specific indexers. |
| Ripgrep haystack | `crates/terraphim_middleware/src/indexer/ripgrep.rs` | Searches local markdown/file haystacks and returns `Document`s. |
| Search service | `crates/terraphim_service/src/lib.rs` | Implements `TerraphimService::search` and relevance-specific ranking. |
| TerraphimGraph ranking | `crates/terraphim_service/src/lib.rs` | Builds/loads thesaurus, indexes haystack docs into rolegraph, performs fallback and TF-IDF blend. |
| RLM public API | `crates/terraphim_rlm/src/rlm.rs` | Manages RLM sessions, query loop, execution and snapshots. |
| RLM LLM bridge | `crates/terraphim_rlm/src/llm_bridge.rs` | Provides recursive LLM calls, currently with a TODO stub for real LLM integration. |
| RLM KG validator | `crates/terraphim_rlm/src/validator.rs` | Validates text/commands against thesaurus and role graph. |
| KG-first build precedent | `scripts/build-runner-llm.sh` and `.docs/ARCHITECTURE-build-runner-llm.md` | Shows KG-first, LLM-on-exception architecture and learning capture. |

### Data Flow Today

```text
SearchQuery
  -> TerraphimService::search
  -> search_haystacks(role.haystacks)
  -> relevance-specific ranking
  -> optional KG preprocessing / AI summarisation
  -> Vec<Document>
```

### Integration Points

- `terraphim_middleware::search_haystacks` for retrieval.
- `ConfigState::search_indexed_documents` and rolegraph for KG ranking.
- `terraphim_types::Document`, `IndexedDocument`, `SearchQuery` for shared data.
- `terraphim_rlm` MCP tools for recursive fallback and judgement once bridge integration is real.
- `terraphim-agent learn` and KG haystack files for durable learning.

## Constraints

### Technical Constraints

- Do not duplicate the haystack dispatch layer; reuse `search_haystacks`.
- Do not add a separate RLM-first search engine as the primary path.
- Preserve role-specific KG semantics and existing relevance functions.
- RLM fallback must operate on bounded top-K evidence, not whole-repository unbounded context.
- KG concept creation must be reviewable or judge-gated to avoid graph pollution.

### Business Constraints

- Keep hot-path cost close to current KG/ripgrep/BM25 search cost.
- Preserve agent-friendly output contracts for automation.
- Avoid destabilising existing search endpoints.

### Non-Functional Requirements

| Requirement | Target | Current |
|-------------|--------|---------|
| Hot-path LLM calls | 0 | Existing search generally avoids RLM; RLM bridge is stubbed. |
| Fallback grounding | All answers cite source evidence | Existing search returns documents, but no hybrid judgement contract. |
| KG mutation safety | No direct unjudged mutation | Current RLM is not creating concepts. |
| Search reuse | Reuse current service/middleware | Existing components are available. |

## Vital Few

### Essential Constraints

| Constraint | Why It's Vital | Evidence |
|------------|----------------|----------|
| KG/haystack first | Keeps cost low and leverages Terraphim's unique capability. | `build-runner-llm` precedent and existing search stack. |
| Reuse existing search/ranking | Prevents duplicated functionality and divergence. | `TerraphimService::search` and `search_haystacks` already cover core retrieval. |
| Judge-gated KG concepts | Prevents RLM hallucinations from degrading future search. | RLM concept mining is probabilistic. |

### Eliminated from Scope

| Eliminated Item | Why Eliminated |
|-----------------|----------------|
| Direct Python `rlmgrep` dependency | Would duplicate retrieval, ranking and configuration in another runtime. |
| Whole-repository RLM corpus loading | Expensive, less scalable and bypasses Terraphim haystacks. |
| Automatic unconditional KG writes | High risk of KG pollution. |
| New vector database | Not required to solve this; existing KG/BM25/TF-IDF should be used first. |

## Dependencies

### Internal Dependencies

| Dependency | Impact | Risk |
|------------|--------|------|
| `terraphim_service` | Main orchestration point for hybrid search. | Medium: search method is large and should not grow further without careful factoring. |
| `terraphim_middleware` | Existing haystack retrieval. | Low: reuse rather than replace. |
| `terraphim_rlm` | Fallback reasoning and concept mining. | Medium: real LLM bridge is incomplete. |
| `terraphim_types` | Shared request/result types. | Low: additive types can be introduced. |
| `terraphim_rolegraph` | KG concept matching and connectivity. | Medium: concept proposal flow must not break existing graph semantics. |

### External Dependencies

| Dependency | Version | Risk | Alternative |
|------------|---------|------|-------------|
| `rlmgrep` | External GitHub project | Informational only; do not depend directly. | Reuse design ideas in Rust/Terraphim. |
| LLM providers through Terraphim routing | Existing project integration | Medium due to cost/rate limits. | Deterministic-only degraded mode. |

## Risks and Unknowns

### Known Risks

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| KG pollution from bad concepts | Medium | High | Store proposals separately; require evidence and judge confidence. |
| Duplicating search logic | Medium | High | Implement orchestration over existing `search_haystacks` and `TerraphimService::search`. |
| RLM bridge incompleteness | High | Medium | Start with interface and gated fallback; wire bridge via existing router as prerequisite. |
| Search service growth | Medium | Medium | Add a small orchestrator module rather than expanding the main `search` match arms. |
| Cost creep | Medium | High | Explicit confidence gate, top-K evidence cap and telemetry. |

### Open Questions

1. What exact role should own hybrid search by default: selected role, `Terraphim Engineer`, or an explicit request role?
2. Should concept proposals be persisted as KG markdown, a separate review queue, or both?
3. Which judge implementation should be used first: `terraphim_rlm` role prompt, existing validation crate, or a dedicated MCP tool?

### Assumptions Explicitly Stated

| Assumption | Basis | Risk if Wrong | Verified? |
|------------|-------|---------------|-----------|
| Existing search results can provide enough evidence for most queries. | TerraphimGraph plus haystack search already returns ranked documents. | RLM fallback may be needed more often than expected. | Partially |
| RLM should be a fallback and enrichment mechanism, not the default retriever. | Cost and existing build-runner-llm architecture. | If deterministic retrieval is too weak, quality may lag. | Yes |
| Concept creation must be gated. | LLM outputs are probabilistic. | Ungated writes degrade KG quality. | Yes |

### Multiple Interpretations Considered

| Interpretation | Implications | Why Chosen/Rejected |
|----------------|--------------|---------------------|
| Port `rlmgrep` directly | Fast to prototype but duplicates Terraphim retrieval and relies on Python/DSPy. | Rejected: conflicts with reuse requirement. |
| Add RLM as first search stage | Better semantic recall but high cost and weak KG leverage. | Rejected: violates KG-first architecture. |
| Add hybrid orchestrator over existing search | Minimal duplication, preserves Terraphim strengths, bounded RLM use. | Chosen. |

## Research Findings

### Key Insights

1. `rlmgrep`'s most valuable idea is not its exact implementation, but its stable agent-facing contract: grounded matches, optional narrative answer and structured output.
2. Terraphim already has broader retrieval than `rlmgrep` through role haystacks and KG ranking.
3. The missing piece is an explicit confidence gate and RLM-assisted enrichment loop.
4. `build-runner-llm` provides the correct operating model: KG hot path, LLM cold path, learning capture.

### Relevant Prior Art

- `halfprice06/rlmgrep`: grep-shaped semantic search with verified path/line matches.
- `build-runner-llm`: KG-first deterministic transformation with LLM only for cold-start/edge cases.
- TerraphimGraph search: role-specific graph ranking with lexical fallback and TF-IDF blending.

### Technical Spikes Needed

| Spike | Purpose | Estimated Effort |
|-------|---------|------------------|
| Hybrid confidence scoring | Define thresholds and metadata from existing ranked documents. | 0.5 day |
| RLM bridge routing | Replace bridge stub with existing LLM router integration. | 1 day |
| Concept proposal persistence | Decide minimal reviewable representation for new KG concepts. | 0.5 day |

## Recommendations

### Proceed/No-Proceed

Proceed with a small hybrid-search orchestrator that reuses current search and ranking, adds confidence/judge semantics, and invokes RLM only as fallback/enrichment.

### Scope Recommendations

- Start with service-level orchestration and CLI/MCP-facing structured output.
- Use existing haystacks and relevance functions unchanged.
- Add code line-level metadata only where current `Document` output lacks enough grounding.
- Defer non-text conversion parity with `rlmgrep` unless a specific haystack requires it.

### Risk Mitigation Recommendations

- Make KG concept proposals opt-in or reviewable by default.
- Add telemetry for `fallback_used`, `rlm_calls`, `concepts_proposed` and confidence.
- Keep deterministic search usable when LLM/RLM is unavailable.

## Next Steps

If approved:

1. Implement shared hybrid search request/result types.
2. Add an orchestrator module that calls existing search and confidence scoring.
3. Add RLM fallback behind a confidence threshold.
4. Add judge-gated KG concept proposal flow.
5. Expose through CLI/MCP with stable JSON output.

## Appendix

### Reference Materials

- `https://github.com/halfprice06/rlmgrep`
- `.docs/ARCHITECTURE-build-runner-llm.md`
- `scripts/build-runner-llm.sh`
- `crates/terraphim_service/src/lib.rs`
- `crates/terraphim_middleware/src/indexer/mod.rs`
- `crates/terraphim_rlm/src/llm_bridge.rs`
