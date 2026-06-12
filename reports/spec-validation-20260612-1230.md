# Spec Validation Report — 2026-06-12 12:30 CEST

**Agent**: spec-validator (cron)
**HEAD**: 6ca9b50db1
**Verdict**: CONDITIONAL_PASS
**Cycle**: 12

## Test Results

| Crate | Passed | Failed | Ignored |
|-------|--------|--------|---------|
| `terraphim_rlm` | 137 | 0 | 0 |

Clippy: clean (no errors).

## Plans — Compliance Status

| Plan File | Status | Impl Status |
|-----------|--------|-------------|
| `design-gitea82-correction-event.md` | approved | POLYREPO_SKIP — terraphim_agent not in workspace |
| `design-gitea84-trigger-based-retrieval.md` | draft | POLYREPO_SKIP — terraphim_agent not in workspace |
| `design-single-agent-listener.md` | draft | POLYREPO_SKIP — terraphim_agent not in workspace |
| `d3-session-auto-capture-plan.md` | draft | POLYREPO_SKIP — terraphim_sessions feature-gated |
| `learning-correction-system-plan.md` | draft | POLYREPO_SKIP |
| `research-single-agent-listener.md` | research | POLYREPO_SKIP |

Note: `terraphim_agent` is outside the main Cargo workspace (`crates/*`). Plans targeting
`crates/terraphim_agent/src/…` cannot be validated here. See workspace Cargo.toml `exclude` list.

## Active Spec Gaps

### P1 — #2415 (cycle 12+): Hot-path validate() bypass

**Location**: `crates/terraphim_rlm/src/rlm.rs` and `crates/terraphim_rlm/src/query_loop.rs`

**Gap**: Neither `rlm.rs` nor `query_loop.rs` contain any call to `validate()` or
the `Validator` type. The spec requires `validate()` to be invoked at the hot-path
entry points before document indexing or query execution.

**Evidence**:
```
grep -n "validate|validator" rlm.rs        → (no output)
grep -n "validate|validator" query_loop.rs → (no output)
```

Validator exists at `src/validator.rs` with full implementations in
`executor/docker.rs` (6 tests) and `executor/firecracker.rs` (6 tests), but the
hot-path in `rlm.rs` and `query_loop.rs` bypasses the gate entirely.

**Status**: Unchanged from cycles 1–11.

### P2 — #2495 (cycle 7+): Doc lie in config.rs:356

**Location**: `crates/terraphim_rlm/src/config.rs`, lines 354–357

**Gap**: The doc comment on `KgStrictness::blocks_unknown()` claims:
> "Returns `true` for `Normal` and `Strict`, matching the hot-path behaviour
> in `query_loop.rs` and `rlm.rs`."

This is false. As confirmed by P1 above, neither `query_loop.rs` nor `rlm.rs`
call `validate()` or consult `blocks_unknown()`. The documentation describes
a coupling that does not exist in the code.

**Status**: Unchanged from cycles 1–6.

## Validated Implementations (This Cycle)

- `DockerExecutor::validate()` — present, 6 strictness-propagation tests pass
- `FirecrackerExecutor::validate()` — present, 6 strictness-propagation tests pass
- `Validator::validate()` / `validate_with_context()` — present, 10 unit tests pass
- All 137 `terraphim_rlm` tests pass with 0 failures

## Recommendations

1. **P1 #2415**: Wire `executor.validate(command)` at the start of `TerraphimRlm::execute_command`
   and `TerraphimRlm::execute_code` in `rlm.rs`. Similar gate needed in `query_loop.rs`
   before dispatch.
2. **P2 #2495**: Update `blocks_unknown()` doc to remove the false hot-path claim, or
   add the actual hot-path call (making the claim true).
3. **Polyrepo plans**: gitea82/84/d3 plans should reference the terraphim_agent polyrepo
   issue tracker, not the main terraphim-ai workspace.

## Session Context

Previous cycle: 6ca9b50db1 (same HEAD — no new commits since 12:25 CEST cron).
