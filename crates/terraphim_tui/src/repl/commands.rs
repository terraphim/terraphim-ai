//! Command definitions for REPL interface

use anyhow::{anyhow, Result};
use std::str::FromStr;

#[derive(Debug, Clone, PartialEq)]
pub enum ReplCommand {
    // Base commands (always available with 'repl' feature)
    Search {
        query: String,
        role: Option<String>,
        limit: Option<usize>,
    },
    Config {
        subcommand: ConfigSubcommand,
    },
    Role {
        subcommand: RoleSubcommand,
    },
    Graph {
        top_k: Option<usize>,
    },

    // Chat commands (requires 'repl-chat' feature)
    #[cfg(feature = "repl-chat")]
    Chat {
        message: Option<String>,
    },

    #[cfg(feature = "repl-chat")]
    Summarize {
        target: String,
    },

    // MCP commands (requires 'repl-mcp' feature)
    #[cfg(feature = "repl-mcp")]
    Autocomplete {
        query: String,
        limit: Option<usize>,
    },

    #[cfg(feature = "repl-mcp")]
    Extract {
        text: String,
        exclude_term: bool,
    },

    #[cfg(feature = "repl-mcp")]
    Find {
        text: String,
    },

    #[cfg(feature = "repl-mcp")]
    Replace {
        text: String,
        format: Option<String>,
    },

    #[cfg(feature = "repl-mcp")]
    Thesaurus {
        role: Option<String>,
    },

    // Utility commands
    Help {
        command: Option<String>,
    },
    Quit,
    Exit,
    Clear,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ConfigSubcommand {
    Show,
    Set { key: String, value: String },
}

#[derive(Debug, Clone, PartialEq)]
pub enum RoleSubcommand {
    List,
    Select { name: String },
}

impl FromStr for ReplCommand {
    type Err = anyhow::Error;

    fn from_str(input: &str) -> Result<Self> {
        let input = input.trim();
        if input.is_empty() {
            return Err(anyhow!("Empty command"));
        }

        // Handle commands without leading slash
        let input = if input.starts_with('/') {
            &input[1..]
        } else {
            input
        };

        let parts: Vec<&str> = input.split_whitespace().collect();
        if parts.is_empty() {
            return Err(anyhow!("Empty command"));
        }

        match parts[0] {
            "search" => {
                if parts.len() < 2 {
                    return Err(anyhow!("Search command requires a query"));
                }

                let mut query = String::new();
                let mut role = None;
                let mut limit = None;
                let mut i = 1;

                while i < parts.len() {
                    match parts[i] {
                        "--role" => {
                            if i + 1 < parts.len() {
                                role = Some(parts[i + 1].to_string());
                                i += 2;
                            } else {
                                return Err(anyhow!("--role requires a value"));
                            }
                        }
                        "--limit" => {
                            if i + 1 < parts.len() {
                                limit = Some(
                                    parts[i + 1]
                                        .parse::<usize>()
                                        .map_err(|_| anyhow!("Invalid limit value"))?,
                                );
                                i += 2;
                            } else {
                                return Err(anyhow!("--limit requires a value"));
                            }
                        }
                        _ => {
                            if !query.is_empty() {
                                query.push(' ');
                            }
                            query.push_str(parts[i]);
                            i += 1;
                        }
                    }
                }

                if query.is_empty() {
                    return Err(anyhow!("Search query cannot be empty"));
                }

                Ok(ReplCommand::Search { query, role, limit })
            }

            "config" => {
                if parts.len() < 2 {
                    return Err(anyhow!(
                        "Config command requires a subcommand (show | set <key> <value>)"
                    ));
                }

                match parts[1] {
                    "show" => Ok(ReplCommand::Config {
                        subcommand: ConfigSubcommand::Show,
                    }),
                    "set" => {
                        if parts.len() < 4 {
                            return Err(anyhow!("Config set requires key and value"));
                        }
                        Ok(ReplCommand::Config {
                            subcommand: ConfigSubcommand::Set {
                                key: parts[2].to_string(),
                                value: parts[3..].join(" "),
                            },
                        })
                    }
                    _ => Err(anyhow!("Invalid config subcommand: {}", parts[1])),
                }
            }

            "role" => {
                if parts.len() < 2 {
                    return Err(anyhow!(
                        "Role command requires a subcommand (list | select <name>)"
                    ));
                }

                match parts[1] {
                    "list" => Ok(ReplCommand::Role {
                        subcommand: RoleSubcommand::List,
                    }),
                    "select" => {
                        if parts.len() < 3 {
                            return Err(anyhow!("Role select requires a role name"));
                        }
                        Ok(ReplCommand::Role {
                            subcommand: RoleSubcommand::Select {
                                name: parts[2..].join(" "),
                            },
                        })
                    }
                    _ => Err(anyhow!("Invalid role subcommand: {}", parts[1])),
                }
            }

            "graph" => {
                let mut top_k = None;
                let mut i = 1;

                while i < parts.len() {
                    match parts[i] {
                        "--top-k" => {
                            if i + 1 < parts.len() {
                                top_k = Some(
                                    parts[i + 1]
                                        .parse::<usize>()
                                        .map_err(|_| anyhow!("Invalid top-k value"))?,
                                );
                                i += 2;
                            } else {
                                return Err(anyhow!("--top-k requires a value"));
                            }
                        }
                        _ => {
                            return Err(anyhow!("Unknown graph option: {}", parts[i]));
                        }
                    }
                }

                Ok(ReplCommand::Graph { top_k })
            }

            #[cfg(feature = "repl-chat")]
            "chat" => {
                let message = if parts.len() > 1 {
                    Some(parts[1..].join(" "))
                } else {
                    None
                };
                Ok(ReplCommand::Chat { message })
            }

            #[cfg(not(feature = "repl-chat"))]
            "chat" => Err(anyhow!(
                "Chat feature not enabled. Rebuild with --features repl-chat"
            )),

            #[cfg(feature = "repl-chat")]
            "summarize" => {
                if parts.len() < 2 {
                    return Err(anyhow!(
                        "Summarize command requires a target (document ID or text)"
                    ));
                }
                Ok(ReplCommand::Summarize {
                    target: parts[1..].join(" "),
                })
            }

            #[cfg(not(feature = "repl-chat"))]
            "summarize" => Err(anyhow!(
                "Summarize feature not enabled. Rebuild with --features repl-chat"
            )),

            #[cfg(feature = "repl-mcp")]
            "autocomplete" => {
                if parts.len() < 2 {
                    return Err(anyhow!("Autocomplete command requires a query"));
                }

                let mut query = String::new();
                let mut limit = None;
                let mut i = 1;

                while i < parts.len() {
                    match parts[i] {
                        "--limit" => {
                            if i + 1 < parts.len() {
                                limit = Some(
                                    parts[i + 1]
                                        .parse::<usize>()
                                        .map_err(|_| anyhow!("Invalid limit value"))?,
                                );
                                i += 2;
                            } else {
                                return Err(anyhow!("--limit requires a value"));
                            }
                        }
                        _ => {
                            if !query.is_empty() {
                                query.push(' ');
                            }
                            query.push_str(parts[i]);
                            i += 1;
                        }
                    }
                }

                if query.is_empty() {
                    return Err(anyhow!("Autocomplete query cannot be empty"));
                }

                Ok(ReplCommand::Autocomplete { query, limit })
            }

            #[cfg(not(feature = "repl-mcp"))]
            "autocomplete" => Err(anyhow!(
                "MCP tools not enabled. Rebuild with --features repl-mcp"
            )),

            #[cfg(feature = "repl-mcp")]
            "extract" => {
                if parts.len() < 2 {
                    return Err(anyhow!("Extract command requires text"));
                }

                let mut text = String::new();
                let mut exclude_term = false;
                let mut i = 1;

                while i < parts.len() {
                    match parts[i] {
                        "--exclude-term" => {
                            exclude_term = true;
                            i += 1;
                        }
                        _ => {
                            if !text.is_empty() {
                                text.push(' ');
                            }
                            text.push_str(parts[i]);
                            i += 1;
                        }
                    }
                }

                if text.is_empty() {
                    return Err(anyhow!("Extract text cannot be empty"));
                }

                Ok(ReplCommand::Extract { text, exclude_term })
            }

            #[cfg(not(feature = "repl-mcp"))]
            "extract" => Err(anyhow!(
                "MCP tools not enabled. Rebuild with --features repl-mcp"
            )),

            #[cfg(feature = "repl-mcp")]
            "find" => {
                if parts.len() < 2 {
                    return Err(anyhow!("Find command requires text"));
                }
                Ok(ReplCommand::Find {
                    text: parts[1..].join(" "),
                })
            }

            #[cfg(not(feature = "repl-mcp"))]
            "find" => Err(anyhow!(
                "MCP tools not enabled. Rebuild with --features repl-mcp"
            )),

            #[cfg(feature = "repl-mcp")]
            "replace" => {
                if parts.len() < 2 {
                    return Err(anyhow!("Replace command requires text"));
                }

                let mut text = String::new();
                let mut format = None;
                let mut i = 1;

                while i < parts.len() {
                    match parts[i] {
                        "--format" => {
                            if i + 1 < parts.len() {
                                format = Some(parts[i + 1].to_string());
                                i += 2;
                            } else {
                                return Err(anyhow!("--format requires a value"));
                            }
                        }
                        _ => {
                            if !text.is_empty() {
                                text.push(' ');
                            }
                            text.push_str(parts[i]);
                            i += 1;
                        }
                    }
                }

                if text.is_empty() {
                    return Err(anyhow!("Replace text cannot be empty"));
                }

                Ok(ReplCommand::Replace { text, format })
            }

            #[cfg(not(feature = "repl-mcp"))]
            "replace" => Err(anyhow!(
                "MCP tools not enabled. Rebuild with --features repl-mcp"
            )),

            #[cfg(feature = "repl-mcp")]
            "thesaurus" => {
                let mut role = None;
                let mut i = 1;

                while i < parts.len() {
                    match parts[i] {
                        "--role" => {
                            if i + 1 < parts.len() {
                                role = Some(parts[i + 1].to_string());
                                i += 2;
                            } else {
                                return Err(anyhow!("--role requires a value"));
                            }
                        }
                        _ => {
                            return Err(anyhow!("Unknown thesaurus option: {}", parts[i]));
                        }
                    }
                }

                Ok(ReplCommand::Thesaurus { role })
            }

            #[cfg(not(feature = "repl-mcp"))]
            "thesaurus" => Err(anyhow!(
                "MCP tools not enabled. Rebuild with --features repl-mcp"
            )),

            "help" => {
                let command = if parts.len() > 1 {
                    Some(parts[1].to_string())
                } else {
                    None
                };
                Ok(ReplCommand::Help { command })
            }

            "quit" | "q" => Ok(ReplCommand::Quit),
            "exit" => Ok(ReplCommand::Exit),
            "clear" => Ok(ReplCommand::Clear),

            _ => Err(anyhow!("Unknown command: {}", parts[0])),
        }
    }
}

impl ReplCommand {
    /// Get available commands based on compiled features
    pub fn available_commands() -> Vec<&'static str> {
        let mut commands = vec![
            "search", "config", "role", "graph", "help", "quit", "exit", "clear",
        ];

        #[cfg(feature = "repl-chat")]
        {
            commands.extend_from_slice(&["chat", "summarize"]);
        }

        #[cfg(feature = "repl-mcp")]
        {
            commands.extend_from_slice(&[
                "autocomplete",
                "extract",
                "find",
                "replace",
                "thesaurus",
            ]);
        }

        commands
    }

    /// Get command description for help system
    pub fn get_command_help(command: &str) -> Option<&'static str> {
        match command {
            "search" => Some("/search <query> [--role <role>] [--limit <n>] - Search documents"),
            "config" => Some("/config show | set <key> <value> - Manage configuration"),
            "role" => Some("/role list | select <name> - Manage roles"),
            "graph" => Some("/graph [--top-k <n>] - Show knowledge graph"),
            "help" => Some("/help [command] - Show help information"),
            "quit" | "q" => Some("/quit, /q - Exit REPL"),
            "exit" => Some("/exit - Exit REPL"),
            "clear" => Some("/clear - Clear screen"),

            #[cfg(feature = "repl-chat")]
            "chat" => Some("/chat [message] - Interactive chat with AI"),
            #[cfg(feature = "repl-chat")]
            "summarize" => Some("/summarize <doc-id|text> - Summarize content"),

            #[cfg(feature = "repl-mcp")]
            "autocomplete" => Some("/autocomplete <query> [--limit <n>] - Autocomplete terms"),
            #[cfg(feature = "repl-mcp")]
            "extract" => Some("/extract <text> [--exclude-term] - Extract paragraphs"),
            #[cfg(feature = "repl-mcp")]
            "find" => Some("/find <text> - Find matches in text"),
            #[cfg(feature = "repl-mcp")]
            "replace" => Some("/replace <text> [--format <format>] - Replace matches"),
            #[cfg(feature = "repl-mcp")]
            "thesaurus" => Some("/thesaurus [--role <role>] - Show thesaurus entries"),

            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_search_command_parsing() {
        let cmd = "/search hello world".parse::<ReplCommand>().unwrap();
        assert_eq!(
            cmd,
            ReplCommand::Search {
                query: "hello world".to_string(),
                role: None,
                limit: None
            }
        );

        let cmd = "/search test --role Engineer --limit 5"
            .parse::<ReplCommand>()
            .unwrap();
        assert_eq!(
            cmd,
            ReplCommand::Search {
                query: "test".to_string(),
                role: Some("Engineer".to_string()),
                limit: Some(5)
            }
        );
    }

    #[test]
    fn test_config_command_parsing() {
        let cmd = "/config show".parse::<ReplCommand>().unwrap();
        assert_eq!(
            cmd,
            ReplCommand::Config {
                subcommand: ConfigSubcommand::Show
            }
        );

        let cmd = "/config set selected_role Engineer"
            .parse::<ReplCommand>()
            .unwrap();
        assert_eq!(
            cmd,
            ReplCommand::Config {
                subcommand: ConfigSubcommand::Set {
                    key: "selected_role".to_string(),
                    value: "Engineer".to_string()
                }
            }
        );
    }

    #[test]
    fn test_utility_commands() {
        assert_eq!("/quit".parse::<ReplCommand>().unwrap(), ReplCommand::Quit);
        assert_eq!("/exit".parse::<ReplCommand>().unwrap(), ReplCommand::Exit);
        assert_eq!("/clear".parse::<ReplCommand>().unwrap(), ReplCommand::Clear);

        let help_cmd = "/help search".parse::<ReplCommand>().unwrap();
        assert_eq!(
            help_cmd,
            ReplCommand::Help {
                command: Some("search".to_string())
            }
        );
    }
}
