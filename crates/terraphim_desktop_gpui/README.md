# Terraphim Desktop GPUI

**Status:** 🚧 Early Development / Prototype

GPUI-based desktop application for Terraphim AI, focused on delivering a high-performance, native Rust UI experience.

## Overview

This crate implements the core user journey for Terraphim Desktop using the GPUI framework:

- **Search** with KG-powered autocomplete
- **Markdown Editor** with slash commands and MCP integration
- **Chat Interface** with context management and session history

## Architecture

```
terraphim_desktop_gpui/
├── src/
│   ├── lib.rs                    # Public API exports
│   ├── main.rs                   # App entry point
│   ├── app.rs                    # Main app state and navigation
│   ├── actions.rs                # Global actions and key bindings
│   ├── theme.rs                  # Theme system (light/dark mode)
│   ├── autocomplete.rs           # ✨ Autocomplete engine integration
│   ├── search_service.rs         # ✨ Search service integration
│   ├── models.rs                 # ✨ Data models (chips, results)
│   ├── views/                    # UI components
│   │   ├── search/               # Search interface
│   │   │   ├── mod.rs            # Search view coordinator
│   │   │   ├── input.rs          # Search input
│   │   │   ├── results.rs        # Results list
│   │   │   └── autocomplete.rs   # ✨ UI autocomplete state
│   │   ├── chat/                 # Chat interface
│   │   └── editor/               # Markdown editor
│   └── state/                    # Application state management
│       └── search.rs             # ✨ Enhanced search state
├── tests/                        # ✨ Unit tests
│   ├── autocomplete_tests.rs
│   ├── search_service_tests.rs
│   └── models_tests.rs
└── Cargo.toml
```

✨ = **New in Phase 1, Week 2** (Business Logic Layer)

## Features

### Implemented ✅

- [x] Basic GPUI app structure
- [x] Theme system (light/dark mode support)
- [x] Navigation between Search/Chat/Editor views
- [x] Keyboard shortcuts (cmd-1/2/3 for navigation)
- [x] Search view placeholder
- [x] Direct integration with terraphim_* crates
- [x] **Autocomplete engine** with terraphim_automata integration
- [x] **Search service** with full terraphim_service integration
- [x] **Term chip management** for AND/OR queries
- [x] **Result view models** with highlighting
- [x] **Query parsing** for complex multi-term searches
- [x] **Unit tests** for business logic

### In Progress 🚧

- [ ] GPUI UI components for autocomplete popover
- [ ] Result list rendering with VirtualList
- [ ] Result detail modals with dialogs
- [ ] State synchronization between UI and business logic

### Planned 📋

- [ ] Chat interface with message bubbles
- [ ] Context management panel
- [ ] Session history
- [ ] Markdown editor with slash commands
- [ ] MCP tool integration

## Building

### Prerequisites

**IMPORTANT:** GPUI is still in pre-1.0 development. This crate requires:

1. **GPUI from git** (not yet on crates.io):
   ```toml
   gpui = { git = "https://github.com/zed-industries/zed", branch = "main" }
   ```

2. **Platform requirements**:
   - **macOS**: Xcode command line tools
   - **Linux**: Additional system dependencies (see below)

### Linux Dependencies

```bash
# Ubuntu/Debian
sudo apt install libxcb-shape0-dev libxcb-xfixes0-dev \
  libxkbcommon-dev libssl-dev libfontconfig1-dev

# Fedora
sudo dnf install libxcb-devel libxkbcommon-devel \
  openssl-devel fontconfig-devel
```

### Build Commands

```bash
# Build the GPUI desktop app
cargo build -p terraphim_desktop_gpui

# Run in development mode
cargo run -p terraphim_desktop_gpui

# Build release version
cargo build -p terraphim_desktop_gpui --release
```

## Development

### Key Concepts

**GPUI Architecture:**
- **Views**: Stateful UI components (similar to React components)
- **Models**: Shared state accessible across views
- **Actions**: User-defined commands triggered by keyboard/UI
- **Elements**: Low-level rendering primitives

**State Management:**
- Use `Model<T>` for shared reactive state
- Use `View<T>` for component-local state
- `cx.notify()` triggers re-renders

### Adding New Features

1. **New View**: Create in `src/views/`
2. **New State**: Create Model in `src/state/`
3. **New Action**: Add to `src/actions.rs`
4. **Update Navigation**: Modify `src/app.rs`

### Keyboard Shortcuts

| Shortcut | Action |
|----------|--------|
| `cmd-1` | Navigate to Search |
| `cmd-2` | Navigate to Chat |
| `cmd-3` | Navigate to Editor |
| `cmd-k` | Global Search |
| `cmd-shift-t` | Toggle Theme |
| `cmd-,` | Open Settings |

## Integration with Terraphim

Direct Rust integration with terraphim core crates:

- `terraphim_service` - Search, chat, and indexing
- `terraphim_automata` - Autocomplete engine
- `terraphim_rolegraph` - Knowledge graph
- `terraphim_config` - Configuration management
- `terraphim_persistence` - Data storage

**No IPC overhead** - All calls are direct function calls.

## Known Issues

### GPUI Availability

⚠️ **GPUI is not yet on crates.io**. To build this crate:

1. Update `Cargo.toml` to use git dependency:
   ```toml
   [dependencies]
   gpui = { git = "https://github.com/zed-industries/zed" }
   ```

2. Or wait for GPUI 1.0 release

### Alternative: Use Tauri/Svelte Version

While GPUI implementation is in progress, use the stable Tauri version:
```bash
cd desktop
yarn run tauri dev
```

## Migration Status

This is part of the **GPUI Migration Plan** documented in `docs/gpui-migration-plan.md`.

**Current Phase:** Phase 1, Week 1 - Foundation & Setup

**Progress:**
- ✅ Project structure created
- ✅ Basic app framework implemented
- ✅ Theme system configured
- 🚧 Search interface foundation
- ⏳ Autocomplete integration (pending)

## Performance Targets

- **60+ FPS** scrolling with 1000+ results
- **<50ms** autocomplete latency
- **<100ms** search response time
- **<100MB** idle memory usage

## Contributing

See the main [CONTRIBUTING.md](../../CONTRIBUTING.md) for guidelines.

**GPUI-Specific Notes:**
- Follow Zed editor patterns for GPUI usage
- Keep views small and composable
- Use `Model<T>` for shared state
- Profile rendering performance early

## Resources

- [GPUI Documentation](https://www.gpui.rs/)
- [Zed Editor Source](https://github.com/zed-industries/zed) - Reference implementation
- [Migration Plan](../../docs/gpui-migration-plan.md)
- [Awesome GPUI Projects](https://github.com/zed-industries/awesome-gpui)

## License

Apache-2.0 (same as main Terraphim project)
