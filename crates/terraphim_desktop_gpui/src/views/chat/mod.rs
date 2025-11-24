use gpui::*;
use gpui::prelude::FluentBuilder;
use gpui_component::StyledExt;
use std::sync::Arc;
use terraphim_types::{ChatMessage, ContextItem, Conversation, RoleName};

// Import from library crate
mod context {
    pub use gpui::*;
    pub struct ContextManager {
        items: Vec<std::sync::Arc<terraphim_types::ContextItem>>,
    }
    impl ContextManager {
        pub fn new(_cx: &mut Context<Self>) -> Self {
            Self { items: Vec::new() }
        }
        pub fn count(&self) -> usize {
            self.items.len()
        }
        pub fn clear_all(&mut self, _cx: &mut Context<Self>) {
            self.items.clear();
        }
        pub fn add_item(&mut self, item: terraphim_types::ContextItem, _cx: &mut Context<Self>) -> Result<(), String> {
            self.items.push(std::sync::Arc::new(item));
            Ok(())
        }
    }
}
use context::ContextManager;

/// Chat view with full context integration
pub struct ChatView {
    conversation: Option<Arc<Conversation>>,
    context_manager: Entity<ContextManager>,
    message_input: SharedString,
    is_sending: bool,
    show_context_panel: bool,
}

impl ChatView {
    pub fn new(_window: &mut Window, cx: &mut Context<Self>) -> Self {
        log::info!("ChatView initialized");

        let context_manager = cx.new(|cx| ContextManager::new(cx));

        Self {
            conversation: None,
            context_manager,
            message_input: "".into(),
            is_sending: false,
            show_context_panel: true,
        }
    }

    /// Initialize with a conversation
    pub fn with_conversation(mut self, conversation: Conversation) -> Self {
        self.conversation = Some(Arc::new(conversation));
        self
    }

    /// Create a new conversation
    pub fn new_conversation(&mut self, title: String, role: RoleName, cx: &mut Context<Self>) {
        log::info!("Creating new conversation: {} (role: {})", title, role);

        let conversation = Conversation::new(title, role);
        self.conversation = Some(Arc::new(conversation));

        // Clear context for new conversation
        self.context_manager.update(cx, |mgr, cx| {
            mgr.clear_all(cx);
        });

        cx.notify();
    }

    /// Add context item to conversation
    pub fn add_context(&mut self, item: ContextItem, cx: &mut Context<Self>) {
        self.context_manager.update(cx, |mgr, cx| {
            if let Err(e) = mgr.add_item(item, cx) {
                log::error!("Failed to add context item: {}", e);
            }
        });
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

        if let Some(ref mut _conversation) = self.conversation {
            // Add user message
            let _user_message = ChatMessage::user(content.clone());

            // Simulate assistant response
            cx.spawn(async move |this, cx| {
                // Simulate network delay
                tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

                this.update(cx, |this, cx| {
                    // Add simulated response
                    let _response = format!("This is a simulated response to: '{}'", content);
                    let _assistant_message = ChatMessage::assistant(_response, Some("claude-sonnet-4-5".to_string()));

                    this.is_sending = false;
                    cx.notify();
                }).ok();
            }).detach();
        }
    }

    /// Toggle context panel visibility
    pub fn toggle_context_panel(&mut self, cx: &mut Context<Self>) {
        self.show_context_panel = !self.show_context_panel;
        log::info!("Context panel {}", if self.show_context_panel { "shown" } else { "hidden" });
        cx.notify();
    }

    /// Render chat header
    fn render_header(&self, _cx: &Context<Self>) -> impl IntoElement {
        let title = self.conversation
            .as_ref()
            .map(|conv| conv.title.clone())
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
                            .children(
                                self.conversation.as_ref().map(|conv| {
                                    div()
                                        .text_xs()
                                        .text_color(rgb(0x7a7a7a))
                                        .child(format!("Role: {}", conv.role))
                                })
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
        if let Some(conversation) = &self.conversation {
            if conversation.messages.is_empty() {
                return self.render_empty_state().into_any_element();
            }

            div()
                .flex()
                .flex_col()
                .gap_4()
                .px_6()
                .py_4()
                .children(
                    conversation.messages.iter().map(|msg| self.render_message(msg))
                )
                .into_any_element()
        } else {
            self.render_no_conversation().into_any_element()
        }
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
                                                    self.context_manager.read(cx).count()
                                                )),
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
