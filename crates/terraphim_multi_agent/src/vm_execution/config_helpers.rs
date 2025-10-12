use serde_json::Value;
use terraphim_config::Role;

use super::models::VmExecutionConfig;

/// Extract VM execution configuration from role extra parameters
pub fn extract_vm_config_from_role(role: &Role) -> Option<VmExecutionConfig> {
    // First try direct vm_execution key, then try nested extra field (handles serialization quirk)
    let vm_value = role.extra.get("vm_execution").or_else(|| {
        role.extra
            .get("extra")
            .and_then(|nested| nested.get("vm_execution"))
    });

    vm_value.and_then(|vm_value| {
        match vm_value {
            Value::Object(vm_obj) => {
                // Extract VM execution configuration from role extra
                let enabled = vm_obj
                    .get("enabled")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);

                if !enabled {
                    return None;
                }

                let api_base_url = vm_obj
                    .get("api_base_url")
                    .and_then(|v| v.as_str())
                    .unwrap_or("http://localhost:8080")
                    .to_string();

                let vm_pool_size = vm_obj
                    .get("vm_pool_size")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(1) as u32;

                let default_vm_type = vm_obj
                    .get("default_vm_type")
                    .and_then(|v| v.as_str())
                    .unwrap_or("focal-optimized")
                    .to_string();

                let execution_timeout_ms = vm_obj
                    .get("execution_timeout_ms")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(30000);

                let allowed_languages = vm_obj
                    .get("allowed_languages")
                    .and_then(|v| v.as_array())
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|lang| lang.as_str())
                            .map(|s| s.to_string())
                            .collect()
                    })
                    .unwrap_or_else(|| {
                        vec![
                            "python".to_string(),
                            "javascript".to_string(),
                            "bash".to_string(),
                            "rust".to_string(),
                        ]
                    });

                let auto_provision = vm_obj
                    .get("auto_provision")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(true);

                let code_validation = vm_obj
                    .get("code_validation")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(true);

                let max_code_length = vm_obj
                    .get("max_code_length")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(10000) as usize;

                let history = vm_obj
                    .get("history")
                    .and_then(|v| serde_json::from_value(v.clone()).ok())
                    .unwrap_or_default();

                Some(VmExecutionConfig {
                    enabled: true,
                    api_base_url,
                    vm_pool_size,
                    default_vm_type,
                    execution_timeout_ms,
                    allowed_languages,
                    auto_provision,
                    code_validation,
                    max_code_length,
                    history,
                })
            }
            Value::Bool(true) => {
                // Simple boolean true = enable with defaults
                Some(VmExecutionConfig::default())
            }
            _ => None,
        }
    })
}

/// Create an agent configuration with VM execution enabled
pub fn create_agent_config_with_vm_execution(
    role: &Role,
    base_config: Option<crate::agent::AgentConfig>,
) -> crate::agent::AgentConfig {
    let mut config = base_config.unwrap_or_default();

    // Extract VM config from role and set it
    config.vm_execution = extract_vm_config_from_role(role);

    config
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_extract_vm_config_basic() {
        let mut role = Role::default();
        role.extra.insert(
            "vm_execution".to_string(),
            json!({
                "enabled": true,
                "api_base_url": "http://test:8080",
                "default_vm_type": "test-vm"
            }),
        );

        let vm_config = extract_vm_config_from_role(&role).unwrap();
        assert!(vm_config.enabled);
        assert_eq!(vm_config.api_base_url, "http://test:8080");
        assert_eq!(vm_config.default_vm_type, "test-vm");
    }

    #[test]
    fn test_extract_vm_config_boolean_true() {
        let mut role = Role::default();
        role.extra.insert("vm_execution".to_string(), json!(true));

        let vm_config = extract_vm_config_from_role(&role).unwrap();
        assert!(vm_config.enabled);
        assert_eq!(vm_config.api_base_url, "http://localhost:8080");
    }

    #[test]
    fn test_extract_vm_config_disabled() {
        let mut role = Role::default();
        role.extra.insert(
            "vm_execution".to_string(),
            json!({
                "enabled": false
            }),
        );

        let vm_config = extract_vm_config_from_role(&role);
        assert!(vm_config.is_none());
    }

    #[test]
    fn test_extract_vm_config_missing() {
        let role = Role::default();
        let vm_config = extract_vm_config_from_role(&role);
        assert!(vm_config.is_none());
    }

    #[test]
    fn test_create_agent_config_with_vm() {
        let mut role = Role::default();
        role.extra.insert(
            "vm_execution".to_string(),
            json!({
                "enabled": true,
                "allowed_languages": ["python", "rust"]
            }),
        );

        let config = create_agent_config_with_vm_execution(&role, None);

        assert!(config.vm_execution.is_some());
        let vm_config = config.vm_execution.unwrap();
        assert_eq!(vm_config.allowed_languages, vec!["python", "rust"]);
    }
}
