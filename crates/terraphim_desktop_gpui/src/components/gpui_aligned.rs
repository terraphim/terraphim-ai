use gpui::*;
use serde::{Deserialize, Serialize};
/// GPUI-Aligned Component System
///
/// Simplified component patterns aligned with gpui-component best practices.
/// Focuses on stateless RenderOnce components, Theme support, and practical reusability.
/// Based on analysis of longbridge/gpui-component patterns.
use std::any::Any;
use std::marker::PhantomData;

/// Core component trait aligned with GPUI patterns
///
/// Simpler than the original ReusableComponent, focusing on:
/// - Stateless RenderOnce patterns where possible
/// - Theme integration
/// - Practical reusability without excessive abstraction
/// - Easy testing and debugging
pub trait GpuiComponent: 'static + Send + Sync {
    /// Component state (if any)
    type State: Default + Send + Sync + 'static;

    /// Component configuration
    type Config: Clone + Send + Sync + 'static;

    /// Component events
    type Event: Clone + Send + Sync + 'static;

    /// Component identifier for debugging and profiling
    fn component_name() -> &'static str;

    /// Create a new component instance
    fn new(config: Self::Config) -> Self;

    /// Render the component
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement;

    /// Handle component configuration updates
    fn update_config(&mut self, config: Self::Config, cx: &mut Context<Self>) {
        // Default implementation - components can override
    }

    /// Get current configuration
    fn config(&self) -> &Self::Config;

    /// Get current state
    fn state(&self) -> &Self::State;

    /// Optional: Handle custom events
    fn handle_event(&mut self, event: Self::Event, cx: &mut Context<Self>) {
        // Default implementation - components can override
    }
}

/// Simplified stateful component base
///
/// For components that need to maintain state between renders.
/// Uses standard GPUI patterns without excessive abstraction.
pub struct StatefulComponent<T: GpuiComponent> {
    component: T,
    _phantom: PhantomData<T>,
}

impl<T: GpuiComponent> StatefulComponent<T> {
    pub fn new(config: T::Config) -> Self {
        Self {
            component: T::new(config),
            _phantom: PhantomData,
        }
    }

    pub fn update_config(&mut self, config: T::Config, cx: &mut Context<Self>) {
        self.component.update_config(config, cx);
        cx.notify();
    }

    pub fn handle_event(&mut self, event: T::Event, cx: &mut Context<Self>) {
        self.component.handle_event(event, cx);
        cx.notify();
    }

    pub fn config(&self) -> &T::Config {
        self.component.config()
    }

    pub fn state(&self) -> &T::State {
        self.component.state()
    }
}

impl<T: GpuiComponent> Render for StatefulComponent<T> {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.component.render(window, cx)
    }
}

/// Stateless component trait for simple components
///
/// Based on gpui-component's RenderOnce pattern.
/// Ideal for components that don't need internal state.
pub trait StatelessComponent: 'static + Send + Sync {
    /// Component configuration
    type Config: Clone + Send + Sync + 'static;

    /// Component identifier
    fn component_name() -> &'static str;

    /// Render the component (stateless)
    fn render(
        config: &Self::Config,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement;
}

/// Wrapper for stateless components
pub struct StatelessWrapper<T: StatelessComponent> {
    config: T::Config,
    _phantom: PhantomData<T>,
}

impl<T: StatelessComponent> StatelessWrapper<T> {
    pub fn new(config: T::Config) -> Self {
        Self {
            config,
            _phantom: PhantomData,
        }
    }

    pub fn update_config(&mut self, config: T::Config, cx: &mut Context<Self>) {
        self.config = config;
        cx.notify();
    }

    pub fn config(&self) -> &T::Config {
        &self.config
    }
}

impl<T: StatelessComponent> Render for StatelessWrapper<T> {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        T::render(&self.config, window, cx)
    }
}

/// Component configuration trait
///
/// Simplified configuration pattern aligned with gpui-component
pub trait GpuiComponentConfig: Clone + Send + Sync + 'static {
    /// Validate configuration
    fn validate(&self) -> Result<(), String> {
        Ok(())
    }

    /// Get default configuration
    fn default() -> Self
    where
        Self: Sized;

    /// Merge with another configuration (for updates)
    fn merge_with(&self, other: &Self) -> Self;
}

/// Theme-aware component trait
///
/// Integration with GPUI's theme system
pub trait ThemeAware {
    /// Apply theme changes to the component
    fn theme_changed(&mut self, cx: &mut Context<Self>);
}

/// Size variants for responsive components
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum ComponentSize {
    XSmall,
    Small,
    Medium,
    Large,
    XLarge,
}

impl Default for ComponentSize {
    fn default() -> Self {
        ComponentSize::Medium
    }
}

/// Component variant for different styles
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ComponentVariant {
    Default,
    Primary,
    Secondary,
    Success,
    Warning,
    Error,
}

impl Default for ComponentVariant {
    fn default() -> Self {
        ComponentVariant::Default
    }
}

/// Common component properties
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommonProps {
    pub size: ComponentSize,
    pub variant: ComponentVariant,
    pub disabled: bool,
    pub test_id: Option<String>,
}

impl Default for CommonProps {
    fn default() -> Self {
        Self {
            size: ComponentSize::default(),
            variant: ComponentVariant::default(),
            disabled: false,
            test_id: None,
        }
    }
}

impl GpuiComponentConfig for CommonProps {
    fn validate(&self) -> Result<(), String> {
        if let Some(ref test_id) = self.test_id {
            if test_id.is_empty() {
                return Err("test_id cannot be empty".to_string());
            }
        }
        Ok(())
    }

    fn default() -> Self {
        Self::default()
    }

    fn merge_with(&self, other: &Self) -> Self {
        Self {
            size: other.size,
            variant: other.variant,
            disabled: other.disabled,
            test_id: other.test_id.clone().or_else(|| self.test_id.clone()),
        }
    }
}

/// Utility functions for component development
pub mod utils {
    use super::*;

    /// Create a conditional class name
    pub fn conditional_class(condition: bool, class_name: &str) -> Option<&'static str> {
        if condition { Some(class_name) } else { None }
    }

    /// Generate test ID for component
    pub fn test_id(component_name: &str, specific_id: Option<&str>) -> Option<String> {
        Some(match specific_id {
            Some(id) => format!("{}-{}", component_name, id),
            None => component_name.to_string(),
        })
    }

    /// Apply common GPUI styling patterns
    pub fn apply_common_styling(
        size: ComponentSize,
        variant: ComponentVariant,
        base_classes: Vec<&str>,
    ) -> Vec<&'static str> {
        let mut classes = base_classes;

        // Add size classes
        match size {
            ComponentSize::XSmall => classes.push("size-xs"),
            ComponentSize::Small => classes.push("size-sm"),
            ComponentSize::Medium => classes.push("size-md"),
            ComponentSize::Large => classes.push("size-lg"),
            ComponentSize::XLarge => classes.push("size-xl"),
        }

        // Add variant classes
        match variant {
            ComponentVariant::Default => {} // No additional class
            ComponentVariant::Primary => classes.push("variant-primary"),
            ComponentVariant::Secondary => classes.push("variant-secondary"),
            ComponentVariant::Success => classes.push("variant-success"),
            ComponentVariant::Warning => classes.push("variant-warning"),
            ComponentVariant::Error => classes.push("variant-error"),
        }

        classes
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_component_size_default() {
        assert_eq!(ComponentSize::default(), ComponentSize::Medium);
    }

    #[test]
    fn test_component_variant_default() {
        assert_eq!(ComponentVariant::default(), ComponentVariant::Default);
    }

    #[test]
    fn test_common_props_validation() {
        let mut props = CommonProps::default();
        assert!(props.validate().is_ok());

        props.test_id = Some("".to_string());
        assert!(props.validate().is_err());
    }

    #[test]
    fn test_common_props_merge() {
        let props1 = CommonProps {
            size: ComponentSize::Small,
            variant: ComponentVariant::Primary,
            disabled: false,
            test_id: Some("test1".to_string()),
        };

        let props2 = CommonProps {
            size: ComponentSize::Large,
            variant: ComponentVariant::Secondary,
            disabled: true,
            test_id: None,
        };

        let merged = props1.merge_with(&props2);
        assert_eq!(merged.size, ComponentSize::Large);
        assert_eq!(merged.variant, ComponentVariant::Secondary);
        assert_eq!(merged.disabled, true);
        assert_eq!(merged.test_id, Some("test1".to_string()));
    }

    #[test]
    fn test_conditional_class() {
        assert_eq!(utils::conditional_class(true, "active"), Some("active"));
        assert_eq!(utils::conditional_class(false, "active"), None);
    }

    #[test]
    fn test_test_id_generation() {
        assert_eq!(
            utils::test_id("button", Some("submit")),
            Some("button-submit".to_string())
        );
        assert_eq!(utils::test_id("input", None), Some("input".to_string()));
    }
}
