# ADF Agent Fleet Reference

Complete roster of all AI Dark Factory agents across all three projects.
Validated 2026-04-27 by triggering each agent via `adf-ctl` and confirming
meaningful output (see `docs/adf/operations.md` for the trigger method).

## Fleet summary

| Project | Agents | Dispatch path |
|---|---|---|
| terraphim | 22 | cron + mention poll + webhook |
| odilo | 2 | cron + webhook (no mention poll) |
| digital-twins | 2 | cron + webhook (reviewer-2 mention-triggered) |

Maximum concurrent agents: 8 (mention-driven); cron agents are additional.

---

## Project: terraphim

Config: `/opt/ai-dark-factory/conf.d/terraphim.toml`
Repo: `git.terraphim.cloud/terraphim/terraphim-ai`

### Core layer agents

#### log-analyst

| Field | Value |
|---|---|
| Layer | Core |
| Persona | Conduit |
| Skills | (none — task-driven) |
| Model | sonnet (KG fallback) |
| Schedule | hourly |
| Max wall | 7200s |

**Role:** Queries the Quickwit `adf-logs` index for failure patterns, clusters
by root cause, identifies degrading agents, and posts structured analysis to
issue #328. Currently degraded due to Quickwit telemetry blackout (P0 since
2026-04-21); exits success but cannot query the index.

**Validated output:** exit_class=success, 299s, Conduit persona, isolated worktree.

---

#### security-sentinel

| Field | Value |
|---|---|
| Layer | Core |
| Persona | Vigil |
| Skills | security-audit, via-negativa-analysis, disciplined-verification, disciplined-validation |
| Model | sonnet (KG fallback) |
| Schedule | every 2h |
| Max wall | 7200s |

**Role:** Runs `cargo audit`, secret scan, unsafe block audit, UBS static
analysis, port scan. Classifies findings P0-P3. Creates Gitea issues for
P0/P1. Writes scan artefacts to `/tmp/security-sentinel-*.txt`.

**Validated output:** PASS verdict on issue #925 — licence clean, supply chain
4 CVEs tracked, zero secrets, zero unsafe blocks, GDPR-compatible persistence.

---

#### compliance-watchdog

| Field | Value |
|---|---|
| Layer | Core |
| Persona | Vigil |
| Skills | disciplined-research, disciplined-verification, security-audit, responsible-ai, via-negativa-analysis |
| CLI | opencode (kimi fallback) |
| Schedule | every 5h |
| Max wall | 7200s |

**Role:** `cargo deny check licenses`, supply chain advisory scan, GDPR/data
handling pattern audit. Posts PASS/FAIL verdict to the most-ready issue.

**Validated output:** PASS verdict posted to #925 — all licences documented,
zero injection vectors, GDPR-compatible.

---

#### drift-detector

| Field | Value |
|---|---|
| Layer | Core |
| Persona | Conduit |
| Skills | (task-driven) |
| Model | sonnet |
| Schedule | every 2h |
| Max wall | 7200s |

**Role:** Compares running `orchestrator.toml` against git-tracked version,
checks systemd service states, SSH key permissions. Creates a Gitea issue when
drift is detected.

**Validated output:** Created issues #989 "Config drift detected 2026-04-27"
and #993 "config-drift-persistence" — real drift findings from configuration
changes not reflected in git.

---

#### runtime-guardian

| Field | Value |
|---|---|
| Layer | Core |
| Persona | Ferrox |
| Skills | (task-driven) |
| Model | sonnet |
| Schedule | every 30 min |
| Max wall | 1200s |

**Role:** System health monitoring — disk, memory, process counts, swap usage.
Escalates anomalies to Gitea issues.

**Validated output:** exit_class=success, 298s. Port scan and process inventory
written to scan files on each run.

---

#### meta-learning

| Field | Value |
|---|---|
| Layer | Core |
| Persona | Mneme |
| Skills | (task-driven) |
| Model | sonnet |
| Schedule | every 2h |
| Max wall | 7200s |

**Role:** Synthesises learnings from recent agent sessions; writes fleet health
reports to wiki; updates `terraphim-agent learn` database with new patterns.

**Validated output:** Published `Fleet-Health-20260426-Mneme` wiki page
summarising fleet status, agent success rates, and remediation recommendations.

---

#### upstream-synchronizer

| Field | Value |
|---|---|
| Layer | Core |
| Persona | Carthos |
| Skills | (task-driven) |
| Model | haiku |
| Schedule | nightly |
| Max wall | 1200s |

**Role:** Checks upstream GitHub mirrors, evaluates applicability of upstream
changes, creates sync issues when upstream has diverged.

**Validated output:** exit_class=success, 294s. No open sync issues today
(upstream in sync).

---

#### repo-steward

| Field | Value |
|---|---|
| Layer | Growth |
| Persona | Carthos |
| Skills | (task-driven) |
| Model | sonnet |
| Schedule | every 2h |
| Max wall | 1200s |

**Role:** Monitors repo health — stale branches, unresolved review requests,
open PRs without assignees, recurring failure themes. Creates labelled Gitea
issues with `[Repo Stewardship]` prefix.

**Validated output:** Created #1007 "ai-test-regression-cycle", #1008
"security-port-exposure-unresolved", #1009 "orchestrator-permission-denied-reports"
— all sourced from observed fleet failure patterns.

---

#### spec-validator

| Field | Value |
|---|---|
| Layer | Core |
| Persona | Echo |
| Skills | disciplined-research, disciplined-verification, disciplined-validation, scope-gate |
| Model | sonnet |
| Schedule | every 30 min |
| Max wall | 7200s |

**Role:** Scope-gate quality gate. Uses Haiku to assess whether open issues have
clear bounded scope and acceptance criteria before agents pick them up. Posts
clarification requests on issues that fail the gate.

**Validated output:** Long-running (50+ min observed) when processing a large
issue backlog. Scope-gate prevents agents from working on underspecified issues.

---

#### roadmap-planner

| Field | Value |
|---|---|
| Layer | Core |
| Persona | Carthos |
| Skills | (task-driven) |
| Model | sonnet |
| Schedule | every 2h |
| Max wall | 1200s |

**Role:** Reads WIGs from `progress.md`, runs 5/25 essentialism filter on open
backlog, writes Compound-RICE roadmap report to
`/opt/ai-dark-factory/reports/roadmap-YYYYMMDD-HHmm.md`.

**Validated output:** 3 roadmap reports written today: `roadmap-20260427-0903.md`,
`roadmap-20260427-1000.md`, `roadmap-20260427-1103.md`.

---

#### product-owner

| Field | Value |
|---|---|
| Layer | Core |
| Persona | Themis |
| Skills | (task-driven) |
| Model | opus |
| Schedule | every 2h |
| Max wall | 7200s |

**Role:** Full Compound-RICE + Essentialism cycle. 5/25 essentialism filter on
backlog, RICE scoring on top 5, creates up to 2 new issues with WIG alignment
and mini-UAT acceptance criteria. Posts scoring summary as Gitea comment.

**Note:** Themis persona was not loading until 2026-04-27 restart (file existed
on disk but orchestrator had started before the file was written). Fixed by
restarting orchestrator.

**Validated output:** success, 584s — Compound-RICE cycle completed with Themis.
Previously producing 67% empty_success when running bare task (no persona).

---

#### meta-coordinator

| Field | Value |
|---|---|
| Layer | Core |
| Persona | Ferrox |
| Skills | disciplined-research, disciplined-verification, devops, quality-oversight, dispatch, scope-gate |
| Model | sonnet |
| Schedule | every 30 min |
| Max wall | 1200s |

**Role:** Scope gate + agent dispatcher. Picks highest-PageRank ready issue,
runs Haiku scope assessment, dispatches the right specialist agent via
`@adf:<role>` mention, or posts clarification request if scope is unclear.

**Note:** max_cpu_seconds bumped 300 → 1200 on 2026-04-27. Was being killed 9
times/day due to mismatched timeout.

**Validated output:** Coordinated complete Shopify Commerce twin delivery on
digital-twins #45 — design doc, 32/32 tests, reviewer GO, PR #46 merged.

---

#### merge-coordinator

| Field | Value |
|---|---|
| Layer | Growth |
| Persona | Conduit |
| Skills | (task-driven) |
| Model | sonnet |
| Schedule | every 2h |
| Max wall | 7200s |

**Role:** Review gate aggregator. Reads all reviewer verdicts on a PR, merges
when all required reviewers post GO, posts merge report, closes linked issue,
deletes branch.

**Validated output:** Merged digital-twins PR #48 — reviewer NO-GO → developer
fixes all 6 items → reviewer GO → merge at commit `a1a1afa`. Branch deleted,
issue #47 closed.

---

#### reviewer

| Field | Value |
|---|---|
| Layer | Core |
| Persona | Ferrox |
| Skills | (task-driven) |
| Model | sonnet |
| Schedule | mention-driven |
| Max wall | 7200s |

**Role:** Code review against quality gates. Posts structured verdicts with
GO/NO-GO, identified blockers (C-N), and important items (I-N).

**Validated output:** NO-GO verdict on digital-twins #47 catching two real
blockers — C-1 hardcoded webhook secret, C-2 private EventBus. Agent posted
with its own Gitea token (`reviewer` login, not `root`).

---

#### quality-coordinator

| Field | Value |
|---|---|
| Layer | Growth |
| Persona | Echo |
| Skills | (task-driven) |
| Model | sonnet |
| Schedule | every 2h |
| Max wall | 7200s |

**Role:** End-to-end quality pipeline coordination across the fleet.

**Validated output:** exit_class=success, running 35+ min on the day of validation.

---

#### documentation-generator

| Field | Value |
|---|---|
| Layer | Core |
| Persona | Lux |
| Skills | (task-driven) |
| Model | sonnet |
| Schedule | every 2h |
| Max wall | 7200s |

**Role:** Adds missing module-level documentation to Rust crates, updates
CHANGELOG. Commits directly to the working branch.

**Validated output:** Three commits today — "docs: add module-level docs to 8
crates and update CHANGELOG", "docs: add module-level docs to 7 crates" (×2).

---

#### developer

| Field | Value |
|---|---|
| Layer | Core |
| Persona | Echo |
| Skills | disciplined-research, disciplined-design, disciplined-implementation, disciplined-verification, disciplined-validation, implementation, rust-development, rust-mastery, testing |
| Model | sonnet |
| Schedule | mention-driven |
| Max wall | 7200s |

**Role:** Full implementation agent. Picks highest-PageRank ready issue,
implements using the disciplined V-model skill chain, commits, raises PR.

**Validated output:** Committed `d3fd605d8 fix(agent): wire
pagination+token_budget into search response and fix test alignments Refs #672`
— real code change fixing agent search response contract, 897s runtime.

---

#### implementation-swarm

| Field | Value |
|---|---|
| Layer | Core |
| Persona | Echo |
| Skills | disciplined-research, disciplined-design, disciplined-implementation, disciplined-verification, disciplined-validation, implementation, rust-development, rust-mastery, testing |
| Model | sonnet |
| Schedule | mention-driven |
| Max wall | 7200s |

**Role:** Parallel implementation capacity — same V-model skill chain as
developer, dispatched to additional issues when developer is already active.

**Validated output:** Committed `c193b3aa3 docs: update CHANGELOG with recent
commits` — maintains project history alongside implementation work.

---

#### test-guardian

| Field | Value |
|---|---|
| Layer | Core |
| Persona | Vigil |
| Skills | (task-driven) |
| Model | sonnet |
| Schedule | mention-driven |
| Max wall | 7200s |

**Role:** Full test suite execution. Runs `cargo test --workspace`, analyses
failures, posts structured report to Gitea. Long-running by design.

**Validated output:** exit_class=success, 7140s (119 min) — a comprehensive test
run posting output to issue #673. Matched_patterns=["timeout"] is a text
heuristic (agent mentions timeout in its analysis), not a real process timeout.

---

#### product-development

| Field | Value |
|---|---|
| Layer | Core |
| Persona | Themis |
| Skills | (task-driven) |
| Model | haiku |
| Schedule | mention-driven |
| Max wall | 7200s |

**Role:** Product feature development — creates well-scoped Gitea issues with
acceptance criteria from product direction, manages feature decomposition.

**Validated output:** Created 5 issues today with acceptance criteria (Tasks
1.1–1.6 for robot mode and token budget management). Issues include Compound-RICE
estimates and testable Given/When/Then blocks.

---

#### browser-qa

| Field | Value |
|---|---|
| Layer | Growth |
| Persona | Echo |
| Skills | disciplined-research, disciplined-verification, testing, acceptance-testing, dev-browser |
| Model | sonnet |
| Schedule | mention-driven |
| Max wall | 7200s |

**Role:** Browser-based acceptance testing using the dev-browser skill (Playwright).
Reads scenario file from `/opt/ai-dark-factory/scenarios/browser-qa-current.md`,
executes each test, writes JSON and markdown reports to
`/opt/ai-dark-factory/reports/browser-qa-*.`.

**Validated output:** exit_class=success, 298s. No prior reports on disk —
scenarios reference localhost:3000/localhost:8080 which may not be running in
cron context. Runs cleanly when invoked; produces reports when the target
service is live.

---

## Project: odilo

Config: `/opt/ai-dark-factory/conf.d/odilo.toml`
Repo: `git.terraphim.cloud/zestic-ai/odilo`

No `[projects.mentions]` section — both agents are cron-driven or triggered
via webhook from `odilo-developer`'s task output.

---

#### odilo-developer

| Field | Value |
|---|---|
| Layer | Core |
| Persona | Lux |
| Skills | disciplined-research, disciplined-design, disciplined-implementation, testing |
| Model | sonnet |
| Schedule | 01:00–09:00 UTC daily |
| Max wall | 7200s |

**Role:** Implementation agent for the Odilo content ingestion pipeline.
Picks highest-PageRank ready issue from `zestic-ai/odilo`, implements using
disciplined V-model, commits, raises PR, then posts `@adf:odilo-reviewer`.

**Validated output:** Completed Phase 1-3 of Epic #51 — sales-catalogue xlsx
import, EPUB adapter, bilingual runner. Merged PR #17 with 23/23 requirements
traced, 8/8 V-model verification gates PASS. Runs 6/9 successful today (3
empty_success = no ready issues).

---

#### odilo-reviewer

| Field | Value |
|---|---|
| Layer | Core |
| Persona | Vigil |
| Skills | code-review, testing |
| CLI | opencode (kimi fallback) |
| Schedule | none (webhook-triggered by odilo-developer) |
| Max wall | 7200s |

**Role:** Code review for odilo PRs. Triggered when odilo-developer posts
`@adf:odilo-reviewer please review PR` as a Gitea comment.

**Trigger note:** Requires webhook from `zestic-ai/odilo` repo, or direct
webhook POST with `"repository": {"full_name": "zestic-ai/odilo"}`.

**Validated output:** Spawned 2026-04-27 via direct webhook — Vigil persona,
2 skills, isolated worktree. 5 open PRs (#61-65) available for review.

---

## Project: digital-twins

Config: `/opt/ai-dark-factory/conf.d/digital-twins.toml`
Repo: `git.terraphim.cloud/terraphim/digital-twins`

---

#### reviewer-2

| Field | Value |
|---|---|
| Layer | Core |
| Persona | conduit |
| Skills | disciplined-research, disciplined-design, disciplined-implementation, testing |
| Model | sonnet |
| Schedule | mention-driven |
| Max wall | 7200s |

**Role:** Second code review pass for digital-twins project. Same verdict
format as `reviewer` — GO/NO-GO with classified blockers.

**Validated output:** Posted verification report to digital-twins #45 —
cargo build clean, clippy clean, 14/14 unit tests, 32/32 SDK tests, all 6
original review blockers resolved in commit `461cf3d`. Used own Gitea token
(`reviewer-2` login).

---

## Dispatch pipeline

```
                       ┌─────────────────────────────┐
                       │   Reconciliation tick        │
                       │   (every 300s)               │
                       └──────────┬──────────────────┘
                                  │
            ┌─────────────────────┼────────────────────────┐
            │                     │                        │
     Cron schedule          Mention poll             Webhook queue
   (per-agent cron)    (every 2nd tick per        (immediate on
                        project, cursor-          webhook receipt)
                        tracked Gitea API)
            │                     │                        │
            └─────────────────────┴────────────────────────┘
                                  │
                      should_skip_dispatch?
                      - issue_number == 0 → skip ALL dedup
                      - active_agents guard
                      - Gitea assignee check
                                  │
                      resolve_mention(project, agent_name, agents)
                      - terraphim project: searches terraphim agents
                      - odilo project: searches odilo agents
                      - CLI trigger always uses project=None → legacy
                                  │
                      PersonaRegistry.get(persona_name)  [case-insensitive]
                      MetapromptRenderer.compose_prompt(persona, task)
                                  │
                      SkillChain: inject each SKILL.md into prompt
                                  │
                      RoutingDecisionEngine.decide_route(ctx)
                      [see docs/adf/model-selection-and-spawn.md]
                                  │
                      git worktree add /tmp/adf-worktrees/<agent>-<uuid>
                                  │
                      spawn_with_fallback → tokio::process::Command
                                  │
                      poll_wall_timeouts (per tick)
                                  │
                      ExitClassifier → exit_class
                                  │
                      OutputPoster → Gitea comment (per-agent token)
                                  │
                      git worktree remove (cleanup)
```

---

## Known operational issues (as of 2026-04-27)

### Stale worktree accumulation

**Problem:** When the orchestrator restarts mid-run, orphaned agent processes
leave their worktrees on disk. Git worktree registrations also accumulate.
856 stale worktrees (123 GB) observed before cleanup on 2026-04-27.

**Cause:** The orchestrator only cleans worktrees it tracks in `active_agents`.
Agents running in a previous process instance are invisible to the new one.

**Fix (applied 2026-04-27):** Manual cleanup:
```bash
# On bigbox:
for dir in /tmp/adf-worktrees/*/; do
  git -C /home/alex/terraphim-ai worktree remove --force "$dir" 2>/dev/null
done
git -C /home/alex/terraphim-ai worktree prune
```

**Permanent fix needed:** On startup, the orchestrator should detect and prune
stale worktrees not in its active agent map.

---

### CLI trigger drops cross-project agents

**Problem:** `adf-ctl trigger <agent>` hardcodes `"repository": {"full_name":
"terraphim/terraphim-ai"}` in the webhook payload. Agents in the `odilo` or
`digital-twins` project configs are not found by `resolve_mention` when the
project resolves to `terraphim`.

**Workaround:** For cross-project agents, either:
1. Post a Gitea mention comment in the agent's native repo
2. Directly POST the webhook with the correct `repository.full_name` via SSH

**Permanent fix needed:** Add `--repo <owner/repo>` flag to `adf-ctl trigger`.

---

### Top-level [mentions] required for webhook dispatch

**Problem:** The orchestrator's `handle_webhook_dispatch` checks
`self.config.mentions` (top-level) before processing CLI-triggered webhooks.
If only project-level `[projects.mentions]` is configured, all webhook
dispatches return silently.

**Fix (applied 2026-04-27):** Added top-level `[mentions]` section to
`orchestrator.toml`:
```toml
[mentions]
poll_modulo = 2
max_dispatches_per_tick = 3
max_concurrent_mention_agents = 8
```

**Permanent fix needed:** The fix from issue #951 (branch
`task/860-f1-2-exit-codes`) should be merged to main and the deployed binary
rebuilt. The fix makes `handle_webhook_dispatch` fall back to project-level
mentions config when top-level is absent.
