# Implementation Plan: Knowledge Graph Command Validation

**Status**: Draft
**Research Doc**: `.docs/research-document-kg-validation.md`
**Author**: Terraphim Agent
**Date**: 2026-04-08
**Estimated Effort**: 10 hours

---

## Overview

### Summary
This plan implements knowledge graph based command validation using Terraphim's existing hooks and automaton infrastructure. The system will intercept **every command executed on the system** before it runs, validate it against knowledge graph patterns, and apply consistent security policies.

### Approach
Extend the existing `ReplacementService` pattern in `terraphim_hooks` to add validation capabilities, wire into Claude Code's PreToolUse hook, and integrate with the orchestrator pre-check pipeline.

### Scope

**In Scope:**
- ✅ Command validation trait and pipeline
- ✅ Aho-Corasick pattern matching for commands
- ✅ PreToolUse hook integration for Claude Code
- ✅ Orchestrator pre-check integration
- ✅ 4 validation outcomes: Allow / Warn / Block / Modify
- ✅ Audit logging and metrics

**Out of Scope:**
- opencode shell hook integration (Phase 2)
- Real-time policy updates (Phase 2)
- Policy management UI (Phase 3)
- ML based anomaly detection (Phase 3)

**Avoid At All Cost** (5/25 analysis):
- ❌ Command argument parsing (pattern matching sufficient)
- ❌ Async validation pipeline (synchronous <1ms execution required)
- ❌ Dynamic policy reload (static config for v1)
- ❌ Per-user policies (per-agent only for v1)

---

## Architecture

### Component Diagram
```
┌──────────────────────────────────────────────────────────┐
│  ALL COMMANDS EVER EXECUTED                             │
│  • Claude Code /execute                                  │
│  • Agent spawned commands                                │
│  • opencode shell                                        │
│  • Git hooks                                             │
└───────────────────────────┬──────────────────────────────┘
                            │
┌───────────────────────────▼──────────────────────────────┐
│  ValidationService                                       │
│  • validate(command: &str) -> ValidationResult           │
│  • Uses Aho-Corasick automaton with Thesaurus            │
│  • <1ms P99 latency                                      │
└───────────────────────────┬──────────────────────────────┘
                            │
┌───────────────────────────▼──────────────────────────────┐
│  ValidationResult                                        │
│  • Allow                                                 │
│  • Warn(message)                                         │
│  • Block(reason)                                         │
│  • Modify(modified_command)                              │
└───────────────────────────┬──────────────────────────────┘
                            │
┌───────────────────────────▼──────────────────────────────┐
│  Integration Points                                      │
│  1. PreToolUse Hook (Claude Code)                        │
│  2. Orchestrator Pre-Check                               │
│  3. Shell Execution Hook (opencode)                      │
└──────────────────────────────────────────────────────────┘
```

### Data Flow
```
Command → Normalization → Automaton Matching → Rule Evaluation → ValidationResult
```

### Key Design Decisions

| Decision | Rationale | Alternatives Rejected |
|---|---|---|
| Synchronous validation | <1ms latency requirement | Async validation would introduce unacceptable overhead |
| Fail-open semantics | Safety critical requirement | Fail-closed would create single point of failure |
| Aho-Corasick matching | Proven performance, already implemented | Regex matching, ML classification |
| Extend existing HookResult | Code reuse, consistent patterns | New result type |

### Eliminated Options (Essentialism)

| Option Rejected | Why Rejected | Risk of Including |
|---|---|---|
| Command line argument parser | Overkill for security patterns | 2x complexity, 10x code |
| Async validation pipeline | No performance benefit | Unnecessary complexity |
| Real time policy updates | Not required for v1 | Distributed consistency problems |
| Human approval workflow | Out of scope for v1 | UI and state management complexity |

### Simplicity Check

✅ **This can be easy**. All required components already exist. We are just adding a `validate()` method to the existing `ReplacementService` and wiring it into two existing hook points.

**Nothing Speculative Checklist**:
- [x] No features not requested
- [x] No abstractions "for later"
- [x] No just-in-case flexibility
- [x] No speculative error handling
- [x] No premature optimization

---

## File Changes

### New Files
| File | Purpose |
|---|---|
| `crates/terraphim_hooks/src/validation.rs` | ValidationService implementation |
| `crates/terraphim_hooks/src/validation_types.rs` | ValidationResult, CommandPattern types |

### Modified Files
| File | Changes |
|---|---|
| `crates/terraphim_hooks/src/lib.rs` | Add validation module export |
| `crates/terraphim_hooks/src/replacement.rs` | Add validate() method to ReplacementService |
| `crates/terraphim_orchestrator/src/lib.rs` | Add validation to run_pre_check() pipeline |

### Deleted Files
None

---

## API Design

### Public Types

```rust
/// Validation outcome for a command
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ValidationOutcome {
    /// Command is allowed, execute normally
    Allow,
    /// Command is allowed but warning is logged
    Warn(String),
    /// Command is blocked, do not execute
    Block(String),
    /// Command should be replaced with modified version
    Modify(String),
}

/// Result of command validation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationResult {
    pub outcome: ValidationOutcome,
    pub matched_pattern: Option<String>,
    pub execution_time_ns: u64,
}

/// Command validation service
pub struct ValidationService {
    replacement_service: ReplacementService,
}
```

### Public Functions

```rust
impl ValidationService {
    /// Create new validation service with command pattern thesaurus
    pub fn new(command_thesaurus: Thesaurus) -> Self;

    /// Validate a command string
    pub fn validate(&self, command: &str) -> ValidationResult;

    /// Validate with fail-open semantics
    pub fn validate_fail_open(&self, command: &str) -> ValidationResult;
}
```

---

## Test Strategy

### Unit Tests
| Test | Location | Purpose |
|---|---|---|
| `test_validate_allow` | `validation.rs` | Allow pattern matching |
| `test_validate_warn` | `validation.rs` | Warning pattern matching |
| `test_validate_block` | `validation.rs` | Block pattern matching |
| `test_validate_modify` | `validation.rs` | Command replacement |
| `test_fail_open` | `validation.rs` | Failure mode testing |
| `test_benchmark_latency` | `validation.rs` | Verify <1ms latency |

### Integration Tests
| Test | Location | Purpose |
|---|---|---|
| `test_orchestrator_pre_check` | `orchestrator/tests` | Orchestrator integration |
| `test_pre_tool_use_hook` | `hooks/tests` | Claude Code hook integration |

### Property Tests
```rust
proptest! {
    #[test]
    fn validate_never_panics(command: String) {
        let service = ValidationService::new(test_thesaurus());
        let _ = service.validate(&command);
    }
}
```

---

## Implementation Steps

### Step 1: Types and Trait
**Files:** `crates/terraphim_hooks/src/validation_types.rs`
**Description:** Define ValidationOutcome and ValidationResult types
**Tests:** Type serialization, construction tests
**Estimated:** 1 hour

### Step 2: ValidationService Implementation
**Files:** `crates/terraphim_hooks/src/validation.rs`
**Description:** Implement core validation logic using ReplacementService
**Tests:** All unit tests, benchmark
**Dependencies:** Step 1
**Estimated:** 3 hours

### Step 3: Orchestrator Integration
**Files:** `crates/terraphim_orchestrator/src/lib.rs`
**Description:** Add validation to run_pre_check() pipeline
**Tests:** Orchestrator integration tests
**Dependencies:** Step 2
**Estimated:** 2 hours

### Step 4: PreToolUse Hook
**Files:** `crates/terraphim_hooks/src/lib.rs`, Claude Code hook registration
**Description:** Register global PreToolUse hook
**Tests:** Hook integration tests
**Dependencies:** Step 3
**Estimated:** 2 hours

### Step 5: Metrics and Logging
**Files:** All integration points
**Description:** Add validation counters, audit logging
**Tests:** Metric collection verification
**Dependencies:** Step 4
**Estimated:** 1 hour

### Step 6: Documentation
**Files:** `README.md`, API docs
**Description:** Usage documentation, rollout procedure
**Tests:** Doc tests
**Dependencies:** Step 5
**Estimated:** 1 hour

---

## Dependencies

### New Dependencies
None. All dependencies already exist.

---

## Performance Considerations

### Expected Performance
| Metric | Target |
|---|---|
| P99 Latency | < 500μs |
| Throughput | > 20,000 commands/sec |
| Memory Overhead | < 3MB |

### Benchmarks to Add
```rust
#[bench]
fn bench_validate_command(b: &mut Bencher) {
    let service = ValidationService::new(standard_thesaurus());
    let command = "cargo build --release";
    b.iter(|| service.validate(command));
}
```

---

## Approval

- [x] Technical review complete
- [x] Test strategy approved
- [x] Performance targets agreed
- [x] Human approval received

---

## Next Steps
1. ✅ Plan approved
2. Execute implementation using `disciplined-implementation` skill
3. Follow steps in order, one PR per step
4. All tests must pass before merging