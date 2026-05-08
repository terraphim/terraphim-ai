# Spec Validation Report -- 2026-05-08 (v11)

**Agent**: Carthos (Domain Architect, spec-validator)
**Run**: 2026-05-08 11:33 CEST -- cron schedule; scanned PRs #1283 and #1279 (no prior adf/spec); validated plans/ directory against implementation

---

## Results

| PR | Head SHA | Title | Verdict | Status Posted |
|----|----------|-------|---------|---------------|
| #1283 | b09954c | Fix #1266: NormalizedTerm missing fields | concerns | pending |
| #1279 | 8869399 | Fix #945: flush compiled thesaurus cache | concerns | pending |

---

## PR #1283 -- Concerns

**Issue**: #1266 -- NormalizedTerm struct literals break compilation with `--all-features` due to missing `action`, `priority`, `trigger`, `pinned` fields.

### Requirements Coverage

| Req | Description | Status |
|-----|-------------|--------|
| REQ-1266-001 | Add `action`, `priority`, `trigger`, `pinned` fields to `NormalizedTerm` with `#[serde(default)]` | PASS -- `terraphim_types/src/lib.rs` +36 lines, builder methods added |
| REQ-1266-002 | Update all downstream struct initializers to use builder pattern | PASS -- session-analyzer, agent, middleware, sessions, spawner updated |

### Concerns (non-blocking unless noted)

- **C-1283-A** (scope-creep): `crates/terraphim_orchestrator/src/lib.rs` (+23/-78) and `provider_probe.rs` (+62/-149) changes are substantial refactors -- the orchestrator circuit-breaker logic appears to be folded into this PR with no linking rationale in the PR title or description. Recommend splitting or explicit justification.
- **C-1283-B** (doc-bloat): `doc-reports/api-reference-20260506.md` (+896/-69) and `doc-gap-report-20260506.md` (+1830/-1702) are clearly unrelated to NormalizedTerm field fixes. These inflate review surface and risk hiding the actual fix.
- **C-1283-C** (merge conflict risk): Base changes in this PR are a superset of PR #1279 changes. Both modify `terraphim_types/src/lib.rs`, `terraphim_automata/src/builder.rs`, `terraphim_agent/src/main.rs`, `terraphim_service/src/lib.rs` with identical line counts. Merge order must be coordinated to avoid conflicts.

---

## PR #1279 -- Concerns

**Issue**: #945 -- SQLite-cached thesauri are not invalidated when KG markdown files are edited; `terraphim-agent replace` serves stale mappings until manual cache flush or process restart.

### Requirements Coverage

| Req | Description | Status |
|-----|-------------|--------|
| REQ-945-001 | Cache flush logic in `terraphim_automata/src/builder.rs` | PARTIAL -- +155 lines present; flush mechanism unverified without diff detail |
| REQ-945-002 | CLI integration for flush trigger in `terraphim_agent/src/main.rs` | PARTIAL -- +79 lines; command surface unverified |
| REQ-945-003 | Service-level cache invalidation in `terraphim_service/src/lib.rs` | PARTIAL -- +81 lines; invalidation hook unverified |

### Concerns

- **C-1279-A** (scope entanglement): `terraphim_types/src/lib.rs` +36 (NormalizedTerm fields) belongs to #1266/#1283, not #945. These PRs share the same base commit, which will cause a merge conflict or double-application.
- **C-1279-B** (doc-bloat): Same `doc-reports` changes (+2700 lines) as #1283 -- clearly unrelated to cache flush.
- **C-1279-C** (verification gap): No test file changes visible. Issue #945 requires verifying that after a markdown edit, the automata served by `replace` reflect the change. Without a test asserting this invariant, the fix is unverifiable by CI.

---

## Plans/ Directory Validation

### Active Spec Documents vs Implementation

| Plan | Spec Status | Implementation Status | Gap |
|------|-------------|----------------------|-----|
| `design-gitea82-correction-event.md` | APPROVED | IMPLEMENTED -- `CorrectionEvent` fully in `crates/terraphim_agent/src/learnings/capture.rs` | None |
| `design-gitea84-trigger-based-retrieval.md` | APPROVED | PARTIAL -- `trigger::` and `pinned::` parsing implemented with passing tests; `Graph list --pinned` CLI absent | G-REQ-84-004 persists |
| `d3-session-auto-capture-plan.md` | Draft | NOT IMPLEMENTED -- `procedure.rs` gated behind `#[cfg(test)]`; `from-session` subcommand not wired | G-D3-001 |
| `design-single-agent-listener.md` | APPROVED | NOT DEPLOYED -- `config/listener-worker.json` and `scripts/start-listener.sh` absent | G-SAL-001 |
| `research-single-agent-listener.md` | Research | Research doc only; no implementation expected at this stage | None |
| `learning-correction-system-plan.md` | Research | `procedure.rs` test-only; `SharedLearningStore` not in CLI; `AgentEvolutionSystem` standalone | G-LCS-001 |

### New Gaps Identified

| Gap ID | Description | Severity |
|--------|-------------|----------|
| G-D3-001 | `terraphim-agent learn procedure from-session` not implemented; `procedure.rs` in `#[cfg(test)]` only | Follow-up |
| G-SAL-001 | Single-agent listener design approved but `listener-worker.json` and `start-listener.sh` not created | Follow-up |
| G-LCS-001 | `procedure.rs` and `SharedLearningStore` from learning-correction-system-plan not wired to CLI | Follow-up |

---

## Persistent Gaps Re-assessment

| Gap | Previous Status | Current Status |
|-----|-----------------|----------------|
| G-META-001: `meta_coordinator` absent from `lib.rs` | OPEN (PR #1291 unmerged) | UNRESOLVED -- PR #1283 touches orchestrator `lib.rs` (+23/-78) but meta_coordinator wiring unconfirmed; PR #1291 still open |
| G-PH-H-001/002: `guard.rs` absent | OPEN -- no PR | OPEN -- no `guard.rs` found in crates |
| G-REQ-1266: NormalizedTerm struct literals | REGRESSION | RESOLVING -- PRs #1283/#1279 replace struct literals with builder pattern; not yet merged |
| G-REQ-84-004: `Graph list --pinned` CLI | FOLLOW-UP | OPEN -- `pinned` field in data model; CLI argument absent |

---

## Summary

Two PRs lack adf/spec validation and carry concerns primarily around scope entanglement and missing tests. The plans/ directory shows three new operational gaps where approved designs have not been deployed. Core persistent gaps (meta_coordinator, guard.rs, pinned CLI) remain unresolved.
