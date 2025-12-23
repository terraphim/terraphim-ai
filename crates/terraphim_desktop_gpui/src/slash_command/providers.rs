//! Suggestion Providers for the Universal Slash Command System
//!
//! This module provides the `SuggestionProvider` trait and implementations
//! for different suggestion sources: command palette, knowledge graph, and
//! KG-enhanced commands.

use std::sync::Arc;
use async_trait::async_trait;

use super::registry::CommandRegistry;
use super::types::*;
use crate::autocomplete::{AutocompleteEngine, AutocompleteSuggestion};

/// Trait for suggestion providers
#[async_trait]
pub trait SuggestionProvider: Send + Sync {
    /// Get the provider name
    fn name(&self) -> &str;

    /// Get the priority (higher = appears first in combined results)
    fn priority(&self) -> i32;

    /// Check if this provider handles the given trigger
    fn handles_trigger(&self, trigger: &TriggerInfo) -> bool;

    /// Generate suggestions for the given query
    async fn suggest(
        &self,
        query: &str,
        trigger: &TriggerInfo,
        limit: usize,
    ) -> Vec<UniversalSuggestion>;

    /// Check if provider is available (e.g., KG engine loaded)
    fn is_available(&self) -> bool {
        true
    }
}

/// Command Palette Provider - provides static command suggestions
pub struct CommandPaletteProvider {
    registry: Arc<CommandRegistry>,
}

impl CommandPaletteProvider {
    pub fn new(registry: Arc<CommandRegistry>) -> Self {
        Self { registry }
    }

    /// Create with default built-in commands
    pub fn with_builtin_commands() -> Self {
        Self {
            registry: Arc::new(CommandRegistry::with_builtin_commands()),
        }
    }
}

#[async_trait]
impl SuggestionProvider for CommandPaletteProvider {
    fn name(&self) -> &str {
        "command_palette"
    }

    fn priority(&self) -> i32 {
        100 // High priority - commands should appear first
    }

    fn handles_trigger(&self, trigger: &TriggerInfo) -> bool {
        // Handle "/" trigger at start of line
        matches!(
            &trigger.trigger_type,
            TriggerType::Char { sequence, start_of_line: true } if sequence == "/"
        )
    }

    async fn suggest(
        &self,
        query: &str,
        trigger: &TriggerInfo,
        limit: usize,
    ) -> Vec<UniversalSuggestion> {
        // Get commands matching the query for this view scope
        self.registry.suggest(query, trigger.view, limit)
    }
}

/// Knowledge Graph Provider - provides KG term suggestions
pub struct KnowledgeGraphProvider {
    engine: Option<Arc<AutocompleteEngine>>,
    trigger_sequence: String,
}

impl KnowledgeGraphProvider {
    /// Create provider with optional autocomplete engine
    pub fn new(engine: Option<Arc<AutocompleteEngine>>) -> Self {
        Self {
            engine,
            trigger_sequence: "++".to_string(),
        }
    }

    /// Create provider without engine (will be set later)
    pub fn without_engine() -> Self {
        Self::new(None)
    }

    /// Set the autocomplete engine
    pub fn set_engine(&mut self, engine: Arc<AutocompleteEngine>) {
        self.engine = Some(engine);
    }

    /// Create from thesaurus JSON
    pub fn from_thesaurus_json(json: &str) -> Result<Self, anyhow::Error> {
        let engine = AutocompleteEngine::from_thesaurus_json(json)?;
        Ok(Self::new(Some(Arc::new(engine))))
    }

    /// Set custom trigger sequence
    pub fn with_trigger(mut self, trigger: impl Into<String>) -> Self {
        self.trigger_sequence = trigger.into();
        self
    }

    /// Convert AutocompleteSuggestion to UniversalSuggestion
    fn convert_suggestion(&self, suggestion: &AutocompleteSuggestion) -> UniversalSuggestion {
        UniversalSuggestion::from_kg_term(
            suggestion.term.clone(),
            suggestion.score,
            suggestion.url.clone(),
        )
    }
}

#[async_trait]
impl SuggestionProvider for KnowledgeGraphProvider {
    fn name(&self) -> &str {
        "knowledge_graph"
    }

    fn priority(&self) -> i32 {
        80 // Below command palette, above other sources
    }

    fn handles_trigger(&self, trigger: &TriggerInfo) -> bool {
        // Handle "++" trigger anywhere
        matches!(
            &trigger.trigger_type,
            TriggerType::Char { sequence, .. } if sequence == &self.trigger_sequence
        )
    }

    async fn suggest(
        &self,
        query: &str,
        _trigger: &TriggerInfo,
        limit: usize,
    ) -> Vec<UniversalSuggestion> {
        let Some(engine) = &self.engine else {
            log::warn!("KnowledgeGraphProvider: engine not loaded");
            return Vec::new();
        };

        // Use fuzzy search for better results
        let suggestions = if query.len() < 3 {
            engine.autocomplete(query, limit)
        } else {
            engine.fuzzy_search(query, limit)
        };

        suggestions
            .iter()
            .map(|s| self.convert_suggestion(s))
            .collect()
    }

    fn is_available(&self) -> bool {
        self.engine.is_some()
    }
}

/// KG-Enhanced Command Provider - commands that use KG for contextual suggestions
///
/// When user types `/search rust`, this provider will show KG terms related to "rust"
/// as sub-suggestions for the search command.
pub struct KGEnhancedCommandProvider {
    registry: Arc<CommandRegistry>,
    engine: Option<Arc<AutocompleteEngine>>,
}

impl KGEnhancedCommandProvider {
    pub fn new(registry: Arc<CommandRegistry>, engine: Option<Arc<AutocompleteEngine>>) -> Self {
        Self { registry, engine }
    }

    /// Set the autocomplete engine
    pub fn set_engine(&mut self, engine: Arc<AutocompleteEngine>) {
        self.engine = Some(engine);
    }

    /// Extract command and args from query
    /// e.g., "search rust" -> ("search", "rust")
    fn parse_command_query(query: &str) -> (&str, &str) {
        let query = query.trim();
        match query.split_once(' ') {
            Some((cmd, args)) => (cmd, args.trim()),
            None => (query, ""),
        }
    }

    /// Generate KG-enhanced suggestions for a command
    fn generate_kg_suggestions(
        &self,
        command: &UniversalCommand,
        args: &str,
        limit: usize,
    ) -> Vec<UniversalSuggestion> {
        let Some(engine) = &self.engine else {
            return Vec::new();
        };

        if args.is_empty() {
            return Vec::new();
        }

        // Get KG terms related to the args
        let kg_terms = if args.len() < 3 {
            engine.autocomplete(args, limit)
        } else {
            engine.fuzzy_search(args, limit)
        };

        kg_terms
            .into_iter()
            .map(|term| {
                // Create suggestion that executes the command with the KG term
                UniversalSuggestion {
                    id: format!("{}-kg-{}", command.id, term.term),
                    text: format!("/{} {}", command.id, term.term),
                    description: Some(format!("{}: {}", command.name, term.term)),
                    snippet: term.url.clone(),
                    icon: command.icon.clone(),
                    category: Some(command.category),
                    score: term.score * 0.9, // Slightly lower than direct command match
                    action: SuggestionAction::ExecuteCommand {
                        command_id: command.id.clone(),
                        args: Some(term.term.clone()),
                    },
                    from_kg: true,
                    metadata: SuggestionMetadata {
                        source: "kg_enhanced_command".to_string(),
                        url: term.url,
                        ..Default::default()
                    },
                }
            })
            .collect()
    }
}

#[async_trait]
impl SuggestionProvider for KGEnhancedCommandProvider {
    fn name(&self) -> &str {
        "kg_enhanced_commands"
    }

    fn priority(&self) -> i32 {
        90 // Between command palette and plain KG
    }

    fn handles_trigger(&self, trigger: &TriggerInfo) -> bool {
        // Handle "/" trigger when there's both command and args
        if let TriggerType::Char { sequence, start_of_line: true } = &trigger.trigger_type {
            if sequence == "/" {
                // Only handle if query contains a space (command + args)
                return trigger.query.contains(' ');
            }
        }
        false
    }

    async fn suggest(
        &self,
        query: &str,
        trigger: &TriggerInfo,
        limit: usize,
    ) -> Vec<UniversalSuggestion> {
        let (cmd_name, args) = Self::parse_command_query(query);

        // Find the command
        let Some(command) = self.registry.get(cmd_name) else {
            return Vec::new();
        };

        // Only enhance KG-enabled commands
        if !command.kg_enhanced {
            return Vec::new();
        }

        // Generate KG-enhanced suggestions
        self.generate_kg_suggestions(command, args, limit)
    }

    fn is_available(&self) -> bool {
        self.engine.is_some()
    }
}

/// Composite provider that combines multiple providers
pub struct CompositeProvider {
    providers: Vec<Arc<dyn SuggestionProvider>>,
}

impl CompositeProvider {
    pub fn new() -> Self {
        Self {
            providers: Vec::new(),
        }
    }

    /// Add a provider
    pub fn add_provider(&mut self, provider: Arc<dyn SuggestionProvider>) {
        self.providers.push(provider);
        // Sort by priority descending
        self.providers.sort_by(|a, b| b.priority().cmp(&a.priority()));
    }

    /// Create with default providers (command palette + KG)
    pub fn with_defaults(
        registry: Arc<CommandRegistry>,
        engine: Option<Arc<AutocompleteEngine>>,
    ) -> Self {
        let mut composite = Self::new();

        // Command palette provider
        composite.add_provider(Arc::new(CommandPaletteProvider::new(registry.clone())));

        // KG-enhanced commands provider
        composite.add_provider(Arc::new(KGEnhancedCommandProvider::new(
            registry,
            engine.clone(),
        )));

        // Knowledge graph provider
        composite.add_provider(Arc::new(KnowledgeGraphProvider::new(engine)));

        composite
    }

    /// Get suggestions from all applicable providers
    pub async fn suggest(
        &self,
        query: &str,
        trigger: &TriggerInfo,
        limit: usize,
    ) -> Vec<UniversalSuggestion> {
        let mut all_suggestions = Vec::new();

        for provider in &self.providers {
            if !provider.is_available() {
                continue;
            }

            if !provider.handles_trigger(trigger) {
                continue;
            }

            let suggestions = provider.suggest(query, trigger, limit).await;
            all_suggestions.extend(suggestions);
        }

        // Sort by score descending and deduplicate
        all_suggestions.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        // Deduplicate by ID
        let mut seen = std::collections::HashSet::new();
        all_suggestions.retain(|s| seen.insert(s.id.clone()));

        // Limit results
        all_suggestions.truncate(limit);

        all_suggestions
    }
}

impl Default for CompositeProvider {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_registry() -> Arc<CommandRegistry> {
        Arc::new(CommandRegistry::with_builtin_commands())
    }

    fn create_test_engine() -> Arc<AutocompleteEngine> {
        let json = r#"[
            {"id": 1, "nterm": "rust", "url": "https://rust-lang.org"},
            {"id": 2, "nterm": "tokio", "url": "https://tokio.rs"},
            {"id": 3, "nterm": "async", "url": "https://async.rs"},
            {"id": 4, "nterm": "gpui", "url": "https://gpui.rs"}
        ]"#;
        Arc::new(AutocompleteEngine::from_thesaurus_json(json).unwrap())
    }

    #[tokio::test]
    async fn test_command_palette_provider() {
        let registry = create_test_registry();
        let provider = CommandPaletteProvider::new(registry);

        let trigger = TriggerInfo {
            trigger_type: TriggerType::Char {
                sequence: "/".to_string(),
                start_of_line: true,
            },
            start_position: 0,
            query: "search".to_string(),
            view: ViewScope::Chat,
        };

        assert!(provider.handles_trigger(&trigger));

        let suggestions = provider.suggest("search", &trigger, 10).await;
        assert!(!suggestions.is_empty());
        assert!(suggestions.iter().any(|s| s.id == "search"));
    }

    #[tokio::test]
    async fn test_command_palette_filter_by_view() {
        let registry = create_test_registry();
        let provider = CommandPaletteProvider::new(registry);

        let chat_trigger = TriggerInfo {
            trigger_type: TriggerType::Char {
                sequence: "/".to_string(),
                start_of_line: true,
            },
            start_position: 0,
            query: "filter".to_string(),
            view: ViewScope::Chat,
        };

        let search_trigger = TriggerInfo {
            trigger_type: TriggerType::Char {
                sequence: "/".to_string(),
                start_of_line: true,
            },
            start_position: 0,
            query: "filter".to_string(),
            view: ViewScope::Search,
        };

        // Filter command should only appear in Search scope
        let chat_suggestions = provider.suggest("filter", &chat_trigger, 10).await;
        let search_suggestions = provider.suggest("filter", &search_trigger, 10).await;

        assert!(!chat_suggestions.iter().any(|s| s.id == "filter"));
        assert!(search_suggestions.iter().any(|s| s.id == "filter"));
    }

    #[tokio::test]
    async fn test_knowledge_graph_provider() {
        let engine = create_test_engine();
        let provider = KnowledgeGraphProvider::new(Some(engine));

        let trigger = TriggerInfo {
            trigger_type: TriggerType::Char {
                sequence: "++".to_string(),
                start_of_line: false,
            },
            start_position: 5,
            query: "ru".to_string(),
            view: ViewScope::Chat,
        };

        assert!(provider.handles_trigger(&trigger));
        assert!(provider.is_available());

        let suggestions = provider.suggest("ru", &trigger, 10).await;
        assert!(!suggestions.is_empty());
        assert!(suggestions.iter().any(|s| s.text == "rust"));
    }

    #[tokio::test]
    async fn test_kg_provider_without_engine() {
        let provider = KnowledgeGraphProvider::without_engine();

        let trigger = TriggerInfo {
            trigger_type: TriggerType::Char {
                sequence: "++".to_string(),
                start_of_line: false,
            },
            start_position: 0,
            query: "rust".to_string(),
            view: ViewScope::Chat,
        };

        assert!(!provider.is_available());

        let suggestions = provider.suggest("rust", &trigger, 10).await;
        assert!(suggestions.is_empty());
    }

    #[tokio::test]
    async fn test_kg_enhanced_command_provider() {
        let registry = create_test_registry();
        let engine = create_test_engine();
        let provider = KGEnhancedCommandProvider::new(registry, Some(engine));

        let trigger = TriggerInfo {
            trigger_type: TriggerType::Char {
                sequence: "/".to_string(),
                start_of_line: true,
            },
            start_position: 0,
            query: "search ru".to_string(),
            view: ViewScope::Chat,
        };

        assert!(provider.handles_trigger(&trigger));

        let suggestions = provider.suggest("search ru", &trigger, 10).await;
        assert!(!suggestions.is_empty());
        // Should have enhanced suggestions with KG terms
        assert!(suggestions.iter().any(|s| s.text.contains("rust")));
        assert!(suggestions.iter().all(|s| s.from_kg));
    }

    #[tokio::test]
    async fn test_kg_enhanced_non_kg_command() {
        let registry = create_test_registry();
        let engine = create_test_engine();
        let provider = KGEnhancedCommandProvider::new(registry, Some(engine));

        let trigger = TriggerInfo {
            trigger_type: TriggerType::Char {
                sequence: "/".to_string(),
                start_of_line: true,
            },
            start_position: 0,
            query: "date foo".to_string(), // date command is not KG-enhanced
            view: ViewScope::Chat,
        };

        let suggestions = provider.suggest("date foo", &trigger, 10).await;
        assert!(suggestions.is_empty()); // No KG suggestions for non-KG commands
    }

    #[tokio::test]
    async fn test_composite_provider() {
        let registry = create_test_registry();
        let engine = create_test_engine();
        let composite = CompositeProvider::with_defaults(registry, Some(engine));

        // Test slash command trigger
        let slash_trigger = TriggerInfo {
            trigger_type: TriggerType::Char {
                sequence: "/".to_string(),
                start_of_line: true,
            },
            start_position: 0,
            query: "se".to_string(),
            view: ViewScope::Chat,
        };

        let suggestions = composite.suggest("se", &slash_trigger, 10).await;
        assert!(!suggestions.is_empty());
        // Should have search command
        assert!(suggestions.iter().any(|s| s.id == "search"));

        // Test KG trigger
        let kg_trigger = TriggerInfo {
            trigger_type: TriggerType::Char {
                sequence: "++".to_string(),
                start_of_line: false,
            },
            start_position: 10,
            query: "tok".to_string(),
            view: ViewScope::Chat,
        };

        let kg_suggestions = composite.suggest("tok", &kg_trigger, 10).await;
        assert!(!kg_suggestions.is_empty());
        assert!(kg_suggestions.iter().any(|s| s.text == "tokio"));
    }

    #[test]
    fn test_parse_command_query() {
        assert_eq!(
            KGEnhancedCommandProvider::parse_command_query("search rust"),
            ("search", "rust")
        );
        assert_eq!(
            KGEnhancedCommandProvider::parse_command_query("search"),
            ("search", "")
        );
        assert_eq!(
            KGEnhancedCommandProvider::parse_command_query("search  multiple  words"),
            ("search", "multiple  words")
        );
        assert_eq!(
            KGEnhancedCommandProvider::parse_command_query(""),
            ("", "")
        );
    }
}
