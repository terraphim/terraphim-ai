//! Application state management for Terraphim AI Egui application
//!
//! This module defines the shared state structures used throughout the application.
//! State is wrapped in Arc<Mutex<>> for thread-safe concurrent access.

use std::sync::{Arc, Mutex};
use terraphim_automata::AutocompleteIndex;
use terraphim_config::Role;
use terraphim_types::Document;
use terraphim_types::RoleName;

/// Application state shared across UI components
#[derive(Debug, Clone)]
pub struct AppState {
    /// Current active role configuration
    pub current_role: Arc<Mutex<Role>>,

    /// Last search results
    pub search_results: Arc<Mutex<Vec<Document>>>,

    /// Context manager for LLM interactions
    pub context_manager: Arc<Mutex<ContextManager>>,

    /// Conversation history
    pub conversation_history: Arc<Mutex<Vec<ChatMessage>>>,

    /// WASM-based autocomplete index
    pub autocomplete_index: Arc<Mutex<Option<AutocompleteIndex>>>,

    /// Application UI state
    pub ui_state: Arc<Mutex<UIState>>,
}

/// Manages context items for LLM interactions
#[derive(Debug, Clone, Default)]
pub struct ContextManager {
    /// Selected documents/articles
    pub selected_documents: Vec<Document>,

    /// Selected concepts/terms
    pub selected_concepts: Vec<String>,

    /// Knowledge graph node IDs
    pub selected_kg_nodes: Vec<String>,

    /// Maximum context size (in tokens)
    pub max_context_size: usize,

    /// Current context size estimate
    pub current_context_size: usize,
}

/// Chat message structure
#[derive(Debug, Clone)]
pub struct ChatMessage {
    /// Message ID
    pub id: uuid::Uuid,

    /// Role (user, assistant, system)
    pub role: ChatMessageRole,

    /// Message content
    pub content: String,

    /// Timestamp
    pub timestamp: chrono::DateTime<chrono::Utc>,

    /// Optional metadata
    pub metadata: Option<ChatMessageMetadata>,
}

/// Chat message role
#[derive(Debug, Clone, PartialEq)]
pub enum ChatMessageRole {
    User,
    Assistant,
    System,
}

/// Chat message metadata
#[derive(Debug, Clone, Default)]
pub struct ChatMessageMetadata {
    /// Tokens used
    pub tokens_used: Option<usize>,

    /// Model used
    pub model: Option<String>,

    /// Context documents used
    pub context_documents: Vec<String>,

    /// Processing time in milliseconds
    pub processing_time_ms: Option<u64>,
}

/// UI state for panel management and theming
#[derive(Debug, Clone)]
pub struct UIState {
    /// Currently active tab
    pub active_tab: ActiveTab,

    /// Panel visibility states
    pub panel_visibility: PanelVisibility,

    /// Theme configuration
    pub theme: Theme,

    /// Application settings
    pub settings: AppSettings,
}

/// Active application tab
#[derive(Debug, Clone, PartialEq, Default)]
pub enum ActiveTab {
    #[default]
    Search,
    Chat,
    KnowledgeGraph,
    Context,
    Configuration,
    Sessions,
}

/// Panel visibility states
#[derive(Debug, Clone)]
pub struct PanelVisibility {
    pub search_panel: bool,
    pub chat_panel: bool,
    pub context_panel: bool,
    pub knowledge_graph_panel: bool,
    pub sessions_panel: bool,
    pub configuration_panel: bool,
}

impl Default for PanelVisibility {
    fn default() -> Self {
        Self {
            search_panel: true,
            chat_panel: true,
            context_panel: true,
            knowledge_graph_panel: true,
            sessions_panel: false,
            configuration_panel: true,
        }
    }
}

/// Application theme
#[derive(Debug, Clone, PartialEq, Default)]
pub enum Theme {
    #[default]
    Light,
    Dark,
    Nord,
    Custom,
}

/// Application settings
#[derive(Debug, Clone)]
pub struct AppSettings {
    /// Show autocomplete suggestions
    pub show_autocomplete: bool,

    /// Autocomplete debounce time in milliseconds
    pub autocomplete_debounce_ms: u64,

    /// Maximum autocomplete results
    pub max_autocomplete_results: usize,

    /// LLM provider (ollama, openrouter)
    pub llm_provider: String,

    /// LLM model
    pub llm_model: String,

    /// LLM base URL
    pub llm_base_url: Option<String>,

    /// Auto-save conversations
    pub auto_save_conversations: bool,

    /// Enable keyboard shortcuts
    pub enable_shortcuts: bool,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            show_autocomplete: true,
            autocomplete_debounce_ms: 50,
            max_autocomplete_results: 5,
            llm_provider: "ollama".to_string(),
            llm_model: "llama3.2:3b".to_string(),
            llm_base_url: Some("http://127.0.0.1:11434".to_string()),
            auto_save_conversations: true,
            enable_shortcuts: true,
        }
    }
}

impl AppState {
    /// Create a new application state
    pub fn new() -> Self {
        let default_role = Role::new("Default");

        Self {
            current_role: Arc::new(Mutex::new(default_role)),
            search_results: Arc::new(Mutex::new(Vec::new())),
            context_manager: Arc::new(Mutex::new(ContextManager::default())),
            conversation_history: Arc::new(Mutex::new(Vec::new())),
            autocomplete_index: Arc::new(Mutex::new(None)),
            ui_state: Arc::new(Mutex::new(UIState {
                active_tab: ActiveTab::Search,
                panel_visibility: PanelVisibility::default(),
                theme: Theme::Light,
                settings: AppSettings::default(),
            })),
        }
    }

    /// Get current role (thread-safe)
    pub fn get_current_role(&self) -> std::sync::MutexGuard<'_, Role> {
        self.current_role
            .lock()
            .expect("Failed to lock current_role")
    }

    /// Set current role (thread-safe)
    pub fn set_current_role(&self, role: Role) {
        let mut current_role = self
            .current_role
            .lock()
            .expect("Failed to lock current_role");
        *current_role = role;
    }

    /// Get search results (thread-safe)
    pub fn get_search_results(&self) -> std::sync::MutexGuard<'_, Vec<Document>> {
        self.search_results
            .lock()
            .expect("Failed to lock search_results")
    }

    /// Set search results (thread-safe)
    pub fn set_search_results(&self, results: Vec<Document>) {
        let mut search_results = self
            .search_results
            .lock()
            .expect("Failed to lock search_results");
        *search_results = results;
    }

    /// Get context manager (thread-safe)
    pub fn get_context_manager(&self) -> std::sync::MutexGuard<'_, ContextManager> {
        self.context_manager
            .lock()
            .expect("Failed to lock context_manager")
    }

    /// Add document to context
    pub fn add_document_to_context(&self, document: Document) {
        let mut context = self
            .context_manager
            .lock()
            .expect("Failed to lock context_manager");
        if !context
            .selected_documents
            .iter()
            .any(|d| d.id == document.id)
        {
            context.selected_documents.push(document);
        }
    }

    /// Remove document from context
    pub fn remove_document_from_context(&self, document_id: &str) {
        let mut context = self
            .context_manager
            .lock()
            .expect("Failed to lock context_manager");
        context.selected_documents.retain(|d| d.id != document_id);
    }

    /// Clear context
    pub fn clear_context(&self) {
        let mut context = self
            .context_manager
            .lock()
            .expect("Failed to lock context_manager");
        *context = ContextManager::default();
    }

    /// Add chat message
    pub fn add_chat_message(&self, message: ChatMessage) {
        let mut history = self
            .conversation_history
            .lock()
            .expect("Failed to lock conversation_history");
        history.push(message);
    }

    /// Get conversation history
    pub fn get_conversation_history(&self) -> std::sync::MutexGuard<'_, Vec<ChatMessage>> {
        self.conversation_history
            .lock()
            .expect("Failed to lock conversation_history")
    }

    /// Clear conversation history
    pub fn clear_conversation(&self) {
        let mut history = self
            .conversation_history
            .lock()
            .expect("Failed to lock conversation_history");
        history.clear();
    }

    /// Get autocomplete index
    pub fn get_autocomplete_index(&self) -> std::sync::MutexGuard<'_, Option<AutocompleteIndex>> {
        self.autocomplete_index
            .lock()
            .expect("Failed to lock autocomplete_index")
    }

    /// Set autocomplete index
    pub fn set_autocomplete_index(&self, index: AutocompleteIndex) {
        let mut autocomplete_index = self
            .autocomplete_index
            .lock()
            .expect("Failed to lock autocomplete_index");
        *autocomplete_index = Some(index);
    }

    /// Get UI state
    pub fn get_ui_state(&self) -> std::sync::MutexGuard<'_, UIState> {
        self.ui_state.lock().expect("Failed to lock ui_state")
    }

    /// Update UI state
    pub fn update_ui_state<F>(&self, f: F)
    where
        F: FnOnce(&mut UIState),
    {
        let mut ui_state = self.ui_state.lock().expect("Failed to lock ui_state");
        f(&mut ui_state);
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}
