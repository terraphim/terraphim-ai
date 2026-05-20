## Problem

Terraphim needs an agent-facing semantic search workflow comparable in usefulness to `rlmgrep`, but implemented in a way that reuses existing Terraphim capabilities rather than duplicating them.

`rlmgrep` proves the value of natural-language, grep-shaped search with grounded path/line output and optional narrative answers. Terraphim already has stronger primitives: role-specific knowledge graphs, multiple haystack backends, TerraphimGraph ranking, BM25/TF-IDF scoring, and `terraphim_rlm` orchestration. The missing capability is a single KG-first hybrid workflow that searches existing haystacks first, judges confidence, and only then invokes RLM for low-confidence, ambiguous, synthesis-heavy, or concept-mining cases.

## Required Approach

Implement a hybrid search orchestrator using the `build-runner-llm` operating model:

1. Use Terraphim KG and configured role haystacks first.
2. Reuse existing retrieval/ranking rather than adding a second search stack.
3. Invoke RLM only when deterministic results are weak, ambiguous, conflicting, or synthesis is requested.
4. Use RLM to propose missing KG concepts only with evidence and judge gating.
5. Return stable JSON suitable for agents, with grounded evidence and fallback metadata.

## Disciplined Research and Design Artefacts

- Research: `.docs/research-hybrid-kg-rlm-search.md`
- Implementation plan: `.docs/implementation-plan-hybrid-kg-rlm-search.md`

## Existing Functionality To Reuse

- `crates/terraphim_config/src/lib.rs`: `Role`, `Haystack`, `ServiceType`, relevance and LLM settings.
- `crates/terraphim_middleware/src/indexer/mod.rs`: `search_haystacks` dispatch across configured haystacks.
- `crates/terraphim_middleware/src/indexer/ripgrep.rs`: local file/markdown retrieval.
- `crates/terraphim_service/src/lib.rs`: `TerraphimService::search` and existing relevance-specific ranking.
- `crates/terraphim_service/src/lib.rs`: `RelevanceFunction::TerraphimGraph` branch for thesaurus/rolegraph ranking, lexical fallback and TF-IDF blending.
- `crates/terraphim_rlm/src/rlm.rs`: sessions, budgets, execution and query loop.
- `crates/terraphim_rlm/src/validator.rs`: KG validation against thesaurus and rolegraph.
- `scripts/build-runner-llm.sh`: precedent for KG-first, LLM-on-exception behaviour.

## Acceptance Criteria

- Add shared hybrid search request/result types with evidence, confidence, fallback and KG concept proposal metadata.
- Add a service-level hybrid search orchestrator that calls existing `TerraphimService::search` and/or `search_haystacks` instead of duplicating haystack retrieval.
- Add confidence scoring that decides whether deterministic KG/haystack results are sufficient.
- Ensure the hot path performs zero RLM calls when confidence is high.
- Add bounded RLM fallback using top-K evidence only; do not load an entire repository into RLM context by default.
- Add judge-gated KG concept proposal output; concepts must default to proposed/reviewable rather than accepted KG mutation.
- Expose an agent-friendly CLI or MCP entry point with stable JSON output.
- Add tests using real temporary local haystacks and existing service configuration; do not use mocks.
- Document the workflow, especially the no-duplication rule and KG concept safety gate.

## Non-Goals

- Do not add a direct Python/DSPy `rlmgrep` runtime dependency.
- Do not create a parallel haystack dispatcher.
- Do not replace existing relevance functions.
- Do not add a new vector database or embedding store for this increment.
- Do not automatically write unjudged RLM concepts into the accepted KG.

## Suggested Implementation Steps

1. Add `hybrid_search` shared types in `terraphim_types`.
2. Add `terraphim_service::hybrid_search` module with confidence scoring.
3. Add `TerraphimService::hybrid_search` that wraps existing search and converts documents to evidence.
4. Add RLM fallback adapter once real LLM routing is available through `terraphim_rlm`.
5. Add concept proposal parsing and judge gating.
6. Expose via CLI/MCP.
7. Add tests, documentation and telemetry.

## Verification

- `cargo test -p terraphim_types hybrid_search`
- `cargo test -p terraphim_service hybrid_search`
- Relevant CLI/MCP tests for the exposed entry point
- Coverage check for changed crates
- UBS scan on changed files before commit

## Risks

- KG pollution if RLM concept proposals are accepted without evidence and judgement.
- Cost creep if RLM fallback triggers too often.
- Regression risk if existing `TerraphimService::search` is modified directly rather than wrapped.

## Constraints

- Keep deterministic search usable when RLM is unavailable.
- Prefer an additive orchestrator over invasive changes to existing search internals.
- Preserve British English in documentation and generated output.
