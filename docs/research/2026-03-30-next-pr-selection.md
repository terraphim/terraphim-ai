# Research Document: Next PR Selection -- Post 5-PR Merge Sprint

**Status**: Draft
**Author**: Planning Orchestrator
**Date**: 2026-03-30
**Reviewers**: Alex

## Executive Summary

After merging 5 PRs (#732-#736) into upstream/main, we need to select the best next task. Analysis of 3 conflicting PRs, 8 Gitea ready issues, and 7 code improvement issues reveals that **Gitea #138 (Wire up read_url for remote thesaurus loading)** is the strongest candidate. It is a 1-function fix in a single file, directly builds on our just-merged #733 URL validation fix, and unblocks the entire remote thesaurus loading feature which is currently dead code.

## Essential Questions Check

| Question | Answer | Evidence |
|----------|--------|----------|
| Energizing? | Yes | Completes unfinished work from #733; dead code in the codebase is a clear signal |
| Leverages strengths? | Yes | We just fixed the URL validation in the same file -- deep context is fresh |
| Meets real need? | Yes | Remote thesaurus loading is a documented feature that currently silently fails |

**Proceed**: Yes (3/3 YES)

## Candidate Analysis

### Category A: Conflicting PRs Requiring Rebase

**PR #731 (ADF agent spawning, 10 files, +991/-31)**
- Scope: Large, touches spawner, config, CLI
- Conflict risk: Medium -- depends on what changed in spawner recently
- Effort: 2-4 hours for rebase + testing
- Value: Important but not urgent; agent spawning is experimental

**PR #726 (terraphim-agent fixes, 17 files, +464/-167)**
- Scope: Medium-large, touches automata, config, middleware
- Conflict risk: HIGH -- directly conflicts with #733 (AutomataPath::from_remote) and #734 (LazyLock)
- Effort: 3-5 hours for rebase + conflict resolution
- Value: High (fixes real user-facing bugs) but conflict resolution is risky

**PR #426 (RLM orchestration, 106 files, +12544/-200)**
- Scope: Massive, experimental
- Verdict: ELIMINATED -- too large, too risky for single session

### Category B: Well-Scoped Gitea Issues

**#138: Wire up read_url for remote thesaurus loading**
- Location: `crates/terraphim_automata/src/lib.rs` lines 346-400
- Problem: `read_url()` function exists (lines 348-378) but is `#[allow(dead_code)]`. The `Remote` match arm (lines 391-395) returns an error instead of calling `read_url()`.
- Fix: Change `Remote` arm to call `read_url(url.clone()).await?` instead of returning error
- Tests: Existing `test_load_thesaurus_from_url` test (line 449, currently `#[ignore]`) validates this
- Effort: 30 minutes
- Risk: Very low -- the function already exists and was tested before being broken
- Value: HIGH -- unblocks remote thesaurus loading, the primary use case for `AutomataPath::Remote`
- Dependency: Builds directly on #733 (URL validation fix we just merged)

**#143: Remove unnecessary memoization from magic_pair/magic_unpair**
- Location: `crates/terraphim_rolegraph/src/lib.rs` lines 1268-1287
- Problem: `magic_pair(x,y)` and `magic_unpair(z)` use `#[memoize(CustomHasher: ahash::AHashMap)]` but the functions are pure arithmetic (2-3 operations). Memoization adds hash table overhead that exceeds computation cost.
- Fix: Remove `#[memoize]` attributes, remove `memoize = "0.5.1"` from Cargo.toml, remove `use memoize::memoize;` import
- Callers: 26 occurrences across 5 files (but all call the same 2 functions)
- Effort: 30 minutes
- Risk: Very low -- pure functions, removing cache can only make behavior more predictable
- Value: Medium -- removes unnecessary dependency, simplifies code, may improve performance for small inputs
- Bonus: Can be combined with #138 in same PR

**#142: Remove redundant async wrappers in RoleGraph**
- Location: `crates/terraphim_rolegraph/src/lib.rs`
- Problem: `RoleGraph::new()` (line 287) and `from_serializable()` (line 360) are `async fn` that just call sync versions. `RoleGraphSync` methods (lines 1210-1241) are legitimately async (they acquire locks).
- Fix: Could deprecate async wrappers on RoleGraph, but callers use them widely
- Effort: 1-2 hours (need to update all callers across crates)
- Risk: Medium -- changing public API signatures ripples across workspace
- Value: Low -- the wrapper is harmless (zero-cost when optimized)
- Verdict: Defer -- high effort-to-value ratio, breaking API change

**#141: Improve ID generation for Concept uniqueness per KG**
- Effort: 2-4 hours (design needed)
- Risk: Medium-high -- changes fundamental data model
- Verdict: Defer -- needs its own research phase

**#140: Reduce cloning in RoleGraph hot paths**
- Effort: 2-3 hours
- Risk: Medium -- needs profiling to identify actual hot paths
- Verdict: Defer -- needs benchmarking first

**#137: Make TriggerIndex threshold and stopwords configurable**
- Location: `crates/terraphim_rolegraph/src/lib.rs` lines 52-211
- Problem: Threshold is hardcoded to 0.3, stopwords hardcoded in `is_stopword()`
- Fix: Accept threshold and optional stopword set in constructor, thread through from config
- Effort: 1-2 hours
- Risk: Low -- additive change, backward compatible with defaults
- Value: Medium -- enables tuning for different domains
- Verdict: Good second-tier candidate

**#135: Clean up dead code and deprecated functions**
- Effort: 1-2 hours (audit + removal)
- Risk: Low but tedious
- Verdict: Could combine with #138 (read_url IS the dead code issue)

### Category C: High-Priority Gitea Issues (Too Large for One Session)

**#116, #144, #45, #100, #57**: All require multi-session design and implementation. ELIMINATED from consideration.

## Recommendation: Combined #138 + #143

### Rationale

1. **#138 is the natural next step** after #733. We fixed `from_remote()` URL validation; now we need to actually wire up the remote loading that was broken. This is a 1-line fix in a function that already exists.

2. **#143 is a clean dependency removal**. The `memoize` crate adds a runtime hash map cache for 2 arithmetic operations. Removing it simplifies the dependency tree and makes the code more predictable.

3. **Both changes are orthogonal** -- they touch different crates (`terraphim_automata` vs `terraphim_rolegraph`), so no merge conflicts between them.

4. **Combined effort: ~1 hour**. Both are well-scoped with existing test coverage.

5. **Combined value: HIGH**. One unblocks a documented feature. The other removes unnecessary complexity.

## Constraints

### Technical Constraints
- #138 requires `remote-loading` feature to be enabled for testing
- #138's live test (`test_load_thesaurus_from_url`) requires network access and is `#[ignore]`
- #143's `memoize` removal needs verification that no other code in the workspace uses the `memoize` crate

### Vital Few (Max 3)

| Constraint | Why Vital | Evidence |
|------------|-----------|----------|
| Must not break WASM build | `terraphim_automata` has WASM target; remote-loading is feature-gated | Feature gate already handles this |
| Must keep backward compatibility | `magic_pair`/`magic_unpair` are public API | Removing memoize doesn't change signatures |
| Must have test coverage | Both changes touch core functionality | Existing tests cover both paths |

### Eliminated from Scope

| Eliminated Item | Why Eliminated |
|-----------------|----------------|
| PR #426 (RLM) | 12K+ lines, experimental, too large |
| PR #726 rebase | High conflict with #733/#734, 3-5 hours |
| PR #731 rebase | Medium scope but agent spawning is not urgent |
| #142 async wrappers | Breaking API change, low value |
| #141 ID generation | Needs design phase, changes data model |
| #140 reduce cloning | Needs profiling first |
| #116/#144/#45/#100/#57 | Multi-session features, not one-session tasks |

## Risks and Unknowns

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Remote thesaurus URL in test is stale | Low | Low | Test is `#[ignore]`, can verify manually |
| `memoize` crate used elsewhere in workspace | Low | Low | Grep confirms only in `terraphim_rolegraph` |
| Removing memoize regresses performance for repeated calls | Very Low | Low | Pure math is faster than hash lookup for u64 |

### Assumptions

| Assumption | Basis | Risk if Wrong | Verified? |
|------------|-------|---------------|-----------|
| `read_url` function body is correct | It was written intentionally, just not wired up | Would need debugging | Partially (code reads correctly) |
| `reqwest` is available when `remote-loading` feature is enabled | Feature gate in Cargo.toml | Build failure | Yes (feature exists in Cargo.toml) |
| No other workspace crate depends on `memoize` | Grep search | Build failure | Need to verify |

## Code Locations

| Component | Location | Purpose |
|-----------|----------|---------|
| `load_thesaurus` (remote) | `crates/terraphim_automata/src/lib.rs:346-400` | Broken Remote arm |
| `read_url` (dead code) | `crates/terraphim_automata/src/lib.rs:348-378` | HTTP fetch for thesaurus |
| `magic_pair` | `crates/terraphim_rolegraph/src/lib.rs:1268-1271` | Memoized pairing function |
| `magic_unpair` | `crates/terraphim_rolegraph/src/lib.rs:1282-1287` | Memoized unpairing function |
| `memoize` import | `crates/terraphim_rolegraph/src/lib.rs:3` | Dependency import |
| `memoize` dep | `crates/terraphim_rolegraph/Cargo.toml:23` | Crate dependency |

## Next Steps

If approved, proceed to Phase 2 (Design) for the combined #138 + #143 implementation plan.
