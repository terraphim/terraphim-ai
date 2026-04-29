# Spec Validation Report: 2026-04-28 (Re-validation)

**Agent:** Carthos (Domain Architect / spec-validator)  
**Date:** 2026-04-28 11:45 CEST  
**Previous Report:** `reports/spec-validation-20260428.md` (09:33 CEST)  
**Scope:** Active plans in `plans/` directory cross-referenced with crate implementations  

---

## Executive Summary

Re-validation after the 09:33 report reveals **additional gaps not previously identified**. While commit `1bad203c1` resolved the CLI deviations noted in the earlier validation, **new violations and unimplemented features** have been discovered through deeper code inspection.

**Previous verdict:** PASS  
**Current verdict:** **FAIL** -- New critical and medium gaps identified.

---

## Plans Validated

| Plan | File | Previous Status | Current Status |
|------|------|----------------|----------------|
| Single Agent Listener (design) | `plans/design-single-agent-listener.md` | PASS | PASS |
| Single Agent Listener (research) | `plans/research-single-agent-listener.md` | PASS | PASS |
| Learning & Correction System | `plans/learning-correction-system-plan.md` | PARTIAL | **PARTIAL (new gaps)** |
| D3 Session Auto-Capture | `plans/d3-session-auto-capture-plan.md` | PASS | PASS |
| Gitea #82 CorrectionEvent | `plans/design-gitea82-correction-event.md` | PASS | PASS |
| Gitea #84 Trigger-Based Retrieval | `plans/design-gitea84-trigger-based-retrieval.md` | PASS | **PARTIAL (new gaps)** |

---

## New Findings (Not in Previous Report)

### 1. CRITICAL: Hook stdout redaction violates spec (Gitea #480)

**Previous report:** "Redaction wired in capture.rs and hook.rs" -- marked PASS  
**Current finding:** **VIOLATION**

The spec for Gitea #480 explicitly states that redaction should apply **only to stored content**, and the stdout passthrough must emit the original unredacted JSON:

> "The gap: when the hook passes through stdout, the original unredacted JSON goes to the agent. Redaction only applies to what gets stored."

**Actual code** (`crates/terraphim_agent/src/learnings/hook.rs:124-135`):
```rust
// Redact secrets before passing through to stdout
let output = if contains_secrets(&buffer) {
    log::debug!("Hook passthrough: secrets detected, redacting before stdout");
    redact_secrets(&buffer)
} else {
    buffer
};
```

**Problem:** Secrets are redacted **before stdout passthrough**. The agent never receives the original unredacted output. This contradicts the design requirement that redaction is storage-side only.

**Fix:** Remove `redact_secrets()` from the passthrough path. Apply redaction only in `capture_from_hook()` before persisting to storage.

---

### 2. MEDIUM: `--include-pinned` not propagated to all search paths (Gitea #84)

**Previous report:** "`--include-pinned` CLI flag PASS" -- claimed fully wired  
**Current finding:** **PARTIAL**

While the flag exists on the `Search` CLI variant, it is **hardcoded to `false`** in three execution paths:

| Path | File | Line | Evidence |
|------|------|------|----------|
| Single-term offline search | `terraphim_service/src/service.rs` | 326 | `search_with_role()` hardcodes `include_pinned: false` in `SearchQuery` |
| TUI mode | `crates/terraphim_agent/src/main.rs` | 4953 | `include_pinned: false` literal |
| REPL mode | `crates/terraphim_agent/src/repl/handler.rs` | 542 | `include_pinned: false` literal |

**Impact:** Users in TUI or REPL mode, or doing single-term offline searches, cannot retrieve pinned entries even when `--include-pinned` is passed.

**Fix:** Propagate the flag through all search entrypoints.

---

### 3. MEDIUM: `kg list --pinned` subcommand missing (Gitea #84)

**Previous report:** Acknowledged `graph --pinned` as "functionally equivalent"  
**Current finding:** **MISSING**

The spec explicitly mandates:
> "CLI: `--include-pinned` flag and `kg list --pinned` command"

The implementation provides only a flat `graph --pinned` flag (`main.rs:731-739`). There is no `KgSub` enum, no `list` subcommand, and no `kg list --pinned` path.

**Impact:** CLI surface does not match approved design. Users cannot list pinned entries independently of a search query.

**Fix:** Convert `Graph` command to a subcommand-bearing enum or add explicit `kg list` alias.

---

### 4. MEDIUM: Auto-suggest corrections unimplemented (Issue #703)

**Previous report:** Not mentioned  
**Current finding:** **MISSING**

The doc comment on `capture_failed_command()` (`capture.rs:920`) claims:
> "Auto-suggests correction from existing learnings (optional)"

The actual implementation (`capture.rs:969-991`) only:
1. Annotates with KG entities via `annotate_with_entities()`
2. Calculates `ImportanceScore`
3. Sets tags

The `correction` field on `CapturedLearning` remains `None` during capture. There is **no code that queries past learnings** for similar errors and populates the correction.

**Note:** The spec referenced a "TODO at line 609" -- line 609 is actually `CorrectionEvent::from_markdown()`, and no TODO exists in the file.

**Fix:** Implement `suggest_correction_from_history()` that queries stored learnings by similarity and populates the `correction` field.

---

### 5. LOW: `full_chain` parsing bug

**Previous report:** Not mentioned  
**Current finding:** **BUG**

In `CapturedLearning::from_markdown()` (`capture.rs:379`):
```rust
let full_chain = None; // Shadowed -- never populated from frontmatter
```

The variable is declared but immediately shadowed and never populated from parsed frontmatter data.

**Fix:** Parse `full_chain` from frontmatter if present.

---

### 6. LOW: `OPCODE_HOOK` env var typo

**Previous report:** Not mentioned  
**Current finding:** **TYPO**

In `install.rs:219`:
```rust
writeln!(script, "export OPCODE_HOOK=...")?; // Should be OPENCODE_HOOK
```

The Opencode hook environment variable is misspelled as `OPCODE_HOOK` instead of `OPENCODE_HOOK`.

**Fix:** Correct the spelling.

---

## Confirmed Previous Findings

These items were correctly identified in the 09:33 report and remain unchanged:

| Item | Status | Evidence |
|------|--------|----------|
| Phase H: Graduated Guard (#704) | **FAIL** | No `guard.rs` found |
| Phase I: Agent Evolution (#727-#730) | **FAIL** | Crate exists but not integrated |
| Session Auto-Capture (D3) | PASS | All items implemented |
| CorrectionEvent (Gitea #82) | PASS | All items implemented |
| Single Agent Listener | PASS | Operational setup only |
| `--robot`/`--format` flags (#578) | PASS | Fully wired |
| Procedure CLI (#693) | PASS | All subcommands exist |
| Replay engine (#694) | PASS | `replay.rs` exists and matches spec |
| Multi-hook types (#599) | PASS | `LearnHookType` enum present |
| Shared learning CLI (#727) | PASS | Feature-gated `shared-learning` |

---

## Updated Risk Summary

| Risk | Likelihood | Impact | Mitigation |
|------|-----------|--------|------------|
| Hook redaction violates fail-open/storage-only contract | High | High | Fix passthrough path |
| `--include-pinned` gaps confuse users | Medium | Low | Propagate flag |
| `kg list --pinned` missing | Medium | Low | Add subcommand structure |
| Auto-suggest unimplemented | Medium | Medium | Implement history query |
| Agent evolution orphaned | Low | Medium | Add dependency + wiring |

---

## Verdict

**OVERALL: FAIL**

New critical and medium gaps have been identified that were not caught in the 09:33 validation:

1. **Hook stdout redaction** (CRITICAL) -- contradicts storage-only redaction requirement.
2. **`--include-pinned` propagation** (MEDIUM) -- flag exists but is not honoured in TUI, REPL, or single-term offline search.
3. **`kg list --pinned` structure** (MEDIUM) -- flat flag provided where spec mandates subcommand.
4. **Auto-suggest corrections** (MEDIUM) -- documented but unimplemented.

These gaps require remediation before the spec can be considered fully satisfied.

---

## Recommendations

### P0 (Fix Immediately)
- **Fix hook stdout redaction** (`hook.rs:124-135`): Remove `redact_secrets()` from passthrough. Apply only to storage path.

### P1 (Fix This Sprint)
- **Propagate `--include-pinned`** through TUI, REPL, and `service.rs` single-term search.
- **Add `kg list --pinned`** or restructure `Graph` command to match spec.
- **Implement auto-suggest** in `capture_failed_command()`.

### P2 (Fix Next Sprint)
- **Fix `OPCODE_HOOK` typo** in `install.rs:219`.
- **Fix `full_chain` parsing** in `capture.rs:379`.
- **Wire agent evolution** crate into main binary.
- **Implement Graduated Guard** (Phase H).

---

*Report generated by Carthos, Domain Architect. Boundaries verified, relationships mapped, gaps surfaced.*

**Previous report:** `reports/spec-validation-20260428.md` (09:33 CEST)  
**This report:** `reports/spec-validation-20260428-revalidation.md`
