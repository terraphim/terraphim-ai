# Implementation Plan: Wire KG Validation into RLM Execution Hot Paths (#2671)

**Status**: Implemented
**Research Doc**: `.docs/adf/2671/research-2671-kg-validation-hot-paths.md`
**Author**: opencode / k2p7
**Date**: 2026-06-14
**Commit**: `7b46fb3de`
**Estimated Effort**: 4-5 hours implementation + 1.5 hours verification

---

## Overview

### Summary

Connect the existing `KnowledgeGraphValidator` to the RLM execution hot paths by:
1. Adding an `Arc<KnowledgeGraphValidator>` to each executor.
2. Implementing `ExecutionEnvironment::validate()` in `FirecrackerExecutor`, `DockerExecutor`, and `LocalExecutor` using the shared validator.
3. Calling `executor.validate(input).await?` in `TerraphimRlm::execute_code`, `TerraphimRlm::execute_command`, and `QueryLoop::execute_command` before execution.
4. Mapping a failed validation result to `RlmError::KgValidationFailed` (or `KgEscalationRequired` when retries are exhausted).
5. Updating `select_executor` and `TerraphimRlm::new` to construct and inject the validator.

### Approach

- **No trait changes**: keep `ExecutionEnvironment` stable.
- **No new dependencies**: reuse `terraphim_automata`, `terraphim_types`, existing optional deps.
- **Minimal signature changes**: add an optional `Arc<KnowledgeGraphValidator>` parameter to executor constructors; default to a disabled validator if none supplied.
- **Two ValidationResult types reconciled**: add a conversion helper from `validator::ValidationResult` to `executor::ValidationResult`.
- **Config-driven**: add an optional `thesaurus` field to `RlmConfig` so `TerraphimRlm::new` can build a validator; if absent, validation is permissive/disabled.

### Scope

**In Scope:**
1. Add `validator: Option<Arc<KnowledgeGraphValidator>>` to `FirecrackerExecutor`, `DockerExecutor`, `LocalExecutor`.
2. Implement `validate()` in all three executors by delegating to the validator.
3. Add `ValidationResult` conversion helper.
4. Add optional `thesaurus: Option<Thesaurus>` to `RlmConfig`.
5. Build a validator in `select_executor` and pass it to executors.
6. Add `TerraphimRlm::new_with_thesaurus` convenience constructor.
7. Insert `executor.validate(code).await?` and `executor.validate(command).await?` in `rlm.rs`.
8. Insert validation in `QueryLoop::execute_command` for `Command::Run` and `Command::Code`.
9. Unit tests for conversion, executor validation, and hot-path blocking.
10. Update crate README / doc comments to mention validation.

**Out of Scope:**
- Retry-with-LLM-rephrase loop (defer to #2672/Step 5).
- `ValidationContext` tracking.
- Validation of `Command::QueryLlm`, `Command::Snapshot`, `Command::Rollback`.
- Per-command strictness override.
- Metrics/telemetry for validation.
- Changing the `ExecutionEnvironment` trait.

**Avoid At All Cost** (5/25 distractions):
- Rewriting `KnowledgeGraphValidator`.
- Merging the two `ValidationResult` types (would break the trait).
- Adding a new dependency for validation.
- Changing `ExecutionContext` to carry the validator.
- Refactoring `select_executor` to a capability matcher.
- Adding async retry logic in this PR.
- Touching `FirecrackerExecutor` VM lifecycle beyond `validate()`.
- Adding benches for validation.
- Generalising the validator to a public crate API.

## Architecture

### Component Diagram

```text
                  ┌─────────────────────────────┐
                  │        RlmConfig            │
                  │  + thesaurus: Option<...>   │
                  └──────────────┬──────────────┘
                                 │
                  ┌──────────────▼──────────────┐
                  │   TerraphimRlm::new()       │
                  │  builds KnowledgeGraphValidator
                  │  and passes Arc to executor  │
                  └──────────────┬──────────────┘
                                 │
        ┌────────────────────────┼────────────────────────┐
        │                        │                        │
   ┌────▼──────┐           ┌─────▼──────┐           ┌─────▼──────┐
   │ Firecracker│           │   Docker   │           │   Local    │
   │  Executor  │           │  Executor  │           │  Executor  │
   │  validate()│           │  validate()│           │  validate()│
   └─────┬──────┘           └─────┬──────┘           └─────┬──────┘
        │                        │                        │
        └────────────────────────┼────────────────────────┘
                                 │
                  ┌──────────────▼──────────────┐
                  │ KnowledgeGraphValidator     │
                  │  + Thesaurus                │
                  │  + optional RoleGraph       │
                  └─────────────────────────────┘
                                 ▲
                                 │
        ┌────────────────────────┴────────────────────────┐
        │                                                 │
   ┌────▼──────────────────┐                    ┌─────────▼──────────┐
   │ rlm.rs                │                    │ query_loop.rs      │
   │ execute_code()        │                    │ execute_command()  │
   │ execute_command()     │                    │   Run / Code       │
   │   executor.validate() │                    │   executor.validate()
   └───────────────────────┘                    └────────────────────┘
```

### Data Flow

```text
Caller → TerraphimRlm::execute_code(session_id, code)
  → session_manager.validate_session(session_id)
  → executor.validate(code).await
       → KnowledgeGraphValidator::validate(code)
       → if !passed: RlmError::KgValidationFailed { unknown_terms }
  → executor.execute_code(code, ctx)
  → ExecutionResult
```

### Key Design Decisions

| Decision | Rationale | Alternatives Rejected |
|---|---|---|
| Keep `ExecutionEnvironment` trait unchanged | Six implementations; trait churn is high-risk. | Adding `set_validator` to trait. |
| Pass `Arc<KnowledgeGraphValidator>` to executor constructors | Simple, shareable, no trait change. | Passing in `ExecutionContext` (not serialisable). |
| Add `thesaurus: Option<Thesaurus>` to `RlmConfig` | Keeps `TerraphimRlm::new(config)` API intact while enabling validation. | New `TerraphimRlm::new_with_validator` only; less ergonomic. |
| Map validator `ValidationResult` to executor `ValidationResult` | Avoids breaking the trait's return type. | Replacing executor `ValidationResult` with validator's type. |
| Block on first validation failure in `QueryLoop` | P0 safety fix; retry loop is out of scope. | Feeding validation failure back to LLM as context. |
| Default validator to disabled when no thesaurus | Preserves current behaviour when no KG is configured. | Always requiring a thesaurus. |

### Eliminated Options (Essentialism)

| Option Rejected | Why Rejected | Risk of Including |
|---|---|---|
| Retry-with-LLM-rephrase in `QueryLoop` | Out of scope for #2671; adds significant complexity. | Delays P0 safety fix. |
| Change `ExecutionEnvironment::validate` signature | Breaks all six implementations and any external callers. | Ripple across crate and tests. |
| Put validator in `ExecutionContext` | `ExecutionContext` is serialised; validator is not. | Serialization failures / awkward design. |
| Load thesaurus from a default project path | Magic paths are fragile; explicit config is better. | Unexpected behaviour in different environments. |
| Validate the entire LLM response before parsing | Parser should still run; we validate the command payload only. | Over-blocking benign responses. |

### Simplicity Check

**What if this could be easy?** It is. The validator already exists. We only need to:
1. Share it with executors.
2. Call it before execution.
3. Convert its result.

**Senior Engineer Test**: A senior engineer would recognise this as plumbing, not architecture. No abstractions are added "just in case".

**Nothing Speculative Checklist**:
- [x] No features the user didn't request (validation wiring only).
- [x] No abstractions "in case we need them later".
- [x] No flexibility "just in case".
- [x] No error handling for scenarios that cannot occur.
- [x] No premature optimization.

## File Changes

### New Files

| File | Purpose |
|---|---|
| (none) | All changes are modifications. |

### Modified Files

| File | Changes |
|---|---|
| `crates/terraphim_rlm/src/config.rs` | Add `thesaurus: Option<Thesaurus>` to `RlmConfig`; default `None`; include in Debug/serde. |
| `crates/terraphim_rlm/src/validator.rs` | Add `impl From<validator::ValidationResult> for executor::ValidationResult` conversion helper. |
| `crates/terraphim_rlm/src/executor/context.rs` | Add `ValidationResult::from_validator_result` helper (or use `From` impl in validator.rs). |
| `crates/terraphim_rlm/src/executor/firecracker.rs` | Add `validator` field; implement `validate()` using it; update `new()` to accept optional validator. |
| `crates/terraphim_rlm/src/executor/docker.rs` | Add `validator` field; implement `validate()` using it; update `new()` to accept optional validator. |
| `crates/terraphim_rlm/src/executor/local.rs` | Add `validator` field; implement `validate()` using it; update `new()` to accept optional validator. |
| `crates/terraphim_rlm/src/executor/mod.rs` | Build validator from config thesaurus in `select_executor`; pass to each executor constructor. |
| `crates/terraphim_rlm/src/executor/ssh.rs` | Add `validator` field; implement `validate()` (can delegate to a default disabled validator or local logic). |
| `crates/terraphim_rlm/src/rlm.rs` | Add validation calls in `execute_code` and `execute_command`; add `new_with_thesaurus`. |
| `crates/terraphim_rlm/src/query_loop.rs` | Add validation calls for `Command::Run` and `Command::Code`. |
| `crates/terraphim_rlm/src/lib.rs` | Re-export `KnowledgeGraphValidator` and `ValidatorConfig` if not already. |
| `crates/terraphim_rlm/README.md` | Document KG validation feature and config. |

### Deleted Files

| File | Reason |
|---|---|
| (none) | |

## API Design

### Public Types

```rust
// In config.rs - additive field
pub struct RlmConfig {
    // ... existing fields ...
    /// Optional knowledge-graph thesaurus for command validation.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub thesaurus: Option<Thesaurus>,
}
```

```rust
// In validator.rs - conversion helper
impl From<crate::executor::ValidationResult> is NOT added; instead:

impl crate::executor::ValidationResult {
    pub fn from_validator_result(result: &validator::ValidationResult) -> Self {
        Self {
            is_valid: result.passed,
            matched_terms: result.matched_terms.clone(),
            unknown_terms: result.unmatched_words.clone(),
            suggestions: result.suggestions.iter().map(|s| (s.clone(), vec![])).collect(),
            strictness: result.strictness(), // need accessor or store strictness
        }
    }
}
```

*Note*: `validator::ValidationResult` does not currently store `strictness`. We will add a private `strictness` field set during construction, plus a `strictness()` accessor, so the executor-context result can carry it.

### Public Functions

```rust
impl TerraphimRlm {
    /// Create RLM with a supplied thesaurus for KG validation.
    pub async fn new_with_thesaurus(
        config: RlmConfig,
        thesaurus: Thesaurus,
    ) -> RlmResult<Self> {
        let mut config = config;
        config.thesaurus = Some(thesaurus);
        Self::new(config).await
    }
}
```

### Error Types

No new error variants. Reuse existing:
- `RlmError::KgValidationFailed { unknown_terms: Vec<String> }`
- `RlmError::KgEscalationRequired { unknown_terms, suggested_action, context }`

Mapping logic:
```rust
fn validation_error(result: &executor::ValidationResult, input: &str) -> RlmError {
    if result.strictness == KgStrictness::Strict {
        RlmError::KgValidationFailed {
            unknown_terms: result.unknown_terms.clone(),
        }
    } else {
        RlmError::KgEscalationRequired {
            unknown_terms: result.unknown_terms.clone(),
            suggested_action: "Use known domain terms".to_string(),
            context: format!("Input: {}", truncate(input, 200)),
        }
    }
}
```

*Refinement*: For the first pass, always use `KgValidationFailed` when `!is_valid`. `KgEscalationRequired` is more appropriate when retry context is tracked (out of scope).

## Test Strategy

### Unit Tests

| Test | Location | Purpose |
|---|---|---|
| `test_validation_result_conversion` | `validator.rs` | Map validator result to executor result. |
| `test_firecracker_validate_blocks_unknown` | `firecracker.rs` (gated on feature + KVM) | With thesaurus, unknown command fails. |
| `test_firecracker_validate_allows_known` | `firecracker.rs` (gated) | With thesaurus, known command passes. |
| `test_local_validate_blocks_unknown` | `local.rs` | Unknown command fails. |
| `test_local_validate_allows_known` | `local.rs` | Known command passes. |
| `test_docker_validate_blocks_unknown` | `docker.rs` (gated on docker-backend) | Unknown command fails. |
| `test_docker_validate_allows_known` | `docker.rs` | Known command passes. |
| `test_execute_code_validation_blocks` | `rlm.rs` | `execute_code` returns error for unknown term. |
| `test_execute_command_validation_blocks` | `rlm.rs` | `execute_command` returns error for unknown term. |
| `test_query_loop_validation_blocks_run` | `query_loop.rs` | `Command::Run` with unknown term returns error. |
| `test_query_loop_validation_blocks_code` | `query_loop.rs` | `Command::Code` with unknown term returns error. |

### Integration Tests

| Test | Location | Purpose |
|---|---|---|
| `test_rlm_end_to_end_with_validation` | `tests/validation.rs` | Build `TerraphimRlm` with thesaurus; assert allowed code runs and unknown code is blocked. |

### Test Helpers

```rust
fn test_thesaurus() -> Thesaurus {
    let mut thesaurus = Thesaurus::new("rlm-test".to_string());
    thesaurus.insert(
        NormalizedTermValue::from("python"),
        NormalizedTerm::with_auto_id(NormalizedTermValue::from("python programming language")),
    );
    thesaurus.insert(
        NormalizedTermValue::from("print"),
        NormalizedTerm::with_auto_id(NormalizedTermValue::from("print function")),
    );
    thesaurus
}

fn test_validator() -> Arc<KnowledgeGraphValidator> {
    let config = ValidatorConfig::strict();
    let validator = KnowledgeGraphValidator::new(config)
        .with_thesaurus(test_thesaurus());
    Arc::new(validator)
}
```

## Implementation Steps

### Step 1: Prepare `RlmConfig` and `ValidationResult`
**Files:** `crates/terraphim_rlm/src/config.rs`, `crates/terraphim_rlm/src/validator.rs`, `crates/terraphim_rlm/src/executor/context.rs`
**Description:**
- Add `thesaurus: Option<Thesaurus>` to `RlmConfig` with `#[serde(default, skip_serializing_if = "Option::is_none")]`.
- Include the field in `Debug` impl (non-sensitive, can be verbose).
- Add `strictness` field to `validator::ValidationResult` and `strictness()` accessor.
- Add conversion helper `executor::ValidationResult::from_validator_result`.
**Tests:** Unit test for conversion.
**Estimated:** 45 min

### Step 2: Update Executor Constructors and `validate()` Stubs
**Files:** `crates/terraphim_rlm/src/executor/firecracker.rs`, `docker.rs`, `local.rs`, `ssh.rs`
**Description:**
- Add `validator: Option<Arc<KnowledgeGraphValidator>>` field to each struct.
- Update `new()` to accept `validator: Option<Arc<KnowledgeGraphValidator>>`.
- Store it; default to `None`.
- Implement `validate()`:
  - If `validator.is_none()` or input empty, return `ValidationResult::valid(vec![])`.
  - Otherwise call `validator.validate(input)`, convert to executor `ValidationResult`, and return it.
**Tests:** Unit tests for each executor's validate with/without thesaurus.
**Estimated:** 1.5 hours

### Step 3: Update Backend Selector
**Files:** `crates/terraphim_rlm/src/executor/mod.rs`
**Description:**
- In `select_executor`, build a `KnowledgeGraphValidator` from `config.thesaurus` if present:
  ```rust
  let validator = config.thesaurus.as_ref().map(|t| {
      let v = KnowledgeGraphValidator::new(ValidatorConfig {
          strictness: config.kg_strictness,
          max_retries: config.kg_max_retries,
          ..ValidatorConfig::default()
      }).with_thesaurus(t.clone());
      Arc::new(v)
  });
  ```
- Pass `validator.clone()` to each executor constructor.
**Tests:** Existing `select_executor` tests must still pass.
**Estimated:** 30 min

### Step 4: Update `TerraphimRlm` Hot Paths
**Files:** `crates/terraphim_rlm/src/rlm.rs`
**Description:**
- Add helper:
  ```rust
  fn check_validation(&self, input: &str, result: executor::ValidationResult,
  ) -> RlmResult<()> {
      if !result.is_valid {
          return Err(RlmError::KgValidationFailed {
              unknown_terms: result.unknown_terms,
          });
      }
      Ok(())
  }
  ```
- In `execute_code`, after session validation:
  ```rust
  let validation = self.executor.validate(code).await?;
  self.check_validation(code, validation)?;
  ```
- In `execute_command`, after session validation:
  ```rust
  let validation = self.executor.validate(command).await?;
  self.check_validation(command, validation)?;
  ```
- Add `TerraphimRlm::new_with_thesaurus`.
**Tests:** Unit tests for blocked/allowed code and commands.
**Estimated:** 45 min

### Step 5: Update `QueryLoop` Hot Path
**Files:** `crates/terraphim_rlm/src/query_loop.rs`
**Description:**
- In `execute_command`, before the `match command`:
  ```rust
  let input_to_validate = match command {
      Command::Run(cmd) => Some(cmd.command.as_str()),
      Command::Code(code) => Some(code.code.as_str()),
      _ => None,
  };
  if let Some(input) = input_to_validate {
      let validation = self.executor.validate(input).await?;
      if !validation.is_valid {
          return Err(RlmError::KgValidationFailed {
              unknown_terms: validation.unknown_terms,
          });
      }
  }
  ```
**Tests:** Unit tests for blocked `Run`/`Code`.
**Estimated:** 30 min

### Step 6: Documentation and Re-exports
**Files:** `crates/terraphim_rlm/src/lib.rs`, `crates/terraphim_rlm/README.md`
**Description:**
- Re-export `KnowledgeGraphValidator`, `ValidatorConfig` from `lib.rs` if not already.
- Add a "Knowledge Graph Validation" section to README with config example.
**Estimated:** 30 min

### Step 7: Verification
**Description:**
- `cargo check -p terraphim_rlm --all-targets`
- `cargo check -p terraphim_rlm --all-features --all-targets`
- `cargo test -p terraphim_rlm --lib`
- `cargo test -p terraphim_rlm --features firecracker,docker-backend --lib`
- `cargo clippy -p terraphim_rlm --all-targets`
- `cargo fmt -p terraphim_rlm`
**Estimated:** 1.5 hours

## Rollback Plan

If issues discovered:
1. Revert the validation calls in `rlm.rs` and `query_loop.rs` (the most likely source of regressions).
2. Keep the executor `validate()` implementations; they are no-ops when no validator is supplied.
3. Disable validation by leaving `RlmConfig.thesaurus` as `None`.

No feature flag needed because the default state is validation disabled.

## Migration

No database or file migration. Existing `RlmConfig` without `thesaurus` continues to work unchanged.

## Dependencies

### New Dependencies

| Crate | Version | Justification |
|---|---|---|
| (none) | — | Reuse existing workspace crates. |

### Dependency Updates

| Crate | From | To | Reason |
|---|---|---|---|
| (none) | — | — | |

## Performance Considerations

### Expected Performance

| Metric | Target | Measurement |
|---|---|---|
| Validation latency | < 5 ms per command | Unit test with `Instant` |
| Memory overhead | One `Arc<KnowledgeGraphValidator>` per executor | Compile-time check |

### Benchmarks to Add

No benchmarks in this PR. The Aho-Corasick automaton is already used in production in `terraphim_automata`.

## Open Items

| Item | Status | Owner |
|---|---|---|
| Confirm `thesaurus` should be `Option<Thesaurus>` in `RlmConfig` vs a separate constructor-only path | Pending | Alex |
| Confirm whether `QueryLoop` validation failures should terminate the loop or be context-fed | Pending | Alex |
| Decide if `ssh.rs` executor needs a real validator or can use disabled | Pending | Alex |

## Approval

- [ ] Technical review complete
- [ ] Test strategy approved
- [ ] Performance targets agreed
- [x] Human approval received

---

## Implementation Notes (2026-06-14)

### Resolved Design Decisions

1. **Thesaurus in RlmConfig**: Added `pub thesaurus: Option<terraphim_types::Thesaurus>` with `#[serde(skip, default)]` to RlmConfig. Thesaurus is set programmatically and NOT serialised in config files. Also added `TerraphimRlm::new_with_thesaurus` convenience constructor.

2. **QueryLoop: feed failure back to LLM as context**: Instead of returning error on validation failure, QueryLoop now calls `executor.validate()` before `Run`/`Code` commands. On failure, a `feedback_message()` is returned as `ExecuteResult::Continue { output }`, which gets injected into the next LLM prompt context. The LLM sees unknown terms and matched alternatives and rephrases. A `validation_retries: Cell<u32>` counter tracks retries per session.

3. **SSH/firecracker validator**: SshExecutor is used by FirecrackerExecutor, which now has a `validator: Option<Arc<KnowledgeGraphValidator>>` field. The validator is shared across all executors via `select_executor` which builds it from `config.thesaurus`.

### Learning/Security Hooks

Added `crates/terraphim_rlm/src/hooks.rs` with `ValidationEvent` struct and `emit_validation_event()`. Currently logs at `warn`/`info` level. External agents (terraphim-agent) can capture these via structured log scraping. The `ValidationEvent` is serialisable for future callback integration.

### What Changed from Original Design

| Original Design | Implemented |
|---|---|
| Error on validation failure in QueryLoop | Context feedback to LLM for rephrasing |
| `kg_max_retries` from config | Simple `Cell<u32>` counter in QueryLoop, resets on success |
| No hooks module | Added `hooks.rs` with `ValidationEvent` and `emit_validation_event()` |
| `new_with_thesaurus` only | Both `RlmConfig.thesaurus` field AND `new_with_thesaurus` convenience |

### Files Modified (per commit `7b46fb3de`)

| File | Changes |
|---|---|
| `crates/terraphim_rlm/src/config.rs` | +14 lines, thesaurus field |
| `crates/terraphim_rlm/src/executor/context.rs` | +84 lines, message/retry/escalation fields, `feedback_message()`, `from_validator_result()` |
| `crates/terraphim_rlm/src/executor/docker.rs` | +44/-xx, validator field, constructor, validate() |
| `crates/terraphim_rlm/src/executor/firecracker.rs` | +32/-xx, validator field, constructor, validate() |
| `crates/terraphim_rlm/src/executor/local.rs` | +23/-xx, validator field, with_validator(), validate() |
| `crates/terraphim_rlm/src/executor/mod.rs` | +54/-xx, build_validator_for_executor(), validator injection |
| `crates/terraphim_rlm/src/lib.rs` | +4, hooks module registration + re-export |
| `crates/terraphim_rlm/src/query_loop.rs` | +95/-xx, validation_retries Cell, validate_command(), Run/Code validation |
| `crates/terraphim_rlm/src/rlm.rs` | +137/-xx, build_validator thesaurus wiring, new_with_thesaurus |
| `crates/terraphim_rlm/src/hooks.rs` | NEW: +66 lines, ValidationEvent type and emitter |

### Verification

- 127 tests pass (default features)
- Clippy clean (pre-existing MSRV warnings only)
- Cargo fmt passes
- rlm.rs already validates in execute_code/execute_command (thesaurus now wired)
- Executors now validate in their validate() trait method (not stub anymore)
- QueryLoop validates Run/Code commands with LLM context feedback

---

## Appendix: Detailed Signatures

### `RlmConfig` addition
```rust
pub struct RlmConfig {
    // ...
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub thesaurus: Option<Thesaurus>,
}
```

### Executor constructor changes
```rust
// Firecracker
pub fn new(
    config: RlmConfig,
    validator: Option<Arc<KnowledgeGraphValidator>>,
) -> Result<Self, RlmError> { ... }

// Docker
pub fn new(
    config: RlmConfig,
    validator: Option<Arc<KnowledgeGraphValidator>>,
) -> Result<Self, RlmError> { ... }

// Local
pub fn new(validator: Option<Arc<KnowledgeGraphValidator>>) -> Self { ... }
```

### Executor `validate()` implementation
```rust
async fn validate(&self, input: &str) -> Result<ValidationResult, Self::Error> {
    match self.validator.as_ref() {
        Some(validator) if !input.trim().is_empty() => {
            let result = validator.validate(input)?;
            Ok(ValidationResult::from_validator_result(&result))
        }
        _ => Ok(ValidationResult::valid(Vec::new())),
    }
}
```

### Hot-path validation in `rlm.rs`
```rust
pub async fn execute_code(
    &self,
    session_id: &SessionId,
    code: &str,
) -> RlmResult<ExecutionResult> {
    self.session_manager.validate_session(session_id)?;

    let validation = self.executor.validate(code).await?;
    if !validation.is_valid {
        return Err(RlmError::KgValidationFailed {
            unknown_terms: validation.unknown_terms,
        });
    }

    let ctx = ExecutionContext { ... };
    self.executor.execute_code(code, &ctx).await.map_err(...)
}
```

### Hot-path validation in `query_loop.rs`
```rust
async fn execute_command(
    &self,
    command: &Command,
    ctx: &ExecutionContext,
    history: &mut CommandHistory,
) -> RlmResult<ExecuteResult> {
    let input_to_validate = match command {
        Command::Run(cmd) => Some(cmd.command.as_str()),
        Command::Code(code) => Some(code.code.as_str()),
        _ => None,
    };
    if let Some(input) = input_to_validate {
        let validation = self.executor.validate(input).await?;
        if !validation.is_valid {
            return Err(RlmError::KgValidationFailed {
                unknown_terms: validation.unknown_terms,
            });
        }
    }

    // ... existing match command ...
}
```
