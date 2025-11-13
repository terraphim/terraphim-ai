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
        semantic: bool,
        concepts: bool,
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

    // File commands (requires 'repl-file' feature)
    #[cfg(feature = "repl-file")]
    File {
        subcommand: FileSubcommand,
    },

    // Web commands (requires 'repl-web' feature)
    #[cfg(feature = "repl-web")]
    Web {
        subcommand: WebSubcommand,
    },

    // VM commands
    Vm {
        subcommand: VmSubcommand,
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

#[derive(Debug, Clone, PartialEq)]
#[cfg(feature = "repl-file")]
pub enum FileSubcommand {
    Search { query: String },
    List,
    Info { path: String },
}

#[derive(Debug, Clone, PartialEq)]
pub enum VmSubcommand {
    List,
    Pool,
    Status {
        vm_id: Option<String>,
    },
    Metrics {
        vm_id: Option<String>,
    },
    Execute {
        code: String,
        language: String,
        vm_id: Option<String>,
    },
    Agent {
        agent_id: String,
        task: String,
        vm_id: Option<String>,
    },
    Tasks {
        vm_id: String,
    },
    Allocate {
        vm_id: String,
    },
    Release {
        vm_id: String,
    },
    Monitor {
        vm_id: String,
        refresh: Option<u32>,
    },
}

#[derive(Debug, Clone, PartialEq)]
#[cfg(feature = "repl-web")]
pub enum WebSubcommand {
    Get {
        url: String,
        headers: Option<std::collections::HashMap<String, String>>,
    },
    Post {
        url: String,
        body: String,
        headers: Option<std::collections::HashMap<String, String>>,
    },
    Scrape {
        url: String,
        selector: Option<String>,
        wait_for_element: Option<String>,
    },
    Screenshot {
        url: String,
        width: Option<u32>,
        height: Option<u32>,
        full_page: Option<bool>,
    },
    Pdf {
        url: String,
        page_size: Option<String>,
    },
    Form {
        url: String,
        form_data: std::collections::HashMap<String, String>,
    },
    Api {
        endpoint: String,
        method: String,
        data: Option<serde_json::Value>,
    },
    Status {
        operation_id: String,
    },
    Cancel {
        operation_id: String,
    },
    History {
        limit: Option<usize>,
    },
    Config {
        subcommand: WebConfigSubcommand,
    },
}

#[derive(Debug, Clone, PartialEq)]
#[cfg(feature = "repl-web")]
pub enum WebConfigSubcommand {
    Show,
    Set { key: String, value: String },
    Reset,
}

impl FromStr for ReplCommand {
    type Err = anyhow::Error;

    fn from_str(input: &str) -> Result<Self> {
        let input = input.trim();
        if input.is_empty() {
            return Err(anyhow!("Empty command"));
        }

        // Handle commands without leading slash
        let input = if let Some(stripped) = input.strip_prefix('/') {
            stripped
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
                let _semantic = false;
                let _concepts = false;
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

                // Handle --semantic and --concepts flags that might be in the query
                let mut semantic = false;
                let mut concepts = false;
                let query_parts: Vec<&str> = query.split_whitespace().collect();
                let mut filtered_query_parts = Vec::new();

                for part in query_parts {
                    match part {
                        "--semantic" => semantic = true,
                        "--concepts" => concepts = true,
                        _ => filtered_query_parts.push(part),
                    }
                }

                query = filtered_query_parts.join(" ");

                if query.is_empty() {
                    return Err(anyhow!("Search query cannot be empty"));
                }

                Ok(ReplCommand::Search {
                    query,
                    role,
                    limit,
                    semantic,
                    concepts,
                })
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

            #[cfg(feature = "repl-file")]
            "file" => {
                if parts.len() < 2 {
                    return Err(anyhow!("File command requires a subcommand"));
                }

                match parts[1] {
                    "search" => {
                        if parts.len() < 3 {
                            return Err(anyhow!("File search requires a query"));
                        }
                        let query = parts[2..].join(" ");
                        Ok(ReplCommand::File {
                            subcommand: FileSubcommand::Search { query },
                        })
                    }
                    "list" => Ok(ReplCommand::File {
                        subcommand: FileSubcommand::List,
                    }),
                    "info" => {
                        if parts.len() < 3 {
                            return Err(anyhow!("File info requires a path"));
                        }
                        let path = parts[2].to_string();
                        Ok(ReplCommand::File {
                            subcommand: FileSubcommand::Info { path },
                        })
                    }
                    _ => Err(anyhow!(
                        "Unknown file subcommand: {}. Use: search, list, info",
                        parts[1]
                    )),
                }
            }

            #[cfg(not(feature = "repl-file"))]
            "file" => Err(anyhow!(
                "File operations not enabled. Rebuild with --features repl-file"
            )),

            #[cfg(feature = "repl-web")]
            "web" => {
                if parts.len() < 2 {
                    return Err(anyhow!("Web command requires a subcommand"));
                }

                match parts[1] {
                    "get" => {
                        if parts.len() < 3 {
                            return Err(anyhow!("Web GET requires a URL"));
                        }
                        let url = parts[2].to_string();
                        Ok(ReplCommand::Web {
                            subcommand: WebSubcommand::Get { url, headers: None },
                        })
                    }
                    "post" => {
                        if parts.len() < 3 {
                            return Err(anyhow!("Web POST requires a URL"));
                        }
                        let url = parts[2].to_string();
                        Ok(ReplCommand::Web {
                            subcommand: WebSubcommand::Post { url, body: String::new(), headers: None },
                        })
                    }
                    "scrape" => {
                        if parts.len() < 3 {
                            return Err(anyhow!("Web scrape requires a URL"));
                        }
                        let url = parts[2].to_string();
                        Ok(ReplCommand::Web {
                            subcommand: WebSubcommand::Scrape { url, selector: None, wait_for_element: None },
                        })
                    }
                    "screenshot" => {
                        if parts.len() < 3 {
                            return Err(anyhow!("Web screenshot requires a URL"));
                        }
                        let url = parts[2].to_string();
                        Ok(ReplCommand::Web {
                            subcommand: WebSubcommand::Screenshot { url, width: None, height: None, full_page: None },
                        })
                    }
                    "pdf" => {
                        if parts.len() < 3 {
                            return Err(anyhow!("Web PDF requires a URL"));
                        }
                        let url = parts[2].to_string();
                        Ok(ReplCommand::Web {
                            subcommand: WebSubcommand::Pdf { url, page_size: None },
                        })
                    }
                    "form" => {
                        if parts.len() < 3 {
                            return Err(anyhow!("Web form requires a URL"));
                        }
                        let url = parts[2].to_string();
                        Ok(ReplCommand::Web {
                            subcommand: WebSubcommand::Form { url, form_data: std::collections::HashMap::new() },
                        })
                    }
                    "api" => {
                        if parts.len() < 3 {
                            return Err(anyhow!("Web API requires an endpoint"));
                        }
                        let endpoint = parts[2].to_string();
                        Ok(ReplCommand::Web {
                            subcommand: WebSubcommand::Api { endpoint, method: "GET".to_string(), data: None },
                        })
                    }
                    "status" => {
                        if parts.len() < 3 {
                            return Err(anyhow!("Web status requires an operation ID"));
                        }
                        let operation_id = parts[2].to_string();
                        Ok(ReplCommand::Web {
                            subcommand: WebSubcommand::Status { operation_id },
                        })
                    }
                    "cancel" => {
                        if parts.len() < 3 {
                            return Err(anyhow!("Web cancel requires an operation ID"));
                        }
                        let operation_id = parts[2].to_string();
                        Ok(ReplCommand::Web {
                            subcommand: WebSubcommand::Cancel { operation_id },
                        })
                    }
                    "history" => {
                        let limit = if parts.len() > 2 {
                            Some(parts[2].parse::<usize>().map_err(|_| anyhow!("Invalid limit"))?)
                        } else {
                            None
                        };
                        Ok(ReplCommand::Web {
                            subcommand: WebSubcommand::History { limit },
                        })
                    }
                    "config" => {
                        if parts.len() < 2 {
                            return Err(anyhow!("Web config requires a subcommand"));
                        }
                        let subcommand = if parts.len() > 2 {
                            match parts[2] {
                                "show" => WebConfigSubcommand::Show,
                                "set" => {
                                    if parts.len() < 5 {
                                        return Err(anyhow!("Web config set requires key and value"));
                                    }
                                    WebConfigSubcommand::Set {
                                        key: parts[3].to_string(),
                                        value: parts[4].to_string(),
                                    }
                                }
                                "reset" => WebConfigSubcommand::Reset,
                                _ => return Err(anyhow!("Unknown web config subcommand")),
                            }
                        } else {
                            WebConfigSubcommand::Show
                        };
                        Ok(ReplCommand::Web {
                            subcommand: WebSubcommand::Config { subcommand },
                        })
                    }
                    _ => Err(anyhow!(
                        "Unknown web subcommand: {}. Use: get, post, scrape, screenshot, pdf, form, api, status, cancel, history, config",
                        parts[1]
                    )),
                }
            }

            #[cfg(not(feature = "repl-web"))]
            "web" => Err(anyhow!(
                "Web operations not enabled. Rebuild with --features repl-web"
            )),

            "vm" => {
                if parts.len() < 2 {
                    return Err(anyhow!("VM command requires a subcommand"));
                }

                match parts[1] {
                    "list" => Ok(ReplCommand::Vm {
                        subcommand: VmSubcommand::List,
                    }),
                    "pool" => Ok(ReplCommand::Vm {
                        subcommand: VmSubcommand::Pool,
                    }),
                    "status" => {
                        let vm_id = if parts.len() > 2 {
                            Some(parts[2].to_string())
                        } else {
                            None
                        };
                        Ok(ReplCommand::Vm {
                            subcommand: VmSubcommand::Status { vm_id },
                        })
                    }
                    "metrics" => {
                        let vm_id = if parts.len() > 2 {
                            Some(parts[2].to_string())
                        } else {
                            None
                        };
                        Ok(ReplCommand::Vm {
                            subcommand: VmSubcommand::Metrics { vm_id },
                        })
                    }
                    "execute" => {
                        if parts.len() < 3 {
                            return Err(anyhow!("VM execute requires a language"));
                        }
                        let language = parts[2].to_string();
                        if parts.len() < 4 {
                            return Err(anyhow!("VM execute requires code to execute"));
                        }

                        let mut code = String::new();
                        let mut vm_id = None;
                        let mut i = 3;

                        while i < parts.len() {
                            match parts[i] {
                                "--vm-id" => {
                                    if i + 1 < parts.len() {
                                        vm_id = Some(parts[i + 1].to_string());
                                        i += 2;
                                    } else {
                                        return Err(anyhow!("--vm-id requires a value"));
                                    }
                                }
                                _ => {
                                    if !code.is_empty() {
                                        code.push(' ');
                                    }
                                    code.push_str(parts[i]);
                                    i += 1;
                                }
                            }
                        }

                        Ok(ReplCommand::Vm {
                            subcommand: VmSubcommand::Execute { code, language, vm_id },
                        })
                    }
                    "agent" => {
                        if parts.len() < 3 {
                            return Err(anyhow!("VM agent requires an agent ID"));
                        }
                        let agent_id = parts[2].to_string();
                        if parts.len() < 4 {
                            return Err(anyhow!("VM agent requires a task"));
                        }

                        let mut task = String::new();
                        let mut vm_id = None;
                        let mut i = 3;

                        while i < parts.len() {
                            match parts[i] {
                                "--vm-id" => {
                                    if i + 1 < parts.len() {
                                        vm_id = Some(parts[i + 1].to_string());
                                        i += 2;
                                    } else {
                                        return Err(anyhow!("--vm-id requires a value"));
                                    }
                                }
                                _ => {
                                    if !task.is_empty() {
                                        task.push(' ');
                                    }
                                    task.push_str(parts[i]);
                                    i += 1;
                                }
                            }
                        }

                        Ok(ReplCommand::Vm {
                            subcommand: VmSubcommand::Agent { agent_id, task, vm_id },
                        })
                    }
                    "tasks" => {
                        if parts.len() < 3 {
                            return Err(anyhow!("VM tasks requires a VM ID"));
                        }
                        let vm_id = parts[2].to_string();
                        Ok(ReplCommand::Vm {
                            subcommand: VmSubcommand::Tasks { vm_id },
                        })
                    }
                    "allocate" => {
                        if parts.len() < 3 {
                            return Err(anyhow!("VM allocate requires a VM ID"));
                        }
                        let vm_id = parts[2].to_string();
                        Ok(ReplCommand::Vm {
                            subcommand: VmSubcommand::Allocate { vm_id },
                        })
                    }
                    "release" => {
                        if parts.len() < 3 {
                            return Err(anyhow!("VM release requires a VM ID"));
                        }
                        let vm_id = parts[2].to_string();
                        Ok(ReplCommand::Vm {
                            subcommand: VmSubcommand::Release { vm_id },
                        })
                    }
                    "monitor" => {
                        if parts.len() < 3 {
                            return Err(anyhow!("VM monitor requires a VM ID"));
                        }
                        let vm_id = parts[2].to_string();
                        let mut refresh = None;
                        let mut i = 3;

                        while i < parts.len() {
                            match parts[i] {
                                "--refresh" => {
                                    if i + 1 < parts.len() {
                                        if let Ok(refresh_val) = parts[i + 1].parse::<u32>() {
                                            refresh = Some(refresh_val);
                                        } else {
                                            return Err(anyhow!("--refresh must be a positive integer"));
                                        }
                                        i += 2;
                                    } else {
                                        return Err(anyhow!("--refresh requires a value"));
                                    }
                                }
                                _ => {
                                    return Err(anyhow!("Unknown monitor option: {}", parts[i]));
                                }
                            }
                        }

                        Ok(ReplCommand::Vm {
                            subcommand: VmSubcommand::Monitor { vm_id, refresh },
                        })
                    }
                    _ => Err(anyhow!(
                        "Unknown VM subcommand: {}. Use: list, pool, status, metrics, execute, agent, tasks, allocate, release, monitor",
                        parts[1]
                    )),
                }
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

            _ => Err(anyhow!("Unknown command: {}", parts[0])),
        }
    }
}

impl ReplCommand {
    /// Get available commands based on compiled features
    pub fn available_commands() -> Vec<&'static str> {
        let mut commands = vec![
            "search", "config", "role", "graph", "vm", "help", "quit", "exit", "clear",
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

        #[cfg(feature = "repl-file")]
        {
            commands.extend_from_slice(&["file"]);
        }

        #[cfg(feature = "repl-web")]
        {
            commands.extend_from_slice(&["web"]);
        }

        commands
    }

    /// Get command description for help system
    pub fn get_command_help(command: &str) -> Option<&'static str> {
        match command {
            "search" => Some("/search <query> [--role <role>] [--limit <n>] [--semantic] [--concepts] - Search documents"),
            "config" => Some("/config show | set <key> <value> - Manage configuration"),
            "role" => Some("/role list | select <name> - Manage roles"),
            "graph" => Some("/graph [--top-k <n>] - Show knowledge graph"),
            "help" => Some("/help [command] - Show help information"),
            "quit" | "q" => Some("/quit, /q - Exit REPL"),
            "exit" => Some("/exit - Exit REPL"),
            "clear" => Some("/clear - Clear screen"),
            "vm" => Some("/vm <subcommand> [args] - VM management (list, pool, status, metrics, execute, agent, tasks, allocate, release, monitor)"),

            #[cfg(feature = "repl-file")]
            "file" => Some("/file <subcommand> [args] - File operations (search, list, info)"),

            #[cfg(feature = "repl-web")]
            "web" => Some("/web <subcommand> [args] - Web operations (get, post, scrape, screenshot, pdf, form, api, status, cancel, history, config)"),

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
