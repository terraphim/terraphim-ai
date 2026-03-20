# Migration Guide: Dual Mode Orchestrator

This guide covers migrating from the legacy time-only orchestrator to the new dual-mode orchestrator that supports both time-based and issue-driven task scheduling.

## Table of Contents

- [Overview](#overview)
- [Breaking Changes](#breaking-changes)
- [Migration Steps](#migration-steps)
- [Configuration Changes](#configuration-changes)
- [Backward Compatibility](#backward-compatibility)
- [Troubleshooting](#troubleshooting)

## Overview

The terraphim orchestrator now supports three modes of operation:

1. **Time-Only Mode** (Legacy): The original mode using cron-based scheduling
2. **Issue-Only Mode**: New mode that schedules tasks based on issue tracker events
3. **Dual Mode**: Combines both time-based and issue-driven scheduling

### Key Features

- **Unified Dispatch Queue**: Both time and issue tasks share a priority queue with fairness
- **Mode Coordinator**: Manages both TimeMode and IssueMode simultaneously
- **Stall Detection**: Automatically detects and warns when the task queue grows too large
- **Graceful Shutdown**: Coordinated shutdown that drains queues and waits for active tasks

## Breaking Changes

There are **no breaking changes** for existing configurations. The orchestrator is fully backward compatible:

- Old `orchestrator.toml` files without the `[workflow]` section continue to work
- Time-only mode is the default behavior when no workflow configuration is present
- All existing APIs and methods remain functional

## Migration Steps

### Step 1: Update Configuration (Optional)

To enable dual mode, add the following sections to your `orchestrator.toml`:

```toml
[workflow]
mode = "dual"  # Options: "time_only", "issue_only", "dual"
poll_interval_secs = 60
max_concurrent_tasks = 5

[tracker]
tracker_type = "gitea"
url = "https://git.example.com"
token_env_var = "GITEA_TOKEN"
owner = "myorg"
repo = "myrepo"

[concurrency]
max_parallel_agents = 3
queue_depth = 100
starvation_timeout_secs = 300
```

### Step 2: Set Environment Variables

Ensure the tracker token environment variable is set:

```bash
export GITEA_TOKEN="your-api-token"
```

### Step 3: Restart the Orchestrator

```bash
cargo run --bin adf -- --config /path/to/orchestrator.toml
```

## Configuration Changes

### New Configuration Sections

#### `[workflow]` Section

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `mode` | string | `"time_only"` | Execution mode: `time_only`, `issue_only`, or `dual` |
| `poll_interval_secs` | integer | 60 | How often to poll for new issues |
| `max_concurrent_tasks` | integer | 5 | Maximum parallel tasks across all modes |

#### `[tracker]` Section

Required for `issue_only` and `dual` modes:

| Field | Type | Description |
|-------|------|-------------|
| `tracker_type` | string | Tracker type: `gitea` or `linear` |
| `url` | string | Tracker API URL |
| `token_env_var` | string | Environment variable containing auth token |
| `owner` | string | Repository owner/organization |
| `repo` | string | Repository name |

#### `[concurrency]` Section

Optional performance tuning:

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `max_parallel_agents` | integer | 3 | Maximum parallel agent executions |
| `queue_depth` | integer | 100 | Maximum queue depth before stall warnings |
| `starvation_timeout_secs` | integer | 300 | Timeout before considering a task starved |

### Example Configurations

#### Time-Only Mode (Legacy)

```toml
# No [workflow] section needed - this is the default
# Existing configuration continues to work
```

#### Dual Mode

```toml
[workflow]
mode = "dual"
poll_interval_secs = 60
max_concurrent_tasks = 5

[tracker]
tracker_type = "gitea"
url = "https://git.terraphim.cloud"
token_env_var = "GITEA_TOKEN"
owner = "terraphim"
repo = "terraphim-ai"

[concurrency]
max_parallel_agents = 3
queue_depth = 50
starvation_timeout_secs = 300
```

#### Issue-Only Mode

```toml
[workflow]
mode = "issue_only"
poll_interval_secs = 30
max_concurrent_tasks = 3

[tracker]
tracker_type = "gitea"
url = "https://git.example.com"
token_env_var = "GITEA_TOKEN"
owner = "myorg"
repo = "myrepo"
```

## Backward Compatibility

### No Changes Required

Existing orchestrator.toml files without `[workflow]` will continue to work exactly as before:

- Safety agents spawn immediately
- Core agents follow their cron schedules
- Growth agents run on-demand
- All existing monitoring and drift detection works unchanged

### Code Compatibility

The public API remains unchanged:

```rust
// Existing code continues to work
let config = OrchestratorConfig::from_file("orchestrator.toml")?;
let mut orch = AgentOrchestrator::new(config)?;
orch.run().await?;
```

### Compatibility Helpers

For migration assistance, use the compatibility layer:

```rust
use terraphim_orchestrator::compat::{SymphonyAdapter, SymphonyOrchestratorExt};

// Check if running in legacy mode
if orch.is_legacy_mode() {
    println!("Running in legacy mode");
}

// Get mode description
println!("Mode: {}", orch.mode_description());

// Convert config to legacy mode
let legacy_config = SymphonyAdapter::to_legacy_config(config);
```

## Troubleshooting

### Issue: "Token cannot be empty" Error

**Cause**: The tracker token environment variable is not set.

**Solution**:
```bash
export GITEA_TOKEN="your-token-here"
```

### Issue: Tasks Not Being Dispatched from Queue

**Cause**: Concurrency limit reached or no available agents.

**Solution**: Check the logs for:
- "concurrency limit reached, skipping dispatch"
- Verify agents are defined in the config
- Increase `max_concurrent_tasks` if needed

### Issue: "STALL DETECTED" Warnings

**Cause**: The dispatch queue is growing faster than tasks are being processed.

**Solution**:
- Increase `max_parallel_agents` in `[concurrency]`
- Review task execution time
- Check if agents are completing successfully
- Consider increasing `queue_depth` if needed

### Issue: Issue Mode Not Starting

**Cause**: Missing tracker configuration or invalid tracker type.

**Solution**: Verify:
- `[tracker]` section exists in config
- `tracker_type` is either "gitea" or "linear"
- All required tracker fields are set
- Token environment variable is set and valid

### Issue: Fairness Concerns

**Cause**: One task type is dominating the queue.

**Solution**: The dispatch queue automatically applies fairness rules:
- Alternates between time and issue tasks at equal priority
- Higher priority issue tasks are processed first
- Monitor with `check_stall()` to detect buildup

## Additional Resources

- See `tests/e2e_tests.rs` for usage examples
- See `src/compat.rs` for migration helper functions
- See `CLAUDE.md` for architecture details

## Support

For issues or questions about migration:

1. Check the test suite for working examples
2. Review the compatibility layer in `src/compat.rs`
3. Consult the architecture documentation in `CLAUDE.md`
