# Research Document: terraphim_lsp Foundation (Gitea #2668)

**Status**: Draft
**Author**: AI Agent
**Date**: 2026-06-13
**Issue**: terraphim/terraphim-ai#2668
**Epic**: terraphim/terraphim-ai#2667

## Executive Summary

The `terraphim_lsp` crate currently exists in the `task/adf-flow-fix-phase1-automerge` branch as an excluded, broken workspace member. It has an orphaned `Cargo.lock`, edition 2021 (behind the workspace's 2024), no dependencies, and a placeholder `lib.rs`. This blocks the 11-step components-functionality epic (#2667) because Steps 2 and 3 depend on a compilable LSP foundation. The immediate goal is to make `cargo check -p terraphim_lsp` succeed from the workspace root with minimal, KG-focused dependencies.

## Essential Questions Check

| Question | Answer | Evidence |
|----------|--------|----------|
| Energizing? | Yes | Unblocks LSP server and KG analysis engine work |
| Leverages strengths? | Yes | Reuses existing terraphim_automata/terraphim_types/terraphim_rolegraph crates already in workspace |
| Meets real need? | Yes | Issue #2668 is P1-high and explicitly asks for a compilable foundation |

**Proceed**: Yes -- 3/3 YES.

## Problem Statement

### Description
`terraphim_lsp` cannot be compiled from the workspace root. The crate is excluded in root `Cargo.toml`, its own `Cargo.toml` declares edition 2021 and declares no dependencies, and `src/lib.rs` is only a doc-comment placeholder. An orphaned `Cargo.lock` shadows the workspace lockfile and will conflict with workspace resolution.

### Impact
- Steps 2 and 3 of epic #2667 (KG analysis engine and LSP server) cannot be implemented because the crate does not build.
- `cargo check --workspace` silently skips `terraphim_lsp`, hiding future compilation regressions.
- Editor support for Terraphim KG markdown remains blocked.

### Success Criteria
1. `cargo check -p terraphim_lsp` succeeds from workspace root.
2. `cargo check --workspace` still succeeds for all other crates.
3. `crates/terraphim_lsp/Cargo.lock` is removed.
4. The crate uses workspace edition 2024.
5. The crate declares minimal dependencies: `tower-lsp`, `tokio`, `serde_json`, `terraphim_automata`, `terraphim_types`, `terraphim_rolegraph`.
6. `src/lib.rs` contains working boilerplate (module declarations, no-op LSP server).

## Current State Analysis

### Existing Implementation

The `terraphim_lsp` crate was historically implemented with EDM diagnostics (`terraphim_negative_contribution`) and later revived with `tower-lsp` in commit `c620cffc1`. In the current `task/adf-flow-fix-phase1-automerge` branch it has been stripped to a placeholder and excluded from the workspace.

### Code Locations

| Component | Location | Purpose |
|-----------|----------|---------|
| Workspace manifest | `Cargo.toml` | Excludes `crates/terraphim_lsp` |
| Crate manifest | `crates/terraphim_lsp/Cargo.toml` | Edition 2021, no dependencies |
| Orphaned lockfile | `crates/terraphim_lsp/Cargo.lock` | Conflicts with workspace `Cargo.lock` |
| Library root | `crates/terraphim_lsp/src/lib.rs` | Placeholder only |

### Data Flow

Not applicable for foundation step; future flow will be:

```
Editor → LSP request → tower-lsp → KgAnalysis (Step 2) → LSP response
```

### Integration Points

1. **Workspace membership**: Must be removed from `exclude` in root `Cargo.toml`.
2. **Registry/local core crates**: `terraphim_automata`, `terraphim_types`, `terraphim_rolegraph` are present as workspace members in this branch and should be referenced via path dependencies.
3. **tower-lsp ecosystem**: `tower-lsp = "0.20"` is the target version based on prior implementation.

## Constraints

### Technical Constraints
- Workspace edition is 2024; crate must match.
- `crates/*` glob in root `Cargo.toml` will auto-include `terraphim_lsp` once removed from `exclude`.
- The workspace uses resolver "2".
- `tower-lsp` 0.20 depends on `lsp-types` and `tower-service`; these will be pulled transitively.

### Business Constraints
- Estimated effort: 1 hour.
- Must not break existing workspace compilation.
- Must be reviewable as a single, focused PR.

### Non-Functional Requirements
| Requirement | Target | Current |
|-------------|--------|---------|
| Build time | < 30s incremental | N/A (does not build) |
| Workspace check | Must pass | Fails silently due to exclusion |

## Vital Few (Essentialism)

### Essential Constraints (Max 3)

| Constraint | Why It's Vital | Evidence |
|------------|----------------|----------|
| Remove orphaned `Cargo.lock` | Cargo will otherwise use it instead of the workspace lockfile, causing resolution conflicts | Issue #2668 explicitly lists deletion |
| Align edition to workspace 2024 | Mismatch causes edition-related compile errors and inconsistent tooling | Root `Cargo.toml` declares `edition = "2024"` |
| Keep dependencies minimal | Prevents scope creep; only adds what Steps 2 and 3 need | Issue #2668 dependency list |

### Eliminated from Scope

| Eliminated Item | Why Eliminated |
|-----------------|----------------|
| Real LSP handlers (hover/completion/diagnostics) | Step 3 responsibility; foundation only needs boilerplate |
| KG analysis engine implementation | Step 2 responsibility |
| EDM diagnostics (historical implementation) | Superseded by KG-focused design in epic #2667 |
| CI workflow changes | Step 11 responsibility |

## Dependencies

### Internal Dependencies
| Dependency | Impact | Risk |
|------------|--------|------|
| `terraphim_automata` | Aho-Corasick term matching for Step 2 | Low -- workspace member |
| `terraphim_types` | Shared types (`Thesaurus`, `ReviewFinding`, etc.) | Low -- workspace member |
| `terraphim_rolegraph` | KG connectivity checks for Step 3 | Low -- workspace member |

### External Dependencies
| Dependency | Version | Risk | Alternative |
|------------|---------|------|-------------|
| `tower-lsp` | 0.20 | Low | None -- standard LSP framework in Rust |
| `tokio` | workspace | Low | None -- already in workspace |
| `serde_json` | workspace | Low | None -- already in workspace |

## Risks and Unknowns

### Known Risks
| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Adding `terraphim_lsp` back to workspace increases Cargo.lock churn | Medium | Low | Run `cargo check` and commit resulting lockfile changes only for this crate |
| `tower-lsp` 0.20 API differences from historical 0.20 usage | Low | Medium | Use only no-op boilerplate; no handler logic yet |
| Other workspace crates fail after un-exclusion | Low | High | Run `cargo check --workspace` before and after |

### Open Questions
1. Should `terraphim_lsp` use path dependencies for core crates, or registry dependencies? -- Answer: path dependencies in this branch because the crates are workspace members.
2. Should a binary target (`src/bin/terraphim-lsp.rs`) be added in Step 1? -- Answer: no, Step 3 will add the server binary.

### Assumptions Explicitly Stated

| Assumption | Basis | Risk if Wrong | Verified? |
|------------|-------|---------------|-----------|
| `task/adf-flow-fix-phase1-automerge` is the correct base branch | User explicitly stated plans are in this branch | Would need to rebase onto main | Yes -- verified by user |
| Core crates (`terraphim_automata`, `terraphim_types`, `terraphim_rolegraph`) compile in this branch | They are workspace members and `cargo check --workspace` will validate | Step 1 blocked until they build | Yes -- inspect crate directories |
| `tower-lsp` 0.20 is compatible with workspace tokio version | Historical usage in commit `c620cffc1` | Would need to adjust version | Partially -- historical evidence |

### Multiple Interpretations Considered

| Interpretation | Implications | Why Chosen/Rejected |
|----------------|--------------|---------------------|
| Implement full LSP server in Step 1 | Would blur scope boundaries and delay Step 2/3 | Rejected -- foundation only |
| Keep crate excluded and build standalone | Would not satisfy "compilable from workspace root" | Rejected -- issue requires workspace inclusion |
| Use registry dependencies for core crates | Would work if crates were not local members | Rejected -- path deps are simpler in this branch |

## Research Findings

### Key Insights

1. The automerge branch has restored the polyrepo-split core crates (`terraphim_automata`, `terraphim_types`, `terraphim_rolegraph`) as local workspace members, making path dependencies viable.
2. The historical `terraphim_lsp` implementation (commit `c620cffc1`) used `tower-lsp` 0.20 with EDM diagnostics; the new design should be KG-focused and intentionally minimal.
3. `terraphim_automata::find_matches(text, thesaurus, true)` is the established API for Aho-Corasick term matching and will be used in Step 2.
4. `KnowledgeGraphValidator` in `terraphim_rlm` is not a suitable dependency for `terraphim_lsp` because it would create a dependency from an integration crate back to an executor crate; direct use of `terraphim_automata` is preferred.

### Relevant Prior Art
- Commit `c620cffc1`: revived `terraphim_lsp` with `tower-lsp` 0.20 and EDM diagnostics.
- Commit `5f246be3c`: original `terraphim_lsp` creation with EDM diagnostics.
- `crates/terraphim_rlm/src/validator.rs`: shows how `terraphim_automata::find_matches` and `Thesaurus` are used together.

### Technical Spikes Needed
| Spike | Purpose | Estimated Effort |
|-------|---------|------------------|
| Verify `tower-lsp` 0.20 compiles with current workspace deps | Confirm no version conflicts | ~15 min |
| Verify no-op `LanguageServer` impl satisfies `tower-lsp` | Confirm boilerplate shape | ~15 min |

## Recommendations

### Proceed/No-Proceed
**Proceed** with un-excluding `terraphim_lsp`, deleting the orphaned `Cargo.lock`, fixing `Cargo.toml`, and adding minimal boilerplate.

### Scope Recommendations
- Strictly limit Step 1 to compilability; no handler logic.
- Reserve `kg_analysis.rs` module declaration in `lib.rs` but leave implementation to Step 2.
- Reserve `server.rs` module declaration but leave implementation to Step 3.

### Risk Mitigation Recommendations
- Run `cargo check -p terraphim_lsp` and `cargo check --workspace` after every file change.
- Commit the `Cargo.lock` delta separately if it becomes large.

## Next Steps

If approved:
1. Remove `crates/terraphim_lsp` from root `Cargo.toml` exclude list.
2. Delete `crates/terraphim_lsp/Cargo.lock`.
3. Update `crates/terraphim_lsp/Cargo.toml` to edition 2024 and add dependencies.
4. Replace `src/lib.rs` with module declarations and no-op LSP server boilerplate.
5. Run verification commands and commit.

## Appendix

### Reference Materials
- Issue #2668: Step 1: terraphim_lsp -- Foundation
- Issue #2667: Epic: Full Functionality Audit and Fixes
- Historical implementation: `git show c620cffc1:crates/terraphim_lsp/`
- `terraphim_automata` usage: `crates/terraphim_rlm/src/validator.rs:400`
