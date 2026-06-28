# Handover: ADF gate hardening + repo-local autonomous agents

**Date:** 2026-06-04
**Focus:** Gitea epic #1910 follow-ups -- harden the ADF pre-merge gate and enable
ADF to autonomously maintain the polyrepos via repo-local agent config.
**Orchestrator:** bigbox `adf-orchestrator.service` -- healthy/active on a binary
built from terraphim-agents `main` (includes #2174, #2199, #2203).

## 1. Progress summary

### Shipped, merged, gated, deployed
| Issue | What | Where |
|-------|------|-------|
| #2174 | AutoMerge re-fetches required commit statuses + re-runs reconcile_pr_gate immediately before merge_pr (closes a same-SHA status-regression race). Closed. | terraphim-agents PR #5 |
| #2175 | Verdict-agent gate replicated to ALL 6 polyrepos: `extra_projects` (multi-project agent registration) + multi-repo spawn/status routing (re-point def.project to the PR's project; resolve owner/repo from project gitea) + `synchronized` PR-action webhook fix. Branch protection on all 6 now requires native-ci + adf/pr-reviewer + adf/validation + adf/verification. Closed. | terraphim-agents PRs #6, #8, #11 |
| #2199 | Review/CI agents (pr-reviewer/validator/verifier, build-runner) no longer auto-commit their working tree (they post a status, not code) -- gated on `commit_status_post`. | terraphim-agents PR #12 |
| #2203 | Repo-local `.terraphim/adf.toml` agents loaded into the live fleet via `OrchestratorConfig.project_sources`; `last_cron_fire` re-keyed to `(project, name)` for per-repo cron. | terraphim-agents PR #13 |
| docs | adf rebuild/redeploy procedure added to `docs/adf/operations.md`. | terraphim-ai PR #2201 (merged to main) |

### Canary PROVEN end-to-end (#2203)
On terraphim-agents: issue **#15** -> repo-local `implementation-swarm` fired on its
`:30` cron (per-project) -> branched `task/15-impl` off origin/main -> implemented the
exact test asked for -> opened PR **#16** _on terraphim-agents_ -> 4-check verdict gate
green -> merged (`a5cc3c6`) -> #15 auto-closed. ADF can now autonomously maintain a
polyrepo from that repo's own config.

### Working
- All 6 polyrepos enforce the 4-check verdict gate; coordinator auto-merges only at 5/5
  confidence (else human review -- verified on terraphim-ai PR #2201).
- Repo-local agent loading is live (terraphim-agents `.terraphim/adf.toml` parsed into the
  fleet when its project_source is present).

### Paused / blocked
- **terraphim-agents `implementation-swarm` is PAUSED**: its `[[project_sources]]` entry was
  removed from `/opt/ai-dark-factory/orchestrator.toml` and the orchestrator restarted, per
  "stop for now". It will not fire. `terraphim-agents/.terraphim/adf.toml` remains in the repo
  (dormant). terraphim-agents **#1** (dead-code, 24 suppressions) is open + unblocked +
  untouched -- it was the agent's next pick.

## 2. Technical context

- **Working checkout:** `/tmp/agents-fix` (terraphim-agents). Research/design docs in
  `/tmp/agents-fix/.docs/{research,design}-implementation-swarm-portability.md`,
  `.docs/*-2116-premerge-gate.md`.
- **Repos:** code = terraphim/terraphim-agents (the `terraphim_orchestrator` crate, builds the
  `adf` binary); issue tracker + docs = terraphim/terraphim-ai. Both on git.terraphim.cloud.
- **Key code (terraphim_orchestrator/src):** `config.rs` (ProjectSource + merge_project_sources;
  gitea_owner_repo_for_project; extra_projects validation), `agent_registry.rs` (extra_projects
  registration), `pr_handlers_impl.rs` (ReviewPr dispatch repo-routing + status posts),
  `reconcile_impl.rs` (#2199 auto-commit gate; terminal status routing), `scheduling_impl.rs`
  + `lib.rs` (last_cron_fire (project,name)), `webhook.rs` (synchronized action),
  `auto_merge_impl.rs` (#2174 re-check), `scheduler.rs`.
- **Deploy procedure:** `docs/adf/operations.md` -> "Rebuilding and redeploying the adf binary"
  (build from a clean worktree at origin/main with shared CARGO_TARGET_DIR; `adf --check` before
  restart; atomic-rename the busy binary; rollback `.bak`). Backups on bigbox:
  `/usr/local/bin/adf.bak-*`, `/opt/ai-dark-factory/orchestrator.toml.bak-*`.

## 3. Outstanding

1. **Resume / expand repo-local agents (#2203 rollout)** -- canary done; extend to the other 5
   polyrepos: per repo add a `[[project_sources]]` (id + root) to orchestrator.toml + commit a
   `.terraphim/adf.toml` defining its implementation-swarm, with STAGGERED schedules (avoid
   native-runner starvation #2185). The project (gitea/token) must stay central in
   `conf.d/<repo>.toml`; do NOT redeclare the shared verdict agents (they are central via
   extra_projects) or the dup-check refuses startup.
2. **#2185** -- native runner: stuck push-runs, double-fetch with 2 instances, terraphim-ai
   starving the shared org runner. P2.
3. **#2076** -- Firecracker route; also the hardening for untrusted repo-local sources.
4. **Robustness:** consider making `merge_project_sources` fail-open (skip+warn on a duplicate)
   rather than refuse startup -- the #2203 deploy crash-looped on a stale terraphim-ai
   project_source (its agents are central; removed it).
5. **Carried backlog (re-triage via `gtr ready`):** dead-code epic #2072, #2077, #2078, #2176,
   ~34 conflicted fleet PRs.

## 4. Resume recipe

Re-enable the terraphim-agents autonomous implementer:
```bash
ssh bigbox
F=/opt/ai-dark-factory/orchestrator.toml
sudo cp -p "$F" "$F.bak-resume"
printf '\n[[project_sources]]\nid = "terraphim-agents"\nroot = "/data/projects/terraphim/terraphim-agents"\n' | sudo tee -a "$F"
cd /opt/ai-dark-factory
GITEA_TOKEN=$(systemctl cat adf-orchestrator.service | grep -oP 'Environment=GITEA_TOKEN=\K\S+') \
  /usr/local/bin/adf --check orchestrator.toml   # expect routing table, terraphim-agents implementation-swarm
sudo systemctl restart adf-orchestrator.service
```
It will fire at `:30` and pick the top unblocked terraphim-agents issue (currently #1
dead-code). Gate protects any merge.

## 5. Memory written this session
`project_adf_rebuild_deploy_procedure`, `project_adf_agent_task_semantics` (cli_tool runs the
`task` as a PROMPT, not bash; repo-local loading rules), `project_native_gitea_runner_quirks`,
`feedback_fmt_check_before_native_ci_push`. Index in
`~/.claude/projects/.../memory/MEMORY.md`.
