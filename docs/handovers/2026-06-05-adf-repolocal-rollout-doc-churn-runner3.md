# ADF Recap & Outstanding Actions -- 2026-06-05

Session focus: complete the #2203 repo-local agent rollout to all 6 polyrepos, remediate the #2225
documentation-generator churn, and (incidentally) fix a silently-broken native CI runner instance.
All work driven through Gitea (`git.terraphim.cloud`) + the ADF orchestrator on bigbox.

## 1. Completed this session

### #2203 -- repo-local `.terraphim/adf.toml` agents live in all 6 polyrepos
- **5 rollout PRs merged** through the 4-check verdict gate (`native-ci/build(push)` + `adf/pr-reviewer`
  + `adf/validation` + `adf/verification`), coordinator auto-merge at 5/5 confidence:
  - terraphim-core#3, terraphim-config-persistence#3, terraphim-kg-agents#3, terraphim-service#3,
    terraphim-clients#4 (terraphim-agents was the earlier proven canary).
- Each repo carries a repo-local `implementation-swarm` (`model = zai-coding-plan/glm-5.1`, Core layer)
  + a `build-runner` (Growth). `project_id` matches the repo in every file.
- **Schedules are staggered** (no thundering herd; mitigates the #2185 starvation concern):
  - agents `30 * * * *`, core `35`, config-persistence `40`, service `45`, kg-agents `50`, clients `55`.
  - central terraphim-ai swarm stays `0 * * * *`.
- `adf --check orchestrator.toml` routing table confirms all 6 repo-local swarms + central terraphim-ai
  load cleanly -- no dup-check failure, no errors.
- Orchestrator restarted (active; webhook listening `172.18.0.1:9091`; 9 personas, KG router, 36 per-agent
  tokens loaded). Issue commented + closed.

### #2225 -- documentation-generator churn fixed
- **Root cause**: doc-generator ran 11x/day (`40 0-10 * * *`) creating *dated* doc-gap issues
  (`docs: documentation gaps <date>` -- #2225 was itself one of these). Closed dated duplicates escaped
  its open-issue `Theme-ID` dedup, so they accrued; `implementation-swarm` then picked the unblocked
  doc-gap issues from `gtr ready` and produced large conflicting doc PRs (e.g. the 900-line #2248).
- **Remediation (deployed to bigbox `conf.d/terraphim.toml`, backup retained, validated by `adf --check`,
  picked up in the restart):**
  - (C) cadence `40 0-10 * * *` -> `40 7 * * *` (11x/day -> once daily).
  - (D) doc-gap title de-dated -> `documentation gaps (rolling)` so the `Theme-ID` dedup always updates a
    single rolling issue instead of spawning dated duplicates.
  - (E) `implementation-swarm` now SKIPS any issue whose body contains `Theme-ID: doc-gap` (rule 4b;
    those belong to `@adf:reviewer`), closing the conflicting-doc-PR path.
  - Cleanup: closed 4 stale dated doc-gap issues (#1979, #2035, #2071, #2136) + commented/closed PR #2248.
- Issue commented + closed.

### Incidental -- native runner instance-3 was silently broken (regression from #2185)
- **Symptom**: terraphim-service#3 + terraphim-clients#4 failed `native-ci` on a no-op `.terraphim/adf.toml`
  change while main was green; the failing step was `cargo fmt --all -- --check`, completing in <1s.
- **Root cause**: instance-3 (added during #2185 for capacity) was registered/online but had **no
  `~/.cargo-runner-3/bin`** -- its `env-3` PATH resolved no `cargo`/`rustfmt`, so it failed *every* job it
  fetched instantly at the first cargo step, silently poisoning ~1/3 of all org CI.
- **Fix**: `mkdir -p ~/.cargo-runner-3 && cp -a ~/.cargo-runner-2/bin ~/.cargo-runner-3/bin` + restart;
  both PRs re-triggered via empty-commit push (push event) -> all 4 checks green -> merged.
- Saved to memory `project_native_gitea_runner_quirks` with the lesson: "an instance being 'online' in the
  runners API is not proof it works -- prove it ran a real cargo step green; mirror the toolchain bin
  BEFORE `enable --now`."
- All 3 runner instances now genuinely healthy.

## 2. Current state (verified 2026-06-05 13:42 BST)
- adf-orchestrator: active, no errors/warns since restart (one pre-existing reviewer-parse WARN was from
  the old pid pre-shutdown -- unrelated).
- Native runners: `terraphim-gitea-runner{,-2,-3}.service` all active; instance-3 toolchain repaired.
- 6 polyrepos: 0 open PRs each; `.terraphim/adf.toml` present on main in every bigbox checkout.
- orchestrator.toml: 6 `[[project_sources]]` (skip-safe -- missing file logs a warn rather than refusing
  startup); conf.d remediation live.

## 3. Outstanding actions

### Observe (no code change; confirm the fix holds)
- [ ] **#2225 churn**: confirm over the next 24-48h that the doc-generator daily run updates ONE rolling
  doc-gap issue and `implementation-swarm` no longer opens doc PRs. Check `gtr list-issues` for any new
  dated `docs: documentation gaps <date>` issues (there should be none).
- [ ] **Repo-local swarms first fire**: watch the staggered :30-:55 cron runs land their first PRs on the
  5 newly-onboarded polyrepos; verify each goes through its repo's verdict gate and merges (or queues for
  human review at <5/5), and that none collide for runner capacity.

### Known open items (carried, not regressions)
- [ ] **terraphim-agents#1** (dead-code): open, unblocked, untouched -- available for the agents swarm to
  pick up on its `30 * * * *` cron.
- [ ] **`merge_project_sources` fail-open**: still fail-CLOSED on a malformed repo-local `adf.toml`
  (dup/id-mismatch/missing-project refuses startup; missing-file is already skip-safe). Consider making a
  bad repo-local file skip-with-warn so one repo cannot wedge the whole fleet. (Design follow-up; low
  priority now that all 6 files are known-good.)

### #2185 native-runner follow-ups (deferred, from the approved design)
- [ ] Optional label-tier isolation (`runs-on: terraphim-native-ai` for terraphim-ai) so its CI volume
  cannot starve polyrepo runs -- capacity (3 instances) + staggered swarms currently mitigate this.
- [ ] Milestones 2-3 of the native-runner plan remain out of scope: Firecracker route wiring, artifacts,
  broad `uses:` action emulation, branch-protection cutover automation.

## 4. Key references
- Gitea: terraphim/terraphim-ai #2203 (closed), #2225 (closed), #2185 (runner reliability), #2241 (merged
  #2185 Fix A/B), #2201 (adf ops docs).
- Memory: `project_native_gitea_runner_quirks`, `project_adf_rebuild_deploy_procedure`,
  `project_adf_agent_task_semantics`, `feedback_fmt_check_before_native_ci_push`.
- bigbox: `/opt/ai-dark-factory/{orchestrator.toml,conf.d/terraphim.toml}` (conf.d root-owned, sudo to
  edit); runner toolchains `~/.cargo-runner-{2,3}/bin`; deploy adf/runner from the `gitea` remote, never
  `origin` (GitHub mirror is behind).
