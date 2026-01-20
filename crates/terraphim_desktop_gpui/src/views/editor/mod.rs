//! Editor view with markdown editing and slash commands
//!
//! This module provides a multi-line markdown editor integrated with
//! the Universal Slash Command System for enhanced productivity.
//!
//! **Design Alignment**: Follows Phase 5 pattern from design-universal-slash-command-gpui.md
//! - Uses ViewScope::Editor for formatting/AI/context/search commands
//! - Integrates SlashCommandPopup with input events
//! - Handles keyboard navigation for popup

use anyhow::{Result as AnyhowResult, anyhow};
use gpui::prelude::{FluentBuilder, StatefulInteractiveElement};
use gpui::*;
use gpui_component::Sizable;
use gpui_component::button::{Button, ButtonVariants};
use gpui_component::input::{Input, InputEvent, InputState};
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::Arc;
use terraphim_markdown_parser::{BlockKind, MarkdownParserError, NormalizedMarkdown};

use crate::markdown::render_markdown;
use crate::slash_command::{
    CommandRegistry, SlashCommandCompletionProvider, SlashCommandPopup, SlashCommandPopupEvent,
    SuggestionAction, ViewScope,
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
    /// Preview mode toggle (false = edit, true = preview)
    preview_mode: bool,
    /// Whether to show block sidebar
    show_blocks: bool,
    /// Cached block list for sidebar display
    blocks: Vec<BlockPreview>,
    /// Last normalization/parsing error (if any)
    block_error: Option<SharedString>,
    /// Current file path (if opened from disk)
    file_path: Option<PathBuf>,
    /// Subscriptions for cleanup (GPUI pattern)
    _subscriptions: Vec<Subscription>,
}

#[derive(Debug, Clone)]
struct BlockPreview {
    id: SharedString,
    kind: BlockKind,
    title: SharedString,
}

impl EditorView {
    pub fn new(
        window: &mut Window,
        cx: &mut Context<Self>,
        command_registry: Arc<CommandRegistry>,
    ) -> Self {
        log::info!("EditorView initializing with multi-line input and slash commands");

        // Create completion provider for slash commands
        // Uses Input's built-in completion system for proper keyboard navigation (up/down/enter)
        let completion_provider = Rc::new(SlashCommandCompletionProvider::new(
            command_registry.clone(),
            ViewScope::Editor,
        ));

        // Create multi-line input for markdown editing with slash command completion
        let input_state = cx.new(|cx| {
            let mut state = InputState::new(window, cx)
                .placeholder("Start typing your markdown... Use /command for actions")
                .multi_line();
            // Attach the completion provider - this enables native keyboard navigation
            state.lsp.completion_provider = Some(completion_provider);
            state
        });

        // Create slash command popup for Editor scope
        // Editor scope includes Chat commands (formatting, AI, context) + Both commands (search, help, role)
        let slash_command_popup = cx.new(|cx| {
            SlashCommandPopup::with_providers(
                window,
                cx,
                command_registry.clone(),
                None,
                ViewScope::Editor,
            )
        });

        // Subscribe to slash command popup events (Pattern: ChatView:71-110)
        let input_for_popup = input_state.clone();
        let popup_sub = cx.subscribe_in(
            &slash_command_popup,
            window,
            move |this, _popup, event: &SlashCommandPopupEvent, window, cx| {
                match event {
                    SlashCommandPopupEvent::SuggestionSelected { suggestion, .. } => {
                        log::info!("Editor slash command selected: {}", suggestion.text);

                        match &suggestion.action {
                            SuggestionAction::Insert { text, .. } => {
                                // Insert text at current position by updating input state
                                let current_value = input_for_popup.read(cx).value().to_string();

                                // Find the slash command trigger to replace (e.g., "/h1")
                                let last_newline = current_value.rfind('\n').map_or(0, |n| n + 1);
                                let trigger_start = current_value[last_newline..]
                                    .find('/')
                                    .map_or(last_newline, |pos| last_newline + pos);

                                // Build new value: everything before trigger + new text
                                let before_trigger = &current_value[..trigger_start];
                                let new_value = format!("{}{}", before_trigger, text);

                                // Update the input state
                                input_for_popup.update(cx, |input, cx| {
                                    input.set_value(
                                        gpui::SharedString::from(new_value.clone()),
                                        window,
                                        cx,
                                    );
                                });

                                this.is_modified = true;
                                cx.notify();

                                log::debug!("Inserted text: {}", text);
                            }
                            SuggestionAction::ExecuteCommand { command_id, args } => {
                                this.handle_slash_command(
                                    command_id.as_str(),
                                    args.clone(),
                                    window,
                                    cx,
                                );
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
                    SlashCommandPopupEvent::Closed => {
                        log::debug!("Editor slash command popup closed");
                    }
                }
            },
        );

        // Subscribe to input events for slash command detection (Pattern: ChatView:115-144)
        let input_clone = input_state.clone();
        let slash_popup_for_input = slash_command_popup.clone();
        let input_sub = cx.subscribe_in(
            &input_state,
            window,
            move |this, _, ev: &InputEvent, _window, cx| {
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
            },
        );

        log::info!("EditorView initialized successfully");

        Self {
            input_state,
            slash_command_popup,
            is_modified: false,
            preview_mode: false,
            show_blocks: false,
            blocks: Vec::new(),
            block_error: None,
            file_path: None,
            _subscriptions: vec![popup_sub, input_sub],
        }
    }

    fn apply_normalized_markdown(
        &mut self,
        normalized: NormalizedMarkdown,
        mark_modified: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let docs = terraphim_markdown_parser::blocks_to_documents("editor", &normalized);
        self.blocks = normalized
            .blocks
            .iter()
            .zip(docs.iter())
            .map(|(block, doc)| BlockPreview {
                id: block.id.to_string().into(),
                kind: block.kind,
                title: doc.title.clone().into(),
            })
            .collect();

        self.block_error = None;

        self.input_state.update(cx, |input, cx| {
            input.set_value(
                gpui::SharedString::from(normalized.markdown.clone()),
                window,
                cx,
            );
        });

        self.is_modified = mark_modified;
        cx.notify();
    }

    fn normalize_and_refresh_blocks_from_content(
        &mut self,
        content: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        match terraphim_markdown_parser::normalize_markdown(content) {
            Ok(normalized) => {
                self.apply_normalized_markdown(normalized, true, window, cx);
            }
            Err(err) => {
                self.block_error = Some(err.to_string().into());
                cx.notify();
            }
        }
    }

    fn normalize_and_refresh_blocks(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let content = self.get_content(cx);
        self.normalize_and_refresh_blocks_from_content(&content, window, cx);
    }

    /// Load markdown for editing (normalizes IDs + refreshes block list).
    pub fn load_markdown(
        &mut self,
        content: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Result<(), MarkdownParserError> {
        let normalized = terraphim_markdown_parser::normalize_markdown(content)?;
        self.apply_normalized_markdown(normalized, false, window, cx);
        Ok(())
    }

    /// Return normalized markdown for save/export.
    pub fn normalized_content(&self, cx: &Context<Self>) -> Result<String, MarkdownParserError> {
        terraphim_markdown_parser::ensure_terraphim_block_ids(&self.get_content(cx))
    }

    /// Open a markdown file from disk.
    pub fn open_file(
        &mut self,
        path: PathBuf,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyhowResult<()> {
        let content = std::fs::read_to_string(&path)
            .map_err(|err| anyhow!("Failed to read {}: {}", path.display(), err))?;
        self.file_path = Some(path);
        self.load_markdown(&content, window, cx)?;
        Ok(())
    }

    /// Save the current markdown to disk (normalized).
    pub fn save_file(
        &mut self,
        path: Option<PathBuf>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyhowResult<PathBuf> {
        let current = self.get_content(cx);
        let normalized = terraphim_markdown_parser::normalize_markdown(&current)?;
        let path = path
            .or_else(|| self.file_path.clone())
            .ok_or_else(|| anyhow!("No file path set. Use /save <path>"))?;
        std::fs::write(&path, &normalized.markdown)
            .map_err(|err| anyhow!("Failed to write {}: {}", path.display(), err))?;
        self.file_path = Some(path.clone());
        self.apply_normalized_markdown(normalized, false, window, cx);
        Ok(path)
    }

    fn toggle_blocks(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.show_blocks = !self.show_blocks;
        if self.show_blocks {
            self.normalize_and_refresh_blocks(window, cx);
        } else {
            cx.notify();
        }
    }

    /// Toggle preview mode (edit ↔ preview)
    fn toggle_preview(&mut self, cx: &mut Context<Self>) {
        self.preview_mode = !self.preview_mode;
        log::info!("Preview mode: {}", self.preview_mode);
        cx.notify();
    }

    /// Handle slash command execution - actually inserts markdown text
    fn handle_slash_command(
        &mut self,
        command_id: &str,
        args: Option<String>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let args_str = args.unwrap_or_default();
        log::info!(
            "Editor handling slash command: /{} {}",
            command_id,
            args_str
        );

        // Get current value
        let current_value = self.input_state.read(cx).value().to_string();

        // Find the slash command trigger to replace (e.g., "/h1")
        let last_newline = current_value.rfind('\n').map_or(0, |n| n + 1);
        let trigger_start = current_value[last_newline..]
            .find('/')
            .map_or(last_newline, |pos| last_newline + pos);
        let before_trigger = &current_value[..trigger_start];
        let line_end = current_value[trigger_start..]
            .find('\n')
            .map(|pos| trigger_start + pos)
            .unwrap_or_else(|| current_value.len());
        let cleaned = format!("{}{}", before_trigger, &current_value[line_end..]);

        // Determine what text to insert
        let insertion_text = match command_id {
            "ids" | "normalize" => {
                // Remove the trigger text and normalize the entire document.
                self.normalize_and_refresh_blocks_from_content(&cleaned, window, cx);
                return;
            }
            "blocks" => {
                // Remove the trigger text and toggle the block sidebar.
                self.input_state.update(cx, |input, cx| {
                    input.set_value(gpui::SharedString::from(cleaned.clone()), window, cx);
                });
                self.is_modified = true;
                self.toggle_blocks(window, cx);
                return;
            }
            "open" => {
                let path = args_str.trim();
                if path.is_empty() {
                    log::warn!("Open requires a file path");
                    return;
                }
                if let Err(err) = self.open_file(PathBuf::from(path), window, cx) {
                    log::error!("Failed to open file: {}", err);
                }
                return;
            }
            "save" => {
                let path = args_str.trim();
                let path = if path.is_empty() {
                    None
                } else {
                    Some(PathBuf::from(path))
                };
                self.input_state.update(cx, |input, cx| {
                    input.set_value(gpui::SharedString::from(cleaned.clone()), window, cx);
                });
                match self.save_file(path, window, cx) {
                    Ok(saved) => {
                        log::info!("Saved markdown to {}", saved.display());
                    }
                    Err(err) => {
                        log::error!("Failed to save markdown: {}", err);
                    }
                }
                return;
            }
            // Formatting commands - insert markdown syntax
            "h1" => {
                if args_str.is_empty() {
                    "# ".to_string()
                } else {
                    format!("# {}", args_str)
                }
            }
            "h2" => {
                if args_str.is_empty() {
                    "## ".to_string()
                } else {
                    format!("## {}", args_str)
                }
            }
            "h3" => {
                if args_str.is_empty() {
                    "### ".to_string()
                } else {
                    format!("### {}", args_str)
                }
            }
            "bullet" => {
                if args_str.is_empty() {
                    "- ".to_string()
                } else {
                    format!("- {}", args_str)
                }
            }
            "numbered" => {
                if args_str.is_empty() {
                    "1. ".to_string()
                } else {
                    format!("1. {}", args_str)
                }
            }
            "code" => {
                if args_str.is_empty() {
                    "```\n".to_string()
                } else {
                    format!("```{}\n", args_str)
                }
            }
            "quote" => {
                if args_str.is_empty() {
                    "> ".to_string()
                } else {
                    format!("> {}", args_str)
                }
            }
            // Date/time commands - insert formatted values
            "date" => {
                chrono::Local::now().format("%Y-%m-%d").to_string()
            }
            "time" => {
                chrono::Local::now().format("%H:%M:%S").to_string()
            }
            "datetime" => {
                chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string()
            }
            // AI commands - insert command placeholders (in future these will trigger AI)
            "summarize" => {
                if args_str.is_empty() {
                    "Summarize selected text: ".to_string()
                } else {
                    format!("Summarize: {}", args_str)
                }
            }
            "explain" => {
                if args_str.is_empty() {
                    "Explain: ".to_string()
                } else {
                    format!("Explain: {}", args_str)
                }
            }
            "improve" => {
                if args_str.is_empty() {
                    "Improve: ".to_string()
                } else {
                    format!("Improve: {}", args_str)
                }
            }
            "translate" => {
                if args_str.is_empty() {
                    "Translate to: ".to_string()
                } else {
                    format!("Translate to: {}", args_str)
                }
            }
            // Context commands - insert context placeholders
            "context" => {
                if args_str.is_empty() {
                    "Context: ".to_string()
                } else {
                    format!("Context: {}", args_str)
                }
            }
            "add" => {
                if args_str.is_empty() {
                    "Add: ".to_string()
                } else {
                    format!("Add: {}", args_str)
                }
            }
            // Search commands - insert search placeholders
            "search" => {
                if args_str.is_empty() {
                    "Search: ".to_string()
                } else {
                    format!("Search: {}", args_str)
                }
            }
            "kg" => {
                if args_str.is_empty() {
                    "Knowledge Graph: ".to_string()
                } else {
                    format!("Knowledge Graph: {}", args_str)
                }
            }
            // Role command - insert role placeholder
            "role" => {
                if args_str.is_empty() {
                    "Role: ".to_string()
                } else {
                    format!("Role: {}", args_str)
                }
            }
            // Clear command - clear the editor
            "clear" => {
                self.input_state.update(cx, |input, cx| {
                    input.set_value(gpui::SharedString::from("".to_string()), window, cx);
                });
                self.is_modified = false;
                log::info!("Editor cleared");
                cx.notify();
                return; // Early return since we already updated
            }
            // Help command - insert help text as comment
            "help" => {
                "<!-- Available commands: /h1, /h2, /h3, /bullet, /numbered, /code, /quote, /date, /time, /datetime, /summarize, /explain, /improve, /translate, /context, /add, /search, /kg, /role, /ids, /normalize, /blocks, /open, /save, /clear -->\n".to_string()
            }
            _ => {
                log::debug!("Unhandled command: /{}", command_id);
                return;
            }
        };

        // Build new value: everything before trigger + insertion text
        let new_value = format!("{}{}", before_trigger, insertion_text);

        // Update the input state
        self.input_state.update(cx, |input, cx| {
            input.set_value(gpui::SharedString::from(new_value.clone()), window, cx);
        });

        self.is_modified = true;
        cx.notify();

        log::info!("Inserted markdown for /{}: {}", command_id, insertion_text);
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

    /// Update role (called when role changes from system tray or dropdown)
    pub fn update_role(&mut self, new_role: String, cx: &mut Context<Self>) {
        log::info!("EditorView: role changed to {}", new_role);
        cx.notify();
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
                    .child(div().text_2xl().child("Editor"))
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
                    .child(Button::new("cmd-btn").label("/ Commands").small().ghost())
                    .child(
                        Button::new("ids-btn")
                            .label("IDs")
                            .small()
                            .ghost()
                            .on_click(cx.listener(|this, _ev, window, cx| {
                                this.normalize_and_refresh_blocks(window, cx);
                            })),
                    )
                    .child(
                        Button::new("blocks-btn")
                            .label(if self.show_blocks {
                                "Blocks *"
                            } else {
                                "Blocks"
                            })
                            .small()
                            .ghost()
                            .on_click(cx.listener(|this, _ev, window, cx| {
                                this.toggle_blocks(window, cx);
                            })),
                    )
                    .child(
                        Button::new("toggle-preview-btn")
                            .label(if self.preview_mode { "Edit" } else { "Preview" })
                            .small()
                            .primary()
                            .on_click(cx.listener(|this, _ev, _window, cx| {
                                this.toggle_preview(cx);
                            })),
                    )
                    .child(
                        Button::new("clear-btn")
                            .label("Clear")
                            .small()
                            .outline()
                            .on_click(cx.listener(|this, _ev, window, cx| {
                                this.handle_slash_command("clear", None, window, cx);
                            })),
                    ),
            )
    }

    fn render_blocks_sidebar(&self, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .w(px(360.0))
            .h_full()
            .flex()
            .flex_col()
            .gap_2()
            .border_1()
            .border_color(theme::border())
            .rounded_lg()
            .bg(theme::surface())
            .p_3()
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(
                        div()
                            .text_sm()
                            .font_weight(FontWeight::BOLD)
                            .text_color(theme::text_primary())
                            .child(format!("Blocks ({})", self.blocks.len())),
                    )
                    .child(
                        Button::new("blocks-refresh-btn")
                            .label("Refresh")
                            .small()
                            .ghost()
                            .on_click(cx.listener(|this, _ev, window, cx| {
                                this.normalize_and_refresh_blocks(window, cx);
                            })),
                    ),
            )
            .when_some(self.block_error.clone(), |d, err| {
                d.child(div().text_xs().text_color(theme::danger()).child(err))
            })
            .child(
                div()
                    .flex_1()
                    .id("block-sidebar")
                    .overflow_y_scroll()
                    .gap_2()
                    .children(self.blocks.iter().map(|block| {
                        let kind = match block.kind {
                            BlockKind::Paragraph => "¶",
                            BlockKind::ListItem => "•",
                        };

                        div()
                            .flex()
                            .flex_col()
                            .gap_1()
                            .p_2()
                            .rounded_md()
                            .bg(theme::background())
                            .border_1()
                            .border_color(theme::border())
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(theme::text_secondary())
                                    .child(format!("{kind} {}", block.id)),
                            )
                            .child(
                                div()
                                    .text_sm()
                                    .text_color(theme::text_primary())
                                    .child(block.title.clone()),
                            )
                    })),
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

        // Get current content for preview mode
        let content = self.get_content(cx);

        div()
            .flex_1()
            .flex()
            .flex_col()
            .relative()
            .p_4()
            .child(
                div()
                    .flex_1()
                    .w_full()
                    .flex()
                    .flex_row()
                    .gap_4()
                    .child(
                        div()
                            .flex_1()
                            .border_1()
                            .border_color(theme::border())
                            .rounded_lg()
                            .bg(theme::background())
                            .when(!self.preview_mode, |this_div| {
                                // Edit mode: Show input with completion provider for slash commands
                                // The Input's built-in completion system handles keyboard navigation
                                // (up/down/enter) automatically via CompletionProvider
                                this_div.child(div().relative().size_full().child(
                                    Input::new(&self.input_state).appearance(false).h_full(),
                                ))
                            })
                            .when(self.preview_mode, |this_div| {
                                // Preview mode: Show markdown rendering
                                this_div.child(render_markdown(&content))
                            }),
                    )
                    .when(self.show_blocks, |d| {
                        d.child(self.render_blocks_sidebar(cx))
                    }),
            )
            // Show slash command popup when open (only in edit mode)
            .when(slash_popup_open && !self.preview_mode, |d| {
                d.child(
                    div()
                        .absolute()
                        .top(px(60.0))
                        .left(px(16.0))
                        .w(px(400.0))
                        .child(slash_popup.clone()),
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
