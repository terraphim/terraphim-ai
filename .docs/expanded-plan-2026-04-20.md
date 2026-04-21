# Expanded Implementation Plan: 2026-04-20

**Date**: 2026-04-20 19:05 BST
**Branch**: `main` at `261884ae`
**Status**: Plan -- Awaiting Approval

---

## Current State Summary

### Recent Completed Work (since last handover 2026-04-05)

| Item | Status | Reference |
|------|--------|-----------|
| PR #754: KG-boosted file search with ExternalScorer | Merged, v1.17.0 | handover-2026-04-03 |
| PR #796-803: Release stabilisation (v1.16.33-1.16.34) | Merged | git log |
| CI shallow-clone test fixes | Merged | `261884ae` |
| Docker multi-arch build fixes | Merged | `00ae6594`, `df218c59` |
| chrono-to-jiff timestamp migration (listener) | Merged | `7fc74dc0` |
| ADF compound review auto-file fix (Gitea 422) | Deployed | handover-2026-04-04-adf |
| Synthetic time for 3 ADF crates | Done, NOT committed | handover-2026-04-04-synthetic |

### Outstanding Workstreams (from all handovers)

---

## Workstream 1: Post-Merge Cleanup (3 Items)

**Priority**: HIGH -- design doc approved, ready to implement
**Estimated**: 3-4 hours
**Design doc**: `.docs/design-post-merge-implementation-plan.md`
**Research doc**: `.docs/research-remaining-work-2026-04-05.md`

### 1A: Disable Discord Default Feature (15 min)

- Remove `discord` from `default` features in `crates/terraphim_tinyclaw/Cargo.toml`
- Re-add RUSTSEC-2026-0049 to `deny.toml` ignore list
- Eliminates CVE vector without breaking builds

### 1B: Feature-Gate shared_learning (10 min)

- Add `shared-learning` feature flag to `crates/terraphim_agent/Cargo.toml`
- Gate the module with `#[cfg(feature = "shared-learning")]` in `lib.rs`
- Remove sqlx comment from Cargo.toml

### 1C: Rewrite store.rs with Persistable (60-90 min)

- Replace sqlx-backed SQLite store with `Persistable + in-memory HashMap`
- Each learning stored as `shared-learning/{id}.json`
- BM25 scoring runs against in-memory map (fine for < 10K learnings)
- Keep all public method signatures unchanged
- ~952 lines -> ~400 lines net reduction

### 1D: Build Verification and Tests (30 min)

- Full workspace build with `--features shared-learning`
- All existing tests pass

### 1E: Document NormalizedTerm Decision (5 min)

- Already documented in research doc -- verify, close out

---

## Workstream 2: Synthetic Time (Uncommitted)

**Priority**: MEDIUM -- code works, needs committing
**Estimated**: 30 min (commit + verify)
**Handover**: `.docs/handover-2026-04-04-synthetic-time.md`

### 2A: Commit Synthetic Time Changes

- 7 functions parameterised with `now: DateTime<Utc>`
- 3 crates: terraphim_symphony, terraphim_agent_supervisor, terraphim_agent_messaging
- 139 + 24 + 38 tests passing
- MUST separate from unrelated orchestrator changes in working tree

### 2B: Fix LinearTracker Pre-existing Issue

- `LinearTracker::from_config` missing in symphony binary
- Blocks full `cargo test` (including binary) and `cargo clippy --tests`
- Location: `crates/terraphim_tracker/src/linear.rs`

### 2C: Add tokio::time::pause Tests (Future)

- Infrastructure ready, not yet used in existing tests
- For retry timer firing and health check intervals

---

## Workstream 3: ADF Dark Factory Operations

**Priority**: MEDIUM -- operational, needs maintenance
**Handover**: `.docs/handover-2026-04-04-adf-compound-review.md`

### 3A: Kill Duplicate ADF Processes (Immediate)

- Multiple instances may be running on bigbox
- Verify single instance, clean up

### 3B: Fix Command Collision (Short Term)

- `@adf:compound-review` triggers BOTH swarm + single agent
- Rename `compound-review` agent to `quality-coordinator` in orchestrator.toml
- Or add priority logic in parser

### 3C: Wire SharedLearningStore into Orchestrator (Medium Term)

- Depends on Workstream 1C (store.rs rewrite)
- Gitea issue #242

### 3D: Implement Deduplication for Auto-Filed Issues (Medium Term)

- Compound review may re-file the same finding
- Needs content-based dedup before creating Gitea issues

### 3E: Add Labels Support Back (Medium Term)

- Currently removed from Gitea API payload
- Requires fetching label IDs from Gitea API first
- Or fix in Gitea fork to accept string names

---

## Workstream 4: TLA+ Verification Bugs

**Priority**: MEDIUM -- 8 bugs filed from TLA+ model checking
**Handover**: `.docs/handover-2026-04-04-synthetic-time.md`
**Issues**: Gitea #251-#261

### Priority Bugs (from TLA+ specs)

| Issue | Severity | Description |
|-------|----------|-------------|
| #251 | P0 Critical | RetryBound violation -- retry count can exceed configured maximum |
| #252-#254 | P1 | Race conditions in concurrent state transitions |
| #255-#258 | P2 | Liveness properties violated under specific timing |
| #259-#260 | Enhancement | Additional safety invariants to add |
| #261 | Epic | TLA+ verification framework integration |

### Approach

1. Start with #251 (RetryBound) -- highest impact, affects supervisor restart logic
2. Use synthetic time (Workstream 2) to write deterministic tests for each bug
3. Fix bugs, verify with TLA+ model checker

---

## Workstream 5: Orchestrator Phase 2

**Priority**: LOW -- Phase 1 MVP complete
**Handover**: `.docs/handover-dark-factory-orchestration.md`

### 5A: Wire evaluate() into Reconciliation Loop (VAL-3)

- Add `tokio::time::interval` branch to `select!`
- Periodic NightwatchMonitor drift evaluation

### 5B: Implement PR Creation in Compound Review (VAL-1)

- Call `gh pr create` when `create_prs=true`
- Currently placeholder/dry-run only

### 5C: Integration Test with Real CLI Tools on BigBox

- Spawn actual `echo`/`codex`/`claude` processes
- Test AgentSpawner with real commands

### 5D: BigBox tmux Deployment

- Set up tmux sessions per dark factory architecture
- Production-like environment for orchestrator

---

## Workstream 6: Infrastructure and Hygiene

**Priority**: LOW -- important but not urgent

### 6A: Clean Up Uncommitted terraphim_multi_agent Changes

- 4 files modified (Cargo.toml, lib.rs, pool.rs, registry.rs)
- Review and either commit or discard

### 6B: Clean Up Git Stashes

- 14 stashes accumulated
- Old stashes (especially stash@{0} with hash performance benchmarks) need review

### 6C: Remove Old Zig Installations on BigBox

- Versions 0.9.1 and 0.13.0 still present
- Current: 0.15.2

### 6D: Old fff.nvim Fork Dependency

- Pinned to branch -- consider tagging a release for stability
- `AlexMikhalev/fff.nvim` branch `feat/external-scorer`

---

## Recommended Execution Order

```
Week 1 (Immediate):
  Day 1: Workstream 2A (commit synthetic time)
  Day 1: Workstream 1A + 1B (CVE fix + feature gate)
  Day 1: Workstream 3A (kill duplicate ADF processes)

  Day 2: Workstream 1C + 1D (store.rs rewrite + verification)
  Day 2: Workstream 3B (command collision fix)

  Day 3: Workstream 4 - Bug #251 (RetryBound violation)
  Day 3: Workstream 2B (fix LinearTracker)

Week 2:
  Day 4: Workstream 4 - Bugs #252-#254 (race conditions)
  Day 4: Workstream 3C (wire SharedLearningStore)

  Day 5: Workstream 4 - Bugs #255-#258 (liveness)
  Day 5: Workstream 5A (evaluate() in reconciliation loop)

Week 3:
  Day 6: Workstream 5B + 5C (PR creation + real CLI tests)
  Day 6: Workstream 6A-6D (hygiene cleanup)
```

---

## Dependency Graph

```
Workstream 1A (CVE) ────────────────────────────────> Independent, do first
Workstream 1B (feature gate) ───────────────────────> Independent
Workstream 1C (store rewrite) ──> depends on 1B
Workstream 1D (verify) ─────────> depends on 1C
Workstream 2A (commit synthetic) ──────────────────> Independent
Workstream 2B (LinearTracker) ─────────────────────> Independent
Workstream 3A (kill dupes) ────────────────────────> Independent
Workstream 3B (cmd collision) ─────────────────────> Independent
Workstream 3C (wire store) ─────> depends on 1C + 1D
Workstream 3D (dedup issues) ───> depends on 3C
Workstream 3E (labels) ─────────> Independent
Workstream 4 (TLA+ bugs) ───────> benefits from 2A (synthetic time for tests)
Workstream 5A-5D (orchestrator) > Independent
Workstream 6A-6D (hygiene) ─────> Independent
```

---

## Gitea Issue Tracker: Ready Issues (Top 20 by PageRank)

**Source**: `gtr ready --owner terraphim --repo terraphim-ai` (2026-04-20)
**Total open issues**: 199 across 630 total

### Highest-Value Ready (Unblocked, Highest PageRank)

| # | Issue | PageRank | Priority | Maps to Workstream |
|---|-------|----------|----------|--------------------|
| #630 | Security checklist: CVEs in rustls-webpki + port 11434 exposure | 0.150 | P2 | WS1A (CVE fix) |
| #625 | Epic: EDM Scanner | 0.150 | P40 | New -- see WS7 below |
| #626 | EDM scanner: terraphim_negative_contribution crate | 0.006 | P40 | New -- see WS7 below |
| #624 | Orphaned crates/terraphim_settings directory | 0.150 | P4 | WS6 (Hygiene) |
| #591 | feat(automata): extract_context for byte-offset context | 0.150 | P0 | WS8 (automata) |
| #578 | feat(orchestrator): wire agent_evolution into ADF | 0.150 | P38 | WS5 (orchestrator) |
| #574 | terraphim.ai Ship Systems: wire to live benchmark data | 0.150 | P6 | WS9 (infra) |
| #579 | OpenAI Codex session limit causing provider probe timeouts | 0.150 | P5 | WS3 (ADF ops) |
| #518 | Add validation metrics and observability | 0.002 | P25 | WS10 (quality) |
| #144 | Epic: Inter-agent orchestration via Gitea mentions | 0.017 | P50 | WS3 (ADF ops) |

### Top-10 Highest PageRank Across ALL Issues (by dependency impact)

| PageRank | Issue | Status |
|----------|-------|--------|
| 0.0167 | #144 Epic: Inter-agent orchestration via Gitea mentions | Open |
| 0.0094 | #85 Phase 4: Suggestion approval workflow | Open |
| 0.0093 | #238 ADF: Re-enable test-guardian and full deployment | Open |
| 0.0075 | #149 Phase 5: Webhook-driven mention detection | Open |
| 0.0066 | #45 Implement execution tiers with risk classification | Open |
| 0.0063 | #278 AgentRunRecord with ExitClass taxonomy | Open |
| 0.0059 | #626 EDM scanner: terraphim_negative_contribution | Open |
| 0.0055 | #279 Nightly failure clustering via Flow DAG | Open |
| 0.0045 | #423 Phase 1 test suite for robot mode | Open |
| 0.0045 | #242 Phase 1: SQLite shared learning store | Open |

### Blocked Issues (High Value, Waiting on Dependencies)

| # | Issue | Blocked By |
|---|-------|------------|
| #629 | EDM scanner: terraphim_lsp workspace crate | 1 blocker |
| #628 | EDM scanner: wire static pre-check into CompoundReview | 1 blocker |
| #627 | EDM scanner: integration test baseline | 1 blocker |
| #258 | TLA+: Dispatch error paths bypass MaxRetries | #251 (RetryBound fix) |
| #256 | TLA+: ExactlyOnceNoDuplicates not runtime-validated | 1 blocker |
| #243 | Phase 2: Gitea wiki sync for L2+ learnings | 1 blocker |
| #244 | Phase 3: Quality loop and verification patterns | 1 blocker |

---

## Workstream 7: EDM Scanner (New from Gitea)

**Priority**: MEDIUM -- new epic, 4 sub-issues
**Epic issue**: Gitea #625
**Status**: Epic unblocked, first crate (#626) ready to start

### 7A: New crate terraphim_negative_contribution (#626)

- Tier 1 patterns for detecting "negative contribution" code
- Unblocked, ready to implement

### 7B: terraphim_lsp workspace crate (#629) -- BLOCKED

- publishDiagnostics and 250ms debounce
- Blocked by 1 dependency

### 7C: Wire static pre-check into CompoundReviewWorkflow (#628) -- BLOCKED

- Blocked by 1 dependency

### 7D: Integration test baseline (#627) -- BLOCKED

- Blocked by 1 dependency

---

## Workstream 8: Automata extract_context (#591)

**Priority**: MEDIUM -- P0, ready, no blockers
- `feat(terraphim_automata): add extract_context function for byte-offset-based context extraction`
- Unblocked, ready to implement

---

## GitHub Open Issues (30 open)

Relevant active items from GitHub:

| # | Title | Relevance |
|---|-------|-----------|
| #810 | Learning-driven command correction: Phase 2 & 3 | Follow-on to learning system |
| #727 | Wire terraphim_agent_evolution into ADF orchestrator | Maps to Gitea #578 |
| #728 | Add AgentEvolution haystack ServiceType | Phase 2 of evolution |
| #729 | Cross-run compounding via lesson injection | Phase 3 of learning |
| #682 | Epic: Evaluate Pi architectural patterns | Cross-repo research |
| #691 | Epic: ToCS-based agent evaluation framework | ADF quality |
| #637 | Epic: Leverage Paperclip features into ADF | ADF enhancement |

---

## Updated Recommended Execution Order (Gitea-informed)

```
Immediate (Today):
  1. WS1A: #630 -- CVE fix (disable discord default)           [15 min]
  2. WS2A: Commit synthetic time changes                        [30 min]
  3. WS3A: Kill duplicate ADF processes on bigbox               [10 min]
  4. WS1B: Feature-gate shared_learning                         [10 min]

Day 2:
  5. WS1C: #242 -- Rewrite store.rs with Persistable            [90 min]
  6. WS1D: Build verification and tests                         [30 min]
  7. WS6A: #624 -- Remove orphaned terraphim_settings directory  [15 min]

Day 3:
  8. WS4: #251 -- TLA+ RetryBound violation fix                 [60 min]
  9. WS2B: Fix LinearTracker pre-existing issue                 [30 min]
  10. WS3B: Fix command collision (rename compound-review agent) [20 min]

Day 4:
  11. WS4: #252 -- TLA+ RestForOne UUID sort bug                [45 min]
  12. WS4: #253 -- TLA+ Per-agent restart counter divergence    [45 min]
  13. WS8: #591 -- Automata extract_context function             [60 min]

Day 5:
  14. WS4: #254 -- TLA+ ExactlyOnce dedup pipeline bug          [45 min]
  15. WS4: #255 -- TLA+ Missing escalated state field           [45 min]
  16. WS3C: Wire SharedLearningStore into orchestrator           [60 min]

Week 2:
  17. WS4: #257 -- TLA+ Retry task race condition               [45 min]
  18. WS4: #259 -- TLA+ Model agent events in spec              [45 min]
  19. WS7A: #626 -- EDM scanner: terraphim_negative_contribution [90 min]
  20. WS5A: Wire evaluate() into reconciliation loop            [30 min]
```

---

## Open Questions for Alex

1. **Discord adapter**: Disable feature default permanently, or re-enable after serenity 0.13 release?
2. **TLA+ bug priority**: Start with #251 (RetryBound), or prefer a different ordering?
3. **terraphim_multi_agent uncommitted changes**: Commit or discard?
4. **Old stashes**: Review and cherry-pick, or bulk delete?
5. **Orchestrator Phase 2**: Proceed now, or wait until Phase 1 is battle-tested on BigBox?
6. **Automata mention rewrite (PR #185)**: File new issue for u64-based approach, or defer indefinitely?
7. **EDM Scanner (#625)**: Start #626 now, or wait until blocked sub-issues (#627-#629) are unblocked?
8. **Security checklist #630**: Confirms rustls-webpki CVE -- validate against WS1A approach?

---

## Success Criteria

| Criterion | Measurement |
|-----------|-------------|
| CVE eliminated | `cargo tree -p terraphim_tinyclaw` shows no serenity by default |
| shared_learning works | `cargo test -p terraphim_agent --features shared-learning` passes |
| Synthetic time committed | All 201 tests pass, changes pushed to main |
| ADF stable | Single process, no command collisions, auto-file works |
| TLA+ bugs fixed | RetryBound + race conditions resolved, verified with model checker |
| CI green | All workflow checks pass on main |
| Gitea issues closed | #630, #242, #251-#257, #624, #591 closed after implementation |
