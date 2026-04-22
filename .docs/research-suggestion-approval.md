# Research Document: Suggestion Approval Workflow and Notification (Phase 4)

**Status**: Draft
**Author**: opencode (AI agent)
**Date**: 2026-04-22
**Gitea Issue**: #85 (part of Epic #81)

## Executive Summary

Issue #85 requests a suggestion approval workflow that surfaces knowledge suggestions to users at natural breakpoints and provides batch operations. The infrastructure is 70% built: `CorrectionEvent` capture (Phase 2) and trigger-based retrieval (Phase 3) are complete. What's missing is the **approval state machine** (pending/approved/rejected), **session-end surfacing**, **batch CLI operations**, and **metrics tracking**. The existing `SharedLearningStore` with L1/L2/L3 trust levels maps directly to the approval workflow -- L1 = pending, L3 = approved.

## Essential Questions Check

| Question | Answer | Evidence |
|----------|--------|----------|
| Energizing? | Yes | Closes the feedback loop -- the most impactful missing piece of the learning capture system |
| Leverages strengths? | Yes | Builds on existing `SharedLearningStore`, `BM25Scorer`, `TrustLevel`, and `GiteaWikiClient` |
| Meets real need? | Yes | Epic #81 target: "3-5 KG entries/week from suggestions, 60%+ approval rate" -- currently 0 because no approval path exists |

**Proceed**: Yes (3/3)

## Problem Statement

### Description

The learning capture system (Phases 1-3 of Epic #81) captures corrections and failed commands, but there is no mechanism to:
1. Surface accumulated suggestions to users for review
2. Let users approve/reject suggestions individually or in batch
3. Track approval metrics over time
4. Integrate suggestion review into daily workflow (morning brief)

### Impact

Without this, all captured corrections remain "unverified" (L1) forever. The knowledge graph never grows from user feedback. The epic's success criteria (60%+ approval rate, 3-5 entries/week) cannot be met.

### Success Criteria

1. Users can review pending suggestions at session end (one-line summary)
2. Users can batch approve/reject by confidence threshold
3. Suggestion metrics are tracked in `suggestion-metrics.jsonl`
4. Daily sweep integration exists in morning brief template

## Current State Analysis

### Existing Implementation

#### Phase 2 (COMPLETE): CorrectionEvent Capture
- `CorrectionEvent` struct with 6 correction types + Other
- `capture_correction()` function with secret redaction
- `LearningEntry` enum unifying Learning + Correction
- Markdown storage with YAML frontmatter
- CLI: `learn capture`, `learn correction`, `learn list`, `learn query`

#### Phase 3 (COMPLETE): Trigger-Based Contextual Retrieval
- `TriggerIndex` with TF-IDF scoring
- `compile_corrections_to_thesaurus()` compiles corrections into thesaurus
- `build_kg_thesaurus_from_dir()` loads KG concepts
- Entity annotation via Aho-Corasick on captured text

#### SharedLearningStore (COMPLETE, WIRED TO CLI)
- `SharedLearningStore` with BM25 dedup and trust levels (L1/L2/L3)
- `MarkdownLearningStore` backend
- `GiteaWikiClient` for L2/L3 wiki sync
- `QualityMetrics` tracking: applied_count, effective_count, agent_count
- Auto-promotion from L1 to L2 (3+ applications, 2+ agents)
- CLI: `learn shared list/promote/import/stats` (behind `shared-learning` feature)

### Code Locations

| Component | Location | Purpose |
|-----------|----------|---------|
| `CorrectionEvent` | `capture.rs:502-521` | User correction struct |
| `LearningEntry` | `capture.rs:1200-1205` | Unified entry enum |
| `capture_correction()` | `capture.rs:642-722` | Store correction to disk |
| `SharedLearningStore` | `shared_learning/store.rs` | Trust-gated shared store |
| `QualityMetrics` | `terraphim_types/src/shared_learning.rs` | Quality tracking |
| `TrustLevel` | `terraphim_types/src/shared_learning.rs` | L1/L2/L3 enum |
| `Bm25Scorer` | `shared_learning/store.rs` | Similarity scoring |
| `GiteaWikiClient` | `shared_learning/wiki_sync.rs` | Wiki sync for L2+ |
| CLI LearnSub | `main.rs:843-943` | CLI subcommands |
| CLI SharedLearningSub | `main.rs:945-979` | Shared learning subcommands |
| `compile_corrections_to_thesaurus()` | `learnings/compile.rs` | Correction to thesaurus |

### Data Flow (Current)

```
Failed command → capture_failed_command() → learning-*.md
User correction → capture_correction() → correction-*.md
Corrections → compile_corrections_to_thesaurus() → compiled-corrections.json
Local learnings → learn shared import → SharedLearningStore (L1)
L1 → promote_to_l2() (auto via QualityMetrics) → L2
L2 → promote_to_l3() (manual) → L3
L2+ → GiteaWikiClient → wiki pages
```

### Gap Analysis

What exists vs. what #85 requires:

| Task (#85) | Status | Gap |
|------------|--------|-----|
| 4.1 Session-end suggestion prompt | **Missing** | No `suggest` subcommand or session-end hook |
| 4.2 Daily sweep integration | **Missing** | No morning brief template integration |
| 4.3 Batch approve/reject | **Partial** | `learn shared promote` exists but no batch, no confidence thresholds, no reject |
| 4.4 Suggestion metrics | **Missing** | No metrics tracking, no `suggestion-metrics.jsonl` |

### Integration Points

- **CLI**: `LearnSub` and `SharedLearningSub` enums in `main.rs`
- **SharedLearningStore**: Already supports `suggest()` method with BM25 context matching
- **GiteaWikiClient**: Ready for syncing approved suggestions
- **Hook system**: `learnings/hook.rs` can be extended for session-end prompts

## Constraints

### Technical Constraints
- Must use existing `SharedLearningStore` -- no new storage backend
- Must not introduce new external dependencies (use existing `serde_json`, `chrono`, `tokio`)
- Feature-gated behind `shared-learning` (already exists)
- CLI must remain backward compatible (existing `learn` subcommands unchanged)

### Business Constraints
- Epic #81 targets: 60%+ approval rate, 3-5 entries/week, < 24h correction-to-KG
- Effort estimate in #85: 1.5h (optimistic -- realistic estimate: 1-2 days)

### Non-Functional Requirements

| Requirement | Target | Notes |
|-------------|--------|-------|
| Batch operation latency | < 2s for 100 items | BM25 already fast |
| Metrics file size | < 1MB/year | JSONL append-only |
| Session-end prompt | < 100ms | Simple count + top suggestion display |

## Vital Few (Essentialism)

### Essential Constraints (Max 3)

| Constraint | Why It's Vital | Evidence |
|------------|----------------|----------|
| Approval state machine (pending/approved/rejected) | Without it, no suggestion can transition to KG | #85 tasks 4.1-4.3 all depend on this |
| Confidence-based batch operations | Users won't review 100s of suggestions individually | #85 task 4.3 is explicit about thresholds |
| Metrics tracking | Can't improve what you don't measure | Epic #81 success criteria require measurable outcomes |

### Eliminated from Scope

| Eliminated Item | Why Eliminated |
|-----------------|----------------|
| Interactive TUI for suggestion review | CLI-first approach per project philosophy; `--interactive` can be a later enhancement |
| Real-time suggestion notifications (websocket/push) | Session-end and daily sweep are sufficient per #85 spec |
| LLM-based auto-approval | Risk of false approvals; human review is core to the trust model |
| Cross-agent suggestion sharing via CRDT | Gitea wiki sync already covers this |

## Dependencies

### Internal Dependencies

| Dependency | Impact | Risk |
|------------|--------|------|
| `SharedLearningStore.suggest()` | Already exists, returns contextually relevant learnings | Low -- just needs a CLI wrapper |
| `SharedLearningStore.promote_to_l3()` | Already exists for approve path | Low |
| `SharedLearningStore.store_with_dedup()` | For importing suggestions | Low |
| `TrustLevel` enum | L3 = approved, L1 = pending, need "rejected" state | Medium -- no rejected state exists |

### External Dependencies

| Dependency | Version | Risk | Alternative |
|------------|---------|------|-------------|
| `serde_json` | workspace | None | N/A |
| `chrono` | workspace | None | N/A |
| `tokio` | workspace | None | N/A |

## Risks and Unknowns

### Known Risks

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| No "rejected" state in TrustLevel | High | Medium | Add `Rejected` variant or use separate `suggestion_status` field |
| `suggestion-metrics.jsonl` grows unbounded | Low | Low | Add rotation/pruning (out of scope, document for future) |
| Session-end prompt may not fire if hook is not installed | Medium | Low | Fall back to `learn suggest` CLI command |
| BM25 suggestion quality may be low initially | Medium | Medium | Use confidence thresholds; tune after observing approval rates |

### Open Questions

1. Should "rejected" be a new TrustLevel variant or a separate field? -- Separate `SuggestionStatus` enum is cleaner (doesn't pollute TrustLevel semantics)
2. Should session-end prompt require the `shared-learning` feature? -- Yes, naturally gated behind it
3. What confidence score should `suggest()` return? -- BM25 normalised score (0.0-1.0) already available, just not surfaced to CLI

### Assumptions Explicitly Stated

| Assumption | Basis | Risk if Wrong | Verified? |
|------------|-------|---------------|-----------|
| `SharedLearningStore.suggest()` returns usable suggestions | Code analysis: BM25 with trust-level weighting | Suggestions may be irrelevant | Partially -- needs real-world testing |
| Users run `learn shared import` before seeing suggestions | CLI flow analysis | Suggestions empty if not imported | Yes |
| JSONL metrics file is adequate for tracking | Low write volume (1-10 entries/day) | Growth over years | Yes |

## Research Findings

### Key Insights

1. **The `suggest()` method already exists** on `SharedLearningStore` (store.rs:317-357) -- it accepts a context string and agent name, returns relevant `SharedLearning` entries ranked by BM25 score. This is the core engine for the approval workflow.

2. **Trust levels map naturally to approval states**: L1 = pending suggestion, L2 = peer-validated, L3 = human-approved. What's missing is a "rejected" state and batch operations.

3. **The `QualityMetrics` struct already tracks everything needed for metrics** (applied_count, effective_count, agent_count, success_rate). The gap: no aggregation into a metrics file, no time-to-review tracking.

4. **`compile_corrections_to_thesaurus()`** already converts approved corrections into KG thesaurus entries. The approval workflow feeds directly into this.

5. **The CLI dispatch is in `run_shared_learning_command()`** (main.rs:2768-2970). New suggestion subcommands would extend `SharedLearningSub`.

### Relevant Prior Art

- **Devin's knowledge system**: The source inspiration (per Epic #81 body). Surfaces suggestions at session end, batch approve/reject.
- **VS Code settings sync**: Approval workflow for extension recommendations -- similar batch approve/reject UX.

### Technical Spikes Needed

| Spike | Purpose | Estimated Effort |
|-------|---------|------------------|
| SuggestionStatus enum design | Determine if new enum or extend TrustLevel | 30 min |
| suggest() score surfacing | Expose BM25 confidence score to CLI output | 1h |

## Recommendations

### Proceed/No-Proceed

**Proceed** -- the infrastructure is substantially complete. The work is primarily CLI integration and adding a `SuggestionStatus` tracking layer on top of `SharedLearningStore`.

### Scope Recommendations

1. Add `SuggestionStatus` (Pending/Approved/Rejected) as a separate concern from `TrustLevel`
2. Add `learn suggest` subcommand that calls `SharedLearningStore::suggest()` and displays results with confidence scores
3. Add `learn suggest approve-all --min-confidence`, `reject-all --max-confidence`, `review`
4. Add `suggestion-metrics.jsonl` append-only logging
5. Session-end prompt as a separate `learn suggest --session-end` command (hooks into existing hook system)
6. Daily sweep integration: document the `learn suggest --pending` command for morning brief template

### Risk Mitigation Recommendations

- Start with `SuggestionStatus` as a field on `SharedLearning` (not a TrustLevel variant) to avoid breaking existing consumers
- Use feature gate `shared-learning` (already exists) for all new code
- Write metrics to JSONL with automatic daily rotation logic

## Next Steps

If approved:
1. Proceed to Phase 2 (Design) -- create implementation plan
2. Design `SuggestionStatus` enum and metrics types
3. Specify exact CLI subcommand additions
4. Plan test strategy (all existing tests must pass, new tests for each subcommand)
