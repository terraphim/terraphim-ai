//! Core types for the Universal Slash Command System
//!
//! This module defines the foundational types used across all slash command
//! components including commands, suggestions, triggers, and execution context.

use serde::{Deserialize, Serialize};
use std::fmt;
use std::sync::Arc;

/// View scope for commands - determines where commands are available
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ViewScope {
    /// Commands available in ChatView
    Chat,
    /// Commands available in SearchInput
    Search,
    /// Commands available in EditorView
    Editor,
    /// Commands available in both views
    Both,
}

impl ViewScope {
    /// Check if this scope includes the given scope
    pub fn includes(&self, other: ViewScope) -> bool {
        match self {
            ViewScope::Both => true,
            ViewScope::Chat => other == ViewScope::Chat || other == ViewScope::Both,
            ViewScope::Search => other == ViewScope::Search || other == ViewScope::Both,
            ViewScope::Editor => other == ViewScope::Editor || other == ViewScope::Both,
        }
    }
}

impl fmt::Display for ViewScope {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ViewScope::Chat => write!(f, "Chat"),
            ViewScope::Search => write!(f, "Search"),
            ViewScope::Editor => write!(f, "Editor"),
            ViewScope::Both => write!(f, "Both"),
        }
    }
}

/// Command category for organization and filtering
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CommandCategory {
    /// Text formatting commands (heading, bold, list)
    Formatting,
    /// Search and navigation commands
    Search,
    /// AI-powered commands (summarize, explain)
    AI,
    /// Context management commands
    Context,
    /// Editor actions (insert, replace)
    Editor,
    /// System commands (settings, help)
    System,
    /// Knowledge graph commands
    KnowledgeGraph,
}

impl fmt::Display for CommandCategory {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CommandCategory::Formatting => write!(f, "Formatting"),
            CommandCategory::Search => write!(f, "Search"),
            CommandCategory::AI => write!(f, "AI"),
            CommandCategory::Context => write!(f, "Context"),
            CommandCategory::Editor => write!(f, "Editor"),
            CommandCategory::System => write!(f, "System"),
            CommandCategory::KnowledgeGraph => write!(f, "Knowledge Graph"),
        }
    }
}

/// Icon representation for commands
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum CommandIcon {
    /// Unicode emoji
    Emoji(String),
    /// Icon name from gpui-component IconName
    Named(String),
    /// No icon
    None,
}

impl Default for CommandIcon {
    fn default() -> Self {
        CommandIcon::None
    }
}

/// Action to execute when a suggestion is selected
#[derive(Clone)]
pub enum SuggestionAction {
    /// Insert text at cursor position
    Insert {
        text: String,
        /// Whether to replace the trigger text
        replace_trigger: bool,
    },
    /// Execute a command by ID
    ExecuteCommand {
        command_id: String,
        args: Option<String>,
    },
    /// Trigger a search with the given query
    Search {
        query: String,
        /// Use KG-enhanced search
        use_kg: bool,
    },
    /// Navigate to a view or open a modal
    Navigate {
        target: String,
        data: Option<String>,
    },
    /// Custom action with callback
    Custom(Arc<dyn Fn() + Send + Sync>),
}

impl fmt::Debug for SuggestionAction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SuggestionAction::Insert { text, replace_trigger } => {
                f.debug_struct("Insert")
                    .field("text", text)
                    .field("replace_trigger", replace_trigger)
                    .finish()
            }
            SuggestionAction::ExecuteCommand { command_id, args } => {
                f.debug_struct("ExecuteCommand")
                    .field("command_id", command_id)
                    .field("args", args)
                    .finish()
            }
            SuggestionAction::Search { query, use_kg } => {
                f.debug_struct("Search")
                    .field("query", query)
                    .field("use_kg", use_kg)
                    .finish()
            }
            SuggestionAction::Navigate { target, data } => {
                f.debug_struct("Navigate")
                    .field("target", target)
                    .field("data", data)
                    .finish()
            }
            SuggestionAction::Custom(_) => write!(f, "Custom(<fn>)"),
        }
    }
}

/// Result of command execution
#[derive(Clone, Debug)]
pub struct CommandResult {
    /// Whether execution succeeded
    pub success: bool,
    /// Result content (text to insert, message to show, etc.)
    pub content: Option<String>,
    /// Error message if failed
    pub error: Option<String>,
    /// Whether to close the popup after execution
    pub close_popup: bool,
    /// Whether to clear the input after execution
    pub clear_input: bool,
    /// Optional follow-up action
    pub follow_up: Option<Box<SuggestionAction>>,
}

impl CommandResult {
    /// Create a successful result with content
    pub fn success(content: impl Into<String>) -> Self {
        Self {
            success: true,
            content: Some(content.into()),
            error: None,
            close_popup: true,
            clear_input: false,
            follow_up: None,
        }
    }

    /// Create a successful result without content
    pub fn ok() -> Self {
        Self {
            success: true,
            content: None,
            error: None,
            close_popup: true,
            clear_input: false,
            follow_up: None,
        }
    }

    /// Create a failed result with error message
    pub fn error(message: impl Into<String>) -> Self {
        Self {
            success: false,
            content: None,
            error: Some(message.into()),
            close_popup: false,
            clear_input: false,
            follow_up: None,
        }
    }

    /// Set whether to close popup
    pub fn with_close_popup(mut self, close: bool) -> Self {
        self.close_popup = close;
        self
    }

    /// Set whether to clear input
    pub fn with_clear_input(mut self, clear: bool) -> Self {
        self.clear_input = clear;
        self
    }
}

/// Universal command definition
#[derive(Clone)]
pub struct UniversalCommand {
    /// Unique command identifier (e.g., "search", "summarize")
    pub id: String,
    /// Display name (e.g., "Search", "Summarize Text")
    pub name: String,
    /// Short description
    pub description: String,
    /// Usage syntax (e.g., "/search <query>")
    pub syntax: String,
    /// Command category
    pub category: CommandCategory,
    /// View scope
    pub scope: ViewScope,
    /// Display icon
    pub icon: CommandIcon,
    /// Keywords for fuzzy matching
    pub keywords: Vec<String>,
    /// Priority for sorting (higher = more important)
    pub priority: i32,
    /// Whether command accepts arguments
    pub accepts_args: bool,
    /// Whether command integrates with KG for suggestions
    pub kg_enhanced: bool,
    /// Command handler
    pub handler: CommandHandler,
}

impl fmt::Debug for UniversalCommand {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("UniversalCommand")
            .field("id", &self.id)
            .field("name", &self.name)
            .field("category", &self.category)
            .field("scope", &self.scope)
            .finish()
    }
}

/// Command handler types
#[derive(Clone)]
pub enum CommandHandler {
    /// Insert text directly
    Insert(String),
    /// Insert dynamic content (date, time, etc.)
    InsertDynamic(Arc<dyn Fn() -> String + Send + Sync>),
    /// Execute search
    Search,
    /// Execute KG search
    KGSearch,
    /// Trigger autocomplete
    Autocomplete,
    /// AI action (summarize, explain, etc.)
    AI(String),
    /// Context action (add, clear, etc.)
    Context(String),
    /// Custom async handler
    Custom(Arc<dyn Fn(CommandContext) -> CommandResult + Send + Sync>),
}

impl fmt::Debug for CommandHandler {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CommandHandler::Insert(s) => write!(f, "Insert({:?})", s),
            CommandHandler::InsertDynamic(_) => write!(f, "InsertDynamic(<fn>)"),
            CommandHandler::Search => write!(f, "Search"),
            CommandHandler::KGSearch => write!(f, "KGSearch"),
            CommandHandler::Autocomplete => write!(f, "Autocomplete"),
            CommandHandler::AI(action) => write!(f, "AI({:?})", action),
            CommandHandler::Context(action) => write!(f, "Context({:?})", action),
            CommandHandler::Custom(_) => write!(f, "Custom(<fn>)"),
        }
    }
}

/// Context passed to command handlers
#[derive(Clone, Debug)]
pub struct CommandContext {
    /// Arguments passed to the command
    pub args: String,
    /// Current view scope
    pub view: ViewScope,
    /// Current role name
    pub role: String,
    /// Current input text (full)
    pub input_text: String,
    /// Cursor position in input
    pub cursor_position: usize,
}

impl CommandContext {
    pub fn new(args: impl Into<String>, view: ViewScope) -> Self {
        Self {
            args: args.into(),
            view,
            role: String::new(),
            input_text: String::new(),
            cursor_position: 0,
        }
    }

    pub fn with_role(mut self, role: impl Into<String>) -> Self {
        self.role = role.into();
        self
    }

    pub fn with_input(mut self, text: impl Into<String>, cursor: usize) -> Self {
        self.input_text = text.into();
        self.cursor_position = cursor;
        self
    }
}

/// Universal suggestion for display in popup
#[derive(Clone, Debug)]
pub struct UniversalSuggestion {
    /// Unique identifier
    pub id: String,
    /// Display text (main)
    pub text: String,
    /// Secondary description
    pub description: Option<String>,
    /// Snippet/preview text
    pub snippet: Option<String>,
    /// Display icon
    pub icon: CommandIcon,
    /// Category for grouping
    pub category: Option<CommandCategory>,
    /// Relevance score (0.0 - 1.0)
    pub score: f64,
    /// Action to execute when selected
    pub action: SuggestionAction,
    /// Whether this is from knowledge graph
    pub from_kg: bool,
    /// Additional metadata
    pub metadata: SuggestionMetadata,
}

impl UniversalSuggestion {
    /// Create a suggestion from a command
    pub fn from_command(command: &UniversalCommand) -> Self {
        Self {
            id: command.id.clone(),
            text: command.name.clone(),
            description: Some(command.description.clone()),
            snippet: Some(command.syntax.clone()),
            icon: command.icon.clone(),
            category: Some(command.category),
            score: command.priority as f64 / 100.0,
            action: SuggestionAction::ExecuteCommand {
                command_id: command.id.clone(),
                args: None,
            },
            from_kg: command.kg_enhanced,
            metadata: SuggestionMetadata::default(),
        }
    }

    /// Create a KG term suggestion
    pub fn from_kg_term(term: String, score: f64, url: Option<String>) -> Self {
        Self {
            id: format!("kg-{}", term),
            text: term.clone(),
            description: url.clone(),
            snippet: None,
            icon: CommandIcon::Emoji("📚".to_string()),
            category: Some(CommandCategory::KnowledgeGraph),
            score,
            action: SuggestionAction::Insert {
                text: term,
                replace_trigger: true,
            },
            from_kg: true,
            metadata: SuggestionMetadata {
                source: "knowledge_graph".to_string(),
                url,
                ..Default::default()
            },
        }
    }
}

/// Metadata for suggestions
#[derive(Clone, Debug, Default)]
pub struct SuggestionMetadata {
    /// Source of the suggestion
    pub source: String,
    /// URL if applicable
    pub url: Option<String>,
    /// Document ID if from search
    pub document_id: Option<String>,
    /// Additional context
    pub context: Option<String>,
}

/// Trigger type for activating suggestions
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TriggerType {
    /// Character-based trigger (e.g., "/", "++")
    Char {
        sequence: String,
        /// Require start of line
        start_of_line: bool,
    },
    /// Auto-trigger based on typing
    Auto {
        /// Minimum characters to trigger
        min_chars: usize,
    },
    /// Manual trigger (keybinding)
    Manual,
}

/// Trigger information when popup is activated
#[derive(Clone, Debug)]
pub struct TriggerInfo {
    /// Type of trigger
    pub trigger_type: TriggerType,
    /// Position in input where trigger started
    pub start_position: usize,
    /// Current query text (after trigger)
    pub query: String,
    /// View where trigger occurred
    pub view: ViewScope,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_view_scope_includes() {
        assert!(ViewScope::Both.includes(ViewScope::Chat));
        assert!(ViewScope::Both.includes(ViewScope::Search));
        assert!(ViewScope::Both.includes(ViewScope::Editor));
        assert!(ViewScope::Both.includes(ViewScope::Both));

        assert!(ViewScope::Chat.includes(ViewScope::Chat));
        assert!(!ViewScope::Chat.includes(ViewScope::Search));
        assert!(!ViewScope::Chat.includes(ViewScope::Editor));
        assert!(ViewScope::Chat.includes(ViewScope::Both));

        assert!(ViewScope::Search.includes(ViewScope::Search));
        assert!(!ViewScope::Search.includes(ViewScope::Chat));
        assert!(!ViewScope::Search.includes(ViewScope::Editor));
        assert!(ViewScope::Search.includes(ViewScope::Both));

        assert!(ViewScope::Editor.includes(ViewScope::Editor));
        assert!(!ViewScope::Editor.includes(ViewScope::Chat));
        assert!(!ViewScope::Editor.includes(ViewScope::Search));
        assert!(ViewScope::Editor.includes(ViewScope::Both));
    }

    #[test]
    fn test_command_result_builders() {
        let success = CommandResult::success("Hello");
        assert!(success.success);
        assert_eq!(success.content, Some("Hello".to_string()));
        assert!(success.close_popup);

        let error = CommandResult::error("Failed");
        assert!(!error.success);
        assert_eq!(error.error, Some("Failed".to_string()));
        assert!(!error.close_popup);

        let ok = CommandResult::ok().with_close_popup(false);
        assert!(ok.success);
        assert!(!ok.close_popup);
    }

    #[test]
    fn test_universal_suggestion_from_command() {
        let command = UniversalCommand {
            id: "search".to_string(),
            name: "Search".to_string(),
            description: "Search knowledge graph".to_string(),
            syntax: "/search <query>".to_string(),
            category: CommandCategory::Search,
            scope: ViewScope::Both,
            icon: CommandIcon::Emoji("🔍".to_string()),
            keywords: vec!["find".to_string(), "query".to_string()],
            priority: 100,
            accepts_args: true,
            kg_enhanced: true,
            handler: CommandHandler::Search,
        };

        let suggestion = UniversalSuggestion::from_command(&command);
        assert_eq!(suggestion.id, "search");
        assert_eq!(suggestion.text, "Search");
        assert!(suggestion.from_kg);
    }

    #[test]
    fn test_kg_term_suggestion() {
        let suggestion = UniversalSuggestion::from_kg_term(
            "rust".to_string(),
            0.95,
            Some("https://rust-lang.org".to_string()),
        );

        assert_eq!(suggestion.text, "rust");
        assert_eq!(suggestion.score, 0.95);
        assert!(suggestion.from_kg);
        assert_eq!(suggestion.metadata.url, Some("https://rust-lang.org".to_string()));
    }

    #[test]
    fn test_command_context() {
        let ctx = CommandContext::new("query", ViewScope::Chat)
            .with_role("engineer")
            .with_input("Hello world", 5);

        assert_eq!(ctx.args, "query");
        assert_eq!(ctx.view, ViewScope::Chat);
        assert_eq!(ctx.role, "engineer");
        assert_eq!(ctx.cursor_position, 5);
    }

    #[test]
    fn test_trigger_type() {
        let slash_trigger = TriggerType::Char {
            sequence: "/".to_string(),
            start_of_line: true,
        };

        let auto_trigger = TriggerType::Auto { min_chars: 2 };

        assert_ne!(slash_trigger, auto_trigger);
    }
}
