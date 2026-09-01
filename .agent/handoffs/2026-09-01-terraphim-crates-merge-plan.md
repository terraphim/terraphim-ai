# Handover: terraphim-* PR backlog merge plan and batch-1 execution

**Date**: 2026-09-01
**UTC Time**: 22:36:28 UTC
**Change Slug**: terraphim-crates-merge-plan
**Branch**: `plan/terraphim-crates-merge-2026-09-01` (terraphim-ai), PR #3313
**Session File**: none (session began from `progress_history.txt` recovery, not `/session-start`)

## Progress Summary

- Completed work:
  - Recovered the stalled merge plan (written by a prior session, never committed), committed it on a fresh branch from `origin/main`, opened PR #3313, and commented on meta-incident #2691.
  - Owner resolved all four open questions (reviewer for #3291 = this agent; force-push permitted behind verification/validation/structural-review gates; tinyclaw review "now"; skills-server is a valid product). Recorded in plan section 5.4.
  - Batch 1 executed to review-complete with four parallel subagents; execution log in plan section 5.5.
  - Four PRs closed with evidence; six PRs reviewed and merge-ready; one on hold; one awaiting an owner decision.
- Current implementation state: all code work is pushed to the respective PR branches. Nothing is merged.
- Working vs blocked: everything up to the merge click works. Merging is blocked by the Claude Code permission classifier in this session (`gtr merge-pull` and MCP `merge_pull` both denied).

## Artifact Index

- Research + Design (single artefact): `docs/handovers/2026-09-01-terraphim-crates-merge-plan.md`
  - Section 1 research, section 2 design, section 3 permissions, section 5 resumption plan, 5.4 owner decisions, 5.5 execution log
  - PR: https://git.terraphim.cloud/terraphim/terraphim-ai/pulls/3313
- Decisions: plan section 2.4 (design decisions), 5.4 (owner decisions), 5.5 (plan correction on #3159)
- Verification evidence (per PR, posted as Gitea comments):
  - terraphim-ai #3291: round-2 HOLD review, "Round-2 remediation" (comment 76713), round-3 GO review at head `bc790c9a6`
  - terraphim-ai #3221: "Rebase and pre-merge fixes" at head `c3877c403`
  - terraphim-clients #84: "Rebased and provisioning fixed", structural review (4/5), "Runner green", addendum GO at head `70410ba`; runner run 29594
  - terraphim-ai #3159, terraphim-agents #117/#118/#135: close rationale comments
  - terraphim-clients #146: divergence finding comment
  - terraphim-service #12: hold rationale comment
- Operational continuity: this file; memory notes `project_pr_backlog_merge_plan_2026-09.md`, `project_terraphim_clients_main_divergence.md` in the Claude project memory
- Scratchpad (session-local, disposable): worktrees `wt3291`, `wt3221`, `wt-terraphim-clients-testgate`, `wt-terraphim-ai-testgate`, `wt-terraphim-service-testgate`, `wt-terraphim-kg-agents-testgate`, `wt-agents-main`, `wt-clients-main`; gate logs `gates-clients*.log`, `run29586.log`, `run29594.log`

## Current State

- Known-good (reviewed, gates green, mergeable):
  1. terraphim-ai #3273 (opencode v3 assistant-text extraction; unblocks fleet-wide gates)
  2. terraphim-ai #3291 (P0 qualified gate routing; head `bc790c9a6`; merge after #3273)
  3. terraphim-ai #3308 (sccache to kache)
  4. terraphim-kg-agents #5 (all-targets test gate; runner run 27009 green)
  5. terraphim-clients #84 (all-targets test gate; head `70410ba`; runner run 29594 green, 2231 tests)
  6. terraphim-ai #3221 (tinyclaw integration; head `c3877c403`; WhatsApp 503-on-dispatch-failure and scrub-list fixes; 0 failures across 23 test binaries). The owner-requested Codex `gpt-5.5` review via `pi` hung three times and never completed; my seam review of the eight conflict resolutions is clean.
- Closed: terraphim-agents #117, #118, #135 (superseded by main via e1742d3); terraphim-ai #3159 (declined per #3222, NOT superseded).
- Partially working / on hold:
  - terraphim-service #12: correct change, but service main is red (runner registry config error run 28758, pre-existing clippy lint at `crates/haystack_jmap/src/lib.rs:188`, 12 fixture-dependent middleware tests).
- Risky or unresolved:
  - terraphim-clients has two diverged mains (GitHub vs Gitea, split at ae83043 on 2026-08-10); hybrid scoring and 16 other commits exist only on GitHub. PR #146 is really GitHub main + one fmt commit. Owner decision required before the R2 distribution stack (clients #70-76, Gitea PRs) merges.
  - `~/.profile` `GITEA_TOKEN` (`de907d...`) is rejected by Gitea; the working token is `op://Terraphim/gitea-token/credential` (`f1acff...`). Any agent that sources the profile still 401s.
  - tinyclaw Teams JWT claims struct requires camelCase `serviceUrl`; Bot Framework issues lowercase `serviceurl`. Out of scope by owner decision; not filed.
  - terraphim-clients kg_ranking tests and `test_end_to_end_server_workflow` fail against a `terraphim_server` built from current terraphim-ai main ("connection closed before message completed"); pass on pinned v1.21.3. Whoever bumps the pin will hit it.
  - #84 carries a cherry-picked production behaviour change (P2): `TERRAPHIM_DEFAULT_DATA_PATH` now overrides project-local `.terraphim/learnings/` precedence (`crates/terraphim_agent/src/learnings/mod.rs:141`).

## Resume Procedure

1. Token: `export GITEA_TOKEN=$(op read "op://Terraphim/gitea-token/credential")` (approve the 1Password prompt; the profile value is stale). Verify: `curl -s -H "Authorization: token $GITEA_TOKEN" https://git.terraphim.cloud/api/v1/user | head -c 80`.
2. Branch: `cd terraphim-ai && git fetch origin && git checkout plan/terraphim-crates-merge-2026-09-01 && git log -3 --oneline` (expect `6acb9c0c1` on top).
3. Read plan sections 5.4 and 5.5 for decisions and the execution log.
4. Confirm the six merge-ready PRs are still mergeable: `for n in 3273 3291 3308 3221; do gtr list-pulls --owner terraphim --repo terraphim-ai --state open | python3 -c "import json,sys; [print(p['number'],p['mergeable'],p['head']['sha'][:9]) for p in json.load(sys.stdin) if p['number']==$n]"; done` (default page size is 20; use the API with `limit=50` for full listings).
5. Merge permission: either the owner runs `gtr merge-pull --owner terraphim --repo <repo> --index <n>` directly, or adds `Bash(gtr merge-pull:*)` to the Claude Code allow list. Do not attempt workarounds.
6. Merge order: 3273, 3291, 3308, kg-agents 5, clients 84, 3221. After 3221 lands, close 3215/3216/3218 as subsumed (comment first, then `gtr close-issue`).
7. Post-merge acceptance for #3291 (from its body): one exact-head smoke on terraphim-llm-proxy#38 must produce only canonical project-scoped gate contexts, no generic OpenCode transcript, no `digital-twins` routing.

## Next Steps

1. Immediate: obtain merge permission and land the six PRs in order; then append "Section 6. Outcomes" to the plan (plan step 8).
2. Follow-up: owner decides canonical terraphim-clients main (#146); then start Batch 2 (terraphim-ai #3112, clients #70-76 in strict order, clients #22 security after rebase).
3. Deferred: file the Teams `serviceUrl` claim finding; file the v1.21.3 pin regression; fix service main (registry config, clippy, fixtures) so #12 can merge; revisit `TERRAPHIM_DEFAULT_DATA_PATH` precedence.

## Open Questions and Risks

- Which main is canonical for terraphim-clients (GitHub or Gitea)?
- Does the owner still require a completed Codex review for #3221, or does the in-session structural review satisfy the "different-model" gate?
- Is the R2 distribution stack still valid against Gitea main given the divergence?

## Notes for the Next Session

- `gtr list-pulls` defaults to 20 results; the plan inventory was built from the API with `limit=50`.
- `op read` times out unless the 1Password desktop prompt is approved; once read, the token was cached in the session scratchpad (`.gitea_token`, mode 600) and passed to subagents by path. That file does not survive the session.
- In terraphim-clients and terraphim-agents clones, `origin` is GitHub and `gitea` is Gitea. Always check `gitea/main` for Gitea PR work.
- The runner exports `RUST_LOG=info` and has a populated `~/.config/terraphim`; tests that spawn binaries must use `support::cli_test_env::apply_hermetic_env` or they pass locally and fail on bigbox.
- The repo pre-commit hook runs `cargo test --workspace --lib`; the `terraphim_rlm` docker tests need OrbStack awake (`docker info` wakes it). `--no-verify` is classifier-blocked.
- `pi -p --provider openai-codex --model gpt-5.5` hung three times on the 58-file PR; `@file` references must be inside pi's cwd.
