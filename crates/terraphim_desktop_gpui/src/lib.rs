//! Terraphim Desktop GPUI
//!
//! Business logic and integration layers for GPUI-based desktop application.
//!
//! This library provides framework-agnostic business logic that can be used
//! with GPUI or adapted to other UI frameworks.

pub mod autocomplete;
pub mod editor;
pub mod kg_search;
pub mod models;
pub mod search_service;

// Re-exports for convenience
pub use autocomplete::{AutocompleteEngine, AutocompleteSuggestion};
pub use editor::{EditorState, SlashCommand, SlashCommandHandler, SlashCommandManager};
pub use kg_search::{KGSearchResult, KGSearchService, KGTerm};
pub use models::{ChipOperator, ResultItemViewModel, TermChip, TermChipSet};
pub use search_service::{
    LogicalOperator, ParsedQuery, SearchOptions, SearchResults, SearchService,
};

// Re-export core terraphim types for convenience
pub use terraphim_types::{ChatMessage, ContextItem, ContextType, Conversation};
