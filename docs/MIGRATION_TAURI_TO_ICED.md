# Tauri to Iced Migration Guide

This document describes the migration of the Terraphim desktop application from Tauri (web-based) to Iced (native Rust GUI).

## Overview

**Old Stack**: Tauri + Svelte + TypeScript
**New Stack**: Iced (Pure Rust)

## Motivation

1. **Performance**: Eliminate webview overhead, reduce memory usage
2. **Binary Size**: Smaller executables without bundling webview
3. **Simplicity**: Single language (Rust) for both frontend and backend
4. **Native Feel**: True native widgets instead of web rendering
5. **Future-Proof**: Better WASM support, easier to maintain

## Architecture Comparison

### Tauri Architecture

```
┌─────────────────────────────────────┐
│   Svelte/TypeScript Frontend        │
│   (HTML/CSS/JavaScript)             │
└─────────────┬───────────────────────┘
              │ IPC Commands
              │ (invoke/emit)
              v
┌─────────────────────────────────────┐
│   Rust Backend (Tauri)              │
│   - Config management               │
│   - Search service                  │
│   - System tray                     │
└─────────────────────────────────────┘
```

### Iced Architecture

```
┌─────────────────────────────────────┐
│   Iced Application (Pure Rust)      │
│   - Model (State)                   │
│   - Update (Event handling)         │
│   - View (Rendering)                │
│                                     │
│   Directly uses backend:            │
│   - Config management               │
│   - Search service                  │
│   - System tray (TBD)              │
└─────────────────────────────────────┘
```

## Key Differences

| Aspect | Tauri | Iced |
|--------|-------|------|
| **UI Language** | Svelte/TypeScript | Rust |
| **Rendering** | WebView (HTML/CSS) | Native Widgets |
| **State Management** | Svelte stores | Elm Architecture |
| **Async** | Promises/async-await | Tokio Tasks |
| **Styling** | CSS (Bulma) | Iced styling |
| **Hot Reload** | Yes (Vite) | No |
| **IPC** | Tauri commands | Direct function calls |
| **Bundle Size** | ~80-100 MB | ~20-30 MB |

## File Mapping

### Frontend Files

| Tauri (Svelte) | Iced (Rust) | Notes |
|----------------|-------------|-------|
| `desktop/src/App.svelte` | `terraphim_desktop_iced/src/main.rs::TerraphimApp` | Main app structure |
| `desktop/src/lib/Search/Search.svelte` | `view_search()` | Search interface |
| `desktop/src/lib/Chat/Chat.svelte` | `view_chat()` | Chat interface |
| `desktop/src/lib/RoleGraphVisualization.svelte` | `view_graph()` | Graph view (placeholder) |
| `desktop/src/lib/ConfigWizard.svelte` | `view_config()` | Config UI (placeholder) |
| `desktop/src/lib/stores.ts` | `TerraphimApp` struct fields | State management |

### Backend Files

| Location | Status |
|----------|--------|
| `desktop/src-tauri/src/main.rs` | Replaced with Iced app |
| `desktop/src-tauri/src/cmd.rs` | Logic moved to `TerraphimApp::update()` |
| `crates/*` | **Reused** - No changes needed |

## Feature Mapping

### Completed Features

| Feature | Tauri | Iced | Status |
|---------|-------|------|--------|
| Main window | ✅ | ✅ | Complete |
| Navigation tabs | ✅ | ✅ | Complete |
| Search view | ✅ | ✅ | Basic version |
| Chat interface | ✅ | ✅ | Basic version |
| Theme switching | ✅ | ✅ | Complete |
| Config loading | ✅ | ✅ | Complete |

### Pending Features

| Feature | Tauri | Iced | Blocker |
|---------|-------|------|---------|
| System tray | ✅ | ⏸️ | Dependency conflict with Tauri |
| Global shortcuts | ✅ | ⏸️ | Dependency conflict with Tauri |
| Graph visualization | ✅ | 🚧 | Needs custom rendering |
| Config wizard | ✅ | 🚧 | UI implementation |
| JSON editor | ✅ | 🚧 | UI implementation |
| Autocomplete | ✅ | 🚧 | Widget implementation |
| Context management | ✅ | 🚧 | Integration needed |

## Code Patterns

### State Management

**Tauri (Svelte)**:
```typescript
// Reactive stores
import { writable } from 'svelte/store';
export const searchInput = writable('');
export const searchResults = writable([]);

// Usage in component
$: console.log($searchInput);
```

**Iced (Rust)**:
```rust
// Immutable state in struct
struct TerraphimApp {
    search_input: String,
    search_results: Vec<Document>,
}

// Updated via messages
enum Message {
    SearchInputChanged(String),
    SearchCompleted(Vec<Document>),
}
```

### Event Handling

**Tauri (Svelte)**:
```typescript
async function handleSearch() {
    const results = await invoke('search', {
        searchQuery: {
            query: $input,
            role: $role,
        }
    });
    searchResults.set(results);
}
```

**Iced (Rust)**:
```rust
fn update(&mut self, message: Message) -> Task<Message> {
    match message {
        Message::SearchSubmitted => {
            Task::perform(
                Self::perform_search(self.search_input.clone()),
                Message::SearchCompleted
            )
        }
        Message::SearchCompleted(results) => {
            self.search_results = results;
            Task::none()
        }
    }
}
```

### UI Rendering

**Tauri (Svelte)**:
```svelte
<div class="search-container">
    <input
        type="text"
        bind:value={$input}
        on:submit={handleSearch}
        placeholder="Search..."
    />
    {#each results as result}
        <div class="result-item">
            <h3>{result.title}</h3>
            <p>{result.description}</p>
        </div>
    {/each}
</div>
```

**Iced (Rust)**:
```rust
fn view_search(&self) -> Element<Message> {
    let search_input = text_input("Search...", &self.search_input)
        .on_input(Message::SearchInputChanged)
        .on_submit(Message::SearchSubmitted);

    let mut results_col = column![];
    for result in &self.search_results {
        results_col = results_col.push(
            column![
                text(&result.title).size(18),
                text(&result.description).size(14),
            ]
        );
    }

    column![search_input, results_col].into()
}
```

## Migration Steps

### Phase 1: Basic Application ✅

- [x] Create Iced project structure
- [x] Set up Elm Architecture (Model-Update-View)
- [x] Implement basic window and navigation
- [x] Integrate with existing backend crates

### Phase 2: Core Features ✅

- [x] Port search view
- [x] Port chat view
- [x] Add theme switching
- [x] Implement configuration loading

### Phase 3: Advanced Features 🚧

- [ ] Complete configuration UI
- [ ] Implement graph visualization
- [ ] Add autocomplete widget
- [ ] Port context management
- [ ] Implement keyboard shortcuts

### Phase 4: Platform Features ⏸️

- [ ] System tray integration (after removing Tauri)
- [ ] Global shortcuts (after removing Tauri)
- [ ] Window management
- [ ] State persistence

### Phase 5: Polish & Optimization 📅

- [ ] Custom styling
- [ ] Animations
- [ ] Accessibility
- [ ] Performance optimization
- [ ] User testing

## Dependency Conflict Resolution

**Current Issue**: Both Tauri and Iced depend on GTK (via different versions), causing a workspace-level conflict.

**Solution Options**:

1. **Temporary**: Disable system tray in Iced until Tauri is removed
   - Status: ✅ Implemented
   - Pros: Quick, allows parallel development
   - Cons: Missing features

2. **Parallel Workspaces**: Create separate workspace for Iced
   - Status: ⏸️ Not chosen
   - Pros: No conflicts
   - Cons: More complex build

3. **Complete Migration**: Remove Tauri entirely
   - Status: 📅 Planned
   - Pros: Clean solution
   - Cons: Requires feature parity first

## Testing Strategy

### Unit Tests
```bash
# Test Iced application logic
cargo test -p terraphim-desktop-iced
```

### Integration Tests
```bash
# Test with backend
cargo test -p terraphim-desktop-iced --features full-db
```

### Manual Testing
```bash
# Run in development
cargo run -p terraphim-desktop-iced

# Test with different themes
RUST_LOG=debug cargo run -p terraphim-desktop-iced
```

## Performance Benchmarks (Estimated)

| Metric | Tauri | Iced | Improvement |
|--------|-------|------|-------------|
| Cold Start | ~500ms | ~100ms | 5x faster |
| Memory (Idle) | ~150 MB | ~50 MB | 3x less |
| Binary Size | ~90 MB | ~25 MB | 3.6x smaller |
| Search Latency | ~50ms | ~30ms | 1.7x faster |

*Note: Iced values are estimates pending actual measurements*

## Known Issues

1. **GTK Dependency Conflict**: Cannot build both apps in same workspace with system tray
2. **Placeholder Views**: Graph and Config need implementation
3. **Missing Widgets**: Custom autocomplete widget needed
4. **Styling**: Need to replicate Bulma CSS look in Iced

## Next Steps

1. **Remove Tauri dependency** once feature parity achieved
2. **Implement missing widgets** for autocomplete and complex UIs
3. **Add system tray** after dependency conflict resolved
4. **Performance testing** and optimization
5. **User feedback** and iterative improvements

## Resources

- [Iced Documentation](https://docs.rs/iced/)
- [Iced Examples](https://github.com/iced-rs/iced/tree/master/examples)
- [Elm Architecture](https://guide.elm-lang.org/architecture/)
- [Current Implementation](../terraphim_desktop_iced/)

## Contributing

When contributing to the migration:

1. Follow Rust best practices and idioms
2. Use the Elm Architecture pattern consistently
3. Reuse existing backend crates (don't duplicate logic)
4. Add tests for new features
5. Update this migration guide for significant changes

---

**Status**: 🚧 In Progress
**Last Updated**: 2025-11-24
**Maintainer**: Terraphim Team
