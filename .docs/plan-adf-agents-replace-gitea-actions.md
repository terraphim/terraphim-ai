---
date: 2026-04-26
type: plan
status: scoped
owner: alex
tags: [adf, gitea, ci, webhooks, status-checks, github-actions]
parent_design: ".docs/design-implementation-plan-2026-04-17.md"
prior_handover: ".docs/handover-2026-04-25-adf-session.md"
decisions_locked: 2026-04-26
---

## Decisions locked (AskUserQuestion 2026-04-26)

| # | Decision | Effect on plan |
|---|----------|----------------|
| D1 | **Push to GitHub is manual / opt-in.** Gitea is the canonical event source. | Phase 4 (GH check-run mirror) **dropped from scope**. `build-runner` via rch is the only deterministic path. |
| D2 | **Start minimal**: Phase 2 ships `build-runner` + `pr-reviewer` only. Add other PR-event agents incrementally after soak. | Phase 2 scope reduced from 5 agents to 2. New Phase 2b/2c/2d for incremental rollout. |
| D3 | **Status check naming**: `adf/<agent>` prefix. | Used throughout: `adf/build`, `adf/pr-reviewer`, `adf/security`, `adf/spec`, `adf/compliance`, `adf/test`. |
| D4 | **All ADF status checks REQUIRED** (block merge): `adf/build`, `adf/pr-reviewer`, `adf/security`, `adf/spec`, `adf/compliance`, `adf/test`. | Branch protection configured at end of Phase 2; subsequent agents go straight to `required` as they ship. **Risk surfaced in §10.** |
| D5 | **`pr_dispatch` is per-project** (declared inside `IncludeFragment` so each `conf.d/<project>.toml` carries its own block). Top-level `[pr_dispatch]` kept as backward-compat fallback. | §5 Phase 2 deliverable updated: the block lives in `conf.d/<project>.toml`, not `orchestrator.toml`. Multi-project deployment unblocked (was §13 out-of-scope). See `.docs/design-pr-dispatch-per-project.md` (Gitea issue #962). |

# Plan: Replace Gitea Actions with ADF Agents (leveraging existing GitHub Actions stack)

## 1. Position statement

We do **not** want to spin up a separate Gitea Actions runner pool (`act_runner` /
forgejo-runner) for the `terraphim-ai` repo. The existing GitHub Actions
investment -- `[self-hosted, bigbox]` runners + `rch exec` dispatch + SeaweedFS S3
cache (82.83% hit rate, run `24929848023`) -- is the optimal deterministic CI
path and is already paid for.

Instead, we will use the **AI Dark Factory orchestrator** as a Gitea-native CI
substitute that:

- consumes Gitea events via the already-deployed webhook handler,
- dispatches **deterministic checks** (cargo fmt / clippy / build / test) by
  invoking `rch exec` from agent shells, reusing the bigbox + SeaweedFS pipeline,
- dispatches **intelligent checks** (pr-reviewer, compliance-watchdog,
  spec-validator, security-sentinel, test-guardian) as LLM agents,
- folds both kinds of verdict back into Gitea PRs as **commit status checks** so
  they appear as native red/green gates in the Gitea UI.

**Architectural guardrail**: deterministic checks (`cargo build/test/clippy`)
stay on rch + SeaweedFS. They are not regressed onto LLM execution. ADF agents
only *trigger* and *summarise* deterministic runs; they don't replace them.

## 2. What already exists (do not redesign)

Verified during scoping (commits, file paths, ADF live config):

| Capability | Location | Status |
|------------|----------|--------|
| Webhook router on `172.18.0.1:9091` | `scripts/adf-setup/scripts/adf-setup/orchestrator.toml` `[webhook]` block | Live |
| HMAC-verified `/webhooks/gitea` endpoint | `crates/terraphim_orchestrator/src/webhook.rs` | Live |
| `WebhookDispatch::SpawnAgent` (issue comments) | `webhook.rs` | Live |
| `WebhookDispatch::SpawnPersona` | `webhook.rs` | Live |
| `WebhookDispatch::CompoundReview` | `webhook.rs` | Live |
| `WebhookDispatch::ReviewPr` (pull_request opened) | `webhook.rs`, dispatched in `lib.rs:1776` | Live (commit `9cfd4867` ROC v1 Step C) |
| `pr-reviewer` agent on PR open | `scripts/adf-setup/agents/pr-reviewer.toml` | Live |
| Issue body `@adf:` mention scan | commit `ec0c3967` | Live |
| Project-aware mention resolution `@adf:project/agent` | commit `f3a14b37` | Live |
| Polling fallback (`terraphim-agent listen`) | `crates/terraphim_agent/src/listener.rs` | Live (laptop tmux) |
| ADF agent registry (19 agents) | `/opt/ai-dark-factory/conf.d/terraphim.toml` (bigbox) | Live |
| GitHub Actions on bigbox via rch | `.github/workflows/ci-{firecracker,main,pr,native}.yml` | Live, optimal |
| `merge-coordinator` (4-hourly review-gate) | `scripts/adf-setup/agents/merge-coordinator.toml` | Live |

This plan is a **delta** on top of that, not a parallel design.

## 3. Gaps that block "ADF replaces Gitea Actions"

| # | Gap | Why it blocks the goal |
|---|-----|------------------------|
| G1 | Agent verdicts are posted as **PR comments**, not **commit statuses** | Without status checks, Gitea PR UI cannot show "ADF must pass before merge"; merge-coordinator's review gate isn't visible to humans |
| G2 | Only `pr-reviewer` runs on `pull_request.opened` | spec-validator, compliance-watchdog, security-sentinel, test-guardian still cron-only — they fire hourly regardless of PR state, wasting runs and missing real-time PR feedback |
| G3 | No `WebhookDispatch::Push` handler | Push to Gitea-only branches gets no build/test verdict. GH Actions only fires when push reaches `github` remote (manual or via mirror) |
| G4 | No bridge to fold GH Actions check-run results into Gitea status checks | Repos that mirror to GH duplicate compute or hide the verdict from Gitea reviewers |
| G5 | Polling listener still runs in parallel with webhook | Two paths to claim issues; risk of double-dispatch; tmux session is operationally fragile |
| G6 | No deterministic `build-runner` agent | The shell-glue that runs `rch exec -- cargo …` and posts a status doesn't exist as a first-class agent template |

## 4. Architecture (target state)

```
                  Gitea (git.terraphim.cloud)
                          |
                event: push | pull_request | issues | issue_comment
                          |
                          v
        +-----------------------------------------+
        |  ADF webhook endpoint (172.18.0.1:9091) |
        |  HMAC-verified, dedup by delivery_id    |
        +--------------------+--------------------+
                             |
       +---------------------+----------------------+
       |                     |                      |
       v                     v                      v
  WebhookDispatch::Push  WebhookDispatch::ReviewPr  SpawnAgent (mention)
       |                     |                      |
       v                     |                      v
  build-runner agent         |               existing path
  (rch exec --               v                      |
   cargo fmt/clippy/    fan-out:                    |
   build/test)         + pr-reviewer                |
   posts: adf/build    + compliance-watchdog        |
                       + spec-validator             |
                       + security-sentinel          |
                       + test-guardian              |
                       (each posts own status)      |
                             |                      |
                             +----------+-----------+
                                        |
                             +----------v----------+
                             | StatusReporter      |
                             | POST /api/v1/repos/ |
                             |  {o}/{r}/statuses/  |
                             |  {sha}              |
                             +---------------------+
                                        |
                                        v
                        Gitea PR UI: red/green checks
                                        |
                                        v
                              merge-coordinator
                              (gates merge on all status=success)


                Optional Phase 4 bridge:
                +--------------------------+
                | gh api check-runs poller |
                | (only for mirrored PRs)  |
                +--------------------------+
                            |
                            v
                  posts adf/gh-mirror status
                  (deterministic check from GH Actions)
```

### Reuse map

| Existing component | Reused for | New code |
|-------------------|-----------|----------|
| `webhook.rs` router | `Push` variant | Add enum arm + payload struct (~50 LOC) |
| `pr-dispatch` (`crates/terraphim_orchestrator/src/pr_dispatch.rs`) | Each new PR-fan-out agent | One row per agent in dispatch list |
| `rch exec` + SeaweedFS S3 cache | `build-runner` shell | New agent template, no infra changes |
| `pr-reviewer.toml` skeleton | New `*.toml` per agent | Copy-paste with different skill_chain |
| `terraphim_tracker::gitea` REST client | StatusReporter | Add one method `set_commit_status(sha, state, ctx, desc, url)` |
| `merge-coordinator` | Same agent, status-aware | Replace comment-scrape with status-API read |

## 5. Phased delivery (vital five, ordered by unblock value)

**5/25 rule applied**: out of ~20 things we could do, these five are vital and
ordered. Anything else is in §9 "Avoid At All Cost".

### Phase 1 -- StatusReporter for ADF verdicts (G1)

**Goal**: every ADF agent that produces a PR/commit verdict posts a Gitea
commit status. Agent comments stay (richer detail), but the status is the gate.

**Files**:
- `crates/terraphim_tracker/src/gitea.rs` -- add
  ```rust
  pub async fn set_commit_status(
      &self,
      sha: &str,
      state: StatusState,    // pending | success | failure | error
      context: &str,         // e.g. "adf/pr-reviewer"
      description: &str,     // <=140 chars
      target_url: Option<&str>,
  ) -> Result<(), TrackerError>;
  ```
  Endpoint: `POST /api/v1/repos/{owner}/{repo}/statuses/{sha}` (Gitea spec).
- `crates/terraphim_orchestrator/src/lib.rs` -- after `handle_review_pr`
  finishes, call `set_commit_status(head_sha, …)` with the routed agent's
  context.
- `scripts/adf-setup/agents/pr-reviewer.toml` -- task already posts a comment
  with `Last reviewed commit: $HEAD_SHORT`. Add a final shell line that
  computes verdict (pass / fail / error from the structural-pr-review
  Confidence Score) and calls a new `gtr set-status` shim.
- `gtr set-status` -- new gitea-robot subcommand, thin wrapper over the REST
  endpoint. Two-line addition.

**Tests**:
- Unit: `tracker::gitea::tests::set_commit_status_posts_correct_payload` -- mock
  Gitea HTTP, assert path + body shape (`{state, context, description, target_url}`).
- Integration: spawn a fake Gitea PR via gtr, run pr-reviewer dry-run, assert
  status appears via `gh api gitea` equivalent (our `gtr view-pull` already
  returns `statuses_url`).

**Done-when**: Open Gitea PR shows `adf/pr-reviewer pending → success` next to
GH Actions check rows.

**Effort**: 1 day.

### Phase 2 -- Minimal PR fan-out: build-runner + pr-reviewer (G2, partial; per D2)

**Goal**: `pull_request.opened` enqueues exactly two agents -- the deterministic
`build-runner` (Phase 3 dependency, brought forward) and the existing
`pr-reviewer`. The other three (`spec-validator`, `compliance-watchdog`,
`security-sentinel`, `test-guardian`) wait for Phases 2b-2d after soak.

**Files**:
- `crates/terraphim_orchestrator/src/pr_dispatch.rs` -- replace
  `pr-reviewer`-only routing with an iteration over a config list. **Phase 2
  ships with only two entries.** Per D5, the block lives in the project's
  own `conf.d/<project>.toml` (see
  `.docs/design-pr-dispatch-per-project.md`, Gitea issue #962), not in the
  top-level `orchestrator.toml`:
  ```toml
  # conf.d/terraphim.toml
  [[projects]]
  id = "terraphim"
  working_dir = "/var/lib/adf/terraphim"

  [pr_dispatch]
  agents_on_pr_open = [
      { name = "build-runner", context = "adf/build", skill = "" },
      { name = "pr-reviewer", context = "adf/pr-reviewer", skill = "structural-pr-review" },
  ]
  # Phase 2b adds: spec-validator   (context = "adf/spec",       skill = "requirements-traceability")
  # Phase 2c adds: security-sentinel (context = "adf/security",   skill = "security-audit")
  # Phase 2d adds: compliance-watchdog (context = "adf/compliance", skill = "responsible-ai")
  # Phase 2e adds: test-guardian   (context = "adf/test",        skill = "testing")
  ```
- New PR-event agent templates land per phase, not all at once:
  - Phase 2:  `build-runner.toml` (defined in Phase 3 below; Phase 2 brings it forward)
  - Phase 2b: `pr-spec-validator.toml`
  - Phase 2c: `pr-security-sentinel.toml`
  - Phase 2d: `pr-compliance-watchdog.toml`
  - Phase 2e: `pr-test-guardian.toml`
  Each is a copy of the cron template with: no `schedule` field, scoped task
  (operate on `$ADF_PR_DIFF` not full repo), context-specific skill_chain.
  Each phase soaks 7 days before the next agent is added to
  `agents_on_pr_open` and to the required-checks list.
- `lib.rs` -- `handle_review_pr` becomes `handle_review_pr_fanout` that
  iterates `agents_on_pr_open`, applies subscription/budget gating per agent,
  spawns each with the same `ADF_PR_*` env injection pattern.

**Tests**:
- Unit: `pr_dispatch::tests::fanout_routes_all_five_agents` -- assert each
  agent gets a routing decision and either spawns or is skipped with reason.
- Integration: simulate webhook, observe 5 `set_commit_status` POSTs (one per
  context), all initially `pending`.

**Risk control**: budget. 5x agents on every PR open = 5x cost. Three guards:
1. **Diff-size filter**: agents that read full diff skip if `diff_loc > 5000`
   (rely on cron pass for big PRs).
2. **Path filter per agent**: spec-validator only runs if `plans/**` or
   `crates/**/src/**` changed; compliance-watchdog only if `Cargo.toml`,
   `Cargo.lock`, or `LICENSE*` changed; test-guardian only if `**/tests/**`
   or `crates/**/src/**` changed.
3. **Per-agent monthly budget** -- already exists in routing decision; just
   wire each agent name into its own budget bucket.

**Done-when**: New PR triggers `adf/build` and `adf/pr-reviewer` status checks
within 5 minutes; both are configured as required in branch protection so the
merge button is disabled until both `success` (per D4).

**Branch protection setup at end of Phase 2**: configure
`terraphim/terraphim-ai` Gitea branch-protection rule on `main` to require
`adf/build` and `adf/pr-reviewer`. Each subsequent Phase 2b-2e adds its
context to the required list as it ships.

**Effort**: 2 days.

### Phases 2b-2e -- incremental PR-fan-out (D2)

Each phase: 1 day, 7-day soak before the next.

| Phase | Adds agent | New required check | Path filter |
|-------|-----------|-------------------|-------------|
| 2b | `pr-spec-validator` | `adf/spec` | `plans/**`, `crates/**/src/**`, `**.rs` |
| 2c | `pr-security-sentinel` | `adf/security` | `Cargo.toml`, `Cargo.lock`, `crates/**/src/**`, `**/secrets/**` |
| 2d | `pr-compliance-watchdog` | `adf/compliance` | `Cargo.toml`, `Cargo.lock`, `LICENSE*`, `**/THIRD_PARTY*` |
| 2e | `pr-test-guardian` | `adf/test` | `**/tests/**`, `crates/**/src/**` |

Same skeleton as Phase 2; differs only in agent template, path filter, and
required-check addition. Each ships only after the prior phase's soak metric
shows < 5% false-positive rate (operator-judged from blocked PRs that
warranted override).

### Phase 3 -- Push event handler + build-runner agent (G3, G6)

**Goal**: push to a Gitea branch (PR or main) gets the same deterministic
build/test verdict as a GitHub push, without GitHub Actions in the loop.

**Files**:
- `crates/terraphim_orchestrator/src/webhook.rs`:
  ```rust
  pub enum WebhookDispatch {
      ...,
      Push {
          project: String,
          ref_name: String,    // refs/heads/main, refs/pull/N/head, etc.
          before_sha: String,
          after_sha: String,
          files_changed: Vec<String>,
      },
  }
  ```
  Handler maps Gitea `push` event to `Push`; HMAC verify; dedup by delivery_id.
- `crates/terraphim_orchestrator/src/lib.rs` -- new `handle_push` that:
  1. Posts `adf/build pending` immediately for `after_sha`.
  2. Spawns `build-runner` agent with env `ADF_PUSH_SHA`, `ADF_PUSH_REF`,
     `ADF_PUSH_FILES`.
- `scripts/adf-setup/agents/build-runner.toml` -- new template, **no LLM**:
  ```toml
  [[agents]]
  name = "build-runner"
  layer = "Growth"
  cli_tool = "/bin/bash"          # plain shell, no model
  max_cpu_seconds = 1800
  capabilities = ["build", "test", "deterministic-ci"]
  task = '''
  source ~/.profile
  cd "$GITEA_WORKING_DIR"
  git fetch origin "$ADF_PUSH_REF" && git checkout "$ADF_PUSH_SHA"
  set -e
  /home/alex/.local/bin/rch exec -- cargo fmt -- --check
  /home/alex/.local/bin/rch exec -- cargo clippy --workspace --all-targets -- -D warnings
  /home/alex/.local/bin/rch exec -- cargo test --workspace --no-fail-fast
  gtr set-status --owner "$GITEA_OWNER" --repo "$GITEA_REPO" \
      --sha "$ADF_PUSH_SHA" --state success --context "adf/build" \
      --description "fmt+clippy+test pass via rch"
  '''
  ```
  Failure path: bash trap on non-zero exit posts `failure` with the failing
  step name.
- `agents_on_pr_open` (Phase 2) gains `build-runner` so PR open also runs the
  deterministic gate.

**Why this is the actual "replace Gitea Actions" core**: nothing in this path
needs `act_runner`. The compute is rch dispatching to bigbox-local cargo with
SeaweedFS cache -- identical to GitHub Actions but driven by Gitea events.

**Tests**:
- Live: push a no-op commit to a test branch, observe `adf/build` status
  go pending → success in <90s (warm cache) or <8min (cold cache).
- Unit: `webhook::tests::push_event_parses` -- payload fixture from Gitea docs.

**Done-when**: PR opened on Gitea-only branch (no GH push) shows
`adf/build success` from rch+SeaweedFS within timeout window.

**Effort**: 2 days.

### ~~Phase 4 -- GitHub Actions check-run mirror~~ (DROPPED per D1)

**Status**: dropped from scope. Push to GitHub is manual / opt-in (D1), so
GH Actions doesn't fire on every Gitea event. `build-runner` (Phase 3) is the
sole deterministic path. If auto-mirror is enabled in the future, reinstate
this phase.

### Phase 5 -- Decommission polling listener (G5)

**Goal**: webhook is the only path; tmux listener is retired.

**Procedure** (no code change unless metrics show drop):
1. After Phases 1-3 are live for 7 days, query Gitea webhook delivery log:
   `gh api repos/terraphim/terraphim-ai/hooks/{id}/deliveries` (Gitea has same
   endpoint). Compute success rate. Target: >99.5%.
2. Add `webhook_event_count` gauge to ADF orchestrator metrics
   (Prometheus already in `[webhook]` block? -- verify).
3. Compare to `terraphim-agent listen` poll-resolved mention count for the
   same period. If webhook >=99% of polled volume, kill tmux session.
4. Listener code stays in tree (offline use case + dev environments still
   need it); only the production tmux session stops.

**Done-when**: ADF orchestrator journal shows zero `mention resolved via
listener` events for 48 h, all dispatches via webhook.

**Effort**: 0.5 day (operational only).

## 6. Rollout sequencing (post-decisions)

```
Phase 1 (StatusReporter)
        │
        ▼
Phase 3 (build-runner agent + Push handler)   ◀── brought before Phase 2
        │   because Phase 2 needs build-runner to exist
        ▼
Phase 2 (PR fan-out: build-runner + pr-reviewer)
        │   branch protection: require adf/build, adf/pr-reviewer
        ▼
7-day soak
        ▼
Phase 2b (pr-spec-validator)        → require adf/spec
        ▼
7-day soak
        ▼
Phase 2c (pr-security-sentinel)     → require adf/security
        ▼
7-day soak
        ▼
Phase 2d (pr-compliance-watchdog)   → require adf/compliance
        ▼
7-day soak
        ▼
Phase 2e (pr-test-guardian)         → require adf/test
        ▼
Phase 5 (Decommission polling listener)
```

**Phase 3 moved before Phase 2** because the minimal Phase 2 (per D2) requires
`build-runner` to already exist as an agent template.

Phases 1-3 are the substantive shipping. Phase 4 is reactive (only if mirror
exists). Phase 5 is operational (after enough soak time).

Each phase ships behind no feature flag -- all changes are additive (new
endpoint, new agents, new statuses) and merge-coordinator already gates on
status presence.

## 7. Migration impact on existing agents

| Existing agent | Today | After this plan |
|----------------|-------|-----------------|
| `pr-reviewer` (mention + PR open) | comment only | comment + `adf/pr-reviewer` status |
| `compliance-watchdog` (cron 5 0-10) | cron only, full repo scan | cron stays for full audits + new `pr-compliance-watchdog` for diff-scoped PR feedback |
| `spec-validator` (cron 30 0-10) | cron only | cron stays + new `pr-spec-validator` |
| `security-sentinel` (cron 0 \*/6) | cron only | cron stays + new `pr-security-sentinel` |
| `test-guardian` (cron 35 0-10) | cron only | cron stays + new `pr-test-guardian` |
| `merge-coordinator` (cron 0 \*/4) | scrapes comments | reads status checks via API |
| `build-runner` (NEW) | -- | event-driven, shell-only, no LLM |
| `terraphim-agent listen` (laptop) | mentions only | retire after Phase 5 |

Cron agents remain because they audit the full repo, not just diffs. PR-scoped
variants are diff-aware and cheaper.

## 8. Why this leverages the existing GitHub Actions investment

1. **Same compute path**: `build-runner` dispatches via `rch exec`. rch is the
   same binary, dispatches to the same bigbox slots, hits the same SeaweedFS S3
   cache. The 82.83% cache hit rate observed in run `24929848023` applies
   verbatim. No new cache infrastructure, no second cargo target dir.
2. **Same workflow shape**: the steps `cargo fmt --check`, `cargo clippy
   -- -D warnings`, `cargo test --workspace --no-fail-fast` are lifted directly
   from `.github/workflows/ci-pr.yml`. Any future workflow optimisation in GH
   Actions ports to `build-runner` by edit, not rewrite.
3. **GitHub Actions stays canonical for terraphim-ai**. We don't move that
   work. Phase 4 is a **bridge**, not a replacement.
4. **No `act_runner` deployment**. The most expensive line item -- a separate
   Gitea-side runner pool with its own image, secrets, scheduler -- is
   eliminated. ADF orchestrator + rch is the runner.

## 9. Avoid At All Cost (5/25 rule)

| Rejected | Why dangerous |
|----------|---------------|
| Deploy `act_runner` / `forgejo-runner` | Duplicate compute, second secret pool, second cache, paid for nothing |
| Move `cargo build/test` into LLM agents | Pure regression -- LLM is wrong tool for deterministic work |
| Build a second webhook handler | One handler, one HMAC secret, one dedup table |
| Add a status-check DB | Status check is the DB. Don't shadow it. |
| Real-time WebSocket coordination | Webhooks + status polling are sufficient and proven |
| Custom JSON-RPC between agents | Gitea issues + comments are the bus -- already is |
| Multi-region webhook receivers | bigbox single-host is fine for current load |
| Block PR open until all 5 agents finish | Use Gitea required-status-checks + branch protection; let agents run async |
| Replace `merge-coordinator` with status-only gate | Coordinator does context-aware judgement on partial-pass scenarios; keep it |
| Couple ADF agents to GitHub Checks API as the storage | Gitea is canonical; mirror is one-way |

## 10. Risks and mitigations

| Risk | Mitigation |
|------|-----------|
| Webhook delivery delayed/dropped | Gitea retries 3x with backoff; orchestrator dedup via delivery_id; cron `compound-review` catches misses; Phase 5 only after 7 days of >99.5% delivery |
| 5x cost increase on PR open | Path filters per agent; diff-LOC ceiling; per-agent monthly budgets; subscription-only models (C1 invariant) |
| Status check spam if agents flap | Each agent posts `pending` once; subsequent posts are state transitions only (`set_commit_status` is idempotent on (sha, context)) |
| `build-runner` queue saturation | rch already enforces `slots = nproc/4 = 4` cap; queueing is deliberate; status stays `pending` until job leaves queue |
| HMAC secret rotation | Already in orchestrator config; rotate via 1Password and restart |
| Loss of cron coverage if PR-scoped agents misfire | Cron agents stay scheduled; PR-scoped variants are additive |
| Listener decommission too early | Phase 5 gate is metric-driven (>99.5% webhook delivery for 7 days), not calendar-driven |
| **D4 risk: required-from-day-one with no informational soak** | The 7-day soak between Phases 2b-2e is the safety net. Each new required check ships only after operator review of the prior phase's blocked-PR overrides shows < 5% false-positive rate. If a phase fails the soak, the agent gets demoted to informational (status posted but not in branch protection's required list) and the next phase blocks until the issue is fixed. Operator override path: a `adf-bypass:<context>` Gitea label that the merge-coordinator treats as a forced-pass for that one check on that one PR -- audited via merge-coordinator log. |

## 11. Success metrics

- **Latency**: 95th percentile time from `pull_request.opened` webhook receipt
  to first `adf/build` status `success` < 5 min on warm cache.
- **Coverage**: 100% of new PRs in `terraphim/terraphim-ai` carry the 5 ADF
  status checks within 60 s of open.
- **Cost**: monthly LLM spend on PR-fan-out agents < 1.5x of pre-Phase-2
  baseline (cron-only).
- **Cache reuse**: SeaweedFS hit rate on `build-runner` >= 80% (matches GH
  Actions baseline).
- **Compute deduplication**: when GH mirror exists, `build-runner` skipped on
  >= 90% of mirrored PRs (Phase 4 only).
- **Listener retirement**: 48 h with zero polling-resolved mentions before
  shutdown (Phase 5 gate).

## 12. First Gitea issue to file

Title: `feat(adf): add Gitea Commit Status API to webhook agent verdicts (Phase 1)`
Labels: `area/orchestrator`, `area/webhook`, `priority/high`
Body: links this plan, scope = Phase 1 only, acceptance = `adf/pr-reviewer`
status appears on next PR opened.

This is the smallest measurable shipment that proves the architecture works.
Phases 2-5 follow as separate Gitea issues, each blocked on its predecessor.

## 13. Out of scope (current plan)

- Replacing GitHub Actions for `terraphim-ai` -- keep it; it's optimal.
- Setting up Gitea Actions runners for any repo.
- ~~ADF agents in non-`terraphim-ai` projects~~ -- now in scope per D5; the
  `pr_dispatch` table is per-project (declared inside
  `conf.d/<project>.toml`), so terraphim + odilo + digital-twins on bigbox
  share one orchestrator instance with independent fan-out lists. See
  `.docs/design-pr-dispatch-per-project.md` (Gitea issue #962).
- Real-time UI for ADF agent state (Gitea's status check UI is enough).
- Replacing `merge-coordinator`'s LLM-based judgement with pure status logic.

## 14. Resolved by AskUserQuestion 2026-04-26

D1-D4 captured at top of plan. One open item remains:

- **Per-agent budget partition**: should the PR-fan-out agents share the
  existing monthly LLM pool, or get their own bucket? Recommendation:
  separate bucket so PR fan-out can be capped without starving cron agents.
  Will surface as the first design question of Phase 2 implementation.
