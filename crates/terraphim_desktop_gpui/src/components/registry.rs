/// ServiceRegistry for dependency injection and component lifecycle management
///
/// This module provides a centralized registry for managing component dependencies,
/// service instances, and their lifecycles with loose coupling.
use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};

use async_trait::async_trait;
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::components::{ComponentError, ComponentMetadata, LifecycleEvent, ReusableComponent};

/// Service registry for dependency injection and component management
pub struct ServiceRegistry {
    /// Registered services by TypeId
    services: RwLock<HashMap<TypeId, Arc<dyn Any + Send + Sync>>>,

    /// Registered component factories
    component_factories: RwLock<HashMap<String, Box<dyn ComponentFactory>>>,

    /// Component metadata registry
    component_metadata: RwLock<HashMap<String, ComponentMetadata>>,

    /// Service dependency graph
    dependencies: RwLock<HashMap<String, Vec<String>>>,

    /// Service lifecycle listeners
    lifecycle_listeners: Mutex<Vec<Box<dyn LifecycleListener>>>,

    /// Registry configuration
    config: RegistryConfig,

    /// Registry statistics
    stats: RegistryStats,
}

/// Registry configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistryConfig {
    /// Enable automatic dependency resolution
    pub auto_resolve: bool,

    /// Enable circular dependency detection
    pub detect_circular: bool,

    /// Maximum service instantiation attempts
    pub max_attempts: u32,

    /// Service instantiation timeout
    pub service_timeout: Duration,

    /// Enable performance monitoring
    pub enable_performance_monitoring: bool,

    /// Registry name for identification
    pub name: String,
}

impl Default for RegistryConfig {
    fn default() -> Self {
        Self {
            auto_resolve: true,
            detect_circular: true,
            max_attempts: 3,
            service_timeout: Duration::from_secs(30),
            enable_performance_monitoring: true,
            name: "default".to_string(),
        }
    }
}

impl Default for RegistryStats {
    fn default() -> Self {
        Self {
            service_count: 0,
            component_count: 0,
            lookup_count: 0,
            instantiation_count: 0,
            failure_count: 0,
            avg_instantiation_time: Duration::ZERO,
            created_at: Instant::now(),
        }
    }
}

/// Registry statistics
#[derive(Debug, Clone)]
pub struct RegistryStats {
    /// Total number of registered services
    pub service_count: usize,

    /// Total number of registered components
    pub component_count: usize,

    /// Number of service lookups
    pub lookup_count: u64,

    /// Number of successful instantiations
    pub instantiation_count: u64,

    /// Number of failed instantiations
    pub failure_count: u64,

    /// Average instantiation time
    pub avg_instantiation_time: Duration,

    /// Registry creation time
    pub created_at: Instant,
}

/// Registry errors
#[derive(Debug, Clone, Error)]
pub enum RegistryError {
    #[error("Service not registered: {0}")]
    ServiceNotRegistered(String),

    #[error("Component not registered: {0}")]
    ComponentNotRegistered(String),

    #[error("Circular dependency detected: {0}")]
    CircularDependency(String),

    #[error("Dependency not satisfied: {0}")]
    DependencyNotSatisfied(String),

    #[error("Instantiation failed: {0}")]
    InstantiationFailed(String),

    #[error("Type mismatch: expected {expected}, got {actual}")]
    TypeMismatch { expected: String, actual: String },

    #[error("Configuration error: {0}")]
    Configuration(String),

    #[error("Timeout error: {0}")]
    Timeout(String),

    #[error("Registry is locked")]
    Locked,
}

/// Component factory trait for creating component instances
pub trait ComponentFactory: Any + Send + Sync {
    /// Create a new component instance
    fn create(
        &self,
        config: serde_json::Value,
        registry: &ServiceRegistry,
    ) -> Result<Box<dyn Any + Send + Sync>, RegistryError>;

    /// Get component metadata
    fn metadata(&self) -> &ComponentMetadata;

    /// Clone the factory
    fn clone_box(&self) -> Box<dyn ComponentFactory>;

    /// Convert to Any for downcasting
    fn as_any(&self) -> &dyn Any;
}

impl Clone for Box<dyn ComponentFactory> {
    fn clone(&self) -> Box<dyn ComponentFactory> {
        self.clone_box()
    }
}

/// Lifecycle listener trait for component lifecycle events
pub trait LifecycleListener: Send + Sync {
    /// Called when a component lifecycle event occurs
    fn on_lifecycle_event(&self, component_id: &str, event: &LifecycleEvent);

    /// Called when a service is registered
    fn on_service_registered(&self, service_type: &str);

    /// Called when a service is unregistered
    fn on_service_unregistered(&self, service_type: &str);

    /// Convert to Any for downcasting
    fn as_any(&self) -> &dyn Any;
}

/// Service trait for registered services
pub trait Service: Any + Send + Sync {
    /// Get service name
    fn service_name() -> &'static str
    where
        Self: Sized;

    /// Get service dependencies
    fn dependencies() -> Vec<&'static str>
    where
        Self: Sized;

    /// Initialize the service
    fn initialize(&mut self, registry: &ServiceRegistry) -> Result<(), RegistryError>;

    /// Cleanup the service
    fn cleanup(&mut self) -> Result<(), RegistryError>;

    /// Check if service is healthy
    fn health_check(&self) -> Result<(), RegistryError>;

    /// Convert to Any for downcasting
    fn as_any(&self) -> &dyn Any;

    /// Convert to mutable Any for downcasting
    fn as_any_mut(&mut self) -> &mut dyn Any;
}

/// Async service trait for services that need async initialization
#[async_trait]
pub trait AsyncService: Service {
    /// Initialize the service asynchronously
    async fn initialize_async(&mut self, registry: &ServiceRegistry) -> Result<(), RegistryError>;

    /// Cleanup the service asynchronously
    async fn cleanup_async(&mut self) -> Result<(), RegistryError>;

    /// Health check asynchronously
    async fn health_check_async(&self) -> Result<(), RegistryError>;
}

/// Service reference for lazy initialization
#[derive(Debug)]
pub struct ServiceRef<T: ?Sized> {
    inner: Arc<T>,
}

impl<T: ?Sized> ServiceRef<T> {
    /// Create new service reference
    pub fn new(service: Arc<T>) -> Self {
        Self { inner: service }
    }

    /// Get reference to the service
    pub fn get(&self) -> &T {
        &self.inner
    }

    /// Get Arc reference to the service
    pub fn clone_arc(&self) -> Arc<T> {
        Arc::clone(&self.inner)
    }
}

impl<T: ?Sized> Clone for ServiceRef<T> {
    fn clone(&self) -> Self {
        Self {
            inner: Arc::clone(&self.inner),
        }
    }
}

impl<T: ?Sized> std::ops::Deref for ServiceRef<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl ServiceRegistry {
    /// Create new service registry
    pub fn new(config: RegistryConfig) -> Self {
        Self {
            services: RwLock::new(HashMap::new()),
            component_factories: RwLock::new(HashMap::new()),
            component_metadata: RwLock::new(HashMap::new()),
            dependencies: RwLock::new(HashMap::new()),
            lifecycle_listeners: Mutex::new(Vec::new()),
            config,
            stats: RegistryStats {
                created_at: Instant::now(),
                ..Default::default()
            },
        }
    }

    /// Create service registry with default configuration
    pub fn default() -> Self {
        Self::new(RegistryConfig::default())
    }

    /// Register a service instance
    pub fn register_service<T: Service + 'static>(&self, service: T) -> Result<(), RegistryError> {
        let type_name = std::any::type_name::<T>();
        let type_id = TypeId::of::<T>();

        // Check for existing service
        {
            let services = self.services.read().map_err(|_| RegistryError::Locked)?;
            if services.contains_key(&type_id) {
                return Err(RegistryError::InstantiationFailed(format!(
                    "Service {} already registered",
                    type_name
                )));
            }
        }

        // Validate dependencies
        for dep in T::dependencies() {
            if !self.is_service_registered(dep) {
                return Err(RegistryError::DependencyNotSatisfied(dep.to_string()));
            }
        }

        // Store the service
        {
            let mut services = self.services.write().map_err(|_| RegistryError::Locked)?;
            services.insert(type_id, Arc::new(service));
        }

        // Update stats
        {
            let mut services = self.services.read().map_err(|_| RegistryError::Locked)?;
            self.stats.service_count = services.len();
        }

        // Notify listeners
        self.notify_service_registered(type_name);

        Ok(())
    }

    /// Register a service factory for lazy initialization
    pub fn register_service_factory<T, F>(&self, factory: F) -> Result<(), RegistryError>
    where
        T: Service + 'static,
        F: Fn() -> T + Send + Sync + 'static,
    {
        // This is a placeholder for factory-based service registration
        // Implementation would depend on specific factory requirements
        todo!("Service factory registration not yet implemented")
    }

    /// Get a service instance
    pub fn get_service<T: Service + 'static>(&self) -> Result<ServiceRef<T>, RegistryError> {
        let type_id = TypeId::of::<T>();

        // Update stats
        self.stats.lookup_count += 1;

        let services = self.services.read().map_err(|_| RegistryError::Locked)?;

        match services.get(&type_id) {
            Some(service) => {
                let service =
                    service
                        .clone()
                        .downcast::<T>()
                        .map_err(|_| RegistryError::TypeMismatch {
                            expected: std::any::type_name::<T>().to_string(),
                            actual: "unknown".to_string(),
                        })?;

                Ok(ServiceRef::new(service))
            }
            None => Err(RegistryError::ServiceNotRegistered(
                std::any::type_name::<T>().to_string(),
            )),
        }
    }

    /// Check if a service is registered
    pub fn is_service_registered(&self, service_name: &str) -> bool {
        let services = self.services.read().ok()?;

        services.values().any(|service| {
            // This is a simplified check - in practice, you'd want to maintain
            // a mapping from type names to TypeIds for efficient lookup
            service.as_ref().type_id() == TypeId::of::<()>() // Placeholder
        })
    }

    /// Unregister a service
    pub fn unregister_service<T: Service + 'static>(&self) -> Result<(), RegistryError> {
        let type_name = std::any::type_name::<T>();
        let type_id = TypeId::of::<T>();

        {
            let mut services = self.services.write().map_err(|_| RegistryError::Locked)?;
            services
                .remove(&type_id)
                .ok_or_else(|| RegistryError::ServiceNotRegistered(type_name.to_string()))?;
        }

        // Update stats
        {
            let services = self.services.read().map_err(|_| RegistryError::Locked)?;
            self.stats.service_count = services.len();
        }

        // Notify listeners
        self.notify_service_unregistered(type_name);

        Ok(())
    }

    /// Register a component factory
    pub fn register_component_factory<F: ComponentFactory + 'static>(
        &self,
        factory: F,
    ) -> Result<(), RegistryError> {
        let metadata = factory.metadata();
        let component_id = metadata.id.clone();

        // Store factory
        {
            let mut factories = self
                .component_factories
                .write()
                .map_err(|_| RegistryError::Locked)?;
            factories.insert(component_id.clone(), Box::new(factory));
        }

        // Store metadata
        {
            let mut metadata_map = self
                .component_metadata
                .write()
                .map_err(|_| RegistryError::Locked)?;
            metadata_map.insert(component_id.clone(), metadata.clone());
        }

        // Store dependencies
        {
            let mut dependencies = self
                .dependencies
                .write()
                .map_err(|_| RegistryError::Locked)?;
            dependencies.insert(component_id, metadata.dependencies);
        }

        // Update stats
        {
            let factories = self
                .component_factories
                .read()
                .map_err(|_| RegistryError::Locked)?;
            self.stats.component_count = factories.len();
        }

        Ok(())
    }

    /// Create a component instance
    pub fn create_component(
        &self,
        component_id: &str,
        config: serde_json::Value,
    ) -> Result<Box<dyn Any + Send + Sync>, RegistryError> {
        let factories = self
            .component_factories
            .read()
            .map_err(|_| RegistryError::Locked)?;

        let factory = factories
            .get(component_id)
            .ok_or_else(|| RegistryError::ComponentNotRegistered(component_id.to_string()))?;

        let start_time = Instant::now();

        match factory.create(config, self) {
            Ok(component) => {
                // Update stats
                self.stats.instantiation_count += 1;
                let duration = start_time.elapsed();
                self.update_avg_instantiation_time(duration);

                Ok(component)
            }
            Err(e) => {
                self.stats.failure_count += 1;
                Err(e)
            }
        }
    }

    /// Get component metadata
    pub fn get_component_metadata(&self, component_id: &str) -> Option<ComponentMetadata> {
        let metadata = self.component_metadata.read().ok()?;
        metadata.get(component_id).cloned()
    }

    /// Get all registered component metadata
    pub fn list_components(&self) -> Vec<ComponentMetadata> {
        let metadata = self.component_metadata.read().unwrap();
        metadata.values().cloned().collect()
    }

    /// Add lifecycle listener
    pub fn add_lifecycle_listener<L: LifecycleListener + 'static>(&self, listener: L) {
        let mut listeners = self.lifecycle_listeners.lock();
        listeners.push(Box::new(listener));
    }

    /// Notify lifecycle listeners
    fn notify_lifecycle_event(&self, component_id: &str, event: &LifecycleEvent) {
        let listeners = self.lifecycle_listeners.lock();
        for listener in listeners.iter() {
            listener.on_lifecycle_event(component_id, event);
        }
    }

    /// Notify service registration
    fn notify_service_registered(&self, service_type: &str) {
        let listeners = self.lifecycle_listeners.lock();
        for listener in listeners.iter() {
            listener.on_service_registered(service_type);
        }
    }

    /// Notify service unregistration
    fn notify_service_unregistered(&self, service_type: &str) {
        let listeners = self.lifecycle_listeners.lock();
        for listener in listeners.iter() {
            listener.on_service_unregistered(service_type);
        }
    }

    /// Check for circular dependencies
    fn detect_circular_dependencies(
        &self,
        component_id: &str,
        visited: &mut std::collections::HashSet<String>,
    ) -> Result<(), RegistryError> {
        if visited.contains(component_id) {
            return Err(RegistryError::CircularDependency(format!(
                "Circular dependency detected: {} -> {}",
                component_id,
                visited
                    .iter()
                    .map(|s| s.as_str())
                    .collect::<Vec<_>>()
                    .join(" -> ")
            )));
        }

        visited.insert(component_id.to_string());

        let dependencies = self
            .dependencies
            .read()
            .map_err(|_| RegistryError::Locked)?;
        if let Some(deps) = dependencies.get(component_id) {
            for dep in deps {
                self.detect_circular_dependencies(dep, visited)?;
            }
        }

        visited.remove(component_id);
        Ok(())
    }

    /// Update average instantiation time
    fn update_avg_instantiation_time(&mut self, duration: Duration) {
        let total = self.stats.instantiation_count;
        if total > 0 {
            let current_total = self.stats.avg_instantiation_time * (total - 1) as u32;
            self.stats.avg_instantiation_time = (current_total + duration) / total as u32;
        }
    }

    /// Get registry statistics
    pub fn stats(&self) -> RegistryStats {
        self.stats.clone()
    }

    /// Perform health check on all services
    pub async fn health_check(&self) -> HashMap<String, Result<(), RegistryError>> {
        let mut results = HashMap::new();

        let services = self.services.read().unwrap();
        for (type_id, service) in services.iter() {
            let type_name = format!("{:?}", type_id);

            // This is a simplified health check - in practice, you'd want
            // to maintain a mapping from TypeIds to service names
            let result = service.as_ref().type_id() == TypeId::of::<()>();

            if result {
                results.insert(type_name, Ok(()));
            } else {
                results.insert(
                    type_name,
                    Err(RegistryError::InstantiationFailed(
                        "Health check not implemented for service type".to_string(),
                    )),
                );
            }
        }

        results
    }

    /// Shutdown the registry and cleanup all services
    pub async fn shutdown(&self) -> Result<(), RegistryError> {
        let services = self.services.read().map_err(|_| RegistryError::Locked)?;

        for (type_id, _service) in services.iter() {
            // In a real implementation, you'd call cleanup on each service
            // This is a placeholder for cleanup logic
            println!("Cleaning up service: {:?}", type_id);
        }

        // Clear all registries
        drop(services);

        {
            let mut services = self.services.write().map_err(|_| RegistryError::Locked)?;
            services.clear();
        }

        {
            let mut factories = self
                .component_factories
                .write()
                .map_err(|_| RegistryError::Locked)?;
            factories.clear();
        }

        {
            let mut metadata = self
                .component_metadata
                .write()
                .map_err(|_| RegistryError::Locked)?;
            metadata.clear();
        }

        {
            let mut dependencies = self
                .dependencies
                .write()
                .map_err(|_| RegistryError::Locked)?;
            dependencies.clear();
        }

        Ok(())
    }
}

/// Macro to easily implement ComponentFactory
#[macro_export]
macro_rules! impl_component_factory {
    ($component_type:ty, $config_type:ty, $constructor:expr) => {
        impl $crate::components::registry::ComponentFactory for $component_type {
            fn create(
                &self,
                config: serde_json::Value,
                registry: &$crate::ServiceRegistry,
            ) -> Result<
                Box<dyn std::any::Any + Send + Sync>,
                $crate::components::registry::RegistryError,
            > {
                let config: $config_type = serde_json::from_value(config).map_err(|e| {
                    $crate::components::registry::RegistryError::Configuration(e.to_string())
                })?;

                let component = $constructor(config, registry)?;
                Ok(Box::new(component))
            }

            fn metadata(&self) -> &$crate::components::ComponentMetadata {
                &Self::METADATA
            }

            fn clone_box(&self) -> Box<dyn $crate::components::registry::ComponentFactory> {
                Box::new(self.clone())
            }

            fn as_any(&self) -> &dyn std::any::Any {
                self
            }
        }
    };
}
