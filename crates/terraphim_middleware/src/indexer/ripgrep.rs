use cached::proc_macro::cached;
use std::collections::HashSet;
use std::fs::{self};
use std::path::Path;
use terraphim_types::{Document, Index};

use super::{hash_as_string, IndexMiddleware};
use crate::command::ripgrep::{Data, Message, RipgrepCommand};
use crate::Result;

/// Middleware that uses ripgrep to index Markdown haystacks.
#[derive(Default)]
pub struct RipgrepIndexer {
    command: RipgrepCommand,
}

impl IndexMiddleware for RipgrepIndexer {
    /// Index the haystack using ripgrep and return an index of documents
    ///
    /// # Errors
    ///
    /// Returns an error if the middleware fails to index the haystack
    async fn index(&self, needle: &str, haystack: &Path) -> Result<Index> {
        let messages = self.command.run(needle, haystack).await?;
        let documents = index_inner(messages);
        Ok(documents)
    }
}

#[cached]
/// This is the inner function that indexes the documents
/// which allows us to cache requests to the index service
fn index_inner(messages: Vec<Message>) -> Index {
    // Cache of already processed documents
    let mut index: Index = Index::default();
    let mut existing_paths: HashSet<String> = HashSet::new();

    let mut document = Document::default();
    for message in messages {
        match message {
            Message::Begin(message) => {
                document = Document::default();

                let Some(path) = message.path() else {
                    continue;
                };
                if existing_paths.contains(&path) {
                    continue;
                }
                existing_paths.insert(path.clone());

                document.id = hash_as_string(&path);
                let title = Path::new(&path).file_stem().unwrap().to_str().unwrap().to_string();
                document.title = title;
                document.url = path.clone();
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
                document.body = body;

                let lines = match &message.lines {
                    Data::Text { text } => text,
                    _ => {
                        println!("Error: lines is not text: {:?}", message.lines);
                        continue;
                    }
                };
                match document.description {
                    Some(description) => {
                        document.description = Some(description + " " + &lines);
                    }
                    None => {
                        document.description = Some(lines.clone());
                    }
                }
            }
            Message::Context(message) => {
                let document_url = document.url.clone();
                let Some(path) = message.path() else {
                    continue;
                };

                // We got a context for a different document
                if document_url != *path {
                    println!(
                            "Error: Context for differrent document. document_url != path: {document_url:?} != {path:?}"
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
                match document.description {
                    Some(description) => {
                        document.description = Some(description + " " + &lines);
                    }
                    None => {
                        document.description = Some(lines.clone());
                    }
                }
            }
            Message::End(_) => {
                // The `End` message could be received before the `Begin`
                // message causing the document to be empty
                index.insert(document.id.to_string(), document.clone());
            }
            _ => {}
        };
    }

    index
}
