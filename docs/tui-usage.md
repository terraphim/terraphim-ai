# Terraphim Terminal User Interface (TUI)

Terraphim includes a comprehensive terminal user interface (TUI) that provides both interactive REPL functionality and CLI commands for advanced operations including VM management, web operations, and intelligent file operations with semantic awareness. The TUI is ideal for CI/CD environments, headless servers, development workflows, and integration with terminal-based automation.

## Installation

### Prerequisites
- Rust toolchain (cargo)
- Running Terraphim server (local or remote)

### Building from Source

Build the TUI from the workspace with optional feature flags:

```bash
# Clone the repository (if you haven't already)
git clone https://github.com/terraphim/terraphim-ai.git
cd terraphim-ai

# Build with all features (recommended)
cargo build -p terraphim_tui --features repl-full --release

# Build with specific features
cargo build -p terraphim_tui --features repl,repl-chat,repl-file,repl-mcp --release

# Build minimal TUI (basic functionality only)
cargo build -p terraphim_tui --release

# The binary will be available at
# ./target/release/terraphim-agent
```

### Feature Flags

The TUI supports modular feature flags for different capabilities:

- `repl` - Basic REPL functionality and search commands
- `repl-chat` - AI chat integration with OpenRouter and Ollama
- `repl-file` - Enhanced file operations with semantic awareness
- `repl-mcp` - Model Context Protocol (MCP) tools integration
- `repl-full` - All features enabled (recommended)

**Example:** Install with chat and file operations:
```bash
cargo install --path crates/terraphim_tui --features repl-chat,repl-file
```

### Configuration

Set the Terraphim server URL (defaults to `http://localhost:8000`):

```bash
export TERRAPHIM_SERVER=http://localhost:8000
```

This environment variable is **required** for the TUI to connect to the server. If not set, the default will be used.

## Command Reference

### Interactive REPL Mode

The TUI features a comprehensive REPL (Read-Eval-Print Loop) that provides access to all advanced functionality:

```bash
terraphim-agent
```

In interactive mode, you have access to:
- **Smart Search**: Type queries and get intelligent suggestions from the rolegraph
- **VM Management**: Control Firecracker VMs for isolated operations
- **Web Operations**: Perform web requests through secure VM sandboxing
- **File Operations**: Intelligent file management with semantic analysis
- **AI Chat**: Integrated AI assistant with context awareness
- **Configuration**: Real-time role and configuration management
- **Help System**: Built-in command help and suggestions

#### REPL Commands

**Navigation and Help:**
```bash
/help                    # Show all available commands
/quit                    # Exit the REPL
/clear                   # Clear the screen
```

**Search and Knowledge:**
```bash
/search "async rust"     # Search with semantic understanding
/graph                   # Show rolegraph visualization
/roles list              # List available roles
/role select Developer   # Switch to Developer role
```

**VM Management:**
```bash
/vm list                 # List all VMs
/vm create my-vm         # Create new VM
/vm start my-vm          # Start VM
/vm stop my-vm           # Stop VM
/vm status my-vm         # Check VM status
/vm logs my-vm           # View VM logs
```

**Web Operations:**
```bash
/web get https://api.example.com/data
/web post https://api.example.com/submit '{"data": "value"}'
/web scrape https://example.com '.content'
/web screenshot https://example.com
/web pdf https://example.com/article
/web history             # View operation history
```

**File Operations (Semantic):**
```bash
/file search "error handling" --path ./src --semantic
/file classify ./src --recursive --update-metadata
/file analyze ./main.rs --classification --extract-entities
/file summarize ./README.md --detailed --key-points
/file tag ./lib.rs rust,core,module --auto-suggest
/file index ./docs --recursive
/file metadata ./src/main.rs --extract-concepts
```

**AI Chat:**
```bash
/chat "Explain async patterns in Rust" --role Developer
/chat "Help me debug this issue" --context ./src/main.rs
```

### Search Command

Search for documents using the CLI:

```bash
terraphim-agent search --query "terraphim-graph" --role "Default" --limit 10
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
terraphim-agent roles list
```

Select a role for future queries:

```bash
terraphim-agent roles select "Engineer"
```

### Configuration Commands

Display current configuration:

```bash
terraphim-agent config show
```

Update configuration settings:

```bash
# Change selected role
terraphim-agent config set selected_role=Engineer

# Update global shortcut
terraphim-agent config set global_shortcut=Ctrl+X

# Change theme for a specific role
terraphim-agent config set role.Default.theme=spacelab
```

### Rolegraph Visualization

Display ASCII representation of the rolegraph:

```bash
terraphim-agent graph --role "Default" --top-k 10
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

### Chat Command (AI Integration)

Interact with AI models through OpenRouter or Ollama:

```bash
# REPL mode
/chat "Explain async patterns in Rust" --role Developer

# CLI mode
terraphim-agent chat --role "Default" --prompt "Summarize terraphim graph" --model anthropic/claude-3-sonnet
```

Parameters:
- `--prompt` (required): The message to send to the AI
- `--role` (optional): Role context to use (default: "Default")
- `--model` (optional): Specific model to use (overrides role default)
- `--context` (optional): File or directory to provide as context

**Supported AI Providers:**
- **OpenRouter**: Multiple models including Claude, GPT, Llama (requires `OPENROUTER_KEY`)
- **Ollama**: Local models (requires `OLLAMA_BASE_URL`)

### VM Management (Firecracker Integration)

Manage isolated Firecracker virtual machines for secure operations:

```bash
# List all VMs and their status
/vm list

# Create a new VM with specific configuration
/vm create my-dev-vm --cpu 2 --memory 2048 --image ubuntu

# Start and stop VMs
/vm start my-dev-vm
/vm stop my-dev-vm
/vm restart my-dev-vm

# Monitor VM status and resources
/vm status my-dev-vm
/vm metrics my-dev-vm

# View logs and connect to VM
/vm logs my-dev-vm
/vm shell my-dev-vm

# VM pool management
/vm pool list
/vm pool create dev-pool --size 3 --cpu 2 --memory 2048
```

**VM Features:**
- **Isolated Execution**: Secure sandboxing for sensitive operations
- **Resource Management**: CPU, memory, and storage allocation
- **Lifecycle Control**: Start, stop, pause, and restart operations
- **Monitoring**: Real-time metrics and health status
- **Integration**: Seamless integration with web and file operations

### Web Operations (VM-Sandboxed)

Perform web operations through secure VM isolation:

```bash
# HTTP requests
/web get https://api.example.com/data
/web post https://api.example.com/submit '{"data": "value"}' --headers "Content-Type: application/json"

# Web scraping and content extraction
/web scrape https://example.com '.content' --selector '.article'
/web screenshot https://example.com --full-page --wait 2000
/web pdf https://example.com/article --output article.pdf

# Form interactions
/web form https://example.com/login '{"username": "user", "password": "pass"}' --submit

# API exploration
/web api https://api.github.com /users/user1,/repos/repo1 --auth token

# Operation management
/web history
/web status webop-1642514400000
/web cancel webop-1642514400000

# Configuration
/web config show
/web config set timeout_ms 45000
/web config set user_agent "Terraphim-TUI/1.0"
```

**Web Operation Features:**
- **VM Sandboxing**: All requests run in isolated Firecracker VMs
- **Security**: No direct network access from host system
- **Versatility**: GET, POST, scraping, screenshots, PDF generation
- **Authentication**: Support for API keys, tokens, and form-based auth
- **Operation Tracking**: History, status monitoring, and cancellation

### File Operations (Semantic Intelligence)

Intelligent file operations with semantic understanding and analysis:

```bash
# Semantic file search
/file search "error handling" --path ./src --semantic --limit 10
/file search "async patterns" --file-types rs,js --recursive

# Content-based classification
/file classify ./src --recursive --update-metadata
/file analyze ./main.rs --classification --semantic --extract-entities

# Content summarization
/file summarize ./README.md --detailed --key-points
/file summarize ./docs/api.md --brief

# Semantic metadata extraction
/file metadata ./src/main.rs --extract-concepts --extract-entities --extract-keywords

# Intelligent file indexing
/file index ./docs --recursive --force-reindex
/file find "function_name" --path ./src --type rs

# Smart file listing
/file list ./src --show-metadata --show-tags --sort-by name
/file list ./src --sort-by relevance --limit 20

# Semantic file tagging
/file tag ./lib.rs rust,core,module --auto-suggest
/file suggest --context "error handling" --limit 5

# Operation status
/file status indexing
/file status classification
```

**File Operation Features:**
- **Semantic Search**: Understanding of content meaning, not just text matching
- **Content Classification**: Automatic detection of file types, languages, frameworks
- **Metadata Extraction**: Concepts, entities, keywords, complexity metrics
- **Relationship Discovery**: Find related files based on content similarity
- **Smart Tagging**: Automatic tag suggestions based on content analysis
- **Performance Analysis**: Reading time estimation and complexity scoring

**File Categories Supported:**
- **Code**: Language detection (Rust, JavaScript, Python, etc.) with framework identification
- **Documentation**: Markdown, HTML, text files with complexity analysis
- **Configuration**: JSON, YAML, TOML, XML with purpose inference
- **Data**: CSV, JSON data files with structure analysis
- **Media**: Images, videos, audio files with metadata extraction
- **Archives**: ZIP, TAR, GZ with content type detection
- **Scripts**: Shell scripts, batch files with purpose analysis

## Configuration Requirements

### Environment Variables

| Variable | Required | Default | Description |
|----------|----------|---------|-------------|
| `TERRAPHIM_SERVER` | Yes | `http://localhost:8000` | URL of the Terraphim server |
| `OPENROUTER_KEY` | No | None | OpenRouter API key for chat functionality |
| `OLLAMA_BASE_URL` | No | `http://127.0.0.1:11434` | Ollama server URL for local models |
| `VM_STORAGE_PATH` | No | `./vm-storage` | Directory for VM disk images |
| `VM_NETWORK_BRIDGE` | No | `virbr0` | Network bridge for VM networking |
| `WEB_OPERATION_TIMEOUT` | No | `30000` | Default timeout for web operations (ms) |
| `FILE_INDEX_PATH` | No | `./file-index` | Directory for file operation indexes |

### VM Configuration

The VM management requires Firecracker and proper system configuration:

```bash
# Install Firecracker (Ubuntu/Debian)
sudo apt-get update
sudo apt-get install -y firecracker

# Set up kernel permissions for microVMs
sudo sysctl -w vm.nr_hugepages=1024
sudo chmod 666 /dev/kvm
sudo chown $USER:$USER /dev/kvm

# Create VM storage directory
mkdir -p ./vm-storage
chmod 755 ./vm-storage
```

**VM Requirements:**
- Linux kernel with KVM support
- Firecracker binary installed
- Proper permissions for `/dev/kvm` and `/dev/vhost-net`
- Sufficient memory and CPU for microVMs

### MCP Tools Configuration

For Model Context Protocol integration:

```bash
# Set MCP server URL (optional)
export MCP_SERVER_URL=http://localhost:3001

# Set MCP authentication token (if required)
export MCP_AUTH_TOKEN=your-mcp-token
```

### Server Requirements

The TUI requires a running Terraphim server with the following endpoints:
- `/config` - Configuration management
- `/config/selected_role` - Role selection
- `/documents/search` - Document search
- `/rolegraph` - Role graph data
- `/chat` - AI chat functionality (optional)
- `/documents/summarize` - AI-powered document summarization (optional)

**Feature-specific server requirements:**
- **Chat functionality**: Server compiled with `openrouter` feature flag
- **VM operations**: Firecracker integration and proper system permissions
- **File operations**: Sufficient disk space for indexes and metadata
- **Web operations**: Network connectivity through VM sandboxing

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
- **VM Operations**: VM startup and shutdown may take 10-30 seconds depending on system resources
- **Web Operations**: Timeout configurable via `WEB_OPERATION_TIMEOUT` (default: 30 seconds)
- **File Indexing**: Large repositories may require significant time for initial indexing
- **Network Connectivity**: Intermittent network issues may cause unexpected behavior

### Command Support Limitations

- Configuration editing is limited to `selected_role`, `global_shortcut`, and role themes
- Chat functionality lacks streaming responses (planned for future releases)
- ASCII graph visualization is limited to basic node-neighbor representation
- VM operations require root privileges and proper KVM setup
- File operations are currently read-only (no file modification capabilities)
- Web operations through VMs may have limited JavaScript execution support

### Platform Limitations

- **VM Management**: Currently only supported on Linux with KVM support
- **Firecracker**: Requires specific kernel configurations and permissions
- **File Operations**: Semantic analysis optimized for code and documentation files
- **Web Scraping**: Complex single-page applications may have limited support

### Resource Requirements

- **VM Operations**: Minimum 2GB RAM recommended for running microVMs
- **File Indexing**: Disk space required for semantic indexes (varies by repository size)
- **Web Operations**: Memory usage scales with the complexity of web pages processed

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

### Example Integration Scripts

**Build Script Integration:**
```bash
#!/bin/bash
# Example of integrating Terraphim TUI into a build script

export TERRAPHIM_SERVER="http://knowledge.internal.example.com:8000"

# Run search and capture results
SEARCH_RESULTS=$(terraphim-agent search --query "deployment best practices" --role "DevOps" --limit 5)

# Process results
if echo "$SEARCH_RESULTS" | grep -q "deployment automation"; then
  echo "Found deployment automation documentation"
  # Additional processing...
fi
```

**Code Review Automation:**
```bash
#!/bin/bash
# Automated code analysis using TUI file operations

# Classify files in the repository
terraphim-agent file classify ./src --recursive --update-metadata

# Find potential issues
terraphim-agent file search "TODO" "FIXME" --path ./src --semantic

# Generate summary of changes
terraphim-agent file summarize ./CHANGELOG.md --detailed
```

**Security Analysis:**
```bash
#!/bin/bash
# Security analysis using VM-sandboxed web operations

# Check dependencies for known vulnerabilities
terraphim-agent web get "https://api.github.com/advisories?ecosystem=npm" --auth "$GITHUB_TOKEN"

# Scan web application securely
terraphim-agent web screenshot "https://app.example.com" --full-page
terraphim-agent web scrape "https://app.example.com" '.security-info'
```

## Roadmap

### Near-term (Next 3 months)
- **Enhanced VM Management**: VM snapshots, cloning, and migration
- **Streaming Chat**: Real-time streaming responses in the TUI
- **File Editing**: Safe file modification capabilities with backup
- **Advanced Web Scraping**: JavaScript execution and dynamic content support
- **Performance Improvements**: Optimized indexing and caching

### Medium-term (3-6 months)
- **Cross-platform VM Support**: Windows and macOS VM management
- **Collaborative Features**: Shared sessions and real-time collaboration
- **Advanced Analytics**: Usage statistics and performance metrics
- **Plugin System**: Extensible architecture for custom operations
- **Enhanced Security**: Certificate management and secure enclaves

### Long-term (6+ months)
- **GUI Integration**: Seamless integration with desktop application
- **Mobile Support**: Mobile-friendly interface and capabilities
- **Cloud Integration**: Direct cloud service integrations (AWS, GCP, Azure)
- **AI-Powered Automation**: Intelligent workflow automation
- **Enterprise Features**: SSO, audit logging, and compliance tools

### Community Contributions
We welcome contributions for:
- **Language Support**: Expanding semantic analysis to more programming languages
- **Documentation**: Improving documentation and adding examples
- **Testing**: Comprehensive test coverage for all features
- **Performance**: Optimization and benchmarking improvements
- **Accessibility**: Making the TUI more accessible to all users
