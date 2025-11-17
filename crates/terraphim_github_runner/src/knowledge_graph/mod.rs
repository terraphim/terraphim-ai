//! Knowledge graph construction and management
//!
//! This module builds and manages the knowledge graph from parsed build files,
//! integrating with terraphim_automata for efficient text matching.

pub mod thesaurus_builder;
pub mod action_graph;

pub use thesaurus_builder::ThesaurusBuilder;
pub use action_graph::ActionGraph;
