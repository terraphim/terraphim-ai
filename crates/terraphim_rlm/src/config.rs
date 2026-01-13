//! Configuration types for the RLM orchestration system.
//!
//! This module defines the configuration structures for RLM including
//! backend selection, budget limits, security settings, and operational parameters.

use serde::{Deserialize, Serialize};

/// Main configuration for the RLM system.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RlmConfig {
    // ============================================
    // VM Pool Configuration
    // ============================================
    /// Minimum number of pre-warmed VMs in the pool.
    pub pool_min_size: usize,

    /// Maximum number of VMs in the pool (excluding overflow).
    pub pool_max_size: usize,

    /// Target number of VMs to maintain in the pool.
    pub pool_target_size: usize,

    /// VM boot timeout in milliseconds.
    pub vm_boot_timeout_ms: u64,

    /// VM allocation timeout in milliseconds (target: 500ms).
    pub allocation_timeout_ms: u64,

    /// Maximum number of overflow VMs that can be spawned concurrently.
    pub max_overflow_vms: u32,

    /// Queue depth that triggers pool scale-up.
    pub scale_up_queue_depth: u32,

    /// Idle seconds before pool scale-down.
    pub scale_down_idle_secs: u64,

    // ============================================
    // Budget Configuration
    // ============================================
    /// Maximum tokens allowed per session.
    pub token_budget: u64,

    /// Maximum execution time per session in milliseconds.
    pub time_budget_ms: u64,

    /// Maximum recursion depth for nested LLM calls.
    pub max_recursion_depth: u32,

    /// Maximum iterations in the query loop before forcing termination.
    pub max_iterations: u32,

    // ============================================
    // Session Configuration
    // ============================================
    /// Default session duration in seconds.
    pub session_duration_secs: u64,

    /// Time increment for session extension in seconds.
    pub extension_increment_secs: u64,

    /// Maximum number of session extensions allowed.
    pub max_extensions: u32,

    /// Maximum snapshots per session.
    pub max_snapshots_per_session: u32,

    // ============================================
    // Output Configuration
    // ============================================
    /// Maximum bytes to return inline (larger outputs streamed to file).
    pub max_inline_output_bytes: u64,

    /// Enable verbose content tracing.
    pub enable_verbose_tracing: bool,

    // ============================================
    // Knowledge Graph Configuration
    // ============================================
    /// Knowledge graph validation strictness level.
    pub kg_strictness: KgStrictness,

    /// Maximum retries for KG validation before escalation.
    pub kg_max_retries: u32,

    // ============================================
    // Network Security Configuration
    // ============================================
    /// DNS allowlist (domains that VMs can resolve).
    pub dns_allowlist: Vec<String>,

    /// Whether to log blocked DNS queries.
    pub log_blocked_dns: bool,

    // ============================================
    // OverlayFS Configuration
    // ============================================
    /// Initial overlay filesystem size in MB.
    pub initial_overlay_mb: u32,

    /// Maximum overlay filesystem size in MB.
    pub max_overlay_mb: u32,

    // ============================================
    // Operations Configuration
    // ============================================
    /// Alert webhook URL for escalation.
    pub alert_webhook_url: Option<String>,

    /// Number of failures in window before alerting.
    pub alert_failure_threshold: u32,

    /// Failure window in seconds for alert threshold.
    pub alert_failure_window_secs: u64,

    /// Enable auto-remediation for common failures.
    pub enable_auto_remediation: bool,

    // ============================================
    // Backend Configuration
    // ============================================
    /// Preferred backend order (first available is used).
    pub backend_preference: Vec<BackendType>,

    /// E2B API key (required for E2B backend).
    pub e2b_api_key: Option<String>,

    /// E2B sandbox template name.
    pub e2b_template: Option<String>,

    /// Per-backend session model configuration.
    pub backend_session_models: Vec<BackendSessionConfig>,

    // ============================================
    // LLM Configuration
    // ============================================
    /// LLM provider to use.
    pub llm_provider: Option<String>,

    /// Default model for LLM calls.
    pub default_model: Option<String>,
}

impl Default for RlmConfig {
    fn default() -> Self {
        Self {
            // VM Pool
            pool_min_size: 2,
            pool_max_size: 10,
            pool_target_size: 4,
            vm_boot_timeout_ms: 2000,
            allocation_timeout_ms: 500,
            max_overflow_vms: 3,
            scale_up_queue_depth: 5,
            scale_down_idle_secs: 300,

            // Budget
            token_budget: crate::DEFAULT_TOKEN_BUDGET,
            time_budget_ms: crate::DEFAULT_TIME_BUDGET_MS,
            max_recursion_depth: crate::DEFAULT_MAX_RECURSION_DEPTH,
            max_iterations: 30,

            // Session
            session_duration_secs: 1800, // 30 minutes
            extension_increment_secs: 1800,
            max_extensions: 3,
            max_snapshots_per_session: crate::DEFAULT_MAX_SNAPSHOTS_PER_SESSION,

            // Output
            max_inline_output_bytes: crate::DEFAULT_MAX_INLINE_OUTPUT_BYTES,
            enable_verbose_tracing: false,

            // Knowledge Graph
            kg_strictness: KgStrictness::Normal,
            kg_max_retries: 3,

            // Network Security
            dns_allowlist: crate::DEFAULT_DNS_ALLOWLIST
                .iter()
                .map(|s| s.to_string())
                .collect(),
            log_blocked_dns: true,

            // OverlayFS
            initial_overlay_mb: 100,
            max_overlay_mb: 2048,

            // Operations
            alert_webhook_url: None,
            alert_failure_threshold: 3,
            alert_failure_window_secs: 300,
            enable_auto_remediation: true,

            // Backend
            backend_preference: vec![
                BackendType::Firecracker,
                BackendType::E2b,
                BackendType::Docker,
            ],
            e2b_api_key: None,
            e2b_template: None,
            backend_session_models: vec![
                BackendSessionConfig {
                    backend: BackendType::Firecracker,
                    session_model: SessionModel::Affinity,
                },
                BackendSessionConfig {
                    backend: BackendType::E2b,
                    session_model: SessionModel::Stateless,
                },
                BackendSessionConfig {
                    backend: BackendType::Docker,
                    session_model: SessionModel::Affinity,
                },
            ],

            // LLM
            llm_provider: None,
            default_model: None,
        }
    }
}

impl RlmConfig {
    /// Create a new config with minimal settings for testing.
    pub fn minimal() -> Self {
        Self {
            pool_min_size: 1,
            pool_max_size: 2,
            pool_target_size: 1,
            max_overflow_vms: 1,
            ..Default::default()
        }
    }

    /// Create a config optimized for development (shorter timeouts).
    pub fn development() -> Self {
        Self {
            session_duration_secs: 600, // 10 minutes
            time_budget_ms: 60_000,     // 1 minute
            token_budget: 10_000,
            enable_verbose_tracing: true,
            kg_strictness: KgStrictness::Permissive,
            ..Default::default()
        }
    }

    /// Validate the configuration.
    pub fn validate(&self) -> Result<(), String> {
        if self.pool_min_size > self.pool_max_size {
            return Err("pool_min_size cannot be greater than pool_max_size".to_string());
        }
        if self.pool_target_size > self.pool_max_size {
            return Err("pool_target_size cannot be greater than pool_max_size".to_string());
        }
        if self.initial_overlay_mb > self.max_overlay_mb {
            return Err("initial_overlay_mb cannot be greater than max_overlay_mb".to_string());
        }
        if self.backend_preference.is_empty() {
            return Err("backend_preference cannot be empty".to_string());
        }
        Ok(())
    }

    /// Get the session model for a specific backend.
    pub fn session_model_for_backend(&self, backend: BackendType) -> SessionModel {
        self.backend_session_models
            .iter()
            .find(|c| c.backend == backend)
            .map(|c| c.session_model)
            .unwrap_or(SessionModel::Affinity)
    }
}

/// Knowledge graph validation strictness levels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum KgStrictness {
    /// Warn about unknown terms but don't block.
    Permissive,
    /// Retry N times, then warn (default).
    #[default]
    Normal,
    /// Block any unknown terms immediately.
    Strict,
}

impl KgStrictness {
    /// Check if unknown terms should block execution.
    pub fn blocks_unknown(&self) -> bool {
        matches!(self, KgStrictness::Strict)
    }

    /// Check if retries are allowed.
    pub fn allows_retry(&self) -> bool {
        matches!(self, KgStrictness::Normal)
    }
}

impl std::fmt::Display for KgStrictness {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            KgStrictness::Permissive => write!(f, "permissive"),
            KgStrictness::Normal => write!(f, "normal"),
            KgStrictness::Strict => write!(f, "strict"),
        }
    }
}

/// Execution backend types.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum BackendType {
    /// Firecracker microVM (full isolation, requires KVM).
    Firecracker,
    /// E2B cloud sandboxes (cloud-hosted Firecracker).
    E2b,
    /// Docker containers (gVisor or runc).
    Docker,
}

impl std::fmt::Display for BackendType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BackendType::Firecracker => write!(f, "firecracker"),
            BackendType::E2b => write!(f, "e2b"),
            BackendType::Docker => write!(f, "docker"),
        }
    }
}

/// Session model for execution backends.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum SessionModel {
    /// Same conversation routes to same VM (sticky sessions).
    #[default]
    Affinity,
    /// Each request can go to any available VM.
    Stateless,
}

/// Per-backend session configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackendSessionConfig {
    /// The backend this config applies to.
    pub backend: BackendType,
    /// The session model for this backend.
    pub session_model: SessionModel,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config_validates() {
        let config = RlmConfig::default();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_invalid_pool_config() {
        let config = RlmConfig {
            pool_min_size: 10,
            pool_max_size: 5,
            ..Default::default()
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_kg_strictness_behavior() {
        assert!(!KgStrictness::Permissive.blocks_unknown());
        assert!(!KgStrictness::Normal.blocks_unknown());
        assert!(KgStrictness::Strict.blocks_unknown());

        assert!(!KgStrictness::Permissive.allows_retry());
        assert!(KgStrictness::Normal.allows_retry());
        assert!(!KgStrictness::Strict.allows_retry());
    }

    #[test]
    fn test_session_model_for_backend() {
        let config = RlmConfig::default();
        assert_eq!(
            config.session_model_for_backend(BackendType::Firecracker),
            SessionModel::Affinity
        );
        assert_eq!(
            config.session_model_for_backend(BackendType::E2b),
            SessionModel::Stateless
        );
    }

    #[test]
    fn test_config_serialization() {
        let config = RlmConfig::default();
        let json = serde_json::to_string(&config).unwrap();
        let deserialized: RlmConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(config.pool_min_size, deserialized.pool_min_size);
    }
}
