use gpui::*;
use gpui::prelude::FluentBuilder;
use gpui_component::StyledExt;
use std::sync::Arc;
use terraphim_service::context::{ContextConfig, ContextManager as TerraphimContextManager};
use terraphim_types::{ChatMessage, ContextItem, Conversation, ConversationId, RoleName};
use tokio::sync::Mutex as TokioMutex;

/// Chat view with real ContextManager backend integration
pub struct ChatView {
    context_manager: Arc<TokioMutex<TerraphimContextManager>>,
    current_conversation_id: Option<ConversationId>,
    messages: Vec<ChatMessage>,
    message_input: SharedString,
    is_sending: bool,
    show_context_panel: bool,
    context_items: Vec<ContextItem>,
}

impl ChatView {
    pub fn new(_window: &mut Window, _cx: &mut Context<Self>) -> Self {
        log::info!("ChatView initialized with backend ContextManager");

        // Initialize ContextManager using Tauri pattern (cmd.rs:937-947)
        let context_manager = Arc::new(TokioMutex::new(
            TerraphimContextManager::new(ContextConfig::default())
        ));

        Self {
            context_manager,
            current_conversation_id: None,
            messages: Vec::new(),
            message_input: "".into(),
            is_sending: false,
            show_context_panel: true,
            context_items: Vec::new(),
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

    /// Send a message
    pub fn send_message(&mut self, content: String, cx: &mut Context<Self>) {
        if content.trim().is_empty() {
            return;
        }

        log::info!("Sending message: {}", content);

        self.is_sending = true;
        self.message_input = "".into();
        cx.notify();

        // In a real implementation, this would call the LLM service
        // For now, just add a user message and simulate a response

        // Add user message to local history
        self.messages.push(ChatMessage::user(content.clone()));

        // Simulate assistant response (will be replaced with real LLM in next phase)
        cx.spawn(async move |this, cx| {
            // Simulate network delay
            tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

            this.update(cx, |this, cx| {
                let response = format!("Simulated response to: '{}'", content);
                this.messages.push(ChatMessage::assistant(response, Some("claude-sonnet-4-5".to_string())));
                this.is_sending = false;
                cx.notify();
            }).ok();
        }).detach();
    }

    /// Toggle context panel visibility
    pub fn toggle_context_panel(&mut self, cx: &mut Context<Self>) {
        self.show_context_panel = !self.show_context_panel;
        log::info!("Context panel {}", if self.show_context_panel { "shown" } else { "hidden" });
        cx.notify();
    }

    /// Render chat header
    fn render_header(&self, _cx: &Context<Self>) -> impl IntoElement {
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
                        // Toggle context panel button
                        div()
                            .px_3()
                            .py_2()
                            .rounded_md()
                            .bg(if self.show_context_panel {
                                rgb(0x3273dc)
                            } else {
                                rgb(0xf5f5f5)
                            })
                            .text_color(if self.show_context_panel {
                                rgb(0xffffff)
                            } else {
                                rgb(0x363636)
                            })
                            .hover(|style| style.opacity(0.9).cursor_pointer())
                            .child("📚 Context"),
                    ),
            )
    }

    /// Render message input
    fn render_input(&self, _cx: &Context<Self>) -> impl IntoElement {
        div()
            .flex()
            .gap_2()
            .px_6()
            .py_4()
            .border_t_1()
            .border_color(rgb(0xdbdbdb))
            .child(
                div()
                    .flex_1()
                    .px_4()
                    .py_3()
                    .bg(rgb(0xffffff))
                    .border_1()
                    .border_color(rgb(0xdbdbdb))
                    .rounded_md()
                    .child(
                        if self.message_input.is_empty() {
                            div()
                                .text_color(rgb(0xb5b5b5))
                                .child("Type your message...")
                        } else {
                            div()
                                .text_color(rgb(0x363636))
                                .child(self.message_input.clone())
                        }
                    ),
            )
            .child(
                div()
                    .px_6()
                    .py_3()
                    .rounded_md()
                    .bg(if self.is_sending || self.message_input.is_empty() {
                        rgb(0xdbdbdb)
                    } else {
                        rgb(0x3273dc)
                    })
                    .text_color(rgb(0xffffff))
                    .when(!self.is_sending && !self.message_input.is_empty(), |this| {
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
                                            self.context_items.iter().map(|item| {
                                                div()
                                                    .px_3()
                                                    .py_2()
                                                    .mb_2()
                                                    .bg(rgb(0xffffff))
                                                    .border_1()
                                                    .border_color(rgb(0xdbdbdb))
                                                    .rounded_md()
                                                    .child(
                                                        div()
                                                            .text_sm()
                                                            .font_medium()
                                                            .text_color(rgb(0x363636))
                                                            .child(item.title.clone())
                                                    )
                                                    .child(
                                                        div()
                                                            .text_xs()
                                                            .text_color(rgb(0x7a7a7a))
                                                            .child(format!("{} chars", item.content.len()))
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
