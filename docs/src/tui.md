# Terraphim TUI

Terraphim includes a terminal interface with three execution modes:
- Fullscreen TUI (`terraphim-agent`) - server-backed interactive UI
- REPL (`terraphim-agent repl`) - offline-capable by default
- CLI subcommands (`terraphim-agent <command>`) - offline-capable by default unless `--server` is used

## Installation

Build from the workspace with optional feature flags:

```bash
# Build with all features (recommended)
cargo build -p terraphim_tui --features repl-full --release

# Build with specific features
cargo build -p terraphim_tui --features repl,repl-chat,repl-file,repl-mcp --release

# Build minimal TUI (basic functionality only)
cargo build -p terraphim_tui --release
```

Binary: `terraphim-agent`

Set the server URL (defaults to `http://localhost:8000`):

```bash
export TERRAPHIM_SERVER=http://localhost:8000
```

## Feature Flags

- `repl` - Basic REPL functionality and search commands
- `repl-chat` - AI chat integration with OpenRouter and Ollama
- `repl-file` - Enhanced file operations with semantic awareness
- `repl-mcp` - Model Context Protocol (MCP) tools integration
- `repl-sessions` - AI coding session history search (Claude Code, Cursor, Aider)
- `repl-full` - All features enabled (recommended)

## Interactive Modes

```bash
# Fullscreen TUI (server-backed)
terraphim-agent

# REPL (offline-capable)
terraphim-agent repl
```

Use `terraphim-agent repl --server --server-url http://localhost:8000` for server-backed REPL.

REPL mode provides access to all slash commands below:

**Navigation and Help:**
- `/help` - Show all available commands
- `/quit` - Exit the REPL
- `/clear` - Clear the screen

**Search and Knowledge:**
- `/search "query"` - Semantic search with role context
- `/graph` - Rolegraph visualization
- `/roles list` - List available roles
- `/role select name` - Switch role

**VM Management** (requires Firecracker):
- `/vm list` - List all VMs
- `/vm create name` - Create new VM
- `/vm start/stop name` - Control VM lifecycle
- `/vm status name` - Check VM status

**Web Operations** (VM-sandboxed):
- `/web get URL` - HTTP GET request
- `/web post URL data` - HTTP POST with data
- `/web scrape URL selector` - Web scraping
- `/web screenshot URL` - Capture screenshot
- `/web history` - View operation history

**File Operations** (semantic analysis):
- `/file search "query"` - Semantic file search
- `/file classify path` - Content-based classification
- `/file analyze file` - Multi-type analysis
- `/file summarize file` - Content summarization
- `/file tag file tags` - Semantic tagging

**AI Chat:**
- `/chat "message"` - Interactive AI conversation

**Session Search** (requires `repl-sessions` feature):
- `/sessions sources` - Detect available session sources
- `/sessions import [source] [--limit N]` - Import sessions
- `/sessions list [source] [--limit N]` - List imported sessions
- `/sessions search "query"` - Full-text search across sessions
- `/sessions stats` - Show session statistics
- `/sessions show <id>` - Show session details
- `/sessions concepts "term"` - Knowledge graph concept search
- `/sessions related <id> [--min N]` - Find related sessions
- `/sessions timeline [--group day|week|month]` - Timeline view
- `/sessions export [--format json|md] [--output file]` - Export sessions
- `/sessions enrich [id]` - Enrich with knowledge graph concepts

## CLI subcommands

Traditional CLI commands are also supported:

- **Search**
  ```bash
  terraphim-agent search --query "terraphim-graph" --role "Default" --limit 10
  ```

- **Roles**
  ```bash
  terraphim-agent roles list
  terraphim-agent roles select "Default"
  ```

- **Config**
  ```bash
  terraphim-agent config show
  terraphim-agent config set selected_role=Default
  terraphim-agent config set global_shortcut=Ctrl+X
  terraphim-agent config set role.Default.theme=spacelab
  ```

- **Rolegraph (ASCII)**
  ```bash
  terraphim-agent graph --role "Default" --top-k 10
  # Prints: - [rank] label -> neighbor1, neighbor2, ...
  ```

- **Chat** (OpenRouter/Ollama)
  ```bash
  terraphim-agent chat --role "Default" --prompt "Summarize terraphim graph" --model anthropic/claude-3-sonnet
  ```

## Behavior

- `terraphim-agent` (no args) runs fullscreen TUI and requires a reachable server.
- `terraphim-agent repl` runs REPL offline by default (or server mode with `--server`).
- CLI subcommands (for example `search`, `config`, `roles`) run offline-capable mode by default.
- Uses `/config`, `/config/selected_role`, `/documents/search`, and `/rolegraph` endpoints.
- Chat posts to `/chat` (requires server compiled with openrouter feature and configured role or `OPENROUTER_KEY`).
- VM operations require Firecracker integration and proper system permissions.
- Web operations run in isolated microVMs for security.
- File operations use semantic analysis and content understanding.
- Suggestions source labels from `/rolegraph` for the selected role.

## Key Features

### VM Management
- Firecracker microVM integration for secure isolation
- VM lifecycle management (create, start, stop, delete)
- Resource monitoring and metrics
- VM pool management for scaling

### Web Operations
- Secure web requests through VM sandboxing
- HTTP methods: GET, POST, PUT, DELETE
- Web scraping and screenshot capture
- PDF generation and content extraction
- Operation history and status tracking

### File Operations
- Semantic file search and classification
- Content analysis and metadata extraction
- File relationship discovery
- Intelligent tagging and suggestions
- Reading time estimation and complexity scoring

### AI Integration
- OpenRouter and Ollama model support
- Context-aware conversations
- Role-based AI interactions
- Streaming responses (planned)

### Session Search
- Multi-source support: Claude Code, Cursor, Aider, OpenCode
- Full-text search across all messages and metadata
- Knowledge graph concept enrichment for semantic search
- Related session discovery by shared concepts
- Timeline visualization by day, week, or month
- Export to JSON or Markdown formats
- Session statistics and analytics

**Supported Sources:**

| Source | Location | Description |
|--------|----------|-------------|
| claude-code-native | `~/.claude/projects/` | Native Claude Code sessions |
| claude-code | `~/.claude/projects/` | CLA-parsed Claude Code sessions |
| cursor | `~/.cursor/` | Cursor IDE sessions |
| aider | `.aider.chat.history.md` | Aider chat history |

**Example Workflow:**

```bash
# Launch REPL
terraphim-agent

# In REPL:
/sessions sources              # See available sources
/sessions import --limit 100   # Import sessions
/sessions search "rust async"  # Search for topics
/sessions concepts "error"     # Concept-based search
/sessions timeline --group week # View timeline
```

## Roadmap

### Near-term
- Enhanced VM management (snapshots, cloning)
- Streaming chat responses
- File editing capabilities
- Advanced web scraping (JavaScript support)

### Medium-term
- Cross-platform VM support
- Plugin system for extensibility
- Collaborative features
- Advanced analytics and metrics

### Long-term
- GUI integration
- Mobile support
- Cloud service integrations
- Enterprise features (SSO, audit logging)
