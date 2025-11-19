# Terraphim AI v1.0.0 Release Notes

## 🎉 First Major Release

This is the first major release of Terraphim AI, featuring a complete desktop application with auto-update capabilities, RAG workflows, and LLM integration.

## ✨ Key Features

### Desktop Application
- **Tauri Desktop**: Native cross-platform desktop application
- **Auto-Update**: Signed updates with automatic installation
- **System Tray**: Background operation with system tray integration

### AI & Search
- **RAG Workflow**: Retrieval-Augmented Generation for contextual AI responses
- **LLM Integration**: Support for OpenRouter and Ollama
- **Semantic Search**: Knowledge graph-based search across multiple sources
- **MCP Server**: Model Context Protocol for AI tool integration

### Infrastructure
- **Docker Compose**: Production-ready deployment with UID/GID management
- **TUI Interface**: Terminal user interface for server-less operation
- **Multiple Haystacks**: Search across local files, Notion, email, and more

## 📦 Installation

### Desktop Application (Recommended)
Download the appropriate installer for your platform from the [releases page](https://github.com/terraphim/terraphim-ai/releases/tag/v1.0.0):

- **macOS**: `Terraphim.Desktop_1.0.0_x64.dmg`
- **Windows**: `Terraphim.Desktop_1.0.0_x64_en-US.msi`
- **Linux**: `terraphim-ai-desktop_1.0.0_amd64.deb` or `.AppImage`

### Docker
```bash
docker pull ghcr.io/terraphim/terraphim-ai:v1.0.0
docker-compose up -d
```

## 🔐 Auto-Update Configuration

This release includes signed updates. The desktop app will automatically check for and download updates.

**For maintainers**: To sign future releases, set the `TAURI_PRIVATE_KEY` GitHub secret with the value from `.reports/tauri_keys.txt` (private key).

## 🔧 Configuration

### Environment Variables
```bash
# For signed releases (maintainers only)
export TAURI_PRIVATE_KEY="your_private_key"
export TAURI_KEY_PASSWORD="optional_password"
```

### Docker Environment
The `.env` file is auto-generated with your UID/GID for proper file permissions.

## 📚 Documentation

- [Quick Start Guide](../README.md)
- [Configuration Guide](../CLAUDE.md)
- [Release Management](RELEASE_ACTION_PLAN.md)
- [Component Testing](test_components.sh)

## 🐛 Known Issues

- PR #277 (Code Assistant) - Pending CI fixes
- PR #268 (TUI/REPL build) - Needs investigation
- Server build requires frontend dependencies preinstalled

## 🔄 Upgrading

### From v0.x
This is a breaking change release. Please backup your configuration before upgrading.

### Auto-Update
Desktop app users will be prompted automatically when v1.0.1+ is released.

## 🤝 Contributing

See [CONTRIBUTING.md](../CONTRIBUTING.md) for development guidelines.

## 📜 License

Apache-2.0 - See [LICENSE](../LICENSE-Apache-2.0) for details.

## 🙏 Acknowledgments

Thanks to all contributors who made this release possible!

---

**Next Steps:**
1. Download and install the desktop app
2. Configure your haystacks
3. Start searching and chatting with your knowledge base
4. Join our community discussions

**Support**: Open an issue on GitHub for bugs or feature requests.
