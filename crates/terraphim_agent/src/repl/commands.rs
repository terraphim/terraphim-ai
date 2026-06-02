//! Command definitions for REPL interface

use anyhow::{Result, anyhow};
use std::str::FromStr;

/// All commands that can be issued in the Terraphim REPL.
#[derive(Debug, Clone, PartialEq)]
pub enum ReplCommand {
    // Base commands (always available with 'repl' feature)
    /// Search the configured haystacks using the knowledge graph.
    Search {
        /// The search query string.
        query: String,
        /// Optional role override; uses the currently selected role if absent.
        role: Option<String>,
        /// Maximum number of results to return.
        limit: Option<usize>,
        /// Enable semantic (embedding-based) search.
        semantic: bool,
        /// Include knowledge-graph concept matches in results.
        concepts: bool,
    },
    /// View or modify the current configuration.
    Config {
        /// Configuration subcommand to execute.
        subcommand: ConfigSubcommand,
    },
    /// List or switch between roles.
    Role {
        /// Role subcommand to execute.
        subcommand: RoleSubcommand,
    },
    /// Display the top concepts from the knowledge graph.
    Graph {
        /// Number of top concepts to display.
        top_k: Option<usize>,
    },

    // Chat commands (requires 'llm' feature)
    /// Send a message to the configured LLM.
    #[cfg(feature = "llm")]
    Chat {
        /// Message to send; starts interactive session if absent.
        message: Option<String>,
    },

    /// Summarise a document or piece of text using the LLM.
    #[cfg(feature = "llm")]
    Summarize {
        /// Document ID or raw text to summarise.
        target: String,
    },

    // MCP commands (requires 'repl-mcp' feature)
    /// Autocomplete a partial term against the thesaurus.
    #[cfg(feature = "repl-mcp")]
    Autocomplete {
        /// Partial query string to complete.
        query: String,
        /// Maximum number of suggestions to return.
        limit: Option<usize>,
    },

    /// Extract paragraphs that contain thesaurus-matched terms.
    #[cfg(feature = "repl-mcp")]
    Extract {
        /// Input text to extract paragraphs from.
        text: String,
        /// When true, omit the matched term from each extracted paragraph.
        exclude_term: bool,
    },

    /// Find all thesaurus-term matches within the given text.
    #[cfg(feature = "repl-mcp")]
    Find {
        /// Input text to search for matches.
        text: String,
    },

    /// Replace thesaurus-matched terms in text with hyperlinks.
    #[cfg(feature = "repl-mcp")]
    Replace {
        /// Input text whose matched terms will be replaced.
        text: String,
        /// Link format: `"markdown"`, `"wiki"`, `"html"`, or `"plain"`.
        format: Option<String>,
    },

    /// Display thesaurus entries for the selected role.
    #[cfg(feature = "repl-mcp")]
    Thesaurus {
        /// Role whose thesaurus to display; defaults to the current role.
        role: Option<String>,
    },

    // File commands (requires 'repl-file' feature)
    /// Perform file system operations.
    #[cfg(feature = "repl-file")]
    File {
        /// File subcommand to execute.
        subcommand: FileSubcommand,
    },

    // Web commands (requires 'repl-web' feature)
    /// Perform web operations (HTTP requests, scraping, screenshots).
    #[cfg(feature = "repl-web")]
    Web {
        /// Web subcommand to execute.
        subcommand: WebSubcommand,
    },

    // VM commands (requires 'firecracker' feature)
    /// Manage Firecracker microVMs.
    #[cfg(feature = "firecracker")]
    Vm {
        /// VM subcommand to execute.
        subcommand: VmSubcommand,
    },

    // Robot mode commands (for AI agents)
    /// Access robot-mode self-documentation for AI agent integration.
    Robot {
        /// Robot subcommand to execute.
        subcommand: RobotSubcommand,
    },

    // Session commands (requires 'repl-sessions' feature)
    /// Browse and search AI coding session history.
    #[cfg(feature = "repl-sessions")]
    Sessions {
        /// Sessions subcommand to execute.
        subcommand: SessionsSubcommand,
    },

    // Update management commands (always available)
    /// Manage binary self-updates and rollbacks.
    Update {
        /// Update subcommand to execute.
        subcommand: UpdateSubcommand,
    },

    // Utility commands
    /// Show help information, optionally for a specific command.
    Help {
        /// Command to show help for; shows all commands if absent.
        command: Option<String>,
    },
    /// Exit the REPL (alias for `/exit`).
    Quit,
    /// Exit the REPL.
    Exit,
    /// Clear the terminal screen.
    Clear,
}

/// Subcommands for robot mode self-documentation.
#[derive(Debug, Clone, PartialEq)]
pub enum RobotSubcommand {
    /// Get capabilities summary
    Capabilities,
    /// Get schema for a command (or all commands)
    Schemas {
        /// Specific command name to fetch schema for, or all schemas if absent.
        command: Option<String>,
    },
    /// Get examples for a command
    Examples {
        /// Specific command name to fetch examples for, or all examples if absent.
        command: Option<String>,
    },
    /// List exit codes
    ExitCodes,
}

/// Subcommands for update management
#[derive(Debug, Clone, PartialEq)]
pub enum UpdateSubcommand {
    /// Check if updates are available
    Check,
    /// Install available updates
    Install,
    /// Rollback to a previous version
    Rollback {
        /// Semver version string to roll back to.
        version: String,
    },
    /// List available backup versions
    List,
}

/// Subcommands for the `/config` command.
#[derive(Debug, Clone, PartialEq)]
pub enum ConfigSubcommand {
    /// Display the current configuration as JSON.
    Show,
    /// Set a configuration key to a value.
    Set {
        /// Configuration key to set.
        key: String,
        /// New value for the key.
        value: String,
    },
}

/// Subcommands for the `/role` command.
#[derive(Debug, Clone, PartialEq)]
pub enum RoleSubcommand {
    /// List all available roles.
    List,
    /// Switch the active role by name or shortname.
    Select {
        /// Role name or shortname to activate.
        name: String,
    },
}

/// Subcommands for file system operations.
#[derive(Debug, Clone, PartialEq)]
#[cfg(feature = "repl-file")]
pub enum FileSubcommand {
    /// Search for files matching the given query.
    Search {
        /// Query string to match against file names or content.
        query: String,
    },
    /// List files in the current context.
    List,
    /// Show metadata for a specific file path.
    Info {
        /// Path of the file to inspect.
        path: String,
    },
}

/// Subcommands for AI coding session history management.
#[derive(Debug, Clone, PartialEq)]
#[cfg(feature = "repl-sessions")]
pub enum SessionsSubcommand {
    /// Detect available session sources
    Sources,
    /// List imported sessions (auto-imports if cache is empty)
    List {
        /// Filter sessions to those from a specific source identifier.
        source: Option<String>,
        /// Maximum number of sessions to display.
        limit: Option<usize>,
    },
    /// Search sessions by query
    Search {
        /// Full-text search query.
        query: String,
    },
    /// Show session statistics
    Stats,
    /// Show details of a specific session
    Show {
        /// Session ID to display.
        session_id: String,
    },
    /// Search sessions by concept (Phase 3 - requires enrichment)
    Concepts {
        /// Concept name to match sessions against.
        concept: String,
    },
    /// Find sessions related to a given session
    Related {
        /// ID of the reference session.
        session_id: String,
        /// Minimum number of shared concepts required for a related match.
        min_shared: Option<usize>,
    },
    /// Show session timeline grouped by period
    Timeline {
        /// Grouping period: `"day"`, `"week"`, or `"month"`.
        group_by: Option<String>,
        /// Maximum number of periods to display.
        limit: Option<usize>,
    },
    /// Export sessions to file
    Export {
        /// Output format: `"json"` or `"markdown"`.
        format: Option<String>,
        /// File path to write the export to.
        output: Option<String>,
        /// Limit export to a single session by ID.
        session_id: Option<String>,
    },
    /// Enrich sessions with concepts (Phase 3)
    Enrich {
        /// Specific session to enrich, or all sessions if absent.
        session_id: Option<String>,
    },
    /// List files accessed by a session
    Files {
        /// Session ID whose file accesses to list.
        session_id: String,
        /// Output as machine-readable JSON.
        json: bool,
    },
    /// Find sessions by file path
    ByFile {
        /// File path to search for in session records.
        file_path: String,
        /// Output as machine-readable JSON.
        json: bool,
    },
    /// Build search index and show index statistics
    Index {
        /// Show verbose index statistics.
        verbose: bool,
    },
    /// Cluster sessions by concept similarity (Spec F5.2)
    Cluster {
        /// Maximum number of clusters (auto-detect if None)
        k: Option<usize>,
        /// Minimum sessions per cluster
        min_sessions: Option<usize>,
        /// Output format: "json" for machine-readable output
        format: Option<String>,
    },
}

/// Subcommands for Firecracker microVM management.
#[cfg(feature = "firecracker")]
#[derive(Debug, Clone, PartialEq)]
pub enum VmSubcommand {
    /// List currently running VMs.
    List,
    /// Show VM pool utilisation statistics.
    Pool,
    /// Show status of a specific VM, or all VMs if absent.
    Status {
        /// VM identifier to query; shows all VMs if absent.
        vm_id: Option<String>,
    },
    /// Show metrics for a specific VM, or all VMs if absent.
    Metrics {
        /// VM identifier to query; shows all VM metrics if absent.
        vm_id: Option<String>,
    },
    /// Execute code in a VM.
    Execute {
        /// Source code to execute.
        code: String,
        /// Programming language identifier (e.g., `"python"`, `"rust"`).
        language: String,
        /// VM to run the code on; uses a pooled VM if absent.
        vm_id: Option<String>,
    },
    /// Run an agent task inside a VM.
    Agent {
        /// Identifier of the agent to invoke.
        agent_id: String,
        /// Task description to pass to the agent.
        task: String,
        /// VM to run the agent on; uses a pooled VM if absent.
        vm_id: Option<String>,
    },
    /// List tasks running on a specific VM.
    Tasks {
        /// VM identifier whose tasks to list.
        vm_id: String,
    },
    /// Allocate a VM from the pool by ID.
    Allocate {
        /// VM identifier to allocate.
        vm_id: String,
    },
    /// Release a VM back to the pool.
    Release {
        /// VM identifier to release.
        vm_id: String,
    },
    /// Continuously monitor a VM's status and metrics.
    Monitor {
        /// VM identifier to monitor.
        vm_id: String,
        /// Polling interval in seconds (default: 5).
        refresh: Option<u32>,
    },
}

/// Subcommands for web operations (HTTP, scraping, screenshots).
#[derive(Debug, Clone, PartialEq)]
#[cfg(feature = "repl-web")]
pub enum WebSubcommand {
    /// Perform an HTTP GET request.
    Get {
        /// Target URL.
        url: String,
        /// Optional HTTP headers to include in the request.
        headers: Option<std::collections::HashMap<String, String>>,
    },
    /// Perform an HTTP POST request.
    Post {
        /// Target URL.
        url: String,
        /// Request body payload.
        body: String,
        /// Optional HTTP headers to include in the request.
        headers: Option<std::collections::HashMap<String, String>>,
    },
    /// Scrape content from a web page.
    Scrape {
        /// URL of the page to scrape.
        url: String,
        /// CSS selector to extract a specific element.
        selector: Option<String>,
        /// CSS selector to wait for before extracting content.
        wait_for_element: Option<String>,
    },
    /// Capture a screenshot of a web page.
    Screenshot {
        /// URL of the page to screenshot.
        url: String,
        /// Viewport width in pixels.
        width: Option<u32>,
        /// Viewport height in pixels.
        height: Option<u32>,
        /// Capture the full scrollable page height.
        full_page: Option<bool>,
    },
    /// Render a web page to PDF.
    Pdf {
        /// URL of the page to render.
        url: String,
        /// Paper size (e.g., `"A4"`, `"Letter"`).
        page_size: Option<String>,
    },
    /// Submit an HTML form on a page.
    Form {
        /// URL of the page containing the form.
        url: String,
        /// Field-name to value map for form submission.
        form_data: std::collections::HashMap<String, String>,
    },
    /// Call a REST API endpoint.
    Api {
        /// API endpoint URL.
        endpoint: String,
        /// HTTP method (e.g., `"GET"`, `"POST"`).
        method: String,
        /// Optional JSON body for the request.
        data: Option<serde_json::Value>,
    },
    /// Poll the status of a long-running web operation.
    Status {
        /// Operation ID returned by a prior async web command.
        operation_id: String,
    },
    /// Cancel a long-running web operation.
    Cancel {
        /// Operation ID of the operation to cancel.
        operation_id: String,
    },
    /// Show the history of recent web operations.
    History {
        /// Maximum number of history entries to display.
        limit: Option<usize>,
    },
    /// View or modify web operation configuration.
    Config {
        /// Web configuration subcommand.
        subcommand: WebConfigSubcommand,
    },
}

/// Subcommands for web operation configuration.
#[derive(Debug, Clone, PartialEq)]
#[cfg(feature = "repl-web")]
pub enum WebConfigSubcommand {
    /// Display the current web configuration.
    Show,
    /// Set a web configuration key to a value.
    Set {
        /// Configuration key to set.
        key: String,
        /// New value for the key.
        value: String,
    },
    /// Reset web configuration to defaults.
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

            #[cfg(feature = "llm")]
            "chat" => {
                let message = if parts.len() > 1 {
                    Some(parts[1..].join(" "))
                } else {
                    None
                };
                Ok(ReplCommand::Chat { message })
            }

            #[cfg(not(feature = "llm"))]
            "chat" => Err(anyhow!(
                "Chat feature not enabled. Rebuild with --features llm"
            )),

            #[cfg(feature = "llm")]
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

            #[cfg(not(feature = "llm"))]
            "summarize" => Err(anyhow!(
                "Summarize feature not enabled. Rebuild with --features llm"
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
                            subcommand: WebSubcommand::Post {
                                url,
                                body: String::new(),
                                headers: None,
                            },
                        })
                    }
                    "scrape" => {
                        if parts.len() < 3 {
                            return Err(anyhow!("Web scrape requires a URL"));
                        }
                        let url = parts[2].to_string();
                        Ok(ReplCommand::Web {
                            subcommand: WebSubcommand::Scrape {
                                url,
                                selector: None,
                                wait_for_element: None,
                            },
                        })
                    }
                    "screenshot" => {
                        if parts.len() < 3 {
                            return Err(anyhow!("Web screenshot requires a URL"));
                        }
                        let url = parts[2].to_string();
                        Ok(ReplCommand::Web {
                            subcommand: WebSubcommand::Screenshot {
                                url,
                                width: None,
                                height: None,
                                full_page: None,
                            },
                        })
                    }
                    "pdf" => {
                        if parts.len() < 3 {
                            return Err(anyhow!("Web PDF requires a URL"));
                        }
                        let url = parts[2].to_string();
                        Ok(ReplCommand::Web {
                            subcommand: WebSubcommand::Pdf {
                                url,
                                page_size: None,
                            },
                        })
                    }
                    "form" => {
                        if parts.len() < 3 {
                            return Err(anyhow!("Web form requires a URL"));
                        }
                        let url = parts[2].to_string();
                        Ok(ReplCommand::Web {
                            subcommand: WebSubcommand::Form {
                                url,
                                form_data: std::collections::HashMap::new(),
                            },
                        })
                    }
                    "api" => {
                        if parts.len() < 3 {
                            return Err(anyhow!("Web API requires an endpoint"));
                        }
                        let endpoint = parts[2].to_string();
                        Ok(ReplCommand::Web {
                            subcommand: WebSubcommand::Api {
                                endpoint,
                                method: "GET".to_string(),
                                data: None,
                            },
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
                            Some(
                                parts[2]
                                    .parse::<usize>()
                                    .map_err(|_| anyhow!("Invalid limit"))?,
                            )
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
                                        return Err(anyhow!(
                                            "Web config set requires key and value"
                                        ));
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

            #[cfg(not(feature = "firecracker"))]
            "vm" => Err(anyhow!(
                "VM commands not enabled. Rebuild with --features firecracker"
            )),

            #[cfg(feature = "firecracker")]
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
                            subcommand: VmSubcommand::Execute {
                                code,
                                language,
                                vm_id,
                            },
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
                            subcommand: VmSubcommand::Agent {
                                agent_id,
                                task,
                                vm_id,
                            },
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
                                            return Err(anyhow!(
                                                "--refresh must be a positive integer"
                                            ));
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

            "robot" => {
                if parts.len() < 2 {
                    return Err(anyhow!(
                        "Robot command requires a subcommand (capabilities | schemas [command] | examples [command] | exit-codes)"
                    ));
                }

                match parts[1] {
                    "capabilities" | "caps" => Ok(ReplCommand::Robot {
                        subcommand: RobotSubcommand::Capabilities,
                    }),
                    "schemas" | "schema" => {
                        let command = if parts.len() > 2 {
                            Some(parts[2].to_string())
                        } else {
                            None
                        };
                        Ok(ReplCommand::Robot {
                            subcommand: RobotSubcommand::Schemas { command },
                        })
                    }
                    "examples" | "example" => {
                        let command = if parts.len() > 2 {
                            Some(parts[2].to_string())
                        } else {
                            None
                        };
                        Ok(ReplCommand::Robot {
                            subcommand: RobotSubcommand::Examples { command },
                        })
                    }
                    "exit-codes" | "exitcodes" | "codes" => Ok(ReplCommand::Robot {
                        subcommand: RobotSubcommand::ExitCodes,
                    }),
                    _ => Err(anyhow!(
                        "Unknown robot subcommand: {}. Use: capabilities, schemas, examples, exit-codes",
                        parts[1]
                    )),
                }
            }

            #[cfg(feature = "repl-sessions")]
            "sessions" | "session" => {
                if parts.len() < 2 {
                    return Err(anyhow!(
                        "Sessions command requires a subcommand (sources | import | list | search | stats | show)"
                    ));
                }

                match parts[1] {
                    "sources" | "detect" => Ok(ReplCommand::Sessions {
                        subcommand: SessionsSubcommand::Sources,
                    }),
                    "import" => Err(anyhow!(
                        "The 'import' command has been removed. Sessions are now automatically imported when needed. Use '/sessions list' or '/sessions search <query>' instead."
                    )),
                    "list" | "ls" => {
                        let mut source = None;
                        let mut limit = None;
                        let mut i = 2;

                        while i < parts.len() {
                            match parts[i] {
                                "--source" => {
                                    if i + 1 < parts.len() {
                                        source = Some(parts[i + 1].to_string());
                                        i += 2;
                                    } else {
                                        return Err(anyhow!("--source requires a value"));
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
                                _ => i += 1,
                            }
                        }

                        Ok(ReplCommand::Sessions {
                            subcommand: SessionsSubcommand::List { source, limit },
                        })
                    }
                    "search" => {
                        if parts.len() < 3 {
                            return Err(anyhow!("Sessions search requires a query"));
                        }
                        let query = parts[2..].join(" ");
                        Ok(ReplCommand::Sessions {
                            subcommand: SessionsSubcommand::Search { query },
                        })
                    }
                    "stats" | "statistics" => Ok(ReplCommand::Sessions {
                        subcommand: SessionsSubcommand::Stats,
                    }),
                    "show" | "get" => {
                        if parts.len() < 3 {
                            return Err(anyhow!("Sessions show requires a session ID"));
                        }
                        let session_id = parts[2].to_string();
                        Ok(ReplCommand::Sessions {
                            subcommand: SessionsSubcommand::Show { session_id },
                        })
                    }
                    "concepts" | "by-concept" => {
                        if parts.len() < 3 {
                            return Err(anyhow!("Sessions concepts requires a concept name"));
                        }
                        let concept = parts[2..].join(" ");
                        Ok(ReplCommand::Sessions {
                            subcommand: SessionsSubcommand::Concepts { concept },
                        })
                    }
                    "related" => {
                        if parts.len() < 3 {
                            return Err(anyhow!("Sessions related requires a session ID"));
                        }
                        let session_id = parts[2].to_string();
                        let min_shared = if parts.len() > 3 {
                            parts
                                .iter()
                                .position(|&p| p == "--min")
                                .and_then(|i| parts.get(i + 1).and_then(|v| v.parse().ok()))
                        } else {
                            None
                        };
                        Ok(ReplCommand::Sessions {
                            subcommand: SessionsSubcommand::Related {
                                session_id,
                                min_shared,
                            },
                        })
                    }
                    "timeline" => {
                        let mut group_by = None;
                        let mut limit = None;
                        let mut i = 2;

                        while i < parts.len() {
                            match parts[i] {
                                "--group-by" | "--group" => {
                                    if i + 1 < parts.len() {
                                        group_by = Some(parts[i + 1].to_string());
                                        i += 2;
                                    } else {
                                        return Err(anyhow!(
                                            "--group-by requires a value (day, week, month)"
                                        ));
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
                                _ => i += 1,
                            }
                        }

                        Ok(ReplCommand::Sessions {
                            subcommand: SessionsSubcommand::Timeline { group_by, limit },
                        })
                    }
                    "export" => {
                        let mut format = None;
                        let mut output = None;
                        let mut session_id = None;
                        let mut i = 2;

                        while i < parts.len() {
                            match parts[i] {
                                "--format" => {
                                    if i + 1 < parts.len() {
                                        format = Some(parts[i + 1].to_string());
                                        i += 2;
                                    } else {
                                        return Err(anyhow!(
                                            "--format requires a value (json, markdown)"
                                        ));
                                    }
                                }
                                "--output" | "-o" => {
                                    if i + 1 < parts.len() {
                                        output = Some(parts[i + 1].to_string());
                                        i += 2;
                                    } else {
                                        return Err(anyhow!("--output requires a file path"));
                                    }
                                }
                                "--session" | "--id" => {
                                    if i + 1 < parts.len() {
                                        session_id = Some(parts[i + 1].to_string());
                                        i += 2;
                                    } else {
                                        return Err(anyhow!("--session requires a session ID"));
                                    }
                                }
                                _ => i += 1,
                            }
                        }

                        Ok(ReplCommand::Sessions {
                            subcommand: SessionsSubcommand::Export {
                                format,
                                output,
                                session_id,
                            },
                        })
                    }
                    "enrich" => {
                        let session_id = if parts.len() > 2 {
                            Some(parts[2].to_string())
                        } else {
                            None
                        };
                        Ok(ReplCommand::Sessions {
                            subcommand: SessionsSubcommand::Enrich { session_id },
                        })
                    }
                    "files" => {
                        if parts.len() < 3 {
                            return Err(anyhow!("Sessions files requires a session ID"));
                        }
                        let session_id = parts[2].to_string();
                        let json = parts.contains(&"--json");
                        Ok(ReplCommand::Sessions {
                            subcommand: SessionsSubcommand::Files { session_id, json },
                        })
                    }
                    "by-file" => {
                        if parts.len() < 3 {
                            return Err(anyhow!("Sessions by-file requires a file path"));
                        }
                        let file_path = parts[2].to_string();
                        let json = parts.contains(&"--json");
                        Ok(ReplCommand::Sessions {
                            subcommand: SessionsSubcommand::ByFile { file_path, json },
                        })
                    }
                    "index" => {
                        let verbose = parts.contains(&"--verbose") || parts.contains(&"-v");
                        Ok(ReplCommand::Sessions {
                            subcommand: SessionsSubcommand::Index { verbose },
                        })
                    }
                    "cluster" => {
                        let mut k = None;
                        let mut min_sessions = None;
                        let mut format = None;
                        let mut i = 2;
                        while i < parts.len() {
                            match parts[i] {
                                "--k" | "-k" => {
                                    if i + 1 < parts.len() {
                                        k = parts[i + 1].parse::<usize>().ok();
                                        i += 2;
                                    } else {
                                        i += 1;
                                    }
                                }
                                "--min-sessions" => {
                                    if i + 1 < parts.len() {
                                        min_sessions = parts[i + 1].parse::<usize>().ok();
                                        i += 2;
                                    } else {
                                        i += 1;
                                    }
                                }
                                "--format" => {
                                    if i + 1 < parts.len() {
                                        format = Some(parts[i + 1].to_string());
                                        i += 2;
                                    } else {
                                        i += 1;
                                    }
                                }
                                _ => i += 1,
                            }
                        }
                        Ok(ReplCommand::Sessions {
                            subcommand: SessionsSubcommand::Cluster {
                                k,
                                min_sessions,
                                format,
                            },
                        })
                    }
                    _ => Err(anyhow!(
                        "Unknown sessions subcommand: {}. Use: sources, list, search, stats, show, concepts, related, cluster, timeline, export, enrich, files, by-file, index",
                        parts[1]
                    )),
                }
            }

            #[cfg(not(feature = "repl-sessions"))]
            "sessions" | "session" => Err(anyhow!(
                "Sessions feature not enabled. Rebuild with --features repl-sessions"
            )),

            "update" => {
                if parts.len() < 2 {
                    return Err(anyhow!(
                        "Update command requires a subcommand (check | install | rollback <version> | list)"
                    ));
                }

                match parts[1] {
                    "check" => Ok(ReplCommand::Update {
                        subcommand: UpdateSubcommand::Check,
                    }),
                    "install" => Ok(ReplCommand::Update {
                        subcommand: UpdateSubcommand::Install,
                    }),
                    "rollback" => {
                        if parts.len() < 3 {
                            return Err(anyhow!("Update rollback requires a version"));
                        }
                        Ok(ReplCommand::Update {
                            subcommand: UpdateSubcommand::Rollback {
                                version: parts[2].to_string(),
                            },
                        })
                    }
                    "list" => Ok(ReplCommand::Update {
                        subcommand: UpdateSubcommand::List,
                    }),
                    _ => Err(anyhow!(
                        "Unknown update subcommand: {}. Use: check, install, rollback <version>, list",
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
    #[allow(unused_mut)]
    pub fn available_commands() -> Vec<&'static str> {
        // Allow unused_mut because mut is conditionally needed based on features
        #[allow(unused_mut)]
        let mut commands = vec![
            "search", "config", "role", "graph", "vm", "robot", "update", "help", "quit", "exit",
            "clear",
        ];

        #[cfg(feature = "llm")]
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

        #[cfg(feature = "repl-sessions")]
        {
            commands.extend_from_slice(&["sessions"]);
        }

        commands
    }

    /// Get command description for help system
    pub fn get_command_help(command: &str) -> Option<&'static str> {
        match command {
            "search" => Some(
                "/search <query> [--role <role>] [--limit <n>] [--semantic] [--concepts] - Search documents",
            ),
            "config" => Some("/config show | set <key> <value> - Manage configuration"),
            "role" => Some("/role list | select <name> - Manage roles"),
            "graph" => Some("/graph [--top-k <n>] - Show knowledge graph"),
            "help" => Some("/help [command] - Show help information"),
            "quit" | "q" => Some("/quit, /q - Exit REPL"),
            "exit" => Some("/exit - Exit REPL"),
            "clear" => Some("/clear - Clear screen"),
            "vm" => Some(
                "/vm <subcommand> [args] - VM management (list, pool, status, metrics, execute, agent, tasks, allocate, release, monitor)",
            ),
            "robot" => Some(
                "/robot <subcommand> - AI agent self-documentation (capabilities, schemas [cmd], examples [cmd], exit-codes)",
            ),
            "update" => Some(
                "/update <subcommand> - Manage updates (check, install, rollback <version>, list)",
            ),

            #[cfg(feature = "repl-file")]
            "file" => Some("/file <subcommand> [args] - File operations (search, list, info)"),

            #[cfg(feature = "repl-web")]
            "web" => Some(
                "/web <subcommand> [args] - Web operations (get, post, scrape, screenshot, pdf, form, api, status, cancel, history, config)",
            ),

            #[cfg(feature = "llm")]
            "chat" => Some("/chat [message] - Interactive chat with AI"),
            #[cfg(feature = "llm")]
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

            #[cfg(feature = "repl-sessions")]
            "sessions" => Some(
                "/sessions <subcommand> - AI coding session history (sources, list, search, stats, show, concepts, related, timeline, export, enrich, files, by-file)",
            ),

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
                semantic: false,
                concepts: false,
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
                semantic: false,
                concepts: false,
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

    #[test]
    fn test_update_command_parsing() {
        // Test update check
        let cmd = "/update check".parse::<ReplCommand>().unwrap();
        assert_eq!(
            cmd,
            ReplCommand::Update {
                subcommand: UpdateSubcommand::Check
            }
        );

        // Test update install
        let cmd = "/update install".parse::<ReplCommand>().unwrap();
        assert_eq!(
            cmd,
            ReplCommand::Update {
                subcommand: UpdateSubcommand::Install
            }
        );

        // Test update rollback with version
        let cmd = "/update rollback 1.0.0".parse::<ReplCommand>().unwrap();
        assert_eq!(
            cmd,
            ReplCommand::Update {
                subcommand: UpdateSubcommand::Rollback {
                    version: "1.0.0".to_string()
                }
            }
        );

        // Test update list
        let cmd = "/update list".parse::<ReplCommand>().unwrap();
        assert_eq!(
            cmd,
            ReplCommand::Update {
                subcommand: UpdateSubcommand::List
            }
        );
    }

    #[test]
    fn test_update_command_errors() {
        // Test update without subcommand
        let result = "/update".parse::<ReplCommand>();
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("requires a subcommand")
        );

        // Test update rollback without version
        let result = "/update rollback".parse::<ReplCommand>();
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("requires a version")
        );

        // Test unknown update subcommand
        let result = "/update unknown".parse::<ReplCommand>();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Unknown update"));
    }
}
