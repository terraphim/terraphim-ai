/// Component configuration framework
///
/// This module provides the ComponentConfig trait and supporting types
/// for configuration-driven component customization.

use std::any::Any;
use std::collections::HashMap;
use std::fmt::Debug;

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Core trait that all component configurations must implement
pub trait ComponentConfig: Debug + Send + Sync + 'static {
    /// Get configuration schema for validation
    fn schema() -> ConfigSchema
    where
        Self: Sized;

    /// Validate configuration values
    fn validate(&self) -> Result<(), ConfigError>;

    /// Get default configuration
    fn default() -> Self
    where
        Self: Sized;

    /// Merge with another configuration (this takes precedence)
    fn merge(&self, other: &Self) -> Result<Self, ConfigError>
    where
        Self: Sized;

    /// Convert to generic value map
    fn to_map(&self) -> HashMap<String, ConfigValue>;

    /// Create from generic value map
    fn from_map(map: HashMap<String, ConfigValue>) -> Result<Self, ConfigError>
    where
        Self: Sized;

    /// Serialize to JSON
    fn to_json(&self) -> Result<String, ConfigError> {
        serde_json::to_string(self)
            .map_err(|e| ConfigError::Serialization(e.to_string()))
    }

    /// Deserialize from JSON
    fn from_json(json: &str) -> Result<Self, ConfigError>
    where
        Self: Sized,
    {
        serde_json::from_str(json)
            .map_err(|e| ConfigError::Serialization(e.to_string()))
    }

    /// Check if configuration is equal to another (ignoring irrelevant fields)
    fn is_equivalent(&self, other: &Self) -> bool;

    /// Get configuration hash for caching
    fn config_hash(&self) -> u64 {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        self.to_map().hash(&mut hasher);
        hasher.finish()
    }

    /// Convert to Any for downcasting
    fn as_any(&self) -> &dyn Any;

    // TODO: Fix ComponentConfig dyn compatibility issue
    // Clone configuration - commented out due to dyn compatibility issues
    // fn clone_config(&self) -> Box<dyn ComponentConfig>;
}

/// Configuration errors
#[derive(Debug, Clone, Error)]
pub enum ConfigError {
    #[error("Validation error: {0}")]
    Validation(String),

    #[error("Missing required field: {0}")]
    MissingField(String),

    #[error("Invalid field value: {0} = {1}")]
    InvalidValue(String, String),

    #[error("Type mismatch: expected {expected}, got {actual}")]
    TypeMismatch { expected: String, actual: String },

    #[error("Serialization error: {0}")]
    Serialization(String),

    #[error("Deserialization error: {0}")]
    Deserialization(String),

    #[error("Merge error: {0}")]
    Merge(String),

    #[error("Schema error: {0}")]
    Schema(String),
}

/// Generic configuration value that can hold different types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", content = "value")]
pub enum ConfigValue {
    String(String),
    Integer(i64),
    Float(f64),
    Boolean(bool),
    Array(Vec<ConfigValue>),
    Object(HashMap<String, ConfigValue>),
    Null,
}

impl ConfigValue {
    /// Get string value
    pub fn as_string(&self) -> Option<&str> {
        match self {
            ConfigValue::String(s) => Some(s),
            _ => None,
        }
    }

    /// Get integer value
    pub fn as_integer(&self) -> Option<i64> {
        match self {
            ConfigValue::Integer(i) => Some(*i),
            _ => None,
        }
    }

    /// Get float value
    pub fn as_float(&self) -> Option<f64> {
        match self {
            ConfigValue::Float(f) => Some(*f),
            ConfigValue::Integer(i) => Some(*i as f64),
            _ => None,
        }
    }

    /// Get boolean value
    pub fn as_boolean(&self) -> Option<bool> {
        match self {
            ConfigValue::Boolean(b) => Some(*b),
            _ => None,
        }
    }

    /// Get array value
    pub fn as_array(&self) -> Option<&Vec<ConfigValue>> {
        match self {
            ConfigValue::Array(arr) => Some(arr),
            _ => None,
        }
    }

    /// Get object value
    pub fn as_object(&self) -> Option<&HashMap<String, ConfigValue>> {
        match self {
            ConfigValue::Object(obj) => Some(obj),
            _ => None,
        }
    }

    /// Check if value is null
    pub fn is_null(&self) -> bool {
        matches!(self, ConfigValue::Null)
    }
}

impl From<String> for ConfigValue {
    fn from(value: String) -> Self {
        ConfigValue::String(value)
    }
}

impl From<&str> for ConfigValue {
    fn from(value: &str) -> Self {
        ConfigValue::String(value.to_string())
    }
}

impl From<i64> for ConfigValue {
    fn from(value: i64) -> Self {
        ConfigValue::Integer(value)
    }
}

impl From<f64> for ConfigValue {
    fn from(value: f64) -> Self {
        ConfigValue::Float(value)
    }
}

impl From<bool> for ConfigValue {
    fn from(value: bool) -> Self {
        ConfigValue::Boolean(value)
    }
}

/// Configuration field definition for validation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigField {
    /// Field name
    pub name: String,

    /// Field type
    pub field_type: ConfigFieldType,

    /// Whether field is required
    pub required: bool,

    /// Default value (if any)
    pub default: Option<ConfigValue>,

    /// Field description
    pub description: String,

    /// Validation rules
    pub validation: Vec<ValidationRule>,

    /// Field documentation
    pub docs: Option<String>,
}

/// Configuration field types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ConfigFieldType {
    String,
    Integer,
    Float,
    Boolean,
    Array(Box<ConfigFieldType>),
    Object(HashMap<String, ConfigFieldType>),
    Enum(Vec<String>),
    Color,
    Url,
    FilePath,
    Duration,
    Size,
    Timestamp,
}

/// Validation rules for configuration fields
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ValidationRule {
    /// Minimum value for numeric fields
    MinValue(f64),
    /// Maximum value for numeric fields
    MaxValue(f64),
    /// Minimum length for strings/arrays
    MinLength(usize),
    /// Maximum length for strings/arrays
    MaxLength(usize),
    /// Regular expression pattern for strings
    Pattern(String),
    /// Set of allowed values
    Enum(Vec<ConfigValue>),
    /// Custom validation function name
    Custom(String),
    /// Field must not be null
    NotNull,
    /// Field must be a valid URL
    Url,
    /// Field must be a valid email
    Email,
    /// Field must be a valid file path
    FilePath,
}

/// Configuration schema for validation and documentation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigSchema {
    /// Schema name
    pub name: String,

    /// Schema version
    pub version: String,

    /// Schema description
    pub description: String,

    /// Configuration fields
    pub fields: HashMap<String, ConfigField>,

    /// Additional metadata
    pub metadata: HashMap<String, ConfigValue>,
}

impl ConfigSchema {
    /// Create new configuration schema
    pub fn new(name: String, version: String, description: String) -> Self {
        Self {
            name,
            version,
            description,
            fields: HashMap::new(),
            metadata: HashMap::new(),
        }
    }

    /// Add field to schema
    pub fn with_field(mut self, field: ConfigField) -> Self {
        self.fields.insert(field.name.clone(), field);
        self
    }

    /// Add metadata
    pub fn with_metadata(mut self, key: String, value: ConfigValue) -> Self {
        self.metadata.insert(key, value);
        self
    }

    /// Validate configuration values against schema
    pub fn validate(&self, values: &HashMap<String, ConfigValue>) -> Result<(), ConfigError> {
        // Check required fields
        for (field_name, field) in &self.fields {
            if field.required && !values.contains_key(field_name) {
                return Err(ConfigError::MissingField(field_name.clone()));
            }
        }

        // Validate each field value
        for (field_name, value) in values {
            if let Some(field) = self.fields.get(field_name) {
                self.validate_field_value(field, value)?;
            }
        }

        Ok(())
    }

    /// Validate individual field value
    fn validate_field_value(&self, field: &ConfigField, value: &ConfigValue) -> Result<(), ConfigError> {
        // Type validation
        if !self.is_type_compatible(&field.field_type, value) {
            return Err(ConfigError::TypeMismatch {
                expected: format!("{:?}", field.field_type),
                actual: format!("{:?}", value),
            });
        }

        // Validation rules
        for rule in &field.validation {
            self.apply_validation_rule(rule, value)?;
        }

        Ok(())
    }

    /// Check if value type is compatible with field type
    fn is_type_compatible(&self, field_type: &ConfigFieldType, value: &ConfigValue) -> bool {
        match (field_type, value) {
            (ConfigFieldType::String, ConfigValue::String(_)) => true,
            (ConfigFieldType::Integer, ConfigValue::Integer(_)) => true,
            (ConfigFieldType::Float, ConfigValue::Float(_)) => true,
            (ConfigFieldType::Float, ConfigValue::Integer(_)) => true, // Allow integer as float
            (ConfigFieldType::Boolean, ConfigValue::Boolean(_)) => true,
            (ConfigFieldType::Array(_), ConfigValue::Array(_)) => true,
            (ConfigFieldType::Object(_), ConfigValue::Object(_)) => true,
            (ConfigFieldType::Enum(_), ConfigValue::String(_)) => true,
            (ConfigFieldType::Color, ConfigValue::String(_)) => true,
            (ConfigFieldType::Url, ConfigValue::String(_)) => true,
            (ConfigFieldType::FilePath, ConfigValue::String(_)) => true,
            _ => false,
        }
    }

    /// Apply validation rule to value
    fn apply_validation_rule(&self, rule: &ValidationRule, value: &ConfigValue) -> Result<(), ConfigError> {
        match rule {
            ValidationRule::MinValue(min) => {
                if let Some(num) = value.as_float() {
                    if num < *min {
                        return Err(ConfigError::Validation(
                            format!("Value {} is below minimum {}", num, min)
                        ));
                    }
                }
            }
            ValidationRule::MaxValue(max) => {
                if let Some(num) = value.as_float() {
                    if num > *max {
                        return Err(ConfigError::Validation(
                            format!("Value {} is above maximum {}", num, max)
                        ));
                    }
                }
            }
            ValidationRule::MinLength(min) => {
                let length = match value {
                    ConfigValue::String(s) => s.len(),
                    ConfigValue::Array(arr) => arr.len(),
                    ConfigValue::Object(obj) => obj.len(),
                    _ => return Ok(()), // Length doesn't apply to other types
                };

                if length < *min {
                    return Err(ConfigError::Validation(
                        format!("Length {} is below minimum {}", length, min)
                    ));
                }
            }
            ValidationRule::MaxLength(max) => {
                let length = match value {
                    ConfigValue::String(s) => s.len(),
                    ConfigValue::Array(arr) => arr.len(),
                    ConfigValue::Object(obj) => obj.len(),
                    _ => return Ok(()), // Length doesn't apply to other types
                };

                if length > *max {
                    return Err(ConfigError::Validation(
                        format!("Length {} is above maximum {}", length, max)
                    ));
                }
            }
            ValidationRule::Pattern(pattern) => {
                if let Some(s) = value.as_string() {
                    let regex = regex::Regex::new(pattern)
                        .map_err(|e| ConfigError::Validation(format!("Invalid regex: {}", e)))?;
                    if !regex.is_match(s) {
                        return Err(ConfigError::Validation(
                            format!("Value '{}' doesn't match pattern '{}'", s, pattern)
                        ));
                    }
                }
            }
            ValidationRule::Enum(allowed) => {
                if !allowed.contains(value) {
                    return Err(ConfigError::Validation(
                        format!("Value {:?} not in allowed enum values: {:?}", value, allowed)
                    ));
                }
            }
            ValidationRule::NotNull => {
                if value.is_null() {
                    return Err(ConfigError::Validation("Value cannot be null".to_string()));
                }
            }
            ValidationRule::Url => {
                if let Some(s) = value.as_string() {
                    if url::Url::parse(s).is_err() {
                        return Err(ConfigError::Validation(format!("'{}' is not a valid URL", s)));
                    }
                }
            }
            ValidationRule::Email => {
                if let Some(s) = value.as_string() {
                    // Simple email validation
                    if !s.contains('@') || !s.contains('.') {
                        return Err(ConfigError::Validation(format!("'{}' is not a valid email", s)));
                    }
                }
            }
            ValidationRule::FilePath => {
                if let Some(s) = value.as_string() {
                    if std::path::Path::new(s).components().next().is_none() {
                        return Err(ConfigError::Validation(format!("'{}' is not a valid file path", s)));
                    }
                }
            }
            ValidationRule::Custom(_) => {
                // Custom validation would be implemented by component-specific code
                // This is a placeholder for future extensibility
            }
        }

        Ok(())
    }
}

/// Default configuration trait for simple configurations
pub trait DefaultConfig: ComponentConfig {
    /// Get default values as a map
    fn default_values() -> HashMap<String, ConfigValue> {
        Self::default().to_map()
    }
}

/// Mergeable configuration trait for combining configurations
pub trait MergeableConfig: ComponentConfig + Sized {
    /// Merge with another configuration, preferring self values
    fn merge_preferences(&self, other: &Self) -> Self {
        self.merge(other).unwrap_or_else(|_| self.clone())
    }
}

/// Macro to easily implement ComponentConfig for structs
#[macro_export]
macro_rules! impl_component_config {
    (
        $(#[$meta:meta])*
        $vis:vis struct $name:ident {
            $(
                $field_vis:vis $field_name:ident : $field_type:ty $(= $default:expr)?
            ),* $(,)?
        }

        schema: $schema_name:expr,
        version: $schema_version:expr,
        description: $schema_description:expr,
    ) => {
        $(#[$meta])*
        $vis struct $name {
            $(
                $field_vis $field_name : $field_type,
            )*
        }

        impl Default for $name {
            fn default() -> Self {
                Self {
                    $(
                        $field_name: $crate::impl_component_config!(@default $field_name $field_type $($default)?),
                    )*
                }
            }
        }

        impl $crate::components::config::ComponentConfig for $name {
            fn schema() -> $crate::components::config::ConfigSchema {
                let mut schema = $crate::components::config::ConfigSchema::new(
                    $schema_name.to_string(),
                    $schema_version.to_string(),
                    $schema_description.to_string(),
                );

                $(
                    $crate::impl_component_config!(@add_field schema $field_name $field_type);
                )*

                schema
            }

            fn validate(&self) -> Result<(), $crate::components::config::ConfigError> {
                let values = self.to_map();
                Self::schema().validate(&values)
            }

            fn default() -> Self {
                Self::default()
            }

            fn merge(&self, other: &Self) -> Result<Self, $crate::components::config::ConfigError> {
                Ok(Self {
                    $(
                        $field_name: if self.$field_name != Self::default().$field_name {
                            self.$field_name.clone()
                        } else {
                            other.$field_name.clone()
                        },
                    )*
                })
            }

            fn to_map(&self) -> std::collections::HashMap<String, $crate::components::config::ConfigValue> {
                let mut map = std::collections::HashMap::new();
                $(
                    map.insert(stringify!($field_name).to_string(),
                              $crate::impl_component_config!(@to_value self.$field_name));
                )*
                map
            }

            fn from_map(map: std::collections::HashMap<String, $crate::components::config::ConfigValue>) -> Result<Self, $crate::components::config::ConfigError> {
                Ok(Self {
                    $(
                        $field_name: $crate::impl_component_config!(@from_value map stringify!($field_name) $field_type)?,
                    )*
                })
            }

            fn is_equivalent(&self, other: &Self) -> bool {
                self.to_map() == other.to_map()
            }

            fn as_any(&self) -> &dyn std::any::Any {
                self
            }

            fn clone_config(&self) -> Box<dyn $crate::components::config::ComponentConfig> {
                Box::new(self.clone())
            }
        }

        impl Clone for $name {
            fn clone(&self) -> Self {
                Self {
                    $(
                        $field_name: self.$field_name.clone(),
                    )*
                }
            }
        }
    };

    // Helper macro for default values
    (@default $field_name:ident $field_type:ty) => {
        <$field_type>::default()
    };
    (@default $field_name:ident $field_type:tt $default:expr) => {
        $default
    };

    // Helper macro for adding fields to schema
    (@add_field $schema:ident $field_name:ident String) => {
        $schema = $schema.with_field($crate::components::config::ConfigField {
            name: stringify!($field_name).to_string(),
            field_type: $crate::components::config::ConfigFieldType::String,
            required: false,
            default: None,
            description: stringify!($field_name).to_string(),
            validation: vec![],
            docs: None,
        });
    };
    (@add_field $schema:ident $field_name:ident bool) => {
        $schema = $schema.with_field($crate::components::config::ConfigField {
            name: stringify!($field_name).to_string(),
            field_type: $crate::components::config::ConfigFieldType::Boolean,
            required: false,
            default: Some($crate::components::config::ConfigValue::Boolean(false)),
            description: stringify!($field_name).to_string(),
            validation: vec![],
            docs: None,
        });
    };
    (@add_field $schema:ident $field_name:ident u32) => {
        $schema = $schema.with_field($crate::components::config::ConfigField {
            name: stringify!($field_name).to_string(),
            field_type: $crate::components::config::ConfigFieldType::Integer,
            required: false,
            default: Some($crate::components::config::ConfigValue::Integer(0)),
            description: stringify!($field_name).to_string(),
            validation: vec![],
            docs: None,
        });
    };
    (@add_field $schema:ident $field_name:ident f64) => {
        $schema = $schema.with_field($crate::components::config::ConfigField {
            name: stringify!($field_name).to_string(),
            field_type: $crate::components::config::ConfigFieldType::Float,
            required: false,
            default: Some($crate::components::config::ConfigValue::Float(0.0)),
            description: stringify!($field_name).to_string(),
            validation: vec![],
            docs: None,
        });
    };

    // Helper macro for converting values to ConfigValue
    (@to_value $expr:expr) => {
        $crate::components::config::ConfigValue::from($expr.clone())
    };

    // Helper macro for converting from ConfigValue
    (@from_value $map:ident $field_name:ident String) => {
        $map.remove(stringify!($field_name))
            .and_then(|v| v.as_string().map(|s| s.to_string()))
            .unwrap_or_default()
    };
    (@from_value $map:ident $field_name:ident bool) => {
        $map.remove(stringify!($field_name))
            .and_then(|v| v.as_boolean())
            .unwrap_or(false)
    };
    (@from_value $map:ident $field_name:ident u32) => {
        $map.remove(stringify!($field_name))
            .and_then(|v| v.as_integer())
            .and_then(|i| u32::try_from(i).ok())
            .unwrap_or_default()
    };
    (@from_value $map:ident $field_name:ident f64) => {
        $map.remove(stringify!($field_name))
            .and_then(|v| v.as_float())
            .unwrap_or(0.0)
    };
}