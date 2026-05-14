# Summary: terraphim_agent/src/lib.rs

**Purpose:** TUI, robot mode, and multi-agent coordination for Terraphim AI.

**Feature-Gated Modules:**
- `server`: HTTP client for API communication
- `repl`: Interactive REPL
- `repl-custom`: Custom REPL commands
- `shared-learning`: Shared learning store

**Always-Available Modules:**
- `robot`: Robot mode for AI agent integration
- `forgiving`: Forgiving CLI with typo-tolerant parsing
- `mcp_tool_index`: MCP tool discovery and search
- `tui_backend`: Terminal UI backend
- `onboarding`: User onboarding workflows

**Key Exports:**

| Module | Types |
|--------|-------|
| `robot` | `BudgetEngine`, `BudgetError`, `BudgetedResults`, `ExitCode`, `OutputFormat`, `RobotConfig`, `RobotResponse` |
| `forgiving` | `AliasRegistry`, `ForgivingParser`, `ParseResult` |
| `repl` | REPL types and functions |
| `commands` | Custom command implementations |

**Robot Mode:**
- JSON output format for AI agent integration
- Budget engine for result limiting
- Formatter for structured output
- Self-documentation support

**Forgiving Parser:**
- Typo-tolerant command parsing
- Alias registry for command shortcuts
- Parse result with alternatives

**MCP Tool Index:**
- Discovers MCP (Model Context Protocol) tools
- Provides search functionality
- Integrates with agent workflows

**Testing:**
- `test_exports` module exposes internals for testing
- Feature-gated test support