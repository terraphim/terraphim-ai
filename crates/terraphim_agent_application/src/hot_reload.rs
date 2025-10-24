//! Hot code reloading capabilities for agent behavior updates

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use async_trait::async_trait;
use log::{debug, info, warn};
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;

use crate::{ApplicationConfig, ApplicationError, ApplicationResult};

/// Hot reload management trait
#[async_trait]
pub trait HotReloadManagement: Send + Sync {
    /// Start hot reload manager
    async fn start(&self) -> ApplicationResult<()>;

    /// Stop hot reload manager
    async fn stop(&self) -> ApplicationResult<()>;

    /// Reload a specific component
    async fn reload_component(&self, component: &str) -> ApplicationResult<()>;

    /// Reload all components
    async fn reload_all(&self) -> ApplicationResult<()>;

    /// Get reload status
    async fn get_reload_status(&self) -> ApplicationResult<ReloadStatus>;

    /// Register component for hot reloading
    async fn register_component(&self, component: ComponentSpec) -> ApplicationResult<()>;

    /// Unregister component from hot reloading
    async fn unregister_component(&self, component_name: &str) -> ApplicationResult<()>;
}

/// Hot reload manager implementation
pub struct HotReloadManager {
    /// Configuration
    config: ApplicationConfig,
    /// Registered components
    components: Arc<RwLock<HashMap<String, ComponentSpec>>>,
    /// Reload history
    reload_history: Arc<RwLock<Vec<ReloadEvent>>>,
}

/// Component specification for hot reloading
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentSpec {
    /// Component name
    pub name: String,
    /// Component type
    pub component_type: ComponentType,
    /// File paths to watch
    pub watch_paths: Vec<PathBuf>,
    /// Reload strategy
    pub reload_strategy: ReloadStrategy,
    /// Dependencies (components that must be reloaded first)
    pub dependencies: Vec<String>,
    /// Configuration
    pub config: serde_json::Value,
}

/// Types of components that can be hot reloaded
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ComponentType {
    /// Agent behavior
    AgentBehavior,
    /// Configuration
    Configuration,
    /// Plugin
    Plugin,
    /// Service
    Service,
    /// Library
    Library,
}

/// Reload strategy
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ReloadStrategy {
    /// Graceful reload (wait for current operations to complete)
    Graceful,
    /// Immediate reload (interrupt current operations)
    Immediate,
    /// Rolling reload (reload instances one by one)
    Rolling,
    /// Blue-green reload (create new instances, then switch)
    BlueGreen,
}

/// Reload event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReloadEvent {
    /// Component name
    pub component: String,
    /// Reload type
    pub reload_type: ReloadType,
    /// Timestamp
    pub timestamp: std::time::SystemTime,
    /// Success status
    pub success: bool,
    /// Error message (if failed)
    pub error_message: Option<String>,
    /// Reload duration
    pub duration: std::time::Duration,
}

/// Types of reload operations
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ReloadType {
    /// Manual reload triggered by user
    Manual,
    /// Automatic reload triggered by file change
    Automatic,
    /// Scheduled reload
    Scheduled,
    /// Dependency-triggered reload
    Dependency,
}

/// Reload status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReloadStatus {
    /// Total registered components
    pub total_components: usize,
    /// Components by type
    pub components_by_type: HashMap<ComponentType, usize>,
    /// Recent reload events
    pub recent_events: Vec<ReloadEvent>,
    /// Reload statistics
    pub statistics: ReloadStatistics,
    /// Currently reloading components
    pub reloading_components: Vec<String>,
}

/// Reload statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReloadStatistics {
    /// Total reloads performed
    pub total_reloads: u64,
    /// Successful reloads
    pub successful_reloads: u64,
    /// Failed reloads
    pub failed_reloads: u64,
    /// Average reload time
    pub average_reload_time: std::time::Duration,
    /// Success rate
    pub success_rate: f64,
}

impl HotReloadManager {
    /// Create a new hot reload manager
    pub async fn new(config: ApplicationConfig) -> ApplicationResult<Self> {
        Ok(Self {
            config,
            components: Arc::new(RwLock::new(HashMap::new())),
            reload_history: Arc::new(RwLock::new(Vec::new())),
        })
    }

    /// Record reload event
    async fn record_reload_event(&self, event: ReloadEvent) {
        let mut history = self.reload_history.write().await;
        history.push(event);

        // Keep only recent events (last 100)
        if history.len() > 100 {
            history.remove(0);
        }
    }

    /// Calculate reload statistics
    async fn calculate_statistics(&self) -> ReloadStatistics {
        let history = self.reload_history.read().await;

        let total_reloads = history.len() as u64;
        let successful_reloads = history.iter().filter(|e| e.success).count() as u64;
        let failed_reloads = total_reloads - successful_reloads;

        let average_reload_time = if !history.is_empty() {
            let total_duration: std::time::Duration = history.iter().map(|e| e.duration).sum();
            total_duration / history.len() as u32
        } else {
            std::time::Duration::ZERO
        };

        let success_rate = if total_reloads > 0 {
            successful_reloads as f64 / total_reloads as f64
        } else {
            0.0
        };

        ReloadStatistics {
            total_reloads,
            successful_reloads,
            failed_reloads,
            average_reload_time,
            success_rate,
        }
    }

    /// Perform component reload
    async fn perform_reload(
        &self,
        component_name: &str,
        reload_type: ReloadType,
    ) -> ApplicationResult<()> {
        let start_time = std::time::Instant::now();
        let mut success = false;
        let mut error_message = None;

        let component = {
            let components = self.components.read().await;
            components.get(component_name).cloned()
        };

        if let Some(component) = component {
            info!(
                "Reloading component: {} (type: {:?})",
                component_name, component.component_type
            );

            // Reload dependencies first
            for dependency in &component.dependencies {
                if let Err(e) = self
                    .perform_reload(dependency, ReloadType::Dependency)
                    .await
                {
                    warn!("Failed to reload dependency {}: {}", dependency, e);
                }
            }

            // Perform the actual reload based on component type and strategy
            match self.reload_component_impl(&component).await {
                Ok(()) => {
                    success = true;
                    info!("Successfully reloaded component: {}", component_name);
                }
                Err(e) => {
                    error_message = Some(e.to_string());
                    warn!("Failed to reload component {}: {}", component_name, e);
                }
            }
        } else {
            error_message = Some(format!("Component {} not found", component_name));
        }

        // Record the reload event
        let event = ReloadEvent {
            component: component_name.to_string(),
            reload_type,
            timestamp: std::time::SystemTime::now(),
            success,
            error_message: error_message.clone(),
            duration: start_time.elapsed(),
        };

        self.record_reload_event(event).await;

        if success {
            Ok(())
        } else {
            Err(ApplicationError::HotReloadFailed(
                error_message.unwrap_or_else(|| "Unknown error".to_string()),
            ))
        }
    }

    /// Implementation-specific reload logic
    async fn reload_component_impl(&self, component: &ComponentSpec) -> ApplicationResult<()> {
        match component.component_type {
            ComponentType::AgentBehavior => {
                // In a real implementation, this would reload agent behavior code
                debug!("Reloading agent behavior: {}", component.name);
                tokio::time::sleep(std::time::Duration::from_millis(100)).await; // Simulate reload time
                Ok(())
            }
            ComponentType::Configuration => {
                // In a real implementation, this would reload configuration
                debug!("Reloading configuration: {}", component.name);
                tokio::time::sleep(std::time::Duration::from_millis(50)).await;
                Ok(())
            }
            ComponentType::Plugin => {
                // In a real implementation, this would reload plugin
                debug!("Reloading plugin: {}", component.name);
                tokio::time::sleep(std::time::Duration::from_millis(200)).await;
                Ok(())
            }
            ComponentType::Service => {
                // In a real implementation, this would reload service
                debug!("Reloading service: {}", component.name);
                tokio::time::sleep(std::time::Duration::from_millis(300)).await;
                Ok(())
            }
            ComponentType::Library => {
                // In a real implementation, this would reload library
                debug!("Reloading library: {}", component.name);
                tokio::time::sleep(std::time::Duration::from_millis(150)).await;
                Ok(())
            }
        }
    }
}

#[async_trait]
impl HotReloadManagement for HotReloadManager {
    async fn start(&self) -> ApplicationResult<()> {
        info!("Starting hot reload manager");

        if !self.config.hot_reload.enabled {
            info!("Hot reload is disabled in configuration");
            return Ok(());
        }

        // In a real implementation, this would start file watchers
        info!(
            "Hot reload manager started with {} watch paths",
            self.config.hot_reload.watch_paths.len()
        );
        Ok(())
    }

    async fn stop(&self) -> ApplicationResult<()> {
        info!("Stopping hot reload manager");
        // In a real implementation, this would stop file watchers and cleanup
        Ok(())
    }

    async fn reload_component(&self, component: &str) -> ApplicationResult<()> {
        self.perform_reload(component, ReloadType::Manual).await
    }

    async fn reload_all(&self) -> ApplicationResult<()> {
        info!("Reloading all components");

        let component_names: Vec<String> = {
            let components = self.components.read().await;
            components.keys().cloned().collect()
        };

        let mut errors = Vec::new();

        for component_name in component_names {
            if let Err(e) = self.reload_component(&component_name).await {
                errors.push(format!("{}: {}", component_name, e));
            }
        }

        if errors.is_empty() {
            info!("All components reloaded successfully");
            Ok(())
        } else {
            Err(ApplicationError::HotReloadFailed(format!(
                "Failed to reload some components: {}",
                errors.join(", ")
            )))
        }
    }

    async fn get_reload_status(&self) -> ApplicationResult<ReloadStatus> {
        let components = self.components.read().await;
        let history = self.reload_history.read().await;

        let total_components = components.len();
        let mut components_by_type = HashMap::new();

        for component in components.values() {
            *components_by_type
                .entry(component.component_type.clone())
                .or_insert(0) += 1;
        }

        let recent_events = history.iter().rev().take(10).cloned().collect();
        let statistics = self.calculate_statistics().await;

        Ok(ReloadStatus {
            total_components,
            components_by_type,
            recent_events,
            statistics,
            reloading_components: Vec::new(), // In a real implementation, track active reloads
        })
    }

    async fn register_component(&self, component: ComponentSpec) -> ApplicationResult<()> {
        info!("Registering component for hot reload: {}", component.name);

        let mut components = self.components.write().await;
        components.insert(component.name.clone(), component);

        Ok(())
    }

    async fn unregister_component(&self, component_name: &str) -> ApplicationResult<()> {
        info!(
            "Unregistering component from hot reload: {}",
            component_name
        );

        let mut components = self.components.write().await;
        if components.remove(component_name).is_some() {
            Ok(())
        } else {
            Err(ApplicationError::HotReloadFailed(format!(
                "Component {} not found",
                component_name
            )))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ApplicationConfig;

    #[tokio::test]
    async fn test_hot_reload_manager_creation() {
        let config = ApplicationConfig::default();
        let manager = HotReloadManager::new(config).await;
        assert!(manager.is_ok());
    }

    #[tokio::test]
    async fn test_component_registration() {
        let config = ApplicationConfig::default();
        let manager = HotReloadManager::new(config).await.unwrap();

        let component = ComponentSpec {
            name: "test_component".to_string(),
            component_type: ComponentType::AgentBehavior,
            watch_paths: vec![PathBuf::from("test.rs")],
            reload_strategy: ReloadStrategy::Graceful,
            dependencies: Vec::new(),
            config: serde_json::json!({}),
        };

        let result = manager.register_component(component).await;
        assert!(result.is_ok());

        let status = manager.get_reload_status().await.unwrap();
        assert_eq!(status.total_components, 1);
    }

    #[tokio::test]
    async fn test_component_reload() {
        let config = ApplicationConfig::default();
        let manager = HotReloadManager::new(config).await.unwrap();

        let component = ComponentSpec {
            name: "test_component".to_string(),
            component_type: ComponentType::Configuration,
            watch_paths: vec![PathBuf::from("config.toml")],
            reload_strategy: ReloadStrategy::Immediate,
            dependencies: Vec::new(),
            config: serde_json::json!({}),
        };

        manager.register_component(component).await.unwrap();

        let result = manager.reload_component("test_component").await;
        assert!(result.is_ok());

        let status = manager.get_reload_status().await.unwrap();
        assert_eq!(status.statistics.total_reloads, 1);
        assert_eq!(status.statistics.successful_reloads, 1);
    }
}
