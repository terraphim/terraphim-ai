# Terraphim TUI

Terraphim includes a comprehensive terminal user interface (TUI) that provides both interactive REPL functionality and CLI commands for advanced operations including VM management, web operations, and intelligent file operations with semantic awareness.

## Installation

Build from the workspace with optional feature flags:

```bash
# Build with all features (recommended)
cargo build -p terraphim_agent --features repl-full --release

# Build with specific features
cargo build -p terraphim_agent --features repl,repl-chat,repl-file,repl-mcp --release

# Build minimal TUI (basic functionality only)
cargo build -p terraphim_agent --release
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
- `repl-full` - All features enabled (recommended)

## Interactive REPL Mode

```bash
terraphim-agent
```

The TUI provides a comprehensive REPL (Read-Eval-Print Loop) with access to all features:

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
