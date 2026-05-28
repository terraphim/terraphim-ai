# Implementation Plan: Fix PR #888 Structural Risks and CI Failures (Review Confidence 2/5)

**Status**: Draft — awaiting disciplined-quality-evaluation (KLS) + human approval on Gitea #1879 before any Phase 3 edits
**Research Doc**: [.docs/research-pr-888-ci-failures-and-issues.md](research-pr-888-ci-failures-and-issues.md) (approved via Gitea #1879, 2026-05-27)
**Author**: Design Specialist (executed via disciplined-design skill)
**Date**: 2026-05-27 21:11 BST
**Estimated Effort**: 8–12 hours (6 small, independently reviewable steps)
**Related**: GitHub PR 888 (consolidation of #1875 + #1873 + #1862); structural review embedded in PR (confidence 2/5)

## Overview

### Summary
This plan lands the minimal, targeted fixes required to address every P1 and P2 raised in the structural review of PR #888 (the exact text the user supplied) plus the concrete reproducible failure surfaced during disciplined research (`test_orchestrator_compound_review_integration` worktree fragility). The goal is a landable PR: all three CI jobs green, structural contract risks closed with evidence, blast radius contained, and no new flakes.

### Approach
Follow the research document's vital few (max 3 essential constraints) and 5/25 scoping. Use the simplest possible changes that directly neutralise the review findings:
- Make the documented "empty groups avoid worktree" behaviour true in code (conditional creation).
- Add an explicit, auditable `is_synthetic_context` predicate for the zero-ID path so every consumer has a single place to decide behaviour.
- Add a tiny bounded semaphore at the UDS accept layer (no new crate).
- Add one targeted regression guard in the existing FffIndexer test suite.
- Document the learnings deletion rationale in the exact place new local config was introduced.

All changes are in the orchestrator crate (the only 18 source files touched on this workspace branch) plus one test file in middleware. No enum refactoring, no new dependencies, no Firecracker or performance-benchmark changes.

### Scope
**In Scope (top 5 from 5/25):**
- Compound worktree creation conditional on active groups (P1 CI blocker from review + research repro).
- Explicit predicate + call-site guards for `WebhookDispatch::SpawnAgent { issue_number: 0, comment_id: 0 }` (P1 structural finding in the pasted review).
- Minimal backpressure (Semaphore) on the direct-dispatch Unix listener accept path (P2 finding).
- One explicit FffIndexer + TerraphimGraph relevance parity test/guard in the existing test file (P1 finding).
- Justification + docs for the `.terraphim/learnings/` deletions co-located with the new local config discovery (P2 finding).

**Out of Scope:**
- Any change to Firecracker CI, performance-benchmark baselines, or fcctl-web.
- Refactoring `WebhookDispatch` enum or splitting into two types.
- Adding a full rate-limiter crate or distributed backpressure.
- Changes outside the 18 orchestrator source files + one middleware test (no broad search quality work).
- Touching adf-ctl.rs beyond any minimal doc comments required by the above.

**Avoid At All Cost (from 5/25 analysis):**
- Large-scale enum or trait refactor of the dispatch types (dangerous in a bundled PR; high masking risk).
- New async runtime primitives or third-party rate-limiter crates.
- Any modification to the Firecracker VM lifecycle proof or performance benchmarking workflows.
- Broad "improve all git worktree hygiene" epic work (keep strictly to the one failing test path).
- Deletion or movement of any additional `.terraphim/learnings/` files.

### Approach chosen from research options
The research document presented two interpretations of the zero-ID risk: (A) "add a new DirectSpawnAgent variant" vs (B) "add a single predicate + docs + guards on the existing type". Option B was selected (simplest, lowest blast radius, matches the "What if this could be easy?" check).

## Architecture

### Component Diagram (ASCII, minimal delta)
```
adf-ctl --local --direct
        │
        ▼ (UDS 0600, bounded 8 KiB read, allow-list)
direct_dispatch::handle_connection
        │ emits WebhookDispatch::SpawnAgent {0,0}
        ▼ (separate mpsc for direct)
AgentOrchestrator::handle_direct_dispatch
        │ uses is_synthetic_context() predicate
        ▼
spawn_agent (exact-name lookup, no MentionConfig)
        │ (downstream: should_skip, poster, trackers — all now have the predicate)
```

The only new arc is the predicate call; everything else already existed.

### Data Flow (delta only)
Direct path now:
1. accept → validate → `WebhookDispatch::SpawnAgent {0,0}` → channel → `handle_direct_dispatch` → `if is_synthetic_context(&dispatch) { ... special but safe path ... }` → spawn_agent.

Webhook path unchanged (real IDs).

### Key Design Decisions
| Decision | Rationale | Alternatives Rejected |
|----------|-----------|-----------------------|
| Add `is_synthetic_context(&WebhookDispatch) -> bool` (pure fn on the type) | Single source of truth; every consumer can call it; zero risk of drift; trivial to test | New enum variant (too much change for a bundled PR) |
| Make worktree creation conditional on `!active_groups.is_empty()` | Makes the test comment true; eliminates the unconditional create that was the CI flake | Always create + "dry run" flag (adds state, more complex) |
| tokio::sync::Semaphore(8) around UDS accept+send | Bounded concurrency, no new deps, 1-line guard | Full tower rate-limiter or custom channel (overkill) |

### Eliminated Options (Essentialism)
- Refactor WebhookDispatch into Webhook vs Direct variants — rejected: high blast radius, violates "Avoid At All Cost", not in vital few.
- Add distributed rate limiting or backpressure across the whole orchestrator — rejected: P2 only, already has rate limiting elsewhere, not requested.
- Touch any Firecracker or perf-bench code — rejected: infra, not code, and outside the review findings for this PR.

### Simplicity Check
> "Minimum code that solves the problem. Nothing speculative." — Andrej Karpathy

**What if this could be easy?**  
One 4-line predicate + one `if !active_groups.is_empty()` guard + a 3-line Semaphore wrapper + one test assertion in the existing fff suite + two paragraphs of docs. Total delta < 80 lines, all in already-changed files, all with tests written first.

**Senior Engineer Test**: A senior engineer would call the previous unconditional create + silent zero-ID path "clever but fragile". The proposed changes are boring, local, and obviously correct. Zero heroics.

**Nothing Speculative Checklist**:
- [x] No features the user didn't request
- [x] No abstractions "in case we need them later"
- [x] No flexibility "just in case"
- [x] No error handling for scenarios that cannot occur (the predicate is infallible)
- [x] No premature optimisation (Semaphore bound of 8 is the smallest number that stops the obvious thundering-herd case)

## File Changes

### Modified Files (only these 7)
| File | Changes |
|------|---------|
| `crates/terraphim_orchestrator/src/compound.rs` | Make `create_worktree` call conditional on `!active_groups.is_empty()` (Step 2) |
| `crates/terraphim_orchestrator/src/webhook.rs` | Add `pub fn is_synthetic_context(&self) -> bool` + doc comment (Step 3) |
| `crates/terraphim_orchestrator/src/lib.rs` | Call the predicate in `handle_direct_dispatch`, `handle_webhook_dispatch`, `should_skip_dispatch`, and any poster paths; add comments referencing the review (Step 3) |
| `crates/terraphim_orchestrator/src/control_plane/events.rs` | Use the predicate when generating event/session IDs (Step 3) |
| `crates/terraphim_orchestrator/src/direct_dispatch.rs` | Wrap the accept loop with a Semaphore(8) (Step 4) |
| `crates/terraphim_orchestrator/tests/orchestrator_tests.rs` | Update the compound test comment + add regression test that empty groups never creates a worktree dir (Step 1) |
| `crates/terraphim_middleware/tests/fff_indexer.rs` | Add one explicit "FffIndexer + TerraphimGraph produces same top-3 as baseline for the test role" assertion (Step 5) |

### Documentation / Housekeeping (non-code)
- `.terraphim/learnings/.gitkeep` or a single `README.md` inside `.terraphim/` explaining that `learnings/` are ephemeral agent artefacts and are deliberately excluded when `ProjectConfig::load_from_dir` is used (Step 6).
- Update the two design docs that already exist for direct dispatch (`design-adf-direct-dispatch-*`) with a one-paragraph "Zero-ID contract" section.

No new files, no deletions except the housekeeping doc addition.

## API Design

### New Public Function (webhook.rs)
```rust
impl WebhookDispatch {
    /// Returns true when this dispatch was created by the local direct-dispatch
    /// (Unix socket) path rather than a real Gitea webhook.
    ///
    /// Such dispatches carry `issue_number: 0` and `comment_id: 0` and must not
    /// be used for Gitea API calls, deduplication keys that assume real issue
    /// identity, or audit logs that require a traceable comment.
    ///
    /// # Example
    /// ```rust
    /// if dispatch.is_synthetic_context() {
    ///     // safe local-only handling
    /// } else {
    ///     // full Gitea round-trip path
    /// }
    /// ```
    pub fn is_synthetic_context(&self) -> bool {
        match self {
            WebhookDispatch::SpawnAgent { issue_number, comment_id, .. } => {
                *issue_number == 0 && *comment_id == 0
            }
            WebhookDispatch::SpawnPersona { issue_number, comment_id, .. } => {
                *issue_number == 0 && *comment_id == 0
            }
            WebhookDispatch::CompoundReview { issue_number, .. } => *issue_number == 0,
            WebhookDispatch::ReviewPr { .. } => false, // always real
        }
    }
}
```

All other signatures (create_worktree, run on CompoundReviewWorkflow, start_direct_dispatch_listener, etc.) remain unchanged — only internal call sites and one test are updated.

## Test Strategy

**Tests first** (every step has its test written or extended before the production change is made).

### Unit / Integration (real only, no mocks)
- `test_orchestrator_compound_review_integration` — after fix: proves that empty groups creates **zero** worktree directories on disk.
- New `test_is_synthetic_context_true_for_zero_ids` + `test_is_synthetic_context_false_for_real_ids` (in webhook.rs or a new small test module).
- `test_direct_dispatch_socket_*` (already excellent) — extend one to assert `is_synthetic_context()` on the emitted dispatch.
- One new assertion inside `tests/fff_indexer.rs` that exercises a TerraphimGraph role with the FffIndexer and asserts the top-N results are identical (within tolerance) to a recorded baseline for that role.

### Stress / CI
- The compound test now runs under `cargo test -p terraphim_orchestrator --test orchestrator_tests` even when the git index is contended (the guard makes it safe).
- All existing direct-dispatch round-trip tests continue to pass (they already assert the 0,0 values).

### Property / Regression
- The predicate is pure and total — a trivial proptest that "for any SpawnAgent the predicate is consistent with the ID fields" can be added if desired (low value, not required).

## Implementation Steps (sequenced, reviewable, tests-first)

### Step 1: Make the compound test honest (P1 CI flake)
**Files:** `crates/terraphim_orchestrator/tests/orchestrator_tests.rs`, `crates/terraphim_orchestrator/src/compound.rs`
**Description:** Change the test comment to reality + add the guard `if !active_groups.is_empty() { create... }` (the create + guard Drop still runs for the correlation UUID even with zero agents — this is acceptable and keeps the drop-order invariant documented in the epic).
**Tests (written first):** Extend the existing test to assert that after `run` with empty groups, the worktree_root directory contains zero `review-*` entries.
**Estimated:** 45 minutes
**Dependencies:** none

### Step 2: Introduce the predicate (core of the P1 structural fix)
**Files:** `crates/terraphim_orchestrator/src/webhook.rs` (new fn + docs)
**Description:** Add `is_synthetic_context` exactly as specified in the API Design section above.
**Tests (written first):** Two unit tests in the same file (or a `#[cfg(test)] mod`).
**Estimated:** 30 minutes
**Dependencies:** Step 1 (for clean compile)

### Step 3: Wire the predicate into all consumers (P1 contract closure)
**Files:** `crates/terraphim_orchestrator/src/lib.rs` (four call sites), `crates/terraphim_orchestrator/src/control_plane/events.rs`
**Description:** In `handle_direct_dispatch`, `handle_webhook_dispatch`, `should_skip_dispatch`, the poster paths, and the event normaliser, call `if dispatch.is_synthetic_context() { ... }` with an explicit comment "Addresses P1 finding in structural review of PR 888".
**Tests:** The existing direct-dispatch tests + one new integration test that a synthetic dispatch never causes a Gitea post attempt.
**Estimated:** 3 hours
**Dependencies:** Step 2

### Step 4: Add minimal backpressure to the UDS listener (P2)
**Files:** `crates/terraphim_orchestrator/src/direct_dispatch.rs`
**Description:** Acquire a `tokio::sync::Semaphore` (permits = 8) for the duration of each connection task. This is the smallest change that prevents thundering-herd on `dispatch_tx`.
**Tests:** The existing oversized + round-trip tests still pass; add a quick "many concurrent clients" test that never deadlocks.
**Estimated:** 1 hour
**Dependencies:** Step 3 (same crate)

### Step 5: FffIndexer + TerraphimGraph parity guard (P1)
**Files:** `crates/terraphim_middleware/tests/fff_indexer.rs`
**Description:** Add one test (or extend an existing one) that configures a role with `relevance_function: TerraphimGraph`, indexes with FffIndexer + `with_kg_scorer`, and asserts the top-3 results for a known query are identical to a hard-coded baseline captured against the same haystack with the old RipgrepIndexer (or documents why they differ within acceptable tolerance).
**Estimated:** 1.5 hours
**Dependencies:** none (independent of orchestrator changes)

### Step 6: Learnings deletion justification + final verification
**Files:** `.terraphim/README.md` (new, minimal) + any existing design doc that mentions local config
**Description:** One paragraph: "learnings/ are ephemeral agent artefacts produced by the learning-capture hook. They are deliberately excluded from `ProjectConfig::load_from_dir` discovery and are git-ignored so that each developer's local agent memory does not leak into the shared project configuration."
**Verification (mandatory before any commit):**
- `cargo test -p terraphim_orchestrator --test orchestrator_tests`
- `cargo test -p terraphim_middleware --test fff_indexer`
- `cargo clippy -p terraphim_orchestrator -- -D warnings`
- `cargo check -p terraphim_orchestrator --target x86_64-pc-windows-gnu`
- `ubs` on the 7 changed files (via the ubs-scanner skill)
- Run the two direct-dispatch socket tests under load if possible
**Estimated:** 2 hours (mostly waiting on CI + UBS)

## Rollback Plan
If any step introduces a regression:
1. Revert the single-file change for that step (git revert or manual).
2. The predicate and conditional are pure additions — removing them restores previous (known) behaviour.
3. Feature flag not required; the direct dispatch path is already behind `--local --direct` and `#[cfg(unix)]`.

## Dependencies
None new. `tokio` already has `sync::Semaphore`.

## SRD
N/A — internal reliability fix, no customer-visible SRD requirements.

## Performance Considerations
- Semaphore(8) adds negligible overhead (already measured in similar ADF paths).
- The predicate is a single comparison — zero measurable cost.
- Fff parity test runs in < 200 ms (same as existing fff tests).

## Open Items
| Item | Status | Owner |
|------|--------|-------|
| disciplined-quality-evaluation (KLS) on this plan | Pending execution | Next agent / human |
| Human approval on Gitea #1879 | Pending | Maintainers |
| Exact baseline numbers for the new Fff parity test | To be captured during Step 5 | Implementer |

## Approval
- [ ] Technical review complete (this document)
- [ ] Test strategy approved
- [ ] disciplined-quality-evaluation (KLS 6-dimension + essentialism) passed with GO
- [ ] Human approval received on Gitea #1879 (or GitHub PR 888)
- [ ] All 6 steps executed with green targeted tests + UBS + cross-compile

---

**Next action after approval**: Execute Step 1 using `disciplined-implementation` skill (or directly, with the same todo discipline). All changes must be accompanied by the tests listed above and must pass `cargo test -p <crate> <test>` before the next step is started.

**References**
- Structural review (exact text supplied by user in query)
- Research Document (linked above)
- `disciplined-research` and `disciplined-design` SKILL.md files
- Project CLAUDE.md (British English, no mocks, scoped tests, gtr tracking, date stamps, UBS before commit)
