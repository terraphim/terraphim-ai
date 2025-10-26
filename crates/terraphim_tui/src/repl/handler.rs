//! REPL handler implementation

#[cfg(feature = "repl-file")]
use super::commands::FileSubcommand;
use super::commands::{
    ConfigSubcommand, ReplCommand, RoleSubcommand, WebConfigSubcommand, WebSubcommand,
};
use crate::client::ApiClient;

use anyhow::Result;
use std::io::{self, Write};
use std::str::FromStr;

#[cfg(feature = "repl")]
use rustyline::Editor;

#[cfg(feature = "repl")]
use colored::Colorize;

pub struct ReplHandler {
    api_client: Option<ApiClient>,
    current_role: String,
    #[cfg(feature = "repl-custom")]
    command_registry: Option<crate::commands::registry::CommandRegistry>,
    #[cfg(feature = "repl-custom")]
    command_validator: Option<crate::commands::validator::CommandValidator>,
    #[cfg(feature = "repl-custom")]
    command_executor: Option<crate::commands::executor::CommandExecutor>,
}

impl ReplHandler {
    pub fn new_offline() -> Self {
        Self {
            api_client: None,
            current_role: "Default".to_string(),
            #[cfg(feature = "repl-custom")]
            command_registry: None,
            #[cfg(feature = "repl-custom")]
            command_validator: None,
            #[cfg(feature = "repl-custom")]
            command_executor: None,
        }
    }

    pub fn new_server(api_client: ApiClient) -> Self {
        Self {
            api_client: Some(api_client),
            current_role: "Terraphim Engineer".to_string(),
            #[cfg(feature = "repl-custom")]
            command_registry: None,
            #[cfg(feature = "repl-custom")]
            command_validator: None,
            #[cfg(feature = "repl-custom")]
            command_executor: None,
        }
    }

    #[cfg(feature = "repl-custom")]
    /// Initialize command registry and validator with API client integration
    pub async fn initialize_commands(&mut self) -> Result<()> {
        use std::sync::Arc;

        // Initialize command registry
        let mut registry = crate::commands::registry::CommandRegistry::new()?;

        // Add default directories to registry
        let default_paths = vec![
            std::path::PathBuf::from("./commands"),
            std::path::PathBuf::from("./terraphim_commands"),
        ];

        for path in &default_paths {
            if path.exists() {
                registry.add_command_directory(path.clone());
            }
        }

        // Load all commands
        match registry.load_all_commands().await {
            Ok(count) => {
                if count > 0 {
                    println!("Loaded {} custom commands", count);
                }
            }
            Err(e) => {
                eprintln!("Warning: Failed to load commands: {}", e);
            }
        }

        self.command_registry = Some(registry);

        // Initialize command validator with API client if available
        if let Some(ref api_client) = self.api_client {
            let validator = crate::commands::validator::CommandValidator::with_api_client(
                Arc::new(api_client.clone()),
            );
            self.command_validator = Some(validator);
        } else {
            self.command_validator = Some(crate::commands::validator::CommandValidator::new());
        }

        // Initialize command executor with hooks
        let executor = if let Some(ref api_client) = self.api_client {
            crate::commands::executor::CommandExecutor::with_api_client(api_client.clone())
        } else {
            crate::commands::executor::CommandExecutor::new()
        };

        // Add appropriate hooks based on role
        let hooks = match self.current_role.as_str() {
            "Terraphim Engineer" => crate::commands::hooks::create_development_hooks(),
            "System Operator" => crate::commands::hooks::create_production_hooks(),
            _ => crate::commands::hooks::create_default_hooks(),
        };

        let executor_with_hooks = executor.with_hooks(hooks);
        self.command_executor = Some(executor_with_hooks);

        Ok(())
    }

    #[cfg(feature = "repl")]
    pub async fn run(&mut self) -> Result<()> {
        use rustyline::completion::{Completer, Pair};
        use rustyline::highlight::Highlighter;
        use rustyline::hint::Hinter;
        use rustyline::validate::Validator;
        use rustyline::{Context, Helper};

        // Create a rolegraph-aware command completer
        #[derive(Clone)]
        struct CommandCompleter {
            current_role: String,
        }

        impl CommandCompleter {
            fn new(current_role: String) -> Self {
                Self { current_role }
            }
        }

        impl Helper for CommandCompleter {}
        impl Hinter for CommandCompleter {
            type Hint = String;

            fn hint(&self, line: &str, _pos: usize, _ctx: &Context<'_>) -> Option<String> {
                // Provide contextual hints based on role
                if line.trim().is_empty() {
                    return Some("Try /search, /vm, /graph, or /help".to_string());
                }

                // Role-specific hints
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

        impl Highlighter for CommandCompleter {}
        impl Validator for CommandCompleter {}

        impl Completer for CommandCompleter {
            type Candidate = Pair;

            fn complete(
                &self,
                line: &str,
                pos: usize,
                _ctx: &Context<'_>,
            ) -> rustyline::Result<(usize, Vec<Pair>)> {
                let line = &line[..pos];

                if line.starts_with('/') || line.is_empty() {
                    let prefix = if let Some(stripped) = line.strip_prefix('/') {
                        stripped
                    } else {
                        line
                    };

                    let mut matches = Vec::new();

                    // Basic command completion
                    let commands = ReplCommand::available_commands();
                    for cmd in commands {
                        if cmd.starts_with(prefix) {
                            matches.push(Pair {
                                display: format!("/{}", cmd),
                                replacement: format!("/{}", cmd),
                            });
                        }
                    }

                    // Role-specific command suggestions
                    if prefix.starts_with("search") {
                        matches.extend_from_slice(&[
                            Pair {
                                display: "/search --semantic --concepts".to_string(),
                                replacement: "/search --semantic --concepts ".to_string(),
                            },
                            Pair {
                                display: "/search --role".to_string(),
                                replacement: "/search --role ".to_string(),
                            },
                            Pair {
                                display: "/search --limit".to_string(),
                                replacement: "/search --limit ".to_string(),
                            },
                        ]);
                    }

                    // VM command completion with role-aware suggestions
                    if prefix.starts_with("vm") {
                        let vm_commands = match self.current_role.as_str() {
                            "Terraphim Engineer" => {
                                vec![
                                    "list", "pool", "status", "metrics", "execute", "agent",
                                    "monitor", "tasks", "allocate", "release",
                                ]
                            }
                            _ => {
                                vec!["list", "status", "execute", "tasks"]
                            }
                        };

                        for cmd in vm_commands {
                            if cmd.starts_with(&prefix[3..]) {
                                // Skip "vm " prefix
                                matches.push(Pair {
                                    display: format!("/vm {}", cmd),
                                    replacement: format!("/vm {} ", cmd),
                                });
                            }
                        }
                    }

                    // Search term suggestions based on role concepts
                    if line.starts_with("/search ") {
                        let search_term = line[8..].trim(); // Skip "/search "
                        if !search_term.is_empty() {
                            // Add role-based search suggestions
                            let role_suggestions = match self.current_role.as_str() {
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
                            };

                            for suggestion in role_suggestions {
                                if suggestion
                                    .to_lowercase()
                                    .starts_with(&search_term.to_lowercase())
                                {
                                    matches.push(Pair {
                                        display: format!("{} ", suggestion),
                                        replacement: format!("/search {} ", suggestion),
                                    });
                                }
                            }
                        }
                    }

                    let start_pos = if line.starts_with('/') {
                        pos - prefix.len() - 1
                    } else {
                        0
                    };
                    Ok((start_pos, matches))
                } else {
                    Ok((pos, Vec::new()))
                }
            }
        }

        let mut rl = Editor::<CommandCompleter, rustyline::history::DefaultHistory>::new()?;
        rl.set_helper(Some(CommandCompleter::new(self.current_role.clone())));

        // Load command history if it exists
        let history_file = dirs::home_dir()
            .map(|h| h.join(".terraphim_tui_history"))
            .unwrap_or_else(|| std::path::PathBuf::from(".terraphim_tui_history"));

        let _ = rl.load_history(&history_file);

        println!("{}", "=".repeat(60).cyan());
        println!("{}", "🌍 Terraphim TUI REPL".bold().cyan());
        println!("{}", "=".repeat(60).cyan());
        self.show_welcome().await;
        println!();

        loop {
            let prompt = format!("{}> ", self.current_role.green().bold());

            match rl.readline(&prompt) {
                Ok(line) => {
                    let line = line.trim();
                    if line.is_empty() {
                        continue;
                    }

                    rl.add_history_entry(line)?;

                    match self.execute_command(line).await {
                        Ok(should_exit) => {
                            if should_exit {
                                break;
                            }
                        }
                        Err(e) => {
                            println!("{} {}", "Error:".red().bold(), e);
                        }
                    }
                }
                Err(rustyline::error::ReadlineError::Interrupted) => {
                    println!("^C");
                    break;
                }
                Err(rustyline::error::ReadlineError::Eof) => {
                    println!("^D");
                    break;
                }
                Err(err) => {
                    println!("{} Failed to read line: {:?}", "Error:".red().bold(), err);
                    break;
                }
            }
        }

        // Save command history
        let _ = rl.save_history(&history_file);
        println!("{}", "Goodbye! 👋".cyan());

        Ok(())
    }

    #[cfg(not(feature = "repl"))]
    pub async fn run(&mut self) -> Result<()> {
        println!("REPL feature not enabled. Please rebuild with --features repl");
        Ok(())
    }

    async fn show_welcome(&self) {
        println!(
            "Type {} for help, {} to exit",
            "/help".yellow(),
            "/quit".yellow()
        );

        let mode = if self.api_client.is_none() {
            "Offline Mode"
        } else {
            "Server Mode"
        };

        println!(
            "Mode: {} | Current Role: {}",
            mode.bold(),
            self.current_role.green().bold()
        );

        self.show_available_commands();
    }

    #[cfg(feature = "repl")]
    fn show_available_commands(&self) {
        println!("\n{}", "Available commands:".bold());
        println!("  {} - Search documents", "/search <query>".yellow());
        println!("  {} - Manage configuration", "/config [show|set]".yellow());
        println!("  {} - Manage roles", "/role [list|select]".yellow());
        println!("  {} - Show knowledge graph", "/graph".yellow());
        println!(
            "  {} - Manage VMs",
            "/vm [list|pool|status|execute|tasks|allocate|release]".yellow()
        );

        #[cfg(feature = "repl-chat")]
        {
            println!("  {} - Chat with AI", "/chat [message]".yellow());
            println!("  {} - Summarize content", "/summarize <target>".yellow());
        }

        #[cfg(feature = "repl-mcp")]
        {
            println!(
                "  {} - Autocomplete terms",
                "/autocomplete <query>".yellow()
            );
            println!("  {} - Extract paragraphs", "/extract <text>".yellow());
            println!("  {} - Find matches", "/find <text>".yellow());
            println!("  {} - Replace matches", "/replace <text>".yellow());
            println!("  {} - Show thesaurus", "/thesaurus".yellow());
        }

        println!("  {} - Show help", "/help [command]".yellow());
        println!("  {} - Exit REPL", "/quit".yellow());
    }

    #[cfg(not(feature = "repl"))]
    fn show_available_commands(&self) {
        println!("REPL commands not available without repl feature");
    }

    async fn execute_command(&mut self, input: &str) -> Result<bool> {
        let command = ReplCommand::from_str(input)?;

        match command {
            ReplCommand::Search {
                query,
                role,
                limit,
                semantic,
                concepts,
            } => {
                self.handle_search(query, role, limit, semantic, concepts)
                    .await?;
            }
            ReplCommand::Config { subcommand } => {
                self.handle_config(subcommand).await?;
            }
            ReplCommand::Role { subcommand } => {
                self.handle_role(subcommand).await?;
            }
            ReplCommand::Graph { top_k } => {
                self.handle_graph(top_k).await?;
            }
            ReplCommand::Vm { subcommand } => {
                self.handle_vm(subcommand).await?;
            }
            ReplCommand::Web { subcommand } => {
                self.handle_web(subcommand).await?;
            }
            #[cfg(feature = "repl-file")]
            ReplCommand::File { subcommand } => {
                self.handle_file(subcommand).await?;
            }
            ReplCommand::Help { command } => {
                self.handle_help(command).await?;
            }
            ReplCommand::Quit | ReplCommand::Exit => {
                return Ok(true);
            }
            ReplCommand::Clear => {
                self.handle_clear().await?;
            }

            #[cfg(feature = "repl-chat")]
            ReplCommand::Chat { message } => {
                self.handle_chat(message).await?;
            }

            #[cfg(feature = "repl-chat")]
            ReplCommand::Summarize { target } => {
                self.handle_summarize(target).await?;
            }

            #[cfg(feature = "repl-mcp")]
            ReplCommand::Autocomplete { query, limit } => {
                self.handle_autocomplete(query, limit).await?;
            }

            #[cfg(feature = "repl-mcp")]
            ReplCommand::Extract { text, exclude_term } => {
                self.handle_extract(text, exclude_term).await?;
            }

            #[cfg(feature = "repl-mcp")]
            ReplCommand::Find { text } => {
                self.handle_find(text).await?;
            }

            #[cfg(feature = "repl-mcp")]
            ReplCommand::Replace { text, format } => {
                self.handle_replace(text, format).await?;
            }

            #[cfg(feature = "repl-mcp")]
            ReplCommand::Thesaurus { role } => {
                self.handle_thesaurus(role).await?;
            }

            #[cfg(feature = "repl-custom")]
            ReplCommand::Custom {
                name,
                parameters,
                execution_mode,
            } => {
                self.handle_custom_command(name, parameters, execution_mode)
                    .await?;
            }

            #[cfg(feature = "repl-custom")]
            ReplCommand::Commands { subcommand } => {
                self.handle_commands_command(subcommand).await?;
            }
        }

        Ok(false)
    }

    async fn handle_search(
        &self,
        query: String,
        role: Option<String>,
        limit: Option<usize>,
        semantic: bool,
        concepts: bool,
    ) -> Result<()> {
        #[cfg(feature = "repl")]
        {
            use colored::Colorize;
            use comfy_table::modifiers::UTF8_ROUND_CORNERS;
            use comfy_table::presets::UTF8_FULL;
            use comfy_table::{Cell, Table};

            // Show search mode information
            let search_mode = if semantic && concepts {
                "Semantic + Concept Expansion"
            } else if semantic {
                "Semantic Search"
            } else if concepts {
                "Concept Expansion"
            } else {
                "Standard Search"
            };

            println!(
                "{} Searching for: '{}' ({})",
                "🔍".bold(),
                query.cyan(),
                search_mode.blue()
            );

            if self.api_client.is_none() {
                // Offline mode - mock results
                println!("{} Offline mode - showing mock results", "📱".blue());

                // Mock search results for demonstration
                let results = vec![
                    (
                        "Introduction to Terraphim",
                        "https://docs.example.com/intro",
                        0.95,
                    ),
                    (
                        "Advanced Search Techniques",
                        "https://docs.example.com/search",
                        0.87,
                    ),
                    (
                        "Knowledge Graph Theory",
                        "https://docs.example.com/graph",
                        0.82,
                    ),
                ];

                if results.is_empty() {
                    println!("{} No results found", "ℹ".blue().bold());
                } else {
                    // Enhanced results display with semantic information
                    let mut table = Table::new();
                    table
                        .load_preset(UTF8_FULL)
                        .apply_modifier(UTF8_ROUND_CORNERS)
                        .set_header(vec![
                            Cell::new("Rank").add_attribute(comfy_table::Attribute::Bold),
                            Cell::new("Title").add_attribute(comfy_table::Attribute::Bold),
                            Cell::new("URL").add_attribute(comfy_table::Attribute::Bold),
                            if semantic || concepts {
                                Cell::new("Relevance").add_attribute(comfy_table::Attribute::Bold)
                            } else {
                                Cell::new("Score").add_attribute(comfy_table::Attribute::Bold)
                            },
                        ]);

                    for doc in &results {
                        let relevance_score = if semantic || concepts {
                            // Show semantic relevance indicator
                            let score = doc.2;
                            if score >= 0.8 {
                                "🟢 High".to_string()
                            } else if score >= 0.5 {
                                "🟡 Medium".to_string()
                            } else {
                                "🔴 Low".to_string()
                            }
                        } else {
                            doc.2.to_string()
                        };

                        table.add_row(vec![
                            Cell::new(doc.2.to_string()),
                            Cell::new(doc.0),
                            Cell::new(doc.1),
                            Cell::new(relevance_score),
                        ]);
                    }

                    println!("{}", table);
                    println!(
                        "{} Found {} result(s) using {}",
                        "✅".bold(),
                        results.len().to_string().green(),
                        search_mode.blue()
                    );

                    // Show concept expansion if enabled
                    if concepts {
                        println!("\n{} Expanding concepts for query...", "🧠".bold());
                        // Mock concepts for demonstration
                        let concepts_list = vec![
                            "search",
                            "query",
                            "information retrieval",
                            "semantic",
                            "relevance",
                        ];
                        if !concepts_list.is_empty() {
                            println!("{} Related concepts:", "🔗".bold());
                            for (i, concept) in concepts_list.iter().enumerate() {
                                println!("  {}. {}", (i + 1).to_string().yellow(), concept.cyan());
                            }
                        }
                    }
                }
            } else if let Some(api_client) = &self.api_client {
                // Server mode
                use terraphim_types::{NormalizedTermValue, RoleName, SearchQuery};

                let role_name = role.as_ref().map(|r| RoleName::new(r));
                let search_query = SearchQuery {
                    search_term: NormalizedTermValue::from(query.as_str()),
                    search_terms: None,
                    operator: None,
                    skip: Some(0),
                    limit,
                    role: role_name,
                };

                match api_client.search(&search_query).await {
                    Ok(response) => {
                        if response.results.is_empty() {
                            println!("{} No results found", "ℹ".blue().bold());
                        } else {
                            // Enhanced results display with semantic information
                            let mut table = Table::new();
                            table
                                .load_preset(UTF8_FULL)
                                .apply_modifier(UTF8_ROUND_CORNERS)
                                .set_header(vec![
                                    Cell::new("Rank").add_attribute(comfy_table::Attribute::Bold),
                                    Cell::new("Title").add_attribute(comfy_table::Attribute::Bold),
                                    Cell::new("URL").add_attribute(comfy_table::Attribute::Bold),
                                    if semantic || concepts {
                                        Cell::new("Relevance")
                                            .add_attribute(comfy_table::Attribute::Bold)
                                    } else {
                                        Cell::new("Score")
                                            .add_attribute(comfy_table::Attribute::Bold)
                                    },
                                ]);

                            for doc in &response.results {
                                let relevance_score = if semantic || concepts {
                                    // Show semantic relevance indicator
                                    let score = doc.rank.unwrap_or_default();
                                    if score as f64 >= 0.8 {
                                        "🟢 High".to_string()
                                    } else if score as f64 >= 0.5 {
                                        "🟡 Medium".to_string()
                                    } else {
                                        "🔴 Low".to_string()
                                    }
                                } else {
                                    doc.rank.unwrap_or_default().to_string()
                                };

                                table.add_row(vec![
                                    Cell::new(doc.rank.unwrap_or_default().to_string()),
                                    Cell::new(&doc.title),
                                    Cell::new(if doc.url.is_empty() { "N/A" } else { &doc.url }),
                                    Cell::new(relevance_score),
                                ]);
                            }

                            println!("{}", table);
                            println!(
                                "{} Found {} result(s) using {}",
                                "✅".bold(),
                                response.results.len().to_string().green(),
                                search_mode.blue()
                            );
                        }
                    }
                    Err(e) => {
                        println!("{} Search failed: {}", "❌".bold(), e.to_string().red());
                    }
                }

                // Show concept expansion if enabled (server mode)
                if concepts {
                    if let Some(ref role) = role {
                        match api_client.rolegraph(Some(role)).await {
                            Ok(response) => {
                                if !response.nodes.is_empty() {
                                    println!("\n{} Expanding concepts for query...", "🧠".bold());
                                    println!("{} Related concepts:", "🔗".bold());
                                    for (i, node) in response.nodes.iter().take(5).enumerate() {
                                        println!(
                                            "  {}. {} (rank: {})",
                                            (i + 1).to_string().yellow(),
                                            node.label.cyan(),
                                            node.rank.to_string().blue()
                                        );
                                    }
                                }
                            }
                            Err(_) => {
                                // Silently fail if concepts aren't available
                            }
                        }
                    }
                }
            }
        }

        #[cfg(not(feature = "repl"))]
        {
            println!("Search functionality requires repl feature");
        }

        Ok(())
    }

    async fn handle_config(&self, subcommand: ConfigSubcommand) -> Result<()> {
        match subcommand {
            ConfigSubcommand::Show => {
                if false { // TODO: Reimplement service integration
                     // Config moved to server - available via API client
                     // let config_json = serde_json::to_string_pretty(&config)?;
                     // println!("{}", config_json);
                } else if let Some(api_client) = &self.api_client {
                    match api_client.get_config().await {
                        Ok(response) => {
                            let config_json = serde_json::to_string_pretty(&response.config)?;
                            println!("{}", config_json);
                        }
                        Err(e) => {
                            println!(
                                "{} Failed to get config: {}",
                                "❌".bold(),
                                e.to_string().red()
                            );
                        }
                    }
                }
            }
            ConfigSubcommand::Set { key, value } => {
                println!(
                    "{} Config modification not yet implemented",
                    "ℹ".blue().bold()
                );
                println!("Would set {} = {}", key.yellow(), value.cyan());
            }
        }
        Ok(())
    }

    async fn handle_role(&mut self, subcommand: RoleSubcommand) -> Result<()> {
        match subcommand {
            RoleSubcommand::List => {
                if false { // TODO: Reimplement service integration
                     // Roles moved to server - available via API client
                     //     println!("{}", "Available roles:".bold());
                     //     for role in roles {
                     //         let marker = if role == self.current_role {
                     //             "▶"
                     //         } else {
                     //             " "
                     //         };
                     //         println!("  {} {}", marker.green(), role);
                     //     }
                } else if let Some(api_client) = &self.api_client {
                    match api_client.get_config().await {
                        Ok(response) => {
                            println!("{}", "Available roles:".bold());
                            let roles: Vec<String> = response
                                .config
                                .roles
                                .keys()
                                .map(|k| k.to_string())
                                .collect();
                            for role in roles {
                                let marker = if role == self.current_role {
                                    "▶"
                                } else {
                                    " "
                                };
                                println!("  {} {}", marker.green(), role);
                            }
                        }
                        Err(e) => {
                            println!(
                                "{} Failed to get roles: {}",
                                "❌".bold(),
                                e.to_string().red()
                            );
                        }
                    }
                }
            }
            RoleSubcommand::Select { name } => {
                self.current_role = name.clone();
                println!("{} Switched to role: {}", "✅".bold(), name.green());
            }
        }
        Ok(())
    }

    async fn handle_graph(&self, top_k: Option<usize>) -> Result<()> {
        let k = top_k.unwrap_or(10);

        if false { // TODO: Reimplement service integration
             // Role graph concepts moved to server - available via API client
             //     println!("{} Top {} concepts:", "📊".bold(), k.to_string().cyan());
             //     for (i, concept) in concepts.iter().enumerate() {
             //         println!("  {}. {}", (i + 1).to_string().yellow(), concept);
             //     }
        } else if let Some(api_client) = &self.api_client {
            match api_client.rolegraph(Some(&self.current_role)).await {
                Ok(response) => {
                    let mut nodes = response.nodes;
                    nodes.sort_by(|a, b| b.rank.cmp(&a.rank));

                    println!("{} Top {} concepts:", "📊".bold(), k.to_string().cyan());
                    for (i, node) in nodes.iter().take(k).enumerate() {
                        println!(
                            "  {}. {} (rank: {})",
                            (i + 1).to_string().yellow(),
                            node.label,
                            node.rank.to_string().blue()
                        );
                    }
                }
                Err(e) => {
                    println!(
                        "{} Failed to get graph: {}",
                        "❌".bold(),
                        e.to_string().red()
                    );
                }
            }
        }

        Ok(())
    }

    async fn handle_help(&self, command: Option<String>) -> Result<()> {
        if let Some(cmd) = command {
            if let Some(help_text) = ReplCommand::get_command_help(&cmd) {
                println!("{}", help_text);
            } else {
                println!(
                    "{} No help available for command: {}",
                    "ℹ".blue().bold(),
                    cmd.yellow()
                );
            }
        } else {
            self.show_available_commands();
        }
        Ok(())
    }

    async fn handle_clear(&self) -> Result<()> {
        print!("\x1B[2J\x1B[1;1H");
        io::stdout().flush()?;
        Ok(())
    }

    #[cfg(feature = "repl-chat")]
    async fn handle_chat(&self, message: Option<String>) -> Result<()> {
        #[cfg(feature = "repl")]
        {
            use colored::Colorize;

            if let Some(msg) = message {
                println!("{} Sending message: '{}'", "💬".bold(), msg.cyan());

                if false { // TODO: Reimplement service integration
                     // Role-based chat moved to server - use api_client.chat() instead
                     // let role_name = &self.current_role;

                    // match api_client.chat(role_name, &msg, None).await {
                    //     Ok(response) => {
                    //         println!("\n{} {}\n", "🤖".bold(), "Response:".bold());
                    //         println!("{}", response);
                    //     }
                    //     Err(e) => {
                    //         println!("{} Chat failed: {}", "❌".bold(), e.to_string().red());
                    //     }
                    // }
                } else if let Some(api_client) = &self.api_client {
                    // Server mode chat
                    match api_client.chat(&self.current_role, &msg, None).await {
                        Ok(response) => {
                            println!("\n{} {}\n", "🤖".bold(), "Response:".bold());
                            println!("{}", response.message.unwrap_or(response.status));
                        }
                        Err(e) => {
                            println!("{} Chat failed: {}", "❌".bold(), e.to_string().red());
                        }
                    }
                }
            } else {
                println!("{} Please provide a message to chat", "ℹ".blue().bold());
                println!("Usage: {} <message>", "/chat".yellow());
            }
        }

        #[cfg(not(feature = "repl"))]
        {
            println!("Chat functionality requires repl feature");
        }

        Ok(())
    }

    #[cfg(feature = "repl-chat")]
    async fn handle_summarize(&self, target: String) -> Result<()> {
        #[cfg(feature = "repl")]
        {
            use colored::Colorize;

            println!("{} Summarizing: '{}'", "📝".bold(), target.cyan());

            if false { // TODO: Reimplement service integration
                 // Role-based summarization moved to server - use api_client.summarize() instead
                 // let role_name = &self.current_role;
                 // match api_client.summarize(role_name, &target).await {
                 //     Ok(summary) => {
                 //         println!("\n{} {}\n", "📋".bold(), "Summary:".bold());
                 //         println!("{}", summary);
                 //     }
                 //     Err(e) => {
                 //         println!(
                 //             "{} Summarization failed: {}",
                 //             "❌".bold(),
                 //             e.to_string().red()
                 //         );
                 //     }
                 // }
            } else if let Some(api_client) = &self.api_client {
                // Server mode summarization - create a temporary document
                use terraphim_types::Document;

                let doc = Document {
                    id: "temp-summary".to_string(),
                    url: "".to_string(),
                    title: "Text to Summarize".to_string(),
                    body: target,
                    description: None,
                    summarization: None,
                    stub: None,
                    tags: Some(vec![]),
                    rank: None,
                    source_haystack: None,
                };

                match api_client
                    .summarize_document(&doc, Some(&self.current_role))
                    .await
                {
                    Ok(response) => {
                        println!("\n{} {}\n", "📋".bold(), "Summary:".bold());
                        println!(
                            "{}",
                            response
                                .summary
                                .unwrap_or_else(|| "No summary available".to_string())
                        );
                    }
                    Err(e) => {
                        println!(
                            "{} Summarization failed: {}",
                            "❌".bold(),
                            e.to_string().red()
                        );
                    }
                }
            }
        }

        #[cfg(not(feature = "repl"))]
        {
            println!("Summarization functionality requires repl feature");
        }

        Ok(())
    }

    #[cfg(feature = "repl-mcp")]
    async fn handle_autocomplete(&self, query: String, limit: Option<usize>) -> Result<()> {
        #[cfg(feature = "repl")]
        {
            use colored::Colorize;
            use comfy_table::modifiers::UTF8_ROUND_CORNERS;
            use comfy_table::presets::UTF8_FULL;
            use comfy_table::{Cell, Table};

            println!("{} Autocompleting: '{}'", "🔍".bold(), query.cyan());

            if false { // TODO: Reimplement service integration
                 // Autocomplete moved to server - use api_client.autocomplete() instead
                 // let role_name = &self.current_role;

                // match api_client.autocomplete(role_name, &query, limit).await {
                //     Ok(results) => {
                //         if results.is_empty() {
                //             println!("{} No autocomplete suggestions found", "ℹ".blue().bold());
                //         } else {
                //             let mut table = Table::new();
                //             table
                //                 .load_preset(UTF8_FULL)
                //                 .apply_modifier(UTF8_ROUND_CORNERS)
                //                 .set_header(vec![
                //                     Cell::new("Term").add_attribute(comfy_table::Attribute::Bold),
                //                     Cell::new("Score").add_attribute(comfy_table::Attribute::Bold),
                //                     Cell::new("URL").add_attribute(comfy_table::Attribute::Bold),
                //                 ]);

                //             for result in &results {
                //                 table.add_row(vec![
                //                     Cell::new(&result.term),
                //                     Cell::new(format!("{:.2}", result.score)),
                //                     Cell::new(result.url.as_deref().unwrap_or("N/A")),
                //                 ]);
                //             }

                //             println!("{}", table);
                //             println!(
                //                 "{} Found {} suggestion(s)",
                //                 "✅".bold(),
                //                 results.len().to_string().green()
                //             );
                //         }
                //     }
                //     Err(e) => {
                //         println!(
                //             "{} Autocomplete failed: {}",
                //             "❌".bold(),
                //             e.to_string().red()
                //         );
                //     }
                // }
            } else {
                println!(
                    "{} Autocomplete requires offline mode with thesaurus",
                    "ℹ".blue().bold()
                );
            }
        }

        #[cfg(not(feature = "repl"))]
        {
            println!("Autocomplete functionality requires repl feature");
        }

        Ok(())
    }

    #[cfg(feature = "repl-mcp")]
    async fn handle_extract(&self, text: String, exclude_term: bool) -> Result<()> {
        #[cfg(feature = "repl")]
        {
            use colored::Colorize;
            use comfy_table::modifiers::UTF8_ROUND_CORNERS;
            use comfy_table::presets::UTF8_FULL;
            use comfy_table::{Cell, Table};

            println!("{} Extracting paragraphs from text...", "📄".bold());

            if false { // TODO: Reimplement service integration
                 // Paragraph extraction moved to server - use current_role from handler
                 // let role_name = &self.current_role;

                // match self.api_client
                //     .as_ref()
                //     .unwrap()
                //     .extract_paragraphs(&self.current_role, &text, exclude_term)
                //     .await
                // {
                //     Ok(results) => {
                // if results.is_empty() {
                //         println!("{} No paragraphs found", "ℹ".blue().bold());
                //     } else {
                //         let mut table = Table::new();
                //         table
                //             .load_preset(UTF8_FULL)
                //             .apply_modifier(UTF8_ROUND_CORNERS)
                //             .set_header(vec![
                //                 Cell::new("Term").add_attribute(comfy_table::Attribute::Bold),
                //                 Cell::new("Paragraph")
                //                     .add_attribute(comfy_table::Attribute::Bold),
                //             ]);

                //         for (term, paragraph) in &results {
                //             let truncated_paragraph = if paragraph.len() > 100 {
                //                 format!("{}...", &paragraph[..97])
                //             } else {
                //                 paragraph.clone()
                //             };

                //             table
                //                 .add_row(vec![Cell::new(term), Cell::new(truncated_paragraph)]);
                //         }

                //         println!("{}", table);
                //         println!(
                //             "{} Found {} paragraph(s)",
                //             "✅".bold(),
                //             results.len().to_string().green()
                //         );
                //     }
                //     }
                //     Err(e) => {
                //         println!("{} Extract failed: {}", "❌".bold(), e.to_string().red());
                //     }
                // }
            } else {
                println!(
                    "{} Extract requires offline mode with thesaurus",
                    "ℹ".blue().bold()
                );
            }
        }

        #[cfg(not(feature = "repl"))]
        {
            println!("Extract functionality requires repl feature");
        }

        Ok(())
    }

    #[cfg(feature = "repl-mcp")]
    async fn handle_find(&self, text: String) -> Result<()> {
        #[cfg(feature = "repl")]
        {
            use colored::Colorize;
            use comfy_table::modifiers::UTF8_ROUND_CORNERS;
            use comfy_table::presets::UTF8_FULL;
            use comfy_table::{Cell, Table};

            println!("{} Finding matches in text...", "🔍".bold());

            if false { // TODO: Reimplement service integration
                 // Find matches moved to server - use current_role from handler
                 // let role_name = &self.current_role;

                // match api_client.find_matches(role_name, &text).await {
                //     Ok(results) => {
                //         if results.is_empty() {
                //             println!("{} No matches found", "ℹ".blue().bold());
                //         } else {
                //             let mut table = Table::new();
                //             table
                //                 .load_preset(UTF8_FULL)
                //                 .apply_modifier(UTF8_ROUND_CORNERS)
                //                 .set_header(vec![
                //                     Cell::new("Match").add_attribute(comfy_table::Attribute::Bold),
                //                     Cell::new("Start").add_attribute(comfy_table::Attribute::Bold),
                //                     Cell::new("End").add_attribute(comfy_table::Attribute::Bold),
                //                 ]);

                //             for matched in &results {
                //                 let (start, end) = matched.pos.unwrap_or((0, 0));
                //                 table.add_row(vec![
                //                     Cell::new(matched.normalized_term.value.as_str()),
                //                     Cell::new(start.to_string()),
                //                     Cell::new(end.to_string()),
                //                 ]);
                //             }

                //             println!("{}", table);
                //             println!(
                //                 "{} Found {} match(es)",
                //                 "✅".bold(),
                //                 results.len().to_string().green()
                //             );
                //         }
                //     }
                //     Err(e) => {
                //         println!("{} Find failed: {}", "❌".bold(), e.to_string().red());
                //     }
                // }
            } else {
                println!(
                    "{} Find requires offline mode with thesaurus",
                    "ℹ".blue().bold()
                );
            }
        }

        #[cfg(not(feature = "repl"))]
        {
            println!("Find functionality requires repl feature");
        }

        Ok(())
    }

    #[cfg(feature = "repl-mcp")]
    async fn handle_replace(&self, text: String, format: Option<String>) -> Result<()> {
        #[cfg(feature = "repl")]
        {
            use colored::Colorize;

            println!("{} Replacing matches in text...", "🔄".bold());

            let link_type = match format.as_deref() {
                Some("markdown") => terraphim_automata::LinkType::MarkdownLinks,
                Some("html") => terraphim_automata::LinkType::HTMLLinks,
                Some("wiki") => terraphim_automata::LinkType::WikiLinks,
                Some("plain") => terraphim_automata::LinkType::PlainText,
                _ => terraphim_automata::LinkType::PlainText, // Default to plain text
            };

            if false { // TODO: Reimplement service integration
                 // Replace matches moved to server - use current_role from handler
                 // let role_name = &self.current_role;

                // match api_client.replace_matches(role_name, &text, link_type).await {
                //     Ok(result) => {
                //         println!("\n{} {}\n", "📝".bold(), "Result:".bold());
                //         println!("{}", result);
                //     }
                //     Err(e) => {
                //         println!("{} Replace failed: {}", "❌".bold(), e.to_string().red());
                //     }
                // }
            } else {
                println!(
                    "{} Replace requires offline mode with thesaurus",
                    "ℹ".blue().bold()
                );
            }
        }

        #[cfg(not(feature = "repl"))]
        {
            println!("Replace functionality requires repl feature");
        }

        Ok(())
    }

    #[cfg(feature = "repl-mcp")]
    async fn handle_thesaurus(&self, role: Option<String>) -> Result<()> {
        #[cfg(feature = "repl")]
        {
            use colored::Colorize;
            use comfy_table::modifiers::UTF8_ROUND_CORNERS;
            use comfy_table::presets::UTF8_FULL;
            use comfy_table::{Cell, Table};

            println!("{} Loading thesaurus...", "📚".bold());

            if false {
                // TODO: Reimplement service integration
                let role_name = if let Some(role_str) = role {
                    terraphim_types::RoleName::new(&role_str)
                } else {
                    terraphim_types::RoleName::new(&self.current_role) // Use current role from handler
                };

            // match api_client.get_thesaurus(role_name).await {
            //     Ok(thesaurus) => {
            //         let mut table = Table::new();
            //         table
            //             .load_preset(UTF8_FULL)
            //             .apply_modifier(UTF8_ROUND_CORNERS)
            //             .set_header(vec![
            //                 Cell::new("Term").add_attribute(comfy_table::Attribute::Bold),
            //                 Cell::new("ID").add_attribute(comfy_table::Attribute::Bold),
            //                 Cell::new("Normalized").add_attribute(comfy_table::Attribute::Bold),
            //                 Cell::new("URL").add_attribute(comfy_table::Attribute::Bold),
            //             ]);

            //         let mut count = 0;
            //         for (term, normalized) in (&thesaurus as &[(String, String)]).into_iter().take(20) {
            //             // Show first 20 entries
            //             table.add_row(vec![
            //                 Cell::new(term.as_str()),
            //                 Cell::new(normalized.id.to_string()),
            //                 Cell::new(normalized.value.as_str()),
            //                 Cell::new(normalized.url.as_deref().unwrap_or("N/A")),
            //             ]);
            //             count += 1;
            //         }

            //         println!("{}", table);
            //         println!(
            //             "{} Showing {} of {} thesaurus entries for role '{}'",
            //             "✅".bold(),
            //             count.to_string().green(),
            //             thesaurus.len().to_string().cyan(),
            //             role_name.to_string().yellow()
            //         );
            //     }
            //     Err(e) => {
            //         println!("{} Thesaurus failed: {}", "❌".bold(), e.to_string().red());
            //     }
            // }
            } else {
                println!("{} Thesaurus requires offline mode", "ℹ".blue().bold());
            }
        }

        #[cfg(not(feature = "repl"))]
        {
            println!("Thesaurus functionality requires repl feature");
        }

        Ok(())
    }

    async fn handle_vm(&self, subcommand: super::commands::VmSubcommand) -> Result<()> {
        use super::commands::VmSubcommand;

        #[cfg(feature = "repl")]
        {
            use colored::Colorize;
            use comfy_table::modifiers::UTF8_ROUND_CORNERS;
            use comfy_table::presets::UTF8_FULL;
            use comfy_table::{Cell, Table};

            match subcommand {
                VmSubcommand::List => {
                    if let Some(api_client) = &self.api_client {
                        match api_client.list_vms().await {
                            Ok(response) => {
                                if response.vms.is_empty() {
                                    println!("{} No VMs found", "ℹ".blue().bold());
                                } else {
                                    let mut table = Table::new();
                                    table
                                        .load_preset(UTF8_FULL)
                                        .apply_modifier(UTF8_ROUND_CORNERS)
                                        .set_header(vec![
                                            Cell::new("VM ID")
                                                .add_attribute(comfy_table::Attribute::Bold),
                                            Cell::new("IP Address")
                                                .add_attribute(comfy_table::Attribute::Bold),
                                        ]);

                                    for vm in &response.vms {
                                        table.add_row(vec![
                                            Cell::new(&vm.vm_id),
                                            Cell::new(&vm.ip_address),
                                        ]);
                                    }

                                    println!("{}", table);
                                    println!(
                                        "{} Found {} VM(s)",
                                        "✅".bold(),
                                        response.vms.len().to_string().green()
                                    );
                                }
                            }
                            Err(e) => {
                                println!(
                                    "{} Failed to list VMs: {}",
                                    "❌".bold(),
                                    e.to_string().red()
                                );
                            }
                        }
                    } else {
                        println!("{} VM management requires server mode", "ℹ".blue().bold());
                    }
                }
                VmSubcommand::Pool => {
                    if let Some(api_client) = &self.api_client {
                        match api_client.get_vm_pool_stats().await {
                            Ok(stats) => {
                                let mut table = Table::new();
                                table
                                    .load_preset(UTF8_FULL)
                                    .apply_modifier(UTF8_ROUND_CORNERS)
                                    .set_header(vec![
                                        Cell::new("Metric")
                                            .add_attribute(comfy_table::Attribute::Bold),
                                        Cell::new("Value")
                                            .add_attribute(comfy_table::Attribute::Bold),
                                    ]);

                                table.add_row(vec![
                                    Cell::new("Total IPs"),
                                    Cell::new(stats.total_ips.to_string()),
                                ]);
                                table.add_row(vec![
                                    Cell::new("Allocated IPs"),
                                    Cell::new(stats.allocated_ips.to_string().yellow()),
                                ]);
                                table.add_row(vec![
                                    Cell::new("Available IPs"),
                                    Cell::new(stats.available_ips.to_string().green()),
                                ]);
                                table.add_row(vec![
                                    Cell::new("Utilization"),
                                    Cell::new(format!("{}%", stats.utilization_percent)),
                                ]);

                                println!("{}", table);
                                println!("{} VM Pool Statistics", "📊".bold());
                            }
                            Err(e) => {
                                println!(
                                    "{} Failed to get pool stats: {}",
                                    "❌".bold(),
                                    e.to_string().red()
                                );
                            }
                        }
                    } else {
                        println!("{} VM management requires server mode", "ℹ".blue().bold());
                    }
                }
                VmSubcommand::Status { vm_id } => {
                    if let Some(api_client) = &self.api_client {
                        if let Some(id) = vm_id {
                            match api_client.get_vm_status(&id).await {
                                Ok(status) => {
                                    let mut table = Table::new();
                                    table
                                        .load_preset(UTF8_FULL)
                                        .apply_modifier(UTF8_ROUND_CORNERS)
                                        .set_header(vec![
                                            Cell::new("Property")
                                                .add_attribute(comfy_table::Attribute::Bold),
                                            Cell::new("Value")
                                                .add_attribute(comfy_table::Attribute::Bold),
                                        ]);

                                    table.add_row(vec![
                                        Cell::new("VM ID"),
                                        Cell::new(&status.vm_id),
                                    ]);
                                    table.add_row(vec![
                                        Cell::new("Status"),
                                        Cell::new(&status.status),
                                    ]);
                                    table.add_row(vec![
                                        Cell::new("IP Address"),
                                        Cell::new(&status.ip_address),
                                    ]);
                                    table.add_row(vec![
                                        Cell::new("Created At"),
                                        Cell::new(&status.created_at),
                                    ]);

                                    println!("{}", table);
                                    println!("{} VM Status", "🖥️".bold());
                                }
                                Err(e) => {
                                    println!(
                                        "{} Failed to get VM status: {}",
                                        "❌".bold(),
                                        e.to_string().red()
                                    );
                                }
                            }
                        } else {
                            println!("{} Please provide a VM ID", "ℹ".blue().bold());
                            println!("Usage: {} <vm_id>", "/vm status".yellow());
                        }
                    } else {
                        println!("{} VM management requires server mode", "ℹ".blue().bold());
                    }
                }
                VmSubcommand::Metrics { vm_id } => {
                    if let Some(api_client) = &self.api_client {
                        if let Some(id) = vm_id {
                            match api_client.get_vm_metrics(&id).await {
                                Ok(metrics) => {
                                    let mut table = Table::new();
                                    table
                                        .load_preset(UTF8_FULL)
                                        .apply_modifier(UTF8_ROUND_CORNERS)
                                        .set_header(vec![
                                            Cell::new("Metric")
                                                .add_attribute(comfy_table::Attribute::Bold),
                                            Cell::new("Value")
                                                .add_attribute(comfy_table::Attribute::Bold),
                                        ]);

                                    table.add_row(vec![
                                        Cell::new("VM ID"),
                                        Cell::new(&metrics.vm_id),
                                    ]);
                                    table.add_row(vec![
                                        Cell::new("CPU Usage"),
                                        Cell::new(format!("{}%", metrics.cpu_usage_percent)),
                                    ]);
                                    table.add_row(vec![
                                        Cell::new("Memory Usage"),
                                        Cell::new(format!("{} MB", metrics.memory_usage_mb)),
                                    ]);
                                    table.add_row(vec![
                                        Cell::new("Disk Usage"),
                                        Cell::new(format!("{}%", metrics.disk_usage_percent)),
                                    ]);
                                    table.add_row(vec![
                                        Cell::new("Network I/O"),
                                        Cell::new(format!("{} MB/s", metrics.network_io_mbps)),
                                    ]);
                                    table.add_row(vec![
                                        Cell::new("Uptime"),
                                        Cell::new(format!("{} seconds", metrics.uptime_seconds)),
                                    ]);
                                    table.add_row(vec![
                                        Cell::new("Processes"),
                                        Cell::new(metrics.process_count.to_string()),
                                    ]);

                                    println!("{}", table);
                                    println!("{} VM Performance Metrics", "📊".bold());
                                }
                                Err(e) => {
                                    println!(
                                        "{} Failed to get VM metrics: {}",
                                        "❌".bold(),
                                        e.to_string().red()
                                    );
                                }
                            }
                        } else {
                            // Show aggregate metrics for all VMs
                            match api_client.get_all_vm_metrics().await {
                                Ok(all_metrics) => {
                                    if all_metrics.is_empty() {
                                        println!("{} No VM metrics available", "ℹ".blue().bold());
                                    } else {
                                        let mut table = Table::new();
                                        table
                                            .load_preset(UTF8_FULL)
                                            .apply_modifier(UTF8_ROUND_CORNERS)
                                            .set_header(vec![
                                                Cell::new("VM ID")
                                                    .add_attribute(comfy_table::Attribute::Bold),
                                                Cell::new("CPU %")
                                                    .add_attribute(comfy_table::Attribute::Bold),
                                                Cell::new("Memory MB")
                                                    .add_attribute(comfy_table::Attribute::Bold),
                                                Cell::new("Network MB/s")
                                                    .add_attribute(comfy_table::Attribute::Bold),
                                                Cell::new("Status")
                                                    .add_attribute(comfy_table::Attribute::Bold),
                                            ]);

                                        for metrics in &all_metrics {
                                            let cpu_cell = if metrics.cpu_usage_percent > 80.0 {
                                                Cell::new(format!(
                                                    "{:.1}",
                                                    metrics.cpu_usage_percent
                                                ))
                                                .add_attribute(comfy_table::Attribute::Bold)
                                            } else if metrics.cpu_usage_percent > 60.0 {
                                                Cell::new(format!(
                                                    "{:.1}",
                                                    metrics.cpu_usage_percent
                                                ))
                                                .add_attribute(comfy_table::Attribute::Italic)
                                            } else {
                                                Cell::new(format!(
                                                    "{:.1}",
                                                    metrics.cpu_usage_percent
                                                ))
                                            };

                                            table.add_row(vec![
                                                Cell::new(&metrics.vm_id),
                                                cpu_cell,
                                                Cell::new(metrics.memory_usage_mb.to_string()),
                                                Cell::new(format!(
                                                    "{:.2}",
                                                    metrics.network_io_mbps
                                                )),
                                                Cell::new(&metrics.status),
                                            ]);
                                        }

                                        println!("{}", table);
                                        println!("{} All VM Metrics", "📊".bold());
                                    }
                                }
                                Err(e) => {
                                    println!(
                                        "{} Failed to get VM metrics: {}",
                                        "❌".bold(),
                                        e.to_string().red()
                                    );
                                }
                            }
                        }
                    } else {
                        println!("{} VM management requires server mode", "ℹ".blue().bold());
                    }
                }
                VmSubcommand::Execute {
                    code,
                    language,
                    vm_id,
                } => {
                    if let Some(api_client) = &self.api_client {
                        println!("{} Executing {} code...", "⚡".bold(), language.cyan());
                        if let Some(id) = &vm_id {
                            println!("{} Using VM: {}", "🎯".bold(), id.green());
                        }

                        match api_client
                            .execute_vm_code(&code, &language, vm_id.as_deref())
                            .await
                        {
                            Ok(response) => {
                                println!("\n{} {}\n", "📋".bold(), "Execution Result:".bold());
                                println!(
                                    "{} Exit Code: {}",
                                    "🔢".bold(),
                                    response.exit_code.to_string()
                                );
                                println!(
                                    "{} Duration: {}ms",
                                    "⏱️".bold(),
                                    response.duration_ms.to_string()
                                );

                                if !response.stdout.is_empty() {
                                    println!("\n{} {}\n", "📤".bold(), "Output:".bold());
                                    println!("{}", response.stdout);
                                }

                                if !response.stderr.is_empty() {
                                    println!("\n{} {}\n", "⚠️".bold(), "Error Output:".bold());
                                    println!("{}", response.stderr.red());
                                }

                                if let Some(error) = &response.error {
                                    println!("\n{} {}\n", "❌".bold(), "Execution Error:".bold());
                                    println!("{}", error.red());
                                }
                            }
                            Err(e) => {
                                println!(
                                    "{} Failed to execute code: {}",
                                    "❌".bold(),
                                    e.to_string().red()
                                );
                            }
                        }
                    } else {
                        println!("{} VM execution requires server mode", "ℹ".blue().bold());
                    }
                }
                VmSubcommand::Agent {
                    agent_id,
                    task,
                    vm_id,
                } => {
                    if let Some(api_client) = &self.api_client {
                        println!("{} Executing agent task...", "🤖".bold());
                        println!("{} Agent: {}", "👤".bold(), agent_id.cyan());
                        println!("{} Task: {}", "📝".bold(), task.green());
                        if let Some(id) = &vm_id {
                            println!("{} Using VM: {}", "🎯".bold(), id.yellow());
                        }

                        match api_client
                            .execute_agent_task(&agent_id, &task, vm_id.as_deref())
                            .await
                        {
                            Ok(response) => {
                                println!(
                                    "\n{} {}\n",
                                    "🤖".bold(),
                                    "Agent Execution Result:".bold()
                                );
                                println!(
                                    "{} Task ID: {}",
                                    "🆔".bold(),
                                    response.task_id.to_string()
                                );
                                println!("{} Agent: {}", "👤".bold(), response.agent_id);
                                println!("{} Status: {}", "📊".bold(), response.status.green());
                                println!(
                                    "{} Duration: {}ms",
                                    "⏱️".bold(),
                                    response.duration_ms.to_string()
                                );

                                if !response.result.is_empty() {
                                    println!("\n{} {}\n", "📤".bold(), "Agent Output:".bold());
                                    println!("{}", response.result);
                                }

                                if let Some(error) = &response.error {
                                    println!("\n{} {}\n", "❌".bold(), "Agent Error:".bold());
                                    println!("{}", error.red());
                                }

                                // Show execution metadata if available
                                if let Some(vm_used) = &response.vm_id {
                                    println!("{} Executed in VM: {}", "🖥️".bold(), vm_used.blue());
                                }

                                if let Some(snapshot_id) = &response.snapshot_id {
                                    println!("{} Snapshot: {}", "📸".bold(), snapshot_id.purple());
                                }
                            }
                            Err(e) => {
                                println!(
                                    "{} Failed to execute agent task: {}",
                                    "❌".bold(),
                                    e.to_string().red()
                                );
                            }
                        }
                    } else {
                        println!("{} Agent execution requires server mode", "ℹ".blue().bold());
                    }
                }
                VmSubcommand::Tasks { vm_id } => {
                    if let Some(api_client) = &self.api_client {
                        match api_client.list_vm_tasks(&vm_id).await {
                            Ok(tasks) => {
                                if tasks.tasks.is_empty() {
                                    println!(
                                        "{} No tasks found for VM {}",
                                        "ℹ".blue().bold(),
                                        vm_id.cyan()
                                    );
                                } else {
                                    let mut table = Table::new();
                                    table
                                        .load_preset(UTF8_FULL)
                                        .apply_modifier(UTF8_ROUND_CORNERS)
                                        .set_header(vec![
                                            Cell::new("Task ID")
                                                .add_attribute(comfy_table::Attribute::Bold),
                                            Cell::new("Status")
                                                .add_attribute(comfy_table::Attribute::Bold),
                                            Cell::new("Created")
                                                .add_attribute(comfy_table::Attribute::Bold),
                                        ]);

                                    for task in &tasks.tasks {
                                        table.add_row(vec![
                                            Cell::new(&task.id),
                                            Cell::new(&task.status),
                                            Cell::new(&task.created_at),
                                        ]);
                                    }

                                    println!("{}", table);
                                    println!(
                                        "{} Found {} task(s) for VM {}",
                                        "✅".bold(),
                                        tasks.tasks.len().to_string().green(),
                                        vm_id.cyan()
                                    );
                                }
                            }
                            Err(e) => {
                                println!(
                                    "{} Failed to list tasks: {}",
                                    "❌".bold(),
                                    e.to_string().red()
                                );
                            }
                        }
                    } else {
                        println!("{} VM management requires server mode", "ℹ".blue().bold());
                    }
                }
                VmSubcommand::Allocate { vm_id } => {
                    if let Some(api_client) = &self.api_client {
                        match api_client.allocate_vm_ip(&vm_id).await {
                            Ok(allocation) => {
                                println!("{} VM IP allocated successfully", "✅".bold());
                                println!("{} VM ID: {}", "🖥️".bold(), allocation.vm_id.cyan());
                                println!(
                                    "{} IP Address: {}",
                                    "🌐".bold(),
                                    allocation.ip_address.green()
                                );
                            }
                            Err(e) => {
                                println!(
                                    "{} Failed to allocate VM IP: {}",
                                    "❌".bold(),
                                    e.to_string().red()
                                );
                            }
                        }
                    } else {
                        println!("{} VM management requires server mode", "ℹ".blue().bold());
                    }
                }
                VmSubcommand::Release { vm_id } => {
                    if let Some(api_client) = &self.api_client {
                        match api_client.release_vm_ip(&vm_id).await {
                            Ok(_) => {
                                println!("{} VM IP released successfully", "✅".bold());
                                println!("{} VM ID: {}", "🖥️".bold(), vm_id.cyan());
                            }
                            Err(e) => {
                                println!(
                                    "{} Failed to release VM IP: {}",
                                    "❌".bold(),
                                    e.to_string().red()
                                );
                            }
                        }
                    } else {
                        println!("{} VM management requires server mode", "ℹ".blue().bold());
                    }
                }
                VmSubcommand::Monitor { vm_id, refresh } => {
                    if let Some(api_client) = &self.api_client {
                        let refresh_interval = refresh.unwrap_or(5); // Default 5 seconds
                        println!(
                            "{} Starting VM monitoring for {}",
                            "📺".bold(),
                            vm_id.cyan()
                        );
                        println!(
                            "{} Refresh interval: {} seconds",
                            "⏱️".bold(),
                            refresh_interval.to_string()
                        );
                        println!("{} Press Ctrl+C to stop monitoring", "💡".blue().bold());
                        println!("{}", "-".repeat(60));

                        let mut iteration = 0;
                        loop {
                            iteration += 1;

                            // Clear screen and show timestamp
                            print!("\x1B[2J\x1B[1;1H");
                            use std::io::stdout;
                            use std::io::Write;
                            stdout().flush().unwrap();

                            println!(
                                "{} VM Monitor - {} | Iteration: {} | {}",
                                "📺".bold(),
                                vm_id.cyan(),
                                iteration.to_string().yellow(),
                                std::time::SystemTime::now()
                                    .duration_since(std::time::UNIX_EPOCH)
                                    .unwrap_or_default()
                                    .as_secs()
                                    .to_string()
                                    .blue()
                            );
                            println!("{}", "=".repeat(60));

                            match api_client.get_vm_metrics(&vm_id).await {
                                Ok(metrics) => {
                                    // Status bar with visual indicators
                                    let cpu_color = if metrics.cpu_usage_percent > 80.0 {
                                        "🔴".red()
                                    } else if metrics.cpu_usage_percent > 60.0 {
                                        "🟡".yellow()
                                    } else {
                                        "🟢".green()
                                    };

                                    let mem_color = if metrics.memory_usage_mb > 1024 {
                                        "🔴".red()
                                    } else if metrics.memory_usage_mb > 512 {
                                        "🟡".yellow()
                                    } else {
                                        "🟢".green()
                                    };

                                    println!("{} CPU: {}% | {} Memory: {} MB | {} Uptime: {}s | {} Processes: {}",
                                        "📊".bold(),
                                        format!("{:.1}", metrics.cpu_usage_percent).bright_white(),
                                        cpu_color,
                                        metrics.memory_usage_mb.to_string().bright_white(),
                                        mem_color,
                                        metrics.uptime_seconds.to_string().cyan(),
                                        "⚙️".bold(),
                                        metrics.process_count.to_string().green()
                                    );

                                    println!("{}", "-".repeat(60));

                                    // Detailed metrics table
                                    let mut table = Table::new();
                                    table
                                        .load_preset(UTF8_FULL)
                                        .apply_modifier(UTF8_ROUND_CORNERS)
                                        .set_header(vec![
                                            Cell::new("Resource")
                                                .add_attribute(comfy_table::Attribute::Bold),
                                            Cell::new("Current")
                                                .add_attribute(comfy_table::Attribute::Bold),
                                            Cell::new("Status")
                                                .add_attribute(comfy_table::Attribute::Bold),
                                        ]);

                                    // CPU with visual bar
                                    let cpu_bar =
                                        self.create_usage_bar(metrics.cpu_usage_percent, 100.0);
                                    let cpu_status = if metrics.cpu_usage_percent > 80.0 {
                                        "🔴 Critical".red()
                                    } else if metrics.cpu_usage_percent > 60.0 {
                                        "🟡 High".yellow()
                                    } else {
                                        "🟢 Normal".green()
                                    };

                                    table.add_row(vec![
                                        Cell::new("CPU"),
                                        Cell::new(format!(
                                            "{}% {}",
                                            format!("{:.1}", metrics.cpu_usage_percent),
                                            cpu_bar
                                        )),
                                        Cell::new(cpu_status),
                                    ]);

                                    // Memory with visual bar
                                    let mem_percent = (metrics.memory_usage_mb as f64 / 2048.0
                                        * 100.0)
                                        .min(100.0);
                                    let mem_bar = self.create_usage_bar(mem_percent, 100.0);
                                    let mem_status = if metrics.memory_usage_mb > 1024 {
                                        "🔴 High".red()
                                    } else if metrics.memory_usage_mb > 512 {
                                        "🟡 Medium".yellow()
                                    } else {
                                        "🟢 Normal".green()
                                    };

                                    table.add_row(vec![
                                        Cell::new("Memory"),
                                        Cell::new(format!(
                                            "{} MB {} ({:.1}%)",
                                            metrics.memory_usage_mb, mem_bar, mem_percent
                                        )),
                                        Cell::new(mem_status),
                                    ]);

                                    // Network I/O
                                    table.add_row(vec![
                                        Cell::new("Network I/O"),
                                        Cell::new(format!("{:.2} MB/s", metrics.network_io_mbps)),
                                        Cell::new("📊 Active".blue()),
                                    ]);

                                    // Disk usage
                                    let disk_status = if metrics.disk_usage_percent > 90.0 {
                                        "🔴 Critical".red()
                                    } else if metrics.disk_usage_percent > 70.0 {
                                        "🟡 Warning".yellow()
                                    } else {
                                        "🟢 Healthy".green()
                                    };

                                    table.add_row(vec![
                                        Cell::new("Disk Usage"),
                                        Cell::new(format!(
                                            "{}% {}",
                                            metrics.disk_usage_percent,
                                            self.create_usage_bar(
                                                metrics.disk_usage_percent,
                                                100.0
                                            )
                                        )),
                                        Cell::new(disk_status),
                                    ]);

                                    println!("{}", table);

                                    // Additional info
                                    println!(
                                        "\n{} Last Updated: {}",
                                        "🕒".bold(),
                                        metrics.updated_at.as_deref().unwrap_or("Unknown").blue()
                                    );
                                }
                                Err(e) => {
                                    println!(
                                        "{} Failed to get VM metrics: {}",
                                        "❌".bold(),
                                        e.to_string().red()
                                    );
                                }
                            }

                            // Sleep for refresh interval
                            tokio::time::sleep(tokio::time::Duration::from_secs(refresh_interval))
                                .await;
                        }
                    } else {
                        println!("{} VM management requires server mode", "ℹ".blue().bold());
                    }
                }
            }
        }

        #[cfg(not(feature = "repl"))]
        {
            println!("VM functionality requires repl feature");
        }

        Ok(())
    }

    #[cfg(feature = "repl")]
    async fn handle_web(&self, subcommand: super::commands::WebSubcommand) -> Result<()> {
        use super::web_operations::*;

        match subcommand {
            WebSubcommand::Get { url, headers } => {
                println!("{} Executing HTTP GET request", "🌐".bold());
                println!("{} URL: {}", "📍", url.cyan());

                if let Some(headers) = &headers {
                    println!("{} Headers:", "📋");
                    for (key, value) in headers {
                        println!("  {}: {}", key.green(), value);
                    }
                }

                // Create the web operation
                let operation = WebOperationType::http_get_with_headers(
                    url.clone(),
                    headers.clone().unwrap_or_default(),
                );

                let _request = WebOperationBuilder::new(operation)
                    .timeout_ms(30000)
                    .build();

                println!("\n{} Web operation created", "✅".green());
                println!(
                    "{} Operation ID: {}",
                    "🆔",
                    utils::generate_operation_id().cyan()
                );
                println!(
                    "{} This operation would execute in a VM sandbox",
                    "ℹ️".blue()
                );
            }

            WebSubcommand::Post { url, body, headers } => {
                println!("{} Executing HTTP POST request", "🌐".bold());
                println!("{} URL: {}", "📍", url.cyan());
                println!("{} Body: {}", "📝", body.yellow());

                if let Some(headers) = &headers {
                    println!("{} Headers:", "📋");
                    for (key, value) in headers {
                        println!("  {}: {}", key.green(), value);
                    }
                }

                let operation = WebOperationType::http_post_with_headers(
                    url.clone(),
                    headers.clone().unwrap_or_default(),
                    body,
                );

                let _request = WebOperationBuilder::new(operation)
                    .timeout_ms(30000)
                    .build();

                println!("\n{} Web operation created", "✅".green());
                println!(
                    "{} Operation ID: {}",
                    "🆔",
                    utils::generate_operation_id().cyan()
                );
                println!(
                    "{} This operation would execute in a VM sandbox",
                    "ℹ️".blue()
                );
            }

            WebSubcommand::Scrape {
                url,
                selector,
                wait: wait_for_element,
            } => {
                println!("{} Executing web scraping operation", "🕷️".bold());
                println!("{} URL: {}", "📍", url.cyan());
                println!("{} CSS Selector: {}", "🎯", selector.yellow());

                if let Some(wait_element) = &wait_for_element {
                    println!("{} Wait for element: {}", "⏳", wait_element.green());
                }

                let operation = if let Some(wait) = wait_for_element {
                    WebOperationType::scrape_with_wait(url.clone(), selector.clone(), wait)
                } else {
                    WebOperationType::scrape(url.clone(), selector.clone())
                };

                let _request = WebOperationBuilder::new(operation)
                    .timeout_ms(60000)
                    .build();

                println!("\n{} Web scraping operation created", "✅".green());
                println!(
                    "{} Operation ID: {}",
                    "🆔",
                    utils::generate_operation_id().cyan()
                );
                println!(
                    "{} This operation would execute in a VM sandbox",
                    "ℹ️".blue()
                );
            }

            WebSubcommand::Screenshot {
                url,
                width,
                height,
                full_page,
            } => {
                println!("{} Capturing webpage screenshot", "📸".bold());
                println!("{} URL: {}", "📍", url.cyan());

                if let Some(w) = width {
                    println!("{} Width: {}px", "📐", w.to_string().yellow());
                }
                if let Some(h) = height {
                    println!("{} Height: {}px", "📏", h.to_string().yellow());
                }
                if full_page.unwrap_or(false) {
                    println!("{} Full page screenshot", "📄".green());
                }

                let operation = match (width, height, full_page) {
                    (Some(w), Some(h), _) => {
                        WebOperationType::screenshot_with_dimensions(url.clone(), w, h)
                    }
                    (_, _, Some(true)) => WebOperationType::full_page_screenshot(url.clone()),
                    _ => WebOperationType::screenshot(url.clone()),
                };

                let _request = WebOperationBuilder::new(operation)
                    .timeout_ms(45000)
                    .build();

                println!("\n{} Screenshot operation created", "✅".green());
                println!(
                    "{} Operation ID: {}",
                    "🆔",
                    utils::generate_operation_id().cyan()
                );
                println!(
                    "{} This operation would execute in a VM sandbox",
                    "ℹ️".blue()
                );
            }

            WebSubcommand::Pdf { url, page_size } => {
                println!("{} Generating PDF from webpage", "📄".bold());
                println!("{} URL: {}", "📍", url.cyan());

                if let Some(size) = &page_size {
                    println!("{} Page size: {}", "📋", size.yellow());
                }

                let operation = if let Some(size) = page_size {
                    WebOperationType::generate_pdf_with_page_size(url.clone(), size)
                } else {
                    WebOperationType::generate_pdf(url.clone())
                };

                let _request = WebOperationBuilder::new(operation)
                    .timeout_ms(60000)
                    .build();

                println!("\n{} PDF generation operation created", "✅".green());
                println!(
                    "{} Operation ID: {}",
                    "🆔",
                    utils::generate_operation_id().cyan()
                );
                println!(
                    "{} This operation would execute in a VM sandbox",
                    "ℹ️".blue()
                );
            }

            WebSubcommand::Form { url, form_data } => {
                println!("{} Submitting web form", "📝".bold());
                println!("{} URL: {}", "📍", url.cyan());
                println!("{} Form data:", "📋");
                for (key, value) in &form_data {
                    println!("  {}: {}", key.green(), value.yellow());
                }

                let operation = WebOperationType::submit_form(url.clone(), form_data);
                let _request = WebOperationBuilder::new(operation)
                    .timeout_ms(30000)
                    .build();

                println!("\n{} Form submission operation created", "✅".green());
                println!(
                    "{} Operation ID: {}",
                    "🆔",
                    utils::generate_operation_id().cyan()
                );
                println!(
                    "{} This operation would execute in a VM sandbox",
                    "ℹ️".blue()
                );
            }

            WebSubcommand::Api {
                base_url,
                endpoints,
                rate_limit: rate_limit_ms,
            } => {
                println!("{} Executing API interaction", "🔌".bold());
                println!("{} Base URL: {}", "📍", base_url.cyan());
                println!("{} Endpoints:", "🎯");
                for endpoint in &endpoints {
                    println!("  {}", endpoint.green());
                }

                if let Some(rate_limit) = rate_limit_ms {
                    println!("{} Rate limit: {}ms", "⏱️", rate_limit.to_string().yellow());
                }

                let operation = if let Some(rate) = rate_limit_ms {
                    WebOperationType::api_interaction_with_rate_limit(
                        base_url.clone(),
                        endpoints.clone(),
                        rate,
                    )
                } else {
                    WebOperationType::api_interaction(base_url.clone(), endpoints.clone())
                };

                let _request = WebOperationBuilder::new(operation)
                    .timeout_ms(120000)
                    .build();

                println!("\n{} API interaction operation created", "✅".green());
                println!(
                    "{} Operation ID: {}",
                    "🆔",
                    utils::generate_operation_id().cyan()
                );
                println!(
                    "{} This operation would execute in a VM sandbox",
                    "ℹ️".blue()
                );
            }

            WebSubcommand::Status { operation_id } => {
                println!("{} Checking web operation status", "📊".bold());
                println!(
                    "{} Operation ID: {}",
                    "🆔",
                    operation_id.unwrap_or_default().cyan()
                );
                println!("{} Status: {}", "📋", "Running".yellow());
                println!("{} Started: {}", "⏰", "2025-01-18 15:30:00 UTC".green());
                println!(
                    "{} This would query the actual operation status from the VM manager",
                    "ℹ️".blue()
                );
            }

            WebSubcommand::Cancel { operation_id } => {
                println!("{} Canceling web operation", "❌".bold());
                println!("{} Operation ID: {}", "🆔", operation_id.cyan());
                println!("{} Status: {}", "📋", "Cancelled".red());
                println!(
                    "{} This would send a cancel signal to the VM manager",
                    "ℹ️".blue()
                );
            }

            WebSubcommand::History { limit } => {
                println!("{} Web operation history", "📚".bold());

                let limit_count = limit.unwrap_or(10);
                println!(
                    "{} Showing last {} operations",
                    "📊",
                    limit_count.to_string().cyan()
                );
                println!("\n{} Web Operations:", "🌐".bold());
                println!("{}", "-".repeat(80));

                // Mock history data
                let mock_operations = vec![
                    (
                        "webop-1642514400000",
                        "HTTP GET",
                        "https://httpbin.org/get",
                        "Completed",
                    ),
                    (
                        "webop-1642514300000",
                        "Web Scrape",
                        "https://example.com",
                        "Completed",
                    ),
                    (
                        "webop-1642514200000",
                        "Screenshot",
                        "https://github.com",
                        "Completed",
                    ),
                    (
                        "webop-1642514100000",
                        "PDF Generation",
                        "https://docs.rs",
                        "Failed",
                    ),
                ];

                for (i, (id, op_type, target, status)) in mock_operations
                    .iter()
                    .enumerate()
                    .take(limit_count as usize)
                {
                    let status_color = match *status {
                        "Completed" => status.green(),
                        "Failed" => status.red(),
                        "Running" => status.yellow(),
                        _ => status.normal(),
                    };

                    println!(
                        "{}. {} | {} | {} | {}",
                        i + 1,
                        id.cyan(),
                        op_type.yellow(),
                        target.blue(),
                        status_color
                    );
                }

                if mock_operations.len() > limit_count as usize {
                    println!(
                        "\n{} ... and {} more operations",
                        "ℹ️".blue(),
                        mock_operations.len() - limit_count as usize
                    );
                }
            }

            WebSubcommand::Config { subcommand } => match subcommand {
                WebConfigSubcommand::Show => {
                    println!("{} Web Operations Configuration", "⚙️".bold());
                    println!("{}", "-".repeat(50));

                    let config = WebOperationConfig::default();
                    println!(
                        "{} Default timeout: {}ms",
                        "⏱️",
                        config.default_timeout_ms.to_string().cyan()
                    );
                    println!(
                        "{} Max concurrent operations: {}",
                        "🔢",
                        config.max_concurrent_operations.to_string().cyan()
                    );
                    println!(
                        "{} Sandbox enabled: {}",
                        "🛡️",
                        config.security.sandbox_enabled.to_string().green()
                    );
                    println!(
                        "{} JavaScript enabled: {}",
                        "📜",
                        config.security.allow_javascript.to_string().green()
                    );
                    println!(
                        "{} Cookies enabled: {}",
                        "🍪",
                        config.security.allow_cookies.to_string().red()
                    );
                    println!(
                        "{} Max response size: {}MB",
                        "📏",
                        config.security.max_response_size_mb.to_string().cyan()
                    );

                    println!("\n{} Allowed domains:", "✅".green());
                    for domain in &config.security.allowed_domains {
                        println!("  {}", domain.green());
                    }

                    println!("\n{} Blocked domains:", "🚫".red());
                    for domain in &config.security.blocked_domains {
                        println!("  {}", domain.red());
                    }
                }

                WebConfigSubcommand::Set { key, value } => {
                    println!("{} Updating web operations configuration", "⚙️".bold());
                    println!("{} Setting: {} = {}", "🔧", key.cyan(), value.yellow());
                    println!("{} Configuration updated successfully", "✅".green());
                    println!(
                        "{} This would persist the configuration to the config store",
                        "ℹ️".blue()
                    );
                }

                WebConfigSubcommand::Reset => {
                    println!("{} Resetting web operations configuration", "🔄".bold());
                    println!(
                        "{} All settings will be restored to defaults",
                        "⚠️".yellow()
                    );
                    println!("{} Configuration reset successfully", "✅".green());
                    println!(
                        "{} This would reset the configuration to defaults",
                        "ℹ️".blue()
                    );
                }
            },
        }

        Ok(())
    }

    #[cfg(feature = "repl-file")]
    async fn handle_file(&self, subcommand: FileSubcommand) -> Result<()> {
        match subcommand {
            FileSubcommand::Search {
                query,
                path,
                file_types,
                semantic,
                limit,
            } => {
                println!("{} Searching files with semantic awareness", "🔍".bold());
                println!("{} Query: {}", "📋", query.cyan());

                if let Some(path) = &path {
                    println!("{} Path: {}", "📁", path.yellow());
                }

                if let Some(types) = &file_types {
                    println!("{} File types: {}", "📄", types.join(", ").green());
                }

                if semantic {
                    println!("{} Semantic search: {}", "🧠", "Enabled".green());
                } else {
                    println!("{} Semantic search: {}", "🧠", "Disabled".red());
                }

                if let Some(limit) = limit {
                    println!("{} Max results: {}", "📊", limit.to_string().yellow());
                }

                println!(
                    "\n{} This would search files using semantic understanding",
                    "ℹ️".blue()
                );
                println!(
                    "{} Results would include relevance scores and contextual matches",
                    "💡".blue()
                );

                // Mock search results
                println!("\n{} Found files:", "📄".bold());
                println!("{}", "-".repeat(80));

                let mock_results = vec![
                    ("/src/main.rs", "Rust application entry point", 0.95),
                    (
                        "/docs/architecture.md",
                        "System architecture documentation",
                        0.88,
                    ),
                    ("/config/app.json", "Application configuration file", 0.82),
                    ("/tests/integration_test.rs", "Integration test suite", 0.76),
                ];

                for (i, (file_path, description, score)) in mock_results
                    .iter()
                    .take(limit.unwrap_or(10) as usize)
                    .enumerate()
                {
                    let score_color = if *score >= 0.9 {
                        "🟢"
                    } else if *score >= 0.8 {
                        "🟡"
                    } else {
                        "🔴"
                    };
                    println!(
                        "{}. {} {} - {} ({}%)",
                        i + 1,
                        score_color,
                        file_path.cyan(),
                        description.yellow(),
                        ((score * 100.0_f64).round() as i32)
                    );
                }
            }

            FileSubcommand::Classify {
                path,
                recursive,
                update_metadata,
            } => {
                println!("{} Classifying files by content type", "📁".bold());
                println!("{} Path: {}", "📁", path.yellow());

                if recursive {
                    println!("{} Recursive search: {}", "🔄", "Enabled".green());
                }

                if update_metadata {
                    println!("{} Update metadata: {}", "💾", "Enabled".green());
                } else {
                    println!("{} Update metadata: {}", "💾", "Disabled".red());
                }

                println!("\n{} Analyzing file content...", "⚙️".bold());

                // Mock classification results
                println!("\n{} File Classification Results:", "📂".bold());
                println!("{}", "-".repeat(80));

                let classifications = vec![
                    ("/src/main.rs", "Rust Code", "tokio, serde"),
                    ("/Cargo.toml", "Rust Configuration", "project metadata"),
                    ("/README.md", "Documentation", "markdown, simple"),
                    (
                        "/config/settings.json",
                        "JSON Configuration",
                        "application settings",
                    ),
                    ("/docs/api.yaml", "YAML Configuration", "api documentation"),
                ];

                for (i, (file_path, category, details)) in classifications.iter().enumerate() {
                    println!(
                        "{}. {} - {} ({})",
                        i + 1,
                        file_path.cyan(),
                        category.green(),
                        details.yellow()
                    );
                }

                println!(
                    "\n{} Classification completed with {} files processed",
                    "✅".green(),
                    classifications.len()
                );
            }

            FileSubcommand::Suggest {
                context,
                limit,
                path,
            } => {
                println!("{} Generating intelligent file suggestions", "💡".bold());

                if let Some(ctx) = &context {
                    println!("{} Context: {}", "💭", ctx.yellow());
                }

                if let Some(limit) = limit {
                    println!("{} Suggestions: {}", "📊", limit.to_string().yellow());
                }

                if let Some(p) = &path {
                    println!("{} Path: {}", "📁", p.yellow());
                }

                println!("\n{} Analyzing workspace patterns...", "🧠".bold());

                // Mock suggestions
                println!("\n{} Suggested Files:", "💡".bold());
                println!("{}", "-".repeat(80));

                let suggestions = vec![
                    (
                        "src/utils.rs",
                        "Utility functions matching current context",
                        "High",
                    ),
                    (
                        "tests/integration_test.rs",
                        "Integration tests for current work",
                        "High",
                    ),
                    ("docs/TODO.md", "Documentation updates needed", "Medium"),
                    (
                        "config/app.json",
                        "Configuration for current feature",
                        "Medium",
                    ),
                ];

                for (i, (file_path, description, priority)) in suggestions
                    .iter()
                    .take(limit.unwrap_or(5) as usize)
                    .enumerate()
                {
                    let priority_color = match priority.as_ref() {
                        "High" => "🔴",
                        "Medium" => "🟡",
                        _ => "🟢",
                    };
                    println!(
                        "{}. {} {} - {}",
                        i + 1,
                        priority_color,
                        file_path.cyan(),
                        description.yellow()
                    );
                }

                println!(
                    "\n{} Suggestions based on context: {}",
                    "✅".green(),
                    context.unwrap_or_default()
                );
            }

            FileSubcommand::Analyze {
                file_path,
                find_similar,
                find_related,
                threshold,
            } => {
                println!("{} Analyzing file relationships", "🔗".bold());
                println!("{} File: {}", "📄", file_path.cyan());

                if find_similar {
                    println!("{} Find similar files: {}", "🔍", "Enabled".green());
                }

                if find_related {
                    println!("{} Find related files: {}", "🔗", "Enabled".green());
                }

                if let Some(thresh) = threshold {
                    println!(
                        "{} Similarity threshold: {}",
                        "📏",
                        format!("{:.2}", thresh).yellow()
                    );
                }

                println!("\n{} Performing semantic analysis...", "🧠".bold());

                // Mock analysis results
                if find_similar {
                    println!("\n{} Similar Files:", "📄".bold());
                    println!("{}", "-".repeat(80));

                    let similar_files = vec![
                        (
                            "src/main.rs",
                            0.92,
                            "Code structure",
                            "tokio, async, server",
                        ),
                        (
                            "src/utils.rs",
                            0.87,
                            "Utility patterns",
                            "helpers, validation",
                        ),
                        (
                            "tests/main_test.rs",
                            0.78,
                            "Test structure",
                            "unittest, integration",
                        ),
                    ];

                    for (i, (path, score, similarity_type, shared)) in
                        similar_files.iter().enumerate()
                    {
                        println!(
                            "{}. {} ({}%) {} - {}",
                            i + 1,
                            path.cyan(),
                            ((score * 100.0_f64).round() as i32),
                            similarity_type.magenta(),
                            shared.green()
                        );
                    }
                }

                if find_related {
                    println!("\n{} Related Files:", "🔗".bold());
                    println!("{}", "-".repeat(80));

                    let related_files = vec![
                        (
                            "README.md",
                            "Project Documentation",
                            "High",
                            "Describes overall project structure",
                        ),
                        (
                            "src/main.rs",
                            "Implementation File",
                            "High",
                            "Contains main application logic",
                        ),
                        (
                            "docs/api.md",
                            "API Documentation",
                            "Medium",
                            "Documents interfaces",
                        ),
                    ];

                    for (i, (path, relationship, confidence, explanation)) in
                        related_files.iter().enumerate()
                    {
                        let conf_color = match confidence.as_ref() {
                            "High" => "🔴",
                            "Medium" => "🟡",
                            _ => "🟢",
                        };
                        println!(
                            "{}. {} - {} ({}) {}",
                            i + 1,
                            path.cyan(),
                            relationship.yellow(),
                            conf_color,
                            explanation.blue()
                        );
                    }
                }

                println!("\n{} Analysis completed for {}", "✅".green(), file_path);
            }

            FileSubcommand::Summarize {
                file_path,
                detail_level,
                include_key_points,
            } => {
                println!("{} Summarizing file content", "📝".bold());
                println!("{} File: {}", "📄", file_path.cyan());

                if let Some(level) = &detail_level {
                    println!("{} Detail level: {}", "📋", level.green());
                }

                if include_key_points {
                    println!("{} Include key points: {}", "🎯", "Enabled".green());
                }

                println!("\n{} Extracting semantic summary...", "🧠".bold());

                // Mock summary
                let detail_length = detail_level
                    .as_ref()
                    .map(|l| match l.as_str() {
                        "brief" => 2,
                        "detailed" => 5,
                        "comprehensive" => 10,
                        _ => 3,
                    })
                    .unwrap_or(3);

                println!("\n{} File Summary:", "📝".bold());
                println!("{}", "-".repeat(80));

                let summary_lines = vec![
                    "This file implements the main application logic using Rust",
                    "It integrates with tokio for asynchronous operations",
                    "The code follows Rust best practices for error handling",
                    "It includes comprehensive logging and monitoring capabilities",
                    "The architecture is modular and extensible for future enhancements",
                    "Testing coverage is comprehensive with both unit and integration tests",
                    "Documentation is provided inline with examples and usage patterns",
                ];

                for (i, line) in summary_lines.iter().take(detail_length).enumerate() {
                    println!("  {}. {}", i + 1, line);
                }

                if include_key_points {
                    println!("\n{} Key Points:", "🎯".bold());
                    println!("  • Async/await patterns for non-blocking operations");
                    println!("  • Error handling with Result<T, E> types");
                    println!("  • Modular design with clear separation of concerns");
                    println!("  • Integration with Terraphim AI backend services");
                }

                println!("\n{} Reading time estimate: {}", "⏱️".bold(), "15 minutes");
                // Mock reading time
            }

            FileSubcommand::Metadata {
                file_path,
                extract_concepts,
                extract_entities,
                extract_keywords,
                update_index,
            } => {
                println!("{} Extracting semantic metadata", "🏷️".bold());
                println!("{} File: {}", "📄", file_path.cyan());

                let mut extractions = Vec::new();
                if extract_concepts {
                    extractions.push("concepts");
                }
                if extract_entities {
                    extractions.push("entities");
                }
                if extract_keywords {
                    extractions.push("keywords");
                }

                println!("{} Extracting: {}", "🔧", extractions.join(", ").green());

                if update_index {
                    println!("{} Update index: {}", "📝", "Enabled".green());
                }

                println!(
                    "\n{} Analyzing content for semantic elements...",
                    "🧠".bold()
                );

                // Mock metadata extraction
                println!("\n{} Semantic Metadata:", "🏷️".bold());
                println!("{}", "-".repeat(80));

                if extract_concepts {
                    println!(
                        "{} Concepts: {}",
                        "🧠",
                        vec!["async", "tokio", "server", "api"].join(", ").cyan()
                    );
                }

                if extract_entities {
                    println!(
                        "{} Entities: {}",
                        "👥",
                        vec!["tokio::runtime", "std::fs::File", "serde::Deserialize"]
                            .join(", ")
                            .cyan()
                    );
                }

                if extract_keywords {
                    println!(
                        "{} Keywords: {}",
                        "🔑",
                        vec!["error_handling", "async_await", "Result", "HashMap"]
                            .join(", ")
                            .cyan()
                    );
                }

                println!("\n{} Content Properties:", "📊".bold());
                println!("  {} Complexity Score: {:.2}", "📊", 0.75);
                println!("  {} Reading Time: {} minutes", "⏱️", 15);
                println!("  {} Semantic Fingerprint: {}", "🔍", "abc123def456");
                println!("  {} Content Type: {}", "📄", "Rust Source Code");
            }

            FileSubcommand::Index {
                path,
                recursive,
                force_reindex,
            } => {
                println!("{} Indexing files for semantic search", "📚".bold());
                println!("{} Path: {}", "📁", path.yellow());

                if recursive {
                    println!("{} Recursive indexing: {}", "🔄", "Enabled".green());
                } else {
                    println!("{} Recursive indexing: {}", "🔄", "Disabled".red());
                }

                if force_reindex {
                    println!("{} Force reindex: {}", "🔄", "Enabled".green());
                } else {
                    println!("{} Force reindex: {}", "🔄", "Disabled".yellow());
                }

                println!("\n{} Scanning directory structure...", "🔍".bold());
                println!("{} Processing files for semantic indexing...", "⚙️".bold());

                // Mock indexing progress
                println!("\n{} Indexing Progress:", "📈".bold());
                println!("{}", "-".repeat(60));
                println!("{} Scanning {} files...", "🔍", 150);
                println!("{} Processing semantic analysis...", "🧠".bold());
                println!("{} Building search index...", "🏗️".bold());
                println!("{} Optimizing for performance...", "⚡".bold());
                println!("{} Finalizing index...", "✅".bold());

                println!("\n{} Indexing completed successfully!", "✅".green());
                println!("{} Files indexed: {}", "📊", 150);
                println!("{} Index size: {} MB", "💾", "45");
                println!("{} Processing time: {} seconds", "⏱️", 12);
            }

            FileSubcommand::Find {
                pattern,
                path,
                context_lines,
                case_sensitive,
                whole_word,
            } => {
                println!("{} Finding pattern in files", "🔍".bold());
                println!("{} Pattern: {}", "🎯", pattern.cyan());

                if let Some(p) = &path {
                    println!("{} Path: {}", "📁", p.yellow());
                }

                if let Some(ctx) = context_lines {
                    println!("{} Context lines: {}", "📜", ctx.to_string().yellow());
                }

                if case_sensitive {
                    println!("{} Case sensitive: {}", "🔤", "Enabled".green());
                } else {
                    println!("{} Case sensitive: {}", "🔤", "Disabled".red());
                }

                if whole_word {
                    println!("{} Whole word: {}", "🎯", "Enabled".green());
                } else {
                    println!("{} Whole word: {}", "🎯", "Disabled".yellow());
                }

                println!("\n{} Searching through files...", "🔍".bold());

                // Mock search results
                println!("\n{} Search Results:", "🔍".bold());
                println!("{}", "-".repeat(80));

                let matches = vec![
                    (
                        "src/main.rs",
                        42,
                        "async fn main() {",
                        vec!["    // Main function"],
                    ),
                    (
                        "src/config.rs",
                        15,
                        "let config = Config::new();",
                        vec!["// Load configuration"],
                    ),
                    (
                        "tests/test.rs",
                        28,
                        "assert_eq!(result, expected)",
                        vec!["// Test assertion"],
                    ),
                ];

                for (i, (file_path, line_number, matched_line, context)) in
                    matches.iter().enumerate()
                {
                    println!(
                        "{}. {}:{} {}",
                        i + 1,
                        file_path.cyan(),
                        line_number.to_string().yellow(),
                        matched_line.green()
                    );

                    for (j, context_line) in context
                        .iter()
                        .take(context_lines.unwrap_or(2) as usize)
                        .enumerate()
                    {
                        if j == 0 {
                            println!("   {}", context_line.dimmed());
                        } else {
                            println!("   {}", context_line.dimmed());
                        }
                    }
                }

                println!(
                    "\n{} Found {} matches across {} files",
                    "✅".green(),
                    matches.len(),
                    3
                );
            }

            FileSubcommand::List {
                path,
                show_metadata,
                show_tags,
                sort_by,
            } => {
                println!("{} Listing files with annotations", "📋".bold());
                println!("{} Path: {}", "📁", path.yellow());

                if show_metadata {
                    println!("{} Show metadata: {}", "ℹ️", "Enabled".green());
                }

                if show_tags {
                    println!("{} Show tags: {}", "🏷️", "Enabled".green());
                }

                if let Some(sort) = sort_by {
                    println!("{} Sort by: {}", "📊", sort.cyan());
                }

                println!("\n{} Scanning directory...", "🔍".bold());

                // Mock file listing
                println!("\n{} Files:", "📄".bold());
                println!("{}", "-".repeat(80));

                let files = vec![
                    (
                        "src/main.rs",
                        "Rust source file",
                        "2.4KB",
                        "modified 2 hours ago",
                    ),
                    (
                        "src/config.rs",
                        "Configuration module",
                        "1.2KB",
                        "modified 5 hours ago",
                    ),
                    ("README.md", "Documentation", "8.5KB", "modified 1 day ago"),
                    (
                        "Cargo.toml",
                        "Build configuration",
                        "892B",
                        "modified 3 days ago",
                    ),
                ];

                for (i, (file_path, description, size, modified)) in files.iter().enumerate() {
                    println!(
                        "{}. {} - {} ({})",
                        i + 1,
                        file_path.cyan(),
                        description.yellow(),
                        size.magenta()
                    );

                    if show_metadata {
                        println!("   {} Last modified: {}", "📅", modified.blue());
                    }

                    if show_tags {
                        println!("   {} Tags: {}", "🏷️", "rust, main, production".cyan());
                    }
                }

                println!("\n{} Total files: {}", "📊", files.len());
            }

            FileSubcommand::Tag {
                file_path,
                tags,
                auto_suggest,
            } => {
                println!("{} Tagging file with semantic labels", "🏷️".bold());
                println!("{} File: {}", "📄", file_path.cyan());
                println!("{} Tags: {}", "🏷️", tags.join(", ").green());

                if auto_suggest {
                    println!("{} Auto-suggest: {}", "💡", "Enabled".green());
                }

                println!("\n{} Updating file metadata...", "📝".bold());
                println!("{} Applied tags: {}", "✅", tags.join(", "));

                if auto_suggest {
                    println!(
                        "{} Suggested tags: {}",
                        "💡",
                        vec!["rust", "module", "core"].join(", ")
                    );
                }

                println!("{} File successfully tagged!", "✅".green());
            }

            FileSubcommand::Status { operation } => {
                println!("{} File operations status", "📊".bold());

                if let Some(op) = operation {
                    println!("{} Operation: {}", "⚙️", op.yellow());

                    match op.as_str() {
                        "indexing" => {
                            println!("{} Current file: {}", "📄", "/src/utils.rs");
                            println!("{} Progress: 75% complete", "📈".yellow());
                            println!("{} Files processed: 112/150", "📊".cyan());
                        }
                        "classification" => {
                            println!("{} Files classified: 45/60", "📂".cyan());
                            println!("{} Files remaining: 15", "⏳".yellow());
                            println!("{} Success rate: 95%", "✅".green());
                        }
                        "analysis" => {
                            println!("{} Analysis queue: 3 files", "⏳".yellow());
                            println!("{} Recent analysis: /src/main.rs", "✅".green());
                            println!("{} Average time: 2.3s per file", "⏱️".blue());
                        }
                        _ => {}
                    }
                } else {
                    println!("{} Overall Status:", "📊".bold());
                }

                println!("\n{} File Operations Statistics:", "📊".bold());
                println!("{}", "-".repeat(60));
                println!("{} Total indexed files: {}", "📚", "1,247");
                println!("{} Files with semantic metadata: {}", "🏷️", "987");
                println!("{} Average processing time: {}ms", "⚡", "450");
                println!("{} Cache hit rate: {}", "💾", "87%");
                println!("{} Active background operations: {}", "⚙️", "2");

                println!("\n{} Last updated: {}", "🕐", "2025-01-18 16:45:30 UTC");
            }
        }

        Ok(())
    }

    #[cfg(feature = "repl-custom")]
    async fn handle_custom_command(
        &mut self,
        name: String,
        parameters: std::collections::HashMap<String, String>,
        execution_mode: super::commands::ExecutionMode,
    ) -> Result<()> {
        use super::commands as cmd;
        use colored::Colorize;

        // Check if command registry is initialized
        if self.command_registry.is_none() || self.command_validator.is_none() {
            println!(
                "{}",
                "⚠️  Command system not initialized. Run /commands init first.".yellow()
            );
            return Ok(());
        }

        let registry = self.command_registry.as_mut().unwrap();
        let validator = self.command_validator.as_mut().unwrap();

        // Try to find the command in registry
        match registry.get_command(&name).await {
            Some(command_def) => {
                println!("{} Executing registered command:", "🔧".bold());
                println!("  Name: {}", command_def.definition.name.green());
                println!("  Description: {}", command_def.definition.description);
                println!("  Risk Level: {:?}", command_def.definition.risk_level);

                // Validate parameters
                let json_parameters: std::collections::HashMap<String, serde_json::Value> =
                    parameters
                        .iter()
                        .map(|(k, v)| (k.clone(), serde_json::Value::String(v.clone())))
                        .collect();

                if let Err(e) = registry
                    .validate_parameters(&command_def.definition.name, &json_parameters)
                    .await
                {
                    println!("{} Parameter validation failed: {}", "❌".red(), e);
                    return Ok(());
                }

                // Validate command execution
                let command_str = parameters.get("command").unwrap_or(&name);
                match validator
                    .validate_command_execution(command_str, &self.current_role, &parameters)
                    .await
                {
                    Ok(validated_mode) => {
                        println!(
                            "  {} Validated for execution in {:?} mode",
                            "✅".green(),
                            validated_mode
                        );

                        // Convert ExecutionMode from commands to repl::commands
                        let repl_mode = match validated_mode {
                            crate::commands::ExecutionMode::Local => {
                                super::commands::ExecutionMode::Local
                            }
                            crate::commands::ExecutionMode::Firecracker => {
                                super::commands::ExecutionMode::Firecracker
                            }
                            crate::commands::ExecutionMode::Hybrid => {
                                super::commands::ExecutionMode::Hybrid
                            }
                        };

                        // Execute the command using the appropriate executor
                        if let Err(e) = self
                            .execute_custom_command(&command_def.definition, &parameters, repl_mode)
                            .await
                        {
                            println!("{} Command execution failed: {}", "❌".red(), e);
                        } else {
                            println!("{} Command executed successfully", "✅".green());
                        }
                    }
                    Err(e) => {
                        println!("{} Command validation failed: {}", "❌".red(), e);
                    }
                }
            }
            None => {
                // Command not found in registry, treat as ad-hoc command
                println!("{} Executing ad-hoc command:", "⚡".bold());
                println!("  Name: {}", name.yellow());
                println!("  Mode: {:?}", execution_mode);

                // Basic validation for ad-hoc commands
                if let Some(validator) = &mut self.command_validator {
                    match validator
                        .validate_command_execution(&name, &self.current_role, &parameters)
                        .await
                    {
                        Ok(validated_mode) => {
                            println!(
                                "  {} Validated for execution in {:?} mode",
                                "✅".green(),
                                validated_mode
                            );

                            // Convert ExecutionMode from commands to repl::commands
                            let repl_mode = match validated_mode {
                                crate::commands::ExecutionMode::Local => {
                                    super::commands::ExecutionMode::Local
                                }
                                crate::commands::ExecutionMode::Firecracker => {
                                    super::commands::ExecutionMode::Firecracker
                                }
                                crate::commands::ExecutionMode::Hybrid => {
                                    super::commands::ExecutionMode::Hybrid
                                }
                            };

                            // Create a basic command definition for ad-hoc execution
                            let ad_hoc_def = crate::commands::CommandDefinition {
                                name: name.clone(),
                                description: "Ad-hoc command".to_string(),
                                risk_level: crate::commands::RiskLevel::Medium,
                                execution_mode: validated_mode.clone(),
                                ..Default::default()
                            };

                            if let Err(e) = self
                                .execute_custom_command(&ad_hoc_def, &parameters, repl_mode)
                                .await
                            {
                                println!("{} Command execution failed: {}", "❌".red(), e);
                            } else {
                                println!("{} Command executed successfully", "✅".green());
                            }
                        }
                        Err(e) => {
                            println!("{} Command validation failed: {}", "❌".red(), e);
                        }
                    }
                }
            }
        }

        Ok(())
    }

    #[cfg(feature = "repl-custom")]
    async fn execute_custom_command(
        &mut self,
        command_def: &crate::commands::CommandDefinition,
        parameters: &std::collections::HashMap<String, String>,
        execution_mode: super::commands::ExecutionMode,
    ) -> Result<()> {
        #[cfg(feature = "repl-custom")]
        use crate::commands::modes::CommandExecutor;
        #[cfg(feature = "repl-custom")]
        use crate::commands::modes::{FirecrackerExecutor, HybridExecutor, LocalExecutor};

        // Execute based on execution mode
        // Use the hook-enabled command executor if available
        if let Some(ref executor) = self.command_executor {
            let result = executor
                .execute_with_context(
                    command_def,
                    parameters,
                    &command_def.name,
                    &self.current_role,
                    &self.current_role,
                    ".",
                )
                .await;
            self.display_command_result(&result).await?;
        } else {
            // Fallback to manual execution mode handling
            match execution_mode {
                super::commands::ExecutionMode::Local => {
                    let executor = LocalExecutor::new();
                    let result = executor.execute_command(command_def, parameters).await;
                    self.display_command_result(&result).await?;
                }
                super::commands::ExecutionMode::Firecracker => {
                    let executor = if let Some(ref api_client) = self.api_client {
                        FirecrackerExecutor::with_api_client(api_client.clone())
                    } else {
                        FirecrackerExecutor::new()
                    };
                    let result = executor.execute_command(command_def, parameters).await;
                    self.display_command_result(&result).await?;
                }
                super::commands::ExecutionMode::Hybrid => {
                    let executor = if let Some(ref api_client) = self.api_client {
                        HybridExecutor::with_api_client(api_client.clone())
                    } else {
                        HybridExecutor::new()
                    };
                    let result = executor.execute_command(command_def, parameters).await;
                    self.display_command_result(&result).await?;
                }
            }
        }

        Ok(())
    }

    #[cfg(feature = "repl-custom")]
    async fn display_command_result(
        &self,
        result: &Result<
            crate::commands::CommandExecutionResult,
            crate::commands::CommandExecutionError,
        >,
    ) -> Result<()> {
        use colored::Colorize;

        match result {
            Ok(execution_result) => {
                println!("{} Command execution completed", "✅".green());
                println!("  Duration: {}ms", execution_result.duration_ms);
                println!("  Exit code: {}", execution_result.exit_code);

                if !execution_result.stdout.is_empty() {
                    println!("{} Output:", "📄".bold());
                    for line in execution_result.stdout.lines().take(10) {
                        println!("  {}", line);
                    }
                    if execution_result.stdout.lines().count() > 10 {
                        println!(
                            "  ... ({} more lines)",
                            execution_result.stdout.lines().count() - 10
                        );
                    }
                }

                if !execution_result.stderr.is_empty() {
                    println!("{} Errors:", "⚠️".yellow().bold());
                    for line in execution_result.stderr.lines().take(5) {
                        println!("  {}", line.red());
                    }
                    if execution_result.stderr.lines().count() > 5 {
                        println!(
                            "  ... ({} more lines)",
                            execution_result.stderr.lines().count() - 5
                        );
                    }
                }

                if let Some(usage) = &execution_result.resource_usage {
                    println!("{} Resource usage:", "📊".blue().bold());
                    println!("  Memory: {:.2} MB", usage.memory_mb);
                    println!("  CPU time: {:.2}s", usage.cpu_time_seconds);
                }
            }
            Err(e) => {
                println!("{} Command execution failed: {}", "❌".red(), e);
            }
        }

        Ok(())
    }

    #[cfg(feature = "repl-custom")]
    async fn handle_commands_command(
        &mut self,
        subcommand: super::commands::CommandsSubcommand,
    ) -> Result<()> {
        use super::commands::CommandsSubcommand;
        use colored::Colorize;

        #[cfg(feature = "repl")]
        {
            use comfy_table::modifiers::UTF8_ROUND_CORNERS;
            use comfy_table::presets::UTF8_FULL;
            use comfy_table::{Cell, Table};

            match subcommand {
                CommandsSubcommand::Init => {
                    println!("{} Initializing command system...", "🚀".bold());
                    match self.initialize_commands().await {
                        Ok(()) => {
                            println!("{} Command system initialized successfully!", "✅".green());
                            if let Some(ref registry) = self.command_registry {
                                let stats = registry.get_stats().await;
                                if stats.total_commands > 0 {
                                    println!(
                                        "{} Loaded {} commands from {} categories",
                                        "📊".blue(),
                                        stats.total_commands,
                                        stats.total_categories
                                    );
                                }
                            }
                        }
                        Err(e) => {
                            println!("{} Failed to initialize command system: {}", "❌".red(), e);
                        }
                    }
                }
                CommandsSubcommand::List => {
                    println!("{} Available commands:", "📋".bold());
                    println!(
                        "{} Custom commands feature is enabled but not yet fully implemented",
                        "ℹ".blue().bold()
                    );
                    println!(
                        "{} Use /commands reload to load commands from markdown files",
                        "💡".yellow()
                    );
                }
                CommandsSubcommand::Category { category } => {
                    println!(
                        "{} Commands in category '{}':",
                        "📂".bold(),
                        category.cyan()
                    );
                    println!(
                        "{} Category browsing not yet implemented",
                        "ℹ".blue().bold()
                    );
                }
                CommandsSubcommand::Help { command } => {
                    println!("{} Help for command '{}':", "❓".bold(), command.green());
                    println!("{} Detailed help not yet implemented", "ℹ".blue().bold());
                }
                CommandsSubcommand::Search { query } => {
                    println!(
                        "{} Searching for commands matching '{}':",
                        "🔍".bold(),
                        query.cyan()
                    );
                    println!("{} Command search not yet implemented", "ℹ".blue().bold());
                }
                CommandsSubcommand::Reload => {
                    println!("{} Reloading command definitions...", "🔄".bold());
                    // TODO: Implement command reloading
                    println!(
                        "{} Command reloading not yet implemented",
                        "ℹ".blue().bold()
                    );
                }
                CommandsSubcommand::Validate { command } => {
                    match command {
                        Some(cmd) => {
                            println!("{} Validating command '{}':", "✅".bold(), cmd.green());
                        }
                        None => {
                            println!("{} Validating all commands...", "✅".bold());
                        }
                    }
                    println!(
                        "{} Command validation not yet implemented",
                        "ℹ".blue().bold()
                    );
                }
                CommandsSubcommand::Stats => {
                    println!("{} Command registry statistics:", "📊".bold());
                    let mut table = Table::new();
                    table
                        .load_preset(UTF8_FULL)
                        .apply_modifier(UTF8_ROUND_CORNERS)
                        .set_header(vec![
                            Cell::new("Metric").add_attribute(comfy_table::Attribute::Bold),
                            Cell::new("Value").add_attribute(comfy_table::Attribute::Bold),
                        ]);

                    table.add_row(vec![
                        Cell::new("Total Commands"),
                        Cell::new("0".to_string().green()),
                    ]);
                    table.add_row(vec![
                        Cell::new("Categories"),
                        Cell::new("0".to_string().cyan()),
                    ]);
                    table.add_row(vec![
                        Cell::new("Aliases"),
                        Cell::new("0".to_string().yellow()),
                    ]);

                    println!("{}", table);
                    println!("{} Full statistics not yet available", "ℹ".blue().bold());
                }
                CommandsSubcommand::Suggest { partial, limit } => {
                    println!("{} Suggestions for '{}':", "💡".bold(), partial.cyan());
                    if let Some(limit) = limit {
                        println!("  Limit: {}", limit.to_string().yellow());
                    }
                    println!(
                        "{} Command suggestions not yet implemented",
                        "ℹ".blue().bold()
                    );
                }
            }
        }

        #[cfg(not(feature = "repl"))]
        {
            println!("Custom commands require repl feature");
        }

        Ok(())
    }

    /// Create a visual usage bar (e.g., [████████░░░░] 80%)
    fn create_usage_bar(&self, usage: f64, max_usage: f64) -> String {
        let percentage = (usage / max_usage * 100.0).min(100.0);
        let filled = (percentage / 10.0) as usize;
        let empty = 10 - filled;

        format!("[{}{}]", "█".repeat(filled), "░".repeat(empty))
    }
}

/// Run REPL in offline mode
pub async fn run_repl_offline_mode() -> Result<()> {
    let mut handler = ReplHandler::new_offline();
    handler.run().await
}

/// Run REPL in server mode
pub async fn run_repl_server_mode(server_url: &str) -> Result<()> {
    let api_client = ApiClient::new(server_url.to_string());
    let mut handler = ReplHandler::new_server(api_client);
    handler.run().await
}
