# Terraphim AI Documentation

Complete documentation for Terraphim AI - a privacy-first semantic search assistant.

## Quick Start

- [Installation Guide](./installation.md) - Getting started with Terraphim
- [Platform-Specific Installation](./platform-specific-installation.md) - OS-specific setup instructions
- [TUI Usage Guide](./tui-usage.md) - Using the Terminal UI
- [Context Engineering Quick Start](./context-engineering-quick-start.md) - Setting up your knowledge context

## Core Features

### Terminal User Interface (TUI)

- **[TUI Features](./tui-features.md)** - Complete feature list and capabilities
- **[TUI Usage Guide](./tui-usage.md)** - Comprehensive usage instructions
- **[Command Execution System](./command-execution-system.md)** ⭐ NEW
  - Local, Hybrid, and Firecracker VM execution modes
  - Intelligent risk assessment and security
  - Complete execution flow examples
  - REPL integration guide

### Configuration & Setup

- [Deployment Guide](./deployment.md) - Production deployment instructions
- [LLM Proxy Configuration](./llm-proxy-configuration.md) - AI model integration
- [Perplexity Integration](./perplexity-integration.md) - Search engine integration

## Architecture & Design

- [Component Diagram](./component-diagram.md) - System architecture overview
- [Knowledge Graph](./knowledge_graph_1758618546122.md) - Semantic graph structure
- [Context Collections](./context-collections.md) - Managing knowledge contexts

## Advanced Features

### MCP (Model Context Protocol)

- [MCP File Context Tools](./mcp-file-context-tools.md) - File operations and indexing
- [Command Execution System](./command-execution-system.md) - Multi-mode execution with VMs

### Integrations

- [Conare Comparison](./conare-comparison.md) - Alternative approaches
- [Context Engineering](./context-engineering-quick-start.md) - Advanced context setup

## Development & Contributing

### Implementation Notes

- [Bun Replacement Implementation](./BUN_REPLACEMENT_IMPLEMENTATION.md) - Runtime migration notes
- [Conare Implementation Summary](./CONARE_IMPLEMENTATION_SUMMARY.md) - Alternative implementation
- [Ripgrep Tag Filtering](./RIPGREP_TAG_FILTERING.md) - Search optimization

### Release & CI/CD

- [Release Process](./RELEASE_PROCESS.md) - How to release new versions
- [CI Status (PR #179)](./CI_STATUS_PR179.md) - Continuous integration status
- [OpenRouter Testing Plan](./OPENROUTER_TESTING_PLAN.md) - LLM testing strategy

## Documentation Index

### By Category

#### Getting Started
- [Installation](./installation.md)
- [Platform-Specific Installation](./platform-specific-installation.md)
- [Context Engineering Quick Start](./context-engineering-quick-start.md)
- [TUI Usage Guide](./tui-usage.md)

#### User Guides
- [TUI Features](./tui-features.md)
- [TUI Usage](./tui-usage.md)
- [Command Execution System](./command-execution-system.md)

#### Configuration
- [Deployment](./deployment.md)
- [LLM Proxy Configuration](./llm-proxy-configuration.md)
- [Perplexity Integration](./perplexity-integration.md)

#### Architecture
- [Component Diagram](./component-diagram.md)
- [Knowledge Graph](./knowledge_graph_1758618546122.md)
- [Context Collections](./context-collections.md)

#### Advanced Topics
- [MCP File Context Tools](./mcp-file-context-tools.md)
- [Command Execution System](./command-execution-system.md)
- [Conare Comparison](./conare-comparison.md)

#### Development
- [Bun Replacement Implementation](./BUN_REPLACEMENT_IMPLEMENTATION.md)
- [Conare Implementation Summary](./CONARE_IMPLEMENTATION_SUMMARY.md)
- [Ripgrep Tag Filtering](./RIPGREP_TAG_FILTERING.md)
- [Release Process](./RELEASE_PROCESS.md)
- [CI Status (PR #179)](./CI_STATUS_PR179.md)
- [OpenRouter Testing Plan](./OPENROUTER_TESTING_PLAN.md)

### By Audience

#### For End Users
1. Start with [Installation](./installation.md)
2. Read [TUI Usage Guide](./tui-usage.md)
3. Learn about [TUI Features](./tui-features.md)
4. Explore [Command Execution System](./command-execution-system.md)

#### For Administrators
1. Read [Deployment Guide](./deployment.md)
2. Configure [LLM Proxy](./llm-proxy-configuration.md)
3. Set up [Perplexity Integration](./perplexity-integration.md)
4. Review [Command Execution System](./command-execution-system.md) for security

#### For Developers
1. Understand [Component Architecture](./component-diagram.md)
2. Study [Command Execution System](./command-execution-system.md)
3. Review [MCP File Context Tools](./mcp-file-context-tools.md)
4. Follow [Release Process](./RELEASE_PROCESS.md)

## Key Highlights

### ⭐ Command Execution System (NEW)

The **[Command Execution System](./command-execution-system.md)** provides secure, multi-mode command execution:

- **Local Mode** - Safe whitelisted commands (ls, cat, grep, etc.)
- **Hybrid Mode** - Intelligent risk assessment and automatic mode selection
- **Firecracker Mode** - Complete VM isolation for high-risk operations

**Features:**
- ✅ Multi-layer security validation
- ✅ Automatic risk assessment
- ✅ Resource limiting and monitoring
- ✅ Hook system for extensibility
- ✅ Full REPL integration
- ✅ Complete audit logging

[Read the full documentation →](./command-execution-system.md)

### 🖥️ Terminal UI (TUI)

Rich terminal interface with:
- Interactive search
- Keyboard shortcuts with Ctrl modifiers (Ctrl+R, Ctrl+S, Ctrl+Q)
- Real-time autocomplete
- Multi-role support
- Knowledge graph integration

[Learn more about TUI features →](./tui-features.md)

### 🔌 MCP Integration

Model Context Protocol support for:
- File operations and indexing
- Semantic search
- Command execution in isolated VMs
- AI tool integration

[Explore MCP tools →](./mcp-file-context-tools.md)

## Contributing

Terraphim AI is open source and welcomes contributions. See the main [README](../README.md) for contribution guidelines.

## Support

- **Issues**: [GitHub Issues](https://github.com/terraphim/terraphim-ai/issues)
- **Discussions**: [GitHub Discussions](https://github.com/terraphim/terraphim-ai/discussions)
- **Documentation**: This directory

## Project Links

- **Repository**: https://github.com/terraphim/terraphim-ai
- **Website**: https://terraphim.io
- **Documentation**: https://docs.terraphim.io

---

**Last Updated**: 2025-10-27
**Version**: 0.2.3
