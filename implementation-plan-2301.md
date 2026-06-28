# Detailed Implementation Plan: ADF Verdict Comment Posting (#2301)

**Status**: Phase 2 (Design) Complete, Phase 3 (Implementation) 90% Complete
**Date**: 2026-06-08
**Branch**: `task/2301-verdict-comment-posting` (5 commits ahead of `origin/main` at `e483551`)
**Research Document**: `/tmp/agents-fix/.docs/research-2301-verdict-comment-posting.md`
**Design Document**: `/tmp/agents-fix/.docs/design-2301-verdict-comment-posting.md`

---

## 1. Executive Summary

### Problem Statement
The ADF review agents (pr-reviewer, pr-validator, pr-verifier) run real structural reviews but never post parseable verdict comments with the required `Confidence Score: N/5` and `Last reviewed commit: <head[:7]>` footer. Since #2450 replaced the agent's self-posting task script with a one-line `build_review_task` prompt, the orchestrator's verdict-driven auto-merge and remediation paths have been non-functional. All "merged through the gate" PRs were actually manual clean-merges on green exit-code statuses, not real verdicts.

### Impact
- **Auto-merge**: Never functions -- `evaluate_pr_verdict` always returns `StaleReview` or `NoReviewerComment`
- **Remediation loop (#2264)**: Cannot dispatch fix-agents because no parseable CONDITIONAL verdict exists
- **PR #17**: Dead-code cleanup stuck at 3/5 with no path to autonomous remediation

### Success Criteria (4 Acceptance Criteria)
1. **AC1**: Comment present and parseable -- a `Last reviewed commit: <head[:7]>` comment is posted by the orchestrator after every review run
2. **AC2**: Status verdict-derived -- `adf/pr-reviewer` status reflects the parsed verdict (Success only when parseable + >= min_confidence), not the agent exit code
3. **AC3**: Classifier sees what the drain sees -- the broadcast receiver Lagged-tolerance + drain .log read ensures complete output capture
4. **AC4**: Verdict-driven merge + remediation resolve -- a fresh verdict comment for a new head enables auto-merge and remediation dispatch

---

## 2. Progress Comparison: latest.txt vs latest2.txt

### latest.txt (Earlier Session, 2026-06-05 to 2026-06-07)

| Issue | Status | Delivered |
|-------|--------|-----------|
| #2203 -- repo-local agents (6 polyrepos) | DONE | Merged, deployed, verified |
| #2225 -- doc-generator churn fix | DONE | Merged, deployed, verified |
| #2264 -- remediation loop | PARTIAL | Implemented, merged (PR #18), deployed, **rolled back** due to stampede |
| #2285 -- deploy-safe hardening | DONE | Implemented, merged (PR #21), deployed with kill-switch, verified |
| #2275 -- review-gate robustness | DONE | Implemented, merged (PR #23), deployed |
| #2301 -- verdict comment posting | RESEARCH/DESIGN STARTED | Research complete, design approved, implementation NOT started |

### latest2.txt (Continuation Session, 2026-06-08)

| Issue | Status | Delivered |
|-------|--------|-----------|
| #2301 -- verdict comment posting | IMPLEMENTATION 90% COMPLETE | 5 commits, 918 tests pass, P0/P1 fixes applied |
| P0 bug fix -- duplicate status post | FIXED | Removed duplicate `post_terminal_commit_status` that overwrote verdict-derived status |
| P1 fixes -- dead code, unused fields | FIXED | Removed `#[allow(dead_code)]` annotations, integrated fields into `render_verdict_body` |
| Verification + Validation | COMPLETE | 918 lib tests pass, fmt clean, structural review performed |

### Current Delta (What Remains)

| Item | Status | Owner |
|------|--------|-------|
| Step 5: Integration tests (fake-Gitea harness) | NOT STARTED | Next step |
| Push branch to remote | NOT STARTED | Next step |
| Open PR via gitea-robot / gtr | NOT STARTED | Next step |
| Safe redeploy (disabled-first, then enable) | NOT STARTED | Post-merge |
| End-to-end verification on PR #17 | NOT STARTED | Post-deploy |

---

## 3. Research Findings (Phase 1 Summary)

### Root Cause Analysis
The #2450 change (replacing `def.task` with `build_review_task`) had an unintended side effect:
- **Before #2450**: The agent ran a bash script that reviewed + posted the verdict comment + verdict-derived status itself
- **After #2450**: The agent receives a one-line prompt summary, reviews, but is never told to post anything
- **Status**: Falls back to exit-code (green if exit 0, regardless of whether a verdict was produced)
- **Classifier**: `try_recv` on the broadcast receiver lags out over 240s of streaming (256-slot ring overflow), producing `empty_success` classification

### Fix Strategy (Orchestrator-Only)
Rather than resurrect the `def.task` script (rejected in design -- model adherence issues + double review spend), the fix makes the orchestrator own verdict posting:
1. **Template anchors** in the dispatched prompt instruct the model to emit the four required sections
2. **Per-CLI extractor** reads the orchestrator-written drain `.log` and returns the final assistant text
3. **Drain .log as capture source** at reconcile time -- the orchestrator already writes it via `start_output_log_drain`
4. **Verdict poster** reads the drain, runs the extractor, runs `parse_verdict`, and posts the comment under the `pr-reviewer` login

### Cross-Repo Concern (Explicitly Out of Scope)
The `terraphim_spawner` broadcast-lag issue (root cause of `empty_success` classification) is a separate concern in the `terraphim-ai` repo. This design routes around it by reading the drain .log, which is complete regardless of broadcast overflow.

---

## 4. Design Summary (Phase 2 Summary)

### Architecture

```
pr_dispatch.rs:build_review_task()
    |
    v
[Agent runs, outputs to stdout/stderr]
    |
    v
lib.rs:start_output_log_drain()  -->  drain .log file (complete record)
    |
    v
reconcile_impl.rs (agent exits)
    |
    +-- read_drain_log()  -->  Vec<String> (complete lines)
    |
    +-- Lagged-tolerant try_recv loop  -->  Vec<String> (partial, from broadcast)
    |
    v
pr_review/extractor.rs:extract_final_assistant_text()
    |
    v
pr_review/poster.rs:post_verdict_from_drain()
    |
    +-- find_existing_verdict_for_head()  [idempotency check]
    +-- count_existing_reviews_for_head()  [Reviews (N) counter]
    +-- render_verdict_body()  [wrap extracted text with canonical footer]
    +-- parse_verdict()  [sanity check before posting]
    |
    v
OutputPoster::post_raw_as_agent_for_project("pr-reviewer", body)
    |
    v
Gitea API POST /repos/{owner}/{repo}/issues/{pr_number}/comments
```

### Key Design Decisions

| Decision | Rationale | Alternatives Rejected |
|----------|-----------|----------------------|
| Orchestrator owns posting (not agent) | #2450 broke agent self-posting; orchestrator already has the drain .log | Resurrect `def.task` script (model adherence issues, double spend) |
| Drain .log over broadcast receiver | Broadcast 256-slot ring overflows at 240s; drain file is complete | Fix spawner broadcast (cross-repo, larger scope) |
| Per-CLI extractor | Different CLIs emit different JSON stream formats | Single parser (would miss format-specific events) |
| Idempotency on (project, pr, head_sha) | Re-gated PRs get new SHAs; same-SHA retries increment Reviews (N) | Idempotency on PR number only (would skip re-reviews) |
| NoParseableVerdict -> StatusState::Failure | The load-bearing guard: no verdict = NOT green | Exit-code fallback (preserves the bug) |

### 5/25 Rule Application

**Top 5 (IN scope):**
1. Template anchors in dispatched prompt
2. Per-CLI assistant-text extractor
3. Drain .log read at reconcile
4. Verdict comment poster + verdict-derived status
5. Integration tests against fake-Gitea harness

**Avoid At All Cost (20 eliminated):**
- Resurrecting `def.task` script (model adherence + double review spend)
- Hybrid A+B approach (no gain over A alone)
- LLM-assisted synthesis (expensive, prompt-injectable)
- Retry on parse failure (compounds 240s cost)
- terraphim_spawner migration (out of scope, sidestepped by drain read)
- Changing `parse_verdict` signature (keep unchanged, it's the anchor)
- Mock-based tests (use real fixtures)
- Config-wired kill-switch (already exists from #2285)

---

## 5. Implementation Steps (Phase 3 Status)

### Step 1: Template Anchors in Dispatched Prompt
**Status**: DONE (commit `f71b077`)
**File**: `crates/terraphim_orchestrator/src/pr_dispatch.rs`
**Change**: `build_review_task()` now emits:
```
### Summary

### Confidence Score: N/5

### Inline Findings

<sub>Last reviewed commit: <first-7-hex-of-ADF_PR_HEAD_SHA></sub>
```
**Test**: `pr_dispatch::tests::build_review_task_carries_template_anchors` (passes)

### Step 2: Per-CLI Assistant-Text Extractor
**Status**: DONE (commit `a7e8d71`)
**File**: `crates/terraphim_orchestrator/src/pr_review/extractor.rs` (NEW)
**Change**: Pure function `extract_final_assistant_text(lines, cli_tool)` with per-CLI routing:
- `pi-rust` / `kimi`: newline-delimited JSON events (`message.content[*].text`)
- `claude` stream-json: `content_block_delta` / `message_end` events
- `opencode`: `type: "text"` / `part: {"text": ...}` events
- Non-JSON pass-through: concatenates non-empty, non-JSON, non-header lines
**Tests**: 8 unit tests against 6 real-captured drain fixtures (all pass)
**Fixtures**: `tests/fixtures/pr_review/drain_*.log` (6 files)

### Step 3: Drain .log as Capture Source + Lagged-Tolerant Broadcast Drain
**Status**: DONE (commit `72f8bda`)
**Files**: `crates/terraphim_orchestrator/src/reconcile_impl.rs`, `crates/terraphim_orchestrator/src/lib.rs`
**Changes**:
- `read_drain_log(path)` helper reads the orchestrator-written drain file
- Lagged-tolerant `try_recv` loop: `Err(Lagged(n))` continues draining instead of bailing
- Drain lines preferred over broadcast receiver lines for downstream classifier
**Tests**: 4 unit tests in `drain_log_tests` (all pass)

### Step 4: Orchestrator-Owned Verdict Poster + Verdict-Derived Status
**Status**: DONE (commits `8edc6c6` + `ededa27`)
**Files**:
- `crates/terraphim_orchestrator/src/pr_review/poster.rs` (NEW)
- `crates/terraphim_orchestrator/src/pr_handlers_impl.rs`
- `crates/terraphim_orchestrator/src/reconcile_impl.rs`
- `crates/terraphim_orchestrator/src/lib.rs`
- `crates/terraphim_orchestrator/src/spawn_impl.rs`

**Changes**:
- `PrVerdictMeta` struct carries PR metadata for posting
- `VerdictPostOutcome` enum: `Posted`, `AlreadyPresent`, `NoParseableVerdict`, `NoDrain`
- `post_verdict_from_drain()` async function: reads drain -> extracts -> idempotency check -> renders body -> sanity-checks with `parse_verdict` -> posts comment
- `ManagedAgent` gains `verdict_post: Option<PrVerdictMeta>` field
- `pr_handlers_impl.rs`: pr-reviewer and pr-validator spawns set `verdict_post: Some(meta)`
- `reconcile_impl.rs`: verdict-derived status branch (Success for Posted/AlreadyPresent, Failure for NoParseableVerdict, Error for NoDrain)
- **P0 fix**: Removed duplicate `post_terminal_commit_status` block that overwrote verdict-derived status with exit-code status
- **P1 fixes**: Removed dead code (`_path_marker` stub), integrated `author_login`, `title`, `diff_loc` into `render_verdict_body`

**Tests**: 4 unit tests in `poster::tests` (all pass)

### Step 5: Integration Tests (NOT YET DONE)
**Target**: `crates/terraphim_orchestrator/tests/verdict_poster_tests.rs` (NEW)
**Harness**: Existing real-axum fake-Gitea in `tests/remediation_tests.rs:295-322`
**Test Cases**:
1. Verdict comment posted after agent exit (fake-Gitea receives POST)
2. Status is verdict-derived (not exit-code) -- verify `set_commit_status` call state
3. No false empty_success -- agent with no parseable text gets StatusState::Failure
4. Idempotency -- second dispatch on same SHA returns AlreadyPresent
5. Verdict-driven merge -- fresh verdict comment enables auto-merge path

---

## 6. Verification Report (Phase 4 Partial)

### Metrics

| Metric | Target | Actual | Status |
|--------|--------|--------|--------|
| Unit tests covering 4 ACs | >= 8 | 16 (8 extractor + 4 poster + 4 drain_log) | PASS |
| Gate: cargo fmt --check | clean | clean | PASS |
| Gate: cargo clippy default | clean | clean | PASS |
| Gate: cargo clippy --features quickwit | clean | clean | PASS |
| Gate: cargo test --lib (default) | 0 regressions | 918 pass + 1 pre-existing failure | PASS |
| Gate: cargo test --lib (--features quickwit) | 0 regressions | 918 pass + 1 pre-existing failure | PASS |
| AC1, AC2, AC3, AC4 | all present | all present in code | PASS |
| O1, O2, O3 (deploy-safety from #2285) | unchanged | tests still pass | PASS |

### Pre-Existing Failure
`fleet_config::test_source_template_build_runner_event_only` -- missing fixture `scripts/adf-setup/agents/build-runner.toml`. Fails identically on `origin/main` (not a regression).

### Open Defects (from Structural PR Review)

| Severity | Issue | Status |
|----------|-------|--------|
| P0 | Duplicate `post_terminal_commit_status` overwrote verdict-derived status | FIXED in `ededa27` |
| P1 | `_path_marker` dead-code stub | FIXED in `ededa27` |
| P1 | `#[allow(dead_code)]` on `PrVerdictMeta` fields | FIXED in `ededa27` |
| P1 | `Posted { comment_id: 0, confidence: 0 }` sentinels | DOCUMENTED (requires `OutputPoster` API change, deferred) |
| P2 | Sequential `fetch_comments` calls (idempotency + counter) | ACCEPTABLE (design chose degradation) |
| P2 | Silent error swallowing on Gitea list errors | ACCEPTABLE (design chose degradation) |

---

## 7. Remaining Work Plan

### Immediate (Before PR)

1. **Write Integration Tests (Step 5)**
   - File: `crates/terraphim_orchestrator/tests/verdict_poster_tests.rs`
   - Harness: Extend existing `ServerState` from `remediation_tests.rs`
   - Tests: 5 test cases covering the 4 ACs + idempotency
   - Time estimate: 2-3 hours

2. **Run Full Gate**
   - `cargo fmt --all -- --check`
   - `cargo clippy -p terraphim_orchestrator --all-targets -- -D warnings`
   - `cargo test -p terraphim_orchestrator` (default + `--features quickwit`)
   - Time estimate: 15 min

3. **Commit Integration Tests**
   - Commit: `test(orchestrator): add verdict poster integration tests (Refs #2301)`

### PR + Merge

4. **Push Branch**
   ```bash
   cd /tmp/agents-fix
   git push origin task/2301-verdict-comment-posting
   ```

5. **Open PR via gitea-robot**
   ```bash
   gtr create-pull --owner terraphim --repo terraphim-agents \
     --title "Fix #2301: orchestrator-owned verdict comment posting" \
     --base main --head task/2301-verdict-comment-posting
   ```

6. **Monitor PR Through Gate**
   - Wait for haiku/glm reviewers to run
   - Verify reviewers produce verdict comments (this is the dogfood test)
   - If green, merge; if issues, fix and re-push

### Deploy (Post-Merge)

7. **Rebuild adf from gitea/main**
   ```bash
   cd /data/projects/terraphim/terraphim-agents
   git worktree add --force /home/alex/adf-build-clean origin/main
   cd /home/alex/adf-build-clean
   export CARGO_TARGET_DIR=/data/projects/terraphim/terraphim-agents/target
   cargo build --release -p terraphim_orchestrator --bin adf
   ```

8. **--check Before Install**
   ```bash
   cd /opt/ai-dark-factory
   export GITEA_URL=https://git.terraphim.cloud
   export GITEA_TOKEN=...
   /usr/local/bin/adf --check orchestrator.toml
   ```
   Must show: pr-remediation on glm, reviewers on haiku/review_tier, no errors

9. **Atomic Install + Restart**
   ```bash
   sudo cp target/release/adf /usr/local/bin/adf.new
   sudo mv -f /usr/local/bin/adf.new /usr/local/bin/adf
   sudo systemctl restart adf-orchestrator.service
   ```

10. **Verify Startup**
    ```bash
    sudo systemctl is-active adf-orchestrator.service
    sudo journalctl -u adf-orchestrator.service --since "1 min ago" | grep -E "reconciliation|webhook|error|panic"
    ```

### End-to-End Verification

11. **Disabled-First Verification**
    - Keep `max_remediation_attempts = 0` in `[auto_merge]` config
    - Verify 0 remediation dispatches over 2-3 poll cycles
    - Confirm no stampede (from #2285 hardening)

12. **Enable Conservatively**
    - Set `max_remediation_attempts = 3`, `max_dispatches_per_tick = 1`
    - Restart
    - Verify <= 1 dispatch per tick, terraphim-agents only

13. **Test on PR #17**
    - Re-trigger review on PR #17 (empty commit to task/1-impl)
    - Watch for: fresh verdict comment with `Last reviewed commit: <new-head[:7]>`
    - Watch for: `adf/pr-reviewer` status reflects verdict (not just exit-code)
    - Watch for: remediation loop dispatches if confidence < 5/5
    - Expected: fix-agent commits load_state() fix, re-gates, scores 4-5/5, auto-merges

---

## 8. Risk Register

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Integration tests fail on fake-Gitea harness | Medium | Medium | Harness exists and works for remediation tests; extend carefully |
| Reviewers crash on PR (size/quota) | Low | High | #2275 hardening + glm routing (non-Claude) already deployed |
| Deploy stampede on restart | Low | High | #2285 kill-switch + cap + persistence already verified |
| Verdict comment still not posted (deeper bug) | Low | Critical | This is the exact fix; if it fails, the extractor/poster logic is the issue |
| Merge conflict with main | Medium | Low | Rebase if needed; branch is off e483551 which is before #2275 merge |

---

## 9. Decision Points

### For User Approval

1. **Proceed with Step 5 (integration tests)?**
   - The 4 core commits are complete and verified
   - Integration tests add coverage but are not blocking for the core fix
   - Recommendation: Yes, write them before PR for completeness

2. **Deploy immediately after merge, or wait for observation window?**
   - #2285 hardening means the deploy is safe (kill-switch + cap)
   - But #2301 changes the core verdict path -- a rollback would be needed if broken
   - Recommendation: Deploy during low-activity window, keep kill-switch ready

3. **Should PR #17 be the first live test, or a simpler test PR?**
   - #17 has real complexity (load_state() P1 + 2 P2s)
   - A simpler test PR (e.g., typo fix) would verify the loop mechanics first
   - Recommendation: Use a simple test PR first, then #17

---

## 10. Appendix

### Files Changed (5 commits)

| File | Lines | Type | Purpose |
|------|-------|------|---------|
| `src/pr_dispatch.rs` | +65/-3 | Modified | Template anchors in review prompt |
| `src/pr_review/extractor.rs` | +327 | NEW | Per-CLI assistant-text extractor |
| `src/pr_review/poster.rs` | +361 | NEW | Verdict comment poster |
| `src/pr_review.rs` | +4/-1 | Modified | Module declarations |
| `src/reconcile_impl.rs` | +122/-19 | Modified | Drain read, Lagged-tolerance, verdict-derived status |
| `src/lib.rs` | +6 | Modified | `verdict_post` field on `ManagedAgent` |
| `src/spawn_impl.rs` | +1 | Modified | Default `verdict_post: None` |
| `src/pr_handlers_impl.rs` | +45 | Modified | Set `verdict_post: Some(meta)` on pr-reviewer/validator |
| `tests/fixtures/pr_review/*.log` | +6 files | NEW | Real-captured drain fixtures |

### Commits (Chronological)

1. `f71b077` -- feat(pr_dispatch): carry verdict-template anchors in review task prompt
2. `a7e8d71` -- feat(pr_review): add per-CLI assistant-text extractor with real-captured fixtures
3. `72f8bda` -- feat(orchestrator): source capture from drain .log + Lagged-tolerant broadcast drain
4. `8edc6c6` -- feat(pr_review): orchestrator-owned verdict comment poster + verdict-derived status
5. `ededa27` -- fix(orchestrator): address structural PR review findings on #2301

### Reference Documents

- Research: `/tmp/agents-fix/.docs/research-2301-verdict-comment-posting.md` (382 lines)
- Design: `/tmp/agents-fix/.docs/design-2301-verdict-comment-posting.md` (406 lines)
- Structural Review: `/tmp/agents-fix/.docs/structural-review-2301.md` (generated inline in session)
- Plan: `/Users/alex/.claude/plans/snuggly-giggling-quill.md`

---

## 11. Next Action

**Recommended**: Write the 5 integration tests (Step 5), commit, push branch, open PR, monitor through gate, deploy with disabled-first posture, verify on simple test PR, then on #17.

**Alternative**: Push the 4 commits as-is (without integration tests), open PR, and write tests in parallel while PR is under review.

**Risk of delaying**: The verdict-comment bug is the binding constraint on the entire ADF auto-merge and remediation system. Every day without this fix means:
- PRs cannot auto-merge (all require manual intervention)
- The remediation loop cannot fire (no CONDITIONAL verdicts to consume)
- PR #17 (and similar below-threshold PRs) sit indefinitely
