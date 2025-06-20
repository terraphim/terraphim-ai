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
        log::debug!("RipgrepIndexer::index called with needle: '{}' haystack: {:?}", needle, haystack);
        
        // Check if haystack path exists
        if !haystack.exists() {
            log::warn!("Haystack path does not exist: {:?}", haystack);
            return Ok(Index::default());
        }
        
        // List files in haystack directory
        if let Ok(entries) = fs::read_dir(haystack) {
            let files: Vec<_> = entries
                .filter_map(|entry| entry.ok())
                .filter(|entry| entry.path().extension().map_or(false, |ext| ext == "md"))
                .collect();
            log::debug!("Found {} markdown files in haystack: {:?}", files.len(), files.iter().map(|e| e.path()).collect::<Vec<_>>());
        }
        
        let messages = self.command.run(needle, haystack).await?;
        log::debug!("Ripgrep returned {} messages", messages.len());
        
        let documents = index_inner(messages);
        log::debug!("Index_inner created {} documents", documents.len());
        
        Ok(documents)
    }
}

#[cached]
/// This is the inner function that indexes the documents
/// which allows us to cache requests to the index service
fn index_inner(messages: Vec<Message>) -> Index {
    log::debug!("index_inner called with {} messages", messages.len());
    
    // Cache of already processed documents
    let mut index: Index = Index::default();
    let mut existing_paths: HashSet<String> = HashSet::new();

    let mut document = Document::default();
    let mut document_count = 0;
    let mut match_count = 0;
    
    for message in messages {
        match message {
            Message::Begin(message) => {
                document = Document::default();
                document_count += 1;

                let Some(path) = message.path() else {
                    log::warn!("Begin message without path");
                    continue;
                };
                
                if existing_paths.contains(&path) {
                    log::warn!("Skipping duplicate document: {}", path);
                    continue;
                }
                existing_paths.insert(path.clone());

                document.id = hash_as_string(&path);
                let title = Path::new(&path)
                    .file_stem()
                    .unwrap()
                    .to_str()
                    .unwrap()
                    .to_string();
                document.title = title;
                document.url = path.clone();
                
                log::debug!("Creating document {}: {} ({})", document_count, document.title, document.id);
            }
            Message::Match(message) => {
                match_count += 1;
                let Some(path) = message.path() else {
                    log::warn!("Match message without path");
                    continue;
                };
                
                log::trace!("Processing match {} for document: {}", match_count, path);
                
                let body = match fs::read_to_string(&path) {
                    Ok(body) => {
                        log::trace!("Successfully read file: {} ({} bytes)", path, body.len());
                        body
                    },
                    Err(e) => {
                        log::warn!("Failed to read file: {} - {:?}", path, e);
                        continue;
                    }
                };
                document.body = body;

                let lines = match &message.lines {
                    Data::Text { text } => {
                        log::trace!("Match text: {}", text);
                        text
                    },
                    _ => {
                        log::warn!("Match lines is not text: {:?}", message.lines);
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
                    log::warn!("Context message without path");
                    continue;
                };

                // We got a context for a different document
                if document_url != *path {
                    log::warn!("Context for different document. document_url != path: {document_url:?} != {path:?}");
                    continue;
                }

                let lines = match &message.lines {
                    Data::Text { text } => text,
                    _ => {
                        log::warn!("Context lines is not text: {:?}", message.lines);
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
                if !document.title.is_empty() {
                    log::debug!("Inserting document into index: {} ({})", document.title, document.id);
                    index.insert(document.id.to_string(), document.clone());
                } else {
                    log::debug!("Skipping empty document");
                }
            }
            _ => {
                log::trace!("Other message type: {:?}", message);
            }
        };
    }

    log::debug!("Index_inner completed: {} documents processed, {} matches found, {} documents in final index", 
             document_count, match_count, index.len());
    index
}
