//! OAuth authentication module for LLM providers.
//!
//! This module provides OAuth2 authentication support for various LLM providers:
//! - Claude (Anthropic) - OAuth2 PKCE flow
//! - Gemini (Google) - OAuth2 with Google authentication
//! - OpenAI (Codex) - OAuth2 PKCE flow with JWT parsing
//! - GitHub Copilot - Device code flow
//!
//! # Architecture
//!
//! The OAuth module is organized into several components:
//!
//! - [`types`] - Core types like `TokenBundle` and `AuthFlowState`
//! - [`error`] - Error types for OAuth operations
//! - [`provider`] - Trait definitions for OAuth providers
//! - [`store`] - Token storage abstraction
//! - [`codex_importer`] - Import existing Codex CLI tokens
//! - [`token_monitor`] - Monitor token expiration
//!
//! # Example
//!
//! ```rust,ignore
//! use terraphim_llm_proxy::oauth::{TokenStore, TokenBundle, MemoryTokenStore};
//!
//! let store = MemoryTokenStore::new();
//!
//! // Store a token
//! let bundle = TokenBundle::new(
//!     "access_token".to_string(),
//!     "Bearer".to_string(),
//!     "claude".to_string(),
//!     "user@example.com".to_string(),
//! );
//! store.store("claude", "user@example.com", &bundle).await?;
//!
//! // Load a token
//! let loaded = store.load("claude", "user@example.com").await?;
//! ```

pub mod callback;
pub mod claude;
pub mod claude_api_key;
pub mod codex_importer;
pub mod copilot;
pub mod error;
pub mod file_store;
pub mod gemini;
pub mod openai;
pub mod provider;
pub mod store;
pub mod token_manager;
pub mod token_monitor;
pub mod types;

// Re-export commonly used types
pub use callback::{oauth_routes, OAuthFlowManager, OAuthState};
pub use claude::{
    generate_code_challenge, generate_code_verifier, generate_state_token, ClaudeOAuthProvider,
};
pub use codex_importer::{import_codex_tokens_on_startup, CodexTokenImporter};
pub use copilot::CopilotOAuthProvider;
pub use error::{OAuthError, OAuthResult};
pub use file_store::FileTokenStore;
pub use gemini::GeminiOAuthProvider;
pub use openai::OpenAiOAuthProvider;
pub use provider::{DeviceCodeInfo, DeviceCodeProvider, OAuthProvider, ProviderRegistry};
pub use store::{MemoryTokenStore, TokenStore, TokenStoreStats, TokenSummary};
pub use token_manager::TokenManager;
pub use token_monitor::{monitor_tokens_on_startup, TokenMonitor, TokenStatus};
pub use types::{
    AccountInfo, AuthFlowState, FlowStatus, FlowStatusResponse, TokenBundle, TokenValidation,
};
