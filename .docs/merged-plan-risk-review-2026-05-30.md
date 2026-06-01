# Merged Plan Risk Review: 2026-05-30

**Status**: Review complete  
**Reviewed at**: 2026-05-30 22:07 BST  
**Scope**: `.docs/merged-plan-2026-05-30.md` and all 20 open Gitea PRs listed in the plan  
**Method**: Structured PR review batches using code-review and structural-risk criteria

## Executive Assessment

The merge plan correctly identifies that the PR backlog should be reduced in small, verified batches. The main risk is that it over-trusts Gitea's `mergeable=true` flag and treats several PRs as low risk when they are empty, stale, overlapping, or broader than their titles imply.

The safest path is not a bulk merge. Merge only no-op-free, focused, verified PRs first; close empty or superseded PRs; rebase stale branches; and require targeted tests plus remote convergence after each small batch.

## Plan-Level Risks

| Severity | Risk | Impact | Mitigation |
|----------|------|--------|------------|
| P1 | Mergeability is treated as readiness | PRs can compile-fail, regress behaviour, or add unrelated changes despite being marked mergeable | Require targeted verification per PR before merge |
| P1 | Empty PRs are listed as merge candidates | Merging no-op PRs damages issue traceability and may hide missing work | Close or refresh #1865 and #1849 |
| P1 | Stale branches need semantic rebase, not just `cargo fmt` | Rebase mistakes can drop newer main functionality | Rebase and diff against current `main` before merge |
| P1 | Overlapping PRs modify the same files | CHANGELOG/test/config conflicts can produce accidental duplicate or contradictory changes | Merge in dependency order and re-review after each merge |
| P2 | Batch rollback plan is too coarse | Reverting after multi-PR batches becomes hard | Merge PRs individually or in very small homogeneous batches |
| P2 | Verification criteria prove commits landed, not behaviour | `git log` does not show quality | Require tests, clippy/doc checks, and status checks |
| P2 | Tantivy remains in future-work references | Planning ambiguity after descoping #2288 | Update plan to reference the ADR and hybrid FFF direction |

## Recommended Merge Order

1. **Close or refresh no-op PRs**: #1865, #1849.
2. **Merge truly low-risk focused PRs after checks**: #1852, #1789, #1787, #1891.
3. **Resolve overlapping documentation/CHANGELOG PRs**: #1860, #1867, #1836.
4. **Handle test/CI changes carefully**: #1858 before #1828 only if scope is accepted; #1828 before #1832 because both touch nextest config.
5. **Fix and re-review broad or stale ADF/skills PRs**: #1786 before #1788, then #1800.
6. **Rebase security/debug redaction PRs**: #1791 and #1767.
7. **Do not merge broad KG/RLM PR #1850 until the P1 persistence issue is fixed.**

## Per-PR Structured Review Summary

| PR | Recommendation | Confidence | Blocking Findings | Required Action |
|----|----------------|------------|-------------------|-----------------|
| #1898 | Do not merge yet | 3/5 | P2 scope mismatch; not mergeable | Resolve mergeability; verify actual `.terraphim/skills/` search path behaviour |
| #1891 | Merge after checks | 5/5 | None | Run rustdoc and `cargo check -p terraphim_types` |
| #1867 | Rebase or supersede | 4/5 | Not mergeable; overlaps #1891 | Rebase after #1891 or close as superseded |
| #1865 | Close or refresh | 5/5 for no-op | Empty diff | Close if already on `main`, otherwise recreate with actual diff |
| #1860 | Do not merge yet | 2/5 | P1 Cargo.lock not updated for new `jsonschema` dependency | Remove unrelated dependency or update lockfile and verify `--locked` |
| #1859 | Rebase/re-scope | 3/5 | P2 title/scope mismatch; not mergeable | Reduce to formatting-only or explicitly review test/CHANGELOG changes |
| #1858 | Merge after minor fix | 4/5 | P2 misleading integration-test opt-in instructions | Fix env-var claim or use runtime gating; run targeted tests |
| #1852 | Merge after checks | 5/5 | None | Run `cargo metadata` and workspace check |
| #1850 | Do not merge | 3/5 | P1 KG persistence failures are swallowed | Propagate or report persistence errors; rebase; rerun tests |
| #1849 | Close or refresh | 5/5 for no-op | Empty diff | Close as superseded or push actual README/Cargo metadata diff |
| #1836 | Rebase then merge | 4/5 | Not mergeable | Rebase and run rustdoc/test for `terraphim_merge_coordinator` |
| #1832 | Rebase then merge | 4/5 | Not mergeable; overlaps #1828 | Merge/rebase after #1828 or reconcile `.config/nextest.toml` |
| #1828 | Merge with caution | 4/5 | P2 local hooks assume `cargo-nextest` is installed | Add hook guard/fallback; verify nextest CI path |
| #1800 | Do not merge | 3/5 | P1 incomplete Git environment isolation | Clear full Git local env set; add poisoned-env tests |
| #1791 | Rebase then merge | 4/5 | Not mergeable; excluded crate verification gap | Rebase; verify `terraphim_github_runner_server` tests where buildable |
| #1789 | Merge after checks | 5/5 | None | Run shared-learning tests |
| #1788 | Do not merge | 2/5 | P1 token passed via `curl` argv; inherits #1786 collision bug | Replace argv token handling; wait for #1786 fix; split artefacts |
| #1787 | Merge after audit checks | 4/5 | None found, but suppresses real advisory | Run `cargo audit`/`cargo deny`; track follow-up removal |
| #1786 | Do not merge | 3/5 | P1 active agents still keyed only by name | Key runtime active agents by project-scoped key; add collision test |
| #1767 | Rebase then merge | 3/5 | P1 stale branch can regress newer spawner behaviour | Rebase preserving current `pi-rust`/`pi` paths; run targeted tests |

## Highest Priority Blockers

### P1: #1860 Cargo.lock mismatch

`crates/terraphim_config/Cargo.toml` adds `jsonschema = "0.29"` without a matching `Cargo.lock` update. Any `--locked` CI path can fail before compilation. This dependency also appears unrelated to the CHANGELOG-only purpose.

### P1: #1850 silent KG persistence failure

`crates/terraphim_grep/src/kg_curation.rs` can return success even when learned concepts are not written. This creates silent data loss for the KG learning loop.

### P1: #1800 incomplete Git environment isolation

The PR clears only a subset of Git environment variables. Poisoned variables such as `GIT_COMMON_DIR`, `GIT_OBJECT_DIRECTORY`, and `GIT_ALTERNATE_OBJECT_DIRECTORIES` can still redirect repository operations.

### P1: #1788 token exposure through process argv

The Gitea token is passed in a `curl` `Authorization` header argument. Command-line arguments can be visible through process inspection on shared systems.

### P1: #1786 active-agent collision

The new registry is project-scoped, but runtime active-agent tracking remains keyed only by agent name. Same-named agents across projects can overwrite each other.

### P1: #1767 stale branch regression risk

The redaction change is valid, but the stale branch can drop newer spawner support if conflict resolution takes the PR-side file wholesale.

## Merge Plan Changes Recommended

1. Replace the Phase A list with a verified list: #1852, #1789, #1787, #1891 only.
2. Move #1860 out of Phase A until the Cargo.lock/dependency issue is resolved.
3. Remove #1865 and #1849 from merge candidates; close or refresh them.
4. Move #1858 into a small test-scope batch after fixing the misleading ignored-test instructions.
5. Treat #1828 as a CI migration requiring hook fallback and remote-worker verification.
6. Replace the Phase B “apply fmt” step with “semantic rebase plus targeted review”.
7. Update Phase D to state: Tantivy session search is descoped by ADR; future search work should enhance hybrid FFF-based search.

## Evidence Limitations

The reviews inspected PR metadata, branch diffs, and key changed files. Tests and CI were not executed as part of this review pass. Some very large PRs, especially #1788, were sampled for substantive Rust/code paths rather than every generated or operational artefact.
