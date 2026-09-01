# Research Document: terraphim_sessions NFR benchmark (#3014)

**Status**: Approved (faithful-mirror, self-contained)
**Author**: Echo (implementation-swarm-A)
**Date**: 2026-07-15
**Issue**: #3014

## Executive Summary

The session-search spec (`docs/specifications/terraphim-agent-session-search-spec.md`)
asserts two performance NFRs that have **no executable proof**:

- G1 (line 29 / NFR table line 544): *Search latency <100ms for 10K sessions*
- F4 §Performance (line 374): *BM25 over 10K sessions is <10ms in benchmarks*

`terraphim_sessions` (present workspace member, `crates/*`) implements the in-memory
hybrid BM25 + KG search in `src/search.rs`, but has **no `criterion` dev-dep, no
`benches/` dir, no `[[bench]]` target**. A future regression in the BM25 scorer could
silently breach 100ms with zero CI signal. This task adds the missing reproducible
benchmark — the faithful mirror that proves (or refutes) the asserted NFR.

## Essential Questions Check

| Question | Answer | Evidence |
|----------|--------|----------|
| Energizing? | Yes | NFRs are asserted but unmeasured — drift between claim and reality is the precise bug class Echo exists to eliminate |
| Leverages strengths? | Yes | reproducibility/criterion benchmarking is Echo's SFIA SINT/TEST L4 domain |
| Meets real need? | Yes | protects the headline local-first-vs-Tantivy design decision (WIG-aligned) |

## Current State Analysis

### Code Locations
| Component | Location | Purpose |
|-----------|----------|---------|
| `search_sessions` | `crates/terraphim_sessions/src/search.rs:95` | BM25-ranked search entry point (gated by `search-index` feature) |
| `OkapiBM25Scorer` | `terraphim_types::score` (registry dep) | The BM25 implementation under measurement |
| `Session` / `Message` | `crates/terraphim_sessions/src/model.rs:249,165` | Domain types the bench must seed |
| Existing test helper `make_session` | `search.rs:236` | Pattern to mirror for synthetic-session generation |
| Precedent bench | `crates/terraphim_tinyclaw/benches/tinyclaw_benchmarks.rs` | criterion 0.8, `harness=false` pattern to twin |

### Constraints
- `search_sessions` is only compiled under the `search-index` feature (lib.rs:40-41,59).
  The bench target must enable it: `required-features = ["search-index"]`.
- `criterion = "0.8"` is the workspace precedent version (tinyclaw). Use the same — zero deviation.
- No mocks (project rule): use real `Session`/`Message`/`OkapiBM25Scorer` instances.
- Build/runs offline: `terraphim_types` is a registry dep already in `Cargo.lock`; `cargo check --features search-index` is green (verified).

## Vital Few (Essentialism)

| Constraint | Why Vital |
|------------|-----------|
| Seed **exactly 10,000** sessions (matches the NFR's stated corpus) | The NFR is quantified at 10K — measuring at any other scale would not mirror the claim |
| Time **one `search_sessions()` query** end-to-end | That is the unit the NFR ("Search latency") measures |
| Query a term that realistically appears across many sessions | Avoids degenerate "0 results, trivially fast" non-result |

### Eliminated from scope
| Eliminated | Why |
|------------|-----|
| Hybrid (`search_sessions_hybrid`) benchmark | Requires the `enrichment` feature + a `Thesaurus`; the F4 line-374 claim is specifically about BM25. Separate NFR. |
| Cold/warm cache breakdown | BM25 scorer is rebuilt per call (`OkapiBM25Scorer::new()` inside `search_sessions`); there is no warm cache to measure. Measuring it would not mirror the function's actual contract. |
| 1K / 100K scales | The NFR is stated at 10K only. Additional scales = scope creep. |

## Risks and Unknowns

| Assumption | Basis | Risk if wrong | Verified? |
|------------|-------|---------------|-----------|
| `search-index` compiles & runs in this worktree | `cargo check -p terraphim_sessions --features search-index` → Finished 1.86s | low | Yes |
| `OkapiBM25Scorer` is the "BM25" the F4 line-374 claim refers to | It is the only BM25 scorer in the search path (search.rs:102) | low | Yes |
| criterion is not in workspace `[workspace.dependencies]` | grep shows only tinyclaw's direct dep | low — add as direct dev-dep mirroring tinyclaw | Yes |

## Recommendations

**Proceed** — self-contained, buildable, no infra/network/cross-repo deps. Add criterion
dev-dep + `benches/search_nfr.rs` seeded with 10K synthetic sessions; assert via criterion
that a single query completes well under the 100ms budget (and observe the BM25-only time).
