<h3>Requirements Traceability Summary</h3>

PR #1291 — *Fix #1275: wire meta_coordinator module into lib.rs*

**Scope:** Two-line change adding `pub mod meta_coordinator;` to `crates/terraphim_orchestrator/src/lib.rs` and updating the module-level doc comment. This single declaration was the only thing preventing the 741-line `meta_coordinator.rs` (added in commit `57594168`) from entering the compilation unit and the public API.

---

<h3>Verdict: pass</h3>

The change is minimal, correct, and directly closes the blocker gap (META-001) identified in the spec validation report. Two pre-existing follow-up items are noted below; neither blocks merge.

---

<h3>Traceability Matrix</h3>

| Req ID | Requirement | Design Ref | Impl Ref | Tests | Evidence | Status |
|-------:|-------------|------------|----------|-------|----------|--------|
| META-001 | `meta_coordinator` declared in `lib.rs` public API | Gitea #1275 | `lib.rs:47` (`pub mod meta_coordinator;`) | 5 tests in `meta_coordinator.rs`: `test_compute_score_pagerank_dominates`, `test_compute_score_priority_penalty`, `test_select_agent_security_checklist`, `test_dispatch_dedup` (async), `test_cleanup_expired` (async) | `grep "pub mod meta_coordinator" crates/terraphim_orchestrator/src/lib.rs` -> 1 match | PASS |
| META-002 | `MetaCoordinator::dispatch_cycle` integration invariant reachable | Gitea #1275 | `meta_coordinator.rs:327` | `test_dispatch_dedup` covers dedup path; cleanup path covered by `test_cleanup_expired` | Tests now compile with module declaration present | PASS |
| META-DOC | `plans/` spec document for MetaCoordinator bounded context | -- | ABSENT | N/A | No `plans/design-meta-coordinator.md` exists | FOLLOW-UP |

---

<h3>Gaps</h3>

**Follow-up 1 -- Missing spec document (not blocking merge)**

`plans/design-meta-coordinator.md` does not exist. The MetaCoordinator is a 741-line bounded context (scoring formula, agent selection rules, TTL dispatch dedup, PageRank integration) with no design artefact in `plans/`. Recommended: create a short ADR or design doc covering the scoring formula, agent selection precedence, and TTL rationale. Tracked via the existing recommendation in the spec validation report.

**Follow-up 2 -- `last_cleanup` mutation bug (pre-existing, not introduced by this PR)**

`MetaCoordinator` holds `last_cleanup: Instant` as a plain field. `dispatch_cycle` takes `&self` (immutable receiver), so `last_cleanup` can never be updated after `cleanup_expired()` is called. After one hour, the condition `self.last_cleanup.elapsed() > Duration::from_secs(3600)` is permanently true: cleanup runs on every call rather than every hour. Fix: wrap in `Arc<Mutex<Instant>>` or `AtomicU64` epoch, or change receiver to `&mut self` at the call site. This bug pre-dates PR #1291 and is a separate concern; it does not block this declaration-only change.

No blocker gaps found in this PR.

---

<sub>Last spec-validated commit: 925b4e0</sub>
