# ADF Phase A/B + Ops Remediation — 2026-06-13

Session focus: land runner infra fixes, close four operational blockers (#2666, #2661, #2660, Phase A deploy), and fix Phase B duplicate `[ADF] Auto-merge failed` issue creation. Orchestrator work lives in **terraphim-agents** (crate extracted from terraphim-ai per #1910).

## 1. Completed this session

### Runner / Gitea commit status (terraphim-ai — merged)

- **PR #2665** (`task/runner-status-fallback-env-policy`): `RUNNER_STATUS_TOKEN` / `GITEA_TOKEN` fallback for `/statuses`; `strip_env_assignments()` strips `RUSTDOC=$(rustup …)` env-prefix policy rejects in agent worktree `native-ci.yml`.
- **#2464** closed (401 commit status stale after `workflow_dispatch` rerun).
- **Bigbox runners** rebuilt **on Linux** from `main` @ `14b90b15d` (never SCP macOS binary). `RUNNER_STATUS_TOKEN` in `~/.config/terraphim-gitea-runner/env*`.
- **terraphim-agents** workflow run **#110** green via `workflow_dispatch`.

### Four operational items ("proceed with all 4")

| Item | Result |
|------|--------|
| **#2666** | Already **closed** — false-positive auto-merge failure for PR #2665 (status checks not yet green when ADF tried merge; PR later merged manually). |
| **#2661** | **Fixed & closed** — meta-coordinator cron lacked `GITEA_TOKEN` → HTTP 401. Patched `/opt/ai-dark-factory/conf.d/terraphim.toml`: `source ~/.profile` at start of scheduled task. |
| **#2660** | **Fixed & closed** — `.worktrees` on bigbox pruned **20G → 131M** (removed 19G stale `implementation-swarm-63093d18` + 37 orphaned dirs; kept active swarm worktree). |
| **Phase A** | Implemented in **terraphim-agents** — `evaluate_pr_gates()` + auto-merge wired to `PrGateResult` commit statuses; tests pass; ADF binary deployed from branch work on bigbox. **Gitea PR #48** open. |

### Phase B — failure issue dedup (terraphim-agents)

**Root cause:** Dedup searched Gitea `q` without verifying `ADF-Failure-Key:` in issue body; search errors still created new issues; in-memory TTL skipped Gitea recurrence comments.

**Fix (branch `task/phase-b-failure-dedup` @ `f2951a7`):**

- `crates/terraphim_tracker/src/gitea.rs` — `search_open_issues_by_dedup_key`, body marker verification, exact-title legacy fallback, fail-closed on search error.
- `crates/terraphim_orchestrator/src/auto_merge_impl.rs` — always route failures through `open_failure_issue` (dedup path).
- Tests: 3 tracker dedup tests + false-positive regression; `auto_merge_tests` green.

**Deployed:** ADF **1.20.2** on bigbox (`adf-orchestrator` active) — built from agents tree before PR merge.

**PR:** [terraphim-agents #49](https://git.terraphim.cloud/terraphim/terraphim-agents/pulls/49)

**Batch-close audit:** 0 duplicate-key issues closed — 28 open `[ADF] Auto-merge failed` trackers are **one per still-open failing PR**, not duplicate keys. Recurrence now posts **comments** (verified on issues updated ~23:42 with 2 comments each).

## 2. Current state (verified 2026-06-13 ~23:45 BST)

### Repositories

| Repo | Branch / SHA | Notes |
|------|----------------|-------|
| **terraphim-ai** `origin/main` | `14b90b15d` | Runner fix merged (#2665). Orchestrator crate **removed** — do not rebase `task/adf-flow-fix-phase1-automerge` onto main. |
| **terraphim-ai** local | `task/adf-flow-fix-phase1-automerge` @ `22e64bd4c` | Historical Phase A work; canonical home is terraphim-agents. |
| **terraphim-agents** `main` | `694d55e1` | Three open PRs: #48 (Phase A), #49 (Phase B), #44 (#2285 remediation). |

### Bigbox (`/data/projects/terraphim/`)

- **ADF:** `/opt/ai-dark-factory/adf` v1.20.2; `adf-orchestrator` **active**.
- **Agents checkout:** `/data/projects/terraphim/terraphim-agents` — Phase A+B commits deployed locally; PRs not merged to `main`.
- **Worktrees:** ~390M (grew from 131M post-prune — monitor).
- **Runners:** Only **runner-1** confirmed active last check; runner-2/3 **inactive** — verify if intentional.
- **Config:** meta-coordinator uses `source ~/.profile` for `GITEA_TOKEN`.

### Open tracker noise

- **28** open `[ADF] Auto-merge failed` issues — legitimate trackers for open PRs failing merge API (mostly `405 Method Not Allowed: Please try again later` on backlog PRs). Dedup prevents *new* duplicates per `(project, pr, head_sha)` key.

## 3. What's working

- Gitea API health with `source ~/.profile` on bigbox cron paths.
- Runner status posting with token fallback (when runners active).
- ADF auto-merge qualification via **`evaluate_pr_gates`** (native-ci + adf/pr-reviewer + adf/validation + adf/verification + `adf:gate-result` blocks).
- Phase B dedup in **deployed** binary (body `ADF-Failure-Key:` marker + comment-on-recurrence).
- Worktree disk under control after manual prune (re-monitor).

## 4. Outstanding actions

### Immediate (merge + redeploy)

- [ ] **Babysit terraphim-agents #49** (Phase B) → merge after CI green.
- [ ] **Merge terraphim-agents #48** (Phase A) — rebase onto main after #49 if stack order matters.
- [ ] **Redeploy ADF from terraphim-agents `main`** after merges (not from feature branches):
  ```bash
  cd /data/projects/terraphim/terraphim-agents
  git fetch && git checkout main && git pull
  cargo build --release -p terraphim_orchestrator --bin adf
  sudo systemctl stop adf-orchestrator
  sudo cp target/release/adf /opt/ai-dark-factory/adf
  sudo systemctl start adf-orchestrator
  ```
- [ ] **Verify dedup live:** trigger recurring auto-merge failure on same PR+SHA → expect **comment** on existing issue, not new issue.

### Operational

- [ ] **Runner-2/3:** confirm inactive state; restart `terraphim-gitea-runner{,-2,-3}` if capacity needed.
- [ ] **Worktree monitor:** alert at 10GB again; consider automated prune cron.
- [ ] **28 auto-merge failure issues:** close when linked PR merges/closes (optional batch script with full PR pagination — ~100+ open PRs on terraphim-ai).

### Phase C+ (from `.docs/plan-adf-flow-outstanding-actions-2026-06-13.md`)

- [ ] **#2285** remediation loop — PR [terraphim-agents #44](https://git.terraphim.cloud/terraphim/terraphim-agents/pulls/44) exists; **do not redeploy** until merged + proven; blocks full unattended flow.
- [ ] **#2465** `blocker_kind` classification — distinguish CI vs confidence vs policy in logs/issues; stop spurious failure issues for pre-merge gate blocks.
- [ ] **implementation-swarm cooldown** — planned 1h no-work cooldown not yet wired (`grace_period_secs = 30` still).
- [ ] **Branch protection alignment** — verify all `terraphim/*` repos have exact emitted status contexts.
- [ ] **End-to-end proof** — `.docs/validation-report-adf-flow-fix.md` not yet written.

## 5. Technical context

### Local terraphim-ai

```bash
git branch --show-current   # task/adf-flow-fix-phase1-automerge
git log -1 --oneline        # 22e64bd4c fix(orchestrator): unify auto-merge on PrGateResult commit statuses
git log origin/main -1      # 14b90b15d merge(github): converge github/main into gitea main
```

### Session checkpoint (next agent)

```bash
source ~/.profile
git fetch origin 2>/dev/null
echo "=== Existing task branches ===" && git branch -r | grep "task/" || true
echo "=== Open PRs (agents) ===" && gitea-robot list-pulls --owner terraphim --repo terraphim-agents --state open
gitea-robot ready --owner terraphim --repo terraphim-ai
```

### Key files (terraphim-agents — not terraphim-ai main)

| File | Change |
|------|--------|
| `crates/terraphim_orchestrator/src/pr_poller.rs` | `evaluate_pr_gates()` — Phase A |
| `crates/terraphim_orchestrator/src/auto_merge_impl.rs` | Gate-based merge + always-dedup path |
| `crates/terraphim_tracker/src/gitea.rs` | Body-verified dedup — Phase B |
| `crates/terraphim_orchestrator/tests/auto_merge_tests.rs` | Gate fixture tests |

### Key files (terraphim-ai — merged)

| File | Change |
|------|--------|
| `crates/terraphim_gitea_runner/` | Status token fallback + env-prefix strip |

### Bigbox config patch

`/opt/ai-dark-factory/conf.d/terraphim.toml` — meta-coordinator scheduled task starts with `source ~/.profile`.

### Test commands

```bash
# terraphim-agents (on bigbox or local clone)
cargo test -p terraphim_tracker create_issue_or_comment
cargo test -p terraphim_orchestrator --test auto_merge_tests
cargo test -p terraphim_orchestrator --test remediation_tests  # #2285 PR

# terraphim-ai runner
cargo test -p terraphim_gitea_runner
```

## 6. Landing notes

- **No terraphim-ai `main` push** required for Phase A/B — work is on **terraphim-agents** branches only.
- Local terraphim-ai has extensive **untracked** `.docs/` scratch — do not bulk-commit.
- Phase B deployed binary on bigbox **ahead of** merged `main` — reconcile after PR merges.
- GitHub mirror: terraphim-ai #2665 ↔ Gitea #2665 converged on runner fix.

## 7. Recommended next session

1. `/pr-babysit` terraphim-agents **#49** then **#48**.
2. Redeploy ADF from agents `main`; confirm version string and gate behaviour.
3. Observe one poll cycle — recurring failures comment rather than spawn.
4. Pick next unblocked issue from `gitea-robot ready` after checkpoint (likely #2465 or #2285 merge path).
5. Update `.docs/plan-adf-flow-outstanding-actions-2026-06-13.md` Phase A/B rows to **Complete** after PR merge + redeploy proof.