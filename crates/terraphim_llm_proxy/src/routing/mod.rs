//! Advanced routing module for model aliasing and routing strategies.
//!
//! This module provides:
//! - Model aliasing with glob pattern matching
//! - Provider-scoped model exclusion filtering
//! - Routing strategies (fill-first, round-robin)
//!
//! # Example
//!
//! ```rust,ignore
//! use terraphim_llm_proxy::routing::{ModelMapping, resolve_model, ModelExclusion, is_excluded};
//!
//! // Model aliasing
//! let mappings = vec![
//!     ModelMapping::new("claude-fast", "deepseek,deepseek-chat"),
//!     ModelMapping::with_bidirectional("claude-*", "openrouter,claude-3-opus"),
//! ];
//!
//! let (resolved, _) = resolve_model("claude-fast", &mappings);
//! assert_eq!(resolved, "deepseek,deepseek-chat");
//!
//! // Provider-scoped exclusions
//! let exclusions = vec![
//!     ModelExclusion::new("openrouter", vec!["*-preview", "*-beta"]),
//! ];
//!
//! assert!(is_excluded("claude-3-preview", &exclusions, "openrouter"));
//! ```

pub mod aliasing;
pub mod exclusion;
pub mod model_mapper;
pub mod strategy;

// Re-export commonly used types
pub use aliasing::{matches_exclusion, resolve_model, reverse_resolve, ModelMapping};
pub use exclusion::{get_exclusions_for_provider, is_excluded, ModelExclusion};
pub use model_mapper::{
    translate_model, FallbackStrategy, ModelMapper, ModelMapping as MapperModelMapping,
    ModelMappingError,
};
pub use strategy::{
    create_candidates_without_health, filter_healthy_providers, select_provider,
    select_provider_from_candidates, FillFirstStrategy, ProviderCandidate, RoundRobinStrategy,
    RoutingStrategy, StrategyState,
};
