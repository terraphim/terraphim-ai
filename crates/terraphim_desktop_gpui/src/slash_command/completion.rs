//! CompletionProvider implementation for slash commands
//!
//! Integrates the slash command system with gpui-component's Input completion system,
//! providing native keyboard navigation (up/down/enter) for slash command suggestions.

use gpui::{Context, Task, Window};
use gpui_component::input::{CompletionProvider, InputState};
use lsp_types::{
    CompletionContext, CompletionItem, CompletionItemKind, CompletionResponse, InsertTextFormat,
};
use ropey::Rope;
use std::sync::Arc;

use super::{CommandRegistry, SuggestionAction, ViewScope};

/// CompletionProvider for slash commands
///
/// Integrates with the Input component's built-in completion system to provide
/// proper keyboard navigation for slash command suggestions.
pub struct SlashCommandCompletionProvider {
    registry: Arc<CommandRegistry>,
    view_scope: ViewScope,
}

impl SlashCommandCompletionProvider {
    /// Create a new slash command completion provider
    pub fn new(registry: Arc<CommandRegistry>, view_scope: ViewScope) -> Self {
        Self {
            registry,
            view_scope,
        }
    }

    /// Create with default registry
    pub fn with_defaults(view_scope: ViewScope) -> Self {
        Self::new(
            Arc::new(CommandRegistry::with_builtin_commands()),
            view_scope,
        )
    }
}

impl CompletionProvider for SlashCommandCompletionProvider {
    fn completions(
        &self,
        text: &Rope,
        offset: usize,
        _trigger: CompletionContext,
        _window: &mut Window,
        _cx: &mut Context<InputState>,
    ) -> Task<Result<CompletionResponse, anyhow::Error>> {
        // Get text up to cursor
        let text_before_cursor: String = text.slice(..offset.min(text.len())).chars().collect();

        // Find the start of the current line
        let line_start = text_before_cursor.rfind('\n').map(|p| p + 1).unwrap_or(0);
        let line_text = &text_before_cursor[line_start..];

        // Check if line starts with /
        if !line_text.starts_with('/') {
            return Task::ready(Ok(CompletionResponse::Array(vec![])));
        }

        // Extract query after /
        let query = &line_text[1..];
        log::debug!("Slash command completion query: '{}'", query);

        // Get suggestions from registry
        let suggestions = self.registry.suggest(query, self.view_scope, 10);

        let items: Vec<CompletionItem> = suggestions
            .into_iter()
            .map(|suggestion| {
                // Determine what text to insert
                let insert_text = match &suggestion.action {
                    SuggestionAction::Insert { text, .. } => text.clone(),
                    SuggestionAction::ExecuteCommand { command_id, .. } => {
                        // For commands, insert the full command with a trailing space
                        format!("/{} ", command_id)
                    }
                    _ => format!("/{}", suggestion.id),
                };

                // Calculate the range to replace (from / to cursor)
                // The Input's completion system will handle the replacement

                CompletionItem {
                    label: format!("/{}", suggestion.id),
                    kind: Some(CompletionItemKind::FUNCTION),
                    detail: suggestion.description.clone(),
                    documentation: suggestion
                        .description
                        .clone()
                        .map(|d| lsp_types::Documentation::String(d)),
                    insert_text: Some(insert_text),
                    insert_text_format: Some(InsertTextFormat::PLAIN_TEXT),
                    // Use filter_text to enable fuzzy matching
                    filter_text: Some(suggestion.id.clone()),
                    sort_text: Some(format!("{:05}", suggestion.score as i32)),
                    ..Default::default()
                }
            })
            .collect();

        log::debug!("Returning {} slash command completions", items.len());
        Task::ready(Ok(CompletionResponse::Array(items)))
    }

    fn is_completion_trigger(
        &self,
        _offset: usize,
        new_text: &str,
        _cx: &mut Context<InputState>,
    ) -> bool {
        // Trigger completion when user types /
        new_text == "/"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_provider_creation() {
        let provider = SlashCommandCompletionProvider::with_defaults(ViewScope::Editor);
        assert!(Arc::strong_count(&provider.registry) == 1);
    }

    #[test]
    fn test_is_trigger() {
        let provider = SlashCommandCompletionProvider::with_defaults(ViewScope::Editor);
        // Note: We can't fully test is_completion_trigger without a Context,
        // but we can verify the provider is created correctly
        assert_eq!(provider.view_scope, ViewScope::Editor);
    }
}
