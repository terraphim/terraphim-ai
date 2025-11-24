use gpui::*;

use terraphim_desktop_gpui::{EditorState, SlashCommand, SlashCommandManager};

/// Editor view with markdown editing and slash commands
pub struct EditorView {
    editor_state: EditorState,
    slash_manager: SlashCommandManager,
    show_command_palette: bool,
    command_input: SharedString,
    filtered_commands: Vec<SlashCommand>,
}

impl EditorView {
    pub fn new(_cx: &mut ViewContext<Self>) -> Self {
        log::info!("EditorView initialized");

        Self {
            editor_state: EditorState::new(),
            slash_manager: SlashCommandManager::new(),
            show_command_palette: false,
            command_input: "".into(),
            filtered_commands: Vec::new(),
        }
    }

    /// Insert text at cursor
    pub fn insert_text(&mut self, text: &str, cx: &mut ViewContext<Self>) {
        self.editor_state.insert_text(text);
        cx.notify();
    }

    /// Get current content
    pub fn get_content(&self) -> String {
        self.editor_state.get_content()
    }

    /// Clear editor content
    pub fn clear(&mut self, cx: &mut ViewContext<Self>) {
        self.editor_state.clear();
        cx.notify();
    }

    /// Show command palette
    fn show_command_palette(&mut self, cx: &mut ViewContext<Self>) {
        self.show_command_palette = true;
        self.command_input = "/".into();
        self.update_command_suggestions();
        cx.notify();
    }

    /// Hide command palette
    fn hide_command_palette(&mut self, cx: &mut ViewContext<Self>) {
        self.show_command_palette = false;
        self.command_input = "".into();
        self.filtered_commands.clear();
        cx.notify();
    }

    /// Update command suggestions based on input
    fn update_command_suggestions(&mut self) {
        let input = self.command_input.to_string();
        let prefix = input.trim_start_matches('/');

        if prefix.is_empty() {
            self.filtered_commands = self.slash_manager.list_commands().to_vec();
        } else {
            self.filtered_commands = self.slash_manager.suggest_commands(prefix);
        }
    }

    /// Execute a slash command
    fn execute_command(&mut self, command: &str, args: &str, cx: &mut ViewContext<Self>) {
        log::info!("Executing command: /{} {}", command, args);

        let slash_manager = self.slash_manager.clone();
        let command = command.to_string();
        let args = args.to_string();

        cx.spawn(|this, mut cx| async move {
            match slash_manager.execute_command(&command, &args).await {
                Ok(result) => {
                    this.update(&mut cx, |this, cx| {
                        this.editor_state.insert_text(&result);
                        this.editor_state.insert_text("\n\n");
                        cx.notify();
                    }).ok();
                }
                Err(e) => {
                    log::error!("Command execution failed: {}", e);
                    this.update(&mut cx, |this, cx| {
                        this.editor_state.insert_text(&format!("Error: {}\n\n", e));
                        cx.notify();
                    }).ok();
                }
            }
        }).detach();

        self.hide_command_palette(cx);
    }

    /// Render editor header with toolbar
    fn render_header(&self, _cx: &ViewContext<Self>) -> impl IntoElement {
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
                            .child("📝"),
                    )
                    .child(
                        div()
                            .text_lg()
                            .font_bold()
                            .text_color(rgb(0x363636))
                            .child("Markdown Editor"),
                    ),
            )
            .child(
                div()
                    .flex()
                    .gap_2()
                    .child(self.render_toolbar_button("📋", "Commands"))
                    .child(self.render_toolbar_button("💾", "Save"))
                    .child(self.render_toolbar_button("🗑️", "Clear")),
            )
    }

    /// Render toolbar button
    fn render_toolbar_button(&self, icon: &str, _label: &str) -> impl IntoElement {
        div()
            .px_3()
            .py_2()
            .rounded_md()
            .bg(rgb(0xf5f5f5))
            .hover(|style| style.bg(rgb(0xe8e8e8)).cursor_pointer())
            .child(icon)
    }

    /// Render editor statistics
    fn render_stats(&self, _cx: &ViewContext<Self>) -> impl IntoElement {
        div()
            .flex()
            .items_center()
            .gap_4()
            .px_6()
            .py_2()
            .bg(rgb(0xf9f9f9))
            .border_t_1()
            .border_color(rgb(0xdbdbdb))
            .text_xs()
            .text_color(rgb(0x7a7a7a))
            .child(format!("Lines: {}", self.editor_state.line_count()))
            .child("•")
            .child(format!("Characters: {}", self.editor_state.char_count()))
            .child("•")
            .child(if self.editor_state.is_modified() {
                "Modified"
            } else {
                "Saved"
            })
    }

    /// Render command palette
    fn render_command_palette(&self, _cx: &ViewContext<Self>) -> impl IntoElement {
        div()
            .absolute()
            .top(px(100.0))
            .left_1_2()
            .w(px(500.0))
            .bg(rgb(0xffffff))
            .border_1()
            .border_color(rgb(0xdbdbdb))
            .rounded_lg()
            .shadow_xl()
            .z_index(100)
            .overflow_hidden()
            .child(
                // Input
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .px_4()
                    .py_3()
                    .border_b_1()
                    .border_color(rgb(0xf0f0f0))
                    .child(
                        div()
                            .text_lg()
                            .child("⌘"),
                    )
                    .child(
                        div()
                            .flex_1()
                            .text_sm()
                            .child(self.command_input.clone()),
                    ),
            )
            .child(
                // Command list
                div()
                    .flex()
                    .flex_col()
                    .max_h(px(400.0))
                    .overflow_y_scroll()
                    .children(
                        if self.filtered_commands.is_empty() {
                            vec![self.render_no_commands()]
                        } else {
                            self.filtered_commands
                                .iter()
                                .map(|cmd| self.render_command_item(cmd))
                                .collect()
                        },
                    ),
            )
    }

    /// Render a command palette item
    fn render_command_item(&self, command: &SlashCommand) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .gap_1()
            .px_4()
            .py_3()
            .hover(|style| style.bg(rgb(0xf5f5f5)).cursor_pointer())
            .border_b_1()
            .border_color(rgb(0xf9f9f9))
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .child(
                        div()
                            .text_sm()
                            .font_medium()
                            .text_color(rgb(0x3273dc))
                            .child(format!("/{}", command.name)),
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(rgb(0xb5b5b5))
                            .child(command.syntax.clone()),
                    ),
            )
            .child(
                div()
                    .text_xs()
                    .text_color(rgb(0x7a7a7a))
                    .child(command.description.clone()),
            )
    }

    /// Render no commands message
    fn render_no_commands(&self) -> impl IntoElement {
        div()
            .flex()
            .items_center()
            .justify_center()
            .py_8()
            .child(
                div()
                    .text_sm()
                    .text_color(rgb(0xb5b5b5))
                    .child("No commands found"),
            )
    }

    /// Render the main editor area
    fn render_editor_area(&self, _cx: &ViewContext<Self>) -> impl IntoElement {
        let content = self.editor_state.get_content();

        div()
            .flex_1()
            .p_6()
            .overflow_y_scroll()
            .child(
                if content.is_empty() {
                    div()
                        .flex()
                        .flex_col()
                        .items_center()
                        .justify_center()
                        .h_full()
                        .gap_4()
                        .child(
                            div()
                                .text_6xl()
                                .child("📝"),
                        )
                        .child(
                            div()
                                .text_xl()
                                .text_color(rgb(0x7a7a7a))
                                .child("Start typing..."),
                        )
                        .child(
                            div()
                                .text_sm()
                                .text_color(rgb(0xb5b5b5))
                                .child("Type / to see available commands"),
                        )
                } else {
                    div()
                        .font_family("monospace")
                        .text_sm()
                        .line_height(px(24.0))
                        .text_color(rgb(0x363636))
                        .whitespace_pre_wrap()
                        .child(content)
                }
            )
    }
}

impl Render for EditorView {
    fn render(&mut self, cx: &mut ViewContext<Self>) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .size_full()
            .relative()
            .child(self.render_header(cx))
            .child(self.render_editor_area(cx))
            .child(self.render_stats(cx))
            .when(self.show_command_palette, |this| {
                this.child(self.render_command_palette(cx))
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_editor_state_operations() {
        let mut editor = EditorState::new();

        editor.insert_text("Hello, world!");
        assert_eq!(editor.char_count(), 13);

        editor.insert_text("\nNew line");
        assert_eq!(editor.line_count(), 2);

        assert!(editor.is_modified());
    }

    #[test]
    fn test_slash_command_manager() {
        let manager = SlashCommandManager::new();
        let commands = manager.list_commands();

        assert!(!commands.is_empty());
        assert!(commands.iter().any(|cmd| cmd.name == "date"));
        assert!(commands.iter().any(|cmd| cmd.name == "time"));
        assert!(commands.iter().any(|cmd| cmd.name == "search"));
    }
}
