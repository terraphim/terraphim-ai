# Validation Report: Native PR Gate Producers (#2334)

**Status**: Conditional
**Date**: 2026-06-10 09:40 BST
**Implementation Commit**: `dae72cb98 feat(orchestrator): build native PR gate prompts Refs #2334`
**Verification Report**: `.docs/adf/2334/verification-report.md`
**Issue**: terraphim-ai#2334

## Executive Summary

The implementation satisfies the intended architectural direction: PR gate producers no longer depend on shell-owned fetch/comment/status logic, and the orchestrator now builds bounded native prompts. Full end-to-end validation is conditional because this new native slice has not yet been deployed to bigbox and exercised through a synthetic PR webhook.

## Acceptance Criteria Validation

| Acceptance Criterion | Evidence | Status |
|----------------------|----------|--------|
| Reject bash workaround and avoid returning to shell-owned producer logic | Design moved to `.docs/adf/2334/research-design.md`; `pr-reviewer.toml` simplified | PASS |
| Orchestrator builds bounded native PR gate prompt | `pr_gate_context.rs`, `pr_gate_prompt.rs`, `pr_handlers_impl.rs` wiring | PASS |
| Use Terraphim matching/native crates where applicable | `terraphim_automata` concept matching in `pr_gate_context.rs` | PASS |
| Producer prompt forbids comments/statuses and tool roaming | Prompt contract and unit tests | PASS |
| Existing fail-closed `PrGateResult` handling remains intact | Existing parser/reconcile tests still pass | PASS |
| Gate agents complete usefully on real PR webhook | Requires live deployment and synthetic webhook | CONDITIONAL |
| Terminal Gitea statuses reflect parsed native gate results | Requires live deployment and synthetic webhook | CONDITIONAL |

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

- **Not yet executed** for `dae72cb98`.

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

- Unit fallback path verified; live dispatch not yet executed.

## Non-Functional Validation

| NFR | Target | Evidence | Status |
|-----|--------|----------|--------|
| Responsiveness | Normal gate finishes before 300s cap | Requires live synthetic webhook | CONDITIONAL |
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
| Is it ready for production deployment? | Not without live bigbox deployment and synthetic webhook validation. |
| What would block sign-off? | Any live run that still times out, emits malformed gate blocks, or produces producer-side comments/statuses. |

## Defect Register

| ID | Description | Origin Phase | Severity | Resolution | Status |
|----|-------------|--------------|----------|------------|--------|
| VAL-001 | Live native PR gate path not deployed/tested | Validation | High | Execute E2E-2334-001 before production sign-off | OPEN |
| VAL-002 | `terraphim_grep`/`terraphim_file_search` not yet integrated | Scope phasing | Medium | Track as next native context-enrichment increment if needed | DEFERRED |

## Validation Gate Checklist

- [x] Original architectural requirement addressed: no bash fallback
- [x] Unit and focused integration-boundary verification passed
- [x] Orchestrator-owned comment/status design preserved
- [x] No critical staged UBS findings introduced
- [ ] Live bigbox deployment completed for this commit
- [ ] Synthetic PR webhook executed for this commit
- [ ] All three gate statuses become terminal before 300s without fallback envelopes
- [ ] Stakeholder approves production deployment after live evidence

## Validation Decision

**CONDITIONAL PASS**: The implementation is valid against the architectural requirement and verified locally, but production readiness remains conditional on live ADF deployment and synthetic PR webhook proof.

## Recommended Next Step

Deploy `dae72cb98` or later to bigbox with binary/config backups, then execute `E2E-2334-001`. If the live run succeeds, update this report to `Validated` and close #2334 after PR merge.
