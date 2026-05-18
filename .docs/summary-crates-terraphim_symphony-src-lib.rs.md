# Summary: terraphim_symphony/src/lib.rs

**Purpose:** Long-running orchestration service that reads from issue trackers, creates per-issue workspaces, and runs coding agent sessions.

**Core Components:**
- **OrchestratorRuntimeState**: Runtime state management
- **SymphonyOrchestrator**: Main orchestrator
- **WorkspaceManager**: Per-issue workspace lifecycle
- **IssueTracker**: Issue tracking integration (Gitea)
- **Worker**: Agent execution with outcome tracking

**Key Modules:**
- `api`: HTTP API endpoints
- `config`: Configuration management
- `error`: Error types
- `orchestrator`: Main orchestration logic
- `runner`: Agent execution with CodexSession
- `tracker`: Issue tracking (Gitea)
- `workspace`: Workspace lifecycle

**Key Exports:**
- `SymphonyOrchestrator`, `OrchestratorRuntimeState`, `StateSnapshot`
- `WorkerOutcome`, `AgentEvent`, `CodexSession`
- `ReviewAgentOutput`, `ReviewFinding`, `FindingCategory`, `FindingSeverity`
- `TokenCounts`, `TokenTotals`
- `Issue`, `IssueTracker`
- `WorkspaceManager`

**Features:**
- Isolated per-issue workspaces
- Agent session management (Codex)
- Finding deduplication
- Token tracking and budgets
- Gitea issue integration