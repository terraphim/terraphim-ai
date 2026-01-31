use std::io;

use anyhow::Result;
use clap::{Parser, Subcommand};
use crossterm::{
    event::{
        self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEvent, KeyModifiers,
    },
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{
    Terminal,
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::Line,
    widgets::{Block, Borders, List, ListItem, Paragraph},
};
use tokio::runtime::Runtime;

mod client;
mod guard_patterns;
mod onboarding;
mod service;

// Robot mode and forgiving CLI - always available
mod forgiving;
mod robot;

#[cfg(feature = "repl")]
mod repl;

use client::{ApiClient, SearchResponse};
use service::TuiService;
use terraphim_types::{Document, LogicalOperator, NormalizedTermValue, RoleName, SearchQuery};
use terraphim_update::{check_for_updates, check_for_updates_startup, update_binary};

#[derive(clap::ValueEnum, Debug, Clone)]
enum LogicalOperatorCli {
    And,
    Or,
}

/// Show helpful usage information when run without a TTY
fn show_usage_info() {
    println!("Terraphim AI Agent v{}", env!("CARGO_PKG_VERSION"));
    println!();
    println!("Interactive Mode (requires TTY):");
    println!("  terraphim-agent              # Start REPL (default)");
    println!("  terraphim-agent --tui        # Start TUI (requires server)");
    println!("  terraphim-agent repl         # Explicit REPL mode");
    println!();
    println!("Common Commands:");
    println!("  search <query>               # Search documents");
    println!("  roles list                   # List available roles");
    println!("  config show                  # Show configuration");
    println!("  replace <text>               # Replace terms using thesaurus");
    println!("  validate <text>              # Validate against knowledge graph");
    println!();
    println!("For more information:");
    println!("  terraphim-agent --help       # Show full help");
    println!("  terraphim-agent help         # Show command-specific help");
}

impl From<LogicalOperatorCli> for LogicalOperator {
    fn from(op: LogicalOperatorCli) -> Self {
        match op {
            LogicalOperatorCli::And => LogicalOperator::And,
            LogicalOperatorCli::Or => LogicalOperator::Or,
        }
    }
}

/// Hook types for Claude Code integration
#[derive(clap::ValueEnum, Debug, Clone)]
pub enum HookType {
    /// Pre-tool-use hook (intercepts tool calls)
    PreToolUse,
    /// Post-tool-use hook (processes tool results)
    PostToolUse,
    /// Pre-commit hook (validate before commit)
    PreCommit,
    /// Prepare-commit-msg hook (enhance commit message)
    PrepareCommitMsg,
}

/// Boundary mode for text replacement
#[derive(clap::ValueEnum, Debug, Clone, Default)]
pub enum BoundaryMode {
    /// Match anywhere (default, current behavior)
    #[default]
    None,
    /// Only match at word boundaries
    Word,
}

/// Check if a character is a word boundary character (not alphanumeric).
fn is_word_boundary_char(c: char) -> bool {
    !c.is_alphanumeric() && c != '_'
}

/// Check if a match position is at word boundaries in the text.
/// Returns true if the character before start (or start of string) and
/// the character after end (or end of string) are word boundary characters.
fn is_at_word_boundary(text: &str, start: usize, end: usize) -> bool {
    // Check character before start
    let before_ok = if start == 0 {
        true
    } else {
        text[..start]
            .chars()
            .last()
            .map(is_word_boundary_char)
            .unwrap_or(true)
    };

    // Check character after end
    let after_ok = if end >= text.len() {
        true
    } else {
        text[end..]
            .chars()
            .next()
            .map(is_word_boundary_char)
            .unwrap_or(true)
    };

    before_ok && after_ok
}

/// Format a replacement link from a NormalizedTerm and LinkType.
fn format_replacement_link(
    term: &terraphim_types::NormalizedTerm,
    link_type: terraphim_hooks::LinkType,
) -> String {
    let display_text = term.display();
    match link_type {
        terraphim_hooks::LinkType::WikiLinks => format!("[[{}]]", display_text),
        terraphim_hooks::LinkType::HTMLLinks => format!(
            "<a href=\"{}\">{}</a>",
            term.url.as_deref().unwrap_or_default(),
            display_text
        ),
        terraphim_hooks::LinkType::MarkdownLinks => format!(
            "[{}]({})",
            display_text,
            term.url.as_deref().unwrap_or_default()
        ),
        terraphim_hooks::LinkType::PlainText => display_text.to_string(),
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TuiAction {
    None,
    Quit,
    SearchOrOpen,
    MoveUp,
    MoveDown,
    Autocomplete,
    SwitchRole,
    SummarizeSelection,
    SummarizeDetail,
    Backspace,
    InsertChar(char),
    BackToSearch,
}

#[cfg(test)]
fn key_event(code: KeyCode, modifiers: KeyModifiers) -> KeyEvent {
    KeyEvent::new(code, modifiers)
}

fn map_search_key_event(event: KeyEvent) -> TuiAction {
    match (event.code, event.modifiers) {
        (KeyCode::Char('q'), KeyModifiers::CONTROL) => TuiAction::Quit,
        (KeyCode::Esc, KeyModifiers::NONE) => TuiAction::Quit,
        (KeyCode::Enter, KeyModifiers::NONE) => TuiAction::SearchOrOpen,
        (KeyCode::Up, KeyModifiers::NONE) => TuiAction::MoveUp,
        (KeyCode::Down, KeyModifiers::NONE) => TuiAction::MoveDown,
        (KeyCode::Tab, KeyModifiers::NONE) => TuiAction::Autocomplete,
        (KeyCode::Char('r'), KeyModifiers::CONTROL) => TuiAction::SwitchRole,
        (KeyCode::Char('s'), KeyModifiers::CONTROL) => TuiAction::SummarizeSelection,
        (KeyCode::Backspace, KeyModifiers::NONE) => TuiAction::Backspace,
        (KeyCode::Char(c), KeyModifiers::NONE) => TuiAction::InsertChar(c),
        _ => TuiAction::None,
    }
}

fn map_detail_key_event(event: KeyEvent) -> TuiAction {
    match (event.code, event.modifiers) {
        (KeyCode::Esc, KeyModifiers::NONE) => TuiAction::BackToSearch,
        (KeyCode::Char('q'), KeyModifiers::CONTROL) => TuiAction::Quit,
        (KeyCode::Char('s'), KeyModifiers::CONTROL) => TuiAction::SummarizeDetail,
        _ => TuiAction::None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn map_search_key_event_allows_plain_letters() {
        assert_eq!(
            map_search_key_event(key_event(KeyCode::Char('s'), KeyModifiers::NONE)),
            TuiAction::InsertChar('s')
        );
        assert_eq!(
            map_search_key_event(key_event(KeyCode::Char('r'), KeyModifiers::NONE)),
            TuiAction::InsertChar('r')
        );
        // 'q' should also be typeable now
        assert_eq!(
            map_search_key_event(key_event(KeyCode::Char('q'), KeyModifiers::NONE)),
            TuiAction::InsertChar('q')
        );
    }

    #[test]
    fn map_search_key_event_ctrl_shortcuts() {
        assert_eq!(
            map_search_key_event(key_event(KeyCode::Char('s'), KeyModifiers::CONTROL)),
            TuiAction::SummarizeSelection
        );
        assert_eq!(
            map_search_key_event(key_event(KeyCode::Char('r'), KeyModifiers::CONTROL)),
            TuiAction::SwitchRole
        );
        // Ctrl+q quits
        assert_eq!(
            map_search_key_event(key_event(KeyCode::Char('q'), KeyModifiers::CONTROL)),
            TuiAction::Quit
        );
        // Esc also quits in search mode
        assert_eq!(
            map_search_key_event(key_event(KeyCode::Esc, KeyModifiers::NONE)),
            TuiAction::Quit
        );
    }

    #[test]
    fn map_detail_key_event_ctrl_s_summarizes() {
        assert_eq!(
            map_detail_key_event(key_event(KeyCode::Char('s'), KeyModifiers::CONTROL)),
            TuiAction::SummarizeDetail
        );
        assert_eq!(
            map_detail_key_event(key_event(KeyCode::Char('s'), KeyModifiers::NONE)),
            TuiAction::None
        );
    }

    #[test]
    fn map_detail_key_event_ctrl_q_quits() {
        // Ctrl+q quits in detail mode
        assert_eq!(
            map_detail_key_event(key_event(KeyCode::Char('q'), KeyModifiers::CONTROL)),
            TuiAction::Quit
        );
        // Plain 'q' does nothing (no typing in detail mode)
        assert_eq!(
            map_detail_key_event(key_event(KeyCode::Char('q'), KeyModifiers::NONE)),
            TuiAction::None
        );
        // Esc goes back to search, not quit
        assert_eq!(
            map_detail_key_event(key_event(KeyCode::Esc, KeyModifiers::NONE)),
            TuiAction::BackToSearch
        );
    }

    #[test]
    fn test_is_word_boundary_char() {
        // Non-alphanumeric chars are boundaries
        assert!(is_word_boundary_char(' '));
        assert!(is_word_boundary_char('\t'));
        assert!(is_word_boundary_char('\n'));
        assert!(is_word_boundary_char('.'));
        assert!(is_word_boundary_char(','));
        assert!(is_word_boundary_char('('));
        assert!(is_word_boundary_char(')'));
        assert!(is_word_boundary_char('"'));

        // Alphanumeric chars are NOT boundaries
        assert!(!is_word_boundary_char('a'));
        assert!(!is_word_boundary_char('Z'));
        assert!(!is_word_boundary_char('0'));
        assert!(!is_word_boundary_char('9'));

        // Underscore is NOT a boundary (word char in most regex)
        assert!(!is_word_boundary_char('_'));
    }

    #[test]
    fn test_is_at_word_boundary_start_of_string() {
        // At start of string, "npm" should be at boundary
        let text = "npm install";
        assert!(is_at_word_boundary(text, 0, 3)); // "npm" at start
    }

    #[test]
    fn test_is_at_word_boundary_end_of_string() {
        // At end of string, "npm" should be at boundary
        let text = "install npm";
        assert!(is_at_word_boundary(text, 8, 11)); // "npm" at end
    }

    #[test]
    fn test_is_at_word_boundary_middle_with_spaces() {
        // In middle with spaces, "npm" should be at boundary
        let text = "run npm install";
        assert!(is_at_word_boundary(text, 4, 7)); // "npm" surrounded by spaces
    }

    #[test]
    fn test_is_at_word_boundary_not_at_boundary() {
        // "npm" embedded in "anpmb" should NOT be at boundary
        let text = "anpmb";
        assert!(!is_at_word_boundary(text, 1, 4)); // "npm" embedded
    }

    #[test]
    fn test_is_at_word_boundary_partial_boundary() {
        // "npm" at start but not end: "npma"
        let text = "npma";
        assert!(!is_at_word_boundary(text, 0, 3)); // "npm" no boundary after

        // "npm" at end but not start: "anpm"
        let text2 = "anpm";
        assert!(!is_at_word_boundary(text2, 1, 4)); // "npm" no boundary before
    }

    #[test]
    fn test_is_at_word_boundary_with_punctuation() {
        // Punctuation counts as boundary
        let text = "(npm)";
        assert!(is_at_word_boundary(text, 1, 4)); // "npm" between parens

        let text2 = "use npm, please";
        assert!(is_at_word_boundary(text2, 4, 7)); // "npm" followed by comma
    }
}

#[derive(clap::ValueEnum, Debug, Clone, Default)]
pub enum OutputFormat {
    /// Human-readable output (default)
    #[default]
    Human,
    /// Machine-readable JSON output
    Json,
    /// Compact JSON for piping
    JsonCompact,
}

#[derive(Parser, Debug)]
#[command(name = "terraphim-agent", version, about = "Terraphim TUI interface")]
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
    /// Enable robot mode for AI agent integration (JSON output, exit codes)
    #[arg(long, default_value_t = false)]
    robot: bool,
    /// Output format (human, json, json-compact)
    #[arg(long, value_enum, default_value_t = OutputFormat::Human)]
    format: OutputFormat,
    /// Force TUI mode (default is REPL)
    #[arg(long, default_value_t = false)]
    tui: bool,
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
        /// Text to replace (reads from stdin if not provided)
        text: Option<String>,
        #[arg(long)]
        role: Option<String>,
        /// Output format: plain (default), markdown, wiki, html
        #[arg(long)]
        format: Option<String>,
        /// Boundary mode: none (match anywhere) or word (only at word boundaries)
        #[arg(long, default_value = "none")]
        boundary: BoundaryMode,
        /// Output as JSON with metadata (for hook integration)
        #[arg(long, default_value_t = false)]
        json: bool,
        /// Suppress errors and pass through unchanged on failure
        #[arg(long, default_value_t = false)]
        fail_open: bool,
    },
    /// Validate text against knowledge graph
    Validate {
        /// Text to validate (reads from stdin if not provided)
        text: Option<String>,
        /// Role to use for validation
        #[arg(long)]
        role: Option<String>,
        /// Check if all matched terms are connected by a single path
        #[arg(long, default_value_t = false)]
        connectivity: bool,
        /// Validate against a named checklist (e.g., "code_review", "security")
        #[arg(long)]
        checklist: Option<String>,
        /// Output as JSON
        #[arg(long, default_value_t = false)]
        json: bool,
    },
    /// Suggest similar terms using fuzzy matching
    Suggest {
        /// Query to search for (reads from stdin if not provided)
        query: Option<String>,
        /// Role to use for suggestions
        #[arg(long)]
        role: Option<String>,
        /// Enable fuzzy matching
        #[arg(long, default_value_t = true)]
        fuzzy: bool,
        /// Minimum similarity threshold (0.0-1.0)
        #[arg(long, default_value_t = 0.6)]
        threshold: f64,
        /// Maximum number of suggestions
        #[arg(long, default_value_t = 10)]
        limit: usize,
        /// Output as JSON
        #[arg(long, default_value_t = false)]
        json: bool,
    },
    /// Unified hook handler for Claude Code integration
    Hook {
        /// Hook type (pre-tool-use, post-tool-use, pre-commit, etc.)
        #[arg(long, value_enum)]
        hook_type: HookType,
        /// JSON input from Claude Code (reads from stdin if not provided)
        #[arg(long)]
        input: Option<String>,
        /// Role to use for processing
        #[arg(long)]
        role: Option<String>,
        /// Output as JSON (always true for hooks, but explicit)
        #[arg(long, default_value_t = true)]
        json: bool,
        /// Include guard check for destructive commands (git reset --hard, rm -rf, etc.)
        #[arg(long, default_value_t = false)]
        with_guard: bool,
    },
    /// Check command against safety guard patterns (blocks destructive git/fs commands)
    Guard {
        /// Command to check (reads from stdin if not provided)
        command: Option<String>,
        /// Output as JSON
        #[arg(long, default_value_t = false)]
        json: bool,
        /// Suppress errors and pass through unchanged on failure
        #[arg(long, default_value_t = false)]
        fail_open: bool,
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

    /// Interactive setup wizard for first-time configuration
    Setup {
        /// Apply a specific template directly (skip interactive wizard)
        #[arg(long)]
        template: Option<String>,
        /// Path to use with the template (required for some templates like local-notes)
        #[arg(long)]
        path: Option<String>,
        /// Add a new role to existing configuration (instead of replacing)
        #[arg(long, default_value_t = false)]
        add_role: bool,
        /// List available templates and exit
        #[arg(long, default_value_t = false)]
        list_templates: bool,
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
    let cli = Cli::parse();

    // Check for updates on startup (non-blocking, logs warning on failure)
    let rt = Runtime::new()?;
    rt.block_on(async {
        if let Err(e) = check_for_updates_startup("terraphim-agent").await {
            eprintln!("Update check failed: {}", e);
        }
    });

    match cli.command {
        Some(Command::Interactive) => {
            // Explicit interactive command - force TUI mode
            // Check if we're in a TTY for interactive mode (both stdout and stdin required)
            use atty::Stream;
            if !atty::is(Stream::Stdout) {
                show_usage_info();
                std::process::exit(0);
            }

            if !atty::is(Stream::Stdin) {
                show_usage_info();
                std::process::exit(0);
            }

            if cli.server {
                run_tui_server_mode(&cli.server_url, cli.transparent)
            } else {
                // Run TUI mode - it will create its own runtime
                run_tui_offline_mode(cli.transparent)
            }
        }

        None => {
            // No command specified - default to REPL mode or TUI mode if --tui flag is set
            if cli.tui {
                // Check if we're in a TTY for TUI mode
                use atty::Stream;
                if !atty::is(Stream::Stdout) {
                    eprintln!("Error: TUI mode requires a TTY (terminal)");
                    std::process::exit(1);
                }

                if !atty::is(Stream::Stdin) {
                    eprintln!("Error: TUI mode requires a TTY (terminal)");
                    std::process::exit(1);
                }

                if cli.server {
                    run_tui_server_mode(&cli.server_url, cli.transparent)
                } else {
                    run_tui_offline_mode(cli.transparent)
                }
            } else {
                // Default to REPL mode
                #[cfg(feature = "repl")]
                {
                    let rt = Runtime::new()?;
                    if cli.server {
                        rt.block_on(repl::run_repl_server_mode(&cli.server_url))
                    } else {
                        rt.block_on(repl::run_repl_offline_mode())
                    }
                }

                #[cfg(not(feature = "repl"))]
                {
                    // If repl feature is not enabled, show error
                    anyhow::bail!(
                        "REPL mode requires 'repl' feature. Build with: cargo build --features repl"
                    );
                }
            }
        }

        #[cfg(feature = "repl")]
        Some(Command::Repl { server, server_url }) => {
            let rt = Runtime::new()?;
            if server {
                rt.block_on(repl::run_repl_server_mode(&server_url))
            } else {
                rt.block_on(repl::run_repl_offline_mode())
            }
        }

        Some(command) => {
            let rt = Runtime::new()?;
            if cli.server {
                rt.block_on(run_server_command(command, &cli.server_url))
            } else {
                rt.block_on(run_offline_command(command))
            }
        }
    }
}
fn run_tui_offline_mode(transparent: bool) -> Result<()> {
    // Note: TUI mode currently requires a running server to connect to.
    // It will try TERRAPHIM_SERVER env var, or default to http://localhost:8000.
    // For fully offline operation, use 'terraphim-agent repl' instead.
    run_tui(None, transparent)
}

fn run_tui_server_mode(server_url: &str, transparent: bool) -> Result<()> {
    run_tui(Some(server_url.to_string()), transparent)
}

async fn run_offline_command(command: Command) -> Result<()> {
    // Handle stateless commands that don't need TuiService first
    if let Command::Guard {
        command,
        json,
        fail_open,
    } = &command
    {
        let input_command = match command {
            Some(c) => c.clone(),
            None => {
                use std::io::Read;
                let mut buffer = String::new();
                std::io::stdin().read_to_string(&mut buffer)?;
                buffer.trim().to_string()
            }
        };

        let guard = guard_patterns::CommandGuard::new();
        let result = guard.check(&input_command);

        if *json {
            println!("{}", serde_json::to_string(&result)?);
        } else if result.decision == "block" {
            if let Some(reason) = &result.reason {
                eprintln!("BLOCKED: {}", reason);
                if !fail_open {
                    std::process::exit(1);
                }
            }
        }
        // If allowed, no output in non-JSON mode (silent success)
        return Ok(());
    }

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
                    let roles_with_info = service.list_roles_with_info().await;
                    let selected = service.get_selected_role().await;
                    for (name, shortname) in roles_with_info {
                        let marker = if name == selected.to_string() {
                            "*"
                        } else {
                            " "
                        };
                        if let Some(short) = shortname {
                            println!("{} {} ({})", marker, name, short);
                        } else {
                            println!("{} {}", marker, name);
                        }
                    }
                }
                RolesSub::Select { name } => {
                    // Find role by name or shortname
                    let role_name = service
                        .find_role_by_name_or_shortname(&name)
                        .await
                        .ok_or_else(|| {
                            anyhow::anyhow!(
                                "Role '{}' not found (checked name and shortname)",
                                name
                            )
                        })?;
                    service.update_selected_role(role_name.clone()).await?;
                    service.save_config().await?;
                    println!("selected:{}", role_name);
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
        Command::Replace {
            text,
            role,
            format,
            boundary,
            json,
            fail_open,
        } => {
            let input_text = match text {
                Some(t) => t,
                None => {
                    use std::io::Read;
                    let mut buffer = String::new();
                    std::io::stdin().read_to_string(&mut buffer)?;
                    buffer
                }
            };

            let role_name = if let Some(role) = role {
                RoleName::new(&role)
            } else {
                service.get_selected_role().await
            };

            let link_type = match format.as_deref() {
                Some("markdown") => terraphim_hooks::LinkType::MarkdownLinks,
                Some("wiki") => terraphim_hooks::LinkType::WikiLinks,
                Some("html") => terraphim_hooks::LinkType::HTMLLinks,
                _ => terraphim_hooks::LinkType::PlainText,
            };

            let thesaurus = match service.get_thesaurus(&role_name).await {
                Ok(t) => t,
                Err(e) => {
                    if fail_open {
                        let hook_result = terraphim_hooks::HookResult::fail_open(
                            input_text.clone(),
                            e.to_string(),
                        );
                        if json {
                            println!("{}", serde_json::to_string(&hook_result)?);
                        } else {
                            eprintln!("Warning: {}", e);
                            print!("{}", input_text);
                        }
                        return Ok(());
                    } else {
                        return Err(e);
                    }
                }
            };

            let replacement_service = terraphim_hooks::ReplacementService::new(thesaurus.clone())
                .with_link_type(link_type);

            let hook_result = match boundary {
                BoundaryMode::None => {
                    // Standard replacement - match anywhere
                    if fail_open {
                        replacement_service.replace_fail_open(&input_text)
                    } else {
                        replacement_service.replace(&input_text)?
                    }
                }
                BoundaryMode::Word => {
                    // Word boundary mode - only match at word boundaries
                    let matches_result = replacement_service.find_matches(&input_text);
                    match matches_result {
                        Ok(matches) => {
                            // Filter matches to only those at word boundaries
                            let filtered_matches: Vec<_> = matches
                                .into_iter()
                                .filter(|m| {
                                    if let Some((start, end)) = m.pos {
                                        is_at_word_boundary(&input_text, start, end)
                                    } else {
                                        false
                                    }
                                })
                                .collect();

                            if filtered_matches.is_empty() {
                                terraphim_hooks::HookResult::pass_through(input_text.clone())
                            } else {
                                // Apply filtered matches in reverse order to preserve positions
                                let mut result = input_text.clone();
                                let mut sorted_matches = filtered_matches;
                                sorted_matches.sort_by(|a, b| b.pos.cmp(&a.pos)); // Reverse sort by position

                                for m in sorted_matches {
                                    if let Some((start, end)) = m.pos {
                                        let replacement =
                                            format_replacement_link(&m.normalized_term, link_type);
                                        result.replace_range(start..end, &replacement);
                                    }
                                }

                                terraphim_hooks::HookResult::success(input_text.clone(), result)
                            }
                        }
                        Err(e) => {
                            if fail_open {
                                terraphim_hooks::HookResult::fail_open(
                                    input_text.clone(),
                                    e.to_string(),
                                )
                            } else {
                                return Err(anyhow::anyhow!("Failed to find matches: {}", e));
                            }
                        }
                    }
                }
            };

            if json {
                println!("{}", serde_json::to_string(&hook_result)?);
            } else {
                if let Some(ref err) = hook_result.error {
                    eprintln!("Warning: {}", err);
                }
                print!("{}", hook_result.result);
            }

            Ok(())
        }
        Command::Validate {
            text,
            role,
            connectivity,
            checklist,
            json,
        } => {
            let input_text = match text {
                Some(t) => t,
                None => {
                    use std::io::Read;
                    let mut buffer = String::new();
                    std::io::stdin().read_to_string(&mut buffer)?;
                    buffer.trim().to_string()
                }
            };

            let role_name = if let Some(role) = role {
                RoleName::new(&role)
            } else {
                service.get_selected_role().await
            };

            if connectivity {
                let result = service.check_connectivity(&role_name, &input_text).await?;

                if json {
                    println!("{}", serde_json::to_string(&result)?);
                } else {
                    println!("Connectivity Check for role '{}':", role_name);
                    println!("  Connected: {}", result.connected);
                    println!("  Matched terms: {:?}", result.matched_terms);
                    println!("  {}", result.message);
                }
            } else if let Some(checklist_name) = checklist {
                // Checklist validation mode
                let result = service
                    .validate_checklist(&role_name, &checklist_name, &input_text)
                    .await?;

                if json {
                    println!("{}", serde_json::to_string(&result)?);
                } else {
                    println!(
                        "Checklist '{}' Validation for role '{}':",
                        checklist_name, role_name
                    );
                    println!("  Passed: {}", result.passed);
                    println!("  Score: {}/{}", result.satisfied.len(), result.total_items);
                    if !result.satisfied.is_empty() {
                        println!("  Satisfied items:");
                        for item in &result.satisfied {
                            println!("    ✓ {}", item);
                        }
                    }
                    if !result.missing.is_empty() {
                        println!("  Missing items:");
                        for item in &result.missing {
                            println!("    ✗ {}", item);
                        }
                    }
                }
            } else {
                // Default validation: find matches
                let matches = service.find_matches(&role_name, &input_text).await?;

                if json {
                    let output = serde_json::json!({
                        "role": role_name.to_string(),
                        "matched_count": matches.len(),
                        "matches": matches.iter().map(|m| m.term.clone()).collect::<Vec<_>>()
                    });
                    println!("{}", serde_json::to_string(&output)?);
                } else {
                    println!("Validation for role '{}':", role_name);
                    println!("  Found {} matched term(s)", matches.len());
                    for m in &matches {
                        println!("    - {}", m.term);
                    }
                }
            }

            Ok(())
        }
        Command::Suggest {
            query,
            role,
            fuzzy: _,
            threshold,
            limit,
            json,
        } => {
            let input_query = match query {
                Some(q) => q,
                None => {
                    use std::io::Read;
                    let mut buffer = String::new();
                    std::io::stdin().read_to_string(&mut buffer)?;
                    buffer.trim().to_string()
                }
            };

            let role_name = if let Some(role) = role {
                RoleName::new(&role)
            } else {
                service.get_selected_role().await
            };

            let suggestions = service
                .fuzzy_suggest(&role_name, &input_query, threshold, Some(limit))
                .await?;

            if json {
                println!("{}", serde_json::to_string(&suggestions)?);
            } else if suggestions.is_empty() {
                println!(
                    "No suggestions found for '{}' with threshold {}",
                    input_query, threshold
                );
            } else {
                println!(
                    "Suggestions for '{}' (threshold: {}):",
                    input_query, threshold
                );
                for s in &suggestions {
                    println!("  {} (similarity: {:.2})", s.term, s.similarity);
                }
            }

            Ok(())
        }
        Command::Hook {
            hook_type,
            input,
            role,
            json: _,
            with_guard,
        } => {
            // Read JSON input from argument or stdin
            let input_json = match input {
                Some(i) => i,
                None => {
                    use std::io::Read;
                    let mut buffer = String::new();
                    std::io::stdin().read_to_string(&mut buffer)?;
                    buffer
                }
            };

            let role_name = if let Some(role) = role {
                RoleName::new(&role)
            } else {
                service.get_selected_role().await
            };

            // Parse input JSON
            let input_value: serde_json::Value = serde_json::from_str(&input_json)
                .map_err(|e| anyhow::anyhow!("Invalid JSON input: {}", e))?;

            match hook_type {
                HookType::PreToolUse => {
                    // Extract tool_name and tool_input from the hook input
                    let tool_name = input_value
                        .get("tool_name")
                        .and_then(|v| v.as_str())
                        .unwrap_or("");

                    // Only process Bash commands
                    if tool_name == "Bash" {
                        if let Some(command) = input_value
                            .get("tool_input")
                            .and_then(|v| v.get("command"))
                            .and_then(|v| v.as_str())
                        {
                            // Guard check if --with-guard flag is set
                            if with_guard {
                                let guard = guard_patterns::CommandGuard::new();
                                let guard_result = guard.check(command);

                                if guard_result.decision == "block" {
                                    // Output deny response for Claude Code
                                    let output = serde_json::json!({
                                        "hookSpecificOutput": {
                                            "hookEventName": "PreToolUse",
                                            "permissionDecision": "deny",
                                            "permissionDecisionReason": format!(
                                                "BLOCKED: {}",
                                                guard_result.reason.unwrap_or_default()
                                            )
                                        }
                                    });
                                    println!("{}", serde_json::to_string(&output)?);
                                    return Ok(());
                                }
                            }

                            // Get thesaurus and perform replacement
                            let thesaurus = service.get_thesaurus(&role_name).await?;
                            let replacement_service =
                                terraphim_hooks::ReplacementService::new(thesaurus);
                            let hook_result = replacement_service.replace_fail_open(command);

                            // If replacement occurred, output modified input
                            if hook_result.replacements > 0 {
                                let mut output = input_value.clone();
                                if let Some(tool_input) = output.get_mut("tool_input") {
                                    if let Some(obj) = tool_input.as_object_mut() {
                                        obj.insert(
                                            "command".to_string(),
                                            serde_json::Value::String(hook_result.result.clone()),
                                        );
                                    }
                                }
                                println!("{}", serde_json::to_string(&output)?);
                            } else {
                                // No changes, pass through
                                println!("{}", input_json);
                            }
                        } else {
                            // No command to process
                            println!("{}", input_json);
                        }
                    } else {
                        // Not a Bash command, pass through
                        println!("{}", input_json);
                    }
                }
                HookType::PostToolUse => {
                    // Post-tool-use: validate output against checklist or connectivity
                    let tool_result = input_value
                        .get("tool_result")
                        .and_then(|v| v.as_str())
                        .unwrap_or("");

                    // Check connectivity of the output
                    let connectivity = service.check_connectivity(&role_name, tool_result).await?;

                    let output = serde_json::json!({
                        "original": input_value,
                        "validation": {
                            "connected": connectivity.connected,
                            "matched_terms": connectivity.matched_terms
                        }
                    });
                    println!("{}", serde_json::to_string(&output)?);
                }
                HookType::PreCommit | HookType::PrepareCommitMsg => {
                    // Extract commit message or diff
                    let content = input_value
                        .get("message")
                        .or_else(|| input_value.get("diff"))
                        .and_then(|v| v.as_str())
                        .unwrap_or("");

                    // Extract concepts from the content
                    let matches = service.find_matches(&role_name, content).await?;
                    let concepts: Vec<String> = matches.iter().map(|m| m.term.clone()).collect();

                    let output = serde_json::json!({
                        "original": input_value,
                        "concepts": concepts,
                        "concept_count": concepts.len()
                    });
                    println!("{}", serde_json::to_string(&output)?);
                }
            }

            Ok(())
        }
        Command::Guard { .. } => {
            // Handled above before TuiService initialization
            unreachable!("Guard command should be handled before TuiService initialization")
        }
        Command::Setup {
            template,
            path,
            add_role,
            list_templates,
        } => {
            use onboarding::{
                SetupMode, SetupResult, apply_template, list_templates as get_templates,
                run_setup_wizard,
            };

            // List templates and exit if requested
            if list_templates {
                println!("Available templates:\n");
                for template in get_templates() {
                    let path_note = if template.requires_path {
                        " (requires --path)"
                    } else if template.default_path.is_some() {
                        &format!(" (default: {})", template.default_path.as_ref().unwrap())
                    } else {
                        ""
                    };
                    println!("  {} - {}{}", template.id, template.description, path_note);
                }
                println!("\nUse --template <id> to apply a template directly.");
                return Ok(());
            }

            // Apply template directly if specified
            if let Some(template_id) = template {
                println!("Applying template: {}", template_id);
                match apply_template(&template_id, path.as_deref()) {
                    Ok(role) => {
                        // Save the role to config
                        if add_role {
                            service.add_role(role.clone()).await?;
                            println!("Role '{}' added to configuration.", role.name);
                        } else {
                            service.set_role(role.clone()).await?;
                            println!("Configuration set to role '{}'.", role.name);
                        }
                        return Ok(());
                    }
                    Err(e) => {
                        eprintln!("Failed to apply template: {}", e);
                        std::process::exit(1);
                    }
                }
            }

            // Run interactive wizard
            let mode = if add_role {
                SetupMode::AddRole
            } else {
                SetupMode::FirstRun
            };

            match run_setup_wizard(mode).await {
                Ok(SetupResult::Template {
                    template,
                    custom_path: _,
                    role,
                }) => {
                    if add_role {
                        service.add_role(role.clone()).await?;
                        println!(
                            "\nRole '{}' added from template '{}'.",
                            role.name, template.id
                        );
                    } else {
                        service.set_role(role.clone()).await?;
                        println!(
                            "\nConfiguration set to role '{}' from template '{}'.",
                            role.name, template.id
                        );
                    }
                }
                Ok(SetupResult::Custom { role }) => {
                    if add_role {
                        service.add_role(role.clone()).await?;
                        println!("\nCustom role '{}' added to configuration.", role.name);
                    } else {
                        service.set_role(role.clone()).await?;
                        println!("\nConfiguration set to custom role '{}'.", role.name);
                    }
                }
                Ok(SetupResult::Cancelled) => {
                    println!("\nSetup cancelled.");
                }
                Err(onboarding::OnboardingError::NotATty) => {
                    eprintln!(
                        "Interactive mode requires a terminal. Use --template for non-interactive setup."
                    );
                    std::process::exit(1);
                }
                Err(e) => {
                    eprintln!("Setup failed: {}", e);
                    std::process::exit(1);
                }
            }

            Ok(())
        }
        Command::CheckUpdate => {
            println!("Checking for terraphim-agent updates...");
            match check_for_updates("terraphim-agent").await {
                Ok(status) => {
                    println!("{}", status);
                    Ok(())
                }
                Err(e) => {
                    eprintln!("Failed to check for updates: {}", e);
                    std::process::exit(1);
                }
            }
        }
        Command::Update => {
            println!("Updating terraphim-agent...");
            match update_binary("terraphim-agent").await {
                Ok(status) => {
                    println!("{}", status);
                    Ok(())
                }
                Err(e) => {
                    eprintln!("Update failed: {}", e);
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
                    let selected = cfg.config.selected_role.to_string();
                    for (name, role) in cfg.config.roles.iter() {
                        let marker = if name.to_string() == selected {
                            "*"
                        } else {
                            " "
                        };
                        if let Some(ref short) = role.shortname {
                            println!("{} {} ({})", marker, name, short);
                        } else {
                            println!("{} {}", marker, name);
                        }
                    }
                }
                RolesSub::Select { name } => {
                    // Try to find role by name or shortname
                    let cfg = api.get_config().await?;
                    let query_lower = name.to_lowercase();
                    let role_name = cfg
                        .config
                        .roles
                        .iter()
                        .find(|(n, _)| n.to_string().to_lowercase() == query_lower)
                        .or_else(|| {
                            cfg.config.roles.iter().find(|(_, role)| {
                                role.shortname
                                    .as_ref()
                                    .map(|s| s.to_lowercase() == query_lower)
                                    .unwrap_or(false)
                            })
                        })
                        .map(|(n, _)| n.to_string())
                        .ok_or_else(|| {
                            anyhow::anyhow!(
                                "Role '{}' not found (checked name and shortname)",
                                name
                            )
                        })?;
                    let _ = api.update_selected_role(&role_name).await?;
                    println!("selected:{}", role_name);
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
            println!("🔍 Checking for terraphim-agent updates...");
            match check_for_updates("terraphim-agent").await {
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
            println!("🚀 Updating terraphim-agent...");
            match update_binary("terraphim-agent").await {
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
        Command::Replace {
            text,
            role: _,
            format: _,
            boundary: _,
            json,
            fail_open,
        } => {
            let input_text = match text {
                Some(t) => t,
                None => {
                    use std::io::Read;
                    let mut buffer = String::new();
                    std::io::stdin().read_to_string(&mut buffer)?;
                    buffer
                }
            };

            if fail_open {
                let hook_result = terraphim_hooks::HookResult::fail_open(
                    input_text.clone(),
                    "Replace command requires offline mode for full functionality".to_string(),
                );
                if json {
                    println!("{}", serde_json::to_string(&hook_result)?);
                } else {
                    eprintln!("Warning: {}", hook_result.error.as_deref().unwrap_or(""));
                    print!("{}", input_text);
                }
                Ok(())
            } else {
                eprintln!("Replace command is only available in offline mode");
                std::process::exit(1);
            }
        }
        Command::Validate { json, .. } => {
            if json {
                let err = serde_json::json!({
                    "error": "Validate command is only available in offline mode"
                });
                println!("{}", serde_json::to_string(&err)?);
            } else {
                eprintln!("Validate command is only available in offline mode");
            }
            std::process::exit(1);
        }
        Command::Suggest { json, .. } => {
            if json {
                let err = serde_json::json!({
                    "error": "Suggest command is only available in offline mode"
                });
                println!("{}", serde_json::to_string(&err)?);
            } else {
                eprintln!("Suggest command is only available in offline mode");
            }
            std::process::exit(1);
        }
        Command::Hook { .. } => {
            let err = serde_json::json!({
                "error": "Hook command is only available in offline mode"
            });
            println!("{}", serde_json::to_string(&err)?);
            std::process::exit(1);
        }
        Command::Guard {
            command,
            json,
            fail_open,
        } => {
            // Guard works the same in server mode - no server needed for pattern matching
            let input_command = match command {
                Some(c) => c,
                None => {
                    use std::io::Read;
                    let mut buffer = String::new();
                    std::io::stdin().read_to_string(&mut buffer)?;
                    buffer.trim().to_string()
                }
            };

            let guard = guard_patterns::CommandGuard::new();
            let result = guard.check(&input_command);

            if json {
                println!("{}", serde_json::to_string(&result)?);
            } else if result.decision == "block" {
                if let Some(reason) = &result.reason {
                    eprintln!("BLOCKED: {}", reason);
                    if !fail_open {
                        std::process::exit(1);
                    }
                }
            }

            Ok(())
        }
        Command::Setup {
            template,
            path,
            add_role,
            list_templates,
        } => {
            // Setup command - can run in server mode to add roles to running config
            if list_templates {
                println!("Available templates:");
                for t in onboarding::list_templates() {
                    let path_info = if t.requires_path {
                        " (requires --path)"
                    } else if t.default_path.is_some() {
                        " (optional --path)"
                    } else {
                        ""
                    };
                    println!("  {} - {}{}", t.id, t.description, path_info);
                }
                return Ok(());
            }

            if let Some(template_id) = template {
                // Apply template directly
                let role = onboarding::apply_template(&template_id, path.as_deref())
                    .map_err(|e| anyhow::anyhow!("{}", e))?;

                println!("Configured role: {}", role.name);
                println!("To add this role to a running server, restart with the new config.");

                // In server mode, we could potentially add the role via API
                // For now, just show what was configured
                if !role.haystacks.is_empty() {
                    println!("Haystacks:");
                    for h in &role.haystacks {
                        println!("  - {} ({:?})", h.location, h.service);
                    }
                }
                if role.kg.is_some() {
                    println!("Knowledge graph: configured");
                }
                if role.llm_enabled {
                    println!("LLM: enabled");
                }
            } else {
                // Interactive wizard
                let mode = if add_role {
                    onboarding::SetupMode::AddRole
                } else {
                    onboarding::SetupMode::FirstRun
                };

                match onboarding::run_setup_wizard(mode).await {
                    Ok(onboarding::SetupResult::Template {
                        template,
                        role,
                        custom_path,
                    }) => {
                        println!("\nApplied template: {}", template.name);
                        if let Some(ref path) = custom_path {
                            println!("Custom path: {}", path);
                        }
                        println!("Role '{}' configured successfully.", role.name);
                    }
                    Ok(onboarding::SetupResult::Custom { role }) => {
                        println!("\nCustom role '{}' configured successfully.", role.name);
                    }
                    Ok(onboarding::SetupResult::Cancelled) => {
                        println!("\nSetup cancelled.");
                    }
                    Err(e) => {
                        eprintln!("Setup error: {}", e);
                        std::process::exit(1);
                    }
                }
            }
            Ok(())
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

fn run_tui(server_url: Option<String>, transparent: bool) -> Result<()> {
    // Attempt to set up terminal for TUI
    let stdout = io::stdout();
    let backend = CrosstermBackend::new(stdout);

    // Try to enter raw mode and alternate screen
    // These operations can fail in non-interactive environments
    match enable_raw_mode() {
        Ok(()) => {
            // Successfully entered raw mode, proceed with TUI setup
            let mut stdout = io::stdout();
            if let Err(e) = execute!(stdout, EnterAlternateScreen, EnableMouseCapture) {
                // Clean up raw mode before returning error
                let _ = disable_raw_mode();
                return Err(anyhow::anyhow!(
                    "Failed to initialize terminal for interactive mode: {}. \
                     Try using 'repl' mode instead: terraphim-agent repl",
                    e
                ));
            }

            let mut terminal = match Terminal::new(backend) {
                Ok(t) => t,
                Err(e) => {
                    // Clean up before returning
                    let _ = execute!(io::stdout(), LeaveAlternateScreen, DisableMouseCapture);
                    let _ = disable_raw_mode();
                    return Err(anyhow::anyhow!(
                        "Failed to create terminal: {}. \
                         Try using 'repl' mode instead: terraphim-agent repl",
                        e
                    ));
                }
            };

            let res = ui_loop(&mut terminal, server_url, transparent);

            // Always clean up terminal state
            let _ = disable_raw_mode();
            let _ = execute!(
                terminal.backend_mut(),
                LeaveAlternateScreen,
                DisableMouseCapture
            );
            let _ = terminal.show_cursor();

            res
        }
        Err(e) => {
            // Failed to enter raw mode - not a TTY
            Err(anyhow::anyhow!(
                "Terminal does not support raw mode (not a TTY?). \
                 Interactive mode requires a terminal. \
                 Try using 'repl' mode instead: terraphim-agent repl. \
                 Error: {}",
                e
            ))
        }
    }
}

fn ui_loop(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    server_url: Option<String>,
    transparent: bool,
) -> Result<()> {
    let mut input = String::new();
    let mut results: Vec<String> = Vec::new();
    let mut detailed_results: Vec<Document> = Vec::new();
    let mut terms: Vec<String> = Vec::new();
    let mut suggestions: Vec<String> = Vec::new();
    let mut current_role = String::from("Terraphim Engineer"); // Default to Terraphim Engineer
    let mut selected_result_index = 0;
    let mut view_mode = ViewMode::Search;
    // Use provided server_url, then TERRAPHIM_SERVER env var, then default to localhost
    let effective_url = server_url.unwrap_or_else(|| {
        std::env::var("TERRAPHIM_SERVER").unwrap_or_else(|_| "http://localhost:8000".to_string())
    });
    let api = ApiClient::new(effective_url);

    // Create a tokio runtime for this TUI session
    // We need a local runtime because we're in a synchronous function (terminal event loop)
    let rt = tokio::runtime::Runtime::new()?;

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

                    let input_title = format!(
                        "Search [Role: {}] • Enter: search, Tab: autocomplete, Ctrl+r: switch role, q: quit",
                        current_role
                    );
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
                    let list = List::new(items).block(create_block(
                        "Results • ↑↓: select, Enter: view details, Ctrl+s: summarize",
                        transparent,
                    ));
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
                            .block(create_block(
                                "Content • Ctrl+s: summarize, Esc: back to search",
                                transparent,
                            ))
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
                        match map_search_key_event(key) {
                            TuiAction::Quit => break,
                            TuiAction::SearchOrOpen => {
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
                            TuiAction::MoveUp => {
                                selected_result_index = selected_result_index.saturating_sub(1);
                            }
                            TuiAction::MoveDown => {
                                if selected_result_index + 1 < results.len() {
                                    selected_result_index += 1;
                                }
                            }
                            TuiAction::Autocomplete => {
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
                            TuiAction::SwitchRole => {
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
                            TuiAction::SummarizeSelection => {
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
                            TuiAction::Backspace => {
                                input.pop();
                                update_local_suggestions(&input, &terms, &mut suggestions);
                            }
                            TuiAction::InsertChar(c) => {
                                input.push(c);
                                update_local_suggestions(&input, &terms, &mut suggestions);
                            }
                            TuiAction::None
                            | TuiAction::BackToSearch
                            | TuiAction::SummarizeDetail => {}
                        }
                    }
                    ViewMode::ResultDetail => {
                        match map_detail_key_event(key) {
                            TuiAction::BackToSearch => {
                                view_mode = ViewMode::Search;
                            }
                            TuiAction::SummarizeDetail => {
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
                            TuiAction::Quit => break,
                            TuiAction::None
                            | TuiAction::SearchOrOpen
                            | TuiAction::MoveUp
                            | TuiAction::MoveDown
                            | TuiAction::Autocomplete
                            | TuiAction::SwitchRole
                            | TuiAction::SummarizeSelection
                            | TuiAction::Backspace
                            | TuiAction::InsertChar(_) => {}
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
