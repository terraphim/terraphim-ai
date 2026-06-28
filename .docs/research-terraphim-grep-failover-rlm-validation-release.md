# Research Document: Terraphim Grep KG Failover, RLM Validation, and Release Readiness

**Status**: Draft | Pending human approval before design
**Author**: OpenCode / Terraphim Engineer
**Date**: 2026-06-27
**Reviewers**: TBD

## Executive Summary

Three interrelated problems block a clean release of the terraphim toolchain:

1. `terraphim-grep` fails hard when no knowledge-graph thesaurus is configured, even though its underlying `fff-search` backend can already search code without KG boosting.
2. Issue #2491 claims `terraphim_rlm::QueryLoop` bypasses `executor.validate()` on the LLM-driven execution path; a recent commit appears to have wired validation in, but no unit test proves it and the issue remains open.
3. The installed `terraphim-grep` and `terraphim-agent` binaries are at `1.20.5` while the workspace and latest Gitea release are at `1.21.0`, and `terraphim-rlm` is not installed at all.

This research document maps the current code, identifies constraints, and recommends a bounded scope so the next design phase can produce an implementable plan.

## Essential Questions Check

| Question | Answer | Evidence |
|----------|--------|----------|
| Energizing? | Yes | Makes the CLI usable out-of-the-box and closes a P1 RLM security gap. |
| Leverages strengths? | Yes | Reuses existing `fff-search` integration; no new search engine needed. |
| Meets real need? | Yes | User explicitly asked for failover behaviour and release readiness. |

**Proceed**: Yes — 3/3 YES.

## Problem Statement

### 1. `terraphim-grep` requires a thesaurus

**Description**: In `terraphim-clients/crates/terraphim_grep/src/main.rs`, if `--thesaurus` is not provided and `find_default_thesaurus()` returns `None`, the CLI exits with:

```text
No thesaurus specified and could not find default. Use --thesaurus to specify path.
```

**Impact**: Users cannot use `terraphim-grep` as a fast, enhanced code grep until they have built a knowledge graph.

**Success Criteria**: When no thesaurus/KG is configured, `terraphim-grep` falls back to `fff-search`-powered plain code search, returning results in the same `GrepResult` shape but with empty concepts and no KG boost.

### 2. `terraphim_rlm` QueryLoop validation gap (Issue #2491)

**Description**: Issue #2491 states that `QueryLoop::execute()` calls `executor.execute_command()` / `execute_code()` without first calling `executor.validate()`. Code inspection shows that commit `7b46fb3de` (`feat(rlm): wire KG validation into executor hot paths and QueryLoop`) added `validate_command()` calls inside the `Command::Run` and `Command::Code` arms. However, the issue is still open and no unit test asserts that `validate()` is invoked.

**Impact**: Without a test, the fix can regress silently; the issue also blocks confidence in the RLM security posture.

**Success Criteria**: A unit test with a mock executor proves that `validate()` is called once for every `execute_command()` / `execute_code()` invocation on the QueryLoop hot path.

### 3. Release readiness mismatch

**Description**:

- `terraphim-ai` workspace version is `1.21.0`; Gitea release `v1.21.0` exists; HEAD is `v1.21.0-29-g236d7479c`.
- `terraphim-clients` workspace version is still `1.20.5`.
- Installed cargo binaries report `1.20.5` for `terraphim-grep` and `terraphim-agent`; `terraphim-rlm` is not on `PATH`.
- `terraphim-ai/Cargo.lock` has uncommitted changes (proptest dev-dependency from `terraphim_spawner`).
- Many untracked `.docs/` design/research/handover files exist.

**Impact**: Users cannot install the latest released toolchain from `cargo install`; local builds and releases are inconsistent.

**Success Criteria**: Version numbers align, `Cargo.lock` is committed, binaries are built/installed, and untracked documentation is either committed or removed.

## Current State Analysis

### `terraphim-grep` architecture

| Component | Location | Purpose |
|-----------|----------|---------|
| CLI entry | `terraphim-clients/crates/terraphim_grep/src/main.rs` | Parses args, loads thesaurus, builds searcher, prints results |
| Search core | `terraphim-clients/crates/terraphim_grep/src/lib.rs` | `TerraphimGrep::search`, sufficiency judge, RLM fallback |
| Hybrid search | `terraphim-clients/crates/terraphim_grep/src/hybrid_searcher.rs` | Concurrent KG + `fff-search` code search, KG boosting |
| Cargo manifest | `terraphim-clients/crates/terraphim_grep/Cargo.toml` | Features: `default = ["llm"]`, optional `code-search = ["dep:fff-search"]` |

**Data flow**:

```text
query -> HybridSearcher::search
  ├─ KG path: RoleGraph + thesaurus -> KgConcepts
  └─ Code path: fff-search (FilePicker.grep) -> RetrievedChunk[]
  -> boost_chunks_with_kg(...) -> HybridResults
  -> SufficiencyJudge -> maybe RLM synthesis -> GrepResult
```

When no `thesaurus` is available, the code path already compiles behind `#[cfg(feature = "code-search")]`, but the CLI refuses to start because `main.rs` treats the thesaurus as mandatory.

### `terraphim_rlm` validation architecture

| Component | Location | Purpose |
|-----------|----------|---------|
| QueryLoop | `crates/terraphim_rlm/src/query_loop.rs` | LLM-driven execution loop |
| Execution trait | `crates/terraphim_rlm/src/executor/trait.rs` | `ExecutionEnvironment::validate()` |
| Validation result | `crates/terraphim_rlm/src/executor/context.rs` | `ValidationResult { is_valid, unknown_terms, ... }` |
| Validator impl | `crates/terraphim_rlm/src/validator.rs` | `KnowledgeGraphValidator` |
| Config | `crates/terraphim_rlm/src/config.rs` | `KgStrictness` and `RlmConfig` |

**Data flow (current)**:

```text
LLM response -> Command::Run(cmd) / Command::Code(code)
  -> QueryLoop::validate_command(input)
    -> executor.validate(input) -> ValidationResult
  -> if !is_valid: record history failure + return Continue(feedback)
  -> else: executor.execute_command/execute_code
```

`validate()` is now called before execution, but there is no test coverage for the ordering.

### Release state

| Repository | Current version | Latest tag/release | Dirty state |
|------------|-----------------|--------------------|-------------|
| `terraphim-ai` | `1.21.0` | `v1.21.0` (2026-06-22) | `Cargo.lock` modified; ~20 untracked docs |
| `terraphim-clients` | `1.20.5` | `3a67627` | clean |
| Installed binaries | `terraphim-grep 1.20.5`, `terraphim-agent 1.20.5` | N/A | `terraphim-rlm` missing |

## Constraints

### Technical Constraints

- `terraphim-grep` lives in the separate `terraphim-clients` repository, versioned independently of `terraphim-ai`.
- `fff-search` is an optional dependency gated by the `code-search` feature.
- `HybridSearcher::new` requires a `Thesaurus` because it constructs a `RoleGraph`.
- `terraphim-clients` workspace version is `1.20.5` and depends on `terraphim_service` `1.20.5` from the Gitea registry.
- `terraphim-ai` workspace version is `1.21.0`; release `v1.21.0` is already published.

### Business Constraints

- Do not break existing KG-boosted search behaviour.
- The failover must be automatic and discoverable; no new mandatory flags.
- Issue #2491 is tracked in `terraphim/terraphim-ai`, so the RLM test work belongs there.

### Non-Functional Requirements

| Requirement | Target | Current |
|-------------|--------|---------|
| `terraphim-grep` cold-start latency | < 1 s without KG | Fails before searching |
| `terraphim_rlm` test coverage for validation ordering | 1 unit test | None |
| Workspace compile | `cargo check --workspace` passes | Passes |
| Binary install parity | All three binaries at latest version | Two at 1.20.5, one missing |

## Vital Few (Essentialism)

### Essential Constraints (Max 3)

| Constraint | Why It's Vital | Evidence |
|------------|----------------|----------|
| Failover must not require a thesaurus | Enables CLI use before any KG setup | Current hard failure in `main.rs` |
| Validation ordering must be tested | Prevents silent regression of #2491 | Issue still open, no test |
| Version alignment before release | Users install from crates; mismatch is confusing | Installed binaries report 1.20.5 |

### Eliminated from Scope

| Eliminated Item | Why Eliminated |
|-----------------|----------------|
| Re-architecting `fff-search` integration | Existing integration already works; only the CLI gate needs changing |
| Adding doc/MD search failover without `fff-search` | `code-search` feature is the existing, tested path |
| Rewriting `KnowledgeGraphValidator` | Validator is functional; only test coverage and strictness wiring need review |
| Major new release automation (CI release-plz tuning) | Out of scope for this fix; focus on manual release readiness |

## Dependencies

### Internal Dependencies

| Dependency | Impact | Risk |
|------------|--------|------|
| `terraphim_rolegraph::RoleGraph` | Required by `HybridSearcher::new`; must be made optional or constructible from empty thesaurus | Medium |
| `terraphim_types::Thesaurus` | Currently mandatory; failover needs an empty/placeholder path | Low |
| `terraphim_rlm::executor::ExecutionEnvironment` | Mock needed for QueryLoop tests | Low |
| `terraphim_rlm::validator::KnowledgeGraphValidator` | Already wired into executors | Low |

### External Dependencies

| Dependency | Version | Risk | Alternative |
|------------|---------|------|-------------|
| `fff-search` | `0.8.4` | Optional feature only | Make default or auto-enable |

## Risks and Unknowns

### Known Risks

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Making `code-search` default increases compile time / binary size | Medium | Medium | Benchmark before/after; keep it optional if too costly |
| Empty thesaurus breaks `RoleGraph` invariants | Low | High | Add unit test for empty-thesaurus searcher |
| Version bump in `terraphim-clients` needs matching `terraphim_service` publish | Medium | High | Bump workspace version and publish/service dependency together |
| QueryLoop mock tests are fragile | Medium | Low | Keep tests focused on call counts, not output parsing |

### Open Questions

1. Should `code-search` become a default feature so the failover always works, or should failover only work when the feature is enabled? *(Decision needed before design.)*
2. Does the user want `terraphim-grep` to prefer the failover path silently, or emit an `info!` log that KG is unavailable? *(Decision needed.)*
3. Is Issue #2491 already functionally fixed, or is there a remaining strictness/escalation behaviour gap? *(Verify with test spike.)*

### Assumptions Explicitly Stated

| Assumption | Basis | Risk if Wrong | Verified? |
|------------|-------|---------------|-----------|
| `fff-search` is the correct "enhanced grep" backend | It is already integrated in `hybrid_searcher.rs` | We would need a different search fallback | Yes — code inspected |
| `RoleGraph` can be constructed with an empty thesaurus | `RoleGraph::new_sync` signature takes a `Thesaurus`; empty `Thesaurus::new(...)` is a public constructor | Failover path panics at startup | No — needs spike |
| Validation is already wired in QueryLoop | Lines 429 and 474 call `validate_command` before execution | Issue description is still accurate and requires larger refactor | Yes — code inspected |
| `terraphim-clients` and `terraphim-ai` can be released separately | They are separate repos/workspaces | Release process is more complex than assumed | Partially — tags are independent |

### Multiple Interpretations Considered

| Interpretation | Implications | Why Chosen/Rejected |
|----------------|--------------|---------------------|
| A: Add `--no-kg` flag to bypass thesaurus | Simple but shifts burden to user | Rejected — failover should be automatic |
| B: Make thesaurus optional and fall back to fff-search | Matches user request; transparent | Chosen as primary approach |
| C: Bundle a minimal default thesaurus | Increases binary size and maintenance | Rejected — not "enhanced grep" semantics |

## Research Findings

### Key Insights

1. The `terraphim-grep` "failover" is mostly a CLI-layer change: the underlying `HybridSearcher::search_code` already uses `fff-search`; only `main.rs` and `HybridSearcher::new` need to tolerate missing KG.
2. `TerraphimGrep::search` already degrades gracefully when no LLM client is configured (`search_with_rlm_fallback` returns `SearchOnly`), so the LLM fallback path can be reused for no-KG results.
3. `terraphim_rlm` validation ordering exists in source but is untested; the safest fix is a focused unit test plus a strictness-policy review.
4. Release readiness requires cross-repo version coordination and committing the `Cargo.lock` change that added `proptest`.

### Relevant Prior Art

- `terraphim_grep/src/lib.rs` already has `search_without_llm_degrades_to_search_only` test showing the desired degradation pattern.
- `terraphim_middleware/src/indexer/fff.rs` shows how `fff-search` is used elsewhere in the ecosystem.

### Technical Spikes Needed

| Spike | Purpose | Estimated Effort |
|-------|---------|------------------|
| Construct `HybridSearcher` with empty `Thesaurus` | Verify `RoleGraph` tolerates no indexed terms | 30 min |
| Build `terraphim_grep` with `code-search` as default feature | Measure compile-time and binary-size impact | 30 min |
| Mock executor for QueryLoop tests | Confirm test pattern compiles and runs | 1 hour |

## Recommendations

### Proceed/No-Proceed

**Proceed** with a bounded scope covering:

1. `terraphim-grep` no-KG failover (terraphim-clients).
2. `terraphim_rlm` QueryLoop validation-order unit test (terraphim-ai).
3. Version alignment, `Cargo.lock` hygiene, and binary installation (both repos).

### Scope Recommendations

- Keep the failover purely additive: no changes to existing KG-boosted ranking.
- Do not change `KgStrictness` semantics unless the spike shows a real gap.
- Treat documentation cleanup as a separate "chore" commit, not part of the functional fixes.

### Risk Mitigation Recommendations

- Add a new unit test for empty-thesaurus `HybridSearcher` before changing `main.rs`.
- Run `cargo build --release -p terraphim_grep` before and after the feature change to check binary size.
- Verify `cargo test -p terraphim_rlm` after adding the mock test.

## Next Steps

If approved:

1. Conduct the three technical spikes above.
2. Move to Phase 2 (disciplined design) and produce an implementation plan.
3. Create Gitea issues for each work stream and link them appropriately.

## Appendix

### Reference Materials

- `terraphim-clients/crates/terraphim_grep/src/main.rs`
- `terraphim-clients/crates/terraphim_grep/src/lib.rs`
- `terraphim-clients/crates/terraphim_grep/src/hybrid_searcher.rs`
- `terraphim-clients/crates/terraphim_grep/Cargo.toml`
- `terraphim-ai/crates/terraphim_rlm/src/query_loop.rs`
- `terraphim-ai/crates/terraphim_rlm/src/executor/trait.rs`
- `terraphim-ai/crates/terraphim_rlm/src/executor/context.rs`
- `terraphim-ai/crates/terraphim_rlm/src/validator.rs`
- `terraphim-ai/crates/terraphim_rlm/src/config.rs`
- Gitea issue #2491 (`terraphim/terraphim-ai`)
