# Implementation Plan: Terraphim AI Vital Few Clusters

**Status**: Draft
**Research Doc**: `.docs/research-issue-backlog-2026-04-17.md`
**Author**: Strategic Synthesis Specialist
**Date**: 2026-04-17
**Estimated Effort**: 10 weeks (5 clusters, phased)

---

## Overview

### Summary
This plan implements the five Vital Few clusters identified in the Phase 1 research: (1) Security Remediation, (2) ADF Reliability, (3) Inter-Agent Orchestration, (4) Learning and KG Evolution, and (5) PA/SO Roles. Execution follows a strict phase gate: stop the bleeding, stabilise the factory, wire the nervous system, build the brain, and deliver user value.

### Approach
A waterfall-with-parallel-tracks approach. Cluster C (Security) and Cluster A (ADF Reliability) are sequential blockers for all downstream work. Cluster F (PA/SO) can proceed in parallel with Cluster A because it touches disjoint code paths (agent roles versus orchestrator core). Clusters B and D are strictly gated behind Cluster A.

### Scope

**In Scope (The Vital Five):**
- **Cluster C -- Security Remediation**: RUSTSEC-2026-0049 remediation, CI runner restoration, cascading sentinel failure fixes
- **Cluster A -- ADF Reliability**: Timeout tuning, disk guard, TOML duplicate-key hardening, provider probe circuit breakers, AgentRunRecord taxonomy
- **Cluster B -- Inter-Agent Orchestration**: Gitea mention parsing, webhook-driven detection, suggestion approval workflow, agent evolution wiring
- **Cluster D -- Learning and KG Evolution**: SQLite shared learning store (Phase 1), trust-gated promotion, cross-agent injection
- **Cluster F -- PA/SO Roles**: JMAP + Obsidian integration, PA role in embedded config, SO durable path cloning

**Out of Scope:**
- TLA+ formal verification epic (#349) -- deferred to Phase 6
- Community launch and marketing content (#608, #644-#648)
- TinyClaw OpenClaw parity (#590)
- SNOMED/MeSH ontology benchmarks (#231)
- Criterion benchmarks for symbolic embeddings (#232)
- Web terminal demo (#178)

**Avoid At All Cost** (from 5/25 analysis):
- **Adding new providers or LLM backends** until timeout rate is below 10%. New providers compound the 52% failure surface.
- **Refactoring the entire orchestrator concurrency model** before AgentRunRecord telemetry is in production. Without data, the refactor is speculative.
- **Implementing a distributed consensus protocol** for multi-agent coordination. Gitea issues and webhooks provide sufficient synchronisation; Raft or Paxos is over-engineering.
- **Building a custom time-series database** for metrics. Quickwit and SQLite are already integrated; adding TSDB is a distraction.
- **Rewriting terraphim-agent in another language** (e.g., Go). The Rust codebase is the strategic asset; fragmentation destroys maintainability.
- **Adding AI-generated code without human review** in security-critical paths. The RUSTSEC cascade started with transitive dependency drift; AI-generated dependency changes amplify this risk.
- **Starting Phase 3 (Inter-Agent Orchestration) before Phase 2 ADF reliability is above 90%**. This is the most dangerous temptation -- #225 has the highest PageRank, but premature coordination of unstable agents multiplies failures.

---

## Cross-Cluster Dependency Graph

```
Phase 1 (Week 1)
[C] Security Remediation
     |-- #451 RUSTSEC fix
     |-- #514 CI runner restore
     +-- Exit: cargo audit == 0, CI green
            |
            v
Phase 2 (Weeks 2-3)
[A] ADF Reliability
     |-- #320 config-only reliability
     |-- #255 TOML hardening
     |-- #256 timeout tuning
     |-- #200 disk guard
     |-- #366 AgentRunRecord
     +-- Exit: timeout < 10%, disk < 70%, test-guardian green
            |
            |------------------+------------------+
            v                  v                  v
Phase 3 (Weeks 4-5)     Parallel Track        Phase 4 (Weeks 6-8)
[B] Inter-Agent           [F] PA/SO Roles       [D] Learning & KG
Orchestration             |-- #731 jmap rebuild  |-- #330 SQLite store
|-- #225 mentions         |-- #742 SO clone      |-- #266 trust-gated
|-- #230 webhooks         |-- #739 Obsidian      |-- #267 injection
|-- #160 approval         +-- Exit: PA setup     +-- Exit: cross-agent
+-- Exit: 2+ agents         < 10 min               learning < 5s
      coordinate

CRITICAL GATED DEPENDENCY:
"Cannot start Phase 3 inter-agent orchestration until Phase 2 ADF
reliability metrics show agent success rate above 90% for 48 hours."
```

---

## Cluster C: Security Remediation

### Scope

**In Scope:**
- Upgrade `rustls-webpki` to resolve RUSTSEC-2026-0049 (#448, #451)
- Restore all 5 GitHub Actions runners (#514)
- Fix `lint-and-format` CI job (`fff-search` build.rs detection)
- Fix `security-sentinel` cascading failures on downstream PRs (#506, #505, #518, #499)
- Fix `compliance-watchdog` license field failures (#494, #529, #519)
- Fix `test-guardian` Default role failures (#504, #491)
- Fix `compound-review` and `spec-validator` FAILs (#501, #498, #539, #520, #503)
- Fix zlob macOS build (#486)

**Out of Scope:**
- Full SBOM generation (not required for RUSTSEC fix)
- CI migration to another platform (GitHub Actions is sufficient)

**Avoid At All Cost:**
- Blind `cargo update` without auditing the blast radius
- Ignoring macOS build failures (#486) as "Linux only"
- Disabling security-sentinel instead of fixing root cause

### Architecture

```
+------------------+     +------------------+     +------------------+
|  Cargo.lock      | --> |  cargo audit     | --> |  CI Pipeline     |
|  (vulnerable     |     |  (RUSTSEC-2026-  |     |  (5 runners,     |
|   webpki)        |     |   0049 detected) |     |   lint, test,    |
+------------------+     +------------------+     |   security)      |
        |                      |                  +------------------+
        v                      v                          |
+-------------------------------------------+             v
|  Blast Radius Analysis                    |      +-----------+
|  -- cargo tree -i rustls-webpki           |      |  green CI |
|  -- Check reqwest, hyper, tonic versions  |      +-----------+
+-------------------------------------------+
```

### Key Design Decisions

| Decision | Rationale | Alternatives Rejected |
|----------|-----------|----------------------|
| `cargo update --precise rustls-webpki` first | Minimises API breakage; only bump the vulnerable crate | `cargo update` (rejected: blast radius too large, could break hyper/reqwest APIs) |
| Pin `rustls-webpki >= 0.102.8` in workspace `Cargo.toml` | Forces all workspace crates to use the fixed version; prevents reintroduction | Per-crate pinning (rejected: error-prone, 40+ crates to update) |
| Restore runners by fixing `fff-search` build.rs CI detection | The lint-and-format job fails because build.rs detects CI and changes behaviour; make CI detection idempotent | Disable lint-and-format job (rejected: masks real issues) |
| Use `cargo deny check advisories` in CI | Catches RUSTSEC before merge; `cargo audit` is point-in-time, `cargo deny` is gate | `cargo audit` only (rejected: no gate capability) |

### Eliminated Options (Essentialism)

| Option Rejected | Why Rejected | Risk of Including |
|-----------------|--------------|-------------------|
| Migrate from rustls to native-tls | Not in vital few; native-tls adds platform dependencies | Weeks of cross-platform testing for marginal security gain |
| Rewrite CI in Gitea Actions | GitHub Actions works; migration is a distraction | Diverts focus from actual security fixes |
| Add fuzzing suite now | Important but not on critical path | 2+ weeks of work before CI is green |

### Simplicity Check

> "What if this could be easy?"

The easiest security fix is a precise dependency bump with a workspace pin. The easiest CI fix is making `fff-search` build.rs CI-aware without side effects. No new tools, no migrations, no rewrites.

**Senior Engineer Test**: A senior engineer would approve -- this is a textbook "bump and pin" with a CI guard.

**Nothing Speculative Checklist**:
- [x] No features the user did not request
- [x] No abstractions "in case we need them later"
- [x] No premature optimisation
- [x] No error handling for scenarios that cannot occur

### File Changes

#### New Files
| File | Purpose |
|------|---------|
| `.github/workflows/security-gate.yml` | Dedicated workflow for `cargo deny check advisories` on every PR |
| `deny.toml` | `cargo-deny` configuration with RUSTSEC-2026-0049 explicit allow-list removal |

#### Modified Files
| File | Changes |
|------|---------|
| `Cargo.toml` (workspace root) | Add `rustls-webpki = ">=0.102.8"` to `[workspace.dependencies]` |
| `crates/*/Cargo.toml` | Update any direct `rustls-webpki` deps to use workspace version |
| `crates/terraphim_file_search/build.rs` | Fix CI environment detection logic |
| `.github/workflows/lint-and-format.yml` | Fix or temporarily neutralise `fff-search` build.rs interaction |
| `.github/workflows/ci.yml` | Restore runner matrix, fix any stale action versions |

#### Deleted Files
| File | Reason |
|------|--------|
| None | No deletions required -- this is a fix-and-gate cluster |

### API Design

No new public APIs. This cluster modifies dependency metadata and CI configuration.

### Error Types

No new error types. Existing `cargo audit` and CI failures are the signal.

### Test Strategy

#### Unit Tests
None -- this cluster is infrastructure and dependency management.

#### Integration Tests
| Test | Location | Purpose |
|------|----------|---------|
| `cargo audit --json` passes | CI `security-gate.yml` | Verify no critical advisories |
| `cargo deny check advisories` passes | CI `security-gate.yml` | Gate future PRs |
| `cargo build --workspace` on macOS | CI runner | Verify #486 fix |
| `cargo build --workspace` on Linux | CI runner | Verify no regressions |
| `cargo test --workspace` | CI runner | Verify test-guardian green |

#### Property Tests
None.

### Implementation Steps

#### Step C1: Blast Radius Analysis
**Files**: `Cargo.lock`, workspace `Cargo.toml`
**Description**: Run `cargo tree -i rustls-webpki` to identify every crate in the dependency graph that pulls in the vulnerable version. Document the upgrade path.
**Tests**: `cargo audit --json` to capture baseline
**Estimated**: 2 hours

#### Step C2: Precise Dependency Bump
**Files**: Workspace `Cargo.toml`, affected `crates/*/Cargo.toml`
**Description**: Run `cargo update --precise rustls-webpki` or manually bump. Add workspace-level pin.
**Tests**: `cargo build --workspace` passes
**Dependencies**: Step C1
**Estimated**: 2 hours

#### Step C3: Fix Cascading Sentinel Failures
**Files**: CI workflows, any crate with stale `deny.toml` or audit config
**Description**: Fix `security-sentinel`, `compliance-watchdog`, `compound-review`, `spec-validator`, `test-guardian` configurations that reference the old advisory ID or have stale licence fields.
**Tests**: Each sentinel passes in CI
**Dependencies**: Step C2
**Estimated**: 4 hours

#### Step C4: Restore CI Runners
**Files**: `.github/workflows/*.yml`, `crates/terraphim_file_search/build.rs`
**Description**: Fix `fff-search` build.rs CI detection. Restore all 5 runners. Fix `lint-and-format` job.
**Tests**: Full CI matrix green
**Dependencies**: Step C3
**Estimated**: 4 hours

#### Step C5: Fix zlob macOS Build
**Files**: Relevant `Cargo.toml` (zlob dependency)
**Description**: Bump or patch zlob to fix macOS compilation.
**Tests**: `cargo build` on macOS runner
**Dependencies**: None (can run in parallel with C1-C4)
**Estimated**: 2 hours

#### Step C6: Add Security Gate Workflow
**Files**: `.github/workflows/security-gate.yml`, `deny.toml`
**Description**: Add `cargo deny check advisories` as a required PR check.
**Tests**: Verify it catches a test advisory
**Dependencies**: Step C4
**Estimated**: 2 hours

### Rollback Plan

If the `rustls-webpki` bump breaks `reqwest` or `hyper` APIs:
1. Revert the workspace pin in root `Cargo.toml`
2. Revert affected `crates/*/Cargo.toml` changes
3. Restore previous `Cargo.lock` from git
4. Evaluate patch-level bump instead of minor-level bump

Feature flag: None applicable (this is dependency management).

### Performance Considerations

- `cargo deny` adds ~30 seconds to CI. Acceptable for security gating.
- No runtime performance impact -- this cluster is build-time only.

---

## Cluster A: ADF Reliability

### Scope

**In Scope:**
- Config-only agent reliability improvements A2-A5 (#320)
- Fix TOML config duplicate key crashes (#255)
- Diagnose and tune 52% agent timeout rate (#256)
- Disk usage guard and cleanup (#200, #319)
- Structured AgentRunRecord with ExitClass taxonomy (#366)
- Execution tiers with risk classification (#120)
- Provider probe hardening with circuit breakers (existing `provider_probe.rs`)
- Drift cooldown, failure cooldown, mention retry (#321-#324)

**Out of Scope:**
- Full rewrite of the orchestrator scheduler (#318 epic -- too large, start with #320)
- Nightly failure clustering via Flow DAG (#367 -- blocked by #366)
- Cost optimisation via self-improvement loop (#197 -- depends on #365 epic)

**Avoid At All Cost:**
- Changing the concurrency model without profiling data
- Adding new agent types before timeout rate is below 10%
- Ignoring disk cleanup as "DevOps, not engineering"
- Tuning timeouts blindly without AgentRunRecord telemetry

### Architecture

```
+------------------+     +------------------+     +------------------+
|  TOML Config     | --> |  ConfigValidator | --> |  AgentDefinition |
|  (agents.toml)   |     |  (dup-key guard, |     |  (sanitised)     |
|                  |     |   tier check)    |     |                  |
+------------------+     +------------------+     +------------------+
                                                        |
+------------------+     +------------------+          v
|  DiskGuard       | <-- |  Orchestrator    | <-- +-----------+
|  (< 90% usage)   |     |  (reconcile loop)|     | Scheduler |
+------------------+     +------------------+     +-----------+
        |                       |                         |
        v                       v                         v
+------------------+     +------------------+     +------------------+
|  ProviderProbe   | --> |  AgentSpawn      | --> |  AgentRunRecord  |
|  (circuit breaker|     |  (timeout, tier) |     |  (ExitClass,     |
|   per provider)  |     |                  |     |   metrics)       |
+------------------+     +------------------+     +------------------+
                                |                         |
                                v                         v
                         +-----------+            +------------------+
                         | Telemetry |            |  Persistence     |
                         | (Quickwit)|            |  (SQLite/JSON)   |
                         +-----------+            +------------------+
```

### Key Design Decisions

| Decision | Rationale | Alternatives Rejected |
|----------|-----------|----------------------|
| Fail-open on disk guard | If disk check itself fails, agents still spawn; prevents total deadlock | Fail-closed (rejected: one bad disk stat blocks all agents) |
| Tier-based timeout multipliers | Low-risk tiers get shorter timeouts; high-risk tiers get longer but monitored timeouts | Single global timeout (rejected: too coarse for diverse agent workloads) |
| TOML duplicate-key rejection at parse time | Use `toml::de::Deserializer` with custom visitor that errors on duplicates | Post-parse validation (rejected: allows invalid state to exist transiently) |
| AgentRunRecord as first-class persistable type | Every run produces a record; records feed the learning loop and nightly clustering | Ad-hoc logging only (rejected: unstructured logs cannot drive #367) |
| Per-provider circuit breakers in `ProviderHealthMap` | Existing code already has circuit breakers; harden the probe logic and TTL | Global circuit breaker (rejected: one bad provider blocks all) |

### Eliminated Options (Essentialism)

| Option Rejected | Why Rejected | Risk of Including |
|-----------------|--------------|-------------------|
| Replace TOML with YAML/JSON for config | TOML is the project's choice; migration is busywork | Weeks of migration for marginal gain |
| Add Kubernetes operator for agent scheduling | Massive over-engineering; the orchestrator is a single binary | Months of work, new failure modes, requires K8s expertise |
| Implement backpressure via custom queue | Tokio's built-in channels and semaphores are sufficient | Unnecessary complexity; tokio::sync::Semaphore handles this |

### Simplicity Check

> "What if this could be easy?"

The easiest timeout fix is: collect telemetry first (AgentRunRecord), then adjust timeout multipliers based on actual data. The easiest disk fix is: refuse to spawn when disk is above 90%. The easiest TOML fix is: reject duplicate keys at deserialization. All three are small, surgical changes.

**Senior Engineer Test**: A senior engineer would say "measure first, tune second" -- the AgentRunRecord telemetry justifies the timeout tuning.

**Nothing Speculative Checklist**:
- [x] No features the user did not request
- [x] No abstractions "in case we need them later"
- [x] No flexibility "just in case"
- [x] No premature optimisation

### File Changes

#### New Files
| File | Purpose |
|------|---------|
| `crates/terraphim_orchestrator/src/config_validator.rs` | TOML duplicate-key detection and tier validation |
| `crates/terraphim_orchestrator/src/disk_guard.rs` | Disk usage checking with fail-open semantics |
| `crates/terraphim_orchestrator/src/execution_tier.rs` | Risk classification enum and timeout multiplier mapping |
| `crates/terraphim_orchestrator/tests/adf_reliability_tests.rs` | Integration tests for timeout, disk, and tier behaviour |

#### Modified Files
| File | Changes |
|------|---------|
| `crates/terraphim_orchestrator/src/config.rs` | Add `ExecutionTier` to `AgentDefinition`; add `disk_usage_threshold` validation |
| `crates/terraphim_orchestrator/src/agent_run_record.rs` | Add `AgentRunRecord::from_spawn_result` constructor; integrate with scheduler |
| `crates/terraphim_orchestrator/src/provider_probe.rs` | Harden `ProviderHealthMap` TTL and circuit breaker thresholds |
| `crates/terraphim_orchestrator/src/scheduler.rs` | Apply tier-based timeouts; check disk guard before spawn |
| `crates/terraphim_orchestrator/src/error.rs` | Add `ConfigValidationError`, `DiskGuardError`, `TimeoutError` variants |
| `crates/terraphim_orchestrator/src/lib.rs` | Wire new modules |

#### Deleted Files
| File | Reason |
|------|--------|
| None | No deletions required |

### API Design

#### Public Types
```rust
/// Risk classification for an agent execution.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExecutionTier {
    /// Read-only, no side effects (e.g., query, search)
    LowRisk,
    /// Standard agent with bounded side effects (e.g., lint, format)
    Standard,
    /// Potentially destructive (e.g., code generation, migration)
    HighRisk,
    /// Human approval required before execution
    Critical,
}

impl ExecutionTier {
    /// Timeout multiplier relative to base timeout.
    pub fn timeout_multiplier(&self) -> f64 {
        match self {
            ExecutionTier::LowRisk => 0.5,
            ExecutionTier::Standard => 1.0,
            ExecutionTier::HighRisk => 2.0,
            ExecutionTier::Critical => 3.0,
        }
    }
}

/// Result of disk guard check.
#[derive(Debug, Clone)]
pub struct DiskGuardResult {
    pub usage_percent: u8,
    pub threshold_percent: u8,
    pub allowed: bool,
}

/// Validated orchestrator config with invariant enforcement.
pub struct ValidatedConfig {
    inner: OrchestratorConfig,
    duplicates_checked: bool,
    tiers_assigned: bool,
}
```

#### Public Functions
```rust
/// Parse and validate orchestrator configuration from TOML bytes.
///
/// # Errors
/// Returns `ConfigValidationError::DuplicateKey` if duplicate keys detected.
/// Returns `ConfigValidationError::InvalidTier` if an agent references an unknown tier.
pub fn parse_config(toml_bytes: &[u8]) -> Result<ValidatedConfig, ConfigValidationError>;

/// Check disk usage against configured threshold.
///
/// # Returns
/// `DiskGuardResult` with `allowed == true` if usage is below threshold.
/// Always returns `allowed == true` if the check itself fails (fail-open).
pub async fn check_disk_guard(threshold_percent: u8) -> DiskGuardResult;

/// Compute tier-adjusted timeout from base duration.
pub fn adjusted_timeout(base: Duration, tier: ExecutionTier) -> Duration;
```

### Error Types
```rust
#[derive(Debug, thiserror::Error)]
pub enum ConfigValidationError {
    #[error("duplicate key '{key}' in section '{section}'")]
    DuplicateKey { section: String, key: String },

    #[error("agent '{agent}' references unknown execution tier '{tier}'")]
    InvalidTier { agent: String, tier: String },

    #[error("disk guard check failed: {0}")]
    DiskCheckFailed(String),

    #[error("timeout exceeded after {elapsed:?} (tier {tier:?}, limit {limit:?})")]
    TimeoutExceeded {
        elapsed: Duration,
        tier: ExecutionTier,
        limit: Duration,
    },
}
```

### Test Strategy

#### Unit Tests
| Test | Location | Purpose |
|------|----------|---------|
| `test_duplicate_key_rejected` | `config_validator.rs` | Verify TOML with duplicate keys fails |
| `test_valid_tier_assigned` | `config_validator.rs` | Verify standard tier parsing |
| `test_disk_guard_below_threshold` | `disk_guard.rs` | Verify spawn allowed |
| `test_disk_guard_above_threshold` | `disk_guard.rs` | Verify spawn refused |
| `test_disk_guard_fail_open` | `disk_guard.rs` | Verify spawn allowed when stat fails |
| `test_timeout_multiplier_low_risk` | `execution_tier.rs` | Verify 0.5x multiplier |
| `test_timeout_multiplier_critical` | `execution_tier.rs` | Verify 3.0x multiplier |

#### Integration Tests
| Test | Location | Purpose |
|------|----------|---------|
| `test_agent_spawn_respects_disk_guard` | `tests/adf_reliability_tests.rs` | End-to-end disk guard |
| `test_agent_timeout_triggers_exit_class` | `tests/adf_reliability_tests.rs` | Verify Timeout exit class |
| `test_tier_based_timeout_observed` | `tests/adf_reliability_tests.rs` | High-risk agent gets longer timeout |
| `test_provider_circuit_breaker_opens` | `tests/adf_reliability_tests.rs` | Circuit breaker trips after threshold |

#### Property Tests
```rust
proptest! {
    #[test]
    fn disk_guard_never_panics(usage: u8, threshold: u8) {
        let _ = check_disk_guard(threshold);
    }

    #[test]
    fn timeout_multiplier_is_positive(tier: ExecutionTier) {
        assert!(tier.timeout_multiplier() > 0.0);
    }
}
```

### Implementation Steps

#### Step A1: Config Validator and TOML Hardening
**Files**: `config_validator.rs`, `config.rs`
**Description**: Implement duplicate-key detection. Add `ExecutionTier` to `AgentDefinition`.
**Tests**: Unit tests for validator
**Estimated**: 4 hours

#### Step A2: Disk Guard
**Files**: `disk_guard.rs`, `scheduler.rs`
**Description**: Implement `check_disk_guard`. Integrate into scheduler pre-spawn check.
**Tests**: Unit and integration tests
**Dependencies**: Step A1
**Estimated**: 3 hours

#### Step A3: Execution Tiers and Timeout Multipliers
**Files**: `execution_tier.rs`, `scheduler.rs`, `config.rs`
**Description**: Define tiers. Apply multipliers in scheduler. Add to TOML schema.
**Tests**: Unit tests for multipliers; integration for scheduler
**Dependencies**: Step A1
**Estimated**: 3 hours

#### Step A4: AgentRunRecord Integration
**Files**: `agent_run_record.rs`, `scheduler.rs`
**Description**: Ensure every spawn produces a record. Wire ExitClass classification into spawn lifecycle.
**Tests**: Integration test verifying record creation
**Dependencies**: Step A3
**Estimated**: 4 hours

#### Step A5: Provider Probe Hardening
**Files**: `provider_probe.rs`
**Description**: Tune circuit breaker thresholds (failure_threshold: 3, cooldown: 30s). Reduce TTL to 60s for faster recovery.
**Tests**: Integration test for circuit breaker trip
**Dependencies**: None (can parallel with A1-A3)
**Estimated**: 3 hours

#### Step A6: Timeout Tuning (Data-Driven)
**Files**: `scheduler.rs`, metrics analysis
**Description**: After A4 produces 24h of telemetry, analyse AgentRunRecord timeout patterns. Adjust base timeout and multipliers.
**Tests**: Verify timeout rate < 10% in staging
**Dependencies**: Steps A3, A4 (must have telemetry)
**Estimated**: 4 hours

#### Step A7: Re-enable Test-Guardian and Full Deployment
**Files**: `lib.rs`, CI workflows
**Description**: Flip `test-guardian` back on. Deploy to D1-D3 environments.
**Tests**: Full CI green; deployment smoke tests
**Dependencies**: Steps A2, A5, A6
**Estimated**: 2 hours

### Rollback Plan

If timeout tuning makes things worse:
1. Revert `scheduler.rs` timeout multiplier changes
2. Restore previous base timeout in `config.rs`
3. Keep AgentRunRecord telemetry active for further analysis

Feature flag: `TIER_TIMEOUTS_ENABLED` environment variable (default true after verification).

### Performance Considerations

| Metric | Target | Measurement |
|--------|--------|-------------|
| Agent timeout rate | < 10% | AgentRunRecord telemetry |
| Disk utilisation | < 70% | `check_disk_guard` + OS metrics |
| Config parse time | < 5ms | Benchmark in `config_validator.rs` |
| Provider probe latency | < 500ms | `ProbeResult.latency_ms` |

- Disk guard runs once per reconciliation tick, not per spawn, to avoid `statvfs` syscall overhead.
- AgentRunRecord persistence is async via tokio channel to avoid blocking the scheduler.

---

## Cluster B: Inter-Agent Orchestration

### Scope

**In Scope:**
- Gitea mention parsing and routing (#225)
- Webhook-driven mention detection (#230)
- Suggestion approval workflow (#160)
- AgentEvolution haystack ServiceType (#728)
- Wire `agent_evolution` into orchestrator (#727, #704)
- Steering/follow-up message queues (#687)
- Typed tool hooks (#686)

**Out of Scope:**
- Cross-provider context serialisation (#685) -- requires Cluster D shared learning first
- JSONL RPC envelope (#688) -- protocol negotiation, not essential for Gitea coordination
- Full multi-agent consensus -- Gitea issue state is sufficient synchronisation

**Avoid At All Cost:**
- Starting this cluster before Cluster A achieves > 90% agent success rate for 48 hours
- Building a custom message bus when Gitea issues + webhooks suffice
- Adding real-time WebSocket coordination -- polling webhooks are simpler and more reliable
- Orchestrating more than 3 agents in a single workflow before proven stable

### Architecture

```
+---------------+     +------------------+     +------------------+
|  Gitea Webhook | --> |  MentionRouter   | --> |  AgentQueue      |
|  (issue_comment|     |  (parse @agent,  |     |  (per-agent      |
|   mention)     |     |   extract task)  |     |   work queue)    |
+---------------+     +------------------+     +------------------+
                                                        |
+------------------+     +------------------+          v
|  ApprovalGate    | <-- |  Orchestrator    | <-- +-----------+
|  (human-in-loop  |     |  (multi-agent    |     | Scheduler |
|   for critical)  |     |   dispatch)      |     +-----------+
+------------------+     +------------------+          |
        |                       |                      v
        v                       v               +-----------+
+-------------------------------------------+   | AgentRun  |
|  AgentEvolution Wiring                    |   | Record    |
|  -- terraphim_agent_evolution crate       |   +-----------+
|  -- ServiceType::AgentEvolution           |         |
+-------------------------------------------+         v
                                               +------------------+
                                               |  SharedLearning  |
                                               |  Store (Cluster D|
                                               |   downstream)    |
                                               +------------------+
```

### Key Design Decisions

| Decision | Rationale | Alternatives Rejected |
|----------|-----------|----------------------|
| Gitea webhooks as the sole coordination transport | Single source of truth; audit trail built-in; no new infrastructure | Custom MQTT/Redis pub-sub (rejected: new infrastructure, no audit trail) |
| Per-agent work queues in SQLite | Simple, durable, survives restarts; same store as Cluster D | In-memory channels (rejected: lost on restart) |
| Approval workflow as a Gitea PR review requirement | Uses existing Gitea permissions; no custom auth | Custom approval UI (rejected: months of frontend work) |
| AgentEvolution wired as a `ServiceType` variant | Fits existing haystack architecture; minimal intrusion | Standalone microservice (rejected: deployment complexity) |
| Typed tool hooks via `serde_json::Value` schema validation | Flexible enough for evolution; strict enough for safety | Full DSL parser (rejected: over-engineered for current needs) |

### Eliminated Options (Essentialism)

| Option Rejected | Why Rejected | Risk of Including |
|-----------------|--------------|-------------------|
| Real-time WebSocket coordination | Adds operational complexity; webhooks are sufficient | Requires WebSocket server, reconnection logic, state management |
| Custom consensus protocol (Raft) | Gitea issue state is the consensus | Months of distributed systems work; Raft is not needed for agent task assignment |
| Agent marketplace / plugin registry | Not in vital few; no user demand yet | Scope explosion; marketplace security model is its own project |

### Simplicity Check

> "What if this could be easy?"

The easiest multi-agent coordination is: webhook arrives, parse mention, append task to agent's queue, agent polls queue, executes, posts result as Gitea comment. No WebSockets, no consensus, no custom bus. Gitea IS the bus.

**Senior Engineer Test**: A senior engineer would approve -- this is a well-known pattern (GitOps for agents) and leverages existing infrastructure.

**Nothing Speculative Checklist**:
- [x] No features the user did not request
- [x] No abstractions "in case we need them later"
- [x] No flexibility "just in case"
- [x] No premature optimisation

### File Changes

#### New Files
| File | Purpose |
|------|---------|
| `crates/terraphim_orchestrator/src/mention_router.rs` | Parse Gitea issue comments for @agent mentions |
| `crates/terraphim_orchestrator/src/agent_queue.rs` | Per-agent SQLite-backed work queue |
| `crates/terraphim_orchestrator/src/approval_gate.rs` | Human-in-the-loop approval before critical actions |
| `crates/terraphim_orchestrator/src/webhook_handler.rs` | Axum handler for Gitea webhooks |
| `crates/terraphim_orchestrator/tests/inter_agent_tests.rs` | Integration tests for multi-agent coordination |

#### Modified Files
| File | Changes |
|------|---------|
| `crates/terraphim_orchestrator/src/mention.rs` | Extend existing mention parsing with routing logic |
| `crates/terraphim_orchestrator/src/webhook.rs` | Add Gitea webhook verification (HMAC signature) |
| `crates/terraphim_orchestrator/src/dispatcher.rs` | Multi-agent dispatch logic; respect approval gates |
| `crates/terraphim_orchestrator/src/scheduler.rs` | Poll agent queues during reconciliation |
| `crates/terraphim_orchestrator/src/kg_router.rs` | Add `ServiceType::AgentEvolution` variant |
| `crates/terraphim_service/src/lib.rs` | Register AgentEvolution service type |
| `crates/terraphim_orchestrator/src/error.rs` | Add `WebhookError`, `QueueError`, `ApprovalError` |
| `crates/terraphim_orchestrator/src/lib.rs` | Wire new modules |

#### Deleted Files
| File | Reason |
|------|--------|
| None | No deletions required |

### API Design

#### Public Types
```rust
/// A task extracted from a Gitea mention.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MentionTask {
    pub issue_number: u64,
    pub comment_id: u64,
    pub mentioned_agent: String,
    pub command: String,
    pub requester: String,
    pub timestamp: DateTime<Utc>,
    pub approval_status: ApprovalStatus,
}

/// Human approval state for critical actions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ApprovalStatus {
    NotRequired,
    Pending,
    Approved,
    Rejected,
}

/// Per-agent durable work queue.
pub struct AgentQueue {
    agent_name: String,
    db_pool: SqlitePool,
}

/// Result of webhook processing.
#[derive(Debug, Clone)]
pub struct WebhookResult {
    pub tasks_queued: Vec<MentionTask>,
    pub agents_notified: Vec<String>,
}
```

#### Public Functions
```rust
/// Parse a Gitea issue_comment webhook payload into tasks.
///
/// # Errors
/// Returns `WebhookError::InvalidPayload` if JSON is malformed.
/// Returns `WebhookError::UnsupportedEvent` if not an issue_comment.
pub fn parse_webhook_payload(body: &str, secret: &str) -> Result<WebhookResult, WebhookError>;

/// Enqueue a task for the specified agent.
///
/// # Errors
/// Returns `QueueError::AgentNotFound` if agent does not exist.
pub async fn enqueue_task(&self, task: &MentionTask) -> Result<u64, QueueError>;

/// Dequeue the next pending task for this agent.
pub async fn dequeue_task(&self) -> Result<Option<MentionTask>, QueueError>;

/// Check if a task requires and has received human approval.
pub async fn check_approval(&self, task_id: u64) -> Result<ApprovalStatus, ApprovalError>;
```

### Error Types
```rust
#[derive(Debug, thiserror::Error)]
pub enum WebhookError {
    #[error("invalid webhook payload: {0}")]
    InvalidPayload(String),

    #[error("unsupported event type: {0}")]
    UnsupportedEvent(String),

    #[error("HMAC signature mismatch")]
    SignatureMismatch,

    #[error("mention parsing failed: {0}")]
    ParseError(String),
}

#[derive(Debug, thiserror::Error)]
pub enum QueueError {
    #[error("agent '{0}' not found in registry")]
    AgentNotFound(String),

    #[error("database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("queue full for agent '{0}'")]
    QueueFull(String),
}

#[derive(Debug, thiserror::Error)]
pub enum ApprovalError {
    #[error("task {0} not found")]
    TaskNotFound(u64),

    #[error("approval timeout for task {0}")]
    Timeout(u64),
}
```

### Test Strategy

#### Unit Tests
| Test | Location | Purpose |
|------|----------|---------|
| `test_parse_simple_mention` | `mention_router.rs` | Extract @agent from comment |
| `test_parse_mention_with_command` | `mention_router.rs` | Extract command after mention |
| `test_webhook_hmac_verification` | `webhook_handler.rs` | Reject bad signatures |
| `test_enqueue_dequeue_roundtrip` | `agent_queue.rs` | Queue persistence |
| `test_approval_gate_blocks_critical` | `approval_gate.rs` | Critical tier requires approval |

#### Integration Tests
| Test | Location | Purpose |
|------|----------|---------|
| `test_two_agents_coordinate_on_issue` | `tests/inter_agent_tests.rs` | Agent A mentions Agent B; B executes |
| `test_approval_workflow_end_to_end` | `tests/inter_agent_tests.rs` | Human approves critical suggestion |
| `test_webhook_driven_task_creation` | `tests/inter_agent_tests.rs` | POST webhook -> task enqueued |
| `test_agent_evolution_service_wired` | `tests/inter_agent_tests.rs` | AgentEvolution service type resolves |

#### Property Tests
```rust
proptest! {
    #[test]
    fn mention_router_never_panics(comment: String) {
        let _ = extract_mentions(&comment);
    }
}
```

### Implementation Steps

#### Step B1: Mention Router
**Files**: `mention_router.rs`, `mention.rs`
**Description**: Parse @agent mentions from Gitea issue comments. Extract commands.
**Tests**: Unit tests for parsing
**Estimated**: 4 hours

#### Step B2: Webhook Handler with HMAC Verification
**Files**: `webhook_handler.rs`, `webhook.rs`
**Description**: Axum handler for Gitea webhooks. Verify HMAC signature. Route to mention parser.
**Tests**: Unit tests for HMAC; integration for routing
**Dependencies**: Step B1
**Estimated**: 4 hours

#### Step B3: Agent Work Queue (SQLite)
**Files**: `agent_queue.rs`
**Description**: Durable per-agent queue. Uses same SQLite connection pool as Cluster D shared learning store.
**Tests**: Unit tests for enqueue/dequeue
**Estimated**: 4 hours

#### Step B4: Approval Gate
**Files**: `approval_gate.rs`
**Description**: Check `ApprovalStatus` before dispatching critical-tier tasks. Integrate with Gitea PR review API for human approval.
**Tests**: Unit and integration tests
**Dependencies**: Step B3
**Estimated**: 4 hours

#### Step B5: Multi-Agent Dispatcher
**Files**: `dispatcher.rs`, `scheduler.rs`
**Description**: Extend dispatcher to poll agent queues. Respect approval gates. Route completed tasks back to Gitea comments.
**Tests**: Integration test for two-agent coordination
**Dependencies**: Steps B2, B3, B4
**Estimated**: 6 hours

#### Step B6: AgentEvolution Wiring
**Files**: `kg_router.rs`, `terraphim_service/src/lib.rs`
**Description**: Add `ServiceType::AgentEvolution`. Wire `terraphim_agent_evolution` crate into orchestrator.
**Tests**: Integration test verifying service resolution
**Dependencies**: None (can parallel with B1-B4)
**Estimated**: 3 hours

### Rollback Plan

If multi-agent coordination causes cascading failures:
1. Set `INTER_AGENT_DISPATCH_ENABLED=false` environment variable
2. Revert `dispatcher.rs` to single-agent-only mode
3. Keep webhook handler running in "log only" mode for debugging

Feature flag: `INTER_AGENT_DISPATCH_ENABLED` (default false until B5 integration tests pass).

### Performance Considerations

| Metric | Target | Measurement |
|--------|--------|-------------|
| Webhook processing latency | < 200ms | Handler timing |
| Queue depth per agent | < 10 | Queue monitor |
| Approval workflow latency | < 1 hour (human) | Gitea review API |
| Multi-agent coordination success | > 80% | Integration test pass rate |

- Webhook handler is idempotent: duplicate deliveries are deduplicated by comment_id.
- Agent queue polling uses exponential backoff when empty to reduce database load.

---

## Cluster D: Learning and KG Evolution

### Scope

**In Scope:**
- Phase 1: SQLite shared learning store (#330)
- SharedLearning store with trust-gated promotion (#266)
- Cross-agent learning injection (#267)
- EIDOS confidence-driven KG promotion (#602)
- Dimensional verdict scoring (#600)
- Learning-driven command correction Phases 2 and 3 (#810)
- Session connectors for Aider and Cline (#566)

**Out of Scope:**
- Phase 2: Gitea wiki sync (#331) -- blocked by #330
- Phase 3: Quality loop and verification (#332) -- blocked by #331
- Learning evolution DAG (#269) -- graph complexity not justified yet
- EIDOS episodic reasoning (#601) -- depends on #602

**Avoid At All Cost:**
- Building a distributed learning store before a single-node SQLite store works
- Promoting learnings to KG without trust gating (poisoning risk)
- Sharing learnings across agents before verification loop exists
- Starting wiki sync before SQLite store schema is stable

### Architecture

```
+------------------+     +------------------+     +------------------+
|  AgentRunRecord  | --> |  LearningExtractor| --> |  SQLite Store    |
|  (from Cluster A)|     |  (pattern ->     |     |  (lessons table) |
|                  |     |   lesson)        |     |                  |
+------------------+     +------------------+     +------------------+
                                                        |
+------------------+     +------------------+          v
|  KG Promotion    | <-- |  TrustGate       | <-- +-----------+
|  (EIDOS scoring) |     |  (confidence >   |     | Verification |
|                  |     |   threshold)     |     | Loop        |
+------------------+     +------------------+     +-----------+
        |                       |
        v                       v
+------------------+     +------------------+
|  terraphim-automata |  |  Cross-Agent     |
|  (thesaurus update)   |  |  Injection       |
+------------------+     +------------------+
```

### Key Design Decisions

| Decision | Rationale | Alternatives Rejected |
|----------|-----------|----------------------|
| SQLite for shared learning store | Single file, zero ops, ACID, works on D1-D3 without new infrastructure | PostgreSQL (rejected: ops overhead for edge deployment) |
| Trust-gated promotion with confidence threshold | Prevents low-quality learnings from poisoning the KG | Immediate promotion (rejected: quality risk) |
| Lesson schema: (pattern, correction, exit_class, confidence, verified) | Minimal schema that captures the essential learning | Full provenance graph (rejected: schema complexity) |
| EIDOS scoring as composite of exit class frequency and human verification | Combines statistical and human signals | Pure statistical (rejected: cold start problem) |
| Cross-agent injection via SQLite read, not message passing | Simple, durable, agents poll at their own rate | Pub-sub injection (rejected: adds infrastructure) |

### Eliminated Options (Essentialism)

| Option Rejected | Why Rejected | Risk of Including |
|-----------------|--------------|-------------------|
| PostgreSQL with replication | Operational complexity for edge deployments | Requires DBA expertise, connection pooling, migrations |
| Blockchain for learning provenance | Massive over-engineering | Irrelevant to current scale; adds latency and cost |
| Neural embedding for lesson similarity | Not in vital few; Aho-Corasick matching is sufficient | Adds ML inference dependency; slows down learning loop |

### Simplicity Check

> "What if this could be easy?"

The easiest learning store is a SQLite table with five columns. The easiest trust gate is a confidence threshold. The easiest cross-agent injection is SELECT * FROM lessons WHERE verified = true. No message queues, no distributed databases, no ML models.

**Senior Engineer Test**: A senior engineer would say "start with SQLite, migrate to Postgres when you have a DBA" -- this is the canonical path.

**Nothing Speculative Checklist**:
- [x] No features the user did not request
- [x] No abstractions "in case we need them later"
- [x] No flexibility "just in case"
- [x] No premature optimisation

### File Changes

#### New Files
| File | Purpose |
|------|---------|
| `crates/terraphim_persistence/src/learning_store.rs` | SQLite schema and CRUD for lessons |
| `crates/terraphim_persistence/src/trust_gate.rs` | Confidence threshold and verification logic |
| `crates/terraphim_persistence/src/eidos_scorer.rs` | EIDOS confidence scoring |
| `crates/terraphim_persistence/tests/learning_store_tests.rs` | Integration tests |
| `crates/terraphim_agent/src/shared_learning/injector.rs` | Cross-agent learning injection client |

#### Modified Files
| File | Changes |
|------|---------|
| `crates/terraphim_persistence/src/lib.rs` | Wire learning_store, trust_gate, eidos_scorer |
| `crates/terraphim_persistence/src/settings.rs` | Add SQLite path configuration |
| `crates/terraphim_orchestrator/src/learning.rs` | Integrate learning extraction from AgentRunRecord |
| `crates/terraphim_orchestrator/src/agent_run_record.rs` | Feed records into learning extractor |
| `crates/terraphim_automata/src/matcher.rs` | Accept dynamic thesaurus updates from learning store |
| `crates/terraphim_agent/src/shared_learning/mod.rs` | Add injector module |
| `crates/terraphim_sessions/src/connector/aider.rs` | Aider session connector |
| `crates/terraphim_sessions/src/connector/cline.rs` | Cline session connector |

#### Deleted Files
| File | Reason |
|------|--------|
| None | No deletions required |

### API Design

#### Public Types
```rust
/// A learned lesson extracted from agent execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Lesson {
    pub id: Uuid,
    pub pattern: String,
    pub correction: String,
    pub exit_class: ExitClass,
    pub confidence: f64,
    pub verified: bool,
    pub source_agent: String,
    pub created_at: DateTime<Utc>,
}

/// Trust gate configuration.
#[derive(Debug, Clone)]
pub struct TrustGateConfig {
    pub min_confidence: f64,
    pub require_verification: bool,
    pub max_lessons_per_day: u32,
}

/// EIDOS scoring result.
#[derive(Debug, Clone)]
pub struct EidosScore {
    pub lesson_id: Uuid,
    pub composite_score: f64,
    pub frequency_component: f64,
    pub verification_component: f64,
}

/// Shared learning store handle.
pub struct LearningStore {
    pool: SqlitePool,
    trust_gate: TrustGateConfig,
}
```

#### Public Functions
```rust
/// Create or open the shared learning store at the given path.
pub async fn open_learning_store(path: &Path) -> Result<LearningStore, PersistenceError>;

/// Store a new lesson. Confidence is initialised from exit class frequency.
///
/// # Errors
/// Returns `PersistenceError::DuplicateLesson` if an identical pattern exists.
pub async fn store_lesson(&self, lesson: &Lesson) -> Result<Uuid, PersistenceError>;

/// Promote lessons to KG that pass the trust gate.
///
/// # Returns
/// Vec of lesson IDs that were promoted.
pub async fn promote_verified_lessons(&self) -> Result<Vec<Uuid>, PersistenceError>;

/// Compute EIDOS scores for all pending lessons.
pub async fn score_pending_lessons(&self) -> Result<Vec<EidosScore>, PersistenceError>;

/// Inject verified lessons from other agents into this agent's context.
pub async fn inject_cross_agent_lessons(
    &self,
    agent_name: &str,
) -> Result<Vec<Lesson>, PersistenceError>;
```

### Error Types
```rust
#[derive(Debug, thiserror::Error)]
pub enum PersistenceError {
    #[error("database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("duplicate lesson for pattern '{0}'")]
    DuplicateLesson(String),

    #[error("trust gate rejected lesson {0}: confidence {1} below threshold {2}")]
    TrustGateRejected(Uuid, f64, f64),

    #[error("lesson {0} not found")]
    LessonNotFound(Uuid),

    #[error("thesaurus update failed: {0}")]
    ThesaurusUpdate(String),
}
```

### Test Strategy

#### Unit Tests
| Test | Location | Purpose |
|------|----------|---------|
| `test_store_and_retrieve_lesson` | `learning_store.rs` | Round-trip CRUD |
| `test_duplicate_lesson_rejected` | `learning_store.rs` | Uniqueness enforcement |
| `test_trust_gate_blocks_low_confidence` | `trust_gate.rs` | Confidence threshold |
| `test_eidos_score_computation` | `eidos_scorer.rs` | Composite scoring |
| `test_inject_filters_by_agent` | `injector.rs` | Cross-agent isolation |

#### Integration Tests
| Test | Location | Purpose |
|------|----------|---------|
| `test_learning_end_to_end` | `tests/learning_store_tests.rs` | Record -> lesson -> promotion |
| `test_cross_agent_injection` | `tests/learning_store_tests.rs` | Agent A lesson visible to Agent B |
| `test_aider_session_import` | `terraphim_sessions/tests` | Aider sessions produce lessons |
| `test_cline_session_import` | `terraphim_sessions/tests` | Cline sessions produce lessons |

#### Property Tests
```rust
proptest! {
    #[test]
    fn eidos_score_is_between_zero_and_one(
        freq: u32,
        verified: bool,
    ) {
        let score = compute_eidos_score(freq, verified);
        assert!(score.composite_score >= 0.0 && score.composite_score <= 1.0);
    }
}
```

### Implementation Steps

#### Step D1: SQLite Schema and Learning Store
**Files**: `learning_store.rs`, `settings.rs`
**Description**: Create `lessons` table. Implement CRUD.
**Tests**: Unit tests for round-trip
**Estimated**: 4 hours

#### Step D2: Trust Gate
**Files**: `trust_gate.rs`
**Description**: Implement confidence threshold and verification requirements.
**Tests**: Unit tests for blocking and allowing
**Dependencies**: Step D1
**Estimated**: 3 hours

#### Step D3: EIDOS Scorer
**Files**: `eidos_scorer.rs`
**Description**: Composite scoring from frequency and verification.
**Tests**: Unit tests for score computation
**Dependencies**: Step D2
**Estimated**: 3 hours

#### Step D4: Learning Extraction from AgentRunRecord
**Files**: `terraphim_orchestrator/src/learning.rs`, `agent_run_record.rs`
**Description**: Extract lessons from failed runs. Store via LearningStore.
**Tests**: Integration test for record -> lesson pipeline
**Dependencies**: Steps D1, Cluster A Step A4
**Estimated**: 4 hours

#### Step D5: Cross-Agent Injection
**Files**: `terraphim_agent/src/shared_learning/injector.rs`
**Description**: Poll LearningStore for verified lessons from other agents. Inject into prompt context.
**Tests**: Integration test for cross-agent visibility
**Dependencies**: Steps D2, D4
**Estimated**: 4 hours

#### Step D6: Session Connectors (Aider, Cline)
**Files**: `terraphim_sessions/src/connector/aider.rs`, `cline.rs`
**Description**: Import Aider and Cline session files. Extract patterns for learning.
**Tests**: Integration tests for session import
**Dependencies**: None (can parallel with D1-D3)
**Estimated**: 4 hours

### Rollback Plan

If learning store corruption occurs:
1. Stop `learning.rs` extraction in orchestrator
2. Restore SQLite from last backup (single file copy)
3. Re-enable extraction with validation stricter

Feature flag: `LEARNING_STORE_ENABLED` (default true after D4 integration tests pass).

### Performance Considerations

| Metric | Target | Measurement |
|--------|--------|-------------|
| Lesson store latency | < 10ms | SQLite benchmark |
| EIDOS scoring batch | < 100ms for 1000 lessons | Batch benchmark |
| Cross-agent injection latency | < 5s | Integration test |
| Store size growth | < 100 MB/month | File size monitoring |

- SQLite WAL mode enabled for concurrent reads during writes.
- Learning extraction is async via channel to avoid blocking agent spawn.
- EIDOS scoring runs as a nightly batch, not inline.

---

## Cluster F: PA/SO Roles

### Scope

**In Scope:**
- PA: Rebuild terraphim-agent with JMAP (#731)
- PA: Populate Obsidian vault (#739)
- SO: Clone to durable path and add role (#742)
- PA: Add PA role to embedded_config (#732)
- PA: Wrapper script `~/bin/terraphim-agent-pa` (#733)
- PA: How-to documentation (#734)

**Out of Scope:**
- PA blog post (#735) -- marketing, not engineering
- PA full epic end-to-end (#730) -- blocked by sub-issues; work on unblocked ones
- SO demo refresh (#741) -- blocked by 4 issues

**Avoid At All Cost:**
- Rebuilding terraphim-agent from scratch instead of adding JMAP as a service module
- Hard-coding Obsidian vault paths -- must be configurable
- Skipping the wrapper script -- it is the user-facing installation experience
- Blocking #731 on orchestrator fixes -- this cluster is parallelisable with Cluster A

### Architecture

```
+------------------+     +------------------+     +------------------+
|  terraphim-agent | --> |  JMAP Service    | --> |  Obsidian Vault  |
|  (PA role)       |     |  (haystack_jmap) |     |  (markdown files)|
|                  |     |                  |     |                  |
+------------------+     +------------------+     +------------------+
        |
        v
+------------------+     +------------------+
|  Embedded Config | --> |  PA Role Config  |
|  (terraphim_config|     |  (jmap_server,   |
|   crate)         |     |   vault_path)    |
+------------------+     +------------------+
        |
        v
+------------------+     +------------------+
|  SO Role         | --> |  Durable Path    |
|  (system operator)|     |  (clone + setup) |
+------------------+     +------------------+
```

### Key Design Decisions

| Decision | Rationale | Alternatives Rejected |
|----------|-----------|----------------------|
| JMAP as a `ServiceType` in existing haystack architecture | Reuses `haystack_jmap` crate; no new protocol implementation | Custom JMAP client (rejected: `haystack_jmap` already exists) |
| Obsidian vault as a local directory of markdown files | Simple, standard, user-controls the location | Custom sync protocol (rejected: Obsidian users expect plain files) |
| PA role configured via `embedded_config` TOML | Consistent with existing config patterns; no new config system | Environment variables only (rejected: harder to document and validate) |
| Wrapper script generated by terraphim-agent on first run | Self-documenting installation; no external installer | Makefile or package manager (rejected: adds build complexity) |
| SO role as a separate agent profile in same binary | One binary, multiple roles; simpler deployment | Separate `terraphim-agent-so` binary (rejected: code duplication) |

### Eliminated Options (Essentialism)

| Option Rejected | Why Rejected | Risk of Including |
|-----------------|--------------|-------------------|
| Full Electron desktop app for PA | Months of work; existing CLI + Obsidian is sufficient | Diverts engineering from core infrastructure |
| Real-time bidirectional JMAP sync | Over-engineered; polling is sufficient for personal assistant | Adds WebSocket complexity; not needed for search |
| Cloud-hosted Obsidian sync | Requires infrastructure; local vault is the privacy-first approach | Violates project's privacy-first positioning |

### Simplicity Check

> "What if this could be easy?"

The easiest PA role is: add JMAP server config to existing TOML, query JMAP on command, write results to a markdown file in a directory. The easiest SO role is: clone a repo to a known path, set a role flag. Both are configuration and wiring, not new architecture.

**Senior Engineer Test**: A senior engineer would approve -- this is "connect existing pipes, don't build new pipes."

**Nothing Speculative Checklist**:
- [x] No features the user did not request
- [x] No abstractions "in case we need them later"
- [x] No flexibility "just in case"
- [x] No premature optimisation

### File Changes

#### New Files
| File | Purpose |
|------|---------|
| `crates/terraphim_agent/src/onboarding/pa_setup.rs` | PA role onboarding flow |
| `crates/terraphim_agent/src/onboarding/so_setup.rs` | SO role onboarding flow |
| `crates/terraphim_agent/src/commands/pa.rs` | PA command handlers (search, sync) |
| `crates/terraphim_agent/src/commands/so.rs` | SO command handlers (clone, status) |
| `scripts/generate-pa-wrapper.sh` | Generate `~/bin/terraphim-agent-pa` wrapper |
| `docs/pa-how-to.md` | End-user PA setup documentation |

#### Modified Files
| File | Changes |
|------|---------|
| `crates/terraphim_config/src/lib.rs` | Add `PaConfig` and `SoConfig` structs |
| `crates/terraphim_config/src/llm_router.rs` | Add PA/SO role routing rules |
| `crates/terraphim_agent/src/lib.rs` | Wire PA and SO command modules |
| `crates/terraphim_agent/src/main.rs` | Add `--role pa` and `--role so` CLI flags |
| `crates/terraphim_agent/src/service.rs` | Route PA queries through JMAP service |
| `crates/terraphim_agent/src/embedded_config.rs` | Add PA role defaults (if file exists) |
| `crates/haystack_jmap/src/lib.rs` | Expose search and fetch helpers for PA |

#### Deleted Files
| File | Reason |
|------|--------|
| None | No deletions required |

### API Design

#### Public Types
```rust
/// Configuration for the Personal Assistant role.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaConfig {
    pub jmap_server_url: String,
    pub jmap_credentials: JmapCredentials,
    pub obsidian_vault_path: PathBuf,
    pub sync_interval_minutes: u64,
}

/// Configuration for the System Operator role.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SoConfig {
    pub durable_repo_url: String,
    pub durable_clone_path: PathBuf,
    pub role_name: String,
}

/// Result of a PA search query.
#[derive(Debug, Clone)]
pub struct PaSearchResult {
    pub source: String,
    pub title: String,
    pub body: String,
    pub written_to: Option<PathBuf>,
}
```

#### Public Functions
```rust
/// Onboard a new PA role. Prompts for JMAP credentials and vault path.
///
/// # Errors
/// Returns `PaError::InvalidVaultPath` if path is not a directory.
pub async fn onboard_pa(config_path: &Path) -> Result<PaConfig, PaError>;

/// Execute a PA search query against JMAP and optionally write to Obsidian vault.
pub async fn pa_search(
    query: &str,
    config: &PaConfig,
) -> Result<Vec<PaSearchResult>, PaError>;

/// Onboard a new SO role. Clones durable repo to configured path.
pub async fn onboard_so(config_path: &Path) -> Result<SoConfig, SoError>;

/// Generate the PA wrapper script at the given path.
pub fn generate_pa_wrapper(script_path: &Path) -> Result<(), std::io::Error>;
```

### Error Types
```rust
#[derive(Debug, thiserror::Error)]
pub enum PaError {
    #[error("invalid Obsidian vault path: {0}")]
    InvalidVaultPath(String),

    #[error("JMAP request failed: {0}")]
    JmapRequest(String),

    #[error("JMAP authentication failed")]
    JmapAuth,

    #[error("vault write failed: {0}")]
    VaultWrite(#[from] std::io::Error),

    #[error("configuration error: {0}")]
    Config(String),
}

#[derive(Debug, thiserror::Error)]
pub enum SoError {
    #[error("clone failed: {0}")]
    CloneFailed(String),

    #[error("durable path already exists and is not a git repository: {0}")]
    InvalidDurablePath(String),

    #[error("configuration error: {0}")]
    Config(String),
}
```

### Test Strategy

#### Unit Tests
| Test | Location | Purpose |
|------|----------|---------|
| `test_pa_config_serialization` | `terraphim_config/tests` | TOML round-trip |
| `test_so_config_serialization` | `terraphim_config/tests` | TOML round-trip |
| `test_generate_wrapper_script` | `pa_setup.rs` | Script contains correct paths |
| `test_pa_search_filters_empty` | `pa.rs` | Empty query rejected |

#### Integration Tests
| Test | Location | Purpose |
|------|----------|---------|
| `test_pa_onboarding_end_to_end` | `terraphim_agent/tests` | Onboarding -> config file created |
| `test_so_clone_end_to_end` | `terraphim_agent/tests` | Onboarding -> repo cloned |
| `test_jmap_search_integration` | `haystack_jmap/tests` | JMAP query returns results |
| `test_obsidian_write_integration` | `terraphim_agent/tests` | Markdown file written to vault |

#### Property Tests
None -- this cluster is primarily user-facing I/O and configuration.

### Implementation Steps

#### Step F1: PA and SO Config Types
**Files**: `terraphim_config/src/lib.rs`
**Description**: Add `PaConfig` and `SoConfig` with serde support.
**Tests**: Config serialization tests
**Estimated**: 2 hours

#### Step F2: JMAP Integration for PA
**Files**: `terraphim_agent/src/commands/pa.rs`, `haystack_jmap/src/lib.rs`
**Description**: Wire `haystack_jmap` into PA command handlers.
**Tests**: Integration test for JMAP search
**Dependencies**: Step F1
**Estimated**: 4 hours

#### Step F3: Obsidian Vault Population
**Files**: `terraphim_agent/src/commands/pa.rs`
**Description**: Write JMAP search results as markdown files to configured vault path.
**Tests**: Integration test for vault write
**Dependencies**: Step F2
**Estimated**: 3 hours

#### Step F4: SO Role Onboarding and Clone
**Files**: `terraphim_agent/src/commands/so.rs`, `so_setup.rs`
**Description**: Implement durable path clone. Add role to config.
**Tests**: Integration test for clone
**Dependencies**: Step F1
**Estimated**: 3 hours

#### Step F5: Wrapper Script Generation
**Files**: `scripts/generate-pa-wrapper.sh`, `pa_setup.rs`
**Description**: Generate `~/bin/terraphim-agent-pa` on first run or via setup command.
**Tests**: Unit test verifying script contents
**Dependencies**: Steps F1, F3
**Estimated**: 2 hours

#### Step F6: CLI Role Routing
**Files**: `terraphim_agent/src/main.rs`
**Description**: Add `--role pa` and `--role so` flags. Route to correct command module.
**Tests**: CLI argument parsing tests
**Dependencies**: Steps F2, F4
**Estimated**: 2 hours

#### Step F7: Documentation
**Files**: `docs/pa-how-to.md`
**Description**: End-user setup guide.
**Tests**: Doc tests for code snippets
**Dependencies**: Steps F5, F6
**Estimated**: 2 hours

### Rollback Plan

If PA role breaks existing terraphim-agent behaviour:
1. Default role remains unchanged; PA is opt-in via `--role pa`
2. Remove wrapper script if it conflicts with existing `~/bin`
3. Revert `main.rs` CLI routing to pre-PA state

Feature flag: `--role` flag defaults to existing behaviour; PA/SO are explicit opt-in.

### Performance Considerations

| Metric | Target | Measurement |
|--------|--------|-------------|
| PA search latency | < 3s | JMAP query timing |
| Obsidian vault write | < 100ms per file | File I/O benchmark |
| SO clone time | < 30s | Git clone timing |
| PA setup time | < 10 min | User acceptance test |

- JMAP connections are pooled via `reqwest` connection pooling.
- Obsidian writes are batched: multiple results written in a single tokio spawn.
- Vault path validation happens at config load, not per query.

---

## 5/25 Rule: Full Feature Analysis

### All 25 Features Considered

1. RUSTSEC-2026-0049 fix
2. CI runner restoration
3. TOML duplicate-key hardening
4. Disk usage guard
5. Agent timeout tuning
6. AgentRunRecord telemetry
7. Execution tiers
8. Provider probe hardening
9. Gitea mention routing
10. Webhook-driven detection
11. Approval workflow
12. Agent queue (SQLite)
13. AgentEvolution wiring
14. SQLite shared learning store
15. Trust-gated promotion
16. EIDOS scoring
17. Cross-agent injection
18. Session connectors (Aider, Cline)
19. PA JMAP integration
20. PA Obsidian vault
21. SO durable clone
22. PA wrapper script
23. TLA+ bug fixes
24. Token tracking (terraphim_usage)
25. Community launch content

### Top 5 (IN SCOPE -- The Vital Few)

1. **Security remediation (RUSTSEC + CI)** -- existential blocker
2. **ADF reliability (timeouts, disk, TOML, telemetry)** -- factory must run
3. **Inter-agent orchestration (Gitea mentions, queues, approval)** -- strategic differentiator
4. **Learning and KG (SQLite store, trust gate, injection)** -- core thesis
5. **PA/SO roles (JMAP, Obsidian, wrapper)** -- user value delivery

### Remaining 20 (AVOID AT ALL COST)

6. TLA+ bug fixes -- deferred to Phase 6
7. Token tracking -- operational visibility, not critical path
8. Community launch content -- premature until product stable
9. TinyClaw OpenClaw parity -- not on critical path
10. Full TLA+ formal verification epic -- too deep for current phase
11. SNOMED/MeSH ontology benchmarks -- research, not engineering
12. Criterion benchmarks for symbolic embeddings -- optimisation without measurement
13. Web terminal demo -- nice-to-have, not vital
14. Dynamic provider benchmark tool -- can use existing probe logic
15. Kubernetes operator for scheduling -- massive over-engineering
16. Custom message bus (MQTT/Redis) -- Gitea is sufficient
17. Real-time WebSocket coordination -- webhooks are simpler
18. Neural embedding for lesson similarity -- Aho-Corasick is sufficient
19. Full Electron desktop app -- months of work
20. Cloud-hosted Obsidian sync -- violates privacy-first principle

**These 20 are not "future work" or "nice to haves". They are dangerous distractions that threaten the essential five.**

---

## Critical Assumptions

1. **RUSTSEC-2026-0049 has a compatible patch release** -- if not, the dependency cascade may force hyper/reqwest upgrades.
2. **The 52% timeout rate is tunable** -- if it is a fundamental architectural flaw, Phase 2 timeline extends.
3. **Gitea webhooks can be received by the orchestrator** -- assumes network path from Gitea to ADF is available.
4. **SQLite is sufficient for learning store scale** -- assumes < 100K lessons in the first 6 months.
5. **JMAP server is accessible for PA role** -- assumes users have a JMAP-compatible mail server.
6. **macOS build failure (#486) is isolated to zlob** -- assumes no other macOS-specific issues exist.

---

## Risk Factors

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| RUSTSEC fix triggers dependency cascade | Medium | High | Use `--precise` bump first; map blast radius |
| Timeout rate not tunable (architectural) | Low | High | AgentRunRecord telemetry provides data for informed decision |
| Gitea webhook delivery unreliable | Medium | Medium | Implement idempotent handler with comment_id dedup |
| SQLite concurrency bottlenecks learning store | Low | Medium | WAL mode; batch scoring nightly |
| PA role JMAP auth complexity | Medium | Medium | Support Basic Auth first; OAuth2 deferred |
| Scope creep from highest-PageRank issue (#225) | High | High | Strict phase gate: no Phase 3 until Phase 2 > 90% for 48h |

---

## Approval

- [ ] Technical review complete
- [ ] Test strategy approved
- [ ] Performance targets agreed
- [ ] Cross-cluster dependencies acknowledged
- [ ] 5/25 Rule applied and documented
- [ ] Human approval received

---

*Document generated on 2026-04-17. Phase 2 of disciplined development. Proceed to Phase 2.5 (specification interview) after approval.*
