//! Terraphim Desktop GPUI
//!
//! Business logic and integration layers for GPUI-based desktop application.
//!
//! This library provides framework-agnostic business logic that can be used
//! with GPUI or adapted to other UI frameworks.

#![recursion_limit = "1024"]

// Business logic modules (framework-agnostic)
pub mod autocomplete;
#[cfg(feature = "legacy-components")]
pub mod components;
pub mod editor;
pub mod kg_search;
pub mod models;
pub mod search_service;
pub mod slash_command;

// UI layer modules (GPUI-specific)
pub mod actions;
pub mod app;
pub mod state;
pub mod theme;
pub mod views;

// Utility modules
pub mod utils;

// Markdown rendering
pub mod markdown;

// Security modules
pub mod security;

// Platform-specific modules
pub mod platform;

// Re-exports for convenience
pub use autocomplete::{AutocompleteEngine, AutocompleteSuggestion};
pub use editor::{EditorState, SlashCommand, SlashCommandHandler, SlashCommandManager};
pub use kg_search::{KGSearchResult, KGSearchService, KGTerm};
pub use models::{ChipOperator, ResultItemViewModel, TermChip, TermChipSet};
pub use search_service::{
    LogicalOperator, ParsedQuery, SearchOptions, SearchResults, SearchService,
};
pub use slash_command::{
    CommandCategory, CommandContext, CommandRegistry, UniversalCommand, UniversalSuggestion,
    ViewScope,
};

// Re-export core terraphim types for convenience
pub use terraphim_types::{ChatMessage, ContextItem, ContextType, Conversation};
