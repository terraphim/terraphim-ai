# Terraphim Desktop: Dioxus Design & Implementation Plan

**Version:** 1.0
**Date:** 2025-11-09
**Status:** Design Phase
**Linked Document**: [DIOXUS_MIGRATION_SPECIFICATION.md](./DIOXUS_MIGRATION_SPECIFICATION.md)

---

## Table of Contents

1. [Design Overview](#design-overview)
2. [System Architecture](#system-architecture)
3. [Component Design](#component-design)
4. [State Management Design](#state-management-design)
5. [Editor Solution Design](#editor-solution-design)
6. [Implementation Plan](#implementation-plan)
7. [Questions for User](#questions-for-user)

---

## Design Overview

### Design Principles

1. **Type Safety First**: Leverage Rust's type system throughout
2. **Composability**: Small, focused components
3. **Performance**: Direct function calls, no IPC overhead
4. **Maintainability**: Clear separation of concerns
5. **Testability**: Components testable in isolation
6. **User Experience**: Maintain or improve current UX

### Architecture Layers

```
┌─────────────────────────────────────────────────────────┐
│                    Dioxus Desktop                       │
│  ┌───────────────────────────────────────────────────┐  │
│  │              UI Layer (RSX)                       │  │
│  │  - Components (Search, Chat, Graph, etc.)        │  │
│  │  - Routing (dioxus-router)                       │  │
│  │  - Styling (Bulma CSS)                           │  │
│  └───────────────┬───────────────────────────────────┘  │
│                  │                                       │
│  ┌───────────────▼───────────────────────────────────┐  │
│  │         State Management Layer                    │  │
│  │  - Signals (use_signal)                          │  │
│  │  - Contexts (use_context)                        │  │
│  │  - Resources (use_resource)                      │  │
│  └───────────────┬───────────────────────────────────┘  │
│                  │                                       │
│  ┌───────────────▼───────────────────────────────────┐  │
│  │           Service Layer                          │  │
│  │  - Autocomplete Service                          │  │
│  │  - Theme Manager                                 │  │
│  │  - Storage Service                               │  │
│  └───────────────┬───────────────────────────────────┘  │
│                  │                                       │
└──────────────────┼───────────────────────────────────────┘
                   │
┌──────────────────▼───────────────────────────────────────┐
│              Terraphim Core (Existing)                   │
│  - terraphim_config                                      │
│  - terraphim_service                                     │
│  - terraphim_automata                                    │
│  - terraphim_rolegraph                                   │
│  - terraphim_middleware                                  │
│  - terraphim_persistence                                 │
└──────────────────────────────────────────────────────────┘
```

### Technology Stack

| **Layer** | **Current (Svelte)** | **Target (Dioxus)** |
|-----------|---------------------|---------------------|
| **Framework** | Svelte 5 + TypeScript | Dioxus 0.7 + Rust |
| **Desktop Runtime** | Tauri 1.5 | Dioxus Desktop (or Tauri hybrid) |
| **Routing** | Tinro | dioxus-router |
| **State** | Svelte stores | Signals + Contexts |
| **Styling** | Bulma CSS + Bulmaswatch | Bulma CSS (preserved) |
| **HTTP** | fetch API | reqwest (async) |
| **Storage** | localStorage | serde_json + filesystem |
| **Build** | Vite + Yarn | Dioxus CLI + Cargo |

---

## System Architecture

### Application Entry Point

**File**: `src/main.rs`

```rust
use dioxus::prelude::*;
use dioxus::desktop::{Config as WindowConfig, WindowBuilder};
use terraphim_config::{ConfigBuilder, ConfigId};
use terraphim_persistence::Persistable;

mod app;
mod components;
mod state;
mod services;
mod routes;

fn main() {
    // Initialize logging
    terraphim_service::logging::init_logging(
        terraphim_service::logging::detect_logging_config()
    );

    // Load configuration
    let config = load_or_create_config();

    // Set up system tray
    let tray_menu = build_tray_menu(&config);

    // Launch app
    dioxus::LaunchBuilder::desktop()
        .with_cfg(
            WindowConfig::new()
                .with_title("Terraphim AI")
                .with_window_size((1024.0, 768.0))
        )
        .with_context(config)
        .launch(app::App);
}

fn load_or_create_config() -> terraphim_config::Config {
    // Load from persistence or create default
    tokio::runtime::Runtime::new()
        .unwrap()
        .block_on(async {
            match ConfigBuilder::new_with_id(ConfigId::Desktop).build() {
                Ok(mut config) => match config.load().await {
                    Ok(loaded) => loaded,
                    Err(_) => ConfigBuilder::new()
                        .build_default_desktop()
                        .build()
                        .unwrap(),
                },
                Err(_) => ConfigBuilder::new()
                    .build_default_desktop()
                    .build()
                    .unwrap(),
            }
        })
}

fn build_tray_menu(config: &terraphim_config::Config) -> Menu {
    // System tray implementation (see below)
}
```

### System Tray Implementation

**File**: `src/system_tray.rs`

```rust
use dioxus::desktop::trayicon::{
    menu::{Menu, MenuItem, PredefinedMenuItem},
    TrayIconBuilder, Icon, TrayIconEvent
};
use terraphim_config::Config;

pub fn build_tray_menu(config: &Config) -> Menu {
    let menu = Menu::new();

    // Toggle window item
    let toggle = MenuItem::new("Show/Hide", true, Some("toggle".into()));
    menu.append(&toggle).unwrap();
    menu.append(&PredefinedMenuItem::separator()).unwrap();

    // Role items
    for (role_name, _) in &config.roles {
        let item_id = format!("role_{}", role_name.original);
        let mut item = MenuItem::new(
            &role_name.original,
            true,
            Some(item_id.into())
        );

        if role_name == &config.selected_role {
            item.set_checked(true);
        }

        menu.append(&item).unwrap();
    }

    menu.append(&PredefinedMenuItem::separator()).unwrap();

    // Quit item
    let quit = MenuItem::new("Quit", true, Some("quit".into()));
    menu.append(&quit).unwrap();

    menu
}

pub fn setup_tray_icon(config: &Config) -> TrayIcon {
    let menu = build_tray_menu(config);
    let icon = Icon::from_path("assets/icons/icon.png", None)
        .expect("Failed to load tray icon");

    TrayIconBuilder::new()
        .with_menu(Box::new(menu))
        .with_icon(icon)
        .with_tooltip("Terraphim AI")
        .build()
        .expect("Failed to create tray icon")
}

pub fn handle_tray_event(event: TrayIconEvent, window: &Window) {
    match event {
        TrayIconEvent::Click { .. } => {
            // Single click - could show/hide window
        }
        TrayIconEvent::DoubleClick { .. } => {
            // Double click - restore window
            window.show().ok();
        }
        _ => {}
    }
}

pub fn handle_menu_event(event: MenuEvent, config: &mut Config, window: &Window) {
    if let Some(id) = event.id {
        match id.as_str() {
            "quit" => std::process::exit(0),
            "toggle" => {
                if window.is_visible().unwrap_or(false) {
                    window.hide().ok();
                } else {
                    window.show().ok();
                }
            }
            id if id.starts_with("role_") => {
                let role_name = id.strip_prefix("role_").unwrap();
                // Update selected role
                config.selected_role = role_name.into();
                // Trigger config update
            }
            _ => {}
        }
    }
}
```

### Root Application Component

**File**: `src/app.rs`

```rust
use dioxus::prelude::*;
use dioxus_router::prelude::*;

use crate::components::navigation::Navbar;
use crate::routes::*;
use crate::state::{ConfigState, ThemeState};

#[component]
pub fn App(cx: Scope) -> Element {
    // Initialize global state
    let config = use_context_provider(cx, || ConfigState::new());
    let theme = use_context_provider(cx, || ThemeState::new());

    // Set up global shortcuts
    setup_global_shortcuts(cx, &config);

    // Apply theme on mount
    use_effect(cx, &theme.current, |theme_name| async move {
        load_theme_css(&theme_name).await;
    });

    rsx! {
        div { class: "is-full-height",
            Router::<Route> {}
        }
    }
}

#[derive(Clone, Routable, Debug, PartialEq)]
enum Route {
    #[route("/")]
    SearchPage {},
    #[route("/chat")]
    ChatPage {},
    #[route("/graph")]
    GraphPage {},
    #[route("/config/wizard")]
    ConfigWizardPage {},
    #[route("/config/json")]
    ConfigJsonPage {},
}

#[component]
fn SearchPage(cx: Scope) -> Element {
    rsx! {
        Navbar {}
        crate::routes::search::SearchRoute {}
    }
}

#[component]
fn ChatPage(cx: Scope) -> Element {
    rsx! {
        Navbar {}
        crate::routes::chat::ChatRoute {}
    }
}

#[component]
fn GraphPage(cx: Scope) -> Element {
    rsx! {
        Navbar {}
        crate::routes::graph::GraphRoute {}
    }
}

#[component]
fn ConfigWizardPage(cx: Scope) -> Element {
    rsx! {
        crate::routes::config::WizardRoute {}
    }
}

#[component]
fn ConfigJsonPage(cx: Scope) -> Element {
    rsx! {
        crate::routes::config::JsonEditorRoute {}
    }
}

fn setup_global_shortcuts(cx: Scope, config: &ConfigState) {
    let window = use_window(cx);
    let shortcut = config.get().global_shortcut.clone();

    use_global_shortcut(cx, &shortcut, move |_| {
        toggle_window_visibility(&window);
    });
}

fn toggle_window_visibility(window: &Window) {
    if window.is_visible().unwrap_or(false) {
        window.hide().ok();
    } else {
        window.show().ok();
        window.set_focus().ok();
    }
}

async fn load_theme_css(theme_name: &str) {
    let css_path = format!("assets/bulmaswatch/{}/bulmaswatch.min.css", theme_name);
    // Load CSS via filesystem or embed
    // Inject into <head> via eval or native method
}
```

---

## Component Design

### Navigation Component

**File**: `src/components/navigation/navbar.rs`

```rust
use dioxus::prelude::*;
use dioxus_router::prelude::*;
use crate::state::ConfigState;
use crate::components::common::ThemeSwitcher;

#[component]
pub fn Navbar(cx: Scope) -> Element {
    let nav = use_navigator(cx);
    let current_route = use_route::<Route>(cx);

    rsx! {
        div { class: "top-controls",
            div { class: "main-navigation",
                div { class: "navigation-row",
                    // Logo/Back button
                    button {
                        class: "logo-back-button",
                        onclick: move |_| {
                            if nav.can_go_back() {
                                nav.go_back();
                            } else {
                                nav.push(Route::SearchPage {});
                            }
                        },
                        img {
                            src: "assets/terraphim_gray.png",
                            alt: "Terraphim",
                            class: "logo-image"
                        }
                    }

                    // Tabs
                    div { class: "tabs is-boxed",
                        ul {
                            TabItem {
                                route: Route::SearchPage {},
                                icon: "fa-search",
                                label: "Search",
                                active: matches!(current_route, Route::SearchPage {})
                            }
                            TabItem {
                                route: Route::ChatPage {},
                                icon: "fa-comments",
                                label: "Chat",
                                active: matches!(current_route, Route::ChatPage {})
                            }
                            TabItem {
                                route: Route::GraphPage {},
                                icon: "fa-project-diagram",
                                label: "Graph",
                                active: matches!(current_route, Route::GraphPage {})
                            }
                        }
                    }
                }
            }

            div { class: "role-selector",
                ThemeSwitcher {}
            }
        }
    }
}

#[component]
fn TabItem<R: Routable>(
    cx: Scope,
    route: R,
    icon: &'static str,
    label: &'static str,
    active: bool
) -> Element {
    let nav = use_navigator(cx);

    rsx! {
        li { class: if *active { "is-active" } else { "" },
            a {
                onclick: move |_| nav.push(route.clone()),
                span { class: "icon is-small",
                    i { class: "fas {icon}" }
                }
                span { "{label}" }
            }
        }
    }
}
```

### Search Component

**File**: `src/components/search/search.rs`

```rust
use dioxus::prelude::*;
use terraphim_types::{Document, SearchQuery};
use crate::services::autocomplete::AutocompleteService;
use crate::state::ConfigState;

#[component]
pub fn Search(cx: Scope) -> Element {
    let config = use_context::<ConfigState>(cx);
    let input = use_signal(cx, String::new);
    let results = use_signal(cx, Vec::<Document>::new);
    let suggestions = use_signal(cx, Vec::<String>::new);
    let selected_terms = use_signal(cx, Vec::<SelectedTerm>::new);
    let operator = use_signal(cx, || None::<LogicalOperator>);

    // Load persisted state
    use_effect(cx, (), |_| async move {
        if let Some(state) = load_search_state(&config.get().selected_role).await {
            input.set(state.input);
            results.set(state.results);
        }
    });

    // Autocomplete hook
    use_effect(cx, &input.cloned(), |input_text| {
        let config = config.clone();
        let suggestions = suggestions.clone();

        async move {
            if input_text.is_empty() {
                suggestions.set(vec![]);
                return;
            }

            let svc = AutocompleteService::new(config);
            let suggs = svc.get_suggestions(&input_text, 8).await;
            suggestions.set(suggs);
        }
    });

    // Search handler
    let on_search = move |_| {
        let config = config.clone();
        let input = input.clone();
        let results = results.clone();

        cx.spawn(async move {
            let search_query = build_search_query(&input.read(), &config.get().selected_role);
            let mut service = terraphim_service::TerraphimService::new(config.inner().clone());

            match service.search(&search_query).await {
                Ok(docs) => {
                    results.set(docs);
                    save_search_state(&config.get().selected_role, &SearchState {
                        input: input.read().clone(),
                        results: results.read().clone(),
                    }).await;
                }
                Err(e) => {
                    log::error!("Search failed: {:?}", e);
                }
            }
        });
    };

    rsx! {
        form {
            onsubmit: move |evt| {
                evt.prevent_default();
                on_search(());
            },

            div { class: "field",
                div { class: "control has-icons-left",
                    input {
                        class: "input is-large",
                        r#type: "search",
                        placeholder: "Search knowledge graph...",
                        value: "{input}",
                        oninput: move |evt| input.set(evt.value.clone()),
                        autofocus: true
                    }
                    span { class: "icon is-left",
                        i { class: "fas fa-search" }
                    }

                    // Autocomplete dropdown
                    if !suggestions.is_empty() {
                        ul { class: "suggestions",
                            for suggestion in suggestions.read().iter() {
                                li {
                                    onclick: move |_| {
                                        input.set(suggestion.clone());
                                        suggestions.set(vec![]);
                                    },
                                    "{suggestion}"
                                }
                            }
                        }
                    }
                }

                // Term chips
                if !selected_terms.is_empty() {
                    TermChips {
                        terms: selected_terms.read().clone(),
                        operator: operator.read().clone(),
                        on_remove: move |term| {
                            selected_terms.write().retain(|t| t.value != term);
                        },
                        on_clear: move |_| {
                            selected_terms.set(vec![]);
                            operator.set(None);
                        }
                    }
                }
            }

            // Results
            div { class: "results-container",
                for doc in results.read().iter() {
                    ResultItem { document: doc.clone() }
                }
            }
        }
    }
}

#[derive(Clone, PartialEq)]
struct SelectedTerm {
    value: String,
    is_from_kg: bool,
}

#[derive(Clone, Copy, PartialEq)]
enum LogicalOperator {
    And,
    Or,
}

fn build_search_query(input: &str, role: &terraphim_types::RoleName) -> SearchQuery {
    // Parse input into SearchQuery
    // Handle AND/OR operators
    // Return structured query
    todo!()
}

async fn load_search_state(role: &terraphim_types::RoleName) -> Option<SearchState> {
    // Load from localStorage equivalent (filesystem)
    todo!()
}

async fn save_search_state(role: &terraphim_types::RoleName, state: &SearchState) {
    // Save to filesystem
    todo!()
}

#[derive(Clone)]
struct SearchState {
    input: String,
    results: Vec<Document>,
}
```

### Result Item Component

**File**: `src/components/search/result_item.rs`

```rust
use dioxus::prelude::*;
use terraphim_types::Document;

#[component]
pub fn ResultItem(cx: Scope, document: Document) -> Element {
    let expanded = use_signal(cx, || false);

    rsx! {
        div { class: "card result-item",
            div { class: "card-content",
                div { class: "media",
                    div { class: "media-content",
                        p { class: "title is-4",
                            a { href: "{document.url}", "{document.id}" }
                        }
                        if let Some(desc) = &document.description {
                            p { class: "subtitle is-6", "{desc}" }
                        }
                    }
                    div { class: "media-right",
                        if let Some(rank) = document.rank {
                            span { class: "tag is-info", "Score: {rank:.2}" }
                        }
                    }
                }

                // Summary
                if let Some(summary) = &document.summarization {
                    div { class: "content",
                        p { class: "has-text-grey-dark",
                            strong { "AI Summary: " }
                            "{summary}"
                        }
                    }
                }

                // Body preview
                if *expanded.read() {
                    if let Some(body) = &document.body {
                        div { class: "content",
                            dangerous_inner_html: markdown::to_html(body)
                        }
                    }
                }

                // Expand button
                button {
                    class: "button is-small is-text",
                    onclick: move |_| expanded.set(!*expanded.read()),
                    if *expanded.read() {
                        "Show less"
                    } else {
                        "Show more"
                    }
                }
            }
        }
    }
}
```

---

## State Management Design

### Global State Contexts

**File**: `src/state/config.rs`

```rust
use dioxus::prelude::*;
use terraphim_config::{Config, ConfigState as CoreConfigState};
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Clone)]
pub struct ConfigState {
    inner: Arc<Mutex<CoreConfigState>>,
}

impl ConfigState {
    pub fn new() -> Self {
        // Initialize from persistence
        let config = load_config_blocking();
        let inner = Arc::new(Mutex::new(CoreConfigState::new(config).unwrap()));
        Self { inner }
    }

    pub async fn get(&self) -> Config {
        self.inner.lock().await.config.lock().await.clone()
    }

    pub async fn update(&self, config: Config) -> Result<(), Box<dyn std::error::Error>> {
        let mut state = self.inner.lock().await;
        *state.config.lock().await = config.clone();
        config.save().await?;
        Ok(())
    }

    pub async fn select_role(&self, role_name: String) -> Result<Config, Box<dyn std::error::Error>> {
        let mut state = self.inner.lock().await;
        let mut config = state.config.lock().await;
        config.selected_role = role_name.into();
        config.save().await?;
        Ok(config.clone())
    }

    pub fn inner(&self) -> Arc<Mutex<CoreConfigState>> {
        self.inner.clone()
    }
}

fn load_config_blocking() -> Config {
    tokio::runtime::Runtime::new()
        .unwrap()
        .block_on(async {
            // Load config logic
            terraphim_config::ConfigBuilder::new()
                .build_default_desktop()
                .build()
                .unwrap()
        })
}
```

**File**: `src/state/theme.rs`

```rust
use dioxus::prelude::*;

#[derive(Clone, PartialEq)]
pub struct ThemeState {
    pub current: Signal<String>,
    available: Vec<String>,
}

impl ThemeState {
    pub fn new(cx: Scope) -> Self {
        let current = use_signal(cx, || "spacelab".to_string());
        let available = vec![
            "cerulean", "cosmo", "cyborg", "darkly", "flatly",
            "journal", "litera", "lumen", "lux", "materia",
            "minty", "nuclear", "pulse", "sandstone", "simplex",
            "slate", "solar", "spacelab", "superhero", "united",
            "yeti"
        ].into_iter().map(String::from).collect();

        // Load from storage
        if let Some(saved_theme) = load_theme_from_storage() {
            current.set(saved_theme);
        }

        Self { current, available }
    }

    pub fn set_theme(&self, theme: String) {
        self.current.set(theme.clone());
        save_theme_to_storage(&theme);
        // Trigger CSS reload
    }

    pub fn available_themes(&self) -> &[String] {
        &self.available
    }
}

fn load_theme_from_storage() -> Option<String> {
    // Read from filesystem or config
    None
}

fn save_theme_to_storage(theme: &str) {
    // Write to filesystem
}
```

### Component-Local State

Use `use_signal` for component-local state:
- `input` in Search component
- `expanded` in ResultItem
- `show_modal` in modals

Use `use_memo` for derived state:
- Filtered lists
- Computed values

Use `use_resource` for async data:
- Search results
- Config loading

---

## Editor Solution Design

### **Decision Required**: Editor Strategy

Based on the specification, we have three options:

#### Option 1: Simple Command Input (Recommended for MVP)

**Architecture**:
```rust
#[component]
pub fn CommandEditor(cx: Scope) -> Element {
    let input = use_signal(cx, String::new);
    let suggestions = use_signal(cx, Vec::<Suggestion>::new);
    let mode = use_signal(cx, || EditorMode::Normal);

    // Detect slash commands
    use_effect(cx, &input.cloned(), |text| async move {
        if text.starts_with('/') {
            mode.set(EditorMode::Command);
            let commands = get_slash_commands();
            suggestions.set(filter_commands(&text, commands));
        } else if text.starts_with("++") {
            mode.set(EditorMode::Autocomplete);
            let terms = fetch_kg_terms(&text).await;
            suggestions.set(terms);
        } else {
            mode.set(EditorMode::Normal);
            suggestions.set(vec![]);
        }
    });

    rsx! {
        div { class: "editor-container",
            textarea {
                class: "textarea",
                value: "{input}",
                oninput: move |evt| input.set(evt.value.clone()),
                placeholder: "Type / for commands, ++ for autocomplete"
            }

            if !suggestions.is_empty() {
                SuggestionDropdown {
                    suggestions: suggestions.read().clone(),
                    on_select: move |item| {
                        apply_suggestion(&input, item);
                        suggestions.set(vec![]);
                    }
                }
            }
        }
    }
}

enum EditorMode {
    Normal,
    Command,
    Autocomplete,
}

struct Suggestion {
    text: String,
    snippet: Option<String>,
    description: Option<String>,
}

fn get_slash_commands() -> Vec<Suggestion> {
    vec![
        Suggestion {
            text: "/search".into(),
            snippet: Some("Search knowledge graph".into()),
            description: None,
        },
        Suggestion {
            text: "/chat".into(),
            snippet: Some("Start chat session".into()),
            description: None,
        },
        // ... more commands
    ]
}

async fn fetch_kg_terms(query: &str) -> Vec<Suggestion> {
    // Call autocomplete service
    vec![]
}

fn apply_suggestion(input: &Signal<String>, suggestion: &Suggestion) {
    // Replace current word/command with suggestion
}
```

**Pros**:
- ✅ Simple to implement (1-2 days)
- ✅ Pure Rust, type-safe
- ✅ Supports slash commands and autocomplete
- ✅ Lightweight

**Cons**:
- ❌ No rich text formatting
- ❌ Basic UX compared to TipTap

#### Option 2: Rich Text via iframe + TipTap

**Architecture**:
```rust
#[component]
pub fn RichEditor(cx: Scope, content: Signal<String>) -> Element {
    let iframe_ref = use_node_ref(cx);

    use_effect(cx, (), |_| async move {
        // Initialize iframe communication
        setup_iframe_bridge(iframe_ref);
    });

    rsx! {
        iframe {
            reference: iframe_ref,
            src: "assets/editor.html",
            class: "rich-editor-frame",
            // Set up postMessage communication
        }
    }
}

fn setup_iframe_bridge(iframe_ref: &NodeRef) {
    // Listen for messages from iframe
    // Send autocomplete suggestions back
}
```

**assets/editor.html**:
```html
<!DOCTYPE html>
<html>
<head>
    <script src="https://unpkg.com/@tiptap/core"></script>
    <script src="https://unpkg.com/@tiptap/starter-kit"></script>
    <script src="https://unpkg.com/@tiptap/extension-mention"></script>
</head>
<body>
    <div id="editor"></div>
    <script>
        // Initialize TipTap
        // Listen for autocomplete triggers
        // Send events to parent via postMessage
    </script>
</body>
</html>
```

**Pros**:
- ✅ Rich text editing
- ✅ Reuse existing TipTap setup
- ✅ Slash commands supported

**Cons**:
- ❌ Complex communication layer
- ❌ JS dependency
- ❌ Harder to debug

#### Option 3: Native Dioxus Editor (Future)

Build a contenteditable-based editor from scratch.

**Pros**:
- ✅ Full control
- ✅ Pure Rust

**Cons**:
- ❌ High effort (2-3 weeks)
- ❌ Reinventing the wheel

### **Recommendation**

**For MVP**: Use **Option 1 (Simple Command Input)**
- Implement in Phase 4 (Week 4)
- Takes 1-2 days
- Provides core functionality

**For v2.0**: Consider **Option 2 (Rich Text iframe)** or **Option 3 (Native editor)** based on user feedback.

---

## Implementation Plan

### Phase 0: Setup (Days 1-2) ✅

**Tasks**:
- [x] Create specification document
- [ ] Create Dioxus project structure
- [ ] Set up Cargo.toml with dependencies
- [ ] Configure Dioxus.toml
- [ ] Copy Bulma CSS assets to `assets/`
- [ ] Create basic window + system tray
- [ ] Test window show/hide

**Deliverable**: Minimal Dioxus app with window and tray.

**Files to Create**:
```
terraphim-desktop-dioxus/
├── Cargo.toml
├── Dioxus.toml
├── src/
│   ├── main.rs
│   ├── app.rs
│   └── system_tray.rs
└── assets/
    ├── icons/
    └── bulmaswatch/
```

### Phase 1: Core Infrastructure (Days 3-9)

**Tasks**:
1. Implement main application component with routing
2. Create ConfigState and ThemeState contexts
3. Implement navbar with tabs
4. Create theme switcher component
5. Set up global shortcuts
6. Implement window visibility toggle
7. Add role switching via tray menu
8. Test all navigation

**Deliverable**: Working app shell with navigation, themes, system tray.

**Files to Create**:
- `src/state/config.rs`
- `src/state/theme.rs`
- `src/components/navigation/navbar.rs`
- `src/components/common/theme_switcher.rs`
- `src/routes/mod.rs`

### Phase 2: Search Feature (Days 10-16)

**Tasks**:
1. Create search input component
2. Implement autocomplete service
3. Add term chips component
4. Create result item component
5. Implement search logic (direct calls to terraphim_service)
6. Add state persistence (filesystem)
7. Test search with different queries

**Deliverable**: Functional search with autocomplete.

**Files to Create**:
- `src/components/search/search.rs`
- `src/components/search/result_item.rs`
- `src/components/search/term_chip.rs`
- `src/services/autocomplete.rs`
- `src/services/storage.rs`

### Phase 3: Chat Feature (Days 17-23)

**Tasks**:
1. Create chat UI component
2. Implement session list component
3. Add context management
4. Create context edit modal
5. Implement conversation service calls
6. Add persistence for conversations
7. Test context attachment

**Deliverable**: Working chat with context management.

**Files to Create**:
- `src/components/chat/chat.rs`
- `src/components/chat/session_list.rs`
- `src/components/chat/context_edit_modal.rs`
- `src/state/conversation.rs`

### Phase 4: Editor (Days 24-30)

**Tasks**:
1. Implement simple command editor
2. Add slash command detection
3. Integrate autocomplete service
4. Add snippet support
5. Test with different roles
6. Add markdown preview (optional)

**Deliverable**: Editor with slash commands and autocomplete.

**Files to Create**:
- `src/components/editor/command_editor.rs`
- `src/components/editor/suggestion_dropdown.rs`

### Phase 5: Graph Visualization (Days 31-37)

**Tasks**:
1. Evaluate D3 interop vs native Rust
2. Implement graph rendering
3. Add node/edge interaction
4. Integrate with get_rolegraph
5. Add filtering
6. Performance optimization

**Deliverable**: Interactive graph view.

**Files to Create**:
- `src/components/graph/rolegraph_viz.rs`

### Phase 6: Configuration (Days 38-44)

**Tasks**:
1. Create configuration wizard
2. Implement JSON editor
3. Add startup screen
4. Create all modals (KG search, Atomic save)
5. Add error handling
6. Polish UI/UX

**Deliverable**: Complete configuration management.

**Files to Create**:
- `src/components/config/wizard.rs`
- `src/components/config/json_editor.rs`
- `src/components/common/startup_screen.rs`

### Phase 7: Testing & Refinement (Days 45-51)

**Tasks**:
1. Write integration tests
2. Cross-platform testing
3. Performance profiling
4. Accessibility audit
5. Documentation

**Deliverable**: Production-ready app.

### Phase 8: Migration & Deployment (Days 52-56)

**Tasks**:
1. Create migration guide
2. Update CI/CD
3. Release v1.0
4. Archive Svelte version

**Deliverable**: Released Dioxus version.

---

## Questions for User

### Critical Decisions

**1. Editor Strategy (HIGHEST PRIORITY)**

**Input**: Which editor approach should we use?

- [ ] **Option A**: Simple command input (MVP, 1-2 days)
  - Textarea with slash command and autocomplete
  - No rich text formatting
  - Fast to implement

- [ ] **Option B**: TipTap via iframe (Feature parity, 5-7 days)
  - Rich text editing
  - Complex iframe communication
  - Reuse existing solution

- [ ] **Option C**: Native Dioxus editor (Future, 2-3 weeks)
  - Build from scratch
  - Full control
  - High effort

**My Recommendation**: Start with **Option A** for MVP, evaluate **Option B** if rich text is critical.

**Action Required**: Choose option and confirm priority.

---

**2. Graph Visualization**

**Input**: How to handle D3.js-based graph?

- [ ] **Option A**: Keep D3 via WASM interop
  - Reuse existing code
  - Interactive
  - JS dependency

- [ ] **Option B**: Port to native Rust (plotters or egui)
  - Pure Rust
  - More effort
  - Different UX

**My Recommendation**: **Option A** for faster migration, consider **Option B** for v2.0.

**Action Required**: Confirm approach.

---

**3. Tauri Integration**

**Input**: Pure Dioxus or Dioxus + Tauri hybrid?

- [ ] **Option A**: Pure Dioxus Desktop (Recommended)
  - Direct function calls (no IPC)
  - Better type safety
  - Simpler architecture

- [ ] **Option B**: Dioxus + Tauri hybrid
  - Keep existing Tauri commands
  - Use tauri-dioxus crate
  - More complex

**My Recommendation**: **Option A** - Refactor Tauri commands to direct function calls.

**Action Required**: Confirm approach.

---

**4. State Persistence**

**Input**: How to persist UI state (search history, theme, etc.)?

- [ ] **Option A**: Filesystem (JSON files)
  - Simple, Rust-native
  - Easy to implement

- [ ] **Option B**: Embed in Config/DeviceSettings
  - Centralized
  - Reuse existing persistence

- [ ] **Option C**: SQLite database
  - Queryable
  - Overkill for simple data

**My Recommendation**: **Option A** for UI state, **Option B** for app config.

**Action Required**: Confirm approach.

---

**5. CSS Strategy**

**Input**: Keep Bulma or migrate?

- [ ] **Option A**: Keep Bulma + Bulmaswatch
  - No change needed
  - 20+ themes preserved
  - Fastest migration

- [ ] **Option B**: Migrate to Tailwind
  - Modern utility-first CSS
  - More work

- [ ] **Option C**: Dioxus native styling
  - Rust-based styling
  - Experimental

**My Recommendation**: **Option A** - Keep Bulma for consistency and speed.

**Action Required**: Confirm approach.

---

### Feature Prioritization

**6. Must-Have Features for v1.0**

Please confirm which features are **must-have** for the initial release:

- [ ] Search with autocomplete ✅ (confirmed)
- [ ] Chat with context management ✅ (confirmed)
- [ ] Knowledge graph visualization
- [ ] Editor with slash commands
- [ ] Configuration wizard
- [ ] System tray with role switching ✅ (confirmed)
- [ ] Global shortcuts ✅ (confirmed)
- [ ] Theme switching
- [ ] Conversation persistence

**Action Required**: Mark must-haves and nice-to-haves.

---

**7. Testing Requirements**

**Input**: What level of testing is required?

- [ ] **Option A**: Basic smoke tests
  - Quick validation
  - Manual testing

- [ ] **Option B**: Comprehensive automated tests
  - Unit tests for all components
  - Integration tests for workflows
  - E2E tests for critical paths

- [ ] **Option C**: Minimal (prototype)
  - Focus on implementation speed
  - Test later

**My Recommendation**: **Option B** - Comprehensive tests for production quality.

**Action Required**: Confirm testing scope.

---

**8. Timeline**

**Input**: What is the target timeline?

- [ ] **8 weeks** (as per plan above)
- [ ] **Faster** (which phases to skip/defer?)
- [ ] **Slower/more thorough** (additional phases?)

**My Recommendation**: 8 weeks for full feature parity.

**Action Required**: Confirm timeline and any adjustments.

---

### Next Steps After Questions Answered

1. **Finalize design** based on answers
2. **Create detailed implementation plan** for Phase 0
3. **Set up development environment**
4. **Begin implementation** starting with Phase 0

---

**End of Design & Plan Document**
