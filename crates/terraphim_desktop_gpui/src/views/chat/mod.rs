use gpui::prelude::FluentBuilder;
use gpui::*;
use gpui_component::button::*;
use gpui_component::input::{Input, InputEvent as GpuiInputEvent, InputState};
use gpui_component::{IconName, StyledExt};
use std::sync::Arc;
use terraphim_config::ConfigState;
use terraphim_service::context::{ContextConfig, TerraphimContextManager};
use terraphim_service::llm;
use terraphim_types::{ChatMessage, ContextItem, ContextType, ConversationId, RoleName};
use tokio::sync::Mutex as TokioMutex;

use crate::markdown::render_markdown;
use crate::slash_command::{
    CommandRegistry, SlashCommandPopup, SlashCommandPopupEvent, SuggestionAction, ViewScope,
};
use crate::theme::colors::theme;

mod context_edit_modal;
pub use context_edit_modal::{ContextEditModal, ContextEditModalEvent, ContextEditMode};

mod virtual_scroll;
use virtual_scroll::{VirtualScrollConfig, VirtualScrollState};

impl EventEmitter<ContextEditModalEvent> for ChatView {}

/// Chat view with real ContextManager and LLM backend
pub struct ChatView {
    context_manager: Arc<TokioMutex<TerraphimContextManager>>,
    config_state: Option<ConfigState>,
    current_conversation_id: Option<ConversationId>,
    current_role: RoleName,
    messages: Vec<ChatMessage>,
    virtual_scroll_state: VirtualScrollState,
    input_state: Option<Entity<InputState>>,
    is_sending: bool,
    show_context_panel: bool,
    context_items: Vec<ContextItem>,
    context_edit_modal: Entity<ContextEditModal>,
    context_warning: Option<String>, // Warning message when context exceeds limits
    /// Slash command popup for / commands
    slash_command_popup: Entity<SlashCommandPopup>,
    _subscriptions: Vec<Subscription>,
}

impl ChatView {
    pub fn new(
        window: &mut Window,
        cx: &mut Context<Self>,
        command_registry: Arc<CommandRegistry>,
    ) -> Self {
        log::info!("ChatView initialized with backend ContextManager");

        // Initialize ContextManager using Tauri pattern (cmd.rs:937-947)
        // Use a more permissive config for desktop app (allow larger context)
        let context_config = ContextConfig {
            max_context_items: 100,      // Increased from default 50
            max_context_length: 500_000, // Increased from default 100K to 500K characters
            max_conversations_cache: 100,
            default_search_results_limit: 5,
            enable_auto_suggestions: true,
        };
        let context_manager = Arc::new(TokioMutex::new(TerraphimContextManager::new(
            context_config,
        )));

        // Initialize input for message composition
        let input_state =
            cx.new(|cx| InputState::new(window, cx).placeholder("Type your message..."));

        // Create slash command popup
        let slash_command_popup = cx.new(|cx| {
            SlashCommandPopup::with_providers(
                window,
                cx,
                command_registry.clone(),
                None,
                ViewScope::Chat,
            )
        });

        // Subscribe to slash command popup events
        let slash_sub = cx.subscribe(
            &slash_command_popup,
            move |this, _popup, event: &SlashCommandPopupEvent, cx| {
                match event {
                    SlashCommandPopupEvent::SuggestionSelected { suggestion, .. } => {
                        log::info!("Slash command suggestion selected: {}", suggestion.text);

                        // Handle the suggestion action
                        match &suggestion.action {
                            SuggestionAction::Insert {
                                text,
                                replace_trigger,
                            } => {
                                if let Some(input) = &this.input_state {
                                    // For now, append the text
                                    // TODO: Replace trigger text when replace_trigger is true
                                    input.update(cx, |input, _cx| {
                                        // Input doesn't have direct append, so we'd need window context
                                        log::debug!("Would insert: {}", text);
                                    });
                                }
                            }
                            SuggestionAction::ExecuteCommand { command_id, args } => {
                                log::info!("Execute command: {} with args: {:?}", command_id, args);
                                this.handle_slash_command(command_id.as_str(), args.clone(), cx);
                            }
                            SuggestionAction::Search { query, use_kg } => {
                                log::info!("Search: {} (use_kg: {})", query, use_kg);
                                // TODO: Integrate with search
                            }
                            _ => {}
                        }
                    }
                    SlashCommandPopupEvent::Closed => {
                        log::debug!("Slash command popup closed");
                    }
                }
            },
        );

        // Subscribe to input events for message sending and slash command detection
        let input_clone = input_state.clone();
        let slash_popup_for_input = slash_command_popup.clone();
        let input_sub = cx.subscribe_in(
            &input_state,
            window,
            move |this, _, ev: &GpuiInputEvent, window, cx| {
                match ev {
                    GpuiInputEvent::Change => {
                        // Detect slash commands
                        let value = input_clone.read(cx).value();
                        let cursor = value.len(); // Approximate cursor at end

                        slash_popup_for_input.update(cx, |popup, cx| {
                            popup.process_input(&value, cursor, cx);
                        });
                    }
                    GpuiInputEvent::PressEnter { .. } => {
                        // Check if slash popup is open - if so, accept selection
                        let popup_open = slash_popup_for_input.read(cx).is_open();

                        if popup_open {
                            slash_popup_for_input.update(cx, |popup, cx| {
                                popup.accept_selected(cx);
                            });
                        } else {
                            let value = input_clone.read(cx).value();
                            if !value.is_empty() {
                                this.send_message(value.to_string(), cx);
                                // Input will keep text (clearing not critical for now)
                            }
                        }
                    }
                    _ => {}
                }
            },
        );

        // Create context edit modal
        let context_edit_modal = cx.new(|cx| ContextEditModal::new(window, cx));

        // Subscribe to context edit modal events
        let _modal_clone = context_edit_modal.clone();
        let modal_sub = cx.subscribe(
            &context_edit_modal,
            move |this, _modal, event: &ContextEditModalEvent, cx| match event {
                ContextEditModalEvent::Create(context_item) => {
                    log::info!("ContextEditModal: Create event received");
                    this.add_context(context_item.clone(), cx);
                }
                ContextEditModalEvent::Update(context_item) => {
                    log::info!("ContextEditModal: Update event received");
                    this.update_context(context_item.clone(), cx);
                }
                ContextEditModalEvent::Delete(context_id) => {
                    log::info!(
                        "ContextEditModal: Delete event received for: {}",
                        context_id
                    );
                    this.delete_context(context_id.clone(), cx);
                }
                ContextEditModalEvent::Close => {
                    log::info!("ContextEditModal: Close event received");
                }
            },
        );

        Self {
            context_manager,
            config_state: None,
            current_conversation_id: None,
            current_role: RoleName::from("Terraphim Engineer"),
            messages: Vec::new(),
            virtual_scroll_state: VirtualScrollState::new(VirtualScrollConfig::default()),
            input_state: Some(input_state),
            is_sending: false,
            show_context_panel: true,
            context_items: Vec::new(),
            context_edit_modal,
            context_warning: None,
            slash_command_popup,
            _subscriptions: vec![input_sub, modal_sub, slash_sub],
        }
    }

    /// Initialize with config for LLM access
    pub fn with_config(mut self, config_state: ConfigState) -> Self {
        self.config_state = Some(config_state);
        self
    }

    /// Update role (called when role changes from system tray or dropdown)
    pub fn update_role(&mut self, new_role: String, cx: &mut Context<Self>) {
        if self.current_role.to_string() != new_role {
            log::info!(
                "ChatView: role changed from {} to {}",
                self.current_role,
                new_role
            );
            self.current_role = RoleName::from(new_role.as_str());
            cx.notify();
        }
    }

    /// Create a new conversation (pattern from Tauri cmd.rs:950-978)
    pub fn create_conversation(&mut self, title: String, role: RoleName, cx: &mut Context<Self>) {
        log::info!("Creating conversation: {} (role: {})", title, role);

        let manager = self.context_manager.clone();

        cx.spawn(async move |this, cx| {
            let mut mgr = manager.lock().await;

            match mgr.create_conversation(title, role).await {
                Ok(conversation_id) => {
                    log::info!("✅ Created conversation: {}", conversation_id.as_str());

                    this.update(cx, |this, cx| {
                        this.current_conversation_id = Some(conversation_id);
                        this.messages.clear();
                        this.context_items.clear();
                        this.update_virtual_scroll_state(cx);
                        cx.notify();
                    })
                    .ok();
                }
                Err(e) => {
                    log::error!("❌ Failed to create conversation: {}", e);
                }
            }
        })
        .detach();
    }

    /// Add document directly to context (no modal - used from search results)
    pub fn add_document_as_context_direct(
        &mut self,
        document: terraphim_types::Document,
        cx: &mut Context<Self>,
    ) {
        log::info!("Adding document directly to context: {}", document.title);

        // Create ContextItem from Document (Tauri pattern)
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
                if let Some(rank) = document.rank {
                    meta.insert("rank".to_string(), rank.to_string());
                }
                meta
            },
            created_at: chrono::Utc::now(),
            relevance_score: document.rank.map(|r| r as f64),
        };

        self.add_context(context_item, cx);
    }

    /// Add context to current conversation (pattern from Tauri cmd.rs:1078-1140)
    /// Automatically creates a conversation if one doesn't exist
    pub fn add_context(&mut self, context_item: ContextItem, cx: &mut Context<Self>) {
        // If no conversation exists, create one automatically
        if self.current_conversation_id.is_none() {
            log::info!("No active conversation, creating one automatically");
            let role = self.current_role.clone();
            let title = format!("Context: {}", context_item.title);

            // Create conversation first, then add context
            let manager = self.context_manager.clone();
            let context_item_clone = context_item.clone();

            cx.spawn(async move |this, cx| {
                let mut mgr = manager.lock().await;

                // Create conversation
                match mgr.create_conversation(title.clone(), role.clone()).await {
                    Ok(conversation_id) => {
                        log::info!("✅ Created conversation: {}", conversation_id.as_str());

                        // Now add context to the newly created conversation
                        match mgr
                            .add_context(&conversation_id, context_item_clone.clone())
                            .await
                        {
                            Ok(result) => {
                                log::info!("✅ Added context to new conversation");
                                let warning = result.warning.clone();

                                this.update(cx, |this, cx| {
                                    this.current_conversation_id = Some(conversation_id);
                                    this.context_items.push(context_item_clone);
                                    this.context_warning = warning;
                                    cx.notify();
                                })
                                .ok();
                            }
                            Err(e) => {
                                log::error!("❌ Failed to add context to new conversation: {}", e);
                            }
                        }
                    }
                    Err(e) => {
                        log::error!("❌ Failed to create conversation: {}", e);
                    }
                }
            })
            .detach();
            return;
        }

        // Conversation exists, add context normally
        let conv_id = self.current_conversation_id.as_ref().unwrap().clone();
        let manager = self.context_manager.clone();

        cx.spawn(async move |this, cx| {
            let mut mgr = manager.lock().await;

            match mgr.add_context(&conv_id, context_item.clone()).await {
                Ok(result) => {
                    log::info!("✅ Added context to conversation");
                    let warning = result.warning.clone();

                    this.update(cx, |this, cx| {
                        this.context_items.push(context_item);
                        this.context_warning = warning;
                        cx.notify();
                    })
                    .ok();
                }
                Err(e) => {
                    log::error!("❌ Failed to add context: {}", e);
                }
            }
        })
        .detach();
    }

    /// Update context item in conversation
    pub fn update_context(&mut self, context_item: ContextItem, cx: &mut Context<Self>) {
        let conv_id = match &self.current_conversation_id {
            Some(id) => id.clone(),
            None => {
                log::warn!("Cannot update context: no active conversation");
                return;
            }
        };

        let manager = self.context_manager.clone();

        cx.spawn(async move |this, cx| {
            let mut mgr = manager.lock().await;

            // TODO: Implement update_context in ContextManager if it doesn't exist
            // For now, delete and re-add
            match mgr.delete_context(&conv_id, &context_item.id).await {
                Ok(()) => {
                    match mgr.add_context(&conv_id, context_item.clone()).await {
                        Ok(result) => {
                            log::info!("✅ Updated context in conversation");
                            let warning = result.warning.clone();
                            this.update(cx, |this, cx| {
                                // Update in local list
                                if let Some(item) = this
                                    .context_items
                                    .iter_mut()
                                    .find(|item| item.id == context_item.id)
                                {
                                    *item = context_item.clone();
                                }
                                this.context_warning = warning;
                                cx.notify();
                            })
                            .ok();
                        }
                        Err(e) => {
                            log::error!("❌ Failed to re-add context after delete: {}", e);
                        }
                    }
                }
                Err(e) => {
                    log::error!("❌ Failed to delete context for update: {}", e);
                }
            }
        })
        .detach();
    }

    /// Delete context from conversation (pattern from Tauri cmd.rs:1180-1211)
    pub fn delete_context(&mut self, context_id: String, cx: &mut Context<Self>) {
        let conv_id = match &self.current_conversation_id {
            Some(id) => id.clone(),
            None => return,
        };

        let manager = self.context_manager.clone();

        cx.spawn(async move |this, cx| {
            let mut mgr = manager.lock().await;

            match mgr.delete_context(&conv_id, &context_id).await {
                Ok(()) => {
                    log::info!("✅ Deleted context: {}", context_id);

                    this.update(cx, |this, cx| {
                        this.context_items.retain(|item| item.id != context_id);
                        cx.notify();
                    })
                    .ok();
                }
                Err(e) => {
                    log::error!("❌ Failed to delete context: {}", e);
                }
            }
        })
        .detach();
    }

    /// Handle slash command execution
    fn handle_slash_command(
        &mut self,
        command_id: &str,
        args: Option<String>,
        cx: &mut Context<Self>,
    ) {
        let args_str = args.unwrap_or_default();
        log::info!("Handling slash command: /{} {}", command_id, args_str);

        match command_id {
            "summarize" => {
                // Add summarize request to chat
                self.send_message("Please summarize the current context.".to_string(), cx);
            }
            "explain" => {
                let message = if args_str.is_empty() {
                    "Please explain the last message.".to_string()
                } else {
                    format!("Please explain: {}", args_str)
                };
                self.send_message(message, cx);
            }
            "improve" => {
                let message = if args_str.is_empty() {
                    "Please improve the last message.".to_string()
                } else {
                    format!("Please improve: {}", args_str)
                };
                self.send_message(message, cx);
            }
            "translate" => {
                let message = if args_str.is_empty() {
                    "Please translate the last message.".to_string()
                } else {
                    format!("Please translate to {}", args_str)
                };
                self.send_message(message, cx);
            }
            "search" => {
                if !args_str.is_empty() {
                    log::info!("Search triggered from chat: {}", args_str);
                    // Add search query to chat
                    self.send_message(format!("Search: {}", args_str), cx);
                    // TODO: Switch to search view and trigger actual search
                }
            }
            "kg" => {
                if !args_str.is_empty() {
                    log::info!("KG search triggered from chat: {}", args_str);
                    self.send_message(format!("KG Search: {}", args_str), cx);
                    // TODO: Trigger actual KG search
                }
            }
            "context" => {
                // Show context panel
                let count = self.context_items.len();
                if count == 0 {
                    self.send_message(
                        "No context items yet. Use /add to add documents to context.".to_string(),
                        cx,
                    );
                } else {
                    self.send_message(
                        format!(
                            "Current context has {} items. Use /add to add more documents.",
                            count
                        ),
                        cx,
                    );
                }
                cx.notify();
            }
            "add" => {
                if !args_str.is_empty() {
                    log::info!("Add to context: {}", args_str);
                    // TODO: Add document to context
                    self.send_message(
                        format!("Add '{}' to context (TODO: implement)", args_str),
                        cx,
                    );
                } else {
                    log::info!("Please provide content to add to context");
                }
            }
            "clear" => {
                let count = self.context_items.len();
                self.context_items.clear();
                cx.notify();
                log::info!("Context cleared ({} items were removed)", count);
            }
            "help" => {
                // Show help message in chat
                self.send_message("Available commands: /summarize, /explain, /improve, /translate, /search, /kg, /context, /add, /clear, /help".to_string(), cx);
            }
            _ => {
                log::debug!("Unhandled command: {}", command_id);
            }
        }
    }

    /// Send message with LLM (pattern from Tauri cmd.rs:1668-1838)
    pub fn send_message(&mut self, content: String, cx: &mut Context<Self>) {
        if content.trim().is_empty() {
            return;
        }

        log::info!("Sending message: {}", content);

        // Add user message to local history
        self.messages.push(ChatMessage::user(content.clone()));
        self.is_sending = true;
        self.update_virtual_scroll_state(cx);
        cx.notify();

        let config_state = match &self.config_state {
            Some(state) => state.clone(),
            None => {
                log::error!("Cannot send message: config not initialized");
                self.is_sending = false;
                cx.notify();
                return;
            }
        };

        let role = self.current_role.clone();
        let context_manager = self.context_manager.clone();
        let conv_id = self.current_conversation_id.clone();

        cx.spawn(async move |this, cx| {
            // Get role config (Tauri pattern cmd.rs:1679-1694)
            let config = config_state.config.lock().await;
            let role_config = match config.roles.get(&role) {
                Some(rc) => rc.clone(),
                None => {
                    log::error!("Role '{}' not found", role);
                    this.update(cx, |this, cx| {
                        this.is_sending = false;
                        cx.notify();
                    })
                    .ok();
                    return;
                }
            };
            drop(config);

            // Build LLM client (Tauri pattern cmd.rs:1760)
            let llm_client = match llm::build_llm_from_role(&role_config) {
                Some(client) => client,
                None => {
                    log::warn!("No LLM configured for role, using simulated response");
                    this.update(cx, |this, cx| {
                        let response =
                            format!("Simulated response (no LLM configured): {}", content);
                        this.messages.push(ChatMessage::assistant(
                            response,
                            Some("simulated".to_string()),
                        ));
                        this.is_sending = false;
                        this.update_virtual_scroll_state(cx);
                        cx.notify();
                    })
                    .ok();
                    return;
                }
            };

            // Build messages with context (Tauri pattern cmd.rs:1769-1816)
            let mut messages_json: Vec<serde_json::Value> = Vec::new();

            // Inject context if conversation exists
            if let Some(conv_id) = &conv_id {
                let manager = context_manager.lock().await;
                if let Ok(conversation) = manager.get_conversation(conv_id).await {
                    if !conversation.global_context.is_empty() {
                        let mut context_content = String::from("=== CONTEXT ===\n");
                        for (idx, item) in conversation.global_context.iter().enumerate() {
                            context_content.push_str(&format!(
                                "{}. {}\n{}\n\n",
                                idx + 1,
                                item.title,
                                item.content
                            ));
                        }
                        context_content.push_str("=== END CONTEXT ===\n");
                        messages_json.push(
                            serde_json::json!({"role": "system", "content": context_content}),
                        );
                    }
                }
            }

            // Add user message
            messages_json.push(serde_json::json!({"role": "user", "content": content}));

            // Call LLM (Tauri pattern cmd.rs:1824)
            let opts = llm::ChatOptions {
                max_tokens: Some(1024),
                temperature: Some(0.7),
            };

            match llm_client.chat_completion(messages_json, opts).await {
                Ok(reply) => {
                    log::info!("✅ LLM response received ({} chars)", reply.len());
                    this.update(cx, |this, cx| {
                        this.messages.push(ChatMessage::assistant(
                            reply,
                            Some(llm_client.name().to_string()),
                        ));
                        this.is_sending = false;
                        this.update_virtual_scroll_state(cx);
                        cx.notify();
                    })
                    .ok();
                }
                Err(e) => {
                    log::error!("❌ LLM call failed: {}", e);
                    this.update(cx, |this, cx| {
                        this.messages
                            .push(ChatMessage::system(format!("Error: {}", e)));
                        this.is_sending = false;
                        this.update_virtual_scroll_state(cx);
                        cx.notify();
                    })
                    .ok();
                }
            }
        })
        .detach();
    }

    /// Toggle context panel visibility
    pub fn toggle_context_panel(
        &mut self,
        _event: &ClickEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.show_context_panel = !self.show_context_panel;
        log::info!(
            "Context panel {}",
            if self.show_context_panel {
                "shown"
            } else {
                "hidden"
            }
        );
        cx.notify();
    }

    /// Open context edit modal for creating a new context item
    pub fn open_add_context_modal(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        log::info!("Opening context edit modal to add new context item");
        self.context_edit_modal.update(cx, |modal, modal_cx| {
            modal.open_create(window, modal_cx);
        });
    }

    /// Open context edit modal for editing an existing context item
    pub fn open_edit_context_modal(
        &mut self,
        context_item: ContextItem,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        log::info!("Opening context edit modal to edit: {}", context_item.title);
        self.context_edit_modal.update(cx, |modal, modal_cx| {
            modal.open_edit(context_item, window, modal_cx);
        });
    }

    /// Handle delete context button click
    fn handle_delete_context(&mut self, context_id: String, cx: &mut Context<Self>) {
        log::info!("Deleting context: {}", context_id);
        self.delete_context(context_id, cx);
    }

    /// Handle create new conversation button
    fn handle_create_conversation(
        &mut self,
        _event: &ClickEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        log::info!("Creating new conversation");
        self.create_conversation(
            "New Conversation".to_string(),
            self.current_role.clone(),
            cx,
        );
    }

    /// Render chat header
    fn render_header(&self, cx: &Context<Self>) -> impl IntoElement {
        let title = self
            .current_conversation_id
            .as_ref()
            .map(|id| {
                format!(
                    "Conversation {}",
                    id.as_str().chars().take(8).collect::<String>()
                )
            })
            .unwrap_or_else(|| "No Conversation".to_string());

        div()
            .flex()
            .items_center()
            .justify_between()
            .px_6()
            .py_4()
            .border_b_1()
            .border_color(theme::border())
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_3()
                    .child(div().text_2xl().child("Chat"))
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .child(
                                div()
                                    .text_lg()
                                    .font_bold()
                                    .text_color(theme::text_primary())
                                    .child(title),
                            )
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(theme::text_secondary())
                                    .child(format!("{} messages", self.messages.len())),
                            ),
                    ),
            )
            .child(
                div()
                    .flex()
                    .gap_2()
                    .child(
                        // Toggle context panel button with icon
                        Button::new("toggle-context-panel")
                            .label("Context")
                            .icon(IconName::BookOpen)
                            .when(self.show_context_panel, |btn| btn.primary())
                            .when(!self.show_context_panel, |btn| btn.outline())
                            .on_click(cx.listener(Self::toggle_context_panel)),
                    )
                    .child(
                        // New conversation button
                        Button::new("new-conversation")
                            .label("New")
                            .icon(IconName::Plus)
                            .outline()
                            .on_click(cx.listener(Self::handle_create_conversation)),
                    ),
            )
    }

    /// Render message input with real Input component
    fn render_input(&self, cx: &Context<Self>) -> impl IntoElement {
        let popup = &self.slash_command_popup;

        div()
            .relative() // For popup positioning
            .flex()
            .flex_col()
            .child(
                // Slash command popup (positioned above input)
                div()
                    .absolute()
                    .bottom(px(60.0)) // Position above input
                    .left(px(24.0))
                    .child(popup.clone()),
            )
            .child(
                // Input row with keyboard navigation for slash popup
                div()
                    .flex()
                    .gap_2()
                    .px_6()
                    .py_4()
                    .border_t_1()
                    .border_color(theme::border())
                    .children(self.input_state.as_ref().map(|input| {
                        div()
                            .flex_1()
                            .track_focus(&input.focus_handle(cx))
                            // Keyboard navigation for slash popup (arrow keys + escape)
                            .on_key_down(cx.listener(|this, ev: &KeyDownEvent, _window, cx| {
                                let popup_open = this.slash_command_popup.read(cx).is_open();

                                if popup_open {
                                    match &ev.keystroke.key {
                                        key if key == "down" => {
                                            this.slash_command_popup.update(cx, |popup, cx| {
                                                popup.select_next(cx);
                                            });
                                        }
                                        key if key == "up" => {
                                            this.slash_command_popup.update(cx, |popup, cx| {
                                                popup.select_previous(cx);
                                            });
                                        }
                                        key if key == "escape" => {
                                            this.slash_command_popup.update(cx, |popup, cx| {
                                                popup.close(cx);
                                            });
                                        }
                                        _ => {}
                                    }
                                }
                            }))
                            .child(Input::new(input))
                    }))
                    .child(
                        div()
                            .px_6()
                            .py_3()
                            .rounded_md()
                            .bg(if self.is_sending {
                                theme::border()
                            } else {
                                theme::primary()
                            })
                            .text_color(theme::primary_text())
                            .when(!self.is_sending, |d| {
                                d.hover(|style| style.bg(theme::primary_hover()).cursor_pointer())
                            })
                            .when(self.is_sending, |d| {
                                d.flex()
                                    .items_center()
                                    .gap_2()
                                    .child(
                                        // Spinner for sending state
                                        div()
                                            .w_4()
                                            .h_4()
                                            .border_2()
                                            .border_color(theme::text_secondary())
                                            .rounded_full(),
                                    )
                                    .child("Sending...")
                            })
                            .when(!self.is_sending, |d| d.child("Send")),
                    ),
            )
    }

    /// Render messages area with virtual scrolling
    fn render_messages(&self, _cx: &Context<Self>) -> impl IntoElement {
        if self.messages.is_empty() {
            return self.render_empty_state().into_any_element();
        }

        let visible_range = self.virtual_scroll_state.get_visible_range();
        log::trace!(
            "Rendering messages in virtual scroll range: {:?}",
            visible_range
        );

        let scroll_offset = self.virtual_scroll_state.get_scroll_offset();

        div()
            .relative()
            .size_full()
            .overflow_hidden()
            .child(
                div()
                    .absolute()
                    .top(px(-scroll_offset))
                    .left(px(0.0))
                    .w_full()
                    .children(self.messages.iter().enumerate().map(|(idx, msg)| {
                        let y_position = self.virtual_scroll_state.get_message_position(idx);
                        self.render_message_at_position(msg, idx, y_position)
                    })),
            )
            .into_any_element()
    }

    /// Render a single message
    fn render_message(&self, message: &ChatMessage) -> impl IntoElement {
        let is_user = message.role == "user";
        let is_system = message.role == "system";
        let is_assistant = message.role == "assistant";
        let role_label = match message.role.as_str() {
            "user" => "You".to_string(),
            "system" => "System".to_string(),
            "assistant" => message.model.as_deref().unwrap_or("Assistant").to_string(),
            _ => "Unknown".to_string(),
        };
        let content = message.content.clone();

        div().flex().when(is_user, |this| this.justify_end()).child(
            div()
                .max_w(px(600.0))
                .px_4()
                .py_3()
                .rounded_lg()
                .when(is_user, |this| {
                    this.bg(theme::primary()).text_color(theme::primary_text())
                })
                .when(!is_user && !is_system, |this| {
                    this.bg(theme::surface()).text_color(theme::text_primary())
                })
                .when(is_system, |this| {
                    this.bg(theme::warning()).text_color(theme::text_primary())
                })
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap_1()
                        .child(div().text_xs().opacity(0.8).child(role_label))
                        .child(div().text_sm().child(
                            // Render markdown for assistant messages
                            if is_assistant {
                                render_markdown(&content).into_any_element()
                            } else {
                                content.clone().into_any_element()
                            },
                        )),
                ),
        )
    }

    /// Render a message at a specific position (for virtual scrolling)
    fn render_message_at_position(
        &self,
        message: &ChatMessage,
        _index: usize,
        y_position: f32,
    ) -> impl IntoElement {
        let is_user = message.role == "user";
        let is_system = message.role == "system";
        let is_assistant = message.role == "assistant";
        let role_label = match message.role.as_str() {
            "user" => "You".to_string(),
            "system" => "System".to_string(),
            "assistant" => message.model.as_deref().unwrap_or("Assistant").to_string(),
            _ => "Unknown".to_string(),
        };
        let content = message.content.clone();

        // Calculate dynamic height based on content length (for future use)
        let _estimated_height = self.calculate_message_height(&content, is_user, is_system);

        div()
            .absolute()
            .top(px(y_position))
            .left_0()
            .right_0()
            .flex()
            .when(is_user, |this| this.justify_end())
            .child(
                div()
                    .max_w(px(600.0))
                    .px_4()
                    .py_3()
                    .rounded_lg()
                    .when(is_user, |this| {
                        this.bg(theme::primary()).text_color(theme::primary_text())
                    })
                    .when(!is_user && !is_system, |this| {
                        this.bg(theme::surface()).text_color(theme::text_primary())
                    })
                    .when(is_system, |this| {
                        this.bg(theme::warning()).text_color(theme::text_primary())
                    })
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap_1()
                            .child(div().text_xs().opacity(0.8).child(role_label))
                            .child(div().text_sm().child(
                                // Render markdown for assistant messages
                                if is_assistant {
                                    render_markdown(&content).into_any_element()
                                } else {
                                    content.clone().into_any_element()
                                },
                            )),
                    ),
            )
    }

    /// Calculate estimated height for a message based on content
    fn calculate_message_height(&self, content: &str, _is_user: bool, _is_system: bool) -> f32 {
        // Base height for the message bubble
        let mut height = 60.0; // Base height with padding

        // Add height based on content length (approximate 20px per line)
        let lines = (content.len() / 50).max(1) as f32;
        height += lines * 20.0;

        // Add extra height for role label
        height += 20.0;

        // Add some padding
        height += 16.0;

        // Minimum height
        height.max(80.0)
    }

    /// Update virtual scroll state with current messages
    fn update_virtual_scroll_state(&mut self, _cx: &mut Context<Self>) {
        // Calculate heights for all messages
        let heights: Vec<f32> = self
            .messages
            .iter()
            .map(|msg| {
                let is_user = msg.role == "user";
                let is_system = msg.role == "system";
                self.calculate_message_height(&msg.content, is_user, is_system)
            })
            .collect();

        // Update virtual scroll state
        self.virtual_scroll_state
            .update_message_count(self.messages.len(), heights);

        log::trace!(
            "Updated virtual scroll state: {} messages",
            self.messages.len()
        );
    }

    /// Render empty state
    fn render_empty_state(&self) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            .flex_1()
            .child(div().text_2xl().mb_4().child("Chat"))
            .child(
                div()
                    .text_xl()
                    .text_color(theme::text_secondary())
                    .mb_2()
                    .child("Start a conversation"),
            )
            .child(
                div()
                    .text_sm()
                    .text_color(theme::text_disabled())
                    .child("Type a message to begin chatting with Terraphim AI"),
            )
    }

    /// Render no conversation state
    fn render_no_conversation(&self) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            .flex_1()
            .child(div().text_2xl().mb_4().child("Editor"))
            .child(
                div()
                    .text_xl()
                    .text_color(theme::text_secondary())
                    .mb_2()
                    .child("No conversation loaded"),
            )
            .child(
                div()
                    .text_sm()
                    .text_color(theme::text_disabled())
                    .child("Create a new conversation to get started"),
            )
    }
}

impl Render for ChatView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .size_full()
            .child(self.render_header(cx))
            .child(
                div()
                    .flex()
                    .flex_1()
                    .overflow_hidden()
                    .child(
                        // Main chat area
                        div()
                            .flex()
                            .flex_col()
                            .flex_1()
                            .child(
                                div()
                                    .flex_1()
                                    .child(self.render_messages(cx)),
                            )
                            .child(self.render_input(cx)),
                    )
                    .when(self.show_context_panel, |this| {
                        this.child(
                            // Context panel (sidebar)
                            div()
                                .w(px(320.0))
                                .border_l_1()
                                .border_color(theme::border())
                                .bg(theme::surface())
                                .child(
                                    div()
                                        .p_4()
                                        .child(
                                            div()
                                                .flex()
                                                .items_center()
                                                .justify_between()
                                                .mb_4()
                                                .child(
                                                    div()
                                                        .text_lg()
                                                        .font_bold()
                                                        .text_color(theme::text_primary())
                                                        .child("Context"),
                                                )
                                                .child(
                                                    Button::new("add-context-item")
                                                        .icon(IconName::Plus)
                                                        .ghost()
                                                        .on_click(cx.listener(|this, _ev, window, cx| {
                                                            this.open_add_context_modal(window, cx);
                                                        }))
                                                )
                                        )
                                        .child(
                                            div()
                                                .text_sm()
                                                .text_color(theme::text_secondary())
                                                .mb_4()
                                                .child(format!(
                                                    "{} items",
                                                    self.context_items.len()
                                                )),
                                        )
                                        .children(
                                            self.context_warning.as_ref().map(|warning| {
                                                div()
                                                    .mb_3()
                                                    .px_3()
                                                    .py_2()
                                                    .rounded_md()
                                                    .bg(theme::warning())
                                                    .text_color(theme::text_primary())
                                                    .child(
                                                        div()
                                                            .text_sm()
                                                            .font_medium()
                                                            .child("Context limits reached")
                                                    )
                                                    .child(
                                                        div()
                                                            .text_xs()
                                                            .child(warning.clone())
                                                    )
                                            })
                                        )
                                        .children(
                                            self.context_items.iter().enumerate().map(|(idx, item)| {
                                                let item_id = item.id.clone();
                                                let item_title = item.title.clone();
                                                let item_content_len = item.content.len();
                                                let item_clone = item.clone();

                                                div()
                                                    .flex()
                                                    .items_start()
                                                    .justify_between()
                                                    .px_3()
                                                    .py_2()
                                                    .mb_2()
                                                    .bg(theme::background())
                                                    .border_1()
                                                    .border_color(theme::border())
                                                    .rounded_md()
                                                    .cursor_pointer()
                                                    .hover(|style| style.bg(theme::surface_hover()))
                                                    .child(
                                                        // Clickable area for editing
                                                        Button::new(("edit-ctx", idx))
                                                            .ghost()
                                                            .flex_1()
                                                            .justify_start()
                                                            .on_click(cx.listener(move |this, _ev, window, cx| {
                                                                // Click to edit
                                                                this.open_edit_context_modal(item_clone.clone(), window, cx);
                                                            }))
                                                            .child(
                                                                div()
                                                                    .flex()
                                                                    .flex_col()
                                                                    .gap_1()
                                                                    .child(
                                                                        div()
                                                                            .text_sm()
                                                                            .font_medium()
                                                                            .text_color(theme::text_primary())
                                                                            .child(item_title)
                                                                    )
                                                                    .child(
                                                                        div()
                                                                            .text_xs()
                                                                            .text_color(theme::text_secondary())
                                                                            .child(format!("{} chars", item_content_len))
                                                                    )
                                                            )
                                                    )
                                                    .child(
                                                        Button::new(("delete-ctx", idx))
                                                            .icon(IconName::Delete)
                                                            .ghost()
                                                            .on_click(cx.listener(move |this, _ev, _window, cx| {
                                                                this.handle_delete_context(item_id.clone(), cx);
                                                            }))
                                                    )
                                            })
                                        ),
                                ),
                        )
                    }),
            )
            .child(self.context_edit_modal.clone()) // Render context edit modal
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chat_message_roles() {
        let user_msg = ChatMessage::user("Hello".to_string());
        let assistant_msg = ChatMessage::assistant("Hi there".to_string(), None);
        let system_msg = ChatMessage::system("System message".to_string());

        assert_eq!(user_msg.role, "user");
        assert_eq!(assistant_msg.role, "assistant");
        assert_eq!(system_msg.role, "system");
    }
}
