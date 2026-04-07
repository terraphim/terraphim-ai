use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::Path;
use tokio::fs;

/// Application configuration for Terraphim VM Manager
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// Firecracker configuration
    pub firecracker: FirecrackerConfig,
    /// VM pool configuration
    pub pool: PoolConfig,
    /// Performance configuration
    pub performance: PerformanceConfig,
    /// Logging configuration
    pub logging: LoggingConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FirecrackerConfig {
    /// Directory for Firecracker sockets
    pub socket_dir: String,
    /// Path to Firecracker binary
    pub binary_path: String,
    /// Jailer configuration
    pub jailer: JailerConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JailerConfig {
    /// Enable jailer for sandboxing
    pub enabled: bool,
    /// Jailer executable path
    pub executable_path: String,
    /// Base directory for jailed VMs
    pub base_dir: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PoolConfig {
    /// Minimum pool size
    pub min_size: usize,
    /// Maximum pool size
    pub max_size: usize,
    /// Target pool size
    pub target_size: usize,
    /// Maximum age of prewarmed VMs (seconds)
    pub max_age_seconds: u64,
    /// Health check interval (seconds)
    pub health_check_interval_seconds: u64,
    /// Prewarming interval (seconds)
    pub prewarming_interval_seconds: u64,
    /// Allocation timeout (milliseconds)
    pub allocation_timeout_ms: u64,
    /// Enable snapshot-based instant boot
    pub enable_snapshots: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceConfig {
    /// Enable sub-2 second optimizations
    pub enable_sub2_optimizations: bool,
    /// Boot time target (milliseconds)
    pub boot_time_target_ms: u64,
    /// Enable performance monitoring
    pub enable_monitoring: bool,
    /// Metrics retention period (hours)
    pub metrics_retention_hours: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
    /// Log level (trace, debug, info, warn, error)
    pub level: String,
    /// Enable structured logging
    pub structured: bool,
    /// Log file path (optional)
    pub file_path: Option<String>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            firecracker: FirecrackerConfig {
                socket_dir: "/tmp/firecracker".to_string(),
                binary_path: "/usr/bin/firecracker".to_string(),
                jailer: JailerConfig {
                    enabled: false,
                    executable_path: "/usr/bin/jailer".to_string(),
                    base_dir: "/var/run/firecracker".to_string(),
                },
            },
            pool: PoolConfig {
                min_size: 2,
                max_size: 10,
                target_size: 5,
                max_age_seconds: 300,
                health_check_interval_seconds: 30,
                prewarming_interval_seconds: 60,
                allocation_timeout_ms: 500,
                enable_snapshots: true,
            },
            performance: PerformanceConfig {
                enable_sub2_optimizations: true,
                boot_time_target_ms: 2000,
                enable_monitoring: true,
                metrics_retention_hours: 24,
            },
            logging: LoggingConfig {
                level: "info".to_string(),
                structured: false,
                file_path: None,
            },
        }
    }
}

#[allow(dead_code)]
impl Config {
    /// Load configuration from file
    pub async fn load_from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let content = fs::read_to_string(path).await?;
        let config: Config = toml::from_str(&content)?;
        Ok(config)
    }

    /// Save configuration to file
    pub async fn save_to_file<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let content = toml::to_string_pretty(self)?;
        fs::write(path, content).await?;
        Ok(())
    }

    /// Validate configuration
    pub fn validate(&self) -> Result<()> {
        // Validate pool configuration
        if self.pool.min_size > self.pool.max_size {
            return Err(anyhow::anyhow!(
                "Pool min_size ({}) cannot be greater than max_size ({})",
                self.pool.min_size,
                self.pool.max_size
            ));
        }

        if self.pool.target_size < self.pool.min_size || self.pool.target_size > self.pool.max_size
        {
            return Err(anyhow::anyhow!(
                "Pool target_size ({}) must be between min_size ({}) and max_size ({})",
                self.pool.target_size,
                self.pool.min_size,
                self.pool.max_size
            ));
        }

        // Validate Firecracker configuration
        if !Path::new(&self.firecracker.binary_path).exists() {
            return Err(anyhow::anyhow!(
                "Firecracker binary not found at: {}",
                self.firecracker.binary_path
            ));
        }

        // Validate performance configuration
        if self.performance.boot_time_target_ms > 10000 {
            return Err(anyhow::anyhow!(
                "Boot time target ({}) seems too high for sub-2 second optimization",
                self.performance.boot_time_target_ms
            ));
        }

        Ok(())
    }

    /// Get configuration as environment variables
    pub fn as_env_vars(&self) -> Vec<(String, String)> {
        vec![
            (
                "FIRESOCKET_DIR".to_string(),
                self.firecracker.socket_dir.clone(),
            ),
            (
                "FIRECRACKER_BINARY".to_string(),
                self.firecracker.binary_path.clone(),
            ),
            ("POOL_MIN_SIZE".to_string(), self.pool.min_size.to_string()),
            ("POOL_MAX_SIZE".to_string(), self.pool.max_size.to_string()),
            (
                "POOL_TARGET_SIZE".to_string(),
                self.pool.target_size.to_string(),
            ),
            (
                "BOOT_TIME_TARGET_MS".to_string(),
                self.performance.boot_time_target_ms.to_string(),
            ),
            ("LOG_LEVEL".to_string(), self.logging.level.clone()),
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_config_default() {
        let config = Config::default();
        assert_eq!(config.pool.min_size, 2);
        assert_eq!(config.pool.max_size, 10);
        assert_eq!(config.performance.boot_time_target_ms, 2000);
    }

    #[tokio::test]
    async fn test_config_save_load() -> Result<()> {
        let config = Config::default();
        let temp_dir = tempdir()?;
        let config_path = temp_dir.path().join("config.toml");

        // Save config
        config.save_to_file(&config_path).await?;

        // Load config
        let loaded_config = Config::load_from_file(&config_path).await?;

        assert_eq!(config.pool.min_size, loaded_config.pool.min_size);
        assert_eq!(
            config.firecracker.socket_dir,
            loaded_config.firecracker.socket_dir
        );

        Ok(())
    }

    #[test]
    fn test_config_validation() {
        let mut config = Config::default();

        // Use a dummy path for testing since firecracker binary may not exist
        config.firecracker.binary_path = "/tmp/dummy-firecracker".to_string();

        // Valid config should pass (except for missing binary)
        assert!(config.validate().is_err()); // Should fail due to missing binary

        // Invalid pool config should fail
        config.pool.min_size = 10;
        config.pool.max_size = 5;
        assert!(config.validate().is_err());

        // Fix pool config
        config.pool.min_size = 2;
        config.pool.max_size = 10;
        config.pool.target_size = 15; // Too high
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_config_env_vars() {
        let config = Config::default();
        let env_vars = config.as_env_vars();

        assert!(!env_vars.is_empty());
        assert!(env_vars.iter().any(|(k, _)| k == "POOL_MIN_SIZE"));
        assert!(env_vars.iter().any(|(k, _)| k == "BOOT_TIME_TARGET_MS"));
    }
}
