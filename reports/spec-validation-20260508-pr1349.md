<h3>Requirements Traceability Summary</h3>

**PR:** #1349 — Fix #251: enforce RetryBound invariant in Symphony on_retry_timer  
**Author:** root  
**Head SHA:** 0adaa20  
**Validator:** Carthos (Domain Architect)  
**Date:** 2026-05-08 07:27 CEST

---

<h3>Verdict: concerns</h3>

Implementation is correct in both guarded paths. Config accessor is unit-tested. The critical gap is the absence of an orchestrator-level integration test verifying that the claimed set is actually cleared when the retry bound is hit at runtime.

---

<h3>Traceability Matrix</h3>

| Req ID | Requirement | Design Ref | Impl Ref | Tests | Evidence | Status |
|-------:|-------------|------------|----------|-------|----------|--------|
| REQ-251-001 | RetryBound guard on poll-failure path in `on_retry_timer` | Gitea #251 §Fix | `orchestrator/mod.rs:588–604` | None | Diff line ~+596: `if next >= max_attempts { claimed.remove }` | ⚠️ |
| REQ-251-002 | RetryBound guard on no-slots path in `on_retry_timer` | Gitea #251 §Fix | `orchestrator/mod.rs:620–633` | None | Diff line ~+622: `if next >= max_attempts { claimed.remove }` | ⚠️ |
| REQ-251-003 | `max_retry_attempts()` config accessor, default 10 | Gitea #251 §Expected Behaviour | `config/mod.rs:184–191` | `max_retry_attempts_default`, `max_retry_attempts_configurable` | `config/mod.rs:533–546` | ✅ |
| REQ-251-004 | TLA+ `RetryBound` invariant: `retryCount[i] <= MaxRetries` holds at runtime | TLA+ spec `specs/symphony/SymphonyOrchestrator.tla:127–134` (external repo) | Both paths in `orchestrator/mod.rs` | No Rust integration test | Model proves invariant for 9,983 states (referenced in issue); no Rust regression test | ⚠️ |
| CONF-251-001 | `agent.max_retry_attempts` YAML key is configurable | Commit message (WORKFLOW.md mention) | `config/mod.rs:188` | `max_retry_attempts_configurable` | `config/mod.rs:544` | ✅ |

---

<h3>Gaps</h3>

**⚠️ G-251-001 — No orchestrator integration test for RetryGiveUp behaviour**

The two config unit tests confirm the accessor works, but no test exercises `on_retry_timer` with a retry entry whose `attempt` has reached `max_retry_attempts`. Concretely, there is no test that:

1. Creates a fake tracker and claimed-set entry
2. Fires the retry timer with `attempt == max_retry_attempts`
3. Asserts the issue is no longer in `self.state.claimed`

Without this, a future refactor could silently remove the bound check and the test suite would remain green. This is a follow-up, not a blocker, because the TLA+ model provides formal proof of the invariant — but formal models and Rust runtime behaviour are not automatically kept in sync.

*Recommended path:* Add a unit test in `orchestrator/mod.rs mod tests` using `OrchestratorRuntimeState` directly, or a small integration test in `crates/terraphim_symphony/tests/` that exercises the retry exhaustion path with a stub tracker.

**ℹ️ G-251-002 — No ADR or plans document for Symphony retry semantics**

The retry semantics (backoff, MaxRetries, claimed-set lifecycle) live entirely in the Gitea issue and an external TLA+ repo. A short ADR or `plans/design-symphony-retry-semantics.md` would anchor future changes and make the invariant discoverable without following the issue chain.

*Recommended path:* Not blocking merge. Add as a follow-up on #251.

---

**Persistent v5 gaps (unchanged):**

| Gap | Status |
|-----|--------|
| G-META-001: `meta_coordinator` absent from `lib.rs` | ❌ OPEN — PR #1291 unmerged |
| G-PH-H-001/002: `guard.rs` absent | ❌ OPEN — no PR |
| G-REQ-1266: `NormalizedTerm` struct literals on main | ❌ REGRESSION — PR #1343 unmerged |
| G-REQ-84-004: `Graph list --pinned` CLI absent | ⚠️ FOLLOW-UP — no PR |

---

<sub>Last spec-validated commit: 0adaa20  
PR #1349 verdict: concerns — two config unit tests present; orchestrator claimed-set release on retry exhaustion is unverified by Rust test.  
Persistent v5 gaps: 3 blockers, 1 follow-up (all unchanged — PR #1291 and PR #1343 remain unmerged).</sub>
