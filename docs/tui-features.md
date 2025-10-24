# Terraphim TUI Features

This document provides a comprehensive overview of all features available in the Terraphim Terminal User Interface.

## Table of Contents

- [Interactive REPL](#interactive-repl)
- [VM Management](#vm-management)
- [Web Operations](#web-operations)
- [File Operations](#file-operations)
- [AI Chat Integration](#ai-chat-integration)
- [Knowledge Graph](#knowledge-graph)
- [Configuration Management](#configuration-management)
- [MCP Tools Integration](#mcp-tools-integration)

## Interactive REPL

The Terraphim TUI provides a powerful REPL (Read-Eval-Print Loop) that gives you access to all functionality through an intuitive command interface.

### Starting the REPL

```bash
terraphim-tui
```

### REPL Features

- **Smart Autocompletion**: Context-aware command suggestions
- **Syntax Highlighting**: Color-coded commands and output
- **Command History**: Navigate through previous commands
- **Error Handling**: Clear error messages and suggestions
- **Progress Indicators**: Real-time feedback for long-running operations

### REPL Commands Reference

```bash
# Navigation and Help
/help                    # Show comprehensive help
/quit                    # Exit the REPL
/clear                   # Clear the screen
/history                 # Show command history

# Search and Discovery
/search "query"          # Semantic search
/search "query" --role Developer --limit 10
/graph                   # Show rolegraph visualization
/roles list              # List available roles
/role select Developer    # Switch to Developer role
```

## VM Management

Manage Firecracker virtual machines for secure, isolated operations.

### Prerequisites

- Linux with KVM support
- Firecracker binary installed
- Proper permissions for `/dev/kvm`

### VM Commands

```bash
# VM Lifecycle Management
/vm list                 # List all VMs with status
/vm create my-vm         # Create new VM
/vm create dev-vm --cpu 2 --memory 2048 --image ubuntu
/vm start my-vm          # Start VM
/vm stop my-vm           # Stop VM
/vm restart my-vm        # Restart VM
/vm delete my-vm         # Delete VM

# VM Monitoring
/vm status my-vm         # Check VM status
/vm metrics my-vm        # View resource usage
/vm logs my-vm           # View VM logs
/vm shell my-vm          # Connect to VM shell

# VM Pool Management
/vm pool list            # List VM pools
/vm pool create dev-pool --size 3 --cpu 2 --memory 2048
/vm pool status dev-pool # Check pool status
```

### VM Features

- **Isolated Execution**: Complete sandboxing for security
- **Resource Management**: CPU, memory, storage allocation
- **Network Isolation**: Separate network namespaces
- **Snapshot Support**: Save and restore VM states
- **Performance Monitoring**: Real-time metrics and health checks

## Web Operations

Perform web operations through secure VM isolation.

### Web Commands

```bash
# HTTP Requests
/web get https://api.example.com/data
/web post https://api.example.com/submit '{"data": "value"}'
/web put https://api.example.com/update '{"field": "newvalue"}'
/web delete https://api.example.com/resource/123

# Headers and Authentication
/web get https://api.example.com/data --headers "Authorization: Bearer token"
/web post https://api.example.com/submit '{"data": "value"}' --auth basic:username:password

# Web Scraping
/web scrape https://example.com '.content'
/web scrape https://example.com '.article' --selector '.title'
/web scrape https://example.com '.product' --extract-images

# Screenshots and PDFs
/web screenshot https://example.com
/web screenshot https://example.com --full-page --wait 2000
/web pdf https://example.com/article
/web pdf https://example.com/documentation --output docs.pdf

# Form Interactions
/web form https://example.com/login '{"username": "user", "password": "pass"}'
/web form https://example.com/login '{"username": "user", "password": "pass"}' --submit

# API Exploration
/web api https://api.github.com /users/user1,/repos/repo1
/web api https://api.example.com /endpoints --auth token

# Operation Management
/web history             # View operation history
/web status webop-1642514400000
/web cancel webop-1642514400000
/web retry webop-1642514400000

# Configuration
/web config show
/web config set timeout_ms 45000
/web config set user_agent "Terraphim-TUI/1.0"
/web config set proxy "http://proxy.example.com:8080"
```

### Web Operation Features

- **VM Sandboxing**: All requests run in isolated microVMs
- **Multiple HTTP Methods**: GET, POST, PUT, DELETE, PATCH
- **Authentication Support**: Basic auth, Bearer tokens, API keys
- **Content Handling**: JSON, XML, form data, file uploads
- **Media Capture**: Screenshots, PDFs, image extraction
- **Operation Tracking**: History, status, retry, cancellation

## File Operations

Intelligent file operations with semantic understanding and analysis.

### File Commands

```bash
# Semantic File Search
/file search "error handling" --path ./src --semantic
/file search "async patterns" --file-types rs,js --recursive
/file search "TODO" --path . --semantic --limit 20

# Content Classification
/file classify ./src --recursive --update-metadata
/file classify ./docs --recursive --output classification.json
/classify ./project --report detailed

# Content Analysis
/file analyze ./main.rs --classification --semantic --extract-entities
/file analyze ./README.md --summarization --relationship-analysis
/file analyze ./src --all-analysis-types --output analysis.json

# Content Summarization
/file summarize ./README.md --detailed --key-points
/file summarize ./docs/api.md --brief
/file summarize ./documentation/ --comprehensive --output-summary summary.md

# Metadata Extraction
/file metadata ./src/main.rs --extract-concepts --extract-entities --extract-keywords
/file metadata ./docs --update-index --semantic-fingerprint
/file metadata ./project --complexity-analysis --reading-time

# File Indexing
/file index ./docs --recursive --force-reindex
/file index ./src --semantic-index --update-existing
/file reindex ./project --incremental

# Pattern Finding
/file find "function_name" --path ./src --type rs
/file find "TODO|FIXME" --path . --regex --recursive
/file find "error" --context 2 --path ./src

# Smart File Listing
/file list ./src --show-metadata --show-tags --sort-by name
/file list ./docs --sort-by relevance --limit 20
/file list ./ --type-filter code --recursive

# Semantic Tagging
/file tag ./lib.rs rust,core,module --auto-suggest
/file tag ./docs README.md,API,documentation --semantic-analysis
/file tag ./project --batch --suggest-threshold 0.8

# File Suggestions
/file suggest --context "error handling" --limit 5
/file suggest --path ./src --semantic --similar-to ./main.rs
/file suggest --context "authentication" --type-filter code

# Operation Status
/file status indexing
/file status classification
/file status all --verbose
```

### File Operation Features

- **Semantic Search**: Understanding of content meaning, not just text matching
- **Content Classification**: Automatic detection of file types, languages, frameworks
- **Metadata Extraction**: Concepts, entities, keywords, complexity metrics
- **Relationship Discovery**: Find related files based on content similarity
- **Smart Tagging**: Automatic tag suggestions based on content analysis
- **Performance Analysis**: Reading time estimation and complexity scoring

### Supported File Types

- **Code**: Rust, JavaScript, Python, Go, Java, C++, etc.
- **Documentation**: Markdown, HTML, LaTeX, Text
- **Configuration**: JSON, YAML, TOML, XML, INI
- **Data**: CSV, JSON Lines, Parquet, SQLite
- **Media**: Images (PNG, JPG, SVG), Videos, Audio
- **Archives**: ZIP, TAR, GZ, 7Z, RAR
- **Scripts**: Shell, Batch, PowerShell, Python scripts

## AI Chat Integration

Conversational AI integration with multiple model providers.

### Chat Commands

```bash
# Basic Chat
/chat "Explain async patterns in Rust"
/chat "Help me debug this authentication issue"

# Context-Aware Chat
/chat "Review this code" --context ./src/main.rs
/chat "Explain this error" --context ./logs/error.log --role Developer

# Role-Based Chat
/chat "Best practices for API design" --role Architect
/chat "Security considerations for this implementation" --role Security

# Model Selection
/chat "Compare these approaches" --model anthropic/claude-3-sonnet
/chat "Generate Python code" --model openai/gpt-4

# Advanced Chat Features
/chat "Previous conversation summary" --history 5
/chat "Continue previous discussion" --session-id abc123
```

### AI Providers

- **OpenRouter**: Claude, GPT-4, Llama, and more (requires `OPENROUTER_KEY`)
- **Ollama**: Local models like Llama3, Mistral (requires `OLLAMA_BASE_URL`)

### Chat Features

- **Context Awareness**: Uses provided files and directories as context
- **Role-Based Responses**: Tailored responses based on selected role
- **History Management**: Conversation history and session persistence
- **Multi-Model Support**: Switch between different AI models
- **Streaming Responses**: Real-time response streaming (planned)

## Knowledge Graph

Interactive visualization and navigation of the rolegraph knowledge structure.

### Graph Commands

```bash
# Basic Graph Visualization
/graph                   # Show current role's rolegraph
/graph --role Developer  # Show specific role's graph
/graph --top-k 20       # Limit to top 20 nodes
/graph --min-weight 5   # Filter by connection strength

# Graph Analysis
/graph --statistics      # Show graph statistics
/graph --connections     # Show node connections
/graph --clusters        # Identify concept clusters
/graph --paths "concept1" "concept2"  # Find paths between concepts

# Graph Export
/graph --export json     # Export as JSON
/graph --export dot      # Export as Graphviz DOT
/graph --export mermaid   # Export as Mermaid diagram
```

### Graph Features

- **Interactive Visualization**: ASCII art representation of knowledge graphs
- **Role-Based Views**: Different knowledge graphs for different roles
- **Path Finding**: Discover connections between concepts
- **Cluster Analysis**: Identify related concept groups
- **Export Capabilities**: Export graphs in various formats

## Configuration Management

Real-time configuration and role management.

### Configuration Commands

```bash
# Role Management
/roles list              # List available roles
/roles select Developer  # Switch to Developer role
/roles create "New Role" --template developer
/roles delete "Test Role"

# Configuration Display
/config show             # Show current configuration
/config show --role Developer  # Show role-specific config
/config list available   # List all configuration options

# Configuration Updates
/config set selected_role=Developer
/config set global_shortcut=Ctrl+X
/config set role.Default.theme=spacelab
/config set search.max_results=20

# Configuration Export/Import
/config export --file my-config.json
/config import --file my-config.json
/config reset            # Reset to defaults
```

### Configuration Features

- **Role Switching**: Dynamic role changes during runtime
- **Real-Time Updates**: Configuration changes take effect immediately
- **Validation**: Configuration validation and error reporting
- **Import/Export**: Backup and restore configurations
- **Templates**: Role configuration templates

## MCP Tools Integration

Model Context Protocol integration for extended tool capabilities.

### MCP Commands

```bash
# MCP Connection
/mcp connect http://localhost:3001
/mcp disconnect
/mcp status              # Show connection status
/mcp list-tools          # List available tools

# Tool Execution
/mcp run search_files --query "async" --path ./src
/mcp run analyze_code --file ./main.rs
/mcp run generate_docs --project ./

# MCP Configuration
/mcp config show
/mcp config set timeout=30
/mcp config set auth_token=your-token
```

### MCP Features

- **Tool Discovery**: Automatic tool discovery and registration
- **Secure Communication**: Encrypted communication with MCP servers
- **Tool Execution**: Execute remote tools with local context
- **Authentication**: Secure authentication with MCP servers

## Advanced Usage Patterns

### Workflow Automation

```bash
# Code Review Workflow
/file classify ./src --recursive --update-metadata
/file search "TODO|FIXME" --path ./src --semantic
/chat "Review these code changes" --context ./src/main.rs
```

### Security Analysis

```bash
# Security Review Workflow
/vm create security-sandbox --image security-tools
/web get "https://api.github.com/advisories?ecosystem=rust" --auth $GITHUB_TOKEN
/file search "password|secret|token" --path ./src --semantic
/chat "Security analysis of this code" --context ./src/auth.rs
```

### Documentation Generation

```bash
# Documentation Workflow
/file index ./docs --recursive --force-reindex
/file summarize ./src --detailed --key-points --output-summary docs-summary.md
/chat "Generate API documentation" --context ./src/api/
/graph --export mermaid --output docs/architecture.mmd
```

## Integration Examples

### CI/CD Pipeline Integration

```bash
#!/bin/bash
# Build pipeline with TUI integration

export TERRAPHIM_SERVER="http://knowledge.internal.company.com"

# Analyze changes
terraphim-tui file classify ./src --recursive --update-metadata
terraphim-tui file search "BREAKING CHANGE" --path ./CHANGELOG.md

# Generate release notes
terraphim-tui file summarize ./CHANGELOG.md --detailed --key-points
terraphim-tui chat "Generate release notes for version 1.2.0" --context ./CHANGELOG.md

# Security scan
terraphim-tui file search "hardcoded.*password|secret.*key" --path ./src --semantic
```

### Development Workflow Integration

```bash
#!/bin/bash
# Development helper script

# Code analysis
terraphim-tui file analyze ./src/main.rs --all-analysis-types
terraphim-tui file suggest --context "improve performance" --path ./src

# Documentation
terraphim-tui file summarize ./README.md --brief
terraphim-tui chat "Generate API examples" --context ./src/api/

# Testing
terraphim-tui file search "unittest|test" --path ./src --semantic
terraphim-tui vm create test-env --image testing-tools
```

## Performance Considerations

- **VM Operations**: Allow 10-30 seconds for VM startup/shutdown
- **File Indexing**: Initial indexing may take time for large repositories
- **Web Operations**: Configure timeouts based on network conditions
- **Memory Usage**: Monitor memory usage during large file operations
- **Disk Space**: Ensure sufficient space for VM images and file indexes

## Troubleshooting

### Common Issues

1. **VM operations fail**: Check KVM permissions and Firecracker installation
2. **Web operations timeout**: Increase `WEB_OPERATION_TIMEOUT`
3. **File operations slow**: Consider limiting scope or using filters
4. **Chat not working**: Verify API keys and server feature flags

### Debug Mode

```bash
# Enable debug logging
export LOG_LEVEL=debug
terraphim-tui

# Check feature availability
terraphim-tui /help
```

For more detailed troubleshooting, see the [main TUI documentation](docs/tui-usage.md).
