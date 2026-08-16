# Handover: Orchestrator re-include + native-ci unblock (PR #3195)

**Date**: 2026-08-16
**UTC Time**: 10:27 UTC
**Change Slug**: orchestrator-workspace-native-ci
**Branch**: `task/3191-orchestrator-workspace` (merged → `main`, branch deleted)
**PR**: #3195 — "fix(build): re-include terraphim_orchestrator and repoint extracted deps"
**Merge commit**: `bfd779589` on `origin/main`

## Progress Summary

- Completed work: fixed two stale golden tests in `crates/terraphim_orchestrator/src/kg_router.rs` to assert **dynamic routing** (tier + resolution) instead of hardcoded `provider == "anthropic"`/model-contains assertions; unblocked `native-ci / build` with three follow-up fixes.
- Current implementation state: **merged to `main`**; `native-ci / build` green on both `push` and `pull_request`.
- Working vs blocked: fully working and merged. No blockers on this change.

### Fixes landed (5 commits: `bb81511dd` → `d8f008ef8`)

1. **Golden tests → dynamic routing** (`3d626e8d8`) — `loads_real_adf_taxonomy_4_tiers` and `e2e_all_adf_agents_route_to_correct_tier` now assert tier + priority + non-empty provider/model, covering fastest (`MiniMax-M2.7-highspeed`), plan (`zai-coding-plan`), thinking (`kimi-k2-thinking`).
2. **Clippy `-D warnings`** (`935350274`) — 6 lints in the re-included orchestrator (`agent_runner.rs`, `dispatcher.rs`, `mentions_impl.rs`, `reconcile_impl.rs`): `map_or(true, ..)` → `is_none_or(..)`, `% m ==/!= 0` → `is_multiple_of(..)`. CI runs clippy with `-D warnings`; the local pre-commit hook does not.
3. **Spawner BrokenPipe race** (`935350274`) — child exiting before reading stdin is now an early exit, not a `SpawnFailed`.
4. **Agent working-dir creation** (`d8f008ef8`) — the real CI blocker. `spawn_agent_with_event` no longer relies on `create_dir_all(agent_log_dir)` to materialise the config `working_dir`; it creates `agent_working_dir` explicitly before spawning.
5. `bb81511dd`/`c95d28894` — original PR commits (re-include orchestrator; rustfmt + `author_is_agent` signature fix).

## Artifact Index

- Operational continuity (this file): `.agent/handoffs/2026-08-16-orchestrator-workspace-native-ci.md`
- Workspace review handover: `~/.openclaw-autoclaw/workspace/.cluster/pr-review/handover-report.md`
- Lessons learned: `lessons-learned.md` (2026-08-16 section)
- Memory (daily): `~/.openclaw-autoclaw/workspace/memory/2026-08-16.md`
- PR review comment: round 3 on PR #3195 (verdict merge, 5/5)

## Current State

- Known-good: PR #3195 merged; `native-ci / build` success on `bfd779589` and `d8f008ef8`.
- Partially working: n/a (change is merged).
- Risky or broken: n/a for this change. The local `main` checkout (`db6b2bc0d`) is **behind** `origin/main` and has unrelated uncommitted changes — reconcile before further work.

## Resume Procedure

If resuming from a fresh clone of `origin/main` (`bfd779589`):

1. `git fetch origin && git checkout main && git pull`
2. `cargo check --workspace` — confirm orchestrator re-includes cleanly.
3. `cargo test -p terraphim_orchestrator --lib` — 879 passed (golden routing tests + spawn tests).
4. `cargo clippy --workspace --all-targets -- -D warnings` — clean.

## Next Steps

1. **Immediate**: none — change is merged.
2. **Follow-up**: the two remaining sweep candidates (#3185 docs-only, #3199 Apple container backend) still need their `native-ci / build` green before merge (previously rejected by branch protection).
3. **Deferred**: the round-1/2 "Cargo.toml dual declaration of `terraphim_types`" finding was confirmed a **false positive** (`[workspace.dependencies]` line 94 vs `[patch.crates-io]` line 105 — both intentional). No action.

## Open Questions and Risks

- The 14 stale PRs (#3114–#3215) are `mergeable=false` against `main` (17 commits ahead of merge-base `d755a1e9`) — still need rebasing.
- Tinyclaw parity chain (#3215→#3218) needs integration, not independent merges.

## Notes for the Next Session

- **"Linux-only" test failures are often environment-only.** `test_handle_direct_dispatch_spawns_agent_without_mentions` "failing on Linux" was actually `/opt/ai-dark-factory/logs` existing on the CI host → `agent_log_dir` resolved to `/opt/ai-dark-factory/logs/agents` (a sibling of the config `working_dir`) → the synthetic `working_dir` was never created → `AgentValidator::validate_working_dir()` rejected the spawn. Reproduce any such "platform" failure by simulating the environment, not by guessing at OS-specific code paths.
- `gtr merge-pull` can return a client-side `context deadline exceeded` even when the merge **succeeds** server-side. Verify via the PR API (`merged: true` / `merged_at`), not the CLI exit code.
- Gitea credentials: `GITEA_URL`/`GITEA_TOKEN` live in `~/.zshrc`; `gtr` is `~/.local/bin/gtr` (plain `gtr` is macOS `tr`).
