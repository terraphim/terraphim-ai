# Implementation Plan: ADF Agent Validation & Verification Plan

**Status**: Draft
**Research Doc**: `.docs/research-adf-agent-validation-plan.md`
**Author**: Alex
**Date**: 2026-05-24
**Estimated Effort**: 3-4 days

## Overview

### Summary
Systematic validation of all 15 ADF agents across their four declared trigger modes (scheduled, webhook/mention, push/PR event, local CLI) using `adf agent validate` as the primary tooling, extended with synthetic trigger injection for modes that require a live orchestrator.

### Approach
- **Mode 1 (Global)**: Extend `adf agent validate` to include a minimal spawn attempt (not full run — just verify CLI resolves and model probe succeeds)
- **Mode 2 (Scheduled)**: Use `adf agent validate` + schedule-simulated invocation via `AgentOrchestrator::validate_agent_runtime`; cron firing is verified by inspection of `scheduler.rs` tick registration
- **Mode 3 (Webhook/Mention)**: Use `adf-ctl trigger <agent>` against a live orchestrator; verify `handle_webhook_dispatch` path exercised
- **Mode 4 (Push/PR Event)**: Inject synthetic `ADF_PUSH_*` / `ADF_PR_*` env vars and call `spawn_agent` directly via a new `adf agent run --synthetic-event` subcommand
- **Mode 5 (Local CLI)**: `adf --local --agent <name>` with output capture

### Scope

**In Scope:**
- Extend `adf agent validate` with spawn probe and model availability check
- New `adf agent run --synthetic-event <agent> [--pr | --push]` for event-only agents
- Per-agent validation reports in human and JSON format
- Nightly CI/CD pipeline (`adf agents validate-all --config CONFIG`)
- Documentation of each agent's trigger modes

**Out of Scope:**
- Real Gitea webhook injection (covered by Phase 5 overnight validation)
- LLM output quality assessment
- Multi-agent interaction tests
- Performance benchmarking

**Avoid At All Cost:**
- Full agent spawn with actual LLM calls in validation (too expensive, not CI-friendly)
- Mocking CLI tools (must test against real binaries)
- Real Git push/PR events in CI

## Architecture

### Component Diagram

```
┌─────────────────────────────────────────────────────────────┐
│  adf agent validate                                         │
│  ├── read-only: config parsing, working_dir, cli_tool     │
│  ├── spawn probe: CLI resolution, model availability       │
│  └── synthetic event: env var injection for event-only     │
└──────────────────────┬──────────────────────────────────────┘
                       │
        ┌──────────────┼──────────────────┐
        ▼              ▼                  ▼
   ┌─────────┐  ┌──────────┐     ┌──────────────┐
   │  Cron   │  │ Webhook/ │     │  Local CLI   │
   │ Schedule│  │ Mention  │     │  --local     │
   │(inspect)│  │ (ctl     │     │  --agent     │
   │         │  │ trigger) │     │              │
   └─────────┘  └──────────┘     └──────────────┘
```

### Data Flow

```
adf agent validate-all
  → OrchestratorConfig::from_file()
  → For each agent:
       Mode 1: validate_agent_runtime() → AgentRuntimeValidationReport
       Mode 2: verify schedule registered in TimeScheduler
       Mode 3: adf-ctl trigger (if orchestrator live) OR skip with note
       Mode 4: adf agent run --synthetic-event (env injection)
       Mode 5: adf --local --agent (output capture)
  → JSON report per agent per mode
  → CI gate: any runnable=false fails pipeline
```

## Agent Trigger Matrix

| Agent | Layer | Schedule | event_only | Mode 1 Global | Mode 2 Cron | Mode 3 Mention | Mode 4 Push/PR | Mode 5 Local |
|-------|-------|----------|------------|---------------|-------------|---------------|---------------|--------------|
| security-sentinel | Core | `0 */6 * * *` | No | validate | inspect | adf-ctl trigger | N/A | adf --local |
| runtime-guardian | Core | `15 0-10 * * *` | No | validate | inspect | adf-ctl trigger | N/A | adf --local |
| merge-coordinator | Growth | `0 */4 * * *` | No | validate | inspect | adf-ctl trigger | N/A | adf --local |
| meta-coordinator | Core | `*/30 * * * *` | No | validate | inspect | N/A | N/A | adf --local |
| meta-learning | Core | `0 11 * * *` | No | validate | inspect | N/A | N/A | adf --local |
| product-owner | Core | `25 0-10 * * *` | No | validate | inspect | adf-ctl trigger | N/A | adf --local |
| product-development | Core | `25 0-10 * * *` | No | validate | inspect | adf-ctl trigger | N/A | adf --local |
| repo-steward | Growth | `15 */6 * * *` | No | validate | inspect | N/A | N/A | adf --local |
| upstream-synchronizer | Core | `30 1 * * *` | No | validate | inspect | N/A | N/A | adf --local |
| pr-reviewer | Growth | None | Yes | validate | N/A | adf-ctl trigger | synthetic PR | adf --local |
| pr-security-sentinel | Growth | None | Yes | validate | N/A | adf-ctl trigger | synthetic PR | adf --local |
| pr-spec-validator | Safety | None | Yes | validate | N/A | adf-ctl trigger | synthetic PR | adf --local |
| pr-test-guardian | Growth | None | Yes | validate | N/A | adf-ctl trigger | synthetic PR | adf --local |
| pr-compliance-watchdog | Growth | None | Yes | validate | N/A | adf-ctl trigger | synthetic PR | adf --local |
| build-runner | Growth | None | Yes | validate | N/A | adf-ctl trigger | synthetic push | adf --local |

## File Changes

### New Files

| File | Purpose |
|------|---------|
| `crates/terraphim_orchestrator/src/agent_runner.rs` | (exists from Step 1 — extend with spawn probe) |
| `crates/terraphim_orchestrator/src/agent_run_command.rs` | New `adf agent run` subcommand for synthetic event injection |
| `crates/terraphim_orchestrator/tests/agent_validation_tests.rs` | Unit tests for all trigger modes per agent |
| `crates/terraphim_orchestrator/tests/agent_validation_integrity_tests.rs` | Integration tests for cross-mode consistency |

### Modified Files

| File | Changes |
|------|---------|
| `crates/terraphim_orchestrator/src/bin/adf.rs` | Add `agent run --synthetic-event [--pr N \| --push]` subcommand; extend `parse_args`; update help |
| `crates/terraphim_orchestrator/src/lib.rs` | Export new types from agent_runner |
| `crates/terraphim_orchestrator/src/agent_runner.rs` | Add spawn probe to `validate_agent_runtime`; add `run_agent_synthetic` |

## API Design

### Public Types (extensions to existing)

```rust
// In agent_runner.rs (extend existing)

/// Mode of agent trigger for validation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TriggerMode {
    Global,    // adf agent validate
    Cron,      // scheduled (verified by schedule registration check)
    Webhook,   // adf-ctl trigger OR synthetic event injection
    Push,      // synthetic push event
    PullRequest, // synthetic PR event
    Local,     // adf --local --agent
}

/// Extended report including per-mode results
#[derive(Debug, Clone, Serialize)]
pub struct AgentValidationReport {
    pub agent_name: String,
    pub mode_results: EnumMap<TriggerMode, ModeResult>,
    pub all_modes_runnable: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct ModeResult {
    pub runnable: bool,
    pub warnings: Vec<String>,
    pub spawn_probe_ok: Option<bool>,  // Some(true) if CLI resolved, None if not attempted
    pub synthetic_event_ok: Option<bool>,  // Some(true) if synthetic event dispatched
}

/// Synthetic event to inject for event-only agents
#[derive(Debug, Clone)]
pub enum SyntheticEvent {
    PullRequest {
        number: u64,
        head_sha: String,
        author: String,
        title: String,
        diff_loc: usize,
    },
    Push {
        sha: String,
        ref_name: String,
        pusher: String,
        files: Vec<String>,
    },
}
```

### New CLI Subcommands

```rust
// adf agent run --synthetic-event <agent> [--pr N | --push] [--project PROJECT] [--format json|human]
enum AgentRunMode {
    ValidateOnly,           // adf agent validate (existing)
    SyntheticPr { number: u64 },  // Inject ADF_PR_* env vars
    SyntheticPush,               // Inject ADF_PUSH_* env vars
    Local,                      // adf --local --agent
}
```

### Public Functions

```rust
/// Validate an agent's runtime in all applicable trigger modes
pub fn validate_agent_all_modes(
    config: &OrchestratorConfig,
    request: &AgentRunRequest,
    mode: TriggerMode,
) -> Result<ModeResult, OrchestratorError>;

/// Probe whether the agent's CLI tool resolves and is executable
pub fn probe_cli_tool(cli_tool: &str) -> Result<bool, OrchestratorError>;

/// Probe whether the agent's model is available via provider
pub fn probe_model_available(model: &str, provider: Option<&str>) -> Result<bool, OrchestratorError>;

/// Inject synthetic event env vars and attempt minimal agent run
pub fn run_agent_synthetic(
    config: &OrchestratorConfig,
    request: &AgentRunRequest,
    event: SyntheticEvent,
) -> Result<ModeResult, OrchestratorError>;
```

## Test Strategy

### Unit Tests

| Test | Location | Purpose |
|------|----------|---------|
| `test_probe_cli_tool_existing` | `agent_runner.rs` | Returns Ok(true) for `/bin/ls` |
| `test_probe_cli_tool_missing` | `agent_runner.rs` | Returns Ok(false) for `/nonexistent` |
| `test_probe_model_available_haiku` | `agent_runner.rs` | Sonnet/haiku availability probe |
| `test_validate_all_modes_security_sentinel` | `agent_validation_tests.rs` | All 5 modes for security-sentinel |
| `test_validate_all_modes_pr_reviewers` | `agent_validation_tests.rs` | All 4 modes for each PR agent |
| `test_synthetic_pr_injects_env_vars` | `agent_validation_tests.rs` | ADF_PR_* vars set correctly |
| `test_synthetic_push_injects_env_vars` | `agent_validation_tests.rs` | ADF_PUSH_* vars set correctly |
| `test_mode_result_serialization` | `agent_validation_tests.rs` | JSON round-trip for ModeResult |

### Integration Tests

| Test | Location | Purpose |
|------|----------|---------|
| `test_local_agent_spawns_and_exits` | `agent_validation_integrity_tests.rs` | `adf --local --agent` for each agent |
| `test_validate_all_agents_all_modes` | `agent_validation_integrity_tests.rs` | Full matrix across all 15 agents |
| `test_cron_agents_have_schedule` | `agent_validation_integrity_tests.rs` | Every cron agent has a non-empty schedule |
| `test_event_only_agents_no_schedule` | `agent_validation_integrity_tests.rs` | Every event-only agent has None schedule |
| `test_adf_ctl_trigger_fires_webhook` | `agent_validation_integrity_tests.rs` | `adf-ctl trigger` produces a 200 response |
| `test_synthetic_pr_fires_handler` | `agent_validation_integrity_tests.rs` | Synthetic PR triggers handle_webhook_dispatch |

### Property Tests

```rust
proptest! {
    #[test]
    fn probe_cli_tool_never_panics(path: String) {
        let _ = probe_cli_tool(&path);
    }

    #[test]
    fn synthetic_pr_env_vars_well_formed(number: u64, sha: String) {
        let event = SyntheticEvent::PullRequest {
            number,
            head_sha: sha,
            author: "test".to_string(),
            title: "Test PR".to_string(),
            diff_loc: 100,
        };
        let env = synthetic_event_env_vars(&event);
        prop_assert!(env.iter().all(|(k, _)| k.starts_with("ADF_PR_")));
    }
}
```

## Implementation Steps

### Step 1: Extend `agent_runner.rs` with spawn probe

**Files:** `crates/terraphim_orchestrator/src/agent_runner.rs`
**Tests:** `test_probe_cli_tool_existing`, `test_probe_cli_tool_missing`
**Estimated:** 4 hours

- Add `probe_cli_tool(path: &str) -> Result<bool>`
  - Uses `std::process::Command::new(path).arg("--version")` — if it resolves without error, the binary exists and is executable
  - Returns `Ok(false)` on ENOENT/permission denied; `Err` on unexpected errors
- Add `probe_model_available(model: &str, provider: Option<&str>) -> Result<bool>`
  - Uses `AgentSpawner::probe_model` (to be implemented in spawner) — pings the provider with a minimal request
  - Returns `Ok(false)` if model/provider unavailable; `Err` if probe fails for non-availability reasons
- Extend `AgentRuntimeValidationReport` with:
  - `cli_tool_probe: Option<bool>` — Some(true/false) if probe was attempted
  - `model_probe: Option<bool>` — Some(true/false) if probe was attempted
- Extend `validate_agent_runtime` to run both probes when `runnable == true`

### Step 2: Add synthetic event types and injection

**Files:** `crates/terraphim_orchestrator/src/agent_runner.rs`
**Tests:** `test_synthetic_pr_env_vars_well_formed`, `test_synthetic_push_env_vars_well_formed`
**Estimated:** 4 hours

- Add `SyntheticEvent` enum with `PullRequest` and `Push` variants
- Add `synthetic_event_env_vars(event: &SyntheticEvent) -> Vec<(String, String)>`
- Add `run_agent_synthetic(config, request, event) -> Result<ModeResult, OrchestratorError>`
  - Injects env vars via `SpawnContext`
  - Calls `spawn_agent` with a 10-second timeout (just enough to verify CLI resolves and first env var is read)
  - Returns `ModeResult` with `synthetic_event_ok: Some(true/false)`

### Step 3: Add `adf agent run` CLI subcommand

**Files:** `crates/terraphim_orchestrator/src/bin/adf.rs`
**Tests:** CLI integration tests in `adf_agent_cli_tests.rs`
**Estimated:** 4 hours

- Add `AgentRunMode` enum and `parse_agent_run_args()`
- Add `run_agent_run(config, agent_name, mode, project, format) -> ExitCode`
- New usage: `adf agent run --synthetic-event <agent> [--pr N | --push] [--project PROJECT] [--format json|human]`
- Output: `AgentValidationReport` in human or JSON format

### Step 4: Add `adf agent validate-all` command

**Files:** `crates/terraphim_orchestrator/src/bin/adf.rs`
**Tests:** Integration test `test_validate_all_agents_all_modes`
**Estimated:** 4 hours

- New usage: `adf agent validate-all --config CONFIG [--format json|human]`
- Iterates all agents in config
- For each agent, runs all applicable trigger modes
- Produces a `HashMap<AgentName, AgentValidationReport>`
- Exit code: 0 if all modes pass, 1 if any mode fails

### Step 5: Add schedule registration inspection

**Files:** `crates/terraphim_orchestrator/src/scheduler.rs`, `crates/terraphim_orchestrator/src/agent_runner.rs`
**Tests:** `test_cron_agents_have_schedule`, `test_event_only_agents_no_schedule`
**Estimated:** 2 hours

- Add `TimeScheduler::is_agent_scheduled(agent_name: &str) -> bool`
- Add `schedule_for_agent(config, agent_name) -> Option<String>` — returns cron expression if scheduled
- In validation: for cron-mode agents, verify schedule expression is set and parseable

### Step 6: Integration tests

**Files:** `crates/terraphim_orchestrator/tests/agent_validation_integrity_tests.rs`
**Estimated:** 6 hours

- `test_local_agent_spawns_and_exits`: for each agent, run `adf --local --agent` with a 60s timeout; verify process starts and exits (success or failure, but not panic)
- `test_validate_all_agents_all_modes`: run `adf agent validate-all --config <test_config>` and verify all modes report runnable
- `test_adf_ctl_trigger_fires_webhook`: start orchestrator in background, call `adf-ctl trigger security-sentinel`, verify webhook received

### Step 7: CI/CD integration

**Files:** `.github/workflows/adf-agent-validation.yml` (new)
**Estimated:** 2 hours

- Runs nightly at 02:00 UTC
- Runs `adf agent validate-all --config /opt/ai-dark-factory/orchestrator.toml --format json`
- Posts results to Gitea PR as JSON artifact
- Fails pipeline if any agent is `runnable: false` in any mode

## Rollback Plan

If issues discovered:
1. Revert `agent_runner.rs` extensions — existing `adf agent validate` remains functional
2. Remove new CLI subcommands from `adf.rs` — existing `run_check` continues to work
3. Disable nightly CI job

Feature flag: No feature flag needed — `adf agent validate-all` is additive.

## Performance Considerations

| Metric | Target | Measurement |
|--------|--------|-------------|
| Single agent validate (global mode) | < 2s | Benchmark |
| All 15 agents validate-all (no spawn probe) | < 30s | Benchmark |
| All 15 agents validate-all (with spawn probe) | < 5 min | Benchmark (spawn probe requires model probe) |
| CLI parse overhead | < 100ms | Benchmark |

**Note**: Spawn probe (Step 1) and model probe add latency. Make model probe optional via `--skip-model-probe` for fast CI runs.

## Open Items

| Item | Status | Owner |
|------|--------|-------|
| `AgentSpawner::probe_model` implementation in spawner | Pending | spawner crate |
| `adf-ctl trigger` verification against live orchestrator | Pending | Needs test env |
| Working dir existence verification for all agents | Pending | Step 1 |
| Schedule registration inspection | Pending | Step 5 |

## Approval

- [ ] Technical review complete
- [ ] Test strategy approved
- [ ] Performance targets agreed
- [ ] Human approval received
