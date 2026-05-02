mod knowledge;
mod models;

use anyhow::Result;
use clap::{Parser, ValueEnum};
use std::fs::File;
use std::io::{self, BufRead, Write};
use std::path::PathBuf;
use tracing::{info, warn};

use knowledge::KnowledgeGraph;
use models::{SemanticAnalysis, SemanticGroup};

#[derive(Parser)]
#[command(name = "terraphim_dsm")]
#[command(about = "Semantic module grouping using Terraphim knowledge graphs")]
#[command(version)]
struct Cli {
    /// Input file with module paths (one per line). Defaults to stdin.
    #[arg(short, long)]
    input: Option<PathBuf>,

    /// Output format
    #[arg(short, long, value_enum, default_value = "text")]
    format: OutputFormat,

    /// Output file (default: stdout)
    #[arg(short, long)]
    output: Option<PathBuf>,

    /// Knowledge graph path (default: ~/.config/terraphim/kg)
    #[arg(long)]
    kg_path: Option<PathBuf>,

    /// Filter by specific domain concept
    #[arg(short, long)]
    concept: Option<String>,

    /// Show uncategorized modules
    #[arg(long)]
    show_uncategorized: bool,

    /// Verbose logging
    #[arg(short, long)]
    verbose: bool,
}

#[derive(Clone, ValueEnum)]
enum OutputFormat {
    Text,
    Json,
    Csv,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    // Initialize logging
    let subscriber = tracing_subscriber::fmt()
        .with_env_filter(if cli.verbose {
            "terraphim_dsm=debug"
        } else {
            "terraphim_dsm=info"
        })
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;

    // Load knowledge graph
    info!("Loading knowledge graph...");
    let kg = if let Some(kg_path) = &cli.kg_path {
        let mut kg = KnowledgeGraph::new(kg_path.clone());
        kg.load_from_directory(kg_path)?;
        kg
    } else {
        match KnowledgeGraph::load_default() {
            Ok(kg) => kg,
            Err(e) => {
                warn!("Failed to load default KG: {}", e);
                return Ok(());
            }
        }
    };

    info!(
        "Loaded {} concepts from knowledge graph",
        kg.concept_count()
    );

    // Read module paths from input
    let modules = read_module_paths(&cli.input)?;
    info!("Read {} module paths", modules.len());

    if modules.is_empty() {
        warn!("No module paths provided");
        return Ok(());
    }

    // Group modules by domain concept
    let groups = kg.group_by_concept(&modules);

    // Build analysis
    let mut semantic_groups: Vec<SemanticGroup> = Vec::new();
    let mut uncategorized_count = 0;

    for (concept, modules) in &groups {
        if concept == "uncategorized" {
            uncategorized_count = modules.len();
        } else {
            semantic_groups.push(SemanticGroup {
                concept: concept.clone(),
                modules: modules.clone(),
                count: modules.len(),
            });
        }
    }

    // Sort by module count (descending)
    semantic_groups.sort_by_key(|b| std::cmp::Reverse(b.count));

    // Filter by concept if specified
    if let Some(filter) = &cli.concept {
        semantic_groups.retain(|g| g.concept.to_lowercase() == filter.to_lowercase());
    }

    let analysis = SemanticAnalysis {
        groups: semantic_groups,
        total_modules: modules.len(),
        uncategorized_count,
        knowledge_graph_concepts: kg.concept_count(),
    };

    // Output
    let mut output_writer: Box<dyn Write> = if let Some(output_path) = &cli.output {
        Box::new(File::create(output_path)?)
    } else {
        Box::new(io::stdout())
    };

    match cli.format {
        OutputFormat::Text => {
            format_text(
                &analysis,
                &groups,
                cli.show_uncategorized,
                &mut output_writer,
            )?;
        }
        OutputFormat::Json => {
            let json = serde_json::to_string_pretty(&analysis)?;
            output_writer.write_all(json.as_bytes())?;
        }
        OutputFormat::Csv => {
            format_csv(
                &analysis,
                &groups,
                cli.show_uncategorized,
                &mut output_writer,
            )?;
        }
    }

    if let Some(ref path) = cli.output {
        info!("Output written to: {}", path.display());
    }

    Ok(())
}

fn read_module_paths(input: &Option<PathBuf>) -> Result<Vec<String>> {
    let reader: Box<dyn BufRead> = if let Some(path) = input {
        Box::new(io::BufReader::new(File::open(path)?))
    } else {
        Box::new(io::BufReader::new(io::stdin()))
    };

    let mut modules = Vec::new();
    for line in reader.lines() {
        let line = line?;
        let trimmed = line.trim();
        if !trimmed.is_empty() && !trimmed.starts_with('#') {
            modules.push(trimmed.to_string());
        }
    }

    Ok(modules)
}

fn format_text(
    analysis: &SemanticAnalysis,
    groups: &std::collections::HashMap<String, Vec<String>>,
    show_uncategorized: bool,
    writer: &mut dyn Write,
) -> Result<()> {
    writeln!(writer, "=== Semantic Module Grouping ===")?;
    writeln!(
        writer,
        "Modules: {} | KG Concepts: {} | Uncategorized: {}",
        analysis.total_modules, analysis.knowledge_graph_concepts, analysis.uncategorized_count
    )?;
    writeln!(writer)?;

    for group in &analysis.groups {
        writeln!(writer, "[{}] {} modules", group.concept, group.count)?;
        for module in &group.modules {
            writeln!(writer, "  - {}", module)?;
        }
        writeln!(writer)?;
    }

    if show_uncategorized {
        if let Some(uncategorized) = groups.get("uncategorized") {
            writeln!(writer, "[uncategorized] {} modules", uncategorized.len())?;
            for module in uncategorized {
                writeln!(writer, "  - {}", module)?;
            }
        }
    }

    Ok(())
}

fn format_csv(
    analysis: &SemanticAnalysis,
    groups: &std::collections::HashMap<String, Vec<String>>,
    show_uncategorized: bool,
    writer: &mut dyn Write,
) -> Result<()> {
    writeln!(writer, "concept,module")?;

    for group in &analysis.groups {
        for module in &group.modules {
            writeln!(writer, "{},\"{}\"", group.concept, module)?;
        }
    }

    if show_uncategorized {
        if let Some(uncategorized) = groups.get("uncategorized") {
            for module in uncategorized {
                writeln!(writer, "uncategorized,\"{}\"", module)?;
            }
        }
    }

    Ok(())
}
