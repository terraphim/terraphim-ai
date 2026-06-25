# Spec Validation Report: 2026-06-25 21:33 CEST

**Validator**: Carthos (Domain Architect — spec-validator)
**Trigger**: cron schedule
**Status**: FAIL

---

## Executive Summary

The `plans/` directory contains 6 design/research documents. **Zero of those 6 plans target any of the 13 currently active workspace crates.** All 6 plans reference crates that were extracted to separate polyrepos under Gitea #1910. The plans directory is fully stale.

---

## Active Workspace Members (13 crates)

Derived from `cargo metadata --workspace`:

| Crate | Notes |
|-------|-------|
| `terraphim-firecracker` | Active |
| `terraphim_ai_nodejs` | Active |
| `terraphim_dsm` | Active |
| `terraphim_lsp` | Active |
| `terraphim_merge_coordinator` | Active |
| `terraphim_rlm` | Active |
| `terraphim_server` | Active |
| `terraphim_spawner` | Active |
| `terraphim_tinyclaw` | Active |
| `terraphim_update` | Active |
| `terraphim_validation` | Active |
| `terraphim_weather_report` | Active |
| `terraphim_workspace` | Active |

---

## Plans vs Implementation Cross-Reference

| Plan File | Targets | Extraction Status | Coverage |
|-----------|---------|-------------------|----------|
| `d3-session-auto-capture-plan.md` | `crates/terraphim_agent/src/learnings/procedure.rs` | Extracted (E4a, #1910) — terraphim-agents polyrepo | STALE |
| `design-gitea82-correction-event.md` | `crates/terraphim_agent/src/learnings/capture.rs` | Extracted (E4a, #1910) — terraphim-agents polyrepo | STALE |
| `design-gitea84-trigger-based-retrieval.md` | `crates/terraphim_types`, `crates/terraphim_automata` | Extracted (E1, #1910) — terraphim-core polyrepo | STALE |
| `design-single-agent-listener.md` | Operational setup — no Rust code changes | N/A | NOT APPLICABLE |
| `learning-correction-system-plan.md` | `crates/terraphim_agent/src/learnings/` | Extracted (E4a, #1910) — terraphim-agents polyrepo | STALE |
| `research-single-agent-listener.md` | Research only — no code | N/A | NOT APPLICABLE |

**Plan coverage of active workspace crates: 0 / 13 crates**

---

## Traceability Matrix

| Req ID | Requirement | Design Ref | Impl Ref | Tests | Status |
|--------|-------------|------------|----------|-------|--------|
| PLAN-01 | Session auto-capture (`from-session`) | `d3-session-auto-capture-plan.md` | terraphim-agents repo (not local) | terraphim-agents tests | ❌ STALE PATH |
| PLAN-02 | CorrectionEvent capture | `design-gitea82-correction-event.md` | terraphim-agents repo | terraphim-agents tests | ❌ STALE PATH |
| PLAN-03 | trigger:: / pinned:: KG directives | `design-gitea84-trigger-based-retrieval.md` | terraphim-core repo | terraphim-core tests | ❌ STALE PATH |
| PLAN-04 | Single Gitea listener | `design-single-agent-listener.md` | N/A (operational) | N/A | ℹ️ N/A |
| PLAN-05 | Learning correction system | `learning-correction-system-plan.md` | terraphim-agents repo | terraphim-agents tests | ❌ STALE PATH |
| PLAN-06 | Listener research | `research-single-agent-listener.md` | N/A (research) | N/A | ℹ️ N/A |

---

## Gap Summary

| Gap | Severity | Issue | PR |
|-----|----------|-------|-----|
| 6 plans reference extracted polyrepo code | Blocker | #2972 (open) | #2954 (mb=T, unmerged) |
| CI: rust-clippy/rust-compile undefined in pr-summary | Blocker | #2940 (open) | #2954 (mb=T, unmerged) |
| cargo audit gate missing from CI | Blocker | #2937 (open) | #2955 (mb=T, unmerged) |
| RUSTSEC-2026-0185/0186 in Cargo.lock | P1 Security | #2937 | #2950 (mb=T, unmerged) |
| design-gitea84 query_graph() not wired | Follow-up | #2890 (open) | None |

---

## Key Finding: PR #2954 Unblocks Plans Staleness

PR #2954 ("Fix #2940: add rust-clippy and rust-compile jobs to ci-pr.yml; archive stale plans") is:
- mergeable=True
- Addresses both the CI gap (#2940) and archives the stale plans

**Blocker**: `native-ci / build` has been showing state=None on recent main commits, suggesting the CI gate may be failing and blocking the merge queue.

---

## Workspace Tests

`cargo test --workspace --no-run` compiled successfully (all executables built). Full test run pending.

---

## Verdict: FAIL

Recurrence #8+ — same stale plans pattern persists. Root fix is merging PR #2954.
