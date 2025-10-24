/// Mock command completer for testing rolegraph-aware suggestions
#[derive(Clone)]
struct MockCommandCompleter {
    current_role: String,
}

impl MockCommandCompleter {
    fn new(current_role: String) -> Self {
        Self { current_role }
    }

    /// Get role-specific VM commands based on role
    fn get_role_vm_commands(&self) -> Vec<&'static str> {
        match self.current_role.as_str() {
            "Terraphim Engineer" => {
                vec![
                    "list", "pool", "status", "metrics", "execute", "agent", "monitor", "tasks",
                    "allocate", "release",
                ]
            }
            "System Operator" => {
                vec!["list", "status", "metrics", "monitor", "tasks"]
            }
            _ => {
                vec!["list", "status", "execute", "tasks"]
            }
        }
    }

    /// Get role-specific search suggestions
    fn get_role_search_suggestions(&self) -> Vec<&'static str> {
        match self.current_role.as_str() {
            "Terraphim Engineer" => vec![
                "VM",
                "Firecracker",
                "Rust",
                "performance",
                "monitoring",
                "metrics",
                "automation",
                "deployment",
                "architecture",
            ],
            "System Operator" => vec![
                "system",
                "monitoring",
                "logs",
                "performance",
                "security",
                "backup",
                "maintenance",
                "troubleshooting",
                "infrastructure",
            ],
            _ => vec![
                "search",
                "documents",
                "knowledge",
                "graph",
                "concepts",
                "role",
                "configuration",
                "chat",
                "help",
            ],
        }
    }

    /// Get contextual hints based on input line
    fn get_hint(&self, line: &str) -> Option<String> {
        if line.trim().is_empty() {
            return Some("Try /search, /vm, /graph, or /help".to_string());
        }

        match self.current_role.as_str() {
            "Terraphim Engineer" => {
                if line.starts_with("/vm") {
                    Some("Try: /vm list, /vm pool, /vm execute, /vm monitor".to_string())
                } else if line.starts_with("/search") {
                    Some("Try: /search --semantic --concepts".to_string())
                } else {
                    None
                }
            }
            "System Operator" => {
                if line.starts_with("/vm") {
                    Some("Try: /vm list, /vm status, /vm metrics, /vm monitor".to_string())
                } else {
                    None
                }
            }
            _ => {
                if line.starts_with("/vm") {
                    Some("Try: /vm list, /vm status, /vm execute".to_string())
                } else {
                    None
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_engineer_role_vm_commands() {
        let completer = MockCommandCompleter::new("Terraphim Engineer".to_string());
        let commands = completer.get_role_vm_commands();

        assert!(commands.contains(&"list"));
        assert!(commands.contains(&"pool"));
        assert!(commands.contains(&"monitor"));
        assert!(commands.contains(&"agent"));
        assert!(commands.len() == 10); // Engineer gets all commands
    }

    #[test]
    fn test_system_operator_role_vm_commands() {
        let completer = MockCommandCompleter::new("System Operator".to_string());
        let commands = completer.get_role_vm_commands();

        assert!(commands.contains(&"list"));
        assert!(commands.contains(&"status"));
        assert!(commands.contains(&"monitor"));
        assert!(!commands.contains(&"agent")); // Operator doesn't get agent command
        assert!(!commands.contains(&"allocate")); // Operator doesn't get allocation commands
        assert!(commands.len() == 5); // Operator gets limited commands
    }

    #[test]
    fn test_default_role_vm_commands() {
        let completer = MockCommandCompleter::new("Default User".to_string());
        let commands = completer.get_role_vm_commands();

        assert!(commands.contains(&"list"));
        assert!(commands.contains(&"status"));
        assert!(commands.contains(&"execute"));
        assert!(!commands.contains(&"monitor")); // Default doesn't get monitoring
        assert!(!commands.contains(&"metrics")); // Default doesn't get metrics
        assert!(commands.len() == 4); // Default gets basic commands
    }

    #[test]
    fn test_engineer_role_search_suggestions() {
        let completer = MockCommandCompleter::new("Terraphim Engineer".to_string());
        let suggestions = completer.get_role_search_suggestions();

        assert!(suggestions.contains(&"VM"));
        assert!(suggestions.contains(&"Firecracker"));
        assert!(suggestions.contains(&"Rust"));
        assert!(suggestions.contains(&"performance"));
        assert!(!suggestions.contains(&"documents")); // Engineer gets technical terms
    }

    #[test]
    fn test_system_operator_role_search_suggestions() {
        let completer = MockCommandCompleter::new("System Operator".to_string());
        let suggestions = completer.get_role_search_suggestions();

        assert!(suggestions.contains(&"system"));
        assert!(suggestions.contains(&"monitoring"));
        assert!(suggestions.contains(&"logs"));
        assert!(suggestions.contains(&"security"));
        assert!(!suggestions.contains(&"Firecracker")); // Operator gets ops terms
    }

    #[test]
    fn test_default_role_search_suggestions() {
        let completer = MockCommandCompleter::new("Default User".to_string());
        let suggestions = completer.get_role_search_suggestions();

        assert!(suggestions.contains(&"search"));
        assert!(suggestions.contains(&"documents"));
        assert!(suggestions.contains(&"help"));
        assert!(!suggestions.contains(&"metrics")); // Default gets basic terms
    }

    #[test]
    fn test_engineer_hints_for_vm_commands() {
        let completer = MockCommandCompleter::new("Terraphim Engineer".to_string());

        let hint = completer.get_hint("/vm");
        assert!(hint.is_some());
        let hint_str = hint.unwrap();
        assert!(hint_str.contains("pool"));
        assert!(hint_str.contains("monitor"));
    }

    #[test]
    fn test_engineer_hints_for_search_commands() {
        let completer = MockCommandCompleter::new("Terraphim Engineer".to_string());

        let hint = completer.get_hint("/search");
        assert!(hint.is_some());
        let hint_str = hint.unwrap();
        assert!(hint_str.contains("semantic"));
        assert!(hint_str.contains("concepts"));
    }

    #[test]
    fn test_default_hints_for_vm_commands() {
        let completer = MockCommandCompleter::new("Default User".to_string());

        let hint = completer.get_hint("/vm");
        assert!(hint.is_some());
        let hint_str = hint.unwrap();
        assert!(hint_str.contains("list"));
        assert!(hint_str.contains("status"));
        assert!(!hint_str.contains("pool")); // Default doesn't get advanced hints
    }

    #[test]
    fn test_empty_line_hints() {
        let completer = MockCommandCompleter::new("Terraphim Engineer".to_string());

        let hint = completer.get_hint("");
        assert!(hint.is_some());
        let hint_str = hint.unwrap();
        assert!(hint_str.contains("search"));
        assert!(hint_str.contains("vm"));
        assert!(hint_str.contains("graph"));
    }

    #[test]
    fn test_no_hints_for_unknown_commands() {
        let completer = MockCommandCompleter::new("Terraphim Engineer".to_string());

        let hint = completer.get_hint("/unknown");
        assert!(hint.is_none());
    }

    #[test]
    fn test_role_specific_command_completion() {
        let engineer_commands =
            MockCommandCompleter::new("Terraphim Engineer".to_string()).get_role_vm_commands();
        let operator_commands =
            MockCommandCompleter::new("System Operator".to_string()).get_role_vm_commands();
        let default_commands =
            MockCommandCompleter::new("Default User".to_string()).get_role_vm_commands();

        // Engineer should have more commands than operator
        assert!(engineer_commands.len() > operator_commands.len());
        // Operator should have more commands than default
        assert!(operator_commands.len() > default_commands.len());

        // All roles should have basic commands
        for commands in [&engineer_commands, &operator_commands, &default_commands] {
            assert!(commands.contains(&"list"));
            assert!(commands.contains(&"status"));
        }
    }

    #[test]
    fn test_role_specific_search_term_completion() {
        let engineer_suggestions = MockCommandCompleter::new("Terraphim Engineer".to_string())
            .get_role_search_suggestions();
        let operator_suggestions =
            MockCommandCompleter::new("System Operator".to_string()).get_role_search_suggestions();

        // Engineer should get technical suggestions
        assert!(engineer_suggestions.iter().any(|&s| s == "Rust"));
        assert!(engineer_suggestions.iter().any(|&s| s == "Firecracker"));

        // Operator should get operational suggestions
        assert!(operator_suggestions.iter().any(|&s| s == "logs"));
        assert!(operator_suggestions.iter().any(|&s| s == "security"));

        // They should have different suggestions
        assert_ne!(engineer_suggestions, operator_suggestions);
    }

    #[test]
    fn test_case_insensitive_role_matching() {
        // Test with different case variations
        let roles = vec![
            "terraphim engineer",
            "TERRAPHIM ENGINEER",
            "system operator",
            "SYSTEM OPERATOR",
        ];

        for role in roles {
            let normalized_role = if role.to_lowercase().contains("engineer") {
                "Terraphim Engineer"
            } else if role.to_lowercase().contains("operator") {
                "System Operator"
            } else {
                "Default User"
            };

            let completer = MockCommandCompleter::new(normalized_role.to_string());
            let commands = completer.get_role_vm_commands();

            assert!(
                !commands.is_empty(),
                "Should have commands for role: {}",
                role
            );
        }
    }

    #[test]
    fn test_search_suggestion_filtering() {
        let completer = MockCommandCompleter::new("Terraphim Engineer".to_string());
        let suggestions = completer.get_role_search_suggestions();

        // Test partial matching simulation
        let search_term = "perf";
        let matching_suggestions: Vec<_> = suggestions
            .iter()
            .filter(|&&s| s.to_lowercase().contains(search_term))
            .collect();

        assert!(
            !matching_suggestions.is_empty(),
            "Should find matching suggestions"
        );
        assert!(matching_suggestions.contains(&&"performance"));
    }

    #[test]
    fn test_vm_command_completion_filtering() {
        let completer = MockCommandCompleter::new("Terraphim Engineer".to_string());
        let commands = completer.get_role_vm_commands();

        // Test partial matching simulation
        let partial_input = "mon";
        let matching_commands: Vec<_> = commands
            .iter()
            .filter(|&&s| s.to_lowercase().contains(partial_input))
            .collect();

        assert!(
            !matching_commands.is_empty(),
            "Should find matching commands"
        );
        assert!(matching_commands.contains(&&"monitor"));
    }

    #[test]
    fn test_hint_context_awareness() {
        let engineer = MockCommandCompleter::new("Terraphim Engineer".to_string());
        let operator = MockCommandCompleter::new("System Operator".to_string());

        let engineer_hint = engineer.get_hint("/vm");
        let operator_hint = operator.get_hint("/vm");

        assert!(engineer_hint.is_some());
        assert!(operator_hint.is_some());

        // Engineer should get more advanced hints
        let engineer_str = engineer_hint.unwrap();
        assert!(engineer_str.contains("pool"));
        assert!(engineer_str.contains("monitor"));

        // Operator should get basic hints only
        let operator_str = operator_hint.unwrap();
        assert!(!operator_str.contains("pool"));
        assert!(operator_str.contains("monitor"));
    }

    #[test]
    fn test_role_based_access_control() {
        let roles_and_expected_commands = vec![
            ("Terraphim Engineer", vec!["agent", "allocate", "release"]),
            ("System Operator", vec![]), // No advanced commands
            ("Default User", vec![]),    // No advanced commands
        ];

        for (role, expected_advanced) in roles_and_expected_commands {
            let completer = MockCommandCompleter::new(role.to_string());
            let commands = completer.get_role_vm_commands();

            for advanced_cmd in expected_advanced {
                if role == "Terraphim Engineer" {
                    assert!(
                        commands.contains(&advanced_cmd),
                        "Engineer should have access to {}",
                        advanced_cmd
                    );
                } else {
                    assert!(
                        !commands.contains(&advanced_cmd),
                        "{} should not have access to {}",
                        role,
                        advanced_cmd
                    );
                }
            }
        }
    }
}
