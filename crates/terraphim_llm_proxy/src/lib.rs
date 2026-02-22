//! Terraphim LLM Proxy Library
//!
//! Core library for the LLM proxy, providing intelligent routing, token counting,
//! and provider adapters.

pub mod analyzer;
pub mod cerebras_client;
pub mod cerebras_models;
pub mod cerebras_models_fallback;
pub mod client;
pub mod client_detection;
pub mod config;
pub mod cost;
pub mod error;
pub mod groq_client;
pub mod groq_models;
pub mod groq_models_fallback;
pub mod kimi_client;
pub mod kimi_models;
pub mod kimi_models_fallback;
pub mod management;
pub mod metrics;
pub mod minimax_client;
pub mod oauth;
pub mod openai_codex_client;
pub mod openrouter_client;
pub mod performance;
pub mod production_metrics;
pub mod provider_health;
pub mod retry;
pub mod rolegraph_client;
pub mod router;
pub mod routing;
pub mod security;
pub mod server;
pub mod session;
pub mod token_counter;
pub mod tool_call_utils;
pub mod transformer;
pub mod webhooks;
pub mod zai_client;
// pub mod wasm_router; // Disabled for now (Issue #2)

// Re-export commonly used types
pub use client_detection::{
    client_expects_anthropic_format, client_expects_openai_format, detect_client, ApiFormat,
    ClientInfo, ClientType, DetectionMethod,
};
pub use config::{ConfigFormat, ProxyConfig};
pub use error::{ProxyError, Result};

// OAuth re-exports
pub use oauth::{OAuthProvider, TokenBundle, TokenStore};

// Management re-exports
pub use management::{
    init_start_time, management_routes, ConfigManager, ManagementAuthState, ManagementError,
    ManagementState,
};

// Routing re-exports
pub use routing::{
    create_candidates_without_health, filter_healthy_providers, get_exclusions_for_provider,
    is_excluded, matches_exclusion, resolve_model, reverse_resolve, select_provider,
    select_provider_from_candidates, FallbackStrategy, FillFirstStrategy, ModelExclusion,
    ModelMapper, ModelMapping, ProviderCandidate, RoundRobinStrategy, RoutingStrategy,
    StrategyState,
};

// Webhook re-exports
pub use webhooks::{
    format_signature_header, parse_signature_header, sign_payload, verify_signature,
    DeliveryResult, WebhookDispatcher, WebhookEvent, WebhookEventType, WebhookSettings,
    WebhookStats,
};
