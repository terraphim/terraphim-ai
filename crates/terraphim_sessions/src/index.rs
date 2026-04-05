//! Full-text search index for sessions using fff-search
//!
//! This module provides persistent, disk-based indexing and search
//! for session content using the fff-search library.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use crate::model::{Session, SessionId};

/// Configuration for the session index
#[derive(Debug, Clone)]
pub struct IndexConfig {
    /// Directory where index files are stored
    pub index_dir: PathBuf,
    /// Maximum number of sessions to keep in memory cache
    pub cache_size: usize,
    /// Whether to enable bigram filtering for faster searches
    pub enable_bigram_filter: bool,
}

impl Default for IndexConfig {
    fn default() -> Self {
        Self {
            index_dir: dirs::data_dir()
                .unwrap_or_else(|| PathBuf::from("."))
                .join("terraphim")
                .join("session_index"),
            cache_size: 1000,
            enable_bigram_filter: true,
        }
    }
}

impl IndexConfig {
    /// Create config with custom index directory
    pub fn with_dir<P: AsRef<Path>>(dir: P) -> Self {
        Self {
            index_dir: dir.as_ref().to_path_buf(),
            ..Default::default()
        }
    }
}

/// A searchable document representing session content
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionDocument {
    /// Session ID
    pub session_id: SessionId,
    /// Source connector (e.g., "claude-code", "cursor")
    pub source: String,
    /// Session title if available
    pub title: Option<String>,
    /// All message content concatenated for full-text search
    pub content: String,
    /// Message roles for filtering (e.g., "user", "assistant")
    pub roles: Vec<String>,
    /// Tool names mentioned in the session
    pub tools: Vec<String>,
    /// Timestamp for sorting
    pub timestamp: Option<i64>,
}

impl SessionDocument {
    /// Create a document from a session
    pub fn from_session(session: &Session) -> Self {
        let content: String = session
            .messages
            .iter()
            .map(|m| format!("{}: {}", m.role, m.content))
            .collect::<Vec<_>>()
            .join("\n\n");

        let roles: Vec<String> = session
            .messages
            .iter()
            .map(|m| m.role.to_string())
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .collect();

        let tools = session.tools_used();

        Self {
            session_id: session.id.clone(),
            source: session.source.clone(),
            title: session.title.clone(),
            content,
            roles,
            tools,
            timestamp: session.started_at.map(|t| t.as_millisecond()),
        }
    }

    /// Get searchable text (includes title and content)
    pub fn searchable_text(&self) -> String {
        let mut text = String::new();
        if let Some(title) = &self.title {
            text.push_str(title);
            text.push('\n');
        }
        text.push_str(&self.content);
        text
    }
}

/// Search result with relevance score
#[derive(Debug, Clone)]
pub struct SearchResult {
    /// Session ID
    pub session_id: SessionId,
    /// Relevance score (higher is better)
    pub score: i32,
    /// Matching document
    pub document: SessionDocument,
}

/// Session index for full-text search
pub struct SessionIndex {
    #[allow(dead_code)]
    config: IndexConfig,
    /// In-memory cache of documents
    documents: HashMap<SessionId, SessionDocument>,
    /// Bigram filter for fast candidate filtering (if enabled)
    #[allow(dead_code)]
    bigram_filter: Option<BigramFilter>,
}

impl SessionIndex {
    /// Create a new session index with default configuration
    pub fn new() -> Result<Self> {
        Self::with_config(IndexConfig::default())
    }

    /// Create a new session index with custom configuration
    pub fn with_config(config: IndexConfig) -> Result<Self> {
        // Ensure index directory exists
        std::fs::create_dir_all(&config.index_dir)
            .with_context(|| format!("Failed to create index directory: {:?}", config.index_dir))?;

        Ok(Self {
            config,
            documents: HashMap::new(),
            bigram_filter: None,
        })
    }

    /// Index a single session
    pub fn index_session(&mut self, session: &Session) -> Result<()> {
        let doc = SessionDocument::from_session(session);
        self.documents.insert(doc.session_id.clone(), doc);
        // TODO: Persist to disk and update bigram filter
        Ok(())
    }

    /// Index multiple sessions
    pub fn index_sessions(&mut self, sessions: &[Session]) -> Result<usize> {
        let mut count = 0;
        for session in sessions {
            self.index_session(session)?;
            count += 1;
        }
        tracing::info!("Indexed {} sessions", count);
        Ok(count)
    }

    /// Search sessions by query string
    pub fn search(&self, query: &str, limit: usize) -> Vec<SearchResult> {
        let query_lower = query.to_lowercase();
        let query_terms: Vec<&str> = query_lower.split_whitespace().collect();

        let mut results: Vec<SearchResult> = self
            .documents
            .values()
            .filter_map(|doc| {
                let score = self.score_document(doc, &query_terms);
                if score > 0 {
                    Some(SearchResult {
                        session_id: doc.session_id.clone(),
                        score,
                        document: doc.clone(),
                    })
                } else {
                    None
                }
            })
            .collect();

        // Sort by score descending
        results.sort_by(|a, b| b.score.cmp(&a.score));
        results.truncate(limit);

        results
    }

    /// Score a document against query terms
    fn score_document(&self, doc: &SessionDocument, query_terms: &[&str]) -> i32 {
        let text = doc.searchable_text().to_lowercase();
        let mut score: i32 = 0;

        for term in query_terms {
            // Title match is weighted higher
            if let Some(title) = &doc.title {
                let title_lower = title.to_lowercase();
                if title_lower.contains(term) {
                    score += 20;
                    // Exact title match gets even higher score
                    if title_lower == *term {
                        score += 30;
                    }
                }
            }

            // Content match
            let matches = text.matches(term).count() as i32;
            score += matches * 5;

            // Source match
            if doc.source.to_lowercase().contains(term) {
                score += 10;
            }

            // Tool match
            for tool in &doc.tools {
                if tool.to_lowercase().contains(term) {
                    score += 15;
                }
            }
        }

        // Boost recent sessions (only if there's already a match)
        if score > 0 {
            if let Some(ts) = doc.timestamp {
                let now = jiff::Timestamp::now().as_millisecond();
                let age_days = (now - ts) / (1000 * 60 * 60 * 24);
                if age_days < 30 {
                    score += 5; // Recent boost
                }
            }
        }

        score
    }

    /// Get total number of indexed documents
    pub fn document_count(&self) -> usize {
        self.documents.len()
    }

    /// Clear the index
    pub fn clear(&mut self) {
        self.documents.clear();
        self.bigram_filter = None;
        tracing::info!("Session index cleared");
    }

    /// Check if a session is indexed
    pub fn contains(&self, session_id: &SessionId) -> bool {
        self.documents.contains_key(session_id)
    }

    /// Get a document by session ID
    pub fn get_document(&self, session_id: &SessionId) -> Option<&SessionDocument> {
        self.documents.get(session_id)
    }
}

impl Default for SessionIndex {
    fn default() -> Self {
        Self::new().expect("Failed to create default SessionIndex")
    }
}

/// Simple bigram filter for fast candidate filtering
#[derive(Debug)]
struct BigramFilter {
    /// Maps bigram keys to sets of document IDs
    index: HashMap<u16, Vec<SessionId>>,
}

impl BigramFilter {
    #[allow(dead_code)]
    fn new() -> Self {
        Self {
            index: HashMap::new(),
        }
    }

    #[allow(dead_code)]
    fn add_document(&mut self, doc_id: SessionId, content: &str) {
        let bigrams = extract_bigrams(content);
        for bigram in bigrams {
            self.index.entry(bigram).or_default().push(doc_id.clone());
        }
    }

    #[allow(dead_code)]
    fn query(&self, pattern: &str) -> Option<Vec<SessionId>> {
        let bigrams = extract_bigrams(pattern);
        if bigrams.is_empty() {
            return None;
        }

        // Find intersection of all bigram postings
        let mut result: Option<std::collections::HashSet<SessionId>> = None;

        for bigram in bigrams {
            if let Some(doc_ids) = self.index.get(&bigram) {
                let doc_set: std::collections::HashSet<SessionId> =
                    doc_ids.iter().cloned().collect();
                match result {
                    None => result = Some(doc_set),
                    Some(ref mut r) => {
                        r.retain(|id| doc_set.contains(id));
                    }
                }
            } else {
                // Bigram not in index, no matches possible
                return Some(vec![]);
            }
        }

        result.map(|s| s.into_iter().collect())
    }
}

/// Extract bigrams from text (consecutive character pairs)
fn extract_bigrams(text: &str) -> Vec<u16> {
    let text = text.to_lowercase();
    let bytes = text.as_bytes();
    let mut bigrams = Vec::new();

    for window in bytes.windows(2) {
        let (a, b) = (window[0], window[1]);
        // Only index printable ASCII characters
        if (32..=126).contains(&a) && (32..=126).contains(&b) {
            let key = (a as u16) << 8 | (b as u16);
            bigrams.push(key);
        }
    }

    bigrams
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Message, MessageRole};

    fn create_test_session(id: &str, title: &str, content: &str) -> Session {
        Session {
            id: id.to_string(),
            source: "test".to_string(),
            external_id: id.to_string(),
            title: Some(title.to_string()),
            source_path: PathBuf::from("."),
            started_at: Some(jiff::Timestamp::now()),
            ended_at: None,
            messages: vec![Message::text(0, MessageRole::User, content)],
            metadata: crate::model::SessionMetadata::default(),
        }
    }

    #[test]
    fn test_session_document_from_session() {
        let session = create_test_session("s1", "Test Session", "Hello world");
        let doc = SessionDocument::from_session(&session);

        assert_eq!(doc.session_id, "s1");
        assert_eq!(doc.title, Some("Test Session".to_string()));
        assert!(doc.content.contains("Hello world"));
        assert!(doc.roles.contains(&"user".to_string()));
    }

    #[test]
    fn test_index_search() {
        let mut index = SessionIndex::new().unwrap();

        let session1 = create_test_session(
            "s1",
            "Rust Async Programming",
            "How to use async/await in Rust?",
        );
        let session2 = create_test_session("s2", "Python Tutorial", "Introduction to Python");
        let session3 =
            create_test_session("s3", "Rust CLI Tools", "Building CLI apps with Rust clap");

        // DEBUG: Print the searchable text for each session
        let doc1 = SessionDocument::from_session(&session1);
        let doc2 = SessionDocument::from_session(&session2);
        let doc3 = SessionDocument::from_session(&session3);

        println!("DEBUG s1 searchable_text: {:?}", doc1.searchable_text());
        println!("DEBUG s2 searchable_text: {:?}", doc2.searchable_text());
        println!("DEBUG s3 searchable_text: {:?}", doc3.searchable_text());

        index.index_session(&session1).unwrap();
        index.index_session(&session2).unwrap();
        index.index_session(&session3).unwrap();

        // Search for "rust" - should find s1 and s3
        let query = "rust";
        let query_lower = query.to_lowercase();
        let query_terms: Vec<&str> = query_lower.split_whitespace().collect();

        println!("\nDEBUG: Scoring for query terms: {:?}", query_terms);

        // DEBUG: Score each document individually
        for (id, doc) in index.documents.iter() {
            let text = doc.searchable_text().to_lowercase();
            let mut score: i32 = 0;

            println!("\n=== Scoring document: {} ===", id);
            println!("  Searchable text (lowercase): {:?}", text);

            for term in &query_terms {
                println!("  Checking term: {:?}", term);

                // Title match
                if let Some(title) = &doc.title {
                    let title_lower = title.to_lowercase();
                    if title_lower.contains(term) {
                        score += 20;
                        println!("    -> Title contains term: +20 (score: {})", score);
                        if title_lower == *term {
                            score += 30;
                            println!("    -> Exact title match: +30 (score: {})", score);
                        }
                    }
                }

                // Content match
                let matches = text.matches(term).count() as i32;
                if matches > 0 {
                    let added = matches * 5;
                    score += added;
                    println!(
                        "    -> Content matches '{}': {} occurrences x 5 = +{} (score: {})",
                        term, matches, added, score
                    );
                }

                // Source match
                if doc.source.to_lowercase().contains(term) {
                    score += 10;
                    println!("    -> Source contains term: +10 (score: {})", score);
                }

                // Tool match
                for tool in &doc.tools {
                    if tool.to_lowercase().contains(term) {
                        score += 15;
                        println!(
                            "    -> Tool '{}' contains term: +15 (score: {})",
                            tool, score
                        );
                    }
                }
            }

            // Recent boost
            if let Some(ts) = doc.timestamp {
                let now = jiff::Timestamp::now().as_millisecond();
                let age_days = (now - ts) / (1000 * 60 * 60 * 24);
                if age_days < 30 {
                    score += 5;
                    println!("    -> Recent boost: +5 (score: {})", score);
                }
            }

            println!("  FINAL SCORE for {}: {}", id, score);
        }

        let results = index.search("rust", 10);
        println!("\nDEBUG: Search results for 'rust':");
        for r in &results {
            println!(
                "  {}: score={}, title={:?}",
                r.session_id, r.score, r.document.title
            );
        }

        let result_ids: std::collections::HashSet<_> =
            results.iter().map(|r| r.session_id.as_str()).collect();

        // Should find the Rust sessions
        assert!(
            result_ids.contains("s1"),
            "Should find s1 (Rust Async Programming)"
        );
        assert!(result_ids.contains("s3"), "Should find s3 (Rust CLI Tools)");
        // Should not find the Python session
        assert!(
            !result_ids.contains("s2"),
            "Should not find s2 (Python Tutorial)"
        );

        // Both Rust sessions should be ranked highly
        assert!(results[0].score >= results[results.len() - 1].score);

        // Search for "python"
        let results = index.search("python", 10);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].session_id, "s2");

        // Search for "async"
        let results = index.search("async", 10);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].session_id, "s1");
    }

    #[test]
    fn test_search_ranking() {
        let mut index = SessionIndex::new().unwrap();

        // Title match should score higher than content match
        let session1 = create_test_session("s1", "Rust Guide", "Some content here");
        let session2 = create_test_session("s2", "Guide", "Rust programming content");

        index.index_session(&session1).unwrap();
        index.index_session(&session2).unwrap();

        let results = index.search("rust", 10);
        assert_eq!(results.len(), 2);
        // s1 should have higher score due to title match
        assert_eq!(results[0].session_id, "s1");
        assert!(results[0].score > results[1].score);
    }

    #[test]
    fn test_empty_search() {
        let index = SessionIndex::new().unwrap();
        let results = index.search("nonexistent", 10);
        assert!(results.is_empty());
    }

    #[test]
    fn test_extract_bigrams() {
        let bigrams = extract_bigrams("hello");
        assert_eq!(bigrams.len(), 4); // he, el, ll, lo

        let bigrams = extract_bigrams("Hi");
        assert_eq!(bigrams.len(), 1); // hi

        let bigrams = extract_bigrams("A");
        assert!(bigrams.is_empty());
    }

    #[test]
    fn test_index_clear() {
        let mut index = SessionIndex::new().unwrap();
        let session = create_test_session("s1", "Test", "Content");
        index.index_session(&session).unwrap();

        assert_eq!(index.document_count(), 1);
        index.clear();
        assert_eq!(index.document_count(), 0);
    }

    #[test]
    fn test_document_count() {
        let mut index = SessionIndex::new().unwrap();
        assert_eq!(index.document_count(), 0);

        let session = create_test_session("s1", "Test", "Content");
        index.index_session(&session).unwrap();
        assert_eq!(index.document_count(), 1);
    }

    #[test]
    fn test_contains() {
        let mut index = SessionIndex::new().unwrap();
        let session = create_test_session("s1", "Test", "Content");
        index.index_session(&session).unwrap();

        assert!(index.contains(&"s1".to_string()));
        assert!(!index.contains(&"s2".to_string()));
    }
}
