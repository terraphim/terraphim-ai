# Design & Implementation Plan: ADF OTP Orchestrator

## 1. Summary of Target Behaviour

ADF will move from a single large `AgentOrchestrator` loop toward an OTP-style orchestration core while preserving production behaviour during migration. The new implementation is a strangler refactor inside the existing `terraphim_orchestrator` crate, not a replacement daemon in the first slice.

The target implementation introduces an explicit agent registry, project controllers, run supervisors, and event actors. Existing proven behaviours stay intact while their ownership moves behind smaller boundaries.

The first implementation slice must cover both normal agent dispatch and deterministic `adf/build` dispatch. `build-runner` remains a first-class event-only agent that can be launched from push webhooks and PR fan-out with `ADF_PUSH_*` environment variables, posts `adf/build` pending only after successful spawn, and always resolves terminal commit status on exit.

The OTP orchestrator will maintain the authoritative runtime registry of agents. The registry will compile global fleet agents, include-fragment agents, and repo-local `.terraphim/adf.toml` agents into project-scoped entries. It will preserve the existing merge and validation rules from `OrchestratorConfig::from_file`, while exposing lookup APIs that prevent accidental global-name collisions.

This design intentionally leverages existing implementation:

| Existing Implementation | Reused For | Notes |
|-------------------------|------------|-------|
| `OrchestratorConfig::from_file`, `merge_project_sources`, `validate` | Config loading and merge semantics | Keep as source of truth for initial migration |
| `project_adf.rs` | Repo-local ADF config conversion | Keep project id checks and agent conversion |
| `dispatcher.rs` | Dispatch queue and project fairness | Wrap rather than replace initially |
| `handle_push`, `dispatch_build_runner_for_pr`, `post_pending_status`, `post_terminal_commit_status` | `adf/build` behaviour | Extract into build dispatch service after tests lock behaviour |
| `ManagedAgent`, `poll_agent_exits`, `drain_output_events` | Runtime lifecycle reference path | Move behind run supervisor incrementally |
| `terraphim_symphony::orchestrator::state` | Serialised runtime state concepts | Borrow `running`, `claimed`, `retrying`, `completed` semantics |
| `terraphim_agent_supervisor` | OTP restart and lifecycle primitives | Use as child supervisor substrate, not as config registry |
| `terraphim_spawner` | Subprocess execution and output capture | Keep for CLI runtime adapter |
| `.docs/research-tlaplus-symphony-validation.md` and `tlaplus-ts` | Formal state-machine validation | Use bounded model checking as a refactor guardrail for registry, build dispatch, and supervision |

## 2. Key Invariants and Acceptance Criteria

### Invariants

| Invariant | Guarantee |
|-----------|-----------|
| Project-scoped identity | Runtime keys use `(project_id, agent_name)` or an explicit legacy scope; no global `build-runner` lock can block another project's `build-runner`. |
| Registry authority | Runtime dispatch never scans `config.agents` directly after registry construction; it asks `AgentRegistry`. |
| Merge compatibility | Global includes and `project_sources` produce the same effective projects, agents, and `pr_dispatch_per_project` as current `OrchestratorConfig::from_file`. |
| `adf/build` coverage | Push and PR-open paths can launch `build-runner`, inject the correct `ADF_PUSH_*` keys, post `adf/build` pending after spawn, and post terminal success/failure/error on exit. |
| Event-only safety | `event_only` agents cannot be dispatched by mention/comment paths. |
| Fail closed on workspace creation | Mutating agents, including `build-runner` where worktree isolation is required, do not fall back to shared checkout on worktree creation failure. |
| Slow external work isolation | Provider probing, PR gate reconciliation, mention polling, and workspace sweep do not block safety-critical run exit polling. |
| Backward compatibility | Existing configs keep loading through `OrchestratorConfig::load_and_validate`; no production TOML migration is required for the first slice. |
| No secret leakage | Registry snapshots and diagnostics redact tokens and do not print sensitive config values. |
| Durable observability | Run lifecycle emits structured events that can later feed Quickwit and Gitea comments; text output remains human-facing only. |
| Formal safety guardrail | Registry, build dispatch, and run supervision transitions are modelled with bounded TLA+ checks before their implementation is considered complete. |

### Acceptance Criteria

| ID | Criterion |
|----|-----------|
| AC1 | `AgentRegistry::from_config(&config)` returns project-scoped entries for global include agents and repo-local project-source agents. |
| AC2 | Duplicate agent names are allowed across different projects and rejected within the same project. |
| AC3 | `AgentRegistry::lookup(project, "build-runner")` returns the project-local `build-runner` when present and does not return a different project's runner. |
| AC4 | Push webhook dispatch through the new build dispatch path posts `adf/build` pending only when `build-runner` was spawned. |
| AC5 | PR-open fan-out through the new build dispatch path posts both `adf/build` and `adf/pr-reviewer` pending when both agents spawn. |
| AC6 | A skipped/gated `build-runner` does not post `adf/build` pending. |
| AC7 | Build-runner exit success posts `adf/build=success`; build-runner exit failure posts `adf/build=failure`; spawn failure posts `adf/build=error` or no pending depending on whether a run id was created. |
| AC8 | Mention dispatch cannot launch `event_only` agents, including `build-runner`. |
| AC9 | Provider probe actor can time out probes at 15 seconds without causing `reconcile_tick exceeded timeout`. |
| AC10 | The old `AgentOrchestrator` remains deployable behind the new boundaries until each call site is migrated. |
| AC11 | TLA+ model checking covers the first registry/build/supervision slice and proves no project-crossing lookup, no duplicate active build run per project+sha, bounded retry, and eventual terminal status after `adf/build` pending. |

## 3. High-Level Design and Boundaries

### Component Diagram

```text
orchestrator.toml + conf.d + project_sources
                 |
                 v
        OrchestratorConfig::load_and_validate
                 |
                 v
             AgentRegistry
                 |
                 v
Fleet Kernel -> ProjectController(project_id) -> DispatchService
     |                 |                         |
     |                 |                         v
     |                 |                  RunSupervisor
     |                 |                         |
     |                 |                         v
     |                 |                  RuntimeAdapter
     |                 |
     |                 v
     |          BuildDispatchService -> adf/build statuses
     |
     v
Background actors: ProviderHealthActor, PrGateActor, MentionActor, WorkspaceSweepActor, ActivityJournalActor
```

### Boundary Definitions

| Component | Responsibility | Must Not Own |
|-----------|----------------|--------------|
| `FleetKernel` | Load config, build registry, start/stop supervised actors, expose shutdown orchestration | Individual agent spawn details, provider probe subprocesses, PR gate API loops |
| `AgentRegistry` | Authoritative lookup, merge result indexing, project-scoped identity, registry diagnostics | Loading raw TOML from disk, spawning processes, posting statuses |
| `ProjectController` | Per-project dispatch decisions, project concurrency, queue drains, project pause/circuit breaker checks | Global config merge, runtime process management, provider probe execution |
| `DispatchService` | Convert mention/issue/cron/webhook intents into `RunRequest` objects | Directly managing `AgentHandle` lifecycle |
| `BuildDispatchService` | Specialised deterministic CI path for `build-runner` and `adf/build` statuses | General LLM routing or mention dispatch |
| `RunSupervisor` | Own one agent run lifecycle: spawn, output subscription, timeout, exit, terminal state, cleanup | Agent registry mutation, config loading, project-source merging |
| `RuntimeAdapter` | Execute subprocess/Claude SDK/pi-rust runtimes behind one event contract | Dispatch eligibility, PR gate policy, Gitea issue state |
| `ProviderHealthActor` | Run health probes on independent cadence and cache verdicts | Blocking project dispatch loop |
| `PrGateActor` | Reconcile PR gate state with capability cache and backoff | Build-runner subprocess execution |
| `ActivityJournalActor` | Append structured lifecycle events and feed sinks | Business decisions about whether a run should start |

### OTP Relationship

`RunSupervisor` is an ADF-specific lifecycle owner that uses `terraphim_agent_supervisor` concepts and, where practical, the existing `AgentSupervisor` primitives. It is not a duplicate agent registry.

The existing OTP crate manages child process lifecycle, restart policy, and failure isolation. The new ADF registry manages declarative agent definitions and project scope. The bridge is:

| ADF Concept | OTP/Supervisor Concept |
|-------------|------------------------|
| `RegisteredAgent` | Input used to build an `AgentSpec` |
| `RunRequest` | Request to spawn a supervised child |
| `RunId` | ADF lifecycle id mapped to `AgentPid` |
| `RunSupervisor` | ADF facade over `AgentSupervisor` plus spawner integration |
| `RunEvent` | Lifecycle and output event emitted to the activity journal |

### Registry Model

The new registry is constructed after current config loading and validation. This avoids changing merge semantics during the first slice.

Core types to introduce:

```text
AgentScope = Legacy | Project(String)
AgentKey = { scope: AgentScope, name: String }
RegisteredAgent = { key, definition, project, source, event_only, runtime_kind }
AgentRegistry = { by_key, by_project, diagnostics }
```

Source attribution is important for debugging merge behaviour:

| Source | Meaning |
|--------|---------|
| `BaseConfig` | Agent declared in top-level `orchestrator.toml` |
| `IncludeFragment(path)` | Agent declared in `conf.d/*.toml` |
| `ProjectSource { id, root, config }` | Agent declared in repo-local `.terraphim/adf.toml` |

Initial implementation may only store `ConfigMerged` as source because `OrchestratorConfig` currently loses per-agent provenance during merge. A later enhancement can preserve precise source paths by extending `merge_project_sources` and include parsing.

### `adf/build` Design

`adf/build` becomes a first-class build dispatch flow rather than duplicated logic in `handle_push` and `dispatch_build_runner_for_pr`.

`BuildDispatchService` inputs:

| Input | Push Event | PR Open Event |
|-------|------------|---------------|
| `project_id` | Gitea project id | Gitea project id |
| `head_sha` | Push `after_sha` | PR head SHA |
| `ref_name` | Push ref | `refs/pull/<pr>/head` |
| `before_sha` | Push `before_sha` | Empty or PR base SHA when available |
| `actor` | Pusher login | PR author login |
| `files_changed` | Commit file list | Empty unless PR diff files are available |
| `status_context` | `adf/build` | `adf/build` |

`BuildDispatchService` behaviour:

| Step | Behaviour |
|------|-----------|
| Lookup | Resolve `(project_id, build-runner)` through `AgentRegistry`. |
| Gate | Apply event-only safety, provider allow-list defensive check, budget check, project pause/circuit breaker, and active-run check using project-scoped `AgentKey`. |
| Spawn | Build `RunRequest` with `def.task`, `def.cli_tool`, resource limits, working dir, and `ADF_PUSH_*` env. |
| Status pending | Post `adf/build=pending` only after `RunSupervisor` confirms spawn. |
| Terminal status | Convert run exit to `success`, `failure`, or `error` exactly once. |
| Activity | Emit `BuildStarted`, `BuildStatusPosted`, `BuildFinished`, and `BuildStatusPostFailed` events. |

This removes the current risk where `active_agents.contains_key("build-runner")` is global and can suppress another project's build-runner. It also makes `adf/build` part of the same run lifecycle model as other agents.

### Formal Validation Boundary

The existing TLA+ validation research should be used as a design constraint for this refactor. The refactor changes ownership of shared state, so it must not rely only on example-based Rust tests. TLA+ is used to validate the finite state machines that are most likely to regress during extraction.

TLA+ will model at the message-passing boundary, not at individual Rust `await` points. The model is intentionally bounded so it can run quickly in CI and during local development.

| Model | Scope | Safety Properties | Liveness Properties |
|-------|-------|-------------------|---------------------|
| `AdfRegistry` | Global agents, include agents, project-source agents, lookup by `(project, name)` | No same-project duplicate; cross-project duplicate allowed; lookup never returns another project's agent | Every enabled source is either indexed or rejected with a validation error |
| `AdfBuildDispatch` | Push event, PR-open event, missing agent, gated agent, spawn success/failure, run exit | No `adf/build` pending before spawn success; skipped build-runner posts no pending; at most one active build per `(project, sha)` | Any posted `adf/build` pending eventually reaches success, failure, error, or recovery-needed |
| `AdfRunSupervisor` | Spawn, running, timeout, normal exit, failure exit, retry, shutdown, escalation | RetryBound; NoRestartAfterEscalation; terminal status emitted at most once; active slot released on terminal state | A non-shutdown run eventually reaches terminal or retry-waiting state |
| `AdfProviderHealthActor` | Probe tick, probe timeout, cached verdict, stale cache, dispatch read | Probe delay does not block run-exit processing; stale cache is explicit | Probe actor eventually refreshes or marks stale when runtime is available |

Initial bounded constants:

| Constant | Value | Rationale |
|----------|-------|-----------|
| Projects | 2 | Proves project-scoped lookup and cross-project `build-runner` behaviour |
| Agents per project | 2 | Covers `build-runner` plus one non-build agent |
| Events | 2 push/PR events | Enough to expose duplicate dispatch and pending-status races |
| Max retries | 1 or 2 | Enough to verify retry bound and escalation transitions |
| Provider probes | 1 slow, 1 fast | Enough to prove probe isolation from lifecycle events |

The TLA+ checks are not a substitute for Rust tests. They are a pre-flight and regression guard for invariants that are hard to cover exhaustively with async tests.

## 4. File/Module-Level Change Plan

| File/Module | Action | Before | After | Dependencies |
|-------------|--------|--------|-------|--------------|
| `crates/terraphim_orchestrator/src/agent_registry.rs` | Create | No dedicated registry; code scans `config.agents` | Owns `AgentKey`, `AgentScope`, `RegisteredAgent`, `AgentRegistry`, lookup APIs, diagnostics | `config.rs` |
| `crates/terraphim_orchestrator/src/lib.rs` | Modify | `AgentOrchestrator` owns config scans, active map, dispatch, lifecycle, statuses | Constructs registry and delegates build dispatch/run lifecycle through services | `agent_registry`, `build_dispatch`, `run_supervisor` |
| `crates/terraphim_orchestrator/src/build_dispatch.rs` | Create | Push and PR build-runner logic duplicated in `lib.rs` | Single `BuildDispatchService` handles push and PR `adf/build` | `agent_registry`, `status_service`, `run_supervisor`, `dispatcher` |
| `crates/terraphim_orchestrator/src/run_supervisor.rs` | Create | `ManagedAgent` lifecycle lives in `AgentOrchestrator` | ADF facade for spawn, output subscription, timeout, exit classification, cleanup | `terraphim_spawner`, `terraphim_agent_supervisor` |
| `crates/terraphim_orchestrator/src/status_service.rs` | Create | Pending/terminal status helpers are private methods in `lib.rs` | Reusable service for commit status pending/terminal/error with context | `terraphim_tracker`, workflow config |
| `crates/terraphim_orchestrator/src/runtime_adapter.rs` | Create | Spawner calls are embedded in dispatch paths | Trait boundary for subprocess first, SDK/pi-rust later | `terraphim_spawner` |
| `crates/terraphim_orchestrator/src/project_controller.rs` | Create | Project policy spread across config helpers and `lib.rs` | Per-project controller owns dispatch queue drains and project-scoped concurrency | `dispatcher`, `agent_registry`, `project_control` |
| `crates/terraphim_orchestrator/src/provider_health_actor.rs` | Create | Provider probe called inside `reconcile_tick` | Independent actor updates cached provider health with TTL | `provider_probe.rs` |
| `crates/terraphim_orchestrator/src/activity_journal.rs` | Create | Logs and output posts are the main audit trail | Append structured `RunEvent` / `BuildEvent` records and fan out to sinks | Quickwit later, Gitea output later |
| `crates/terraphim_orchestrator/tla/AdfRegistry.tla` | Create | No ADF-specific registry model | Bounded model for global/local agent merge and project-scoped lookup | `tlaplus-ts`, registry design |
| `crates/terraphim_orchestrator/tla/AdfBuildDispatch.tla` | Create | No build dispatch formal model | Bounded model for `adf/build` pending/terminal lifecycle | `BuildDispatchService` design |
| `crates/terraphim_orchestrator/tla/AdfRunSupervisor.tla` | Create | Supervisor invariants exist in code/docs but not ADF run model | Bounded model for retry, escalation, terminal status, active slot release | `RunSupervisor` design, `terraphim_agent_supervisor` |
| `crates/terraphim_orchestrator/tests/tla_validation_tests.rs` | Create | No orchestrator TLA+ test harness | Rust/TypeScript-invoked smoke test that runs bounded TLC checks where tooling is available | `tlaplus-ts`, TLC Java |
| `crates/terraphim_orchestrator/src/config.rs` | Modify narrowly | Merge produces `projects`, `agents`, `pr_dispatch_per_project` only | Add optional helpers to construct registry; preserve current merge semantics | `agent_registry` tests |
| `crates/terraphim_orchestrator/src/dispatcher.rs` | Modify narrowly | Queue stores dispatch tasks; main tick drains | Project controller drains through registry-aware dispatch services | `project_controller` |
| `crates/terraphim_orchestrator/tests/project_source_tests.rs` | Extend | Tests config merge behaviour | Add registry construction and lookup coverage for project-source agents | `agent_registry` |
| `crates/terraphim_orchestrator/tests/provider_gate_tests.rs` | Extend if needed | Tests provider/budget gates | Add build-runner defensive provider-gate no-op and rejection cases if moved | `build_dispatch` |
| `crates/terraphim_orchestrator/src/lib.rs` unit tests | Move or duplicate first | Existing PR fan-out and build-runner tests live in large module | New service-level tests lock `adf/build` behaviour before extraction | `build_dispatch`, `status_service` |
| `.docs/design/adf-otp-orchestrator-implementation-plan-2026-05-21.md` | Create | Phase 2 plan absent | Contract for Phase 3 implementation | Phase 1 research |

## 5. Step-by-Step Implementation Sequence

1. Add formal validation pre-flight for the first registry/build/supervision slice.

Purpose: Convert the highest-risk state transitions into bounded TLA+ models before implementation changes move ownership of state.

Deployable state: Yes, documentation/spec/test-harness only.

Feature flag needed: No. CI may mark TLC unavailable as skipped until the toolchain is installed.

Tests: TLC passes for `AdfRegistry`, `AdfBuildDispatch`, and `AdfRunSupervisor` with bounded constants.

2. Add `AgentRegistry` as a read-only index over existing `OrchestratorConfig`.

Purpose: Establish project-scoped agent identity without changing runtime behaviour.

Deployable state: Yes.

Feature flag needed: No.

Tests: Unit tests for `(project, agent)` lookup, duplicate handling, legacy mode, and `build-runner` lookup.

3. Replace direct `config.agents.iter().find(...)` lookups in build-runner paths with registry lookups.

Purpose: Fix global-name coupling for `build-runner` while preserving existing spawn code.

Deployable state: Yes.

Feature flag needed: No.

Tests: Existing PR fan-out tests plus new multi-project build-runner lookup test.

4. Introduce `AgentKey`-keyed active run tracking for build-runner.

Purpose: Prevent one project's active `build-runner` from suppressing another project's `build-runner`.

Deployable state: Yes if compatibility accessors preserve existing tests.

Feature flag needed: No.

Tests: Two projects can run `build-runner` concurrently; duplicate same-project `build-runner` is skipped.

5. Extract commit status posting into `StatusService`.

Purpose: Make `adf/build`, `adf/pr-reviewer`, and future contexts share one status contract.

Deployable state: Yes.

Feature flag needed: No.

Tests: Pending and terminal post success/failure with local Axum test server, preserving existing request shape.

6. Extract `BuildDispatchService` from `handle_push` and `dispatch_build_runner_for_pr`.

Purpose: Make `adf/build` a first-class dispatch flow with one implementation for push and PR events.

Deployable state: Yes.

Feature flag needed: Optional `adf_build_dispatch_service` if risk is high, but prefer direct replacement after test parity.

Tests: Push env injection, PR env injection, skipped build-runner no pending, spawned build-runner pending, terminal status on exit.

7. Introduce `RunRequest`, `RunId`, and `RunSupervisor` facade while still using `terraphim_spawner` internally.

Purpose: Move lifecycle ownership out of direct `AgentOrchestrator` maps.

Deployable state: Yes if initially only build-runner uses it.

Feature flag needed: Use build-runner-only adoption first.

Tests: Spawn success, spawn failure, output subscription, exit classification, timeout, terminal event exactly once.

8. Bridge `RunSupervisor` to `terraphim_agent_supervisor` primitives.

Purpose: Reuse OTP restart policies and child tracking while keeping ADF-specific run metadata.

Deployable state: Yes behind the facade.

Feature flag needed: `adf_run_supervisor_otp_bridge` until parity is proven.

Tests: restart policy mapping, no restart after escalation, stop all on shutdown, status snapshot.

9. Move provider health into `ProviderHealthActor`.

Purpose: Remove slow provider probes from the main reconcile loop.

Deployable state: Yes if routing uses cached health and falls back to unknown/stale verdicts.

Feature flag needed: Yes, `provider_health_actor` for quick rollback.

Tests: 15s probe timeout, TTL cache, stale health behaviour, tick does not wait for probe completion.

10. Introduce `ProjectController` for one project and route only `terraphim-ai` through it in shadow mode.

Purpose: Validate per-project queue draining and registry lookup without disrupting the full fleet.

Deployable state: Yes with shadow logs only.

Feature flag needed: Yes, `project_controller_shadow`.

Tests: dispatch eligibility, project concurrency, pause flag, circuit breaker, no duplicate claims.

11. Switch build-runner dispatch to project controller path.

Purpose: Prove `adf/build` under the new OTP/project-controller architecture.

Deployable state: Yes after service-level and integration tests pass.

Feature flag needed: Yes for first production deploy.

Tests: live-style push dispatch integration test with fake tracker and fake spawner.

12. Add `ActivityJournalActor` for run/build events.

Purpose: Create a structured source of truth for lifecycle events before migrating more agents.

Deployable state: Yes as append-only side effect.

Feature flag needed: No if fail-open on write errors.

Tests: event append, redaction, ordering by `RunId`, Quickwit sink disabled failure path.

13. Migrate PR review and mention dispatch to `DispatchService` and `RunSupervisor`.

Purpose: Reduce `AgentOrchestrator` responsibility after build-runner parity is proven.

Deployable state: Yes incrementally by dispatch kind.

Feature flag needed: Per-dispatch-kind flags during rollout.

Tests: current mention, PR fan-out, output posting, and timeout tests moved to service-level tests.

14. Retire direct config scans and global active-agent keys.

Purpose: Complete registry and project-scope migration.

Deployable state: Yes after all dispatch paths use registry.

Feature flag needed: No, cleanup step.

Tests: grep-based or unit assertion that dispatch paths do not use direct `config.agents.iter().find` for runtime lookup.

15. Decompose remaining background work into actors.

Purpose: Move PR gates, mention polling, workspace sweep, telemetry, and learning archive out of the monolithic tick.

Deployable state: Yes actor by actor.

Feature flag needed: Actor-specific flags for PR gate and mention polling.

Tests: actor cadence, backoff, shutdown, and failure isolation tests.

## 6. Testing & Verification Strategy

| Acceptance Criteria | Test Type | Test Location |
|---------------------|-----------|---------------|
| AC1 | Unit | `crates/terraphim_orchestrator/src/agent_registry.rs` tests |
| AC2 | Unit and config integration | `crates/terraphim_orchestrator/tests/project_source_tests.rs` |
| AC3 | Unit | `agent_registry.rs` tests with two projects both declaring `build-runner` |
| AC4 | Integration with fake tracker and fake spawner | `crates/terraphim_orchestrator/tests/build_dispatch_tests.rs` |
| AC5 | Integration with local Axum status API | Existing `handle_review_pr_pending_status_posted_per_agent`, moved or mirrored in `build_dispatch_tests.rs` |
| AC6 | Integration | Existing skipped-agent no-pending test, moved or mirrored in `build_dispatch_tests.rs` |
| AC7 | Unit and integration | `run_supervisor.rs` tests plus `build_dispatch_tests.rs` terminal status cases |
| AC8 | Regression | Existing mention dispatch tests plus new registry-aware event-only test |
| AC9 | Actor unit and integration | `provider_health_actor.rs` tests and `provider_gate_tests.rs` |
| AC10 | Full crate regression | `cargo test -p terraphim_orchestrator`, `adf --check`, bigbox smoke test |
| AC11 | Formal model checking | `crates/terraphim_orchestrator/tla/*.tla` plus `tests/tla_validation_tests.rs` |

Required verification commands for Phase 3 changes:

```bash
cargo fmt --all -- --check
cargo clippy -p terraphim_orchestrator -- -D warnings
cargo test -p terraphim_orchestrator --test project_source_tests
cargo test -p terraphim_orchestrator --test provider_gate_tests
cargo test -p terraphim_orchestrator --test build_dispatch_tests
cargo test -p terraphim_orchestrator --test tla_validation_tests
cargo test -p terraphim_orchestrator
cargo llvm-cov -p terraphim_orchestrator --summary-only
ubs <changed-rust-files>
```

Production validation on bigbox after the build-runner slice:

| Check | Evidence |
|-------|----------|
| Service starts | `systemctl is-active adf-orchestrator` returns `active` |
| Registry load | Logs show project and agent counts without secret values |
| Push build | New push to task branch creates `adf/build=pending` then terminal status |
| PR build | PR-open or synchronise path creates `adf/build` and `adf/pr-reviewer` pending when configured |
| No global collision | Two project-scoped build-runners can be active independently in test or staging |
| Tick health | No `reconcile_tick exceeded timeout`; provider health actor logs independent probe completion |

## 7. Risk & Complexity Review

| Risk | Mitigation | Residual Risk |
|------|------------|---------------|
| Registry changes alter effective config semantics | Build registry after existing validated config first; do not change `from_file` merge semantics in slice 1 | Source provenance remains coarse until later enhancement |
| `adf/build` remains duplicated and diverges | Extract `BuildDispatchService` after tests capture push and PR parity | Short period of duplication during extraction |
| Project-scoped active keys break existing code expecting `active_agents[agent_name]` | Introduce compatibility helper and migrate build-runner first | Some tests may need updates to inspect `AgentKey` rather than string names |
| OTP supervisor crate is not a perfect fit for subprocess handles | Wrap it behind `RunSupervisor`; use spawner directly until bridge is proven | Bridge may need adapter code rather than direct reuse |
| Provider health actor uses stale data | Add TTL, age in routing diagnostics, and safe fallback when cache is empty | Routing may choose a temporarily unhealthy provider if cache is stale |
| More modules increase apparent complexity | Keep each module narrow and remove duplicated paths after migration | Temporary complexity during strangler period |
| Commit status can remain pending if process crashes after pending post | RunSupervisor owns terminal status finalisation and emits recovery event on startup for orphaned runs | Crash between external status post and journal write can still need reconciliation |
| Build-runner workspace `cargo clippy --workspace` failures block PR #1782 | Treat as separate build gate issue; design keeps terminal failure visible instead of hiding it | PR stays red until broader workspace build issue is fixed |
| Background actors fail silently | OTP supervision restarts actors and activity journal records actor exits | Repeated restart storm requires operator alerting |
| TLA+ model diverges from Rust implementation | Keep the model small and tie each invariant to a Rust test; update model in the same PR as state-machine changes | The model will not capture every async scheduling nuance |
| TLA+ toolchain blocks development when unavailable | Make local smoke tests skip with an explicit reason when TLC/tlaplus-ts is absent; CI should install the toolchain before enforcing | A skipped local check can miss a violation until CI |

## 8. Open Questions / Decisions for Human Review

| Question | Recommended Decision |
|----------|----------------------|
| Should this be a new daemon or in-place refactor? | In-place strangler refactor first; new daemon shell only after actor boundaries are stable. |
| Should Symphony become the canonical dispatcher? | Borrow its serialised state model now; defer full crate integration until project controller slice. |
| Should `RunSupervisor` be identical to `terraphim_agent_supervisor::AgentSupervisor`? | No. Make it an ADF facade that can use `AgentSupervisor` internally. |
| Should provider health be fully decoupled from `reconcile_tick`? | Yes, behind a feature flag and TTL cache. |
| Should `adf/build` be migrated first? | Yes. It is deterministic, currently failing visibly, and proves registry, project scope, status posting, and run supervision. |
| Should the registry preserve exact source paths in slice 1? | No. Start with registry over merged config; add provenance once dispatch paths are registry-backed. |
| Should `event_only` remain an `AgentDefinition` field? | Yes. Registry should index it and dispatch services should enforce it. |
| Should project-local agents override global agents with the same name? | No implicit override. Same-project duplicates remain invalid; cross-project same names remain valid. |
| Should TLA+ be required before the first refactor slice merges? | Yes for registry/build/supervision models, with bounded constants and CI enforcement once toolchain availability is confirmed. |

## 9. Phase 3 Starting Point

Start Phase 3 with the smallest useful vertical slice:

1. TLA+ pre-flight models for registry, build dispatch, and run supervision.
2. `AgentRegistry` over existing validated config.
3. Build-runner lookup through registry.
4. Project-scoped active key for build-runner.
5. `BuildDispatchService` extraction with existing `adf/build` tests moved or mirrored.
6. `StatusService` extraction if needed to keep build dispatch small.

This slice directly addresses the user's explicit constraints:

| User Constraint | Covered By |
|-----------------|------------|
| Leverage existing implementation | Use existing config merge, project-source conversion, spawner, status helpers, and tests first |
| New implementation covers `adf/build` | `BuildDispatchService`, `adf/build` acceptance criteria, terminal status tests |
| OTP orchestrator maintains registry of agents | `AgentRegistry` plus `RunSupervisor` facade over OTP primitives |
| Can merge global and local agents | Registry indexes the existing `OrchestratorConfig::from_file` merge output from global includes and `project_sources` |
| Leverage TLA+ validation PR | Add bounded TLA+ models as pre-flight checks for registry lookup, `adf/build`, retry, terminal status, and project-scoped active runs |
