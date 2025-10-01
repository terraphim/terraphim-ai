//! REPL handler implementation

use super::commands::{ConfigSubcommand, ReplCommand, RoleSubcommand};
use crate::{client::ApiClient, service::TuiService};
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
            ReplCommand::Search { query, role, limit } => {
                self.handle_search(query, role, limit).await?;
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
        }

        Ok(false)
    }

    async fn handle_search(
        &self,
        query: String,
        role: Option<String>,
        limit: Option<usize>,
    ) -> Result<()> {
        #[cfg(feature = "repl")]
        {
            use colored::Colorize;
            use comfy_table::modifiers::UTF8_ROUND_CORNERS;
            use comfy_table::presets::UTF8_FULL;
            use comfy_table::{Cell, Table};

            println!("{} Searching for: '{}'", "🔍".bold(), query.cyan());

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
                _ => terraphim_automata::LinkType::MarkdownLinks, // Default
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
