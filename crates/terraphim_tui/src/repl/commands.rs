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

    // Web operations commands
    Web {
        subcommand: WebSubcommand,
    },

    // File operations commands (requires 'repl-file' feature)
    #[cfg(feature = "repl-file")]
    File {
        subcommand: FileSubcommand,
    },

    // Command management
    #[cfg(feature = "repl-custom")]
    Commands {
        subcommand: CommandsSubcommand,
    },

    // RAG workflow - Context management
    #[cfg(feature = "repl-chat")]
    Context {
        subcommand: ContextSubcommand,
    },

    // RAG workflow - Conversation management
    #[cfg(feature = "repl-chat")]
    Conversation {
        subcommand: ConversationSubcommand,
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
pub enum WebSubcommand {
    Get {
        url: String,
        headers: Option<std::collections::HashMap<String, String>>,
    },
    Post {
        url: String,
        headers: Option<std::collections::HashMap<String, String>>,
        body: String,
    },
    Scrape {
        url: String,
        selector: String,
        wait: Option<String>,
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
        base_url: String,
        endpoints: Vec<String>,
        rate_limit: Option<u64>,
    },
    Status {
        operation_id: Option<String>,
    },
    Cancel {
        operation_id: String,
    },
    History {
        limit: Option<u32>,
    },
    Config {
        subcommand: WebConfigSubcommand,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub enum WebConfigSubcommand {
    Show,
    Set { key: String, value: String },
    Reset,
}

/// Commands subcommand enum for command management
#[derive(Debug, Clone, PartialEq)]
pub enum CommandsSubcommand {
    /// Initialize command system
    Init,
    /// List all available commands
    List,
    /// List commands in a specific category
    Category { category: String },
    /// Show detailed help for a command
    Help { command: String },
    /// Search commands by name or description
    Search { query: String },
    /// Reload commands from markdown files
    Reload,
    /// Validate command definitions
    Validate { command: Option<String> },
    /// Show registry statistics
    Stats,
    /// Suggest commands based on partial input
    Suggest {
        partial: String,
        limit: Option<usize>,
    },
}

#[derive(Debug, Clone, PartialEq)]
#[cfg(feature = "repl-file")]
pub enum FileSubcommand {
    /// Search files with semantic understanding
    Search {
        query: String,
        path: Option<String>,
        file_types: Option<Vec<String>>,
        semantic: bool,
        limit: Option<usize>,
    },
    /// Classify files by content type and purpose
    Classify {
        path: String,
        recursive: bool,
        update_metadata: bool,
    },
    /// Get intelligent file suggestions based on context
    Suggest {
        context: Option<String>,
        limit: Option<usize>,
        path: Option<String>,
    },
    /// Analyze file relationships and similarity
    Analyze {
        file_path: String,
        find_similar: bool,
        find_related: bool,
        threshold: Option<f64>,
    },
    /// Summarize file contents with semantic understanding
    Summarize {
        file_path: String,
        detail_level: Option<String>, // "brief", "detailed", "comprehensive"
        include_key_points: bool,
    },
    /// Extract semantic metadata from files
    Metadata {
        file_path: String,
        extract_concepts: bool,
        extract_entities: bool,
        extract_keywords: bool,
        update_index: bool,
    },
    /// Index files for semantic search
    Index {
        path: String,
        recursive: bool,
        force_reindex: bool,
    },
    /// Search within file contents with context awareness
    Find {
        pattern: String,
        path: Option<String>,
        context_lines: Option<usize>,
        case_sensitive: bool,
        whole_word: bool,
    },
    /// List files with semantic annotations
    List {
        path: String,
        show_metadata: bool,
        show_tags: bool,
        sort_by: Option<String>, // "name", "size", "modified", "relevance"
    },
    /// Tag files with semantic labels
    Tag {
        file_path: String,
        tags: Vec<String>,
        auto_suggest: bool,
    },
    /// Show file operation status and statistics
    Status {
        operation: Option<String>, // "indexing", "classification", "analysis"
    },
    /// Edit file using multi-strategy matching (Phase 1 integration)
    Edit {
        file_path: String,
        search: String,
        replace: String,
        strategy: Option<String>, // "exact", "fuzzy", "block-anchor", "auto"
    },
    /// Validate edit without applying (dry-run)
    ValidateEdit {
        file_path: String,
        search: String,
        replace: String,
    },
    /// Show diff for last/proposed edit
    Diff { file_path: Option<String> },
    /// Undo last file operation
    Undo { steps: Option<usize> },
}

/// Context management subcommands for RAG workflow
#[derive(Debug, Clone, PartialEq)]
#[cfg(feature = "repl-chat")]
pub enum ContextSubcommand {
    /// Add documents from last search results by indices (e.g., "1,2,3" or "1-5")
    Add { indices: String },
    /// List all context items in current conversation
    List,
    /// Clear all context from current conversation
    Clear,
    /// Remove a specific context item by index
    Remove { index: usize },
}

/// Conversation management subcommands for RAG workflow
#[derive(Debug, Clone, PartialEq)]
#[cfg(feature = "repl-chat")]
pub enum ConversationSubcommand {
    /// Create a new conversation
    New { title: Option<String> },
    /// Load an existing conversation by ID
    Load { id: String },
    /// List all conversations
    List { limit: Option<usize> },
    /// Show current conversation details
    Show,
    /// Delete a conversation by ID
    Delete { id: String },
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
                let mut semantic = false;
                let mut concepts = false;
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
                        "--semantic" => {
                            semantic = true;
                            i += 1;
                        }
                        "--concepts" => {
                            concepts = true;
                            i += 1;
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

            "web" => {
                if parts.len() < 2 {
                    return Err(anyhow!("Web command requires a subcommand. Use: /web <get|post|scrape|screenshot|pdf|form|api|status|cancel|history|config>"));
                }

                match parts[1] {
                    "get" => {
                        if parts.len() < 3 {
                            return Err(anyhow!("Web GET requires URL. Use: /web get <url> [--headers <json>]"));
                        }
                        let url = parts[2].to_string();
                        let mut headers = None;
                        let mut i = 3;
                        while i < parts.len() {
                            match parts[i] {
                                "--headers" => {
                                    if i + 1 < parts.len() {
                                        headers = Some(serde_json::from_str(parts[i + 1])?);
                                        i += 2;
                                    } else {
                                        return Err(anyhow!("--headers requires JSON argument"));
                                    }
                                }
                                _ => return Err(anyhow!("Unknown option: {}", parts[i])),
                            }
                            i += 1;
                        }
                        Ok(ReplCommand::Web { subcommand: WebSubcommand::Get { url, headers } })
                    }
                    "post" => {
                        if parts.len() < 4 {
                            return Err(anyhow!("Web POST requires URL and body. Use: /web post <url> <body> [--headers <json>]"));
                        }
                        let url = parts[2].to_string();
                        let body = parts[3].to_string();
                        let mut headers = None;
                        let mut i = 4;
                        while i < parts.len() {
                            match parts[i] {
                                "--headers" => {
                                    if i + 1 < parts.len() {
                                        headers = Some(serde_json::from_str(parts[i + 1])?);
                                        i += 2;
                                    } else {
                                        return Err(anyhow!("--headers requires JSON argument"));
                                    }
                                }
                                _ => return Err(anyhow!("Unknown option: {}", parts[i])),
                            }
                            i += 1;
                        }
                        Ok(ReplCommand::Web { subcommand: WebSubcommand::Post { url, body, headers } })
                    }
                    "scrape" => {
                        if parts.len() < 4 {
                            return Err(anyhow!("Web scrape requires URL and selector. Use: /web scrape <url> <selector> [--wait <element>]"));
                        }
                        let url = parts[2].to_string();
                        let selector = parts[3].to_string();
                        let mut wait_for_element = None;
                        let mut i = 4;
                        while i < parts.len() {
                            match parts[i] {
                                "--wait" => {
                                    if i + 1 < parts.len() {
                                        wait_for_element = Some(parts[i + 1].to_string());
                                        i += 2;
                                    } else {
                                        return Err(anyhow!("--wait requires element argument"));
                                    }
                                }
                                _ => return Err(anyhow!("Unknown option: {}", parts[i])),
                            }
                            i += 1;
                        }
                        Ok(ReplCommand::Web { subcommand: WebSubcommand::Scrape { url, selector, wait: wait_for_element } })
                    }
                    "screenshot" => {
                        if parts.len() < 3 {
                            return Err(anyhow!("Web screenshot requires URL. Use: /web screenshot <url> [--width <px>] [--height <px>] [--full-page]"));
                        }
                        let url = parts[2].to_string();
                        let mut width = None;
                        let mut height = None;
                        let mut full_page = None;
                        let mut i = 3;
                        while i < parts.len() {
                            match parts[i] {
                                "--width" => {
                                    if i + 1 < parts.len() {
                                        width = Some(parts[i + 1].parse()?);
                                        i += 2;
                                    } else {
                                        return Err(anyhow!("--width requires pixel value"));
                                    }
                                }
                                "--height" => {
                                    if i + 1 < parts.len() {
                                        height = Some(parts[i + 1].parse()?);
                                        i += 2;
                                    } else {
                                        return Err(anyhow!("--height requires pixel value"));
                                    }
                                }
                                "--full-page" => {
                                    full_page = Some(true);
                                    i += 1;
                                }
                                _ => return Err(anyhow!("Unknown option: {}", parts[i])),
                            }
                            i += 1;
                        }
                        Ok(ReplCommand::Web { subcommand: WebSubcommand::Screenshot { url, width, height, full_page } })
                    }
                    "pdf" => {
                        if parts.len() < 3 {
                            return Err(anyhow!("Web PDF requires URL. Use: /web pdf <url> [--page-size <size>]"));
                        }
                        let url = parts[2].to_string();
                        let mut page_size = None;
                        let mut i = 3;
                        while i < parts.len() {
                            match parts[i] {
                                "--page-size" => {
                                    if i + 1 < parts.len() {
                                        page_size = Some(parts[i + 1].to_string());
                                        i += 2;
                                    } else {
                                        return Err(anyhow!("--page-size requires size argument"));
                                    }
                                }
                                _ => return Err(anyhow!("Unknown option: {}", parts[i])),
                            }
                            i += 1;
                        }
                        Ok(ReplCommand::Web { subcommand: WebSubcommand::Pdf { url, page_size } })
                    }
                    "form" => {
                        if parts.len() < 3 {
                            return Err(anyhow!("Web form requires URL and form data JSON. Use: /web form <url> <json_data>"));
                        }
                        let url = parts[2].to_string();
                        let form_data: std::collections::HashMap<String, String> = serde_json::from_str(parts[3])?;
                        Ok(ReplCommand::Web { subcommand: WebSubcommand::Form { url, form_data } })
                    }
                    "api" => {
                        if parts.len() < 4 {
                            return Err(anyhow!("Web API requires base URL and endpoints. Use: /web api <base_url> <endpoint1,endpoint2,...> [--rate-limit <ms>]"));
                        }
                        let base_url = parts[2].to_string();
                        let endpoints: Vec<String> = parts[3].split(',').map(|s| s.trim().to_string()).collect();
                        let mut rate_limit_ms = None;
                        let mut i = 4;
                        while i < parts.len() {
                            match parts[i] {
                                "--rate-limit" => {
                                    if i + 1 < parts.len() {
                                        rate_limit_ms = Some(parts[i + 1].parse()?);
                                        i += 2;
                                    } else {
                                        return Err(anyhow!("--rate-limit requires millisecond value"));
                                    }
                                }
                                _ => return Err(anyhow!("Unknown option: {}", parts[i])),
                            }
                            i += 1;
                        }
                        Ok(ReplCommand::Web { subcommand: WebSubcommand::Api { base_url, endpoints, rate_limit: rate_limit_ms } })
                    }
                    "status" => {
                        if parts.len() < 3 {
                            return Err(anyhow!("Web status requires operation ID. Use: /web status <operation_id>"));
                        }
                        let operation_id = parts[2].to_string();
                        Ok(ReplCommand::Web { subcommand: WebSubcommand::Status { operation_id: Some(operation_id) } })
                    }
                    "cancel" => {
                        if parts.len() < 3 {
                            return Err(anyhow!("Web cancel requires operation ID. Use: /web cancel <operation_id>"));
                        }
                        let operation_id = parts[2].to_string();
                        Ok(ReplCommand::Web { subcommand: WebSubcommand::Cancel { operation_id } })
                    }
                    "history" => {
                        let mut limit = None;
                        let mut i = 2;
                        while i < parts.len() {
                            match parts[i] {
                                "--limit" => {
                                    if i + 1 < parts.len() {
                                        limit = Some(parts[i + 1].parse()?);
                                        i += 2;
                                    } else {
                                        return Err(anyhow!("--limit requires numeric value"));
                                    }
                                }
                                _ => return Err(anyhow!("Unknown option: {}", parts[i])),
                            }
                            i += 1;
                        }
                        Ok(ReplCommand::Web { subcommand: WebSubcommand::History { limit } })
                    }
                    "config" => {
                        if parts.len() < 3 {
                            return Err(anyhow!("Web config requires subcommand. Use: /web config <show|set|reset> [args...]"));
                        }
                        match parts[2] {
                            "show" => Ok(ReplCommand::Web { subcommand: WebSubcommand::Config { subcommand: WebConfigSubcommand::Show } }),
                            "set" => {
                                if parts.len() < 5 {
                                    return Err(anyhow!("Web config set requires key and value. Use: /web config set <key> <value>"));
                                }
                                let key = parts[3].to_string();
                                let value = parts[4].to_string();
                                Ok(ReplCommand::Web { subcommand: WebSubcommand::Config { subcommand: WebConfigSubcommand::Set { key, value } } })
                            }
                            "reset" => Ok(ReplCommand::Web { subcommand: WebSubcommand::Config { subcommand: WebConfigSubcommand::Reset } }),
                            _ => Err(anyhow!("Unknown web config subcommand: {}", parts[2])),
                        }
                    }
                    _ => Err(anyhow!("Unknown web subcommand: {}. Use: get, post, scrape, screenshot, pdf, form, api, status, cancel, history, config", parts[1])),
                }
            }

            #[cfg(feature = "repl-file")]
            "file" => {
                if parts.len() < 2 {
                    return Err(anyhow!("File command requires a subcommand. Use: /file <search|classify|suggest|analyze|summarize|metadata|index|find|list|tag|status|edit|validate-edit|diff|undo>"));
                }

                match parts[1] {
                    "search" => {
                        if parts.len() < 3 {
                            return Err(anyhow!("File search requires query. Use: /file search <query> [--path <path>] [--types <ext1,ext2>] [--semantic] [--limit <n>]"));
                        }
                        let query = parts[2].to_string();
                        let mut path = None;
                        let mut file_types = None;
                        let mut semantic = false;
                        let mut limit = None;
                        let mut i = 3;
                        while i < parts.len() {
                            match parts[i] {
                                "--path" => {
                                    if i + 1 < parts.len() {
                                        path = Some(parts[i + 1].to_string());
                                        i += 2;
                                    } else {
                                        return Err(anyhow!("--path requires directory path"));
                                    }
                                }
                                "--types" => {
                                    if i + 1 < parts.len() {
                                        file_types = Some(parts[i + 1].split(',').map(|s| s.trim().to_string()).collect());
                                        i += 2;
                                    } else {
                                        return Err(anyhow!("--types requires comma-separated file extensions"));
                                    }
                                }
                                "--semantic" => {
                                    semantic = true;
                                    i += 1;
                                }
                                "--limit" => {
                                    if i + 1 < parts.len() {
                                        limit = Some(parts[i + 1].parse()?);
                                        i += 2;
                                    } else {
                                        return Err(anyhow!("--limit requires numeric value"));
                                    }
                                }
                                _ => return Err(anyhow!("Unknown option: {}", parts[i])),
                            }
                            i += 1;
                        }
                        Ok(ReplCommand::File { subcommand: FileSubcommand::Search { query, path, file_types, semantic, limit } })
                    }
                    "classify" => {
                        if parts.len() < 3 {
                            return Err(anyhow!("File classify requires path. Use: /file classify <path> [--recursive] [--update]"));
                        }
                        let path = parts[2].to_string();
                        let mut recursive = false;
                        let mut update_metadata = false;
                        let mut i = 3;
                        while i < parts.len() {
                            match parts[i] {
                                "--recursive" => {
                                    recursive = true;
                                    i += 1;
                                }
                                "--update" => {
                                    update_metadata = true;
                                    i += 1;
                                }
                                _ => return Err(anyhow!("Unknown option: {}", parts[i])),
                            }
                            i += 1;
                        }
                        Ok(ReplCommand::File { subcommand: FileSubcommand::Classify { path, recursive, update_metadata } })
                    }
                    "suggest" => {
                        let mut context = None;
                        let mut limit = None;
                        let mut path = None;
                        let mut i = 2;
                        while i < parts.len() {
                            match parts[i] {
                                "--context" => {
                                    if i + 1 < parts.len() {
                                        context = Some(parts[i + 1].to_string());
                                        i += 2;
                                    } else {
                                        return Err(anyhow!("--context requires description"));
                                    }
                                }
                                "--limit" => {
                                    if i + 1 < parts.len() {
                                        limit = Some(parts[i + 1].parse()?);
                                        i += 2;
                                    } else {
                                        return Err(anyhow!("--limit requires numeric value"));
                                    }
                                }
                                "--path" => {
                                    if i + 1 < parts.len() {
                                        path = Some(parts[i + 1].to_string());
                                        i += 2;
                                    } else {
                                        return Err(anyhow!("--path requires directory path"));
                                    }
                                }
                                _ => return Err(anyhow!("Unknown option: {}", parts[i])),
                            }
                            i += 1;
                        }
                        Ok(ReplCommand::File { subcommand: FileSubcommand::Suggest { context, limit, path } })
                    }
                    "analyze" => {
                        if parts.len() < 3 {
                            return Err(anyhow!("File analyze requires file path. Use: /file analyze <file> [--similar] [--related] [--threshold <value>]"));
                        }
                        let file_path = parts[2].to_string();
                        let mut find_similar = false;
                        let mut find_related = false;
                        let mut threshold = None;
                        let mut i = 3;
                        while i < parts.len() {
                            match parts[i] {
                                "--similar" => {
                                    find_similar = true;
                                    i += 1;
                                }
                                "--related" => {
                                    find_related = true;
                                    i += 1;
                                }
                                "--threshold" => {
                                    if i + 1 < parts.len() {
                                        threshold = Some(parts[i + 1].parse()?);
                                        i += 2;
                                    } else {
                                        return Err(anyhow!("--threshold requires numeric value"));
                                    }
                                }
                                _ => return Err(anyhow!("Unknown option: {}", parts[i])),
                            }
                            i += 1;
                        }
                        Ok(ReplCommand::File { subcommand: FileSubcommand::Analyze { file_path, find_similar, find_related, threshold } })
                    }
                    "summarize" => {
                        if parts.len() < 3 {
                            return Err(anyhow!("File summarize requires file path. Use: /file summarize <file> [--level <brief|detailed|comprehensive>] [--key-points]"));
                        }
                        let file_path = parts[2].to_string();
                        let mut detail_level = None;
                        let mut include_key_points = false;
                        let mut i = 3;
                        while i < parts.len() {
                            match parts[i] {
                                "--level" => {
                                    if i + 1 < parts.len() {
                                        let level = parts[i + 1].to_string();
                                        if matches!(level.as_str(), "brief" | "detailed" | "comprehensive") {
                                            detail_level = Some(level);
                                        } else {
                                            return Err(anyhow!("Level must be one of: brief, detailed, comprehensive"));
                                        }
                                        i += 2;
                                    } else {
                                        return Err(anyhow!("--level requires level specification"));
                                    }
                                }
                                "--key-points" => {
                                    include_key_points = true;
                                    i += 1;
                                }
                                _ => return Err(anyhow!("Unknown option: {}", parts[i])),
                            }
                            i += 1;
                        }
                        Ok(ReplCommand::File { subcommand: FileSubcommand::Summarize { file_path, detail_level, include_key_points } })
                    }
                    "metadata" => {
                        if parts.len() < 3 {
                            return Err(anyhow!("File metadata requires file path. Use: /file metadata <file> [--concepts] [--entities] [--keywords] [--update]"));
                        }
                        let file_path = parts[2].to_string();
                        let mut extract_concepts = false;
                        let mut extract_entities = false;
                        let mut extract_keywords = false;
                        let mut update_index = false;
                        let mut i = 3;
                        while i < parts.len() {
                            match parts[i] {
                                "--concepts" => {
                                    extract_concepts = true;
                                    i += 1;
                                }
                                "--entities" => {
                                    extract_entities = true;
                                    i += 1;
                                }
                                "--keywords" => {
                                    extract_keywords = true;
                                    i += 1;
                                }
                                "--update" => {
                                    update_index = true;
                                    i += 1;
                                }
                                _ => return Err(anyhow!("Unknown option: {}", parts[i])),
                            }
                            i += 1;
                        }
                        Ok(ReplCommand::File { subcommand: FileSubcommand::Metadata { file_path, extract_concepts, extract_entities, extract_keywords, update_index } })
                    }
                    "index" => {
                        if parts.len() < 3 {
                            return Err(anyhow!("File index requires path. Use: /file index <path> [--recursive] [--force]"));
                        }
                        let path = parts[2].to_string();
                        let mut recursive = false;
                        let mut force_reindex = false;
                        let mut i = 3;
                        while i < parts.len() {
                            match parts[i] {
                                "--recursive" => {
                                    recursive = true;
                                    i += 1;
                                }
                                "--force" => {
                                    force_reindex = true;
                                    i += 1;
                                }
                                _ => return Err(anyhow!("Unknown option: {}", parts[i])),
                            }
                            i += 1;
                        }
                        Ok(ReplCommand::File { subcommand: FileSubcommand::Index { path, recursive, force_reindex } })
                    }
                    "find" => {
                        if parts.len() < 3 {
                            return Err(anyhow!("File find requires pattern. Use: /file find <pattern> [--path <path>] [--context <n>] [--case-sensitive] [--whole-word]"));
                        }
                        let pattern = parts[2].to_string();
                        let mut path = None;
                        let mut context_lines = None;
                        let mut case_sensitive = false;
                        let mut whole_word = false;
                        let mut i = 3;
                        while i < parts.len() {
                            match parts[i] {
                                "--path" => {
                                    if i + 1 < parts.len() {
                                        path = Some(parts[i + 1].to_string());
                                        i += 2;
                                    } else {
                                        return Err(anyhow!("--path requires directory path"));
                                    }
                                }
                                "--context" => {
                                    if i + 1 < parts.len() {
                                        context_lines = Some(parts[i + 1].parse()?);
                                        i += 2;
                                    } else {
                                        return Err(anyhow!("--context requires numeric value"));
                                    }
                                }
                                "--case-sensitive" => {
                                    case_sensitive = true;
                                    i += 1;
                                }
                                "--whole-word" => {
                                    whole_word = true;
                                    i += 1;
                                }
                                _ => return Err(anyhow!("Unknown option: {}", parts[i])),
                            }
                            i += 1;
                        }
                        Ok(ReplCommand::File { subcommand: FileSubcommand::Find { pattern, path, context_lines, case_sensitive, whole_word } })
                    }
                    "list" => {
                        if parts.len() < 3 {
                            return Err(anyhow!("File list requires path. Use: /file list <path> [--metadata] [--tags] [--sort <field>]"));
                        }
                        let path = parts[2].to_string();
                        let mut show_metadata = false;
                        let mut show_tags = false;
                        let mut sort_by = None;
                        let mut i = 3;
                        while i < parts.len() {
                            match parts[i] {
                                "--metadata" => {
                                    show_metadata = true;
                                    i += 1;
                                }
                                "--tags" => {
                                    show_tags = true;
                                    i += 1;
                                }
                                "--sort" => {
                                    if i + 1 < parts.len() {
                                        let field = parts[i + 1].to_string();
                                        if matches!(field.as_str(), "name" | "size" | "modified" | "relevance") {
                                            sort_by = Some(field);
                                        } else {
                                            return Err(anyhow!("Sort field must be one of: name, size, modified, relevance"));
                                        }
                                        i += 2;
                                    } else {
                                        return Err(anyhow!("--sort requires field name"));
                                    }
                                }
                                _ => return Err(anyhow!("Unknown option: {}", parts[i])),
                            }
                            i += 1;
                        }
                        Ok(ReplCommand::File { subcommand: FileSubcommand::List { path, show_metadata, show_tags, sort_by } })
                    }
                    "tag" => {
                        if parts.len() < 4 {
                            return Err(anyhow!("File tag requires file path and tags. Use: /file tag <file> <tag1,tag2,...> [--suggest]"));
                        }
                        let file_path = parts[2].to_string();
                        let tags: Vec<String> = parts[3].split(',').map(|s| s.trim().to_string()).collect();
                        let mut auto_suggest = false;
                        let mut i = 4;
                        while i < parts.len() {
                            match parts[i] {
                                "--suggest" => {
                                    auto_suggest = true;
                                    i += 1;
                                }
                                _ => return Err(anyhow!("Unknown option: {}", parts[i])),
                            }
                            i += 1;
                        }
                        Ok(ReplCommand::File { subcommand: FileSubcommand::Tag { file_path, tags, auto_suggest } })
                    }
                    "status" => {
                        let mut operation = None;
                        let mut i = 2;
                        while i < parts.len() {
                            match parts[i] {
                                "--operation" => {
                                    if i + 1 < parts.len() {
                                        let op = parts[i + 1].to_string();
                                        if matches!(op.as_str(), "indexing" | "classification" | "analysis") {
                                            operation = Some(op);
                                        } else {
                                            return Err(anyhow!("Operation must be one of: indexing, classification, analysis"));
                                        }
                                        i += 2;
                                    } else {
                                        return Err(anyhow!("--operation requires operation name"));
                                    }
                                }
                                _ => return Err(anyhow!("Unknown option: {}", parts[i])),
                            }
                            i += 1;
                        }
                        Ok(ReplCommand::File { subcommand: FileSubcommand::Status { operation } })
                    }
                    "edit" => {
                        if parts.len() < 5 {
                            return Err(anyhow!("File edit requires file path, search, and replace. Use: /file edit <path> \"<search>\" \"<replace>\" [--strategy <auto|exact|fuzzy|block-anchor>]"));
                        }
                        let file_path = parts[2].to_string();
                        let search = parts[3].to_string();
                        let replace = parts[4].to_string();
                        let mut strategy = None;
                        let mut i = 5;
                        while i < parts.len() {
                            match parts[i] {
                                "--strategy" => {
                                    if i + 1 < parts.len() {
                                        strategy = Some(parts[i + 1].to_string());
                                        i += 2;
                                    } else {
                                        return Err(anyhow!("--strategy requires value: auto, exact, fuzzy, or block-anchor"));
                                    }
                                }
                                _ => return Err(anyhow!("Unknown option: {}", parts[i])),
                            }
                            i += 1;
                        }
                        Ok(ReplCommand::File { subcommand: FileSubcommand::Edit { file_path, search, replace, strategy } })
                    }
                    "validate-edit" => {
                        if parts.len() < 5 {
                            return Err(anyhow!("File validate-edit requires file path, search, and replace. Use: /file validate-edit <path> \"<search>\" \"<replace>\""));
                        }
                        let file_path = parts[2].to_string();
                        let search = parts[3].to_string();
                        let replace = parts[4].to_string();
                        Ok(ReplCommand::File { subcommand: FileSubcommand::ValidateEdit { file_path, search, replace } })
                    }
                    "diff" => {
                        let file_path = if parts.len() >= 3 {
                            Some(parts[2].to_string())
                        } else {
                            None
                        };
                        Ok(ReplCommand::File { subcommand: FileSubcommand::Diff { file_path } })
                    }
                    "undo" => {
                        let steps = if parts.len() >= 3 {
                            Some(parts[2].parse()?)
                        } else {
                            None
                        };
                        Ok(ReplCommand::File { subcommand: FileSubcommand::Undo { steps } })
                    }
                    _ => Err(anyhow!("Unknown file subcommand: {}. Use: search, classify, suggest, analyze, summarize, metadata, index, find, list, tag, status, edit, validate-edit, diff, undo", parts[1])),
                }
            }

            #[cfg(feature = "repl-custom")]
            "commands" => {
                if parts.len() < 2 {
                    return Err(anyhow!("Commands command requires a subcommand. Use: init | list | category <name> | help <command> | search <query> | reload | validate [command] | stats | suggest <partial> [--limit <n>]"));
                }

                match parts[1] {
                    "init" => Ok(ReplCommand::Commands {
                        subcommand: CommandsSubcommand::Init,
                    }),
                    "list" => Ok(ReplCommand::Commands {
                        subcommand: CommandsSubcommand::List,
                    }),
                    "category" => {
                        if parts.len() < 3 {
                            return Err(anyhow!("Commands category requires a category name"));
                        }
                        Ok(ReplCommand::Commands {
                            subcommand: CommandsSubcommand::Category {
                                category: parts[2].to_string(),
                            },
                        })
                    }
                    "help" => {
                        if parts.len() < 3 {
                            return Err(anyhow!("Commands help requires a command name"));
                        }
                        Ok(ReplCommand::Commands {
                            subcommand: CommandsSubcommand::Help {
                                command: parts[2].to_string(),
                            },
                        })
                    }
                    "search" => {
                        if parts.len() < 3 {
                            return Err(anyhow!("Commands search requires a query"));
                        }
                        Ok(ReplCommand::Commands {
                            subcommand: CommandsSubcommand::Search {
                                query: parts[2..].join(" "),
                            },
                        })
                    }
                    "reload" => Ok(ReplCommand::Commands {
                        subcommand: CommandsSubcommand::Reload,
                    }),
                    "validate" => {
                        let command = if parts.len() > 2 {
                            Some(parts[2].to_string())
                        } else {
                            None
                        };
                        Ok(ReplCommand::Commands {
                            subcommand: CommandsSubcommand::Validate { command },
                        })
                    }
                    "stats" => Ok(ReplCommand::Commands {
                        subcommand: CommandsSubcommand::Stats,
                    }),
                    "suggest" => {
                        if parts.len() < 3 {
                            return Err(anyhow!("Commands suggest requires a partial command name"));
                        }
                        let mut partial = parts[2].to_string();
                        let mut limit = None;
                        let mut i = 3;

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
                                        return Err(anyhow!("--limit requires a numeric value"));
                                    }
                                }
                                _ => {
                                    if !partial.is_empty() {
                                        partial.push(' ');
                                    }
                                    partial.push_str(parts[i]);
                                    i += 1;
                                }
                            }
                        }

                        Ok(ReplCommand::Commands {
                            subcommand: CommandsSubcommand::Suggest { partial, limit },
                        })
                    }
                    _ => Err(anyhow!("Unknown commands subcommand: {}. Use: init, list, category, help, search, reload, validate, stats, suggest", parts[1])),
                }
            }

            #[cfg(feature = "repl-chat")]
            "context" => {
                if parts.len() < 2 {
                    return Err(anyhow!("Context command requires a subcommand. Use: /context <add|list|clear|remove>"));
                }

                match parts[1] {
                    "add" => {
                        if parts.len() < 3 {
                            return Err(anyhow!(
                                "Context add requires indices. Use: /context add <indices>"
                            ));
                        }
                        Ok(ReplCommand::Context {
                            subcommand: ContextSubcommand::Add {
                                indices: parts[2].to_string(),
                            },
                        })
                    }
                    "list" => Ok(ReplCommand::Context {
                        subcommand: ContextSubcommand::List,
                    }),
                    "clear" => Ok(ReplCommand::Context {
                        subcommand: ContextSubcommand::Clear,
                    }),
                    "remove" => {
                        if parts.len() < 3 {
                            return Err(anyhow!(
                                "Context remove requires an index. Use: /context remove <index>"
                            ));
                        }
                        let index = parts[2]
                            .parse::<usize>()
                            .map_err(|_| anyhow!("Invalid index value"))?;
                        Ok(ReplCommand::Context {
                            subcommand: ContextSubcommand::Remove { index },
                        })
                    }
                    _ => Err(anyhow!(
                        "Unknown context subcommand: {}. Use: add, list, clear, remove",
                        parts[1]
                    )),
                }
            }

            #[cfg(feature = "repl-chat")]
            "conversation" => {
                if parts.len() < 2 {
                    return Err(anyhow!("Conversation command requires a subcommand. Use: /conversation <new|load|list|show|delete>"));
                }

                match parts[1] {
                    "new" => {
                        let title = if parts.len() > 2 {
                            Some(parts[2..].join(" "))
                        } else {
                            None
                        };
                        Ok(ReplCommand::Conversation {
                            subcommand: ConversationSubcommand::New { title },
                        })
                    }
                    "load" => {
                        if parts.len() < 3 {
                            return Err(anyhow!(
                                "Conversation load requires an ID. Use: /conversation load <id>"
                            ));
                        }
                        Ok(ReplCommand::Conversation {
                            subcommand: ConversationSubcommand::Load {
                                id: parts[2].to_string(),
                            },
                        })
                    }
                    "list" => {
                        let mut limit = None;
                        if parts.len() > 2 && parts[2] == "--limit" {
                            if parts.len() > 3 {
                                limit = Some(
                                    parts[3]
                                        .parse::<usize>()
                                        .map_err(|_| anyhow!("Invalid limit value"))?,
                                );
                            }
                        }
                        Ok(ReplCommand::Conversation {
                            subcommand: ConversationSubcommand::List { limit },
                        })
                    }
                    "show" => Ok(ReplCommand::Conversation {
                        subcommand: ConversationSubcommand::Show,
                    }),
                    "delete" => {
                        if parts.len() < 3 {
                            return Err(anyhow!("Conversation delete requires an ID. Use: /conversation delete <id>"));
                        }
                        Ok(ReplCommand::Conversation {
                            subcommand: ConversationSubcommand::Delete {
                                id: parts[2].to_string(),
                            },
                        })
                    }
                    _ => Err(anyhow!(
                        "Unknown conversation subcommand: {}. Use: new, load, list, show, delete",
                        parts[1]
                    )),
                }
            }

            _ => Err(anyhow!("Unknown command: {}", parts[0])),
        }
    }
}

impl ReplCommand {
    /// Get available commands based on compiled features
    pub fn available_commands() -> Vec<&'static str> {
        let mut commands = vec![
            "search", "config", "role", "graph", "web", "help", "quit", "exit", "clear",
        ];

        #[cfg(feature = "repl-chat")]
        {
            commands.extend_from_slice(&["chat", "summarize", "context", "conversation"]);
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

        #[cfg(feature = "repl-custom")]
        {
            commands.extend_from_slice(&["commands"]);
        }

        commands
    }

    /// Get command description for help system
    pub fn get_command_help(command: &str) -> Option<&'static str> {
        match command {
            "search" => Some("/search <query> [--role <role>] [--limit <n>] [--semantic] [--concepts] - Search documents with semantic options"),
            "config" => Some("/config show | set <key> <value> - Manage configuration"),
            "role" => Some("/role list | select <name> - Manage roles"),
            "graph" => Some("/graph [--top-k <n>] - Show knowledge graph"),
            "web" => Some("/web get <url> | post <url> <body> | scrape <url> <selector> | screenshot <url> | pdf <url> | form <url> <json> | api <base_url> <endpoints> | status <id> | cancel <id> | history | config <show|set|reset> - Web operations through VM sandboxing"),
            "file" => Some("/file search <query> | classify <path> | suggest [--context <desc>] | analyze <file> | summarize <file> | metadata <file> | index <path> | find <pattern> | list <path> | tag <file> <tags> | status - Enhanced file operations with semantic awareness"),
            "help" => Some("/help [command] - Show help information"),
            "quit" | "q" => Some("/quit, /q - Exit REPL"),
            "exit" => Some("/exit - Exit REPL"),
            "clear" => Some("/clear - Clear screen"),

            #[cfg(feature = "repl-chat")]
            "chat" => Some("/chat [message] - Interactive chat with AI"),
            #[cfg(feature = "repl-chat")]
            "summarize" => Some("/summarize <doc-id|text> - Summarize content"),
            #[cfg(feature = "repl-chat")]
            "context" => Some("/context add <indices> | list | clear | remove <index> - Manage conversation context for RAG"),
            #[cfg(feature = "repl-chat")]
            "conversation" => Some("/conversation new [title] | load <id> | list [--limit <n>] | show | delete <id> - Manage chat conversations"),

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

            #[cfg(feature = "repl-custom")]
            "commands" => Some("/commands list | category <name> | help <cmd> | search <query> | reload | validate [cmd] | stats | suggest <partial> [--limit <n>] - Manage custom markdown-defined commands"),

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
                concepts: false,
                semantic: false
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
                concepts: false,
                semantic: false
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

    #[cfg(feature = "repl-file")]
    #[test]
    fn test_file_edit_command_parsing() {
        let cmd = "/file edit test.rs old_code new_code"
            .parse::<ReplCommand>()
            .unwrap();

        if let ReplCommand::File {
            subcommand:
                FileSubcommand::Edit {
                    file_path,
                    search,
                    replace,
                    strategy,
                },
        } = cmd
        {
            assert_eq!(file_path, "test.rs");
            assert_eq!(search, "old_code");
            assert_eq!(replace, "new_code");
            assert_eq!(strategy, None);
        } else {
            panic!("Failed to parse edit command");
        }
    }

    #[cfg(feature = "repl-file")]
    #[test]
    fn test_file_edit_with_strategy() {
        let cmd = "/file edit test.rs old new --strategy fuzzy"
            .parse::<ReplCommand>()
            .unwrap();

        if let ReplCommand::File {
            subcommand: FileSubcommand::Edit { strategy, .. },
        } = cmd
        {
            assert_eq!(strategy, Some("fuzzy".to_string()));
        } else {
            panic!("Failed to parse edit command with strategy");
        }
    }

    #[cfg(feature = "repl-file")]
    #[test]
    fn test_file_validate_edit_command() {
        let cmd = "/file validate-edit test.rs search replace"
            .parse::<ReplCommand>()
            .unwrap();

        if let ReplCommand::File {
            subcommand:
                FileSubcommand::ValidateEdit {
                    file_path,
                    search,
                    replace,
                },
        } = cmd
        {
            assert_eq!(file_path, "test.rs");
            assert_eq!(search, "search");
            assert_eq!(replace, "replace");
        } else {
            panic!("Failed to parse validate-edit command");
        }
    }

    #[cfg(feature = "repl-file")]
    #[test]
    fn test_file_diff_command() {
        // With file path
        let cmd = "/file diff test.rs".parse::<ReplCommand>().unwrap();
        if let ReplCommand::File {
            subcommand: FileSubcommand::Diff { file_path },
        } = cmd
        {
            assert_eq!(file_path, Some("test.rs".to_string()));
        } else {
            panic!("Failed to parse diff command with file");
        }

        // Without file path
        let cmd = "/file diff".parse::<ReplCommand>().unwrap();
        if let ReplCommand::File {
            subcommand: FileSubcommand::Diff { file_path },
        } = cmd
        {
            assert_eq!(file_path, None);
        } else {
            panic!("Failed to parse diff command without file");
        }
    }

    #[cfg(feature = "repl-file")]
    #[test]
    fn test_file_undo_command() {
        // With steps
        let cmd = "/file undo 2".parse::<ReplCommand>().unwrap();
        if let ReplCommand::File {
            subcommand: FileSubcommand::Undo { steps },
        } = cmd
        {
            assert_eq!(steps, Some(2));
        } else {
            panic!("Failed to parse undo command with steps");
        }

        // Without steps (default to 1)
        let cmd = "/file undo".parse::<ReplCommand>().unwrap();
        if let ReplCommand::File {
            subcommand: FileSubcommand::Undo { steps },
        } = cmd
        {
            assert_eq!(steps, None);
        } else {
            panic!("Failed to parse undo command without steps");
        }
    }

    #[cfg(feature = "repl-file")]
    #[test]
    fn test_file_edit_missing_args_error() {
        let result = "/file edit test.rs".parse::<ReplCommand>();
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("requires file path, search, and replace"));
    }
}
