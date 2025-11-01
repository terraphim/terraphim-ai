use crate::{McpNamespace, McpServerConfig};

#[cfg(not(feature = "json-schema"))]
use ahash::HashMap as HashMap;

#[cfg(feature = "json-schema")]
use std::collections::HashMap;

#[derive(Debug, Clone, Default)]
pub struct NamespaceManager {
    /// Map of namespace name to namespace configuration
    namespaces: HashMap<String, McpNamespace>,
}

impl NamespaceManager {
    /// Create a new namespace manager
    pub fn new() -> Self {
        Self {
            namespaces: HashMap::default(),
        }
    }

    /// Add a namespace
    pub fn add_namespace(&mut self, namespace: McpNamespace) {
        self.namespaces.insert(namespace.name.clone(), namespace);
    }

    /// Get a namespace by name
    pub fn get_namespace(&self, name: &str) -> Option<&McpNamespace> {
        self.namespaces.get(name)
    }

    /// Get a mutable namespace by name
    pub fn get_namespace_mut(&mut self, name: &str) -> Option<&mut McpNamespace> {
        self.namespaces.get_mut(name)
    }

    /// Remove a namespace
    pub fn remove_namespace(&mut self, name: &str) -> Option<McpNamespace> {
        self.namespaces.remove(name)
    }

    /// List all namespace names
    pub fn list_namespaces(&self) -> Vec<String> {
        self.namespaces.keys().cloned().collect()
    }

    /// Get all enabled namespaces
    pub fn get_enabled_namespaces(&self) -> Vec<&McpNamespace> {
        self.namespaces.values().filter(|ns| ns.enabled).collect()
    }

    /// Get all servers across all enabled namespaces
    pub fn get_all_servers(&self) -> Vec<(String, &McpServerConfig)> {
        let mut servers = Vec::new();
        for namespace in self.get_enabled_namespaces() {
            for server in &namespace.servers {
                servers.push((namespace.name.clone(), server));
            }
        }
        servers
    }

    /// Clear all namespaces
    pub fn clear(&mut self) {
        self.namespaces.clear();
    }

    /// Get total number of namespaces
    pub fn count(&self) -> usize {
        self.namespaces.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::McpServerConfig;

    #[test]
    fn test_namespace_manager() {
        let mut manager = NamespaceManager::new();

        let ns1 = McpNamespace {
            name: "dev-tools".to_string(),
            servers: vec![McpServerConfig::stdio(
                "filesystem",
                "npx",
                vec!["-y".to_string()],
            )],
            tool_overrides: HashMap::default(),
            enabled: true,
        };

        let ns2 = McpNamespace {
            name: "data-tools".to_string(),
            servers: vec![],
            tool_overrides: HashMap::default(),
            enabled: false,
        };

        manager.add_namespace(ns1);
        manager.add_namespace(ns2);

        assert_eq!(manager.count(), 2);
        assert_eq!(manager.list_namespaces().len(), 2);

        let enabled = manager.get_enabled_namespaces();
        assert_eq!(enabled.len(), 1);
        assert_eq!(enabled[0].name, "dev-tools");

        assert!(manager.get_namespace("dev-tools").is_some());
        assert!(manager.remove_namespace("dev-tools").is_some());
        assert_eq!(manager.count(), 1);
    }

    #[test]
    fn test_get_all_servers() {
        let mut manager = NamespaceManager::new();

        let ns1 = McpNamespace {
            name: "ns1".to_string(),
            servers: vec![
                McpServerConfig::stdio("server1", "cmd1", vec![]),
                McpServerConfig::stdio("server2", "cmd2", vec![]),
            ],
            tool_overrides: HashMap::default(),
            enabled: true,
        };

        let ns2 = McpNamespace {
            name: "ns2".to_string(),
            servers: vec![McpServerConfig::stdio("server3", "cmd3", vec![])],
            tool_overrides: HashMap::default(),
            enabled: true,
        };

        manager.add_namespace(ns1);
        manager.add_namespace(ns2);

        let servers = manager.get_all_servers();
        assert_eq!(servers.len(), 3);
    }
}
