//! Knowledge Graph Enrichment for Sessions
//!
//! This module provides concept extraction and enrichment capabilities
//! for sessions using `terraphim_automata` and `terraphim_rolegraph`.
//!
//! ## Features
//!
//! - Extract concepts from message content using thesaurus matching
//! - Track concept occurrences and positions
//! - Detect concept connections via knowledge graph
//! - Identify dominant topics in sessions
//!
//! ## Example
//!
//! ```rust,ignore
//! use terraphim_sessions::enrichment::{SessionEnricher, EnrichmentConfig};
//! use terraphim_automata::Thesaurus;
//!
//! let thesaurus = Thesaurus::local_example();
//! let enricher = SessionEnricher::new(thesaurus);
//!
//! let enrichment = enricher.enrich_session(&session).await?;
//! println!("Found {} concepts", enrichment.concepts.len());
//! ```

mod concept;
mod enricher;

pub use concept::{ConceptMatch, ConceptOccurrence, SessionConcepts};
pub use enricher::{
    EnrichmentConfig, EnrichmentResult, SessionEnricher, find_related_sessions, search_by_concept,
};
