//! Configuration types for the RLM orchestration system.
//!
//! This module defines the configuration structures for RLM including
//! backend selection, budget limits, security settings, and operational parameters.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Default OCI image used by the Apple Container backend.
pub const DEFAULT_APPLE_CONTAINER_IMAGE: &str = "python:3.11-slim";

/// Default CPU allocation per Apple Container session container.
pub const DEFAULT_APPLE_CONTAINER_CPUS: u32 = 1;

/// Default memory allocation per Apple Container session container.
pub const DEFAULT_APPLE_CONTAINER_MEMORY: &str = "512M";

fn default_apple_container_image() -> String {
    DEFAULT_APPLE_CONTAINER_IMAGE.to_string()
}

fn default_apple_container_cpus() -> u32 {
    DEFAULT_APPLE_CONTAINER_CPUS
}

fn default_apple_container_memory() -> String {
    DEFAULT_APPLE_CONTAINER_MEMORY.to_string()
}

/// Whether `value` is a memory size Apple's `container --memory` accepts.
///
/// Accepted shape, matching Apple's documented `--memory` contract exactly: a
/// positive integer with an optional `K`/`M`/`G`/`T`/`P` unit suffix
/// (case-insensitive), e.g. `512M`, `2G`, `4P`, `1048576`. Validating here keeps
/// a typo from becoming a `container run` failure whose CLI message names
/// neither the field nor the cause.
///
/// Surrounding whitespace is **rejected rather than trimmed**: the configured
/// string is forwarded verbatim as the `--memory` argument, so accepting
/// `" 512M "` here and passing it unchanged to the CLI would let validation and
/// the actual argv disagree.
fn is_valid_memory_size(value: &str) -> bool {
    let digits = match value
        .strip_suffix(|c: char| matches!(c.to_ascii_uppercase(), 'K' | 'M' | 'G' | 'T' | 'P'))
    {
        Some(rest) => rest,
        None => value,
    };
    !digits.is_empty()
        && digits.bytes().all(|b| b.is_ascii_digit())
        && digits.parse::<u64>().is_ok_and(|n| n > 0)
}

/// Main configuration for the RLM system.
#[derive(Clone, Serialize, Deserialize)]
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

    /// Optional absolute path to the Apple `container` CLI.
    ///
    /// When `None` (the default) the binary is resolved as `container` from
    /// `PATH`. Only used by the Apple Container backend.
    #[serde(default)]
    pub apple_container_binary: Option<PathBuf>,

    /// OCI image used for Apple Container sessions.
    #[serde(default = "default_apple_container_image")]
    pub apple_container_image: String,

    /// CPU allocation passed to `container run --cpus` for session containers.
    #[serde(default = "default_apple_container_cpus")]
    pub apple_container_cpus: u32,

    /// Memory allocation passed to `container run --memory` for session
    /// containers: a positive integer with an optional `K`/`M`/`G`/`T`/`P`
    /// suffix (e.g. `512M`, `2G`). Forwarded verbatim, so no surrounding
    /// whitespace is accepted.
    #[serde(default = "default_apple_container_memory")]
    pub apple_container_memory: String,

    // ============================================
    // LLM Configuration
    // ============================================
    /// LLM provider to use.
    pub llm_provider: Option<String>,

    /// Default model for LLM calls.
    pub default_model: Option<String>,

    // ============================================
    // Knowledge Graph Thesaurus (runtime, not serialised)
    // ============================================
    /// Knowledge graph thesaurus for term matching during validation.
    /// Set via `TerraphimRlm::new_with_thesaurus` or programmatically.
    /// When `None`, KG validation passes without term matching.
    #[serde(skip, default)]
    pub thesaurus: Option<terraphim_types::Thesaurus>,
}

impl std::fmt::Debug for RlmConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RlmConfig")
            // VM Pool
            .field("pool_min_size", &self.pool_min_size)
            .field("pool_max_size", &self.pool_max_size)
            .field("pool_target_size", &self.pool_target_size)
            .field("vm_boot_timeout_ms", &self.vm_boot_timeout_ms)
            .field("allocation_timeout_ms", &self.allocation_timeout_ms)
            .field("max_overflow_vms", &self.max_overflow_vms)
            .field("scale_up_queue_depth", &self.scale_up_queue_depth)
            .field("scale_down_idle_secs", &self.scale_down_idle_secs)
            // Budget
            .field("token_budget", &self.token_budget)
            .field("time_budget_ms", &self.time_budget_ms)
            .field("max_recursion_depth", &self.max_recursion_depth)
            .field("max_iterations", &self.max_iterations)
            // Session
            .field("session_duration_secs", &self.session_duration_secs)
            .field("extension_increment_secs", &self.extension_increment_secs)
            .field("max_extensions", &self.max_extensions)
            .field("max_snapshots_per_session", &self.max_snapshots_per_session)
            // Output
            .field("max_inline_output_bytes", &self.max_inline_output_bytes)
            .field("enable_verbose_tracing", &self.enable_verbose_tracing)
            // Knowledge Graph
            .field("kg_strictness", &self.kg_strictness)
            .field("kg_max_retries", &self.kg_max_retries)
            // Network Security
            .field("dns_allowlist", &self.dns_allowlist)
            .field("log_blocked_dns", &self.log_blocked_dns)
            // OverlayFS
            .field("initial_overlay_mb", &self.initial_overlay_mb)
            .field("max_overlay_mb", &self.max_overlay_mb)
            // Operations
            .field(
                "alert_webhook_url",
                &self.alert_webhook_url.as_ref().map(|_| "***REDACTED***"),
            )
            .field("alert_failure_threshold", &self.alert_failure_threshold)
            .field("alert_failure_window_secs", &self.alert_failure_window_secs)
            .field("enable_auto_remediation", &self.enable_auto_remediation)
            // Backend
            .field("backend_preference", &self.backend_preference)
            .field(
                "e2b_api_key",
                &self.e2b_api_key.as_ref().map(|_| "***REDACTED***"),
            )
            .field("e2b_template", &self.e2b_template)
            .field("backend_session_models", &self.backend_session_models)
            .field("apple_container_binary", &self.apple_container_binary)
            .field("apple_container_image", &self.apple_container_image)
            .field("apple_container_cpus", &self.apple_container_cpus)
            .field("apple_container_memory", &self.apple_container_memory)
            // LLM
            .field("llm_provider", &self.llm_provider)
            .field("default_model", &self.default_model)
            // Thesaurus
            .field("has_thesaurus", &self.thesaurus.is_some())
            .finish()
    }
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
                BackendType::AppleContainer,
                BackendType::Docker,
                BackendType::Local,
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
                    backend: BackendType::AppleContainer,
                    session_model: SessionModel::Affinity,
                },
                BackendSessionConfig {
                    backend: BackendType::Docker,
                    session_model: SessionModel::Affinity,
                },
                BackendSessionConfig {
                    backend: BackendType::Local,
                    session_model: SessionModel::Affinity,
                },
            ],
            apple_container_binary: None,
            apple_container_image: default_apple_container_image(),
            apple_container_cpus: default_apple_container_cpus(),
            apple_container_memory: default_apple_container_memory(),

            // LLM
            llm_provider: None,
            default_model: None,

            // Thesaurus
            thesaurus: None,
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
        if self.apple_container_image.trim().is_empty() {
            // An empty image yields `container run ... "" sleep infinity`,
            // whose CLI error names neither the field nor the cause.
            return Err("apple_container_image cannot be empty".to_string());
        }
        if self.apple_container_cpus == 0 {
            return Err("apple_container_cpus must be at least 1".to_string());
        }
        if !is_valid_memory_size(&self.apple_container_memory) {
            return Err(format!(
                "apple_container_memory must be a positive integer with an optional \
                 K/M/G/T/P suffix and no surrounding whitespace, such as `512M` or `2G`, got `{}`",
                self.apple_container_memory
            ));
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
    /// Apple `container` CLI (one lightweight Linux VM per container,
    /// Apple silicon + macOS 26 only).
    ///
    /// The canonical serialized identity is `apple-container`. The
    /// `applecontainer` alias is accepted for backward compatibility with
    /// the container-level `rename_all = "lowercase"` spelling.
    #[serde(rename = "apple-container", alias = "applecontainer")]
    AppleContainer,
    /// Docker containers (gVisor or runc).
    Docker,
    /// Local process execution (no isolation, direct command execution).
    Local,
}

impl std::fmt::Display for BackendType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BackendType::Firecracker => write!(f, "firecracker"),
            BackendType::E2b => write!(f, "e2b"),
            BackendType::AppleContainer => write!(f, "apple-container"),
            BackendType::Docker => write!(f, "docker"),
            BackendType::Local => write!(f, "local"),
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
    fn empty_apple_container_image_is_rejected() {
        for image in ["", "   "] {
            let config = RlmConfig {
                apple_container_image: image.to_string(),
                ..Default::default()
            };
            let err = config.validate().unwrap_err();
            assert!(err.contains("apple_container_image"), "{err}");
        }
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
    fn apple_container_backend_serializes_as_kebab_case() {
        let json = serde_json::to_string(&BackendType::AppleContainer).unwrap();
        assert_eq!(json, "\"apple-container\"");
        assert_eq!(BackendType::AppleContainer.to_string(), "apple-container");
    }

    #[test]
    fn apple_container_alias_deserializes() {
        let canonical: BackendType = serde_json::from_str("\"apple-container\"").unwrap();
        assert_eq!(canonical, BackendType::AppleContainer);
        let alias: BackendType = serde_json::from_str("\"applecontainer\"").unwrap();
        assert_eq!(alias, BackendType::AppleContainer);
    }

    #[test]
    fn apple_container_default_session_model_is_affinity() {
        let config = RlmConfig::default();
        assert_eq!(
            config.session_model_for_backend(BackendType::AppleContainer),
            SessionModel::Affinity
        );
    }

    #[test]
    fn apple_container_precedes_docker_and_local_in_default_preference() {
        let prefs = RlmConfig::default().backend_preference;
        let idx = |b: BackendType| prefs.iter().position(|p| *p == b).unwrap();
        assert!(idx(BackendType::AppleContainer) < idx(BackendType::Docker));
        assert!(idx(BackendType::AppleContainer) < idx(BackendType::Local));
        assert!(idx(BackendType::Firecracker) < idx(BackendType::AppleContainer));
        assert!(idx(BackendType::E2b) < idx(BackendType::AppleContainer));
    }

    #[test]
    fn apple_container_settings_round_trip_through_json() {
        let config = RlmConfig {
            apple_container_binary: Some(PathBuf::from("/opt/homebrew/bin/container")),
            apple_container_image: "python:3.12-slim".to_string(),
            apple_container_cpus: 4,
            apple_container_memory: "2G".to_string(),
            ..Default::default()
        };
        let json = serde_json::to_string(&config).unwrap();
        let restored: RlmConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(
            restored.apple_container_binary,
            config.apple_container_binary
        );
        assert_eq!(restored.apple_container_image, config.apple_container_image);
        assert_eq!(restored.apple_container_cpus, 4);
        assert_eq!(restored.apple_container_memory, "2G");
        assert_eq!(restored.backend_preference, config.backend_preference);
    }

    #[test]
    fn apple_container_settings_default_when_absent_from_json() {
        // Configs written before this backend existed must still load.
        let mut value = serde_json::to_value(RlmConfig::default()).unwrap();
        let obj = value.as_object_mut().unwrap();
        obj.remove("apple_container_binary");
        obj.remove("apple_container_image");
        obj.remove("apple_container_cpus");
        obj.remove("apple_container_memory");
        let restored: RlmConfig = serde_json::from_value(value).unwrap();
        assert_eq!(restored.apple_container_binary, None);
        assert_eq!(
            restored.apple_container_image,
            DEFAULT_APPLE_CONTAINER_IMAGE
        );
        assert_eq!(restored.apple_container_cpus, DEFAULT_APPLE_CONTAINER_CPUS);
        assert_eq!(
            restored.apple_container_memory,
            DEFAULT_APPLE_CONTAINER_MEMORY
        );
    }

    #[test]
    fn apple_container_resource_defaults_are_the_documented_values() {
        let config = RlmConfig::default();
        assert_eq!(config.apple_container_cpus, 1);
        assert_eq!(config.apple_container_memory, "512M");
        assert!(config.validate().is_ok());
    }

    #[test]
    fn invalid_apple_container_resources_are_rejected() {
        let zero_cpus = RlmConfig {
            apple_container_cpus: 0,
            ..Default::default()
        };
        assert!(
            zero_cpus
                .validate()
                .unwrap_err()
                .contains("apple_container_cpus")
        );

        // Whitespace-padded values are rejected, not trimmed: the string is
        // forwarded verbatim as the `--memory` argv element, so accepting
        // `" 512M "` would let validation and the actual argv diverge.
        for memory in [
            "",
            "   ",
            "M",
            "P",
            "0",
            "0G",
            "0P",
            "512MB",
            "half a gig",
            "-1G",
            " 512M",
            "512M ",
            " 512M ",
            "\t2G",
            "2G\n",
        ] {
            let config = RlmConfig {
                apple_container_memory: memory.to_string(),
                ..Default::default()
            };
            let err = config.validate().unwrap_err();
            assert!(
                err.contains("apple_container_memory"),
                "memory `{memory}` should be rejected, got: {err}"
            );
        }
    }

    #[test]
    fn valid_apple_container_memory_sizes_are_accepted() {
        // Every suffix Apple's `--memory` documents, including `P`.
        for memory in ["512M", "2G", "1048576", "1t", "64k", "4P", "8p"] {
            let config = RlmConfig {
                apple_container_memory: memory.to_string(),
                ..Default::default()
            };
            assert!(config.validate().is_ok(), "`{memory}` should be accepted");
        }
    }

    #[test]
    fn test_config_serialization() {
        let config = RlmConfig::default();
        let json = serde_json::to_string(&config).unwrap();
        let deserialized: RlmConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(config.pool_min_size, deserialized.pool_min_size);
    }

    #[test]
    fn test_debug_redacts_sensitive_fields() {
        let config = RlmConfig {
            alert_webhook_url: Some("https://hooks.slack.com/services/T00/B00/XXXX".to_string()),
            e2b_api_key: Some("e2b_sk_secret_key".to_string()),
            ..Default::default()
        };

        let debug_output = format!("{:?}", config);

        assert!(
            !debug_output.contains("hooks.slack.com"),
            "alert_webhook_url must be redacted"
        );
        assert!(
            !debug_output.contains("e2b_sk_secret_key"),
            "e2b_api_key must be redacted"
        );
        assert!(
            debug_output.contains("***REDACTED***"),
            "Redaction marker must appear"
        );
        assert!(
            debug_output.contains("pool_min_size"),
            "Non-sensitive fields must be visible"
        );
    }
}
