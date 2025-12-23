//! Command Registry for the Universal Slash Command System
//!
//! This module provides the central registry for all slash commands,
//! with view-scoped lookup, filtering, and built-in command definitions.

use std::collections::HashMap;
use std::sync::Arc;

use super::types::*;

/// Central registry for all slash commands
#[derive(Clone)]
pub struct CommandRegistry {
    /// Commands indexed by ID
    commands: HashMap<String, UniversalCommand>,
    /// Commands indexed by category
    by_category: HashMap<CommandCategory, Vec<String>>,
    /// Commands indexed by scope
    by_scope: HashMap<ViewScope, Vec<String>>,
}

impl CommandRegistry {
    /// Create a new empty registry
    pub fn new() -> Self {
        Self {
            commands: HashMap::new(),
            by_category: HashMap::new(),
            by_scope: HashMap::new(),
        }
    }

    /// Create a registry with all built-in commands
    pub fn with_builtin_commands() -> Self {
        let mut registry = Self::new();
        registry.register_builtin_commands();
        registry
    }

    /// Register a command
    pub fn register(&mut self, command: UniversalCommand) {
        let id = command.id.clone();
        let category = command.category;
        let scope = command.scope;

        // Add to main index
        self.commands.insert(id.clone(), command);

        // Add to category index
        self.by_category
            .entry(category)
            .or_default()
            .push(id.clone());

        // Add to scope index
        self.by_scope.entry(scope).or_default().push(id.clone());

        // If scope is Both, also add to Chat, Search, and Editor indices
        if scope == ViewScope::Both {
            self.by_scope
                .entry(ViewScope::Chat)
                .or_default()
                .push(id.clone());
            self.by_scope
                .entry(ViewScope::Search)
                .or_default()
                .push(id.clone());
            // Both scope commands also appear in Editor
            self.by_scope
                .entry(ViewScope::Editor)
                .or_default()
                .push(id.clone());
        }

        // If scope is Chat, also add to Editor index (Editor includes Chat commands)
        if scope == ViewScope::Chat {
            self.by_scope
                .entry(ViewScope::Editor)
                .or_default()
                .push(id);
        }
    }

    /// Get a command by ID
    pub fn get(&self, id: &str) -> Option<&UniversalCommand> {
        self.commands.get(id)
    }

    /// Get all commands
    pub fn all(&self) -> Vec<&UniversalCommand> {
        self.commands.values().collect()
    }

    /// Get commands for a specific view scope
    pub fn for_scope(&self, scope: ViewScope) -> Vec<&UniversalCommand> {
        self.by_scope
            .get(&scope)
            .map(|ids| {
                ids.iter()
                    .filter_map(|id| self.commands.get(id))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Get commands for a specific category
    pub fn for_category(&self, category: CommandCategory) -> Vec<&UniversalCommand> {
        self.by_category
            .get(&category)
            .map(|ids| {
                ids.iter()
                    .filter_map(|id| self.commands.get(id))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Search commands by query string (fuzzy matching on name, description, keywords)
    pub fn search(&self, query: &str, scope: ViewScope) -> Vec<&UniversalCommand> {
        let query_lower = query.to_lowercase();

        let mut results: Vec<(&UniversalCommand, i32)> = self
            .for_scope(scope)
            .into_iter()
            .filter_map(|cmd| {
                let mut score = 0i32;

                // Exact ID match (highest priority)
                if cmd.id.to_lowercase() == query_lower {
                    score += 1000;
                }
                // ID starts with query
                else if cmd.id.to_lowercase().starts_with(&query_lower) {
                    score += 500;
                }
                // Name contains query
                else if cmd.name.to_lowercase().contains(&query_lower) {
                    score += 300;
                }
                // Description contains query
                else if cmd.description.to_lowercase().contains(&query_lower) {
                    score += 100;
                }
                // Keywords match
                else if cmd
                    .keywords
                    .iter()
                    .any(|k| k.to_lowercase().contains(&query_lower))
                {
                    score += 200;
                }

                if score > 0 {
                    // Add command priority
                    score += cmd.priority;
                    Some((cmd, score))
                } else {
                    None
                }
            })
            .collect();

        // Sort by score descending
        results.sort_by(|a, b| b.1.cmp(&a.1));

        results.into_iter().map(|(cmd, _)| cmd).collect()
    }

    /// Get suggestions for a query (converts commands to suggestions)
    pub fn suggest(&self, query: &str, scope: ViewScope, limit: usize) -> Vec<UniversalSuggestion> {
        self.search(query, scope)
            .into_iter()
            .take(limit)
            .map(UniversalSuggestion::from_command)
            .collect()
    }

    /// Execute a command by ID
    pub fn execute(&self, id: &str, context: CommandContext) -> CommandResult {
        match self.get(id) {
            Some(command) => execute_command_handler(command, context),
            None => CommandResult::error(format!("Command '{}' not found", id)),
        }
    }

    /// Get count of registered commands
    pub fn len(&self) -> usize {
        self.commands.len()
    }

    /// Check if registry is empty
    pub fn is_empty(&self) -> bool {
        self.commands.is_empty()
    }

    /// Register all built-in commands
    fn register_builtin_commands(&mut self) {
        // Search commands
        self.register_search_commands();
        // Formatting commands
        self.register_formatting_commands();
        // AI commands
        self.register_ai_commands();
        // Context commands
        self.register_context_commands();
        // Editor commands
        self.register_editor_commands();
        // System commands
        self.register_system_commands();

        log::info!("Registered {} built-in slash commands", self.len());
    }

    fn register_search_commands(&mut self) {
        // /search - Basic search
        self.register(UniversalCommand {
            id: "search".to_string(),
            name: "Search".to_string(),
            description: "Search knowledge graph".to_string(),
            syntax: "/search <query>".to_string(),
            category: CommandCategory::Search,
            scope: ViewScope::Both,
            icon: CommandIcon::Emoji("🔍".to_string()),
            keywords: vec!["find".to_string(), "query".to_string(), "lookup".to_string()],
            priority: 100,
            accepts_args: true,
            kg_enhanced: true,
            handler: CommandHandler::Search,
        });

        // /kg - Knowledge graph exploration
        self.register(UniversalCommand {
            id: "kg".to_string(),
            name: "Knowledge Graph".to_string(),
            description: "Explore knowledge graph terms".to_string(),
            syntax: "/kg <term>".to_string(),
            category: CommandCategory::KnowledgeGraph,
            scope: ViewScope::Both,
            icon: CommandIcon::Emoji("📊".to_string()),
            keywords: vec![
                "graph".to_string(),
                "term".to_string(),
                "concept".to_string(),
            ],
            priority: 90,
            accepts_args: true,
            kg_enhanced: true,
            handler: CommandHandler::KGSearch,
        });

        // /filter - Filter results (Search scope only)
        self.register(UniversalCommand {
            id: "filter".to_string(),
            name: "Filter".to_string(),
            description: "Filter search results".to_string(),
            syntax: "/filter <criteria>".to_string(),
            category: CommandCategory::Search,
            scope: ViewScope::Search,
            icon: CommandIcon::Emoji("🔧".to_string()),
            keywords: vec!["narrow".to_string(), "refine".to_string()],
            priority: 70,
            accepts_args: true,
            kg_enhanced: false,
            handler: CommandHandler::Custom(Arc::new(|ctx| {
                CommandResult::success(format!("Filtering by: {}", ctx.args))
            })),
        });
    }

    fn register_formatting_commands(&mut self) {
        // /h1 - Heading 1
        self.register(UniversalCommand {
            id: "h1".to_string(),
            name: "Heading 1".to_string(),
            description: "Insert level 1 heading".to_string(),
            syntax: "/h1 <text>".to_string(),
            category: CommandCategory::Formatting,
            scope: ViewScope::Chat,
            icon: CommandIcon::Emoji("📝".to_string()),
            keywords: vec!["header".to_string(), "title".to_string()],
            priority: 80,
            accepts_args: true,
            kg_enhanced: false,
            handler: CommandHandler::Insert("# ".to_string()),
        });

        // /h2 - Heading 2
        self.register(UniversalCommand {
            id: "h2".to_string(),
            name: "Heading 2".to_string(),
            description: "Insert level 2 heading".to_string(),
            syntax: "/h2 <text>".to_string(),
            category: CommandCategory::Formatting,
            scope: ViewScope::Chat,
            icon: CommandIcon::Emoji("📝".to_string()),
            keywords: vec!["header".to_string(), "subtitle".to_string()],
            priority: 75,
            accepts_args: true,
            kg_enhanced: false,
            handler: CommandHandler::Insert("## ".to_string()),
        });

        // /h3 - Heading 3
        self.register(UniversalCommand {
            id: "h3".to_string(),
            name: "Heading 3".to_string(),
            description: "Insert level 3 heading".to_string(),
            syntax: "/h3 <text>".to_string(),
            category: CommandCategory::Formatting,
            scope: ViewScope::Chat,
            icon: CommandIcon::Emoji("📝".to_string()),
            keywords: vec!["header".to_string(), "section".to_string()],
            priority: 70,
            accepts_args: true,
            kg_enhanced: false,
            handler: CommandHandler::Insert("### ".to_string()),
        });

        // /bullet - Bullet list
        self.register(UniversalCommand {
            id: "bullet".to_string(),
            name: "Bullet List".to_string(),
            description: "Insert bullet point".to_string(),
            syntax: "/bullet <item>".to_string(),
            category: CommandCategory::Formatting,
            scope: ViewScope::Chat,
            icon: CommandIcon::Emoji("•".to_string()),
            keywords: vec!["list".to_string(), "item".to_string(), "ul".to_string()],
            priority: 65,
            accepts_args: true,
            kg_enhanced: false,
            handler: CommandHandler::Insert("- ".to_string()),
        });

        // /numbered - Numbered list
        self.register(UniversalCommand {
            id: "numbered".to_string(),
            name: "Numbered List".to_string(),
            description: "Insert numbered item".to_string(),
            syntax: "/numbered <item>".to_string(),
            category: CommandCategory::Formatting,
            scope: ViewScope::Chat,
            icon: CommandIcon::Emoji("1️⃣".to_string()),
            keywords: vec!["list".to_string(), "number".to_string(), "ol".to_string()],
            priority: 64,
            accepts_args: true,
            kg_enhanced: false,
            handler: CommandHandler::Insert("1. ".to_string()),
        });

        // /code - Code block
        self.register(UniversalCommand {
            id: "code".to_string(),
            name: "Code Block".to_string(),
            description: "Insert code block".to_string(),
            syntax: "/code [language]".to_string(),
            category: CommandCategory::Formatting,
            scope: ViewScope::Chat,
            icon: CommandIcon::Emoji("💻".to_string()),
            keywords: vec!["snippet".to_string(), "program".to_string()],
            priority: 60,
            accepts_args: true,
            kg_enhanced: false,
            handler: CommandHandler::Custom(Arc::new(|ctx| {
                let lang = if ctx.args.is_empty() {
                    ""
                } else {
                    &ctx.args
                };
                CommandResult::success(format!("```{}\n\n```", lang))
            })),
        });

        // /quote - Blockquote
        self.register(UniversalCommand {
            id: "quote".to_string(),
            name: "Quote".to_string(),
            description: "Insert blockquote".to_string(),
            syntax: "/quote <text>".to_string(),
            category: CommandCategory::Formatting,
            scope: ViewScope::Chat,
            icon: CommandIcon::Emoji("💬".to_string()),
            keywords: vec!["blockquote".to_string(), "citation".to_string()],
            priority: 55,
            accepts_args: true,
            kg_enhanced: false,
            handler: CommandHandler::Insert("> ".to_string()),
        });
    }

    fn register_ai_commands(&mut self) {
        // /summarize - Summarize text
        self.register(UniversalCommand {
            id: "summarize".to_string(),
            name: "Summarize".to_string(),
            description: "Summarize selected text or context".to_string(),
            syntax: "/summarize".to_string(),
            category: CommandCategory::AI,
            scope: ViewScope::Chat,
            icon: CommandIcon::Emoji("📋".to_string()),
            keywords: vec!["summary".to_string(), "brief".to_string(), "tldr".to_string()],
            priority: 95,
            accepts_args: false,
            kg_enhanced: false,
            handler: CommandHandler::AI("summarize".to_string()),
        });

        // /explain - Explain concept
        self.register(UniversalCommand {
            id: "explain".to_string(),
            name: "Explain".to_string(),
            description: "Explain a concept in detail".to_string(),
            syntax: "/explain <topic>".to_string(),
            category: CommandCategory::AI,
            scope: ViewScope::Chat,
            icon: CommandIcon::Emoji("💡".to_string()),
            keywords: vec![
                "clarify".to_string(),
                "describe".to_string(),
                "what".to_string(),
            ],
            priority: 90,
            accepts_args: true,
            kg_enhanced: true,
            handler: CommandHandler::AI("explain".to_string()),
        });

        // /improve - Improve text
        self.register(UniversalCommand {
            id: "improve".to_string(),
            name: "Improve".to_string(),
            description: "Improve writing quality".to_string(),
            syntax: "/improve".to_string(),
            category: CommandCategory::AI,
            scope: ViewScope::Chat,
            icon: CommandIcon::Emoji("✨".to_string()),
            keywords: vec![
                "enhance".to_string(),
                "rewrite".to_string(),
                "better".to_string(),
            ],
            priority: 85,
            accepts_args: false,
            kg_enhanced: false,
            handler: CommandHandler::AI("improve".to_string()),
        });

        // /translate - Translate text
        self.register(UniversalCommand {
            id: "translate".to_string(),
            name: "Translate".to_string(),
            description: "Translate text to another language".to_string(),
            syntax: "/translate <language>".to_string(),
            category: CommandCategory::AI,
            scope: ViewScope::Chat,
            icon: CommandIcon::Emoji("🌐".to_string()),
            keywords: vec!["language".to_string(), "convert".to_string()],
            priority: 80,
            accepts_args: true,
            kg_enhanced: false,
            handler: CommandHandler::AI("translate".to_string()),
        });
    }

    fn register_context_commands(&mut self) {
        // /context - Show current context
        self.register(UniversalCommand {
            id: "context".to_string(),
            name: "Show Context".to_string(),
            description: "Display current conversation context".to_string(),
            syntax: "/context".to_string(),
            category: CommandCategory::Context,
            scope: ViewScope::Chat,
            icon: CommandIcon::Emoji("📎".to_string()),
            keywords: vec!["info".to_string(), "status".to_string()],
            priority: 75,
            accepts_args: false,
            kg_enhanced: false,
            handler: CommandHandler::Context("show".to_string()),
        });

        // /add - Add to context
        self.register(UniversalCommand {
            id: "add".to_string(),
            name: "Add Context".to_string(),
            description: "Add item to conversation context".to_string(),
            syntax: "/add <content>".to_string(),
            category: CommandCategory::Context,
            scope: ViewScope::Chat,
            icon: CommandIcon::Emoji("➕".to_string()),
            keywords: vec!["include".to_string(), "attach".to_string()],
            priority: 70,
            accepts_args: true,
            kg_enhanced: false,
            handler: CommandHandler::Context("add".to_string()),
        });

        // /clear - Clear context
        self.register(UniversalCommand {
            id: "clear".to_string(),
            name: "Clear Context".to_string(),
            description: "Clear conversation context".to_string(),
            syntax: "/clear".to_string(),
            category: CommandCategory::Context,
            scope: ViewScope::Chat,
            icon: CommandIcon::Emoji("🗑️".to_string()),
            keywords: vec!["reset".to_string(), "remove".to_string()],
            priority: 65,
            accepts_args: false,
            kg_enhanced: false,
            handler: CommandHandler::Context("clear".to_string()),
        });
    }

    fn register_editor_commands(&mut self) {
        // /date - Insert current date
        self.register(UniversalCommand {
            id: "date".to_string(),
            name: "Insert Date".to_string(),
            description: "Insert current date".to_string(),
            syntax: "/date".to_string(),
            category: CommandCategory::Editor,
            scope: ViewScope::Both,
            icon: CommandIcon::Emoji("📅".to_string()),
            keywords: vec!["today".to_string(), "calendar".to_string()],
            priority: 50,
            accepts_args: false,
            kg_enhanced: false,
            handler: CommandHandler::InsertDynamic(Arc::new(|| {
                chrono::Local::now().format("%Y-%m-%d").to_string()
            })),
        });

        // /time - Insert current time
        self.register(UniversalCommand {
            id: "time".to_string(),
            name: "Insert Time".to_string(),
            description: "Insert current time".to_string(),
            syntax: "/time".to_string(),
            category: CommandCategory::Editor,
            scope: ViewScope::Both,
            icon: CommandIcon::Emoji("🕐".to_string()),
            keywords: vec!["now".to_string(), "clock".to_string()],
            priority: 45,
            accepts_args: false,
            kg_enhanced: false,
            handler: CommandHandler::InsertDynamic(Arc::new(|| {
                chrono::Local::now().format("%H:%M:%S").to_string()
            })),
        });

        // /datetime - Insert date and time
        self.register(UniversalCommand {
            id: "datetime".to_string(),
            name: "Insert DateTime".to_string(),
            description: "Insert current date and time".to_string(),
            syntax: "/datetime".to_string(),
            category: CommandCategory::Editor,
            scope: ViewScope::Both,
            icon: CommandIcon::Emoji("📆".to_string()),
            keywords: vec!["timestamp".to_string()],
            priority: 40,
            accepts_args: false,
            kg_enhanced: false,
            handler: CommandHandler::InsertDynamic(Arc::new(|| {
                chrono::Local::now().format("%Y-%m-%d %H:%M").to_string()
            })),
        });
    }

    fn register_system_commands(&mut self) {
        // /help - Show help
        self.register(UniversalCommand {
            id: "help".to_string(),
            name: "Help".to_string(),
            description: "Show available commands".to_string(),
            syntax: "/help [command]".to_string(),
            category: CommandCategory::System,
            scope: ViewScope::Both,
            icon: CommandIcon::Emoji("❓".to_string()),
            keywords: vec!["commands".to_string(), "usage".to_string(), "?".to_string()],
            priority: 100,
            accepts_args: true,
            kg_enhanced: false,
            handler: CommandHandler::Custom(Arc::new(|_ctx| {
                CommandResult::success("Use /command to execute commands. Type / to see available commands.")
            })),
        });

        // /role - Show or switch role
        self.register(UniversalCommand {
            id: "role".to_string(),
            name: "Role".to_string(),
            description: "Show or switch current role".to_string(),
            syntax: "/role [name]".to_string(),
            category: CommandCategory::System,
            scope: ViewScope::Both,
            icon: CommandIcon::Emoji("👤".to_string()),
            keywords: vec!["profile".to_string(), "switch".to_string()],
            priority: 60,
            accepts_args: true,
            kg_enhanced: false,
            handler: CommandHandler::Custom(Arc::new(|ctx| {
                if ctx.args.is_empty() {
                    CommandResult::success(format!("Current role: {}", ctx.role))
                } else {
                    CommandResult::success(format!("Switching to role: {}", ctx.args))
                }
            })),
        });
    }
}

impl Default for CommandRegistry {
    fn default() -> Self {
        Self::with_builtin_commands()
    }
}

/// Execute a command handler and return the result
fn execute_command_handler(command: &UniversalCommand, context: CommandContext) -> CommandResult {
    match &command.handler {
        CommandHandler::Insert(text) => {
            let result = if context.args.is_empty() {
                text.clone()
            } else {
                format!("{}{}", text, context.args)
            };
            CommandResult::success(result)
        }
        CommandHandler::InsertDynamic(func) => {
            let result = func();
            CommandResult::success(result)
        }
        CommandHandler::Search => {
            CommandResult::success(format!("Searching for: {}", context.args))
                .with_close_popup(true)
        }
        CommandHandler::KGSearch => {
            CommandResult::success(format!("KG search for: {}", context.args))
                .with_close_popup(true)
        }
        CommandHandler::Autocomplete => {
            CommandResult::success(format!("Autocomplete: {}", context.args))
        }
        CommandHandler::AI(action) => {
            CommandResult::success(format!("AI {} for: {}", action, context.args))
        }
        CommandHandler::Context(action) => {
            CommandResult::success(format!("Context {}: {}", action, context.args))
        }
        CommandHandler::Custom(func) => func(context),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_registry_creation() {
        let registry = CommandRegistry::new();
        assert!(registry.is_empty());

        let registry_with_builtin = CommandRegistry::with_builtin_commands();
        assert!(!registry_with_builtin.is_empty());
        assert!(registry_with_builtin.len() > 10); // Should have many built-in commands
    }

    #[test]
    fn test_command_registration() {
        let mut registry = CommandRegistry::new();

        registry.register(UniversalCommand {
            id: "test".to_string(),
            name: "Test Command".to_string(),
            description: "A test command".to_string(),
            syntax: "/test".to_string(),
            category: CommandCategory::System,
            scope: ViewScope::Both,
            icon: CommandIcon::None,
            keywords: vec![],
            priority: 50,
            accepts_args: false,
            kg_enhanced: false,
            handler: CommandHandler::Custom(Arc::new(|_| CommandResult::ok())),
        });

        assert_eq!(registry.len(), 1);
        assert!(registry.get("test").is_some());
    }

    #[test]
    fn test_view_scope_filtering() {
        let registry = CommandRegistry::with_builtin_commands();

        let chat_commands = registry.for_scope(ViewScope::Chat);
        let search_commands = registry.for_scope(ViewScope::Search);
        let editor_commands = registry.for_scope(ViewScope::Editor);

        // Chat should have formatting commands
        assert!(chat_commands.iter().any(|c| c.id == "h1"));

        // Both should have search command (scope: Both)
        assert!(chat_commands.iter().any(|c| c.id == "search"));
        assert!(search_commands.iter().any(|c| c.id == "search"));

        // Filter should only be in Search
        assert!(!chat_commands.iter().any(|c| c.id == "filter"));
        assert!(search_commands.iter().any(|c| c.id == "filter"));

        // Editor should have Chat commands (formatting, AI, context)
        assert!(editor_commands.iter().any(|c| c.id == "h1"));
        assert!(editor_commands.iter().any(|c| c.id == "summarize"));
        assert!(editor_commands.iter().any(|c| c.id == "context"));

        // Editor should have Both commands (search, kg, help, role, datetime)
        assert!(editor_commands.iter().any(|c| c.id == "search"));
        assert!(editor_commands.iter().any(|c| c.id == "kg"));
        assert!(editor_commands.iter().any(|c| c.id == "help"));
        assert!(editor_commands.iter().any(|c| c.id == "role"));
        assert!(editor_commands.iter().any(|c| c.id == "date"));

        // Editor should NOT have Search-only commands (filter)
        assert!(!editor_commands.iter().any(|c| c.id == "filter"));
    }

    #[test]
    fn test_category_filtering() {
        let registry = CommandRegistry::with_builtin_commands();

        let formatting = registry.for_category(CommandCategory::Formatting);
        let ai = registry.for_category(CommandCategory::AI);

        assert!(formatting.iter().any(|c| c.id == "h1"));
        assert!(formatting.iter().any(|c| c.id == "code"));

        assert!(ai.iter().any(|c| c.id == "summarize"));
        assert!(ai.iter().any(|c| c.id == "explain"));
    }

    #[test]
    fn test_command_search() {
        let registry = CommandRegistry::with_builtin_commands();

        // Exact match
        let results = registry.search("search", ViewScope::Chat);
        assert!(!results.is_empty());
        assert_eq!(results[0].id, "search");

        // Partial match
        let results = registry.search("sum", ViewScope::Chat);
        assert!(!results.is_empty());
        assert!(results.iter().any(|c| c.id == "summarize"));

        // Keyword match
        let results = registry.search("find", ViewScope::Chat);
        assert!(results.iter().any(|c| c.id == "search"));
    }

    #[test]
    fn test_suggest() {
        let registry = CommandRegistry::with_builtin_commands();

        let suggestions = registry.suggest("h", ViewScope::Chat, 5);
        assert!(!suggestions.is_empty());
        assert!(suggestions.iter().any(|s| s.id == "h1"));
        assert!(suggestions.iter().any(|s| s.id == "help"));
    }

    #[test]
    fn test_command_execution() {
        let registry = CommandRegistry::with_builtin_commands();

        // Test Insert command
        let context = CommandContext::new("Hello", ViewScope::Chat);
        let result = registry.execute("h1", context);
        assert!(result.success);
        assert_eq!(result.content, Some("# Hello".to_string()));

        // Test InsertDynamic command
        let context = CommandContext::new("", ViewScope::Chat);
        let result = registry.execute("date", context);
        assert!(result.success);
        assert!(result.content.is_some());
        // Date should match YYYY-MM-DD format
        let content = result.content.unwrap();
        assert!(content.contains("-"));

        // Test non-existent command
        let context = CommandContext::new("", ViewScope::Chat);
        let result = registry.execute("nonexistent", context);
        assert!(!result.success);
        assert!(result.error.is_some());
    }

    #[test]
    fn test_custom_handler() {
        let registry = CommandRegistry::with_builtin_commands();

        let context = CommandContext::new("rust", ViewScope::Chat)
            .with_role("engineer");

        let result = registry.execute("code", context);
        assert!(result.success);
        let content = result.content.unwrap();
        assert!(content.contains("```rust"));
    }

    #[test]
    fn test_role_command() {
        let registry = CommandRegistry::with_builtin_commands();

        // Without args - show current role
        let context = CommandContext::new("", ViewScope::Chat)
            .with_role("Terraphim Engineer");
        let result = registry.execute("role", context);
        assert!(result.success);
        assert!(result.content.unwrap().contains("Terraphim Engineer"));

        // With args - switch role
        let context = CommandContext::new("Developer", ViewScope::Chat);
        let result = registry.execute("role", context);
        assert!(result.success);
        assert!(result.content.unwrap().contains("Developer"));
    }
}
