//! Chat widget for LLM interaction
//!
//! This module provides the chat interface for interacting with LLM models
//! with context management and streaming responses.

use eframe::egui;
use uuid::Uuid;

use crate::state::{AppState, ChatMessage, ChatMessageRole, ChatMessageMetadata};

/// Chat widget for LLM interaction
pub struct ChatWidget {
    /// Current message input
    input: String,
    /// Error message
    error: Option<String>,
}

impl ChatWidget {
    /// Create a new ChatWidget
    pub fn new(_state: &AppState) -> Self {
        Self {
            input: String::new(),
            error: None,
        }
    }

    /// Render the chat widget
    pub fn render(&mut self, ui: &mut egui::Ui, state: &AppState) {
        // Header
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new("💬 Chat").heading().strong());

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("Clear Chat").on_hover_text("Clear conversation history").clicked() {
                    state.clear_conversation();
                    self.input.clear();
                }
            });
        });

        ui.add_space(8.0);

        // Display conversation history
        let history = state.get_conversation_history();
        let history_clone = history.clone();
        drop(history);

        egui::ScrollArea::vertical()
            .max_height(400.0)
            .show(ui, |ui| {
                if history_clone.is_empty() {
                    ui.centered_and_justified(|ui| {
                        ui.label(
                            egui::RichText::new("No messages yet")
                                .weak()
                                .italics(),
                        );
                        ui.add_space(4.0);
                        ui.label(
                            egui::RichText::new("Start a conversation or add context from search")
                                .small()
                                .weak(),
                        );
                    });
                } else {
                    for message in &history_clone {
                        self.render_message(ui, message);
                    }
                }
            });

        ui.add_space(8.0);

        // Context indicator
        let context = state.get_context_manager();
        let context_size = context.selected_documents.len();
        drop(context);

        if context_size > 0 {
            egui::Frame::group(ui.style())
                .show(ui, |ui| {
                    ui.label(egui::RichText::new(format!("📋 Using {} context items", context_size)).small());
                });
            ui.add_space(4.0);
        }

        // Error message
        if let Some(ref error) = self.error {
            egui::Frame::group(ui.style())
                .fill(egui::Color32::from_rgb(50, 0, 0))
                .show(ui, |ui| {
                    ui.label(egui::RichText::new(format!("⚠️ Error: {}", error)).small());
                });
            ui.add_space(4.0);
        }

        // Input area
        ui.horizontal_wrapped(|ui| {
            ui.add_space(4.0);

            let text_edit_output = egui::TextEdit::multiline(&mut self.input)
                .hint_text("Type your message... (Press Send button)")
                .desired_width(ui.available_width() - 100.0)
                .show(ui);

            // Enter key to send
            if text_edit_output.response.lost_focus()
                && ui.input(|i| i.key_pressed(egui::Key::Enter))
            {
                self.send_message(state);
            }

            ui.add_space(4.0);

            let send_button = ui.button("Send");

            if send_button.clicked() {
                self.send_message(state);
            }

            // Use Context toggle
            ui.checkbox(
                &mut true, // TODO: Make this configurable
                "Use Context",
            );
        });

        ui.add_space(4.0);
    }

    fn render_message(&self, ui: &mut egui::Ui, message: &ChatMessage) {
        let is_user = matches!(message.role, ChatMessageRole::User);

        let bg_color = if is_user {
            egui::Color32::from_rgb(0, 100, 200).linear_multiply(0.3)
        } else {
            egui::Color32::from_rgb(50, 50, 50).linear_multiply(0.3)
        };

        let align = if is_user {
            egui::Align::RIGHT
        } else {
            egui::Align::LEFT
        };

        ui.with_layout(egui::Layout::top_down(align), |ui| {
            egui::Frame::default()
                .fill(bg_color)
                .inner_margin(8.0)
                .show(ui, |ui| {
                    // Message header
                    ui.horizontal(|ui| {
                        let role_text = match message.role {
                            ChatMessageRole::User => "👤 You",
                            ChatMessageRole::Assistant => "🤖 Assistant",
                            ChatMessageRole::System => "⚙️ System",
                        };
                        ui.label(egui::RichText::new(role_text).small().weak());
                        ui.add_space(8.0);
                        ui.label(
                            egui::RichText::new(
                                message.timestamp.format("%H:%M:%S").to_string()
                            )
                            .small()
                            .weak(),
                        );
                    });

                    ui.add_space(4.0);

                    // Message content
                    ui.label(&message.content);

                    // Message metadata
                    if let Some(ref metadata) = message.metadata {
                        ui.add_space(4.0);
                        ui.separator();
                        ui.horizontal_wrapped(|ui| {
                            if let Some(tokens) = metadata.tokens_used {
                                ui.label(egui::RichText::new(format!("{} tokens", tokens)).small().weak());
                            }
                            if let Some(ref model) = metadata.model {
                                ui.add_space(8.0);
                                ui.label(egui::RichText::new(model).small().weak());
                            }
                            if let Some(time) = metadata.processing_time_ms {
                                ui.add_space(8.0);
                                ui.label(egui::RichText::new(format!("{}ms", time)).small().weak());
                            }
                        });
                    }
                });
        });
    }

    fn send_message(&mut self, state: &AppState) {
        let message_text = self.input.trim().to_string();
        if message_text.is_empty() {
            return;
        }

        // Clear error
        self.error = None;

        // Add user message
        let user_message = ChatMessage {
            id: Uuid::new_v4(),
            role: ChatMessageRole::User,
            content: message_text.clone(),
            timestamp: chrono::Utc::now(),
            metadata: None,
        };
        state.add_chat_message(user_message);
        self.input.clear();

        // For now, just add a placeholder response
        // TODO: Implement async LLM call properly
        let assistant_message = ChatMessage {
            id: Uuid::new_v4(),
            role: ChatMessageRole::Assistant,
            content: format!("Echo: {}", message_text),
            timestamp: chrono::Utc::now(),
            metadata: Some(ChatMessageMetadata {
                tokens_used: Some(message_text.len() / 4),
                model: Some("echo".to_string()),
                context_documents: vec![],
                processing_time_ms: Some(1),
            }),
        };
        state.add_chat_message(assistant_message);
    }
}
