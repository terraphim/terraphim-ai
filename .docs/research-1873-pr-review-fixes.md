# Research Document: PR Review Fixes for FffIndexer Migration

**Status**: Draft  
**Author**: AI Agent (Phase 1 Disciplined Research)  
**Date**: 2026-05-25  
**Review Source**: PR #1874 structured review comment `31094`  
**Parent Research**: `.docs/research-1873-fffindexer-migration.md`

## Executive Summary

The structured PR review identified two P1 correctness gaps and two P2 quality gaps in the FffIndexer migration. The most important findings are that configured `FffIndexer` state is bypassed by the cache wrapper and source files under `crates` are still filtered out by a hardcoded `.md` predicate. The fix should preserve the stable `IndexMiddleware` trait while making file filtering configurable and ensuring instance-level scorer/frecency state is honoured.

## Essential Questions Check

| Question | Answer | Evidence |
|----------|--------|----------|
| Energising? | Yes | The fixes unblock the stated purpose of #1873: code haystacks searchable through the middleware path. |
| Leverages strengths? | Yes | The codebase already has `ProjectConfig`, role JSON haystacks, `KgPathScorer`, and fff-search integration patterns. |
| Meets real need? | Yes | The PR review found merge-blocking P1 issues affecting runtime behaviour and acceptance criteria. |

**Proceed**: Yes, 3/3 YES.

## Problem Statement

### Description

The current PR implementation compiles and passes smoke tests but does not satisfy the review-confirmed acceptance path:

1. `FffIndexer::index()` calls `cached_fff_index()`, which creates a fresh `FffIndexer::default()` and discards any configured instance state such as `kg_scorer`.
2. `FffIndexer::index_inner()` filters all haystacks to files ending in `.md`, so source-code haystacks such as `.terraphim/role-rust-engineer.json` location `crates` cannot return `.rs` documents.
3. The KG scorer test only asserts non-empty results, so it would pass even when scoring is not applied.
4. `Cargo.lock` on PR head still contains stale crates.io `fff-search 0.8.2` metadata even after Cargo has produced a working-tree update removing it.

### Impact

The PR claims workspace-aligned fff-search and code haystack support, but Rust source search through `terraphim-cli` / middleware remains functionally blocked. KG scoring is also not observable through the public `IndexMiddleware` path.

### Success Criteria

- A configured `FffIndexer` instance preserves `kg_scorer` and frecency state when `index()` is called.
- A `ServiceType::Ripgrep` haystack can index `.rs` files when configured for code search.
- Markdown-only behaviour remains available for documentation haystacks or explicit markdown configuration.
- Tests fail before the fix and pass after the fix for both KG scorer propagation and `.rs` indexing.
- `Cargo.lock` contains only the intended Git-branch fff-search entry.

## Current State Analysis

### Existing Implementation

| Component | Location | Current Behaviour |
|-----------|----------|-------------------|
| `FffIndexer::index()` | `crates/terraphim_middleware/src/indexer/fff.rs:86` | Delegates to `cached_fff_index()` and ignores `self` state. |
| `cached_fff_index()` | `crates/terraphim_middleware/src/indexer/fff.rs:73-75` | Constructs `FffIndexer::default()` internally. |
| File filtering | `crates/terraphim_middleware/src/indexer/fff.rs:191-197` | Keeps only `.md` files for every haystack. |
| KG scorer test | `crates/terraphim_middleware/tests/fff_indexer.rs:186-224` | Only asserts result non-empty, not scorer effect. |
| Role config | `.terraphim/role-rust-engineer.json:21-25` | Uses `ServiceType::Ripgrep` for `crates`, expected to include Rust code. |
| Service write-back | `crates/terraphim_service/src/lib.rs:1030-1038` | Still uses `RipgrepIndexer::update_document()` directly. |
| Lockfile | `Cargo.lock` | PR head still has stale registry `fff-search 0.8.2`; working tree has removal. |

### Data Flow Today

```text
search_haystacks()
  -> FffIndexer::default()
  -> FffIndexer::index()
  -> cached_fff_index()
  -> FffIndexer::default()
  -> index_inner()
  -> collect_files()
  -> filter .md only
  -> grep_search()
  -> Index
```

### Desired Data Flow

```text
FffIndexer { kg_scorer, frecency }.index()
  -> decide cache path based on instance state
  -> index_inner() on the same instance when stateful
  -> collect_files()
  -> apply haystack-derived file filter
  -> apply KG/frecency scoring when configured
  -> grep_search()
  -> Index
```

## Constraints

### Technical Constraints

- Keep `IndexMiddleware` signature unchanged to avoid cascading changes across all indexers.
- Avoid speculative trait redesign; fixes must be local to `FffIndexer`, tests, and haystack parameter interpretation.
- `Index` is map-like and does not preserve ordered results; scorer tests cannot depend on map iteration order unless implementation exposes a helper.
- `Haystack.extra_parameters` is a `HashMap`; filter semantics must tolerate missing keys and string values.
- fff-search Git branch API is the aligned workspace target.

### Business Constraints

- PR #1874 should remain focused on finishing #1873 rather than becoming a broader search architecture rewrite.
- Existing markdown search users must not lose documentation search behaviour.

## Vital Few (Essentialism)

### Essential Constraints (Max 3)

| Constraint | Why It's Vital | Evidence |
|------------|----------------|----------|
| Preserve configured `self` state | Without this, KG/frecency features are dead through `IndexMiddleware`. | PR review P1 finding. |
| Support source file haystacks | Without this, #1873 acceptance criteria are not met. | `.terraphim/role-rust-engineer.json` uses `crates`. |
| Add tests that prove behaviour | Current tests pass despite the bug. | PR review P2 finding. |

### Eliminated from Scope

| Eliminated Item | Why Eliminated |
|-----------------|----------------|
| Redesigning `IndexMiddleware` to pass `ConfigState` | Too broad; not needed for review fixes. |
| Removing `RipgrepIndexer` | Separate cleanup after production confidence. |
| Changing service write-back in this fix set | Outside direct P1 path; note for follow-up or final cleanup. |
| Implementing full query-level result ordering API | `Index` does not expose ordered results; unnecessary for this PR. |
| Adding a new `ServiceType::Fff` | Would require config/schema migration; current service alias is sufficient. |

## Dependencies

### Internal Dependencies

| Dependency | Impact | Risk |
|------------|--------|------|
| `terraphim_config::Haystack` | Supplies `extra_parameters` for file filtering. | Medium: stringly typed config must be documented/testable. |
| `terraphim_file_search::KgPathScorer` | Supplies KG score for file paths. | Low: already integrated and tested in its crate. |
| `fff_search::FileItem` | Supplies relative paths, frecency score update, grep input. | Medium: API is Git-branch specific. |
| `.terraphim/role-*.json` | Real-world haystack examples. | Medium: current configs lack explicit code filter params. |

### External Dependencies

| Dependency | Version | Risk | Alternative |
|------------|---------|------|-------------|
| `fff-search` | Git branch `feat/external-scorer` | Medium: branch API may shift. | Pin commit or publish crate version later. |

## Risks and Unknowns

### Known Risks

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Switching default from markdown-only to broad search changes behaviour | Medium | Medium | Use explicit haystack `extra_parameters` for code mode, preserve markdown default when unset. |
| Cache path still hides stateful behaviour | Medium | High | Bypass cache for stateful instances or include state key. |
| KG scorer test remains non-observable | High | Medium | Extract testable scoring/order helper. |
| Lockfile update includes unrelated changes | Medium | Low | Stage only intended `Cargo.lock` dependency removal/update after review. |

### Open Questions

1. Should `.terraphim/role-rust-engineer.json` explicitly declare `extra_parameters` for source extensions? Recommendation: yes, so behaviour is intentional and reviewable.
2. Should unconfigured `ServiceType::Ripgrep` remain markdown-only? Recommendation: yes, to preserve old behaviour.
3. Should frecency-enabled default instances use cache? Recommendation: keep cached default path only for no KG scorer; frecency initialised from environment is still instance state, so document and bypass cache when present.

### Assumptions Explicitly Stated

| Assumption | Basis | Risk if Wrong | Verified? |
|------------|-------|---------------|-----------|
| `extra_parameters` can safely drive extension filtering. | Existing haystack config already uses this map for provider-specific parameters. | Need schema documentation later. | Partially |
| Markdown-only should remain default when no file filter configured. | Original RipgrepIndexer used `-tmarkdown`. | Code search may require explicit config update. | Yes |
| Source-code haystacks can opt into `.rs` through config. | `.terraphim/role-rust-engineer.json` is repo-local and can be changed. | If not changed, CLI still misses code. | Yes |

### Multiple Interpretations Considered

| Interpretation | Implications | Decision |
|----------------|--------------|----------|
| Make all `Ripgrep` haystacks search all text files by default. | Fastest way to satisfy code search; risks broad behavioural change. | Rejected. |
| Preserve markdown default but add extension opt-in. | Minimal change, explicit code haystack behaviour. | Chosen. |
| Add new `ServiceType::Fff`. | Clean semantics but bigger config migration. | Rejected for this PR. |

## Research Findings

### Key Insights

1. The cache wrapper is the root cause of the ineffective builder pattern; fixing `with_kg_scorer()` alone is insufficient.
2. The `.md` filter is only safe as a default, not as an unconditional rule.
3. The real acceptance test should include a `.rs` fixture or temporary code haystack and assert a Rust source document is returned.
4. A map-like `Index` makes ordering hard to test; scoring should be tested via a helper or reduced page limit behaviour.

### Technical Spikes Needed

| Spike | Purpose | Estimated Effort |
|-------|---------|------------------|
| Confirm exact `extra_parameters` access API | Use `Haystack::get_extra_parameters()` in `FffIndexer`. | 15 minutes |
| Validate fff-search file extension constraints vs client-side filter | Prefer client-side filter for minimal change. | 15 minutes |

## Recommendations

### Proceed/No-Proceed

Proceed. The review findings are valid and directly affect merge readiness.

### Scope Recommendations

Fix only four items:

1. Preserve instance state in `index()`.
2. Add configurable file filtering and update `.terraphim` code haystacks.
3. Add tests that fail on current behaviour.
4. Commit the intended `Cargo.lock` update.

### Risk Mitigation Recommendations

- Keep markdown-only as the default for unset filters.
- Add explicit source extension configuration to role JSON for code haystacks.
- Add tests using temporary fixtures instead of relying only on repository fixture paths.
- Do not remove `RipgrepIndexer` in this fix set.

## Next Steps

If approved:

1. Create Phase 2 design document with exact function signatures and test plan.
2. Implement tests first for P1 behaviours.
3. Apply minimal code changes.
4. Run verification and re-review PR.

## Appendix

### Reference Materials

- PR review: `https://git.terraphim.cloud/terraphim/terraphim-ai/pulls/1874#issuecomment-31094`
- Parent design: `.docs/design-1873-fffindexer-migration.md`
- Current FffIndexer: `crates/terraphim_middleware/src/indexer/fff.rs`
- Role config: `.terraphim/role-rust-engineer.json`
