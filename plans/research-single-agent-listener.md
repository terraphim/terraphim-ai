# Research Document: Spin Single Gitea Listener Agent on Local Laptop

## 1. Problem Restatement and Scope

**Problem**: Run a single terraphim-agent in listener mode on this laptop to poll Gitea issues, claim assigned work, and acknowledge mentions. The agent should run in a tmux session for persistence.

**IN Scope**:
- Configure and run `terraphim-agent listen --identity NAME` connected to `git.terraphim.cloud`
- Agent claims issues when mentioned via `@adf:<agent-name>` in Gitea comments
- Agent posts acknowledgement comments back to Gitea
- Run as a long-lived tmux session
- Use CLI-based tooling (no LLM routing required for the listener itself)
- Agent can delegate to specialists via handoff mechanism

**OUT of Scope**:
- Full autonomous task execution (implementing code, running tests, creating PRs)
- LLM integration for code generation
- Multi-agent orchestration or fleet management
- Symphony/orchestrator binary (separate build)
- Server mode (TUI/fullscreen)

## 2. User and Business Outcomes

- **User-visible**: A running agent that watches Gitea for mentions and claims issues automatically
- **Business value**: Reduces manual issue triage; agent acts as a persistent "always-on" worker
- **Observable outcomes**:
  - Gitea issues get claimed automatically when `@adf:<name>` is used in comments
  - Acknowledgement comments appear on claimed issues
  - tmux session shows listener poll logs

## 3. System Elements and Dependencies

### 3.1 terraphim-agent binary
- **Location**: `~/.cargo/bin/terraphim-agent` (already installed)
- **Role**: CLI entry point with `listen` subcommand
- **Key files**:
  - `crates/terraphim_agent/src/main.rs` - CLI dispatch, listen command at line 1059
  - `crates/terraphim_agent/src/listener.rs` - ListenerConfig, ListenerRuntime, run_listener
- **Dependencies**: tokio runtime, terraphim_tracker, terraphim_orchestrator (ADF parser)

### 3.2 ListenerConfig (JSON)
- **Location**: Passed via `--config PATH` or constructed from `--identity NAME`
- **Structure** (listener.rs:97-111):
  ```json
  {
    "identity": { "agent_name": "NAME", "gitea_login": "LOGIN" },
    "gitea": { "base_url": "https://git.terraphim.cloud", "owner": "terraphim", "repo": "terraphim-ai" },
    "claim_strategy": "prefer_robot",
    "poll_interval_secs": 30,
    "notification_rules": [],
    "delegation": { "allowed_specialists": [], "exclusive_assignment": true, "max_fanout_depth": 1 },
    "repo_scope": "terraphim/terraphim-ai"
  }
  ```
- **Token**: Read from `identity.token_path` file, `gitea.token_path` file, or `GITEA_TOKEN` env var

### 3.3 GiteaTracker (terraphim_tracker)
- **Location**: `crates/terraphim_tracker/src/gitea.rs`
- **Role**: API client for Gitea REST API (issues, comments, assignments)
- **Hardcoded path**: `robot_path: PathBuf::from("/home/alex/go/bin/gitea-robot")` at listener.rs:1252

### 3.4 AdfCommandParser (terraphim_orchestrator)
- **Location**: `crates/terraphim_orchestrator/src/adf_commands.rs`
- **Role**: Parses `@adf:<agent-name>` mentions from comment bodies

### 3.5 Gitea at git.terraphim.cloud
- **Role**: Single source of truth for issues, comments, assignments
- **Auth**: API token required (currently not in env - needs setup)

### 3.6 tmux
- **Role**: Persistent session for long-running listener process
- **Pattern**: Already established in AGENTS.md for background tasks

### 3.7 Existing config
- **Settings**: `~/.config/terraphim/settings.toml` (exists, configured for local dev)
- **Data**: `~/.terraphim/` (exists with config.json, default_thesaurus.json, sqlite)
- **No KG dir**: `~/.config/terraphim/kg/` does not exist (listener does not need KG)

## 4. Constraints and Their Implications

### 4.1 Gitea authentication
- **Constraint**: `GITEA_TOKEN` env var must be set OR `token_path` must point to a file containing the token
- **Why it matters**: Listener will fail to start without a valid token
- **Current state**: Neither `GITEA_URL` nor `GITEA_TOKEN` are set in the current shell (checked)
- **Resolution**: Use `op read "op://TerraphimPlatform/gitea-test-token/credential"` from 1Password (as per CLAUDE.md)

### 4.2 Robot path hardcoded
- **Constraint**: `gitea-robot` binary path is hardcoded to `/home/alex/go/bin/gitea-robot` at listener.rs:1252
- **Implication**: Must exist at that path when `claim_strategy: prefer_robot` is used
- **Status**: Needs verification that the binary exists

### 4.3 Claim strategy
- **Options**: `ApiOnly`, `PreferRobot`, `RobotOnly`
- **Implication**: `PreferRobot` tries gitea-robot first, falls back to API. `ApiOnly` uses REST API directly.
- **Recommendation**: `ApiOnly` is simplest for initial setup (avoids gitea-robot dependency)

### 4.4 Offline-only mode
- **Constraint**: Listen command rejects `--server` flag (main.rs:1061)
- **Implication**: No terraphim_server needed; listener is fully self-contained

### 4.5 No persistence across restarts
- **Constraint**: `seen_events` and `last_seen_at` are in-memory (listener.rs:1270-1271)
- **Implication**: On restart, agent re-processes all comments since `chrono::Utc::now()` (only new ones)
- **Risk**: If agent crashes and restarts, it may miss comments during downtime

### 4.6 System resources
- **Available**: 62 GiB RAM, 16 cores, x86_64 Linux
- **Implication**: More than sufficient for a single listener process

## 5. Risks, Unknowns, and Assumptions

### UNKNOWNS
1. **Gitea token permissions**: Does the token in 1Password have sufficient permissions for issue assignment and comment posting?
2. **Gitea user account**: What login name should the agent use on Gitea? Is there a dedicated agent account?
3. **gitea-robot binary**: Does `/home/alex/go/bin/gitea-robot` exist and work?

### ASSUMPTIONS
1. The Gitea instance at `git.terraphim.cloud` is reachable from this laptop
2. A valid Gitea API token exists in 1Password at the expected path
3. The terraphim-agent binary is current (may need rebuild from source)
4. The agent does not need to execute any work -- only claim and acknowledge

### RISKS
1. **Token expiry**: Gitea token may expire, requiring re-injection. De-risk: check token validity before starting.
2. **Network connectivity**: Laptop may lose connection to Gitea. Mitigation: listener already has retry logic for transient errors (listener.rs:1350-1378).
3. **Binary version mismatch**: Installed binary may not include latest listener fixes (commit 21dc532c, 3efe03c8, 7aa18ec4). De-risk: rebuild from source.
4. **Duplicate processing**: Multiple listener instances with same identity could race on claims. Mitigation: GiteaTracker checks current assignees before claiming.

## 6. Context Complexity vs. Simplicity Opportunities

### Sources of Complexity
1. Token management spans env vars, file paths, and 1Password
2. Hardcoded robot path couples listener to specific machine layout
3. Multiple claim strategies add branching logic

### Simplification Strategies
1. **Minimal config**: Use `ApiOnly` claim strategy to avoid gitea-robot dependency
2. **Env var token**: Set `GITEA_TOKEN` in the tmux session startup script (from `op read`)
3. **Single JSON config file**: Write a static config once, reference with `--config`
4. **Start small, iterate**: Get listener running and claiming before adding delegation

### Proposed Minimal Setup
1. Write listener JSON config
2. Export `GITEA_TOKEN` from 1Password
3. Build latest binary from source (to include recent fixes)
4. Start in tmux with: `terraphim-agent listen --config listener.json`
5. Test by mentioning the agent in a Gitea issue comment

## 7. Questions for Human Reviewer

1. **Agent identity name**: What should the agent_name be (e.g., "alex-agent", "terraphim-worker")?
2. **Gitea login**: What is your Gitea login name at git.terraphim.cloud? Should the agent use your account or a dedicated bot account?
3. **Repo scope**: Should the agent watch only `terraphim/terraphim-ai` or multiple repos?
4. **Rebuild binary**: Should I rebuild terraphim-agent from source to include the latest listener fixes, or use the installed binary?
5. **Delegation**: Should the agent be allowed to delegate to any specialists, or should delegation be disabled initially?
