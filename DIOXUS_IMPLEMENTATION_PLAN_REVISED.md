# Terraphim Desktop: Dioxus Implementation Plan (Revised)

**Version:** 2.0
**Date:** 2025-11-09
**Status:** Implementation Phase
**Based On**: User decisions and requirements

---

## User Requirements Summary

### Editor Decision
- ✅ **Option A**: Simple Command Input with **Markdown Rendering**
- Textarea-based editor
- Slash command detection (`/search`, `/chat`, etc.)
- Autocomplete with `++` trigger
- **Markdown rendering** for output/preview

### Features Scope
- ❌ **Graph Visualization**: Not needed (skip entirely)
- ✅ **Focus on Role Switching**: Replace theme switching with role management UI
- ✅ **Session/Conversation Persistence**: Critical requirement

### Must-Have Features for v1.0

| Feature | Priority | Status |
|---------|----------|--------|
| Search with autocomplete | ✅ Must-have | Planned |
| Chat with context management and history | ✅ Must-have | Planned |
| System tray with role switching | ✅ Must-have | Planned |
| Command/UI for role switching | ✅ Must-have | Planned |
| Global shortcuts | ✅ Must-have | Planned |
| Editor with slash commands | ✅ Must-have | Planned |
| Configuration wizard | ✅ Must-have | Planned |
| Role switching (NOT theme switching) | ✅ Must-have | Planned |
| Conversation persistence and history | ✅ Must-have | Planned |
| Session persistence and history | ✅ Must-have | Planned |

### Testing Requirements
- ✅ **End-to-End tests** required for Dioxus application

---

## Revised Architecture

### Technology Stack

```toml
[dependencies]
dioxus = { version = "0.6", features = ["desktop", "router"] }
dioxus-desktop = "0.6"
dioxus-router = "0.6"
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
tokio = { version = "1", features = ["full"] }
reqwest = { version = "0.12", features = ["json"] }
pulldown-cmark = "0.12"  # Markdown rendering
anyhow = "1.0"
tracing = "0.1"
tracing-subscriber = "0.3"

# Terraphim core crates (existing)
terraphim_config = { path = "../crates/terraphim_config" }
terraphim_service = { path = "../crates/terraphim_service" }
terraphim_types = { path = "../crates/terraphim_types" }
terraphim_automata = { path = "../crates/terraphim_automata" }
terraphim_rolegraph = { path = "../crates/terraphim_rolegraph" }
terraphim_middleware = { path = "../crates/terraphim_middleware" }
terraphim_persistence = { path = "../crates/terraphim_persistence" }
terraphim_settings = { path = "../crates/terraphim_settings" }

[dev-dependencies]
dioxus-playwright = "0.1"  # E2E testing
tokio-test = "0.4"
```

### Project Structure

```
desktop-dioxus/
├── Cargo.toml
├── Dioxus.toml
├── src/
│   ├── main.rs                      # Entry point, system tray, window
│   ├── app.rs                       # Root component, router
│   ├── components/
│   │   ├── mod.rs
│   │   ├── navigation/
│   │   │   ├── mod.rs
│   │   │   ├── navbar.rs           # Navigation bar with tabs
│   │   │   └── role_selector.rs    # Role switching UI
│   │   ├── search/
│   │   │   ├── mod.rs
│   │   │   ├── search.rs           # Search component
│   │   │   ├── result_item.rs      # Result display
│   │   │   ├── term_chip.rs        # Term chips (AND/OR)
│   │   │   └── autocomplete.rs     # Autocomplete dropdown
│   │   ├── chat/
│   │   │   ├── mod.rs
│   │   │   ├── chat.rs             # Chat interface
│   │   │   ├── session_list.rs     # Session sidebar
│   │   │   ├── context_panel.rs    # Context management
│   │   │   └── message_list.rs     # Message display
│   │   ├── editor/
│   │   │   ├── mod.rs
│   │   │   ├── command_editor.rs   # Command input with slash commands
│   │   │   ├── markdown_preview.rs # Markdown rendering
│   │   │   └── suggestion_dropdown.rs
│   │   ├── config/
│   │   │   ├── mod.rs
│   │   │   ├── wizard.rs           # Configuration wizard
│   │   │   └── json_editor.rs      # JSON config editor
│   │   └── common/
│   │       ├── mod.rs
│   │       ├── modal.rs            # Reusable modal
│   │       └── loading.rs          # Loading indicators
│   ├── state/
│   │   ├── mod.rs
│   │   ├── config.rs               # Config state context
│   │   ├── conversation.rs         # Chat state
│   │   └── search.rs               # Search state
│   ├── services/
│   │   ├── mod.rs
│   │   ├── autocomplete.rs         # Autocomplete service
│   │   ├── storage.rs              # Local storage service
│   │   ├── conversation_storage.rs # Conversation persistence
│   │   └── markdown.rs             # Markdown utilities
│   ├── routes/
│   │   ├── mod.rs
│   │   ├── search.rs               # Search page
│   │   ├── chat.rs                 # Chat page
│   │   └── config.rs               # Config pages
│   ├── system_tray.rs              # System tray implementation
│   └── utils/
│       ├── mod.rs
│       └── search_utils.rs
├── assets/
│   ├── icons/
│   │   ├── icon.png
│   │   └── (other icon sizes)
│   ├── bulma/
│   │   └── bulma.min.css
│   └── styles/
│       └── custom.css
├── tests/
│   ├── e2e/
│   │   ├── search_flow.rs          # Search E2E tests
│   │   ├── chat_flow.rs            # Chat E2E tests
│   │   ├── role_switching.rs       # Role switching tests
│   │   └── system_tray.rs          # System tray tests
│   └── integration/
│       ├── autocomplete.rs
│       └── conversation_persistence.rs
└── README.md
```

---

## Implementation Phases (Revised)

### Phase 0: Project Setup (Days 1-2) ⏳

**Goals**:
- Create Dioxus project structure
- Set up dependencies
- Configure build system
- Copy assets
- Basic window + system tray

**Tasks**:
1. Create `desktop-dioxus/` directory
2. Set up `Cargo.toml` with all dependencies
3. Configure `Dioxus.toml`
4. Create module structure (`src/components/`, `src/state/`, etc.)
5. Copy Bulma CSS and icons to `assets/`
6. Create basic `main.rs` with window
7. Implement system tray with role menu
8. Test window show/hide

**Deliverables**:
- ✅ Dioxus project compiles
- ✅ Window opens with basic UI
- ✅ System tray icon appears
- ✅ Global shortcut toggles window

**Files Created**:
- `Cargo.toml`
- `Dioxus.toml`
- `src/main.rs`
- `src/app.rs`
- `src/system_tray.rs`

---

### Phase 1: Core Infrastructure (Days 3-7)

**Goals**:
- Routing system
- Global state management
- Role switching UI
- Navigation

**Tasks**:
1. Implement routing with `dioxus-router`
2. Create `ConfigState` context
3. Create `ConversationState` context
4. Implement navbar with tabs (Search, Chat, Config)
5. Implement role selector component
6. Connect role switching to system tray
7. Set up global shortcuts
8. Add loading states

**Deliverables**:
- ✅ Navigation between pages works
- ✅ Role switching functional (UI + tray)
- ✅ Global state accessible in components
- ✅ Global shortcuts working

**Files Created**:
- `src/routes/mod.rs`
- `src/state/config.rs`
- `src/state/conversation.rs`
- `src/components/navigation/navbar.rs`
- `src/components/navigation/role_selector.rs`

---

### Phase 2: Search Feature (Days 8-12)

**Goals**:
- Search input with autocomplete
- Term chips (AND/OR logic)
- Result display
- State persistence

**Tasks**:
1. Create search input component
2. Implement autocomplete service (direct calls to `terraphim_automata`)
3. Add suggestion dropdown
4. Implement term chips with operators
5. Create result item component
6. Add search result display
7. Implement local storage for search state
8. Add loading/error states

**Deliverables**:
- ✅ Search with autocomplete functional
- ✅ Term chips work (AND/OR)
- ✅ Results display correctly
- ✅ Search state persists per role

**Files Created**:
- `src/components/search/search.rs`
- `src/components/search/autocomplete.rs`
- `src/components/search/result_item.rs`
- `src/components/search/term_chip.rs`
- `src/services/autocomplete.rs`
- `src/state/search.rs`

---

### Phase 3: Chat Feature (Days 13-18)

**Goals**:
- Chat interface with message list
- Session management
- Context attachment
- Conversation persistence

**Tasks**:
1. Create chat UI component
2. Implement message list
3. Create session list sidebar
4. Add context panel (attach docs, KG terms)
5. Implement conversation service calls
6. Add conversation persistence (filesystem)
7. Implement session history
8. Add conversation export/import
9. Create context edit modal

**Deliverables**:
- ✅ Chat interface functional
- ✅ Session switching works
- ✅ Context attachment works
- ✅ Conversations persist to disk
- ✅ History loads correctly

**Files Created**:
- `src/components/chat/chat.rs`
- `src/components/chat/session_list.rs`
- `src/components/chat/context_panel.rs`
- `src/components/chat/message_list.rs`
- `src/services/conversation_storage.rs`
- `src/routes/chat.rs`

---

### Phase 4: Editor with Slash Commands (Days 19-23)

**Goals**:
- Command input editor
- Slash command detection
- Markdown rendering
- Autocomplete integration

**Tasks**:
1. Create command editor component (textarea-based)
2. Implement slash command parsing (`/search`, `/chat`, `/help`)
3. Add command suggestions dropdown
4. Integrate `++` autocomplete trigger
5. Implement markdown rendering (using `pulldown-cmark`)
6. Add markdown preview panel
7. Create suggestion dropdown component
8. Test with different roles

**Deliverables**:
- ✅ Editor detects slash commands
- ✅ Autocomplete works with `++`
- ✅ Markdown renders correctly
- ✅ Command suggestions appear

**Files Created**:
- `src/components/editor/command_editor.rs`
- `src/components/editor/markdown_preview.rs`
- `src/components/editor/suggestion_dropdown.rs`
- `src/services/markdown.rs`

---

### Phase 5: Configuration Wizard (Days 24-28)

**Goals**:
- Multi-step configuration wizard
- Role creation/editing
- Haystack configuration
- Settings persistence

**Tasks**:
1. Create wizard component (multi-step form)
2. Add role creation form
3. Add haystack configuration
4. Implement JSON editor for advanced users
5. Add validation
6. Connect to `terraphim_config` backend
7. Add startup screen (first-time setup)

**Deliverables**:
- ✅ Wizard guides new users
- ✅ Role creation works
- ✅ Config persists correctly
- ✅ Advanced JSON editor available

**Files Created**:
- `src/components/config/wizard.rs`
- `src/components/config/json_editor.rs`
- `src/components/common/modal.rs`
- `src/routes/config.rs`

---

### Phase 6: Polish & Error Handling (Days 29-33)

**Goals**:
- Error handling
- Loading states
- User feedback
- Accessibility

**Tasks**:
1. Add error boundaries
2. Implement toast notifications
3. Add loading spinners
4. Improve keyboard navigation
5. Add tooltips and help text
6. Accessibility audit (ARIA labels)
7. Performance optimization

**Deliverables**:
- ✅ Errors handled gracefully
- ✅ Loading states clear
- ✅ Keyboard navigation works
- ✅ Accessible UI

---

### Phase 7: E2E Testing (Days 34-38)

**Goals**:
- Comprehensive E2E tests
- Integration tests
- Test coverage

**Tasks**:
1. Set up `dioxus-playwright` for E2E testing
2. Write search flow tests
3. Write chat flow tests
4. Write role switching tests
5. Write system tray tests
6. Write conversation persistence tests
7. Integration tests for autocomplete
8. CI/CD test automation

**Deliverables**:
- ✅ E2E tests cover all critical paths
- ✅ Tests run in CI
- ✅ >80% coverage on core features

**Files Created**:
- `tests/e2e/search_flow.rs`
- `tests/e2e/chat_flow.rs`
- `tests/e2e/role_switching.rs`
- `tests/e2e/system_tray.rs`
- `tests/integration/autocomplete.rs`
- `tests/integration/conversation_persistence.rs`

---

### Phase 8: Documentation & Release (Days 39-42)

**Goals**:
- User documentation
- Migration guide
- Release preparation

**Tasks**:
1. Write user guide
2. Create migration guide from Svelte version
3. Update README
4. Create release notes
5. Update CI/CD for Dioxus builds
6. Create installers/packages
7. Test on all platforms (Windows, macOS, Linux)

**Deliverables**:
- ✅ Complete documentation
- ✅ Migration guide
- ✅ v1.0 release

---

## Timeline Summary

| Phase | Duration | Focus | Status |
|-------|----------|-------|--------|
| 0 | Days 1-2 | Project setup, system tray | ⏳ Starting |
| 1 | Days 3-7 | Core infrastructure, routing, role switching | Pending |
| 2 | Days 8-12 | Search with autocomplete | Pending |
| 3 | Days 13-18 | Chat with persistence | Pending |
| 4 | Days 19-23 | Editor with slash commands + markdown | Pending |
| 5 | Days 24-28 | Configuration wizard | Pending |
| 6 | Days 29-33 | Polish & error handling | Pending |
| 7 | Days 34-38 | E2E testing | Pending |
| 8 | Days 39-42 | Documentation & release | Pending |

**Total**: ~42 days (6 weeks)

---

## E2E Testing Strategy

### Testing Framework

Using `dioxus-playwright` for browser-based E2E testing:

```rust
// tests/e2e/search_flow.rs
use dioxus_playwright::prelude::*;

#[tokio::test]
async fn test_search_with_autocomplete() {
    let mut app = launch_app().await;

    // Navigate to search page
    app.goto("/").await.unwrap();

    // Type in search input
    let search_input = app.find("input[type='search']").await.unwrap();
    search_input.fill("terr").await.unwrap();

    // Wait for autocomplete suggestions
    let suggestions = app.find(".suggestions").await.unwrap();
    assert!(suggestions.is_visible().await.unwrap());

    // Click first suggestion
    let first_suggestion = app.find(".suggestions li:first-child").await.unwrap();
    first_suggestion.click().await.unwrap();

    // Verify search input updated
    let input_value = search_input.input_value().await.unwrap();
    assert!(input_value.contains("terraphim"));

    // Submit search
    search_input.press("Enter").await.unwrap();

    // Wait for results
    let results = app.find(".results-container").await.unwrap();
    assert!(results.is_visible().await.unwrap());
}

#[tokio::test]
async fn test_role_switching_via_ui() {
    let mut app = launch_app().await;

    // Open role selector
    let role_selector = app.find(".role-selector select").await.unwrap();
    role_selector.select_option("Llama Engineer").await.unwrap();

    // Verify role changed
    let selected_role = role_selector.input_value().await.unwrap();
    assert_eq!(selected_role, "Llama Engineer");

    // Verify search reflects new role
    let search_placeholder = app
        .find("input[type='search']")
        .await
        .unwrap()
        .get_attribute("placeholder")
        .await
        .unwrap();
    assert!(search_placeholder.contains("Llama Engineer"));
}
```

### Test Coverage Plan

| Area | Test Count | Priority |
|------|-----------|----------|
| Search flow | 5 tests | High |
| Chat flow | 8 tests | High |
| Role switching | 4 tests | High |
| System tray | 3 tests | High |
| Conversation persistence | 6 tests | High |
| Autocomplete | 4 tests | Medium |
| Configuration wizard | 5 tests | Medium |
| Editor slash commands | 4 tests | Medium |

**Total**: ~40 E2E tests

---

## Next Steps (Immediate)

Starting Phase 0 now:

1. ✅ Create `desktop-dioxus/` directory structure
2. ✅ Set up `Cargo.toml`
3. ✅ Configure `Dioxus.toml`
4. ✅ Create initial module structure
5. ✅ Implement basic `main.rs` with window
6. ✅ Implement system tray
7. ✅ Copy assets
8. ✅ Test basic functionality

---

**End of Revised Implementation Plan**
