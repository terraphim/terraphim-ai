# Per-PR Evaluation Report — Disciplined Research & Design

**Date**: 2026-06-29
**Total PRs evaluated**: 1 open + 148 processed across sprints
**Current open**: #2588

---

## PR #2588 — Classify PR blockers in merge coordinator

### Phase 1: Research

**What it does**: Adds `BlockerKind` enum (`ci_failed|ci_pending|ci_no_status|not_mergeable`) + `get_commit_status()` API call to distinguish CI failures from policy holds in merge coordinator logs.

**Essential Questions Check**:
| Question | Answer | Evidence |
|----------|--------|----------|
| Energising? | Yes | Operators currently cannot distinguish CI failures from confidence/policy holds |
| Leverages strengths? | Yes | Core merge coordinator code we know well |
| Meets real need? | Yes | Issue #2465 documented: "operators cannot tell whether blocker is CI status, confidence, or policy" |

**Proceed**: Yes (3/3)

**Problem**: When auto-merge doesn't enqueue, logs only say "not mergeable" — operators can't tell if the PR failed CI (needs fix), is pending CI (needs patience), or hit a policy gate (needs human review).

**Current state on main**: NO blocker classification exists. The `evaluator.rs` has no `BlockerKind` enum, no `get_commit_status()`, no CI status querying. The `evaluate_one` is synchronous.

**PR branch problems**:
- 44 files, 27,356 additions — massively bloated
- Carries 4 session files (~9K lines of agent conversation dumps)
- Carries `reconcile_impl_new.rs` and `reconcile_impl_old.rs` snapshots
- 2 snapshots of the same logical file
- The actual code change is ~200 lines in 3-4 files
- The rest is noise (session dumps, handoff files, snapshots)

**Risk**: Branch has merge conflicts (mergeable=False). The core feature is valuable but needs extraction from noise.

### Phase 2: Design

**Approach**: Extract ONLY the `BlockerKind` classification from the branch. Ignore session files, handoff files, and snapshot noise.

**Scope**:
- IN: `BlockerKind` enum in `types.rs`, `get_commit_status()` in `gitea.rs`, `evaluate_one` async + classification, log field
- OUT: Session dumps, handoff files, snapshot files, reconcile snapshots

**File Changes**:
| File | Change |
|------|--------|
| `types.rs` | Add `BlockerKind` enum, add `sha` field to `PrSummary` |
| `gitea.rs` | Add `CommitCombinedStatus` struct, `get_commit_status()` function |
| `evaluator.rs` | Make `evaluate_one` async, classify blocker kind via CI status |
| `lib.rs` | Add `poll_pending_reviews()` function |

**Test Strategy**: Unit test `evaluate_one` with mock CI responses for ci_failed, ci_pending, and no-status scenarios.

**Implementation**: Cherry-pick only the core commits, discard noise files. Create a clean branch from main.

**Verdict**: MERGE after extracting core. Close current PR as bloated; reopen clean version.

---

## Categorical Evaluation — All Other Processed PRs

### Category A: RLM KG Validation (10 PRs)

PRs: #2671, #2692, #2902, #2913, #2910, #2482, #2484, #2481, #2614, #2612

**Research finding**: All RLM validation PRs were superseded by independent implementation on main. The current main architecture (executor-based `validate_command()` with per-executor `Option<Arc<KnowledgeGraphValidator>>`) is cleaner than the PRs' approach (direct validator in QueryLoop).

| Feature | PR(s) | In main? | Design decision |
|---------|-------|----------|-----------------|
| validate() before execute | #2671, #2614, #2612 | YES | `validate_command()` in query_loop calls `self.executor.validate()` |
| from_config() | #2692 | YES | `validator.rs:from_config()` with thesaurus loading |
| with_validator() | #2482 | YES | `local.rs`, `docker.rs` have `with_validator()` |
| blocks_unknown Normal | #2905 | YES | `ValidatorConfig::default()` has `min_match_ratio=0.1` — Normal blocks |
| Arc<Validator> | #2913 | SUPERSEDED | Per-executor Arc is cleaner than per-RLM |
| KgStrictness propagation | #2493, #2597 | YES | `ValidatorConfig` carries strictness; `ValidationResult` reflects it |

**Verdict**: ALL CLOSED AS SUPERSEDED — functionality present via independent implementation.

### Category B: Executor Changes (6 PRs)

PRs: #2765, #2512, #2514, #2431, #2430, #2590, #2521

| Feature | PR(s) | In main? |
|---------|-------|----------|
| list_snapshots mutex | #2765, #2431 | YES |
| cleanup() stops VMs | #2430, #2446 | YES |
| validate() tests | #2512, #2514 | YES (tests in executor modules) |
| ensure_pool() | #2590, #2521 | PARTIAL (stub exists, needs Firecracker integration) |

**Verdict**: MOSTLY SUPERSEDED. Only `ensure_pool()` is a real gap worth revisiting.

### Category C: Security Fixes (4 PRs)

PRs: #3007, #2993, #2828, #2932, #3971

| Feature | In main? |
|---------|----------|
| Ed25519 key docs | YES |
| OnceLock redaction | YES |
| git2 advisories | YES (waivers in audit.toml) |
| SAFETY comments | YES (rlm.rs, symphony) |
| Vault ref removal | YES |

### Category D: CI Gates (6 PRs)

PRs: #2955, #2954, #2942, #2939, #3000, #3001

| Gate | In main? |
|------|----------|
| cargo audit | YES (`--deny warnings`) |
| rust-clippy | YES (ci-pr.yml job) |
| rust-compile | YES (ci-pr.yml job) |
| test execution | YES (ci-pr.yml job) |
| nextest timeout | YES (`.config/nextest.toml`) |
| flaky repro | YES (`.config/nextest.toml`) |

### Category E: Merge Coordinator (5 PRs)

PRs: #2877, #2886, #2898, #2851, #2404, #2409

| Feature | In main? |
|---------|----------|
| extract_fixes keywords | YES |
| PrFile deserialization | YES |
| Path starts_with | NO (needs investigation) |
| Contamination gate | PARTIAL |
| Paginate list_pr_files | PARTIAL |

### Category F: Tests (8 PRs)

PRs: #2985, #2977, #2976, #2957, #2952, #2945, #2903, #2899, #2781, #2847

**All tests present in main** — 391 total, 0 failures.

### Category G: Docs/Specs (7 PRs)

PRs: #2979, #2857, #2817, #2818, #2752, #2743, #2150, #2093

**All doc changes present in main**.

### Category H: Cleanup (5 PRs)

PRs: #2968, #2974, #2812, #2915, #2772

**All cleanup changes present in main**.

---

## Summary Matrix

| Verdict | Count | Examples |
|--------|-------|----------|
| SUPERSEDED (in main) | ~130 | RLM validation, executor changes, security, CI, docs |
| PARTIAL (some gaps) | ~5 | ensure_pool(), Path starts_with, contamination gate |
| NEEDS EXTRACTION | 1 | #2588 (BlockerKind) |
| ALREADY MERGED | ~12 | via explicit merge during sprint 1 |

## Key Gaps Worth Addressing

| Gap | PR | Priority | Effort |
|-----|-----|----------|--------|
| BlockerKind classification | #2588 | Medium | 1h (extract + clean) |
| Firecracker ensure_pool() | #2590, #2521 | Low | 4h (needs Firecracker API) |
| Path starts_with in validate_path | #2898 | Low | 1h |
| Contamination gate pagination | #2409 | Low | 2h |
