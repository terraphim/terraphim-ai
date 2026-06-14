# Research Document: Wire KG Validation into RLM Execution Hot Paths (#2671)

**Status**: Draft
**Author**: opencode / k2p7
**Date**: 2026-06-14
**Reviewers**: Alex (project owner)
**Source**: Issue #2671, epic #2667

---

## Executive Summary

`terraphim_rlm` has a working `KnowledgeGraphValidator` (`validator.rs`) and an `ExecutionEnvironment::validate()` trait method, but the two are not connected. `TerraphimRlm::execute_code`, `TerraphimRlm::execute_command`, and `QueryLoop::execute_command` skip validation entirely and go straight to execution. All executor `validate()` implementations are stubs returning `ValidationResult::valid(vec![])`. This means LLM-generated code/commands are executed without any KG safety check, bypassing the configured `kg_strictness` level.

The fix is to:
1. Give every executor a shared `KnowledgeGraphValidator` instance.
2. Make each executor's `validate()` call that validator.
3. Call `executor.validate(input).await?` in the three hot paths.
4. Convert a failed validation into `RlmError::KgValidationFailed` or `RlmError::KgEscalationRequired` based on strictness/retry state.

This is a safety-critical (P0) change; it blocks Step 5 (#2672) and unblocks release readiness for `terraphim_rlm`.

## Essential Questions Check

| Question | Answer | Evidence |
|---|---|---|
| Energizing? | Yes | Directly closes a P0 safety gap flagged in the components-functionality epic. |
| Leverages strengths? | Yes | Reuses existing `KnowledgeGraphValidator`, `terraphim_automata`, and executor trait patterns. |
| Meets real need? | Yes | Without this, `kg_strictness` config is ignored and arbitrary LLM output can execute. |

**Proceed**: Yes (3/3 YES).

## Problem Statement

### Description

The RLM orchestrator is designed to validate code/commands against a knowledge graph before execution. The configuration exposes `kg_strictness` (Permissive / Normal / Strict) and `kg_max_retries`. The validator exists and is tested. However, the execution hot paths never invoke it:

- `TerraphimRlm::execute_code` (`rlm.rs:310`) only checks `session_manager.validate_session(session_id)`.
- `TerraphimRlm::execute_command` (`rlm.rs:366`) only checks `session_manager.validate_session(session_id)`.
- `QueryLoop::execute_command` (`query_loop.rs:336`) dispatches `Command::Run` and `Command::Code` directly to the executor without validation.

All executor `validate()` stubs return the executor-context `ValidationResult::valid(vec![])`:
- `FirecrackerExecutor::validate` (`firecracker.rs:500`) has an explicit TODO.
- `LocalExecutor::validate` (`local.rs:159`) returns valid.
- `DockerExecutor::validate` (`docker.rs:371`) returns valid.

### Impact

- **Safety**: Arbitrary bash/Python from an LLM runs unchecked regardless of `kg_strictness`.
- **Config drift**: Users set `kg_strictness: Strict` and expect blocking; it has no effect.
- **Downstream blockers**: Step 5 (#2672) cannot be implemented until validation is invoked from the hot paths.

### Success Criteria

1. `TerraphimRlm::execute_code` validates `code` before execution and blocks if validation fails under the configured strictness.
2. `TerraphimRlm::execute_command` validates `command` before execution and blocks if validation fails.
3. `QueryLoop::execute_command` validates `Command::Run` and `Command::Code` payloads before execution.
4. All three executors (`Firecracker`, `Local`, `Docker`) delegate `validate()` to a real `KnowledgeGraphValidator`.
5. Validation failures surface as `RlmError::KgValidationFailed` or `RlmError::KgEscalationRequired`.
6. Existing tests still pass; new tests cover blocked and allowed commands.

## Current State Analysis

### Existing Implementation

| Component | Location | Purpose |
|---|---|---|
| `ExecutionEnvironment` trait | `crates/terraphim_rlm/src/executor/trait.rs:32` | Defines `validate(&self, input: &str) -> Result<ValidationResult, Self::Error>` |
| `ValidationResult` (executor context) | `crates/terraphim_rlm/src/executor/context.rs:224` | Simple result: `is_valid`, `matched_terms`, `unknown_terms`, `suggestions`, `strictness` |
| `KnowledgeGraphValidator` | `crates/terraphim_rlm/src/validator.rs:198` | Full validator using `terraphim_automata` and optional `RoleGraph` |
| `ValidationResult` (validator) | `crates/terraphim_rlm/src/validator.rs:81` | Rich result: `passed`, `matched_terms`, `unmatched_words`, `match_ratio`, `message`, `suggestions`, `escalation_required` |
| `TerraphimRlm::execute_code` | `crates/terraphim_rlm/src/rlm.rs:310` | Direct executor call, no validation |
| `TerraphimRlm::execute_command` | `crates/terraphim_rlm/src/rlm.rs:366` | Direct executor call, no validation |
| `QueryLoop::execute_command` | `crates/terraphim_rlm/src/query_loop.rs:336` | Dispatches `Run`/`Code` directly |
| `FirecrackerExecutor::validate` | `crates/terraphim_rlm/src/executor/firecracker.rs:500` | Stub |
| `LocalExecutor::validate` | `crates/terraphim_rlm/src/executor/local.rs:159` | Stub |
| `DockerExecutor::validate` | `crates/terraphim_rlm/src/executor/docker.rs:371` | Stub |
| `RlmConfig` | `crates/terraphim_rlm/src/config.rs:80` | Has `kg_strictness` and `kg_max_retries` |
| `RlmError` | `crates/terraphim_rlm/src/error.rs:68` | Already has `KgValidationFailed` and `KgEscalationRequired` |

### Data Flow (Current)

```text
Caller → TerraphimRlm::execute_code(session_id, code)
  → session_manager.validate_session(session_id)
  → executor.execute_code(code, ctx)          # NO KG CHECK
  → VM/container/local process
```

```text
LLM → QueryLoop::execute_command(Command::Run(cmd))
  → executor.execute_command(cmd, ctx)        # NO KG CHECK
  → VM/container/local process
```

### Data Flow (Target)

```text
Caller → TerraphimRlm::execute_code(session_id, code)
  → session_manager.validate_session(session_id)
  → executor.validate(code).await
       → KnowledgeGraphValidator::validate(code)
       → if failed: return RlmError::KgValidationFailed / KgEscalationRequired
  → executor.execute_code(code, ctx)
  → VM/container/local process
```

## Constraints

### Technical Constraints

- **Two `ValidationResult` types exist**. The trait uses the executor-context `ValidationResult` (`is_valid`/`unknown_terms`); the validator returns its own richer type. We must reconcile them without breaking the trait API.
- **Executor trait is widely implemented**. Six implementations exist (Firecracker, Docker, Local, SSH, E2B stub, mock in `rlm.rs`). Adding required trait methods is invasive; adding a field to each struct is manageable.
- **`KnowledgeGraphValidator` is not `Clone` cheaply** because it may hold a `RoleGraph`. It should be shared via `Arc`.
- **No mocks in tests**. Tests must use real `KnowledgeGraphValidator` with a real `Thesaurus`.
- **Feature flags**. Firecracker backend is gated by `#[cfg(feature = "firecracker")]`; Docker by `#[cfg(feature = "docker-backend")]`.
- **British English** in all docs and code comments.

### Business Constraints

- **No breaking changes to Firecracker path**. Production uses `fcctl-core`; validation must be additive.
- **No new HTTP/API changes**. This is an internal orchestrator change.
- **Must unblock Step 5 (#2672)**. Step 5 only replaces stubs with real validator usage; Step 4 must provide the validator to the executors.

### Non-Functional Requirements

| Requirement | Target | Current |
|---|---|---|
| Validation latency | < 10 ms per command | N/A (not called) |
| Coverage of hot paths | 100% (`execute_code`, `execute_command`, `QueryLoop::Run`/`Code`) | 0% |
| Config respect | `kg_strictness` enforced | Ignored |

## Vital Few (Essentialism)

### Essential Constraints (Max 3)

| Constraint | Why It's Vital | Evidence |
|---|---|---|
| **Keep `ExecutionEnvironment` trait stable** | Six implementations; trait churn is high-risk and not needed. | `grep "impl ExecutionEnvironment"` returns 6 hits. |
| **No new dependencies** | Existing `terraphim_automata`, `terraphim_types`, `terraphim_rolegraph` are sufficient. | `Cargo.toml` already declares them as optional deps. |
| **Additive safety only** | Production Firecracker path must keep working; validation must not change execution semantics when passing. | `fcctl-core` integration is live. |

### Eliminated from Scope

| Eliminated Item | Why Eliminated |
|---|---|
| Retry-with-LLM-rephrase loop | The `KnowledgeGraphValidator::validate_with_context` exists, but integrating a full retry loop into `QueryLoop` adds complexity. Defer to a follow-up; first iteration blocks on first failure. |
| Per-command strictness override | No caller needs this; config-level strictness is sufficient. |
| Rich MCP error payloads for validation | `to_mcp_error_data` already handles `KgEscalationRequired`; no change needed. |
| Validation of `Command::QueryLlm` / `Command::Snapshot` | These are meta-commands, not arbitrary code; out of scope for #2671. |
| Changing `ValidationResult` in `executor/context.rs` | We will map the validator's result to the existing type rather than break the trait. |
| Adding validation metrics/telemetry | Useful but not required for safety closure. |

## Dependencies

### Internal Dependencies

| Dependency | Impact | Risk |
|---|---|---|
| `KnowledgeGraphValidator` | Core validator; must be shareable via `Arc`. | Low — already `Send + Sync` candidate. |
| `terraphim_automata::find_matches` | Used inside validator. | None — stable API. |
| `terraphim_types::Thesaurus` | Must be constructible from config or passed in. | Low — already used. |
| `terraphim_rolegraph::RoleGraph` | Optional connectivity check. | Low — optional field. |
| `FirecrackerExecutor` | Must receive validator in `new()` / `initialize()`. | Medium — constructor change affects `select_executor`. |
| `DockerExecutor` / `LocalExecutor` | Must receive validator in `new()`. | Low — constructors are crate-internal. |

### External Dependencies

| Dependency | Version | Risk | Alternative |
|---|---|---|---|
| `fcctl-core` | git (production Firecracker) | Low — validation is pre-execution, does not touch VM lifecycle. | N/A |
| `bollard` | 0.20 | Low — Docker validation is pre-execution. | N/A |

## Risks and Unknowns

### Known Risks

| Risk | Likelihood | Impact | Mitigation |
|---|---|---|---|
| Mapping two `ValidationResult` types introduces confusion | Medium | Medium | Add a clear conversion helper; document the distinction. |
| `KnowledgeGraphValidator` without a thesaurus blocks nothing, even in Strict mode | Low | High | Verify validator behaviour: permissive/no-thesaurus passes; normal/strict with no thesaurus should fail. Already covered by `validator.rs` tests. |
| `FirecrackerExecutor::new` signature change breaks `select_executor` | Medium | Medium | Update `select_executor` to construct and pass the validator. |
| Validation adds latency to every query-loop iteration | Low | Medium | `terraphim_automata` Aho-Corasick matching is sub-millisecond for typical commands; benchmark if concerned. |
| Query loop swallows validation errors as "command failed" context | Medium | Medium | Return `RlmError` directly for validation failures so they are not treated as execution failures. |

### Open Questions

1. **Where does the thesaurus come from?** `RlmConfig` currently has no thesaurus path/role. Should we load a default thesaurus at executor construction, or accept an optional `Thesaurus` in `TerraphimRlm::new`?
   - *Recommendation*: Allow an optional `thesaurus` in `RlmConfig` or a new `TerraphimRlm::new_with_thesaurus`. Default to no thesaurus (permissive/validation disabled) to preserve current behaviour.
   - *Resolver*: Alex.

2. **Should validation failures in `QueryLoop` terminate the loop or feed back to the LLM?**
   - *Recommendation*: For this P0 fix, terminate with `RlmError`. A future enhancement can feed the validation message back as context for a retry.
   - *Resolver*: Alex.

3. **Should `ValidationResult` from the executor trait gain a `message` field to carry the validator's explanation?**
   - *Recommendation*: No — keep the existing struct and map `unmatched_words` to `unknown_terms`; the error we raise can include the validator's `message`.
   - *Resolver*: Alex.

4. **Is `ValidationContext`/retry logic in scope for #2671?**
   - *Recommendation*: No. Use the synchronous `KnowledgeGraphValidator::validate`. Retry/escalation can be added later when #2672 replaces stubs.
   - *Resolver*: Alex.

### Assumptions Explicitly Stated

| Assumption | Basis | Risk if Wrong | Verified? |
|---|---|---|---|
| `KnowledgeGraphValidator::validate` is the correct entry point (not `validate_with_context`). | Issue #2671 asks for blocking unsafe commands; retry/escalation is Step 5 (#2672). | Retry logic missing in first pass; acceptable per scope. | Yes — code inspection. |
| A validator with no thesaurus should not block execution. | `validator.rs:241` explicitly returns pass when `strictness == Permissive && thesaurus.is_none()`. For Normal/Strict without thesaurus, current code fails if `thesaurus.is_some()` is required. | Normal/Strict without thesaurus might incorrectly pass. | Yes — code inspection shows `if matched_terms.is_empty() && self.thesaurus.is_some()` guards. |
| The executor-context `ValidationResult` is the public API we must keep. | It is part of the `ExecutionEnvironment` trait and used by callers. | Breaking change if we replace it. | Yes — code inspection. |
| Validation should happen before building the execution context. | Simpler; no need to pass validation metadata through `ExecutionContext`. | Minor — could also be done inside executor methods. | Design choice; documented. |

### Multiple Interpretations Considered

| Interpretation | Implications | Why Chosen/Rejected |
|---|---|---|
| **A**: Validate inside each executor's `execute_code`/`execute_command` methods. | Centralised, no changes to `rlm.rs` or `query_loop.rs`; harder to surface rich errors. | Rejected — obscures the hot-path safety check and makes testing harder. |
| **B**: Validate in `TerraphimRlm` and `QueryLoop` before calling executor. | Explicit, testable, maps errors cleanly; requires executor to expose `validate`. | **Chosen** — matches issue #2671's stated target state and keeps safety visible at orchestrator level. |
| **C**: Replace executor-context `ValidationResult` with validator's `ValidationResult` everywhere. | Cleaner data model; breaks trait and all impls. | Rejected — too invasive for a P0 safety fix. |
| **D**: Add `validator: Arc<KnowledgeGraphValidator>` to `ExecutionContext`. | Keeps trait unchanged; validation available inside executor methods. | Rejected — `ExecutionContext` is serialised and should stay lightweight; validator is not serialisable. |

## Research Findings

### Key Insights

1. **Validation is fully implemented but unconnected.** The `KnowledgeGraphValidator` already supports Permissive/Normal/Strict modes, match ratios, connectivity checks, and suggestions. The gap is purely wiring.
2. **Executor trait already has the hook.** `ExecutionEnvironment::validate` exists; we only need to give executors a validator instance and call it.
3. **`RlmError` already has the right variants.** `KgValidationFailed` and `KgEscalationRequired` exist and are wired to MCP error codes.
4. **Two `ValidationResult` types are a real hazard.** The trait uses the executor-context type; the validator returns its own type. A conversion function is required.
5. **Config does not currently carry a thesaurus.** We need a way to inject one or default to no validation.

### Relevant Prior Art

- `crates/terraphim_agent/src/commands/modes/hybrid.rs` already does risk assessment before executing commands; similar pattern.
- `crates/terraphim_lsp/src/server.rs` uses `terraphim_automata` matching for KG analysis.
- `.docs/implementation-plan-rlm-executor-hardening.md` establishes the norm of no trait changes and no new dependencies for RLM executor work.

### Technical Spikes Needed

| Spike | Purpose | Estimated Effort |
|---|---|---|
| Confirm `KnowledgeGraphValidator` can be built from a `Thesaurus` without a `RoleGraph` | Validate fallback path | 15 min |
| Measure validation latency with a 100-term thesaurus | Performance confidence | 30 min |

## Recommendations

### Proceed/No-Proceed

**Proceed.** The problem is well-scoped, the validator exists, and the fix is additive wiring. This unblocks Step 5 and improves RLM safety.

### Scope Recommendations

- In scope: wiring validation into three hot paths, giving validators to executors, conversion helper, tests.
- Out of scope: retry loop, LLM rephrase, per-command strictness, metrics, trait changes.

### Risk Mitigation Recommendations

- Add unit tests that construct executors with a real thesaurus and assert blocked/allowed commands.
- Add integration tests in `rlm.rs` that call `execute_code`/`execute_command` with known/unknown terms.
- Keep Firecracker path untouched except for constructor wiring and the `validate` method body.

## Next Steps

1. Approve this research document.
2. Proceed to Phase 2 design: specify exact file changes, function signatures, and test strategy.
3. Implement in Phase 3 using `disciplined-implementation`.

## Appendix

### Code Snippets

Current stub in `FirecrackerExecutor::validate`:
```rust
async fn validate(&self, input: &str) -> Result<ValidationResult, Self::Error> {
    // TODO: Implement KG validation using terraphim_automata
    log::debug!("FirecrackerExecutor::validate called for {} bytes", input.len());
    Ok(ValidationResult::valid(Vec::new()))
}
```

Current hot path in `TerraphimRlm::execute_code`:
```rust
pub async fn execute_code(&self, session_id: &SessionId, code: &str) -> RlmResult<ExecutionResult> {
    self.session_manager.validate_session(session_id)?;
    let ctx = ExecutionContext { session_id: *session_id, timeout_ms: self.config.time_budget_ms, ..Default::default() };
    self.executor.execute_code(code, &ctx).await.map_err(|e| RlmError::ExecutionFailed { ... })
}
```

Current hot path in `QueryLoop::execute_command` for `Command::Run`:
```rust
Command::Run(bash_cmd) => {
    let result = self.executor.execute_command(&bash_cmd.command, ctx).await.map_err(|e| RlmError::ExecutionFailed { ... })?;
    ...
}
```

### Reference Materials

- Issue #2671 (Step 4)
- Issue #2672 (Step 5)
- `crates/terraphim_rlm/src/validator.rs`
- `crates/terraphim_rlm/src/executor/trait.rs`
- `crates/terraphim_rlm/src/executor/context.rs`
- `crates/terraphim_rlm/src/rlm.rs`
- `crates/terraphim_rlm/src/query_loop.rs`
- `.docs/implementation-plan-rlm-executor-hardening.md`
