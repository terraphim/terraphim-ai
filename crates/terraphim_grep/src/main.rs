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
        .with(fmt::layer())
        .with(filter)
        .init();
}

fn find_default_thesaurus() -> Option<PathBuf> {
    // Look for thesaurus in standard locations
    let possible_paths = vec![
        PathBuf::from("."),
        PathBuf::from("../docs/src"),
        PathBuf::from("../../docs/src"),
    ];

    for base in possible_paths {
        if let Ok(cwd) = std::env::current_dir() {
            let candidate = cwd.join(&base);
            if candidate.exists() {
                // Look for any *_thesaurus.json file
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
    let role_name = args.role.unwrap_or_else(|| "default".to_string());

    let thesaurus_path = args.thesaurus.or_else(find_default_thesaurus).context(
        "No thesaurus specified and could not find default. Use --thesaurus to specify path.",
    )?;

    // Load thesaurus
    let automata_path = AutomataPath::from_local(&thesaurus_path);
    let thesaurus = terraphim_automata::load_thesaurus(&automata_path)
        .await
        .with_context(|| format!("Failed to load thesaurus from {:?}", thesaurus_path))?;

    tracing::debug!("Loaded thesaurus with {} entries", thesaurus.len());

    // Determine search path
    let search_path = args.paths.first().cloned().unwrap_or_else(|| PathBuf::from("."));

    // Create hybrid searcher
    let mut hybrid_searcher = HybridSearcher::new(role_name.clone(), thesaurus)
        .map_err(|e| anyhow::anyhow!("Failed to create hybrid searcher: {}", e))?;
    hybrid_searcher = hybrid_searcher.with_search_path(search_path);
    let hybrid_searcher = Arc::new(hybrid_searcher);

    // Create sufficiency judge
    let sufficiency_judge = SufficiencyJudge::default();
    let sufficiency_judge = Arc::new(sufficiency_judge);

    // Create TerraphimGrep
    let terraphim_grep = TerraphimGrep::new(hybrid_searcher, sufficiency_judge);

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
