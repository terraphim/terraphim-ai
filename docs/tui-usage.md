# Terraphim Terminal User Interface (TUI)

Terraphim includes a terminal user interface (TUI) that mirrors key features of the desktop app while providing a lightweight, command-line based experience. The TUI is ideal for CI/CD environments, headless servers, quick searches, and integration with terminal-based workflows.

## Installation

### Prerequisites
- Rust toolchain (cargo)
- Running Terraphim server (local or remote)

### Building from Source

Build the TUI from the workspace:

```bash
# Clone the repository (if you haven't already)
git clone https://github.com/terraphim/terraphim-ai.git
cd terraphim-ai

# Build just the TUI component
cargo build -p terraphim_tui --release

# The binary will be available at
# ./target/release/terraphim-tui
```

### Configuration

Set the Terraphim server URL (defaults to `http://localhost:8000`):

```bash
export TERRAPHIM_SERVER=http://localhost:8000
```

This environment variable is **required** for the TUI to connect to the server. If not set, the default will be used.

## Command Reference

### Interactive Mode

Launch the TUI in interactive mode (the default when no subcommand is specified):

```bash
terraphim-tui
```

In interactive mode:
- **Input box**: Type a query and press Enter to search
- **Suggestions**: View matching terms from the rolegraph
- **Results**: See ranked search results with titles
- **Navigation**: Press `q` to quit

### Search Command

Search for documents using the CLI:

```bash
terraphim-tui search --query "terraphim-graph" --role "Default" --limit 10
```

Parameters:
- `--query` (required): The search term or phrase
- `--role` (optional): Role name to use for search context (default: "Default")
- `--limit` (optional): Maximum number of results to return (default: 10)

Example output:
```
- 0.95    Introduction to Terraphim Graph
- 0.82    Graph-based Knowledge Representation
- 0.75    Implementing Graph Algorithms
```

### Roles Management

List available roles:

```bash
terraphim-tui roles list
```

Select a role for future queries:

```bash
terraphim-tui roles select "Engineer"
```

### Configuration Commands

Display current configuration:

```bash
terraphim-tui config show
```

Update configuration settings:

```bash
# Change selected role
terraphim-tui config set selected_role=Engineer

# Update global shortcut
terraphim-tui config set global_shortcut=Ctrl+X

# Change theme for a specific role
terraphim-tui config set role.Default.theme=spacelab
```

### Rolegraph Visualization

Display ASCII representation of the rolegraph:

```bash
terraphim-tui graph --role "Default" --top-k 10
```

Parameters:
- `--role` (optional): Role name to visualize (default: "Default")
- `--top-k` (optional): Number of top nodes to display (default: 50)

Example output:
```
Nodes: 1250  Edges: 3750
- [95] terraphim-graph -> knowledge representation, semantic networks, graph theory, ontology, rolegraph
- [87] knowledge management -> information retrieval, knowledge base, organization, metadata, taxonomy
- [76] graph database -> neo4j, arangodb, knowledge graph, query language, traversal
```

### Chat Command (OpenRouter Integration)

Interact with AI models through OpenRouter:

```bash
terraphim-tui chat --role "Default" --prompt "Summarize terraphim graph" --model anthropic/claude-3-sonnet
```

Parameters:
- `--prompt` (required): The message to send to the AI
- `--role` (optional): Role context to use (default: "Default")
- `--model` (optional): Specific model to use (overrides role default)

## Configuration Requirements

### Environment Variables

| Variable | Required | Default | Description |
|----------|----------|---------|-------------|
| `TERRAPHIM_SERVER` | Yes | `http://localhost:8000` | URL of the Terraphim server |
| `OPENROUTER_KEY` | No | None | OpenRouter API key for chat functionality |

### Server Requirements

The TUI requires a running Terraphim server with the following endpoints:
- `/config` - Configuration management
- `/config/selected_role` - Role selection
- `/documents/search` - Document search
- `/rolegraph` - Role graph data
- `/chat` - AI chat functionality (optional)

## Known Limitations

### OpenRouter Feature Flag

The chat functionality requires the server to be compiled with the `openrouter` feature flag:

```bash
# Server must be built with OpenRouter support
cargo build --package terraphim_server --features openrouter
```

Without this feature flag enabled during server compilation, the chat command will return an error.

### External Service Timeouts

- **Server Connection**: If the Terraphim server is unreachable, operations will time out after approximately 30 seconds
- **OpenRouter API**: Chat requests may time out after 60 seconds for complex or long requests
- **Network Connectivity**: Intermittent network issues may cause unexpected behavior

### Command Support Limitations

- Configuration editing is limited to `selected_role`, `global_shortcut`, and role themes
- Chat functionality lacks streaming responses (planned for future releases)
- ASCII graph visualization is limited to basic node-neighbor representation

## Integration with Terraphim Ecosystem

The TUI seamlessly integrates with the broader Terraphim ecosystem:

### Server Compatibility

The TUI is compatible with:
- Standard Terraphim server
- Custom server implementations that adhere to the API endpoints
- Cloud-hosted Terraphim instances

### Data Access

- Leverages the same rolegraph and knowledge base as the desktop application
- Uses identical search algorithms and relevance functions
- Maintains consistent role-based context switching

### Workflow Integration

The TUI can be integrated into existing workflows:
- CI/CD pipelines for knowledge retrieval
- Shell scripts for automated searches
- Terminal-based development environments

### Example Integration Script

```bash
#!/bin/bash
# Example of integrating Terraphim TUI into a build script

export TERRAPHIM_SERVER="http://knowledge.internal.example.com:8000"

# Run search and capture results
SEARCH_RESULTS=$(terraphim-tui search --query "deployment best practices" --role "DevOps" --limit 5)

# Process results
if echo "$SEARCH_RESULTS" | grep -q "deployment automation"; then
  echo "Found deployment automation documentation"
  # Additional processing...
fi
```

## Roadmap

Future planned enhancements:
- Expand `config set` key coverage and validation
- ASCII graph filters and alternative sort metrics
- Streaming chat into the TUI pane
- Thesaurus-backed suggestions when published by role config
