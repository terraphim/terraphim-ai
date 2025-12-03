#![recursion_limit = "1024"]

/// Comprehensive tests for the reusable components foundation
///
/// These tests validate the core abstractions, service registry,
/// performance tracking, and configuration management components.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use tokio::time::sleep;

use terraphim_desktop_gpui::components::{
    ComponentConfig, ComponentError, ComponentMetadata, ConfigValue,
    LifecycleEvent, PerformanceTracker, ReusableComponent, ServiceRegistry,
    Service, ComponentFactory, RegistryError
};
use terraphim_desktop_gpui::components::testing::{ComponentTestUtils, PerformanceTestUtils, ComponentTestHarness};

/// Test service implementation
#[derive(Debug)]
struct TestService {
    name: String,
    initialized: bool,
}

impl TestService {
    fn new(name: String) -> Self {
        Self {
            name,
            initialized: false,
        }
    }
}

impl Service for TestService {
    fn service_name() -> &'static str {
        "TestService"
    }

    fn dependencies() -> Vec<&'static str> {
        Vec::new()
    }

    fn initialize(&mut self, _registry: &ServiceRegistry) -> Result<(), RegistryError> {
        self.initialized = true;
        Ok(())
    }

    fn cleanup(&mut self) -> Result<(), RegistryError> {
        self.initialized = false;
        Ok(())
    }

    fn health_check(&self) -> Result<(), RegistryError> {
        if self.initialized {
            Ok(())
        } else {
            Err(RegistryError::InstantiationFailed("Service not initialized".to_string()))
        }
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

/// Test component configuration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct TestComponentConfig {
    pub enabled: bool,
    pub max_items: u32,
    pub timeout_ms: u64,
    pub name: String,
}

impl Default for TestComponentConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            max_items: 100,
            timeout_ms: 5000,
            name: "test-component".to_string(),
        }
    }
}

impl ComponentConfig for TestComponentConfig {
    fn schema() -> terraphim_desktop_gpui::components::ConfigSchema {
        use terraphim_desktop_gpui::components::{ConfigSchema, ConfigField, ConfigFieldType, ValidationRule};

        ConfigSchema::new(
            "TestComponent".to_string(),
            "1.0.0".to_string(),
            "Test component configuration".to_string(),
        )
        .with_field(ConfigField {
            name: "enabled".to_string(),
            field_type: ConfigFieldType::Boolean,
            required: false,
            default: Some(ConfigValue::Boolean(true)),
            description: "Whether the component is enabled".to_string(),
            validation: vec![],
            docs: None,
        })
        .with_field(ConfigField {
            name: "max_items".to_string(),
            field_type: ConfigFieldType::Integer,
            required: false,
            default: Some(ConfigValue::Integer(100)),
            description: "Maximum number of items".to_string(),
            validation: vec![ValidationRule::MinValue(0.0), ValidationRule::MaxValue(1000.0)],
            docs: None,
        })
    }

    fn validate(&self) -> Result<(), terraphim_desktop_gpui::components::ConfigError> {
        if self.max_items == 0 {
            return Err(terraphim_desktop_gpui::components::ConfigError::Validation(
                "max_items must be greater than 0".to_string()
            ));
        }
        Ok(())
    }

    fn default() -> Self {
        Self::default()
    }

    fn merge(&self, other: &Self) -> Result<Self, terraphim_desktop_gpui::components::ConfigError> {
        Ok(Self {
            enabled: other.enabled,
            max_items: if other.max_items != 100 { other.max_items } else { self.max_items },
            timeout_ms: if other.timeout_ms != 5000 { other.timeout_ms } else { self.timeout_ms },
            name: if other.name != "test-component" { other.name.clone() } else { self.name.clone() },
        })
    }

    fn to_map(&self) -> HashMap<String, ConfigValue> {
        let mut map = HashMap::new();
        map.insert("enabled".to_string(), ConfigValue::Boolean(self.enabled));
        map.insert("max_items".to_string(), ConfigValue::Integer(self.max_items as i64));
        map.insert("timeout_ms".to_string(), ConfigValue::Integer(self.timeout_ms as i64));
        map.insert("name".to_string(), ConfigValue::String(self.name.clone()));
        map
    }

    fn from_map(map: HashMap<String, ConfigValue>) -> Result<Self, terraphim_desktop_gpui::components::ConfigError> {
        Ok(Self {
            enabled: map.get("enabled")
                .and_then(|v| v.as_boolean())
                .unwrap_or(true),
            max_items: map.get("max_items")
                .and_then(|v| v.as_integer())
                .and_then(|i| u32::try_from(i).ok())
                .unwrap_or(100),
            timeout_ms: map.get("timeout_ms")
                .and_then(|v| v.as_integer())
                .and_then(|i| u64::try_from(i).ok())
                .unwrap_or(5000),
            name: map.get("name")
                .and_then(|v| v.as_string())
                .unwrap_or("test-component")
                .to_string(),
        })
    }

    fn is_equivalent(&self, other: &Self) -> bool {
        self.enabled == other.enabled &&
        self.max_items == other.max_items &&
        self.timeout_ms == other.timeout_ms
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn clone_config(&self) -> Box<dyn ComponentConfig> {
        Box::new(self.clone())
    }
}

/// Test component state
#[derive(Debug, Clone, PartialEq)]
struct TestComponentState {
    pub initialized: bool,
    pub operation_count: u64,
    pub last_operation: Option<String>,
}

impl Default for TestComponentState {
    fn default() -> Self {
        Self {
            initialized: false,
            operation_count: 0,
            last_operation: None,
        }
    }
}

/// Test component implementation
#[derive(Debug)]
struct TestComponent {
    config: TestComponentConfig,
    state: TestComponentState,
    performance_tracker: PerformanceTracker,
    is_mounted: bool,
}

impl ReusableComponent for TestComponent {
    type Config = TestComponentConfig;
    type State = TestComponentState;
    type Event = TestComponentEvent;

    fn component_id() -> &'static str {
        "test-component"
    }

    fn component_version() -> &'static str {
        "1.0.0"
    }

    fn init(config: Self::Config) -> Self {
        let performance_tracker = PerformanceTracker::default();
        Self {
            config,
            state: TestComponentState::default(),
            performance_tracker,
            is_mounted: false,
        }
    }

    fn config(&self) -> &Self::Config {
        &self.config
    }

    fn update_config(&mut self, config: Self::Config) -> Result<(), ComponentError> {
        self.config = config;
        Ok(())
    }

    fn state(&self) -> &Self::State {
        &self.state
    }

    fn update_state(&mut self, state: Self::State) -> Result<(), ComponentError> {
        self.state = state;
        Ok(())
    }

    fn mount(&mut self, _cx: &mut gpui::ViewContext<Self>) -> Result<(), ComponentError>
    where
        Self: gpui::Entity,
    {
        if self.is_mounted {
            return Err(ComponentError::AlreadyMounted);
        }

        let timer = self.performance_tracker.start_operation();

        // Simulate mounting work
        std::thread::sleep(Duration::from_millis(1));

        self.is_mounted = true;
        self.state.initialized = true;

        timer.complete_success();
        Ok(())
    }

    fn unmount(&mut self, _cx: &mut gpui::ViewContext<Self>) -> Result<(), ComponentError>
    where
        Self: gpui::Entity,
    {
        if !self.is_mounted {
            return Err(ComponentError::NotMounted);
        }

        let timer = self.performance_tracker.start_operation();

        // Simulate unmounting work
        std::thread::sleep(Duration::from_millis(1));

        self.is_mounted = false;
        self.state.initialized = false;

        timer.complete_success();
        Ok(())
    }

    fn handle_lifecycle_event(&mut self, event: LifecycleEvent, _cx: &mut gpui::ViewContext<Self>) -> Result<(), ComponentError>
    where
        Self: gpui::Entity,
    {
        let timer = self.performance_tracker.start_operation();

        self.state.operation_count += 1;
        self.state.last_operation = Some(format!("{:?}", event));

        timer.complete_success();
        Ok(())
    }

    fn is_mounted(&self) -> bool {
        self.is_mounted
    }

    fn performance_metrics(&self) -> &PerformanceTracker {
        &self.performance_tracker
    }

    fn reset_performance_metrics(&mut self) {
        self.performance_tracker.reset();
    }

    fn dependencies(&self) -> Vec<&'static str> {
        Vec::new()
    }

    fn are_dependencies_satisfied(&self, _registry: &ServiceRegistry) -> bool {
        true
    }

    fn cleanup(&mut self) -> Result<(), ComponentError> {
        let timer = self.performance_tracker.start_operation();

        // Simulate cleanup work
        std::thread::sleep(Duration::from_millis(1));

        self.state = TestComponentState::default();

        timer.complete_success();
        Ok(())
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

/// Test component events
#[derive(Debug, Clone, PartialEq)]
enum TestComponentEvent {
    OperationCompleted,
    ErrorOccurred(String),
    StateChanged,
}

#[test]
fn test_component_config_validation() {
    // Valid config
    let config = TestComponentConfig {
        enabled: true,
        max_items: 50,
        timeout_ms: 1000,
        name: "test".to_string(),
    };
    assert!(config.validate().is_ok());

    // Invalid config (max_items = 0)
    let invalid_config = TestComponentConfig {
        enabled: true,
        max_items: 0,
        timeout_ms: 1000,
        name: "test".to_string(),
    };
    assert!(invalid_config.validate().is_err());
}

#[test]
fn test_component_config_serialization() {
    let config = TestComponentConfig::default();

    // Test JSON serialization
    let json = config.to_json();
    assert!(json.is_ok());

    // Test JSON deserialization
    let deserialized: TestComponentConfig = TestComponentConfig::from_json(&json.unwrap());
    assert!(deserialized.is_ok());
    assert_eq!(config, deserialized.unwrap());
}

#[test]
fn test_component_config_map_conversion() {
    let config = TestComponentConfig::default();
    let map = config.to_map();

    assert_eq!(map.len(), 4);
    assert_eq!(map.get("enabled"), Some(&ConfigValue::Boolean(true)));
    assert_eq!(map.get("max_items"), Some(&ConfigValue::Integer(100)));

    // Test conversion back from map
    let restored = TestComponentConfig::from_map(map);
    assert!(restored.is_ok());
    assert_eq!(config, restored.unwrap());
}

#[test]
fn test_component_config_merge() {
    let config1 = TestComponentConfig {
        enabled: true,
        max_items: 100,
        timeout_ms: 5000,
        name: "original".to_string(),
    };

    let config2 = TestComponentConfig {
        enabled: false,
        max_items: 200,
        timeout_ms: 5000, // Same as default
        name: "modified".to_string(),
    };

    let merged = config1.merge(&config2).unwrap();

    assert_eq!(merged.enabled, false); // Should take from config2
    assert_eq!(merged.max_items, 200); // Should take from config2
    assert_eq!(merged.timeout_ms, 5000); // Should take from config1 (config2 has default)
    assert_eq!(merged.name, "modified"); // Should take from config2
}

#[test]
fn test_service_registry_basic_operations() {
    let registry = ServiceRegistry::default();

    // Register a service
    let service = TestService::new("test-service".to_string());
    assert!(registry.register_service(service).is_ok());

    // Check if service is registered
    assert!(registry.is_service_registered("TestService"));

    // Get service
    let service_ref = registry.get_service::<TestService>();
    assert!(service_ref.is_ok());

    // Unregister service
    assert!(registry.unregister_service::<TestService>().is_ok());
    assert!(!registry.is_service_registered("TestService"));
}

#[test]
fn test_service_registry_dependencies() {
    let registry = ServiceRegistry::default();

    // Try to get non-existent service
    let missing_service = registry.get_service::<TestService>();
    assert!(missing_service.is_err());

    // Register service successfully
    let service = TestService::new("dependent-service".to_string());
    assert!(registry.register_service(service).is_ok());
}

#[test]
fn test_component_metadata() {
    let metadata = ComponentMetadata::new(
        "test-component".to_string(),
        "1.0.0".to_string(),
        "Test Component".to_string(),
        "A test component for validation".to_string(),
        "Test Author".to_string(),
    )
    .with_capability(terraphim_desktop_gpui::components::ComponentCapability::Searchable)
    .with_dependency("service-a".to_string())
    .with_dependency("service-b".to_string())
    .with_tag("test".to_string())
    .with_tag("example".to_string());

    assert_eq!(metadata.id, "test-component");
    assert_eq!(metadata.version, "1.0.0");
    assert_eq!(metadata.name, "Test Component");
    assert!(metadata.has_capability(&terraphim_desktop_gpui::components::ComponentCapability::Searchable));
    assert!(metadata.depends_on("service-a"));
    assert!(metadata.depends_on("service-b"));
    assert!(!metadata.depends_on("service-c"));
}

#[test]
fn test_performance_tracker_basic_operations() {
    let tracker = PerformanceTracker::default();

    // Test starting and completing operations
    let timer1 = tracker.start_operation();
    std::thread::sleep(Duration::from_millis(1));
    timer1.complete_success();

    let timer2 = tracker.start_operation();
    std::thread::sleep(Duration::from_millis(1));
    timer2.complete_failure();

    // Check metrics
    let metrics = tracker.current_metrics();
    assert_eq!(metrics.operation_count, 2);
    assert_eq!(metrics.success_count, 1);
    assert_eq!(metrics.failure_count, 1);
    assert!(metrics.avg_response_time > 0.0);
    assert!(metrics.total_response_time > 0);
}

#[test]
fn test_performance_tracker_concurrent_operations() {
    let tracker = Arc::new(PerformanceTracker::default());
    let mut handles = Vec::new();

    // Start multiple concurrent operations
    for _ in 0..10 {
        let tracker_clone = Arc::clone(&tracker);
        let handle = std::thread::spawn(move || {
            let timer = tracker_clone.start_operation();
            std::thread::sleep(Duration::from_millis(1));
            timer.complete_success();
        });
        handles.push(handle);
    }

    // Wait for all operations to complete
    for handle in handles {
        handle.join().unwrap();
    }

    // Check metrics
    let metrics = tracker.current_metrics();
    assert_eq!(metrics.operation_count, 10);
    assert_eq!(metrics.success_count, 10);
    assert_eq!(metrics.failure_count, 0);
    assert!(metrics.peak_concurrent_operations >= 1);
}

#[test]
fn test_reusable_component_lifecycle() {
    let config = TestComponentConfig::default();
    let state = TestComponentState::default();
    let mut component = TestComponent::init(config);

    // Test initial state
    assert_eq!(component.config().name, "test-component");
    assert!(!component.state().initialized);
    assert!(!component.is_mounted());

    // Test state update
    let new_state = TestComponentState {
        initialized: true,
        operation_count: 5,
        last_operation: Some("test".to_string()),
    };
    assert!(component.update_state(new_state.clone()).is_ok());
    assert_eq!(component.state(), &new_state);

    // Test config update
    let new_config = TestComponentConfig {
        enabled: false,
        max_items: 200,
        timeout_ms: 10000,
        name: "modified-component".to_string(),
    };
    assert!(component.update_config(new_config).is_ok());
    assert_eq!(component.config().name, "modified-component");
}

#[test]
fn test_reusable_component_lifecycle_events() {
    let config = TestComponentConfig::default();
    let mut component = TestComponent::init(config);

    // Test lifecycle event handling
    let events = vec![
        LifecycleEvent::Mounting,
        LifecycleEvent::Mounted,
        LifecycleEvent::StateChanged,
        LifecycleEvent::ConfigChanged,
    ];

    for event in events {
        assert!(component.handle_lifecycle_event(event, &mut gpui::ViewContext::new(&mut gpui::App::new())).is_ok());
    }

    // Check that operations were tracked
    let metrics = component.performance_metrics().current_metrics();
    assert_eq!(metrics.operation_count, 4);
    assert_eq!(metrics.success_count, 4);
    assert_eq!(metrics.failure_count, 0);
}

#[test]
fn test_component_test_harness() {
    let config = TestComponentConfig::default();
    let state = TestComponentState::default();
    let mut harness = ComponentTestHarness::<TestComponent>::new(config, state);

    // Run comprehensive tests
    let result = harness.run_comprehensive_test();
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_component_performance_benchmark() {
    let result = PerformanceTestUtils::benchmark(
        || {
            let config = TestComponentConfig::default();
            let state = TestComponentState::default();
            TestComponent::init(config)
        },
        50
    );

    assert_eq!(result.results.len(), 50);
    assert!(result.avg_duration() > Duration::ZERO);

    // Should be reasonably fast
    assert!(result.avg_duration() < Duration::from_millis(10));
    assert!(result.min_duration() <= result.avg_duration());
    assert!(result.max_duration() >= result.avg_duration());
}

#[tokio::test]
async fn test_component_load_testing() {
    let result = PerformanceTestUtils::load_test(
        || {
            let config = TestComponentConfig::default();
            TestComponent::init(config)
        },
        5, // 5 concurrent operations
        Duration::from_millis(500) // 0.5 second test
    ).await;

    assert!(result.results.len() > 0);
    assert!(result.ops_per_second() > 0.0);
    assert!(result.effective_concurrency() > 0.0);
    assert_eq!(result.concurrency, 5);
}

#[test]
fn test_component_error_handling() {
    let config = TestComponentConfig::default();
    let mut component = TestComponent::init(config);

    // Test double mount
    assert!(component.mount(&mut gpui::ViewContext::new(&mut gpui::App::new())).is_ok());
    assert!(component.mount(&mut gpui::ViewContext::new(&mut gpui::App::new())).is_err());
    assert_eq!(component.unmount(&mut gpui::ViewContext::new(&mut gpui::App::new())).unwrap().to_string(), "AlreadyMounted");

    // Test unmount non-mounted component
    assert!(component.unmount(&mut gpui::ViewContext::new(&mut gpui::App::new())).is_ok());
    assert!(component.unmount(&mut gpui::ViewContext::new(&mut gpui::App::new())).is_err());
    assert_eq!(component.unmount(&mut gpui::ViewContext::new(&mut gpui::App::new())).unwrap_err().to_string(), "NotMounted");
}

#[test]
fn test_performance_tracker_reset() {
    let tracker = PerformanceTracker::default();

    // Perform some operations
    let timer = tracker.start_operation();
    timer.complete_success();

    // Verify metrics exist
    let metrics = tracker.current_metrics();
    assert_eq!(metrics.operation_count, 1);

    // Reset and verify cleared
    tracker.reset();
    let reset_metrics = tracker.current_metrics();
    assert_eq!(reset_metrics.operation_count, 0);
    assert_eq!(reset_metrics.success_count, 0);
    assert_eq!(reset_metrics.total_response_time, 0);
}

#[test]
fn test_config_value_conversions() {
    // Test string conversion
    let string_val = ConfigValue::String("test".to_string());
    assert_eq!(string_val.as_string(), Some("test"));
    assert_eq!(string_val.as_integer(), None);
    assert_eq!(string_val.as_boolean(), None);

    // Test integer conversion
    let int_val = ConfigValue::Integer(42);
    assert_eq!(int_val.as_integer(), Some(42));
    assert_eq!(int_val.as_float(), Some(42.0));
    assert_eq!(int_val.as_string(), None);

    // Test float conversion
    let float_val = ConfigValue::Float(3.14);
    assert_eq!(float_val.as_float(), Some(3.14));
    assert_eq!(float_val.as_integer(), None);

    // Test boolean conversion
    let bool_val = ConfigValue::Boolean(true);
    assert_eq!(bool_val.as_boolean(), Some(true));
    assert_eq!(bool_val.as_string(), None);

    // Test null conversion
    let null_val = ConfigValue::Null;
    assert!(null_val.is_null());
    assert_eq!(null_val.as_string(), None);
}

#[test]
fn test_component_type_safety() {
    let config = TestComponentConfig::default();
    let component = TestComponent::init(config);

    // Test type casting
    let any_component = component.as_any();
    assert!(any_component.is::<TestComponent>());

    let downcasted = any_component.downcast_ref::<TestComponent>();
    assert!(downcasted.is_some());

    // Test that we can't cast to wrong type
    let wrong_cast = any_component.downcast_ref::<String>();
    assert!(wrong_cast.is_none());
}

#[test]
fn test_component_configuration_equivalence() {
    let config1 = TestComponentConfig::default();
    let config2 = TestComponentConfig {
        enabled: true,
        max_items: 100,
        timeout_ms: 5000,
        name: "different-name".to_string(), // Different but equivalent for logic
    };

    // Should be equivalent because only name differs (not used in equivalence check)
    assert!(config1.is_equivalent(&config2));

    let config3 = TestComponentConfig {
        enabled: false, // Different
        max_items: 100,
        timeout_ms: 5000,
        name: "test-component".to_string(),
    };

    // Should not be equivalent because enabled differs
    assert!(!config1.is_equivalent(&config3));
}