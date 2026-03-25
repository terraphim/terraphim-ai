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
use serde::Serialize;
use terraphim_persistence::Persistable;
use tokio::runtime::Runtime;

mod client;
mod guard_patterns;
mod onboarding;
mod service;

// Robot mode and forgiving CLI - always available
mod forgiving;
mod robot;

// Learning capture for failed commands
mod learnings;

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
    println!("Interactive Modes (requires TTY):");
    println!("  terraphim-agent              # Start fullscreen TUI (requires running server)");
    println!("  terraphim-agent repl         # Start REPL (offline-capable by default)");
    println!("  terraphim-agent repl --server # Start REPL in server mode");
    println!();
    println!("Common Commands:");
    println!("  search <query>               # Search documents (offline-capable by default)");
    println!("  roles list                   # List available roles");
    println!("  config show                  # Show configuration");
    println!("  replace <text>               # Replace terms using thesaurus");
    println!("  validate <text>              # Validate against knowledge graph");
    println!();
    println!("For more information:");
    println!("  terraphim-agent --help       # Show full help");
    println!("  terraphim-agent help         # Show command-specific help");
}

fn resolve_tui_server_url(explicit: Option<&str>) -> String {
    let env_server = std::env::var("TERRAPHIM_SERVER").ok();
    resolve_tui_server_url_with_env(explicit, env_server.as_deref())
}

fn resolve_tui_server_url_with_env(explicit: Option<&str>, env_server: Option<&str>) -> String {
    explicit
        .map(ToOwned::to_owned)
        .or_else(|| env_server.map(ToOwned::to_owned))
        .unwrap_or_else(|| "http://localhost:8000".to_string())
}

fn tui_server_requirement_error(url: &str, cause: &anyhow::Error) -> anyhow::Error {
    anyhow::anyhow!(
        "Fullscreen TUI requires a running Terraphim server at {}. \
         Start terraphim_server or use offline mode with `terraphim-agent repl`. \
         Connection error: {}",
        url,
        cause
    )
}

fn ensure_tui_server_reachable(
    runtime: &tokio::runtime::Runtime,
    api: &ApiClient,
    url: &str,
) -> Result<()> {
    runtime
        .block_on(api.health())
        .map_err(|err| tui_server_requirement_error(url, &err))
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

    #[test]
    fn resolve_tui_server_url_uses_explicit_then_env_then_default() {
        let explicit = resolve_tui_server_url_with_env(Some("http://explicit:9000"), None);
        assert_eq!(explicit, "http://explicit:9000");

        let from_env = resolve_tui_server_url_with_env(None, Some("http://env:7000"));
        assert_eq!(from_env, "http://env:7000");

        let defaulted = resolve_tui_server_url_with_env(None, None);
        assert_eq!(defaulted, "http://localhost:8000");
    }

    #[test]
    fn tui_server_requirement_error_mentions_repl_fallback() {
        let cause = anyhow::anyhow!("connect error");
        let err = tui_server_requirement_error("http://localhost:8000", &cause);
        let msg = err.to_string();
        assert!(msg.contains("Fullscreen TUI requires a running Terraphim server"));
        assert!(msg.contains("terraphim-agent repl"));
        assert!(msg.contains("http://localhost:8000"));
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CommandOutputMode {
    Human,
    Json,
    JsonCompact,
}

#[derive(Debug, Clone, Copy)]
struct CommandOutputConfig {
    mode: CommandOutputMode,
    robot: bool,
}

impl CommandOutputConfig {
    fn is_machine_readable(self) -> bool {
        self.robot || !matches!(self.mode, CommandOutputMode::Human)
    }
}

fn resolve_output_config(robot: bool, format: OutputFormat) -> CommandOutputConfig {
    let mode = match format {
        OutputFormat::Human => {
            if robot {
                CommandOutputMode::Json
            } else {
                CommandOutputMode::Human
            }
        }
        OutputFormat::Json => CommandOutputMode::Json,
        OutputFormat::JsonCompact => CommandOutputMode::JsonCompact,
    };
    CommandOutputConfig { mode, robot }
}

#[derive(Debug, Serialize)]
struct SearchDocumentOutput {
    id: String,
    title: String,
    url: String,
    rank: Option<u64>,
}

#[derive(Debug, Serialize)]
struct SearchOutput {
    query: String,
    role: String,
    count: usize,
    results: Vec<SearchDocumentOutput>,
}

fn print_json_output<T: Serialize>(value: &T, mode: CommandOutputMode) -> Result<()> {
    let out = match mode {
        CommandOutputMode::Human => serde_json::to_string_pretty(value)?,
        CommandOutputMode::Json => serde_json::to_string_pretty(value)?,
        CommandOutputMode::JsonCompact => serde_json::to_string(value)?,
    };
    println!("{}", out);
    Ok(())
}

#[derive(Parser, Debug)]
#[command(
    name = "terraphim-agent",
    version,
    about = "Terraphim Agent: server-backed fullscreen TUI with offline-capable REPL and CLI commands"
)]
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
    /// Path to a JSON config file (overrides settings.toml and persistence)
    #[arg(long)]
    config: Option<String>,
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand, Debug)]
enum Command {
    /// Search documents using the knowledge graph
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
    /// Manage roles (list, select)
    Roles {
        #[command(subcommand)]
        sub: RolesSub,
    },
    /// Manage configuration (show, set, validate, reload)
    Config {
        #[command(subcommand)]
        sub: ConfigSub,
    },
    /// Display the knowledge graph for a role
    Graph {
        #[arg(long)]
        role: Option<String>,
        #[arg(long, default_value_t = 50)]
        top_k: usize,
    },
    /// Chat with the AI using a specific role
    Chat {
        #[arg(long)]
        role: Option<String>,
        prompt: String,
        #[arg(long)]
        model: Option<String>,
    },
    /// Extract paragraphs matching knowledge graph terms from text
    Extract {
        text: String,
        #[arg(long)]
        role: Option<String>,
        #[arg(long, default_value_t = false)]
        exclude_term: bool,
    },
    /// Replace terms in text using the knowledge graph thesaurus
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
        /// Path to custom destructive patterns thesaurus JSON file
        #[arg(long)]
        guard_thesaurus: Option<String>,
        /// Path to custom allowlist thesaurus JSON file
        #[arg(long)]
        guard_allowlist: Option<String>,
    },
    /// Start fullscreen interactive TUI mode (requires running server)
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

    /// Learning capture for failed commands
    Learn {
        #[command(subcommand)]
        sub: LearnSub,
    },

    /// Session management for AI coding assistant history
    #[cfg(feature = "repl-sessions")]
    Sessions {
        #[command(subcommand)]
        sub: SessionsSub,
    },
}

#[derive(Subcommand, Debug)]
enum LearnSub {
    /// Capture a failed command as a learning
    Capture {
        /// The command that failed
        command: String,
        /// The error output (stderr)
        #[arg(long)]
        error: String,
        /// The exit code
        #[arg(long, default_value_t = 1)]
        exit_code: i32,
        /// Enable debug output
        #[arg(long, default_value_t = false)]
        debug: bool,
    },
    /// List recent learnings
    List {
        /// Number of recent learnings to show
        #[arg(long, default_value_t = 10)]
        recent: usize,
        /// Show global learnings instead of project
        #[arg(long, default_value_t = false)]
        global: bool,
    },
    /// Query learnings by pattern
    Query {
        /// Search pattern
        pattern: String,
        /// Use exact match instead of substring
        #[arg(long, default_value_t = false)]
        exact: bool,
        /// Show global learnings instead of project
        #[arg(long, default_value_t = false)]
        global: bool,
    },
    /// Add correction to an existing learning
    Correct {
        /// Learning ID
        id: String,
        /// The correction to add
        #[arg(long)]
        correction: String,
    },
    /// Record a user correction (tool preference, naming, workflow, etc.)
    Correction {
        /// What the agent said/did originally
        #[arg(long)]
        original: String,
        /// What the user said instead
        #[arg(long)]
        corrected: String,
        /// Type of correction
        #[arg(long, default_value = "other")]
        correction_type: String,
        /// Context description
        #[arg(long, default_value = "")]
        context: String,
        /// Session ID for traceability
        #[arg(long)]
        session_id: Option<String>,
    },
    /// Process hook input from AI agents (reads JSON from stdin)
    Hook {
        /// AI agent format
        #[arg(long, value_enum, default_value = "claude")]
        format: learnings::AgentFormat,
    },
    /// Install hook for AI agent
    InstallHook {
        /// AI agent to install hook for
        #[arg(value_enum)]
        agent: learnings::AgentType,
    },
}

#[derive(Subcommand, Debug)]
enum RolesSub {
    List,
    Select { name: String },
}

#[derive(Subcommand, Debug)]
enum ConfigSub {
    /// Show current configuration as JSON
    Show,
    /// Set a configuration value
    Set { key: String, value: String },
    /// Validate configuration loading (shows what would be loaded and from where)
    Validate,
    /// Reload roles from JSON file specified in settings.toml role_config
    Reload,
}

/// Get the session cache file path
#[cfg(feature = "repl-sessions")]
fn get_session_cache_path() -> std::path::PathBuf {
    let cache_dir = dirs::cache_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join("terraphim-agent");
    std::fs::create_dir_all(&cache_dir).ok();
    cache_dir.join("sessions.json")
}

#[cfg(feature = "repl-sessions")]
#[derive(Subcommand, Debug)]
enum SessionsSub {
    /// Detect available session sources (Claude Code, Cursor, etc.)
    Sources,
    /// List all cached sessions (auto-imports if cache is empty)
    List {
        /// Limit number of sessions to show
        #[arg(long, default_value_t = 20)]
        limit: usize,
    },
    /// Search sessions by query string (auto-imports if cache is empty)
    Search {
        /// Search query
        query: String,
        /// Limit number of results
        #[arg(long, default_value_t = 10)]
        limit: usize,
    },
    /// Show session statistics (auto-imports if cache is empty)
    Stats,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let output = resolve_output_config(cli.robot, cli.format.clone());

    // Check for updates on startup (non-blocking, logs warning on failure)
    let rt = Runtime::new()?;
    rt.block_on(async {
        if let Err(e) = check_for_updates_startup("terraphim-agent").await {
            eprintln!("Update check failed: {}", e);
        }
    });

    match cli.command {
        Some(Command::Interactive) | None => {
            // Check if we're in a TTY for interactive mode (both stdout and stdin required)
            use std::io::IsTerminal;
            if !std::io::stdout().is_terminal() {
                show_usage_info();
                std::process::exit(0);
            }

            if !std::io::stdin().is_terminal() {
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
                rt.block_on(run_server_command(command, &cli.server_url, output))
            } else {
                rt.block_on(run_offline_command(command, output, cli.config))
            }
        }
    }
}
fn run_tui_offline_mode(transparent: bool) -> Result<()> {
    // Fullscreen TUI mode requires a running server.
    // For offline operation, use `terraphim-agent repl`.
    run_tui(None, transparent)
}

fn run_tui_server_mode(server_url: &str, transparent: bool) -> Result<()> {
    run_tui(Some(server_url.to_string()), transparent)
}

/// Stateless config validation -- runs before TuiService initialization.
/// Shows config sources, paths, and what would be loaded.
async fn run_config_validate() -> Result<()> {
    use terraphim_settings::DeviceSettings;

    println!("== Device Settings ==");
    let ds = match DeviceSettings::load_from_env_and_file(None) {
        Ok(s) => {
            let config_path = DeviceSettings::default_config_path();
            println!("  settings.toml: {}/settings.toml", config_path.display());
            println!("  server_hostname: {}", s.server_hostname);
            println!("  api_endpoint: {}", s.api_endpoint);
            println!("  default_data_path: {}", s.default_data_path);
            println!("  profiles: {:?}", s.profiles.keys().collect::<Vec<_>>());
            s
        }
        Err(e) => {
            println!("  FAILED to load: {:?}", e);
            println!("  Would use embedded defaults");
            DeviceSettings::default_embedded()
        }
    };

    println!();
    println!("== Role Configuration ==");
    match &ds.role_config {
        Some(path) => {
            let expanded = terraphim_config::expand_path(path);
            println!("  role_config: {} (expanded: {})", path, expanded.display());
            if expanded.exists() {
                match terraphim_config::Config::load_from_json_file(path) {
                    Ok(config) => {
                        println!("  Status: OK - loaded {} role(s)", config.roles.len());
                        for (name, role) in &config.roles {
                            println!("    - {} (shortname: {:?})", name, role.shortname);
                        }
                        println!("  default_role in file: {}", config.default_role);
                        println!("  selected_role in file: {}", config.selected_role);
                    }
                    Err(e) => {
                        println!("  Status: PARSE ERROR - {:?}", e);
                    }
                }
            } else {
                println!("  Status: FILE NOT FOUND at {}", expanded.display());
            }
        }
        None => {
            println!("  role_config: not set (using persistence/embedded defaults)");
        }
    }

    if let Some(ref role) = ds.default_role {
        println!("  default_role override: {}", role);
    }

    println!();
    println!("== Persistence ==");
    match terraphim_config::ConfigBuilder::new_with_id(terraphim_config::ConfigId::Embedded).build()
    {
        Ok(mut config) => match config.load().await {
            Ok(persisted) => {
                println!(
                    "  Persisted config found with {} role(s):",
                    persisted.roles.len()
                );
                for (name, role) in &persisted.roles {
                    println!("    - {} (shortname: {:?})", name, role.shortname);
                }
                println!("  selected_role: {}", persisted.selected_role);
            }
            Err(_) => {
                println!("  No persisted config found (first run or empty)");
            }
        },
        Err(e) => {
            println!("  Failed to check persistence: {:?}", e);
        }
    }

    println!();
    println!("== Summary ==");
    if ds.role_config.is_some() {
        println!("  Config source: role_config in settings.toml (bootstrap-then-persistence)");
    } else {
        println!("  Config source: persistence layer or embedded defaults");
    }

    Ok(())
}

async fn run_offline_command(
    command: Command,
    output: CommandOutputConfig,
    config_path: Option<String>,
) -> Result<()> {
    // Handle stateless commands that don't need TuiService first
    if let Command::Guard {
        command,
        json,
        fail_open,
        guard_thesaurus,
        guard_allowlist,
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

        let guard = match (guard_thesaurus, guard_allowlist) {
            (Some(thesaurus_path), Some(allowlist_path)) => {
                let destructive_json = std::fs::read_to_string(thesaurus_path)?;
                let allowlist_json = std::fs::read_to_string(allowlist_path)?;
                guard_patterns::CommandGuard::from_json(&destructive_json, &allowlist_json, None)
                    .map_err(|e| {
                        anyhow::anyhow!("Failed to load custom guard thesauruses: {}", e)
                    })?
            }
            (Some(thesaurus_path), None) => {
                let destructive_json = std::fs::read_to_string(thesaurus_path)?;
                guard_patterns::CommandGuard::from_json(
                    &destructive_json,
                    guard_patterns::CommandGuard::default_allowlist_json(),
                    None,
                )
                .map_err(|e| anyhow::anyhow!("Failed to load custom guard thesaurus: {}", e))?
            }
            (None, Some(allowlist_path)) => {
                let allowlist_json = std::fs::read_to_string(allowlist_path)?;
                guard_patterns::CommandGuard::from_json(
                    guard_patterns::CommandGuard::default_destructive_json(),
                    &allowlist_json,
                    None,
                )
                .map_err(|e| anyhow::anyhow!("Failed to load custom guard allowlist: {}", e))?
            }
            (None, None) => guard_patterns::CommandGuard::new(),
        };
        let result = guard.check(&input_command);

        if *json {
            println!("{}", serde_json::to_string(&result)?);
        } else if result.decision == guard_patterns::GuardDecision::Block {
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

    // CheckUpdate is stateless - handle before TuiService initialization
    if let Command::CheckUpdate = &command {
        println!("Checking for terraphim-agent updates...");
        match check_for_updates("terraphim-agent").await {
            Ok(status) => {
                println!("{}", status);
                return Ok(());
            }
            Err(e) => {
                eprintln!("Failed to check for updates: {}", e);
                std::process::exit(1);
            }
        }
    }

    // Update is stateless - handle before TuiService initialization
    if let Command::Update = &command {
        println!("Updating terraphim-agent...");
        match update_binary("terraphim-agent").await {
            Ok(status) => {
                println!("{}", status);
                return Ok(());
            }
            Err(e) => {
                eprintln!("Update failed: {}", e);
                std::process::exit(1);
            }
        }
    }

    // Config validate is stateless - handle before TuiService initialization
    if let Command::Config {
        sub: ConfigSub::Validate,
    } = &command
    {
        return run_config_validate().await;
    }

    // Learn is stateless - handle before TuiService initialization.
    // Must be last early-return because it consumes `command` via destructuring.
    if let Command::Learn { sub } = command {
        return run_learn_command(sub).await;
    }

    let service = TuiService::new(config_path).await?;

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
                let mut all_terms = vec![query.clone()];
                all_terms.extend(additional_terms);

                let op_str = match operator {
                    Some(LogicalOperatorCli::And) => "AND",
                    Some(LogicalOperatorCli::Or) | None => "OR", // Default to OR
                };
                if !output.is_machine_readable() {
                    println!(
                        "Multi-term search: {} terms using {} operator",
                        all_terms.len(),
                        op_str
                    );
                }

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

            if output.is_machine_readable() {
                let payload = SearchOutput {
                    query,
                    role: role_name.to_string(),
                    count: results.len(),
                    results: results
                        .iter()
                        .map(|doc| SearchDocumentOutput {
                            id: doc.id.clone(),
                            title: doc.title.clone(),
                            url: doc.url.clone(),
                            rank: doc.rank,
                        })
                        .collect(),
                };
                print_json_output(&payload, output.mode)?;
            } else {
                for doc in results.iter() {
                    println!("- {}\t{}", doc.rank.unwrap_or_default(), doc.title);
                }
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
                ConfigSub::Validate => {
                    // Handled as early-return above; should not reach here
                    unreachable!("config validate is handled before TuiService init");
                }
                ConfigSub::Reload => {
                    let ds = terraphim_settings::DeviceSettings::load_from_env_and_file(None)
                        .unwrap_or_else(|_| terraphim_settings::DeviceSettings::default_embedded());
                    match &ds.role_config {
                        Some(path) => match service.reload_from_json(path).await {
                            Ok(count) => {
                                println!(
                                    "Reloaded {} role(s) from '{}' and saved to persistence",
                                    count, path
                                );
                            }
                            Err(e) => {
                                eprintln!("Failed to reload from '{}': {:?}", path, e);
                                std::process::exit(1);
                            }
                        },
                        None => {
                            eprintln!("No role_config set in settings.toml. Nothing to reload.");
                            eprintln!(
                                "Add role_config = \"path/to/roles.json\" to your settings.toml"
                            );
                            std::process::exit(1);
                        }
                    }
                }
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

                                if guard_result.decision == guard_patterns::GuardDecision::Block {
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
            unreachable!("CheckUpdate command should be handled before TuiService initialization")
        }
        Command::Update => {
            unreachable!("Update command should be handled before TuiService initialization")
        }
        Command::Learn { .. } => {
            unreachable!("Learn command should be handled before TuiService initialization")
        }

        #[cfg(feature = "repl-sessions")]
        Command::Sessions { sub } => {
            use terraphim_sessions::SessionService;

            let service = SessionService::new();

            // Load cached sessions from disk
            let cache_path = get_session_cache_path();
            if cache_path.exists() {
                if let Ok(data) = std::fs::read_to_string(&cache_path) {
                    if let Ok(cached) =
                        serde_json::from_str::<Vec<terraphim_sessions::Session>>(&data)
                    {
                        service.load_sessions(cached).await;
                        println!("Loaded sessions from cache.");
                    }
                }
            }

            match sub {
                SessionsSub::Sources => {
                    let sources = service.detect_sources();
                    if sources.is_empty() {
                        println!("No session sources detected.");
                    } else {
                        println!("Available session sources:");
                        for source in sources {
                            let status = if source.is_available() {
                                "available"
                            } else {
                                "not found"
                            };
                            println!(
                                "  - {} ({})",
                                source.name.unwrap_or_else(|| source.id.clone()),
                                status
                            );
                        }
                    }
                    Ok(())
                }
                SessionsSub::List { limit } => {
                    let sessions = service.list_sessions().await;
                    if sessions.is_empty() {
                        println!("No sessions found.");
                    } else {
                        println!("Cached sessions ({} total):", sessions.len());
                        for session in sessions.iter().take(limit) {
                            let msg_count = session.message_count();
                            let title = session.title.as_deref().unwrap_or("(untitled)");
                            println!("  - {} ({} messages)", title, msg_count);
                        }
                        if sessions.len() > limit {
                            println!("  ... and {} more", sessions.len() - limit);
                        }
                    }
                    Ok(())
                }
                SessionsSub::Search { query, limit } => {
                    let results = service.search(&query).await;
                    if results.is_empty() {
                        println!("No sessions matching '{}'.", query);
                    } else {
                        println!("Found {} matching sessions:", results.len());
                        for session in results.iter().take(limit) {
                            let title = session.title.as_deref().unwrap_or("(untitled)");
                            println!("  - {}", title);
                            // Show preview of matching content
                            for msg in &session.messages {
                                let content_lower = msg.content.to_lowercase();
                                if content_lower.contains(&query.to_lowercase()) {
                                    let preview: String = msg.content.chars().take(100).collect();
                                    println!("    > {}", preview);
                                    break;
                                }
                            }
                        }
                    }
                    Ok(())
                }
                SessionsSub::Stats => {
                    let stats = service.statistics().await;
                    println!("Session Statistics:");
                    println!("  Total sessions: {}", stats.total_sessions);
                    println!("  Total messages: {}", stats.total_messages);
                    println!("  User messages: {}", stats.total_user_messages);
                    println!("  Assistant messages: {}", stats.total_assistant_messages);
                    if !stats.sessions_by_source.is_empty() {
                        println!("  By source:");
                        for (source, count) in stats.sessions_by_source {
                            println!("    - {}: {}", source, count);
                        }
                    }
                    Ok(())
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

async fn run_learn_command(sub: LearnSub) -> Result<()> {
    use learnings::{
        LearningCaptureConfig, CorrectionType, capture_correction, capture_failed_command, correct_learning, list_all_entries, list_learnings,
        query_all_entries, query_learnings,
    };
    let config = LearningCaptureConfig::default();

    match sub {
        LearnSub::Capture {
            command,
            error,
            exit_code,
            debug,
        } => {
            if debug {
                eprintln!(
                    "Capturing learning: command='{}', exit_code={}",
                    command, exit_code
                );
            }
            match capture_failed_command(&command, &error, exit_code, &config) {
                Ok(path) => {
                    println!("Captured learning: {}", path.display());
                    Ok(())
                }
                Err(e) => {
                    if debug {
                        eprintln!("Failed to capture learning: {}", e);
                    }
                    Err(e.into())
                }
            }
        }
        LearnSub::List { recent, global } => {
            let storage_loc = config.storage_location();
            let storage_dir = if global {
                &config.global_dir
            } else {
                &storage_loc
            };
            match list_all_entries(storage_dir, recent) {
                Ok(entries) => {
                    if entries.is_empty() {
                        println!("No learnings found.");
                    } else {
                        println!("Recent learnings:");
                        for (i, entry) in entries.iter().enumerate() {
                            let source_indicator = match entry.source() {
                                learnings::LearningSource::Project => "[P]",
                                learnings::LearningSource::Global => "[G]",
                            };
                            println!("  {}. {} {}", i + 1, source_indicator, entry.summary());
                            if let Some(correction) = entry.correction_text() {
                                println!("     Correction: {}", correction);
                            }
                        }
                    }
                    Ok(())
                }
                Err(e) => Err(e.into()),
            }
        }
        LearnSub::Query {
            pattern,
            exact,
            global,
        } => {
            let storage_loc = config.storage_location();
            let storage_dir = if global {
                &config.global_dir
            } else {
                &storage_loc
            };
            match query_all_entries(storage_dir, &pattern, exact) {
                Ok(entries) => {
                    if entries.is_empty() {
                        println!("No learnings matching '{}'.", pattern);
                    } else {
                        println!("Learnings matching '{}'.", pattern);
                        for entry in entries {
                            let source_indicator = match entry.source() {
                                learnings::LearningSource::Project => "[P]",
                                learnings::LearningSource::Global => "[G]",
                            };
                            println!("  {} {}", source_indicator, entry.summary());
                            if let Some(correction) = entry.correction_text() {
                                println!("     Correction: {}", correction);
                            }
                        }
                    }
                    Ok(())
                }
                Err(e) => Err(e.into()),
            }
        }
        LearnSub::Correct { id, correction } => {
            let storage_loc = config.storage_location();
            match correct_learning(&storage_loc, &id, &correction) {
                Ok(path) => {
                    println!("Correction added to learning {}: {}", id, path.display());
                    Ok(())
                }
                Err(e) => {
                    eprintln!("Failed to add correction: {}", e);
                    Err(e.into())
                }
            }
        }
        LearnSub::Correction {
            original,
            corrected,
            correction_type,
            context,
            session_id,
        } => {
            let ct: CorrectionType = correction_type
                .parse()
                .unwrap_or(CorrectionType::Other(correction_type.clone()));
            let mut correction = capture_correction(ct, &original, &corrected, &context, &config);
            if let Some(ref sid) = session_id {
                // We need to read the file and update it with session_id
                // For now, just print the session_id
                log::info!("Session ID: {}", sid);
            }
            match correction {
                Ok(path) => {
                    println!("Captured correction: {}", path.display());
                    Ok(())
                }
                Err(e) => {
                    eprintln!("Failed to capture correction: {}", e);
                    Err(e.into())
                }
            }
        }
        LearnSub::Hook { format } => learnings::process_hook_input(format)
            .await
            .map_err(|e| e.into()),
        LearnSub::InstallHook { agent } => {
            learnings::install_hook(agent).await.map_err(|e| e.into())
        }
    }
}

async fn run_server_command(
    command: Command,
    server_url: &str,
    output: CommandOutputConfig,
) -> Result<()> {
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
                if !output.is_machine_readable() {
                    println!(
                        "Multi-term search: '{}' {} {} additional terms using {} operator",
                        query,
                        op_str,
                        additional_terms.len(),
                        op_str
                    );
                }
            }

            if output.is_machine_readable() {
                let payload = SearchOutput {
                    query,
                    role: q
                        .role
                        .as_ref()
                        .map(|r| r.to_string())
                        .unwrap_or_else(|| "unknown".to_string()),
                    count: res.results.len(),
                    results: res
                        .results
                        .iter()
                        .map(|doc| SearchDocumentOutput {
                            id: doc.id.clone(),
                            title: doc.title.clone(),
                            url: doc.url.clone(),
                            rank: doc.rank,
                        })
                        .collect(),
                };
                print_json_output(&payload, output.mode)?;
            } else {
                for doc in res.results.iter() {
                    println!("- {}\t{}", doc.rank.unwrap_or_default(), doc.title);
                }
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
                ConfigSub::Validate => {
                    println!(
                        "config validate is only available in offline mode (without --server)"
                    );
                }
                ConfigSub::Reload => {
                    println!("config reload is only available in offline mode (without --server)");
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
            guard_thesaurus,
            guard_allowlist,
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

            let guard = match (guard_thesaurus, guard_allowlist) {
                (Some(thesaurus_path), Some(allowlist_path)) => {
                    let destructive_json = std::fs::read_to_string(thesaurus_path)?;
                    let allowlist_json = std::fs::read_to_string(allowlist_path)?;
                    guard_patterns::CommandGuard::from_json(
                        &destructive_json,
                        &allowlist_json,
                        None,
                    )
                    .map_err(|e| anyhow::anyhow!("{}", e))?
                }
                (Some(thesaurus_path), None) => {
                    let destructive_json = std::fs::read_to_string(thesaurus_path)?;
                    guard_patterns::CommandGuard::from_json(
                        &destructive_json,
                        guard_patterns::CommandGuard::default_allowlist_json(),
                        None,
                    )
                    .map_err(|e| anyhow::anyhow!("{}", e))?
                }
                (None, Some(allowlist_path)) => {
                    let allowlist_json = std::fs::read_to_string(allowlist_path)?;
                    guard_patterns::CommandGuard::from_json(
                        guard_patterns::CommandGuard::default_destructive_json(),
                        &allowlist_json,
                        None,
                    )
                    .map_err(|e| anyhow::anyhow!("{}", e))?
                }
                (None, None) => guard_patterns::CommandGuard::new(),
            };
            let result = guard.check(&input_command);

            if json {
                println!("{}", serde_json::to_string(&result)?);
            } else if result.decision == guard_patterns::GuardDecision::Block {
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
        Command::Learn { sub } => run_learn_command(sub).await,
        Command::Interactive => {
            unreachable!("Interactive mode should be handled above")
        }

        #[cfg(feature = "repl")]
        Command::Repl { .. } => {
            unreachable!("REPL mode should be handled above")
        }

        #[cfg(feature = "repl-sessions")]
        Command::Sessions { sub } => {
            use terraphim_sessions::SessionService;

            let rt = Runtime::new()?;
            rt.block_on(async {
                let service = SessionService::new();

                match sub {
                    SessionsSub::Sources => {
                        let sources = service.detect_sources();
                        if sources.is_empty() {
                            println!("No session sources detected.");
                        } else {
                            println!("Available session sources:");
                            for source in sources {
                                let status = if source.is_available() {
                                    "available"
                                } else {
                                    "not found"
                                };
                                println!(
                                    "  - {} ({})",
                                    source.name.unwrap_or_else(|| source.id.clone()),
                                    status
                                );
                            }
                        }
                        Ok(())
                    }

                    SessionsSub::List { limit } => {
                        let sessions = service.list_sessions().await;
                        if sessions.is_empty() {
                            println!("No sessions found.");
                        } else {
                            println!("Cached sessions ({} total):", sessions.len());
                            for session in sessions.iter().take(limit) {
                                let msg_count = session.message_count();
                                let title = session.title.as_deref().unwrap_or("(untitled)");
                                println!("  - {} ({} messages)", title, msg_count);
                            }
                            if sessions.len() > limit {
                                println!("  ... and {} more", sessions.len() - limit);
                            }
                        }
                        Ok(())
                    }
                    SessionsSub::Search { query, limit } => {
                        let results = service.search(&query).await;
                        if results.is_empty() {
                            println!("No sessions matching '{}'.", query);
                        } else {
                            println!("Found {} matching sessions:", results.len());
                            for session in results.iter().take(limit) {
                                let title = session.title.as_deref().unwrap_or("(untitled)");
                                println!("  - {}", title);
                                // Show preview of matching content
                                for msg in &session.messages {
                                    let content_lower = msg.content.to_lowercase();
                                    if content_lower.contains(&query.to_lowercase()) {
                                        let preview: String =
                                            msg.content.chars().take(100).collect();
                                        println!("    > {}", preview);
                                        break;
                                    }
                                }
                            }
                        }
                        Ok(())
                    }
                    SessionsSub::Stats => {
                        let stats = service.statistics().await;
                        println!("Session Statistics:");
                        println!("  Total sessions: {}", stats.total_sessions);
                        println!("  Total messages: {}", stats.total_messages);
                        println!("  User messages: {}", stats.total_user_messages);
                        println!("  Assistant messages: {}", stats.total_assistant_messages);
                        if !stats.sessions_by_source.is_empty() {
                            println!("  By source:");
                            for (source, count) in stats.sessions_by_source {
                                println!("    - {}: {}", source, count);
                            }
                        }
                        Ok(())
                    }
                }
            })
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
    let effective_url = resolve_tui_server_url(server_url.as_deref());
    let api = ApiClient::new(effective_url.clone());

    // Create a tokio runtime for this TUI session
    // We need a local runtime because we're in a synchronous function (terminal event loop)
    let rt = tokio::runtime::Runtime::new()?;
    ensure_tui_server_reachable(&rt, &api, &effective_url)?;

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
