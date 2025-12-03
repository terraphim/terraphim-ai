//! Terraphim Desktop GPUI
//!
//! Business logic and integration layers for GPUI-based desktop application.
//!
//! This library provides framework-agnostic business logic that can be used
//! with GPUI or adapted to other UI frameworks.

#![recursion_limit = "1024"]

// Business logic modules (framework-agnostic)
pub mod autocomplete;
// NOTE: The legacy reusable `components` system is currently disabled to
// reduce the compilation surface while we stabilize the GPUI views. The
// new GPUI-aligned views use `state::search` and other modules directly.
// Once the core app is building cleanly we can re-enable this module and
// incrementally wire the new component abstractions back in.
// pub mod components;
pub mod editor;
pub mod kg_search;
pub mod models;
pub mod search_service;

// UI layer modules (GPUI-specific)
pub mod actions;
pub mod app;
pub mod state;
pub mod theme;
pub mod views;

// Utility modules
pub mod utils;

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

// Re-export core terraphim types for convenience
pub use terraphim_types::{ChatMessage, ContextItem, ContextType, Conversation};
