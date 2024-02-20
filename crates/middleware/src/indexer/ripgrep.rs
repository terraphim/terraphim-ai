use ahash::AHashMap;
use cached::proc_macro::cached;
use std::collections::HashSet;
use std::fs::{self};
use std::path::Path;
use std::process::Stdio;
use terraphim_config::ConfigState;
use terraphim_types::{Article, Index};
use tokio::io::AsyncReadExt;
use tokio::process::Command;

use super::{calculate_hash, Data, IndexMiddleware};
use super::{json_decode, Message};
use crate::Result;

/// RipgrepMiddleware is a Middleware that uses ripgrep to index and search
/// through haystacks.
pub struct RipgrepIndexer {
    service: RipgrepService,
    config_state: ConfigState,
}

impl RipgrepIndexer {
    pub fn new(config_state: ConfigState) -> Self {
        Self {
            service: RipgrepService::default(),
            config_state,
        }
    }
}

impl IndexMiddleware for RipgrepIndexer {
    /// Index the haystack using ripgrep and return an index of articles
    ///
    /// # Errors
    ///
    /// Returns an error if the middleware fails to index the haystack
    async fn index(&mut self, needle: String, haystack: &Path) -> Result<Index> {
        let messages = self.service.run(needle, &haystack).await?;
        let articles = index_inner(messages);
        for article in articles.values() {
            self.config_state
                .index_article(&article)
                .await
                .map_err(|e| {
                    crate::Error::Indexation(format!(
                        "Failed to index article `{}` ({}): {e:?}",
                        article.title, article.url
                    ))
                })?;
        }
        Ok(articles)
    }
}

pub struct RipgrepService {
    command: String,
    default_args: Vec<String>,
}

/// Returns a new ripgrep service with default arguments
impl Default for RipgrepService {
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

impl RipgrepService {
    /// Runs ripgrep to find `needle` in `haystack`
    ///
    /// Returns a Vec of Messages, which correspond to ripgrep's internal
    /// JSON output. Learn more about ripgrep's JSON output here:
    /// https://docs.rs/grep-printer/0.2.1/grep_printer/struct.JSON.html
    pub async fn run(&self, needle: String, haystack: &Path) -> Result<Vec<Message>> {
        // Merge the default arguments with the needle and haystack
        let args: Vec<String> = vec![needle, haystack.to_string_lossy().to_string()]
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
            Message::Begin(begin_msg) => {
                article = Article::default();

                let path: Option<Data> = begin_msg.path.clone();
                let path_text = match path {
                    Some(Data::Text { text }) => text,
                    _ => {
                        continue;
                    }
                };

                if existing_paths.contains(&path_text) {
                    continue;
                }
                existing_paths.insert(path_text.clone());

                let id = calculate_hash(&path_text);
                article.id = Some(id.clone());
                article.title = path_text.clone();
                article.url = path_text.clone();
            }
            Message::Match(match_msg) => {
                let path = match &match_msg.path {
                    Some(path) => path,
                    None => {
                        println!("Error: path is None: {:?}", match_msg.path);
                        continue;
                    }
                };

                let path_text = match path {
                    Data::Text { text } => text,
                    _ => {
                        println!("Error: path is not text: {path:?}");
                        continue;
                    }
                };
                let body = match fs::read_to_string(path_text) {
                    Ok(body) => body,
                    Err(e) => {
                        println!("Error: Failed to read file: {:?}", e);
                        continue;
                    }
                };
                article.body = body;

                let lines = match &match_msg.lines {
                    Data::Text { text } => text,
                    _ => {
                        println!("Error: lines is not text: {:?}", match_msg.lines);
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
            Message::Context(context_msg) => {
                let article_url = article.url.clone();
                let path = match context_msg.path {
                    Some(ref path) => path,
                    None => {
                        println!("Error: path is None: {:?}", context_msg.path);
                        continue;
                    }
                };

                let path_text = match path {
                    Data::Text { text } => text,
                    _ => {
                        println!("Error: path is not text: {path:?}");
                        continue;
                    }
                };

                // We got a context for a different article
                if article_url != *path_text {
                    println!(
                            "Error: Context for differrent article. article_url != path_text: {article_url:?} != {path_text:?}"
                        );
                    continue;
                }

                let lines = match &context_msg.lines {
                    Data::Text { text } => text,
                    _ => {
                        println!("Error: lines is not text: {:?}", context_msg.lines);
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
                cached_articles.insert(id.clone().to_string(), article.clone());
            }
            _ => {}
        };
    }

    cached_articles
}
