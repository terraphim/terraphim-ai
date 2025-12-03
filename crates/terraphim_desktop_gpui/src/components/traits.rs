/// ReusableComponent trait and core abstractions
///
/// This module defines the foundational trait that all reusable components
/// must implement, providing standardized lifecycle management, configuration,
/// and performance monitoring capabilities.

use std::any::Any;
use std::fmt::Debug;
use std::sync::Arc;
use std::time::{Duration, Instant};

use gpui::*;
use serde::{Deserialize, Serialize};

use crate::components::{ComponentConfig, PerformanceTracker};
use super::registry::ServiceRegistry;

/// Import GPUI types we need
pub type ViewContext<'a, T> = gpui::Context<'a, T>;

/// Core trait that all reusable components must implement
///
/// This trait provides standardized lifecycle management, configuration handling,
/// performance monitoring, and state management for reusable components.
pub trait ReusableComponent: 'static + Send + Sync + Debug + Sized {
    /// Configuration type for this component
    type Config: ComponentConfig + Clone + Send + Sync + 'static;

    /// State type for this component
    type State: Clone + Send + Sync + 'static;

    /// Event type that this component can emit
    type Event: Clone + Send + Sync + Debug + 'static;

    /// Component identifier
    fn component_id() -> &'static str
    where
        Self: Sized;

    /// Component version for compatibility tracking
    fn component_version() -> &'static str
    where
        Self: Sized;

    /// Initialize the component with configuration
    fn init(config: Self::Config) -> Self
    where
        Self: Sized;

    /// Get current component configuration
    fn config(&self) -> &Self::Config;

    /// Update component configuration
    fn update_config(&mut self, config: Self::Config) -> Result<(), ComponentError>;

    /// Get current component state
    fn state(&self) -> &Self::State;

    /// Update component state
    fn update_state(&mut self, state: Self::State) -> Result<(), ComponentError>;

    /// Mount the component into the GPUI entity tree
    fn mount(&mut self, cx: &mut ViewContext<Self>) -> Result<(), ComponentError>;

    /// Unmount the component from GPUI entity tree
    fn unmount(&mut self, cx: &mut ViewContext<Self>) -> Result<(), ComponentError>;

    /// Handle component lifecycle events
    fn handle_lifecycle_event(&mut self, event: LifecycleEvent, cx: &mut ViewContext<Self>) -> Result<(), ComponentError>;

    /// Check if component is currently mounted/active
    fn is_mounted(&self) -> bool;

    /// Get component performance metrics
    fn performance_metrics(&self) -> &PerformanceTracker;

    /// Reset performance metrics
    fn reset_performance_metrics(&mut self);

    /// Get component dependencies
    fn dependencies(&self) -> Vec<&'static str>;

    /// Check if component dependencies are satisfied
    fn are_dependencies_satisfied(&self, registry: &ServiceRegistry) -> bool;

    /// Perform component cleanup
    fn cleanup(&mut self) -> Result<(), ComponentError>;

    /// Convert to Any for downcasting
    fn as_any(&self) -> &dyn Any;

    /// Convert to mutable Any for downcasting
    fn as_any_mut(&mut self) -> &mut dyn Any;
}

/// Lifecycle events that components can handle
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LifecycleEvent {
    /// Component is being mounted
    Mounting,
    /// Component has been successfully mounted
    Mounted,
    /// Component is being unmounted
    Unmounting,
    /// Component has been unmounted
    Unmounted,
    /// Component configuration changed
    ConfigChanged,
    /// Component state changed
    StateChanged,
    /// Component dependencies changed
    DependenciesChanged,
    /// Component is being suspended (for performance optimization)
    Suspending,
    /// Component has been resumed
    Resumed,
    /// Component is being reloaded
    Reloading,
    /// Component has been reloaded
    Reloaded,
}

/// Component errors
#[derive(Debug, Clone, thiserror::Error)]
pub enum ComponentError {
    #[error("Component not mounted")]
    NotMounted,

    #[error("Component already mounted")]
    AlreadyMounted,

    #[error("Configuration error: {0}")]
    Configuration(String),

    #[error("State error: {0}")]
    State(String),

    #[error("Dependency error: {0}")]
    Dependency(String),

    #[error("Performance error: {0}")]
    Performance(String),

    #[error("IO error: {0}")]
    Io(String),

    #[error("Serialization error: {0}")]
    Serialization(String),

    #[error("Unknown error: {0}")]
    Unknown(String),
}

/// Component capabilities that can be advertised
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ComponentCapability {
    /// Component can be searched
    Searchable,
    /// Component can be filtered
    Filterable,
    /// Component supports keyboard navigation
    KeyboardNavigable,
    /// Component is accessible
    Accessible,
    /// Component supports theming
    Themeable,
    /// Component is configurable
    Configurable,
    /// Component supports real-time updates
    RealTimeUpdates,
    /// Component is virtualized (handles large datasets)
    Virtualized,
    /// Component supports animations
    Animated,
    /// Component is responsive
    Responsive,
}

/// Component metadata for registry and discovery
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentMetadata {
    /// Unique component identifier
    pub id: String,

    /// Component version
    pub version: String,

    /// Human-readable component name
    pub name: String,

    /// Component description
    pub description: String,

    /// Component author
    pub author: String,

    /// Component capabilities
    pub capabilities: Vec<ComponentCapability>,

    /// Component dependencies
    pub dependencies: Vec<String>,

    /// Component tags for categorization
    pub tags: Vec<String>,

    /// Creation timestamp
    pub created_at: std::time::SystemTime,

    /// Last updated timestamp
    pub updated_at: std::time::SystemTime,
}

impl ComponentMetadata {
    /// Create new component metadata
    pub fn new(
        id: String,
        version: String,
        name: String,
        description: String,
        author: String,
    ) -> Self {
        let now = std::time::SystemTime::now();
        Self {
            id,
            version,
            name,
            description,
            author,
            capabilities: Vec::new(),
            dependencies: Vec::new(),
            tags: Vec::new(),
            created_at: now,
            updated_at: now,
        }
    }

    /// Add capability to component
    pub fn with_capability(mut self, capability: ComponentCapability) -> Self {
        self.capabilities.push(capability);
        self
    }

    /// Add dependency to component
    pub fn with_dependency(mut self, dependency: String) -> Self {
        self.dependencies.push(dependency);
        self
    }

    /// Add tag to component
    pub fn with_tag(mut self, tag: String) -> Self {
        self.tags.push(tag);
        self
    }

    /// Check if component has specific capability
    pub fn has_capability(&self, capability: &ComponentCapability) -> bool {
        self.capabilities.contains(capability)
    }

    /// Check if component depends on another component
    pub fn depends_on(&self, component_id: &str) -> bool {
        self.dependencies.contains(&component_id.to_string())
    }

    /// Mark as updated
    pub fn mark_updated(&mut self) {
        self.updated_at = std::time::SystemTime::now();
    }
}

/// Component state snapshot for persistence
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentSnapshot {
    /// Component identifier
    pub component_id: String,

    /// Component version when snapshot was taken
    pub version: String,

    /// Serialized component state
    pub state: serde_json::Value,

    /// Serialized component configuration
    pub config: serde_json::Value,

    /// Snapshot timestamp
    pub timestamp: std::time::SystemTime,

    /// Performance metrics at time of snapshot
    pub performance_metrics: serde_json::Value,
}

impl ComponentSnapshot {
    /// Create new component snapshot
    pub fn new<C: ComponentConfig, S: Serialize>(
        component_id: String,
        version: String,
        state: &S,
        config: &C,
        performance_metrics: &serde_json::Value,
    ) -> Result<Self, ComponentError> {
        Ok(Self {
            component_id,
            version,
            state: serde_json::to_value(state)
                .map_err(|e| ComponentError::Serialization(e.to_string()))?,
            config: serde_json::to_value(config)
                .map_err(|e| ComponentError::Serialization(e.to_string()))?,
            timestamp: std::time::SystemTime::now(),
            performance_metrics: performance_metrics.clone(),
        })
    }
}

/// Helper trait for component builders
pub trait ComponentBuilder: Sized {
    type Component: ReusableComponent;

    /// Create new component builder
    fn new() -> Self;

    /// Set component configuration
    fn with_config(self, config: <Self::Component as ReusableComponent>::Config) -> Self;

    /// Set component initial state
    fn with_state(self, state: <Self::Component as ReusableComponent>::State) -> Self;

    /// Build the component
    fn build(self) -> Result<Self::Component, ComponentError>;
}

/// Default implementation for component builders
#[derive(Debug, Clone)]
pub struct DefaultComponentBuilder<C: ReusableComponent> {
    config: Option<C::Config>,
    state: Option<C::State>,
    _phantom: std::marker::PhantomData<C>,
}

impl<C: ReusableComponent> DefaultComponentBuilder<C> {
    pub fn new() -> Self {
        Self {
            config: None,
            state: None,
            _phantom: std::marker::PhantomData,
        }
    }

    pub fn with_config(mut self, config: C::Config) -> Self {
        self.config = Some(config);
        self
    }

    pub fn with_state(mut self, state: C::State) -> Self {
        self.state = Some(state);
        self
    }

    pub fn build(self) -> Result<C, ComponentError> {
        let config = self.config
            .ok_or_else(|| ComponentError::Configuration("No configuration provided".to_string()))?;

        let mut component = C::init(config);

        if let Some(state) = self.state {
            component.update_state(state)?;
        }

        Ok(component)
    }
}

/// Macro to easily implement ReusableComponent for structs
#[macro_export]
macro_rules! impl_reusable_component {
    (
        $(#[$meta:meta])*
        $vis:vis struct $name:ident {
            $(
                $field_vis:vis $field_name:ident : $field_type:ty
            ),* $(,)?
        }

        config: $config_type:ty,
        state: $state_type:ty,
        event: $event_type:ty,
        component_id: $component_id:expr,
        component_version: $component_version:expr,
    ) => {
        $(#[$meta])*
        $vis struct $name {
            $(
                $field_vis $field_name : $field_type,
            )*
        }

        impl $crate::components::traits::ReusableComponent for $name {
            type Config = $config_type;
            type State = $state_type;
            type Event = $event_type;

            fn component_id() -> &'static str {
                $component_id
            }

            fn component_version() -> &'static str {
                $component_version
            }

            fn init(config: Self::Config) -> Self {
                Self {
                    $(
                        $field_name: Default::default(),
                    )*
                }
            }

            fn config(&self) -> &Self::Config {
                &self.config
            }

            fn update_config(&mut self, config: Self::Config) -> Result<(), $crate::components::traits::ComponentError> {
                self.config = config;
                Ok(())
            }

            fn state(&self) -> &Self::State {
                &self.state
            }

            fn update_state(&mut self, state: Self::State) -> Result<(), $crate::components::traits::ComponentError> {
                self.state = state;
                Ok(())
            }

            fn mount(&mut self, cx: &mut gpui::ViewContext<Self>) -> Result<(), $crate::components::traits::ComponentError>
            where
                Self: gpui::Entity,
            {
                self.is_mounted = true;
                Ok(())
            }

            fn unmount(&mut self, cx: &mut gpui::ViewContext<Self>) -> Result<(), $crate::components::traits::ComponentError>
            where
                Self: gpui::Entity,
            {
                self.is_mounted = false;
                Ok(())
            }

            fn handle_lifecycle_event(&mut self, event: $crate::components::traits::LifecycleEvent, cx: &mut gpui::ViewContext<Self>) -> Result<(), $crate::components::traits::ComponentError>
            where
                Self: gpui::Entity,
            {
                // Default implementation - can be overridden
                Ok(())
            }

            fn is_mounted(&self) -> bool {
                self.is_mounted
            }

            fn performance_metrics(&self) -> &$crate::components::PerformanceTracker {
                &self.performance_tracker
            }

            fn reset_performance_metrics(&mut self) {
                self.performance_tracker.reset();
            }

            fn dependencies(&self) -> Vec<&'static str> {
                Vec::new()
            }

            fn are_dependencies_satisfied(&self, registry: &$crate::ServiceRegistry) -> bool {
                true
            }

            fn cleanup(&mut self) -> Result<(), $crate::components::traits::ComponentError> {
                Ok(())
            }

            fn as_any(&self) -> &dyn std::any::Any {
                self
            }

            fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
                self
            }
        }
    };
}