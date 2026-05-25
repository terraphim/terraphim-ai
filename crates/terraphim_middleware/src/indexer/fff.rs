use std::collections::HashSet;
use std::path::Path;
use std::sync::Arc;

use cached::proc_macro::cached;
use fff_search::external_scorer::ExternalScorer;
use fff_search::{
    grep_search, parse_grep_query, ContentCacheBudget, FFFMode, FilePicker, FilePickerOptions,
    GrepMode, GrepSearchOptions, SharedFrecency,
};
use terraphim_config::Haystack;
use terraphim_persistence::Persistable;
use terraphim_types::{Document, DocumentType, Index};
use tokio::fs as tfs;

use super::IndexMiddleware;
use crate::Result;

/// Find the largest byte index <= `index` that is a valid UTF-8 char boundary.
/// Polyfill for str::floor_char_boundary (stable since Rust 1.91).
fn floor_char_boundary(s: &str, index: usize) -> usize {
    if index >= s.len() {
        return s.len();
    }
    let mut i = index;
    while i > 0 && !s.is_char_boundary(i) {
        i -= 1;
    }
    i
}

/// Middleware that uses fff-search to index Markdown haystacks.
///
/// Replaces `RipgrepIndexer` with a pure-Rust implementation that does
/// not require the external `rg` binary.
///
/// Supports optional knowledge-graph path scoring and frecency tracking
/// via builder methods.
pub struct FffIndexer {
    /// Optional KG path scorer for boosting results by knowledge-graph
    /// concept matches. When `None`, no KG boosting is applied.
    kg_scorer: Option<Arc<terraphim_file_search::kg_scorer::KgPathScorer>>,
    /// Optional persistent frecency tracker (LMDB-backed) for access-frequency scoring.
    frecency: Option<SharedFrecency>,
}

impl Default for FffIndexer {
    fn default() -> Self {
        let frecency = std::env::var("FFF_FRECENCY_PATH").ok().and_then(|path| {
            fff_search::FrecencyTracker::new(&path, false)
                .map(|tracker| {
                    let shared = SharedFrecency::default();
                    shared.init(tracker).ok();
                    shared
                })
                .ok()
        });

        Self {
            kg_scorer: None,
            frecency,
        }
    }
}

/// Cached wrapper that performs fff-search indexing for a given haystack/query.
#[cached(
    result = true,
    size = 64,
    key = "String",
    convert = r#"{ format!("{}::{}::{:?}", haystack.location, needle, haystack.get_extra_parameters()) }"#
)]
async fn cached_fff_index(needle: &str, haystack: &Haystack) -> Result<Index> {
    let indexer = FffIndexer::default();
    indexer.index_inner(needle, haystack).await
}

impl IndexMiddleware for FffIndexer {
    /// Index the haystack using fff-search and return an index of documents.
    ///
    /// # Errors
    ///
    /// Returns an error if the haystack path does not exist, `FilePicker`
    /// initialisation fails, or file I/O errors occur during document
    /// construction.
    async fn index(&self, needle: &str, haystack: &Haystack) -> Result<Index> {
        cached_fff_index(needle, haystack).await
    }
}

impl FffIndexer {
    /// Create a new `FffIndexer` with default settings.
    pub fn new() -> Self {
        Self::default()
    }

    /// Attach a knowledge-graph path scorer for boosting results by
    /// knowledge-graph concept matches in file paths.
    ///
    /// This follows the same builder pattern as `McpService::with_kg_scorer()`.
    pub fn with_kg_scorer(
        mut self,
        scorer: Arc<terraphim_file_search::kg_scorer::KgPathScorer>,
    ) -> Self {
        self.kg_scorer = Some(scorer);
        self
    }

    /// Update the underlying Markdown file on disk with the edited document body.
    ///
    /// The `Document.url` field is expected to hold an absolute or haystack-relative
    /// path to the original file. When haystacks are marked as read-only this
    /// method SHOULD NOT be called.
    ///
    /// # Errors
    ///
    /// Returns an error if the file cannot be written.
    pub async fn update_document(&self, document: &Document) -> Result<()> {
        let path = Path::new(&document.url);

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
        tfs::write(path, content).await?;
        Ok(())
    }

    /// Normalise document ID to match persistence layer expectations.
    fn normalize_document_id(&self, file_path: &str) -> String {
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
            source_haystack: None,
            doc_type: DocumentType::KgEntry,
            synonyms: None,
            route: None,
            priority: None,
            quality_score: None,
        };
        let original_id = format!("fff_{}", file_path);
        dummy_doc.normalize_key(&original_id)
    }

    /// Inner indexing logic using fff-search.
    async fn index_inner(&self, needle: &str, haystack: &Haystack) -> Result<Index> {
        let haystack_path = Path::new(&haystack.location);
        log::debug!(
            "FffIndexer::index called with needle: '{}' haystack: {:?}",
            needle,
            haystack_path
        );

        // Check if haystack path exists
        if !haystack_path.exists() {
            log::warn!("Haystack path does not exist: {:?}", haystack_path);
            return Ok(Index::default());
        }

        // Initialise FilePicker
        let mut picker = FilePicker::new(FilePickerOptions {
            base_path: haystack.location.clone(),
            mode: FFFMode::Ai,
            watch: false,
            cache_budget: None,
            ..FilePickerOptions::default()
        })
        .map_err(|e| crate::Error::FileSearch(e.to_string()))?;

        picker
            .collect_files()
            .map_err(|e| crate::Error::FileSearch(e.to_string()))?;

        // Filter to markdown files only (parity with RipgrepIndexer's -tmarkdown default)
        let mut files: Vec<_> = picker
            .get_files()
            .iter()
            .filter(|f| f.relative_path.ends_with(".md"))
            .cloned()
            .collect();

        log::debug!(
            "Found {} markdown files in haystack: {:?}",
            files.len(),
            haystack_path
        );

        if files.is_empty() {
            return Ok(Index::default());
        }

        // Apply KG path scoring to sort files by relevance when a scorer is configured.
        // Files matching knowledge-graph concepts in their paths are searched first,
        // ensuring conceptually relevant documents appear in results even with pagination.
        if let Some(scorer) = &self.kg_scorer {
            log::debug!("Applying KG path scoring to {} files", files.len());
            files.sort_by_key(|f| std::cmp::Reverse(scorer.score(f)));
        }

        // Apply frecency scoring if a tracker is configured.
        // This updates access-frequency scores on file items for ranking.
        if let Some(ref frecency) = self.frecency {
            log::debug!("Applying frecency scoring to {} files", files.len());
            if let Ok(guard) = frecency.read() {
                if let Some(tracker) = guard.as_ref() {
                    for file in &mut files {
                        if let Err(e) = file.update_frecency_scores(tracker, FFFMode::Ai) {
                            log::trace!(
                                "Failed to update frecency for {}: {}",
                                file.relative_path,
                                e
                            );
                        }
                    }
                }
            }
        }

        // Parse grep query
        let fff_query = parse_grep_query(needle);
        let options = GrepSearchOptions {
            max_file_size: 10 * 1024 * 1024,
            max_matches_per_file: 200,
            smart_case: true,
            file_offset: 0,
            page_limit: 200,
            mode: GrepMode::PlainText,
            time_budget_ms: 0,
            before_context: 0,
            after_context: 0,
            classify_definitions: false,
        };
        let budget = ContentCacheBudget::default();

        // Run grep on the filtered files
        let result = grep_search(&files, &fff_query, &options, &budget, None, None, None);

        log::debug!(
            "fff-search returned {} matches across {} files",
            result.matches.len(),
            result.files.len()
        );

        // Build index from results
        let mut index = Index::default();
        let mut processed_files: HashSet<usize> = HashSet::new();

        for m in &result.matches {
            let file_index = m.file_index;

            // Skip if we've already processed this file
            if processed_files.contains(&file_index) {
                continue;
            }
            processed_files.insert(file_index);

            let file = match result.files.get(file_index) {
                Some(f) => f,
                None => {
                    log::warn!("Match referenced invalid file_index: {}", file_index);
                    continue;
                }
            };

            let relative_path = &file.relative_path;
            let full_path = haystack_path.join(relative_path);
            let path_str = full_path.to_string_lossy().to_string();

            // Read file body
            let body = match tfs::read_to_string(&full_path).await {
                Ok(body) => body,
                Err(e) => {
                    log::warn!("Failed to read file: {} - {:?}", full_path.display(), e);
                    continue;
                }
            };

            // Extract title from file stem
            let title = full_path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("")
                .to_string();

            // Build description from the first matching line
            let description = {
                let cleaned = m.line_content.trim();
                if cleaned.is_empty() {
                    None
                } else if cleaned.len() > 200 {
                    let safe_end = floor_char_boundary(cleaned, 197);
                    Some(format!("{}...", &cleaned[..safe_end]))
                } else {
                    Some(cleaned.to_string())
                }
            };

            let document = Document {
                id: self.normalize_document_id(&path_str),
                title,
                url: path_str,
                body,
                description,
                summarization: None,
                stub: None,
                tags: None,
                rank: None,
                source_haystack: None, // Set by search_haystacks after indexing
                doc_type: DocumentType::KgEntry,
                synonyms: None,
                route: None,
                priority: None,
                quality_score: None,
            };

            log::debug!(
                "Inserting document into index: {} ({})",
                document.title,
                document.id
            );
            index.insert(document.id.clone(), document);
        }

        log::debug!(
            "FffIndexer completed: {} documents in final index",
            index.len()
        );

        Ok(index)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_document_id() {
        let indexer = FffIndexer::default();
        let id = indexer.normalize_document_id("/path/to/test.md");
        assert!(id.starts_with("fff_"));
        assert!(id.contains("test_md"));
    }

    #[test]
    fn test_normalize_document_id_with_spaces() {
        let indexer = FffIndexer::default();
        let id = indexer.normalize_document_id("/path/to/my file.md");
        assert!(id.starts_with("fff_"));
        assert!(id.contains("my_file_md"));
    }
}
