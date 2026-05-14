# Implementation Plan: ADF Configuration Validation

**Status**: Draft
**Research Doc**: `.docs/research-adf-validation.md`
**Author**: Claude (AI Agent)
**Date**: 2026-05-12
**Estimated Effort**: 8-12 hours

## Overview

### Summary
Extend `OrchestratorConfig::validate()` in `config.rs` to validate:
1. Agent `grace_period_secs` range (5s-300s)
2. Agent `max_cpu_seconds` range (60s-7200s)
3. Global NightwatchConfig `probe_ttl_secs` minimum (60s)

### Approach
Additive validation - new rules extend existing `validate()` without modifying existing checks. Each validation is self-contained in its own block with clear error messages.

### Scope

**In Scope:**
- `grace_period_secs` range validation (5s ≤ grace ≤ 300s)
- `max_cpu_seconds` range validation (60s ≤ cpu ≤ 7200s)
- NightwatchConfig `probe_ttl_secs` minimum validation (≥ 60s)
- Error message improvements (include field path + value + constraint)

**Out of Scope:**
- Runtime probe health check (belongs in separate issue 1412)
- Stewardship evaluation criteria (issue 1430 - separate design)
- Auto-tuning timeouts (ML/historical analysis - not validation)
- Provider rate-limit API calls (cannot do at load time)

**Avoid At All Cost:**
- Adding async validation (would require architecture change)
- Probe runtime health checking at load time (external API calls)
- Timeout auto-tuning logic (speculative complexity)

## Architecture

### Component Diagram

```
OrchestratorConfig
       │
       ├── validate() ──────────────────────────────────────┐
       │                                                     │
       │   ┌──────────────────────────────────────────────┐  │
       │   │ Existing Checks (unchanged)                  │  │
       │   │  - Workflow config                          │  │
       │   │  - Pre-check strategies                     │  │
       │   │  - Project ID uniqueness                   │  │
       │   │  - Agent/Flow project references           │  │
       │   │  - C1 provider (model/fallback_model)      │  │
       │   └──────────────────────────────────────────────┘  │
       │                                                     │
       │   ┌──────────────────────────────────────────────┐  │
       │   │ NEW: Extended Checks                         │  │
       │   │  - grace_period_secs (5s-300s)              │  │
       │   │  - max_cpu_seconds (60s-7200s)             │  │
       │   │  - NightwatchConfig probe_ttl_secs (≥60s)  │  │
       │   └──────────────────────────────────────────────┘  │
       │                                                     │
       └─────────────────────────────────────────────────────┘
```

### Field Mapping (Investigation Findings)

| Issue | Field | Location | Semantics |
|-------|-------|----------|-----------|
| Issue 256 (52% timeout) | `grace_period_secs` | `AgentDefinition` line 591 | Grace period before killing timed-out agent (default 5s) |
| Issue 256 (52% timeout) | `max_cpu_seconds` | `AgentDefinition` line 594 | RLIMIT_CPU resource limit via setrlimit(2) |
| Issue 1412 (probe rate-limit) | `probe_ttl_secs` | `NightwatchConfig` line 355 | Provider probe TTL in seconds (default 1800s = 30min) |

### Key Design Decisions

| Decision | Rationale | Alternatives Rejected |
|----------|-----------|----------------------|
| **Sync-only validation** | Cannot make async calls at load time; runtime health checks are separate concern | Async validation - requires arch change |
| **grace_period_secs: 5s-300s** | 5s minimum (already the default), 300s max prevents runaway grace periods | No limit - risk of agents hanging indefinitely |
| **max_cpu_seconds: 60s-7200s** | 60s minimum for CPU-intensive work, 2h max for reasonability | No limit - risk of runaway resource consumption |
| **probe_ttl_secs: ≥60s** | Respect rate limits; 60s minimum ensures probes don't burn budget | No limit - issue 1412 proves this is needed |
| **Additive only** | Must not break existing valid configs | Modify existing checks - risk of regressions |

### Eliminated Options (Essentialism)

| Option Rejected | Why Rejected | Risk of Including |
|-----------------|--------------|-------------------|
| Runtime probe health check | Requires external API calls; cannot do at load time | Complexity, external dependencies |
| Auto-tuning timeouts | ML/historical analysis; not load-time validation | Speculative complexity |
| Stewardship evaluation criteria | Runtime concern; not config validation | Scope creep |

### Simplicity Check

**What if this could be easy?**

Add 3 small validation blocks inside existing `validate()` function. Each block is < 10 lines. No new types, no new functions (except helper constants), no architecture change.

**Senior Engineer Test**: Yes, this is appropriately simple. Each validation is a direct range check.

**Nothing Speculative Checklist**:
- [x] No features the user didn't request (4 open issues cite these gaps)
- [x] No abstractions "in case we need them later"
- [x] No flexibility "just in case"
- [x] No error handling for scenarios that cannot occur
- [x] No premature optimization

## File Changes

### Modified Files

| File | Changes |
|------|---------|
| `crates/terraphim_orchestrator/src/config.rs` | Add 3 validation blocks in `validate()` |

### No New Files Required

Validation is entirely within existing `validate()` function.

## API Design

### Public Types (No Change)

Existing `OrchestratorConfig`, `AgentDefinition`, and `NightwatchConfig` structs unchanged.

### OrchestratorError Variants (Add New)

```rust
// Add to OrchestratorError enum in error.rs
#[derive(Debug, thiserror::Error)]
pub enum OrchestratorError {
    // ... existing variants ...

    #[error("agent '{agent}' {field} value {value}s is outside allowed range [{min}s, {max}s]")]
    AgentFieldOutOfRange {
        agent: String,
        field: String,
        value: u64,
        min: u64,
        max: u64,
    },

    #[error("nightwatch probe_ttl_secs {value}s is below minimum {min}s (rate-limit protection)")]
    ProbeTtlTooShort {
        value: u64,
        min: u64,
    },
}
```

### Public Functions (No New Public API)

No new public functions. Validation happens inside existing `validate()` method.

### Internal Helper Constants

```rust
// Add near top of config.rs module
const GRACE_PERIOD_MIN_SECS: u64 = 5;
const GRACE_PERIOD_MAX_SECS: u64 = 300;
const MAX_CPU_MIN_SECS: u64 = 60;
const MAX_CPU_MAX_SECS: u64 = 7200;
const PROBE_TTL_MIN_SECS: u64 = 60;
```

## Test Strategy

### Unit Tests to Add

| Test | Location | Purpose |
|------|----------|---------|
| `test_validate_grace_period_too_low` | `config.rs` (mod tests) | Reject grace_period_secs < 5s |
| `test_validate_grace_period_too_high` | `config.rs` (mod tests) | Reject grace_period_secs > 300s |
| `test_validate_grace_period_in_range` | `config.rs` (mod tests) | Accept 5s ≤ grace ≤ 300s |
| `test_validate_max_cpu_too_low` | `config.rs` (mod tests) | Reject max_cpu_seconds < 60s |
| `test_validate_max_cpu_too_high` | `config.rs` (mod tests) | Reject max_cpu_seconds > 7200s |
| `test_validate_max_cpu_in_range` | `config.rs` (mod tests) | Accept 60s ≤ cpu ≤ 7200s |
| `test_validate_probe_ttl_too_short` | `config.rs` (mod tests) | Reject probe_ttl_secs < 60s |

### Test Examples

```rust
#[test]
fn test_validate_grace_period_too_low() {
    let toml_str = r#"
working_dir = "/tmp"
[nightwatch]
[compound_review]
schedule = "0 0 * * *"
repo_path = "/tmp"
[[agents]]
name = "test-agent"
layer = "Safety"
cli_tool = "echo"
task = "test"
grace_period_secs = 2
"#;
    let config = OrchestratorConfig::from_toml(toml_str).unwrap();
    let err = config.validate().unwrap_err();
    assert!(matches!(err, OrchestratorError::AgentFieldOutOfRange { field, .. } if field == "grace_period_secs"));
}
```

## Implementation Steps

### Step 1: Add Error Variants

**Files:** `crates/terraphim_orchestrator/src/error.rs`
**Description:** Add `AgentFieldOutOfRange` and `ProbeTtlTooShort` variants to `OrchestratorError`
**Tests:** Compile check
**Estimated:** 1 hour

```rust
// error.rs - Add variants
AgentFieldOutOfRange { agent: String, field: String, value: u64, min: u64, max: u64 },
ProbeTtlTooShort { value: u64, min: u64 },
```

### Step 2: Add Helper Constants

**Files:** `crates/terraphim_orchestrator/src/config.rs`
**Description:** Add range constants for grace_period, max_cpu, probe_ttl
**Tests:** None needed (constants)
**Estimated:** 0.5 hours

### Step 3: Add grace_period_secs Validation Block

**Files:** `crates/terraphim_orchestrator/src/config.rs` (in `validate()`)
**Description:** Add validation that grace_period_secs is in range [5s, 300s]
**Tests:** `test_validate_grace_period_*` tests
**Estimated:** 1 hour

```rust
// In validate(), after C1 provider check (~line 1377):
// D2: grace_period_secs range validation (5s - 300s)
for agent in &self.agents {
    if let Some(grace) = agent.grace_period_secs {
        if grace < GRACE_PERIOD_MIN_SECS || grace > GRACE_PERIOD_MAX_SECS {
            return Err(OrchestratorError::AgentFieldOutOfRange {
                agent: agent.name.clone(),
                field: "grace_period_secs".into(),
                value: grace,
                min: GRACE_PERIOD_MIN_SECS,
                max: GRACE_PERIOD_MAX_SECS,
            });
        }
    }
}
```

### Step 4: Add max_cpu_seconds Validation Block

**Files:** `crates/terraphim_orchestrator/src/config.rs` (in `validate()`)
**Description:** Add validation that max_cpu_seconds is in range [60s, 7200s]
**Tests:** `test_validate_max_cpu_*` tests
**Estimated:** 1 hour

```rust
// D3: max_cpu_seconds range validation (60s - 7200s)
for agent in &self.agents {
    if let Some(cpu) = agent.max_cpu_seconds {
        if cpu < MAX_CPU_MIN_SECS || cpu > MAX_CPU_MAX_SECS {
            return Err(OrchestratorError::AgentFieldOutOfRange {
                agent: agent.name.clone(),
                field: "max_cpu_seconds".into(),
                value: cpu,
                min: MAX_CPU_MIN_SECS,
                max: MAX_CPU_MAX_SECS,
            });
        }
    }
}
```

### Step 5: Add NightwatchConfig probe_ttl_secs Validation

**Files:** `crates/terraphim_orchestrator/src/config.rs` (in `validate()`)
**Description:** Add validation that probe_ttl_secs >= 60s
**Tests:** `test_validate_probe_ttl_too_short`
**Estimated:** 1 hour

```rust
// D4: NightwatchConfig probe_ttl_secs minimum validation (60s)
if self.nightwatch.probe_ttl_secs < PROBE_TTL_MIN_SECS {
    return Err(OrchestratorError::ProbeTtlTooShort {
        value: self.nightwatch.probe_ttl_secs,
        min: PROBE_TTL_MIN_SECS,
    });
}
```

### Step 6: Add Tests

**Files:** `crates/terraphim_orchestrator/src/config.rs` (mod tests)
**Description:** Add all validation tests listed in Test Strategy
**Tests:** Run all new tests
**Estimated:** 3 hours

### Step 7: Run Full Test Suite

**Files:** All modified files
**Description:** `cargo test -p terraphim_orchestrator`
**Tests:** Verify no regressions
**Estimated:** 1 hour

## Rollback Plan

If issues discovered:
1. Revert changes to `config.rs` (remove 3 validation blocks)
2. Revert `error.rs` (remove new variants)
3. All validation reverts to previous state

**Feature flag**: N/A (validation is load-time, cannot be toggled)

## Dependencies

### No New Dependencies

All validation uses existing types and patterns.

## Performance Considerations

### Expected Performance
| Metric | Target | Measurement |
|--------|--------|-------------|
| Validation time | < 10ms | Benchmark with 100 agents |
| Memory | No change | N/A |

### Benchmarks to Add
None required - validation is simple iteration and comparison.

## Open Items

| Item | Status | Owner |
|------|--------|-------|
| None | All resolved via investigation | N/A |

## Approval

- [ ] Technical review complete
- [ ] Test strategy approved
- [ ] Essentialism gates passed
- [ ] Human approval received

---

## Specification Interview Findings

**Interview Date**: 2026-05-12
**Dimensions Covered**: Edge Cases, Failure Modes, User Mental Models, Scale & Performance, Integration Effects
**Convergence Status**: Complete

### Key Decisions from Interview

#### Edge Cases
- **Decision**: Validate both `grace_period_secs` AND `max_cpu_seconds` as separate fields with different ranges.
  - **Rationale**: Investigation showed `grace_period_secs` (line 591, default 5s) controls grace period before killing timed-out agent; `max_cpu_seconds` (line 594) is RLIMIT_CPU resource limit. Different semantics require different bounds.

- **Decision**: `grace_period_secs` range = 5s minimum, 300s maximum.
  - **Rationale**: 5s is already the implicit minimum (noted in code). 300s (5 min) prevents runaway grace periods that would delay recovery from hung agents.

- **Decision**: `max_cpu_seconds` range = 60s minimum, 7200s maximum.
  - **Rationale**: 60s minimum for any real CPU work. 7200s (2 hours) is reasonable upper bound for long-running jobs.

- **Decision**: `probe_ttl_secs` minimum = 60s.
  - **Rationale**: Issue 1412 explicitly states "provider probes are not rate-limit aware -- burn API budget on every probe cycle". 60s minimum enforces rate-limit respect.

#### Failure Modes
- **Decision**: Use generic `AgentFieldOutOfRange` error for both grace_period and max_cpu.
  - **Rationale**: Both are range validation errors; sharing error type reduces enum variants while remaining actionable.

#### Integration Effects
- **Decision**: Probe validation is global NightwatchConfig only, not per-agent.
  - **Rationale**: Investigation showed `probe_ttl_secs` is on `NightwatchConfig` (line 355), not `AgentDefinition`. Per-user answer confirmed this is correct scope.

### Deferred Items
- **Item**: Wall-clock timeout mechanism for agents
  - **Deferred because**: Investigation revealed there is NO separate wall-clock timeout field in `AgentDefinition`. The "timeout" in issue 256 refers to agents timing out from some external mechanism, then `grace_period_secs` controls the kill grace period. This is a schema gap, not a validation gap.

- **Item**: Auto-tuning of grace_period based on historical timeout data
  - **Deferred because**: User preference; ML/historical analysis is out of scope for load-time validation.

### Interview Summary

The specification interview revealed that the original design had incorrect field assumptions - it referenced a non-existent `timeout` field. Investigation of `AgentDefinition` (line 546) and `lib.rs` (line 6110) confirmed:

1. **`grace_period_secs`** (default 5s) is the grace period before killing a timed-out agent
2. **`max_cpu_seconds`** is RLIMIT_CPU via setrlimit(2) - a resource limit, not a wall-clock timeout
3. **No separate wall-clock timeout field exists** - this may be the actual bug behind issue 256

The validation design was corrected to validate these two fields with appropriate ranges, plus the global `probe_ttl_secs` in NightwatchConfig.

---

## Appendix: Validation Pseudocode

```rust
// config.rs:validate() - Full implementation after changes

pub fn validate(&self) -> Result<(), OrchestratorError> {
    // === EXISTING CHECKS (unchanged) ===

    // 1. Workflow validation
    // ... existing code ...

    // 2. Pre-check strategy validation
    // ... existing code ...

    // 3. Project ID uniqueness
    // ... existing code ...

    // 4. Agent/Flow project references
    // ... existing code ...

    // 5. C1 provider (model/fallback_model)
    for agent in &self.agents {
        if let Some(model) = &agent.model {
            validate_model_provider(&agent.name, "model", model)?;
        }
        if let Some(model) = &agent.fallback_model {
            validate_model_provider(&agent.name, "fallback_model", model)?;
        }
    }

    // === NEW CHECKS ===

    // D2: grace_period_secs range validation (5s - 300s)
    for agent in &self.agents {
        if let Some(grace) = agent.grace_period_secs {
            if grace < GRACE_PERIOD_MIN_SECS || grace > GRACE_PERIOD_MAX_SECS {
                return Err(OrchestratorError::AgentFieldOutOfRange {
                    agent: agent.name.clone(),
                    field: "grace_period_secs".into(),
                    value: grace,
                    min: GRACE_PERIOD_MIN_SECS,
                    max: GRACE_PERIOD_MAX_SECS,
                });
            }
        }
    }

    // D3: max_cpu_seconds range validation (60s - 7200s)
    for agent in &self.agents {
        if let Some(cpu) = agent.max_cpu_seconds {
            if cpu < MAX_CPU_MIN_SECS || cpu > MAX_CPU_MAX_SECS {
                return Err(OrchestratorError::AgentFieldOutOfRange {
                    agent: agent.name.clone(),
                    field: "max_cpu_seconds".into(),
                    value: cpu,
                    min: MAX_CPU_MIN_SECS,
                    max: MAX_CPU_MAX_SECS,
                });
            }
        }
    }

    // D4: NightwatchConfig probe_ttl_secs minimum validation (60s)
    if self.nightwatch.probe_ttl_secs < PROBE_TTL_MIN_SECS {
        return Err(OrchestratorError::ProbeTtlTooShort {
            value: self.nightwatch.probe_ttl_secs,
            min: PROBE_TTL_MIN_SECS,
        });
    }

    Ok(())
}
```
