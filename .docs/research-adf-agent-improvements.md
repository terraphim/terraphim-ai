# Research Document: ADF Agent Flow Improvements

**Status**: Draft
**Author**: Claude (research)
**Date**: 2026-05-23
**Reviewers**: [pending]

## Executive Summary

The ADF (AI Dark Factory) orchestrator on bigbox executed multiple agent flows during the night of 2026-05-22 to 2026-05-23. Several critical issues were identified: merge-coordinator spec violations (8/14 decisions unmet), compliance-watchdog failures with credential leakage findings, provider probe failures for Anthropic models, and a missing WORKFLOW.md file. This research documents the current state and identifies improvement opportunities.

## Essential Questions Check

| Question | Answer | Evidence |
|----------|--------|----------|
| Energizing? | Yes | Security/compliance failures require immediate attention |
| Leverages strengths? | Yes | ADF automation already in place, needs hardening |
| Meets real need? | Yes | Production system with security vulnerabilities |

**Proceed**: Yes

## Problem Statement

### Nightly Run Summary (2026-05-22 22:00 UTC - 2026-05-23 08:00 UTC)

**Agents Executed:**

| Agent | Layer | Schedule | Exit Code | Wall Time | Notes |
|-------|-------|----------|-----------|-----------|-------|
| merge-coordinator | Growth | 22:00 | 0 | 329s | Success |
| merge-coordinator | Growth | 02:00 | 1 | 268s | FAIL - spec violations |
| security-sentinel | Core | 02:00 | 0 | 179s | Success (rate limit matched) |
| upstream-synchronizer | Core | 02:00 | 0 | 176s | Success |
| compliance-watchdog | Core | 02:05 | 1 | 29s | FAIL - credential leakage |
| product-development | Core | 02:25 | - | - | Long-running |
| spec-validator | Core | 02:30 | 0 | 209s | Success |
| test-guardian | Core | 02:35 | - | - | Long-running |
| documentation-generator | Core | 02:40 | - | - | Long-running |
| product-owner | Core | 02:55 | - | - | Long-running |
| odilo-developer | Core | 03:00 | - | - | Long-running |

**Orchestrator Health:**
- Tick count: 2490+ (running continuously)
- Tick interval: 30 seconds
- Last reconcile: 2026-05-23T07:07:30Z (elapsed_ms=421)

### Critical Findings

#### 1. merge-coordinator Spec Violations (Severity: HIGH)

**Spec Validation Report (2026-05-23):** 8 of 14 spec decisions FAIL

| Spec Decision | Status |
|--------------|--------|
| Concurrency-1: PID lock file | FAIL |
| Failure-1: Partial failure handling | FAIL |
| Failure-2: Remediation atomicity | FAIL |
| Failure-3: 3 retries with exponential backoff | FAIL |
| Edge-2: Conflicting verdicts logging | FAIL |
| Observability-1: Structured JSON logging | FAIL |
| Operational-1: Exit code semantics | FAIL |
| Security-2: Token not logged | FAIL |

**Files affected:**
- `scripts/merge-coordinator.py` - Python implementation predates spec
- `scripts/merge-coordinator-gate.sh` - Shell implementation lacks error handling

#### 2. Credential Leakage via Debug Derive (Severity: P2)

**Compliance Report (2026-05-21):** FAIL

Affected crates:
- `crates/terraphim_tinyclaw/src/config.rs` - TelegramConfig, DiscordConfig, SlackConfig, MatrixConfig tokens exposed via `#[derive(Debug)]`
- `crates/terraphim_tracker/src/gitea.rs` - GiteaConfig token exposed
- `crates/terraphim_github_runner_server/src/config/mod.rs` - Settings with webhook_secret, github_token exposed

#### 3. Provider Probe Failures (Severity: MEDIUM)

Anthropic models consistently failing probe:
- `anthropic/sonnet` - exit status 1
- `anthropic/opus` - exit status 1
- `anthropic/haiku` - exit status 1

Working providers:
- `openai/gpt-5.4` - latency 22872ms
- `kimi/kimi-for-coding/k2p5` - latency 28025ms
- `minimax/minimax-coding-plan/MiniMax-M2.5` - latency 28205ms
- `openai/gpt-5.4-mini` - latency 28889ms
- `openai/gpt-5.3-codex` - latency 29172ms
- `kimi/kimi-for-coding/k2p6` - latency 30349ms

#### 4. Missing WORKFLOW.md (Severity: LOW)

Configuration references `workflow_file = "WORKFLOW.md"` but file does not exist at `/opt/ai-dark-factory/WORKFLOW.md`.

## Current State Analysis

### System Architecture

```
ADF Orchestrator (bigbox)
├── conf.d/
│   ├── terraphim.toml (main agents)
│   ├── atomic-server.toml
│   ├── digital-twins.toml
│   ├── gitea.toml
│   └── odilo.toml
├── flow-states/ (JSON state files)
├── reports/ (nightly reports)
│   ├── spec-validation-YYYYMMDD.md
│   └── roadmap-YYYYMMDD-HHMM.md
└── logs/
    └── agents/ (per-agent logs)
```

### Agent Configuration (terraphim.toml)

Key agents defined:
- `security-sentinel` - Core layer, every 6h, skill_chain: security-audit, via-negativa-analysis, disciplined-verification, disciplined-validation
- `compliance-watchdog` - Core layer, 0-10h daily, skill_chain: disciplined-research, disciplined-verification, security-audit, responsible-ai, via-negativa-analysis
- `merge-coordinator` - Growth layer, cron-triggered
- `meta-coordinator` - commented out, uses bash dispatch script
- `drift-detector`, `runtime-guardian` - commented out

### Dispatcher Configurations

```toml
[pr_dispatch]
max_dispatches_per_tick = 3
max_concurrent_pr_agents = 4
agents_on_pr_open = [
    { name = "build-runner", context = "adf/build" },
    { name = "pr-reviewer", context = "adf/pr-reviewer" },
]

[workflow]
enabled = true
poll_interval_secs = 300
workflow_file = "WORKFLOW.md"

[compound_review]
schedule = "0 6 * * *"
max_duration_secs = 1800

[nightwatch]
eval_interval_secs = 300
active_start_hour = 2
active_end_hour = 6
```

### Code Locations

| Component | Location | Purpose |
|-----------|----------|---------|
| Orchestrator | `/opt/ai-dark-factory/` | Main orchestrator deployment |
| Agent configs | `conf.d/*.toml` | Agent definitions |
| Merge coordinator scripts | `scripts/merge-coordinator.py`, `scripts/merge-coordinator-gate.sh` | Python + shell implementation |
| Flow states | `flow-states/*.json` | Agent execution state |
| Reports | `reports/*.md` | Nightly validation reports |
| Skills | `/opt/ai-dark-factory/skills/` | Agent skill definitions |

## Constraints

### Technical Constraints
- **Rust rewrite required for merge-coordinator**: Python + shell implementation cannot meet spec
- **Existing skill chain dependencies**: Agent tasks depend on specific skill paths
- **Subscription model providers**: Must use kimi, minimax, zai - no openai/anthropic pay-per-use
- **Gitea token security**: Token must not appear in logs or process listings

### Business Constraints
- **Night hours (02:00-06:00 UTC)**: Core agent execution window
- **ADF uptime requirement**: Orchestrator must run continuously
- **No manual intervention**: Agents must self-remediate where possible

### Non-Functional Requirements

| Requirement | Target | Current |
|-------------|--------|---------|
| Agent spawn time | < 5s | ~1s (observed) |
| Provider probe time | < 30s | 22-31s (variable) |
| Reconcile tick | < 1s | 92-575ms |
| Nightly completion | 04:00 UTC | Variable |
| Exit code accuracy | 0/1/2 semantics | Always 0 |

## Vital Few (Essentialism)

### Essential Constraints (Max 3)

| Constraint | Why Vital | Evidence |
|------------|-----------|----------|
| Fix merge-coordinator atomicity | Data loss risk - merged PR but open issue | Spec FAIL-1 |
| Fix credential leakage P2 | Security vulnerability - secrets in logs | Compliance report |
| Implement structured logging | Cannot debug failures without observability | Spec OBS-1 |

### Eliminated from Scope

| Item | Why Eliminated |
|------|----------------|
| Runtime-guardian implementation | Not critical path |
| Drift-detector implementation | Manual config management acceptable |
| Meta-coordinator bash rewrite | Python dispatch sufficient for now |

## Dependencies

### Internal Dependencies

| Dependency | Impact | Risk |
|------------|--------|------|
| terraphim_spawner | Agent spawning | Low |
| terraphim_orchestrator | Core orchestration | Critical |
| Quickwit integration | Log indexing | Medium |
| Gitea API | Issue creation/commenting | Medium |

### External Dependencies

| Dependency | Version | Risk | Alternative |
|------------|---------|------|-------------|
| kimi-for-coding | k2p5/k2p6 | Low | minimax, zai |
| Gitea API | v1 | Low | - |

## Risks and Unknowns

### Known Risks

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| merge-coordinator race condition | High | High | Implement PID lock |
| Credential leakage in prod | High | Critical | Custom Debug impl |
| Anthropic provider outage | Medium | Low | Already falling back |

### Open Questions

1. Why did compliance-watchdog exit with code 1 after only 29s? (Short run suggests immediate failure)
2. Are the merge-coordinator Python scripts actually being used or are they deprecated?
3. Should we implement Rust rewrite of merge-coordinator or backport fixes to Python?

### Assumptions

| Assumption | Basis | Risk if Wrong |
|------------|-------|---------------|
| Python scripts are current implementation | Spec validation report references them | May be dead code |
| Nightwatch active hours are UTC | Config shows 2-6, no TZ specified | Agent timing issues |
| Skill chain paths are correct | No validation in logs | Agents may skip skills |

## Research Findings

### Key Insights

1. **Two merge-coordinator implementations exist**: Python (`merge-coordinator.py`) and shell (`merge-coordinator-gate.sh`). Both FAIL spec compliance.

2. **Exit code semantics broken**: All agents appear to exit 0 regardless of outcome, making automated monitoring impossible.

3. **Anthropic API issues**: All Anthropic models (sonnet, opus, haiku) failing probe consistently, but agents are still being routed through kimi/openai successfully.

4. **SPEC VALIDATION is working**: The spec-validator agent correctly identified 8/14 failures in merge-coordinator implementation.

5. **Night window underutilized**: Multiple Core agents spawning in 02:00-03:00 window but many are long-running and may not complete before morning.

### Relevant Prior Art

- **Lru RUSTSEC-2026-0002**: Previously issues as #1574, closed but advisory still present in lock file
- **merge-coordinator spec**: `.docs/spec-merge-coordinator.md` defines requirements from 2026-05-19 interview

## Recommendations

### Proceed/No-Proceed
**Proceed** - Security and compliance issues require immediate action.

### Priority Order

1. **P0 (Critical)**: Fix credential leakage - Custom Debug implementations
2. **P1 (High)**: Fix merge-coordinator atomicity and concurrency
3. **P2 (Medium)**: Implement structured JSON logging
4. **P3 (Low)**: Create WORKFLOW.md, fix exit code semantics

### Risk Mitigation

1. For credential leakage: Apply custom `fmt::Debug` redaction pattern (see LinearConfig for reference)
2. For merge-coordinator: Implement PID lock + partial failure handling + retry logic
3. For logging: Replace print statements with structured JSON to stdout

## Next Steps

1. Create Gitea issues for each P0/P1 finding
2. Implement Rust rewrite of merge-coordinator (per spec)
3. Apply Debug redaction to affected config structs
4. Implement structured logging in Python merge-coordinator
5. Validate all fixes with spec-validator agent

## Appendix

### Reference Materials

- Spec validation report: `/opt/ai-dark-factory/reports/spec-validation-20260523.md`
- Orchestrator config: `/opt/ai-dark-factory/orchestrator.toml`
- Agent config: `/opt/ai-dark-factory/conf.d/terraphim.toml`
- Merge coordinator spec: `.docs/spec-merge-coordinator.md`

### Night Logs (key entries)

```
May 22 22:00:29 - spawning agent=merge-coordinator layer=Growth
May 22 22:05:59 - agent exit classified agent=merge-coordinator exit_code=0 confidence=1.0
May 23 02:00:29 - spawning agent=merge-coordinator layer=Growth
May 23 02:00:29 - spawning agent=security-sentinel layer=Core
May 23 02:00:31 - spawning agent=upstream-synchronizer layer=Core
May 23 02:03:29 - agent exit classified agent=security-sentinel exit_code=0
May 23 02:04:59 - agent exit classified agent=merge-coordinator exit_code=1 confidence=0.0
May 23 02:05:29 - spawning agent=compliance-watchdog layer=Core
May 23 02:05:59 - agent exit classified agent=compliance-watchdog exit_code=1
```
