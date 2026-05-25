# Research Document: ADF Agent Validation & Verification Plan

**Status**: Draft
**Author**: Alex
**Date**: 2026-05-24
**Reviewers**: -

## Executive Summary

The ADF agent system has 15 production agents with four distinct trigger mechanisms: scheduled (cron), webhook/mention-dispatched, push/PR event-driven, and local CLI. Each agent must be validated in each trigger mode before production deployment. This document maps the existing architecture, catalogues each agent's trigger type, identifies gaps in current validation tooling, and establishes the essential constraints for a comprehensive verification plan.

## Essential Questions Check

| Question | Answer | Evidence |
|----------|--------|----------|
| Energising? | Yes | Directly enables production confidence and validates the entire agent runner investment |
| Leverages strengths? | Yes | Built on existing `adf agent validate` (Step 1) and orchestrator dispatch architecture |
| Meets real need? | Yes | No systematic validation exists; agents may silently fail in specific trigger modes |

**Proceed**: Yes (3/3 YES)

## Problem Statement

### Description
Each ADF agent can be triggered via four fundamentally different mechanisms. An agent that works when invoked manually via `adf --local --agent` may silently fail when triggered by a cron schedule, a Gitea webhook, or a PR event — because the environment, working directory, environment variables, and context differ across modes. No systematic validation suite exists to catch these discrepancies before production.

### Impact
- Silent agent failures in production (wrong working dir, missing env vars, stale token caches)
- Agents posting incorrect commit statuses or Gitea comments
- Budget exhaustion from repeatedly failing agents
- Security gaps undetected (UBS/cargo audit failing silently)

### Success Criteria
Every agent passes `adf agent validate` with `runnable: true` and produces a meaningful output when executed in each of its four trigger modes.

## Current State Analysis

### Agent Taxonomy

| Agent | Layer | Schedule | Event-Only | Trigger Modes |
|-------|-------|----------|-------------|---------------|
| security-sentinel | Core | `0 */6 * * *` | No | Cron, Mention, Local |
| runtime-guardian | Core | `15 0-10 * * *` | No | Cron, Mention, Local |
| merge-coordinator | Growth | `0 */4 * * *` | No | Cron, Mention, Local |
| meta-coordinator (project-meta) | Core | `*/30 * * * *` | No | Cron, Local |
| meta-learning | Core | `0 11 * * *` | No | Cron, Local |
| product-owner | Core | `25 0-10 * * *` | No | Cron, Mention, Local |
| product-development | Core | `25 0-10 * * *` | No | Cron, Mention, Local |
| repo-steward | Growth | `15 */6 * * *` | No | Cron, Local |
| upstream-synchronizer | Core | `30 1 * * *` | No | Cron, Local |
| pr-reviewer | Growth | None | Yes | PR Event, Local |
| pr-security-sentinel | Growth | None | Yes | PR Event, Local |
| pr-spec-validator | Safety | None | Yes | PR Event, Local |
| pr-test-guardian | Growth | None | Yes | PR Event, Local |
| pr-compliance-watchdog | Growth | None | Yes | PR Event, Local |
| build-runner | Growth | None | Yes | Push Event, Local |

### Trigger Mode Definitions

**Mode 1 — Global/One-shot**: `adf agent validate --config CONFIG AGENT [--project PROJECT]`
- Validates: working dir exists, agent enabled, cli_tool executable, model set, gitea_target resolvable
- Produces: `AgentRuntimeValidationReport` with warnings
- Limitation: read-only; does not attempt actual spawn

**Mode 2 — Scheduled (Cron)**: Triggered internally by `TimeScheduler` in the orchestrator tick loop
- Environment: full orchestrator env, `ADF_AGENT_NAME`, `ADF_WORKING_DIR`, `ADF_PROJECT`
- Pre-condition: orchestrator must be running
- Validation challenge: cron fires on schedule; cannot be trivially triggered on-demand

**Mode 3 — Webhook/Mention**: Triggered by `@adf:agent_name` on Gitea issue/PR comment
- Environment: `ADF_AGENT_NAME`, `ADF_WORKING_DIR`, context from comment
- Validation challenge: requires live Gitea webhook delivery
- Fallback: `adf-ctl trigger <agent_name>` sends a synthetic webhook

**Mode 4 — Push/PR Event**: Triggered by Gitea push or PR webhook events
- Environment: `ADF_PUSH_*` or `ADF_PR_*` env vars injected by `handle_push`/`handle_review_pr`
- Validation challenge: requires a real push or PR event; cannot be trivially simulated

**Mode 5 — Local CLI**: `adf --local --agent <name>`
- Environment: local filesystem, user env, no Gitea context
- Pre-condition: `.terraphim/adf.toml` must exist in current directory
- Validation challenge: requires the orchestrator to be run in local mode

### Code Locations

| Component | Location | Purpose |
|----------|----------|---------|
| CLI entry | `terraphim_orchestrator/src/bin/adf.rs` | `run_check`, `parse_args` |
| Agent runtime validation (new) | `terraphim_orchestrator/src/agent_runner.rs` | `validate_agent_runtime`, `AgentRuntimeValidationReport` |
| Dispatcher | `terraphim_orchestrator/src/dispatcher.rs` | `DispatchTask` enum, priority queue |
| Scheduler | `terraphim_orchestrator/src/scheduler.rs` | `ScheduleEntry`, cron parsing |
| Webhook | `terraphim_orchestrator/src/webhook.rs` | `WebhookDispatch` enum |
| Mention parser | `terraphim_orchestrator/src/adf_commands.rs` | Aho-Corasick pattern matching |
| Mention resolution | `terraphim_orchestrator/src/mention.rs` | `resolve_mention`, `MentionCursor` |
| Agent spawn | `terraphim_orchestrator/src/lib.rs:1772` | `spawn_agent`, gates |
| Spawner | `terraphim_spawner/src/lib.rs` | `AgentSpawner`, `spawn_with_fallback` |
| Validator | `terraphim_spawner/src/config.rs` | `AgentValidator` |
| Local spawner | `terraphim_orchestrator/src/bin/adf.rs:run_local_agent` | `adf --local --agent` |
| Remote trigger | `terraphim_orchestrator/src/bin/adf-ctl.rs` | `adf-ctl trigger` |

### Existing Validation Tools

1. **`adf agent validate`** (new, Step 1): read-only runtime validation producing `AgentRuntimeValidationReport`
2. **`adf --check CONFIG`**: validates orchestrator TOML config and prints routing table
3. **`adf-ctl trigger <agent>`**: sends synthetic webhook to running orchestrator
4. **`AgentValidator`** (`terraphim_spawner/src/config.rs`): validates CLI tool existence and executability

### Gaps

1. **No spawn/functional test**: `adf agent validate` is read-only; it cannot detect a broken CLI tool path or wrong model
2. **No cron simulation**: cannot trigger cron-scheduled agents on-demand without waiting for schedule
3. **No push/PR event simulation**: no tooling to inject a synthetic push or PR event without a real Git event
4. **No working-dir isolation verification**: cannot confirm the agent's working dir is correctly used during spawn
5. **No env var completeness check**: does not verify all required env vars are set for each trigger mode
6. **No output capture for local runs**: `adf --local --agent` does not capture/validate agent stdout/stderr

## Constraints

### Technical Constraints
- **No mocking in tests**: per AGENTS.md — agents must be validated against real CLI tools
- **UBS bug scanner must pass**: no critical UBS findings in changed code
- **Subscription-only models (C1)**: all agents must use subscription models (sonnet, kimi, minimax); no pay-per-use
- **MSRV 1.80**: all code must compile under Rust 1.80.0
- **Gitea webhook requires running orchestrator**: cannot test webhook trigger without live server

### Non-Functional Requirements

| Requirement | Target | Current |
|-------------|--------|---------|
| Validation time per agent | < 30s (local) | N/A |
| Full suite runtime | < 10 min | N/A |
| CLI binary availability | `adf` in PATH | Built from source |
| Gitea credentials | `$GITEA_TOKEN` set | Required for webhook tests |

## Vital Few

### Essential Constraints (Max 3)

| Constraint | Why Vital | Evidence |
|------------|----------|----------|
| Every agent must be executable in every trigger mode it declares | Silent failure in any mode is production risk | PRs #1822, #1823, design doc |
| Validation must be non-destructive (no mocking, no live Gitea writes) | Cannot pollute production state during testing | AGENTS.md constraint |
| Validation must be automatable in CI | Manual testing does not scale to 15 agents × 4 modes | Phase 5 target |

### Eliminated from Scope

| Eliminated | Why |
|------------|-----|
| LLM output quality assessment | Not measurable automatically; requires human review |
| End-to-end Gitea integration (real webhooks) | Requires live server; covered by Phase 5 overnight validation |
| Performance benchmarking | Out of scope for functional validation |
| Multi-agent interaction tests | Only single-agent validation per trigger mode |

## Risks and Unknowns

### Known Risks

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|-----------|
| CLI tool path not found on CI | High | Agent silently fails to spawn | Pre-flight check via `AgentValidator` |
| Working dir deleted/moved | Medium | Agent spawn fails | Validate working dir exists in `adf agent validate` |
| Model credentials missing | Medium | Agent run fails at LLM call | Probe model availability before spawn |
| Gitea token expired | Low | Webhook-dispatched agents fail | Token refresh handled by Gitea client |
| Cron schedule collision | Low | Two instances of same agent run | ConcurrencyController prevents |

### Open Questions

1. **Q1**: Can `adf-ctl trigger` be used to reliably simulate mention-dispatched agents? (Need to verify it posts a real webhook that `handle_webhook_dispatch` processes)
2. **Q2**: Does `adf --local --agent` require the orchestrator to be running, or does it spawn independently? (Based on `run_local_agent`: it calls `AgentOrchestrator::new` then `orch.run_once` — orchestrator IS running)
3. **Q3**: Are there agents that declare multiple trigger modes where conflicts can occur? (e.g., safety agents that are both cron-scheduled AND mention-dispatched)
4. **Q4**: Can the `handle_push` and `handle_review_pr` event paths be exercised without a real Git push/PR? (Need synthetic event injection)

### Assumptions Explicitly Stated

| Assumption | Basis | Risk if Wrong |
|-----------|-------|--------------|
| `adf agent validate` runnable=true means agent will successfully spawn | `runnable` gate checks working_dir exists, agent.enabled, cli_tool non-empty | False positive: cli_tool exists but is broken |
| `adf-ctl trigger` produces a webhook that `handle_webhook_dispatch` processes identically to real Gitea webhook | Both use same `WebhookDispatch::SpawnAgent` path | Simulation is not faithful to real trigger |
| Local mode (`adf --local --agent`) exercises the same spawn path as orchestrator-dispatched | Both call `spawn_agent` → `spawner.spawn_with_fallback` | False confidence: local works but cron fails due to env differences |

## Recommendations

### Proceed/No-Proceed
**Proceed** — the validation gap is real and blocking production confidence. The Step 1 `adf agent validate` provides a foundation to extend.

### Scope Recommendations

**In Scope:**
- `adf agent validate` extended to attempt a minimal spawn (not just read-only check)
- Synthetic webhook trigger via `adf-ctl trigger` as mention-mode simulation
- Synthetic push/PR event injection for event-only agents
- Local spawn test for every agent
- CI/CD pipeline integration (nightly run)

**Out of Scope (Avoid At All Cost):**
- Real Gitea webhook injection in CI (requires live server)
- LLM output quality scoring
- Multi-agent interaction testing
- Performance benchmarking

## Next Steps

1. Proceed to Phase 2 (Design) with this research document
2. Produce implementation plan for `adf agent validate` extensions per trigger mode
3. Define CI/CD integration points
4. Obtain human approval before implementation
