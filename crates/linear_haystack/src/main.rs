//! `linear_haystack` CLI — ad-hoc search of Linear issues for debugging + smoke tests.
//!
//! Resolves the API key via `terraphim-linear-auth` (3 sources: file → env →
//! `op read`), then runs a `searchableContent` query and prints hits as
//! markdown.
//!
//! The CLI is gated behind the `linear` feature because the auth resolver
//! crate is a path dep that won't resolve in CI until
//! terraphim/terraphim-linear#14 lands. Build with
//! `cargo build -p linear_haystack --features linear` for actual use.
//!
//! Example:
//!   cargo run -p linear_haystack --features linear -- search "pgvector"

#[cfg(feature = "linear")]
use anyhow::{Context, Result};
#[cfg(feature = "linear")]
use clap::{Parser, Subcommand};

#[cfg(feature = "linear")]
use linear_haystack::LinearHaystack;

#[cfg(feature = "linear")]
#[derive(Parser, Debug)]
#[command(version, about = "Linear haystack: search issues via the GraphQL API")]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
}

#[cfg(feature = "linear")]
#[derive(Subcommand, Debug)]
enum Cmd {
    /// Search Linear issues for a term (matches title + description).
    Search {
        /// Search term (Linear's `searchableContent.contains`).
        term: String,
        /// Maximum number of results (1-25, hard-capped at 25).
        #[arg(long, default_value = "10")]
        limit: u32,
    },
}

#[cfg(feature = "linear")]
fn main() -> Result<()> {
    let cli = Cli::parse();
    let rt = tokio::runtime::Runtime::new().context("failed to create tokio runtime")?;
    rt.block_on(async_main(cli))
}

#[cfg(feature = "linear")]
async fn async_main(cli: Cli) -> Result<()> {
    match cli.cmd {
        Cmd::Search { term, limit } => {
            let haystack = LinearHaystack::from_auth()
                .context("failed to construct LinearHaystack (auth resolution)")?;
            let limit = limit.clamp(1, 25);
            let issues = haystack
                .client()
                .search(&term, limit)
                .await
                .map_err(|e| anyhow::anyhow!("Linear search failed: {}", e))?;
            println!("# Linear search: `{}`\n", term);
            println!("Found {} issue(s)\n", issues.len());
            for issue in &issues {
                println!("## [{}] {}\n", issue.identifier, issue.title);
                if let Some(desc) = &issue.description {
                    let preview: String = desc.chars().take(200).collect();
                    println!(
                        "{}\n",
                        if desc.chars().count() > 200 {
                            format!("{}…", preview)
                        } else {
                            preview
                        }
                    );
                }
                let mut tag_line = String::new();
                if let Some(team) = &issue.team_key {
                    tag_line.push_str(&format!("**Team:** {} ", team));
                }
                if let Some(state) = &issue.state_name {
                    tag_line.push_str(&format!("**State:** {} ", state));
                }
                tag_line.push_str(&format!("**Priority:** {}", issue.priority));
                if !issue.labels.is_empty() {
                    tag_line.push_str(&format!(" **Labels:** {}", issue.labels.join(", ")));
                }
                println!("{}\n", tag_line);
                if !issue.comments.is_empty() {
                    println!("*Comments:* {}\n", issue.comments.len());
                }
                println!("[View in Linear]({})\n", issue.url);
                println!("---");
            }
        }
    }
    Ok(())
}

// Stub `main` when the `linear` feature is disabled. Without this the
// binary target fails to compile because Rust requires a `main` function.
#[cfg(not(feature = "linear"))]
fn main() {
    eprintln!(
        "linear_haystack is gated behind the 'linear' feature. \
         Build with: cargo build -p linear_haystack --features linear"
    );
    std::process::exit(1);
}
