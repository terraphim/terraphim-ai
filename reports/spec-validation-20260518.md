# Spec Validation Report

**Date:** 2026-05-18
**Agent:** spec-validator (Carthos)
**Issue:** #821 — fix(types): LearningStore::record_effective and record_applied must accept caller agent identity
**PR:** #1615
**Verdict:** PASS

---

## Traceability Report: Issue #821

### Requirements Enumerated

| ID | Requirement |
|----|-------------|
| REQ-A | `LearningStore::record_applied(id, applied_by)` and `record_effective(id, applied_by)` signatures updated in trait and `InMemoryLearningStore` |
| REQ-B | `InMemoryLearningStore` passes `applied_by` to `record_application()`, not `source_agent` |
| REQ-C | `test_in_memory_store_auto_promote_on_effective` passes: after 3 calls with 2 distinct agents, `trust_level == TrustLevel::L2` |
| REQ-D | All call sites in `terraphim_agent` and `terraphim_orchestrator` updated |
| REQ-E | No unused variable warnings in tests |
| REQ-F | `cargo test -p terraphim_types` — all tests pass |
| REQ-G | `cargo check -p terraphim_agent && cargo check -p terraphim_orchestrator` — both compile |

---

### Traceability Matrix

| Req ID | Requirement | Impl Ref | Tests | Evidence | Status |
|-------:|-------------|----------|-------|----------|--------|
| REQ-A | Trait + InMemoryLearningStore signatures | `crates/terraphim_types/src/shared_learning.rs:139-140, 223, 236` | `test_in_memory_store_record_applied_and_effective` | `cargo test -p terraphim_types --features kg-integration` | ✅ |
| REQ-B | `applied_by` forwarded, not `source_agent` | `shared_learning.rs:231` (`record_application(applied_by, false)`), `shared_learning.rs:244` (`record_application(applied_by, true)`) | same | same | ✅ |
| REQ-C | Auto-promote test passes | `shared_learning.rs:992–1011` | `test_in_memory_store_auto_promote_on_effective` | `cargo test -p terraphim_types --features kg-integration test_in_memory_store_auto_promote_on_effective` → 1 passed | ✅ |
| REQ-D | Call sites updated | `crates/terraphim_agent/src/shared_learning/store.rs:682-696` (forwards `applied_by`); `crates/terraphim_orchestrator/src/learning.rs:898-914` (accepts `_applied_by`) | `learning.rs:1136` | cargo check both crates → 0 errors | ⚠️ |
| REQ-E | No unused variable warnings | `learning.rs:901` (`_applied_by` prefix suppresses warning) | — | `cargo check -p terraphim_orchestrator` → 0 warnings | ✅ |
| REQ-F | `terraphim_types` full test suite | all | 111 unit + 15 doctests | `cargo test -p terraphim_types --features kg-integration` → 111 passed, 0 failed | ✅ |
| REQ-G | Compilation of agent + orchestrator | — | — | `cargo check -p terraphim_agent` → Finished; `cargo check -p terraphim_orchestrator` → Finished | ✅ |

---

### Gaps

#### ⚠️ REQ-D — P2 Non-blocking: orchestrator adapter discards `applied_by`

`terraphim_orchestrator::learning::SharedLearningStore` implements the `terraphim_types::LearningStore`
trait and accepts `applied_by: &str` in both `record_applied` and `record_effective`.
However, the parameter is named `_applied_by` and is silently discarded.
The underlying `LearningPersistence` async trait (orchestrator-internal) only accepts `id: &str`.

**Root cause:** The orchestrator's `LearningPersistence` uses a simpler count-based promotion model
(`effective_count >= N`), not `QualityMetrics`-based promotion (`agent_count >= 2`).
It has no structural place to record per-agent attribution.

**Impact on issue #821:** Low. The issue's root bug (broken auto-promotion in `terraphim_types`)
is fully resolved. The orchestrator-backed `SharedLearningStore` is bridging two systems;
the adapter satisfies the interface contract but does not propagate agent identity downstream.

**Recommended follow-up:** Create a separate issue to decide whether the orchestrator's
`LearningPersistence` should also grow `applied_by` tracking, or whether the interface
divergence is intentional (two promotion models: agent-count in types, count-only in orchestrator).

---

### Test Evidence

```
cargo test -p terraphim_types --features kg-integration test_in_memory_store_auto_promote_on_effective
→ running 1 test
→ test shared_learning::tests::test_in_memory_store_auto_promote_on_effective ... ok
→ test result: ok. 1 passed; 0 failed

cargo test -p terraphim_types --features kg-integration
→ running 111 tests
→ test result: ok. 111 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out

cargo check -p terraphim_agent     → Finished
cargo check -p terraphim_orchestrator → Finished
```

---

## Conclusion

The implementation satisfies all hard acceptance criteria from issue #821.
The core invariant — that `InMemoryLearningStore` in `terraphim_types` tracks caller agent identity
and correctly promotes L1 → L2 after 3 applications from 2 distinct agents — is verified.

One P2 non-blocking observation: the orchestrator adapter discards `applied_by` at the boundary
to its internal persistence layer. This is a design gap worth tracking but does not block closure.
