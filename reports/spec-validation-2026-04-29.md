# Spec Validation Report — 2026-04-29

**Validator**: Carthos (Domain Architect, spec-validator)
**Date**: 2026-04-29
**Scope**: All active specification documents in `plans/` directory
**Verdict**: **FAIL**

---

## Specifications Reviewed

| Spec File | Gitea/GitHub Ref | Status |
|-----------|-----------------|--------|
| `design-gitea82-correction-event.md` | Gitea #82 | MOSTLY IMPLEMENTED (90%) |
| `design-gitea84-trigger-based-retrieval.md` | Gitea #84 | PARTIALLY IMPLEMENTED (70%) — **Critical dead code** |
| `learning-correction-system-plan.md` | Master plan (#480-#730) | PARTIALLY IMPLEMENTED (65%) |
| `d3-session-auto-capture-plan.md` | D3 auto-capture | IMPLEMENTED (100%) — **Plus extras** |
| `design-single-agent-listener.md` | Operational | CONFIGURED BUT NOT RUNNING |
| `research-single-agent-listener.md` | Operational | N/A (research only) |

---

## Critical Findings

### 1. Trigger-Based KG Retrieval Dead in Production (Gitea #84)

**Severity**: CRITICAL — Code exists but is unreachable

The `TriggerIndex` TF-IDF fallback and `load_trigger_index()` method are fully implemented in `crates/terraphim_rolegraph/src/lib.rs` (lines 50-488), including:
- `TriggerIndex` struct with build/query/tokenise
- `find_matching_node_ids_with_fallback()` two-pass search
- 9 unit tests (all passing)
- Serialisation roundtrip support

**However**, `load_trigger_index()` is **only called in tests** (lines 2205, 2224, 2243). No production code path:
- Parses markdown directives to extract `trigger::` and `pinned::`
- Maps node IDs to trigger descriptions
- Calls `load_trigger_index()` on the RoleGraph

**Root cause**: `KgRouter` in `terraphim_orchestrator/src/kg_router.rs` (lines 141-142) explicitly discards trigger and pinned data when building `NormalizedTerm`:
```rust
trigger: None,
pinned: false,
```

**Impact**: The entire two-pass TF-IDF fallback is dead code in production. Aho-Corasick-only search continues to work; trigger-based fallback and pinned entries never activate.

**Also missing**:
- `kg list --pinned` CLI subcommand (spec Section 7). Instead, `graph --pinned` was implemented.
- `query_graph_with_operators()` lacks trigger fallback — multi-term AND/OR queries never use the fallback path.

---

### 2. Hook Success Capture Missing (Master Plan Phase B2/B4)

**Severity**: HIGH — Specified in #693 but not implemented

The hook pipeline (`crates/terraphim_agent/src/learnings/hook.rs`) only captures Bash failures (`exit_code != 0`). The following are missing:
- `should_capture_success()` method
- `capture_success_from_hook()` function
- `SessionCommandBuffer` for grouping commands into procedures
- Automatic `CapturedProcedure` creation from successful multi-command sequences

**Impact**: Procedures must be created manually via `learn procedure record`. Auto-capture from sessions (D3 plan) is implemented via `from-session` command, but real-time success capture during hook pipeline execution is absent.

---

### 3. Agent Evolution Not Wired (Master Plan Phase I)

**Severity**: HIGH — Crate exists but isolated

The `terraphim_agent_evolution` crate is fully structured but:
- `LlmAdapterFactory` only creates mock adapters (`llm_adapter.rs:169-217`)
- Zero references in `terraphim_agent` main binary or its `Cargo.toml`
- No production integration path exists

**Impact**: Evolution workflow management, memory snapshots, and lesson tracking are inaccessible to the running agent.

---

### 4. Shared Learning Sync CLI Missing (Master Plan Phase G1)

**Severity**: MEDIUM

The `GiteaWikiClient` and wiki sync logic exist in `shared_learning/wiki_sync.rs`, but there is no `learn shared sync` CLI subcommand to trigger it.

---

### 5. Sandbox/Firecracker Guard Tier Missing (Master Plan Phase H)

**Severity**: MEDIUM

The graduated guard has `Allow` and `Block` decisions (`guard_patterns.rs`, `replay.rs`), but no `Sandbox` execution tier. Firecracker integration (`terraphim_firecracker/`) exists but is not wired to the guard pipeline.

---

### 6. CorrectionEvent Missing Public Exports (Gitea #82)

**Severity**: LOW — Internal functionality works; public API incomplete

`CorrectionEvent` and `LearningEntry` are implemented in `capture.rs` and used internally by `main.rs`, but are **not re-exported** from `learnings/mod.rs` as specified. This limits external crate consumption.

Also missing: Integration test for `terraphim-agent learn correction` CLI subcommand.

---

## Implementation Summary by Phase

| Phase | Issue(s) | Status | Key Gaps |
|-------|----------|--------|----------|
| A | #480, #578 | COMPLETE | None |
| B | #693 | MOSTLY COMPLETE | Hook success capture; SessionCommandBuffer |
| C | #703 | COMPLETE | None |
| D | #694 | COMPLETE | None |
| E | #599, #686 | MOSTLY COMPLETE | #686 evaluation document not found |
| F | #695 | COMPLETE | None |
| G | #727 partial | MOSTLY COMPLETE | `learn shared sync` CLI missing |
| H | #704 | PARTIAL | Sandbox/Firecracker tier missing |
| I | #727-#730 | MISSING | Mock LLM only; not wired to main binary |
| J | #515-#517, #451 | PARTIAL | Partial via kg_validation module |

---

## Traceability Matrix

| Spec | Requirement | Design Ref | Impl Ref | Tests | Status |
|------|-------------|------------|----------|-------|--------|
| Gitea #82 | CorrectionType enum | capture.rs:42 | capture.rs:42-94 | test_correction_type_roundtrip | PASS |
| Gitea #82 | CorrectionEvent struct | capture.rs:497 | capture.rs:497-521 | test_correction_event_to_markdown | PASS |
| Gitea #82 | capture_correction() | capture.rs:1009 | capture.rs:1009-1066 | test_capture_correction | PASS |
| Gitea #82 | list_all_entries() | capture.rs:1297 | capture.rs:1297-1369 | test_list_all_entries_mixed | PASS |
| Gitea #82 | CLI learn correction | main.rs:961 | main.rs:961-978, 3120-3146 | MISSING (no integration test) | ⚠️ |
| Gitea #82 | Public exports | mod.rs | mod.rs:40-49 | — | ❌ |
| Gitea #84 | MarkdownDirectives.trigger | types/lib.rs:498 | types/lib.rs:498 | — | PASS |
| Gitea #84 | MarkdownDirectives.pinned | types/lib.rs:500 | types/lib.rs:500 | — | PASS |
| Gitea #84 | trigger:: parsing | automata/markdown_directives.rs | automata/markdown_directives.rs:215-224 | parses_trigger_directive | PASS |
| Gitea #84 | pinned:: parsing | automata/markdown_directives.rs | automata/markdown_directives.rs:226-230 | parses_pinned_directive | PASS |
| Gitea #84 | TriggerIndex TF-IDF | rolegraph/lib.rs:50 | rolegraph/lib.rs:50-248 | 9 tests in rolegraph + dedicated test file | PASS |
| Gitea #84 | load_trigger_index() in prod | rolegraph/lib.rs:478 | — | — | ❌ |
| Gitea #84 | kg list --pinned | main.rs Section 7 | — | — | ❌ |
| Master Plan | Un-gate procedure.rs | mod.rs:31 | mod.rs:31 (no cfg gate) | — | PASS |
| Master Plan | Hook success capture | hook.rs | — | — | ❌ |
| Master Plan | SessionCommandBuffer | — | — | — | ❌ |
| Master Plan | Real LLM adapters | agent_evolution/llm_adapter.rs | llm_adapter.rs:169-217 (mocks only) | — | ❌ |
| Master Plan | Agent evolution wired | — | — | — | ❌ |
| Master Plan | Sandbox guard tier | guard_patterns.rs | guard_patterns.rs (Allow/Block only) | — | ⚠️ |

---

## Recommendations

### Immediate (Blockers for Spec Compliance)

1. **Wire trigger index in production** (Gitea #84)
   - Modify `KgRouter` in `terraphim_orchestrator/src/kg_router.rs` to preserve `trigger` and `pinned` from `MarkdownDirectives`
   - Add call to `RoleGraph::load_trigger_index()` after thesaurus loading in `terraphim_config` or `terraphim_automata`
   - Add `kg list --pinned` CLI subcommand or update spec to match `graph --pinned`

2. **Implement hook success capture** (GitHub #693)
   - Add `should_capture_success()` to `HookInput`
   - Add `SessionCommandBuffer` to accumulate successful commands
   - Wire `ProcedureStore::save_with_dedup()` into success path

3. **Wire agent evolution** (GitHub #727-#730)
   - Replace mock LLM adapters with real adapter factory calls
   - Add `terraphim_agent_evolution` dependency to `terraphim_agent/Cargo.toml`
   - Add CLI commands to expose evolution workflows

### Short-Term

4. Add `learn shared sync` CLI subcommand
5. Implement Sandbox tier in guard (or update spec to exclude Firecracker integration)
6. Add integration test for `learn correction` CLI
7. Export `CorrectionEvent` and `LearningEntry` from `learnings/mod.rs`

---

## Conclusion

The codebase shows significant progress toward the specifications, with Phases A, C, D, and F fully complete. However, **critical gaps remain**:

- **Gitea #84**: The most significant violation — a complete feature (TF-IDF trigger fallback) is implemented but entirely unreachable in production due to missing wiring.
- **GitHub #693**: Success capture is a major gap in the procedural memory pipeline.
- **GitHub #727-#730**: Agent evolution is an isolated crate with no production path.

**Verdict: FAIL** — Spec violations found in Gitea #84, GitHub #693, #727-#730, and minor omissions in #82.
