# Implementation Plan: Kairo Auto-Merge Review Policy

**Status**: Approved
**Issue**: Gitea #3284
**Author**: Codex
**Date**: 2026-08-24
**Estimated Effort**: 1 hour

## Overview

Add exact Gitea login `kairo` to the recognised fleet-agent allowlist while preserving every existing auto-merge gate. This is an allowlist-only change: `kairo` may pass the author gate, but only when the review, status, confidence, blocking-finding, project concerns, and current-head gates already pass.

## Evidence

- `crates/terraphim_orchestrator/kg/recognised_agents.md:14` is the canonical KG `synonyms::` allowlist. It currently has `claude-code`, `root`, and `implementation-swarm`.
- `agent_allowlist_kg.rs:21` embeds that KG file with `include_str!`, so editing the KG line updates the embedded/default KG fallback at compile time.
- `agent_allowlist_kg.rs:64` falls back to the embedded KG when the on-disk path is absent or unreadable.
- `pr_review.rs:82` has a separate pure, I/O-free fallback in `AutoMergeCriteria::default()`. It must also include `kairo`.
- `pr_review.rs:264` recognises only exact allowlist membership or the `adf-` prefix. No substring, case-folding, or pattern expansion is needed.
- `pr_poller.rs:342` requires every `MERGE_REQUIRED_CONTEXTS` status to be present and `success`.
- `pr_poller.rs:360` requires every ADF gate context to have a canonical `adf:gate-result`.
- `pr_poller.rs:374`, `:390`, `:398`, and `:404` preserve head-SHA, blocking-finding, fail-status, and `terraphim-ai` concerns gates.

## Scope

In scope:

- Add exact login `kairo` to `crates/terraphim_orchestrator/kg/recognised_agents.md`.
- Add exact login `kairo` to `AutoMergeCriteria::default().recognised_agent_logins`.
- Add RED tests before implementation for KG/default recognition and unchanged merge gates.

Out of scope:

- No changes to `author_is_agent` semantics.
- No changes to thresholds, required contexts, reviewer result parsing, project-specific concerns handling, or head-SHA checks.
- No network, git operations, deployment, issue comments, or commits in this design phase.

Avoid at all cost:

- Do not add wildcard recognition for `kairo-*`, prefixes, display names, or email addresses.
- Do not bypass `MERGE_REQUIRED_CONTEXTS` or `ADF_GATE_CONTEXTS`.
- Do not treat reviewer status success as a substitute for canonical gate-result comments.
- Do not relax `terraphim-ai` concerns behavior.
- Do not add config plumbing beyond the existing KG/default allowlist.

## Design

The minimum implementation is two data edits and focused regression tests.

1. Canonical KG:
   - Change `synonyms:: claude-code, root, implementation-swarm`
   - To `synonyms:: claude-code, root, implementation-swarm, kairo`

2. Pure fallback:
   - Change the literal array in `AutoMergeCriteria::default()` from `["claude-code", "root", "implementation-swarm"]`
   - To `["claude-code", "root", "implementation-swarm", "kairo"]`

No production control flow changes are required. Existing `author_is_agent(login, recognised_logins)` remains exact-match plus `adf-` prefix, and `evaluate_pr_gates` remains the canonical end-to-end gate evaluator for PR polling.

## File Changes

Modified files:

| File | Planned change |
| --- | --- |
| `crates/terraphim_orchestrator/kg/recognised_agents.md` | Append `kairo` to the `synonyms::` line. |
| `crates/terraphim_orchestrator/src/pr_review.rs` | Append `kairo` to `AutoMergeCriteria::default().recognised_agent_logins`; add author/default tests if not placed elsewhere. |
| `crates/terraphim_orchestrator/src/agent_allowlist_kg.rs` | Add assertions covering embedded/default KG recognition and unrelated login rejection. |
| `crates/terraphim_orchestrator/src/pr_poller.rs` | Add pure evaluator tests proving `kairo` remains subject to every polling gate. |

New files: none.

Deleted files: none.

## TDD RED Tests

Write these tests before changing production data.

1. `agent_allowlist_kg::tests::parses_embedded_default_includes_kairo`
   - Arrange: `parse_recognised_agents(DEFAULT_KG_MARKDOWN)`.
   - Assert: result contains `kairo`.
   - RED reason: current KG synonyms do not include `kairo`.

2. `pr_review::tests::default_author_policy_recognises_kairo_and_rejects_unrelated_login`
   - Arrange: `let recognised = AutoMergeCriteria::default().recognised_agent_logins`.
   - Assert: `author_is_agent("kairo", &recognised)` is true.
   - Assert: `author_is_agent("not-kairo", &recognised)` and `author_is_agent("kairo-human", &recognised)` are false.
   - RED reason: current pure fallback does not include `kairo`.

3. `pr_poller::tests::kairo_pr_awaits_missing_reviewer_status_and_result`
   - Arrange: `pr(1, "kairo", "abc", 10)`.
   - Case A: omit `ADF_REVIEWER_CONTEXT` from otherwise green statuses, include all gate-result comments.
   - Assert: `EvaluationOutcome::AwaitingGates`, reason mentions missing `adf/pr-reviewer` status.
   - Case B: include all green statuses but omit the `adf/pr-reviewer` gate-result comment.
   - Assert: `EvaluationOutcome::AwaitingGates`, reason mentions no `adf:gate-result` for `adf/pr-reviewer`.

4. `pr_poller::tests::kairo_pr_does_not_merge_with_stale_fail_concerns_or_blocking_reviewer_evidence_under_terraphim_ai`
   - Arrange: `pr(1, "kairo", "fresh", 10)` and all green statuses.
   - Subcases target `ADF_REVIEWER_CONTEXT` while other ADF contexts pass:
     - stale head SHA returns `StaleGates`;
     - status `fail` returns `HumanReviewNeeded`;
     - status `concerns` under project `terraphim-ai` returns `HumanReviewNeeded`;
     - `blocking_findings: 1` returns `HumanReviewNeeded`.
   - Assert no subcase returns `Merge`.

5. `pr_poller::tests::kairo_pr_requires_all_current_head_nonblocking_required_contexts`
   - Arrange: valid `kairo` PR, pass gate-result comments for all ADF contexts, and all statuses green.
   - For each context in `MERGE_REQUIRED_CONTEXTS`, remove that status and assert `AwaitingGates`.
   - For each context in `MERGE_REQUIRED_CONTEXTS`, set that status to a non-success state and assert `AwaitingGates`.
   - Control assertion: with all required statuses green and all ADF gate results pass on current head, outcome is `Merge`.

## Verification

After implementation, run focused tests:

```bash
cargo test -p terraphim_orchestrator agent_allowlist_kg::tests::parses_embedded_default_includes_kairo
cargo test -p terraphim_orchestrator pr_review::tests::default_author_policy_recognises_kairo_and_rejects_unrelated_login
cargo test -p terraphim_orchestrator pr_poller::tests::kairo_pr_awaits_missing_reviewer_status_and_result
cargo test -p terraphim_orchestrator pr_poller::tests::kairo_pr_does_not_merge_with_stale_fail_concerns_or_blocking_reviewer_evidence_under_terraphim_ai
cargo test -p terraphim_orchestrator pr_poller::tests::kairo_pr_requires_all_current_head_nonblocking_required_contexts
```

Then run the crate-level gate:

```bash
cargo test -p terraphim_orchestrator
cargo fmt --check
cargo clippy -p terraphim_orchestrator --all-targets -- -D warnings
```

## Rollback

Remove `kairo` from the KG `synonyms::` line and from `AutoMergeCriteria::default()`. The tests added for #3284 should fail again, confirming rollback restored the prior policy.

## Approval

- [x] Design approved under the user's explicit #3284 policy request
- [x] Proceed to implementation
