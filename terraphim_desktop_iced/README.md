# Terraphim Desktop - Iced Edition

This is the new native Rust desktop application for Terraphim using the [Iced](https://iced.rs/) GUI framework.

## Why Iced?

The migration from Tauri to Iced provides several benefits:

- **Pure Rust**: No web technologies (HTML/CSS/JavaScript), everything is Rust
- **Better Performance**: Native UI rendering without webview overhead
- **Smaller Binary**: No need to bundle a webview engine
- **Better Integration**: Direct access to Rust backend without IPC layer
- **Consistent UI**: Native look and feel across all platforms

## Architecture

### Application Structure

The Iced application follows the Elm Architecture (Model-Update-View):

- **Model (State)**: `TerraphimApp` struct holds all application state
- **Messages**: `Message` enum defines all possible events
- **Update**: Pure function that handles messages and updates state
- **View**: Pure function that renders UI based on current state

### Current Features

✅ **Implemented:**
- Basic window and event loop
- Navigation tabs (Search, Chat, Graph, Config)
- Search view with input and results display
- Chat view with message history and input
- Theme switching (Light/Dark)
- Configuration loading
- Integration with existing Terraphim backend crates

🚧 **Pending:**
- Configuration management UI (wizard and JSON editor)
- Knowledge graph visualization
- System tray integration (blocked by dependency conflict with Tauri)
- Global keyboard shortcuts
- Advanced search features (autocomplete, filters)
- Context management for chat
- File operations and persistence

## Building

### Prerequisites

- Rust 1.87 or later
- Platform-specific dependencies:
  - **Linux**: X11 or Wayland development libraries
  - **macOS**: Xcode Command Line Tools
  - **Windows**: Visual Studio C++ Build Tools

### Build Commands

```bash
# Debug build
cargo build -p terraphim-desktop-iced

# Release build
cargo build -p terraphim-desktop-iced --release

# Run development version
cargo run -p terraphim-desktop-iced

# Run with specific features
cargo build -p terraphim-desktop-iced --features openrouter

# With full database support
cargo build -p terraphim-desktop-iced --features full-db
```

## Running

```bash
# Development mode
cargo run -p terraphim-desktop-iced

# Release mode
./target/release/terraphim-desktop-iced
```

## Features

### Default Features

Currently, default features are minimal to avoid dependency conflicts with the existing Tauri desktop app in the same workspace.

### Optional Features

- `openrouter`: Enable OpenRouter AI integration
- `sqlite`: SQLite database backend
- `rocksdb`: RocksDB database backend
- `redis`: Redis database backend
- `full-db`: Enable all database backends

### Disabled Features (Temporary)

- `system-tray`: System tray integration (conflicts with Tauri's webkit2gtk)
- `global-shortcuts`: Global keyboard shortcuts (conflicts with Tauri's webkit2gtk)

**Note**: These features will be re-enabled once the Tauri desktop app is fully removed from the workspace.

## Code Structure

```
terraphim_desktop_iced/
├── Cargo.toml          # Package configuration
├── README.md           # This file
└── src/
    └── main.rs         # Main application code
```

### Main Components

```rust
// Application State
struct TerraphimApp {
    current_view: View,           // Active tab
    config_state: ConfigState,    // Configuration
    search_input: String,         // Search query
    search_results: Vec<Document>,// Search results
    chat_messages: Vec<ChatMessage>, // Chat history
    theme: Theme,                 // UI theme
    // ...
}

// Events
enum Message {
    SwitchView(View),           // Navigate to different view
    SearchSubmitted,            // Perform search
    SearchCompleted(Result<...>), // Search results received
    ChatMessageSent,            // Send chat message
    // ...
}

// Views
impl TerraphimApp {
    fn view_search(&self) -> Element<Message>  // Search interface
    fn view_chat(&self) -> Element<Message>    // Chat interface
    fn view_graph(&self) -> Element<Message>   // Graph visualization
    fn view_config(&self) -> Element<Message>  // Configuration UI
}
```

## Integration with Backend

The Iced application reuses all existing Terraphim backend crates:

- `terraphim_config`: Configuration management
- `terraphim_service`: Search and AI services
- `terraphim_middleware`: Haystack indexing
- `terraphim_rolegraph`: Knowledge graph
- `terraphim_automata`: Text matching and autocomplete
- `terraphim_types`: Shared type definitions
- `terraphim_persistence`: Storage layer

This ensures feature parity with the Tauri application while using native Rust UI.

## Migration Status

### Completed ✅
- [x] Basic Iced application setup
- [x] Navigation and routing
- [x] **Search view with autocomplete** (FST-based, fuzzy fallback)
- [x] **Chat view with context management** (add/remove context, context panel)
- [x] **KG search modal** (search and add KG terms to context)
- [x] Theme switching (Light/Dark)
- [x] Configuration loading
- [x] Backend integration (automata, service, config)
- [x] Real-time autocomplete suggestions
- [x] Split-pane chat layout
- [x] Context item management UI

### Completed (Phase 2) ✅✅
- [x] **LLM backend integration** (OpenRouter with context-aware prompts)
- [x] **Conversation persistence** (ConversationService with OpenDAL)
- [x] **Full context management** (add/remove/persist context items)

### In Progress 🚧
- [ ] Configuration UI (wizard and JSON editor)

### Planned 📅
- [ ] System tray (after Tauri removal)
- [ ] Global shortcuts (after Tauri removal)
- [ ] Graph visualization
- [ ] Window management
- [ ] Role switching UI
- [ ] Keyboard shortcuts
- [ ] Accessibility features

## Development

### Adding New Views

1. Add a new variant to the `View` enum
2. Implement a `view_*` method in `TerraphimApp`
3. Add navigation button in `view_header`
4. Add a case in the main `view` method

### Adding New Features

1. Add new messages to the `Message` enum
2. Handle messages in the `update` method
3. Update UI in the relevant `view_*` method
4. Add any async operations as `Task::perform` calls

### Testing

```bash
# Run with logging
RUST_LOG=debug cargo run -p terraphim-desktop-iced

# Check for issues
cargo clippy -p terraphim-desktop-iced

# Format code
cargo fmt -p terraphim-desktop-iced
```

## Comparison with Tauri Desktop

| Feature | Tauri | Iced |
|---------|-------|------|
| UI Technology | Svelte + HTML/CSS | Pure Rust |
| Binary Size | ~80-100 MB | ~20-30 MB (estimated) |
| Startup Time | ~500ms | ~100ms (estimated) |
| Memory Usage | ~150-200 MB | ~50-100 MB (estimated) |
| Platform Support | Windows, macOS, Linux | Windows, macOS, Linux, Web (WASM) |
| Development Speed | Faster (web tech) | Slower (Rust only) |
| Hot Reload | Yes (with Vite) | No (compile required) |
| Native Feel | Webview | Native widgets |

## Known Issues

1. **Dependency Conflict**: Cannot enable system tray and global shortcuts while Tauri desktop app is in the same workspace (both depend on different versions of gtk)
2. **Placeholder Views**: Graph and Config views are placeholders
3. **Limited Chat**: Chat implementation is a mock, needs full backend integration

## Future Improvements

- [ ] Complete configuration UI
- [ ] Implement graph visualization with `iced_aw` or custom rendering
- [ ] Add system tray and global shortcuts once dependency conflict is resolved
- [ ] Implement drag-and-drop for files
- [ ] Add keyboard navigation
- [ ] Improve accessibility (screen readers, high contrast themes)
- [ ] Add animations and transitions
- [ ] Implement custom styling system matching Terraphim brand
- [ ] Add WASM support for web deployment

## Contributing

When contributing to the Iced desktop app:

1. Follow the existing Elm Architecture pattern
2. Keep state immutable
3. Use pure functions for rendering
4. Handle all async operations with `Task`
5. Add appropriate logging
6. Update this README for significant changes

## Resources

- [Iced Documentation](https://docs.rs/iced/)
- [Iced Book](https://book.iced.rs/)
- [Iced Examples](https://github.com/iced-rs/iced/tree/master/examples)
- [Terraphim Documentation](https://terraphim.ai/docs)
