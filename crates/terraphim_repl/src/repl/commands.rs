//! Command definitions for REPL interface (minimal release)

use anyhow::{Result, anyhow};
use std::str::FromStr;

#[derive(Debug, Clone, PartialEq)]
pub enum ReplCommand {
    // Core search and navigation
    Search {
        query: String,
        role: Option<String>,
        limit: Option<usize>,
    },

    // Configuration management
    Config {
        subcommand: ConfigSubcommand,
    },

    // Role management
    Role {
        subcommand: RoleSubcommand,
    },

    // Knowledge graph
    Graph {
        top_k: Option<usize>,
    },

    // Knowledge graph operations
    Replace {
        text: String,
        format: Option<String>,
    },
    Find {
        text: String,
    },
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

        // Handle commands with or without leading slash
        let input = input.strip_prefix('/').unwrap_or(input);

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
                    // Default to show if no subcommand
                    return Ok(ReplCommand::Config {
                        subcommand: ConfigSubcommand::Show,
                    });
                }

                match parts[1] {
                    "show" => Ok(ReplCommand::Config {
                        subcommand: ConfigSubcommand::Show,
                    }),
                    _ => Err(anyhow!(
                        "Invalid config subcommand: {}. Use: show",
                        parts[1]
                    )),
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

            "find" => {
                if parts.len() < 2 {
                    return Err(anyhow!("Find command requires text"));
                }
                Ok(ReplCommand::Find {
                    text: parts[1..].join(" "),
                })
            }

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

            _ => Err(anyhow!(
                "Unknown command: {}. Type /help for available commands",
                parts[0]
            )),
        }
    }
}

impl ReplCommand {
    /// Get available commands for the minimal release
    pub fn available_commands() -> Vec<&'static str> {
        vec![
            "search",
            "config",
            "role",
            "graph",
            "replace",
            "find",
            "thesaurus",
            "help",
            "quit",
            "exit",
            "clear",
        ]
    }

    /// Get command description for help system
    pub fn get_command_help(command: &str) -> Option<&'static str> {
        match command {
            "search" => Some(
                "/search <query> [--role <role>] [--limit <n>]\n\
                 Search for documents matching the query.\n\
                 \n\
                 Examples:\n\
                   /search rust async\n\
                   /search api --role Engineer --limit 5",
            ),
            "config" => Some(
                "/config [show]\n\
                 Display current configuration.\n\
                 \n\
                 Example:\n\
                   /config show",
            ),
            "role" => Some(
                "/role list | select <name>\n\
                 Manage roles. List available roles or select a new active role.\n\
                 \n\
                 Examples:\n\
                   /role list\n\
                   /role select Engineer",
            ),
            "graph" => Some(
                "/graph [--top-k <n>]\n\
                 Show the knowledge graph's top concepts.\n\
                 \n\
                 Examples:\n\
                   /graph\n\
                   /graph --top-k 20",
            ),
            "replace" => Some(
                "/replace <text> [--format <format>]\n\
                 Replace matched terms in text with links using the knowledge graph.\n\
                 Formats: markdown (default), html, wiki, plain\n\
                 \n\
                 Examples:\n\
                   /replace rust is a programming language\n\
                   /replace async programming with tokio --format markdown\n\
                   /replace check out rust --format html",
            ),
            "find" => Some(
                "/find <text>\n\
                 Find all terms in text that match the knowledge graph.\n\
                 Shows matched terms with their positions.\n\
                 \n\
                 Examples:\n\
                   /find rust async programming\n\
                   /find this is about tokio and async",
            ),
            "thesaurus" => Some(
                "/thesaurus [--role <role>]\n\
                 Display the thesaurus (knowledge graph terms) for current or specified role.\n\
                 Shows term mappings with IDs and URLs.\n\
                 \n\
                 Examples:\n\
                   /thesaurus\n\
                   /thesaurus --role Engineer",
            ),
            "help" => Some(
                "/help [command]\n\
                 Show help information. Provide a command name for detailed help.\n\
                 \n\
                 Examples:\n\
                   /help\n\
                   /help search",
            ),
            "quit" | "q" => Some("/quit, /q - Exit the REPL"),
            "exit" => Some("/exit - Exit the REPL"),
            "clear" => Some("/clear - Clear the screen"),
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
                limit: None,
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
                limit: Some(5),
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

        // Default to show if no subcommand
        let cmd = "/config".parse::<ReplCommand>().unwrap();
        assert_eq!(
            cmd,
            ReplCommand::Config {
                subcommand: ConfigSubcommand::Show
            }
        );
    }

    #[test]
    fn test_role_command_parsing() {
        let cmd = "/role list".parse::<ReplCommand>().unwrap();
        assert_eq!(
            cmd,
            ReplCommand::Role {
                subcommand: RoleSubcommand::List
            }
        );

        let cmd = "/role select Engineer".parse::<ReplCommand>().unwrap();
        assert_eq!(
            cmd,
            ReplCommand::Role {
                subcommand: RoleSubcommand::Select {
                    name: "Engineer".to_string()
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

    #[test]
    fn test_graph_command_parsing() {
        let cmd = "/graph".parse::<ReplCommand>().unwrap();
        assert_eq!(cmd, ReplCommand::Graph { top_k: None });

        let cmd = "/graph --top-k 15".parse::<ReplCommand>().unwrap();
        assert_eq!(cmd, ReplCommand::Graph { top_k: Some(15) });
    }
}
