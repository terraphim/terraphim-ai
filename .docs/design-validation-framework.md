# Implementation Plan: Validation Framework for terraphim-ai

**Status**: Draft
**Research Doc**: `.docs/research-validation-framework.md`
**Author**: Codex CLI (GPT-5)
**Date**: 2026-01-17
**Estimated Effort**: 5–8 days (integration + tests + docs)
**Owner Approval**: Alex Mikhalev (2026-01-17)

## Overview

### Summary
Adopt PR #413’s **release validation framework** (`crates/terraphim_validation`) and wire **runtime validation hooks** for pre/post LLM + pre/post tool stages. Preserve the new **guard + replacement** hook flow and document boundaries between release validation and runtime validation.

### Approach
- **Release Validation Track**: Merge/cherry‑pick PR #413; ensure workspace/Cargo/CI wiring and config placement.
- **Runtime Validation Track**: Wire pre/post LLM hooks in `terraphim_multi_agent`, keep guard+replacement in Claude Code pre‑tool flow, and document runtime validation behavior.

### Scope
**In Scope:**
- Integrate `crates/terraphim_validation` into workspace and CI
- Validate configuration (`validation-config.toml`) and default paths
- Wire pre/post LLM hooks around LLM generation
- Preserve guard stage for `--no-verify/-n` and document it

**Out of Scope:**
- LSP auto‑fix pipeline
- ML‑based anomaly detection
- Major refactors of execution subsystems

**Avoid At All Cost:**
- Duplicating runtime validation logic inside release validation framework
- Introducing non‑deterministic tests

## Architecture

### Component Diagram
```
[Release Validation]
  terraphim_validation
    -> ValidationSystem
      -> ValidationOrchestrator
        -> download/install/functionality/security/performance

[Runtime Validation]
  terraphim_agent
    -> Claude hook (pre_tool_use.sh) guard + replacement
  terraphim_multi_agent
    -> pre/post LLM hooks
    -> pre/post tool hooks (VM execution)
```

### Data Flow
```
Release QA:
  CI -> terraphim-validation CLI -> orchestrator -> report

Runtime:
  Claude Code -> pre_tool_use.sh (Guard -> Replacement) -> tool exec
  LLM generate -> pre-LLM -> generate -> post-LLM
  VM exec -> pre-tool -> execute -> post-tool
```

### Key Design Decisions
| Decision | Rationale | Alternatives Rejected |
|----------|-----------|-----------------------|
| Keep release vs runtime validation separate | Different concerns and lifecycles | Single monolithic validator |
| Wire pre/post LLM hooks in multi_agent | Existing hooks unused | Ignore LLM validation |
| Preserve guard stage in shell + document | Proven safety | Move entirely to Rust now |

### Eliminated Options (Essentialism)
| Option Rejected | Why Rejected | Risk of Including |
|----------------|-------------|------------------|
| LSP auto‑fix | Not essential | Complexity |
| Unified global config for both tracks | Premature | Coupling |

### Simplicity Check
**What if this could be easy?**
Merge PR #413 as‑is for release validation, then wire minimal runtime LLM hooks and update docs. Avoid refactoring existing hook systems.

### Configuration Decision
Runtime validation config is **separate** from release validation config:
- Runtime config: `~/.config/terraphim/runtime-validation.toml`
- Env overrides: `TERRAPHIM_RUNTIME_VALIDATION_*`
- Release config: `crates/terraphim_validation/config/validation-config.toml`

## File Changes

### New Files (from PR #413)
| File | Purpose |
|------|---------|
| `crates/terraphim_validation/*` | Release validation framework |
| `.github/workflows/performance-benchmarking.yml` | CI benchmarking |
| `PERFORMANCE_BENCHMARKING_README.md` | Docs |
| `scripts/validate-release-enhanced.sh` | Validation entrypoint |

### Modified Files
| File | Changes |
|------|---------|
| `Cargo.toml` | Add `terraphim_validation` to workspace members |
| `Cargo.lock` | Updated deps from PR |
| `crates/terraphim_multi_agent/src/agent.rs` | Pre/post LLM hook wiring |
| `crates/terraphim_agent/src/main.rs` | Document guard+replacement flow in help/output |
| `README.md` | Add validation framework section |

### Deleted Files
| File | Reason |
|------|--------|
| n/a | No deletions |

## API Design

### Release Validation Entry Point
```rust
pub struct ValidationSystem;
impl ValidationSystem {
    pub fn new() -> Result<Self>;
    pub async fn validate_release(&self, version: &str) -> Result<ValidationReport>;
}
```

### Runtime Validation (LLM Hook Wiring)
```rust
// Pre/post LLM hooks are already defined in vm_execution/hooks.rs
// Wire to LLM generation flow in multi_agent
```

## Test Strategy

### Unit Tests
| Test | Location | Purpose |
|------|----------|---------|
| `validation_system_creation` | `crates/terraphim_validation/src/lib.rs` | Basic instantiation |
| `orchestrator_config_load` | `crates/terraphim_validation/src/orchestrator/mod.rs` | Config parsing |
| `pre_post_llm_hook_invoked` | `crates/terraphim_multi_agent/tests/` | LLM hook wiring |

### Integration Tests
| Test | Location | Purpose |
|------|----------|---------|
| `validate_release_smoke` | `crates/terraphim_validation/tests/` | Minimal release validation run |
| `guard_blocks_no_verify` | shell test using `pre_tool_use.sh` | Guard stage behavior |

### Manual/Scripted Validation
- `scripts/validate-release-enhanced.sh` (PR #413)
- `echo '{"tool_name":"Bash","tool_input":{"command":"git commit --no-verify -m test"}}' | ~/.claude/hooks/pre_tool_use.sh`

## Implementation Steps

### Step 1: Integrate PR #413
**Files:** workspace `Cargo.toml`, `crates/terraphim_validation/*`, CI workflow
**Description:** Merge validation framework and ensure build passes.
**Tests:** `cargo build --workspace`.

### Step 2: Wire Runtime LLM Hooks
**Files:** `crates/terraphim_multi_agent/src/agent.rs`
**Description:** Build `PreLlmContext`/`PostLlmContext` and invoke hook manager around LLM generate.
**Call Sites:** Wrap `llm_client.generate(...)` in:
- `handle_generate_command`
- `handle_answer_command`
- `handle_analyze_command`
- `handle_create_command`
- `handle_review_command`
**Tests:** Unit test to assert hook invocation.

### Step 3: Document Guard+Replacement Flow
**Files:** `README.md`, possibly `.docs/`
**Description:** Describe two‑stage hook in runtime validation docs; mention bypass protection.
**Tests:** Manual command execution using shell hook.

### Step 4: CI & Release Validation Entry
**Files:** `.github/workflows/performance-benchmarking.yml`, `scripts/validate-release-enhanced.sh`
**Description:** Ensure release validation can run in CI and locally with documented steps.
**Tests:** CI dry run (if possible) or local smoke test.

## Rollback Plan
1. If release validation fails CI, disable workflow while keeping crate.
2. If LLM hook wiring introduces regressions, guard behind feature flag and revert.

## Dependencies

### New Dependencies
| Crate | Version | Justification |
|------|---------|---------------|
| `terraphim_validation` | PR #413 | Release validation |

## Performance Considerations

| Metric | Target | Measurement |
|--------|--------|-------------|
| LLM hook overhead | < 10ms | microbench or logging |
| Release validation runtime | configurable | PR #413 defaults |

## Open Items

| Item | Status | Owner |
|------|--------|-------|
| Merge PR #413 | Pending | Maintainer |
| Config location for runtime validation | Pending | Team |

## Approval

- [x] Research approved
- [x] Test strategy approved
- [x] Performance targets agreed
- [x] Human approval received

---

**Next:** Run `disciplined-quality-evaluation` on this design before implementation.
