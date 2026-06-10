# Design: terraphim_gitea_runner

**Status**: Active production crate
**Author**: Echo (implementation-swarm)
**Date**: 2026-06-11
**Gitea Issue**: #2394 (P2-2 gap — plan was missing)

---

## Executive Summary

`terraphim_gitea_runner` is the native Gitea Actions runner for Terraphim AI CI. It speaks the Gitea `RunnerService` Connect-RPC protocol directly (JSON wire format) and executes jobs under a Terraphim `PolicyPlanner` which routes each workflow step to the host or through `rch` (sccache-backed). Execution reuses the `terraphim_github_runner` session and executor stack (host-only in Milestone 1).

This document captures the design rationale, architecture, data flows, and operational model for the production crate.

---

## Problem Statement

### Background

The ADF (Agent-Driven Framework) originally used a `build-runner-llm` lane backed by GitHub Actions. Issue #1910 (polyrepo split, Milestone 1) introduced a native runner that communicates directly with the self-hosted Gitea instance, eliminating the GitHub intermediary and the associated cost and latency.

### Goals

1. Poll the Gitea RunnerService for tasks using the Connect-JSON protocol.
2. Route each workflow step via `PolicyPlanner` (host or `rch`).
3. Write results back as Gitea commit statuses via `SingleStatusWriter`.
4. Coexist with the interim ADF lane during migration via a repo allowlist.
5. Support liveness detection to recover from silent poll-hang scenarios.

---

## Architecture

### Component Diagram

```
[Gitea RunnerService]
        |
        | Connect-JSON (HTTP/2)
        v
 GiteaRunnerClient (client.rs)
        |
        v
  Poller (poller.rs)   ←── RunnerConfig (config.rs)
        |                     - poll_interval
        |                     - active_repos
        |                     - legacy_status_mirror
        v
  TaskWorker (task_worker.rs)
        |
        ├── WorkflowPayload → ParsedWorkflow (workflow_payload.rs / build_md.rs)
        |
        ├── PolicyPlanner (policy.rs)
        |       - CommandRoute::Host / CommandRoute::Rch
        |       - TrustLevel
        |
        ├── terraphim_github_runner (execution stack)
        |
        └── SingleStatusWriter (status.rs)
                - Writes commit statuses: terraphim-native/<job>
                - Optional legacy mirror: adf/build
```

### Data Flow

```
1. [Poller::run_forever]
   └─ loop:
       a. fetch_task(state, tasks_version=0) → Option<FetchedTask>
       b. If Some(task):
           i.  coexistence guard: check config.accepts_repo(repo_name)
           ii. If rejected: update_task(skip_task_state) → release claimed task
           iii. Else: TaskWorker::run(state, task)
       c. sleep(poll_interval)

2. [TaskWorker::run]
   └─ a. Decode WorkflowPayload → ParsedWorkflow
       b. For each step: PolicyPlanner::plan(step) → ExecutionPlan
       c. Execute via github_runner stack (host or rch)
       d. Batch-update logs via UpdateLog (logs.rs)
       e. Write commit status via SingleStatusWriter
```

---

## Modules

| Module | Purpose |
|--------|---------|
| `types` | Connect-JSON wire structs (`FetchTaskRequest`, `FetchTaskResponse`, `UpdateTaskRequest`, etc.) |
| `client` | `GiteaRunnerClient` trait + `HttpGiteaRunnerClient` over reqwest |
| `state` | Persisted runner identity: UUID, token, `tasks_version` stored in `.runner` file |
| `config` | `RunnerConfig` with instance URL, org, poll interval, repo allowlist, legacy mirror |
| `policy` | `PolicyPlanner` trait + `DeterministicPlanner`: command allowlist, host/rch routing, trust levels |
| `build_md` | Compile `BUILD.md` from a repository into a `ParsedWorkflow` (native format) |
| `checkout` | Sparse-checkout a target repository at the task's SHA before build |
| `workflow_payload` | Decode a Gitea `WorkflowPayload` into a `ParsedWorkflow` |
| `logs` | `UpdateLog` batching with monotonic ack sequence |
| `status` | `SingleStatusWriter`: post a single commit-status context, used for both native and legacy mirror |
| `task_worker` | End-to-end task execution: decode → plan → execute → log → status |
| `poller` | Outer fetch/dispatch loop with coexistence guard |
| `bin/terraphim-gitea-runner` | Main entry point: load config, register runner, call `Poller::run_forever` |

---

## Key Design Decisions

### Always Poll with `tasks_version = 0`

Gitea gates `PickTask` on `tasks_version != latestVersion` and bumps the version at run *creation* (before the job becomes `Waiting`). If we cached the returned version, a job that becomes `Waiting` after our last poll would never be offered until an unrelated version bump or a runner restart. Sending `0` forces a `PickTask` on every poll; the extra query cost is negligible compared to the reliability win. (Ref: #2185.)

### Coexistence Guard (`active_repos`)

During migration from the interim ADF lane, exactly one lane should process any given repo. `RunnerConfig::active_repos` is a whitelist: if non-empty, tasks for repos not in the list are released as `StatusSkipped` (result code 4) immediately after claim. This releases the task without recording a misleading failure in Gitea's run history. (Ref: #2185.)

### Legacy Status Mirror

During the migration window, the native runner can optionally write a second commit status under a legacy context (e.g. `adf/build`) alongside the native `terraphim-native/<job>` context. This is controlled by `RunnerConfig::legacy_status_mirror` and lets the existing ADF gate checks continue to pass without modification. Once migration is complete, the mirror is removed by setting the field to `None`.

### Execution Reuse from `terraphim_github_runner`

Rather than re-implementing the execution stack, `terraphim_gitea_runner` depends on `terraphim_github_runner` (M1: host-only, Milestone 2 adds Firecracker). This enforces policy at the runner level (step routing) while reusing the proven execution primitives.

---

## Configuration

### `RunnerConfig` Fields

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `instance_url` | `String` | `https://git.terraphim.cloud` | Gitea base URL |
| `org` | `String` | `terraphim` | Org scope for registration |
| `registration_token` | `Option<String>` | `None` | Token from 1Password; required on first run only |
| `state_file` | `PathBuf` | `.runner` | Persisted runner UUID + token |
| `labels` | `Vec<String>` | `["terraphim-native"]` | Labels advertised to Gitea |
| `poll_interval` | `Duration` | 3 seconds | FetchTask polling frequency |
| `active_repos` | `Vec<String>` | `[]` (accept all) | Coexistence allowlist |
| `legacy_status_mirror` | `Option<...>` | `None` | Optional legacy context mirror |

---

## Error Model

`RunnerError` variants:

| Variant | Cause |
|---------|-------|
| `Protocol(String)` | Gitea RunnerService RPC failure |
| `State(String)` | `.runner` file read/write failure |
| `Compile(String)` | Workflow payload decode failure |
| `PolicyRejected(String)` | Policy planner denied a command |
| `Execution(String)` | Execution via github_runner stack failed |

---

## Known Gaps and Open Issues

| Issue | Description |
|-------|-------------|
| #2119 | Silent poll-hang: add liveness watchdog (heartbeat + self-restart if no successful poll in N minutes) |
| #2189 | P1/P2 structural review findings (test isolation, state atomicity, FetchTask claim/release) |
| #2109 | W4: port performance benchmarking to Gitea Actions |
| #2078 | M2: broad `uses:` action emulation |
| #2077 | M2: build artifact upload/download |
| #2076 | M2: Firecracker execution route |

---

## Testing

### Unit Tests

Located in `src/` modules using `#[tokio::test]`. Key test suites:

- `client.rs` — mock HTTP responses for FetchTask/UpdateTask
- `policy.rs` — allowlist matching, trust level assignment, route decisions
- `poller.rs` — `poll_once` with injected mock client (stub server via `axum`)
- `task_worker.rs` — end-to-end single-task execution with mock client

### Integration Tests (Target)

- `tests/protocol_smoke.rs` — spin up a minimal Axum stub server, run `poll_once`, verify UpdateTask is called
- `tests/poller_reliability.rs` — liveness watchdog triggers after configurable timeout

---

## Milestones

| Milestone | Status | Description |
|-----------|--------|-------------|
| M1 | Done | Register runner, poll tasks, execute via host, write commit statuses |
| M2 | Planned | `uses:` emulation, artifact upload/download, Firecracker execution route |
| M3 | Planned | Multi-worker concurrency, rate-limit handling, metrics export |

---

## References

- Gitea issue #1910: polyrepo split / native runner introduction
- Gitea issue #2185: FetchTask double-claim and stuck-run race
- Gitea issue #2119: liveness watchdog
- Gitea issue #2189: structural review P1/P2
- `crates/terraphim_github_runner/` — shared execution stack
- `docs/plans/github-to-gitea-ci-migration.md` — migration strategy
