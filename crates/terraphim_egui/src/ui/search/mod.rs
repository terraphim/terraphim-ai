//! Search module for search-related UI components
//!
//! This module contains the search input, autocomplete, and results widgets.

pub mod input;
pub mod autocomplete;
pub mod results;
pub mod service;

pub use input::SearchInput;
pub use autocomplete::AutocompleteWidget;
pub use results::SearchResults;
pub use service::SearchService;
