# Implementation Plan: ADF CLI Full Agent Runner

**Status**: Draft
**Research Doc**: `.docs/research-adf-cli-full-agent-runner.md`
**Author**: OpenCode
**Date**: 2026-05-23 23:06 CEST
**Estimated Effort**: 3-5 days

## Implementation Approval

Approved to proceed with `opencode` using `minimax-coding-plan/MiniMax-M2.7-highspeed` for implementation work while `pi-rust` is updated. `pi-rust` currently does not expose `MiniMax-M2.7-highspeed`; once that update lands and verifies, replace the temporary `opencode` implementation route with `pi-rust`.

## Overview

### Summary

Extend the ADF CLI so operators can validate and run any configured agent end-to-end through production orchestration semantics. The CLI must cover scheduled, global, local, project-scoped, evolution-enabled, and Gitea-triggered execution without starting the full daemon unless explicitly requested.

### Approach

Add an `agent` command group to `crates/terraphim_orchestrator/src/bin/adf.rs` and introduce a reusable one-shot runner API inside `terraphim_orchestrator`. The runner must reuse `AgentOrchestrator` resolution and spawn behaviour, not duplicate it.

Update after Gitea recheck: `gitea/main` already contains `adf --local --check` and `adf --local --agent NAME`. Human feedback confirms this direct local spawner path is a good start. The implementation plan should preserve that work and evolve it into production-faithful `AgentOrchestrator` one-shot execution, then expose scheduled/Gitea/evolution-aware variants.

### Scope

**In Scope:**

- `adf agent validate --config <path> <agent>`.
- `adf agent run --config <path> <agent> [--context ...] [--wait|--detach] [--format json|human]`.
- `adf agent trigger gitea --config <path> ...` for issue-comment, PR, push, and raw JSON modes.
- `adf agent simulate schedule --config <path> <agent>` for cron resolution and optional event injection.
- Local/global/project agent resolution.
- Evolution-enabled validation and run reporting.
- Structured run result output.
- Integration tests with real config parsing and real local process execution.

**Out of Scope:**

- Replacing the long-running `adf CONFIG` daemon mode.
- Replacing remote SSH `adf-ctl` in this change.
- Building a UI/dashboard.
- Changing provider routing policy.
- General multi-agent workflow execution beyond one targeted agent or one supplied Gitea event.

**Avoid At All Cost:**

- A second spawn engine in the CLI.
- Tests using mocks instead of real config/process execution.
- Unbounded waits by default.
- Silent posting to real Gitea issues during validation.
- Shelling out to `adf-ctl` from `adf`.

## Architecture

### Component Diagram

```text
adf binary
  -> AgentCliArgs
  -> AgentRunRequest / AgentValidationRequest
  -> AgentOrchestrator::from_config_file
  -> AgentOrchestrator::validate_agent_runtime
  -> AgentOrchestrator::run_agent_once
       -> existing spawn_agent(def)
       -> existing poll/output/exit handling
       -> AgentRunReport
```

Gitea simulation path:

```text
adf agent trigger gitea
  -> GiteaTriggerArgs
  -> WebhookDispatch builder or raw webhook parser
  -> AgentOrchestrator::handle_dispatch_once
  -> existing handle_webhook_dispatch / dispatcher path
  -> AgentRunReport or DispatchReport
```

Schedule simulation path:

```text
adf agent simulate schedule
  -> config scheduler lookup
  -> ScheduleSimulationReport
  -> optional ScheduleEvent::Spawn injection
  -> run_agent_once
```

### Data Flow

```text
CLI args
  -> load OrchestratorConfig
  -> config.validate()
  -> resolve AgentDefinition + ProjectRuntime
  -> validate runtime prerequisites
  -> choose mode:
       validate only: return RuntimeValidationReport
       direct run: AgentRunRequest -> run_agent_once
       gitea trigger: WebhookDispatch -> handle_dispatch_once
       schedule simulate: ScheduleEvent -> run_agent_once or report only
  -> print human or JSON output
```

### Key Design Decisions

| Decision | Rationale | Alternatives Rejected |
|----------|-----------|----------------------|
| Extend `adf`, not `adf-ctl` | `adf` owns local config/orchestrator lifecycle. | SSH-only remote trigger remains insufficient. |
| Add public one-shot API | CLI and tests need production spawn without daemon loop. | Making private `spawn_agent` public directly exposes too much state. |
| Return typed reports | Automation should not parse logs. | Journal scraping is brittle. |
| Use internal `WebhookDispatch` for Gitea | Avoids requiring network/webhook server for local validation. | Curling localhost/webhook is slower and less deterministic. |
| Preserve and converge `--local` | Gitea already landed useful local discovery/spawn code. | Replacing it abruptly risks breaking local workflows. |

### Eliminated Options

| Option Rejected | Why Rejected | Risk of Including |
|-----------------|--------------|-------------------|
| New CLI runner that calls CLI tools directly | Bypasses routing, worktrees, budgets, output, evolution. | False confidence. |
| Waiting for real cron | Slow and flaky. | Non-deterministic tests and operator frustration. |
| Always post output to Gitea | Validation could spam real issues. | Operational noise. |
| Make `adf-ctl` authoritative | Tied to SSH and running daemon. | Cannot validate local configs. |

### Simplicity Check

The simplest correct design is a thin CLI facade around a new one-shot orchestration API. It adds command parsing and reporting, but delegates runtime behaviour to existing orchestration internals.

**Nothing Speculative Checklist:**

- [x] No new dashboard.
- [x] No replacement scheduler.
- [x] No replacement dispatcher.
- [x] No new provider routing policy.
- [x] No test mocks.

## File Changes

### New Files

| File | Purpose |
|------|---------|
| `crates/terraphim_orchestrator/src/agent_runner.rs` | One-shot agent validation/run request and report types. |
| `crates/terraphim_orchestrator/tests/adf_agent_cli_tests.rs` | CLI-level integration tests with real temp configs and local processes. |

### Modified Files

| File | Changes |
|------|---------|
| `crates/terraphim_orchestrator/src/lib.rs` | Export `agent_runner`; add orchestration methods that reuse private spawn path. |
| `crates/terraphim_orchestrator/src/bin/adf.rs` | Extend landed `--local --check` / `--local --agent` support with `agent` subcommands and production-faithful one-shot execution. |
| `crates/terraphim_orchestrator/src/webhook.rs` | Expose safe builders/parsers needed by CLI Gitea trigger simulation if currently private. |
| `crates/terraphim_orchestrator/src/project_adf.rs` | Reuse landed `.terraphim/adf.toml` discovery/conversion for local mode; add gaps only where needed for evolution/Gitea/reporting. |
| `crates/terraphim_orchestrator/src/config.rs` | Add helper methods only if current project/agent resolution helpers are insufficient. |
| `crates/terraphim_orchestrator/Cargo.toml` | Add dev-dependencies only if needed for CLI tests. |
| `.docs/summary.md` | Update project overview after implementation. |

### Deleted Files

| File | Reason |
|------|--------|
| None | This is an additive CLI/API change. |

## API Design

### Public Types

```rust
#[derive(Debug, Clone)]
pub struct AgentRunRequest {
    pub agent_name: String,
    pub project: Option<String>,
    pub context: Option<String>,
    pub source: AgentRunSource,
    pub wait: WaitMode,
    pub post_output: OutputPostMode,
}

#[derive(Debug, Clone)]
pub enum AgentRunSource {
    Direct,
    ScheduleSimulation,
    GiteaIssueComment { issue_number: u64, comment_id: u64 },
    GiteaPullRequest { pr_number: u64, head_sha: String },
    GiteaPush { ref_name: String, before_sha: String, after_sha: String },
}

#[derive(Debug, Clone, Copy)]
pub enum WaitMode {
    Detached,
    UntilExit { timeout_secs: u64 },
}

#[derive(Debug, Clone, Copy)]
pub enum OutputPostMode {
    ConfigDefault,
    Disabled,
    Enabled,
}

#[derive(Debug, serde::Serialize)]
pub struct AgentRuntimeValidationReport {
    pub agent_name: String,
    pub project: String,
    pub layer: String,
    pub schedule: Option<String>,
    pub cli_tool: String,
    pub model: Option<String>,
    pub working_dir: String,
    pub repo_ok: bool,
    pub gitea_target: Option<GiteaTargetReport>,
    pub evolution_requested: bool,
    pub evolution_available: bool,
    pub runnable: bool,
    pub warnings: Vec<String>,
}

#[derive(Debug, serde::Serialize)]
pub struct AgentRunReport {
    pub validation: AgentRuntimeValidationReport,
    pub spawned: bool,
    pub detached: bool,
    pub session_id: Option<String>,
    pub worktree_path: Option<String>,
    pub routed_model: Option<String>,
    pub log_path: Option<String>,
    pub exit_status: Option<i32>,
    pub exit_class: Option<String>,
    pub evolution_snapshot_key: Option<String>,
}
```

### Public Functions

```rust
impl AgentOrchestrator {
    pub fn validate_agent_runtime(
        &self,
        request: &AgentRunRequest,
    ) -> Result<AgentRuntimeValidationReport, OrchestratorError>;

    pub async fn run_agent_once(
        &mut self,
        request: AgentRunRequest,
    ) -> Result<AgentRunReport, OrchestratorError>;

    pub async fn handle_dispatch_once(
        &mut self,
        dispatch: webhook::WebhookDispatch,
        wait: WaitMode,
    ) -> Result<Vec<AgentRunReport>, OrchestratorError>;
}
```

### CLI Shape

```text
adf agent validate --config CONFIG AGENT [--project PROJECT] [--format human|json]
adf agent run --config CONFIG AGENT [--project PROJECT] [--context TEXT] [--wait] [--timeout SECS] [--no-post] [--format human|json]
adf agent simulate schedule --config CONFIG AGENT [--project PROJECT] [--fire] [--format human|json]
adf agent trigger gitea issue-comment --config CONFIG --agent AGENT --issue N [--comment-id N] [--context TEXT] [--wait]
adf agent trigger gitea pr --config CONFIG --pr N --head-sha SHA [--project PROJECT] [--wait]
adf agent trigger gitea push --config CONFIG --ref REF --before SHA --after SHA [--project PROJECT] [--files PATHS] [--wait]
adf agent trigger gitea raw --config CONFIG < payload.json
```

### Error Types

Prefer extending `OrchestratorError` minimally:

```rust
AgentNotRunnable { agent: String, reason: String }
RunTimedOut { agent: String, timeout_secs: u64 }
InvalidRunContext { reason: String }
```

## Test Strategy

### Unit Tests

| Test | Location | Purpose |
|------|----------|---------|
| `validate_global_agent_runtime` | `agent_runner.rs` | Global agent resolves working dir and CLI. |
| `validate_project_agent_runtime` | `agent_runner.rs` | Project agent resolves project working dir and Gitea target. |
| `validate_evolution_flags` | `agent_runner.rs` | Reports top-level and per-agent evolution gates. |
| `build_issue_comment_dispatch` | `webhook.rs` or `agent_runner.rs` | CLI args produce expected `WebhookDispatch`. |
| `schedule_simulation_reports_next_fire` | `agent_runner.rs` | Cron schedule is parsed and reported without waiting. |

### Integration Tests

| Test | Location | Purpose |
|------|----------|---------|
| `adf_agent_validate_json` | `tests/adf_agent_cli_tests.rs` | Runs binary with temp TOML and validates JSON. |
| `adf_agent_run_wait_success` | `tests/adf_agent_cli_tests.rs` | Spawns a real local harmless command and reports success. |
| `adf_agent_run_wait_failure` | `tests/adf_agent_cli_tests.rs` | Real local command exits non-zero and report captures failure. |
| `adf_agent_trigger_gitea_comment` | `tests/adf_agent_cli_tests.rs` | Simulates `@adf:<agent>` through dispatch path. |
| `adf_agent_project_working_dir` | `tests/adf_agent_cli_tests.rs` | Confirms project-bound agent runs in project working dir. |

Existing landed tests on Gitea (`adf_check_tests.rs`) already cover `adf --check`, `adf --local --check`, and `adf --local --agent` with real CLI processes. New tests should extend those rather than duplicate them.

## Phase 4 Verification Plan for Landed PRs

Before implementing the full CLI runner, verify the foundation now present on Gitea.

Human direction: this Phase 4 gate is approved as the right next verification scope for landed PRs `#1822` and `#1823`.

Session update: treat `#1823` as a full merge-coordinator implementation reported landed, not just a skeleton. Phase 4 should verify that claim against issue `#1805` acceptance criteria and the 2026-05-19 spec.

### PR `#1822`: Config-Error Circuit-Breaker and Guardrails

| Design/Acceptance Item | Evidence to Collect | Status Target |
|------------------------|---------------------|---------------|
| `ExitClass::ConfigError` is classified distinctly | Unit/integration test and code trace from `agent_run_record.rs` / exit classifier. | PASS |
| Three consecutive config errors quarantine an agent | `quarantine.rs` integration test plus trace to `AgentDefinition.enabled`. | PASS |
| Cron/scheduler skips disabled/quarantined agents | Scheduler/orchestrator test showing disabled agents are not spawned. | PASS |
| Memory watchdog files are present and documented | `systemd/` files and runbook verification. | PASS or explicit ops-only evidence |
| Bigbox sync runbook is safe | Script uses ff-only, refuses dirty tree, no destructive reset. | PASS |

Required verification commands after syncing to the Gitea commit:

```bash
cargo test -p terraphim_orchestrator quarantine --all-features
cargo test -p terraphim_orchestrator scheduler --all-features
ubs crates/terraphim_orchestrator/src/agent_run_record.rs crates/terraphim_orchestrator/src/lib.rs
```

### PR `#1823`: Merge-Coordinator Skeleton

The earlier Gitea PR body described a minimal skeleton, but the session handover reports the full `#1805` implementation landed across follow-up commits before merge. Verification should therefore test the complete claimed surface, not stop at crate-structure checks.

| Spec Requirement from `#1805` | Evidence to Collect | Current Risk |
|-------------------------------|---------------------|--------------|
| PID lock prevents concurrent execution | `pid_lock.rs` tests and review of stale lock handling. | Medium |
| Partial failure exits non-zero immediately | Evaluator/merge tests. | High until verified |
| Remediation failure prevents issue closure | Integration or real Gitea test. | High |
| Retry/backoff 1s/2s/4s | Gitea client tests or deterministic retry injection. | Medium |
| Structured JSON logs | `jsonlog.rs` tests and CLI output check. | Low |
| Exit code semantics 0/1/2 | Binary-level tests. | Medium |
| Token never visible in logs/process list | Security review of `gitea.rs` and command invocation. | High |
| Spec-validator returns PASS | Run spec-validator against `#1805` requirements. | Unknown |

Required verification commands after syncing to the Gitea commit:

```bash
cargo test -p terraphim_merge_coordinator --all-features
cargo test -p terraphim_orchestrator adf_check --all-features
ubs crates/terraphim_merge_coordinator/src/evaluator.rs crates/terraphim_merge_coordinator/src/gitea.rs crates/terraphim_merge_coordinator/src/main.rs crates/terraphim_merge_coordinator/src/pid_lock.rs
```

Verification gate: `#1805` must not be considered closed solely because `#1823` merged. It remains open in Gitea and needs traceability against every acceptance criterion.

Updated gate after session report: if Gitea now shows `#1805` closed, verification still needs to produce the same traceability evidence before accepting it as truly closed.

## Phase 5 Validation Plan

Validate the delivered self-healing work against the original research success criteria:

Human direction: this Phase 5 gate is approved as the right validation target, with the headline outcome of at least 70% overnight spawned-agent success by 2026-06-15.

| Success Criterion | Target | Validation Method | Evidence |
|------------------|--------|-------------------|----------|
| 24h spawned-agent success rate | >= 70% by 2026-06-15 | Query Quickwit/adf logs for exit classes over 24h windows. | Daily success-rate table. |
| Autonomous merged PRs | >= 2 per 24h | Gitea PR query filtered by ADF-created/merged PRs. | PR list and merge timestamps. |
| Config-error quarantine | 100% after 3 repeated config errors | Controlled broken-agent test plus production log observation. | Quarantine event and skipped dispatch. |
| Anthropic-only agents | 0 | Config audit of `conf.d/*.toml` and routing taxonomy. | Config report. |
| Z.AI status | Healthy via pi-rust route or vendor issue tracked | Re-run opencode and pi-rust probes; confirm `#1819` state. | Probe transcript. |
| Orchestrator OOM | 0 per week | systemd/journal review. | systemd status and journal excerpt. |
| Debug secrets visible | 0 | Static review plus focused tests on manual `Debug` impls. | Test output and code review. |

Phase 5 acceptance scenarios:

1. A deliberately broken agent reaches three `ConfigError` exits, is quarantined, and no further scheduled dispatch occurs.
2. A healthy local `.terraphim/adf.toml` agent validates and runs successfully through `adf agent run`, with JSON output containing project, working dir, layer, CLI/model, log path, and exit class.
3. A scheduled Core agent is simulated without waiting for wall-clock cron and can be fired once through the same runner.
4. A Gitea issue-comment `@adf:<agent>` payload is processed locally via `WebhookDispatch`, setting the expected issue context and optional output posting mode.
5. An evolution-enabled agent run reports whether evolution was enabled, whether context was injected, and whether a snapshot key was produced.
6. Merge-coordinator performs a dry run with structured JSON output and, in a controlled test repo, merges a PR containing `Fixes #N` and closes only that issue.

Validation gate: the full CLI runner is not accepted until it can run these scenarios without relying on SSH journal scraping or manual webhook curl.

### Live Self-Healing Validation Additions

The 2026-05-23 session shipped machinery whose value must be measured in production-like operation, not just unit tests.

| Validation Check | Expected Evidence |
|------------------|-------------------|
| Decision tier active | Journal/Quickwit entries showing `tier=decision` and routes to `opencode/gpt-5.5` or pi-rust where appropriate. |
| Z.AI route workaround active | Probe/runtime evidence that Z.AI via opencode is not selected, while pi-rust Z.AI route can complete. |
| Config-error quarantine active | A repeated config-error agent reaches quarantine and subsequent scheduler ticks skip it. |
| Merge-coordinator auto-close | Controlled PR with `Fixes #N` merges and closes only issue `N`; `Refs #N` does not close. |
| Memory watchdog installed | `systemctl show adf-orchestrator` or equivalent confirms `MemoryHigh=80G` and restart wiring. |
| Fresh clone path used | Bigbox runtime uses `/data/projects/terraphim/terraphim-ai-fresh` or a healthy repo path; corrupted path issues `#1818/#1821` are not silently reintroduced. |

These live checks become prerequisites before claiming the North Star target is on track.

### Coverage Check

Run after implementation:

```bash
cargo test -p terraphim_orchestrator adf_agent --all-features
cargo llvm-cov -p terraphim_orchestrator --all-features --summary-only
```

If `cargo llvm-cov` is unavailable, document that coverage could not be checked and run the crate tests plus `cargo test -p terraphim_orchestrator --all-features`.

## Implementation Steps

### Step 0: Rebase Plan on Gitea Main

**Files:** none initially
**Description:** Before coding, sync or branch from `gitea/main`, not local `origin/main`, because the self-healing session landed additional commits there. Confirm no local uncommitted work is overwritten. Treat the session's 11 commits and merged PRs as the baseline for the CLI runner design.
**Tests:** `git diff origin/main..gitea/main --stat` reviewed; no code test.
**Estimated:** 0.25 day

### Step 1: Extract Runtime Resolution

**Files:** `agent_runner.rs`, `lib.rs`, `config.rs`
**Description:** Add read-only runtime validation that resolves agent, project, working dir, Gitea target, evolution gates, schedule, CLI, and model config.
**Tests:** Unit tests for global, project, missing agent, missing project, and evolution reporting.
**Estimated:** 0.5 day

### Step 2: Add One-Shot Run API

**Files:** `agent_runner.rs`, `lib.rs`
**Description:** Implement `run_agent_once` around existing `spawn_agent`, then wait/poll until exit or timeout when requested. Keep the landed direct `AgentSpawner` path as a working baseline, then either subsume it into the one-shot orchestrator API or leave it as a compatibility wrapper over the new API. The report must include `(cli, provider, model)` because the session proved model health differs by CLI path.
**Tests:** Integration tests with real temp config and real short-lived command.
**Estimated:** 1 day

### Step 3: Add CLI `agent validate` and `agent run`

**Files:** `bin/adf.rs`
**Description:** Add subcommand parsing and human/JSON output. Preserve existing `adf CONFIG`, `adf --check CONFIG`, `adf --local --check`, `adf --local --agent`, help, and version behaviours.
**Tests:** CLI integration tests for JSON/human output and exit codes.
**Estimated:** 0.5 day

### Step 4: Add Gitea Trigger Simulation

**Files:** `agent_runner.rs`, `webhook.rs`, `bin/adf.rs`
**Description:** Convert CLI Gitea inputs or raw JSON into `WebhookDispatch` and execute via `handle_dispatch_once`.
**Tests:** Issue-comment direct agent trigger, event-only rejection/allowance semantics, PR dispatch report, push dispatch report.
**Estimated:** 1 day

### Step 5: Add Schedule Simulation

**Files:** `agent_runner.rs`, `scheduler.rs`, `bin/adf.rs`
**Description:** Report schedule metadata and optionally fire the target agent through a synthetic schedule event.
**Tests:** Scheduled Core agent, unscheduled Growth agent, Safety agent immediate-run report.
**Estimated:** 0.5 day

### Step 6: Reporting and Documentation

**Files:** `bin/adf.rs`, `.docs/summary.md`, command docs if present
**Description:** Finalise output fields, examples, and operator guidance.
**Tests:** Snapshot-like assertions on stable JSON keys, not brittle full strings.
**Estimated:** 0.5 day

## Rollback Plan

If the new commands regress existing daemon behaviour:

1. Revert the `agent` subcommand additions in `bin/adf.rs`.
2. Keep internal `agent_runner.rs` only if no public API is exposed and tests pass; otherwise revert it too.
3. Existing `adf CONFIG`, `adf --check CONFIG`, and `adf-ctl` remain the operational fallback.

## Dependencies

### New Dependencies

| Crate | Version | Justification |
|-------|---------|---------------|
| None preferred | N/A | Use existing `serde`, `tokio`, and current CLI parsing unless `clap` is already acceptable for this binary. |

### Dependency Updates

| Crate | From | To | Reason |
|-------|------|----|--------|
| None | N/A | N/A | Avoid dependency churn. |

## Performance Considerations

| Metric | Target | Measurement |
|--------|--------|-------------|
| `agent validate` | < 2s on normal config | Integration test wall time, informational only. |
| `agent run --wait` | Bounded by `--timeout` | Timeout test. |
| Schedule simulation | No wall-clock wait | Unit test confirms no sleep/cron wait needed. |

## Open Items

| Item | Status | Owner |
|------|--------|-------|
| Default Gitea posting mode | Needs decision | Human maintainer |
| Event-only direct run policy | Needs decision | Human maintainer |
| Whether raw webhook JSON is required in v1 | Needs decision | Human maintainer |
| Whether to move `adf` arg parsing to `clap` | Needs decision | Implementer |

## Approval

- [ ] Technical review complete.
- [ ] Test strategy approved.
- [ ] Gitea posting default decided.
- [ ] Event-only run policy decided.
- [ ] Human approval received before implementation.
- [x] Human approval received to start implementation using `opencode`/`minimax-coding-plan/MiniMax-M2.7-highspeed` pending `pi-rust` model support.
