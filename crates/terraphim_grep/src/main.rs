use std::path::PathBuf;
use std::sync::Arc;

use anyhow::{Context, Result};
use clap::{Parser, ValueEnum};
use terraphim_automata::AutomataPath;
use terraphim_grep::{
    GrepOptions, GrepResult, Haystack, HybridSearcher, SufficiencyJudge, TerraphimGrep,
};
use tracing_subscriber::{EnvFilter, fmt, prelude::*};

#[derive(Parser, Debug)]
#[command(name = "terraphim-grep")]
#[command(
    version,
    about = "Intelligent hybrid grep with RLM fallback and KG curation"
)]
struct Args {
    #[arg(help = "Search query")]
    query: String,

    #[arg(
        short = 'C',
        long,
        default_value = "0",
        help = "Context lines before/after match"
    )]
    context: usize,

    #[arg(
        short = 'n',
        long,
        default_value = "50",
        help = "Maximum number of results"
    )]
    max_results: usize,

    #[arg(
        short = 'H',
        long,
        value_enum,
        default_value = "all",
        help = "Haystack to search"
    )]
    haystack: HaystackArg,

    #[arg(long, help = "Force RLM fallback for all queries")]
    force_rlm: bool,

    #[arg(long, help = "Include LLM-generated answer")]
    answer: bool,

    #[arg(long, help = "Output JSON format")]
    json: bool,

    #[arg(long, help = "Search paths (default: current directory)")]
    paths: Vec<PathBuf>,

    #[arg(long, help = "Role to use for search")]
    role: Option<String>,

    #[arg(long, help = "Thesaurus path")]
    thesaurus: Option<PathBuf>,

    #[arg(
        long,
        help = "Path to a JSON file containing a terraphim_config::Role with LLM/router settings"
    )]
    role_config: Option<PathBuf>,
}

#[derive(Debug, Clone, ValueEnum)]
enum HaystackArg {
    All,
    Code,
    Docs,
}

#[allow(clippy::derivable_impls)]
impl Default for HaystackArg {
    fn default() -> Self {
        HaystackArg::All
    }
}

impl From<HaystackArg> for Haystack {
    fn from(arg: HaystackArg) -> Self {
        match arg {
            HaystackArg::All => Haystack::All,
            HaystackArg::Code => Haystack::Code,
            HaystackArg::Docs => Haystack::Docs,
        }
    }
}

impl From<Haystack> for HaystackArg {
    fn from(arg: Haystack) -> Self {
        match arg {
            Haystack::All => HaystackArg::All,
            Haystack::Code => HaystackArg::Code,
            Haystack::Docs => HaystackArg::Docs,
        }
    }
}

fn init_tracing() {
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info,terraphim_grep=debug"));

    tracing_subscriber::registry()
        .with(fmt::layer().with_writer(std::io::stderr))
        .with(filter)
        .init();
}

/// Discover project-level config from `.terraphim/` directory.
///
/// Returns the `.terraphim/` path if found, enabling auto-discovery of
/// thesaurus, role config, and KG path without CLI flags.
fn discover_project_dir() -> Option<std::path::PathBuf> {
    terraphim_config::project::discover(None).ok().flatten()
}

fn load_project_config() -> Option<(PathBuf, terraphim_config::project::ProjectConfig)> {
    let dir = discover_project_dir()?;
    let config = terraphim_config::project::ProjectConfig::load_from_dir(&dir).ok()?;
    Some((dir, config))
}

fn resolve_role_name(
    explicit_role: Option<&str>,
    project_config: Option<&terraphim_config::project::ProjectConfig>,
) -> Result<String> {
    if let Some(config) = project_config {
        if let Some(role) = config.resolve_role_name(explicit_role)? {
            return Ok(role);
        }
    }

    Ok(explicit_role.unwrap_or("default").to_string())
}

/// Find thesaurus path with project config priority.
///
/// Resolution order:
///   1. `.terraphim/thesaurus-<role>.json` (project config)
///   2. `*_thesaurus.json` in CWD or nearby directories (filesystem heuristic)
fn find_default_thesaurus(role_name: &str) -> Option<PathBuf> {
    if let Some(dir) = discover_project_dir() {
        if let Some(path) = terraphim_config::project::discover_thesaurus(&dir, role_name) {
            tracing::info!("Using project thesaurus: {:?}", path);
            return Some(path);
        }
    }

    let possible_paths = vec![
        PathBuf::from("."),
        PathBuf::from("../docs/src"),
        PathBuf::from("../../docs/src"),
    ];

    for base in possible_paths {
        if let Ok(cwd) = std::env::current_dir() {
            let candidate = cwd.join(&base);
            if candidate.exists() {
                if let Ok(entries) = std::fs::read_dir(&candidate) {
                    for entry in entries.flatten() {
                        let name = entry.file_name();
                        let name_str = name.to_string_lossy();
                        if name_str.ends_with("_thesaurus.json") {
                            return Some(candidate.join(&name));
                        }
                    }
                }
            }
        }
    }
    None
}

/// Build an `LlmClient` for the requested role.
///
/// Resolution order:
///   1. If `--role-config <path>` is provided, deserialize a `terraphim_config::Role` from
///      that JSON file and feed it to `terraphim_service::llm::build_llm_from_role`.
///   2. Otherwise construct a minimal in-memory `Role` populated from environment variables
///      (`OPENROUTER_API_KEY`, `OPENROUTER_MODEL`, `OLLAMA_BASE_URL`, `OLLAMA_MODEL`).
///   3. Return `None` if neither source yields a usable LLM client. The grep stays usable in
///      search-only mode -- the LLM is only required when sufficiency falls below threshold.
///
/// `terraphim_service::llm::build_llm_from_role` is the public entry point: it owns the
/// precedence rules and decides internally whether to return a direct provider (Ollama /
/// OpenRouter / GenAi) or a `RouterBridgeLlmClient` based on `role.llm_router_enabled`.
/// Wiring this function rather than the bridge directly keeps grep aligned with how the
/// server, TUI, and RLM consume LLM clients.
#[cfg(feature = "llm")]
fn build_llm_for_role(
    role_name: &str,
    role_config_path: Option<&std::path::Path>,
) -> Option<Arc<dyn terraphim_service::llm::LlmClient>> {
    let role = match role_config_path {
        Some(path) => match std::fs::read_to_string(path) {
            Ok(contents) => match serde_json::from_str::<terraphim_config::Role>(&contents) {
                Ok(r) => r,
                Err(e) => {
                    tracing::warn!("Failed to parse role config at {:?}: {}", path, e);
                    return None;
                }
            },
            Err(e) => {
                tracing::warn!("Failed to read role config at {:?}: {}", path, e);
                return None;
            }
        },
        None => {
            // Try project config (.terraphim/role-<name>.json) before env vars
            if let Some(dir) = discover_project_dir() {
                let role_file = dir.join(format!("role-{}.json", role_name));
                if role_file.is_file() {
                    tracing::info!("Using project role config: {:?}", role_file);
                    if let Ok(contents) = std::fs::read_to_string(&role_file) {
                        if let Ok(r) = serde_json::from_str::<terraphim_config::Role>(&contents) {
                            return terraphim_service::llm::build_llm_from_role(&r);
                        }
                    }
                }
            }
            role_from_env(role_name)?
        }
    };

    terraphim_service::llm::build_llm_from_role(&role)
}

#[cfg(not(feature = "llm"))]
#[allow(dead_code)]
fn build_llm_for_role(
    _role_name: &str,
    _role_config_path: Option<&std::path::Path>,
) -> Option<std::sync::Arc<()>> {
    None
}

/// Construct a minimal `Role` from environment variables. Returns `None` when no LLM
/// credentials are visible -- the CLI then runs in search-only mode.
#[cfg(feature = "llm")]
fn role_from_env(role_name: &str) -> Option<terraphim_config::Role> {
    use serde_json::Value;

    let openrouter_key = std::env::var("OPENROUTER_API_KEY")
        .ok()
        .filter(|s| !s.is_empty());
    let ollama_url = std::env::var("OLLAMA_BASE_URL")
        .ok()
        .filter(|s| !s.is_empty());

    if openrouter_key.is_none() && ollama_url.is_none() {
        return None;
    }

    let mut role = terraphim_config::Role::new(role_name);
    role.llm_enabled = true;

    if let Some(key) = openrouter_key {
        let model = std::env::var("OPENROUTER_MODEL")
            .unwrap_or_else(|_| "qwen/qwen3-coder:free".to_string());
        role.llm_api_key = Some(key);
        role.llm_model = Some(model.clone());
        role.extra.insert(
            "llm_provider".to_string(),
            Value::String("openrouter".to_string()),
        );
        role.extra
            .insert("llm_model".to_string(), Value::String(model));
    } else if let Some(url) = ollama_url {
        let model = std::env::var("OLLAMA_MODEL").unwrap_or_else(|_| "llama3.2:3b".to_string());
        role.llm_model = Some(model.clone());
        role.extra.insert(
            "llm_provider".to_string(),
            Value::String("ollama".to_string()),
        );
        role.extra
            .insert("ollama_base_url".to_string(), Value::String(url));
        role.extra
            .insert("ollama_model".to_string(), Value::String(model));
    }

    Some(role)
}

#[tokio::main]
async fn main() -> Result<()> {
    init_tracing();

    let args = Args::parse();

    let options = GrepOptions {
        haystack: args.haystack.into(),
        context_lines: args.context,
        max_results: args.max_results,
        force_rlm: args.force_rlm,
        include_answer: args.answer,
    };

    // Determine role and thesaurus
    let project_config = load_project_config();
    let role_name = resolve_role_name(
        args.role.as_deref(),
        project_config.as_ref().map(|(_, config)| config),
    )?;

    let thesaurus_path = args
        .thesaurus
        .or_else(|| find_default_thesaurus(&role_name))
        .context(
            "No thesaurus specified and could not find default. Use --thesaurus to specify path.",
        )?;

    // Load thesaurus
    let automata_path = AutomataPath::from_local(&thesaurus_path);
    let thesaurus = terraphim_automata::load_thesaurus(&automata_path)
        .await
        .with_context(|| format!("Failed to load thesaurus from {:?}", thesaurus_path))?;

    tracing::debug!("Loaded thesaurus with {} entries", thesaurus.len());

    // Determine search path
    let search_path = args
        .paths
        .first()
        .cloned()
        .unwrap_or_else(|| PathBuf::from("."));

    // Create hybrid searcher
    let mut hybrid_searcher = HybridSearcher::new(role_name.clone(), thesaurus)
        .map_err(|e| anyhow::anyhow!("Failed to create hybrid searcher: {}", e))?;
    hybrid_searcher = hybrid_searcher.with_search_path(search_path);
    let hybrid_searcher = Arc::new(hybrid_searcher);

    // Create sufficiency judge
    let sufficiency_judge = SufficiencyJudge::default();
    let sufficiency_judge = Arc::new(sufficiency_judge);

    // Create TerraphimGrep and optionally attach an LLM client
    let terraphim_grep = TerraphimGrep::new(hybrid_searcher, sufficiency_judge);
    #[cfg(feature = "llm")]
    let terraphim_grep = match build_llm_for_role(&role_name, args.role_config.as_deref()) {
        Some(client) => {
            tracing::info!("LLM client wired: {}", client.name());
            terraphim_grep.with_llm_client(client)
        }
        None => {
            tracing::debug!(
                "No LLM client available -- running in search-only mode (set OPENROUTER_API_KEY \
                 or --role-config to enable RLM synthesis)"
            );
            terraphim_grep
        }
    };

    // Perform search
    let result = terraphim_grep
        .search(&args.query, options)
        .await
        .context("Search failed")?;

    // Output results
    if args.json {
        let json =
            serde_json::to_string_pretty(&result).context("Failed to serialize result to JSON")?;
        println!("{}", json);
    } else {
        print_results(&result, args.context);
    }

    Ok(())
}

fn print_results(result: &GrepResult, context_lines: usize) {
    println!("=== Terraphim Grep Results ===");
    println!();

    // Print stats
    println!(
        "Search latency: {}ms (RLM: {:?}ms)",
        result.stats.search_latency_ms, result.stats.rlm_latency_ms
    );
    println!("Chunks returned: {}", result.stats.chunks_returned);
    println!("KG hits: {}", result.stats.kg_hits);
    println!("Sufficiency: {:?}", result.sufficiency);
    println!();

    // Print concepts
    if !result.concepts.is_empty() {
        println!("=== Knowledge Graph Concepts ===");
        for concept in &result.concepts {
            println!("  - {} (score: {:.2})", concept.name, concept.score);
        }
        println!();
    }

    // Print chunks
    if !result.chunks.is_empty() {
        println!("=== Retrieved Chunks ===");
        for (i, chunk) in result.chunks.iter().enumerate() {
            println!(
                "{}. {}:{}",
                i + 1,
                chunk.source,
                chunk
                    .line_start
                    .map_or_else(|| "?".to_string(), |l| l.to_string())
            );
            if context_lines > 0 {
                // Simple context display - just show content
                println!(
                    "   {}",
                    chunk
                        .content
                        .lines()
                        .take(context_lines)
                        .collect::<Vec<_>>()
                        .join("\n   ")
                );
            } else {
                println!("   {}", chunk.content);
            }
            println!();
        }
    }

    // Print answer if present
    if let Some(ref answer) = result.answer {
        println!("=== Synthesised Answer ===");
        println!("{}", answer.answer);
        println!();
        if !answer.citations.is_empty() {
            println!("Citations:");
            for citation in &answer.citations {
                println!(
                    "  - {} (line {:?}): {}",
                    citation.source, citation.line, citation.excerpt
                );
            }
        }
    }
}
