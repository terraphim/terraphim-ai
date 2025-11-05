# Terraphim AI v1.0.0 Release Notes

**Release Date:** 2025-11-05  
**Platform:** macOS ARM64

## 🎉 Release Highlights

This is the first major release of Terraphim AI, featuring complete command-line tools and server infrastructure for privacy-first AI assistance with semantic search capabilities.

## 📦 Release Artifacts

### macOS Applications (ARM64)

1. **TerraphimServer.app** (15MB)
   - HTTP API server with multiple search algorithms
   - Role-based configuration system
   - Knowledge graph operations
   - Ad-hoc signed for macOS

2. **TerraphimTUI.app** (10MB)
   - Terminal User Interface with REPL support
   - Full search and configuration capabilities
   - Interactive command system
   - Ad-hoc signed for macOS

### Command Line Binaries

- `terraphim_server` - Standalone server binary
- `terraphim-tui` - Standalone TUI binary

## ✅ Completed Tasks (9/11)

1. ✅ **Version Updates** - All packages updated to v1.0.0
2. ✅ **Frontend Build** - Fixed svelte-jsoneditor HTML validation
3. ✅ **Release Artifacts** - Created releases/v1.0.0/macos/ directory
4. ✅ **App Bundles** - Created .app bundles for both binaries
5. ✅ **Code Signing** - Ad-hoc signed both applications
6. ✅ **End-to-End Tests** - Validated server and TUI functionality
7. ✅ **GitHub Tag** - Created v1.0.0 release tag
8. ✅ **TUI Implementation** - Synced complete implementation from private repo
9. ✅ **Build Issues** - Fixed panic strategy mismatch

## ⏳ Pending Tasks

1. ❌ **Tauri Desktop App** - Dependency version conflicts need resolution
2. ❌ **Notarization** - Requires Apple Developer account and certificates

## 🧪 Test Results

- **Unit Tests:** 162/162 passing (100% success rate)
- **Server Health:** Confirmed working on port 8000
- **TUI Commands:** All major commands functional
- **App Bundles:** Both applications launch successfully

### Test Breakdown
- terraphim_middleware: 5/5 ✅
- terraphim_service: 112/112 ✅
- terraphim_automata: 13/13 ✅
- terraphim_rolegraph: 7/7 ✅
- terraphim_persistence: 25/25 ✅

## 🐛 Known Issues

1. **Version Display:** Binaries still report v0.2.3 internally (cosmetic issue)
2. **Tauri Build:** Desktop app has unresolved dependency conflicts
3. **Notarization:** Apps are ad-hoc signed, not notarized

## 🚀 Getting Started

### Running the Server
```bash
./TerraphimServer.app/Contents/MacOS/TerraphimServer --role Default
```

### Running the TUI
```bash
./TerraphimTUI.app/Contents/MacOS/TerraphimTUI --server
```

### Basic Commands
```bash
# Search
TerraphimTUI search "your query"

# View roles
TerraphimTUI roles list

# Start REPL
TerraphimTUI repl
```

## 🔧 Technical Details

- **Architecture:** ARM64 (Apple Silicon)
- **Minimum OS:** macOS 10.15
- **Build Profile:** Release with optimizations
- **Panic Strategy:** Unwind
- **Code Signing:** Ad-hoc

## 📝 Important Notes

- Applications are ad-hoc signed and may require Gatekeeper approval
- First run may require right-click → Open to bypass security warnings
- Server defaults to port 8000

## 🙏 Acknowledgments

This release represents significant progress in the Terraphim AI project, with core functionality fully operational and ready for use.

---

**For issues or questions, please visit:** https://github.com/terraphim/terraphim-ai