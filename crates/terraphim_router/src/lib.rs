//! Unified routing engine for LLM and Agent providers.
//!
//! This crate provides capability-based routing that works with both
//! LLM models and spawned agents, using the same routing logic.
//!
//! # Example
//!
//! ```
//! use terraphim_router::{Router, RoutingContext};
//!
//! fn route_task() {
//!     let router = Router::new();
//!     
//!     let decision = router.route(
//!         "Implement a secure authentication system",
//!         &RoutingContext::default(),
//!     ).unwrap();
//!     
//!     println!("Routed to: {}", decision.provider.id);
//! }
//! ```

pub mod engine;
pub mod fallback;
pub mod keyword;
pub mod knowledge_graph;
pub mod metrics;
pub mod registry;
pub mod strategy;
pub mod types;

pub use engine::{Router, RoutingEngine};
pub use fallback::{FallbackRouter, FallbackStrategy};
pub use keyword::KeywordRouter;
pub use knowledge_graph::KnowledgeGraphRouter;
pub use metrics::{RouterMetrics, Timer};
#[cfg(feature = "persistence")]
pub use registry::PersistedProviderRegistry;
pub use registry::ProviderRegistry;
pub use strategy::{CapabilityFirst, CostOptimized, LatencyOptimized, RoundRobin, RoutingStrategy};
pub use types::{RoutingContext, RoutingDecision, RoutingError, RoutingResult};

use terraphim_types::capability::Provider;

/// Re-export capability types for convenience
pub use terraphim_types::capability::{Capability, CostLevel, Latency, ProviderType};
