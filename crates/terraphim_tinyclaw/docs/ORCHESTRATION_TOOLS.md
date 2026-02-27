# TinyClaw Orchestration Tools

This document describes the TinyClaw orchestration extensions added for OpenClaw parity sequencing:

- `agent_spawn` (external agent execution via `terraphim_spawner`)
- `cron` (scheduled reminder dispatch into TinyClaw sessions/channels)

## Configuration

Add these sections to `tinyclaw.toml`.

```toml
[spawner]
enabled = true
max_concurrent = 3
default_timeout_secs = 300
shutdown_grace_secs = 5

[[spawner.agents]]
name = "codex"
command = "codex"
working_directory = "/tmp/tinyclaw"
capabilities = ["code_generation", "code_review"]

[[spawner.agents]]
name = "opencode"
command = "opencode"

[[spawner.agents]]
name = "claude-code"
command = "claude"

[cron]
enabled = true
tick_seconds = 1
persist_path = "cron/jobs.json"
max_jobs = 256
```

## `agent_spawn` Behavior

- Uses `terraphim_spawner::AgentSpawner` for process lifecycle and output capture.
- Enforces `spawner.max_concurrent` with a semaphore; excess requests are blocked.
- Supports `agent_type`, `task`, optional `working_directory`, optional `wait_seconds`, and `detach`.
- Uses configured `spawner.agents` mappings; built-ins are also available: `codex`, `opencode`, `claude-code`, `echo`.
- `wait_seconds` is clamped to `1..=1800`.

## `cron` Behavior

- Actions: `status`, `list`, `add`, `remove`.
- Persists job state to `cron.persist_path` as JSON.
- Scheduler tick interval is `cron.tick_seconds`.
- `add` supports:
  - `schedule.kind = "at"` with future RFC3339 `schedule.at`
  - `schedule.kind = "every"` with `schedule.every_seconds` in `1..=604800`
- Channel isolation: cross-channel scheduling is blocked. `requester_session_key` and `session_key` must share the same channel prefix.
- On due runs, TinyClaw appends a system message to the target session and dispatches outbound text to the target channel/chat.

## Testing

- Unit tests cover `agent_spawn` and `cron` parsing/validation.
- Integration test `tests/cron_tools_integration.rs` verifies scheduled dispatch behavior.
