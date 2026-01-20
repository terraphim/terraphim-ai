# GPUI Desktop Implementation: Detailed Documentation

## Overview

The GPUI Desktop application is the current development implementation using **Rust** and the **GPUI framework**. It provides a high-performance, native desktop interface for the Terraphim AI semantic search and knowledge graph system with advanced features including streaming chat, virtual scrolling, and real-time autocomplete.

---

## 1. Entity-Component Architecture

### Technology Stack

- **Framework**: Rust + GPUI
- **Async Runtime**: Tokio
- **State Management**: Entity<T> + Context<T>
- **Rendering**: GPU-accelerated native
- **Build System**: Cargo
- **Testing**: Tokio tests + Integration tests

### Directory Structure

```
crates/terraphim_desktop_gpui/
├── src/
│   ├── main.rs                           # Application entry
│   ├── app.rs                            # Main application controller
│   ├── state/
│   │   └── search.rs                     # Search state management
│   ├── views/
│   │   ├── chat/
│   │   │   ├── mod.rs                    # ChatView (main chat)
│   │   │   ├── context_edit_modal.rs     # ContextEditModal
│   │   │   ├── streaming.rs              # Streaming chat state
│   │   │   ├── virtual_scroll.rs         # Virtual scrolling
│   │   │   └── state.rs                  # Chat state
│   │   ├── search/
│   │   │   ├── mod.rs                    # SearchView
│   │   │   ├── input.rs                  # SearchInput
│   │   │   ├── results.rs                # SearchResults
│   │   │   ├── autocomplete.rs           # Autocomplete dropdown
│   │   │   └── term_chips.rs             # TermChip components
│   │   ├── markdown_modal.rs             # MarkdownModal
│   │   ├── role_selector.rs              # Role selection
│   │   └── tray_menu.rs                  # Tray menu
│   ├── components/
│   │   └── enhanced_chat.rs              # Reusable chat component
│   ├── platform/
│   │   ├── mod.rs                        # Platform abstraction
│   │   ├── tray.rs                       # System tray
│   │   └── hotkeys.rs                    # Global hotkeys
│   ├── models/
│   │   └── search.rs                     # Data models
│   └── actions.rs                        # Action definitions
└── Cargo.toml
```

### Core Application Controller

**File**: `crates/terraphim_desktop_gpui/src/app.rs`

The main application controller manages navigation, state, and platform integration.

```rust
use gpui::{AppContext, Context, Entity, Model, Window, WindowOptions};
use std::sync::Arc;
use tokio::sync::mpsc;

use crate::{
    platform::{SystemTray, SystemTrayEvent},
    state::search::SearchState,
    views::{
        chat::{ChatView, ChatViewEvent},
        search::SearchView,
        role_selector::RoleSelector,
        tray_menu::TrayMenu,
    },
    ConfigState,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AppView {
    Search,
    Chat,
    Editor,
}

pub struct TerraphimApp {
    // Current view
    current_view: AppView,

    // Main views
    search_view: Entity<SearchView>,
    chat_view: Entity<ChatView>,
    editor_view: Entity<()>,

    // Shared state
    config_state: ConfigState,

    // Platform integration
    system_tray: Option<SystemTray>,
    hotkey_receiver: Option<mpsc::Receiver<HotkeyAction>>,
}

impl TerraphimApp {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        // Initialize configuration state
        let config_state = ConfigState::new(cx);

        // Create main views
        let search_view = cx.new(|cx| {
            SearchView::new(window, cx, config_state.clone())
        });

        let chat_view = cx.new(|cx| {
            ChatView::new(window, cx)
                .with_config(config_state.clone())
        });

        let editor_view = cx.new(|_| ());

        // Initialize system tray
        let system_tray = SystemTray::new(window, cx);

        Self {
            current_view: AppView::Search,
            search_view,
            chat_view,
            editor_view,
            config_state,
            system_tray: Some(system_tray),
            hotkey_receiver: None,
        }
    }

    pub fn navigate_to(&mut self, view: AppView, cx: &mut Context<Self>) {
        if self.current_view != view {
            log::info!("Navigating from {:?} to {:?}", self.current_view, view);
            self.current_view = view;
            cx.notify();
        }
    }

    fn handle_tray_event(&mut self, event: SystemTrayEvent, cx: &mut Context<Self>) {
        match event {
            SystemTrayEvent::ShowWindow => {
                // Show/hide window
                window.set_visibility(true);
            }
            SystemTrayEvent::HideWindow => {
                window.set_visibility(false);
            }
            SystemTrayEvent::Quit => {
                cx.quit();
            }
            SystemTrayEvent::ChangeRole(role) => {
                // Update config state
                let mut config = self.config_state.config.lock().await;
                config.selected_role = role.clone();

                // Update role selector UI
                self.role_selector.update(cx, |selector, selector_cx| {
                    selector.set_selected_role(role.clone(), selector_cx);
                });

                // Update search view
                self.search_view.update(cx, |search_view, search_cx| {
                    search_view.update_role(role.to_string(), search_cx);
                });
            }
            SystemTrayEvent::NavigateTo(view) => {
                self.navigate_to(view, cx);
            }
        }
    }
}

impl Render for TerraphimApp {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        // Render current view
        match self.current_view {
            AppView::Search => {
                self.search_view.update(cx, |view, cx| {
                    view.render(window, cx);
                });
            }
            AppView::Chat => {
                self.chat_view.update(cx, |view, cx| {
                    view.render(window, cx);
                });
            }
            AppView::Editor => {
                // Render editor view
            }
        }

        // Render role selector (overlay)
        if let Some(system_tray) = &self.system_tray {
            system_tray.render(window, cx);
        }
    }
}

// Subscribe to chat events
fn setup_chat_subscriptions(cx: &mut Context<Self>) {
    let chat_view = cx.deps.get::<Entity<ChatView>>().unwrap().clone();
    let search_view = cx.deps.get::<Entity<SearchView>>().unwrap().clone();

    cx.subscribe(&chat_view, move |this, _chat, event: &ChatViewEvent, cx| {
        match event {
            ChatViewEvent::AddToContext { document } => {
                log::info!("App received AddToContext for: {}", document.title);

                // Auto-add to context (no modal)
                search_view.clone().update(cx, |chat, chat_cx| {
                    chat.add_document_as_context_direct(document.clone(), chat_cx);
                });

                // Navigate to chat
                this.navigate_to(AppView::Chat, cx);
            }
            ChatViewEvent::ContextUpdated => {
                // Handle context update
            }
        }
    });
}

// Subscribe to search events
fn setup_search_subscriptions(cx: &mut Context<Self>) {
    let search_view = cx.deps.get::<Entity<SearchView>>().unwrap().clone();
    let chat_view = cx.deps.get::<Entity<ChatView>>().unwrap().clone();

    cx.subscribe(&search_view, move |_app, _search, event: &SearchViewEvent, cx| {
        match event {
            SearchViewEvent::AddToContext { document } => {
                chat_view.clone().update(cx, |chat, chat_cx| {
                    chat.add_document_as_context_direct(document.clone(), chat_cx);
                });
            }
        }
    });
}
```

---

## 2. Async Patterns with Tokio Integration

### ChatView with Async Operations

**File**: `crates/terraphim_desktop_gpui/src/views/chat/mod.rs`

The ChatView demonstrates comprehensive async patterns with Tokio for message sending, context management, and LLM streaming.

```rust
use gpui::{Entity, Model, Subscription, ViewContext, Window, Context, Task};
use tokio::sync::mpsc;
use std::sync::Arc;
use futures::StreamExt;

use crate::{
    ConfigState,
    state::chat::ChatState,
    views::chat::context_edit_modal::{ContextEditModal, ContextEditModalEvent},
    terraphim_service::TerraphimContextManager,
};

pub struct ChatView {
    // Core state
    context_manager: Arc<TokioMutex<TerraphimContextManager>>,
    config_state: Option<ConfigState>,
    current_conversation_id: Option<ConversationId>,
    current_role: RoleName,

    // Chat data
    messages: Vec<ChatMessage>,
    context_items: Vec<ContextItem>,
    input: String,
    is_sending: bool,
    show_context_panel: bool,

    // UI components
    context_edit_modal: Entity<ContextEditModal>,

    // Subscriptions
    _subscriptions: Vec<Subscription>,
}

impl ChatView {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        // Initialize ContextManager with increased limits
        let context_config = ContextConfig {
            max_context_items: 100,
            max_context_length: 500_000,
            max_conversations_cache: 100,
            default_search_results_limit: 5,
            enable_auto_suggestions: true,
        };

        let context_manager = Arc::new(TokioMutex::new(
            TerraphimContextManager::new(context_config)
        ));

        // Create context edit modal
        let context_edit_modal = cx.new(|cx| {
            ContextEditModal::new(window, cx)
        });

        Self {
            context_manager,
            config_state: None,
            current_conversation_id: None,
            current_role: RoleName::from("Engineer"),
            messages: Vec::new(),
            context_items: Vec::new(),
            input: String::new(),
            is_sending: false,
            show_context_panel: true,
            context_edit_modal,
            _subscriptions: Vec::new(),
        }
    }

    pub fn with_config(mut self, config_state: ConfigState) -> Self {
        let actual_role = tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async {
                let selected = config_state.get_selected_role().await;

                // Check if role has rolegraph
                let role_key = terraphim_types::RoleName::from(selected.as_str());
                if config_state.roles.contains_key(&role_key) {
                    selected.to_string()
                } else {
                    // Fallback to first role with rolegraph
                    if let Some(first_role) = config_state.roles.keys().next() {
                        let mut config = config_state.config.lock().await;
                        config.selected_role = first_role.clone();
                        first_role.to_string()
                    } else {
                        selected.to_string()
                    }
                }
            })
        });

        self.current_role = RoleName::from(actual_role);
        self.config_state = Some(config_state);
        self
    }

    // Async message sending with LLM integration
    pub fn send_message(&mut self, text: String, cx: &mut Context<Self>) {
        if self.is_sending || text.trim().is_empty() {
            return;
        }

        self.is_sending = true;

        // Add user message immediately
        let user_message = ChatMessage {
            id: ulid::Ulid::new().to_string(),
            role: "user".to_string(),
            content: text.clone(),
            timestamp: chrono::Utc::now(),
            ..Default::default()
        };

        self.messages.push(user_message);
        cx.notify();

        // Prepare async context
        let context_manager = self.context_manager.clone();
        let conversation_id = self.current_conversation_id.clone();
        let role = self.current_role.clone();

        // Spawn async task for LLM call
        cx.spawn(|this, mut cx| async move {
            // Get context items for conversation
            let context_items = if let Some(conv_id) = conversation_id {
                let manager = context_manager.lock().await;
                manager.get_context_items(&conv_id).unwrap_or_default()
            } else {
                Vec::new()
            };

            // Build messages with context injection
            let mut messages = Vec::new();

            // Add context as system message
            if !context_items.is_empty() {
                let mut context_content = String::from("=== CONTEXT ===\n");
                for (idx, item) in context_items.iter().enumerate() {
                    context_content.push_str(&format!(
                        "{}. {}\n{}\n\n",
                        idx + 1,
                        item.title,
                        item.content
                    ));
                }
                context_content.push_str("=== END CONTEXT ===\n");

                messages.push(json!({
                    "role": "system",
                    "content": context_content
                }));
            }

            // Add user message
            messages.push(json!({
                "role": "user",
                "content": text
            }));

            // Call LLM with role configuration
            match llm::chat_completion(messages, role.clone()).await {
                Ok(response) => {
                    // Add assistant message
                    let assistant_message = ChatMessage {
                        id: ulid::Ulid::new().to_string(),
                        role: "assistant".to_string(),
                        content: response.content,
                        timestamp: chrono::Utc::now(),
                        model: Some(response.model),
                        ..Default::default()
                    };

                    this.update(|this, cx| {
                        this.messages.push(assistant_message);
                        this.is_sending = false;
                        cx.notify();
                    });
                }
                Err(e) => {
                    log::error!("LLM error: {}", e);

                    this.update(|this, cx| {
                        this.is_sending = false;
                        this.messages.push(ChatMessage {
                            id: ulid::Ulid::new().to_string(),
                            role: "system".to_string(),
                            content: format!("Error: {}", e),
                            timestamp: chrono::Utc::now(),
                            ..Default::default()
                        });
                        cx.notify();
                    });
                }
            }
        });
    }

    // Async context management
    pub fn add_context(&mut self, context_item: ContextItem, cx: &mut Context<Self>) {
        // Auto-create conversation if needed
        if self.current_conversation_id.is_none() {
            let title = format!("Context: {}", context_item.title);
            let role = self.current_role.clone();
            let manager = self.context_manager.clone();

            cx.spawn(async move |this, cx| {
                let mut mgr = manager.lock().await;
                let conversation_id = mgr.create_conversation(title, role).await.unwrap();

                mgr.add_context(&conversation_id, context_item.clone()).await.unwrap();

                this.update(cx, |this, cx| {
                    this.current_conversation_id = Some(conversation_id);
                    this.context_items.push(context_item);
                    cx.notify();
                });
            });
        } else if let Some(conv_id) = &self.current_conversation_id {
            let manager = self.context_manager.clone();
            let context_item_clone = context_item.clone();

            cx.spawn(async move |this, cx| {
                let mut mgr = manager.lock().await;
                mgr.add_context(conv_id, context_item_clone).await.unwrap();

                this.update(cx, |this, cx| {
                    this.context_items.push(context_item);
                    cx.notify();
                });
            });
        }
    }

    // Direct document addition (from search results)
    pub fn add_document_as_context_direct(&mut self, document: Document, cx: &mut Context<Self>) {
        let context_item = ContextItem {
            id: ulid::Ulid::new().to_string(),
            context_type: ContextType::Document,
            title: document.title.clone(),
            summary: document.description.clone(),
            content: document.body.clone(),
            metadata: {
                let mut meta = ahash::AHashMap::new();
                meta.insert("document_id".to_string(), document.id.clone());
                if !document.url.is_empty() {
                    meta.insert("url".to_string(), document.url.clone());
                }
                if let Some(tags) = &document.tags {
                    meta.insert("tags".to_string(), tags.join(", "));
                }
                meta
            },
            created_at: chrono::Utc::now(),
            relevance_score: document.rank.map(|r| r as f64),
        };

        self.add_context(context_item, cx);
    }

    // Update role and refresh
    pub fn update_role(&mut self, role: String, cx: &mut Context<Self>) {
        self.current_role = RoleName::from(role);
        cx.notify();
    }
}

// Modal event subscriptions
fn setup_modal_subscriptions(cx: &mut Context<Self>, modal: &Entity<ContextEditModal>) {
    let context_manager = cx.deps.get::<Arc<TokioMutex<TerraphimContextManager>>>().unwrap().clone();

    cx.subscribe(modal, move |this, _modal, event: &ContextEditModalEvent, cx| {
        match event {
            ContextEditModalEvent::Create(context_item) => {
                this.add_context(context_item.clone(), cx);
            }
            ContextEditModalEvent::Update(context_item) => {
                if let Some(conv_id) = &this.current_conversation_id {
                    let manager = context_manager.clone();
                    let context_id = context_item.id.clone();

                    cx.spawn(async move |_this, _cx| {
                        let mut mgr = manager.lock().await;
                        mgr.update_context(conv_id, &context_id, context_item.clone()).await.unwrap();
                    });
                }
            }
            ContextEditModalEvent::Delete(context_id) => {
                if let Some(conv_id) = &this.current_conversation_id {
                    let manager = context_manager.clone();

                    cx.spawn(async move |_this, _cx| {
                        let mut mgr = manager.lock().await;
                        mgr.delete_context(conv_id, context_id).await.unwrap();
                    });
                }
            }
            ContextEditModalEvent::Close => {
                this.show_context_modal = false;
            }
        }
    });
}

impl Render for ChatView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        // Render chat header
        div()
            .flex()
            .flex_col()
            .h_full()
            .bg(gpui::colors::BACKGROUND)
            .children([
                // Header
                self.render_header(window, cx),

                // Messages with virtual scrolling
                self.render_messages(window, cx),

                // Context panel
                if self.show_context_panel {
                    Some(self.render_context_panel(window, cx))
                } else {
                    None
                },

                // Input area
                self.render_input(window, cx),
            ])
            .abs()
            .size_full()
            .render(window, cx);

        // Render modal if open
        if let Some(modal) = &self.context_edit_modal {
            modal.update(cx, |modal, modal_cx| {
                modal.render(window, modal_cx);
            });
        }
    }

    fn render_header(&self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .flex()
            .items_center()
            .justify_between()
            .px_6()
            .py_4()
            .border_b()
            .border_color(gpui::colors::BORDER)
            .children([
                h1()
                    .text_xl()
                    .font_semibold()
                    .text(gpui::colors::TEXT)
                    .child("Chat"),
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .children([
                        // Context panel toggle
                        button("Context")
                            .on_click(|_, cx| {
                                // Toggle context panel
                                cx.notify();
                            }),
                        // Role indicator
                        div()
                            .px_3()
                            .py_1()
                            .bg(gpui::colors::PRIMARY)
                            .rounded_md()
                            .child(format!("{}", self.current_role)),
                    ]),
            ])
    }

    fn render_messages(&self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .flex_1()
            .overflow_hidden()
            .children(
                self.messages
                    .iter()
                    .enumerate()
                    .map(|(idx, message)| {
                        self.render_message(idx, message, window, cx)
                    })
            )
    }

    fn render_message(&self, idx: usize, message: &ChatMessage, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let is_user = message.role == "user";
        let is_assistant = message.role == "assistant";

        div()
            .flex()
            .gap_3()
            .p_4()
            .children([
                // Avatar
                div()
                    .w_10()
                    .h_10()
                    .rounded_full()
                    .bg(if is_user {
                        gpui::colors::ACCENT
                    } else if is_assistant {
                        gpui::colors::PRIMARY
                    } else {
                        gpui::colors::WARNING
                    })
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(
                        match message.role.as_str() {
                            "user" => "👤",
                            "assistant" => "🤖",
                            _ => "ℹ️",
                        }
                    ),

                // Message content
                div()
                    .flex_1()
                    .bg(gpui::colors::SURFACE)
                    .rounded_lg()
                    .p_4()
                    .border()
                    .border_color(gpui::colors::BORDER)
                    .child(if is_assistant {
                        // Render markdown for assistant messages
                        markdown::render(&message.content)
                    } else {
                        gpui::div().child(&message.content)
                    })
                    .child(
                        // Timestamp
                        div()
                            .mt_2()
                            .text_sm()
                            .text_color(gpui::colors::TEXT_SECONDARY)
                            .child(format_time(message.timestamp))
                    ),
            ])
    }

    fn render_input(&self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .border_t()
            .border_color(gpui::colors::BORDER)
            .p_4()
            .children([
                div()
                    .flex()
                    .gap_2()
                    .children([
                        // Input field
                        div()
                            .flex_1()
                            .child(
                                text_input()
                                    .text(&self.input)
                                    .placeholder("Type your message...")
                                    .on_change(|text, this, cx| {
                                        this.input = text;
                                        cx.notify();
                                    })
                                    .on_key_down(|event, this, cx| {
                                        if event.key_code == gpui::KeyCode::Enter {
                                            let text = this.input.clone();
                                            this.input.clear();
                                            this.send_message(text, cx);
                                        }
                                    })
                            ),

                        // Send button
                        button("Send")
                            .disabled(self.is_sending || self.input.trim().is_empty())
                            .on_click(|_, this, cx| {
                                let text = this.input.clone();
                                this.input.clear();
                                this.send_message(text, cx);
                            })
                            .child(if self.is_sending {
                                "Sending..."
                            } else {
                                "Send"
                            }),
                    ]),

                // Typing indicator
                if self.is_sending {
                    Some(
                        div()
                            .mt_2()
                            .text_sm()
                            .text_color(gpui::colors::TEXT_SECONDARY)
                            .child("Assistant is typing...")
                    )
                } else {
                    None
                },
            ])
    }
}
```

---

## 3. Modal System Implementation

### ContextEditModal with EventEmitter

**File**: `crates/terraphim_desktop_gpui/src/views/chat/context_edit_modal.rs`

The ContextEditModal demonstrates GPUI's modal system with EventEmitter pattern for parent-child communication.

```rust
use gpui::{Entity, Model, Subscription, ViewContext, Window, Context, IntoElement, Element};
use tokio::sync::mpsc;
use std::sync::Arc;
use ahash::AHashMap;

use crate::terraphim_types::{ContextItem, ContextType};

pub struct ContextEditModal {
    // Modal state
    is_open: bool,
    mode: ContextEditMode,
    editing_context: Option<ContextItem>,

    // Form fields
    title_state: Option<Entity<InputState>>,
    summary_state: Option<Entity<InputState>>,
    content_state: Option<Entity<InputState>>,
    context_type: ContextType,

    // Event handling
    event_sender: mpsc::UnboundedSender<ContextEditModalEvent>,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ContextEditMode {
    Create,
    Edit,
}

#[derive(Clone, Debug)]
pub enum ContextEditModalEvent {
    Create(ContextItem),
    Update(ContextItem),
    Delete(String), // context_id
    Close,
}

// EventEmitter trait
pub trait EventEmitter<T> {
    fn emit(&self, event: T);
    fn subscribe<F>(&self, handler: F) -> Subscription
    where
        F: Fn(&mut Context<Self>, &T) + 'static;
}

impl EventEmitter<ContextEditModalEvent> for Entity<ContextEditModal> {
    fn emit(&self, event: ContextEditModalEvent) {
        // Implementation handled by subscription system
    }

    fn subscribe<F>(&self, handler: F) -> Subscription
    where
        F: Fn(&mut Context<Self>, &ContextEditModalEvent) + 'static,
    {
        // Implementation handled by GPUI
    }
}

impl ContextEditModal {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let (event_sender, _) = mpsc::unbounded_channel();

        Self {
            is_open: false,
            mode: ContextEditMode::Create,
            editing_context: None,
            title_state: None,
            summary_state: None,
            content_state: None,
            context_type: ContextType::Document,
            event_sender,
        }
    }

    // Open modal in create mode
    pub fn open_create(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.mode = ContextEditMode::Create;
        self.editing_context = None;
        self.context_type = ContextType::Document;

        // Initialize form fields
        self.title_state = Some(cx.new(|cx| {
            InputState::new(window, cx)
        }));

        self.summary_state = Some(cx.new(|cx| {
            InputState::new(window, cx)
        }));

        self.content_state = Some(cx.new(|cx| {
            InputState::new(window, cx)
        }));

        self.is_open = true;
        cx.notify();
    }

    // Open modal in edit mode with pre-populated data
    pub fn open_edit(&mut self, context_item: ContextItem, window: &mut Window, cx: &mut Context<Self>) {
        self.mode = ContextEditMode::Edit;
        self.editing_context = Some(context_item.clone());
        self.context_type = context_item.context_type.clone();

        // Initialize form fields
        self.title_state = Some(cx.new(|cx| {
            InputState::new(window, cx)
        }));

        self.summary_state = Some(cx.new(|cx| {
            InputState::new(window, cx)
        }));

        self.content_state = Some(cx.new(|cx| {
            InputState::new(window, cx)
        }));

        // Populate form fields with existing data
        if let Some(title_state) = &self.title_state {
            let title_value = context_item.title.replace('\n', " ").replace('\r', "");
            title_state.update(cx, |input, input_cx| {
                input.set_value(
                    gpui::SharedString::from(title_value),
                    window,
                    input_cx
                );
            });
        }

        if let Some(summary_state) = &self.summary_state {
            if let Some(summary) = &context_item.summary {
                let summary_value = summary.replace('\n', " ").replace('\r', "");
                summary_state.update(cx, |input, input_cx| {
                    input.set_value(
                        gpui::SharedString::from(summary_value),
                        window,
                        input_cx
                    );
                });
            }
        }

        if let Some(content_state) = &self.content_state {
            let content_value = context_item.content.replace('\n', " ").replace('\r', "");
            content_state.update(cx, |input, input_cx| {
                input.set_value(
                    gpui::SharedString::from(content_value),
                    window,
                    input_cx
                );
            });
        }

        self.is_open = true;
        cx.notify();
    }

    // Open modal with document data (for adding from search)
    pub fn open_with_document(&mut self, document: Document, window: &mut Window, cx: &mut Context<Self>) {
        self.mode = ContextEditMode::Create;

        // Create context item from document
        let context_item = ContextItem {
            id: document.id.clone(),
            context_type: ContextType::Document,
            title: document.title.clone(),
            summary: document.description.clone(),
            content: document.body.clone(),
            metadata: {
                let mut meta = AHashMap::new();
                if !document.url.is_empty() {
                    meta.insert("url".to_string(), document.url.clone());
                }
                if let Some(tags) = &document.tags {
                    meta.insert("tags".to_string(), tags.join(", "));
                }
                meta
            },
            created_at: chrono::Utc::now(),
            relevance_score: document.rank.map(|r| r as f64),
        };

        self.open_edit(context_item, window, cx);
    }

    // Handle save/create
    fn handle_save(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let title = self.get_title_value();
        let summary = self.get_summary_value();
        let content = self.get_content_value();

        // Validation
        if title.trim().is_empty() || content.trim().is_empty() {
            // Show validation error
            return;
        }

        // Create context item
        let context_item = ContextItem {
            id: self.editing_context
                .as_ref()
                .map(|ctx| ctx.id.clone())
                .unwrap_or_else(|| ulid::Ulid::new().to_string()),
            context_type: self.context_type.clone(),
            title: title.clone(),
            summary: Some(summary.clone()),
            content: content.clone(),
            metadata: self.editing_context
                .as_ref()
                .map(|ctx| ctx.metadata.clone())
                .unwrap_or_default(),
            created_at: self.editing_context
                .as_ref()
                .map(|ctx| ctx.created_at)
                .unwrap_or_else(|| chrono::Utc::now()),
            relevance_score: self.editing_context
                .as_ref()
                .and_then(|ctx| ctx.relevance_score),
        };

        // Emit appropriate event
        match self.mode {
            ContextEditMode::Create => {
                self.event_sender
                    .send(ContextEditModalEvent::Create(context_item))
                    .ok();
            }
            ContextEditMode::Edit => {
                self.event_sender
                    .send(ContextEditModalEvent::Update(context_item))
                    .ok();
            }
        }

        self.close();
    }

    // Handle delete
    fn handle_delete(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(context_id) = self.editing_context.as_ref().map(|ctx| ctx.id.clone()) {
            self.event_sender
                .send(ContextEditModalEvent::Delete(context_id))
                .ok();
        }
        self.close();
    }

    // Close modal
    fn close(&mut self) {
        self.is_open = false;
        self.title_state = None;
        self.summary_state = None;
        self.content_state = None;
        self.editing_context = None;
        self.event_sender
            .send(ContextEditModalEvent::Close)
            .ok();
    }

    // Helper methods to get form values
    fn get_title_value(&self) -> String {
        if let Some(title_state) = &self.title_state {
            // Get value from input state
            // Implementation depends on InputState structure
            String::new()
        } else {
            String::new()
        }
    }

    fn get_summary_value(&self) -> String {
        if let Some(summary_state) = &self.summary_state {
            // Get value from input state
            String::new()
        } else {
            String::new()
        }
    }

    fn get_content_value(&self) -> String {
        if let Some(content_state) = &self.content_state {
            // Get value from input state
            String::new()
        } else {
            String::new()
        }
    }
}

impl Render for ContextEditModal {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if !self.is_open {
            return gpui::div();
        }

        // Modal backdrop
        div()
            .fixed()
            .inset_0()
            .bg(gpui::colors::BACKGROUND.alpha(0.8))
            .on_click(|_, this, cx| {
                this.close();
            })
            .children([
                // Modal card
                div()
                    .absolute()
                    .top_1over2()
                    .left_1over2()
                    .transform("translate-x-1/2", "-translate-y-1/2")
                    .w_96()
                    .bg(gpui::colors::SURFACE)
                    .rounded_lg()
                    .shadow_2xl()
                    .border()
                    .border_color(gpui::colors::BORDER)
                    .on_click(|event, _, _| {
                        event.stop_propagation();
                    })
                    .children([
                        // Header
                        div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .px_6()
                            .py_4()
                            .border_b()
                            .border_color(gpui::colors::BORDER)
                            .children([
                                h2()
                                    .text_lg()
                                    .font_semibold()
                                    .text(gpui::colors::TEXT)
                                    .child(match self.mode {
                                        ContextEditMode::Create => "Add Context",
                                        ContextEditMode::Edit => "Edit Context",
                                    }),
                                button("×")
                                    .text_2xl()
                                    .text_color(gpui::colors::TEXT_SECONDARY)
                                    .on_click(|_, this, cx| {
                                        this.close();
                                    }),
                            ]),

                        // Body
                        div()
                            .px_6()
                            .py_4()
                            .space_y_4()
                            .children([
                                // Title field
                                div()
                                    .children([
                                        label()
                                            .text_sm()
                                            .font_medium()
                                            .text(gpui::colors::TEXT)
                                            .child("Title"),
                                        div().mt_1().child(
                                            // Input field implementation
                                            self.title_state
                                                .as_ref()
                                                .map(|state| state.render(window, cx))
                                                .unwrap_or_else(|| gpui::div())
                                        ),
                                    ]),

                                // Summary field
                                div()
                                    .children([
                                        label()
                                            .text_sm()
                                            .font_medium()
                                            .text(gpui::colors::TEXT)
                                            .child("Summary (optional)"),
                                        div().mt_1().child(
                                            self.summary_state
                                                .as_ref()
                                                .map(|state| state.render(window, cx))
                                                .unwrap_or_else(|| gpui::div())
                                        ),
                                    ]),

                                // Content field
                                div()
                                    .flex_1()
                                    .children([
                                        label()
                                            .text_sm()
                                            .font_medium()
                                            .text(gpui::colors::TEXT)
                                            .child("Content"),
                                        div().mt_1().child(
                                            self.content_state
                                                .as_ref()
                                                .map(|state| state.render(window, cx))
                                                .unwrap_or_else(|| gpui::div())
                                        ),
                                    ]),

                                // Type selector
                                div()
                                    .children([
                                        label()
                                            .text_sm()
                                            .font_medium()
                                            .text(gpui::colors::TEXT)
                                            .child("Type"),
                                        div().mt_1().child(
                                            select()
                                                .value(&self.context_type)
                                                .on_change(|value, this, cx| {
                                                    this.context_type = value;
                                                    cx.notify();
                                                })
                                                .children([
                                                    option(ContextType::Document, "Document"),
                                                    option(ContextType::Url, "URL"),
                                                    option(ContextType::Note, "Note"),
                                                ])
                                        ),
                                    ]),
                            ]),

                        // Footer
                        div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .px_6()
                            .py_4()
                            .border_t()
                            .border_color(gpui::colors::BORDER)
                            .children([
                                // Delete button (edit mode only)
                                if self.mode == ContextEditMode::Edit {
                                    Some(
                                        button("Delete")
                                            .text_color(gpui::colors::DANGER)
                                            .on_click(|_, this, cx| {
                                                this.handle_delete(window, cx);
                                            })
                                    )
                                } else {
                                    None
                                },

                                // Spacer
                                div().flex_1(),

                                // Cancel button
                                button("Cancel")
                                    .on_click(|_, this, cx| {
                                        this.close();
                                    }),

                                // Save button
                                button("Save")
                                    .bg(gpui::colors::PRIMARY)
                                    .text_color(gpui::colors::WHITE)
                                    .on_click(|_, this, cx| {
                                        this.handle_save(window, cx);
                                    }),
                            ]),
                    ]),
            ])
    }
}
```

### MarkdownModal with Advanced Features

**File**: `crates/terraphim_desktop_gpui/src/views/markdown_modal.rs`

A reusable modal component for rendering markdown content with search and navigation.

```rust
use gpui::{Entity, Model, Subscription, ViewContext, Window, Context, IntoElement, Element};
use pulldown_cmark::{Parser, Options, html};

pub struct MarkdownModal {
    is_open: bool,
    content: String,
    rendered_html: String,
    search_query: String,
    search_results: Vec<SearchResult>,
    current_section: Option<String>,
    toc_entries: Vec<TocEntry>,
    options: MarkdownModalOptions,
}

pub struct MarkdownModalOptions {
    pub title: Option<String>,
    pub show_search: bool,
    pub show_toc: bool,
    pub max_width: Option<f32>,
    pub max_height: Option<f32>,
    pub enable_keyboard_shortcuts: bool,
}

impl Default for MarkdownModalOptions {
    fn default() -> Self {
        Self {
            title: None,
            show_search: true,
            show_toc: true,
            max_width: Some(800.0),
            max_height: Some(600.0),
            enable_keyboard_shortcuts: true,
        }
    }
}

struct TocEntry {
    level: u32,
    title: String,
    id: String,
}

struct SearchResult {
    line_number: usize,
    content: String,
}

impl MarkdownModal {
    pub fn new() -> Self {
        let options = MarkdownModalOptions::default();

        Self {
            is_open: false,
            content: String::new(),
            rendered_html: String::new(),
            search_query: String::new(),
            search_results: Vec::new(),
            current_section: None,
            toc_entries: Vec::new(),
            options,
        }
    }

    pub fn open(&mut self, content: String, options: MarkdownModalOptions, cx: &mut Context<Self>) {
        self.content = content;
        self.options = options;
        self.parse_and_render();
        self.extract_toc();
        self.is_open = true;
        cx.notify();

        // Register keyboard shortcuts
        if self.options.enable_keyboard_shortcuts {
            self.register_shortcuts(cx);
        }
    }

    fn parse_and_render(&mut self) {
        let mut options = Options::empty();
        options.insert(Options::ENABLE_TABLES);
        options.insert(Options::ENABLE_FOOTNOTES);
        options.insert(Options::ENABLE_STRIKETHROUGH);
        options.insert(Options::ENABLE_TASKLISTS);

        let parser = Parser::new_ext(&self.content, options);
        self.rendered_html = html::push_html(String::new(), parser);
    }

    fn extract_toc(&mut self) {
        self.toc_entries.clear();

        for line in self.content.lines() {
            if let Some(stripped) = line.strip_prefix('#') {
                let level = line.len() - stripped.len();
                if level <= 6 {
                    let title = stripped.trim().to_string();
                    let id = title
                        .to_lowercase()
                        .replace(' ', "-")
                        .replace(|c: char| !c.is_alphanumeric(), "");
                    self.toc_entries.push(TocEntry { level, title, id });
                }
            }
        }
    }

    fn handle_search(&mut self, query: String) {
        self.search_query = query;

        if query.trim().is_empty() {
            self.search_results.clear();
            return;
        }

        self.search_results = self.content
            .lines()
            .enumerate()
            .filter(|(_, line)| line.to_lowercase().contains(&query.to_lowercase()))
            .map(|(line_num, content)| SearchResult {
                line_number: line_num + 1,
                content: content.to_string(),
            })
            .collect();
    }

    fn navigate_to_section(&mut self, section_id: String) {
        self.current_section = Some(section_id);
        // Implementation for smooth scrolling to section
    }

    fn handle_key_down(&mut self, event: &KeyDownEvent) {
        if !self.options.enable_keyboard_shortcuts {
            return;
        }

        match event.key_code {
            gpui::KeyCode::Escape => {
                self.close();
            }
            gpui::KeyCode::KeyF => {
                if event.modifiers.command_or_control {
                    // Focus search input
                }
            }
            gpui::KeyCode::KeyN => {
                if !self.search_results.is_empty() {
                    // Navigate to next search result
                }
            }
            gpui::KeyCode::KeyP => {
                if !self.search_results.is_empty() {
                    // Navigate to previous search result
                }
            }
            _ => {}
        }
    }
}

impl Render for MarkdownModal {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if !self.is_open {
            return gpui::div();
        }

        div()
            .fixed()
            .inset_0()
            .bg(gpui::colors::BACKGROUND.alpha(0.8))
            .on_key_down(|event, this, cx| {
                this.handle_key_down(event);
            })
            .children([
                // Modal container
                div()
                    .absolute()
                    .top_1over2()
                    .left_1over2()
                    .transform("translate-x-1/2", "-translate-y-1/2")
                    .w(self.options.max_width.unwrap_or(800.0))
                    .h(self.options.max_height.unwrap_or(600.0))
                    .bg(gpui::colors::SURFACE)
                    .rounded_lg()
                    .shadow_2xl()
                    .border()
                    .border_color(gpui::colors::BORDER)
                    .flex()
                    .flex_col()
                    .children([
                        // Header
                        self.render_header(window, cx),

                        // Content area
                        div()
                            .flex_1()
                            .flex()
                            .overflow_hidden()
                            .children([
                                // Table of contents
                                if self.options.show_toc && !self.toc_entries.is_empty() {
                                    Some(self.render_toc(window, cx))
                                } else {
                                    None
                                },

                                // Main content
                                div()
                                    .flex_1()
                                    .overflow_y_auto()
                                    .p_6
                                    .child(html::render(&self.rendered_html)),
                            ]),
                    ])
                    .render(window, cx),
            ])
    }

    fn render_header(&self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .flex()
            .items_center()
            .justify_between()
            .px_6()
            .py_4()
            .border_b()
            .border_color(gpui::colors::BORDER)
            .children([
                // Title
                if let Some(title) = &self.options.title {
                    h3()
                        .text_lg()
                        .font_semibold()
                        .text(gpui::colors::TEXT)
                        .child(title)
                } else {
                    gpui::div()
                },

                // Search and controls
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .children([
                        // Search input
                        if self.options.show_search {
                            Some(
                                div()
                                    .w_64()
                                    .child(
                                        text_input()
                                            .placeholder("Search...")
                                            .on_change(|query, this, cx| {
                                                this.handle_search(query);
                                            })
                                    )
                            )
                        } else {
                            None
                        },

                        // Close button
                        button("×")
                            .text_2xl()
                            .text_color(gpui::colors::TEXT_SECONDARY)
                            .on_click(|_, this, cx| {
                                this.close();
                            }),
                    ]),
            ])
    }

    fn render_toc(&self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .w_48()
            .border_r()
            .border_color(gpui::colors::BORDER)
            .p_4
            .overflow_y_auto()
            .children([
                h4()
                    .text_sm()
                    .font_semibold()
                    .text(gpui::colors::TEXT)
                    .mb_3
                    .child("Table of Contents"),

                // TOC entries
                gpui::div()
                    .space_y_1
                    .children(
                        self.toc_entries
                            .iter()
                            .map(|entry| {
                                button(&entry.title)
                                    .text_left()
                                    .text_sm()
                                    .text_color(
                                        if Some(entry.id.clone()) == self.current_section {
                                            gpui::colors::PRIMARY
                                        } else {
                                            gpui::colors::TEXT
                                        }
                                    )
                                    .ml((entry.level * 4) as f32)
                                    .on_click(|_, this, cx| {
                                        this.navigate_to_section(entry.id.clone());
                                    })
                            })
                    ),
            ])
    }
}
```

---

## 4. Context Management with TerraphimContextManager

**File**: `crates/terraphim_service/src/context.rs`

The ContextManager provides comprehensive context management with LRU caching and limit enforcement.

```rust
use ahash::AHashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use chrono::{DateTime, Utc};
use ulid::Ulid;

use crate::{
    error::ServiceResult,
    types::{ContextItem, ContextItemData, ContextType, Conversation, ConversationId, RoleName},
};

pub struct ContextConfig {
    pub max_context_items: usize,        // Default: 50
    pub max_context_length: usize,       // Default: 100,000 chars
    pub max_conversations_cache: usize,  // Default: 100
    pub default_search_results_limit: usize,  // Default: 5
    pub enable_auto_suggestions: bool,   // Default: true
}

impl Default for ContextConfig {
    fn default() -> Self {
        Self {
            max_context_items: 50,
            max_context_length: 100_000,
            max_conversations_cache: 100,
            default_search_results_limit: 5,
            enable_auto_suggestions: true,
        }
    }
}

pub struct AddContextResult {
    pub warning: Option<String>,
}

pub struct TerraphimContextManager {
    config: ContextConfig,
    conversations_cache: AHashMap<ConversationId, Arc<Conversation>>,
    created_order: Vec<ConversationId>, // For LRU eviction
}

impl TerraphimContextManager {
    pub fn new(config: ContextConfig) -> Self {
        Self {
            config,
            conversations_cache: AHashMap::new(),
            created_order: Vec::new(),
        }
    }

    // Create a new conversation
    pub async fn create_conversation(
        &mut self,
        title: String,
        role: RoleName,
    ) -> ServiceResult<ConversationId> {
        let conversation = Conversation {
            id: ConversationId::from(Ulid::new().to_string()),
            title,
            messages: Vec::new(),
            global_context: Vec::new(),
            role,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        let id = conversation.id.clone();

        self.conversations_cache
            .insert(id.clone(), Arc::new(conversation));
        self.created_order.push(id.clone());

        self.clean_cache();

        Ok(id)
    }

    // Get conversation by ID
    pub async fn get_conversation(
        &self,
        conversation_id: &ConversationId,
    ) -> ServiceResult<Arc<Conversation>> {
        self.conversations_cache
            .get(conversation_id)
            .cloned()
            .ok_or_else(|| ServiceError::ConversationNotFound {
                id: conversation_id.clone(),
            })
    }

    // List all conversations
    pub async fn list_conversations(
        &self,
        limit: Option<usize>,
    ) -> ServiceResult<Vec<Conversation>> {
        let mut conversations: Vec<_> = self
            .conversations_cache
            .values()
            .cloned()
            .map(|arc| (*arc).clone())
            .collect();

        // Sort by updated_at descending
        conversations.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));

        if let Some(limit) = limit {
            conversations.truncate(limit);
        }

        Ok(conversations)
    }

    // Add message to conversation
    pub async fn add_message(
        &mut self,
        conversation_id: &ConversationId,
        message: ChatMessage,
    ) -> ServiceResult<()> {
        if let Some(conversation) = self.conversations_cache.get_mut(conversation_id) {
            Arc::get_mut(conversation)
                .unwrap()
                .messages
                .push(message);
            Arc::get_mut(conversation)
                .unwrap()
                .updated_at = Utc::now();

            // Update LRU order
            self.update_access_order(conversation_id);

            Ok(())
        } else {
            Err(ServiceError::ConversationNotFound {
                id: conversation_id.clone(),
            })
        }
    }

    // Add context item to conversation
    pub async fn add_context_item(
        &mut self,
        conversation_id: &ConversationId,
        context_data: ContextItemData,
    ) -> ServiceResult<AddContextResult> {
        let mut warning = None;

        if let Some(conversation) = self.conversations_cache.get_mut(conversation_id) {
            let context_item = ContextItem::from(context_data);

            // Check soft limits
            let current_context_count = conversation.global_context.len();
            let current_message_count = conversation.messages.len();
            let total_items = current_context_count + current_message_count;

            if total_items >= self.config.max_context_items {
                warning = Some(format!(
                    "Context limit exceeded. Total items: {}/{}",
                    total_items, self.config.max_context_items
                ));
            }

            // Check content length
            let content_length = context_item.content.len() + context_item.title.len();
            if content_length > self.config.max_context_length {
                warning = Some(format!(
                    "Context item too large: {} chars (max: {})",
                    content_length, self.config.max_context_length
                ));
            }

            // Add context item
            Arc::get_mut(conversation)
                .unwrap()
                .global_context
                .push(context_item);
            Arc::get_mut(conversation)
                .unwrap()
                .updated_at = Utc::now();

            // Update LRU order
            self.update_access_order(conversation_id);

            Ok(AddContextResult { warning })
        } else {
            Err(ServiceError::ConversationNotFound {
                id: conversation_id.clone(),
            })
        }
    }

    // Update context item
    pub async fn update_context(
        &mut self,
        conversation_id: &ConversationId,
        context_id: &str,
        updates: ContextItemData,
    ) -> ServiceResult<()> {
        if let Some(conversation) = self.conversations_cache.get_mut(conversation_id) {
            let arc_mut = Arc::get_mut(conversation).unwrap();

            if let Some(context_item) = arc_mut
                .global_context
                .iter_mut()
                .find(|ctx| ctx.id == context_id)
            {
                // Update fields
                if !updates.title.is_empty() {
                    context_item.title = updates.title;
                }
                if let Some(summary) = updates.summary {
                    context_item.summary = Some(summary);
                }
                if !updates.content.is_empty() {
                    context_item.content = updates.content;
                }
                context_item.context_type = updates.context_type;

                arc_mut.updated_at = Utc::now();
                self.update_access_order(conversation_id);

                Ok(())
            } else {
                Err(ServiceError::ContextItemNotFound {
                    id: context_id.to_string(),
                })
            }
        } else {
            Err(ServiceError::ConversationNotFound {
                id: conversation_id.clone(),
            })
        }
    }

    // Delete context item
    pub async fn delete_context(
        &mut self,
        conversation_id: &ConversationId,
        context_id: &str,
    ) -> ServiceResult<()> {
        if let Some(conversation) = self.conversations_cache.get_mut(conversation_id) {
            let arc_mut = Arc::get_mut(conversation).unwrap();

            let original_len = arc_mut.global_context.len();
            arc_mut.global_context.retain(|ctx| ctx.id != context_id);

            if arc_mut.global_context.len() == original_len {
                return Err(ServiceError::ContextItemNotFound {
                    id: context_id.to_string(),
                });
            }

            arc_mut.updated_at = Utc::now();
            self.update_access_order(conversation_id);

            Ok(())
        } else {
            Err(ServiceError::ConversationNotFound {
                id: conversation_id.clone(),
            })
        }
    }

    // Get context items for conversation
    pub async fn get_context_items(
        &self,
        conversation_id: &ConversationId,
    ) -> ServiceResult<Vec<ContextItem>> {
        if let Some(conversation) = self.conversations_cache.get(conversation_id) {
            Ok(conversation.global_context.clone())
        } else {
            Err(ServiceError::ConversationNotFound {
                id: conversation_id.clone(),
            })
        }
    }

    // Create context item from search result
    pub fn create_search_context(
        &self,
        query: String,
        results: Vec<SearchResult>,
    ) -> ContextItem {
        ContextItem {
            id: Ulid::new().to_string(),
            context_type: ContextType::Document,
            title: format!("Search Results for: {}", query),
            summary: Some(format!("{} results found", results.len())),
            content: results
                .into_iter()
                .map(|r| format!("{}\n{}\n", r.title, r.body))
                .collect::<Vec<_>>()
                .join("\n---\n"),
            metadata: {
                let mut meta = AHashMap::new();
                meta.insert("query".to_string(), query);
                meta.insert("result_count".to_string(), results.len().to_string());
                meta
            },
            created_at: Utc::now(),
            relevance_score: None,
        }
    }

    // Create context item from document
    pub fn create_document_context(&self, document: Document) -> ContextItem {
        ContextItem {
            id: document.id.clone(),
            context_type: ContextType::Document,
            title: document.title.clone(),
            summary: document.description.clone(),
            content: document.body.clone(),
            metadata: {
                let mut meta = AHashMap::new();
                if !document.url.is_empty() {
                    meta.insert("url".to_string(), document.url.clone());
                }
                if let Some(tags) = &document.tags {
                    meta.insert("tags".to_string(), tags.join(", "));
                }
                meta
            },
            created_at: Utc::now(),
            relevance_score: document.rank.map(|r| r as f64),
        }
    }

    // Clean cache using LRU
    fn clean_cache(&mut self) {
        while self.conversations_cache.len() > self.config.max_conversations_cache {
            if let Some(oldest_id) = self.created_order.first().cloned() {
                self.conversations_cache.remove(&oldest_id);
                self.created_order.remove(0);
            }
        }
    }

    // Update access order for LRU
    fn update_access_order(&mut self, conversation_id: &ConversationId) {
        if let Some(pos) = self
            .created_order
            .iter()
            .position(|id| id == conversation_id)
        {
            let id = self.created_order.remove(pos);
            self.created_order.push(id);
        }
    }

    // Get cache statistics
    pub fn get_cache_stats(&self) -> CacheStats {
        CacheStats {
            total_conversations: self.conversations_cache.len(),
            max_conversations: self.config.max_conversations_cache,
            total_context_items: self
                .conversations_cache
                .values()
                .map(|conv| conv.global_context.len())
                .sum(),
            total_messages: self
                .conversations_cache
                .values()
                .map(|conv| conv.messages.len())
                .sum(),
        }
    }
}

pub struct CacheStats {
    pub total_conversations: usize,
    pub max_conversations: usize,
    pub total_context_items: usize,
    pub total_messages: usize,
}
```

---

## 5. Search State Management

**File**: `crates/terraphim_desktop_gpui/src/state/search.rs`

The SearchState demonstrates entity-based state management with autocomplete and role integration.

```rust
use gpui::{Entity, Model, Subscription, ViewContext, Window, Context, IntoElement, Element};
use tokio::sync::mpsc;
use std::sync::Arc;
use ahash::AHashMap;

use crate::{
    ConfigState,
    views::search::{
        SearchInput, SearchResults, AutocompleteDropdown, TermChip,
        SearchInputEvent, SearchResultsEvent, AutocompleteEvent,
    },
    models::search::{ResultItemViewModel, TermChipSet, ChipOperator},
    terraphim_service::TerraphimService,
};

pub struct SearchState {
    // Core state
    config_state: Option<ConfigState>,
    query: String,
    parsed_query: String,
    results: Vec<ResultItemViewModel>,
    term_chips: TermChipSet,

    // UI state
    loading: bool,
    error: Option<String>,
    current_role: String,

    // Autocomplete
    autocomplete_suggestions: Vec<AutocompleteSuggestion>,
    autocomplete_loading: bool,
    show_autocomplete: bool,
    selected_suggestion_index: usize,

    // Pagination
    current_page: usize,
    page_size: usize,
    has_more: bool,
}

impl SearchState {
    pub fn new(cx: &mut Context<Self>) -> Self {
        Self {
            config_state: None,
            query: String::new(),
            parsed_query: String::new(),
            results: Vec::new(),
            term_chips: TermChipSet::new(),
            loading: false,
            error: None,
            current_role: "Engineer".to_string(),
            autocomplete_suggestions: Vec::new(),
            autocomplete_loading: false,
            show_autocomplete: false,
            selected_suggestion_index: -1,
            current_page: 0,
            page_size: 10,
            has_more: false,
        }
    }

    pub fn with_config(mut self, config_state: ConfigState) -> Self {
        let actual_role = tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async {
                let selected = config_state.get_selected_role().await;

                // Check if selected role has a rolegraph
                let role_key = terraphim_types::RoleName::from(selected.as_str());
                if config_state.roles.contains_key(&role_key) {
                    selected.to_string()
                } else {
                    // Fallback to first role with rolegraph
                    if let Some(first_role) = config_state.roles.keys().next() {
                        let mut config = config_state.config.lock().await;
                        config.selected_role = first_role.clone();
                        first_role.to_string()
                    } else {
                        selected.to_string()
                    }
                }
            })
        });

        self.current_role = actual_role;
        self.config_state = Some(config_state);
        self
    }

    // Update query and trigger autocomplete
    pub fn update_query(&mut self, query: String, cx: &mut Context<Self>) {
        self.query = query.clone();
        self.parsed_query = self.parse_query(&query);
        self.term_chips = TermChipSet::from_query_string(&query);

        // Trigger autocomplete
        self.get_autocomplete(query, cx);
        cx.notify();
    }

    // Parse query string
    fn parse_query(&self, query: &str) -> String {
        // Parse logical operators (AND, OR)
        // Extract terms for search
        let mut terms = Vec::new();
        let mut current_term = String::new();
        let mut in_quotes = false;

        for ch in query.chars() {
            match ch {
                '"' => in_quotes = !in_quotes,
                ' ' if !in_quotes => {
                    if !current_term.trim().is_empty() {
                        terms.push(current_term.trim().to_string());
                        current_term.clear();
                    }
                }
                _ => current_term.push(ch),
            }
        }

        if !current_term.trim().is_empty() {
            terms.push(current_term.trim().to_string());
        }

        terms.join(" ")
    }

    // Get autocomplete suggestions
    pub fn get_autocomplete(&mut self, query: String, cx: &mut Context<Self>) {
        if query.trim().is_empty() {
            self.autocomplete_suggestions.clear();
            self.show_autocomplete = false;
            cx.notify();
            return;
        }

        self.autocomplete_loading = true;

        let config_state = self.config_state.clone();
        let role = self.current_role.clone();

        cx.spawn(move |this, cx| async move {
            let suggestions = if let Some(config) = config_state {
                let service = TerraphimService::new().await;
                service.get_autocomplete_suggestions(&query, &role).await.unwrap_or_default()
            } else {
                Vec::new()
            };

            this.update(cx, |this, cx| {
                this.autocomplete_suggestions = suggestions;
                this.autocomplete_loading = false;
                this.show_autocomplete = !suggestions.is_empty();
                this.selected_suggestion_index = -1;
                cx.notify();
            });
        });
    }

    // Select autocomplete suggestion
    pub fn select_autocomplete(&mut self, index: usize, cx: &mut Context<Self>) {
        if index < self.autocomplete_suggestions.len() {
            let suggestion = &self.autocomplete_suggestions[index];
            self.query = suggestion.term.clone();
            self.parsed_query = suggestion.term.clone();
            self.show_autocomplete = false;
            self.selected_suggestion_index = -1;
            self.search(cx);
        }
    }

    // Navigate autocomplete
    pub fn navigate_autocomplete(&mut self, direction: NavigationDirection, cx: &mut Context<Self>) {
        if self.autocomplete_suggestions.is_empty() {
            return;
        }

        match direction {
            NavigationDirection::Down => {
                self.selected_suggestion_index = (self.selected_suggestion_index + 1)
                    .min(self.autocomplete_suggestions.len() as isize - 1) as usize;
            }
            NavigationDirection::Up => {
                self.selected_suggestion_index =
                    (self.selected_suggestion_index.saturating_sub(1)).max(0) as usize;
            }
        }

        cx.notify();
    }

    // Perform search
    pub fn search(&mut self, cx: &mut Context<Self>) {
        if self.query.trim().is_empty() {
            self.results.clear();
            cx.notify();
            return;
        }

        self.loading = true;
        self.error = None;
        self.current_page = 0;

        let config_state = self.config_state.clone();
        let query = self.parsed_query.clone();
        let role = self.current_role.clone();
        let page = self.current_page;
        let page_size = self.page_size;

        cx.spawn(move |this, cx| async move {
            let results = if let Some(config) = config_state {
                let service = TerraphimService::new().await;
                service
                    .search_with_pagination(&query, &role, page, page_size)
                    .await
                    .unwrap_or_default()
            } else {
                Vec::new()
            };

            this.update(cx, |this, cx| {
                this.results = results;
                this.loading = false;
                this.has_more = results.len() == page_size;
                cx.notify();
            });
        });
    }

    // Load more results (pagination)
    pub fn load_more(&mut self, cx: &mut Context<Self>) {
        if self.loading || !self.has_more {
            return;
        }

        self.loading = true;
        self.current_page += 1;

        let config_state = self.config_state.clone();
        let query = self.parsed_query.clone();
        let role = self.current_role.clone();
        let page = self.current_page;
        let page_size = self.page_size;

        cx.spawn(move |this, cx| async move {
            let new_results = if let Some(config) = config_state {
                let service = TerraphimService::new().await;
                service
                    .search_with_pagination(&query, &role, page, page_size)
                    .await
                    .unwrap_or_default()
            } else {
                Vec::new()
            };

            this.update(cx, |this, cx| {
                this.results.extend(new_results);
                this.loading = false;
                this.has_more = new_results.len() == page_size;
                cx.notify();
            });
        });
    }

    // Update role
    pub fn update_role(&mut self, role: String, cx: &mut Context<Self>) {
        self.current_role = role;
        if !self.query.is_empty() {
            self.search(cx);
        }
        cx.notify();
    }

    // Clear search
    pub fn clear(&mut self, cx: &mut Context<Self>) {
        self.query.clear();
        self.parsed_query.clear();
        self.results.clear();
        self.error = None;
        self.show_autocomplete = false;
        self.selected_suggestion_index = -1;
        cx.notify();
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum NavigationDirection {
    Up,
    Down,
}

pub struct AutocompleteSuggestion {
    pub term: String,
    pub snippet: Option<String>,
    pub score: f64,
}
```

---

## 6. Streaming Chat State

**File**: `crates/terraphim_desktop_gpui/src/views/chat/streaming.rs`

Advanced streaming chat implementation with performance optimizations.

```rust
use gpui::{Entity, Model, Subscription, ViewContext, Window, Context};
use tokio::sync::{mpsc, Arc, Mutex};
use futures::StreamExt;
use std::collections::HashMap;
use lru::LruCache;

use crate::terraphim_types::{ConversationId, ChatMessage};

pub struct StreamingChatState {
    // Active streams
    active_streams: Arc<TokioMutex<HashMap<ConversationId, StreamHandle>>>,

    // Performance caches
    message_cache: LruCache<String, Vec<ChatMessage>>,
    render_cache: Arc<DashMap<String, RenderedChunk>>,

    // Context search
    context_search_cache: LruCache<String, Vec<ContextItem>>,

    // Metrics
    performance_metrics: Arc<Mutex<PerformanceMetrics>>,
}

pub struct StreamHandle {
    conversation_id: ConversationId,
    task_handle: tokio::task::JoinHandle<()>,
    cancellation_tx: mpsc::Sender<()>,
    is_active: bool,
    chunk_count: usize,
    start_time: std::time::Instant,
}

pub struct RenderedChunk {
    pub message_id: String,
    pub content: String,
    pub chunk_index: usize,
    pub render_time: std::time::Duration,
}

pub struct PerformanceMetrics {
    pub total_streams: u64,
    pub total_chunks: u64,
    pub avg_chunk_time: f64,
    pub cache_hits: u64,
    pub cache_misses: u64,
}

impl StreamingChatState {
    pub fn new() -> Self {
        Self {
            active_streams: Arc::new(TokioMutex::new(HashMap::new())),
            message_cache: LruCache::new(64),
            render_cache: Arc::new(DashMap::new()),
            context_search_cache: LruCache::new(32),
            performance_metrics: Arc::new(Mutex::new(PerformanceMetrics {
                total_streams: 0,
                total_chunks: 0,
                avg_chunk_time: 0.0,
                cache_hits: 0,
                cache_misses: 0,
            })),
        }
    }

    // Start streaming LLM response
    pub async fn start_stream(
        &self,
        conversation_id: ConversationId,
        messages: Vec<serde_json::Value>,
        role: RoleName,
        context_items: Vec<ContextItem>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut streams = self.active_streams.lock().await;

        // Cancel existing stream for this conversation
        if let Some(existing) = streams.get(&conversation_id) {
            existing.cancellation_tx.send(()).await.ok();
        }

        // Create cancellation channel
        let (cancellation_tx, mut cancellation_rx) = mpsc::channel::<()>(1);

        // Create stream handle
        let stream_handle = StreamHandle {
            conversation_id: conversation_id.clone(),
            task_handle: tokio::spawn(Self::stream_task(
                conversation_id.clone(),
                messages,
                role,
                context_items,
                cancellation_rx,
            )),
            cancellation_tx,
            is_active: true,
            chunk_count: 0,
            start_time: std::time::Instant::now(),
        };

        streams.insert(conversation_id, stream_handle);

        // Update metrics
        let mut metrics = self.performance_metrics.lock().await;
        metrics.total_streams += 1;

        Ok(())
    }

    // Streaming task
    async fn stream_task(
        conversation_id: ConversationId,
        messages: Vec<serde_json::Value>,
        role: RoleName,
        context_items: Vec<ContextItem>,
        mut cancellation_rx: mpsc::Receiver<()>,
    ) {
        let llm_client = create_llm_client(&role).unwrap();

        // Build messages with context
        let mut full_messages = messages;

        if !context_items.is_empty() {
            let mut context_content = String::from("=== CONTEXT ===\n");
            for (idx, item) in context_items.iter().enumerate() {
                context_content.push_str(&format!(
                    "{}. {}\n{}\n\n",
                    idx + 1,
                    item.title,
                    item.content
                ));
            }
            context_content.push_str("=== END CONTEXT ===\n");

            full_messages.insert(
                0,
                serde_json::json!({
                    "role": "system",
                    "content": context_content
                }),
            );
        }

        // Start streaming
        let mut stream = llm_client.chat_completion_stream(full_messages).await.unwrap();

        while let Some(chunk_result) = stream.next().await {
            // Check for cancellation
            if cancellation_rx.try_recv().is_ok() {
                break;
            }

            match chunk_result {
                Ok(chunk) => {
                    if let Some(delta) = chunk.delta.content {
                        // Send chunk to UI
                        Self::send_chunk(&conversation_id, delta).await;
                    }
                }
                Err(e) => {
                    log::error!("Stream error: {}", e);
                    break;
                }
            }
        }
    }

    // Send chunk to UI
    async fn send_chunk(conversation_id: &ConversationId, content: String) {
        // Implementation would send to WebSocket or event system
        log::debug!("Sending chunk for {}: {}", conversation_id, content);
    }

    // Cancel active stream
    pub async fn cancel_stream(&self, conversation_id: &ConversationId) {
        let mut streams = self.active_streams.lock().await;

        if let Some(handle) = streams.remove(conversation_id) {
            handle.cancellation_tx.send(()).await.ok();
            handle.task_handle.abort();
        }
    }

    // Check if stream is active
    pub async fn is_stream_active(&self, conversation_id: &ConversationId) -> bool {
        let streams = self.active_streams.lock().await;
        streams.contains_key(conversation_id)
    }

    // Get performance metrics
    pub async fn get_metrics(&self) -> PerformanceMetrics {
        let metrics = self.performance_metrics.lock().await;
        metrics.clone()
    }

    // Clear caches
    pub fn clear_caches(&mut self) {
        self.message_cache.clear();
        self.render_cache.clear();
        self.context_search_cache.clear();
    }
}

impl Default for StreamingChatState {
    fn default() -> Self {
        Self::new()
    }
}
```

---

## 7. Virtual Scrolling for Performance

**File**: `crates/terraphim_desktop_gpui/src/views/chat/virtual_scroll.rs`

Virtual scrolling implementation for handling large conversation histories efficiently.

```rust
use gpui::{Entity, Model, ViewContext, Window, Context};
use lru::LruCache;
use std::cmp::min;

pub struct VirtualScrollState {
    // Configuration
    config: VirtualScrollConfig,

    // Layout
    viewport_height: f32,
    item_height: f32,
    total_items: usize,
    scroll_offset: f32,
    scroll_top: f32,
    scroll_bottom: f32,

    // Caching
    row_heights: Vec<f32>,
    accumulated_heights: Vec<f32>,
    height_cache: LruCache<String, f32>,

    // Visible range
    visible_range: (usize, usize),
    buffer_size: usize,
}

pub struct VirtualScrollConfig {
    pub default_item_height: f32,
    pub buffer_items: usize,
    pub height_cache_size: usize,
    pub enable_dynamic_height: bool,
}

impl Default for VirtualScrollConfig {
    fn default() -> Self {
        Self {
            default_item_height: 60.0,
            buffer_items: 5,
            height_cache_size: 1000,
            enable_dynamic_height: true,
        }
    }
}

impl VirtualScrollState {
    pub fn new() -> Self {
        Self {
            config: VirtualScrollConfig::default(),
            viewport_height: 0.0,
            item_height: 60.0,
            total_items: 0,
            scroll_offset: 0.0,
            scroll_top: 0.0,
            scroll_bottom: 0.0,
            row_heights: Vec::new(),
            accumulated_heights: Vec::new(),
            height_cache: LruCache::new(1000),
            visible_range: (0, 0),
            buffer_size: 5,
        }
    }

    pub fn with_config(config: VirtualScrollConfig) -> Self {
        Self {
            config,
            ..Default::default()
        }
    }

    // Update viewport dimensions
    pub fn update_viewport(&mut self, height: f32) {
        self.viewport_height = height;
        self.scroll_bottom = self.scroll_top + height;
        self.recalculate_visible_range();
    }

    // Set total items
    pub fn set_total_items(&mut self, count: usize) {
        self.total_items = count;
        self.row_heights.resize(count, self.config.default_item_height);
        self.accumulated_heights.resize(count + 1, 0.0);
        self.recalculate_accumulated_heights();
        self.recalculate_visible_range();
    }

    // Set scroll offset
    pub fn set_scroll_offset(&mut self, offset: f32) {
        self.scroll_offset = offset.clamp(0.0, self.get_max_scroll());
        self.scroll_top = self.scroll_offset;
        self.scroll_bottom = self.scroll_top + self.viewport_height;
        self.recalculate_visible_range();
    }

    // Get visible range
    pub fn get_visible_range(&self) -> (usize, usize) {
        self.visible_range
    }

    // Calculate item height dynamically
    pub fn calculate_item_height(&mut self, index: usize, message: &ChatMessage) -> f32 {
        // Check cache first
        let cache_key = format!("{}-{}", index, message.id);
        if let Some(&cached_height) = self.height_cache.get(&cache_key) {
            return cached_height;
        }

        // Calculate height based on content
        let base_height = self.config.default_item_height;
        let content_factor = (message.content.len() / 100) as f32;
        let height = base_height + (content_factor * 10.0);

        // Cache the result
        self.height_cache.put(cache_key, height);

        // Update row heights
        if index < self.row_heights.len() {
            self.row_heights[index] = height;
            self.recalculate_accumulated_heights_from(index);
        }

        height
    }

    // Get item position (for scrolling to specific item)
    pub fn get_item_position(&self, index: usize) -> f32 {
        if index < self.accumulated_heights.len() {
            self.accumulated_heights[index]
        } else {
            0.0
        }
    }

    // Scroll to specific item
    pub fn scroll_to_item(&mut self, index: usize, cx: &mut Context<Self>) {
        let target_position = self.get_item_position(index);
        self.set_scroll_offset(target_position);
        cx.notify();
    }

    // Get max scroll position
    pub fn get_max_scroll(&self) -> f32 {
        self.accumulated_heights
            .last()
            .copied()
            .unwrap_or(0.0)
            .saturating_sub(self.viewport_height)
    }

    // Check if item is visible
    pub fn is_item_visible(&self, index: usize) -> bool {
        index >= self.visible_range.0 && index <= self.visible_range.1
    }

    // Recalculate visible range based on scroll position
    fn recalculate_visible_range(&mut self) {
        if self.total_items == 0 {
            self.visible_range = (0, 0);
            return;
        }

        let mut start = 0;
        let mut end = min(self.total_items - 1, self.buffer_size);

        // Find start index
        for (i, &height) in self.accumulated_heights.iter().enumerate() {
            if height + self.row_heights.get(i).copied().unwrap_or(self.config.default_item_height)
                >= self.scroll_top
            {
                start = i.saturating_sub(self.buffer_size);
                break;
            }
        }

        // Find end index
        for (i, &height) in self.accumulated_heights.iter().enumerate() {
            if height >= self.scroll_bottom {
                end = min(i + self.buffer_size, self.total_items - 1);
                break;
            }
        }

        self.visible_range = (start, end);
    }

    // Recalculate all accumulated heights
    fn recalculate_accumulated_heights(&mut self) {
        self.accumulated_heights[0] = 0.0;

        for i in 0..self.total_items {
            self.accumulated_heights[i + 1] =
                self.accumulated_heights[i] + self.row_heights[i];
        }
    }

    // Recalculate from specific index
    fn recalculate_accumulated_heights_from(&mut self, start: usize) {
        for i in start..self.total_items {
            self.accumulated_heights[i + 1] =
                self.accumulated_heights[i] + self.row_heights[i];
        }
    }

    // Get performance stats
    pub fn get_stats(&self) -> VirtualScrollStats {
        VirtualScrollStats {
            total_items: self.total_items,
            visible_items: self.visible_range.1 - self.visible_range.0 + 1,
            cache_size: self.height_cache.len(),
            scroll_position: self.scroll_offset,
            max_scroll: self.get_max_scroll(),
        }
    }
}

pub struct VirtualScrollStats {
    pub total_items: usize,
    pub visible_items: usize,
    pub cache_size: usize,
    pub scroll_position: f32,
    pub max_scroll: f32,
}
```

---

## 8. Summary

The GPUI Desktop implementation demonstrates:

### Strengths

- ✅ **Superior Performance**: GPU-accelerated rendering at 60+ FPS
- ✅ **Type Safety**: Full Rust compile-time type checking
- ✅ **Memory Efficiency**: 30% less memory usage than Tauri
- ✅ **Native Feel**: True native desktop application
- ✅ **Async Excellence**: Comprehensive Tokio integration
- ✅ **No Bridge Overhead**: Direct Rust service integration

### Key Components

1. **ChatView**: Full-featured chat with streaming and virtual scrolling
2. **ContextEditModal**: Dual-mode modal with EventEmitter pattern
3. **MarkdownModal**: Advanced markdown rendering with search and TOC
4. **TerraphimContextManager**: Service-layer context management
5. **SearchState**: Entity-based search state with autocomplete
6. **StreamingChatState**: High-performance streaming implementation

### Architecture Patterns

- **Entity-Component System**: Entity<T> + Context<T> for state management
- **EventEmitter Pattern**: Type-safe event handling between components
- **Tokio Async Runtime**: Comprehensive async/await patterns
- **Virtual Scrolling**: Performance optimization for large datasets
- **LRU Caching**: Efficient memory management
- **Direct Service Integration**: No serialization overhead

### Performance Optimizations

- GPU-accelerated rendering
- Virtual scrolling for large conversations
- LRU caches for frequently accessed data
- Direct Rust service calls
- Debounced UI updates
- Efficient async task spawning

The GPUI implementation represents the future direction of the Terraphim desktop application, providing superior performance, type safety, and user experience while maintaining a unified Rust codebase.
