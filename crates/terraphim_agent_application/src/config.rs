//! System-wide configuration management with hot reloading

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use config::{Config, ConfigError, Environment, File};
use log::{debug, info, warn};
use notify::{Event, RecommendedWatcher, RecursiveMode, Watcher};
use serde::{Deserialize, Serialize};
use tokio::sync::{mpsc, RwLock};

use crate::{ApplicationError, ApplicationResult};

/// System-wide configuration for the agent application
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApplicationConfig {
    /// Application metadata
    pub application: ApplicationMetadata,
    /// Supervision tree configuration
    pub supervision: SupervisionConfig,
    /// Agent deployment configuration
    pub deployment: DeploymentConfig,
    /// Health monitoring configuration
    pub health: HealthConfig,
    /// Hot reload configuration
    pub hot_reload: HotReloadConfig,
    /// Logging configuration
    pub logging: LoggingConfig,
    /// Resource limits
    pub resources: ResourceConfig,
    /// Agent-specific configurations
    pub agents: HashMap<String, AgentConfig>,
}

/// Application metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApplicationMetadata {
    /// Application name
    pub name: String,
    /// Application version
    pub version: String,
    /// Application description
    pub description: String,
    /// Environment (development, staging, production)
    pub environment: String,
    /// Node identifier
    pub node_id: String,
}

/// Supervision tree configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SupervisionConfig {
    /// Maximum restart intensity
    pub max_restart_intensity: u32,
    /// Restart period in seconds
    pub restart_period_seconds: u64,
    /// Maximum supervision tree depth
    pub max_supervision_depth: u32,
    /// Default restart strategy
    pub default_restart_strategy: String,
    /// Enable supervision tree monitoring
    pub enable_monitoring: bool,
}

/// Agent deployment configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeploymentConfig {
    /// Maximum concurrent agents
    pub max_concurrent_agents: usize,
    /// Agent startup timeout
    pub agent_startup_timeout_seconds: u64,
    /// Agent shutdown timeout
    pub agent_shutdown_timeout_seconds: u64,
    /// Enable automatic scaling
    pub enable_auto_scaling: bool,
    /// Scaling thresholds
    pub scaling_thresholds: ScalingThresholds,
}

/// Scaling thresholds for automatic agent scaling
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScalingThresholds {
    /// CPU utilization threshold for scaling up
    pub cpu_scale_up_threshold: f64,
    /// CPU utilization threshold for scaling down
    pub cpu_scale_down_threshold: f64,
    /// Memory utilization threshold for scaling up
    pub memory_scale_up_threshold: f64,
    /// Memory utilization threshold for scaling down
    pub memory_scale_down_threshold: f64,
    /// Task queue length threshold for scaling up
    pub queue_scale_up_threshold: usize,
    /// Task queue length threshold for scaling down
    pub queue_scale_down_threshold: usize,
}

/// Health monitoring configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthConfig {
    /// Health check interval in seconds
    pub check_interval_seconds: u64,
    /// Health check timeout in seconds
    pub check_timeout_seconds: u64,
    /// Enable detailed health metrics
    pub enable_detailed_metrics: bool,
    /// Health check endpoints
    pub endpoints: Vec<String>,
    /// Alert thresholds
    pub alert_thresholds: AlertThresholds,
}

/// Alert thresholds for health monitoring
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertThresholds {
    /// CPU usage alert threshold
    pub cpu_alert_threshold: f64,
    /// Memory usage alert threshold
    pub memory_alert_threshold: f64,
    /// Error rate alert threshold
    pub error_rate_alert_threshold: f64,
    /// Response time alert threshold in milliseconds
    pub response_time_alert_threshold: u64,
}

/// Hot reload configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HotReloadConfig {
    /// Enable hot reloading
    pub enabled: bool,
    /// Configuration file watch paths
    pub watch_paths: Vec<String>,
    /// Agent behavior reload paths
    pub agent_behavior_paths: Vec<String>,
    /// Reload debounce time in milliseconds
    pub debounce_ms: u64,
    /// Enable graceful reload
    pub graceful_reload: bool,
}

/// Logging configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
    /// Log level
    pub level: String,
    /// Log format (json, text)
    pub format: String,
    /// Log output (stdout, file)
    pub output: String,
    /// Log file path (if output is file)
    pub file_path: Option<String>,
    /// Enable structured logging
    pub structured: bool,
}

/// Resource configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceConfig {
    /// Maximum memory usage in MB
    pub max_memory_mb: u64,
    /// Maximum CPU cores
    pub max_cpu_cores: u32,
    /// Maximum file descriptors
    pub max_file_descriptors: u64,
    /// Maximum network connections
    pub max_network_connections: u64,
    /// Enable resource monitoring
    pub enable_monitoring: bool,
}

/// Agent-specific configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfig {
    /// Agent type
    pub agent_type: String,
    /// Agent-specific settings
    pub settings: serde_json::Value,
    /// Resource limits for this agent type
    pub resource_limits: Option<ResourceLimits>,
    /// Restart policy
    pub restart_policy: Option<String>,
}

/// Resource limits for individual agents
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceLimits {
    /// Maximum memory usage in MB
    pub max_memory_mb: u64,
    /// Maximum CPU usage (0.0 to 1.0)
    pub max_cpu_usage: f64,
    /// Maximum execution time in seconds
    pub max_execution_time_seconds: u64,
}

impl Default for ApplicationConfig {
    fn default() -> Self {
        Self {
            application: ApplicationMetadata {
                name: "terraphim-agent-system".to_string(),
                version: "0.1.0".to_string(),
                description: "Terraphim AI Agent System".to_string(),
                environment: "development".to_string(),
                node_id: uuid::Uuid::new_v4().to_string(),
            },
            supervision: SupervisionConfig {
                max_restart_intensity: 5,
                restart_period_seconds: 60,
                max_supervision_depth: 10,
                default_restart_strategy: "one_for_one".to_string(),
                enable_monitoring: true,
            },
            deployment: DeploymentConfig {
                max_concurrent_agents: 100,
                agent_startup_timeout_seconds: 30,
                agent_shutdown_timeout_seconds: 10,
                enable_auto_scaling: true,
                scaling_thresholds: ScalingThresholds {
                    cpu_scale_up_threshold: 0.8,
                    cpu_scale_down_threshold: 0.3,
                    memory_scale_up_threshold: 0.8,
                    memory_scale_down_threshold: 0.3,
                    queue_scale_up_threshold: 100,
                    queue_scale_down_threshold: 10,
                },
            },
            health: HealthConfig {
                check_interval_seconds: 30,
                check_timeout_seconds: 5,
                enable_detailed_metrics: true,
                endpoints: vec!["/health".to_string(), "/metrics".to_string()],
                alert_thresholds: AlertThresholds {
                    cpu_alert_threshold: 0.9,
                    memory_alert_threshold: 0.9,
                    error_rate_alert_threshold: 0.05,
                    response_time_alert_threshold: 1000,
                },
            },
            hot_reload: HotReloadConfig {
                enabled: true,
                watch_paths: vec!["config/".to_string()],
                agent_behavior_paths: vec!["agents/".to_string()],
                debounce_ms: 1000,
                graceful_reload: true,
            },
            logging: LoggingConfig {
                level: "info".to_string(),
                format: "json".to_string(),
                output: "stdout".to_string(),
                file_path: None,
                structured: true,
            },
            resources: ResourceConfig {
                max_memory_mb: 4096,
                max_cpu_cores: 8,
                max_file_descriptors: 10000,
                max_network_connections: 1000,
                enable_monitoring: true,
            },
            agents: HashMap::new(),
        }
    }
}

 /// Configuration manager with hot reloading capabilities
 pub struct ConfigurationManager {
     /// Current configuration
     config: Arc<RwLock<ApplicationConfig>>,
     /// Configuration file path
     config_path: PathBuf,
     /// File watcher for hot reloading
     _watcher: Option<RecommendedWatcher>,
     /// Configuration change notifications
     change_tx: mpsc::UnboundedSender<ConfigurationChange>,
     /// Configuration change receiver
     change_rx: Arc<RwLock<Option<mpsc::UnboundedReceiver<ConfigurationChange>>>>,
 }

/// Configuration change notification
#[derive(Debug, Clone)]
pub struct ConfigurationChange {
    /// Change type
    pub change_type: ConfigurationChangeType,
    /// Changed section
    pub section: String,
    /// Previous configuration (if available)
    pub previous_config: Option<ApplicationConfig>,
    /// New configuration
    pub new_config: ApplicationConfig,
    /// Timestamp of change
    pub timestamp: std::time::SystemTime,
}

/// Types of configuration changes
#[derive(Debug, Clone, PartialEq)]
pub enum ConfigurationChangeType {
    /// Configuration file modified
    FileModified,
    /// Configuration reloaded programmatically
    ProgrammaticReload,
    /// Environment variable changed
    EnvironmentChanged,
    /// Hot reload triggered
    HotReload,
}

impl ConfigurationManager {
    /// Create a new configuration manager
    pub async fn new<P: AsRef<Path>>(config_path: P) -> ApplicationResult<Self> {
        let config_path = config_path.as_ref().to_path_buf();
        let config = Self::load_config(&config_path).await?;
        let config = Arc::new(RwLock::new(config));

        let (change_tx, change_rx) = mpsc::unbounded_channel();

        Ok(Self {
            config,
            config_path,
            _watcher: None,
            change_tx,
            change_rx: Arc::new(RwLock::new(Some(change_rx))),
        })
    }

     /// Load configuration from file
     async fn load_config(config_path: &Path) -> ApplicationResult<ApplicationConfig> {
         // Add default configuration first (base layer)
         let default_config = ApplicationConfig::default();
         let default_toml = toml::to_string(&default_config)
             .map_err(|e| ApplicationError::ConfigurationError(e.to_string()))?;

         let config_builder = Config::builder()
             .add_source(config::File::from_str(
                 &default_toml,
                 config::FileFormat::Toml,
             ))
             .add_source(File::from(config_path).required(false))
             .add_source(Environment::with_prefix("TERRAPHIM"));

        let config = config_builder
            .build()
            .map_err(|e| ApplicationError::ConfigurationError(e.to_string()))?;

        let app_config: ApplicationConfig = config
            .try_deserialize()
            .map_err(|e| ApplicationError::ConfigurationError(e.to_string()))?;

        info!("Configuration loaded from {:?}", config_path);
        Ok(app_config)
    }

    /// Start configuration hot reloading
    pub async fn start_hot_reload(&mut self) -> ApplicationResult<()> {
        let config = self.config.read().await;
        if !config.hot_reload.enabled {
            debug!("Hot reload is disabled");
            return Ok(());
        }

        let watch_paths = config.hot_reload.watch_paths.clone();
        drop(config);

        let change_tx = self.change_tx.clone();
        let config_path = self.config_path.clone();
        let config_arc = self.config.clone();

        let mut watcher =
            notify::recommended_watcher(move |res: Result<Event, notify::Error>| match res {
                Ok(event) => {
                    if event.paths.iter().any(|p| p.ends_with(&config_path)) {
                        let change_tx = change_tx.clone();
                        let config_path = config_path.clone();
                        let config_arc = config_arc.clone();

                        tokio::spawn(async move {
                            match Self::load_config(&config_path).await {
                                Ok(new_config) => {
                                    let previous_config = {
                                        let current = config_arc.read().await;
                                        Some(current.clone())
                                    };

                                    {
                                        let mut current = config_arc.write().await;
                                        *current = new_config.clone();
                                    }

                                    let change = ConfigurationChange {
                                        change_type: ConfigurationChangeType::HotReload,
                                        section: "all".to_string(),
                                        previous_config,
                                        new_config,
                                        timestamp: std::time::SystemTime::now(),
                                    };

                                    if let Err(e) = change_tx.send(change) {
                                        warn!(
                                            "Failed to send configuration change notification: {}",
                                            e
                                        );
                                    } else {
                                        info!("Configuration hot reloaded");
                                    }
                                }
                                Err(e) => {
                                    warn!("Failed to reload configuration: {}", e);
                                }
                            }
                        });
                    }
                }
                Err(e) => {
                    warn!("Configuration file watch error: {}", e);
                }
            })
            .map_err(|e| ApplicationError::ConfigurationError(e.to_string()))?;

        // Watch configuration file and additional paths
        watcher
            .watch(&self.config_path, RecursiveMode::NonRecursive)
            .map_err(|e| ApplicationError::ConfigurationError(e.to_string()))?;

        for watch_path in watch_paths {
            let path = Path::new(&watch_path);
            if path.exists() {
                watcher
                    .watch(path, RecursiveMode::Recursive)
                    .map_err(|e| ApplicationError::ConfigurationError(e.to_string()))?;
            }
        }

        self._watcher = Some(watcher);
        info!("Configuration hot reload started");
        Ok(())
    }

    /// Get current configuration
    pub async fn get_config(&self) -> ApplicationConfig {
        self.config.read().await.clone()
    }

    /// Update configuration programmatically
    pub async fn update_config(&self, new_config: ApplicationConfig) -> ApplicationResult<()> {
        let previous_config = {
            let current = self.config.read().await;
            Some(current.clone())
        };

        {
            let mut current = self.config.write().await;
            *current = new_config.clone();
        }

        let change = ConfigurationChange {
            change_type: ConfigurationChangeType::ProgrammaticReload,
            section: "all".to_string(),
            previous_config,
            new_config,
            timestamp: std::time::SystemTime::now(),
        };

        self.change_tx
            .send(change)
            .map_err(|e| ApplicationError::ConfigurationError(e.to_string()))?;

        info!("Configuration updated programmatically");
        Ok(())
    }

    /// Get configuration change receiver
    pub async fn get_change_receiver(
        &self,
    ) -> ApplicationResult<mpsc::UnboundedReceiver<ConfigurationChange>> {
        let mut rx_guard = self.change_rx.write().await;
        rx_guard.take().ok_or_else(|| {
            ApplicationError::ConfigurationError(
                "Configuration change receiver already taken".to_string(),
            )
        })
    }

    /// Save current configuration to file
    pub async fn save_config(&self) -> ApplicationResult<()> {
        let config = self.config.read().await;
        let config_toml = toml::to_string_pretty(&*config)
            .map_err(|e| ApplicationError::ConfigurationError(e.to_string()))?;

        tokio::fs::write(&self.config_path, config_toml)
            .await
            .map_err(|e| ApplicationError::ConfigurationError(e.to_string()))?;

        info!("Configuration saved to {:?}", self.config_path);
        Ok(())
    }

    /// Validate configuration
    pub async fn validate_config(&self) -> ApplicationResult<Vec<String>> {
        let config = self.config.read().await;
        let mut warnings = Vec::new();

        // Validate supervision configuration
        if config.supervision.max_restart_intensity == 0 {
            warnings
                .push("Supervision max_restart_intensity is 0, agents won't restart".to_string());
        }

        // Validate deployment configuration
        if config.deployment.max_concurrent_agents == 0 {
            warnings.push(
                "Deployment max_concurrent_agents is 0, no agents can be deployed".to_string(),
            );
        }

        // Validate health configuration
        if config.health.check_interval_seconds == 0 {
            warnings
                .push("Health check_interval_seconds is 0, health checks are disabled".to_string());
        }

        // Validate resource limits
        if config.resources.max_memory_mb < 512 {
            warnings.push(
                "Resource max_memory_mb is very low, may cause performance issues".to_string(),
            );
        }

        debug!(
            "Configuration validation completed with {} warnings",
            warnings.len()
        );
        Ok(warnings)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    #[tokio::test]
    async fn test_default_configuration() {
        let config = ApplicationConfig::default();
        assert_eq!(config.application.name, "terraphim-agent-system");
        assert!(config.hot_reload.enabled);
        assert!(config.supervision.enable_monitoring);
    }

    #[tokio::test]
    async fn test_configuration_manager_creation() {
        let temp_file = NamedTempFile::new().unwrap();
        let config_manager = ConfigurationManager::new(temp_file.path()).await;
        assert!(config_manager.is_ok());
    }

    #[tokio::test]
    async fn test_configuration_validation() {
        let temp_file = NamedTempFile::new().unwrap();
        let config_manager = ConfigurationManager::new(temp_file.path()).await.unwrap();
        let warnings = config_manager.validate_config().await.unwrap();
        // Default configuration should have no warnings
        assert!(warnings.is_empty());
    }

    #[tokio::test]
    async fn test_configuration_update() {
        let temp_file = NamedTempFile::new().unwrap();
        let config_manager = ConfigurationManager::new(temp_file.path()).await.unwrap();

        let mut new_config = config_manager.get_config().await;
        new_config.application.name = "updated-name".to_string();

        let result = config_manager.update_config(new_config).await;
        assert!(result.is_ok());

        let updated_config = config_manager.get_config().await;
        assert_eq!(updated_config.application.name, "updated-name");
    }
}
