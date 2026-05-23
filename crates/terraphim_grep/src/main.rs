use anyhow::Result;
use clap::{Parser};
use terraphim_grep::{GrepOptions, Haystack};

#[derive(Parser, Debug)]
#[command(name = "terraphim-grep")]
#[command(version, about = "Intelligent hybrid grep with RLM fallback and KG curation")]
struct Args {
    #[arg(help = "Search query")]
    query: String,

    #[arg(short = 'C', long, default_value = "0", help = "Context lines before/after match")]
    context: usize,

    #[arg(short = 'n', long, default_value = "50", help = "Maximum number of results")]
    max_results: usize,

    #[arg(short = 'H', long, value_enum, default_value = "all", help = "Haystack to search")]
    haystack: HaystackArg,

    #[arg(long, help = "Force RLM fallback for all queries")]
    force_rlm: bool,

    #[arg(long, help = "Include LLM-generated answer")]
    answer: bool,

    #[arg(long, help = "Output JSON format")]
    json: bool,

    #[arg(long, help = "Search paths (default: current directory)")]
    paths: Vec<std::path::PathBuf>,

    #[arg(long, help = "Role to use for search")]
    role: Option<String>,
}

#[derive(Debug, Clone, clap::ValueEnum)]
enum HaystackArg {
    All,
    Code,
    Docs,
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

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    let options = GrepOptions {
        haystack: args.haystack.into(),
        context_lines: args.context,
        max_results: args.max_results,
        force_rlm: args.force_rlm,
        include_answer: args.answer,
    };

    // TODO: Initialize properly with RoleGraph and other dependencies
    // For now, just show help on how to use
    eprintln!("terraphim-grep v{}", env!("CARGO_PKG_VERSION"));
    eprintln!("Query: {}", args.query);
    eprintln!("Haystack: {:?}", options.haystack);
    eprintln!("Context: {}", options.context_lines);
    eprintln!("Max results: {}", options.max_results);
    eprintln!("Force RLM: {}", options.force_rlm);
    eprintln!("Include answer: {}", options.include_answer);
    eprintln!("Paths: {:?}", args.paths);
    eprintln!("Role: {:?}", args.role);

    eprintln!("\nFull implementation requires RoleGraph initialization.");
    eprintln!("See docs/implementation-plan-terraphim-grep.md for integration details.");

    Ok(())
}
