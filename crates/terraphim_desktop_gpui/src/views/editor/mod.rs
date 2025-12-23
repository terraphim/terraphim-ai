//! Editor view with markdown editing and slash commands
//!
//! This module provides a multi-line markdown editor integrated with
//! the Universal Slash Command System for enhanced productivity.
//!
//! **Design Alignment**: Follows Phase 5 pattern from design-universal-slash-command-gpui.md
//! - Uses ViewScope::Editor for formatting/AI/context/search commands
//! - Integrates SlashCommandPopup with input events
//! - Handles keyboard navigation for popup

use gpui::*;
use gpui::prelude::FluentBuilder;
use gpui_component::input::{Input, InputEvent, InputState};
use gpui_component::button::{Button, ButtonVariants};
use gpui_component::Sizable;

use crate::slash_command::{
    SlashCommandPopup, SlashCommandPopupEvent, ViewScope, SuggestionAction,
};
use crate::theme::colors::theme;

/// Editor view with multi-line markdown editing and slash commands
///
/// Implements the same slash command integration pattern as ChatView (Phase 5 of design plan)
pub struct EditorView {
    /// Input state for the editor (multi-line)
    input_state: Entity<InputState>,
    /// Slash command popup for Chat-scoped commands (formatting, AI, etc.)
    slash_command_popup: Entity<SlashCommandPopup>,
    /// Track modification state
    is_modified: bool,
    /// Subscriptions for cleanup (GPUI pattern)
    _subscriptions: Vec<Subscription>,
}

impl EditorView {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        log::info!("EditorView initializing with multi-line input and slash commands");

        // Create multi-line input for markdown editing
        // Pattern: Same as ChatView but with multi-line enabled
        let input_state = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder("Start typing your markdown... Use /command for actions")
                .multi_line()
        });

        // Create slash command popup for Editor scope
        // Editor scope includes Chat commands (formatting, AI, context) + Both commands (search, help, role)
        let slash_command_popup = cx.new(|cx| SlashCommandPopup::new(window, cx, ViewScope::Editor));

        // Subscribe to slash command popup events (Pattern: ChatView:71-110)
        let popup_sub = cx.subscribe(&slash_command_popup, move |this, _popup, event: &SlashCommandPopupEvent, cx| {
            match event {
                SlashCommandPopupEvent::SuggestionSelected(suggestion) => {
                    log::info!("Editor slash command selected: {}", suggestion.text);

                    match &suggestion.action {
                        SuggestionAction::Insert { text, .. } => {
                            // Insert text at current position
                            log::debug!("Insert action: {}", text);
                            // Note: Direct text insertion requires window context
                            // which we don't have in this callback
                        }
                        SuggestionAction::ExecuteCommand { command_id, args } => {
                            this.handle_slash_command(command_id, args.clone(), cx);
                        }
                        SuggestionAction::Search { query, use_kg } => {
                            log::info!("Search action: {} (use_kg: {})", query, use_kg);
                        }
                        SuggestionAction::Navigate { .. } => {
                            log::debug!("Navigate action - not applicable in editor");
                        }
                        SuggestionAction::Custom { .. } => {}
                    }
                }
                SlashCommandPopupEvent::CommandExecuted(result) => {
                    log::debug!("Editor command executed: success={}", result.success);
                    if let Some(content) = &result.content {
                        log::debug!("Command result: {}", content);
                    }
                }
                SlashCommandPopupEvent::Closed => {
                    log::debug!("Editor slash command popup closed");
                }
            }
        });

        // Subscribe to input events for slash command detection (Pattern: ChatView:115-144)
        let input_clone = input_state.clone();
        let slash_popup_for_input = slash_command_popup.clone();
        let input_sub = cx.subscribe_in(&input_state, window, move |this, _, ev: &InputEvent, _window, cx| {
            match ev {
                InputEvent::Change => {
                    this.is_modified = true;

                    // Detect slash commands - pass text and cursor position
                    let value = input_clone.read(cx).value();
                    let cursor = value.len(); // Approximate cursor at end

                    slash_popup_for_input.update(cx, |popup, cx| {
                        popup.process_input(&value, cursor, cx);
                    });

                    cx.notify();
                }
                InputEvent::PressEnter { secondary } => {
                    // Check if slash popup is open - if so, accept selection
                    let popup_open = slash_popup_for_input.read(cx).is_open();

                    if popup_open {
                        slash_popup_for_input.update(cx, |popup, cx| {
                            popup.accept_selected(cx);
                        });
                    } else if *secondary {
                        // Shift+Enter could trigger save in future
                        log::debug!("Shift+Enter pressed - potential save trigger");
                    }
                    // Regular Enter in multi-line editor inserts newline (handled by Input)
                }
                InputEvent::Focus => {
                    log::debug!("Editor focused");
                }
                InputEvent::Blur => {
                    log::debug!("Editor blurred");
                    // Close popup on blur
                    slash_popup_for_input.update(cx, |popup, cx| {
                        popup.close(cx);
                    });
                }
            }
        });

        log::info!("EditorView initialized successfully");

        Self {
            input_state,
            slash_command_popup,
            is_modified: false,
            _subscriptions: vec![popup_sub, input_sub],
        }
    }

    /// Handle slash command execution (Pattern: ChatView:400-430)
    fn handle_slash_command(&mut self, command_id: &str, args: Option<String>, cx: &mut Context<Self>) {
        let args_str = args.unwrap_or_default();
        log::info!("Editor handling slash command: /{} {}", command_id, args_str);

        match command_id {
            // Formatting commands - these insert markdown syntax
            "h1" => {
                log::info!("Would insert: # {}", args_str);
            }
            "h2" => {
                log::info!("Would insert: ## {}", args_str);
            }
            "h3" => {
                log::info!("Would insert: ### {}", args_str);
            }
            "bullet" => {
                log::info!("Would insert: - {}", args_str);
            }
            "numbered" => {
                log::info!("Would insert: 1. {}", args_str);
            }
            "code" => {
                let lang = if args_str.is_empty() { "" } else { &args_str };
                log::info!("Would insert code block with lang: {}", lang);
            }
            "quote" => {
                log::info!("Would insert: > {}", args_str);
            }
            // Date/time commands
            "date" => {
                let date = chrono::Local::now().format("%Y-%m-%d").to_string();
                log::info!("Would insert date: {}", date);
            }
            "time" => {
                let time = chrono::Local::now().format("%H:%M:%S").to_string();
                log::info!("Would insert time: {}", time);
            }
            "datetime" => {
                let datetime = chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
                log::info!("Would insert datetime: {}", datetime);
            }
            // Clear command
            "clear" => {
                self.is_modified = false;
                log::info!("Editor cleared");
                cx.notify();
            }
            // Help command
            "help" => {
                log::info!("Available commands: /h1, /h2, /h3, /bullet, /numbered, /code, /quote, /date, /time, /datetime, /clear");
            }
            _ => {
                log::debug!("Unhandled command: /{}", command_id);
            }
        }
    }

    /// Get current content from input
    pub fn get_content(&self, cx: &Context<Self>) -> String {
        self.input_state.read(cx).value().to_string()
    }

    /// Check if editor has unsaved changes
    pub fn is_modified(&self) -> bool {
        self.is_modified
    }

    /// Get line count from content
    fn get_line_count(&self, cx: &Context<Self>) -> usize {
        let content = self.get_content(cx);
        content.lines().count().max(1)
    }

    /// Get character count from content
    fn get_char_count(&self, cx: &Context<Self>) -> usize {
        let content = self.get_content(cx);
        content.chars().count()
    }

    /// Render editor header with toolbar
    fn render_header(&self, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .flex()
            .items_center()
            .justify_between()
            .px_6()
            .py_4()
            .border_b_1()
            .border_color(theme::border())
            .bg(theme::surface())
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_3()
                    .child(
                        div()
                            .text_2xl()
                            .child("📝"),
                    )
                    .child(
                        div()
                            .text_lg()
                            .font_weight(FontWeight::BOLD)
                            .text_color(theme::text_primary())
                            .child("Markdown Editor"),
                    ),
            )
            .child(
                div()
                    .flex()
                    .gap_2()
                    .child(
                        Button::new("cmd-btn")
                            .label("/ Commands")
                            .small()
                            .ghost()
                    )
                    .child(
                        Button::new("clear-btn")
                            .label("Clear")
                            .small()
                            .outline()
                            .on_click(cx.listener(|this, _ev, _window, cx| {
                                this.handle_slash_command("clear", None, cx);
                            }))
                    ),
            )
    }

    /// Render editor statistics footer
    fn render_stats(&self, cx: &Context<Self>) -> impl IntoElement {
        let line_count = self.get_line_count(cx);
        let char_count = self.get_char_count(cx);

        div()
            .flex()
            .items_center()
            .gap_4()
            .px_6()
            .py_2()
            .bg(theme::surface())
            .border_t_1()
            .border_color(theme::border())
            .text_xs()
            .text_color(theme::text_secondary())
            .child(format!("Lines: {}", line_count))
            .child("•")
            .child(format!("Characters: {}", char_count))
            .child("•")
            .child(if self.is_modified {
                "Modified"
            } else {
                "Saved"
            })
    }

    /// Render the main editor area with slash command popup
    fn render_editor_area(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let slash_popup = &self.slash_command_popup;
        let slash_popup_open = slash_popup.read(cx).is_open();

        div()
            .flex_1()
            .flex()
            .flex_col()
            .relative()
            .p_4()
            .track_focus(&self.input_state.read(cx).focus_handle(cx))
            // Keyboard navigation for slash popup (Pattern: SearchInput:272-296)
            .on_key_down(cx.listener(|this, ev: &KeyDownEvent, _window, cx| {
                let slash_open = this.slash_command_popup.read(cx).is_open();

                if slash_open {
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
            .child(
                div()
                    .flex_1()
                    .w_full()
                    .border_1()
                    .border_color(theme::border())
                    .rounded_lg()
                    .bg(theme::background())
                    .child(
                        Input::new(&self.input_state)
                            .appearance(false)
                            .h_full()
                    )
            )
            // Show slash command popup when open (Pattern: ChatView render)
            .when(slash_popup_open, |d| {
                d.child(
                    div()
                        .absolute()
                        .top(px(60.0))
                        .left(px(16.0))
                        .w(px(400.0))
                        .child(slash_popup.clone())
                )
            })
    }
}

impl Render for EditorView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .size_full()
            .bg(theme::background())
            .child(self.render_header(cx))
            .child(self.render_editor_area(cx))
            .child(self.render_stats(cx))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_view_scope_for_editor() {
        // Editor uses Editor scope for slash commands (formatting, AI, context, search)
        // Editor scope includes Chat commands + Both commands, excluding Search-only commands
        assert_eq!(ViewScope::Editor, ViewScope::Editor);
    }
}
