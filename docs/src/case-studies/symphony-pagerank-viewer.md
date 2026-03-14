# Case Study: Symphony Builds a Web Application

How Symphony orchestrated Claude Code agents to implement a complete PageRank Viewer from six Gitea issues -- autonomously.

## Summary

We used `terraphim_symphony` to build the [PageRank Viewer](https://git.terraphim.cloud/terraphim/pagerank-viewer), a vanilla JavaScript web application that visualises Gitea issue dependencies with PageRank scores. Symphony polled a Gitea repository for open issues, dispatched Claude Code agents to implement each one in an isolated workspace, and committed the results back to the repository. Six issues produced approximately 3,000 lines of production JavaScript across nine files, delivered in three batches of parallel agent sessions.

## The Problem

We needed a browser-based graph viewer consuming the Gitea Robot API -- endpoints that return PageRank-ranked dependency graphs, triage recommendations, and unblocked-issue lists for any repository. The viewer needed force-directed graph visualisation, multiple layout modes, graph metrics computation, in-browser SQLite storage for historical snapshots, and a dark-themed UI.

Rather than coding it manually, we decomposed the work into six Gitea issues and let Symphony orchestrate the entire implementation.

## Setup

### Infrastructure

- **Orchestrator host**: Ubuntu server ("bigbox") running Symphony and Claude Code CLI
- **Issue tracker**: Self-hosted Gitea at `git.terraphim.cloud`
- **Target repository**: `terraphim/pagerank-viewer` (empty repo, created with `tea repos create`)
- **Agent**: Claude Code CLI (`claude -p` with `--output-format stream-json --verbose`)

### WORKFLOW.md

A single configuration file controlled the entire operation:

```yaml
---
tracker:
  kind: gitea
  endpoint: https://git.terraphim.cloud
  api_key: $GITEA_TOKEN
  owner: terraphim
  repo: pagerank-viewer

agent:
  runner: claude-code
  max_concurrent_agents: 2
  max_turns: 10
  claude_flags: "--dangerously-skip-permissions --allowedTools Bash,Read,Write,Edit,Glob,Grep"

workspace:
  root: ~/symphony_workspaces

hooks:
  after_create: "git clone https://terraphim:${GITEA_TOKEN}@git.terraphim.cloud/terraphim/pagerank-viewer.git ."
  before_run: "git fetch origin && git checkout main && git pull"
  after_run: "git add -A && git commit -m 'symphony: auto-commit' && git push || true"
  timeout_ms: 120000

codex:
  turn_timeout_ms: 3600000
  stall_timeout_ms: 600000
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

### Issue Decomposition

Six issues were created in the `terraphim/pagerank-viewer` repository:

| # | Title | Files Produced |
|---|-------|----------------|
| 1 | Create Gitea Robot API client (api.js) | `api.js` (237 lines) |
| 2 | Create graph metrics computation library (metrics.js) | `metrics.js` (386 lines) |
| 3 | Create Force-Graph WebGL visualisation (graph.js) | `graph.js` (473 lines) |
| 4 | Create SQL.js in-browser storage (store.js) | `store.js` (472 lines) |
| 5 | Create viewer HTML shell and CSS (index.html, styles.css) | `index.html` (133 lines), `styles.css` (538 lines), `app.js` (392 lines) |
| 6 | Integrate terraphim_automata WASM into viewer | Integration code |

## Execution

### Batch 1: Issues #1, #2, #3

Symphony dispatched two agents in parallel (the concurrency limit). Issues #1 and #2 ran first, each completing in 11 turns. Issue #3 was dispatched as soon as a slot freed up.

**Generated code quality**: The agents produced well-structured, production-quality JavaScript with JSDoc comments, error handling, and proper API patterns:

- **api.js**: `GiteaRobotClient` class with methods for graph, triage, and ready endpoints, token management via `localStorage`, optional CORS proxy support, and auto-refresh
- **metrics.js**: Brandes betweenness centrality, HITS algorithm, eigenvector centrality, critical path analysis, and cascade impact simulation
- **graph.js**: Force-Graph WebGL wrapper with force-directed, hierarchical DAG, and radial layouts, heatmap overlays, and what-if simulation (click a node to simulate closing it)

### Batch 2: Issues #4, #5

After closing the first three issues, Symphony cleaned up their workspaces and dispatched the next batch:

- **store.js**: SQL.js WASM SQLite wrapper for snapshots, dependency tracking, and historical comparison with schema migrations
- **index.html + styles.css + app.js**: SPA shell with FontAwesome icons, dark theme matching the Symphony dashboard palette (`#1a1a2e`, `#e94560`, `#16213e`), responsive grid, and layout switcher

### Batch 3: Issue #6

The final issue integrated terraphim_automata WASM for autocomplete search across issues.

## Timeline

The entire process -- from creating the Gitea repository and issues to having all six issues implemented and committed -- completed in a single session. Each agent session took approximately 11 turns (the configured maximum was 10 but the final result event counts as an additional turn). Symphony managed all workspace creation, hook execution, agent spawning, and cleanup automatically.

## Issues Encountered and Fixes

Three bugs were discovered and fixed during the deployment:

### 1. Git Authentication in Hooks

**Problem**: The `after_create` hook used a plain HTTPS clone URL (`git clone https://git.terraphim.cloud/...`), which prompted for credentials in the non-interactive shell context and timed out after 60 seconds.

**Fix**: Changed to a token-embedded URL: `git clone https://terraphim:${GITEA_TOKEN}@git.terraphim.cloud/...`

SSH was also tested but failed due to missing SSH keys on the server. Token-embedded HTTPS is the most reliable approach for automated environments.

### 2. Missing --verbose Flag

**Problem**: All Claude Code sessions exited immediately with code 1. Manual testing revealed the error: "When using --print, --output-format=stream-json requires --verbose".

**Fix**: Added `--verbose` to the command arguments in `claude_code.rs`. This is a Claude Code CLI requirement -- `--output-format stream-json` only works with `--verbose` in `-p` (print) mode.

### 3. Liquid Templates in Shell Hooks

**Problem**: The `after_run` hook contained `{{ issue.identifier }}` in a commit message, which appeared literally in the shell command because hooks are plain shell scripts run via `sh -lc`, not Liquid-rendered templates.

**Fix**: Changed the commit message to a static string: `'symphony: auto-commit'`. Only the prompt body (below the YAML front matter) is rendered with Liquid.

## Results

| Metric | Value |
|--------|-------|
| Issues implemented | 6 |
| Files generated | 9 |
| Total lines of code | ~3,000 |
| Agent turns per issue | ~11 |
| Concurrent agents | 2 |
| Batches | 3 |
| Bugs fixed during deployment | 3 |

## Key Takeaways

1. **Single-file configuration works**: WORKFLOW.md captures tracker, agent, hooks, and prompt in one place. Hot reload means you can adjust settings without restarting.

2. **Issue decomposition matters**: Each issue should produce files that do not overlap with other issues. When multiple agents push to the same branch, merge conflicts arise. Design issues to touch disjoint files.

3. **Hooks are plain shell**: Do not use Liquid syntax in hook values. Only the prompt body is template-rendered.

4. **Token-embedded URLs for automation**: Non-interactive environments cannot prompt for credentials. Embed tokens in clone URLs or use deploy keys.

5. **Close issues after verification**: Symphony re-dispatches any open issue matching `active_states` on every poll tick. Close issues promptly after verifying the generated code.

6. **Claude Code requires --verbose**: When using `--output-format stream-json` with `-p` mode, the `--verbose` flag is mandatory. Without it, the CLI exits with code 1.

7. **Stall timeout needs headroom**: Claude Code agent sessions can run for several minutes per turn. Set `stall_timeout_ms` generously (600000ms / 10 minutes worked well).

## What This Demonstrates

Symphony implements the **Orchestrator-Workers** pattern from the [AI Agent Workflows](../ai_agents_workflows.md) taxonomy. The orchestrator (Symphony daemon) manages task assignment, workspace isolation, and lifecycle management, while the workers (Claude Code agents) focus purely on code generation. This separation of concerns means:

- The orchestrator handles all operational complexity (polling, retries, concurrency, cleanup)
- The agents receive a clean prompt and a fresh workspace -- they do not need to know about the broader system
- Adding more issues or changing the agent configuration requires only editing the WORKFLOW.md file

This pattern scales naturally: increase `max_concurrent_agents` to run more agents in parallel, point at a different repository by changing `tracker.owner` and `tracker.repo`, or switch from Claude Code to Codex by changing `agent.runner`.
