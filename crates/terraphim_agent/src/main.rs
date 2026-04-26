use std::io;
use std::path::PathBuf;

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

#[cfg(feature = "server")]
mod client;

mod tui_backend;

mod guard_patterns;
mod listener;
mod onboarding;
mod service;
#[allow(dead_code)]
mod shell_dispatch;

// Robot mode and forgiving CLI - always available
mod forgiving;
mod robot;

// Learning capture for failed commands
mod learnings;

// KG-based command validation for PreToolUse hook pipeline
mod kg_validation;

#[cfg(feature = "repl")]
mod repl;

#[cfg(feature = "server")]
use client::{ApiClient, SearchResponse};
use service::TuiService;
use terraphim_types::{
    Document, Layer, LogicalOperator, NormalizedTermValue, RoleName, SearchQuery,
};
use terraphim_update::{check_for_updates, check_for_updates_startup, update_binary};

#[derive(clap::ValueEnum, Debug, Clone)]
enum LogicalOperatorCli {
    And,
    Or,
}

/// Truncate a snippet at a UTF-8 char boundary, appending "..." when truncated.
///
/// Naive `&s[..max]` panics when `max` lands inside a multi-byte char (e.g. typographic
/// quotes from email subjects). This walks char boundaries and stops at the last one
/// whose byte index is ≤ max.
fn truncate_snippet(s: &str, max_bytes: usize) -> String {
    if s.len() <= max_bytes {
        return s.to_string();
    }
    let cutoff = s
        .char_indices()
        .map(|(i, _)| i)
        .take_while(|&i| i <= max_bytes)
        .last()
        .unwrap_or(0);
    format!("{}...", &s[..cutoff])
}

#[cfg(test)]
mod truncate_snippet_tests {
    use super::truncate_snippet;

    #[test]
    fn short_string_unchanged() {
        assert_eq!(truncate_snippet("hello", 120), "hello");
    }

    #[test]
    fn ascii_truncated() {
        let s = "a".repeat(200);
        let out = truncate_snippet(&s, 120);
        assert!(out.ends_with("..."));
        assert_eq!(out.len(), 123);
    }

    #[test]
    fn multibyte_does_not_panic() {
        // Reproduces crates/terraphim_agent/src/main.rs:1414 panic where
        // `&s[..120]` landed inside a typographic quote (3 bytes: e2 80 9c).
        let s = "Includes dependencies for llama.cpp, integration with retreival, and CLI/GUI flows; the project positions itself as \u{201C}ultimate open-source RAG app\u{201D} with curated features.";
        let out = truncate_snippet(s, 120);
        // Must not panic and must be a valid UTF-8 string ending in "..."
        assert!(out.ends_with("..."));
        assert!(out.is_char_boundary(out.len()));
    }

    #[test]
    fn cyrillic_safe() {
        let s = "консенсус ".repeat(20);
        let out = truncate_snippet(&s, 120);
        assert!(out.ends_with("..."));
    }
}

/// Format the one-line stderr explainability message emitted when the search
/// command auto-routes (i.e. the user did not pass `--role`).
///
/// Exact format pinned by the design (section 5):
///   `[auto-route] picked role "<name>" (score=<n>, candidates=<m>); to override, pass --role`
fn format_auto_route_line(result: &terraphim_service::auto_route::AutoRouteResult) -> String {
    format!(
        "[auto-route] picked role \"{}\" (score={}, candidates={}); to override, pass --role",
        result.role.as_str(),
        result.score,
        result.candidates.len(),
    )
}

#[cfg(test)]
mod format_auto_route_line_tests {
    use super::format_auto_route_line;
    use terraphim_service::auto_route::{AutoRouteReason, AutoRouteResult};
    use terraphim_types::RoleName;

    #[test]
    fn pinned_exact_format() {
        let r = AutoRouteResult {
            role: RoleName::new("Personal Assistant"),
            score: 42,
            candidates: vec![
                (RoleName::new("Personal Assistant"), 42),
                (RoleName::new("Default"), 0),
            ],
            reason: AutoRouteReason::ScoredWinner,
        };
        assert_eq!(
            format_auto_route_line(&r),
            "[auto-route] picked role \"Personal Assistant\" (score=42, candidates=2); to override, pass --role"
        );
    }
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

#[cfg(feature = "server")]
fn resolve_tui_server_url(explicit: Option<&str>) -> String {
    let env_server = std::env::var("TERRAPHIM_SERVER").ok();
    resolve_tui_server_url_with_env(explicit, env_server.as_deref())
}

#[cfg(feature = "server")]
fn resolve_tui_server_url_with_env(explicit: Option<&str>, env_server: Option<&str>) -> String {
    explicit
        .map(ToOwned::to_owned)
        .or_else(|| env_server.map(ToOwned::to_owned))
        .unwrap_or_else(|| "http://localhost:8000".to_string())
}

#[cfg(feature = "server")]
fn tui_server_requirement_error(url: &str, cause: &anyhow::Error) -> anyhow::Error {
    anyhow::anyhow!(
        "Fullscreen TUI requires a running Terraphim server at {}. \
         Start terraphim_server or use offline mode with `terraphim-agent repl`. \
         Connection error: {}",
        url,
        cause
    )
}

#[cfg(feature = "server")]
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

#[cfg(feature = "repl-sessions")]
mod session_output {
    use serde::Serialize;

    #[derive(Debug, Serialize)]
    pub struct SourcesOutput {
        pub count: usize,
        pub sources: Vec<SourceEntry>,
    }

    #[derive(Debug, Serialize)]
    pub struct SourceEntry {
        pub id: String,
        pub name: Option<String>,
        pub available: bool,
    }

    #[derive(Debug, Serialize)]
    pub struct SessionListOutput {
        pub total: usize,
        pub shown: usize,
        pub sessions: Vec<SessionEntry>,
    }

    #[derive(Debug, Serialize)]
    pub struct SessionEntry {
        pub id: String,
        pub title: Option<String>,
        pub message_count: usize,
        pub source: String,
    }

    #[derive(Debug, Serialize)]
    pub struct SessionSearchOutput {
        pub query: String,
        pub total: usize,
        pub shown: usize,
        pub sessions: Vec<SessionSearchEntry>,
    }

    #[derive(Debug, Serialize)]
    pub struct SessionSearchEntry {
        pub id: String,
        pub title: Option<String>,
        pub message_count: usize,
        pub preview: Option<String>,
    }

    #[derive(Debug, Serialize)]
    pub struct SessionStatsOutput {
        pub total_sessions: usize,
        pub total_messages: usize,
        pub total_user_messages: usize,
        pub total_assistant_messages: usize,
        pub by_source: std::collections::HashMap<String, usize>,
    }
}

#[allow(dead_code)]
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
    about = "Terraphim Agent: server-backed fullscreen TUI with offline-capable REPL and CLI commands",
    after_long_help = "EXIT CODES (F1.2 contract)\n\
        \n\
        \x20 0  SUCCESS             Operation completed successfully\n\
        \x20 1  ERROR_GENERAL       Unspecified or unexpected error\n\
        \x20 2  ERROR_USAGE         Invalid arguments or unknown command\n\
        \x20 3  ERROR_INDEX_MISSING Required index not initialised\n\
        \x20 4  ERROR_NOT_FOUND     No results (only with --fail-on-empty)\n\
        \x20 5  ERROR_AUTH          Authentication required or failed\n\
        \x20 6  ERROR_NETWORK       Transport-level network error\n\
        \x20 7  ERROR_TIMEOUT       Operation exceeded configured timeout\n"
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
        #[arg(long, default_value_t = false)]
        fail_on_empty: bool,
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
    #[cfg(feature = "llm")]
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

    /// Start listener mode for AI agent communication (offline-only)
    Listen {
        /// Agent identity/name for this listener instance
        #[arg(long, required = true)]
        identity: String,
        /// Optional listener configuration JSON file
        #[arg(long)]
        config: Option<String>,
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
        /// Enable semantic matching via KG entities
        #[arg(long, default_value_t = false)]
        semantic: bool,
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
        /// Hook type for multi-hook pipeline
        #[arg(long, value_enum, default_value = "post-tool-use")]
        learn_hook_type: learnings::LearnHookType,
    },
    /// Install hook for AI agent
    InstallHook {
        /// AI agent to install hook for
        #[arg(value_enum)]
        agent: learnings::AgentType,
    },
    /// Manage captured procedures (recorded command sequences)
    Procedure {
        #[command(subcommand)]
        sub: ProcedureSub,
    },
    /// Compile captured corrections into a thesaurus for the replace command
    Compile {
        /// Output path for compiled thesaurus JSON
        #[arg(long, default_value = "compiled-corrections.json")]
        output: PathBuf,
        /// Optional: merge with this curated thesaurus file
        #[arg(long)]
        merge_with: Option<PathBuf>,
    },
    /// Review and approve/reject knowledge suggestions
    #[cfg(feature = "shared-learning")]
    Suggest {
        #[command(subcommand)]
        sub: SuggestSub,
    },
    /// Export captured corrections as reviewable KG markdown artefacts
    ExportKg {
        /// Output directory for KG markdown files
        #[arg(long)]
        output: PathBuf,
        /// Filter by correction type: tool-preference or all (default: all)
        #[arg(long, default_value = "all")]
        correction_type: String,
    },
    /// Manage shared learnings with trust levels (L1/L2/L3)
    #[cfg(feature = "shared-learning")]
    Shared {
        #[command(subcommand)]
        sub: SharedLearningSub,
    },
}

#[cfg(feature = "shared-learning")]
#[derive(Subcommand, Debug)]
enum SharedLearningSub {
    /// List shared learnings, optionally filtered by trust level
    List {
        /// Filter by trust level: l1, l2, l3
        #[arg(long)]
        trust_level: Option<String>,
        /// Maximum number of learnings to show
        #[arg(long, default_value_t = 20)]
        limit: usize,
    },
    /// Promote a shared learning to a higher trust level
    Promote {
        /// Learning ID
        id: String,
        /// Target trust level: l2 or l3
        #[arg(long)]
        to: String,
    },
    /// Import local captured learnings into the shared learning store at L1
    Import,
    /// Show shared learning statistics by trust level
    Stats,
    /// Inject learnings from shared directory into local store
    #[cfg(feature = "cross-agent-injection")]
    Inject {
        /// Minimum trust level to inject (l1, l2, l3)
        #[arg(long, default_value = "l2")]
        min_trust: String,
        /// Dry run (show what would be injected without injecting)
        #[arg(long, default_value_t = false)]
        dry_run: bool,
    },
}

#[cfg(feature = "shared-learning")]
#[derive(Subcommand, Debug)]
enum SuggestSub {
    /// List pending suggestions, optionally filtered by status
    List {
        /// Filter by status: pending, approved, rejected
        #[arg(long)]
        status: Option<String>,
        #[arg(long, default_value_t = 20)]
        limit: usize,
    },
    /// Show full details of a suggestion
    Show { id: String },
    /// Approve a suggestion (promotes to L3 and marks as approved)
    Approve { id: String },
    /// Reject a suggestion
    Reject {
        id: String,
        #[arg(long)]
        reason: Option<String>,
    },
    /// Approve all pending suggestions above a confidence threshold
    ApproveAll {
        #[arg(long, default_value_t = 0.8)]
        min_confidence: f64,
        #[arg(long, default_value_t = false)]
        dry_run: bool,
    },
    /// Reject all pending suggestions below a confidence threshold
    RejectAll {
        #[arg(long, default_value_t = 0.3)]
        max_confidence: f64,
        #[arg(long, default_value_t = false)]
        dry_run: bool,
    },
    /// Show suggestion approval metrics
    Metrics,
    /// Show session-end suggestion summary
    SessionEnd {
        #[arg(long)]
        context: Option<String>,
    },
}

#[derive(Subcommand, Debug)]
enum ProcedureSub {
    /// List stored procedures (most recent first)
    List {
        /// Number of recent procedures to show
        #[arg(long, default_value_t = 10)]
        recent: usize,
    },
    /// Show full details of a procedure
    Show {
        /// Procedure ID
        id: String,
    },
    /// Create a new empty procedure
    Record {
        /// Procedure title
        title: String,
        /// Optional description
        #[arg(long)]
        description: Option<String>,
    },
    /// Add a step to an existing procedure
    AddStep {
        /// Procedure ID
        id: String,
        /// Command to execute in this step
        command: String,
        /// Precondition that must hold before this step
        #[arg(long)]
        precondition: Option<String>,
        /// Postcondition that should hold after this step
        #[arg(long)]
        postcondition: Option<String>,
    },
    /// Record a successful execution of a procedure
    Success {
        /// Procedure ID
        id: String,
    },
    /// Record a failed execution of a procedure
    Failure {
        /// Procedure ID
        id: String,
    },
    /// Replay a stored procedure (execute its steps in order)
    Replay {
        /// Procedure ID
        id: String,
        /// Print steps without executing them
        #[arg(long, default_value_t = false)]
        dry_run: bool,
    },
    /// Show health status of all procedures (auto-disables critically failing ones)
    Health,
    /// Enable a previously disabled procedure
    Enable {
        /// Procedure ID
        id: String,
    },
    /// Disable a procedure (prevents replay)
    Disable {
        /// Procedure ID
        id: String,
    },
    /// Auto-capture a procedure from a session's Bash commands
    #[cfg(feature = "repl-sessions")]
    FromSession {
        /// Session ID to extract commands from
        session_id: String,
        /// Optional title (auto-generated from first command if not provided)
        #[arg(long)]
        title: Option<String>,
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

fn emit_robot_error_and_exit(
    err: &anyhow::Error,
    code: robot::exit_codes::ExitCode,
    robot: bool,
    format: &OutputFormat,
) -> ! {
    if robot || !matches!(format, OutputFormat::Human) {
        use crate::robot::schema::{ResponseMeta, RobotError, RobotResponse};
        let meta = ResponseMeta::new("unknown");
        let robot_error = RobotError::new(format!("E{:03}", code.code()), format!("{:#}", err));
        let response = RobotResponse::<()>::error(vec![robot_error], meta);
        if let Ok(json) = serde_json::to_string(&response) {
            println!("{}", json);
        }
    }
    eprintln!("Error: {:#}", err);
    std::process::exit(code.code().into())
}

fn classify_error(err: &anyhow::Error) -> robot::exit_codes::ExitCode {
    use robot::exit_codes::ExitCode;

    if err.chain().any(|e| e.is::<tokio::time::error::Elapsed>()) {
        return ExitCode::ErrorTimeout;
    }

    #[cfg(feature = "server")]
    if err.chain().any(|e| e.is::<reqwest::Error>()) {
        let is_timeout = err
            .chain()
            .filter_map(|e| e.downcast_ref::<reqwest::Error>())
            .any(|re| re.is_timeout());
        if is_timeout {
            return ExitCode::ErrorTimeout;
        }
        return ExitCode::ErrorNetwork;
    }

    let msg = err.to_string().to_lowercase();

    if msg.contains("timed out") || msg.contains("timeout") || msg.contains("elapsed") {
        ExitCode::ErrorTimeout
    } else if msg.contains("connection refused")
        || msg.contains("connection reset")
        || msg.contains("network")
        || msg.contains("dns")
        || msg.contains("transport")
        || msg.contains("connect error")
    {
        ExitCode::ErrorNetwork
    } else if msg.contains("unauthori")
        || msg.contains("unauthenticated")
        || msg.contains("forbidden")
        || msg.contains("authentication required")
        || msg.contains("authentication failed")
        || msg.contains(" 401 ")
        || msg.contains(" 403 ")
        || msg.ends_with(" 401")
        || msg.ends_with(" 403")
        || msg.contains("http 401")
        || msg.contains("http 403")
    {
        ExitCode::ErrorAuth
    } else if msg.contains("index not found")
        || msg.contains("index missing")
        || msg.contains("not initialised")
        || msg.contains("not initialized")
        || (msg.contains("not found") && msg.contains("index"))
        || msg.contains("knowledge graph not configured")
        || msg.contains("no local knowledge graph")
        || (msg.contains("thesaurus")
            && (msg.contains("not found") || msg.contains("failed to load")))
    {
        ExitCode::ErrorIndexMissing
    } else {
        ExitCode::ErrorGeneral
    }
}

#[cfg(test)]
mod classify_error_tests {
    use super::*;
    use robot::exit_codes::ExitCode;

    fn err(msg: &str) -> anyhow::Error {
        anyhow::anyhow!("{}", msg)
    }

    #[test]
    fn general_error_maps_to_1() {
        assert_eq!(
            classify_error(&err("something unexpected happened")),
            ExitCode::ErrorGeneral
        );
    }

    #[test]
    fn index_missing_patterns_map_to_3() {
        assert_eq!(
            classify_error(&err("index not found on disk")),
            ExitCode::ErrorIndexMissing
        );
        assert_eq!(
            classify_error(&err("index missing")),
            ExitCode::ErrorIndexMissing
        );
        assert_eq!(
            classify_error(&err("automata index not initialised")),
            ExitCode::ErrorIndexMissing
        );
        assert_eq!(
            classify_error(&err("Config error: knowledge graph not configured")),
            ExitCode::ErrorIndexMissing
        );
        assert_eq!(
            classify_error(&err("no local knowledge graph path available")),
            ExitCode::ErrorIndexMissing
        );
        assert_eq!(
            classify_error(&err("thesaurus not found at path")),
            ExitCode::ErrorIndexMissing
        );
    }

    #[test]
    fn auth_patterns_map_to_5() {
        assert_eq!(
            classify_error(&err("authentication required")),
            ExitCode::ErrorAuth
        );
        assert_eq!(
            classify_error(&err("request forbidden: 403")),
            ExitCode::ErrorAuth
        );
        assert_eq!(
            classify_error(&err("401 Unauthorised")),
            ExitCode::ErrorAuth
        );
        assert_eq!(
            classify_error(&err("server returned 403 Forbidden")),
            ExitCode::ErrorAuth
        );
    }

    #[test]
    fn non_auth_strings_do_not_map_to_5() {
        assert_ne!(
            classify_error(&err("author field missing")),
            ExitCode::ErrorAuth
        );
        assert_ne!(
            classify_error(&err("authority header")),
            ExitCode::ErrorAuth
        );
        assert_ne!(
            classify_error(&err("failed to open auth_tokens.json")),
            ExitCode::ErrorAuth
        );
        assert_ne!(
            classify_error(&err("error code 4010 unknown")),
            ExitCode::ErrorAuth
        );
    }

    #[test]
    fn timeout_patterns_map_to_7() {
        assert_eq!(
            classify_error(&err("operation timed out")),
            ExitCode::ErrorTimeout
        );
        assert_eq!(
            classify_error(&err("deadline elapsed waiting for response")),
            ExitCode::ErrorTimeout
        );
        assert_eq!(
            classify_error(&err("request timeout after 30s")),
            ExitCode::ErrorTimeout
        );
    }

    #[test]
    fn network_patterns_map_to_6() {
        assert_eq!(
            classify_error(&err("connection refused on port 8080")),
            ExitCode::ErrorNetwork
        );
        assert_eq!(
            classify_error(&err("dns resolution failed")),
            ExitCode::ErrorNetwork
        );
        assert_eq!(
            classify_error(&err("network error connecting to host")),
            ExitCode::ErrorNetwork
        );
    }
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let output = resolve_output_config(cli.robot, cli.format.clone());

    // Check for updates on startup (non-blocking, debug logging on failure)
    let rt = Runtime::new()?;
    rt.block_on(async {
        if let Err(e) = check_for_updates_startup("terraphim-agent").await {
            log::debug!("Update check failed: {}", e);
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

            #[cfg(feature = "server")]
            {
                if cli.server {
                    run_tui_server_mode(&cli.server_url, cli.transparent)
                } else {
                    run_tui_offline_mode(cli.transparent)
                }
            }
            #[cfg(not(feature = "server"))]
            {
                if cli.server {
                    eprintln!(
                        "TUI server mode requires the 'server' feature. Use offline mode instead."
                    );
                    return Err(anyhow::anyhow!("TUI server mode requires server feature"));
                }
                run_tui_offline_mode(cli.transparent)
            }
        }

        #[cfg(feature = "repl")]
        Some(Command::Repl { server, .. }) => {
            let rt = Runtime::new()?;
            #[cfg(feature = "server")]
            {
                if server {
                    return rt.block_on(repl::run_repl_server_mode("http://localhost:8000"));
                }
            }
            #[cfg(not(feature = "server"))]
            {
                if server {
                    eprintln!(
                        "REPL server mode requires the 'server' feature. Starting in offline mode instead."
                    );
                }
            }
            rt.block_on(repl::run_repl_offline_mode())
        }

        Some(Command::Listen { identity, config }) => {
            // Listen mode is offline-only - reject --server flag
            if cli.server {
                eprintln!("error: listen mode does not support --server flag");
                eprintln!("The listener runs in offline mode only.");
                std::process::exit(2);
            }
            let listener_config = match config.as_deref() {
                Some(path) => listener::ListenerConfig::load_from_path(path)?,
                None => listener::ListenerConfig::for_identity(identity.clone()),
            };
            listener_config.validate()?;
            println!("listener would start with identity: {}", identity);
            println!(
                "resolved Gitea login: {}",
                listener_config.identity.resolved_gitea_login()
            );
            println!("poll interval: {}s", listener_config.poll_interval_secs);
            if listener_config.gitea.is_none() {
                println!("listener config has no Gitea connection; discovery only");
                return Ok(());
            }
            rt.block_on(listener::run_listener(listener_config))
        }
        Some(command) => {
            let rt = Runtime::new()?;
            let robot_mode = cli.robot;
            let output_format = cli.format.clone();
            #[cfg(feature = "server")]
            {
                if cli.server {
                    let result = rt.block_on(run_server_command(command, &cli.server_url, output));
                    if let Err(ref e) = result {
                        let code = classify_error(e);
                        emit_robot_error_and_exit(e, code, robot_mode, &output_format);
                    }
                    return result;
                }
            }
            let result = rt.block_on(run_offline_command(command, output, cli.config));
            if let Err(ref e) = result {
                let code = classify_error(e);
                emit_robot_error_and_exit(e, code, robot_mode, &output_format);
            }
            result
        }
    }
}
fn run_tui_offline_mode(transparent: bool) -> Result<()> {
    // Fullscreen TUI mode requires a running server.
    // For offline operation, use `terraphim-agent repl`.
    run_tui(None, transparent)
}

#[allow(dead_code)]
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
            fail_on_empty,
        } => {
            let (role_name, auto) = service
                .resolve_or_auto_route(role.as_deref(), &query)
                .await?;
            if let Some(ref ar) = auto {
                eprintln!("{}", format_auto_route_line(ar));
            }

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
                    include_pinned: false,
                    role: Some(role_name.clone()),
                    layer: Layer::default(),
                };

                service.search_with_query(&search_query).await?
            } else {
                // Single term query (backward compatibility)
                service
                    .search_with_role(&query, &role_name, Some(limit))
                    .await?
            };

            let results_count = results.len();
            if output.is_machine_readable() {
                use crate::robot::schema::{SearchResultItem, SearchResultsData};
                use crate::robot::{ResponseMeta, RobotConfig, RobotFormatter, RobotResponse};
                use std::time::Instant;

                let start = Instant::now();
                let robot_format = match output.mode {
                    CommandOutputMode::JsonCompact => crate::robot::output::OutputFormat::Minimal,
                    _ => crate::robot::output::OutputFormat::Json,
                };
                let mut robot_config = RobotConfig::new()
                    .with_format(robot_format)
                    .with_max_results(limit);
                if output.robot {
                    robot_config = robot_config
                        .with_max_content_length(2000)
                        .with_max_tokens(8000);
                }

                let formatter = RobotFormatter::new(robot_config.clone());
                let max_results = robot_config.max_results.unwrap_or(limit);
                let truncated_results: Vec<_> = results.into_iter().take(max_results).collect();
                let total = truncated_results.len();

                let items: Vec<SearchResultItem> = truncated_results
                    .iter()
                    .enumerate()
                    .map(|(i, doc)| {
                        let preview = doc.description.as_deref().or(if doc.body.is_empty() {
                            None
                        } else {
                            Some(doc.body.as_str())
                        });
                        let (preview_text, preview_truncated) = match preview {
                            Some(text) => {
                                let (t, was_truncated) = formatter.truncate_content(text.trim());
                                (Some(t), was_truncated)
                            }
                            None => (None, false),
                        };
                        SearchResultItem {
                            rank: i + 1,
                            id: doc.id.clone(),
                            title: doc.title.clone(),
                            url: if doc.url.is_empty() {
                                None
                            } else {
                                Some(doc.url.clone())
                            },
                            score: doc.rank.unwrap_or_default() as f64,
                            preview: preview_text,
                            source: None,
                            date: None,
                            preview_truncated,
                        }
                    })
                    .collect();

                let data = SearchResultsData {
                    results: items,
                    total_matches: total,
                    concepts_matched: vec![],
                    wildcard_fallback: false,
                };

                let meta =
                    ResponseMeta::new("search").with_elapsed(start.elapsed().as_millis() as u64);
                let response = RobotResponse::success(data, meta);
                let output_str = formatter.format(&response)?;
                println!("{}", output_str);
            } else {
                for doc in results.iter() {
                    let snippet = doc
                        .description
                        .as_deref()
                        .or(if doc.body.is_empty() {
                            None
                        } else {
                            Some(doc.body.as_str())
                        })
                        .map(|s| truncate_snippet(s.trim(), 120));
                    println!("[{}] {}", doc.rank.unwrap_or_default(), doc.title);
                    if !doc.url.is_empty() {
                        println!("    {}", doc.url);
                    }
                    if let Some(snip) = snippet {
                        println!("    {}", snip);
                    }
                    println!();
                }
            }
            if fail_on_empty && results_count == 0 {
                std::process::exit(robot::exit_codes::ExitCode::ErrorNotFound.code().into());
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
            let role_name = service.resolve_role(role.as_deref()).await?;

            let concepts = service.get_role_graph_top_k(&role_name, top_k).await?;
            for concept in concepts {
                println!("{}", concept);
            }
            Ok(())
        }
        #[cfg(feature = "llm")]
        Command::Chat {
            role,
            prompt,
            model,
        } => {
            let role_name = service.resolve_role(role.as_deref()).await?;

            let response = service.chat(&role_name, &prompt, model).await?;
            println!("{}", response);
            Ok(())
        }
        Command::Extract {
            text,
            role,
            exclude_term,
        } => {
            let role_name = service.resolve_role(role.as_deref()).await?;

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

            let role_name = service.resolve_role(role.as_deref()).await?;

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
                                #[allow(clippy::unnecessary_sort_by)]
                                sorted_matches.sort_by(|a, b| b.pos.cmp(&a.pos));

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

            let role_name = service.resolve_role(role.as_deref()).await?;

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

            let role_name = service.resolve_role(role.as_deref()).await?;

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

            let role_name = service.resolve_role(role.as_deref()).await?;

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

                            // KG validation: find patterns with known alternatives
                            let kg_validation = kg_validation::validate_command_against_kg(command);

                            // Get thesaurus and perform replacement
                            let thesaurus = service.get_thesaurus(&role_name).await?;
                            let replacement_service =
                                terraphim_hooks::ReplacementService::new(thesaurus);
                            let hook_result = replacement_service.replace_fail_open(command);

                            // If replacement occurred or KG validation has findings, output modified input
                            if hook_result.replacements > 0 || kg_validation.has_findings {
                                let mut output = input_value.clone();
                                if hook_result.replacements > 0 {
                                    if let Some(tool_input) = output.get_mut("tool_input") {
                                        if let Some(obj) = tool_input.as_object_mut() {
                                            obj.insert(
                                                "command".to_string(),
                                                serde_json::Value::String(
                                                    hook_result.result.clone(),
                                                ),
                                            );
                                        }
                                    }
                                }
                                if kg_validation.has_findings {
                                    if let Some(obj) = output.as_object_mut() {
                                        obj.insert(
                                            "validations".to_string(),
                                            serde_json::to_value(&kg_validation)
                                                .unwrap_or_default(),
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
            use session_output::*;
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
                        if !output.is_machine_readable() {
                            println!("Loaded sessions from cache.");
                        }
                    }
                }
            }

            match sub {
                SessionsSub::Sources => {
                    let sources = service.detect_sources();
                    if output.is_machine_readable() {
                        let payload = SourcesOutput {
                            count: sources.len(),
                            sources: sources
                                .into_iter()
                                .map(|s| {
                                    let available = s.is_available();
                                    SourceEntry {
                                        id: s.id,
                                        name: s.name,
                                        available,
                                    }
                                })
                                .collect(),
                        };
                        print_json_output(&payload, output.mode)?;
                    } else if sources.is_empty() {
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
                    if output.is_machine_readable() {
                        let session_entries: Vec<SessionEntry> = sessions
                            .iter()
                            .take(limit)
                            .map(|s| SessionEntry {
                                id: s.id.to_string(),
                                title: s.title.clone(),
                                message_count: s.message_count(),
                                source: s.source.clone(),
                            })
                            .collect();
                        let shown = session_entries.len();
                        let payload = SessionListOutput {
                            total: sessions.len(),
                            shown,
                            sessions: session_entries,
                        };
                        print_json_output(&payload, output.mode)?;
                    } else if sessions.is_empty() {
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
                    if output.is_machine_readable() {
                        let entries: Vec<SessionSearchEntry> = results
                            .iter()
                            .take(limit)
                            .map(|s| {
                                let preview = s
                                    .messages
                                    .iter()
                                    .find(|msg| {
                                        msg.content.to_lowercase().contains(&query.to_lowercase())
                                    })
                                    .map(|msg| {
                                        let p: String = msg.content.chars().take(100).collect();
                                        p
                                    });
                                SessionSearchEntry {
                                    id: s.id.to_string(),
                                    title: s.title.clone(),
                                    message_count: s.message_count(),
                                    preview,
                                }
                            })
                            .collect();
                        let shown = entries.len();
                        let payload = SessionSearchOutput {
                            query: query.clone(),
                            total: results.len(),
                            shown,
                            sessions: entries,
                        };
                        print_json_output(&payload, output.mode)?;
                        if results.is_empty() {
                            std::process::exit(
                                robot::exit_codes::ExitCode::ErrorNotFound.code().into(),
                            );
                        }
                    } else if results.is_empty() {
                        println!("No sessions matching '{}'.", query);
                    } else {
                        println!("Found {} matching sessions:", results.len());
                        for session in results.iter().take(limit) {
                            let title = session.title.as_deref().unwrap_or("(untitled)");
                            println!("  - {}", title);
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
                    if output.is_machine_readable() {
                        let payload = SessionStatsOutput {
                            total_sessions: stats.total_sessions,
                            total_messages: stats.total_messages,
                            total_user_messages: stats.total_user_messages,
                            total_assistant_messages: stats.total_assistant_messages,
                            by_source: stats.sessions_by_source,
                        };
                        print_json_output(&payload, output.mode)?;
                    } else {
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
                    }
                    Ok(())
                }
            }
        }

        Command::Listen { identity, config } => {
            println!("listener would start with identity: {}", identity);
            if let Some(path) = config.as_deref() {
                println!("listener config: {}", path);
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

async fn run_learn_command(sub: LearnSub) -> Result<()> {
    use learnings::{
        CorrectionType, LearningCaptureConfig, capture_correction, capture_failed_command,
        correct_learning, list_all_entries,
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
            semantic,
        } => {
            let storage_loc = config.storage_location();
            let storage_dir = if global {
                &config.global_dir
            } else {
                &storage_loc
            };
            match learnings::query_all_entries_semantic(storage_dir, &pattern, exact, semantic) {
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
                            let entities = entry.entities();
                            if !entities.is_empty() {
                                println!("     Entities: {}", entities.join(", "));
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
            let correction = capture_correction(ct, &original, &corrected, &context, &config);
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
        LearnSub::Hook {
            format,
            learn_hook_type,
        } => learnings::process_hook_input_with_type(format, learn_hook_type)
            .await
            .map_err(|e| e.into()),
        LearnSub::InstallHook { agent } => {
            learnings::install_hook(agent).await.map_err(|e| e.into())
        }
        LearnSub::Procedure { sub } => {
            let procedures_path = config.global_dir.join("procedures.jsonl");
            let store = learnings::ProcedureStore::new(procedures_path);

            match sub {
                ProcedureSub::List { recent } => {
                    let all = store.load_all()?;
                    if all.is_empty() {
                        println!("No procedures found.");
                    } else {
                        let display_count = recent.min(all.len());
                        println!("Procedures ({} of {}):", display_count, all.len());
                        for proc in all.iter().rev().take(recent) {
                            println!(
                                "  [{}] {} -- {} steps, confidence {:.0}% ({}/{})",
                                proc.id,
                                proc.title,
                                proc.step_count(),
                                proc.confidence.score * 100.0,
                                proc.confidence.success_count,
                                proc.confidence.total_executions(),
                            );
                        }
                    }
                    Ok(())
                }
                ProcedureSub::Show { id } => {
                    match store.find_by_id(&id)? {
                        Some(proc) => {
                            println!("Procedure: {}", proc.title);
                            println!("ID: {}", proc.id);
                            println!("Description: {}", proc.description);
                            println!(
                                "Confidence: {:.0}% ({} successes, {} failures)",
                                proc.confidence.score * 100.0,
                                proc.confidence.success_count,
                                proc.confidence.failure_count,
                            );
                            if proc.disabled {
                                println!("Status: DISABLED");
                            }
                            println!("Created: {}", proc.created_at);
                            println!("Updated: {}", proc.updated_at);
                            if !proc.tags.is_empty() {
                                println!("Tags: {}", proc.tags.join(", "));
                            }
                            if let Some(ref session) = proc.source_session {
                                println!("Source session: {}", session);
                            }
                            println!("Steps ({}):", proc.step_count());
                            for step in &proc.steps {
                                println!("  {}. {}", step.ordinal, step.command);
                                if let Some(ref pre) = step.precondition {
                                    println!("     pre: {}", pre);
                                }
                                if let Some(ref post) = step.postcondition {
                                    println!("     post: {}", post);
                                }
                            }
                        }
                        None => {
                            eprintln!("Procedure '{}' not found.", id);
                        }
                    }
                    Ok(())
                }
                ProcedureSub::Record { title, description } => {
                    use uuid::Uuid;
                    let id = Uuid::new_v4().to_string();
                    let desc = description.unwrap_or_default();
                    let procedure =
                        terraphim_types::procedure::CapturedProcedure::new(id.clone(), title, desc);
                    store.save(&procedure)?;
                    println!("Created procedure: {}", id);
                    Ok(())
                }
                ProcedureSub::AddStep {
                    id,
                    command,
                    precondition,
                    postcondition,
                } => {
                    let mut proc = store
                        .find_by_id(&id)?
                        .ok_or_else(|| anyhow::anyhow!("Procedure '{}' not found", id))?;
                    let ordinal = proc.step_count() as u32 + 1;
                    proc.add_step(terraphim_types::procedure::ProcedureStep {
                        ordinal,
                        command,
                        precondition,
                        postcondition,
                        working_dir: None,
                        privileged: false,
                        tags: vec![],
                    });
                    store.save(&proc)?;
                    println!("Added step {} to procedure '{}'.", ordinal, id);
                    Ok(())
                }
                ProcedureSub::Success { id } => {
                    store.update_confidence(&id, true)?;
                    println!("Recorded success for procedure '{}'.", id);
                    Ok(())
                }
                ProcedureSub::Failure { id } => {
                    store.update_confidence(&id, false)?;
                    println!("Recorded failure for procedure '{}'.", id);
                    Ok(())
                }
                ProcedureSub::Replay { id, dry_run } => {
                    let procedure = store.find_by_id(&id)?;
                    match procedure {
                        None => {
                            eprintln!("Procedure '{}' not found.", id);
                            std::process::exit(1);
                        }
                        Some(proc) => {
                            // Check if procedure is disabled
                            if proc.disabled {
                                eprintln!(
                                    "Procedure '{}' is disabled. Use 'learn procedure enable {}' to re-enable it.",
                                    id, id,
                                );
                                std::process::exit(1);
                            }

                            // Check minimum confidence threshold
                            if proc.confidence.total_executions() > 0 && proc.confidence.score < 0.5
                            {
                                eprintln!(
                                    "Procedure '{}' has low confidence ({:.0}%). \
                                     Use --dry-run to preview, or record more successes first.",
                                    id,
                                    proc.confidence.score * 100.0,
                                );
                                std::process::exit(1);
                            }

                            println!(
                                "Replaying procedure '{}' ({} steps){}",
                                proc.title,
                                proc.step_count(),
                                if dry_run { " [DRY RUN]" } else { "" },
                            );

                            let result = learnings::replay_procedure(&proc, dry_run)?;

                            // Print outcomes
                            for (ordinal, outcome) in &result.outcomes {
                                match outcome {
                                    learnings::StepOutcome::Success { stdout } => {
                                        println!("  step {}: OK", ordinal);
                                        if !stdout.trim().is_empty() && stdout != "(dry-run)" {
                                            for line in stdout.lines() {
                                                println!("    | {}", line);
                                            }
                                        }
                                    }
                                    learnings::StepOutcome::Failed { stderr, exit_code } => {
                                        println!("  step {}: FAILED (exit {})", ordinal, exit_code);
                                        if !stderr.trim().is_empty() {
                                            for line in stderr.lines() {
                                                println!("    | {}", line);
                                            }
                                        }
                                    }
                                    learnings::StepOutcome::Skipped { reason } => {
                                        println!("  step {}: SKIPPED ({})", ordinal, reason);
                                    }
                                }
                            }

                            // Update confidence based on result (skip for dry-run)
                            if !dry_run {
                                store.update_confidence(&id, result.overall_success)?;
                                if result.overall_success {
                                    println!("Replay completed successfully.");
                                } else {
                                    println!("Replay failed.");
                                    std::process::exit(1);
                                }
                            } else {
                                println!("Dry run completed.");
                            }

                            Ok(())
                        }
                    }
                }
                ProcedureSub::Health => {
                    let reports = store.health_check()?;
                    if reports.is_empty() {
                        println!("No procedures found.");
                    } else {
                        println!(
                            "{:<38} {:<12} {:<8} {:<6} {:<9}",
                            "ID", "STATUS", "RATE", "RUNS", "DISABLED"
                        );
                        println!("{}", "-".repeat(73));
                        for report in &reports {
                            println!(
                                "{:<38} {:<12} {:<8.0}% {:<6} {:<9}",
                                report.id,
                                report.status.to_string(),
                                report.success_rate * 100.0,
                                report.total_executions,
                                if report.auto_disabled
                                    || store
                                        .find_by_id(&report.id)?
                                        .map(|p| p.disabled)
                                        .unwrap_or(false)
                                {
                                    "yes"
                                } else {
                                    "no"
                                },
                            );
                        }
                        let auto_disabled_count =
                            reports.iter().filter(|r| r.auto_disabled).count();
                        if auto_disabled_count > 0 {
                            println!(
                                "\n{} procedure(s) auto-disabled due to critical failure rate.",
                                auto_disabled_count,
                            );
                        }
                    }
                    Ok(())
                }
                ProcedureSub::Enable { id } => {
                    store.set_disabled(&id, false)?;
                    println!("Procedure '{}' enabled.", id);
                    Ok(())
                }
                ProcedureSub::Disable { id } => {
                    store.set_disabled(&id, true)?;
                    println!("Procedure '{}' disabled.", id);
                    Ok(())
                }
                #[cfg(feature = "repl-sessions")]
                ProcedureSub::FromSession { session_id, title } => {
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
                            }
                        }
                    }

                    let session = service.get_session(&session_id).await;
                    match session {
                        Some(sess) => {
                            let commands =
                                learnings::procedure::extract_bash_commands_from_session(&sess);
                            if commands.is_empty() {
                                println!("No Bash commands found in session '{}'.", session_id);
                                return Ok(());
                            }
                            let total_cmds = commands.len();
                            let mut procedure =
                                learnings::procedure::from_session_commands(commands, title);
                            procedure.source_session = Some(session_id.clone());
                            let step_count = procedure.step_count();

                            let saved = store.save_with_dedup(procedure)?;
                            println!(
                                "Created procedure '{}' (ID: {}) with {} steps from {} commands.",
                                saved.title, saved.id, step_count, total_cmds
                            );
                            Ok(())
                        }
                        None => {
                            eprintln!(
                                "Session '{}' not found. Try running 'sessions list' first to import sessions.",
                                session_id
                            );
                            std::process::exit(1);
                        }
                    }
                }
            }
        }
        LearnSub::Compile { output, merge_with } => {
            let storage_loc = config.storage_location();
            let compiled = learnings::compile_corrections_to_thesaurus(&storage_loc)
                .map_err(|e| anyhow::anyhow!("Failed to compile corrections: {}", e))?;

            let compiled_count = compiled.len();

            let final_thesaurus = if let Some(ref merge_path) = merge_with {
                let curated_json = std::fs::read_to_string(merge_path).map_err(|e| {
                    anyhow::anyhow!("Failed to read curated thesaurus {:?}: {}", merge_path, e)
                })?;
                let curated: terraphim_types::Thesaurus = serde_json::from_str(&curated_json)
                    .map_err(|e| {
                        anyhow::anyhow!("Failed to parse curated thesaurus {:?}: {}", merge_path, e)
                    })?;
                let curated_count = curated.len();
                let merged = learnings::merge_thesauruses(curated, compiled);
                println!(
                    "Compiled {} correction(s), merged with {} curated entries -> {} total entries.",
                    compiled_count,
                    curated_count,
                    merged.len()
                );
                merged
            } else {
                println!("Compiled {} correction(s).", compiled_count);
                compiled
            };

            learnings::write_thesaurus_json(&final_thesaurus, &output)
                .map_err(|e| anyhow::anyhow!("Failed to write thesaurus to {:?}: {}", output, e))?;

            println!("Thesaurus written to: {}", output.display());
            Ok(())
        }
        LearnSub::ExportKg {
            output,
            correction_type,
        } => {
            let storage_loc = config.storage_location();
            let filter = match correction_type.as_str() {
                "tool-preference" => learnings::CorrectionTypeFilter::ToolPreference,
                "all" => learnings::CorrectionTypeFilter::All,
                _ => {
                    return Err(anyhow::anyhow!(
                        "Invalid correction_type '{}'. Use 'tool-preference' or 'all'.",
                        correction_type
                    ));
                }
            };
            let count = learnings::export_corrections_as_kg(&storage_loc, &output, filter)
                .map_err(|e| anyhow::anyhow!("Failed to export corrections: {}", e))?;
            println!(
                "Exported {} correction(s) as KG markdown to: {}",
                count,
                output.display()
            );
            Ok(())
        }
        #[cfg(feature = "shared-learning")]
        LearnSub::Suggest { sub } => run_suggest_command(sub).await,
        #[cfg(feature = "shared-learning")]
        LearnSub::Shared { sub } => run_shared_learning_command(sub, &config).await,
    }
}

#[cfg(feature = "shared-learning")]
async fn run_suggest_command(sub: SuggestSub) -> Result<()> {
    use crate::learnings::suggest::{SuggestionMetrics, SuggestionMetricsEntry};
    use terraphim_agent::shared_learning::{SharedLearningStore, StoreConfig, SuggestionStatus};
    use terraphim_types::shared_learning::SuggestionStatus as Status;

    let store_config = StoreConfig::default();
    let store = SharedLearningStore::open(store_config)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to open shared learning store: {}", e))?;
    let metrics = SuggestionMetrics::new(SuggestionMetrics::default_path());

    match sub {
        SuggestSub::List { status, limit } => {
            let entries = if let Some(ref s) = status {
                let st: SuggestionStatus = s.parse().map_err(|e| anyhow::anyhow!("{}", e))?;
                store
                    .list_by_status(st)
                    .await
                    .map_err(|e| anyhow::anyhow!("{}", e))?
            } else {
                store
                    .list_pending()
                    .await
                    .map_err(|e| anyhow::anyhow!("{}", e))?
            };
            if entries.is_empty() {
                println!("No suggestions found.");
            } else {
                let display_count = limit.min(entries.len());
                println!("Suggestions ({} of {}):", display_count, entries.len());
                for entry in entries.iter().take(limit) {
                    let confidence = entry
                        .bm25_confidence
                        .map(|c| format!("{:.2}", c))
                        .unwrap_or_else(|| "N/A".to_string());
                    println!(
                        "  [{}] {} (confidence: {}, status: {})",
                        &entry.id[..entry.id.len().min(12)],
                        entry.title,
                        confidence,
                        entry.suggestion_status,
                    );
                }
            }
            Ok(())
        }
        SuggestSub::Show { id } => {
            let entry = store.get(&id).await.map_err(|e| anyhow::anyhow!("{}", e))?;
            println!("ID:          {}", entry.id);
            println!("Title:       {}", entry.title);
            println!("Status:      {}", entry.suggestion_status);
            println!("Trust Level: {}", entry.trust_level);
            println!("Source:      {} ({})", entry.source, entry.source_agent);
            println!("Created:     {}", entry.created_at.to_rfc3339());
            if let Some(ref reason) = entry.rejection_reason {
                println!("Reject Reason: {}", reason);
            }
            if let Some(c) = entry.bm25_confidence {
                println!("Confidence:  {:.4}", c);
            }
            println!("\n{}", entry.content);
            Ok(())
        }
        SuggestSub::Approve { id } => {
            store
                .approve(&id)
                .await
                .map_err(|e| anyhow::anyhow!("{}", e))?;
            let entry = store.get(&id).await.map_err(|e| anyhow::anyhow!("{}", e))?;
            metrics.append(SuggestionMetricsEntry {
                id: id.clone(),
                status: Status::Approved,
                confidence: entry.bm25_confidence.unwrap_or(0.0),
                timestamp: chrono::Utc::now(),
                title: entry.title,
            })?;
            println!("Approved suggestion {}.", id);
            Ok(())
        }
        SuggestSub::Reject { id, reason } => {
            store
                .reject(&id, reason.as_deref())
                .await
                .map_err(|e| anyhow::anyhow!("{}", e))?;
            let entry = store.get(&id).await.map_err(|e| anyhow::anyhow!("{}", e))?;
            metrics.append(SuggestionMetricsEntry {
                id: id.clone(),
                status: Status::Rejected,
                confidence: entry.bm25_confidence.unwrap_or(0.0),
                timestamp: chrono::Utc::now(),
                title: entry.title,
            })?;
            println!("Rejected suggestion {}.", id);
            Ok(())
        }
        SuggestSub::ApproveAll {
            min_confidence,
            dry_run,
        } => {
            let pending = store
                .list_pending()
                .await
                .map_err(|e| anyhow::anyhow!("{}", e))?;
            let to_approve: Vec<_> = pending
                .iter()
                .filter(|l| {
                    l.bm25_confidence
                        .map(|c| c >= min_confidence)
                        .unwrap_or(false)
                })
                .collect();
            if dry_run {
                println!(
                    "Would approve {} suggestions (confidence >= {}):",
                    to_approve.len(),
                    min_confidence
                );
                for entry in &to_approve {
                    println!(
                        "  [{}] {} ({:.2})",
                        &entry.id[..entry.id.len().min(12)],
                        entry.title,
                        entry.bm25_confidence.unwrap_or(0.0)
                    );
                }
                return Ok(());
            }
            let mut approved = 0usize;
            for entry in &to_approve {
                if store.approve(&entry.id).await.is_ok() {
                    let _ = metrics.append(SuggestionMetricsEntry {
                        id: entry.id.clone(),
                        status: Status::Approved,
                        confidence: entry.bm25_confidence.unwrap_or(0.0),
                        timestamp: chrono::Utc::now(),
                        title: entry.title.clone(),
                    });
                    approved += 1;
                }
            }
            println!("Approved {} suggestions.", approved);
            Ok(())
        }
        SuggestSub::RejectAll {
            max_confidence,
            dry_run,
        } => {
            let pending = store
                .list_pending()
                .await
                .map_err(|e| anyhow::anyhow!("{}", e))?;
            let to_reject: Vec<_> = pending
                .iter()
                .filter(|l| {
                    l.bm25_confidence
                        .map(|c| c <= max_confidence)
                        .unwrap_or(true)
                })
                .collect();
            if dry_run {
                println!(
                    "Would reject {} suggestions (confidence <= {}):",
                    to_reject.len(),
                    max_confidence
                );
                for entry in &to_reject {
                    println!(
                        "  [{}] {} ({:.2})",
                        &entry.id[..entry.id.len().min(12)],
                        entry.title,
                        entry.bm25_confidence.unwrap_or(0.0)
                    );
                }
                return Ok(());
            }
            let mut rejected = 0usize;
            for entry in &to_reject {
                if store.reject(&entry.id, None).await.is_ok() {
                    let _ = metrics.append(SuggestionMetricsEntry {
                        id: entry.id.clone(),
                        status: Status::Rejected,
                        confidence: entry.bm25_confidence.unwrap_or(0.0),
                        timestamp: chrono::Utc::now(),
                        title: entry.title.clone(),
                    });
                    rejected += 1;
                }
            }
            println!("Rejected {} suggestions.", rejected);
            Ok(())
        }
        SuggestSub::Metrics => {
            let summary = metrics.summary()?;
            println!("Suggestion Metrics:");
            println!("  Total:   {}", summary.total);
            println!("  Pending: {}", summary.pending);
            println!("  Approved: {}", summary.approved);
            println!("  Rejected: {}", summary.rejected);
            println!("  Approval Rate: {:.1}%", summary.approval_rate * 100.0);
            Ok(())
        }
        SuggestSub::SessionEnd { context } => {
            let pending = store
                .list_pending()
                .await
                .map_err(|e| anyhow::anyhow!("{}", e))?;
            let count = pending.len();
            if count == 0 {
                println!("[suggestions] No pending suggestions.");
                return Ok(());
            }
            let top = if let Some(ref ctx) = context {
                store
                    .suggest(ctx, "session-end", 1)
                    .await
                    .map_err(|e| anyhow::anyhow!("{}", e))
                    .ok()
                    .and_then(|v| v.into_iter().next())
            } else {
                pending.into_iter().next()
            };
            print!("[suggestions] {} suggestion(s) pending", count);
            if let Some(t) = top {
                println!(", top: '{}'", truncate_snippet(&t.title, 60));
            } else {
                println!();
            }
            println!("  Run `terraphim-agent learn suggest list` to review.");
            Ok(())
        }
    }
}

#[cfg(feature = "shared-learning")]
async fn run_shared_learning_command(
    sub: SharedLearningSub,
    config: &learnings::LearningCaptureConfig,
) -> Result<()> {
    use terraphim_agent::shared_learning::{
        SharedLearning, SharedLearningSource, SharedLearningStore, StoreConfig, TrustLevel,
    };

    let store_config = StoreConfig::default();
    let store = SharedLearningStore::open(store_config)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to open shared learning store: {}", e))?;

    match sub {
        SharedLearningSub::List { trust_level, limit } => {
            let learnings = if let Some(ref level_str) = trust_level {
                let level: TrustLevel = level_str.parse().map_err(|e| anyhow::anyhow!("{}", e))?;
                store
                    .list_by_trust_level(level)
                    .await
                    .map_err(|e| anyhow::anyhow!("{}", e))?
            } else {
                store
                    .list_all()
                    .await
                    .map_err(|e| anyhow::anyhow!("{}", e))?
            };

            if learnings.is_empty() {
                println!("No shared learnings found.");
            } else {
                let display_count = limit.min(learnings.len());
                println!(
                    "Shared learnings ({} of {}):",
                    display_count,
                    learnings.len()
                );
                for learning in learnings.iter().take(limit) {
                    println!(
                        "  [{}] {} -- {} ({})",
                        &learning.id[..learning.id.len().min(12)],
                        learning.title,
                        learning.trust_level,
                        learning.source,
                    );
                }
            }
            Ok(())
        }
        SharedLearningSub::Promote { id, to } => {
            let target: TrustLevel = to.parse().map_err(|e| anyhow::anyhow!("{}", e))?;

            match target {
                TrustLevel::L2 => {
                    store
                        .promote_to_l2(&id)
                        .await
                        .map_err(|e| anyhow::anyhow!("{}", e))?;
                    println!("Promoted learning {} to L2 (Peer-Validated).", id);
                }
                TrustLevel::L3 => {
                    store
                        .promote_to_l3(&id)
                        .await
                        .map_err(|e| anyhow::anyhow!("{}", e))?;
                    println!("Promoted learning {} to L3 (Human-Approved).", id);
                }
                TrustLevel::L1 => {
                    return Err(anyhow::anyhow!(
                        "Cannot promote to L1 -- learnings start at L1. Use l2 or l3."
                    ));
                }
                TrustLevel::L0 => {
                    return Err(anyhow::anyhow!(
                        "Cannot promote to L0 -- L0 is for extracted learnings only."
                    ));
                }
            }
            Ok(())
        }
        SharedLearningSub::Import => {
            use crate::learnings::capture::list_learnings;

            let storage_loc = config.storage_location();
            let local_learnings = list_learnings(&storage_loc, usize::MAX).unwrap_or_default();

            if local_learnings.is_empty() {
                println!("No local learnings found to import.");
                return Ok(());
            }

            let mut imported = 0;
            for local in &local_learnings {
                let title = if local.command.len() > 60 {
                    format!("{}...", &local.command[..60])
                } else {
                    local.command.clone()
                };

                let shared = SharedLearning::new(
                    title,
                    local.error_output.clone(),
                    SharedLearningSource::BashHook,
                    "cli-import".to_string(),
                )
                .with_original_command(local.command.clone())
                .with_error_context(local.error_output.clone())
                .with_keywords(local.tags.clone());

                if let Some(ref correction) = local.correction {
                    let shared = shared.with_correction(correction.clone());
                    store
                        .insert(shared)
                        .await
                        .map_err(|e| anyhow::anyhow!("{}", e))?;
                } else {
                    store
                        .insert(shared)
                        .await
                        .map_err(|e| anyhow::anyhow!("{}", e))?;
                }
                imported += 1;
            }

            println!(
                "Imported {} local learning(s) into shared store at L1.",
                imported
            );
            Ok(())
        }
        SharedLearningSub::Stats => {
            let all = store
                .list_all()
                .await
                .map_err(|e| anyhow::anyhow!("{}", e))?;

            let l1_count = all
                .iter()
                .filter(|l| l.trust_level == TrustLevel::L1)
                .count();
            let l2_count = all
                .iter()
                .filter(|l| l.trust_level == TrustLevel::L2)
                .count();
            let l3_count = all
                .iter()
                .filter(|l| l.trust_level == TrustLevel::L3)
                .count();

            println!("Shared Learning Statistics:");
            println!("  Total: {}", all.len());
            println!("  L1 (Unverified):      {}", l1_count);
            println!("  L2 (Peer-Validated):  {}", l2_count);
            println!("  L3 (Human-Approved):  {}", l3_count);

            if !all.is_empty() {
                let total_applied: u32 = all.iter().map(|l| l.quality.applied_count).sum();
                let total_effective: u32 = all.iter().map(|l| l.quality.effective_count).sum();
                let avg_success = if total_applied > 0 {
                    (total_effective as f64 / total_applied as f64) * 100.0
                } else {
                    0.0
                };
                println!("  Avg success rate:     {:.1}%", avg_success);
            }
            Ok(())
        }
        #[cfg(feature = "cross-agent-injection")]
        SharedLearningSub::Inject { min_trust, dry_run } => {
            use terraphim_agent::shared_learning::TrustLevel;
            use terraphim_agent::shared_learning::injector::{InjectorConfig, LearningInjector};

            let trust_level = min_trust
                .to_uppercase()
                .parse::<TrustLevel>()
                .unwrap_or(TrustLevel::L2);

            let config = InjectorConfig::default().with_min_trust_level(trust_level);
            let injector = LearningInjector::new(config);

            let result = injector.run_injection().await?;

            if dry_run {
                println!("Dry run - would inject {} learnings:", result.injected);
                for id in &result.injected_ids {
                    println!("  - {}", id);
                }
            } else {
                println!(
                    "Injection complete: {} injected, {} skipped (trust), {} skipped (context), {} skipped (exists)",
                    result.injected,
                    result.skipped_trust,
                    result.skipped_context,
                    result.skipped_exists
                );
            }
            Ok(())
        }
    }
}

#[cfg(feature = "server")]
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
            fail_on_empty: _,
        } => {
            // Get selected role from server if not specified
            let role_name = if let Some(role) = role {
                api.resolve_role(&role).await?
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
                    layer: Layer::default(),
                    include_pinned: false,
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
                    layer: Layer::default(),
                    include_pinned: false,
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
                use crate::robot::schema::{SearchResultItem, SearchResultsData};
                use crate::robot::{ResponseMeta, RobotConfig, RobotFormatter, RobotResponse};
                use std::time::Instant;

                let start = Instant::now();
                let robot_format = match output.mode {
                    CommandOutputMode::JsonCompact => crate::robot::output::OutputFormat::Minimal,
                    _ => crate::robot::output::OutputFormat::Json,
                };
                let mut robot_config = RobotConfig::new()
                    .with_format(robot_format)
                    .with_max_results(limit);
                if output.robot {
                    robot_config = robot_config
                        .with_max_content_length(2000)
                        .with_max_tokens(8000);
                }

                let formatter = RobotFormatter::new(robot_config.clone());
                let max_results = robot_config.max_results.unwrap_or(limit);
                let truncated_results: Vec<_> = res.results.into_iter().take(max_results).collect();
                let total = truncated_results.len();

                let items: Vec<SearchResultItem> = truncated_results
                    .iter()
                    .enumerate()
                    .map(|(i, doc)| {
                        let preview = doc.description.as_deref().or(if doc.body.is_empty() {
                            None
                        } else {
                            Some(doc.body.as_str())
                        });
                        let (preview_text, preview_truncated) = match preview {
                            Some(text) => {
                                let (t, was_truncated) = formatter.truncate_content(text.trim());
                                (Some(t), was_truncated)
                            }
                            None => (None, false),
                        };
                        SearchResultItem {
                            rank: i + 1,
                            id: doc.id.clone(),
                            title: doc.title.clone(),
                            url: if doc.url.is_empty() {
                                None
                            } else {
                                Some(doc.url.clone())
                            },
                            score: doc.rank.unwrap_or_default() as f64,
                            preview: preview_text,
                            source: None,
                            date: None,
                            preview_truncated,
                        }
                    })
                    .collect();

                let data = SearchResultsData {
                    results: items,
                    total_matches: total,
                    concepts_matched: vec![],
                    wildcard_fallback: false,
                };

                let meta =
                    ResponseMeta::new("search").with_elapsed(start.elapsed().as_millis() as u64);
                let response = RobotResponse::success(data, meta);
                let output_str = formatter.format(&response)?;
                println!("{}", output_str);
            } else {
                for doc in res.results.iter() {
                    let snippet = doc
                        .description
                        .as_deref()
                        .or(if doc.body.is_empty() {
                            None
                        } else {
                            Some(doc.body.as_str())
                        })
                        .map(|s| truncate_snippet(s.trim(), 120));
                    println!("[{}] {}", doc.rank.unwrap_or_default(), doc.title);
                    if !doc.url.is_empty() {
                        println!("    {}", doc.url);
                    }
                    if let Some(snip) = snippet {
                        println!("    {}", snip);
                    }
                    println!();
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
                    // Try to find role by name or shortname via get_config for
                    // case-insensitive convenience. If the server's /config
                    // endpoint is locked (e.g. background KG indexing holds
                    // the config lock during search/extract), fall back to
                    // the user's input as-is and let the server validate. The
                    // server's update_selected_role does its own contains_key
                    // check and returns a clean "Role not found" error on
                    // miss, so we preserve correctness either way.
                    let role_name = match api.get_config().await {
                        Ok(cfg) => {
                            let query_lower = name.to_lowercase();
                            cfg.config
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
                                })?
                        }
                        Err(e) => {
                            log::warn!(
                                "get_config failed during roles select ({}); \
                                 falling back to user-supplied name verbatim",
                                e
                            );
                            name.to_string()
                        }
                    };
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
            #[allow(clippy::unnecessary_sort_by)]
            nodes_sorted.sort_by(|a, b| b.rank.cmp(&a.rank));
            for node in nodes_sorted.into_iter().take(top_k) {
                println!("{}", node.label);
            }
            Ok(())
        }
        #[cfg(feature = "llm")]
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
                    1u64, // Simple ID for CLI usage
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
            use session_output::*;
            use terraphim_sessions::SessionService;

            let rt = Runtime::new()?;
            rt.block_on(async {
                let service = SessionService::new();

                match sub {
                    SessionsSub::Sources => {
                        let sources = service.detect_sources();
                        if output.is_machine_readable() {
                            let payload = SourcesOutput {
                                count: sources.len(),
                                sources: sources
                                    .into_iter()
                                    .map(|s| {
                                        let available = s.is_available();
                                        SourceEntry {
                                            id: s.id,
                                            name: s.name,
                                            available,
                                        }
                                    })
                                    .collect(),
                            };
                            print_json_output(&payload, output.mode)?;
                        } else if sources.is_empty() {
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
                        if output.is_machine_readable() {
                            let session_entries: Vec<SessionEntry> = sessions
                                .iter()
                                .take(limit)
                                .map(|s| SessionEntry {
                                    id: s.id.to_string(),
                                    title: s.title.clone(),
                                    message_count: s.message_count(),
                                    source: s.source.clone(),
                                })
                                .collect();
                            let shown = session_entries.len();
                            let payload = SessionListOutput {
                                total: sessions.len(),
                                shown,
                                sessions: session_entries,
                            };
                            print_json_output(&payload, output.mode)?;
                        } else if sessions.is_empty() {
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
                        if output.is_machine_readable() {
                            let entries: Vec<SessionSearchEntry> = results
                                .iter()
                                .take(limit)
                                .map(|s| {
                                    let preview = s
                                        .messages
                                        .iter()
                                        .find(|msg| {
                                            msg.content
                                                .to_lowercase()
                                                .contains(&query.to_lowercase())
                                        })
                                        .map(|msg| {
                                            let p: String = msg.content.chars().take(100).collect();
                                            p
                                        });
                                    SessionSearchEntry {
                                        id: s.id.to_string(),
                                        title: s.title.clone(),
                                        message_count: s.message_count(),
                                        preview,
                                    }
                                })
                                .collect();
                            let shown = entries.len();
                            let payload = SessionSearchOutput {
                                query: query.clone(),
                                total: results.len(),
                                shown,
                                sessions: entries,
                            };
                            print_json_output(&payload, output.mode)?;
                            if results.is_empty() {
                                std::process::exit(
                                    robot::exit_codes::ExitCode::ErrorNotFound.code().into(),
                                );
                            }
                        } else if results.is_empty() {
                            println!("No sessions matching '{}'.", query);
                        } else {
                            println!("Found {} matching sessions:", results.len());
                            for session in results.iter().take(limit) {
                                let title = session.title.as_deref().unwrap_or("(untitled)");
                                println!("  - {}", title);
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
                        if output.is_machine_readable() {
                            let payload = SessionStatsOutput {
                                total_sessions: stats.total_sessions,
                                total_messages: stats.total_messages,
                                total_user_messages: stats.total_user_messages,
                                total_assistant_messages: stats.total_assistant_messages,
                                by_source: stats.sessions_by_source,
                            };
                            print_json_output(&payload, output.mode)?;
                        } else {
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
                        }
                        Ok(())
                    }
                }
            })
        }
        Command::Listen { .. } => {
            eprintln!("error: listen mode is not available in server mode");
            eprintln!("The listener runs in offline mode only.");
            std::process::exit(1);
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

#[allow(unused_variables)]
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
    let rt = tokio::runtime::Runtime::new()?;

    #[cfg(feature = "server")]
    let backend = {
        let effective_url = resolve_tui_server_url(server_url.as_deref());
        let api = ApiClient::new(effective_url.clone());
        ensure_tui_server_reachable(&rt, &api, &effective_url)?;
        crate::tui_backend::TuiBackend::Remote(api)
    };

    #[cfg(not(feature = "server"))]
    let backend = {
        let service = rt.block_on(async { TuiService::new(None).await })?;
        crate::tui_backend::TuiBackend::Local(service)
    };

    // Initialize terms from rolegraph (selected role)
    if let Ok(cfg) = rt.block_on(async { backend.get_config().await }) {
        current_role = cfg.selected_role.to_string();
        if let Ok(rg) = rt.block_on(async { backend.get_rolegraph_terms(&current_role).await }) {
            terms = rg;
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
                    ViewMode::Search => match map_search_key_event(key) {
                        TuiAction::Quit => break,
                        TuiAction::SearchOrOpen => {
                            let query = input.trim().to_string();
                            let backend = backend.clone();
                            let role = current_role.clone();
                            if !query.is_empty() {
                                if let Ok(docs) = rt.block_on(async move {
                                    let q = SearchQuery {
                                        search_term: NormalizedTermValue::from(query.as_str()),
                                        search_terms: None,
                                        operator: None,
                                        skip: Some(0),
                                        limit: Some(10),
                                        role: Some(RoleName::new(&role)),
                                        layer: Layer::default(),
                                        include_pinned: false,
                                    };
                                    backend.search(&q).await
                                }) {
                                    let lines: Vec<String> = docs
                                        .iter()
                                        .map(|d| {
                                            format!("{} {}", d.rank.unwrap_or_default(), d.title)
                                        })
                                        .collect();
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
                            let query = input.trim();
                            if !query.is_empty() {
                                let backend = backend.clone();
                                let role = current_role.clone();
                                if let Ok(autocomplete_resp) =
                                    rt.block_on(
                                        async move { backend.autocomplete(&role, query).await },
                                    )
                                {
                                    suggestions = autocomplete_resp.into_iter().take(5).collect();
                                }
                            }
                        }
                        TuiAction::SwitchRole => {
                            let backend = backend.clone();
                            if let Ok(cfg) = rt.block_on(async { backend.get_config().await }) {
                                let roles: Vec<String> =
                                    cfg.roles.keys().map(|k| k.to_string()).collect();
                                if !roles.is_empty() {
                                    if let Some(current_idx) =
                                        roles.iter().position(|r| r == &current_role)
                                    {
                                        let next_idx = (current_idx + 1) % roles.len();
                                        current_role = roles[next_idx].clone();
                                        if let Ok(rg) = rt.block_on(async {
                                            backend.get_rolegraph_terms(&current_role).await
                                        }) {
                                            terms = rg;
                                        }
                                    }
                                }
                            }
                        }
                        TuiAction::SummarizeSelection => {
                            #[cfg(feature = "llm")]
                            {
                                if selected_result_index < detailed_results.len() {
                                    let doc = detailed_results[selected_result_index].clone();
                                    let backend = backend.clone();
                                    let role = current_role.clone();
                                    if let Ok(Some(summary_text)) = rt.block_on(async move {
                                        backend.summarize(&doc, Some(&role)).await
                                    }) {
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
                        TuiAction::None | TuiAction::BackToSearch | TuiAction::SummarizeDetail => {}
                    },
                    ViewMode::ResultDetail => match map_detail_key_event(key) {
                        TuiAction::BackToSearch => {
                            view_mode = ViewMode::Search;
                        }
                        TuiAction::SummarizeDetail => {
                            #[cfg(feature = "llm")]
                            {
                                if selected_result_index < detailed_results.len() {
                                    let doc = detailed_results[selected_result_index].clone();
                                    let backend = backend.clone();
                                    let role = current_role.clone();
                                    if let Ok(Some(summary_text)) = rt.block_on(async move {
                                        backend.summarize(&doc, Some(&role)).await
                                    }) {
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
                    },
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
