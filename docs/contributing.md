# Contributing to Terraphim-AI

Welcome! This document captures the conventions and pitfalls that catch
contributors. It's the post-incident record of lessons learned from real
merge-queue and rebase disasters on this repo, plus the day-to-day
mechanics every PR author needs.

## Quick start

1. **Fork or branch** from `main`. The default branch is `main`.
2. **Format and lint locally before pushing**:
   ```bash
   cargo fmt --all -- --check       # canonical; native-ci runs this
   cargo clippy --workspace --all-targets -- -D warnings
   ```
   CI runs both gates. Push only when they pass.
3. **Required status checks** (must be green before merge):
   - `native-ci / build (pull_request)` — the full build lane
   - `native-clippy / lint (pull_request)` — clippy in isolation (added
     2026-09-01, Phase D; see "Lessons learned" below)
4. **Push your branch**. GitHub-style PR flow applies: push triggers
   `pull_request_sync` webhook → Gitea Actions runner → status contexts.

## Lessons learned (must-read for orchestrator work)

### L1. The `#3303` workspace unification renamed and added orchestrator symbols

**Do NOT forward-port an orchestrator PR across the #3303 workspace
unification refactor by taking HEAD on conflict-resolved files.** The
mechanical-rebase approach SILENTLY removes the symbols that later PR
commits depend on, and the resulting cargo clippy run fails with ~66 lib
+ 69 lib-test errors — most of which are `unresolved import`,
`cannot find field`, and `no method named` for symbols that were
uniformly renamed.

The renamed/added symbol set (as of #3303, captured from the rebase
failure of PR #3279 on 2026-09-01):

| Symbol | Module | Notes |
|---|---|---|
| `gate_output` | `pr_gate_result` | new submodule |
| `GateOutputError` | `pr_gate_result::gate_output` | new enum |
| `start_agent_settlement` | crate root | new top-level fn |
| `settling_agents` | `AgentOrchestrator` | new field |
| `ready_handoffs` | `AgentOrchestrator` | new field |
| `TerminalHandoffTestHook` | crate root | new test-only type |
| `ManagedAgent.output_drain` | `ManagedAgent` | new field |
| `ManagedAgent.executed_invocation` | `ManagedAgent` | new field |

**If your PR predates #3303 and touches `crates/terraphim_orchestrator/src/`,
forward-porting is NOT a mechanical rebase.** Get a developer who
understands the #3303 refactor to look at it. The rebase will appear to
succeed (no conflict markers after taking HEAD on the renamed files)
but clippy will fail in the verification step.

Verify with:
```bash
cargo clippy -p terraphim_orchestrator --lib --no-deps -- -D warnings
```
NOT just `cargo check` — check does not warn on missing fields, but
clippy with `-D warnings` does.

### L2. `cargo fmt --all -- --check` is the canonical pre-commit

Rust 1.97's rustfmt regressed on multi-line `if let Some(t) = ...` blocks
that split across three lines, demanding they be collapsed to one. The
native-ci runner enforces the canonical form. If your local `cargo fmt
-p <crate>` says clean but CI fails, run the canonical command:

```bash
cargo fmt --all -- --check                       # check
cargo fmt --all && git add -A && git commit --amend --no-edit  # fix-on-defect
```

### L3. The ADF webhook is the upstream cause of stale `mergeable`

If a PR shows `mergeable=False` even after `workflow_dispatch` returns
success, the cause is almost always that the **required check
`native-ci / build (pull_request)` has not been posted**. That context
fires only on the `pull_request` webhook event, and if the ADF webhook
receiver (172.18.0.1:9091) is down, the event never reaches the runner.

**Workaround for admins only** (root token required):
```bash
# 1. Trigger workflow_dispatch to refresh status
curl -X POST -H "Authorization: token $GITEA_TOKEN" -H "Content-Type: application/json" \
  -d '{"ref":"refs/heads/<branch>"}' \
  "https://git.terraphim.cloud/api/v1/repos/terraphim/terraphim-ai/actions/workflows/native-ci.yml/dispatches"

# 2. Wait for the run to complete (~3-4 minutes)

# 3. Manually post the required status
curl -X POST -H "Authorization: token $GITEA_TOKEN" -H "Content-Type: application/json" \
  -d '{"context":"native-ci / build (pull_request)","state":"success","description":"manual post"}' \
  "https://git.terraphim.cloud/api/v1/repos/terraphim/terraphim-ai/statuses/<head_sha>"
```

Regular contributors should just **request a re-run** via a comment
`/rebase` on the PR — that triggers Gitea's `pull_request_sync` event.

## Code review

This repo uses **reviewer-required** checks via the `kairo` agent for
non-trivial changes. Every PR should:

1. Have a `Refs #<issue>` line in the commit body
2. Pass `cargo fmt`, `cargo clippy`, `cargo build`, `cargo test --workspace --lib`
3. Include a short test strategy in the PR body
4. Reference any upstream polyrepo crate changes (this repo depends on
   `terraphim-agents`, `terraphim-clients`, `terraphim-linear`, etc.)

## CI lanes

| Workflow | Context | Purpose | Trigger |
|---|---|---|---|
| `native-ci` | `native-ci / build ({event})` | Full build + test | push, pull_request, workflow_dispatch |
| `native-clippy` | `native-clippy / lint ({event})` | Clippy in isolation | push, pull_request, workflow_dispatch |
| `runner-health` | `runner_health / heartbeat ({event})` | Heartbeat ping | schedule (15min), workflow_dispatch |

The runner is `terraphim-native` self-hosted on bigbox. Webhook delivery
goes to the ADF agent at `172.18.0.1:9091` — if `nc -zv 172.18.0.1 9091`
from inside the Gitea container returns "NO_CONNECT", the `pull_request`
events are dropping, and you'll need the admin workaround above.

## Workflow permission model

The runner uses an allowlist of interpreter commands (`bash`, `cargo`,
`make`, `bun`, etc.). Anything else (`docker`, `curl`, `python`) is denied
before execution. Repository scripts must be invoked through the
allowlisted interpreter (`bash ./scripts/x.sh`), never by path
(`./scripts/x.sh` is rejected).

See `crates/terraphim_gitea_runner/default_policy.md` for the full
allow/deny list and rationale.
