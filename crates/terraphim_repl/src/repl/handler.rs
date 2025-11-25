//! REPL handler implementation (minimal release)

use super::commands::{ConfigSubcommand, ReplCommand, RoleSubcommand};
use anyhow::Result;
use colored::Colorize;
use comfy_table::modifiers::UTF8_ROUND_CORNERS;
use comfy_table::presets::UTF8_FULL;
use comfy_table::{Cell, Table};
use rustyline::completion::{Completer, Pair};
use rustyline::highlight::Highlighter;
use rustyline::hint::Hinter;
use rustyline::validate::Validator;
use rustyline::{Context, Editor, Helper};
use std::io::{self, Write};
use std::str::FromStr;
use crate::service::TuiService;

pub struct ReplHandler {
    service: TuiService,
    current_role: String,
}

impl ReplHandler {
    pub fn new_offline(service: TuiService) -> Self {
        Self {
            service,
            current_role: "Default".to_string(),
        }
    }

    pub async fn run(&mut self) -> Result<()> {
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
                    let prefix = line.strip_prefix('/').unwrap_or(line);
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
            .map(|h| h.join(".terraphim_repl_history"))
            .unwrap_or_else(|| std::path::PathBuf::from(".terraphim_repl_history"));

        let _ = rl.load_history(&history_file);

        println!("{}", "=".repeat(60).cyan());
        println!("{}", "🌍 Terraphim REPL v1.0.0".bold().cyan());
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

    async fn show_welcome(&self) {
        println!(
            "Type {} for help, {} to exit",
            "/help".yellow(),
            "/quit".yellow()
        );

        println!(
            "Mode: {} | Current Role: {}",
            "Offline Mode".bold(),
            self.current_role.green().bold()
        );

        self.show_available_commands();
    }

    fn show_available_commands(&self) {
        println!("\n{}", "Available commands:".bold());
        println!("  {} - Search documents", "/search <query>".yellow());
        println!("  {} - Display configuration", "/config show".yellow());
        println!("  {} - Manage roles", "/role [list|select]".yellow());
        println!("  {} - Show knowledge graph", "/graph".yellow());
        println!("  {} - Show help", "/help [command]".yellow());
        println!("  {} - Exit REPL", "/quit".yellow());
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
        }

        Ok(false)
    }

    async fn handle_search(
        &self,
        query: String,
        role: Option<String>,
        limit: Option<usize>,
    ) -> Result<()> {
        println!("{} Searching for: '{}'", "🔍".bold(), query.cyan());

        let role_name = if let Some(role) = role {
            terraphim_types::RoleName::new(&role)
        } else {
            self.service.get_selected_role().await
        };

        let results = self
            .service
            .search_with_role(&query, &role_name, limit)
            .await?;

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
                    Cell::new(if doc.url.is_empty() {
                        "N/A"
                    } else {
                        &doc.url
                    }),
                ]);
            }

            println!("{}", table);
            println!(
                "{} Found {} result(s)",
                "✅".bold(),
                results.len().to_string().green()
            );
        }

        Ok(())
    }

    async fn handle_config(&self, subcommand: ConfigSubcommand) -> Result<()> {
        match subcommand {
            ConfigSubcommand::Show => {
                let config = self.service.get_config().await;
                let config_json = serde_json::to_string_pretty(&config)?;
                println!("{}", config_json);
            }
        }
        Ok(())
    }

    async fn handle_role(&mut self, subcommand: RoleSubcommand) -> Result<()> {
        match subcommand {
            RoleSubcommand::List => {
                let roles = self.service.list_roles().await;
                println!("{}", "Available roles:".bold());
                for role in roles {
                    let marker = if role == self.current_role { "▶" } else { " " };
                    println!("  {} {}", marker.green(), role);
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

        let role_name = self.service.get_selected_role().await;
        let concepts = self.service.get_role_graph_top_k(&role_name, k).await?;

        println!("{} Top {} concepts:", "📊".bold(), k.to_string().cyan());
        for (i, concept) in concepts.iter().enumerate() {
            println!("  {}. {}", (i + 1).to_string().yellow(), concept);
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
}

/// Run REPL in offline mode
pub async fn run_repl_offline_mode() -> Result<()> {
    let service = TuiService::new().await?;
    let mut handler = ReplHandler::new_offline(service);
    handler.run().await
}
