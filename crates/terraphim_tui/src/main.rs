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
    style::{Modifier, Style},
    text::Line,
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Terminal,
};
use tokio::runtime::Runtime;

mod client;
use client::{ApiClient, SearchResponse};
use terraphim_types::{SearchQuery, NormalizedTermValue, RoleName};

#[derive(Parser, Debug)]
#[command(name = "terraphim-tui", version, about = "Terraphim TUI interface")] 
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand, Debug)]
enum Command {
    Search { query: String, #[arg(long, default_value = "Default")] role: String, #[arg(long, default_value_t = 10)] limit: usize },
    Roles { #[command(subcommand)] sub: RolesSub },
    Config { #[command(subcommand)] sub: ConfigSub },
    Graph { #[arg(long, default_value = "Default")] role: String, #[arg(long, default_value_t = 50)] top_k: usize },
    Chat { #[arg(long, default_value = "")] role: String, prompt: String, #[arg(long)] model: Option<String> },
    Interactive,
}

#[derive(Subcommand, Debug)]
enum RolesSub { List, Select { name: String } }

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
        Some(Command::Interactive) | None => run_tui(),
        Some(Command::Search { query, role, limit }) => {
            rt.block_on(async move {
                let api = ApiClient::new(std::env::var("TERRAPHIM_SERVER").unwrap_or_else(|_| "http://localhost:8000".to_string()));
                let q = SearchQuery {
                    search_term: NormalizedTermValue::from(query.as_str()),
                    skip: Some(0),
                    limit: Some(limit),
                    role: Some(RoleName::new(&role)),
                };
                let res: SearchResponse = api.search(&q).await?;
                for doc in res.results.iter() {
                    println!("- {}\t{}", doc.rank.unwrap_or_default(), doc.title);
                }
                Ok(())
            })
        }
        Some(Command::Roles { sub }) => {
            let api = ApiClient::new(std::env::var("TERRAPHIM_SERVER").unwrap_or_else(|_| "http://localhost:8000".to_string()));
            rt.block_on(async move {
                match sub {
                    RolesSub::List => {
                        let cfg = api.get_config().await?;
                        let keys: Vec<String> = cfg.config.roles.keys().map(|r| r.to_string()).collect();
                        println!("{}", keys.join("\n"));
                    }
                    RolesSub::Select { name } => {
                        let _ = api.update_selected_role(&name).await?;
                        println!("selected:{}", name);
                    }
                }
                Ok(())
            })
        }
        Some(Command::Config { sub }) => {
            let api = ApiClient::new(std::env::var("TERRAPHIM_SERVER").unwrap_or_else(|_| "http://localhost:8000".to_string()));
            rt.block_on(async move {
                match sub {
                    ConfigSub::Show => {
                        let cfg = api.get_config().await?;
                        println!("{}", serde_json::to_string_pretty(&cfg.config)?);
                    }
                    ConfigSub::Set { key, value } => {
                        // Minimal keys: selected_role, global_shortcut, role.<name>.theme
                        let mut cfg = api.get_config().await?.config;
                        match key.as_str() {
                            "selected_role" => { cfg.selected_role = RoleName::new(&value); }
                            "global_shortcut" => { cfg.global_shortcut = value.clone(); }
                            _ if key.starts_with("role.") && key.ends_with(".theme") => {
                                // role.<name>.theme
                                if let Some(name) = key.strip_prefix("role.").and_then(|s| s.strip_suffix(".theme")) {
                                    let rn = RoleName::new(name);
                                    if let Some(role) = cfg.roles.get_mut(&rn) {
                                        role.theme = value.clone();
                                    } else {
                                        eprintln!("role not found: {}", name);
                                    }
                                }
                            }
                            _ => {
                                eprintln!("unsupported key: {}", key);
                            }
                        }
                        let _ = api.post_config(&cfg).await?;
                        println!("ok");
                    }
                }
                Ok(())
            })
        }
        Some(Command::Graph { role, top_k }) => {
            let api = ApiClient::new(std::env::var("TERRAPHIM_SERVER").unwrap_or_else(|_| "http://localhost:8000".to_string()));
            rt.block_on(async move {
                let rg = api.get_rolegraph_edges(if role.is_empty() { None } else { Some(&role) }).await?;
                println!("Nodes: {}  Edges: {}", rg.nodes.len(), rg.edges.len());
                // Build adjacency: for top_k nodes by rank, show connected nodes (by edges)
                use std::collections::HashMap;
                let mut label_by_id: HashMap<u64, &str> = HashMap::new();
                for n in &rg.nodes { label_by_id.insert(n.id, &n.label); }
                let mut nodes_sorted = rg.nodes.clone();
                nodes_sorted.sort_by(|a,b| b.rank.cmp(&a.rank));
                for n in nodes_sorted.into_iter().take(top_k) {
                    // find neighbors via edges
                    let mut neighbors = Vec::new();
                    for e in &rg.edges {
                        if e.source == n.id { if let Some(lbl) = label_by_id.get(&e.target) { neighbors.push((*lbl, e.rank)); } }
                        if e.target == n.id { if let Some(lbl) = label_by_id.get(&e.source) { neighbors.push((*lbl, e.rank)); } }
                    }
                    neighbors.sort_by(|a,b| b.1.cmp(&a.1));
                    let list = neighbors.into_iter().take(5).map(|(l,_r)| l).collect::<Vec<&str>>().join(", ");
                    println!("- [{}] {} -> {}", n.rank, n.label, list);
                }
                Ok(())
            })
        }
        Some(Command::Chat { role, prompt, model }) => {
            let api = ApiClient::new(std::env::var("TERRAPHIM_SERVER").unwrap_or_else(|_| "http://localhost:8000".to_string()));
            rt.block_on(async move {
                let res = api.chat(if role.is_empty() { "Default" } else { &role }, &prompt, model.as_deref()).await?;
                match (res.status.as_str(), res.message) {
                    ("Success", Some(msg)) => println!("{}", msg),
                    _ => println!("error: {}", res.error.unwrap_or_else(|| "unknown error".into())),
                }
                Ok(())
            })
        }
    }
}

fn run_tui() -> Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let res = ui_loop(&mut terminal);

    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    res
}

fn ui_loop(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>) -> Result<()> {
    let mut input = String::new();
    let mut results: Vec<String> = Vec::new();
    let mut terms: Vec<String> = Vec::new();
    let mut suggestions: Vec<String> = Vec::new();
    let api = ApiClient::new(std::env::var("TERRAPHIM_SERVER").unwrap_or_else(|_| "http://localhost:8000".to_string()));
    let rt = Runtime::new()?;

    // Initialize terms from rolegraph (selected role)
    if let Ok(cfg) = rt.block_on(async { api.get_config().await }) {
        let sel = cfg.config.selected_role.to_string();
        if let Ok(rg) = rt.block_on(async { api.rolegraph(Some(sel.as_str())).await }) {
            terms = rg.nodes.into_iter().map(|n| n.label).collect();
        }
    }

    loop {
        terminal.draw(|f| {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(3), // input
                    Constraint::Length(5), // suggestions
                    Constraint::Min(3),    // results
                    Constraint::Length(1), // status
                ])
                .split(f.size());

            let input_widget = Paragraph::new(Line::from(input.as_str()))
                .block(Block::default().title("Search • Type to query, Enter to run, q to quit").borders(Borders::ALL));
            f.render_widget(input_widget, chunks[0]);

            // Suggestions (fixed height 5)
            let sug_items: Vec<ListItem> = suggestions
                .iter()
                .take(5)
                .map(|s| ListItem::new(s.as_str()))
                .collect();
            let sug_list = List::new(sug_items)
                .block(Block::default().title("Suggestions").borders(Borders::ALL));
            f.render_widget(sug_list, chunks[1]);

            let items: Vec<ListItem> = results
                .iter()
                .map(|r| ListItem::new(r.as_str()))
                .collect();
            let list = List::new(items)
                .block(Block::default().title("Results").borders(Borders::ALL))
                .highlight_style(Style::default().add_modifier(Modifier::BOLD));
            f.render_widget(list, chunks[2]);

            let status = Paragraph::new(Line::from("Terraphim TUI (MVP)"))
                .block(Block::default().borders(Borders::ALL));
            f.render_widget(status, chunks[3]);
        })?;

        if event::poll(std::time::Duration::from_millis(50))? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Char('q') => break,
                    KeyCode::Enter => {
                        let query = input.trim().to_string();
                        let api = api.clone();
                        if !query.is_empty() {
                            if let Ok(lines) = rt.block_on(async move {
                                let q = SearchQuery { search_term: NormalizedTermValue::from(query.as_str()), skip: Some(0), limit: Some(10), role: None };
                                let resp = api.search(&q).await?;
                                let lines: Vec<String> = resp.results.into_iter().map(|d| format!("{} {}", d.rank.unwrap_or_default(), d.title)).collect();
                                Ok::<Vec<String>, anyhow::Error>(lines)
                            }) {
                                results = lines;
                            }
                        }
                    }
                    KeyCode::Backspace => {
                        input.pop();
                        // update suggestions
                        let needle = input.rsplit_once(' ').map(|(_, w)| w).unwrap_or(input.as_str()).to_lowercase();
                        suggestions = if needle.is_empty() { Vec::new() } else {
                            let mut s: Vec<String> = terms.iter().filter(|t| t.to_lowercase().contains(&needle)).take(50).cloned().collect();
                            s.truncate(5);
                            s
                        };
                    }
                    KeyCode::Char(c) => {
                        input.push(c);
                        // update suggestions
                        let needle = input.rsplit_once(' ').map(|(_, w)| w).unwrap_or(input.as_str()).to_lowercase();
                        suggestions = if needle.is_empty() { Vec::new() } else {
                            let mut s: Vec<String> = terms.iter().filter(|t| t.to_lowercase().contains(&needle)).take(50).cloned().collect();
                            s.truncate(5);
                            s
                        };
                    }
                    _ => {}
                }
            }
        }
    }
    Ok(())
}


