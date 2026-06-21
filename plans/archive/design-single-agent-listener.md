# Design & Implementation Plan: Single Gitea Listener Agent on Local Laptop

## 1. Summary of Target Behaviour

A single `terraphim-agent` process runs in a tmux session, polling `git.terraphim.cloud` for new comments mentioning `@adf:terraphim-worker`. When mentioned, the agent:

1. Fetches the referenced issue
2. Claims the issue (assigns itself via Gitea API)
3. Posts an acknowledgement comment on the issue
4. Tracks processed events in memory to avoid duplicates

The agent runs continuously with a configurable poll interval and retries transient failures automatically.

**No code changes to the Rust codebase are required.** This is purely an operational setup task: build, configure, and launch the existing listener infrastructure.

## 2. Key Invariants and Acceptance Criteria

### Invariants
| Invariant | Description |
|-----------|-------------|
| I1: Single instance | Exactly one listener process running at any time |
| I2: At-least-once processing | Every mention is processed (duplicates avoided via seen_events set) |
| I3: No duplicate claims | GiteaTracker checks current assignees before claiming |
| I4: Token never on disk | GITEA_TOKEN injected via `op run` at session start |
| I5: Offline-only | No terraphim_server dependency |

### Acceptance Criteria
| ID | Criterion | Verification |
|----|-----------|-------------|
| AC1 | Agent starts and connects to Gitea successfully | tmux shows "listener config has no Gitea connection" absent; poll logs appear |
| AC2 | Posting `@adf:terraphim-worker` on a Gitea issue triggers a claim | Issue assignee changes to the configured login; ack comment appears |
| AC3 | Agent ignores its own comments | No self-referencing loops |
| AC4 | Agent survives transient Gitea errors (500, 429) | Continues polling after error; log shows retry |
| AC5 | Agent restarts cleanly in tmux | Can kill and restart without side effects |
| AC6 | Token is never written to disk in plaintext | Config file has no token; only env var |

## 3. High-Level Design and Boundaries

### Architecture

```
[tmux: terraphim-worker]
  |
  +-- op run --no-masking -- terraphim-agent listen --config listener.json
        |
        +-- ListenerRuntime (in-memory)
              |
              +-- GiteaTracker (REST API client)
              |     |-- fetch_repo_comments_page()
              |     |-- claim_issue()
              |     |-- post_comment()
              |
              +-- AdfCommandParser (@adf: parsing)
              |
              +-- control_plane::normalize_polled_command()
```

### Boundaries
- **No changes to Rust source code** -- pure operational setup
- **No terraphim_server** -- listener is offline-only
- **No LLM provider** -- agent claims and acknowledges only, does not implement work
- **No gitea-robot** -- using `ApiOnly` claim strategy (binary not present at hardcoded path)

## 4. File/Module-Level Change Plan

| File/Module | Action | Before | After |
|-------------|--------|--------|-------|
| `~/.config/terraphim/listener-worker.json` | Create | N/A | Listener config for terraphim-worker identity |
| `~/.config/terraphim/scripts/start-listener.sh` | Create | N/A | tmux launch script with op token injection |
| `~/.cargo/bin/terraphim-agent` | Rebuild | Current installed binary | Latest from source (includes listener fixes) |

### listener-worker.json contents:
```json
{
  "identity": {
    "agent_name": "terraphim-worker",
    "gitea_login": "<GITEA_LOGIN>"
  },
  "gitea": {
    "base_url": "https://git.terraphim.cloud",
    "owner": "terraphim",
    "repo": "terraphim-ai"
  },
  "claim_strategy": "api_only",
  "poll_interval_secs": 30,
  "notification_rules": [],
  "delegation": {
    "allowed_specialists": [],
    "exclusive_assignment": true,
    "max_fanout_depth": 1
  },
  "repo_scope": "terraphim/terraphim-ai"
}
```

### start-listener.sh outline:
```bash
#!/usr/bin/env bash
set -euo pipefail
source ~/.profile
export GITEA_URL="https://git.terraphim.cloud"
export GITEA_TOKEN=$(op read "op://TerraphimPlatform/gitea-test-token/credential")
tmux new-session -d -s terraphim-worker \
  "~/.cargo/bin/terraphim-agent listen --config ~/.config/terraphim/listener-worker.json"
tmux attach -t terraphim-worker
```

## 5. Step-by-Step Implementation Sequence

### Step 1: Build release binary from source
- **Purpose**: Include latest listener fixes (retry logic, paging, claim verification)
- **Command**: `cargo build -p terraphim_agent --release --features repl-full`
- **Deployable**: Yes (binary upgrade, listener not yet running)
- **Verify**: `target/release/terraphim-agent --version`

### Step 2: Copy binary to cargo bin
- **Purpose**: Make latest binary available at expected path
- **Command**: `cp target/release/terraphim-agent ~/.cargo/bin/terraphim-agent`
- **Deployable**: Yes

### Step 3: Create listener config file
- **Purpose**: Static JSON config for the agent identity
- **Action**: Write `~/.config/terraphim/listener-worker.json`
- **Requires**: Gitea login name from user
- **Verify**: `terraphim-agent listen --identity terraphim-worker` (dry run with no Gitea)

### Step 4: Create launch script
- **Purpose**: Reproducible tmux session startup with 1Password token injection
- **Action**: Write `~/.config/terraphim/scripts/start-listener.sh`, chmod +x
- **Verify**: Script passes shellcheck

### Step 5: Test dry run (no Gitea connection)
- **Purpose**: Verify config loads and validates correctly
- **Command**: `GITEA_TOKEN=test terraphim-agent listen --config ~/.config/terraphim/listener-worker.json`
- **Expected**: Prints identity info, "listener config has no Gitea connection" only if gitea block missing; otherwise starts polling
- **Verify**: Ctrl+C to stop

### Step 6: Launch in tmux with real token
- **Purpose**: Start the agent in production mode
- **Action**: Run start-listener.sh
- **Verify**: `tmux attach -t terraphim-worker` shows poll logs

### Step 7: End-to-end validation
- **Purpose**: Confirm agent claims issues when mentioned
- **Action**: Post a test comment on a Gitea issue with `@adf:terraphim-worker`
- **Verify**: Agent claims issue, posts ack comment, tmux logs show the event

### Step 8: Document and hand off
- **Purpose**: Record setup for future sessions
- **Action**: Update HANDOVER.md or create a brief note in `.docs/`
- **Verify**: Next session can restart agent with single command

## 6. Testing & Verification Strategy

| Acceptance Criterion | Test Type | Verification Method |
|---------------------|-----------|---------------------|
| AC1: Agent connects | Integration | Check tmux logs for successful poll cycle |
| AC2: Mention triggers claim | E2E | Post test comment, check issue assignee and ack comment |
| AC3: Ignores own comments | Unit (existing) | Already tested in listener.rs:442 (listener_runtime_ignores_self_authored_comments) |
| AC4: Survives transient errors | Unit (existing) | Already tested in listener.rs:970 (retry transient failures) |
| AC5: Restarts cleanly | Manual | Kill tmux session, re-run start script, verify clean start |
| AC6: Token not on disk | Manual | Grep listener-worker.json for token-like strings; check env only |

### Existing test coverage (no new tests needed):
- `listener.rs` contains 12 comprehensive unit/integration tests covering:
  - Config validation
  - Claim and ack flow
  - Self-authored comment filtering
  - Pagination
  - Transient error retry
  - Handoff to specialists
  - Agent name aliasing

## 7. Risk & Complexity Review

| Risk | Likelihood | Mitigation | Residual |
|------|-----------|------------|----------|
| Token lacks issue-assign permission | Medium | Test claim on a real issue before relying on agent | May need to adjust token scope in Gitea |
| gitea-robot path hardcoded but missing | N/A | Using `ApiOnly` strategy (no robot dependency) | None |
| Agent crashes and loses seen_events | Low | Acceptable for initial setup; events reprocessed on restart | May see duplicate ack comments on restart |
| Poll interval too aggressive for Gitea | Low | 30s default is conservative; Gitea rate limits are generous | Could increase to 60s if needed |
| Binary rebuild takes time | Low | Release build ~2-5 min on 16-core machine | None |

### Complexity assessment: **LOW**
- No code changes required
- No new dependencies
- Purely operational/configuration task
- Existing test coverage is comprehensive

## 8. Open Questions / Decisions for Human Review

1. **Gitea login name**: What is your login name on `git.terraphim.cloud`? This goes in the `gitea_login` field so the agent can post comments and claim issues as you.
2. **Poll interval**: Is 30 seconds acceptable, or would you prefer a different interval?
3. **Auto-start on boot**: Should the tmux session be launched automatically (e.g., via a systemd user service or crontab @reboot), or is manual start sufficient?
