use ahash::AHashMap;
use cached::proc_macro::cached;
use std::collections::HashSet;
use std::fs::{self};
use std::path::Path;
use terraphim_types::{Article, Index};

use super::{calculate_hash, IndexMiddleware};
use crate::command::ripgrep::{Data, Message, RipgrepCommand};
use crate::Result;

/// Middleware that uses ripgrep to index Markdown haystacks.
#[derive(Default)]
pub struct RipgrepIndexer {
    command: RipgrepCommand,
}

impl IndexMiddleware for RipgrepIndexer {
    /// Index the haystack using ripgrep and return an index of articles
    ///
    /// # Errors
    ///
    /// Returns an error if the middleware fails to index the haystack
    async fn index(&self, needle: &str, haystack: &Path) -> Result<Index> {
        let messages = self.command.run(needle, haystack).await?;
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
    for message in messages {
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

                article.id = calculate_hash(&path);
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
                cached_articles.insert(article.id.to_string(), article.clone());
            }
            _ => {}
        };
    }

    cached_articles
}
