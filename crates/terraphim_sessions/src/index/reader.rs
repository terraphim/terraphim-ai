//! Tantivy index reader and search for sessions

use crate::index::schema::*;
use anyhow::Result;
use std::path::Path;
use tantivy::{
    Index, IndexReader, TantivyDocument, collector::TopDocs, query::QueryParser, schema::Value,
};
use tracing::{debug, info};

/// Single search result with score
#[derive(Debug, Clone)]
pub struct SessionSearchResult {
    /// Session ID
    pub id: String,
    /// Search relevance score
    pub score: f32,
    /// Session title (if available)
    pub title: Option<String>,
    /// Session source
    pub source: String,
}

/// Reader for searching the session index
pub struct SessionIndex {
    index: Index,
    reader: IndexReader,
    query_parser: QueryParser,
}

impl SessionIndex {
    /// Open an existing index
    pub fn open(path: &Path) -> Result<Self> {
        let index = Index::open_in_dir(path)?;
        let reader = index.reader()?;

        let schema = index.schema();
        let default_fields = vec![
            schema.get_field(FIELD_TITLE)?,
            schema.get_field(FIELD_BODY)?,
            schema.get_field(FIELD_TAGS)?,
        ];

        let query_parser = QueryParser::for_index(&index, default_fields);

        info!("Opened session index at {:?}", path);

        Ok(Self {
            index,
            reader,
            query_parser,
        })
    }

    /// Create a new index (or open existing)
    pub fn create_or_open(path: &Path) -> Result<Self> {
        if path.exists() {
            Self::open(path)
        } else {
            let schema = get_schema().clone();
            std::fs::create_dir_all(path)?;
            let index = Index::create_in_dir(path, schema)?;
            let reader = index.reader()?;

            let default_fields = vec![
                index.schema().get_field(FIELD_TITLE)?,
                index.schema().get_field(FIELD_BODY)?,
                index.schema().get_field(FIELD_TAGS)?,
            ];

            let query_parser = QueryParser::for_index(&index, default_fields);

            info!("Created new session index at {:?}", path);

            Ok(Self {
                index,
                reader,
                query_parser,
            })
        }
    }

    /// Search sessions with a query string
    pub fn search(&self, query: &str, limit: usize) -> Result<Vec<SessionSearchResult>> {
        let searcher = self.reader.searcher();
        let parsed_query = self.query_parser.parse_query(query)?;

        debug!("Searching for: '{}' (limit: {})", query, limit);

        let top_docs = searcher.search(&parsed_query, &TopDocs::with_limit(limit))?;

        let mut results = Vec::new();
        for (score, doc_address) in top_docs {
            let doc: TantivyDocument = searcher.doc(doc_address)?;

            let id = doc
                .get_first(self.index.schema().get_field(FIELD_ID)?)
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();

            let title = doc
                .get_first(self.index.schema().get_field(FIELD_TITLE)?)
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());

            let source = doc
                .get_first(self.index.schema().get_field(FIELD_SOURCE)?)
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();

            results.push(SessionSearchResult {
                id,
                score,
                title,
                source,
            });
        }

        info!("Search returned {} results", results.len());
        Ok(results)
    }

    /// Get the number of indexed documents
    pub fn doc_count(&self) -> usize {
        self.reader.searcher().num_docs() as usize
    }

    /// Reload the index to pick up new commits
    pub fn reload(&self) -> Result<()> {
        self.reader.reload()?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Session;
    use crate::index::writer::SessionIndexWriter;
    use crate::model::{Message, MessageRole, SessionMetadata};
    use tempfile::TempDir;

    fn make_test_session(id: &str, title: &str, content: &str) -> Session {
        Session {
            id: id.to_string(),
            source: "test".to_string(),
            external_id: id.to_string(),
            title: Some(title.to_string()),
            source_path: std::path::PathBuf::from(format!("/tmp/{}.jsonl", id)),
            started_at: None,
            ended_at: None,
            messages: vec![Message::text(0, MessageRole::User, content)],
            metadata: SessionMetadata::default(),
        }
    }

    #[test]
    fn test_search_index() {
        let temp_dir = TempDir::new().unwrap();

        // Write
        {
            let mut writer = SessionIndexWriter::open_or_create(temp_dir.path()).unwrap();
            writer
                .add_session(&make_test_session("s1", "Rust async", "How to use tokio?"))
                .unwrap();
            writer
                .add_session(&make_test_session("s2", "Python web", "Flask vs Django"))
                .unwrap();
            writer.commit().unwrap();
        }

        // Search
        let index = SessionIndex::open(temp_dir.path()).unwrap();
        let results = index.search("tokio", 10).unwrap();

        assert!(!results.is_empty());
        assert_eq!(results[0].id, "s1");
    }

    #[test]
    fn test_doc_count() {
        let temp_dir = TempDir::new().unwrap();

        {
            let mut writer = SessionIndexWriter::open_or_create(temp_dir.path()).unwrap();
            writer
                .add_session(&make_test_session("s1", "Test", "content"))
                .unwrap();
            writer.commit().unwrap();
        }

        let index = SessionIndex::open(temp_dir.path()).unwrap();
        assert_eq!(index.doc_count(), 1);
    }
}
