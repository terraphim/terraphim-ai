use ahash::AHashMap;
use cached::proc_macro::cached;
use std::collections::HashSet;
use std::fs::{self};
use std::path::Path;
use std::process::Stdio;
use terraphim_types::{Article, Index};
use tokio::io::AsyncReadExt;
use tokio::process::Command;

use super::{calculate_hash, Data, IndexMiddleware};
use super::{json_decode, Message};
use crate::Result;

/// Middleware that uses ripgrep to index Markdown haystacks.
pub struct RipgrepIndexer {
    service: RipgrepCommand,
}

impl Default for RipgrepIndexer {
    fn default() -> Self {
        Self {
            service: RipgrepCommand::default(),
        }
    }
}

impl IndexMiddleware for RipgrepIndexer {
    /// Index the haystack using ripgrep and return an index of articles
    ///
    /// # Errors
    ///
    /// Returns an error if the middleware fails to index the haystack
    async fn index(&self, needle: &str, haystack: &Path) -> Result<Index> {
        let messages = self.service.run(needle, &haystack).await?;
        let articles = index_inner(messages);
        Ok(articles)
    }
}

#[cached]
/// This is the inner function that indexes the articles
/// which allows us to cache requests to the index service
fn index_inner(messages: Vec<Message>) -> Index {
    // Cache of already processed articles
    let mut cached_articles: Index = AHashMap::new();
    let mut existing_paths: HashSet<String> = HashSet::new();

    let mut article = Article::default();
    for message in messages.iter() {
        match message {
            Message::Begin(message) => {
                article = Article::default();

                let Some(path) = message.path() else {
                    continue;
                };
                if existing_paths.contains(&path) {
                    continue;
                }
                existing_paths.insert(path.clone());

                let id = calculate_hash(&path);
                article.id = Some(id.clone());
                article.title = path.clone();
                article.url = path.clone();
            }
            Message::Match(message) => {
                let Some(path) = message.path() else {
                    continue;
                };
                let body = match fs::read_to_string(path) {
                    Ok(body) => body,
                    Err(e) => {
                        println!("Error: Failed to read file: {:?}", e);
                        continue;
                    }
                };
                article.body = body;

                let lines = match &message.lines {
                    Data::Text { text } => text,
                    _ => {
                        println!("Error: lines is not text: {:?}", message.lines);
                        continue;
                    }
                };
                match article.description {
                    Some(description) => {
                        article.description = Some(description + " " + &lines);
                    }
                    None => {
                        article.description = Some(lines.clone());
                    }
                }
            }
            Message::Context(message) => {
                let article_url = article.url.clone();
                let Some(path) = message.path() else {
                    continue;
                };

                // We got a context for a different article
                if article_url != *path {
                    println!(
                            "Error: Context for differrent article. article_url != path: {article_url:?} != {path:?}"
                        );
                    continue;
                }

                let lines = match &message.lines {
                    Data::Text { text } => text,
                    _ => {
                        println!("Error: lines is not text: {:?}", message.lines);
                        continue;
                    }
                };
                match article.description {
                    Some(description) => {
                        article.description = Some(description + " " + &lines);
                    }
                    None => {
                        article.description = Some(lines.clone());
                    }
                }
            }
            Message::End(_) => {
                // The `End` message could be received before the `Begin`
                // message causing the article to be empty
                let id = match article.id {
                    Some(ref id) => id,
                    None => {
                        println!("Error: End message received before Begin message. Skipping.");
                        continue;
                    }
                };
                cached_articles.insert(id.to_string(), article.clone());
            }
            _ => {}
        };
    }

    cached_articles
}

pub struct RipgrepCommand {
    command: String,
    default_args: Vec<String>,
}

/// Returns a new ripgrep service with default arguments
impl Default for RipgrepCommand {
    fn default() -> Self {
        Self {
            command: "rg".to_string(),
            default_args: ["--json", "--trim", "-C3", "--ignore-case"]
                .into_iter()
                .map(String::from)
                .collect(),
        }
    }
}

impl RipgrepCommand {
    /// Runs ripgrep to find `needle` in `haystack`
    ///
    /// Returns a Vec of Messages, which correspond to ripgrep's internal
    /// JSON output. Learn more about ripgrep's JSON output here:
    /// https://docs.rs/grep-printer/0.2.1/grep_printer/struct.JSON.html
    pub async fn run(&self, needle: &str, haystack: &Path) -> Result<Vec<Message>> {
        // Merge the default arguments with the needle and haystack
        let args: Vec<String> = vec![needle.to_string(), haystack.to_string_lossy().to_string()]
            .into_iter()
            .chain(self.default_args.clone())
            .collect();

        let mut child = Command::new(&self.command)
            .args(args)
            .stdout(Stdio::piped())
            .spawn()?;

        let mut stdout = child.stdout.take().expect("Stdout is not available");
        let read = async move {
            let mut data = String::new();
            stdout.read_to_string(&mut data).await.map(|_| data)
        };
        let output = read.await?;
        json_decode(&output)
    }
}
