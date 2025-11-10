# 🎉 Terraphim AI v1.0.0 - Major Release

## ✨ Highlights

First major release of Terraphim AI featuring a complete desktop application with system tray support, auto-updater, and comprehensive AI-powered search capabilities.

## 🚀 What's New

### Desktop Application (Tauri)
- **System Tray Support**: Persistent menu bar icon with quick access
- **Auto-Updater**: Automatic update checks with cryptographic signing
- **Global Shortcuts**: System-wide hotkey (Ctrl+Shift+T)
- **Configuration Wizard**: Easy setup for new users
- **Knowledge Graph Visualization**: Interactive graph view
- **Chat Interface**: AI-powered chat with OpenRouter integration
- **Atomic Server Integration**: Save and manage articles
- **1Password Support**: Secure credential management

### Command Line Tools
- **terraphim_server**: HTTP API server with multiple search algorithms
- **terraphim-tui**: Terminal interface with REPL support

### Core Features
- ✅ 162/162 unit tests passing
- ✅ All dependencies updated to v1.0.0
- ✅ Multiple search algorithms (BM25, TitleScorer, TerraphimGraph)
- ✅ Knowledge graph operations
- ✅ Persistence layer with multiple backends
- ✅ MCP server integration

## 📦 Downloads

### macOS (Apple Silicon)

| Application | Size | Description |
|------------|------|-------------|
| **[TerraphimDesktop_v1.0.0_aarch64.dmg](https://github.com/terraphim/terraphim-ai/releases/download/v1.0.0/TerraphimDesktop_v1.0.0_aarch64.dmg)** | 30MB | Complete desktop app installer |
| **[terraphim_server_macos_aarch64](https://github.com/terraphim/terraphim-ai/releases/download/v1.0.0/terraphim_server_macos_aarch64)** | 15MB | Standalone server binary |
| **[terraphim-tui_macos_aarch64](https://github.com/terraphim/terraphim-ai/releases/download/v1.0.0/terraphim-tui_macos_aarch64)** | 10MB | Terminal interface binary |

### Installation

#### Desktop App
1. Download `TerraphimDesktop_v1.0.0_aarch64.dmg`
2. Double-click to mount
3. Drag Terraphim Desktop to Applications
4. First launch: Right-click → Open (to bypass Gatekeeper)

#### Command Line Tools
```bash
# Server
chmod +x terraphim_server_macos_aarch64
./terraphim_server_macos_aarch64 --help

# TUI
chmod +x terraphim-tui_macos_aarch64
./terraphim-tui_macos_aarch64 --help
```

## 🔧 System Requirements

- **macOS**: 10.15+ (Catalina or later)
- **Architecture**: ARM64 (Apple Silicon M1/M2/M3)
- **Memory**: 4GB RAM minimum
- **Storage**: 500MB free space

## 📝 Known Issues

- First launch requires Gatekeeper bypass (right-click → Open)
- Apps are ad-hoc signed (not notarized)
- Windows and Linux builds coming in future releases

## 🙏 Contributors

Thanks to all contributors who made this release possible!

## 📄 License

Apache-2.0

---

**Full Changelog**: https://github.com/terraphim/terraphim-ai/compare/v0.2.3...v1.0.0
