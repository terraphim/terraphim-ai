# Summary: terraphim_rlm/src/lib.rs

## Purpose

Recursive Language Model (RLM) orchestration crate providing isolated code execution in Firecracker VMs with sub-500ms allocation, recursive LLM invocation from within VMs via HTTP bridge, and knowledge graph validation of commands.

## Architecture

```
TerraphimRlm (public API)
    ├── SessionManager (VM affinity, context, snapshots, extensions)
    ├── QueryLoop (command parsing, execution, result handling)
    ├── BudgetTracker (token counting, time tracking, depth limits)
    └── KnowledgeGraphValidator (term matching, retry, strictness)

ExecutionEnvironment trait
    ├── FirecrackerExecutor (primary, full isolation)
    ├── DockerExecutor (fallback, gVisor/runc)
    └── E2bExecutor (cloud option)
```

## Key Modules

- **config**: `RlmConfig`, `BackendType`, `KgStrictness`, `SessionModel`
- **executor**: `ExecutionEnvironment` trait, `LocalExecutor`, `FirecrackerExecutor`, `DockerExecutor`, `SshExecutor`
- **session**: `SessionManager`, `SessionStats`
- **budget**: `BudgetTracker` - dual budget system (tokens + time)
- **llm_bridge**: `LlmBridge`, `LlmBridgeConfig`, `QueryRequest`, `QueryResponse`
- **parser**: `CommandParser` - command parsing for query loop
- **query_loop**: `QueryLoop`, `QueryLoopConfig`, `QueryLoopResult`, `TerminationReason`
- **rlm**: `TerraphimRlm`, `SessionStatus`, `LlmQueryResult`
- **validator**: `KnowledgeGraphValidator` (kg-validation feature)
- **mcp_tools**: MCP tools integration (mcp feature)

## Constants

- `DEFAULT_TOKEN_BUDGET`: 100K tokens
- `DEFAULT_TIME_BUDGET_MS`: 5 minutes
- `DEFAULT_MAX_RECURSION_DEPTH`: 10
- `VM_ALLOCATION_TIMEOUT_MS`: 500ms
- `TARGET_BOOT_TIME_MS`: 2 seconds
- `DEFAULT_DNS_ALLOWLIST`: pypi.org, github.com, raw.githubusercontent.com

## Version

1.19.2 (workspace version)