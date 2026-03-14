# Symphony Quickstart Guide

Symphony is a daemon that watches your issue tracker, picks up work automatically, and runs a coding agent against each issue in an isolated workspace. It polls, dispatches, retries, and reports -- you configure it with a single `WORKFLOW.md` file.

## What Symphony Does

```
  Issue Tracker           Symphony                  Coding Agent
  (Gitea/Linear)          Orchestrator              (codex app-server)
  +-----------+      +------------------+      +------------------+
  | Todo      | ---> | Poll & Dispatch  | ---> | Workspace: MT-1  |
  | In Prog   |      | Retry & Backoff  |      | Workspace: MT-2  |
  | Done      | <--- | Reconcile        | <--- | Workspace: MT-3  |
  +-----------+      +------------------+      +------------------+
                            |
                     +------+------+
                     | Dashboard   |
                     | /api/v1/... |
                     +-------------+
```

**Poll** -- Symphony fetches open issues from your tracker every 30 seconds (configurable).

**Dispatch** -- Eligible issues are sorted by priority (lowest number first), then by age (oldest first). Each gets an isolated workspace directory and a rendered prompt.

**Run** -- A coding agent session is spawned per issue. Symphony manages the full lifecycle: handshake, turns, approvals, and shutdown.

**Retry** -- On failure, issues are retried with exponential backoff (10s, 20s, 40s, ... capped at 5 minutes). Clean exits get a quick 1-second continuation retry.

**Reconcile** -- Every tick, Symphony checks for stalled sessions and refreshes issue states from the tracker. Terminal issues are cleaned up automatically.

## 1. Create Your WORKFLOW.md

This single file configures everything: which tracker to use, how many agents to run, what prompt to give them, and what hooks to run.

### Gitea Example

```yaml
---
tracker:
  kind: gitea
  owner: terraphim
  repo: agent-tasks
  api_key: $GITEA_TOKEN
  active_states:
    - Todo
    - In Progress
  terminal_states:
    - Done
    - Closed
    - Cancelled

polling:
  interval_ms: 30000

workspace:
  root: ~/symphony-workspaces

hooks:
  after_create: "git clone https://git.terraphim.cloud/terraphim/agent-tasks.git ."
  before_run: "git pull origin main"
  timeout_ms: 60000

agent:
  max_concurrent_agents: 3
  max_turns: 10

codex:
  command: "codex app-server"
  stall_timeout_ms: 300000
---
You are working on issue {{ issue.identifier }}: {{ issue.title }}.

{% if issue.description %}
## Issue Description

{{ issue.description }}
{% endif %}

## Instructions

1. Read the issue carefully.
2. Examine the relevant code in this workspace.
3. Implement the required changes following project standards.
4. Write tests to verify your changes.
5. Commit with a message referencing {{ issue.identifier }}.

{% if attempt %}
This is retry attempt {{ attempt }}. Review previous work and continue.
{% endif %}
```

### Linear Example

```yaml
---
tracker:
  kind: linear
  project_slug: my-project
  api_key: $LINEAR_API_KEY

agent:
  max_concurrent_agents: 5
  max_turns: 20
---
You are working on {{ issue.identifier }}: {{ issue.title }}.
{{ issue.description }}
```

## 2. Set Up Credentials

**Gitea:**
```bash
# Set your Gitea token (or use 1Password)
export GITEA_TOKEN=$(op read "op://TerraphimPlatform/gitea-test-token/credential")
```

**Linear:**
```bash
# Set your Linear token
export LINEAR_API_KEY=$(op read "op://YourVault/linear-token/credential")
```

The `$VARIABLE` syntax in `api_key` fields resolves from your environment at runtime.

## 3. Build and Run

```bash
cd crates/terraphim_symphony

# Build
cargo build --release --bin symphony

# Run
./target/release/symphony ./WORKFLOW.md

# Run with the HTTP dashboard
cargo build --release --bin symphony --features api
./target/release/symphony ./WORKFLOW.md --port 8080
```

Open `http://localhost:8080` to see the dashboard showing running sessions, retry queue, and token usage.

## 4. What Happens Next

Once running, Symphony operates autonomously:

1. **Fetches issues** from your tracker matching `active_states`
2. **Sorts them** -- priority 1 before priority 5; `None` goes last; oldest first on ties
3. **Creates workspaces** -- one directory per issue under `workspace.root`
4. **Runs hooks** -- `after_create` for new workspaces, `before_run` before each attempt
5. **Spawns agents** -- up to `max_concurrent_agents` in parallel
6. **Monitors progress** -- tracks turns, tokens, and last activity per session
7. **Detects stalls** -- kills sessions with no activity for `stall_timeout_ms`
8. **Retries on failure** -- exponential backoff: 10s, 20s, 40s, 80s... capped at 5 minutes
9. **Cleans up** -- removes workspaces for issues that reach terminal states

## Configuration Reference

### Tracker Settings

| Setting | Required | Default | Description |
|---------|----------|---------|-------------|
| `tracker.kind` | Yes | -- | `"gitea"` or `"linear"` |
| `tracker.api_key` | Yes | -- | API token (use `$ENV_VAR` to reference environment variables) |
| `tracker.owner` | Gitea only | -- | Repository owner |
| `tracker.repo` | Gitea only | -- | Repository name |
| `tracker.project_slug` | Linear only | -- | Linear project slug |
| `tracker.endpoint` | No | Auto-detected | API base URL |
| `tracker.active_states` | No | `[Todo, In Progress]` | States eligible for dispatch |
| `tracker.terminal_states` | No | `[Closed, Cancelled, Canceled, Duplicate, Done]` | States considered finished |

### Polling

| Setting | Default | Description |
|---------|---------|-------------|
| `polling.interval_ms` | `30000` (30s) | How often to check for new issues |

### Workspace

| Setting | Default | Description |
|---------|---------|-------------|
| `workspace.root` | `$TMPDIR/symphony_workspaces` | Root directory for per-issue workspaces. Supports `~` and `$VAR` expansion |

### Hooks

Shell scripts executed at workspace lifecycle points. All run with `sh -lc` in the workspace directory.

| Setting | When | On Failure |
|---------|------|-----------|
| `hooks.after_create` | New workspace created | Workspace removed, dispatch aborted |
| `hooks.before_run` | Before each agent attempt | Attempt aborted, retry scheduled |
| `hooks.after_run` | After each attempt (success or failure) | Logged and ignored |
| `hooks.before_remove` | Before workspace deletion | Logged and ignored |
| `hooks.timeout_ms` | -- | Default: `60000` (60s). Hook execution timeout |

### Agent

| Setting | Default | Description |
|---------|---------|-------------|
| `agent.max_concurrent_agents` | `10` | Maximum parallel agent sessions |
| `agent.max_turns` | `20` | Maximum turns per agent session |
| `agent.max_retry_backoff_ms` | `300000` (5 min) | Cap for exponential retry backoff |
| `agent.max_concurrent_agents_by_state` | -- | Per-state concurrency limits (map of state -> limit) |

### Codex (Agent Process)

| Setting | Default | Description |
|---------|---------|-------------|
| `codex.command` | `"codex app-server"` | Shell command to start the coding agent |
| `codex.turn_timeout_ms` | `3600000` (1 hour) | Maximum time for a single turn |
| `codex.read_timeout_ms` | `5000` (5s) | Timeout for handshake responses |
| `codex.stall_timeout_ms` | `300000` (5 min) | Kill session after this long with no activity. Set to `-1` to disable |

### Server (Optional)

| Setting | Default | Description |
|---------|---------|-------------|
| `server.port` | -- | HTTP dashboard port (requires `api` feature). Overridden by `--port` CLI flag |

## Prompt Template Variables

The prompt body uses [Liquid](https://shopify.github.io/liquid/) syntax with strict mode (unknown variables cause errors).

| Variable | Type | Description |
|----------|------|-------------|
| `issue.identifier` | String | e.g. `"terraphim/agent-tasks#42"` or `"MT-42"` |
| `issue.title` | String | Issue title |
| `issue.description` | String or nil | Issue body/description |
| `issue.state` | String | Current state (e.g. `"Todo"`) |
| `issue.priority` | Integer or nil | Priority number (lower = higher priority) |
| `issue.url` | String or nil | Web URL to the issue |
| `issue.labels` | Array of strings | Labels (normalised to lowercase) |
| `issue.branch_name` | String or nil | Suggested branch name |
| `attempt` | Integer or nil | Retry attempt number (nil on first try) |

## Dispatch Rules

Symphony dispatches issues based on these eligibility rules (all must pass):

1. Issue has `id`, `identifier`, `title`, and `state` fields
2. State is in `active_states` (case-insensitive match)
3. State is not in `terminal_states`
4. Issue is not already running or claimed for retry
5. Global concurrency limit not reached
6. Per-state concurrency limit not reached (if configured)
7. **Todo blocker rule**: if state is "Todo" and the issue has blockers, all blockers must be in terminal states

Eligible issues are sorted: priority ascending (nil last), then oldest first, then identifier alphabetically.

## HTTP Dashboard

Build with `--features api` and pass `--port`:

```bash
./target/release/symphony ./WORKFLOW.md --port 8080
```

| Endpoint | Method | Description |
|----------|--------|-------------|
| `/` | GET | HTML dashboard with running sessions, retry queue, token totals |
| `/api/v1/state` | GET | Full orchestrator state as JSON |
| `/api/v1/{identifier}` | GET | Single issue details (running or retrying), or 404 |
| `/api/v1/refresh` | POST | Trigger immediate poll cycle (returns 202) |

### Example: Check State

```bash
curl http://localhost:8080/api/v1/state | jq .
```

```json
{
  "generated_at": "2026-03-14T12:00:00Z",
  "counts": { "running": 2, "retrying": 1 },
  "running": [
    {
      "issue_id": "42",
      "issue_identifier": "terraphim/agent-tasks#42",
      "state": "In Progress",
      "session_id": "thread-1-turn-1",
      "turn_count": 3,
      "last_event": "turn_completed",
      "started_at": "2026-03-14T11:55:00Z"
    }
  ],
  "retrying": [
    {
      "issue_id": "99",
      "issue_identifier": "terraphim/agent-tasks#99",
      "attempt": 2,
      "error": "turn timeout after 3600000ms"
    }
  ],
  "codex_totals": {
    "input_tokens": 15000,
    "output_tokens": 8000,
    "total_tokens": 23000,
    "seconds_running": 342.5
  }
}
```

## Hot Reload

WORKFLOW.md is watched for changes (via `file-watch` feature, enabled by default). When you edit the file:

- New config values take effect on the next poll tick
- Invalid changes are rejected; the last valid config is kept
- Running sessions are not interrupted

## Troubleshooting

**"validation failed: tracker.kind is required"** -- Your WORKFLOW.md front matter is missing the `tracker.kind` field. Add `tracker:\n  kind: gitea` (or `linear`).

**"authentication missing"** -- Set the appropriate environment variable (`GITEA_TOKEN` or `LINEAR_API_KEY`) before running Symphony.

**Issues not being picked up** -- Check that issue states match your `active_states` list (comparison is case-insensitive). For Gitea, issues without explicit state labels default to "Todo" when open.

**Agent sessions timing out** -- Increase `codex.turn_timeout_ms` or check that the agent command is correct and the binary is on your PATH.

**Too many retries** -- Check the dashboard or logs. Increase `agent.max_retry_backoff_ms` to space out retries, or fix the underlying issue causing failures.

**Stall detection killing sessions** -- Increase `codex.stall_timeout_ms` or set it to `-1` to disable. Some long-running agent operations may not emit events frequently.
