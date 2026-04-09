//! Control-plane routing decision layer for ADF agent dispatch.
//!
//! This module provides a dedicated decision engine for routing agent dispatch
//! requests to the most appropriate provider/model combination based on:
//! - KG routing intent
//! - Provider/model health status
//! - Static configuration fallbacks
//! - Keyword routing fallbacks
//!
//! The design follows the extraction pattern: existing spawn_agent routing logic
//! is moved here without behaviour changes, creating a seam for future enhancements.

pub mod routing;

pub use routing::{DispatchContext, RouteCandidate, RoutingDecision, RoutingDecisionEngine};
