use gpui::*;
use gpui::prelude::FluentBuilder;
use gpui_component::button::*;
use gpui_component::input::{Input, InputEvent as GpuiInputEvent, InputState};
use gpui_component::{IconName, StyledExt};
use std::sync::Arc;
use terraphim_config::ConfigState;
use terraphim_service::context::{ContextConfig, ContextManager as TerraphimContextManager};
use terraphim_service::llm;
use terraphim_types::{ChatMessage, ContextItem, Conversation, ConversationId, RoleName};
use tokio::sync::Mutex as TokioMutex;

/// Chat view with real ContextManager and LLM backend
pub struct ChatView {
    context_manager: Arc<TokioMutex<TerraphimContextManager>>,
    config_state: Option<ConfigState>,
    current_conversation_id: Option<ConversationId>,
    current_role: RoleName,
    messages: Vec<ChatMessage>,
    input_state: Option<Entity<InputState>>,
    is_sending: bool,
    show_context_panel: bool,
    context_items: Vec<ContextItem>,
    _subscriptions: Vec<Subscription>,
}

impl ChatView {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        log::info!("ChatView initialized with backend ContextManager");

        // Initialize ContextManager using Tauri pattern (cmd.rs:937-947)
        let context_manager = Arc::new(TokioMutex::new(
            TerraphimContextManager::new(ContextConfig::default())
        ));

        // Initialize input for message composition
        let input_state = cx.new(|cx| InputState::new(window, cx).placeholder("Type your message..."));

        // Subscribe to input events for message sending
        let input_clone = input_state.clone();
        let input_sub = cx.subscribe_in(&input_state, window, move |this, _, ev: &GpuiInputEvent, _window, cx| {
            match ev {
                GpuiInputEvent::PressEnter { .. } => {
                    let value = input_clone.read(cx).value();
                    if !value.is_empty() {
                        this.send_message(value.to_string(), cx);
                        // Input will keep text (clearing not critical for now)
                    }
                }
                _ => {}
            }
        });

        Self {
            context_manager,
            config_state: None,
            current_conversation_id: None,
            current_role: RoleName::from("Terraphim Engineer"),
            messages: Vec::new(),
            input_state: Some(input_state),
            is_sending: false,
            show_context_panel: true,
            context_items: Vec::new(),
            _subscriptions: vec![input_sub],
        }
    }

    /// Initialize with config for LLM access
    pub fn with_config(mut self, config_state: ConfigState) -> Self {
        self.config_state = Some(config_state);
        self
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
                        cx.notify();
                    }).ok();
                }
                Err(e) => {
                    log::error!("❌ Failed to create conversation: {}", e);
                }
            }
        }).detach();
    }

    /// Add context to current conversation (pattern from Tauri cmd.rs:1078-1140)
    pub fn add_context(&mut self, context_item: ContextItem, cx: &mut Context<Self>) {
        let conv_id = match &self.current_conversation_id {
            Some(id) => id.clone(),
            None => {
                log::warn!("Cannot add context: no active conversation");
                return;
            }
        };

        let manager = self.context_manager.clone();

        cx.spawn(async move |this, cx| {
            let mut mgr = manager.lock().await;

            match mgr.add_context(&conv_id, context_item.clone()) {
                Ok(()) => {
                    log::info!("✅ Added context to conversation");

                    this.update(cx, |this, cx| {
                        this.context_items.push(context_item);
                        cx.notify();
                    }).ok();
                }
                Err(e) => {
                    log::error!("❌ Failed to add context: {}", e);
                }
            }
        }).detach();
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

            match mgr.delete_context(&conv_id, &context_id) {
                Ok(()) => {
                    log::info!("✅ Deleted context: {}", context_id);

                    this.update(cx, |this, cx| {
                        this.context_items.retain(|item| item.id != context_id);
                        cx.notify();
                    }).ok();
                }
                Err(e) => {
                    log::error!("❌ Failed to delete context: {}", e);
                }
            }
        }).detach();
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
                    }).ok();
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
                        let response = format!("Simulated response (no LLM configured): {}", content);
                        this.messages.push(ChatMessage::assistant(response, Some("simulated".to_string())));
                        this.is_sending = false;
                        cx.notify();
                    }).ok();
                    return;
                }
            };

            // Build messages with context (Tauri pattern cmd.rs:1769-1816)
            let mut messages_json: Vec<serde_json::Value> = Vec::new();

            // Inject context if conversation exists
            if let Some(conv_id) = &conv_id {
                let manager = context_manager.lock().await;
                if let Some(conversation) = manager.get_conversation(conv_id) {
                    if !conversation.global_context.is_empty() {
                        let mut context_content = String::from("=== CONTEXT ===\n");
                        for (idx, item) in conversation.global_context.iter().enumerate() {
                            context_content.push_str(&format!("{}. {}\n{}\n\n", idx + 1, item.title, item.content));
                        }
                        context_content.push_str("=== END CONTEXT ===\n");
                        messages_json.push(serde_json::json!({"role": "system", "content": context_content}));
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
                        this.messages.push(ChatMessage::assistant(reply, Some(llm_client.name().to_string())));
                        this.is_sending = false;
                        cx.notify();
                    }).ok();
                }
                Err(e) => {
                    log::error!("❌ LLM call failed: {}", e);
                    this.update(cx, |this, cx| {
                        this.messages.push(ChatMessage::system(format!("Error: {}", e)));
                        this.is_sending = false;
                        cx.notify();
                    }).ok();
                }
            }
        }).detach();
    }

    /// Toggle context panel visibility
    pub fn toggle_context_panel(&mut self, _event: &ClickEvent, _window: &mut Window, cx: &mut Context<Self>) {
        self.show_context_panel = !self.show_context_panel;
        log::info!("Context panel {}", if self.show_context_panel { "shown" } else { "hidden" });
        cx.notify();
    }

    /// Handle delete context button click
    fn handle_delete_context(&mut self, context_id: String, cx: &mut Context<Self>) {
        log::info!("Deleting context: {}", context_id);
        self.delete_context(context_id, cx);
    }

    /// Handle create new conversation button
    fn handle_create_conversation(&mut self, _event: &ClickEvent, _window: &mut Window, cx: &mut Context<Self>) {
        log::info!("Creating new conversation");
        self.create_conversation("New Conversation".to_string(), self.current_role.clone(), cx);
    }

    /// Render chat header
    fn render_header(&self, cx: &Context<Self>) -> impl IntoElement {
        let title = self.current_conversation_id
            .as_ref()
            .map(|id| format!("Conversation {}", id.as_str().chars().take(8).collect::<String>()))
            .unwrap_or_else(|| "No Conversation".to_string());

        div()
            .flex()
            .items_center()
            .justify_between()
            .px_6()
            .py_4()
            .border_b_1()
            .border_color(rgb(0xdbdbdb))
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_3()
                    .child(
                        div()
                            .text_2xl()
                            .child("💬"),
                    )
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .child(
                                div()
                                    .text_lg()
                                    .font_bold()
                                    .text_color(rgb(0x363636))
                                    .child(title),
                            )
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(rgb(0x7a7a7a))
                                    .child(format!("{} messages", self.messages.len()))
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
                            .on_click(cx.listener(Self::toggle_context_panel))
                    )
                    .child(
                        // New conversation button
                        Button::new("new-conversation")
                            .label("New")
                            .icon(IconName::Plus)
                            .outline()
                            .on_click(cx.listener(Self::handle_create_conversation))
                    ),
            )
    }

    /// Render message input with real Input component
    fn render_input(&self, _cx: &Context<Self>) -> impl IntoElement {
        div()
            .flex()
            .gap_2()
            .px_6()
            .py_4()
            .border_t_1()
            .border_color(rgb(0xdbdbdb))
            .children(
                self.input_state.as_ref().map(|input| {
                    div()
                        .flex_1()
                        .child(Input::new(input))
                })
            )
            .child(
                div()
                    .px_6()
                    .py_3()
                    .rounded_md()
                    .bg(if self.is_sending {
                        rgb(0xdbdbdb)
                    } else {
                        rgb(0x3273dc)
                    })
                    .text_color(rgb(0xffffff))
                    .when(!self.is_sending, |this| {
                        this.hover(|style| style.bg(rgb(0x2366d1)).cursor_pointer())
                    })
                    .child(if self.is_sending {
                        "Sending..."
                    } else {
                        "Send"
                    }),
            )
    }

    /// Render messages area
    fn render_messages(&self, _cx: &Context<Self>) -> impl IntoElement {
        if self.messages.is_empty() {
            return self.render_empty_state().into_any_element();
        }

        div()
            .flex()
            .flex_col()
            .gap_4()
            .px_6()
            .py_4()
            .children(
                self.messages.iter().map(|msg| self.render_message(msg))
            )
            .into_any_element()
    }

    /// Render a single message
    fn render_message(&self, message: &ChatMessage) -> impl IntoElement {
        let is_user = message.role == "user";
        let is_system = message.role == "system";
        let role_label = match message.role.as_str() {
            "user" => "You".to_string(),
            "system" => "System".to_string(),
            "assistant" => message.model.as_deref().unwrap_or("Assistant").to_string(),
            _ => "Unknown".to_string(),
        };
        let content = message.content.clone();

        div()
            .flex()
            .when(is_user, |this| this.justify_end())
            .child(
                div()
                    .max_w(px(600.0))
                    .px_4()
                    .py_3()
                    .rounded_lg()
                    .when(is_user, |this| {
                        this.bg(rgb(0x3273dc))
                            .text_color(rgb(0xffffff))
                    })
                    .when(!is_user && !is_system, |this| {
                        this.bg(rgb(0xf5f5f5))
                            .text_color(rgb(0x363636))
                    })
                    .when(is_system, |this| {
                        this.bg(rgb(0xffdd57))
                            .text_color(rgb(0x363636))
                    })
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap_1()
                            .child(
                                div()
                                    .text_xs()
                                    .opacity(0.8)
                                    .child(role_label),
                            )
                            .child(
                                div()
                                    .text_sm()
                                    .child(content),
                            ),
                    ),
            )
    }

    /// Render empty state
    fn render_empty_state(&self) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            .flex_1()
            .child(
                div()
                    .text_2xl()
                    .mb_4()
                    .child("💬"),
            )
            .child(
                div()
                    .text_xl()
                    .text_color(rgb(0x7a7a7a))
                    .mb_2()
                    .child("Start a conversation"),
            )
            .child(
                div()
                    .text_sm()
                    .text_color(rgb(0xb5b5b5))
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
            .child(
                div()
                    .text_2xl()
                    .mb_4()
                    .child("📝"),
            )
            .child(
                div()
                    .text_xl()
                    .text_color(rgb(0x7a7a7a))
                    .mb_2()
                    .child("No conversation loaded"),
            )
            .child(
                div()
                    .text_sm()
                    .text_color(rgb(0xb5b5b5))
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
                                .border_color(rgb(0xdbdbdb))
                                .bg(rgb(0xf9f9f9))
                                .child(
                                    div()
                                        .p_4()
                                        .child(
                                            div()
                                                .text_lg()
                                                .font_bold()
                                                .text_color(rgb(0x363636))
                                                .mb_4()
                                                .child("Context"),
                                        )
                                        .child(
                                            div()
                                                .text_sm()
                                                .text_color(rgb(0x7a7a7a))
                                                .child(format!(
                                                    "{} items",
                                                    self.context_items.len()
                                                )),
                                        )
                                        .children(
                                            self.context_items.iter().enumerate().map(|(idx, item)| {
                                                let item_id = item.id.clone();
                                                let item_title = item.title.clone();
                                                let item_content_len = item.content.len();

                                                div()
                                                    .flex()
                                                    .items_start()
                                                    .justify_between()
                                                    .px_3()
                                                    .py_2()
                                                    .mb_2()
                                                    .bg(rgb(0xffffff))
                                                    .border_1()
                                                    .border_color(rgb(0xdbdbdb))
                                                    .rounded_md()
                                                    .child(
                                                        div()
                                                            .flex_1()
                                                            .flex()
                                                            .flex_col()
                                                            .gap_1()
                                                            .child(
                                                                div()
                                                                    .text_sm()
                                                                    .font_medium()
                                                                    .text_color(rgb(0x363636))
                                                                    .child(item_title)
                                                            )
                                                            .child(
                                                                div()
                                                                    .text_xs()
                                                                    .text_color(rgb(0x7a7a7a))
                                                                    .child(format!("{} chars", item_content_len))
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
