# Specification: Provider Probe Timeout and Health Classification

**Status**: Authoritative
**Source**: `crates/terraphim_orchestrator/src/provider_probe.rs`
**Issue**: #1924 (re-scoped from PR #1788 Slice 8)
**Date**: 2026-06-01

---

## Overview

The ADF orchestrator periodically probes each configured provider+model combination to
determine whether it can serve requests. Probe results feed into per-(cli, provider, model)
circuit breakers that the KG router consults when selecting dispatch targets.

---

## Probe Execution

### Trigger

`ProviderHealthMap::probe_all(&kg_router)` is called when `is_stale()` returns true.
The cache TTL is configurable at construction; default is left to the caller.

### Target Discovery

All unique `(cli_tool, provider, model)` triples from the KG router's rules are probed.
A triple that has already been probed in this sweep is skipped (de-duplication via
`seen` map). Two CLI tools reaching the same `(provider, model)` have **independent**
circuit breakers so a broken CLI does not poison the model's health.

### Execution Model

Each probe spawns a `bash -c <action>` subprocess via `tokio::process::Command`.
The action template is taken from the KG routing rule and rendered with:

```
{{ model }}   →  the model string
{{ prompt }}  →  "echo hello"
```

A PATH prefix of `~/.local/bin:~/.bun/bin:~/bin:~/.cargo/bin:~/go/bin` is prepended
so standard tool locations are found without sourcing `.profile`.

The child process is spawned **before** the timeout future is armed. This ensures
that `tokio::time::timeout` expiry kills the child explicitly rather than merely
dropping the future, which would leave the process running.

### Timeout

**15 seconds** (`Duration::from_secs(15)`). Hard deadline — if the child has not
exited within 15 s, the probe result is `ProbeStatus::Timeout`.

---

## Classification Algorithm

| Exit code | Token-bearing output? | Result |
|-----------|----------------------|--------|
| Success (0) | Yes | `ProbeStatus::Success` |
| Success (0) | No | `ProbeStatus::Error` ("exit 0 but stream produced no token content") |
| Non-zero | any | `ProbeStatus::Error` |
| Timeout | n/a | `ProbeStatus::Timeout` |
| Rate-limited | n/a | `ProbeStatus::RateLimited` |

### Token-Bearing Output Detection (`has_token_bearing_output`)

A probe result is only classified `Success` if the process stdout contains at least
one meaningful token:

- **opencode JSON stream**: requires at least one `"type":"text"` or
  `"type":"step_finish"` event. A stream containing only `"type":"step_start"` is
  classified as not token-bearing.
- **Non-JSON CLIs** (pi-rust, claude, raw shell): any non-empty, non-whitespace
  content is considered token-bearing.
- **Empty stdout**: never token-bearing.

This dual condition (exit 0 AND token-bearing) prevents a known opencode defect
(streaming only `step_start` then exiting cleanly) from being reported as healthy.

### Environment Error Classification (`is_environment_error`)

The following probe errors are classified as environment/configuration errors and do
**not** update the circuit breaker:

| Error pattern | Reason |
|---------------|--------|
| `CLI tool ... not found on PATH` | Local tool missing, not API failure |
| `not in C1 allow-list` | Subscription configuration, not transient API |
| `no action:: template defined` | Routing config incomplete |

---

## Circuit Breaker Configuration

| Parameter | Value |
|-----------|-------|
| `failure_threshold` | 5 consecutive failures to open |
| `cooldown` | 300 s (5 minutes) |
| `success_threshold` | 1 success to close from HalfOpen |

Keys are `<cli>:<provider>:<model>` for independence per CLI tool. Probe open-skipping:
if the circuit breaker for a key is already `Open`, the probe for that key is skipped
to avoid wasting tokens and spawning processes that will timeout.

---

## Health Status Mapping

| Probe/Circuit state | `model_health` | `provider_health` | `is_healthy` |
|--------------------|----------------|-------------------|-------------|
| Probe `Success` | `Healthy` | `Healthy` | `true` |
| Probe `Error/Timeout/RateLimited` | `Unhealthy` | `Unhealthy` (if all models) | `false` |
| No probe, CB `Closed` | `Healthy` | `Healthy` | `true` |
| No probe, CB `HalfOpen` | `Degraded` | `Healthy` | `true` |
| No probe, CB `Open` | `Unhealthy` | `Unhealthy` (if all models) | `false` |
| No probe, no CB | `Healthy` (unknown) | `Healthy` (unknown) | `true` |

A provider is considered healthy if **any** of its models is healthy.

---

## Invariants

| # | Invariant | Source |
|---|-----------|--------|
| I1 | Probe timeout is 15 s; the child process is explicitly terminated at deadline. | `tokio::time::timeout(Duration::from_secs(15), ...)` after `spawn()` |
| I2 | Environment errors do not update the circuit breaker. | `is_environment_error()` guard before `breaker.record_failure()` |
| I3 | C1 allow-list gate skips probe and returns `Error` without spawning. | `is_allowed_provider()` check before `bash -c ...` |
| I4 | A circuit-breaker-`Open` key is not re-probed in the same sweep. | `if matches!(breaker.state(), Open) { continue; }` |
| I5 | Probe results are cached; `is_stale()` controls refresh frequency. | `probed_at: Option<Instant>` and TTL comparison |
| I6 | Exit 0 with no token-bearing output is classified `Error`, not `Success`. | `has_token_bearing_output` && `output.status.success()` both required |
| I7 | Rate-limited responses update the rate limiter, not the circuit breaker. | `ProbeStatus::RateLimited` match arm |

---

## Failure Modes

| Failure | Observable Effect | Recovery |
|---------|-------------------|---------|
| CLI tool not on PATH | `Error` (env); no CB update; WARN logged | Add tool to PATH |
| Provider API down | `Error`/`Timeout`; CB accumulates failures | Provider recovers; CB resets on cooldown |
| Probe hangs past 15 s | `Timeout` result; child killed | Investigate provider latency |
| opencode emits only `step_start` | `Error` ("no token content"); CB updated | Upgrade opencode version |
| All circuit breakers for provider `Open` | `is_healthy()` returns `false`; no dispatch | Wait for cooldown (300 s) |
| Rate limit hit | `RateLimited`; provider added to `rate_limited` set | Respect rate limit window |

---

## Persistence

Results are saved to a directory as JSON via `save_results(dir)`. Two files are written:
- `<timestamp>.json` — timestamped archive
- `latest.json` — always overwritten with most recent results

This is compatible with the pi-benchmark result format.

---

## Verification Note

The following test suite covers the circuit breaker and health classification logic:

```bash
cargo test -p terraphim_orchestrator provider_probe -- --nocapture
```

Key functions verified by tests:
- `is_stale` TTL calculation
- `probe_all` de-duplication and circuit breaker update
- `model_health` and `provider_health` priority rules (probe result > CB state)
- `is_healthy` threshold (Healthy | Degraded → true)
- `unhealthy_providers` aggregation (all-models-unhealthy policy)
- `record_success` / `record_failure` CB propagation
