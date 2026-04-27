# How We Replaced Gitea Actions with AI Agents

**Author**: Alex Mikhalev, CTO Zestic AI  
**Date**: 2026-04-27  
**Status**: shipped

---

## The problem with CI-as-code

Every CI system eventually converges on the same failure mode: a forest of YAML that nobody fully understands, check-boxes that pass without human judgment, and reviewers who merge because the green tick appeared — not because they read the diff.

We ran Gitea Actions for terraphim-ai. It worked. But "cargo clippy passes" is not the same as "this PR is architecturally sound, consistent with requirements, and safe to merge." We wanted agents that actually read the code, not jobs that shell out to cargo.

We had the infrastructure already: the AI Dark Factory (ADF) orchestrator — a Rust service on bigbox that spawns Claude, Kimi, and bash agents in response to Gitea webhook events. What we lacked was wiring.

This post explains the four weeks of work that replaced the Gitea Actions checks with six purpose-built agents, and what we learned along the way.

---

## What we built

Every `pull_request.opened` on `terraphim/terraphim-ai` now dispatches six agents in parallel. Each agent posts a Gitea commit status. All six are required checks before a PR can merge.

```
pull_request.opened
        │
        ▼
  ADF orchestrator (adf-orchestrator.service, bigbox)
        │
        ├─── build-runner ──────► adf/build
        │    (bash + rch exec)    cargo fmt + clippy + test
        │
        ├─── pr-reviewer ───────► adf/pr-reviewer
        │    (claude sonnet)      structural review, confidence score
        │
        ├─── pr-spec-validator ─► adf/spec
        │    (claude sonnet)      requirements traceability, ADR drift
        │
        ├─── pr-security-sentinel► adf/security
        │    (claude sonnet)      CVE, secrets, unsafe-code scan
        │
        ├─── pr-compliance-watchdog► adf/compliance
        │    (claude sonnet)      licence, responsible-AI, supply-chain
        │
        └─── pr-test-guardian ──► adf/test
             (claude sonnet)      coverage, contract, regression risk
```

All six commit statuses are required on `main` (Gitea branch protection). Direct push to `main` is disabled.

---

## The implementation in five phases

### Phase 1: Status reporter

Before we could gate anything, we needed a way for agents to post Gitea commit statuses. We added `set_commit_status` and `post_pending_status` to the orchestrator — a thin wrapper around `POST /api/v1/repos/{o}/{r}/statuses/{sha}` — and wired it into the PR dispatch loop so a `pending` post fires the moment an agent spawns.

### Phase 2: PR fan-out

The orchestrator already handled push webhooks. We extended it with a `[pr_dispatch]` config block:

```toml
[pr_dispatch]
agents_on_pr_open = [
  { name = "build-runner",           context = "adf/build" },
  { name = "pr-reviewer",            context = "adf/pr-reviewer" },
  { name = "pr-spec-validator",      context = "adf/spec" },
  { name = "pr-security-sentinel",   context = "adf/security" },
  { name = "pr-compliance-watchdog", context = "adf/compliance" },
  { name = "pr-test-guardian",       context = "adf/test" },
]
```

The orchestrator's `handle_review_pr` loops over this list and spawns each agent independently. Each agent is gated by the subscription allow-list and per-agent monthly budget before spawning; a gated agent does NOT post `pending` (a hung pending would block the PR forever).

### Phase 3: Build runner

`build-runner` is bash-only — no LLM, deterministic, cheap. It calls `rch exec` to dispatch cargo commands to the local rch worker pool (6 slots, SeaweedFS S3 cache at 82% hit rate):

```bash
/home/alex/.local/bin/rch exec -- cargo fmt --all -- --check
/home/alex/.local/bin/rch exec -- cargo clippy --workspace --all-targets -- -D warnings
/home/alex/.local/bin/rch exec -- cargo test --workspace --no-fail-fast
```

`rch` is already installed at `/home/alex/.local/bin/rch`, with `rchd` running as a user daemon (uptime 50h+). The same binary powers the self-hosted GitHub Actions runners on bigbox.

### Phases 2b–2e: The LLM review agents

Each of the four LLM agents follows the same pattern:
- Read the PR diff from Gitea API
- Apply a domain-specific review skill (`requirements-traceability`, `security-audit`, `responsible-ai`, `testing`)
- Post the review as a Gitea comment
- Post a `success`, `failure`, or `success` with notes as the commit status

A path filter runs before the LLM call. If no security-relevant files changed, `pr-security-sentinel` posts `success` with description "n/a — no security-relevant changes" rather than leaving a `pending` that would block docs-only PRs.

---

## The bugs we hit

### The spawner bug

The single most impactful bug: `SpawnRequest::new(provider, &task_string)` was passing a short runtime summary ("Build/test verdict for PR #999") to the spawner instead of the TOML task body (the 142-line bash script). The bash agent received a one-liner as its script and exited immediately. The LLM agent received 186 characters of context instead of 4000.

Root cause: three call sites in `handle_review_pr` and `handle_push` all passed `&task_string` (runtime summary) instead of `&def.task` (TOML task body). The working mention-trigger path used `&composed_task` correctly.

Fix: swap all three sites to `&def.task`, add `ADF_TASK_SUMMARY` env so scripts can still log the runtime summary:

```rust
// Before (broken)
let mut request = SpawnRequest::new(primary_provider, &task_string);

// After (fixed)
let mut request = SpawnRequest::new(primary_provider, &def.task);
spawn_ctx = spawn_ctx.with_env("ADF_TASK_SUMMARY", task_string.clone());
```

Verification: `task_len=1613` for build-runner and `task_len=4180` for pr-reviewer after the fix, vs `task_len=124` and `task_len=175` before.

### The per-project pr_dispatch gap

`[pr_dispatch]` was originally a global field on `OrchestratorConfig`. But the orchestrator resolved the project from the Gitea repo name (`terraphim-ai`), while the config used `[[projects]] id = "terraphim"`. The mismatch meant agents were never found.

Fix: rename `id` to `"terraphim-ai"` across all 21 agents and the projects block. Then, as the proper long-term fix, move `pr_dispatch` to `IncludeFragment` as a `HashMap<String, PrDispatchConfig>` keyed by project ID (PR #999).

### IncludeFragment rejects pr_dispatch

The `IncludeFragment` TOML parser only accepts a subset of `OrchestratorConfig` fields. `[pr_dispatch]` must live in the top-level `orchestrator.toml`, not in `conf.d/*.toml`. A day of debugging ended with this constraint baked into a locked decision (D5).

### ENOSPC on Mac (target/ = 132 GB)

The pre-commit hook runs `cargo build --workspace` before every commit. With target/ at 132 GB, the Mac hit 100% disk during busy sessions. Fix: `cargo clean -p terraphim_orchestrator` (freed 26 GB). The hook was bypassed with `--no-verify` only when explicitly authorised.

---

## How the branch protection bootstrap works

The first PR cannot pass `adf/build` and `adf/pr-reviewer` until the spawner bug is fixed. The spawner bug fix cannot land until the PR passes `adf/build` and `adf/pr-reviewer`. Classic catch-22.

We resolved it by posting success statuses manually for the spawner-fix PR head SHA, then merging:

```bash
for ctx in adf/build adf/pr-reviewer; do
  curl -X POST ... \
    -d '{"state":"success","context":"'"$ctx"'","description":"bootstrap unblock"}'
done
curl -X POST .../pulls/1021/merge -d '{"Do":"merge"}'
```

After the spawner fix landed, the orchestrator was rebuilt on bigbox and all future PRs get real agent checks.

---

## Operational state after completion

```
adf-orchestrator.service: active (running)
Agent definitions loaded: 32
  - terraphim-ai project: 25 agents
  - odilo project: 2 agents
  - digital-twins project: 2 agents
  - (other): 3 agents

PR fan-out: 6 agents per PR open event
Branch protection: 6 required contexts on main
rch: 1 worker (bigbox-local), 6 slots, rchd uptime 50h+
```

---

## What works now that did not before

| Before | After |
|---|---|
| `cargo clippy passes` = merge | Structural review + requirements check + security scan + compliance + test coverage |
| Review is whoever has time | Sonnet reads every diff, every time |
| Humans post-hoc catch security issues | `pr-security-sentinel` scans every PR, skips docs-only ones |
| Test coverage drift invisible | `pr-test-guardian` flags untested new functions before merge |
| Direct push to main possible | Branch protection enforced; PR-only from 2026-04-27 |

---

## What's next

1. **rch path filter**: build-runner currently runs the full test suite on every PR. Add a changed-file filter so docs-only PRs skip the build.
2. **Confidence escalation**: `pr-reviewer` confidence < 3/5 should create a Gitea discussion thread, not just post a comment.
3. **Per-project fan-out live**: PR #999 (IncludeFragment-based per-project config) is merged. Each project in `conf.d/` can now have its own `[pr_dispatch]` — no global config needed.
4. **Phase 2c–2e agent tuning**: the four LLM agents are wired but their path filters and verdict-parsing logic need one real-world PR cycle each to validate.

---

## Lessons

- **Never pass runtime summaries as task bodies.** The spawner receives a string and hands it to bash or claude. If that string is a one-sentence summary rather than a script, the agent silently succeeds at doing nothing.
- **IncludeFragment is not a full config parser.** Only fields explicitly listed in the `IncludeFragment` struct are accepted. Put global-scope config (mentions, pr_dispatch, workflow) in the top-level file.
- **Bootstrap deadlocks are predictable.** When status checks gate the very PR that fixes the status check system, plan the manual bootstrap before deployment, not after.
- **rch inherits CWD.** `rch exec -- cargo build` works correctly when called from inside the workspace. It SSH-dispatches to the worker, which inherits the caller's working directory. The canonical rch path is `/data/projects/terraphim-ai` (resolves to `/home/alex/projects/terraphim-ai`), but the GITEA_WORKING_DIR (`/home/alex/terraphim-ai`) also works.
