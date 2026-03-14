# Symphony Orchestrator

Symphony is an autonomous daemon that watches your issue tracker, picks up work, and runs a coding agent against each issue in an isolated workspace. It polls, dispatches, retries, and reports -- you configure it with a single `WORKFLOW.md` file.

## How It Works

```
  Issue Tracker           Symphony                  Coding Agent
  (Gitea/Linear)          Orchestrator              (Claude Code / Codex)
  +-----------+      +------------------+      +------------------+
  | Todo      | ---> | Poll & Dispatch  | ---> | Workspace: #1    |
  | In Prog   |      | Retry & Backoff  |      | Workspace: #2    |
  | Done      | <--- | Reconcile        | <--- | Workspace: #3    |
  +-----------+      +------------------+      +------------------+
                            |
                     +------+------+
                     | Dashboard   |
                     | /api/v1/... |
                     +-------------+
```

1. **Poll** -- Fetches open issues from your tracker every 30 seconds (configurable)
2. **Sort** -- Eligible issues sorted by priority (lowest first), then age (oldest first)
3. **Dispatch** -- Each issue gets an isolated workspace directory and a rendered prompt
4. **Run** -- A coding agent session is spawned per issue
5. **Monitor** -- Tracks turns, tokens, and last activity per session
6. **Retry** -- Failed sessions retried with exponential backoff (10s, 20s, 40s... capped at 5 min)
7. **Reconcile** -- Stalled sessions killed, terminal issues cleaned up

## Runners

### Claude Code Runner

Uses `claude -p` (Claude Code CLI) as a single-shot invocation per issue. No handshake, no approval flow -- fire-and-forget with retry on failure. Parses NDJSON event stream for turn counts, token usage, and errors.

```yaml
agent:
  runner: claude-code
  max_concurrent_agents: 2
  max_turns: 10
  claude_flags: "--dangerously-skip-permissions --allowedTools Bash,Read,Write,Edit,Glob,Grep"
```

**Requirements**: `claude` CLI on PATH. Install via `npm install -g @anthropic-ai/claude-code`.

### Codex Runner (default)

Uses JSON-RPC over stdio with the `codex app-server` process. Supports bidirectional messaging: handshake, multi-turn conversations, approval flows, and graceful shutdown.

```yaml
agent:
  runner: codex
  max_concurrent_agents: 3

codex:
  command: "codex app-server"
  stall_timeout_ms: 300000
```

## Configuration

Everything is configured in a single `WORKFLOW.md` file with YAML front matter and a Liquid prompt template body.

### Supported Trackers

- **Gitea** -- Self-hosted or cloud. Requires `owner`, `repo`, and `api_key`
- **Linear** -- SaaS project management. Requires `project_slug` and `api_key`

### Workspace Hooks

Shell scripts executed at workspace lifecycle points. All run with `sh -lc` in the workspace directory.

| Hook | When | On Failure |
|------|------|-----------|
| `after_create` | New workspace created | Workspace removed, dispatch aborted |
| `before_run` | Before each agent attempt | Attempt aborted, retry scheduled |
| `after_run` | After each attempt | Logged and ignored |
| `before_remove` | Before workspace deletion | Logged and ignored |

**Important**: Hooks are plain shell scripts. They are NOT Liquid-rendered. Do not use `{{ }}` template syntax in hook values.

### Prompt Templates

The prompt body (below the YAML front matter) uses [Liquid](https://shopify.github.io/liquid/) syntax with these variables:

| Variable | Description |
|----------|-------------|
| `issue.identifier` | e.g. `"terraphim/repo#42"` or `"MT-42"` |
| `issue.title` | Issue title |
| `issue.description` | Issue body (may be nil) |
| `issue.state` | Current state (e.g. `"Todo"`) |
| `issue.priority` | Priority number (lower = higher) |
| `issue.url` | Web URL to the issue |
| `issue.labels` | Array of label strings |
| `attempt` | Retry attempt number (nil on first try) |

## HTTP Dashboard

Build with `--features api` and pass `--port`:

```bash
cargo build --release --bin symphony --features api
./target/release/symphony ./WORKFLOW.md --port 8080
```

Provides:
- Real-time view of running sessions
- Retry queue with backoff timers
- Cumulative token usage totals
- Per-issue status and turn counts
- Manual refresh trigger via `POST /api/v1/refresh`

## Quick Start

```bash
# Set credentials
export GITEA_TOKEN="your-token"

# Build
cd crates/terraphim_symphony
cargo build --release --bin symphony

# Run
./target/release/symphony ./WORKFLOW.md
```

See the [full Quickstart Guide](../../crates/terraphim_symphony/QUICKSTART.md) for detailed instructions, configuration reference, and troubleshooting.

## Production Deployment

Symphony was used to build the [PageRank Viewer](https://git.terraphim.cloud/terraphim/pagerank-viewer) -- a complete web application implemented autonomously from six Gitea issues. See the [case study](../case-studies/symphony-pagerank-viewer.md) for the full journey.

## Crate Location

Symphony is excluded from the main Cargo workspace. Build from the crate directory:

```bash
cd crates/terraphim_symphony
cargo build --release --bin symphony
cargo test
```
