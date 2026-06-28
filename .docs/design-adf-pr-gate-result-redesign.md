# Implementation Plan: ADF PR Gate Result Redesign

**Status**: Draft
**Research Doc**: `.docs/research-adf-pr-gate-result-redesign.md`
**Author**: OpenCode
**Date**: 2026-06-09
**Estimated Effort**: 1-2 focused implementation sessions plus live deployment proof

## Overview

### Summary

Replace the earlier “parse all PR agents as structural reviews” approach with a canonical `PrGateResult` contract. Each PR gate agent keeps its discipline-specific human report, but emits a shared machine-readable block that the orchestrator parses to post current, trustworthy branch-protection statuses.

### Approach

Add a small parser for an HTML comment block embedded in final agent output:

```markdown
<!-- adf:gate-result
{
  "schema_version": 1,
  "agent": "pr-validator",
  "context": "adf/validation",
  "pr_number": 2268,
  "head_sha": "abc123def456...",
  "status": "pass",
  "confidence": 4,
  "blocking_findings": 0,
  "summary": "Validation passed with minor non-blocking concerns"
}
-->
```

The orchestrator validates this block, checks the current head SHA, posts the human report plus block as the PR comment, and posts the terminal Gitea commit status.

### Scope

**In Scope:**

- New `PrGateResult` type and strict parser.
- Context-aware status policy for `adf/pr-reviewer`, `adf/validation`, and `adf/verification`.
- PR-agent metadata renamed from verdict-specific to gate-specific.
- Reconcile logic changed to derive statuses from `PrGateResult` for PR fan-out agents.
- Production PR agent prompt/script updates to emit the canonical block.
- Tests for pass/fail/stale/malformed gate results.

**Out of Scope:**

- Changing native CI.
- Re-enabling ADF build-runner.
- Implementing the full remediation loop.
- Reworking `zdp-validate-pipeline` to drive branch protection.
- Full comment update/idempotency if existing tracker APIs only support appending.

**Avoid At All Cost**:

- Making `parse_verdict` a generic gate parser.
- Allowing unparseable gate output to pass.
- Letting scripts and orchestrator both own the same commit status in steady state.
- Changing all ADF flows in one risky deployment.

## Architecture

### Component Diagram

```text
Gitea PR event
  -> DispatchTask::ReviewPr
  -> handle_review_pr
  -> PR gate agents
       pr-reviewer    -> human structural report + adf:gate-result
       pr-validator   -> human validation report + adf:gate-result
       pr-verifier    -> human verification report + adf:gate-result
  -> reconcile_impl
  -> extract final output
  -> parse PrGateResult
  -> validate context/head/pr/agent
  -> OutputPoster comment
  -> WorkflowTracker commit status
```

### Data Flow

```text
Agent stdout/drain log
  -> extractor::extract_final_assistant_text
  -> pr_gate_result::extract_gate_result
  -> PrGatePolicy::evaluate
  -> post_raw_as_agent_for_project
  -> post_terminal_commit_status
```

### Key Design Decisions

| Decision | Rationale | Alternatives Rejected |
|----------|-----------|----------------------|
| Use an HTML comment JSON block | Machine-readable, invisible in rendered Markdown, preserves human report freedom. | YAML front matter, prose regex parsing, structural review parser reuse. |
| Fail closed on missing/malformed/stale blocks | Branch protection must be trustworthy. | Exit-code fallback, unparseable success, manual-only gates. |
| Context-aware policy | Review, validation, and verification have different semantics. | One universal confidence threshold. |
| Orchestrator owns statuses | Avoids duplicate/conflicting status writes and centralises gate logic. | Script-level curl status posting. |

### Eliminated Options (Essentialism)

| Option Rejected | Why Rejected | Risk of Including |
|-----------------|--------------|-------------------|
| Continue exit-code-derived statuses | Process success does not prove gate success. | False green statuses. |
| Parse all human prose formats | Brittle and hard to test. | Regex sprawl and accidental false positives. |
| Force all agents into one human template | Loses useful discipline-specific evidence. | Worse reviews and validation reports. |
| Update the flow engine in this change | Not needed to fix required PR contexts. | Scope blow-up. |

### Simplicity Check

The simplest architecture is one small, explicit JSON result block appended to every PR gate report. The parser does not need to understand the human report; it only validates the block. A senior engineer should see this as simpler than maintaining separate shell regexes, exit-code conventions, and structural-review parsing hacks.

**Nothing Speculative Checklist:**

- [x] No features beyond trustworthy PR gate statuses.
- [x] No abstraction for future unknown agents beyond explicit `context` and `status` fields already needed now.
- [x] No remediation loop in this slice.
- [x] No native CI changes.
- [x] No premature optimisation.

## File Changes

### New Files

| File | Purpose |
|------|---------|
| `crates/terraphim_orchestrator/src/pr_gate_result.rs` | Defines `PrGateResult`, parser, validation errors, and status policy helpers. |

### Modified Files

| File | Changes |
|------|---------|
| `crates/terraphim_orchestrator/src/lib.rs` | Add `mod pr_gate_result`; replace/rename `PrVerdictMeta` with `PrGateMeta`. |
| `crates/terraphim_orchestrator/src/pr_handlers_impl.rs` | Attach `PrGateMeta` for PR fan-out agents using configured context and injected head SHA. |
| `crates/terraphim_orchestrator/src/reconcile_impl.rs` | Parse `PrGateResult` from drain output and derive terminal status from gate policy. Remove structural parser dependency for generic PR gates. |
| `crates/terraphim_orchestrator/src/pr_dispatch.rs` | Update PR task prompt to require the canonical gate-result block; retain current `ADF_PR_*` env injection. |
| `crates/terraphim_orchestrator/src/pr_review.rs` | Keep structural parser scoped to review/auto-merge only; do not use it for validation/verification status. |
| `/opt/ai-dark-factory/conf.d/terraphim.toml` | Update `pr-reviewer`, `pr-validator`, and `pr-verifier` prompts/scripts to emit `adf:gate-result`; remove self-status posting after orchestrator path is deployed. |

### Deleted Files

None.

## API Design

### Public/Internal Types

```rust
#[derive(Debug, Clone, PartialEq, Eq, serde::Deserialize)]
pub struct PrGateResult {
    pub schema_version: u8,
    pub agent: String,
    pub context: String,
    pub pr_number: u64,
    pub head_sha: String,
    pub status: GateStatus,
    pub confidence: u8,
    pub blocking_findings: u32,
    pub summary: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GateStatus {
    Pass,
    Concerns,
    Fail,
}

#[derive(Debug, Clone)]
pub(crate) struct PrGateMeta {
    pub pr_number: u64,
    pub project: String,
    pub agent_name: String,
    pub context: String,
    pub head_sha: String,
}
```

### Functions

```rust
pub fn extract_gate_result(markdown: &str) -> Result<PrGateResult, PrGateResultError>;

pub fn validate_gate_result(
    result: &PrGateResult,
    meta: &PrGateMeta,
) -> Result<(), PrGateResultError>;

pub fn status_from_gate_result(
    result: &PrGateResult,
) -> (terraphim_tracker::StatusState, String);

pub fn render_missing_gate_failure(reason: impl Into<String>)
    -> (terraphim_tracker::StatusState, String);
```

### Error Types

```rust
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum PrGateResultError {
    #[error("missing adf:gate-result block")]
    MissingBlock,
    #[error("malformed adf:gate-result JSON: {0}")]
    MalformedJson(String),
    #[error("unsupported schema_version {0}")]
    UnsupportedSchema(u8),
    #[error("result context {actual} does not match expected {expected}")]
    ContextMismatch { actual: String, expected: String },
    #[error("result agent {actual} does not match expected {expected}")]
    AgentMismatch { actual: String, expected: String },
    #[error("result PR #{actual} does not match expected #{expected}")]
    PrMismatch { actual: u64, expected: u64 },
    #[error("result head_sha {actual} does not match expected {expected}")]
    HeadMismatch { actual: String, expected: String },
    #[error("confidence {0}/5 is outside 1..=5")]
    ConfidenceOutOfRange(u8),
}
```

## Test Strategy

### Unit Tests

| Test | Location | Purpose |
|------|----------|---------|
| `extract_gate_result_parses_valid_block` | `pr_gate_result.rs` | Valid JSON block parses. |
| `extract_gate_result_rejects_missing_block` | `pr_gate_result.rs` | Missing block fails closed. |
| `extract_gate_result_rejects_malformed_json` | `pr_gate_result.rs` | Malformed JSON fails closed. |
| `validate_gate_result_rejects_stale_head` | `pr_gate_result.rs` | Head SHA mismatch fails. |
| `validate_gate_result_rejects_wrong_context` | `pr_gate_result.rs` | Context mismatch fails. |
| `status_policy_fails_blocking_findings` | `pr_gate_result.rs` | `blocking_findings > 0` fails even with concerns/pass. |
| `status_policy_maps_pass_to_success` | `pr_gate_result.rs` | Clean pass maps to success. |
| `status_policy_maps_fail_to_failure` | `pr_gate_result.rs` | Explicit fail maps to failure. |

### Integration/Focused Tests

| Test | Location | Purpose |
|------|----------|---------|
| `reconcile_pr_gate_uses_gate_result_not_exit_code` | `reconcile_impl` tests if feasible | Agent exit 0 without block becomes failure. |
| `reconcile_pr_gate_failure_overrides_exit_zero` | `reconcile_impl` tests if feasible | `status=fail` maps to failure even if process exits 0. |
| `reconcile_non_pr_agent_keeps_exit_code_status` | existing/new test | Build/non-PR behaviour unchanged. |

### Verification Commands

```bash
cargo fmt -p terraphim_orchestrator
cargo test -p terraphim_orchestrator pr_gate_result
cargo test -p terraphim_orchestrator reconcile_pr_gate
cargo clippy -p terraphim_orchestrator --all-targets
cargo llvm-cov -p terraphim_orchestrator --no-report -- pr_gate_result
ubs --diff --only=rust crates/terraphim_orchestrator
```

## Implementation Steps

### Step 1: Add `PrGateResult` Parser

**Files:** `crates/terraphim_orchestrator/src/pr_gate_result.rs`, `crates/terraphim_orchestrator/src/lib.rs`

**Description:** Define the gate result schema and parser. Keep it pure and side-effect-free.

**Tests:** Parser and validation unit tests.

**Estimated:** 1-2 hours.

### Step 2: Replace PR Verdict Metadata with Gate Metadata

**Files:** `lib.rs`, `pr_handlers_impl.rs`

**Description:** Replace `PrVerdictMeta` with `PrGateMeta` carrying `context` as well as PR/head/agent/project.

**Tests:** Existing PR dispatch tests plus metadata-focused unit coverage if feasible.

**Dependencies:** Step 1.

**Estimated:** 1 hour.

### Step 3: Rework Reconcile Status Derivation

**Files:** `reconcile_impl.rs`

**Description:** On PR gate agent exit, extract final assistant text, parse `PrGateResult`, validate against `PrGateMeta`, post the report/comment, and set terminal commit status from policy. Non-PR agents keep exit-code-derived statuses.

**Tests:** Focused reconcile tests, or pure helper tests if orchestrator setup is too heavy.

**Dependencies:** Steps 1-2.

**Estimated:** 2-3 hours.

### Step 4: Update PR Agent Prompts/Scripts

**Files:** `/opt/ai-dark-factory/conf.d/terraphim.toml` and the git-tracked config source if present.

**Description:** Update `pr-reviewer`, `pr-validator`, and `pr-verifier` prompts to emit the canonical block to stdout. Remove script-level status posting after orchestrator-owned statuses are proven. Prefer no model-side `gtr comment`; let orchestrator post the captured report.

**Tests:** Static config review plus live dry run on one low-risk PR.

**Dependencies:** Step 3.

**Estimated:** 1-2 hours.

### Step 5: Deployment Proof

**Files:** Deployment artefacts only.

**Description:** Build and deploy orchestrator on bigbox. Restart ADF. Trigger or open a low-risk PR. Verify each required status is posted from the canonical gate result.

**Evidence to Capture:** PR URL, three status contexts, three comments, head SHA match, journal lines.

**Dependencies:** Steps 1-4.

**Estimated:** 1-2 hours.

## Rollback Plan

If the deployment blocks PRs incorrectly:

1. Restore previous `/opt/ai-dark-factory/conf.d/terraphim.toml` backup.
2. Restore previous `/usr/local/bin/adf` binary or service deployment artefact.
3. Restart `adf-orchestrator.service`.
4. Re-run `adf-ctl --local status` and check Gitea PR statuses.
5. Comment on issue #2301 with failure mode and captured logs.

## Migration

No database migration.

Configuration migration required:

- Update live PR agent templates.
- Remove stale comments saying `ADF_PR_HEAD_SHA` is unavailable for validation/verification.
- Decide whether script-level `gtr comment` remains temporarily during rollout.

## Dependencies

### New Dependencies

None expected. Use existing `serde_json`, `thiserror`, and tracker types already available in the crate graph.

### Dependency Updates

None.

## Performance Considerations

| Metric | Target | Measurement |
|--------|--------|-------------|
| Parser latency | Negligible relative to agent runtime | Unit tests; no benchmark needed. |
| Memory | O(output size) with existing drain behaviour | No additional large buffers beyond existing output extraction. |
| Gitea calls | One comment + one status per gate exit | Journal/Gitea evidence during live proof. |

## Open Items

| Item | Status | Owner |
|------|--------|-------|
| Decide `concerns` policy for validation/verification | Pending approval | Human maintainer |
| Decide append-only vs update-in-place comments | Pending API check | Implementer |
| Decide when to remove script-level comment posting | Pending rollout plan | Human maintainer |

## Approval

- [ ] Technical review complete
- [ ] Test strategy approved
- [ ] `concerns` policy approved
- [ ] Deployment/rollback plan approved
- [ ] Human approval received before implementation
