# terraphim_symphony

Symphony is an autonomous orchestrator that watches your issue tracker, dispatches coding agents to implement each issue in an isolated workspace, and manages the full lifecycle of retries, stall detection, and cleanup.

You configure everything -- tracker, agent, hooks, prompt -- in a single `WORKFLOW.md` file.

## Architecture

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

Symphony polls your issue tracker, sorts eligible issues by priority and age, creates isolated workspace directories, renders a Liquid prompt template per issue, and spawns a coding agent session. It monitors progress, detects stalls, retries with exponential backoff, and cleans up workspaces when issues reach terminal states.

## Runners

Symphony supports two agent runners:

### Codex (default)

The Codex runner uses JSON-RPC over stdio with the `codex app-server` process. It supports bidirectional messaging: handshake, multi-turn conversations, approval flows, and graceful shutdown.

### Claude Code

The Claude Code runner invokes `claude -p "<prompt>" --output-format stream-json --verbose --max-turns N` as a single-shot process per issue. It parses the NDJSON event stream to extract turn counts, token usage, and errors. No handshake, no approval flow -- fire-and-forget with retry on failure.

**Requirements**: `claude` CLI on PATH. Install via `npm install -g @anthropic-ai/claude-code`.

**Hooks parity**: By default, `claude -p` sessions do not inherit project-level hooks from the calling environment. To ensure agent sessions have the same PreToolUse and PostToolUse hooks as interactive `claude` sessions, set `agent.settings` to a JSON file containing your hooks configuration:

```yaml
agent:
  runner: claude-code
  settings: ~/.claude/symphony-settings.json
```

See `examples/symphony-settings.json` for a template.

## Quick Start

See [QUICKSTART.md](./QUICKSTART.md) for the full guide. The short version:

```bash
# Set credentials
export GITEA_TOKEN=$(op read "op://TerraphimPlatform/gitea-test-token/credential")

# Build
cd crates/terraphim_symphony
cargo build --release --bin symphony

# Run
./target/release/symphony ./WORKFLOW.md

# Run with HTTP dashboard
cargo build --release --bin symphony --features api
./target/release/symphony ./WORKFLOW.md --port 8080
```

## WORKFLOW.md Format

A single file with YAML front matter (configuration) and a Liquid prompt template body:

```yaml
---
tracker:
  kind: gitea
  owner: terraphim
  repo: my-project
  api_key: $GITEA_TOKEN

agent:
  runner: claude-code
  max_concurrent_agents: 2
  max_turns: 10
  claude_flags: "--dangerously-skip-permissions --allowedTools Bash,Read,Write,Edit,Glob,Grep"

workspace:
  root: ~/symphony_workspaces

hooks:
  after_create: "git clone https://user:${TOKEN}@git.example.com/org/repo.git ."
  before_run: "git fetch origin && git checkout main && git pull"
  after_run: "git add -A && git commit -m 'symphony: auto-commit' && git push || true"
  timeout_ms: 120000
---
You are working on issue {{ issue.identifier }}: {{ issue.title }}.

{{ issue.description }}

## Instructions
1. Read the issue carefully.
2. Implement the required changes.
3. Write tests to verify your changes.
4. Commit with a message referencing {{ issue.identifier }}.
```

## Key Features

- **Two runners**: Codex (JSON-RPC, multi-turn) and Claude Code (single-shot CLI)
- **Two trackers**: Gitea and Linear, with configurable active/terminal states
- **Isolated workspaces**: One directory per issue, with lifecycle hooks
- **Exponential backoff**: 10s, 20s, 40s... capped at 5 minutes on failure
- **Stall detection**: Kills sessions with no activity for a configurable duration
- **Hot reload**: WORKFLOW.md is watched for changes; new config takes effect on the next tick
- **HTTP dashboard**: Real-time view of running sessions, retry queue, and token usage
- **Liquid templates**: Prompt body uses Liquid syntax with issue metadata variables
- **Priority sorting**: Issues dispatched by priority (lowest first), then age (oldest first)
- **Per-state concurrency**: Optional limits on how many agents run per issue state
- **Todo blocker rule**: Issues in "Todo" state with blockers are held until all blockers reach terminal states

## Module Structure

```
src/
  config/          -- WORKFLOW.md parsing, ServiceConfig, validation
  runner/
    session.rs     -- CodexSession (JSON-RPC app-server)
    claude_code.rs -- ClaudeCodeSession (claude -p CLI)
    protocol.rs    -- AgentEvent, TokenCounts, TokenTotals
  orchestrator/    -- Poll/dispatch/retry/reconcile loop
  workspace/       -- Directory management and lifecycle hooks
  api/             -- HTTP dashboard (feature-gated)
  error.rs         -- Error types
  lib.rs           -- Public API
```

## Configuration Reference

See [QUICKSTART.md](./QUICKSTART.md) for the full reference tables covering tracker, polling, workspace, hooks, agent, codex, and server settings.

## Production Deployment: PageRank Viewer Case Study

Symphony was used to build the [PageRank Viewer](https://git.terraphim.cloud/terraphim/pagerank-viewer) -- a web application that visualises Gitea issue dependencies with PageRank scores. Six Gitea issues were created with dependency relationships, and Symphony dispatched Claude Code agents to implement each one autonomously. The result was approximately 3,000 lines of production JavaScript across 9 files, generated in three batches of parallel agent sessions.

See the [case study](../../docs/src/case-studies/symphony-pagerank-viewer.md) for the full end-to-end journey.

## Building

Symphony is excluded from the main workspace (it has its own dependency tree). Build from the crate directory:

```bash
cd crates/terraphim_symphony

# Debug
cargo build --bin symphony

# Release
cargo build --release --bin symphony

# With HTTP dashboard
cargo build --release --bin symphony --features api

# Run tests
cargo test
```

## Lessons Learned

Key findings from production deployment:

1. **Token-embedded clone URLs**: Use `https://user:${TOKEN}@host/...` for non-interactive git auth in hooks. Plain HTTPS prompts for credentials and times out.
2. **`--verbose` is mandatory**: Claude Code CLI requires `--verbose` when using `--output-format stream-json` with `-p` mode.
3. **Hooks are plain shell**: Hook scripts run via `sh -lc` and are NOT Liquid-rendered. Do not use `{{ }}` template syntax in hook values.
4. **Workspace isolation**: Each issue gets its own git clone. When multiple agents push to the same branch, merge conflicts can occur. Design issues to touch different files.
5. **Close issues promptly**: Open issues matching `active_states` are re-dispatched on every poll tick. Close them after verifying the generated code.
