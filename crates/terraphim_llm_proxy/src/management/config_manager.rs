//! Configuration manager with hot-reload support.
//!
//! Provides atomic configuration updates with change notification callbacks.
//! Supports both TOML and YAML configuration files with format auto-detection.

use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

use crate::config::ProxyConfig;
use crate::management::error::ManagementError;

/// Type alias for config change callback functions.
pub type ConfigCallback = Box<dyn Fn(&ProxyConfig) + Send + Sync>;

/// Configuration manager with hot-reload and change notification support.
///
/// Provides thread-safe access to configuration with atomic updates
/// and callbacks for components to react to changes.
///
/// # Example
///
/// ```rust,ignore
/// use terraphim_llm_proxy::management::ConfigManager;
/// use std::path::PathBuf;
///
/// let manager = ConfigManager::new(PathBuf::from("config.toml")).await?;
///
/// // Get current config
/// let config = manager.get().await;
/// println!("Host: {}", config.proxy.host);
///
/// // Reload from file
/// manager.reload().await?;
/// ```
pub struct ConfigManager {
    /// Current configuration, protected by RwLock for concurrent access
    config: Arc<RwLock<ProxyConfig>>,
    /// Path to the configuration file
    file_path: PathBuf,
    /// Callbacks to invoke on configuration changes
    callbacks: Arc<RwLock<Vec<ConfigCallback>>>,
}

impl ConfigManager {
    /// Create a new ConfigManager by loading config from the specified file.
    ///
    /// The file format is auto-detected based on extension (.toml, .yaml, .yml).
    ///
    /// # Arguments
    /// * `file_path` - Path to the configuration file
    ///
    /// # Returns
    /// A new ConfigManager instance or an error if the file cannot be loaded.
    pub async fn new(file_path: PathBuf) -> Result<Self, ManagementError> {
        let config = ProxyConfig::load_auto(&file_path)
            .map_err(|e| ManagementError::Internal(format!("Failed to load config: {}", e)))?;

        info!("Loaded configuration from {:?}", file_path);

        Ok(Self {
            config: Arc::new(RwLock::new(config)),
            file_path,
            callbacks: Arc::new(RwLock::new(Vec::new())),
        })
    }

    /// Create a ConfigManager with an existing configuration (for testing).
    ///
    /// # Arguments
    /// * `config` - Initial configuration
    /// * `file_path` - Path to save configuration to
    pub fn with_config(config: ProxyConfig, file_path: PathBuf) -> Self {
        Self {
            config: Arc::new(RwLock::new(config)),
            file_path,
            callbacks: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Get the current configuration.
    ///
    /// Returns a read guard that allows reading the config without blocking
    /// other readers. The guard should be dropped as soon as possible.
    pub async fn get(&self) -> tokio::sync::RwLockReadGuard<'_, ProxyConfig> {
        self.config.read().await
    }

    /// Get a cloned copy of the current configuration.
    ///
    /// Useful when you need to hold onto the config values without
    /// keeping the lock held.
    pub async fn get_cloned(&self) -> ProxyConfig {
        self.config.read().await.clone()
    }

    /// Update the configuration atomically.
    ///
    /// This replaces the entire configuration and persists it to the file.
    /// All registered callbacks are notified after the update.
    ///
    /// # Arguments
    /// * `new_config` - The new configuration to apply
    ///
    /// # Returns
    /// Ok(()) on success, or an error if persistence fails.
    pub async fn update(&self, new_config: ProxyConfig) -> Result<(), ManagementError> {
        // Validate the new config first
        new_config.validate().map_err(|e| {
            ManagementError::ValidationError(format!("Config validation failed: {}", e))
        })?;

        // Atomically swap the config
        {
            let mut config = self.config.write().await;
            *config = new_config;
        }

        // Persist to file
        self.persist().await?;

        // Notify callbacks
        self.notify_callbacks().await;

        info!("Configuration updated and persisted");
        Ok(())
    }

    /// Reload configuration from the file.
    ///
    /// This re-reads the configuration file and applies the changes atomically.
    /// Useful for picking up external changes to the config file.
    ///
    /// # Returns
    /// Ok(()) on success, or an error if the file cannot be loaded.
    pub async fn reload(&self) -> Result<(), ManagementError> {
        let new_config = ProxyConfig::load_auto(&self.file_path)
            .map_err(|e| ManagementError::Internal(format!("Failed to reload config: {}", e)))?;

        // Validate before applying
        new_config.validate().map_err(|e| {
            ManagementError::ValidationError(format!("Config validation failed: {}", e))
        })?;

        // Atomically swap
        {
            let mut config = self.config.write().await;
            *config = new_config;
        }

        // Notify callbacks
        self.notify_callbacks().await;

        info!("Configuration reloaded from {:?}", self.file_path);
        Ok(())
    }

    /// Register a callback to be invoked on configuration changes.
    ///
    /// Callbacks are invoked after every successful update or reload.
    /// They receive a reference to the new configuration.
    ///
    /// # Arguments
    /// * `callback` - Function to call with the new config
    pub async fn on_change<F>(&self, callback: F)
    where
        F: Fn(&ProxyConfig) + Send + Sync + 'static,
    {
        let mut callbacks = self.callbacks.write().await;
        callbacks.push(Box::new(callback));
        debug!("Registered config change callback");
    }

    /// Get the path to the configuration file.
    pub fn file_path(&self) -> &PathBuf {
        &self.file_path
    }

    /// Persist the current configuration to the file.
    async fn persist(&self) -> Result<(), ManagementError> {
        let config = self.config.read().await;
        config
            .save_auto(&self.file_path)
            .map_err(|e| ManagementError::Internal(format!("Failed to persist config: {}", e)))?;
        debug!("Configuration persisted to {:?}", self.file_path);
        Ok(())
    }

    /// Notify all registered callbacks of a configuration change.
    async fn notify_callbacks(&self) {
        let config = self.config.read().await;
        let callbacks = self.callbacks.read().await;

        for callback in callbacks.iter() {
            // Catch any panics from callbacks to prevent them from
            // affecting the configuration manager
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                callback(&config);
            }));

            if let Err(e) = result {
                warn!("Config change callback panicked: {:?}", e);
            }
        }

        debug!("Notified {} config change callbacks", callbacks.len());
    }
}

impl std::fmt::Debug for ConfigManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ConfigManager")
            .field("file_path", &self.file_path)
            .field("callbacks_count", &"<async>")
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{
        ManagementSettings, OAuthSettings, Provider, ProxySettings, RouterSettings,
        SecuritySettings,
    };
    use crate::routing::RoutingStrategy;
    use crate::webhooks::WebhookSettings;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use tempfile::NamedTempFile;

    fn create_test_config() -> ProxyConfig {
        ProxyConfig {
            proxy: ProxySettings {
                host: "127.0.0.1".to_string(),
                port: 3456,
                api_key: "test-key".to_string(),
                timeout_ms: 60000,
            },
            router: RouterSettings {
                default: "openai,gpt-4".to_string(),
                background: None,
                think: None,
                plan_implementation: None,
                long_context: None,
                long_context_threshold: 60000,
                web_search: None,
                image: None,
                model_mappings: vec![],
                model_exclusions: vec![],
                strategy: RoutingStrategy::default(),
            },
            providers: vec![Provider {
                name: "openai".to_string(),
                api_base_url: "https://api.openai.com/v1".to_string(),
                api_key: "sk-test".to_string(),
                models: vec!["gpt-4".to_string()],
                transformers: vec![],
            }],
            security: SecuritySettings::default(),
            oauth: OAuthSettings::default(),
            management: ManagementSettings::default(),
            webhooks: WebhookSettings::default(),
        }
    }

    #[tokio::test]
    async fn test_config_manager_with_config() {
        let config = create_test_config();
        let temp_file = NamedTempFile::with_suffix(".toml").unwrap();
        let temp_path = temp_file.path().to_path_buf();

        let manager = ConfigManager::with_config(config.clone(), temp_path);

        let loaded = manager.get().await;
        assert_eq!(loaded.proxy.host, "127.0.0.1");
        assert_eq!(loaded.proxy.port, 3456);
    }

    #[tokio::test]
    async fn test_config_manager_get_cloned() {
        let config = create_test_config();
        let temp_file = NamedTempFile::with_suffix(".toml").unwrap();
        let temp_path = temp_file.path().to_path_buf();

        let manager = ConfigManager::with_config(config.clone(), temp_path);

        let cloned = manager.get_cloned().await;
        assert_eq!(cloned.proxy.host, "127.0.0.1");

        // Verify we can use the cloned config after manager is modified
        // (in real usage, not a reference to internal state)
    }

    #[tokio::test]
    async fn test_config_manager_update() {
        let config = create_test_config();
        let temp_file = NamedTempFile::with_suffix(".toml").unwrap();
        let temp_path = temp_file.path().to_path_buf();

        // Save initial config
        config.save_toml(&temp_path).unwrap();

        let manager = ConfigManager::with_config(config.clone(), temp_path.clone());

        // Update config
        let mut new_config = config.clone();
        new_config.proxy.port = 9999;

        manager.update(new_config).await.unwrap();

        // Verify in-memory update
        let loaded = manager.get().await;
        assert_eq!(loaded.proxy.port, 9999);

        // Verify persisted to file
        let from_file = ProxyConfig::load_toml(&temp_path).unwrap();
        assert_eq!(from_file.proxy.port, 9999);
    }

    #[tokio::test]
    async fn test_config_manager_reload() {
        let config = create_test_config();
        let temp_file = NamedTempFile::with_suffix(".toml").unwrap();
        let temp_path = temp_file.path().to_path_buf();

        // Save initial config
        config.save_toml(&temp_path).unwrap();

        let manager = ConfigManager::with_config(config.clone(), temp_path.clone());

        // Modify file externally
        let mut modified_config = config.clone();
        modified_config.proxy.port = 8888;
        modified_config.save_toml(&temp_path).unwrap();

        // Reload
        manager.reload().await.unwrap();

        // Verify change was picked up
        let loaded = manager.get().await;
        assert_eq!(loaded.proxy.port, 8888);
    }

    #[tokio::test]
    async fn test_config_manager_callbacks() {
        let config = create_test_config();
        let temp_file = NamedTempFile::with_suffix(".toml").unwrap();
        let temp_path = temp_file.path().to_path_buf();

        config.save_toml(&temp_path).unwrap();

        let manager = ConfigManager::with_config(config.clone(), temp_path);

        // Track callback invocations
        let counter = Arc::new(AtomicUsize::new(0));
        let counter_clone = counter.clone();

        manager
            .on_change(move |_config| {
                counter_clone.fetch_add(1, Ordering::SeqCst);
            })
            .await;

        // Update should trigger callback
        let mut new_config = config.clone();
        new_config.proxy.port = 7777;
        manager.update(new_config).await.unwrap();

        assert_eq!(counter.load(Ordering::SeqCst), 1);

        // Reload should also trigger callback
        manager.reload().await.unwrap();
        assert_eq!(counter.load(Ordering::SeqCst), 2);
    }

    #[tokio::test]
    async fn test_config_manager_multiple_callbacks() {
        let config = create_test_config();
        let temp_file = NamedTempFile::with_suffix(".toml").unwrap();
        let temp_path = temp_file.path().to_path_buf();

        config.save_toml(&temp_path).unwrap();

        let manager = ConfigManager::with_config(config.clone(), temp_path);

        let counter1 = Arc::new(AtomicUsize::new(0));
        let counter2 = Arc::new(AtomicUsize::new(0));

        let c1 = counter1.clone();
        let c2 = counter2.clone();

        manager
            .on_change(move |_| {
                c1.fetch_add(1, Ordering::SeqCst);
            })
            .await;

        manager
            .on_change(move |_| {
                c2.fetch_add(1, Ordering::SeqCst);
            })
            .await;

        // Update should trigger both callbacks
        let mut new_config = config.clone();
        new_config.proxy.port = 6666;
        manager.update(new_config).await.unwrap();

        assert_eq!(counter1.load(Ordering::SeqCst), 1);
        assert_eq!(counter2.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn test_config_manager_yaml_support() {
        let config = create_test_config();
        let temp_file = NamedTempFile::with_suffix(".yaml").unwrap();
        let temp_path = temp_file.path().to_path_buf();

        // Save as YAML
        config.save_yaml(&temp_path).unwrap();

        // Verify YAML was saved correctly
        let content = std::fs::read_to_string(&temp_path).unwrap();
        assert!(content.contains("proxy:"));

        // Use with_config instead of new to avoid sync file I/O in async context
        let manager = ConfigManager::with_config(config.clone(), temp_path.clone());

        let loaded = manager.get().await;
        assert_eq!(loaded.proxy.host, "127.0.0.1");
    }

    #[tokio::test]
    async fn test_config_manager_yaml_persistence() {
        let config = create_test_config();
        let temp_file = NamedTempFile::with_suffix(".yaml").unwrap();
        let temp_path = temp_file.path().to_path_buf();

        // Save initial config
        config.save_yaml(&temp_path).unwrap();

        let manager = ConfigManager::with_config(config.clone(), temp_path.clone());

        // Update and verify persistence
        let mut new_config = config.clone();
        new_config.proxy.port = 5555;
        manager.update(new_config).await.unwrap();

        // Verify file is still YAML
        let content = std::fs::read_to_string(&temp_path).unwrap();
        assert!(content.contains("proxy:"));
        assert!(content.contains("5555"));
        assert!(!content.contains("[proxy]")); // Not TOML
    }

    #[tokio::test]
    async fn test_config_manager_new_from_file() {
        let config = create_test_config();
        let temp_file = NamedTempFile::with_suffix(".toml").unwrap();
        let temp_path = temp_file.path().to_path_buf();

        // Save initial config
        config.save_toml(&temp_path).unwrap();

        // Load with ConfigManager::new
        let manager = ConfigManager::new(temp_path).await.unwrap();

        let loaded = manager.get().await;
        assert_eq!(loaded.proxy.host, "127.0.0.1");
        assert_eq!(loaded.proxy.port, 3456);
    }

    #[tokio::test]
    async fn test_config_manager_concurrent_reads() {
        let config = create_test_config();
        let temp_file = NamedTempFile::with_suffix(".toml").unwrap();
        let temp_path = temp_file.path().to_path_buf();

        config.save_toml(&temp_path).unwrap();

        let manager = Arc::new(ConfigManager::with_config(config.clone(), temp_path));

        // Spawn multiple concurrent readers
        let mut handles = vec![];

        for _ in 0..10 {
            let m = manager.clone();
            let handle = tokio::spawn(async move {
                let config = m.get().await;
                assert_eq!(config.proxy.host, "127.0.0.1");
            });
            handles.push(handle);
        }

        // All should complete successfully
        for handle in handles {
            handle.await.unwrap();
        }
    }

    #[tokio::test]
    async fn test_config_manager_update_validation_failure() {
        let config = create_test_config();
        let temp_file = NamedTempFile::with_suffix(".toml").unwrap();
        let temp_path = temp_file.path().to_path_buf();

        config.save_toml(&temp_path).unwrap();

        let manager = ConfigManager::with_config(config.clone(), temp_path);

        // Create invalid config (empty providers)
        let mut invalid_config = config.clone();
        invalid_config.providers.clear();

        let result = manager.update(invalid_config).await;
        assert!(result.is_err());

        // Original config should be unchanged
        let loaded = manager.get().await;
        assert_eq!(loaded.providers.len(), 1);
    }

    #[tokio::test]
    async fn test_config_manager_file_path() {
        let config = create_test_config();
        let temp_file = NamedTempFile::with_suffix(".toml").unwrap();
        let temp_path = temp_file.path().to_path_buf();

        let manager = ConfigManager::with_config(config, temp_path.clone());

        assert_eq!(manager.file_path(), &temp_path);
    }

    #[tokio::test]
    async fn test_config_manager_debug() {
        let config = create_test_config();
        let temp_file = NamedTempFile::with_suffix(".toml").unwrap();
        let temp_path = temp_file.path().to_path_buf();

        let manager = ConfigManager::with_config(config, temp_path);

        let debug_str = format!("{:?}", manager);
        assert!(debug_str.contains("ConfigManager"));
        assert!(debug_str.contains("file_path"));
    }
}
