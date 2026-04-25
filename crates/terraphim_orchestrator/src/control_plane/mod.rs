//! Control-plane routing decision layer for ADF agent dispatch.
//!
//! This module provides a dedicated decision engine for routing agent dispatch
//! requests to the most appropriate provider/model combination based on:
//! - KG routing intent
//! - Provider/model health status
//! - Static configuration fallbacks
//! - Keyword routing fallbacks
//! - Live telemetry (throughput, latency, subscription limits)
//! - Session and weekly usage consumption
//!
//! Telemetry is captured from CLI tool output streams (opencode/claude JSON)
//! and stored durably via terraphim_persistence.

pub mod events;
pub mod output_parser;
pub mod policy;
pub mod routing;
pub mod telemetry;
pub mod telemetry_persist;

pub use events::{
    CommandKind, EventOrigin, NormalizedAgentEvent, WebhookContext, dedup_key,
    normalize_polled_command, normalize_webhook_dispatch,
};
pub use routing::{DispatchContext, RouteCandidate, RoutingDecision, RoutingDecisionEngine};
pub use telemetry::{CompletionEvent, TelemetryStore, TelemetrySummary};
