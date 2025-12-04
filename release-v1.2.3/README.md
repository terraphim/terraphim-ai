# Terraphim AI v1.2.3 - Complete Binary Release

## Included Binaries

| Binary | Version | Size | Description |
|---------|---------|-------|-------------|
| terraphim-agent | 1.2.3 | 14.3MB | Interactive TUI interface with semantic search |
| terraphim-cli | 1.0.0 | 13.0MB | Command-line interface with JSON output for automation |
| terraphim_mcp_server | 1.0.0 | 16.0MB | MCP server with configurable profiles (desktop/server) |
| terraphim-repl | 1.0.0 | 12.9MB | Interactive REPL with offline mode capabilities |
| terraphim-build-args | 0.1.0 | 3.7MB | Build argument management tool for CI/CD |
| terraphim_server | 1.0.0 | 22.4MB | Core HTTP API server with role-based config |
| terraphim-ai-desktop | 1.0.0 | 27.4MB | Cross-platform desktop application (Tauri) |

## Quick Start

### 1. Interactive TUI (Recommended for users)
\`\`\`bash
./terraphim-agent search "your query"
\`\`\`

### 2. CLI Automation (Recommended for developers)
\`\`\`bash
./terraphim-cli search "your query" --format json
\`\`\`

### 3. Desktop Application
\`\`\`bash
./terraphim-ai-desktop
\`\`\`

### 4. Server Mode
\`\`\`bash
./terraphim_server --role TerraphimEngineer
\`\`\`

### 5. MCP Server
\`\`\`bash
./terraphim_mcp_server --profile desktop
\`\`\`

## Features

- ✅ Semantic knowledge graph search
- ✅ Multiple role configurations (Default, RustEngineer, TerraphimEngineer)
- ✅ JSON/CLI automation support
- ✅ MCP protocol integration
- ✅ Cross-platform compatibility (Linux, macOS, Windows)
- ✅ Offline operation capabilities
- ✅ Real-time search and indexing
- ✅ Configurable thesaurus integration

## System Requirements

- **OS**: Linux (x86_64), macOS (x86_64, ARM64), Windows (x86_64)
- **Memory**: Minimum 512MB RAM, Recommended 2GB+
- **Storage**: 100MB free space
- **Network**: Optional (for online features)

## Testing Status

All binaries have been comprehensively tested:
- ✅ Help commands functional
- ✅ Version reporting working
- ✅ Core features validated
- ✅ Build integrity confirmed
- ✅ Error handling verified
- ✅ Configuration management tested

## Support

- Documentation: https://terraphim.ai
- Issues: https://github.com/terraphim/terraphim-ai/issues
- Discussions: https://github.com/terraphim/terraphim-ai/discussions

---
Built with ❤️ by the Terraphim AI team
