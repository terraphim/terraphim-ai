//! Management API module for runtime configuration and monitoring.
//!
//! This module provides a REST API for managing the proxy at runtime,
//! including configuration changes, provider management, and health monitoring.
//!
//! # Authentication
//!
//! All management endpoints require authentication via:
//! - `X-Management-Key` header
//! - `Authorization: Bearer <secret>` header
//!
//! # Endpoints
//!
//! The management API provides the following endpoint groups:
//!
//! - `GET/PUT /v0/management/config` - Configuration management
//! - `POST /v0/management/config/reload` - Hot reload config
//! - `GET/POST/DELETE /v0/management/api-keys` - API key management
//! - `GET/PUT /v0/management/logs/level` - Log level control
//! - `GET /v0/management/logs` - Recent log entries
//! - `GET /v0/management/health` - Health status
//! - `GET /v0/management/metrics` - Usage metrics
//!
//! # Example
//!
//! ```rust,ignore
//! use terraphim_llm_proxy::management::{
//!     ManagementAuthState, ConfigManager, management_routes
//! };
//! use std::sync::Arc;
//! use std::path::PathBuf;
//!
//! let auth_state = ManagementAuthState::new("my-secret");
//! let config_manager = Arc::new(ConfigManager::new(PathBuf::from("config.toml")).await?);
//! let routes = management_routes(config_manager, auth_state);
//! ```

pub mod auth;
pub mod config_manager;
pub mod error;
pub mod handlers;
pub mod routes;

// Re-export commonly used types
pub use auth::{
    hash_secret, hash_secret_hex, management_auth_middleware, verify_secret_hex,
    ManagementAuthState, MANAGEMENT_KEY_HEADER,
};
pub use config_manager::ConfigManager;
pub use error::{ManagementError, ManagementResult};
pub use handlers::{init_start_time, ManagementState};
pub use routes::management_routes;
