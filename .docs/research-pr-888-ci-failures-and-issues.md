# Research Document: PR #888 CI Failures and Issues (Bundled Features: #1875 + #1873 + #1862)

**Status**: Draft
**Author**: Research Specialist (Grok subagent, disciplined-research skill)
**Date**: 2026-05-27
**Branch**: task/1875-adf-ctl-local-direct-dispatch (workspace state); github PR 888 (consolidation of three feature branches)
**Reviewers**: [To be assigned]
**Gitea Tracking Issue**: #1879

## Executive Summary

PR #888 on GitHub consolidates three features (adf-ctl local direct dispatch via Unix socket #1875, FffIndexer migration from ripgrep #1873, and local `.terraphim/` project config priority #1862) into a single 65-file / ~7.7k LOC change (net +5.8k insertions on the task/1875 branch vs main). CI fails on three jobs on self-hosted bigbox runners: Rust build + test, Performance Benchmarks, and Firecracker VM lifecycle proof. A structural review (confidence 2/5) flagged two P1 risks: (1) direct dispatch path emits `WebhookDispatch::SpawnAgent` with hardcoded `issue_number:0` / `comment_id:0` (structural API contract hazard for all downstream consumers including dedup, posting, and Gitea trackers); (2) FffIndexer lacks fully demonstrated TerraphimGraph relevance parity at review time (though dedicated tests now pass). One concrete, reproducible-in-principle failure mode is `test_orchestrator_compound_review_integration` (crates/terraphim_orchestrator/tests/orchestrator_tests.rs), which fails with git worktree creation errors ("fatal: failed to read .git/worktrees/sentinel-.../commondir: Success") despite the test's own comment claiming empty groups avoid worktree ops. The test always creates a worktree (code in compound.rs:334 unconditionally calls create_worktree before checking active_groups). The change surface includes new UDS listener (direct_dispatch.rs), expanded adf-ctl CLI, OrchestratorConfig wiring, and .terraphim/ artefacts. Blast radius is high due to bundling and new privileged local dispatch path.

## Essential Questions Check

| Question | Answer | Evidence |
|----------|--------|----------|
| Energizing? | Yes | Unblocks reliable low-latency local ADF agent dispatch on bigbox (core to ADF operator workflows and self-healing loops); investigating flakiness directly improves CI signal and ADF reliability north-star (5+ overnight agents). |
| Leverages strengths? | Yes | Deep expertise in Rust async (tokio mpsc, UnixListener, worktree invariants), concurrent systems, git integration, and test fragility analysis in self-hosted CI environments with no mocks. |
| Meets real need? | Yes | Validated by open gitea PR 1876 (refs #1875), structural review P1s, repeated CI failures blocking merge, and prior ADF issues (#1422 worktree hygiene, #1443 context rot, self-healing epic). Unresolved, this blocks three feature tracks and increases production risk for direct dispatch (new security surface) and search migration. |

**Proceed**: Yes (3/3)

## Problem Statement

### Description

The bundled PR introduces:
- A new Unix-domain-socket direct dispatch path (`adf-ctl --local trigger --direct`) that emits `WebhookDispatch::SpawnAgent` with synthetic zero IDs and bypasses webhook/HMAC.
- Replacement of `RipgrepIndexer` with `FffIndexer` (pure-Rust fff-search + optional KG/frecency scorers) in terraphim_middleware.
- `ProjectConfig::load_from_dir()` + discovery for `.terraphim/role-*.json`, thesaurus, and KG paths, made first-priority for CLI tools.
- Supporting wiring in orchestrator (LoopEvent::DirectDispatch, cfg(unix) gating, config defaults, test updates) plus 40+ .terraphim/learnings/ deletions and many design docs.

CI (self-hosted: sccache/SeaweedFS, rch exec, Firecracker fcctl-web) reports failures in:
- Rust build + test (likely including compound review integration test worktree creation races or lock contention).
- Performance Benchmarks (exact step unknown from sampled logs; may be baseline drift or new code impact).
- Firecracker VM lifecycle proof (infra: exit 22 on VM create; health passes).

Local reproduction: the named compound test passes in clean workspace but is documented as fragile to git index locks; CI runners have concurrent git activity (sentinel worktrees, pre-commit, other agents).

### Impact

- **Blocked features**: Three P1-high ADF/epic tracks (#1875 direct dispatch for latency, #1873 search purity/reliability, #1862 local config for project portability) cannot land.
- **Reliability**: New direct dispatch path is privileged (local 0600 socket, no HMAC) yet shares the `WebhookDispatch` type and spawn paths; zero-ID path skips dedup (should_skip_dispatch early-returns false) and may produce divergent Gitea side-effects (no real issue to post to).
- **CI health**: Flaky or failing jobs reduce trust in merges; Firecracker infra failures compound with code changes.
- **Who affected**: ADF operators (bigbox), agent authors (new dispatch semantics), search users (potential relevance regression), downstream crate consumers (config priority shift).
- **If unresolved**: Direct dispatch lands with latent contract violations; FffIndexer parity unknown in full graph roles; local config + learnings deletions risk data model confusion; CI remains red on critical paths.

### Success Criteria

- All three CI jobs green on the consolidated branch/PR (or clear infra-only for Firecracker with mitigation).
- `test_orchestrator_compound_review_integration` (and similar worktree-using tests) pass reliably or are explicitly skipped in CI with documented reason.
- Structural P1s addressed or explicitly accepted with compensating tests (e.g. all `WebhookDispatch` consumers tolerate 0 IDs; FffIndexer vs Ripgrep relevance parity benchmarked on real roles).
- No new test flakes introduced by the 18 source files changed (orchestrator crate only on this branch).
- Research document approved; open questions resolved or deferred with owner.

## Current State Analysis

### Existing Implementation

Before the branch:
- Dispatch only via HTTP webhook (HMAC-verified) → `WebhookDispatch` variants with real `issue_number`/`comment_id` from Gitea payloads.
- Indexing: `RipgrepIndexer` (external process) in terraphim_middleware.
- Config: env + device settings + hardcoded profiles; `.terraphim/config.json` supported in limited places.
- Worktree management: `WorktreeManager` + `WorktreeGuard` (in scope.rs / worktree_guard.rs) with strict drop-order invariants for review swarms; `CompoundReviewWorkflow::run` always creates a per-correlation review-* worktree before spawning (even for 0 active groups); `should_skip_dispatch` special-cases `issue_number == 0`.
- AgentOrchestrator wires compound workflow at startup and calls sweep_stale.

On branch (task/1875):
- New `direct_dispatch.rs` (#[cfg(unix)]): UDS listener at /tmp/adf-ctl.sock (0600), bounded 8KiB reads via take(), agent name allow-list, emits SpawnAgent{0,0}.
- `LoopEvent::DirectDispatch` variant + separate mpsc; `handle_direct_dispatch` does exact-name lookup (no MentionConfig) and calls `spawn_agent` directly.
- OrchestratorConfig gains `direct_dispatch: Option<DirectDispatchConfig>`.
- adf-ctl.rs expanded with --local/--direct, local config discovery.
- Test helpers and 6+ tests updated for new field and direct path (round-trip UDS, oversized reject, disabled agent, etc.).
- .terraphim/adf.toml added (test agents); .gitignore updated for learnings/.
- No changes to compound.rs, scope.rs, worktree_guard.rs, or fff.rs on this branch (Fff and broader local-config crate changes appear pre-existing or in sibling branches merged only on github PR 888).

### Code Locations

| Component | Location | Purpose |
|-----------|----------|---------|
| WebhookDispatch enum + SpawnAgent | crates/terraphim_orchestrator/src/webhook.rs:81 | Core dispatch type; both webhook and direct paths converge here. |
| Direct dispatch listener + command handling | crates/terraphim_orchestrator/src/direct_dispatch.rs:82 (start_...), 149 (handle_connection), 180 (SpawnAgent{0,0}) | New UDS IPC path; constructs zero-ID events. |
| adf-ctl binary (local + direct) | crates/terraphim_orchestrator/src/bin/adf-ctl.rs | CLI entry for --local/--direct; TOML socket path resolution. |
| Orchestrator wiring + handle_direct_dispatch | crates/terraphim_orchestrator/src/lib.rs:3916 (handle_direct...), 3631 (webhook match), 5297 (should_skip with 0 check), 771 (startup) | Event loop integration, cfg(unix) gating, spawn paths. |
| CompoundReviewWorkflow + worktree creation | crates/terraphim_orchestrator/src/compound.rs:273 (run), 334 (unconditional create_worktree), 294 (active_groups filter after get_changed_files) | Always creates review-<uuid> worktree; test uses empty groups expecting no worktree (comment outdated vs code). |
| WorktreeManager / guards | crates/terraphim_orchestrator/src/scope.rs, src/worktree_guard.rs | Creation, sweep_stale, drop-order kill invariants (epic #1567). |
| OrchestratorConfig + direct_dispatch field | crates/terraphim_orchestrator/src/config.rs | New optional socket config; all test initializers updated on branch. |
| FffIndexer (migration target) | crates/terraphim_middleware/src/indexer/fff.rs, tests/fff_indexer.rs | Pure-Rust replacement + KG scorer; 19 tests pass locally (not modified on this branch). |
| ProjectConfig / local .terraphim discovery | crates/terraphim_config (inferred), crates/terraphim_orchestrator/src/project_adf.rs, .terraphim/adf.toml | New first-priority load_from_dir for role-*.json etc.; adf.toml present for testing. |
| Integration test (failing mode) | crates/terraphim_orchestrator/tests/orchestrator_tests.rs:229 (test_orchestrator_compound_review_integration) | Documents avoidance of worktrees via empty groups; code path still hits create_worktree. |
| CI workflows (self-hosted) | .github/workflows/ (Rust Build, Performance Benchmarking, Test Firecracker...) | Run on bigbox with Firecracker fcctl-web; sccache/SeaweedFS caching. |

### Data Flow

1. Traditional: Gitea webhook → HMAC verify → AdfCommandParser → WebhookDispatch::SpawnAgent {real ids} → mpsc → handle_webhook_dispatch → should_skip (if >0) → spawn_agent (with worktree_guard if needed).
2. New direct (unix): adf-ctl --local --direct → JSON over UDS (0600) → listener (bounded read, allow-list validate) → WebhookDispatch::SpawnAgent {0,0} → separate direct mpsc → handle_direct_dispatch (exact name, no mentions) → spawn_agent.
3. Both converge on spawn_agent / active_agents map / output_poster (which may post to issue_number, skipping or erroring on 0).
4. Compound review (orthogonal but test-flaky): run() → get_changed_files → filter visual_only → **always** create_worktree(review-<uuid>) → spawn 0..N agents in worktree → guard drop removes.
5. Config: CLI flags > .terraphim/ (new) > env/device > profiles.

### Integration Points

- Gitea API (via output_poster, trackers for assignee checks, comment posting) — zero IDs bypass real issue operations.
- Unix sockets (new, 0600, no rate limit visible at listener).
- Git worktrees (pre-existing, now stressed by test + potential direct agents?).
- TerraphimGraph / KG scorers (via FffIndexer, not changed here).
- ProjectConfig consumers (terraphim_grep, terraphim_agent, mcp_server — inferred from PR description).
- Firecracker fcctl-web and self-hosted runner images (infra surface for .terraphim/ or socket presence?).

## Constraints

### Technical Constraints
- **Unix-only for new path**: direct_dispatch and adf-ctl --direct gated with #[cfg(unix)]; cross-compile to windows-gnu must succeed (PR added gates after review finding).
- **No mocks in tests** (per project CLAUDE.md): all integration tests (orchestrator, fff, compound) are real (git worktrees, real UDS, real fff search, real tokio tasks). This amplifies env sensitivity (git locks, socket races, FS state).
- **Self-hosted CI only**: bigbox runners with specific git state (sentinel- worktrees from ADF agents, pre-commit hooks possible, concurrent processes), SeaweedFS/sccache, Firecracker. GitHub-hosted runners not used for these jobs.
- **Async Rust + tokio**: mpsc channels for dispatch (bounded? rate_limiter exists elsewhere), JoinSet for agents, strict drop ordering for guards.
- **Git worktree invariants**: Drop-order (tasks before guard) documented in compound.rs:310; races produce "worktree storm".
- **Large crate**: terraphim_orchestrator ~62k LOC; changes must not regress 788+ lib tests + 26 adf-ctl tests.

### Business Constraints
- Bundled landing: three distinct epics/features (#1875, #1873, #1862) in one PR increases review and rollback risk.
- ADF reliability north-star (Q2): 5+ agents overnight; direct dispatch and stable search are levers; CI red blocks progress.
- Gitea tracking mandatory: all tasks via gtr; commits required.

### Non-Functional Requirements
| Requirement | Target | Current (observed) |
|-------------|--------|--------------------|
| Direct dispatch latency | <10ms local (bypass HTTP) | New; UDS + channel should meet but no benchmark in PR |
| Test reliability (compound) | 0 flakes in CI | Fails under git index lock / concurrent worktree ops |
| FffIndexer relevance parity | No regression vs Ripgrep on TerraphimGraph roles | 19 unit tests pass; full end-to-end graph scoring parity not demonstrated in PR review |
| Socket security | 0600, bounded 8KiB, allow-list only | Implemented post-review fixes |
| Cross-platform build | windows-gnu clean | Gated post-P1 finding |

## Vital Few (Essentialism)

### Essential Constraints (Max 3)

| Constraint | Why It's Vital | Evidence |
|------------|----------------|----------|
| Must not regress WebhookDispatch contract for zero-ID callers | Direct dispatch is a second producer of the same enum; all 10+ consumers (dedup, posting, trackers, pr_dispatch, compound ack) must tolerate 0 without panic or silent wrong behaviour (e.g. posting to issue 0, infinite retry, missed dedup). | Code at lib.rs:5298 (early return only for skip), 3700 (post_raw on issue_number), direct_dispatch.rs:180; structural review P1. |
| Compound review worktree creation must be conditional or test must not claim avoidance | Test explicitly uses empty groups "to avoid git worktree creation" yet code path always creates; this is the exact failure mode seen locally/CI. | compound.rs:308 (comment "Create worktree for this review"), 334 (unconditional), 349 (loop only over active), test comment lines 225-227. |
| Bundled 65-file change must not land without per-feature green CI isolation | One feature's infra/test fragility (worktree, Firecracker) masks or is masked by another's (Fff parity, direct dispatch side-effects). | PR body, 18 source files touched on branch + many docs/learnings; 3 failing CI dimensions. |

### Eliminated from Scope

| Eliminated Item | Why Eliminated |
|-----------------|----------------|
| Full end-to-end relevance benchmark of FffIndexer vs Ripgrep on all production roles + TerraphimGraph | Not in current branch diff (fff.rs unchanged); dedicated 19 tests green; would require data fixtures and is P1 for #1873 but separate spike. |
| Performance Benchmarks root cause (exact regression or drift) | Logs sampling yielded no clear signal in first 50k; job is infra-heavy (self-hosted); time-box prioritises Rust test + structural P1s blocking the PR. |
| Firecracker VM lifecycle (exit 22) deep dive | Explicitly infra (fcctl-web health ok, curl POST fails); pre-existing per prior research snippet; not introduced by code delta on branch. |
| Complete audit of all 11 untested files from sentrux scan in orchestrator | 84% coverage reported; focus on new direct_dispatch + worktree paths only. |
| Desktop (Svelte) / WASM / other crates impact of local config | PR description claims integration in terraphim_agent/mcp/grep; no changes visible in this branch's crates/ diff outside orchestrator. |

## Dependencies

### Internal Dependencies

| Dependency | Impact | Risk |
|------------|--------|------|
| terraphim_orchestrator::webhook::WebhookDispatch | All dispatch paths (webhook, direct, adf_commands) and handlers in lib.rs converge; zero-ID is now live second path. | High — contract change without version or newtype. |
| WorktreeManager + guards (scope/worktree_guard) | Used by compound workflow (always), agent isolation (conditional); sweep at startup. | Medium — pre-existing race surface now hit by test + potential new direct agents. |
| OutputPoster / Gitea trackers | Consume issue_number for post_raw, assignee checks. | High for direct path (0 may error or target wrong issue). |
| ProjectConfig (terraphim_config crate) | New discovery logic interacts with .terraphim/learnings/ deletions in same PR. | Medium — under-specified per structural review P2. |
| FffIndexer + TerraphimGraph scorer | Search path for roles using graph; KG path helpers in new config. | Medium — parity not fully evidenced at review time. |

### External Dependencies

| Dependency | Version | Risk | Alternative |
|------------|---------|------|-------------|
| tokio (UnixListener, mpsc, process) | (workspace) | Low — well exercised. | N/A |
| git (worktree create/remove, index lock) | System on bigbox | High — source of flakes; no control in code. | libgit2 (but would be large refactor). |
| fff-search (pure-Rust) | New in #1873 | Medium — replaces ripgrep; performance/relevance. | Keep ripgrep (status quo). |
| Firecracker / fcctl-web | Self-hosted | High — infra blocker separate from code. | Document as known and gate on runner health. |

## Risks and Unknowns

### Known Risks

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Zero-ID SpawnAgent causes divergent behaviour (no dedup, post to issue 0, audit gaps) in unexercised code paths | High (already in tests with 0) | High (silent wrong dispatch or Gitea pollution) | should_skip already guards 0; add explicit tests asserting no Gitea side-effects for direct; consider new variant or marker type. |
| Compound test (and similar) flakes on CI due to unconditional worktree create + git lock contention | High (documented in test comment; reproduced in principle) | High (blocks Rust CI) | Make worktree creation conditional on active_groups.len() > 0 (or move after filter); or mark test #[ignore] in CI with reason; improve WorktreeManager error handling for "commondir" races. |
| FffIndexer relevance regression on TerraphimGraph roles not caught by unit tests | Medium | High (search quality for pilot roles) | PR review P1; add parity spike or A/B in nightwatch before full migration. |
| New UDS socket path has no rate limiting / backpressure at listener (P2) | Medium (under load from adf-ctl scripts) | Medium (backpressure on main loop or OOM) | Listener uses clone of tx; orchestrator rate_limiter is downstream. |
| .terraphim/learnings/ mass delete + new ProjectConfig discovery interaction | Medium | Medium (lost agent memory or config confusion) | Deletions presented as housekeeping; .gitignore now present. |
| 65-file bundle masks which feature caused which CI failure | High | High (unsafe merge) | Land features separately or require per-feature CI isolation in PR description. |

### Open Questions

1. Why does the compound test's own comment ("uses empty groups to avoid...") contradict the code (worktree created unconditionally before the active_groups loop)? — Owner: author of compound.rs / epic #1567.
2. Are there other consumers of WebhookDispatch::SpawnAgent (outside lib.rs) that branch on issue_number > 0 or perform Gitea writes without guarding 0? (e.g. in pr_dispatch.rs, meta_coordinator, external crates) — Required: full rg across workspace + call-graph.
3. What exactly failed in the Performance Benchmarks job (run 26524614646)? Exact step and metric drift? Is it caused by new code, sccache invalidation, or baseline staleness? — Required: full log analysis or re-run with verbose.
4. Does the Firecracker failure (exit 22) correlate with any file change (new .terraphim/adf.toml, socket expectations in runner image, git worktree pollution)? — Owner: DevOps / bigbox maintainers.
5. On the github PR 888 consolidation (vs this 1875 branch), what additional files from #1873/#1862 are present that could affect Fff parity or config loading in CI? — Clarification: PR body vs actual merge state.
6. Is the UDS listener spawned in all modes (including test Configs)? Does direct_dispatch: None prevent listener start cleanly? — Evidence in lib.rs startup.

### Assumptions Explicitly Stated

| Assumption | Basis | Risk if Wrong | Verified? |
|------------|-------|---------------|-----------|
| The workspace branch (task/1875) accurately represents the code delta causing the github PR 888 CI failures | gitea PR 1876 matches branch name; github PR title refs #1875 as primary; CI checks point to this head | Failures may be from merged sibling branches (#1873 Fff, #1862 config) not visible in local diff | Partial — only orchestrator crate touched here; bundle may include more. |
| The compound test failure mode ("sentinel- commondir") is the primary Rust CI blocker | User-provided key fact; matches error in OrchestratorError::CompoundReviewFailed; test is integration and git-touching | Other tests (e.g. role_switching_persistence from prior research) or windows cross or clippy are the actual failure | Partial — local run passes; CI env differs (locks, concurrency). |
| Zero-ID path is exercised only via direct dispatch (no other producers) | Code search showed construction only in direct_dispatch.rs:180 and webhook path with real ids | Another path (e.g. synthetic in tests or adf_commands) emits 0 unintentionally | Yes for this branch (tests now pass 0 explicitly). |
| FffIndexer changes are not the cause of current branch CI (Rust/perf/Firecracker) | No fff.rs in git diff vs main; 19 tests green locally | On github PR 888 merge commit they are present and cause perf regression or build flag issues | Yes for local analysis; unknown for bundled github state. |
| Firecracker and perf failures are pre-existing infra (not code-induced) | Prior research snippet, health check passes, sampled logs no obvious code signal | New socket/config files or test pollution affect runner provisioning | Partial — .gitignore updated; no direct evidence either way. |

### Multiple Interpretations Considered

| Interpretation | Implications | Why Chosen/Rejected |
|----------------|--------------|---------------------|
| The Rust build+test failure is solely the compound worktree test | Focus remediation on compound.rs conditional creation + test docs | Chosen as concrete, matches user key fact and code contradiction; other tests (role_switching) mentioned in prior docs but not user-provided. |
| The failure is a broader class of git-touching integration tests under CI lock | Requires sweeping all worktree users (agent isolation, review, sweep) for robustness | Considered; eliminated per vital few (focus on the documented test that claims avoidance but doesn't). |
| FffIndexer parity gap is the "vital" CI/perf blocker | Would explain Performance Benchmarks failure | Rejected — no fff diff on branch; tests green; perf job may be unrelated (baseline or infra). |
| Direct dispatch zero-IDs are harmless because should_skip guards 0 and direct handler ignores ids | True for spawn path today, but false for any future consumer or poster.post_raw(0, ...) | Considered; rejected because structural review + poster usage at 3700/3858 etc. show real risk; API is shared. |

## Research Findings

### Key Insights

1. **Test comment vs implementation mismatch is the root of the reported failure**: The orchestrator_tests.rs:225 comment states empty groups "avoid git worktree creation which fails when the git index is locked". The implementation in compound.rs:308-340 creates the worktree *before* the active_groups filter and spawn loop. With groups:[] the test still exercises create_worktree + guard drop, hitting the exact error the comment claims to sidestep. This is a documentation/code desync, not merely env.
2. **Direct dispatch correctly isolates some concerns (LoopEvent, exact-name lookup, no MentionConfig) but re-uses the wrong abstraction**: Emitting SpawnAgent{0,0} re-uses the Gitea-tied type for a non-Gitea trigger. The new handle_direct_dispatch and should_skip guard(0) mitigate today, but the type system does not enforce the distinction. Downstream (poster, trackers, audit) can still see 0.
3. **CI flakiness is amplified by "no mocks" + self-hosted + git**: Worktree ops, UDS bind, real search, real gitea in tests are all sensitive to runner state (concurrent sentinel worktrees, index locks, socket residue). The PR added robustness (cfg(unix), bounded reads, stale socket checks, oversized reject test) but the pre-existing compound path was not hardened.
4. **Bundle size is the meta-risk**: 18 source files + 40+ learnings deletions + docs in one PR means any single job failure (even infra) blocks all three features. Fff and local-config changes not visible on this branch imply the github PR 888 merge base or commits differ.
5. **Sentrux scan**: 84% coverage (58/69 source files tested) in orchestrator; 11 untested files remain. New direct_dispatch.rs has good dedicated tests (per PR), but wiring in lib.rs and project_adf.rs changes increase untested surface.
6. **Local reproduction of named test now passes**: In clean workspace (no pre-commit, no concurrent git), the test succeeds quickly. Failure is env-specific (CI-only or locked-index), confirming fragility not determinism bug.

### Relevant Prior Art

- Prior research docs in .docs/ (research-pr-888-fixes.md, research-adf-*-*.md, design-adf-*-*.md): document similar worktree hygiene issue (#1422 "40 stale worktrees"), self-healing steps, and earlier PR 888 fix attempts (role_switching_persistence flake + Firecracker exit 22).
- Epic #1567 / issues #1569/#1570: Drop-order invariants and sweep_stale for review worktrees (the exact mechanism stressed by the failing test).
- Gitea #1422: Automated worktree pruning proposal (closed); still relevant as root cause of "sentinel-" residue.
- Structural PR review embedded in github PR 888 description: source of P1/P2 findings and confidence 2/5.

### Technical Spikes Needed

| Spike | Purpose | Estimated Effort |
|-------|---------|------------------|
| Audit all WebhookDispatch::SpawnAgent construction sites + call sites of post_raw / fetch_issue_assignees | Prove no other 0-ID emitters and that all Gitea-using paths guard 0 | 2-4 hours |
| Make worktree creation in CompoundReviewWorkflow conditional on !active_groups.is_empty(); update test comment + add lock-contention simulation | Eliminate the documented contradiction and flake source | 1-2 hours + test run on bigbox |
| Re-run Performance Benchmarking job with verbose output + capture full logs for the failing step | Identify if new code, cache, or baseline | 1 hour (infra access) |
| Cross-check github PR 888 merge commit files vs this branch diff | Confirm exact delta causing the reported CI state | 30 min |

## Recommendations

### Proceed/No-Proceed

**Proceed with Phase 2 (Design) only after** the three open questions on test contradiction, zero-ID consumers, and exact CI log causes are answered (or explicitly deferred with owners and compensating acceptance tests). Do not land the bundle until Rust build+test is green in the exact PR state and structural P1s have test evidence.

### Scope Recommendations

- Split the github PR 888 into three separate PRs (one per feature) with independent CI runs. This directly addresses the vital "bundle masks failure" constraint.
- Treat the compound test failure as a pre-existing test-quality debt (exacerbated by no-mocks policy) rather than a new regression from direct dispatch.
- For FffIndexer (#1873): require a dedicated relevance parity report (even if not in this branch) before migration lands.

### Risk Mitigation Recommendations

- Add an explicit `DirectSpawn` variant (or marker) to WebhookDispatch (or a separate enum) so the type system distinguishes "Gitea-backed" vs "local operator" dispatches. This eliminates the P1 contract risk at source.
- Gate the unconditional worktree creation in compound review behind the same active_groups filter the test author intended; document the invariant in code, not just comments.
- Add a CI step that runs `git worktree prune` + counts before/after test suites; fail if >N stale review-* entries appear.
- For the new UDS path: add a simple connection limit or use a bounded channel with explicit backpressure shedding at the listener.

## Next Steps

If approved:
1. Resolve open questions 1-3 (owners assigned via gtr on #1879); update this document with answers.
2. Land minimal hardening PRs: (a) conditional worktree in compound.rs + test fix, (b) zero-ID consumer audit + tests, (c) split or rebase the bundle.
3. Re-run full targeted CI matrix (Rust build+test with the compound test under simulated lock; perf; Firecracker health) on the cleaned branch.
4. Request disciplined-quality-evaluation skill on this research document before Phase 2 design.
5. Update gitea issue #1879 and related epics (#1875, #1873, #1862, #1567) with findings link; commit this document.

## Appendix

### Reference Materials
- GitHub PR: https://github.com/terraphim/terraphim-ai/pull/888 (structural review, commit history, CI checks)
- Gitea PR: https://git.terraphim.cloud/terraphim/terraphim-ai/pulls/1876 (direct dispatch, refs #1875)
- Gitea issue for this research: https://git.terraphim.cloud/terraphim/terraphim-ai/issues/1879
- Prior docs: .docs/research-pr-888-fixes.md, .docs/design-adf-ctl-direct-dispatch.md, .docs/research-adf-direct-dispatch-*.md
- Related gitea issues: #1422 (worktree hygiene), #1443 (context rot), #1807+ (self-healing), #1812 (MetaCoordinator)
- SKILL.md: /home/alex/.claude/skills/disciplined-research/SKILL.md (template source)
- Sentrux scan output (orchestrator): 84% coverage, 11 untested files

### Code Snippets

**Critical mismatch (test intent vs code):**
```rust
// tests/orchestrator_tests.rs:225
/// Uses empty groups to avoid git worktree creation which fails when the git
/// index is locked (e.g. during pre-commit hooks).
let swarm_config = SwarmConfig { groups: vec![], ... };
let result = workflow.run(...).await.unwrap();  // still hits worktree
```

```rust
// compound.rs:308 (after filter)
let active_groups = ...filter...;
let guard = self.worktree_manager.create_worktree(&worktree_name, git_ref).await?;  // unconditional
for group in active_groups { ... }
```

**P1 zero-ID emission (direct path):**
```rust
// direct_dispatch.rs:180
let dispatch = WebhookDispatch::SpawnAgent {
    agent_name: cmd.agent,
    detected_project: None,
    issue_number: 0,
    comment_id: 0,
    context: cmd.context.unwrap_or_default(),
};
```

**Guard that acknowledges 0 (but only for skip):**
```rust
// lib.rs:5298
async fn should_skip_dispatch(&self, agent_name: &str, issue_number: u64) -> bool {
    if issue_number == 0 { return false; }
    ...
}
```

### Code Location Map (Standalone)

See "Code Locations" table above. Full changed file list (current branch vs main):
- 14 Rust source files (all in crates/terraphim_orchestrator/)
- 1 .toml (local test config)
- 1 .gitignore
- ~40 .terraphim/learnings/* (deletions + .gitignore)
- ~15 .docs/*.md (design/research artefacts)

### Risk Register (Standalone, Prioritised)

1. **P1 - API Contract Violation (zero IDs)**: Likelihood High, Impact High, Owner: PR author + reviewers. Evidence: structural review + code at direct_dispatch.rs:180 + all poster sites.
2. **P1 - Test/Code Desync causing CI flake (worktree)**: Likelihood High in CI, Impact High (blocks Rust job). Owner: compound.rs maintainers. Evidence: test comment 225 vs compound.rs:334.
3. **P2 - Bundle Blast Radius**: Likelihood High, Impact High. Owner: release process. Evidence: 65 files, 3 features, 3 failing dimensions.
4. **P2 - Fff Relevance Regression**: Likelihood Medium, Impact High for search users. Owner: #1873. Evidence: PR review P1 (tests now green).
5. **P2 - UDS Rate Limit / DoS Surface**: Likelihood Medium, Impact Medium. Evidence: review P2; listener loop with unbounded accepts.
6. **P2 - Config + Learnings Interaction**: Likelihood Medium, Impact Medium. Evidence: review P2 + mass deletions in PR.
7. **Infra - Firecracker / Perf CI**: Likelihood (for this PR) Medium, Impact High (blocks). Owner: DevOps. Evidence: health vs create failure; no clear code signal.

**End of Research Document**

*Generated 2026-05-27 21:05 BST following disciplined-research/SKILL.md template. All assumptions, interpretations, and constraints surfaced. No design or implementation performed.*