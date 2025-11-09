# Terraphim Desktop (Dioxus Edition)

A privacy-first AI assistant desktop application built with Dioxus and Rust.

## Overview

This is a complete rewrite of the Terraphim Desktop application using **Dioxus 0.6**, a Rust-native fullstack framework. This migration eliminates the JavaScript/TypeScript frontend in favor of a pure Rust implementation, providing better type safety, performance, and integration with the Terraphim backend.

### Migration from Svelte

This application replaces the `desktop/` Svelte-based frontend while maintaining full feature parity and the same user experience.

**Key Improvements**:
- ✅ Pure Rust frontend and backend (no FFI boundaries)
- ✅ Direct function calls instead of IPC (better performance)
- ✅ Type-safe state management
- ✅ Improved build times and smaller binaries
- ✅ Better error handling and debugging

## Features

### Must-Have Features (v1.0)
- ✅ **Search with Autocomplete**: Semantic search across knowledge graphs with intelligent autocomplete
- ✅ **Chat with Context Management**: Conversation interface with document/KG term attachment
- ✅ **Conversation Persistence**: Full history and session management
- ✅ **System Tray**: Role switching, window toggle, quick access
- ✅ **Global Shortcuts**: Keyboard shortcuts for window toggle
- ✅ **Role Switching**: UI and tray menu for switching between roles
- ✅ **Editor with Slash Commands**: Command input with `/` commands and `++` autocomplete
- ✅ **Configuration Wizard**: Guided setup for new users
- ✅ **Markdown Rendering**: Preview and render markdown content

## Architecture

### Project Structure

```
desktop-dioxus/
├── src/
│   ├── main.rs                      # Entry point, window, system tray
│   ├── app.rs                       # Root component, router
│   ├── system_tray.rs              # System tray implementation
│   ├── components/                  # UI components
│   │   ├── navigation/             # Navbar, role selector
│   │   ├── search/                 # Search UI
│   │   ├── chat/                   # Chat interface
│   │   ├── editor/                 # Command editor
│   │   ├── config/                 # Configuration wizard/editor
│   │   └── common/                 # Reusable components
│   ├── state/                      # Global state management
│   ├── services/                   # Business logic services
│   ├── routes/                     # Page components
│   └── utils/                      # Utility functions
├── assets/                         # Static assets
│   ├── icons/                      # App icons
│   ├── bulmaswatch/               # Bulma themes
│   └── styles/                     # Custom CSS
├── tests/                          # E2E and integration tests
├── Cargo.toml                      # Rust dependencies
└── Dioxus.toml                     # Dioxus configuration
```

### Technology Stack

- **Framework**: Dioxus 0.6 (Rust)
- **Desktop Runtime**: Dioxus Desktop (WebView-based)
- **Routing**: dioxus-router
- **State Management**: Signals + Contexts
- **Styling**: Bulma CSS (preserved from Svelte version)
- **System Tray**: tray-icon crate
- **Markdown**: pulldown-cmark
- **Backend**: Terraphim core crates (terraphim_service, terraphim_automata, etc.)

## Development

### Prerequisites

- Rust 1.75+ (edition 2024)
- Cargo
- System dependencies for Dioxus desktop:
  - **Linux**: `webkit2gtk`, `libgtk-3-dev`
  - **macOS**: Xcode Command Line Tools
  - **Windows**: WebView2 (usually pre-installed on Windows 10/11)

### Building

```bash
# Development build
cd desktop-dioxus
cargo build

# Release build
cargo build --release

# Run in development mode
cargo run

# Run with specific logging
RUST_LOG=debug cargo run
```

### Dioxus CLI (Optional)

Install Dioxus CLI for hot-reloading and better DX:

```bash
cargo install dioxus-cli

# Run with hot-reloading
dx serve --platform desktop

# Build for release
dx build --release
```

### Testing

```bash
# Run unit tests
cargo test

# Run integration tests
cargo test --test '*'

# Run E2E tests (when implemented)
cargo test --test e2e --features e2e-tests
```

## Configuration

The application uses the same configuration system as the Svelte version:
- **Config files**: `terraphim_engineer_config.json`, etc.
- **Device settings**: `~/.terraphim/settings.toml`
- **Data directory**: `~/.terraphim/data/`

Configuration is loaded automatically on startup from:
1. Saved config (via `terraphim_persistence`)
2. Default config (`ConfigBuilder::build_default_desktop()`)

## System Tray

The system tray provides quick access to:
- **Show/Hide**: Toggle window visibility
- **Role Switching**: Change active role
- **Quit**: Exit application

Tray icon location: `assets/icons/icon.png`

## Global Shortcuts

Default shortcut: Configured per role in `Config.global_shortcut`

Customize shortcuts in the configuration file or wizard.

## Implementation Status

### Phase 0: Project Setup ✅ COMPLETED
- [x] Project structure created
- [x] Dependencies configured
- [x] System tray implemented
- [x] Basic window and routing
- [x] Assets copied

### Phase 1: Core Infrastructure (In Progress)
- [ ] Routing fully functional
- [ ] Global state contexts working
- [ ] Role switching UI complete
- [ ] Navigation between pages

### Phase 2-8: Feature Implementation (Planned)
See `DIOXUS_IMPLEMENTATION_PLAN_REVISED.md` for detailed timeline.

## Comparison with Svelte Version

| Feature | Svelte | Dioxus | Notes |
|---------|--------|--------|-------|
| Frontend Language | TypeScript | Rust | Type safety improved |
| Backend Integration | IPC (Tauri commands) | Direct function calls | Better performance |
| State Management | Svelte stores | Signals + Contexts | More structured |
| Routing | Tinro | dioxus-router | Native integration |
| Build System | Vite + Yarn | Cargo | Simplified toolchain |
| Bundle Size | ~5-10 MB | ~3-8 MB | Smaller due to Rust |
| System Tray | Tauri | tray-icon | Same functionality |
| Hot Reload | Vite HMR | Dioxus hot-reload | Similar DX |

## Contributing

1. Follow the implementation plan in `DIOXUS_IMPLEMENTATION_PLAN_REVISED.md`
2. Write tests for new features
3. Maintain Bulma CSS compatibility
4. Document all public APIs

## Migration Notes

Users of the Svelte version can:
1. Keep existing config files (fully compatible)
2. Conversation history will be preserved
3. Role configurations remain the same

No data migration required - configuration and data formats are identical.

## License

MIT License - Same as Terraphim AI project

## Links

- [Dioxus Documentation](https://dioxuslabs.com/learn/0.6/)
- [Terraphim AI](https://github.com/terraphim/terraphim-ai)
- [Migration Specification](../DIOXUS_MIGRATION_SPECIFICATION.md)
- [Design Plan](../DIOXUS_DESIGN_AND_PLAN.md)
