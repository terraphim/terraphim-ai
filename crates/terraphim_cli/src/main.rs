//! Terraphim CLI - Automation-friendly semantic knowledge graph search
//!
//! A non-interactive command-line tool for scripting and automation.
//! Outputs JSON for easy parsing and integration with other tools.

use anyhow::{Context, Result};
use clap::{CommandFactory, Parser, Subcommand};
use clap_complete::{Shell, generate};
use serde::Serialize;
use std::io;

mod service;
use service::CliService;

/// Terraphim CLI - Semantic knowledge graph search for automation
#[derive(Parser)]
#[command(name = "terraphim-cli")]
#[command(version, about, long_about = None)]
#[command(arg_required_else_help = true)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    /// Output format
    #[arg(long, global = true, default_value = "json")]
    format: OutputFormat,

    /// Suppress non-JSON output (errors, warnings)
    #[arg(long, global = true)]
    quiet: bool,
}

#[derive(Debug, Clone, clap::ValueEnum)]
enum OutputFormat {
    Json,
    JsonPretty,
    Text,
}

/// Replace mode for the replace command
#[derive(Debug, Clone, clap::ValueEnum, Default)]
enum ReplaceMode {
    /// Add links to matched terms (default, current behavior)
    #[default]
    Link,
    /// Replace with synonyms from knowledge graph (matches terraphim-agent)
    Synonym,
}

/// Roles subcommands
#[derive(Subcommand)]
enum RolesSub {
    /// List available roles
    List,
    /// Select a role by name or shortname
    Select {
        /// Role name or shortname to select
        name: String,
    },
}

#[derive(Subcommand)]
enum Commands {
    /// Search for documents
    Search {
        /// Search query
        query: String,

        /// Role to use for search
        #[arg(long)]
        role: Option<String>,

        /// Maximum number of results
        #[arg(long, short = 'n')]
        limit: Option<usize>,
    },

    /// Show configuration
    Config,

    /// Manage roles
    Roles {
        #[command(subcommand)]
        sub: RolesSub,
    },

    /// Show top concepts from knowledge graph
    Graph {
        /// Number of top concepts to show
        #[arg(long, short = 'k', default_value = "10")]
        top_k: usize,

        /// Role to use
        #[arg(long)]
        role: Option<String>,
    },

    /// Replace matched terms with links or synonyms
    Replace {
        /// Text to process
        text: String,

        /// Replace mode: link (add links) or synonym (knowledge graph replacement)
        #[arg(long, default_value = "link")]
        mode: ReplaceMode,

        /// Link format: markdown, html, wiki, plain (only for --mode link)
        #[arg(long = "link-format", default_value = "markdown")]
        link_format: String,

        /// Role to use
        #[arg(long)]
        role: Option<String>,
    },

    /// Find matched terms in text
    Find {
        /// Text to search in
        text: String,

        /// Role to use
        #[arg(long)]
        role: Option<String>,
    },

    /// Show thesaurus terms
    Thesaurus {
        /// Role to use
        #[arg(long)]
        role: Option<String>,

        /// Maximum number of terms to show
        #[arg(long, default_value = "50")]
        limit: usize,
    },

    /// Generate shell completions
    Completions {
        /// Shell to generate completions for
        shell: Shell,
    },

    /// Check for available updates
    CheckUpdate,

    /// Update to latest version if available
    Update,

    /// Rollback to a previous version
    Rollback {
        /// Version to rollback to
        version: String,
    },
}

#[derive(Serialize)]
struct SearchResult {
    query: String,
    role: String,
    results: Vec<DocumentResult>,
    count: usize,
}

#[derive(Serialize)]
struct DocumentResult {
    id: String,
    title: String,
    url: String,
    rank: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    body: Option<String>,
}

#[derive(Serialize)]
struct ConfigResult {
    selected_role: String,
    roles: Vec<String>,
}

#[derive(Serialize)]
struct GraphResult {
    role: String,
    top_k: usize,
    concepts: Vec<String>,
}

#[derive(Serialize)]
struct ReplaceResult {
    original: String,
    replaced: String,
    format: String,
    mode: String,
}

#[derive(Serialize)]
struct SynonymReplaceResult {
    original: String,
    replaced: String,
    replacements: usize,
    changed: bool,
    mode: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
}

#[derive(Serialize)]
struct FindResult {
    text: String,
    matches: Vec<MatchResult>,
    count: usize,
}

#[derive(Serialize)]
struct MatchResult {
    term: String,
    position: Option<(usize, usize)>,
    normalized: String,
}

#[derive(Serialize)]
struct ThesaurusResult {
    role: String,
    name: String,
    terms: Vec<ThesaurusTerm>,
    total_count: usize,
    shown_count: usize,
}

#[derive(Serialize)]
struct ThesaurusTerm {
    id: u64,
    term: String,
    normalized: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    url: Option<String>,
}

#[derive(Serialize)]
struct ErrorResult {
    error: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    details: Option<String>,
}

#[derive(Serialize)]
struct RolesListResult {
    roles: Vec<RoleInfo>,
    selected: String,
}

#[derive(Serialize)]
struct RoleInfo {
    name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    shortname: Option<String>,
    selected: bool,
}

#[derive(Serialize)]
struct RoleSelectResult {
    selected: String,
    previous: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // Handle completions command specially (doesn't need service)
    if let Some(Commands::Completions { shell }) = &cli.command {
        let mut cmd = Cli::command();
        generate(
            shell.to_owned(),
            &mut cmd,
            "terraphim-cli",
            &mut io::stdout(),
        );
        return Ok(());
    }

    // Initialize service for all other commands
    let service = CliService::new()
        .await
        .context("Failed to initialize service")?;

    // Execute command
    let result = match cli.command {
        Some(Commands::Search { query, role, limit }) => {
            handle_search(&service, query, role, limit).await
        }
        Some(Commands::Config) => handle_config(&service).await,
        Some(Commands::Roles { sub }) => match sub {
            RolesSub::List => handle_roles_list(&service).await,
            RolesSub::Select { name } => handle_roles_select(&service, name).await,
        },
        Some(Commands::Graph { top_k, role }) => handle_graph(&service, top_k, role).await,
        Some(Commands::Replace {
            text,
            mode,
            link_format,
            role,
        }) => handle_replace(&service, text, mode, link_format, role).await,
        Some(Commands::Find { text, role }) => handle_find(&service, text, role).await,
        Some(Commands::Thesaurus { role, limit }) => handle_thesaurus(&service, role, limit).await,
        Some(Commands::CheckUpdate) => handle_check_update().await,
        Some(Commands::Update) => handle_update().await,
        Some(Commands::Rollback { version }) => handle_rollback(&version).await,
        Some(Commands::Completions { .. }) => unreachable!(), // Handled above
        None => {
            eprintln!("No command specified. Use --help for usage information.");
            std::process::exit(1);
        }
    };

    // Output result
    match result {
        Ok(output) => {
            let formatted = match cli.format {
                OutputFormat::Json => serde_json::to_string(&output)?,
                OutputFormat::JsonPretty => serde_json::to_string_pretty(&output)?,
                OutputFormat::Text => format_as_text(&output)
                    .unwrap_or_else(|_| serde_json::to_string(&output).unwrap()),
            };
            println!("{}", formatted);
            Ok(())
        }
        Err(e) => {
            let error_result = ErrorResult {
                error: e.to_string(),
                details: e.source().map(|s| s.to_string()),
            };

            if !cli.quiet {
                eprintln!("Error: {}", e);
            }

            let formatted = match cli.format {
                OutputFormat::Json => serde_json::to_string(&error_result)?,
                OutputFormat::JsonPretty => serde_json::to_string_pretty(&error_result)?,
                OutputFormat::Text => e.to_string(),
            };
            println!("{}", formatted);
            std::process::exit(1);
        }
    }
}

async fn handle_search(
    service: &CliService,
    query: String,
    role: Option<String>,
    limit: Option<usize>,
) -> Result<serde_json::Value> {
    let role_name = if let Some(role) = role {
        terraphim_types::RoleName::new(&role)
    } else {
        service.get_selected_role().await
    };

    let documents = service.search(&query, &role_name, limit).await?;

    let results: Vec<DocumentResult> = documents
        .iter()
        .map(|doc| DocumentResult {
            id: doc.id.clone(),
            title: doc.title.clone(),
            url: doc.url.clone(),
            rank: doc.rank.map(|r| r as f64),
            body: None, // Don't include full body in CLI output
        })
        .collect();

    let result = SearchResult {
        query,
        role: role_name.to_string(),
        results,
        count: documents.len(),
    };

    Ok(serde_json::to_value(result)?)
}

async fn handle_config(service: &CliService) -> Result<serde_json::Value> {
    let config = service.get_config().await;
    let roles = service.list_roles().await;

    let result = ConfigResult {
        selected_role: config.selected_role.to_string(),
        roles,
    };

    Ok(serde_json::to_value(result)?)
}

async fn handle_roles_list(service: &CliService) -> Result<serde_json::Value> {
    let roles_with_info = service.list_roles_with_info().await;
    let selected_role = service.get_selected_role().await;

    let roles: Vec<RoleInfo> = roles_with_info
        .into_iter()
        .map(|(name, shortname)| RoleInfo {
            selected: name == selected_role.to_string(),
            name,
            shortname,
        })
        .collect();

    let result = RolesListResult {
        roles,
        selected: selected_role.to_string(),
    };
    Ok(serde_json::to_value(result)?)
}

async fn handle_roles_select(service: &CliService, name: String) -> Result<serde_json::Value> {
    let previous = service.get_selected_role().await.to_string();

    // Find role by name or shortname
    let role_name = service
        .find_role_by_name_or_shortname(&name)
        .await
        .ok_or_else(|| anyhow::anyhow!("Role '{}' not found (checked name and shortname)", name))?;

    service.update_selected_role(role_name.clone()).await?;
    service.save_config().await?;

    let result = RoleSelectResult {
        selected: role_name.to_string(),
        previous,
    };
    Ok(serde_json::to_value(result)?)
}

async fn handle_graph(
    service: &CliService,
    top_k: usize,
    role: Option<String>,
) -> Result<serde_json::Value> {
    let role_name = if let Some(role) = role {
        terraphim_types::RoleName::new(&role)
    } else {
        service.get_selected_role().await
    };

    let concepts = service.get_top_concepts(&role_name, top_k).await?;

    let result = GraphResult {
        role: role_name.to_string(),
        top_k,
        concepts,
    };

    Ok(serde_json::to_value(result)?)
}

async fn handle_replace(
    service: &CliService,
    text: String,
    mode: ReplaceMode,
    format: String,
    role: Option<String>,
) -> Result<serde_json::Value> {
    let role_name = if let Some(role) = role {
        terraphim_types::RoleName::new(&role)
    } else {
        service.get_selected_role().await
    };

    match mode {
        ReplaceMode::Link => {
            // Existing link replacement logic
            let link_type = match format.as_str() {
                "markdown" => terraphim_automata::LinkType::MarkdownLinks,
                "html" => terraphim_automata::LinkType::HTMLLinks,
                "wiki" => terraphim_automata::LinkType::WikiLinks,
                "plain" => {
                    let result = ReplaceResult {
                        original: text.clone(),
                        replaced: text,
                        format: "plain".to_string(),
                        mode: "link".to_string(),
                    };
                    return Ok(serde_json::to_value(result)?);
                }
                _ => {
                    anyhow::bail!(
                        "Unknown format: {}. Use: markdown, html, wiki, or plain",
                        format
                    );
                }
            };

            let replaced = service
                .replace_matches(&role_name, &text, link_type)
                .await?;

            let result = ReplaceResult {
                original: text,
                replaced,
                format,
                mode: "link".to_string(),
            };

            Ok(serde_json::to_value(result)?)
        }
        ReplaceMode::Synonym => {
            // New synonym replacement (matches terraphim-agent)
            let thesaurus = service.get_thesaurus(&role_name).await?;
            let replacement_service = terraphim_hooks::ReplacementService::new(thesaurus);
            let hook_result = replacement_service.replace_fail_open(&text);

            let result = SynonymReplaceResult {
                original: hook_result.original,
                replaced: hook_result.result,
                replacements: hook_result.replacements,
                changed: hook_result.changed,
                mode: "synonym".to_string(),
                error: hook_result.error,
            };

            Ok(serde_json::to_value(result)?)
        }
    }
}

async fn handle_find(
    service: &CliService,
    text: String,
    role: Option<String>,
) -> Result<serde_json::Value> {
    let role_name = if let Some(role) = role {
        terraphim_types::RoleName::new(&role)
    } else {
        service.get_selected_role().await
    };

    let matches = service.find_matches(&role_name, &text).await?;

    let match_results: Vec<MatchResult> = matches
        .iter()
        .map(|m| MatchResult {
            term: m.term.clone(),
            position: m.pos,
            normalized: m.normalized_term.value.to_string(),
        })
        .collect();

    let result = FindResult {
        text,
        matches: match_results,
        count: matches.len(),
    };

    Ok(serde_json::to_value(result)?)
}

async fn handle_thesaurus(
    service: &CliService,
    role: Option<String>,
    limit: usize,
) -> Result<serde_json::Value> {
    let role_name = if let Some(role) = role {
        terraphim_types::RoleName::new(&role)
    } else {
        service.get_selected_role().await
    };

    let thesaurus = service.get_thesaurus(&role_name).await?;

    let mut entries: Vec<_> = thesaurus.into_iter().collect();
    entries.sort_by_key(|(_, term)| term.id);

    let total_count = entries.len();
    let terms: Vec<ThesaurusTerm> = entries
        .iter()
        .take(limit)
        .map(|(key, term)| ThesaurusTerm {
            id: term.id,
            term: key.to_string(),
            normalized: term.value.to_string(),
            url: term.url.clone(),
        })
        .collect();

    let shown_count = terms.len();
    let result = ThesaurusResult {
        role: role_name.to_string(),
        name: thesaurus.name().to_string(),
        terms,
        total_count,
        shown_count,
    };

    Ok(serde_json::to_value(result)?)
}

async fn handle_check_update() -> Result<serde_json::Value> {
    let current_version = env!("CARGO_PKG_VERSION");
    let bin_name = "terraphim-cli";

    let status = terraphim_update::check_for_updates_auto(bin_name, current_version).await?;

    match status {
        terraphim_update::UpdateStatus::Available {
            ref current_version,
            ref latest_version,
        } => {
            let result = serde_json::json!({
                "update_available": true,
                "current_version": current_version,
                "latest_version": latest_version,
                "message": status.to_string(),
            });
            Ok(result)
        }
        terraphim_update::UpdateStatus::UpToDate(ref version) => {
            let result = serde_json::json!({
                "update_available": false,
                "current_version": version,
                "message": status.to_string(),
            });
            Ok(result)
        }
        terraphim_update::UpdateStatus::Failed(ref error) => {
            let result = serde_json::json!({
                "update_available": false,
                "error": error,
                "message": status.to_string(),
            });
            Ok(result)
        }
        terraphim_update::UpdateStatus::Updated { .. } => {
            let result = serde_json::json!({
                "update_available": false,
                "message": status.to_string(),
            });
            Ok(result)
        }
    }
}

async fn handle_update() -> Result<serde_json::Value> {
    let bin_name = "terraphim-cli";

    let status = terraphim_update::update_binary(bin_name).await?;

    match status {
        terraphim_update::UpdateStatus::Updated {
            ref from_version,
            ref to_version,
        } => {
            let result = serde_json::json!({
                "updated": true,
                "from_version": from_version,
                "to_version": to_version,
                "message": status.to_string(),
            });
            Ok(result)
        }
        terraphim_update::UpdateStatus::UpToDate(ref version) => {
            let result = serde_json::json!({
                "updated": false,
                "current_version": version,
                "message": status.to_string(),
            });
            Ok(result)
        }
        terraphim_update::UpdateStatus::Available {
            ref current_version,
            ref latest_version,
        } => {
            let result = serde_json::json!({
                "updated": false,
                "current_version": current_version,
                "latest_version": latest_version,
                "message": status.to_string(),
            });
            Ok(result)
        }
        terraphim_update::UpdateStatus::Failed(ref error) => {
            let result = serde_json::json!({
                "updated": false,
                "error": error,
                "message": status.to_string(),
            });
            Ok(result)
        }
    }
}

async fn handle_rollback(version: &str) -> Result<serde_json::Value> {
    let bin_name = "terraphim-cli";
    let current_exe = std::env::current_exe()?;
    let backup_path = current_exe.with_extension(format!("bak-{}", version));

    terraphim_update::rollback(&backup_path, &current_exe)?;

    let result = serde_json::json!({
        "rolled_back": true,
        "version": version,
        "message": format!("Successfully rolled back {} to version {}", bin_name, version),
    });
    Ok(result)
}

/// Format JSON as human-readable text (for --format text)
fn format_as_text(value: &serde_json::Value) -> Result<String> {
    // This is a simplified text formatter
    // Could be enhanced with better formatting
    Ok(format!("{:#}", value))
}
