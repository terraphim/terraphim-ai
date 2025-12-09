//! Self-documentation API for robot mode
//!
//! Provides introspection capabilities for AI agents to discover
//! available commands, their arguments, and expected responses.

use serde::{Deserialize, Serialize};

use super::schema::{CapabilitiesData, FeatureFlags};

/// Self-documentation provider
#[derive(Debug)]
pub struct SelfDocumentation {
    commands: Vec<CommandDoc>,
}

impl SelfDocumentation {
    /// Create documentation with all available commands
    pub fn new() -> Self {
        Self {
            commands: Self::build_command_docs(),
        }
    }

    /// Get capabilities summary
    pub fn capabilities(&self) -> Capabilities {
        Capabilities {
            name: "terraphim-agent".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            description: "Privacy-first AI assistant with knowledge graph search".to_string(),
            features: FeatureFlags::default(),
            commands: self.commands.iter().map(|c| c.name.clone()).collect(),
            supported_formats: vec![
                "json".to_string(),
                "jsonl".to_string(),
                "minimal".to_string(),
                "table".to_string(),
            ],
        }
    }

    /// Get capabilities as data structure for JSON response
    pub fn capabilities_data(&self) -> CapabilitiesData {
        CapabilitiesData {
            name: "terraphim-agent".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            description: "Privacy-first AI assistant with knowledge graph search".to_string(),
            features: FeatureFlags::default(),
            commands: self.commands.iter().map(|c| c.name.clone()).collect(),
            supported_formats: vec![
                "json".to_string(),
                "jsonl".to_string(),
                "minimal".to_string(),
                "table".to_string(),
            ],
            index_status: None,
        }
    }

    /// Get schema for a specific command
    pub fn schema(&self, command: &str) -> Option<&CommandDoc> {
        self.commands.iter().find(|c| c.name == command)
    }

    /// Get all command schemas
    pub fn all_schemas(&self) -> &[CommandDoc] {
        &self.commands
    }

    /// Get examples for a specific command
    pub fn examples(&self, command: &str) -> Option<&[ExampleDoc]> {
        self.schema(command).map(|c| c.examples.as_slice())
    }

    /// Build documentation for all commands
    fn build_command_docs() -> Vec<CommandDoc> {
        #[allow(unused_mut)] // mut needed with feature gates
        let mut docs = vec![
            // Search command
            CommandDoc {
                name: "search".to_string(),
                aliases: vec!["q".to_string(), "query".to_string(), "find".to_string()],
                description: "Search documents using semantic and keyword matching".to_string(),
                arguments: vec![ArgumentDoc {
                    name: "query".to_string(),
                    arg_type: "string".to_string(),
                    required: true,
                    description: "Search query text".to_string(),
                    default: None,
                }],
                flags: vec![
                    FlagDoc {
                        name: "--role".to_string(),
                        short: Some("-r".to_string()),
                        flag_type: "string".to_string(),
                        default: Some("current".to_string()),
                        description: "Role context for search".to_string(),
                    },
                    FlagDoc {
                        name: "--limit".to_string(),
                        short: Some("-l".to_string()),
                        flag_type: "integer".to_string(),
                        default: Some("10".to_string()),
                        description: "Maximum results to return".to_string(),
                    },
                    FlagDoc {
                        name: "--semantic".to_string(),
                        short: None,
                        flag_type: "boolean".to_string(),
                        default: Some("false".to_string()),
                        description: "Enable semantic search".to_string(),
                    },
                    FlagDoc {
                        name: "--concepts".to_string(),
                        short: None,
                        flag_type: "boolean".to_string(),
                        default: Some("false".to_string()),
                        description: "Include concept matches".to_string(),
                    },
                ],
                examples: vec![
                    ExampleDoc {
                        description: "Basic search".to_string(),
                        command: "/search async error handling".to_string(),
                        output: None,
                    },
                    ExampleDoc {
                        description: "Search with role and limit".to_string(),
                        command: "/search database migration --role DevOps --limit 5".to_string(),
                        output: None,
                    },
                ],
                response_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "results": {
                            "type": "array",
                            "items": {
                                "type": "object",
                                "properties": {
                                    "rank": {"type": "integer"},
                                    "id": {"type": "string"},
                                    "title": {"type": "string"},
                                    "url": {"type": "string"},
                                    "score": {"type": "number"},
                                    "preview": {"type": "string"}
                                }
                            }
                        },
                        "total_matches": {"type": "integer"},
                        "concepts_matched": {"type": "array", "items": {"type": "string"}}
                    }
                }),
            },
            // Config command
            CommandDoc {
                name: "config".to_string(),
                aliases: vec!["c".to_string(), "cfg".to_string()],
                description: "View and modify configuration".to_string(),
                arguments: vec![ArgumentDoc {
                    name: "subcommand".to_string(),
                    arg_type: "string".to_string(),
                    required: true,
                    description: "Subcommand: show, set".to_string(),
                    default: None,
                }],
                flags: vec![],
                examples: vec![
                    ExampleDoc {
                        description: "Show current configuration".to_string(),
                        command: "/config show".to_string(),
                        output: None,
                    },
                    ExampleDoc {
                        description: "Set configuration value".to_string(),
                        command: "/config set selected_role Engineer".to_string(),
                        output: None,
                    },
                ],
                response_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "config": {"type": "object"}
                    }
                }),
            },
            // Role command
            CommandDoc {
                name: "role".to_string(),
                aliases: vec!["r".to_string()],
                description: "Manage roles".to_string(),
                arguments: vec![ArgumentDoc {
                    name: "subcommand".to_string(),
                    arg_type: "string".to_string(),
                    required: true,
                    description: "Subcommand: list, select".to_string(),
                    default: None,
                }],
                flags: vec![],
                examples: vec![
                    ExampleDoc {
                        description: "List available roles".to_string(),
                        command: "/role list".to_string(),
                        output: None,
                    },
                    ExampleDoc {
                        description: "Select a role".to_string(),
                        command: "/role select Engineer".to_string(),
                        output: None,
                    },
                ],
                response_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "roles": {"type": "array", "items": {"type": "string"}},
                        "current_role": {"type": "string"}
                    }
                }),
            },
            // Graph command
            CommandDoc {
                name: "graph".to_string(),
                aliases: vec!["g".to_string(), "kg".to_string()],
                description: "Display knowledge graph concepts".to_string(),
                arguments: vec![],
                flags: vec![FlagDoc {
                    name: "--top-k".to_string(),
                    short: Some("-k".to_string()),
                    flag_type: "integer".to_string(),
                    default: Some("10".to_string()),
                    description: "Number of top concepts to show".to_string(),
                }],
                examples: vec![
                    ExampleDoc {
                        description: "Show top concepts".to_string(),
                        command: "/graph".to_string(),
                        output: None,
                    },
                    ExampleDoc {
                        description: "Show top 20 concepts".to_string(),
                        command: "/graph --top-k 20".to_string(),
                        output: None,
                    },
                ],
                response_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "concepts": {
                            "type": "array",
                            "items": {
                                "type": "object",
                                "properties": {
                                    "term": {"type": "string"},
                                    "count": {"type": "integer"}
                                }
                            }
                        }
                    }
                }),
            },
            // VM command
            CommandDoc {
                name: "vm".to_string(),
                aliases: vec![],
                description: "Manage Firecracker VMs".to_string(),
                arguments: vec![ArgumentDoc {
                    name: "subcommand".to_string(),
                    arg_type: "string".to_string(),
                    required: true,
                    description: "Subcommand: list, pool, status, metrics, execute, agent, tasks, allocate, release, monitor".to_string(),
                    default: None,
                }],
                flags: vec![
                    FlagDoc {
                        name: "--vm-id".to_string(),
                        short: None,
                        flag_type: "string".to_string(),
                        default: None,
                        description: "VM identifier".to_string(),
                    },
                ],
                examples: vec![
                    ExampleDoc {
                        description: "List VMs".to_string(),
                        command: "/vm list".to_string(),
                        output: None,
                    },
                    ExampleDoc {
                        description: "Execute code in VM".to_string(),
                        command: "/vm execute python print('hello')".to_string(),
                        output: None,
                    },
                ],
                response_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "vms": {"type": "array"},
                        "status": {"type": "string"}
                    }
                }),
            },
            // Help command
            CommandDoc {
                name: "help".to_string(),
                aliases: vec!["h".to_string(), "?".to_string()],
                description: "Show help information".to_string(),
                arguments: vec![ArgumentDoc {
                    name: "command".to_string(),
                    arg_type: "string".to_string(),
                    required: false,
                    description: "Command to get help for".to_string(),
                    default: None,
                }],
                flags: vec![],
                examples: vec![
                    ExampleDoc {
                        description: "Show all commands".to_string(),
                        command: "/help".to_string(),
                        output: None,
                    },
                    ExampleDoc {
                        description: "Get help for search".to_string(),
                        command: "/help search".to_string(),
                        output: None,
                    },
                ],
                response_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "commands": {"type": "array"},
                        "help_text": {"type": "string"}
                    }
                }),
            },
            // Robot command (self-documentation)
            CommandDoc {
                name: "robot".to_string(),
                aliases: vec![],
                description: "Robot mode commands for AI agents".to_string(),
                arguments: vec![ArgumentDoc {
                    name: "subcommand".to_string(),
                    arg_type: "string".to_string(),
                    required: true,
                    description: "Subcommand: capabilities, schemas, examples".to_string(),
                    default: None,
                }],
                flags: vec![
                    FlagDoc {
                        name: "--format".to_string(),
                        short: Some("-f".to_string()),
                        flag_type: "string".to_string(),
                        default: Some("json".to_string()),
                        description: "Output format: json, jsonl, minimal, table".to_string(),
                    },
                ],
                examples: vec![
                    ExampleDoc {
                        description: "Get capabilities".to_string(),
                        command: "/robot capabilities".to_string(),
                        output: None,
                    },
                    ExampleDoc {
                        description: "Get schema for search".to_string(),
                        command: "/robot schemas search".to_string(),
                        output: None,
                    },
                ],
                response_schema: serde_json::json!({
                    "type": "object"
                }),
            },
        ];

        // Add feature-gated commands
        #[cfg(feature = "repl-chat")]
        {
            docs.push(CommandDoc {
                name: "chat".to_string(),
                aliases: vec![],
                description: "Interactive chat with AI".to_string(),
                arguments: vec![ArgumentDoc {
                    name: "message".to_string(),
                    arg_type: "string".to_string(),
                    required: false,
                    description: "Message to send".to_string(),
                    default: None,
                }],
                flags: vec![],
                examples: vec![ExampleDoc {
                    description: "Send a message".to_string(),
                    command: "/chat How do I handle errors in Rust?".to_string(),
                    output: None,
                }],
                response_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "response": {"type": "string"}
                    }
                }),
            });

            docs.push(CommandDoc {
                name: "summarize".to_string(),
                aliases: vec![],
                description: "Summarize content".to_string(),
                arguments: vec![ArgumentDoc {
                    name: "target".to_string(),
                    arg_type: "string".to_string(),
                    required: true,
                    description: "Document ID or text to summarize".to_string(),
                    default: None,
                }],
                flags: vec![],
                examples: vec![ExampleDoc {
                    description: "Summarize a document".to_string(),
                    command: "/summarize doc-123".to_string(),
                    output: None,
                }],
                response_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "summary": {"type": "string"}
                    }
                }),
            });
        }

        #[cfg(feature = "repl-mcp")]
        {
            docs.push(CommandDoc {
                name: "autocomplete".to_string(),
                aliases: vec!["ac".to_string()],
                description: "Autocomplete terms from thesaurus".to_string(),
                arguments: vec![ArgumentDoc {
                    name: "query".to_string(),
                    arg_type: "string".to_string(),
                    required: true,
                    description: "Partial term to complete".to_string(),
                    default: None,
                }],
                flags: vec![FlagDoc {
                    name: "--limit".to_string(),
                    short: Some("-l".to_string()),
                    flag_type: "integer".to_string(),
                    default: Some("10".to_string()),
                    description: "Maximum suggestions".to_string(),
                }],
                examples: vec![ExampleDoc {
                    description: "Autocomplete a term".to_string(),
                    command: "/autocomplete auth".to_string(),
                    output: None,
                }],
                response_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "suggestions": {"type": "array", "items": {"type": "string"}}
                    }
                }),
            });

            docs.push(CommandDoc {
                name: "extract".to_string(),
                aliases: vec![],
                description: "Extract paragraphs containing matched terms".to_string(),
                arguments: vec![ArgumentDoc {
                    name: "text".to_string(),
                    arg_type: "string".to_string(),
                    required: true,
                    description: "Text to extract from".to_string(),
                    default: None,
                }],
                flags: vec![FlagDoc {
                    name: "--exclude-term".to_string(),
                    short: None,
                    flag_type: "boolean".to_string(),
                    default: Some("false".to_string()),
                    description: "Exclude the matched term from output".to_string(),
                }],
                examples: vec![ExampleDoc {
                    description: "Extract paragraphs".to_string(),
                    command:
                        "/extract \"This text contains authentication and authorization concepts.\""
                            .to_string(),
                    output: None,
                }],
                response_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "paragraphs": {"type": "array", "items": {"type": "string"}}
                    }
                }),
            });

            docs.push(CommandDoc {
                name: "find".to_string(),
                aliases: vec![],
                description: "Find term matches in text".to_string(),
                arguments: vec![ArgumentDoc {
                    name: "text".to_string(),
                    arg_type: "string".to_string(),
                    required: true,
                    description: "Text to search".to_string(),
                    default: None,
                }],
                flags: vec![],
                examples: vec![ExampleDoc {
                    description: "Find matches".to_string(),
                    command: "/find \"async programming patterns\"".to_string(),
                    output: None,
                }],
                response_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "matches": {"type": "array", "items": {"type": "object"}}
                    }
                }),
            });

            docs.push(CommandDoc {
                name: "replace".to_string(),
                aliases: vec![],
                description: "Replace matched terms with links".to_string(),
                arguments: vec![ArgumentDoc {
                    name: "text".to_string(),
                    arg_type: "string".to_string(),
                    required: true,
                    description: "Text to process".to_string(),
                    default: None,
                }],
                flags: vec![FlagDoc {
                    name: "--format".to_string(),
                    short: Some("-f".to_string()),
                    flag_type: "string".to_string(),
                    default: Some("markdown".to_string()),
                    description: "Link format: markdown, wiki, html, plain".to_string(),
                }],
                examples: vec![ExampleDoc {
                    description: "Replace with markdown links".to_string(),
                    command: "/replace \"Learn about authentication\" --format markdown"
                        .to_string(),
                    output: None,
                }],
                response_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "result": {"type": "string"}
                    }
                }),
            });

            docs.push(CommandDoc {
                name: "thesaurus".to_string(),
                aliases: vec!["th".to_string()],
                description: "Show thesaurus entries".to_string(),
                arguments: vec![],
                flags: vec![FlagDoc {
                    name: "--role".to_string(),
                    short: Some("-r".to_string()),
                    flag_type: "string".to_string(),
                    default: Some("current".to_string()),
                    description: "Role to get thesaurus for".to_string(),
                }],
                examples: vec![ExampleDoc {
                    description: "Show thesaurus".to_string(),
                    command: "/thesaurus".to_string(),
                    output: None,
                }],
                response_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "entries": {"type": "array"}
                    }
                }),
            });
        }

        docs
    }
}

impl Default for SelfDocumentation {
    fn default() -> Self {
        Self::new()
    }
}

/// Capabilities summary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Capabilities {
    pub name: String,
    pub version: String,
    pub description: String,
    pub features: FeatureFlags,
    pub commands: Vec<String>,
    pub supported_formats: Vec<String>,
}

/// Documentation for a single command
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandDoc {
    pub name: String,
    pub aliases: Vec<String>,
    pub description: String,
    pub arguments: Vec<ArgumentDoc>,
    pub flags: Vec<FlagDoc>,
    pub examples: Vec<ExampleDoc>,
    pub response_schema: serde_json::Value,
}

/// Documentation for a command argument
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArgumentDoc {
    pub name: String,
    #[serde(rename = "type")]
    pub arg_type: String,
    pub required: bool,
    pub description: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default: Option<String>,
}

/// Documentation for a command flag
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlagDoc {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub short: Option<String>,
    #[serde(rename = "type")]
    pub flag_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default: Option<String>,
    pub description: String,
}

/// Documentation for a command example
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExampleDoc {
    pub description: String,
    pub command: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_self_documentation_new() {
        let docs = SelfDocumentation::new();
        assert!(!docs.commands.is_empty());
    }

    #[test]
    fn test_capabilities() {
        let docs = SelfDocumentation::new();
        let caps = docs.capabilities();

        assert_eq!(caps.name, "terraphim-agent");
        assert!(!caps.commands.is_empty());
        assert!(caps.supported_formats.contains(&"json".to_string()));
    }

    #[test]
    fn test_schema_lookup() {
        let docs = SelfDocumentation::new();

        let search_doc = docs.schema("search");
        assert!(search_doc.is_some());
        assert_eq!(search_doc.unwrap().name, "search");

        let unknown_doc = docs.schema("nonexistent");
        assert!(unknown_doc.is_none());
    }

    #[test]
    fn test_examples() {
        let docs = SelfDocumentation::new();

        let examples = docs.examples("search");
        assert!(examples.is_some());
        assert!(!examples.unwrap().is_empty());
    }

    #[test]
    fn test_command_doc_serialization() {
        let docs = SelfDocumentation::new();
        let search_doc = docs.schema("search").unwrap();

        let json = serde_json::to_string(search_doc).unwrap();
        assert!(json.contains("search"));
        assert!(json.contains("query"));
    }
}
