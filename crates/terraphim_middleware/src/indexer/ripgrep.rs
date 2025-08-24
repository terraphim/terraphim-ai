use std::collections::HashSet;
use std::fs::{self};
use std::path::Path;
use terraphim_types::{Document, Index};
use terraphim_persistence::Persistable;

use super::IndexMiddleware;
use crate::command::ripgrep::{Data, Message, RipgrepCommand};
use crate::Result;
use terraphim_config::Haystack;

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
    async fn index(&self, needle: &str, haystack: &Haystack) -> Result<Index> {
        let haystack_path = Path::new(&haystack.location);
        log::debug!(
            "RipgrepIndexer::index called with needle: '{}' haystack: {:?}",
            needle,
            haystack_path
        );

        // Check if haystack path exists
        if !haystack_path.exists() {
            log::warn!("Haystack path does not exist: {:?}", haystack_path);
            return Ok(Index::default());
        }

        // List files in haystack directory
        if let Ok(entries) = fs::read_dir(haystack_path) {
            let files: Vec<_> = entries
                .filter_map(|entry| entry.ok())
                .filter(|entry| entry.path().extension().map_or(false, |ext| ext == "md"))
                .collect();
            log::debug!(
                "Found {} markdown files in haystack: {:?}",
                files.len(),
                files.iter().map(|e| e.path()).collect::<Vec<_>>()
            );
        }

        // Parse extra parameters from haystack configuration
        let extra_params = haystack.get_extra_parameters();
        log::debug!("Haystack extra_parameters: {:?}", extra_params);
        
        let extra_args = self
            .command
            .parse_extra_parameters(extra_params);
        if !extra_args.is_empty() {
            log::info!("🏷️ Using extra ripgrep parameters: {:?}", extra_args);
            log::info!("🔍 This will modify the ripgrep command to include tag filtering");
        } else {
            log::debug!("No extra parameters provided for ripgrep command");
        }

        // Run ripgrep with extra arguments if any
        let messages = if extra_args.is_empty() {
            self.command.run(needle, haystack_path).await?
        } else {
            self.command
                .run_with_extra_args(needle, haystack_path, &extra_args)
                .await?
        };

        log::debug!("Ripgrep returned {} messages", messages.len());

        // Debug: Log the first few messages to understand the JSON structure
        log::debug!("RipgrepIndexer got {} messages", messages.len());
        for (i, message) in messages.iter().take(3).enumerate() {
            log::debug!("Message {}: {:?}", i, message);
        }

        let documents = self.index_inner(messages);
        log::debug!("Index_inner created {} documents", documents.len());

        Ok(documents)
    }
}

impl RipgrepIndexer {
    /// Normalize document ID to match persistence layer expectations
    fn normalize_document_id(&self, file_path: &str) -> String {
        // Create a dummy document to access the normalize_key method
        let dummy_doc = Document {
            id: "dummy".to_string(),
            title: "dummy".to_string(),
            body: "dummy".to_string(),
            url: "dummy".to_string(),
            description: None,
            summarization: None,
            stub: None,
            tags: None,
            rank: None,
        };
        // Create a meaningful ID from the file path
        let original_id = format!("ripgrep_{}", file_path);
        dummy_doc.normalize_key(&original_id)
    }

    /// Update the underlying Markdown file on disk with the edited document body.
    ///
    /// The `Document.url` field is expected to hold an absolute or haystack-relative
    /// path to the original file. When haystacks are marked as read-only this
    /// method SHOULD NOT be called.
    pub async fn update_document(&self, document: &Document) -> Result<()> {
        use std::path::Path;
        use tokio::fs;

        let path = Path::new(&document.url);
        // Ensure the parent directory exists (it should, given the document was
        // indexed from this path). If not, return an IO error via ?.
        if let Some(parent) = path.parent() {
            if !parent.exists() {
                log::warn!("Parent directory does not exist for {:?}", path);
            }
        }

        let mut content = document.body.clone();
        // Heuristically detect HTML (presence of tags). If HTML detected, convert to Markdown.
        if content.contains('<') && content.contains('>') {
            log::debug!("Converting HTML content to Markdown for file {:?}", path);
            content = html2md::parse_html(&content);
        }

        log::info!("Writing updated document back to markdown file: {:?}", path);
        fs::write(path, content).await?;
        Ok(())
    }

    // #[cached] - temporarily disabled for debugging
    /// This is the inner function that indexes the documents
    /// which allows us to cache requests to the index service
    fn index_inner(&self, messages: Vec<Message>) -> Index {
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

                    document.id = self.normalize_document_id(&path);
                    let title = Path::new(&path)
                        .file_stem()
                        .unwrap()
                        .to_str()
                        .unwrap()
                        .to_string();
                    document.title = title;
                    document.url = path.clone();

                    log::debug!(
                        "Creating document {}: {} ({})",
                        document_count,
                        document.title,
                        document.id
                    );
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
                        }
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
                        }
                        _ => {
                            log::warn!("Match lines is not text: {:?}", message.lines);
                            continue;
                        }
                    };

                    // Only use the first match for description to avoid long concatenations
                    // Limit description to 200 characters for readability
                    if document.description.is_none() {
                        let cleaned_lines = lines.trim();
                        if !cleaned_lines.is_empty() {
                            let description = if cleaned_lines.len() > 200 {
                                format!("{}...", &cleaned_lines[..197])
                            } else {
                                cleaned_lines.to_string()
                            };
                            document.description = Some(description);
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

                    // Only use the first context for description to avoid long concatenations
                    // Limit description to 200 characters for readability
                    if document.description.is_none() {
                        let cleaned_lines = lines.trim();
                        if !cleaned_lines.is_empty() {
                            let description = if cleaned_lines.len() > 200 {
                                format!("{}...", &cleaned_lines[..197])
                            } else {
                                cleaned_lines.to_string()
                            };
                            document.description = Some(description);
                        }
                    }
                }
                Message::End(_) => {
                    // The `End` message could be received before the `Begin`
                    // message causing the document to be empty
                    if !document.title.is_empty() {
                        log::debug!(
                            "Inserting document into index: {} ({})",
                            document.title,
                            document.id
                        );
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
}

