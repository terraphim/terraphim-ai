//! REPL handler implementation

#[cfg(feature = "repl-sessions")]
use super::commands::SessionsSubcommand;
use super::commands::{
    ConfigSubcommand, ReplCommand, RobotSubcommand, RoleSubcommand, UpdateSubcommand,
};
use crate::{client::ApiClient, service::TuiService};

// Import robot module types
use crate::robot::{ExitCode, SelfDocumentation};
use anyhow::Result;
use std::io::{self, Write};
use std::str::FromStr;

#[cfg(feature = "repl")]
use rustyline::Editor;

#[cfg(feature = "repl")]
use colored::Colorize;

pub struct ReplHandler {
    service: Option<TuiService>,
    api_client: Option<ApiClient>,
    current_role: String,
}

impl ReplHandler {
    pub fn new_offline(service: TuiService) -> Self {
        Self {
            service: Some(service),
            api_client: None,
            current_role: "Default".to_string(),
        }
    }

    pub fn new_server(api_client: ApiClient) -> Self {
        Self {
            service: None,
            api_client: Some(api_client),
            current_role: "Terraphim Engineer".to_string(),
        }
    }

    #[cfg(feature = "repl")]
    pub async fn run(&mut self) -> Result<()> {
        use rustyline::completion::{Completer, Pair};
        use rustyline::highlight::Highlighter;
        use rustyline::hint::Hinter;
        use rustyline::validate::Validator;
        use rustyline::{Context, Helper};

        // Create a command completer
        #[derive(Clone)]
        struct CommandCompleter;

        impl Helper for CommandCompleter {}
        impl Hinter for CommandCompleter {
            type Hint = String;

            fn hint(&self, _line: &str, _pos: usize, _ctx: &Context<'_>) -> Option<String> {
                None
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
                    let commands = ReplCommand::available_commands();

                    let matches: Vec<Pair> = commands
                        .into_iter()
                        .filter(|cmd| cmd.starts_with(prefix))
                        .map(|cmd| Pair {
                            display: format!("/{}", cmd),
                            replacement: format!("/{}", cmd),
                        })
                        .collect();

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
        rl.set_helper(Some(CommandCompleter));

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

        let mode = if self.service.is_some() {
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

        println!(
            "  {} - Manage updates (check, install, rollback, list)",
            "/update <subcommand>".yellow()
        );
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

            #[cfg(feature = "repl-file")]
            ReplCommand::File { subcommand } => {
                self.handle_file(subcommand).await?;
            }

            #[cfg(feature = "repl-web")]
            ReplCommand::Web { subcommand } => {
                self.handle_web(subcommand).await?;
            }

            ReplCommand::Vm { subcommand } => {
                self.handle_vm(subcommand).await?;
            }

            ReplCommand::Robot { subcommand } => {
                self.handle_robot(subcommand).await?;
            }

            #[cfg(feature = "repl-sessions")]
            ReplCommand::Sessions { subcommand } => {
                self.handle_sessions(subcommand).await?;
            }

            ReplCommand::Update { subcommand } => {
                self.handle_update(subcommand).await?;
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

            let search_mode = if semantic { "semantic " } else { "" }.to_string()
                + if concepts { "concepts " } else { "" };

            println!(
                "{} {}Searching for: '{}'",
                "🔍".bold(),
                search_mode,
                query.cyan()
            );

            if let Some(service) = &self.service {
                // Offline mode
                let role_name = if let Some(role) = role {
                    terraphim_types::RoleName::new(&role)
                } else {
                    service.get_selected_role().await
                };

                let results = service.search_with_role(&query, &role_name, limit).await?;

                if results.is_empty() {
                    println!("{} No results found", "ℹ".blue().bold());
                } else {
                    let mut table = Table::new();
                    table
                        .load_preset(UTF8_FULL)
                        .apply_modifier(UTF8_ROUND_CORNERS)
                        .set_header(vec![
                            Cell::new("Rank").add_attribute(comfy_table::Attribute::Bold),
                            Cell::new("Title").add_attribute(comfy_table::Attribute::Bold),
                            Cell::new("URL").add_attribute(comfy_table::Attribute::Bold),
                        ]);

                    for doc in &results {
                        table.add_row(vec![
                            Cell::new(doc.rank.unwrap_or_default().to_string()),
                            Cell::new(&doc.title),
                            Cell::new(if doc.url.is_empty() { "N/A" } else { &doc.url }),
                        ]);
                    }

                    println!("{}", table);
                    println!(
                        "{} Found {} result(s)",
                        "✅".bold(),
                        results.len().to_string().green()
                    );
                }
            } else if let Some(api_client) = &self.api_client {
                // Server mode
                use terraphim_types::{NormalizedTermValue, RoleName, SearchQuery};

                let role_name = role.map(|r| RoleName::new(&r));
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
                            let mut table = Table::new();
                            table
                                .load_preset(UTF8_FULL)
                                .apply_modifier(UTF8_ROUND_CORNERS)
                                .set_header(vec![
                                    Cell::new("Rank").add_attribute(comfy_table::Attribute::Bold),
                                    Cell::new("Title").add_attribute(comfy_table::Attribute::Bold),
                                    Cell::new("URL").add_attribute(comfy_table::Attribute::Bold),
                                ]);

                            for doc in &response.results {
                                table.add_row(vec![
                                    Cell::new(doc.rank.unwrap_or_default().to_string()),
                                    Cell::new(&doc.title),
                                    Cell::new(if doc.url.is_empty() { "N/A" } else { &doc.url }),
                                ]);
                            }

                            println!("{}", table);
                            println!(
                                "{} Found {} result(s)",
                                "✅".bold(),
                                response.results.len().to_string().green()
                            );
                        }
                    }
                    Err(e) => {
                        println!("{} Search failed: {}", "❌".bold(), e.to_string().red());
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
                if let Some(service) = &self.service {
                    let config = service.get_config().await;
                    let config_json = serde_json::to_string_pretty(&config)?;
                    println!("{}", config_json);
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
                if let Some(service) = &self.service {
                    let roles = service.list_roles().await;
                    println!("{}", "Available roles:".bold());
                    for role in roles {
                        let marker = if role == self.current_role {
                            "▶"
                        } else {
                            " "
                        };
                        println!("  {} {}", marker.green(), role);
                    }
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
                // Update the service's selected role so search uses the new role
                if let Some(service) = &self.service {
                    let role_name = terraphim_types::RoleName::new(&name);
                    if let Err(e) = service.update_selected_role(role_name).await {
                        println!(
                            "{} Warning: Failed to update service role: {}",
                            "⚠".yellow().bold(),
                            e.to_string().yellow()
                        );
                    }
                }
                println!("{} Switched to role: {}", "✅".bold(), name.green());
            }
        }
        Ok(())
    }

    async fn handle_graph(&self, top_k: Option<usize>) -> Result<()> {
        let k = top_k.unwrap_or(10);

        if let Some(service) = &self.service {
            let role_name = service.get_selected_role().await;
            let concepts = service.get_role_graph_top_k(&role_name, k).await?;

            println!("{} Top {} concepts:", "📊".bold(), k.to_string().cyan());
            for (i, concept) in concepts.iter().enumerate() {
                println!("  {}. {}", (i + 1).to_string().yellow(), concept);
            }
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

                if let Some(service) = &self.service {
                    let role_name = service.get_selected_role().await;

                    match service.chat(&role_name, &msg, None).await {
                        Ok(response) => {
                            println!("\n{} {}\n", "🤖".bold(), "Response:".bold());
                            println!("{}", response);
                        }
                        Err(e) => {
                            println!("{} Chat failed: {}", "❌".bold(), e.to_string().red());
                        }
                    }
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

            if let Some(service) = &self.service {
                let role_name = service.get_selected_role().await;

                match service.summarize(&role_name, &target).await {
                    Ok(summary) => {
                        println!("\n{} {}\n", "📋".bold(), "Summary:".bold());
                        println!("{}", summary);
                    }
                    Err(e) => {
                        println!(
                            "{} Summarization failed: {}",
                            "❌".bold(),
                            e.to_string().red()
                        );
                    }
                }
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

            if let Some(service) = &self.service {
                let role_name = service.get_selected_role().await;

                match service.autocomplete(&role_name, &query, limit).await {
                    Ok(results) => {
                        if results.is_empty() {
                            println!("{} No autocomplete suggestions found", "ℹ".blue().bold());
                        } else {
                            let mut table = Table::new();
                            table
                                .load_preset(UTF8_FULL)
                                .apply_modifier(UTF8_ROUND_CORNERS)
                                .set_header(vec![
                                    Cell::new("Term").add_attribute(comfy_table::Attribute::Bold),
                                    Cell::new("Score").add_attribute(comfy_table::Attribute::Bold),
                                    Cell::new("URL").add_attribute(comfy_table::Attribute::Bold),
                                ]);

                            for result in &results {
                                table.add_row(vec![
                                    Cell::new(&result.term),
                                    Cell::new(format!("{:.2}", result.score)),
                                    Cell::new(result.url.as_deref().unwrap_or("N/A")),
                                ]);
                            }

                            println!("{}", table);
                            println!(
                                "{} Found {} suggestion(s)",
                                "✅".bold(),
                                results.len().to_string().green()
                            );
                        }
                    }
                    Err(e) => {
                        println!(
                            "{} Autocomplete failed: {}",
                            "❌".bold(),
                            e.to_string().red()
                        );
                    }
                }
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

            if let Some(service) = &self.service {
                let role_name = service.get_selected_role().await;

                match service
                    .extract_paragraphs(&role_name, &text, exclude_term)
                    .await
                {
                    Ok(results) => {
                        if results.is_empty() {
                            println!("{} No paragraphs found", "ℹ".blue().bold());
                        } else {
                            let mut table = Table::new();
                            table
                                .load_preset(UTF8_FULL)
                                .apply_modifier(UTF8_ROUND_CORNERS)
                                .set_header(vec![
                                    Cell::new("Term").add_attribute(comfy_table::Attribute::Bold),
                                    Cell::new("Paragraph")
                                        .add_attribute(comfy_table::Attribute::Bold),
                                ]);

                            for (term, paragraph) in &results {
                                let truncated_paragraph = if paragraph.len() > 100 {
                                    format!("{}...", &paragraph[..97])
                                } else {
                                    paragraph.clone()
                                };

                                table
                                    .add_row(vec![Cell::new(term), Cell::new(truncated_paragraph)]);
                            }

                            println!("{}", table);
                            println!(
                                "{} Found {} paragraph(s)",
                                "✅".bold(),
                                results.len().to_string().green()
                            );
                        }
                    }
                    Err(e) => {
                        println!("{} Extract failed: {}", "❌".bold(), e.to_string().red());
                    }
                }
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

            if let Some(service) = &self.service {
                let role_name = service.get_selected_role().await;

                match service.find_matches(&role_name, &text).await {
                    Ok(results) => {
                        if results.is_empty() {
                            println!("{} No matches found", "ℹ".blue().bold());
                        } else {
                            let mut table = Table::new();
                            table
                                .load_preset(UTF8_FULL)
                                .apply_modifier(UTF8_ROUND_CORNERS)
                                .set_header(vec![
                                    Cell::new("Match").add_attribute(comfy_table::Attribute::Bold),
                                    Cell::new("Start").add_attribute(comfy_table::Attribute::Bold),
                                    Cell::new("End").add_attribute(comfy_table::Attribute::Bold),
                                ]);

                            for matched in &results {
                                let (start, end) = matched.pos.unwrap_or((0, 0));
                                table.add_row(vec![
                                    Cell::new(matched.normalized_term.value.as_str()),
                                    Cell::new(start.to_string()),
                                    Cell::new(end.to_string()),
                                ]);
                            }

                            println!("{}", table);
                            println!(
                                "{} Found {} match(es)",
                                "✅".bold(),
                                results.len().to_string().green()
                            );
                        }
                    }
                    Err(e) => {
                        println!("{} Find failed: {}", "❌".bold(), e.to_string().red());
                    }
                }
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

            if let Some(service) = &self.service {
                let role_name = service.get_selected_role().await;

                match service.replace_matches(&role_name, &text, link_type).await {
                    Ok(result) => {
                        println!("\n{} {}\n", "📝".bold(), "Result:".bold());
                        println!("{}", result);
                    }
                    Err(e) => {
                        println!("{} Replace failed: {}", "❌".bold(), e.to_string().red());
                    }
                }
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

            if let Some(service) = &self.service {
                let role_name = if let Some(role_str) = role {
                    terraphim_types::RoleName::new(&role_str)
                } else {
                    service.get_selected_role().await
                };

                match service.get_thesaurus(&role_name).await {
                    Ok(thesaurus) => {
                        let mut table = Table::new();
                        table
                            .load_preset(UTF8_FULL)
                            .apply_modifier(UTF8_ROUND_CORNERS)
                            .set_header(vec![
                                Cell::new("Term").add_attribute(comfy_table::Attribute::Bold),
                                Cell::new("ID").add_attribute(comfy_table::Attribute::Bold),
                                Cell::new("Normalized").add_attribute(comfy_table::Attribute::Bold),
                                Cell::new("URL").add_attribute(comfy_table::Attribute::Bold),
                            ]);

                        let mut count = 0;
                        for (term, normalized) in (&thesaurus).into_iter().take(20) {
                            // Show first 20 entries
                            table.add_row(vec![
                                Cell::new(term.as_str()),
                                Cell::new(normalized.id.to_string()),
                                Cell::new(normalized.value.as_str()),
                                Cell::new(normalized.url.as_deref().unwrap_or("N/A")),
                            ]);
                            count += 1;
                        }

                        println!("{}", table);
                        println!(
                            "{} Showing {} of {} thesaurus entries for role '{}'",
                            "✅".bold(),
                            count.to_string().green(),
                            thesaurus.len().to_string().cyan(),
                            role_name.to_string().yellow()
                        );
                    }
                    Err(e) => {
                        println!("{} Thesaurus failed: {}", "❌".bold(), e.to_string().red());
                    }
                }
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

    #[cfg(feature = "repl-file")]
    async fn handle_file(&self, subcommand: super::commands::FileSubcommand) -> Result<()> {
        #[cfg(feature = "repl")]
        {
            use colored::Colorize;

            match subcommand {
                super::commands::FileSubcommand::Search { query } => {
                    println!("🔍 File search: {}", query.green());
                    println!("File search functionality is not yet implemented.");
                    println!("This would search for files matching: {}", query);
                }
                super::commands::FileSubcommand::List => {
                    println!("📂 File listing");
                    println!("File listing functionality is not yet implemented.");
                }
                super::commands::FileSubcommand::Info { path } => {
                    println!("ℹ️  File info: {}", path.green());
                    println!("File info functionality is not yet implemented.");
                    println!("This would show information about: {}", path);
                }
            }
        }

        #[cfg(not(feature = "repl"))]
        {
            println!("File operations require repl feature");
        }

        Ok(())
    }

    #[cfg(feature = "repl-web")]
    async fn handle_web(&self, subcommand: super::commands::WebSubcommand) -> Result<()> {
        #[cfg(feature = "repl")]
        {
            use colored::Colorize;

            match subcommand {
                super::commands::WebSubcommand::Get { url, headers } => {
                    println!("🌐 Web GET: {}", url.green());
                    if let Some(headers) = headers {
                        println!("Headers: {:?}", headers);
                    }
                    println!("Web GET functionality is not yet implemented.");
                }
                super::commands::WebSubcommand::Post { url, body, headers } => {
                    println!("🌐 Web POST: {}", url.green());
                    if !body.is_empty() {
                        println!("Body: {}", body);
                    }
                    if let Some(headers) = headers {
                        println!("Headers: {:?}", headers);
                    }
                    println!("Web POST functionality is not yet implemented.");
                }
                super::commands::WebSubcommand::Scrape {
                    url,
                    selector,
                    wait_for_element,
                } => {
                    println!("🌐 Web Scrape: {}", url.green());
                    if let Some(selector) = selector {
                        println!("Selector: {}", selector);
                    }
                    if let Some(wait) = wait_for_element {
                        println!("Wait for: {}", wait);
                    }
                    println!("Web scraping functionality is not yet implemented.");
                }
                super::commands::WebSubcommand::Screenshot {
                    url,
                    width,
                    height,
                    full_page,
                } => {
                    println!("🌐 Web Screenshot: {}", url.green());
                    if let Some(width) = width {
                        println!("Width: {}", width);
                    }
                    if let Some(height) = height {
                        println!("Height: {}", height);
                    }
                    if let Some(full_page) = full_page {
                        println!("Full page: {}", full_page);
                    }
                    println!("Web screenshot functionality is not yet implemented.");
                }
                super::commands::WebSubcommand::Pdf { url, page_size } => {
                    println!("🌐 Web PDF: {}", url.green());
                    if let Some(page_size) = page_size {
                        println!("Page size: {}", page_size);
                    }
                    println!("Web PDF functionality is not yet implemented.");
                }
                super::commands::WebSubcommand::Form { url, form_data } => {
                    println!("🌐 Web Form: {}", url.green());
                    println!("Form data: {:?}", form_data);
                    println!("Web form functionality is not yet implemented.");
                }
                super::commands::WebSubcommand::Api {
                    endpoint,
                    method,
                    data,
                } => {
                    println!("🌐 Web API: {} {}", method.bold(), endpoint.green());
                    if let Some(data) = data {
                        println!("Data: {}", data);
                    }
                    println!("Web API functionality is not yet implemented.");
                }
                super::commands::WebSubcommand::Status { operation_id } => {
                    println!("🌐 Web Status: {}", operation_id.green());
                    println!("Web status functionality is not yet implemented.");
                }
                super::commands::WebSubcommand::Cancel { operation_id } => {
                    println!("🌐 Web Cancel: {}", operation_id.green());
                    println!("Web cancel functionality is not yet implemented.");
                }
                super::commands::WebSubcommand::History { limit } => {
                    println!("🌐 Web History");
                    if let Some(limit) = limit {
                        println!("Limit: {}", limit);
                    }
                    println!("Web history functionality is not yet implemented.");
                }
                super::commands::WebSubcommand::Config {
                    subcommand: web_config_subcommand,
                } => match web_config_subcommand {
                    super::commands::WebConfigSubcommand::Show => {
                        println!("🌐 Web Config Show");
                        println!("Web config show functionality is not yet implemented.");
                    }
                    super::commands::WebConfigSubcommand::Set { key, value } => {
                        println!("🌐 Web Config Set: {} = {}", key.green(), value.green());
                        println!("Web config set functionality is not yet implemented.");
                    }
                    super::commands::WebConfigSubcommand::Reset => {
                        println!("🌐 Web Config Reset");
                        println!("Web config reset functionality is not yet implemented.");
                    }
                },
            }
        }

        #[cfg(not(feature = "repl"))]
        {
            println!("Web operations require repl feature");
        }

        Ok(())
    }

    async fn handle_vm(&self, subcommand: super::commands::VmSubcommand) -> Result<()> {
        #[cfg(feature = "repl")]
        {
            use colored::Colorize;

            match subcommand {
                super::commands::VmSubcommand::List => {
                    println!("🖥️  VM List");
                    if let Some(api_client) = &self.api_client {
                        match api_client.list_vms().await {
                            Ok(response) => {
                                println!("Available VMs:");
                                if response.vms.is_empty() {
                                    println!("  No VMs currently running");
                                } else {
                                    for vm in &response.vms {
                                        println!(
                                            "  {} ({})",
                                            vm.vm_id.bright_green(),
                                            vm.ip_address.bright_blue()
                                        );
                                    }
                                }
                            }
                            Err(e) => {
                                println!("❌ Error listing VMs: {}", e);
                            }
                        }
                    } else {
                        println!("❌ API client not available");
                    }
                }
                super::commands::VmSubcommand::Pool => {
                    println!("🏊 VM Pool Status");
                    if let Some(api_client) = &self.api_client {
                        match api_client.get_vm_pool_stats().await {
                            Ok(response) => {
                                println!("VM Pool Statistics:");
                                println!(
                                    "  Available IPs: {}",
                                    response.available_ips.to_string().bright_green()
                                );
                                println!(
                                    "  Allocated IPs: {}",
                                    response.allocated_ips.to_string().bright_yellow()
                                );
                                println!(
                                    "  Total IPs: {}",
                                    response.total_ips.to_string().bright_blue()
                                );
                                println!("  Utilization: {}%", response.utilization_percent);
                            }
                            Err(e) => {
                                println!("❌ Error getting VM pool stats: {}", e);
                            }
                        }
                    } else {
                        println!("❌ API client not available");
                    }
                }
                super::commands::VmSubcommand::Status { vm_id } => {
                    if let Some(api_client) = &self.api_client {
                        if let Some(id) = vm_id {
                            println!("📊 VM Status: {}", id.green());
                            match api_client.get_vm_status(&id).await {
                                Ok(response) => {
                                    println!("  Status: {}", response.status.bright_green());
                                    println!("  IP Address: {}", response.ip_address.bright_blue());
                                    println!("  Created: {}", response.created_at);
                                    if let Some(updated_at) = response.updated_at {
                                        println!("  Updated: {}", updated_at);
                                    }
                                }
                                Err(e) => {
                                    println!("❌ Error getting VM status: {}", e);
                                }
                            }
                        } else {
                            println!("📊 All VM Status");
                            match api_client.list_vms().await {
                                Ok(response) => {
                                    for vm in &response.vms {
                                        println!(
                                            "  {} ({})",
                                            vm.vm_id.bright_green(),
                                            vm.ip_address.bright_blue()
                                        );
                                    }
                                }
                                Err(e) => {
                                    println!("❌ Error listing VMs: {}", e);
                                }
                            }
                        }
                    } else {
                        println!("❌ API client not available");
                    }
                }
                super::commands::VmSubcommand::Metrics { vm_id } => {
                    if let Some(api_client) = &self.api_client {
                        if let Some(id) = vm_id {
                            println!("📈 VM Metrics: {}", id.green());
                            match api_client.get_vm_metrics(&id).await {
                                Ok(response) => {
                                    println!("  Status: {}", response.status.bright_green());
                                    println!("  CPU Usage: {:.1}%", response.cpu_usage_percent);
                                    println!("  Memory Usage: {} MB", response.memory_usage_mb);
                                    println!("  Disk Usage: {:.1}%", response.disk_usage_percent);
                                    println!("  Network I/O: {:.1} Mbps", response.network_io_mbps);
                                    println!("  Uptime: {} seconds", response.uptime_seconds);
                                }
                                Err(e) => {
                                    println!("❌ Error getting VM metrics: {}", e);
                                }
                            }
                        } else {
                            println!("📈 All VM Metrics");
                            match api_client.get_all_vm_metrics().await {
                                Ok(metrics) => {
                                    for metric in &metrics {
                                        println!(
                                            "  {} - CPU: {:.1}%, Memory: {} MB",
                                            metric.vm_id.bright_green(),
                                            metric.cpu_usage_percent,
                                            metric.memory_usage_mb
                                        );
                                    }
                                }
                                Err(e) => {
                                    println!("❌ Error getting all VM metrics: {}", e);
                                }
                            }
                        }
                    } else {
                        println!("❌ API client not available");
                    }
                }
                super::commands::VmSubcommand::Execute {
                    code,
                    language,
                    vm_id,
                } => {
                    if let Some(api_client) = &self.api_client {
                        if let Some(id) = vm_id {
                            println!("⚡ Execute on VM: {}", id.green());
                            println!("Language: {}", language.cyan());
                            println!("Code: {}", code.yellow());
                            match api_client
                                .execute_vm_code(&id, &code, Some(&language))
                                .await
                            {
                                Ok(response) => {
                                    println!("✅ Execution successful!");
                                    println!("  Exit Code: {}", response.exit_code);
                                    if !response.stdout.is_empty() {
                                        println!("  Output:\n{}", response.stdout.bright_green());
                                    }
                                    if !response.stderr.is_empty() {
                                        println!("  Errors:\n{}", response.stderr.bright_red());
                                    }
                                    // Note: execution_time_ms field not available in current response type
                                    // if let Some(execution_time) = response.execution_time_ms {
                                    //     println!("  Execution Time: {}ms", execution_time);
                                    // }
                                }
                                Err(e) => {
                                    println!("❌ Error executing code: {}", e);
                                }
                            }
                        } else {
                            println!("⚡ Execute on default VM");
                            println!("Language: {}", language.cyan());
                            println!("Code: {}", code.yellow());
                            println!(
                                "💡 Default VM execution not yet implemented. Please specify a VM ID with --vm-id"
                            );
                        }
                    } else {
                        println!("❌ API client not available");
                    }
                }
                super::commands::VmSubcommand::Agent {
                    agent_id,
                    task,
                    vm_id,
                } => {
                    if let Some(api_client) = &self.api_client {
                        if let Some(id) = vm_id {
                            println!("🤖 Agent: {} on VM: {}", agent_id.green(), id.cyan());
                            println!("Task: {}", task.yellow());
                            match api_client
                                .execute_agent_task(&id, &agent_id, Some(&task))
                                .await
                            {
                                Ok(response) => {
                                    println!("✅ Agent task executed successfully!");
                                    println!("  Task ID: {}", response.task_id.bright_green());
                                    println!("  Status: {}", response.status.bright_yellow());
                                    if !response.result.is_empty() {
                                        println!("  Result:\n{}", response.result.bright_blue());
                                    }
                                }
                                Err(e) => {
                                    println!("❌ Error executing agent task: {}", e);
                                }
                            }
                        } else {
                            println!("🤖 Agent: {} on default VM", agent_id.green());
                            println!("Task: {}", task.yellow());
                            println!(
                                "💡 Default VM agent execution not yet implemented. Please specify a VM ID with --vm-id"
                            );
                        }
                    } else {
                        println!("❌ API client not available");
                    }
                }
                super::commands::VmSubcommand::Tasks { vm_id } => {
                    if let Some(api_client) = &self.api_client {
                        println!("📋 Tasks for VM: {}", vm_id.green());
                        match api_client.list_vm_tasks(&vm_id).await {
                            Ok(response) => {
                                if response.tasks.is_empty() {
                                    println!("  No tasks found for VM");
                                } else {
                                    for task in &response.tasks {
                                        println!(
                                            "  {} - {}",
                                            task.id.bright_green(),
                                            task.status.bright_yellow()
                                        );
                                        // Note: agent_type field not available in current VmTask type
                                        // task.agent_type.bright_blue()
                                        println!("    Created: {}", task.created_at);
                                    }
                                }
                            }
                            Err(e) => {
                                println!("❌ Error listing VM tasks: {}", e);
                            }
                        }
                    } else {
                        println!("❌ API client not available");
                    }
                }
                super::commands::VmSubcommand::Allocate { vm_id } => {
                    if let Some(api_client) = &self.api_client {
                        println!("➕ Allocate VM: {}", vm_id.green());
                        match api_client.allocate_vm_ip(&vm_id).await {
                            Ok(response) => {
                                println!("✅ VM allocated successfully!");
                                println!("  VM ID: {}", response.vm_id.bright_green());
                                println!("  IP Address: {}", response.ip_address.bright_blue());
                                // Note: pool_id field not available in current VmAllocateResponse type
                                // println!("  Pool ID: {}", response.pool_id.bright_yellow());
                            }
                            Err(e) => {
                                println!("❌ Error allocating VM: {}", e);
                            }
                        }
                    } else {
                        println!("❌ API client not available");
                    }
                }
                super::commands::VmSubcommand::Release { vm_id } => {
                    if let Some(api_client) = &self.api_client {
                        println!("➖ Release VM: {}", vm_id.green());
                        match api_client.release_vm_ip(&vm_id).await {
                            Ok(_) => {
                                println!("✅ VM released successfully!");
                                println!("  VM {} resources have been freed", vm_id.bright_green());
                            }
                            Err(e) => {
                                println!("❌ Error releasing VM: {}", e);
                            }
                        }
                    } else {
                        println!("❌ API client not available");
                    }
                }
                super::commands::VmSubcommand::Monitor { vm_id, refresh } => {
                    if let Some(api_client) = &self.api_client {
                        let interval = refresh.unwrap_or(5); // Default 5 seconds
                        println!("👁️  Monitor VM: {} (refresh: {}s)", vm_id.green(), interval);
                        println!("Starting VM monitoring... Press Ctrl+C to stop");

                        // Simple monitoring loop
                        let mut count = 0;
                        loop {
                            count += 1;
                            println!("\n--- Check #{} ---", count);

                            // Get VM status
                            match api_client.get_vm_status(&vm_id).await {
                                Ok(status) => {
                                    println!(
                                        "Status: {} | IP: {} | Created: {}",
                                        status.status.bright_green(),
                                        status.ip_address.bright_blue(),
                                        status.created_at
                                    );
                                }
                                Err(e) => {
                                    println!("❌ Error getting status: {}", e);
                                }
                            }

                            // Get VM metrics
                            match api_client.get_vm_metrics(&vm_id).await {
                                Ok(metrics) => {
                                    println!(
                                        "CPU: {:.1}% | Memory: {}MB | Disk: {:.1}% | Uptime: {}s",
                                        metrics.cpu_usage_percent,
                                        metrics.memory_usage_mb,
                                        metrics.disk_usage_percent,
                                        metrics.uptime_seconds
                                    );
                                }
                                Err(e) => {
                                    println!("❌ Error getting metrics: {}", e);
                                }
                            }

                            // Wait before next check
                            tokio::time::sleep(tokio::time::Duration::from_secs(interval as u64))
                                .await;
                        }
                    } else {
                        println!("❌ API client not available");
                    }
                }
            }
        }

        #[cfg(not(feature = "repl"))]
        {
            println!("VM operations require repl feature");
        }

        Ok(())
    }

    async fn handle_robot(&self, subcommand: RobotSubcommand) -> Result<()> {
        #[cfg(feature = "repl")]
        {
            use colored::Colorize;

            let docs = SelfDocumentation::new();

            match subcommand {
                RobotSubcommand::Capabilities => {
                    println!("{} Robot Mode - Capabilities\n", "🤖".bold());
                    let capabilities = docs.capabilities_data();
                    let json = serde_json::to_string_pretty(&capabilities)?;
                    println!("{}", json);
                }
                RobotSubcommand::Schemas { command } => {
                    println!("{} Robot Mode - Schemas\n", "📋".bold());
                    if let Some(cmd) = command {
                        if let Some(schema) = docs.schema(&cmd) {
                            let json = serde_json::to_string_pretty(schema)?;
                            println!("{}", json);
                        } else {
                            println!(
                                "{} No schema found for command: {}",
                                "ℹ".blue().bold(),
                                cmd.yellow()
                            );
                        }
                    } else {
                        // Show all schemas
                        let schemas = docs.all_schemas();
                        let json = serde_json::to_string_pretty(schemas)?;
                        println!("{}", json);
                    }
                }
                RobotSubcommand::Examples { command } => {
                    println!("{} Robot Mode - Examples\n", "📝".bold());
                    if let Some(cmd) = command {
                        if let Some(examples) = docs.examples(&cmd) {
                            let json = serde_json::to_string_pretty(examples)?;
                            println!("{}", json);
                        } else {
                            println!(
                                "{} No examples found for command: {}",
                                "ℹ".blue().bold(),
                                cmd.yellow()
                            );
                        }
                    } else {
                        // Show examples for all commands
                        let all_examples: Vec<_> = docs
                            .all_schemas()
                            .iter()
                            .flat_map(|s| {
                                s.examples.iter().map(move |e| {
                                    serde_json::json!({
                                        "command": s.name,
                                        "example": e
                                    })
                                })
                            })
                            .collect();
                        let json = serde_json::to_string_pretty(&all_examples)?;
                        println!("{}", json);
                    }
                }
                RobotSubcommand::ExitCodes => {
                    println!("{} Robot Mode - Exit Codes\n", "🚪".bold());
                    let exit_codes = vec![
                        serde_json::json!({
                            "code": ExitCode::Success.code(),
                            "name": ExitCode::Success.name(),
                            "description": ExitCode::Success.description()
                        }),
                        serde_json::json!({
                            "code": ExitCode::ErrorGeneral.code(),
                            "name": ExitCode::ErrorGeneral.name(),
                            "description": ExitCode::ErrorGeneral.description()
                        }),
                        serde_json::json!({
                            "code": ExitCode::ErrorUsage.code(),
                            "name": ExitCode::ErrorUsage.name(),
                            "description": ExitCode::ErrorUsage.description()
                        }),
                        serde_json::json!({
                            "code": ExitCode::ErrorIndexMissing.code(),
                            "name": ExitCode::ErrorIndexMissing.name(),
                            "description": ExitCode::ErrorIndexMissing.description()
                        }),
                        serde_json::json!({
                            "code": ExitCode::ErrorNotFound.code(),
                            "name": ExitCode::ErrorNotFound.name(),
                            "description": ExitCode::ErrorNotFound.description()
                        }),
                        serde_json::json!({
                            "code": ExitCode::ErrorAuth.code(),
                            "name": ExitCode::ErrorAuth.name(),
                            "description": ExitCode::ErrorAuth.description()
                        }),
                        serde_json::json!({
                            "code": ExitCode::ErrorNetwork.code(),
                            "name": ExitCode::ErrorNetwork.name(),
                            "description": ExitCode::ErrorNetwork.description()
                        }),
                        serde_json::json!({
                            "code": ExitCode::ErrorTimeout.code(),
                            "name": ExitCode::ErrorTimeout.name(),
                            "description": ExitCode::ErrorTimeout.description()
                        }),
                    ];
                    let json = serde_json::to_string_pretty(&exit_codes)?;
                    println!("{}", json);
                }
            }
        }

        #[cfg(not(feature = "repl"))]
        {
            println!("Robot mode requires repl feature");
        }

        Ok(())
    }

    /// Handle update management commands
    async fn handle_update(&mut self, subcommand: UpdateSubcommand) -> Result<()> {
        use terraphim_update::{
            check_for_updates_auto, rollback::BackupManager, update_binary, UpdateStatus,
        };

        // Get the current binary name and version
        let bin_name = env!("CARGO_PKG_NAME");
        let current_version = env!("CARGO_PKG_VERSION");

        match subcommand {
            UpdateSubcommand::Check => {
                println!("Checking for updates...");
                match check_for_updates_auto(bin_name, current_version).await {
                    Ok(status) => {
                        println!("{}", status);
                    }
                    Err(e) => {
                        println!("Failed to check for updates: {}", e);
                    }
                }
            }
            UpdateSubcommand::Install => {
                println!("Checking for updates and installing if available...");
                match update_binary(bin_name).await {
                    Ok(status) => match status {
                        UpdateStatus::Updated {
                            from_version,
                            to_version,
                        } => {
                            println!(
                                "Successfully updated from {} to {}",
                                from_version, to_version
                            );
                            println!("Please restart the application to use the new version.");
                        }
                        UpdateStatus::UpToDate(version) => {
                            println!("Already running the latest version: {}", version);
                        }
                        UpdateStatus::Available {
                            current_version,
                            latest_version,
                        } => {
                            println!(
                                "Update available: {} -> {} (installation pending)",
                                current_version, latest_version
                            );
                        }
                        UpdateStatus::Failed(error) => {
                            println!("Update failed: {}", error);
                        }
                    },
                    Err(e) => {
                        println!("Failed to update: {}", e);
                    }
                }
            }
            UpdateSubcommand::Rollback { version } => {
                println!("Rolling back to version {}...", version);

                // Get backup directory (use standard location)
                let backup_dir = dirs::data_local_dir()
                    .unwrap_or_else(|| std::path::PathBuf::from("."))
                    .join("terraphim")
                    .join("backups");

                match BackupManager::new(backup_dir, 3) {
                    Ok(manager) => {
                        // Get current executable path as target
                        let target_path = std::env::current_exe()
                            .unwrap_or_else(|_| std::path::PathBuf::from(bin_name));

                        match manager.rollback_to_version(&version, &target_path) {
                            Ok(()) => {
                                println!("Successfully rolled back to version {}", version);
                                println!("Please restart the application.");
                            }
                            Err(e) => {
                                println!("Rollback failed: {}", e);
                            }
                        }
                    }
                    Err(e) => {
                        println!("Failed to initialize backup manager: {}", e);
                    }
                }
            }
            UpdateSubcommand::List => {
                println!("Available backup versions:");

                // Get backup directory
                let backup_dir = dirs::data_local_dir()
                    .unwrap_or_else(|| std::path::PathBuf::from("."))
                    .join("terraphim")
                    .join("backups");

                match BackupManager::new(backup_dir, 3) {
                    Ok(manager) => {
                        let versions = manager.list_backups();
                        if versions.is_empty() {
                            println!("  No backups available.");
                        } else {
                            for (i, version) in versions.iter().enumerate() {
                                println!("  {}. {}", i + 1, version);
                            }
                        }
                    }
                    Err(e) => {
                        println!("Failed to list backups: {}", e);
                    }
                }
            }
        }

        Ok(())
    }

    #[cfg(feature = "repl-sessions")]
    async fn handle_sessions(&mut self, subcommand: SessionsSubcommand) -> Result<()> {
        use colored::Colorize;
        use comfy_table::modifiers::UTF8_ROUND_CORNERS;
        use comfy_table::presets::UTF8_FULL;
        use comfy_table::{Cell, Table};
        use terraphim_sessions::{
            ConnectorStatus, ImportOptions, MessageRole, Session, SessionService,
        };

        // Get or create session service
        static SESSION_SERVICE: std::sync::OnceLock<
            std::sync::Arc<tokio::sync::Mutex<SessionService>>,
        > = std::sync::OnceLock::new();
        let service = SESSION_SERVICE
            .get_or_init(|| std::sync::Arc::new(tokio::sync::Mutex::new(SessionService::new())));
        let svc = service.lock().await;

        match subcommand {
            SessionsSubcommand::Sources => {
                println!("\n{}", "Available Session Sources:".bold().cyan());

                let sources = svc.detect_sources();
                let mut table = Table::new();
                table
                    .load_preset(UTF8_FULL)
                    .apply_modifier(UTF8_ROUND_CORNERS)
                    .set_header(vec![
                        Cell::new("Source").add_attribute(comfy_table::Attribute::Bold),
                        Cell::new("Status").add_attribute(comfy_table::Attribute::Bold),
                        Cell::new("Sessions").add_attribute(comfy_table::Attribute::Bold),
                    ]);

                for source in sources {
                    let (status, count) = match &source.status {
                        ConnectorStatus::Available {
                            sessions_estimate, ..
                        } => (
                            "Available".green().to_string(),
                            sessions_estimate
                                .map(|c| c.to_string())
                                .unwrap_or("-".to_string()),
                        ),
                        ConnectorStatus::NotFound => {
                            ("Not Found".yellow().to_string(), "-".to_string())
                        }
                        ConnectorStatus::Error(e) => {
                            (format!("Error: {}", e).red().to_string(), "-".to_string())
                        }
                    };

                    table.add_row(vec![
                        Cell::new(&source.id),
                        Cell::new(status),
                        Cell::new(count),
                    ]);
                }

                println!("{}", table);
            }

            SessionsSubcommand::Import { source, limit } => {
                let options = ImportOptions::new().with_limit(limit.unwrap_or(100));

                println!("\n{} Importing sessions...", "⏳".bold());

                let sessions = if let Some(source_id) = source {
                    svc.import_from(&source_id, &options).await?
                } else {
                    svc.import_all(&options).await?
                };

                println!(
                    "{} Imported {} session(s)",
                    "✅".bold(),
                    sessions.len().to_string().green()
                );
            }

            SessionsSubcommand::List { source, limit } => {
                let sessions = if let Some(source_id) = source {
                    svc.sessions_by_source(&source_id).await
                } else {
                    svc.list_sessions().await
                };

                let sessions: Vec<_> = if let Some(lim) = limit {
                    sessions.into_iter().take(lim).collect()
                } else {
                    sessions.into_iter().take(20).collect()
                };

                if sessions.is_empty() {
                    println!(
                        "{} No sessions found. Run '/sessions import' first.",
                        "ℹ".blue().bold()
                    );
                    return Ok(());
                }

                println!("\n{}", "Sessions:".bold().cyan());
                let mut table = Table::new();
                table
                    .load_preset(UTF8_FULL)
                    .apply_modifier(UTF8_ROUND_CORNERS)
                    .set_header(vec![
                        Cell::new("ID").add_attribute(comfy_table::Attribute::Bold),
                        Cell::new("Source").add_attribute(comfy_table::Attribute::Bold),
                        Cell::new("Title").add_attribute(comfy_table::Attribute::Bold),
                        Cell::new("Messages").add_attribute(comfy_table::Attribute::Bold),
                    ]);

                for session in &sessions {
                    let title = session
                        .title
                        .as_ref()
                        .map(|t| {
                            if t.len() > 40 {
                                format!("{}...", &t[..40])
                            } else {
                                t.clone()
                            }
                        })
                        .unwrap_or_else(|| "-".to_string());

                    table.add_row(vec![
                        Cell::new(&session.external_id[..8.min(session.external_id.len())]),
                        Cell::new(&session.source),
                        Cell::new(title),
                        Cell::new(session.message_count().to_string()),
                    ]);
                }

                println!("{}", table);
                println!("Showing {} session(s)", sessions.len().to_string().green());
            }

            SessionsSubcommand::Search { query } => {
                let sessions = svc.search(&query).await;

                if sessions.is_empty() {
                    println!("{} No sessions match '{}'", "ℹ".blue().bold(), query.cyan());
                    return Ok(());
                }

                println!(
                    "\n{} sessions match '{}':",
                    sessions.len().to_string().green(),
                    query.cyan()
                );
                let mut table = Table::new();
                table
                    .load_preset(UTF8_FULL)
                    .apply_modifier(UTF8_ROUND_CORNERS)
                    .set_header(vec![
                        Cell::new("ID").add_attribute(comfy_table::Attribute::Bold),
                        Cell::new("Source").add_attribute(comfy_table::Attribute::Bold),
                        Cell::new("Title").add_attribute(comfy_table::Attribute::Bold),
                    ]);

                for session in sessions.iter().take(10) {
                    let title = session
                        .title
                        .as_ref()
                        .map(|t| {
                            if t.len() > 50 {
                                format!("{}...", &t[..50])
                            } else {
                                t.clone()
                            }
                        })
                        .unwrap_or_else(|| "-".to_string());

                    table.add_row(vec![
                        Cell::new(&session.external_id[..8.min(session.external_id.len())]),
                        Cell::new(&session.source),
                        Cell::new(title),
                    ]);
                }

                println!("{}", table);
            }

            SessionsSubcommand::Stats => {
                let stats = svc.statistics().await;

                println!("\n{}", "Session Statistics:".bold().cyan());
                println!(
                    "  Total Sessions:          {}",
                    stats.total_sessions.to_string().green()
                );
                println!(
                    "  Total Messages:          {}",
                    stats.total_messages.to_string().green()
                );
                println!(
                    "  User Messages:           {}",
                    stats.total_user_messages.to_string().yellow()
                );
                println!(
                    "  Assistant Messages:      {}",
                    stats.total_assistant_messages.to_string().blue()
                );

                if !stats.sessions_by_source.is_empty() {
                    println!("\n  Sessions by Source:");
                    for (source, count) in &stats.sessions_by_source {
                        println!("    {}: {}", source.yellow(), count);
                    }
                }
            }

            SessionsSubcommand::Show { session_id } => {
                let session = svc.get_session(&session_id).await;

                if let Some(session) = session {
                    println!("\n{} Session: {}", "📋".bold(), session.id.cyan());
                    println!("  Source:       {}", session.source.yellow());
                    println!(
                        "  Title:        {}",
                        session.title.as_ref().unwrap_or(&"-".to_string())
                    );
                    println!(
                        "  Messages:     {}",
                        session.message_count().to_string().green()
                    );
                    if let Some(duration) = session.duration_ms() {
                        let minutes = duration / 60000;
                        println!("  Duration:     {} min", minutes);
                    }

                    println!("\n  {} Messages:", "💬".bold());
                    for (i, msg) in session.messages.iter().take(5).enumerate() {
                        let role_color = match msg.role.to_string().as_str() {
                            "user" => msg.role.to_string().blue(),
                            "assistant" => msg.role.to_string().green(),
                            _ => msg.role.to_string().yellow(),
                        };
                        let content_preview = if msg.content.len() > 80 {
                            format!("{}...", &msg.content[..80])
                        } else {
                            msg.content.clone()
                        };
                        println!("    [{}] {}: {}", i + 1, role_color, content_preview);
                    }
                    if session.messages.len() > 5 {
                        println!("    ... and {} more messages", session.messages.len() - 5);
                    }
                } else {
                    println!("{} Session '{}' not found", "⚠".yellow().bold(), session_id);
                }
            }

            SessionsSubcommand::Concepts { concept } => {
                println!(
                    "\n{} Searching sessions by concept: '{}'",
                    "🔍".bold(),
                    concept.cyan()
                );
                println!(
                    "{} This feature requires enrichment. Searching by text match...",
                    "ℹ".blue()
                );

                // Fall back to text search for now (enrichment requires thesaurus)
                let sessions = svc.search(&concept).await;

                if sessions.is_empty() {
                    println!(
                        "{} No sessions contain concept '{}'",
                        "ℹ".blue().bold(),
                        concept
                    );
                    return Ok(());
                }

                let mut table = Table::new();
                table
                    .load_preset(UTF8_FULL)
                    .apply_modifier(UTF8_ROUND_CORNERS)
                    .set_header(vec![
                        Cell::new("ID").fg(comfy_table::Color::Cyan),
                        Cell::new("Source").fg(comfy_table::Color::Yellow),
                        Cell::new("Matches").fg(comfy_table::Color::Green),
                        Cell::new("Title").fg(comfy_table::Color::White),
                    ]);

                for session in sessions.iter().take(10) {
                    let title = session
                        .title
                        .as_ref()
                        .map(|t| {
                            if t.len() > 40 {
                                format!("{}...", &t[..40])
                            } else {
                                t.clone()
                            }
                        })
                        .unwrap_or_else(|| "-".to_string());

                    // Count occurrences of concept
                    let count: usize = session
                        .messages
                        .iter()
                        .filter(|m| m.content.to_lowercase().contains(&concept.to_lowercase()))
                        .count();

                    table.add_row(vec![
                        Cell::new(&session.id[..8]),
                        Cell::new(&session.source),
                        Cell::new(count.to_string()),
                        Cell::new(title),
                    ]);
                }

                println!("{}", table);
            }

            SessionsSubcommand::Related {
                session_id,
                min_shared,
            } => {
                println!(
                    "\n{} Finding sessions related to: {}",
                    "🔗".bold(),
                    session_id.cyan()
                );
                println!(
                    "{} This feature requires enrichment. Showing based on search similarity...",
                    "ℹ".blue()
                );

                let _min = min_shared.unwrap_or(1); // Will be used with enrichment

                // Get the source session
                let source = svc.get_session(&session_id).await;
                if source.is_none() {
                    println!("{} Session '{}' not found", "⚠".yellow().bold(), session_id);
                    return Ok(());
                }
                let source = source.unwrap();

                // Get keywords from first user message
                let keywords = source
                    .messages
                    .iter()
                    .find(|m| m.role == MessageRole::User)
                    .map(|m| {
                        m.content
                            .split_whitespace()
                            .take(3)
                            .collect::<Vec<_>>()
                            .join(" ")
                    })
                    .unwrap_or_default();

                if keywords.is_empty() {
                    println!("{} No keywords found in session", "ℹ".blue().bold());
                    return Ok(());
                }

                let related = svc.search(&keywords).await;
                let related: Vec<_> = related
                    .into_iter()
                    .filter(|s| s.id != session_id)
                    .take(5)
                    .collect();

                if related.is_empty() {
                    println!("{} No related sessions found", "ℹ".blue().bold());
                    return Ok(());
                }

                let mut table = Table::new();
                table
                    .load_preset(UTF8_FULL)
                    .apply_modifier(UTF8_ROUND_CORNERS)
                    .set_header(vec![
                        Cell::new("Session ID").fg(comfy_table::Color::Cyan),
                        Cell::new("Source").fg(comfy_table::Color::Yellow),
                        Cell::new("Messages").fg(comfy_table::Color::Green),
                        Cell::new("Title").fg(comfy_table::Color::White),
                    ]);

                for session in related {
                    let title = session
                        .title
                        .as_ref()
                        .map(|t| {
                            if t.len() > 40 {
                                format!("{}...", &t[..40])
                            } else {
                                t.clone()
                            }
                        })
                        .unwrap_or_else(|| "-".to_string());

                    table.add_row(vec![
                        Cell::new(&session.id[..8]),
                        Cell::new(&session.source),
                        Cell::new(session.message_count().to_string()),
                        Cell::new(title),
                    ]);
                }

                println!("{}", table);
            }

            SessionsSubcommand::Timeline { group_by, limit } => {
                use std::collections::HashMap;

                let group = group_by.as_deref().unwrap_or("day");
                let max_entries = limit.unwrap_or(30);

                println!(
                    "\n{} Session Timeline (grouped by {}):",
                    "📅".bold(),
                    group.cyan()
                );

                let sessions = svc.list_sessions().await;
                if sessions.is_empty() {
                    println!(
                        "{} No sessions found. Import sessions first.",
                        "ℹ".blue().bold()
                    );
                    return Ok(());
                }

                // Group sessions by date
                let mut grouped: HashMap<String, Vec<&Session>> = HashMap::new();

                for session in &sessions {
                    let date_key = if let Some(started) = session.started_at {
                        let date = started.strftime("%Y-%m-%d").to_string();
                        match group {
                            "week" => {
                                // Get week start (Monday)
                                format!("Week of {}", &date[..10])
                            }
                            "month" => {
                                format!("{}-{}", &date[..4], &date[5..7])
                            }
                            _ => date[..10].to_string(), // day
                        }
                    } else {
                        "Unknown".to_string()
                    };

                    grouped.entry(date_key).or_default().push(session);
                }

                // Sort by date key
                let mut sorted: Vec<_> = grouped.into_iter().collect();
                sorted.sort_by(|a, b| b.0.cmp(&a.0)); // Newest first

                let mut table = Table::new();
                table
                    .load_preset(UTF8_FULL)
                    .apply_modifier(UTF8_ROUND_CORNERS)
                    .set_header(vec![
                        Cell::new("Date").fg(comfy_table::Color::Cyan),
                        Cell::new("Sessions").fg(comfy_table::Color::Green),
                        Cell::new("Messages").fg(comfy_table::Color::Yellow),
                        Cell::new("Sources").fg(comfy_table::Color::White),
                    ]);

                for (date, day_sessions) in sorted.into_iter().take(max_entries) {
                    let session_count = day_sessions.len();
                    let message_count: usize = day_sessions.iter().map(|s| s.message_count()).sum();
                    let sources: std::collections::HashSet<_> =
                        day_sessions.iter().map(|s| s.source.as_str()).collect();

                    table.add_row(vec![
                        Cell::new(&date),
                        Cell::new(session_count.to_string()),
                        Cell::new(message_count.to_string()),
                        Cell::new(sources.into_iter().collect::<Vec<_>>().join(", ")),
                    ]);
                }

                println!("{}", table);
            }

            SessionsSubcommand::Export {
                format,
                output,
                session_id,
            } => {
                let fmt = format.as_deref().unwrap_or("json");

                println!(
                    "\n{} Exporting sessions (format: {})...",
                    "📤".bold(),
                    fmt.cyan()
                );

                let sessions: Vec<Session> = if let Some(id) = session_id {
                    if let Some(session) = svc.get_session(&id).await {
                        vec![session]
                    } else {
                        println!("{} Session '{}' not found", "⚠".yellow().bold(), id);
                        return Ok(());
                    }
                } else {
                    svc.list_sessions().await
                };

                if sessions.is_empty() {
                    println!("{} No sessions to export", "ℹ".blue().bold());
                    return Ok(());
                }

                let content = match fmt {
                    "json" => serde_json::to_string_pretty(&sessions)?,
                    "markdown" | "md" => {
                        let mut md = String::new();
                        md.push_str("# AI Coding Sessions Export\n\n");
                        for session in &sessions {
                            md.push_str(&format!("## {}\n\n", session.id));
                            md.push_str(&format!("- **Source**: {}\n", session.source));
                            if let Some(title) = &session.title {
                                md.push_str(&format!("- **Title**: {}\n", title));
                            }
                            md.push_str(&format!(
                                "- **Messages**: {}\n\n",
                                session.message_count()
                            ));
                            md.push_str("### Conversation\n\n");
                            for msg in &session.messages {
                                md.push_str(&format!("**{}**: {}\n\n", msg.role, msg.content));
                            }
                            md.push_str("---\n\n");
                        }
                        md
                    }
                    _ => {
                        println!(
                            "{} Unknown format '{}'. Use: json, markdown",
                            "⚠".yellow().bold(),
                            fmt
                        );
                        return Ok(());
                    }
                };

                if let Some(path) = output {
                    std::fs::write(&path, &content)?;
                    println!(
                        "{} Exported {} sessions to '{}'",
                        "✅".green().bold(),
                        sessions.len(),
                        path.green()
                    );
                } else {
                    println!("{}", content);
                }
            }

            SessionsSubcommand::Enrich { session_id } => {
                println!("\n{} Enriching sessions with concepts...", "🧠".bold());
                println!(
                    "{} This feature requires the 'enrichment' feature flag.",
                    "ℹ".blue()
                );
                println!(
                    "{} Rebuild with: cargo build --features repl-sessions,enrichment",
                    "💡".yellow()
                );

                // For now, show what would be enriched
                if let Some(id) = session_id {
                    if let Some(session) = svc.get_session(&id).await {
                        println!("\n  Would enrich session: {}", session.id.cyan());
                        println!("  Messages to process: {}", session.message_count());

                        // Show sample text
                        if let Some(first_msg) = session.messages.first() {
                            let preview = if first_msg.content.len() > 100 {
                                format!("{}...", &first_msg.content[..100])
                            } else {
                                first_msg.content.clone()
                            };
                            println!("  Sample: {}", preview.italic());
                        }
                    } else {
                        println!("{} Session '{}' not found", "⚠".yellow().bold(), id);
                    }
                } else {
                    let sessions = svc.list_sessions().await;
                    println!(
                        "\n  Would enrich {} sessions",
                        sessions.len().to_string().green()
                    );

                    let total_messages: usize = sessions.iter().map(|s| s.message_count()).sum();
                    println!(
                        "  Total messages to process: {}",
                        total_messages.to_string().green()
                    );
                }
            }
        }

        Ok(())
    }
}

/// Run REPL in offline mode
pub async fn run_repl_offline_mode() -> Result<()> {
    let service = TuiService::new().await?;
    let mut handler = ReplHandler::new_offline(service);
    handler.run().await
}

/// Run REPL in server mode
pub async fn run_repl_server_mode(server_url: &str) -> Result<()> {
    let api_client = ApiClient::new(server_url.to_string());
    let mut handler = ReplHandler::new_server(api_client);
    handler.run().await
}
