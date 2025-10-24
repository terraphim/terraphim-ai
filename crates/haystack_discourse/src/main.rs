use anyhow::{Context, Result};
use clap::{Parser, ValueEnum};
use discourse_haystack::{DiscourseClient, Post};

#[derive(Debug, Clone, ValueEnum)]
enum OutputFormat {
    Markdown,
    Json,
}

#[derive(Parser, Debug)]
#[command(author, version, about)]
struct Args {
    /// Discourse instance URL (e.g., https://meta.discourse.org)
    #[arg(short, long)]
    url: String,

    /// API Key for authentication
    #[arg(short, long, env = "DISCOURSE_API_KEY")]
    api_key: Option<String>,

    /// Username associated with the API key
    #[arg(short = 'n', long, env = "DISCOURSE_API_USERNAME")]
    username: Option<String>,

    /// Search query to filter posts
    #[arg(short, long)]
    query: String,

    /// Maximum number of results to return
    #[arg(short, long, default_value = "20")]
    limit: u32,

    /// Output format (markdown or json)
    #[arg(short = 'f', long, value_enum, default_value = "markdown")]
    format: OutputFormat,
}

fn print_markdown_output(posts: &[Post]) {
    println!("# Search Results\n");

    for post in posts {
        println!("## [{}]({})", post.title, post.url);
        println!(
            "**Author:** {} | **Posted:** {} | **ID:** {}\n",
            post.username, post.created_at, post.id
        );

        if let Some(body) = &post.body {
            println!("### Full Post Content:\n{}\n", body);
        } else {
            println!("### Excerpt:\n{}\n", post.excerpt);
        }

        println!("---\n");
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    println!("🔍 Searching Discourse for: {}", args.query);
    println!("📡 Server: {}", args.url);
    println!("📊 Limit: {}", args.limit);
    println!("📝 Format: {:?}", args.format);
    println!();

    let api_key = args.api_key.context(
        "API key is required. Set DISCOURSE_API_KEY environment variable or provide --api-key",
    )?;
    let username = args.username.context("Username is required. Set DISCOURSE_API_USERNAME environment variable or provide --username")?;

    let client = DiscourseClient::new(&args.url, &api_key, &username)?;

    let posts = client.search_posts(&args.query, args.limit).await?;

    if posts.is_empty() {
        println!("No results found.");
        return Ok(());
    }

    match args.format {
        OutputFormat::Markdown => print_markdown_output(&posts),
        OutputFormat::Json => {
            println!(
                "{}",
                serde_json::to_string_pretty(&posts)
                    .context("Failed to serialize posts to JSON")?
            );
        }
    }

    Ok(())
}
