# Documentation Gap Report

**Generated:** 2026-04-29
**Agent:** documentation-generator (Ferrox)
**Run:** Recurring scan on issue #1046

## Summary

Systematic scan of 11 primary workspace crates reveals **923 missing documentation items** and **53 rustdoc warnings** (excluding the unused tokio-tungstenite patch warning).

| Metric | Count |
|--------|-------|
| Crates scanned | 11 |
| Total missing docs | 923 |
| Total rustdoc warnings | 53 |
| Crates with zero missing docs | 0 |
| Most impacted crate | terraphim_orchestrator (430 missing) |

## Missing Documentation by Crate

| Crate | Missing Docs | Warnings | Severity |
|-------|-------------|----------|----------|
| terraphim_orchestrator | 430 | 16 | Critical |
| terraphim_service | 114 | 3 | High |
| terraphim_tinyclaw | 104 | 3 | High |
| terraphim_types | 79 | 5 | High |
| terraphim_middleware | 40 | 7 | Medium |
| terraphim_config | 38 | 1 | Medium |
| terraphim_tracker | 35 | 4 | Medium |
| terraphim_persistence | 30 | 4 | Medium |
| terraphim_router | 28 | 3 | Medium |
| terraphim_rolegraph | 22 | 4 | Medium |
| terraphim_file_search | 3 | 3 | Low |

## Rustdoc Warnings Breakdown

### Unresolved Links (17)
These are broken intra-doc links that will confuse users:

- `terraphim_types`: `HgncGene`, `HgncNormalizer` (2)
- `terraphim_service`: `kg:term` (1)
- `terraphim_middleware`: `Message` (1)
- `terraphim_orchestrator`: `RoutingDecisionEngine::decide_route`, `DispatchTask::AutoMerge` (x3), `AgentOrchestrator::poll_pending_reviews`, `GateConfig`, `handle_post_merge_test_gate_for_project` (7)
- `terraphim_tracker`: `set_commit_status` (1)
- `terraphim_rolegraph`: `new`, `from_serializable` (2)
- `terraphim_router`: `with_change_notifications` (1)
- `terraphim_file_search`: `ScoringContext` (1)

### Unclosed HTML Tags (9)
These will render incorrectly in generated docs:

- `terraphim_middleware`: `Message` (1)
- `terraphim_persistence`: `DeviceStorage` (2)
- `terraphim_orchestrator`: `name` (4), `HandoffContext` (1)
- `terraphim_service`: `DeviceStorage` (1)

### URLs Not Hyperlinked (7)
Plain URLs in docs that should be markdown links:

- `terraphim_types` (1)
- `terraphim_middleware` (4)
- `terraphim_tracker` (1)
- `terraphim_rolegraph` (1)

### Private Item Links (2)
Public docs linking to private items:

- `terraphim_orchestrator`: `resolve_mention` links to private `MENTION_RE`
- `terraphim_orchestrator`: `poll_pending_reviews` links to private `AgentOrchestrator::reconcile_tick`

## Recommendations

### Priority 1: Fix Broken Links (1-2 days)
The 17 unresolved links are user-facing defects. Most are simple renames or missing `pub` visibility.

### Priority 2: Add Crate-Level Docs (2-3 days)
All 11 crates lack `#![deny(missing_docs)]` enforcement. Start with:
1. `terraphim_orchestrator` -- add module docs to all 14 modules
2. `terraphim_service` -- document all 8 modules and core traits
3. `terraphim_tinyclaw` -- document the 4 modules

### Priority 3: Enforce at CI Gate
Add `cargo doc --workspace --no-deps -D warnings` to CI after warnings drop below 10.

## CHANGELOG Status

CHANGELOG.md updated with commits since v1.17.0 (2026-04-27). New entries added for:
- Session debouncing, PR security/compliance/test guardian templates
- Spawner task-body fix, per-project PR dispatch
- Clippy fixes, test alignment fixes

## Report Location

`doc-reports/documentation-gap-report-20260429.md`

Theme-ID: doc-gap
