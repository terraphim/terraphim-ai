# Triage: Conflicting and Unknown Mergeability PRs

**Status**: Awaiting rebase / author input / stale closure
**Canonical Path**: `docs/plans/triage-conflicting-and-unknown-prs.md`
**Author**: OpenCode
**Date**: 2026-07-06

## Executive Summary

After processing the ready dependency PRs, 22 GitHub PRs remain in `CONFLICTING` or `UNKNOWN` mergeability states. Many have failing historical CI checks and have not been updated since late June 2026. This document groups them by recommended action. Full disciplined research/design should be deferred until each PR is rebased and its mergeability is restored.

## Categories

### 1. Conflicting: needs rebase before any review

These PRs cannot be merged because they conflict with `main`. Authors should rebase and re-run CI before research/design proceeds.

| PR | Title | Recommended Action |
|----|-------|--------------------|
| #934 | Fix #2998: deterministic /health JSON body + shared readiness helper | Rebase on main; verify health endpoint contract still matches current server |
| #932 | fix(lint): resolve clippy --all-targets -D warnings gate (4 findings) | Rebase; likely superseded by later lint fixes on main |
| #928 | Fix #2879: remove dangling meta_coordinator module declaration | Rebase; small cleanup, should be quick once mergeable |
| #927 | Fix #2770: add rust-version = 1.85 to workspace package and all crates | Rebase; CI was historically green but branch is now behind |
| #906 | Fix #1295+#2226: stub fcctl_core private dep; add --all-features CI gate | Rebase; has multiple failing checks, needs careful rebuild |
| #905 | docs: add item-level rustdoc to 7 worst-offender crates Refs #2137 | Rebase; likely overlaps with #891 doc gaps sweep |
| #904 | Fix #1990: wire suggest_learnings (BM25) into pre-tool-use hook | Rebase; feature work, needs design review after rebase |

### 2. Unknown mergeability: verify against current main

GitHub could not determine mergeability for these. They may be behind `main` or have CI noise. They should be rebased and re-evaluated.

| PR | Title | Recommended Action |
|----|-------|--------------------|
| #903 | Fix #2160: sanitise agent_id and learning_id paths in markdown_store (CWE-22) | Security-related; rebase and prioritise review |
| #902 | Fix #2039: add serde(default) to trigger_descriptions and pinned_node_ids | Rebase; small serde fix |
| #901 | chore(docker)(deps): bump rust from 1.95-slim-bookworm to 1.96-slim-bookworm in /docker | Rebase; check toolchain compatibility |
| #900 | Fix #2131: strip anyhow chain from robot JSON error output | Rebase; error-handling improvement |
| #899 | fix(agent): classify HTTP status errors as ErrorGeneral not ErrorNetwork | Rebase; behavioural change needs tests |
| #898 | ci(bench): port performance-benchmarking to Gitea native-ci (sccache), scope GitHub to manual | Rebase; CI infrastructure change |
| #897 | Fix #2049: guard promote_to_l2 against L0 -- error instead of silent no-op | Rebase; shared learning logic |
| #896 | Fix #2035: add missing doc comments to 16 crates (doc gaps 2026-06-03) | Rebase; likely overlaps with #891 |
| #895 | fix(fmt): cargo fmt terraphim_orchestrator | Rebase; formatting-only, should be trivial |
| #894 | Fix #2046: add missing promote_to_l1 steps in SharedLearningStore tests | Rebase; test-only |
| #893 | fix(tests): replace #[test] with #[tokio::test] on 13 async fns | Rebase; test-only |
| #892 | test(sessions): add unit tests for search_by_concept and find_related_sessions | Rebase; test-only |
| #891 | Fix #1979: documentation gap sweep -- add missing_docs lint + fill ~1800 gaps | Rebase; large doc sweep, likely conflicts with many other doc PRs |
| #890 | Fix #1985: embed no_kg fixture via include_str to survive worktree deletion | Rebase; test fixture change |
| #889 | Fix #1458: add cargo deny check as mandatory CI gate (WIG-4) | Rebase; CI/policy change |

### 3. Likely superseded or duplicate

| PR | Title | Reason |
|----|-------|--------|
| #905 | docs: add item-level rustdoc to 7 worst-offender crates | Likely covered by #891 broad doc sweep |
| #896 | Fix #2035: add missing doc comments to 16 crates | Likely covered by #891 broad doc sweep |
| #932 | fix(lint): resolve clippy --all-targets -D warnings gate | May already be fixed on main; verify before rebasing |

## Recommended Next Steps

1. **For all conflicting PRs:** post a standard comment asking authors to rebase on `main` and confirm CI passes.
2. **For security-related PRs (#903, #900, #899):** prioritise rebase and review once mergeable.
3. **For doc-only PRs (#905, #896, #891):** pick one comprehensive approach; consider closing duplicates in favour of #891.
4. **For CI/toolchain PRs (#901, #898, #889):** rebase and run on current main before merging.
5. **Do not start disciplined research/design** for any PR in this list until it is rebased and mergeable.
