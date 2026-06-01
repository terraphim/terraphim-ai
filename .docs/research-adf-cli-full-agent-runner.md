# Research Document: ADF CLI Full Agent Runner

**Status**: Draft
**Author**: OpenCode
**Date**: 2026-05-23 23:06 CEST
**Reviewers**: Human maintainer

## Executive Summary

`adf` currently validates config and runs the full orchestrator, while `adf-ctl` remotely triggers a limited webhook path. The missing capability is a first-class CLI contract that can validate and run any configured agent through the same production dispatch path used by scheduled, mention, PR, push, local, global, project-scoped, and evolution-enabled execution.

The recommended direction is to extend the `adf` binary with an explicit `agent` command group backed by a reusable internal `AgentRunRequest` API. The design should avoid a second execution engine: CLI-triggered runs must resolve config, project context, Gitea context, evolution settings, routing, worktree isolation, output posting, and exit classification through the same `AgentOrchestrator` machinery that production uses.

## Essential Questions Check

| Question | Answer | Evidence |
|----------|--------|----------|
| Energising? | Yes | ADF reliability is currently blocked by difficulty proving an individual agent can run beyond `--check`. |
| Leverages strengths? | Yes | The repo already has orchestrator, scheduler, dispatcher, webhook, Gitea, evolution, and worktree primitives. |
| Meets real need? | Yes | Outstanding ADF issues include corrupted runner paths, degraded fleet, probe failures, malformed branches, and vendor regressions. |

**Proceed**: Yes, 3/3 essential questions pass.

## Problem Statement

### Description

Operators need `adf-cli` to do more than validate TOML. It must prove that a specific agent can be resolved, configured, spawned, observed, and completed under the same constraints as production. It must support these run contexts:

- Scheduled agents: cron-defined Safety/Core/Growth behaviour.
- Global agents: legacy single-project mode with no `project` field.
- Local agents: local repository execution without remote webhook transport.
- Project-scoped agents: configured via `[projects]` with per-project `working_dir`, Gitea owner/repo, workflow, and mention settings.
- Evolution-enabled agents: top-level `evolution.enabled = true` plus per-agent `evolution_enabled = true`.
- Gitea transport: synthetic or real Gitea event payloads for issue comments, PRs, pushes, and explicit issue targeting.

### Impact

Without this, contributors can only run `adf --check` or operate the long-lived daemon. That leaves a gap between config validity and actual agent functionality, causing repeated failures to surface only after webhooks, schedules, or production ticks fire.

### Success Criteria

- `adf agent validate <agent>` resolves the agent, project, runtime context, routing inputs, Gitea target, and evolution eligibility without spawning.
- `adf agent run <agent>` spawns the agent through the production spawn path and returns a structured exit result.
- `adf agent simulate schedule <agent>` validates and optionally fires the schedule path.
- `adf agent trigger gitea ...` validates and optionally executes the Gitea webhook dispatch path locally, without requiring SSH to `bigbox`.
- JSON output exists for automation and includes project id, working directory, Gitea issue/PR/push context, model/CLI selection, worktree path, exit class, log path, and evolution snapshot key when available.
- All new validation paths have integration tests with real config parsing and real process spawning using harmless local commands, not mocks.

## Current State Analysis

### Existing Implementation

The system already contains most runtime primitives, but they are not exposed as a cohesive CLI execution API.

| Component | Location | Purpose |
|-----------|----------|---------|
| `adf` binary | `crates/terraphim_orchestrator/src/bin/adf.rs` | Runs orchestrator or `--check` config/routing table. |
| `adf-ctl` binary | `crates/terraphim_orchestrator/src/bin/adf-ctl.rs` | SSH-based remote control; constructs signed issue-comment webhook payloads. |
| Command parser | `crates/terraphim_orchestrator/src/adf_commands.rs` | Parses `@adf:<agent>` commands from comments. |
| Config schema | `crates/terraphim_orchestrator/src/config.rs` | Defines `OrchestratorConfig`, `Project`, `AgentDefinition`, Gitea, routing, learning, evolution, workflow, webhook config. |
| Main orchestrator | `crates/terraphim_orchestrator/src/lib.rs` | Runs scheduler/webhook/tick loop, spawns agents, handles exits, routes Gitea events. |
| Scheduler | `crates/terraphim_orchestrator/src/scheduler.rs` | Parses cron and separates immediate Safety agents from scheduled entries. |
| Dispatcher | `crates/terraphim_orchestrator/src/dispatcher.rs` | Unified dispatch queue for time, issue, mention, PR, merge, post-merge, and push tasks. |
| Webhook transport | `crates/terraphim_orchestrator/src/webhook.rs` | Parses Gitea issue comment, PR, and push events into `WebhookDispatch`. |
| Evolution | `crates/terraphim_orchestrator/src/evolution.rs` | Records outputs/tasks/lessons and renders context when feature/config/agent gates allow it. |
| Output posting | `crates/terraphim_orchestrator/src/output_poster.rs` | Posts agent output to configured Gitea issues and handles per-agent tokens. |

### Data Flow

Current production flow:

```text
TOML config -> AgentOrchestrator::new/from_config_file
             -> run()
             -> safety spawn OR scheduler event OR webhook dispatch OR reconcile tick
             -> spawn_agent(def)
             -> gates: pause, disk, budget, pre-check, routing, concurrency
             -> persona/skills/learning/evolution prompt injection
             -> worktree isolation
             -> spawner.spawn_with_fallback()
             -> output drain, nightwatch, evolution, output poster, exit handling
```

Current CLI flow:

```text
adf --check CONFIG -> parse config -> validate -> print table -> exit
adf CONFIG         -> start long-running orchestrator -> wait forever
adf-ctl trigger    -> SSH to host -> signed webhook curl -> optional journal polling
```

The missing link is a local, deterministic, bounded CLI flow:

```text
adf agent run CONFIG AGENT CONTEXT -> production resolution -> production spawn -> wait -> structured result
```

### Integration Points

- `AgentOrchestrator::spawn_agent` is private and production-grade, but not directly usable from CLI except by running the daemon.
- `spawn_agent_for_test` exists only as a hidden test helper and does not expose exit results or run context.
- `handle_webhook_dispatch` is production behaviour but currently private and tied to the reconciliation loop.
- `WebhookDispatch` is the right internal representation for Gitea transport simulation.
- `AgentDefinition` already carries `project`, `gitea_issue`, `event_only`, and `evolution_enabled` flags.
- `Project` already carries per-project working directory and Gitea config.
- `EvolutionManager` already integrates at spawn/output/exit, but the CLI needs to expose whether it was active and whether a snapshot was created.

## Constraints

### Technical Constraints

- Reuse production execution. Do not create a second agent runner that bypasses routing, pre-checks, worktrees, concurrency, output, or evolution.
- Keep `adf --check` backward compatible.
- Avoid SSH as the primary validation path. SSH can remain in `adf-ctl`, but full local validation belongs in `adf`.
- Tests must not use mocks. Use real temporary configs and harmless local commands such as shell scripts or `echo`-style agents.
- Secrets must remain redacted. No `.env` modification; Gitea tokens come from config/env/1Password mechanisms already used by the repo.
- Scheduled validation must not wait for wall-clock cron. It should inspect and optionally inject schedule events.

### Business Constraints

- Operators need clear pass/fail evidence for ADF health issues.
- The plan must reduce production-only surprises, not add operational ceremony.
- Automation needs machine-readable JSON output.

### Non-Functional Requirements

| Requirement | Target | Current |
|-------------|--------|---------|
| CLI validation latency | < 2s for `validate` | `--check` is fast but shallow. |
| Bounded run latency | Caller-specified, default <= existing agent limit | `adf-ctl --wait` polls journals remotely. |
| Observability | JSON + human output | `adf-ctl status --format json` is best-effort only. |
| Fidelity | Same path as production spawn | No first-class local CLI run path. |

## Vital Few

| Constraint | Why It's Vital | Evidence |
|------------|----------------|----------|
| One execution path | Divergent CLI runners would hide production bugs. | `spawn_agent` already encodes many gates and side effects. |
| Context resolution | Most failures are project/Gitea/worktree/routing-context failures, not TOML syntax failures. | Open ADF issues mention corrupted repo paths and vendor/provider failures. |
| Structured run result | Automation must decide pass/fail without parsing journals. | `adf-ctl` currently scrapes `journalctl` strings. |

## Eliminated from Scope

| Eliminated Item | Why Eliminated |
|-----------------|----------------|
| Replacing `adf-ctl` SSH operations | Useful, but not required for local full-run validation. |
| New web dashboard | The need is CLI validation and execution. |
| Rewriting scheduler/dispatcher architecture | Existing primitives are adequate. |
| General workflow engine changes | Agent-level validation can be solved first. |
| Persisted historical analytics | Existing logs/Quickwit can be used later. |

## Dependencies

### Internal Dependencies

| Dependency | Impact | Risk |
|------------|--------|------|
| `AgentOrchestrator::spawn_agent` | Central production spawn path. | Private and returns only `Result<(), OrchestratorError>`, not completion evidence. |
| `poll_agent_exits` and exit classification | Needed to wait for agent completion. | Existing behaviour may be spread across tick handling. |
| `WebhookDispatch` | Gitea transport simulation target. | Some variants lack comment ids or direct issue context. |
| `OrchestratorConfig::validate` | Baseline config validation. | Does not validate runtime availability or real spawn viability. |
| `EvolutionManager` | Evolution support. | Feature-gated; output needs a no-op shape when disabled. |

### External Dependencies

| Dependency | Risk | Alternative |
|------------|------|-------------|
| Gitea API/webhook schema | Payload drift can break simulation. | Use internal `WebhookDispatch` builders plus optional raw JSON input. |
| CLI tools (`opencode`, `claude`, `codex`, shell scripts) | Tool availability varies by environment. | Add `--dry-run` and `--validate-tools`; tests use local binaries/scripts. |
| Git worktrees | Corrupt source repo causes failures. | Make worktree validation explicit before spawn. |

## Risks and Unknowns

### Known Risks

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| CLI bypasses production behaviour | Medium | High | Expose a small production `run_agent_once` API, not duplicate spawn logic. |
| Long-running Safety agents make CLI hang | High | Medium | Require `--wait` mode with bounded timeout and support `--detach`. |
| Scheduled validation accidentally starts all Safety agents | Medium | High | New command targets a single agent unless `--all` is explicit. |
| Gitea output posts to real issues during validation | Medium | Medium | Provide `--no-post` / `--transport local` and explicit `--gitea-post` opt-in. |
| Evolution state mutates during validation | Medium | Medium | `validate` is read-only; `run` mutates only when execution is intentional. |

### Open Questions

1. Should `adf agent run` default to posting output to Gitea when `gitea_issue` is configured, or require `--gitea-post`?
2. Should event-only agents be runnable with `adf agent run`, or only through `adf agent trigger push/pr`?
3. Should the CLI accept raw Gitea webhook JSON from stdin as the canonical transport test?
4. Should a successful detached spawn count as pass, or must the agent exit successfully for pass?

### Assumptions Explicitly Stated

| Assumption | Basis | Risk if Wrong | Verified? |
|------------|-------|---------------|-----------|
| `adf` is the right binary to extend | It already owns config load, run, and check. | Could split responsibilities with `adf-ctl`. | Partially |
| `spawn_agent` is the source of truth | It contains gates, routing, evolution, worktree, and output setup. | Hidden side effects may make one-shot runs awkward. | Partially |
| Local command execution is enough for tests | Tests need real execution without remote services. | May miss Gitea-specific API failures. | Yes for CLI runner tests, no for live transport. |

### Multiple Interpretations Considered

| Interpretation | Implications | Why Chosen/Rejected |
|----------------|--------------|---------------------|
| Extend `adf-ctl` only | Keeps remote-control semantics but still depends on SSH and daemon. | Rejected: does not solve local full-run validation. |
| Add `adf agent run` | Local, deterministic, can reuse config and production spawn. | Chosen. |
| Add a new binary `adf-cli` | Clear naming but duplicates packaging and docs. | Rejected unless current binary naming cannot change. |

## Research Findings

### Key Insights

1. The system has production-grade dispatch logic, but the CLI only exposes shallow validation or daemon mode.
2. Gitea transport already normalises events to `WebhookDispatch`; the CLI should build those variants rather than shelling to a webhook where possible.
3. Evolution is already integrated into spawn, output, and exit handling. The missing piece is reporting and targeted CLI enablement/validation.
4. Scheduled execution should be tested by schedule resolution and event injection, not by waiting for cron.
5. `adf-ctl --wait` currently relies on remote journal strings; a local full-run command should return typed run results.

### Technical Spikes Needed

| Spike | Purpose | Estimated Effort |
|-------|---------|------------------|
| One-shot run API | Determine minimal public API around private `spawn_agent` and exit wait. | 0.5 day |
| Gitea dispatch builder | Build `WebhookDispatch` from CLI args and optional raw JSON. | 0.5 day |
| Run result extraction | Capture exit class, logs, worktree path, routed model, output post status, evolution snapshot. | 1 day |

## Recommendations

### Proceed/No-Proceed

Proceed. The system needs a production-faithful CLI runner before more ADF automation is trusted.

### Scope Recommendations

- Phase 1 should add `adf agent validate` and `adf agent run` for direct/local configured agents.
- Phase 2 should add `adf agent trigger gitea` for issue-comment, PR, push, and raw webhook JSON simulation.
- Phase 3 should add schedule simulation and detached execution status querying.

### Risk Mitigation Recommendations

- Make `validate` read-only and explicit.
- Make real Gitea posting opt-in during local validation unless the agent definition intentionally sets `gitea_issue` and the caller passes `--post-output`.
- Surface all resolved runtime values before spawn: project id, working directory, repo health, agent layer, schedule, model, CLI, evolution state, Gitea target.

## Progress Recheck: 2026-05-23 Gitea State

### Wiki Check

The Gitea wiki page exists in the wiki git repository as `Session-2026-05-23-ADF-Self-Healing-Delivery.-.md`, but its current content is only:

```text
Not connected
```

Therefore, the wiki does not currently provide usable delivery evidence. The authoritative evidence for this recheck is the Gitea PR/issue state and `gitea/main` code.

### Landed PRs

| PR | Status | Summary | Research Impact |
|----|--------|---------|-----------------|
| `#1822` | Merged on Gitea at `188bf07b1872072d90329006e4b6030f1aa3dbc6` | Config-error circuit-breaker, `ExitClass::ConfigError`, quarantine after 3 failures, `AgentDefinition.enabled`, memory watchdog files, `bigbox-sync.sh`. | Partially satisfies self-healing operational guardrails. Needs Phase 4 verification against acceptance criteria. |
| `#1823` | Merged on Gitea at `04648c246965fd8abf902c8a7880b0f84ac91056` | Minimal Rust `terraphim_merge_coordinator` skeleton with evaluator, Gitea client, JSON logging, PID lock, binary entry. | Important progress, but the PR body says "minimum viable scaffold" and issue `#1805` remains open, so spec completion is not proven. |

### ADF CLI Progress Already on Gitea

`gitea/main` already contains local ADF CLI functionality:

- `adf --local --check` discovers `.terraphim/adf.toml`, converts it to `OrchestratorConfig`, validates, and prints the routing table.
- `adf --local --agent NAME` discovers `.terraphim/adf.toml`, builds a local `SpawnContext`, directly invokes `AgentSpawner`, streams stdout/stderr, and exits with the agent status code.
- `ProjectAdfConfig` supports project-local `.terraphim/adf.toml`, local agents, `pr_dispatch`, env substitution, and conversion to `Project` plus `AgentDefinition`.
- `adf_check_tests.rs` includes real CLI integration tests for `--check`, `--local --check`, and `--local --agent` using harmless local processes.

This is useful progress and should be treated as a good foundation. The current `--local --agent` path bypasses large parts of production `AgentOrchestrator::spawn_agent`, including worktree isolation, KG routing, scheduler dispatch, mention/Gitea dispatch, output posting, nightwatch, learning/evolution lifecycle, and exit-class reporting. It proves a local command can spawn, which is the correct first step; the next step is to converge that local spawner into a production-faithful one-shot orchestrator runner.

### Outstanding Gaps After Recheck

| Gap | Evidence | Required Direction |
|-----|----------|--------------------|
| Production-fidelity CLI runner missing | `adf --local --agent` uses `AgentSpawner` directly. | Add one-shot `AgentOrchestrator` run API and CLI commands around it. |
| Gitea transport simulation missing | `adf-ctl` remote webhook exists; local `adf` cannot trigger issue-comment/PR/push dispatch from config. | Add `adf agent trigger gitea ...` using `WebhookDispatch`. |
| Scheduled mode simulation missing | `adf --check` validates schedules; no targeted schedule fire/report. | Add `adf agent simulate schedule`. |
| Evolution validation/reporting missing | Local path builds `EvolutionConfig::default()` and does not report snapshot/context lifecycle. | Add evolution-aware validation and run result fields. |
| Merge-coordinator spec remains unclosed | `#1823` merged, but `#1805` remains open with acceptance unchecked. | Phase 4 verification must trace `#1823` against `#1805` spec. |
| Vendor Z.AI issue open | `#1819` shows opencode 1.14.48 Z.AI stream truncation; workaround routes Z.AI through pi-rust. | CLI validation must expose provider/runtime chosen and not treat opencode Z.AI as healthy. |

### Updated Research Conclusion

The research conclusion is stronger after recheck: a partial local CLI runner exists, so the next step should not build another direct runner. It should converge the existing `--local` work into a single production-faithful `adf agent ...` surface backed by `AgentOrchestrator`, while preserving `--local` as a convenience alias or deprecating it only after compatibility review.

Human feedback received: Phase 4 verification on `#1822`/`#1823` and Phase 5 validation against the 70% overnight success criterion are the correct gates. The direct local spawner path is accepted as a good start, not a failure.

## Session Progress Update: ADF Self-Healing Delivery 2026-05-23

The session report changes the planning stance from "foundation partly landed" to "self-healing machinery landed, now needs disciplined verification and live validation".

### Updated Evidence

| Area | Session Evidence | Planning Impact |
|------|------------------|-----------------|
| Rust merge-coordinator | Full `terraphim_merge_coordinator` crate reported landed with PID lock, retry/backoff, JSON logger, auto-close on `Fixes #N`, exit codes 0/1/2. | Treat `#1805` as implementation-complete pending Phase 4 verification, not as merely a skeleton. |
| Operational guardrails | Config-error quarantine, memory watchdog, sync runbook, debug redaction, and implementation-swarm restoration reported shipped. | Phase 4 should verify behaviour and traceability; no additional design needed before verification. |
| Probe redesign | Probe key changed from `(provider, model)` to `(cli, provider, model)` with truncated-stream detection. | CLI runner validation must report the selected CLI as well as provider/model, otherwise it will miss opencode-vs-pi-rust differences. |
| Z.AI workaround | opencode 1.14.48 broken for Z.AI; pi-rust works with same key/network; routes now invoke pi-rust. | Gitea transport and agent validation must surface runtime path/CLI health, not just model health. |
| Local ADF CLI | Direct `adf --local --agent` spawner is accepted as a good first step. | Preserve it as bootstrap; converge into production-faithful one-shot runner rather than discarding. |
| Bigbox repo recovery | `.git` corruption temporarily repaired; fresh clone is canonical longer term. | Runner validation must include repository health and resolved working directory checks. |

### Revised Research Gaps

The remaining gaps are no longer broad self-healing implementation gaps. They are now verification, validation, and CLI fidelity gaps:

1. Verify that landed PRs match their specs and acceptance criteria.
2. Validate that the landed machinery changes live ADF outcomes toward 70% overnight success by 2026-06-15.
3. Extend `adf` from direct local spawning to production-faithful, structured, one-shot execution across local, global, scheduled, evolution-enabled, and Gitea-triggered modes.
4. Ensure runtime reports include `(cli, provider, model)`, project, working directory, repo health, worktree, exit class, log path, Gitea target, output posting, and evolution snapshot state.

## Next Steps

If approved:

1. Run Phase 4 disciplined verification on landed PRs `#1822` and `#1823` before building more on them.
2. Run Phase 5 disciplined validation against the original self-healing success criteria, especially 70% overnight success by 2026-06-15.
3. Implement the design in `.docs/design-adf-cli-full-agent-runner.md`, updated to build on the landed `--local` functionality.
4. Run quality evaluation on both research and design before coding.
5. Create or update a Gitea issue for the remaining full-runner capability if none already covers this exact CLI outcome.
