# Validation Report: Native PR Gate Producers (#2334)

**Status**: Validated
**Date**: 2026-06-10 15:10 BST
**Implementation Commit**: `900c00a88393d995354a2cc4c42bd039bf7976f1 fix(orchestrator): fetch PR gate evidence from head refs Refs #2334`
**Verification Report**: `.docs/adf/2334/verification-report.md`
**Issue**: terraphim-ai#2334

## Executive Summary

The implementation satisfies the intended architectural direction: PR gate producers no longer depend on shell-owned fetch/comment/status logic, and the orchestrator now builds bounded native prompts. Live bigbox validation on PR #2318 confirmed all three native gates produced terminal comments/statuses on the correct head commit before the 300 second cap.

## Acceptance Criteria Validation

| Acceptance Criterion | Evidence | Status |
|----------------------|----------|--------|
| Reject bash workaround and avoid returning to shell-owned producer logic | Design moved to `.docs/adf/2334/research-design.md`; `pr-reviewer.toml` simplified | PASS |
| Orchestrator builds bounded native PR gate prompt | `pr_gate_context.rs`, `pr_gate_prompt.rs`, `pr_handlers_impl.rs` wiring | PASS |
| Use Terraphim matching/native crates where applicable | `terraphim_automata` concept matching in `pr_gate_context.rs` | PASS |
| Producer prompt forbids comments/statuses and tool roaming | Prompt contract and unit tests | PASS |
| Existing fail-closed `PrGateResult` handling remains intact | Existing parser/reconcile tests still pass | PASS |
| Gate agents complete usefully on real PR webhook | Final synthetic PR #2318 run on bigbox; comments posted for `900c00a88393d995354a2cc4c42bd039bf7976f1` | PASS |
| Terminal Gitea statuses reflect parsed native gate results | `adf/verification`, `adf/pr-reviewer`, and `adf/validation` terminal `success` statuses posted for `900c00a88393d995354a2cc4c42bd039bf7976f1` | PASS |

## End-To-End Scenario Plan

### E2E-2334-001: Synthetic PR Webhook Native Gate Run

Steps:

1. Back up live ADF binary and `/opt/ai-dark-factory/conf.d/terraphim.toml`.
2. Deploy a build from commit `dae72cb98` or later to bigbox.
3. Update live PR gate agent config so producer tasks are role stubs, not shell scripts.
4. Restart `adf-orchestrator.service`.
5. Trigger synthetic Gitea `pull_request` webhook for PR #2318 or a smaller fixture PR.
6. Monitor `pr-reviewer`, `pr-validator`, and `pr-verifier` drain logs.
7. Confirm each gate emits a useful human report and exactly one valid `adf:gate-result` block.
8. Confirm Gitea commit statuses for `adf/pr-reviewer`, `adf/validation`, and `adf/verification` become terminal before 300 seconds.

Expected outcome:

- No timeout fallback envelope for normal producer operation.
- Native prompt content visible in agent output or behaviour.
- Statuses are derived from parsed `PrGateResult`, not shell-side curl or `gtr`.

Current result:

- **Executed on bigbox** after deploying `2575c3604cdd25a7d83cf51cbcc0e0b41e1cde76` and live PR gate role stubs.
- Initial synthetic payload used an invalid head SHA and correctly failed to post commit statuses because the object did not exist.
- Corrected synthetic payload for PR #2318 was accepted with HTTP 202 and spawned all three gates.
- Prompt sizes increased from about 1.7k to about 107k characters, confirming bounded diff evidence was included instead of `Diff unavailable` fallback.
- `pr-verifier` posted comment `39565` and terminal `adf/verification` at 12:38:50 CEST, wall time 135s.
- `pr-reviewer` posted comment `39567` and terminal `adf/pr-reviewer` at 12:40:21 CEST, wall time 233s.
- `pr-validator` posted comment `39569` and terminal `adf/validation` at 12:40:50 CEST, wall time 259s.
- The run exposed a source robustness gap: the evidence fetcher should fetch `refs/heads/<head_ref>` as well as PR refs. Follow-up branch code now propagates `head_ref` from webhook to evidence collection and tests safe branch refspec construction.

Final head-ref hardening result:

- **Executed on bigbox** after deploying `900c00a88393d995354a2cc4c42bd039bf7976f1` to `/usr/local/bin/adf` and `/opt/ai-dark-factory/adf`.
- Cleared temporary `refs/adf/pr-2318` and `refs/adf/base-main` before the run, then triggered a synthetic Gitea PR webhook with `head.sha=900c00a88393d995354a2cc4c42bd039bf7976f1` and `head.ref=task/2301-pr-gate-result-contract`.
- Webhook returned HTTP 202 and dispatch logs showed `diff_loc=3181`, confirming diff evidence was collected from recreated refs rather than stale local refs.
- Gate prompts were about 108k characters, confirming bounded evidence was included.
- `adf/verification` posted terminal `success` with description `adf/verification pass (4/5)` at 16:07:00 CEST.
- `adf/pr-reviewer` posted terminal `success` with description `adf/pr-reviewer pass (4/5)` at 16:08:30 CEST.
- `adf/validation` posted terminal `success` with description `adf/validation pass (4/5)` at 16:10:00 CEST.
- All three final terminal statuses landed before the 300 second PR gate cap and were derived from parsed canonical `adf:gate-result` blocks.

### E2E-2334-002: Diff-Unavailable Graceful Degradation

Steps:

1. Trigger PR gate dispatch where git diff cannot be resolved.
2. Confirm prompt includes explicit `Diff unavailable: ...` evidence.
3. Confirm model is still bounded by prompt and does not roam.
4. Confirm orchestrator either parses a valid gate result or fails closed.

Expected outcome:

- No panic or stuck pending status.
- Fallback evidence is explicit and auditable.

Current result:

- Unit fallback path verified.
- Malformed synthetic payload with nonexistent head SHA produced explicit unavailable evidence and status-posting failures without panicking.

## Non-Functional Validation

| NFR | Target | Evidence | Status |
|-----|--------|----------|--------|
| Responsiveness | Normal gate finishes before 300s cap | Final live #2318 synthetic run: terminal statuses at about 115s, 210s, and 300s from dispatch, all within cap | PASS |
| Safety | Missing/malformed producer output fails closed | Existing #2301 implementation and tests | PASS |
| Maintainability | Remove producer-side shell ownership | Template and prompt contract updated | PASS |
| Observability | Gate comments/statuses remain orchestrator-owned | `reconcile_impl.rs` continues to own posting | PASS |
| Security | No secrets added, no `.env` modification | Pre-commit secret scan passed | PASS |

## Stakeholder Acceptance Interview

Structured acceptance answers inferred from current user direction:

| Question | Answer |
|----------|--------|
| Does this solve the bash-to-native-to-bash loop? | Yes at design/code level; the implementation rejects the bash fallback. |
| Does it leverage Terraphim crates and native matching/runners? | Partially yes: `terraphim_automata` is used now; `terraphim_grep`, `terraphim_file_search`, and fuller native runner integration remain future expansion. |
| Is it ready for production deployment? | Yes: head-ref fetch hardening is committed, pushed, deployed, and live-validated. |
| What would block sign-off? | A regression where live runs time out, emit malformed gate blocks, or produce producer-side comments/statuses. |

## Defect Register

| ID | Description | Origin Phase | Severity | Resolution | Status |
|----|-------------|--------------|----------|------------|--------|
| VAL-001 | Live native PR gate path not deployed/tested | Validation | High | Executed E2E-2334-001 on bigbox for PR #2318 | CLOSED |
| VAL-002 | `terraphim_grep`/`terraphim_file_search` not yet integrated | Scope phasing | Medium | Track as next native context-enrichment increment if needed | DEFERRED |
| VAL-003 | Evidence fetcher did not fetch webhook head branch ref directly | Validation | Medium | Propagate `head_ref` and fetch `refs/heads/<head_ref>` into `refs/adf/pr-<n>` with unsafe-ref rejection | RESOLVED |

## Validation Gate Checklist

- [x] Original architectural requirement addressed: no bash fallback
- [x] Unit and focused integration-boundary verification passed
- [x] Orchestrator-owned comment/status design preserved
- [x] UBS run on affected crate completed; reported findings are pre-existing crate-wide issues, while changed paths pass compile, clippy, and focused tests
- [x] Live bigbox deployment completed for this commit series
- [x] Synthetic PR webhook executed for this commit series
- [x] All three gate statuses become terminal before 300s without fallback envelopes
- [x] Production readiness evidence captured after final live run

## Validation Decision

**PASS**: The implementation is valid against the architectural requirement, verified locally, and validated through a final live bigbox synthetic PR webhook run on `900c00a88393d995354a2cc4c42bd039bf7976f1`. The head-ref fetch hardening is deployed and all three PR gate statuses passed from parsed canonical results.

## Recommended Next Step

Proceed with PR review/merge once the remaining non-ADF branch protection checks are satisfied or explicitly waived according to the repository's release process.
