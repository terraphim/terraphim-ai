# Spec Validation Report: 2026-06-12 11:30 CEST (Cron)

**Agent**: spec-validator (Carthos, Domain Architect)
**HEAD**: 0e5042dc78
**Date**: 2026-06-12 11:30 CEST
**Run type**: Cron (no mention context)

## Verdict: CONDITIONAL PASS

137/0 terraphim_rlm unit tests pass; 7 doc-tests pass (1 ignored). Binary remains v1.8.0. **P1 #2415** (hot-path KG validate() bypass in execute_command/execute_code) and **P2 #2495** (blocks_unknown() doc comment lie) remain open and unchanged at this HEAD. Two plans (gitea82, d3) reference polyrepo/uninstalled subcommands — no new binary issued since last cycle. No regressions detected.

---

## Tests

```
terraphim_rlm: 137 PASS, 0 FAIL, 0 ignored
doc-tests:       7 PASS, 0 FAIL, 1 ignored
Binary version: terraphim-agent 1.8.0
```

---

## Plans Validated (5 total)

| Plan | Verdict | Notes |
|------|---------|-------|
| design-gitea82-correction-event.md | POLYREPO_SKIP | terraphim_agent crate is polyrepo; `learn correction` absent from v1.8.0 binary |
| d3-session-auto-capture-plan.md | POLYREPO_SKIP | terraphim_agent crate is polyrepo; `learn procedure from-session` absent from v1.8.0 binary |
| design-gitea84-trigger-based-retrieval.md | POLYREPO_SKIP | terraphim_automata is polyrepo; trigger::/pinned:: directives cannot be validated from main workspace |
| design-single-agent-listener.md | N/A | Operational plan — no Rust code changes; AC1–AC6 are runtime/config checks |
| learning-correction-system-plan.md | RESEARCH_ONLY | Analysis document, not an implementation spec |

---

## Active Spec Gaps

### P1 — #2415: execute_command/execute_code bypass KG validation (cycle 11+)

**Location**: `crates/terraphim_rlm/src/rlm.rs:310-391`

Both `execute_code()` (line 310) and `execute_command()` (line 366) call only `self.session_manager.validate_session(session_id)?` — they do NOT call `validator.validate()` against the KG. The execution path goes directly to `self.executor.execute_code/command()` without passing input through the KG validator.

By contrast, `query()` (line 438) creates a `QueryLoop` which handles KG validation internally via the loop's own logic.

**Status**: Open (`status/in-progress`). No code change at HEAD 0e5042dc78.

---

### P2 — #2495: blocks_unknown() doc comment claims hot-path behaviour not yet implemented (cycle 7+)

**Location**: `crates/terraphim_rlm/src/config.rs:354-360`

The doc comment reads: _"Returns `true` for `Normal` and `Strict`, matching the hot-path behaviour in `query_loop.rs` and `rlm.rs`."_ However, as confirmed by P1 above, `execute_command` and `execute_code` in `rlm.rs` do not call `blocks_unknown()` or any KG validation. The doc comment is inaccurate — it asserts behaviour that doesn't exist until P1 is fixed.

**Status**: Open (no labels). No code change at HEAD 0e5042dc78. Will resolve naturally when P1 is implemented.

---

## Traceability Matrix

| Req | Location | Tests | Status |
|-----|----------|-------|--------|
| terraphim_rlm validate() on execute_command | `rlm.rs:366` | Missing | ❌ P1 #2415 |
| terraphim_rlm validate() on execute_code | `rlm.rs:310` | Missing | ❌ P1 #2415 |
| blocks_unknown() doc comment accurate | `config.rs:354` | Test exists, doc wrong | ⚠️ P2 #2495 |
| CorrectionEvent CLI (learn correction) | polyrepo | N/A | POLYREPO_SKIP |
| Procedure from-session CLI | polyrepo | N/A | POLYREPO_SKIP |
| trigger::/pinned:: parsing | polyrepo | N/A | POLYREPO_SKIP |

---

## Recommendations

1. **P1 #2415** remains the critical unblock. Until `execute_command`/`execute_code` wire `validator.validate()`, the KG safety boundary is porous for direct-execution callers.
2. **P2 #2495** resolves automatically once P1 is implemented — no separate fix needed.
3. **POLYREPO_SKIP plans**: No action from main workspace. Binary v1.8.0 does not carry these features; they require a release from the terraphim_agent polyrepo.
