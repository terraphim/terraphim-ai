use std::io;

use anyhow::Result;
use clap::{Parser, Subcommand};
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::Line,
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Terminal,
};
use tokio::runtime::Runtime;

mod client;
mod service;

#[cfg(feature = "repl")]
mod repl;

use client::{ApiClient, SearchResponse};
use service::TuiService;
use terraphim_types::{Document, LogicalOperator, NormalizedTermValue, RoleName, SearchQuery};
use terraphim_update::{check_for_updates, update_binary};

#[derive(clap::ValueEnum, Debug, Clone)]
enum LogicalOperatorCli {
    And,
    Or,
}

impl From<LogicalOperatorCli> for LogicalOperator {
    fn from(op: LogicalOperatorCli) -> Self {
        match op {
            LogicalOperatorCli::And => LogicalOperator::And,
            LogicalOperatorCli::Or => LogicalOperator::Or,
        }
    }
}

/// Create a transparent style for UI elements
fn transparent_style() -> Style {
    Style::default().bg(Color::Reset)
}

/// Create a block with optional transparent background
fn create_block(title: &str, transparent: bool) -> Block<'_> {
    let block = Block::default().title(title).borders(Borders::ALL);

    if transparent {
        block.style(transparent_style())
    } else {
        block
    }
}

#[derive(Debug, Clone, PartialEq)]
enum ViewMode {
    Search,
    ResultDetail,
}

#[derive(Parser, Debug)]
#[command(name = "terraphim-tui", version, about = "Terraphim TUI interface")]
struct Cli {
    /// Use server API mode instead of self-contained offline mode
    #[arg(long, default_value_t = false)]
    server: bool,
    /// Server URL for API mode
    #[arg(long, default_value = "http://localhost:8000")]
    server_url: String,
    /// Enable transparent background mode
    #[arg(long, default_value_t = false)]
    transparent: bool,
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand, Debug)]
enum Command {
    Search {
        /// Primary search query
        query: String,
        /// Additional search terms for multi-term queries
        #[arg(long, num_args = 1.., value_delimiter = ',')]
        terms: Option<Vec<String>>,
        /// Logical operator for combining multiple search terms (and/or)
        #[arg(long, value_enum)]
        operator: Option<LogicalOperatorCli>,
        #[arg(long)]
        role: Option<String>,
        #[arg(long, default_value_t = 10)]
        limit: usize,
    },
    Roles {
        #[command(subcommand)]
        sub: RolesSub,
    },
    Config {
        #[command(subcommand)]
        sub: ConfigSub,
    },
    Graph {
        #[arg(long)]
        role: Option<String>,
        #[arg(long, default_value_t = 50)]
        top_k: usize,
    },
    Chat {
        #[arg(long)]
        role: Option<String>,
        prompt: String,
        #[arg(long)]
        model: Option<String>,
    },
    Extract {
        text: String,
        #[arg(long)]
        role: Option<String>,
        #[arg(long, default_value_t = false)]
        exclude_term: bool,
    },
    Replace {
        text: String,
        #[arg(long)]
        role: Option<String>,
        #[arg(long)]
        format: Option<String>,
    },
    Interactive,

    /// Start REPL (Read-Eval-Print-Loop) interface
    #[cfg(feature = "repl")]
    Repl {
        /// Start in server mode
        #[arg(long)]
        server: bool,
        /// Server URL for API mode
        #[arg(long, default_value = "http://localhost:8000")]
        server_url: String,
    },

    /// Check for updates without installing
    CheckUpdate,

    /// Update to latest version if available
    Update,
}

#[derive(Subcommand, Debug)]
enum RolesSub {
    List,
    Select { name: String },
}

#[derive(Subcommand, Debug)]
enum ConfigSub {
    Show,
    Set { key: String, value: String },
}

fn main() -> Result<()> {
    // tokio runtime for subcommands; interactive mode runs sync loop and spawns async tasks if needed
    let rt = Runtime::new()?;
    let cli = Cli::parse();

    match cli.command {
        Some(Command::Interactive) | None => {
            if cli.server {
                run_tui_server_mode(&cli.server_url, cli.transparent)
            } else {
                rt.block_on(run_tui_offline_mode(cli.transparent))
            }
        }

        #[cfg(feature = "repl")]
        Some(Command::Repl { server, server_url }) => {
            if server {
                rt.block_on(repl::run_repl_server_mode(&server_url))
            } else {
                rt.block_on(repl::run_repl_offline_mode())
            }
        }

        Some(command) => {
            if cli.server {
                rt.block_on(run_server_command(command, &cli.server_url))
            } else {
                rt.block_on(run_offline_command(command))
            }
        }
    }
}
async fn run_tui_offline_mode(transparent: bool) -> Result<()> {
    let service = TuiService::new().await?;
    run_tui_with_service(service, transparent).await
}

fn run_tui_server_mode(_server_url: &str, transparent: bool) -> Result<()> {
    // TODO: Pass server_url to TUI for API client initialization
    run_tui(transparent)
}

async fn run_tui_with_service(_service: TuiService, transparent: bool) -> Result<()> {
    // TODO: Update interactive TUI to use local service instead of API client
    // For now, fall back to the existing TUI implementation
    run_tui(transparent)
}

async fn run_offline_command(command: Command) -> Result<()> {
    let service = TuiService::new().await?;

    match command {
        Command::Search {
            query,
            terms,
            operator,
            role,
            limit,
        } => {
            let role_name = if let Some(role) = role {
                RoleName::new(&role)
            } else {
                service.get_selected_role().await
            };

            let results = if let Some(additional_terms) = terms {
                // Multi-term query with logical operators
                let mut all_terms = vec![query];
                all_terms.extend(additional_terms);

                let op_str = match operator {
                    Some(LogicalOperatorCli::And) => "AND",
                    Some(LogicalOperatorCli::Or) | None => "OR", // Default to OR
                };
                println!(
                    "Multi-term search: {} terms using {} operator",
                    all_terms.len(),
                    op_str
                );

                let search_query = SearchQuery {
                    search_term: NormalizedTermValue::from(all_terms[0].as_str()),
                    search_terms: if all_terms.len() > 1 {
                        Some(
                            all_terms[1..]
                                .iter()
                                .map(|t| NormalizedTermValue::from(t.as_str()))
                                .collect(),
                        )
                    } else {
                        None
                    },
                    operator: operator.map(|op| op.into()),
                    skip: Some(0),
                    limit: Some(limit),
                    role: Some(role_name.clone()),
                };

                service.search_with_query(&search_query).await?
            } else {
                // Single term query (backward compatibility)
                service
                    .search_with_role(&query, &role_name, Some(limit))
                    .await?
            };

            for doc in results.iter() {
                println!("- {}\t{}", doc.rank.unwrap_or_default(), doc.title);
            }
            Ok(())
        }
        Command::Roles { sub } => {
            match sub {
                RolesSub::List => {
                    let roles = service.list_roles().await;
                    println!("{}", roles.join("\n"));
                }
                RolesSub::Select { name } => {
                    let role_name = RoleName::new(&name);
                    service.update_selected_role(role_name).await?;
                    service.save_config().await?;
                    println!("selected:{}", name);
                }
            }
            Ok(())
        }
        Command::Config { sub } => {
            match sub {
                ConfigSub::Show => {
                    let config = service.get_config().await;
                    println!("{}", serde_json::to_string_pretty(&config)?);
                }
                ConfigSub::Set { key, value } => match key.as_str() {
                    "selected_role" => {
                        let role_name = RoleName::new(&value);
                        service.update_selected_role(role_name).await?;
                        service.save_config().await?;
                        println!("updated selected_role to {}", value);
                    }
                    _ => {
                        println!("unsupported key: {}", key);
                    }
                },
            }
            Ok(())
        }
        Command::Graph { role, top_k } => {
            let role_name = if let Some(role) = role {
                RoleName::new(&role)
            } else {
                service.get_selected_role().await
            };

            let concepts = service.get_role_graph_top_k(&role_name, top_k).await?;
            for concept in concepts {
                println!("{}", concept);
            }
            Ok(())
        }
        Command::Chat {
            role,
            prompt,
            model,
        } => {
            let role_name = if let Some(role) = role {
                RoleName::new(&role)
            } else {
                service.get_selected_role().await
            };

            let response = service.chat(&role_name, &prompt, model).await?;
            println!("{}", response);
            Ok(())
        }
        Command::Extract {
            text,
            role,
            exclude_term,
        } => {
            let role_name = if let Some(role) = role {
                RoleName::new(&role)
            } else {
                service.get_selected_role().await
            };

            let results = service
                .extract_paragraphs(&role_name, &text, exclude_term)
                .await?;

            if results.is_empty() {
                println!("No matches found in the text.");
            } else {
                println!("Found {} paragraph(s):", results.len());
                for (i, (matched_term, paragraph)) in results.iter().enumerate() {
                    println!("\n--- Match {} (term: '{}') ---", i + 1, matched_term);
                    println!("{}", paragraph);
                }
            }

            Ok(())
        }
        Command::Replace { text, role, format } => {
            let role_name = if let Some(role) = role {
                RoleName::new(&role)
            } else {
                service.get_selected_role().await
            };

            let link_type = match format.as_deref() {
                Some("markdown") => terraphim_automata::LinkType::MarkdownLinks,
                Some("wiki") => terraphim_automata::LinkType::WikiLinks,
                Some("html") => terraphim_automata::LinkType::HTMLLinks,
                _ => terraphim_automata::LinkType::PlainText,
            };

            let result = service
                .replace_matches(&role_name, &text, link_type)
                .await?;
            println!("{}", result);

            Ok(())
        }
        Command::CheckUpdate => {
            println!("🔍 Checking for terraphim-tui updates...");
            match check_for_updates("terraphim-tui").await {
                Ok(status) => {
                    println!("{}", status);
                    Ok(())
                }
                Err(e) => {
                    eprintln!("❌ Failed to check for updates: {}", e);
                    std::process::exit(1);
                }
            }
        }
        Command::Update => {
            println!("🚀 Updating terraphim-tui...");
            match update_binary("terraphim-tui").await {
                Ok(status) => {
                    println!("{}", status);
                    Ok(())
                }
                Err(e) => {
                    eprintln!("❌ Update failed: {}", e);
                    std::process::exit(1);
                }
            }
        }
        Command::Interactive => {
            unreachable!("Interactive mode should be handled above")
        }

        #[cfg(feature = "repl")]
        Command::Repl { .. } => {
            unreachable!("REPL mode should be handled above")
        }
    }
}

async fn run_server_command(command: Command, server_url: &str) -> Result<()> {
    let api = ApiClient::new(server_url.to_string());

    match command {
        Command::Search {
            query,
            terms,
            operator,
            role,
            limit,
        } => {
            // Get selected role from server if not specified
            let role_name = if let Some(role) = role {
                RoleName::new(&role)
            } else {
                let config_res = api.get_config().await?;
                config_res.config.selected_role
            };

            let q = if let Some(additional_terms) = terms {
                // Multi-term query with logical operators
                let search_terms: Vec<NormalizedTermValue> = additional_terms
                    .into_iter()
                    .map(|t| NormalizedTermValue::from(t.as_str()))
                    .collect();

                SearchQuery {
                    search_term: NormalizedTermValue::from(query.as_str()),
                    search_terms: Some(search_terms),
                    operator: operator.map(|op| op.into()),
                    skip: Some(0),
                    limit: Some(limit),
                    role: Some(role_name),
                }
            } else {
                // Single term query (backward compatibility)
                SearchQuery {
                    search_term: NormalizedTermValue::from(query.as_str()),
                    search_terms: None,
                    operator: None,
                    skip: Some(0),
                    limit: Some(limit),
                    role: Some(role_name),
                }
            };

            let res: SearchResponse = api.search(&q).await?;

            if let Some(ref additional_terms) = q.search_terms {
                let op_str = match q.operator {
                    Some(LogicalOperator::And) => "AND",
                    Some(LogicalOperator::Or) => "OR",
                    None => "OR", // Default
                };
                println!(
                    "Multi-term search: '{}' {} {} additional terms using {} operator",
                    query,
                    op_str,
                    additional_terms.len(),
                    op_str
                );
            }

            for doc in res.results.iter() {
                println!("- {}\t{}", doc.rank.unwrap_or_default(), doc.title);
            }
            Ok(())
        }
        Command::Roles { sub } => {
            match sub {
                RolesSub::List => {
                    let cfg = api.get_config().await?;
                    let keys: Vec<String> =
                        cfg.config.roles.keys().map(|r| r.to_string()).collect();
                    println!("{}", keys.join("\n"));
                }
                RolesSub::Select { name } => {
                    let _ = api.update_selected_role(&name).await?;
                    println!("selected:{}", name);
                }
            }
            Ok(())
        }
        Command::Config { sub } => {
            match sub {
                ConfigSub::Show => {
                    let cfg = api.get_config().await?;
                    println!("{}", serde_json::to_string_pretty(&cfg.config)?);
                }
                ConfigSub::Set { key, value } => {
                    let mut cfg = api.get_config().await?.config;
                    match key.as_str() {
                        "selected_role" => {
                            cfg.selected_role = RoleName::new(&value);
                            let _ = api.post_config(&cfg).await?;
                            println!("updated selected_role to {}", value);
                        }
                        _ => {
                            println!("unsupported key: {}", key);
                        }
                    }
                }
            }
            Ok(())
        }
        Command::Graph { role, top_k } => {
            let role_name = if let Some(role) = role {
                role
            } else {
                let config_res = api.get_config().await?;
                config_res.config.selected_role.to_string()
            };

            let graph_res = api.rolegraph(Some(&role_name)).await?;
            let mut nodes_sorted = graph_res.nodes.clone();
            nodes_sorted.sort_by(|a, b| b.rank.cmp(&a.rank));
            for node in nodes_sorted.into_iter().take(top_k) {
                println!("{}", node.label);
            }
            Ok(())
        }
        Command::Chat {
            role,
            prompt,
            model,
        } => {
            let role_name = if let Some(role) = role {
                role
            } else {
                let config_res = api.get_config().await?;
                config_res.config.selected_role.to_string()
            };

            let chat_res = api.chat(&role_name, &prompt, model.as_deref()).await?;
            match (chat_res.status.as_str(), chat_res.message) {
                ("Success", Some(msg)) => println!("{}", msg),
                _ => println!(
                    "error: {}",
                    chat_res.error.unwrap_or_else(|| "unknown error".into())
                ),
            }
            Ok(())
        }
        Command::Extract {
            text,
            role,
            exclude_term,
        } => {
            let role_name = if let Some(role) = role {
                role
            } else {
                let config_res = api.get_config().await?;
                config_res.config.selected_role.to_string()
            };

            // Get the thesaurus from the server for the role
            let thesaurus_res = api.get_thesaurus(&role_name).await?;

            // Build thesaurus from response
            let mut thesaurus = terraphim_types::Thesaurus::new(format!("role-{}", role_name));
            for entry in thesaurus_res.terms {
                let normalized_term = terraphim_types::NormalizedTerm::new(
                    1, // Simple ID for CLI usage
                    terraphim_types::NormalizedTermValue::from(entry.nterm.clone()),
                );
                thesaurus.insert(
                    terraphim_types::NormalizedTermValue::from(entry.nterm),
                    normalized_term,
                );
            }

            // Extract paragraphs using automata
            let results = terraphim_automata::matcher::extract_paragraphs_from_automata(
                &text,
                thesaurus,
                !exclude_term, // include_term is opposite of exclude_term
            )?;

            if results.is_empty() {
                println!("No matches found in the text.");
            } else {
                println!("Found {} paragraph(s):", results.len());
                for (i, (matched, paragraph)) in results.iter().enumerate() {
                    println!(
                        "\n--- Match {} (term: '{}') ---",
                        i + 1,
                        matched.normalized_term.value
                    );
                    println!("{}", paragraph);
                }
            }

            Ok(())
        }
        Command::CheckUpdate => {
            println!("🔍 Checking for terraphim-tui updates...");
            match check_for_updates("terraphim-tui").await {
                Ok(status) => {
                    println!("{}", status);
                    Ok(())
                }
                Err(e) => {
                    eprintln!("❌ Failed to check for updates: {}", e);
                    std::process::exit(1);
                }
            }
        }
        Command::Update => {
            println!("🚀 Updating terraphim-tui...");
            match update_binary("terraphim-tui").await {
                Ok(status) => {
                    println!("{}", status);
                    Ok(())
                }
                Err(e) => {
                    eprintln!("❌ Update failed: {}", e);
                    std::process::exit(1);
                }
            }
        }
        Command::Replace { .. } => {
            eprintln!("Replace command is only available in offline mode");
            std::process::exit(1);
        }
        Command::Interactive => {
            unreachable!("Interactive mode should be handled above")
        }

        #[cfg(feature = "repl")]
        Command::Repl { .. } => {
            unreachable!("REPL mode should be handled above")
        }
    }
}

fn run_tui(transparent: bool) -> Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let res = ui_loop(&mut terminal, transparent);

    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    res
}

fn ui_loop(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>, transparent: bool) -> Result<()> {
    let mut input = String::new();
    let mut results: Vec<String> = Vec::new();
    let mut detailed_results: Vec<Document> = Vec::new();
    let mut terms: Vec<String> = Vec::new();
    let mut suggestions: Vec<String> = Vec::new();
    let mut current_role = String::from("Terraphim Engineer"); // Default to Terraphim Engineer
    let mut selected_result_index = 0;
    let mut view_mode = ViewMode::Search;
    let api = ApiClient::new(
        std::env::var("TERRAPHIM_SERVER").unwrap_or_else(|_| "http://localhost:8000".to_string()),
    );
    let rt = Runtime::new()?;

    // Initialize terms from rolegraph (selected role)
    if let Ok(cfg) = rt.block_on(async { api.get_config().await }) {
        current_role = cfg.config.selected_role.to_string();
        if let Ok(rg) = rt.block_on(async { api.rolegraph(Some(current_role.as_str())).await }) {
            terms = rg.nodes.into_iter().map(|n| n.label).collect();
        }
    }

    loop {
        terminal.draw(|f| {
            match view_mode {
                ViewMode::Search => {
                    let chunks = Layout::default()
                        .direction(Direction::Vertical)
                        .constraints([
                            Constraint::Length(3), // input
                            Constraint::Length(5), // suggestions
                            Constraint::Min(3),    // results
                            Constraint::Length(3), // status
                        ])
                        .split(f.area());

                    let input_title = format!("Search [Role: {}] • Enter: search, Tab: autocomplete, r: switch role, q: quit", current_role);
                    let input_widget = Paragraph::new(Line::from(input.as_str())).block(
                        create_block(&input_title, transparent)
                    );
                    f.render_widget(input_widget, chunks[0]);

                    // Suggestions (fixed height 5)
                    let sug_items: Vec<ListItem> = suggestions
                        .iter()
                        .take(5)
                        .map(|s| ListItem::new(s.as_str()))
                        .collect();
                    let sug_list = List::new(sug_items)
                        .block(create_block("Suggestions", transparent));
                    f.render_widget(sug_list, chunks[1]);

                    let items: Vec<ListItem> = results.iter().enumerate().map(|(i, r)| {
                        let item = ListItem::new(r.as_str());
                        if i == selected_result_index {
                            item.style(Style::default().add_modifier(Modifier::REVERSED))
                        } else {
                            item
                        }
                    }).collect();
                    let list = List::new(items)
                        .block(create_block("Results • ↑↓: select, Enter: view details, s: summarize", transparent));
                    f.render_widget(list, chunks[2]);

                    let status_text = format!("Terraphim TUI • {} results • Mode: Search", results.len());
                    let status = Paragraph::new(Line::from(status_text))
                        .block(create_block("", transparent));
                    f.render_widget(status, chunks[3]);
                }
                ViewMode::ResultDetail => {
                    if selected_result_index < detailed_results.len() {
                        let doc = &detailed_results[selected_result_index];

                        let chunks = Layout::default()
                            .direction(Direction::Vertical)
                            .constraints([
                                Constraint::Length(3), // title
                                Constraint::Min(5),    // content
                                Constraint::Length(3), // status
                            ])
                            .split(f.area());

                        let title_widget = Paragraph::new(Line::from(doc.title.as_str()))
                            .block(create_block("Document Title", transparent))
                            .wrap(ratatui::widgets::Wrap { trim: true });
                        f.render_widget(title_widget, chunks[0]);

                        let content_text = if doc.body.is_empty() { "No content available" } else { &doc.body };
                        let content_widget = Paragraph::new(content_text)
                            .block(create_block("Content • s: summarize, Esc: back to search", transparent))
                            .wrap(ratatui::widgets::Wrap { trim: true });
                        f.render_widget(content_widget, chunks[1]);

                        let status_text = format!("Document Detail • ID: {} • URL: {}",
                                                doc.id,
                                                if doc.url.is_empty() { "N/A" } else { &doc.url });
                        let status = Paragraph::new(Line::from(status_text))
                            .block(create_block("", transparent));
                        f.render_widget(status, chunks[2]);
                    }
                }
            }
        })?;

        if event::poll(std::time::Duration::from_millis(50))? {
            if let Event::Key(key) = event::read()? {
                match view_mode {
                    ViewMode::Search => {
                        match key.code {
                            KeyCode::Char('q') => break,
                            KeyCode::Enter => {
                                let query = input.trim().to_string();
                                let api = api.clone();
                                let role = current_role.clone();
                                if !query.is_empty() {
                                    if let Ok((lines, docs)) = rt.block_on(async move {
                                        let q = SearchQuery {
                                            search_term: NormalizedTermValue::from(query.as_str()),
                                            search_terms: None,
                                            operator: None,
                                            skip: Some(0),
                                            limit: Some(10),
                                            role: Some(RoleName::new(&role)),
                                        };
                                        let resp = api.search(&q).await?;
                                        let lines: Vec<String> = resp
                                            .results
                                            .iter()
                                            .map(|d| {
                                                format!(
                                                    "{} {}",
                                                    d.rank.unwrap_or_default(),
                                                    d.title
                                                )
                                            })
                                            .collect();
                                        let docs = resp.results;
                                        Ok::<(Vec<String>, Vec<Document>), anyhow::Error>((
                                            lines, docs,
                                        ))
                                    }) {
                                        results = lines;
                                        detailed_results = docs;
                                        selected_result_index = 0;
                                    }
                                } else if selected_result_index < detailed_results.len() {
                                    view_mode = ViewMode::ResultDetail;
                                }
                            }
                            KeyCode::Up => {
                                selected_result_index = selected_result_index.saturating_sub(1);
                            }
                            KeyCode::Down => {
                                if selected_result_index + 1 < results.len() {
                                    selected_result_index += 1;
                                }
                            }
                            KeyCode::Tab => {
                                // Real autocomplete from API
                                let query = input.trim();
                                if !query.is_empty() {
                                    let api = api.clone();
                                    let role = current_role.clone();
                                    if let Ok(autocomplete_resp) = rt.block_on(async move {
                                        api.get_autocomplete(&role, query).await
                                    }) {
                                        suggestions = autocomplete_resp
                                            .suggestions
                                            .into_iter()
                                            .take(5)
                                            .map(|s| s.text)
                                            .collect();
                                    }
                                }
                            }
                            KeyCode::Char('r') => {
                                // Switch role
                                let api = api.clone();
                                if let Ok(cfg) = rt.block_on(async { api.get_config().await }) {
                                    let roles: Vec<String> =
                                        cfg.config.roles.keys().map(|k| k.to_string()).collect();
                                    if !roles.is_empty() {
                                        if let Some(current_idx) =
                                            roles.iter().position(|r| r == &current_role)
                                        {
                                            let next_idx = (current_idx + 1) % roles.len();
                                            current_role = roles[next_idx].clone();
                                            // Update terms for new role
                                            if let Ok(rg) = rt.block_on(async {
                                                api.rolegraph(Some(&current_role)).await
                                            }) {
                                                terms =
                                                    rg.nodes.into_iter().map(|n| n.label).collect();
                                            }
                                        }
                                    }
                                }
                            }
                            KeyCode::Char('s') => {
                                // Summarize current selection
                                if selected_result_index < detailed_results.len() {
                                    let doc = detailed_results[selected_result_index].clone();
                                    let api = api.clone();
                                    let role = current_role.clone();
                                    if let Ok(summary) = rt.block_on(async move {
                                        api.summarize_document(&doc, Some(&role)).await
                                    }) {
                                        if let Some(summary_text) = summary.summary {
                                            // Replace result with summary for display
                                            if selected_result_index < results.len() {
                                                results[selected_result_index] =
                                                    format!("SUMMARY: {}", summary_text);
                                            }
                                        }
                                    }
                                }
                            }
                            KeyCode::Backspace => {
                                input.pop();
                                update_local_suggestions(&input, &terms, &mut suggestions);
                            }
                            KeyCode::Char(c) => {
                                input.push(c);
                                update_local_suggestions(&input, &terms, &mut suggestions);
                            }
                            _ => {}
                        }
                    }
                    ViewMode::ResultDetail => {
                        match key.code {
                            KeyCode::Esc => {
                                view_mode = ViewMode::Search;
                            }
                            KeyCode::Char('s') => {
                                // Summarize current document in detail view
                                if selected_result_index < detailed_results.len() {
                                    let doc = detailed_results[selected_result_index].clone();
                                    let api = api.clone();
                                    let role = current_role.clone();
                                    if let Ok(summary) = rt.block_on(async move {
                                        api.summarize_document(&doc, Some(&role)).await
                                    }) {
                                        if let Some(summary_text) = summary.summary {
                                            // Update the document body with summary
                                            let original_body = if detailed_results
                                                [selected_result_index]
                                                .body
                                                .is_empty()
                                            {
                                                "No content"
                                            } else {
                                                &detailed_results[selected_result_index].body
                                            };
                                            detailed_results[selected_result_index].body = format!(
                                                "SUMMARY:\n{}\n\nORIGINAL:\n{}",
                                                summary_text, original_body
                                            );
                                        }
                                    }
                                }
                            }
                            KeyCode::Char('q') => break,
                            _ => {}
                        }
                    }
                }
            }
        }
    }
    Ok(())
}

fn update_local_suggestions(input: &str, terms: &[String], suggestions: &mut Vec<String>) {
    let needle = input
        .rsplit_once(' ')
        .map(|(_, w)| w)
        .unwrap_or(input)
        .to_lowercase();
    *suggestions = if needle.is_empty() {
        Vec::new()
    } else {
        let mut s: Vec<String> = terms
            .iter()
            .filter(|t| t.to_lowercase().contains(&needle))
            .take(50)
            .cloned()
            .collect();
        s.truncate(5);
        s
    };
}
