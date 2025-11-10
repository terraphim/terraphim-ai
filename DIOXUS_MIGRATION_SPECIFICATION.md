# Terraphim Desktop: Dioxus Migration Specification

**Version:** 1.0
**Date:** 2025-11-09
**Status:** Design Phase

---

## Table of Contents

1. [Executive Summary](#executive-summary)
2. [Current Architecture Analysis](#current-architecture-analysis)
3. [Dioxus Target Architecture](#dioxus-target-architecture)
4. [Component Mapping](#component-mapping)
5. [Critical Features Requirements](#critical-features-requirements)
6. [Editor Strategy](#editor-strategy)
7. [Migration Phases](#migration-phases)
8. [Open Questions](#open-questions)

---

## Executive Summary

This document specifies the migration of **Terraphim Desktop** from a Svelte/Tauri stack to **Dioxus 0.7**, a Rust-native fullstack framework. The migration aims to:

1. **Unify the tech stack**: Move from JavaScript/TypeScript (Svelte) + Rust (Tauri) to pure Rust (Dioxus + Tauri backend)
2. **Maintain all existing functionality**: System tray, global shortcuts, search, chat, knowledge graph visualization, editor with autocomplete
3. **Improve type safety**: Eliminate FFI boundaries between frontend and backend
4. **Preserve user experience**: Keep the same UI/UX, themes, and workflows

### Current Stack
- **Frontend**: Svelte 5 + TypeScript + Vite
- **Backend**: Rust + Tauri 1.5
- **Styling**: Bulma CSS + Bulmaswatch themes
- **Editor**: TipTap (via @paralect/novel-svelte) with custom autocomplete
- **State Management**: Svelte stores
- **Routing**: Tinro router

### Target Stack
- **Framework**: Dioxus 0.7 (Rust)
- **Backend**: Existing Rust Tauri commands (reusable)
- **Styling**: CSS (Bulma preserved) or Dioxus native styling
- **Editor**: Custom Dioxus component or WASM integration
- **State Management**: Dioxus signals/contexts
- **Routing**: Dioxus Router

---

## Current Architecture Analysis

### Application Structure

```
terraphim-desktop/
├── src/                          # Svelte frontend
│   ├── App.svelte               # Main app shell with routing
│   ├── lib/
│   │   ├── Chat/                # Chat feature components
│   │   │   ├── Chat.svelte
│   │   │   ├── SessionList.svelte
│   │   │   └── ContextEditModal.svelte
│   │   ├── Search/              # Search feature components
│   │   │   ├── Search.svelte
│   │   │   ├── ResultItem.svelte
│   │   │   ├── KGSearchModal.svelte
│   │   │   └── AtomicSaveModal.svelte
│   │   ├── Editor/              # Editor components
│   │   │   ├── NovelWrapper.svelte
│   │   │   └── TerraphimSuggestion.ts
│   │   ├── RoleGraphVisualization.svelte
│   │   ├── ConfigWizard.svelte
│   │   ├── ConfigJsonEditor.svelte
│   │   ├── ThemeSwitcher.svelte
│   │   ├── stores.ts            # Global state
│   │   └── services/
│   │       ├── novelAutocompleteService.ts
│   │       └── chatService.ts
│   └── main.ts
├── src-tauri/                   # Rust backend (REUSABLE)
│   ├── src/
│   │   ├── main.rs             # App setup, system tray, shortcuts
│   │   ├── lib.rs              # Public exports
│   │   ├── cmd.rs              # Tauri commands (40+ commands)
│   │   └── bindings.rs         # TypeScript type generation
│   └── tauri.conf.json
└── package.json
```

### Key Features Inventory

#### 1. **System Tray** ✅ CRITICAL
- **Location**: `src-tauri/src/main.rs:109-131`
- **Features**:
  - Show/Hide toggle
  - Dynamic role switching menu
  - Quit option
  - Selected role indicator (checkmark)
- **Event Handling**: `SystemTrayEvent::MenuItemClick`
- **Menu Updates**: Dynamic rebuilding when role changes

#### 2. **Global Shortcuts** ✅ CRITICAL
- **Location**: `src-tauri/src/main.rs:467-531`
- **Default**: Configured per role in `Config.global_shortcut`
- **Action**: Toggle window visibility
- **Platform**: Cross-platform (Windows, macOS, Linux)

#### 3. **Main Window & Routing** ✅ CRITICAL
- **Routes**:
  - `/` - Search (default)
  - `/chat` - Chat interface
  - `/graph` - Knowledge graph visualization
  - `/config/wizard` - Configuration wizard
  - `/config/json` - JSON editor
- **Navigation**: Tab-based navigation with active state
- **Logo**: Clickable back button functionality

#### 4. **Search Feature** ✅ CRITICAL
- **Location**: `src/lib/Search/Search.svelte`
- **Features**:
  - Real-time autocomplete with operator support (AND/OR)
  - Term chips visualization (KG terms highlighted)
  - Result display with AI summarization
  - SSE streaming for summary updates (web) / polling (Tauri)
  - Multi-haystack search
  - Knowledge graph term suggestions
- **State Persistence**: localStorage per role
- **Backend Command**: `cmd::search`

#### 5. **Chat Interface** ✅ CRITICAL
- **Location**: `src/lib/Chat/Chat.svelte`
- **Features**:
  - Session management (in-memory + persistent)
  - Context attachment (documents, KG terms)
  - Context editing modal
  - Session list panel
  - Conversation export/import
  - Statistics tracking
- **Backend Commands**: `cmd::chat`, `cmd::create_conversation`, etc.

#### 6. **Editor with Autocomplete** ✅ CRITICAL
- **Location**: `src/lib/Editor/NovelWrapper.svelte`
- **Current Implementation**:
  - TipTap-based editor (via @paralect/novel-svelte)
  - Custom TerraphimSuggestion extension
  - Trigger: `++` followed by term
  - Backend: Tauri commands or MCP server
  - Features: Markdown support, snippets, debouncing
- **Requirements**:
  - Slash command support (`/command`)
  - Autocomplete from knowledge graph
  - Markdown rendering
  - Role-aware suggestions

#### 7. **Knowledge Graph Visualization** ✅ HIGH
- **Location**: `src/lib/RoleGraphVisualization.svelte`
- **Technology**: D3.js force-directed graph
- **Features**:
  - Node/edge visualization
  - Interactive exploration
  - Filtering by role
- **Data Source**: `cmd::get_rolegraph`

#### 8. **Configuration Management** ✅ HIGH
- **Wizard**: `src/lib/ConfigWizard.svelte` - Step-by-step setup
- **JSON Editor**: `src/lib/ConfigJsonEditor.svelte` - Advanced editing
- **Theme Switcher**: `src/lib/ThemeSwitcher.svelte` - Bulmaswatch themes
- **Backend Commands**: `cmd::get_config`, `cmd::update_config`

#### 9. **Theme System** ✅ MEDIUM
- **Location**: `src/lib/ThemeSwitcher.svelte`, `src/lib/themeManager.ts`
- **Themes**: 20+ Bulmaswatch themes
- **Storage**: localStorage persistence
- **Dynamic**: CSS file loading at runtime

#### 10. **Startup Screen** ✅ MEDIUM
- **Location**: `src/lib/StartupScreen.svelte`
- **Condition**: Shows when `device_settings.initialized === false`
- **Purpose**: Initial configuration guide

### Tauri Commands (40+ commands)

All commands in `src-tauri/src/cmd.rs` are **fully reusable** with Dioxus:

**Core Search & Config**:
- `search` - Execute search queries
- `get_config` - Fetch current configuration
- `update_config` - Update configuration
- `get_config_schema` - Get JSON schema
- `select_role` - Switch active role
- `publish_thesaurus` - Publish KG thesaurus

**Document Management**:
- `create_document` - Create new document
- `get_document` - Fetch document by ID
- `get_autocomplete_suggestions` - Get autocomplete suggestions

**Knowledge Graph**:
- `get_rolegraph` - Get role-specific graph
- `find_documents_for_kg_term` - Find docs for KG term
- `search_kg_terms` - Search KG terms
- `add_kg_term_context` - Add KG term to context
- `add_kg_index_context` - Add KG index to context

**Chat & Conversations**:
- `chat` - Send chat message
- `create_conversation` - Create new conversation
- `list_conversations` - List all conversations
- `get_conversation` - Get conversation by ID
- `add_message_to_conversation` - Add message
- `add_context_to_conversation` - Add context
- `add_search_context_to_conversation` - Add search results
- `delete_context` - Remove context item
- `update_context` - Update context item

**Persistent Conversations**:
- `list_persistent_conversations` - List saved conversations
- `get_persistent_conversation` - Get saved conversation
- `create_persistent_conversation` - Create persistent conversation
- `update_persistent_conversation` - Update saved conversation
- `delete_persistent_conversation` - Delete saved conversation
- `search_persistent_conversations` - Search conversations
- `export_persistent_conversation` - Export to file
- `import_persistent_conversation` - Import from file
- `get_conversation_statistics` - Get usage stats

**1Password Integration**:
- `onepassword_status` - Check 1Password CLI status
- `onepassword_resolve_secret` - Resolve secret reference
- `onepassword_process_config` - Process config with secrets
- `onepassword_load_settings` - Load settings with secrets

**Device Settings**:
- `save_initial_settings` - Save initial device settings
- `close_splashscreen` - Close startup screen

### State Management

**Current (Svelte Stores)**:
```typescript
// Global stores
- configStore: Config          // Full configuration
- role: string                 // Selected role name
- roles: Role[]               // Available roles
- theme: string               // Current theme
- is_tauri: boolean           // Runtime environment
- serverUrl: string           // API base URL
- input: string               // Search input
- typeahead: boolean          // Typeahead enabled
- thesaurus: Record<string, NormalisedThesaurus>
- isInitialSetupComplete: boolean
- persistentConversations: ConversationSummary[]
- currentPersistentConversationId: string | null
- conversationStatistics: ConversationStatistics
- showSessionList: boolean
- contexts: ContextItem[]
```

### Dependencies to Replace

**Frontend (Svelte → Dioxus)**:
- `svelte` → Dioxus components
- `tinro` → `dioxus-router`
- `svelma` → Custom Dioxus + Bulma components
- `@tauri-apps/api` → `tauri` crate (server-side) + `dioxus::desktop`
- `@paralect/novel-svelte` → Custom editor solution
- `d3` → `plotters` or keep D3 via WASM interop
- `svelte-markdown` → Custom Markdown renderer

**Keep As-Is**:
- Bulma CSS framework
- Bulmaswatch themes
- FontAwesome icons
- All Rust backend code (`terraphim_*` crates)

---

## Dioxus Target Architecture

### Framework Capabilities (Dioxus 0.7)

**Desktop Support**:
- Native window management via `dioxus::desktop`
- System tray support via `dioxus::desktop::trayicon`
- Global shortcuts via `dioxus::desktop` hooks
- File system access
- WebView-based rendering (similar to Tauri)

**Component Model**:
- RSX macro (React-like syntax)
- Props for component configuration
- Hooks for state and side effects
- Contexts for global state

**State Management**:
- `use_signal()` - Local reactive state
- `use_context()` - Shared state across components
- `use_memo()` - Derived state
- `use_resource()` - Async data fetching
- `use_effect()` - Side effects

**Routing**:
- `dioxus-router` crate
- Declarative routing
- Route parameters and query strings
- Navigation guards

**Styling**:
- Inline styles
- CSS classes
- Tailwind integration (optional)
- **Preserve Bulma CSS** for consistency

### Proposed Structure

```
terraphim-desktop-dioxus/
├── src/
│   ├── main.rs                 # App entry, system tray, window setup
│   ├── app.rs                  # Root component, router
│   ├── components/
│   │   ├── mod.rs
│   │   ├── chat/
│   │   │   ├── mod.rs
│   │   │   ├── chat.rs
│   │   │   ├── session_list.rs
│   │   │   └── context_edit_modal.rs
│   │   ├── search/
│   │   │   ├── mod.rs
│   │   │   ├── search.rs
│   │   │   ├── result_item.rs
│   │   │   └── kg_search_modal.rs
│   │   ├── editor/
│   │   │   ├── mod.rs
│   │   │   ├── markdown_editor.rs
│   │   │   └── autocomplete.rs
│   │   ├── graph/
│   │   │   ├── mod.rs
│   │   │   └── rolegraph_viz.rs
│   │   ├── config/
│   │   │   ├── mod.rs
│   │   │   ├── wizard.rs
│   │   │   └── json_editor.rs
│   │   ├── navigation/
│   │   │   ├── mod.rs
│   │   │   ├── navbar.rs
│   │   │   └── footer.rs
│   │   └── common/
│   │       ├── mod.rs
│   │       ├── theme_switcher.rs
│   │       └── back_button.rs
│   ├── state/
│   │   ├── mod.rs
│   │   ├── config.rs           # ConfigState context
│   │   ├── theme.rs            # Theme context
│   │   ├── conversation.rs     # Chat state
│   │   └── search.rs           # Search state
│   ├── services/
│   │   ├── mod.rs
│   │   ├── tauri_commands.rs   # Wrapper for Tauri commands
│   │   ├── autocomplete.rs     # Autocomplete service
│   │   └── theme_manager.rs    # Theme loading
│   ├── utils/
│   │   ├── mod.rs
│   │   └── search_utils.rs     # Shared utilities
│   └── routes/
│       ├── mod.rs
│       ├── search.rs
│       ├── chat.rs
│       ├── graph.rs
│       └── config.rs
├── Cargo.toml
├── Dioxus.toml                 # Dioxus CLI config
├── assets/                     # Static assets
│   ├── bulma/
│   ├── bulmaswatch/
│   ├── icons/
│   └── images/
└── public/                     # Additional static files
```

### Integration with Existing Tauri Backend

**Option A: Pure Dioxus (Recommended)**
- Use `dioxus-desktop` for window management
- Directly call Rust functions (no IPC)
- Share state via Rust types
- System tray via `dioxus::desktop::trayicon`

**Option B: Dioxus + Tauri Hybrid**
- Use `tauri-dioxus` community crate: https://github.com/Desdaemon/tauri-dioxus
- Keep existing Tauri infrastructure
- Call Tauri commands from Dioxus frontend
- System tray via Tauri

**Recommendation**: **Option A** for better type safety and performance. All Tauri commands can be refactored to direct function calls since both frontend and backend are now Rust.

---

## Component Mapping

### Svelte → Dioxus Component Mapping

| **Svelte Component** | **Dioxus Component** | **Complexity** | **Notes** |
|---------------------|---------------------|---------------|-----------|
| `App.svelte` | `app.rs` + `main.rs` | Medium | Router, navigation, system tray setup |
| `Search.svelte` | `components/search/search.rs` | High | Complex autocomplete, term chips, SSE |
| `Chat.svelte` | `components/chat/chat.rs` | High | Session management, context handling |
| `NovelWrapper.svelte` | `components/editor/markdown_editor.rs` | **Very High** | **Needs custom solution** |
| `RoleGraphVisualization.svelte` | `components/graph/rolegraph_viz.rs` | Very High | D3.js → WASM or native Rust |
| `ConfigWizard.svelte` | `components/config/wizard.rs` | Medium | Multi-step form |
| `ConfigJsonEditor.svelte` | `components/config/json_editor.rs` | High | JSON editing with validation |
| `ThemeSwitcher.svelte` | `components/common/theme_switcher.rs` | Low | Dropdown + CSS loading |
| `SessionList.svelte` | `components/chat/session_list.rs` | Medium | List with filtering |
| `ResultItem.svelte` | `components/search/result_item.rs` | Low | Display component |
| `TermChip.svelte` | `components/search/term_chip.rs` | Low | Tag component |
| `KGSearchModal.svelte` | `components/search/kg_search_modal.rs` | Medium | Modal with search |
| `AtomicSaveModal.svelte` | `components/search/atomic_save_modal.rs` | Medium | Modal with form |

### State Migration

| **Svelte Store** | **Dioxus State** | **Implementation** |
|-----------------|-----------------|-------------------|
| `configStore` | `use_context::<ConfigState>()` | Global context |
| `role` | `use_signal(String::new())` | Derived from config |
| `theme` | `use_context::<ThemeState>()` | Global context |
| `is_tauri` | Compile-time constant | `cfg!(feature = "desktop")` |
| `input` | `use_signal(String::new())` | Component-local |
| `persistentConversations` | `use_signal(Vec::<ConversationSummary>::new())` | Component-local |
| `showSessionList` | `use_signal(bool)` | Component-local |
| `contexts` | `use_signal(Vec::<ContextItem>::new())` | Component-local |

---

## Critical Features Requirements

### 1. System Tray - MUST HAVE ✅

**Dioxus Implementation**:
```rust
use dioxus::desktop::trayicon::{
    menu::{Menu, MenuItem, PredefinedMenuItem, AboutMetadata},
    TrayIconBuilder, Icon
};

fn build_tray_menu(config: &Config) -> Menu {
    let menu = Menu::new();

    // Toggle window
    let toggle = MenuItem::new("Show/Hide", true, None);
    menu.append(&toggle).unwrap();
    menu.append(&PredefinedMenuItem::separator()).unwrap();

    // Role switching
    for (role_name, _role) in &config.roles {
        let mut item = MenuItem::new(role_name, true, None);
        if role_name == &config.selected_role {
            item.set_checked(true);
        }
        menu.append(&item).unwrap();
    }

    menu.append(&PredefinedMenuItem::separator()).unwrap();
    let quit = MenuItem::new("Quit", true, None);
    menu.append(&quit).unwrap();

    menu
}

fn setup_tray(config: Config) {
    let menu = build_tray_menu(&config);
    let icon = Icon::from_path("assets/icon.png", None).unwrap();

    let tray = TrayIconBuilder::new()
        .with_menu(Box::new(menu))
        .with_icon(icon)
        .build()
        .unwrap();

    // Event handling
    use_on_tray_icon_menu_event(move |event| {
        match event.id.0.as_str() {
            "quit" => std::process::exit(0),
            "toggle" => {
                // Toggle window visibility
            },
            id if id.starts_with("role_") => {
                // Change role
            },
            _ => {}
        }
    });
}
```

**Status**: ✅ Supported in Dioxus 0.7 via `dioxus::desktop::trayicon`
**Platform Coverage**: Windows, macOS (with known issues as of Jan 2025), Linux

### 2. Global Shortcuts - MUST HAVE ✅

**Dioxus Implementation**:
```rust
use dioxus::desktop::use_global_shortcut;

fn App(cx: Scope) -> Element {
    let window = use_window(cx);
    let shortcut = "Ctrl+Shift+Space"; // From config

    use_global_shortcut(cx, shortcut, move |_| {
        // Toggle window visibility
        if window.is_visible() {
            window.hide();
        } else {
            window.show();
        }
    });

    // ...
}
```

**Status**: ✅ Supported via hooks

### 3. Slash Commands in Editor - MUST HAVE ❓

**Challenge**: No native Dioxus editor with slash commands exists.

**Options**:

**Option A: Custom Dioxus Editor**
- Build from scratch using `<textarea>` + overlay
- Implement slash command detection
- Custom autocomplete dropdown
- **Pros**: Full control, type-safe
- **Cons**: High effort, reinventing the wheel

**Option B: Integrate TipTap via WASM**
- Embed TipTap in a WebView component
- Use JS interop for communication
- **Pros**: Reuse existing solution
- **Cons**: Complexity, FFI overhead

**Option C: Leptos Editor (if exists)**
- Check Leptos ecosystem for editors
- **Pros**: Rust-native, might be compatible
- **Cons**: May not exist or be mature

**Option D: Terminal-style Input**
- Simple text input with prefix detection
- Parse slash commands on submit
- **Pros**: Simple, lightweight
- **Cons**: Less rich UX than WYSIWYG

**Recommendation**: Start with **Option D** for MVP, evaluate **Option A** or **Option B** based on requirements.

### 4. Knowledge Graph Visualization - HIGH PRIORITY

**Options**:

**Option A: Plotters (Rust)**
- Use `plotters` crate for static graphs
- Export as PNG/SVG
- **Pros**: Pure Rust
- **Cons**: Not interactive

**Option B: D3.js via WASM Interop**
- Keep existing D3 code
- Embed in Dioxus component
- Use `wasm-bindgen` for interop
- **Pros**: Reuse existing code, interactive
- **Cons**: JS dependency

**Option C: egui Integration**
- Use `egui` for interactive graphs
- Render in Dioxus window
- **Pros**: Rust-native, interactive
- **Cons**: Different UI paradigm

**Recommendation**: **Option B** for feature parity, consider **Option C** for future improvements.

### 5. Autocomplete from Knowledge Graph - MUST HAVE ✅

**Implementation**:
- Reuse existing `terraphim_automata` crate
- Call `autocomplete_terms()` directly (no IPC)
- Render suggestions in Dioxus dropdown
- **Status**: ✅ Straightforward with direct function calls

### 6. Theme Switching - MEDIUM PRIORITY ✅

**Implementation**:
```rust
use dioxus::prelude::*;

#[component]
fn ThemeSwitcher(cx: Scope) -> Element {
    let theme = use_context::<ThemeState>(cx);
    let themes = vec!["cerulean", "cosmo", "cyborg", /* ... */];

    rsx! {
        select {
            value: "{theme.current}",
            onchange: move |evt| {
                let new_theme = evt.value.clone();
                theme.set(new_theme.clone());
                load_theme_css(&new_theme);
            },
            for theme_name in themes {
                option { value: "{theme_name}", "{theme_name}" }
            }
        }
    }
}

fn load_theme_css(theme: &str) {
    // Inject CSS via eval or filesystem
    let css_path = format!("assets/bulmaswatch/{}/bulmaswatch.min.css", theme);
    // Load and inject
}
```

**Status**: ✅ Feasible with CSS injection

---

## Editor Strategy

### Requirement Analysis

**Current Editor Features**:
1. **Markdown rendering** - Parse and display Markdown
2. **Autocomplete** - Trigger with `++`, suggest from KG
3. **Slash commands** - `/command` for actions (DESIRED)
4. **Snippets** - Show snippets in suggestions
5. **Debouncing** - Delay before fetching suggestions
6. **Role-aware** - Suggestions change with role

**Terraphim-Editor Evaluation**:
- ❌ Not suitable: Standalone WASM app, no slash commands
- ❌ No Dioxus/framework integration

### Proposed Solutions

#### Solution 1: Simple Command Input (MVP)

**Description**: Text input with slash command parsing on submit.

```rust
#[component]
fn CommandInput(cx: Scope) -> Element {
    let input = use_signal(cx, String::new);
    let suggestions = use_signal(cx, Vec::<String>::new);

    let on_input = move |evt: FormEvent| {
        let value = evt.value.clone();
        input.set(value.clone());

        if value.starts_with('/') {
            // Show command suggestions
            let commands = vec!["/search", "/chat", "/graph"];
            suggestions.set(commands.into_iter()
                .filter(|c| c.starts_with(&value))
                .map(String::from)
                .collect());
        } else if value.starts_with("++") {
            // Show KG autocomplete
            let query = value.trim_start_matches("++");
            // Call autocomplete service
        }
    };

    rsx! {
        div { class: "editor-container",
            input {
                value: "{input}",
                oninput: on_input,
                placeholder: "Type / for commands or ++ for autocomplete"
            }
            if !suggestions.is_empty() {
                ul { class: "suggestions",
                    for suggestion in suggestions.read().iter() {
                        li { "{suggestion}" }
                    }
                }
            }
        }
    }
}
```

**Pros**:
- Simple to implement
- No external dependencies
- Type-safe

**Cons**:
- Limited UX (no rich text)
- No WYSIWYG editing

#### Solution 2: Rich Text Editor with Web Components

**Description**: Embed TipTap/ProseMirror via WebView iframe.

```rust
#[component]
fn RichEditor(cx: Scope, content: Signal<String>) -> Element {
    rsx! {
        iframe {
            src: "editor.html",  // Self-hosted editor
            onmessage: move |evt| {
                // Handle editor events
                content.set(evt.data);
            }
        }
    }
}
```

**Implementation**:
1. Create `assets/editor.html` with TipTap
2. Use `postMessage` for communication
3. Listen for autocomplete triggers
4. Send suggestions back to iframe

**Pros**:
- Rich editing experience
- Reuse existing TipTap setup
- Slash commands supported

**Cons**:
- JS dependency
- Complex communication

#### Solution 3: Native Dioxus Editor (Long-term)

**Description**: Build a custom contenteditable-based editor in pure Dioxus.

**Implementation**:
1. Use `contenteditable` div
2. Parse input on `oninput` events
3. Overlay suggestion dropdown
4. Inject autocomplete results

**Status**: Requires significant development effort.

### **DECISION REQUIRED**: Which editor strategy to pursue?

**Input Format**:
- **Use Case**: Chat, note-taking, search refinement
- **Required Features**: Slash commands, autocomplete, snippets
- **Nice-to-Have**: Markdown rendering, rich formatting

**Action**: Choose based on priority:
1. **MVP/Quick Win**: Solution 1 (Simple Command Input)
2. **Feature Parity**: Solution 2 (Rich Text via iframe)
3. **Future-Proof**: Solution 3 (Native Dioxus, roadmap item)

---

## Migration Phases

### Phase 0: Preparation & Setup ✅

**Duration**: 1-2 days

**Tasks**:
1. ✅ Create this specification document
2. ⏳ Create Dioxus workspace structure
3. ⏳ Set up Dioxus.toml configuration
4. ⏳ Add dependencies (dioxus, dioxus-router, etc.)
5. ⏳ Copy Bulma CSS assets
6. ⏳ Create basic window + system tray

**Deliverable**: Empty Dioxus app with system tray and window.

### Phase 1: Core Infrastructure (Week 1)

**Duration**: 5-7 days

**Tasks**:
1. Implement main window with routing
2. Create global state contexts (Config, Theme)
3. Set up system tray with role menu
4. Implement global shortcuts
5. Create navbar with tabs
6. Implement theme switcher
7. Test window show/hide/minimize

**Deliverable**: App shell with navigation and system tray working.

### Phase 2: Search Feature (Week 2)

**Duration**: 5-7 days

**Tasks**:
1. Implement search input component
2. Add autocomplete dropdown
3. Implement term chips (AND/OR logic)
4. Create result list component
5. Integrate with `cmd::search` (refactor to direct calls)
6. Add state persistence (localStorage equivalent)
7. Implement SSE/polling for summaries

**Deliverable**: Fully functional search with autocomplete.

### Phase 3: Chat Feature (Week 3)

**Duration**: 5-7 days

**Tasks**:
1. Implement chat UI with message list
2. Create session list component
3. Add context management (attach docs, KG terms)
4. Implement context edit modal
5. Add conversation persistence
6. Integrate chat service calls
7. Test conversation export/import

**Deliverable**: Complete chat interface with context management.

### Phase 4: Editor Integration (Week 4)

**Duration**: 7-10 days

**Tasks**:
1. **Decision Point**: Choose editor strategy
2. Implement chosen solution
3. Add slash command parsing
4. Integrate autocomplete service
5. Add snippet support
6. Test with different roles
7. Add markdown preview (if applicable)

**Deliverable**: Working editor with slash commands and autocomplete.

### Phase 5: Graph Visualization (Week 5)

**Duration**: 5-7 days

**Tasks**:
1. **Decision Point**: Choose D3 interop vs native Rust
2. Implement graph rendering
3. Add node/edge interaction
4. Integrate with `cmd::get_rolegraph`
5. Add filtering and search
6. Performance optimization

**Deliverable**: Interactive knowledge graph view.

### Phase 6: Configuration & Polish (Week 6)

**Duration**: 5-7 days

**Tasks**:
1. Implement configuration wizard
2. Create JSON editor component
3. Add startup screen
4. Implement all modals (KG search, Atomic save, etc.)
5. Add error handling and loading states
6. Polish UI/UX
7. Test all themes

**Deliverable**: Complete configuration management.

### Phase 7: Testing & Refinement (Week 7)

**Duration**: 5-7 days

**Tasks**:
1. Write integration tests
2. Test all Tauri commands
3. Cross-platform testing (Windows, macOS, Linux)
4. Performance profiling
5. Memory leak detection
6. Accessibility audit
7. Documentation

**Deliverable**: Production-ready application.

### Phase 8: Migration & Deployment (Week 8)

**Duration**: 3-5 days

**Tasks**:
1. Create migration guide for users
2. Set up CI/CD for Dioxus builds
3. Update release workflows
4. Create release notes
5. Deprecate Svelte version
6. Archive old codebase

**Deliverable**: Released Dioxus version.

---

## Open Questions

### Technical Decisions

1. **Q: Editor Strategy**
   **Input**: Which solution for slash commands?
   - A: Simple command input (MVP)
   - B: TipTap iframe integration
   - C: Native Dioxus editor (long-term)

2. **Q: Graph Visualization**
   **Input**: D3.js interop or native Rust?
   - A: Keep D3.js via WASM interop (faster)
   - B: Use native Rust (plotters/egui)

3. **Q: State Persistence**
   **Input**: How to persist UI state?
   - A: Use web storage API via eval
   - B: Use native file system
   - C: Store in Config/DeviceSettings

4. **Q: Tauri Integration**
   **Input**: Pure Dioxus or Dioxus+Tauri hybrid?
   - A: Pure Dioxus (recommended)
   - B: tauri-dioxus crate

5. **Q: CSS Strategy**
   **Input**: Keep Bulma or migrate to Tailwind/native?
   - A: Keep Bulma (faster migration)
   - B: Migrate to Tailwind
   - C: Use Dioxus native styling

### User Experience

6. **Q: Migration Path**
   **Action**: How to migrate existing user data?
   - Config files (already compatible)
   - Conversation history
   - Theme preferences

7. **Q: Feature Parity**
   **Action**: Which features are must-have for v1.0?
   - Search ✅
   - Chat ✅
   - Graph ✅
   - Editor with autocomplete ✅
   - Slash commands ❓

8. **Q: Performance Requirements**
   **Action**: Acceptable load times?
   - Window show/hide: < 100ms
   - Search results: < 500ms
   - Graph rendering: < 2s

9. **Q: Backward Compatibility**
   **Action**: Support old Svelte configs?
   - Yes, auto-migrate on first run
   - No, require manual migration

### Development Process

10. **Q: Testing Strategy**
    **Action**: Test coverage requirements?
    - Unit tests for all components
    - Integration tests for workflows
    - E2E tests for critical paths

11. **Q: Code Review**
    **Action**: Review process for each phase?
    - Self-review with checklist
    - Pair programming
    - Formal review per phase

12. **Q: Documentation**
    **Action**: Documentation requirements?
    - Inline code docs
    - Architecture decision records (ADRs)
    - User guide updates

---

## Next Steps

1. **Answer Open Questions** - Gather requirements and make decisions
2. **Create Implementation Plan** - Detailed task breakdown per phase
3. **Set Up Development Environment** - Initialize Dioxus workspace
4. **Begin Phase 1** - Core infrastructure implementation

---

## Appendix

### Dioxus Resources

- **Official Docs**: https://dioxuslabs.com/learn/0.7/
- **Desktop Guide**: https://dioxuslabs.com/learn/0.7/guides/desktop
- **System Tray Issue**: https://github.com/DioxusLabs/dioxus/issues/2138
- **Community**: Discord, GitHub Discussions

### Reference Implementation

- **tauri-dioxus**: https://github.com/Desdaemon/tauri-dioxus
- **Dioxus Examples**: https://github.com/DioxusLabs/dioxus/tree/main/examples

### Dependencies

```toml
[dependencies]
dioxus = { version = "0.7", features = ["desktop", "router"] }
dioxus-desktop = "0.7"
dioxus-router = "0.7"
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"

# Existing Terraphim crates (reuse as-is)
terraphim_config = { path = "../crates/terraphim_config" }
terraphim_service = { path = "../crates/terraphim_service" }
terraphim_types = { path = "../crates/terraphim_types" }
terraphim_automata = { path = "../crates/terraphim_automata" }
terraphim_rolegraph = { path = "../crates/terraphim_rolegraph" }
# ... other crates
```

---

**End of Specification**
