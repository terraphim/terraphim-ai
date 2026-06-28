# Research Document: ADF PR Gate Result Redesign

**Status**: Draft
**Author**: OpenCode
**Date**: 2026-06-09
**Reviewers**: Human maintainer

## Executive Summary

The current #2301 direction of parsing every PR agent with the structural review parser is the wrong abstraction. ADF needs a small, stable, machine-readable PR gate result contract that works across review, validation, and verification while preserving each agent's human-facing report format. The orchestrator should own branch-protection commit statuses and use agent output only as signed evidence for the relevant gate.

## Essential Questions Check

| Question | Answer | Evidence |
|----------|--------|----------|
| Energising? | Yes | Broken PR gate verdict handling is the blocker for closing the ADF issue-to-merge loop. |
| Leverages strengths? | Yes | The system already has event dispatch, drain logs, Gitea status helpers, and PR metadata injection; the missing piece is a correct contract. |
| Meets real need? | Yes | Gitea issue #2301 documents that review agents run but do not reliably post parseable verdict comments, which blocks auto-merge/remediation. |

**Proceed**: Yes - 3/3 YES.

## Problem Statement

### Description

ADF currently mixes three concerns in PR gates: human-readable agent reports, machine-readable gate outcomes, and Gitea commit status posting. The previous #2301 implementation attempted to solve this by making the orchestrator parse all PR fan-out agent output through the structural PR review parser. That fails because `pr-validator` and `pr-verifier` are not structural reviewers and intentionally produce different verdict formats.

### Impact

- PR checks can show success based on process exit rather than a current, parseable verdict.
- Unparseable validation/verification can currently pass due to script fallback behaviour.
- `pr-reviewer` can duplicate comment/status posting between the agent script and orchestrator.
- Auto-merge and remediation cannot safely trust gate states.

### Success Criteria

- Each required PR gate emits one machine-readable result for the current head SHA.
- The orchestrator posts exactly one terminal commit status per `(PR, head_sha, context)`.
- Missing, malformed, or stale gate results fail closed.
- Human reports remain discipline-specific: structural review, validation, and verification do not need the same prose template.
- The lifecycle proof flow remains separate unless deliberately wired into PR branch protection.

## Current State Analysis

### Existing Implementation

ADF has two related but distinct systems:

1. Event-driven PR fan-out from `[pr_dispatch]`.
2. Standalone flow-engine pipelines under `.terraphim/flows`.

The production PR fan-out is configured in `/opt/ai-dark-factory/orchestrator.toml`:

```toml
[pr_dispatch]
max_dispatches_per_tick = 3
max_concurrent_pr_agents = 6
agents_on_pr_open = [
    { name = "pr-reviewer", context = "adf/pr-reviewer" },
    { name = "pr-validator", context = "adf/validation" },
    { name = "pr-verifier", context = "adf/verification" },
]
```

The deployed agent templates are in `/opt/ai-dark-factory/conf.d/terraphim.toml`.

### Code Locations

| Component | Location | Purpose |
|-----------|----------|---------|
| PR dispatch config | `/opt/ai-dark-factory/orchestrator.toml` | Declares required PR fan-out agents and status contexts. |
| PR agent templates | `/opt/ai-dark-factory/conf.d/terraphim.toml` | Defines `pr-reviewer`, `pr-validator`, `pr-verifier`, and current script-level verdict logic. |
| PR dispatch helpers | `crates/terraphim_orchestrator/src/pr_dispatch.rs` | Builds PR task metadata and injects `ADF_PR_*` environment variables. |
| PR spawn handler | `crates/terraphim_orchestrator/src/pr_handlers_impl.rs` | Spawns PR agents and posts pending statuses. |
| Exit reconciliation | `crates/terraphim_orchestrator/src/reconcile_impl.rs` | Handles agent exit, output drain, and terminal commit statuses. |
| Existing structural parser | `crates/terraphim_orchestrator/src/pr_review.rs` | Parses structural review comments for auto-merge criteria. |
| Extractor prototype | `crates/terraphim_orchestrator/src/pr_review/extractor.rs` | Extracts final assistant text from CLI output shapes. |
| Flow engine | `crates/terraphim_orchestrator/src/flow/*` | Runs standalone TOML-defined pipelines. |
| Deployed lifecycle pipeline | `/opt/ai-dark-factory/.terraphim/flows/zdp-validate-pipeline.toml` | Runs research/design/implementation/review/corrections/verification/validation/final judge and creates a PR. |

### Data Flow

Current production PR fan-out:

```text
Gitea pull_request event
  -> ADF webhook/dispatcher
  -> DispatchTask::ReviewPr
  -> handle_review_pr
  -> spawn pr-reviewer / pr-validator / pr-verifier
  -> agent-specific script analyses PR
  -> script may post comment and/or status, or exit with status
  -> reconcile_impl posts terminal status from process exit for configured context
```

Current pipeline flow:

```text
adf-ctl flow zdp-validate-pipeline --context issue=N
  -> FlowExecutor
  -> sequential agent/action steps
  -> .docs/adf/N artefacts
  -> branch + PR creation action
```

These are separate. The pipeline does not currently produce branch-protection statuses.

### Integration Points

- Gitea issue/PR comments through `gtr`, `gitea-robot`, or `OutputPoster`.
- Gitea commit statuses through `WorkflowTracker::set_commit_status` or script-level `curl`.
- CLI agents through `pi-rust`, `opencode`, or `claude`.
- Drain logs from spawned agent output.
- `ADF_PR_NUMBER`, `ADF_PR_HEAD_SHA`, `ADF_PR_PROJECT`, `ADF_PR_AUTHOR`, `ADF_PR_DIFF_LOC`, `ADF_PR_TITLE` environment injection.

## Constraints

### Technical Constraints

- PR gate contexts are already required by branch protection: `adf/pr-reviewer`, `adf/validation`, `adf/verification`, plus native CI.
- `pr-reviewer`, `pr-validator`, and `pr-verifier` have different human-report semantics.
- Existing `parse_verdict` is structural-review specific and should not become a generic gate parser.
- The orchestrator must fail closed on missing/stale/malformed gate output.
- The solution must not re-enable retired ADF build-runner paths; native CI remains the build gate.

### Business Constraints

- The fix should unblock #2301 without requiring a full rearchitecture of ADF flows.
- Changes must be deployable incrementally and testable on one low-risk PR.
- Agent prompts/scripts should be simple enough for future maintainers to audit.

### Non-Functional Requirements

| Requirement | Target | Current |
|-------------|--------|---------|
| Status correctness | Status derived from current parseable gate result | Mixed: exit code, script curl, parser attempts |
| Idempotency | One effective gate result per `(context, head_sha)` | Partial script-level dedup; inconsistent per head |
| Failure mode | Missing/malformed/stale result fails closed | Some scripts treat unparseable as success |
| Human readability | Discipline-specific report preserved | Present, but mixed with machine/status logic |

## Vital Few (Essentialism)

### Essential Constraints (Max 3)

| Constraint | Why It's Vital | Evidence |
|------------|----------------|----------|
| One canonical machine-readable gate result | Allows one parser and one status policy engine without forcing one prose format. | `pr-validator`/`pr-verifier` do not match structural review parser. |
| Orchestrator owns commit statuses | Prevents duplicate/conflicting status writes and makes branch protection trustworthy. | Deployed `pr-reviewer` currently posts status while orchestrator can also post terminal status. |
| Head SHA must be validated | Prevents stale approvals after force-push/new commits. | `ADF_PR_HEAD_SHA` is available from `pr_dispatch.rs`; branch protection needs current head. |

### Eliminated from Scope

| Eliminated Item | Why Eliminated |
|-----------------|----------------|
| Reusing structural `parse_verdict` for all gates | It conflates human review format with gate result semantics. |
| Reworking `zdp-validate-pipeline` into branch protection now | It is a lifecycle proof flow, not the current production PR fan-out path. |
| Full remediation loop implementation in this slice | Gate correctness must come first; remediation should consume trustworthy failing gates later. |
| Native CI/build-runner changes | Native CI is already the build path and not the #2301 parser/status problem. |

## Dependencies

### Internal Dependencies

| Dependency | Impact | Risk |
|------------|--------|------|
| `OutputPoster` | Needed to post orchestrator-owned PR comments as agents. | May lack comment update/list API, so initial implementation may post append-only comments. |
| `WorkflowTracker` | Needed for orchestrator-owned commit statuses. | Currently configured for `terraphim-ai`; extra-project correctness must be verified. |
| `pr_dispatch::pr_env_overrides` | Provides current PR head metadata. | Deployed comments are stale, but code injects `ADF_PR_HEAD_SHA`. |
| Agent scripts in `conf.d/terraphim.toml` | Must emit the new canonical result block. | Live config changes must be deployed carefully. |

### External Dependencies

| Dependency | Version | Risk | Alternative |
|------------|---------|------|-------------|
| Gitea API | Live service | Status/comment API failures must fail gate safely. | Retry/leave failure status with reason. |
| CLI agents (`pi-rust`, `opencode`, `claude`) | Live binaries | Output shapes vary; drain extraction must be tolerant. | Require final plain-text result block in stdout. |

## Risks and Unknowns

### Known Risks

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Agents do not print the canonical block to stdout | Medium | Gate fails closed and blocks PRs. | Update prompts/scripts and test with one PR before requiring strict branch protection changes. |
| Duplicate comments during transition | High | PR noise and confusing gate evidence. | Disable script-level comment/status posting after orchestrator-owned path is verified, or add idempotency. |
| Existing branch protection expects contexts before new code deploys | Medium | PRs remain blocked/pending. | Deploy orchestrator and config together; test on low-risk PR. |
| Parser accepts malformed JSON block accidentally | Low | False green gate. | Strict JSON parsing, required fields, known status enum, head SHA match. |

### Open Questions

1. Should `concerns` be success or failure for validation/verification? Recommendation: success for non-blocking concerns only if `blocking_findings = 0`; failure otherwise.
2. Should the orchestrator update a previous comment or append a new one per head/context? Recommendation: append initially if no list/update API exists, then add idempotent update later.
3. Should pipeline `zdp-validate-pipeline` eventually emit the same gate-result block? Recommendation: yes for consistency, but out of this #2301 fix.

### Assumptions Explicitly Stated

| Assumption | Basis | Risk if Wrong | Verified? |
|------------|-------|---------------|-----------|
| `ADF_PR_HEAD_SHA` is available to all PR agents | `pr_dispatch.rs` injects it into `pr_env_overrides`. | Validator/verifier prompts would need a different freshness mechanism. | Yes in source. |
| Production branch protection depends on three ADF PR contexts | Deployed `/opt/ai-dark-factory/orchestrator.toml` lists the three fan-out contexts. | Missing context could make some gate work unnecessary. | Yes in deployed config. |
| Flow pipeline and PR fan-out are separate paths | Flow executor creates artefacts/PRs; PR fan-out handles PR events/statuses. | A hidden integration might need updates too. | Partially verified from source/config. |

### Multiple Interpretations Considered

| Interpretation | Implications | Why Chosen/Rejected |
|----------------|--------------|---------------------|
| Treat all PR gates as structural reviews | One parser, but wrong semantics for validation and verification. | Rejected. Caused the design mismatch. |
| Keep each agent script self-posting statuses | Minimal orchestrator change, but duplicate ownership and false-green risk remain. | Rejected for final state; may be temporary during migration. |
| Canonical gate-result block consumed by orchestrator | One stable machine-readable contract with discipline-specific human reports. | Chosen. Minimal correct abstraction. |

## Research Findings

### Key Insights

1. The useful abstraction is `PR gate result`, not `review verdict`.
2. `parse_verdict` should remain a structural-review parser and may still support auto-merge analysis, but it should not drive validation/verification statuses.
3. Scripts should stop owning Gitea statuses once the orchestrator has the canonical gate-result parser.
4. The lifecycle pipeline can be aligned later by emitting the same gate-result block, but it should not block the PR gate fix.

### Relevant Prior Art

- GitHub Actions/Gitea checks separate human logs from machine status conclusions.
- HTML comments are a common way to embed machine-readable metadata in Markdown without disrupting human reports.
- Existing `pr_review::parse_verdict` shows the value of strict parsing but also demonstrates why parser scope matters.

### Technical Spikes Needed

| Spike | Purpose | Estimated Effort |
|-------|---------|------------------|
| Drain-output parser spike | Verify canonical block extraction from `pi-rust`, `opencode`, and `claude` output shapes. | 1-2 hours |
| Gitea comment idempotency spike | Determine whether `OutputPoster`/tracker can list/update comments or only append. | 1 hour |

## Recommendations

### Proceed/No-Proceed

Proceed with redesign. Replace the earlier structural-parser-for-all-agents approach with a canonical `PrGateResult` contract.

### Scope Recommendations

In the next implementation slice:

1. Add `pr_gate_result` parsing and policy mapping.
2. Wire PR fan-out exit reconciliation to the new parser.
3. Update the three production PR agent prompts/scripts to emit the canonical block.
4. Remove or disable script-level status posting after proving orchestrator-owned status posting.

### Risk Mitigation Recommendations

- Keep structural review human content unchanged, but append the gate-result block.
- Initially fail closed on malformed output and test with a low-risk PR.
- Do not modify native CI or `zdp-validate-pipeline` in the same change.

## Next Steps

If approved:

1. Review `.docs/design-adf-pr-gate-result-redesign.md`.
2. Implement the new parser and tests.
3. Update PR agent prompts/scripts.
4. Deploy to bigbox and run a live low-risk PR proof.

## Appendix

### Reference Materials

- Gitea issue: `terraphim-ai#2301`
- Deployed config: `/opt/ai-dark-factory/orchestrator.toml`
- Deployed PR agents: `/opt/ai-dark-factory/conf.d/terraphim.toml`
- Pipeline flow: `/opt/ai-dark-factory/.terraphim/flows/zdp-validate-pipeline.toml`
