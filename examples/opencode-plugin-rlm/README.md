# Terraphim RLM OpenCode Plugin

OpenCode plugin for terraphim_rlm - Recursive Language Model orchestration with secure code execution.

## Features

- **Isolated Code Execution**: Run Python code in Firecracker VMs, Docker containers, or locally
- **Recursive LLM Loops**: Query → Execute → Feedback → Repeat with LLM review
- **Session Management**: Create sessions with snapshots and rollback
- **Knowledge Graph Validation**: Validate commands against KG before execution
- **Budget Tracking**: Token, time, and recursion depth limits

## Requirements

- OpenCode CLI installed
- terraphim_mcp_server with RLM tools, OR terraphim_rlm crate built from source
- For the Claude Code hook (`terraphim-rlm-hook.sh`): `bash` (>= 4) and `jq`
  (the hook builds JSON-RPC requests via `jq -n --arg` for safe escaping)

The hook is portable: it does NOT depend on GNU `timeout`/`gtimeout`, so it
works on macOS without `brew install coreutils`. A pure-POSIX subshell
wrapper enforces request timeouts.

## Installation

### OpenCode Plugin

```bash
# Option 1: Copy manually
cp terraphim-rlm.js ~/.config/opencode/plugin/

# Option 2: Use install script
./install.sh --opencode
```

### Claude Code Hook

```bash
# Install hook
./install.sh --claude

# Or manually
cp terraphim-rlm-hook.sh ~/.claude/hooks/
chmod +x ~/.claude/hooks/terraphim-rlm-hook.sh
```

## Usage

### RLM Commands

| Command | Description |
|---------|-------------|
| `rlm_query "prompt"` | Execute full recursive query loop |
| `rlm_code "python code"` | Execute Python in isolated VM |
| `rlm_bash "command"` | Execute bash command in VM |
| `rlm_status` | Get session status |

### Examples

```bash
# Query with recursive LLM feedback
> rlm_query "Calculate the first 10 fibonacci numbers"

# Execute Python
> rlm_code "print('Hello from RLM!'); import math; print(math.pi)"

# Run bash
> rlm_bash "ls -la /workspace"
```

## Architecture

```
TerraphimRlm (public API)
    ├── SessionManager (VM affinity, context, snapshots, extensions)
    ├── QueryLoop (command parsing, execution, result handling)
    ├── BudgetTracker (token counting, time tracking, depth limits)
    └── KnowledgeGraphValidator (term matching, retry, strictness)

ExecutionEnvironment trait (pluggable)
    ├── FirecrackerExecutor (full VM isolation, requires KVM)
    ├── DockerExecutor (container isolation with gVisor)
    ├── E2bExecutor (cloud-hosted Firecracker)
    └── LocalExecutor (local process execution, no isolation)
```

## Configuration

Set environment variables to customize:

| Variable | Default | Description |
|----------|---------|-------------|
| `TERRAPHIM_AGENT` | `~/.cargo/bin/terraphim-agent` | Path to terraphim-agent |
| `TERRAPHIM_MCP` | `terraphim_mcp_server` | MCP server command |
| `TERRAPHIM_DEBUG` | `0` | Enable debug logging |

## Backends

By default, RLM tries backends in this order:
1. **Firecracker** - Full VM isolation (requires KVM on Linux)
2. **E2B** - Cloud-hosted Firecracker (requires API key)
3. **Docker** - Container isolation (requires Docker daemon)
4. **Local** - Direct execution on host (no isolation)

## Known Issues

The OpenCode plugin (`terraphim-rlm.js`) currently spawns a fresh
`terraphim_mcp_server` process per tool invocation rather than reusing a
long-lived stdio connection. This is correct but inefficient. A rewrite to
share the `ensureMcpServer` process across calls is tracked as a follow-up.

## Tests

A bash smoke-test suite exercises the hook's input parsing, JSON
construction, and portable timeout wrapper:

```bash
bash tests/test_hook.sh
```

Tests do not require a running MCP server; they stub `$TERRAPHIM_MCP`
with a script that captures its stdin so we can inspect the request body.

## License

MIT
