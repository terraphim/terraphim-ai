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
//! async fn route_task() {
//!     let router = Router::new().await.unwrap();
//!     
//!     let decision = router.route(
//!         "Implement a secure authentication system",
//!         &RoutingContext::default(),
//!     ).await.unwrap();
//!     
//!     println!("Routed to: {}", decision.provider.id);
//! }
//! ```

pub mod engine;
pub mod keyword;
pub mod registry;
pub mod strategy;
pub mod types;

pub use engine::{RoutingEngine, Router};
pub use keyword::KeywordRouter;
pub use registry::ProviderRegistry;
pub use strategy::{RoutingStrategy, CostOptimized, LatencyOptimized, CapabilityFirst};
pub use types::{RoutingDecision, RoutingContext, RoutingResult, RoutingError};

use terraphim_types::capability::{Provider, Capability};

/// Re-export capability types for convenience
pub use terraphim_types::capability::{
    Capability,
    Provider,
    ProviderType,
    CostLevel,
    Latency,
};
