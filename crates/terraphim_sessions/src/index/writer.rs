//! Tantivy index writer for sessions

use crate::index::schema::*;
use crate::model::Session;
use anyhow::Result;
use std::path::Path;
use tantivy::schema::Field;
use tantivy::{Index, IndexWriter, doc};
use tracing::{debug, info};

/// Writer for adding sessions to the Tantivy index
pub struct SessionIndexWriter {
    writer: IndexWriter,
    id_field: Field,
    title_field: Field,
    body_field: Field,
    source_field: Field,
    role_field: Field,
    timestamp_field: Field,
    tags_field: Field,
}

impl SessionIndexWriter {
    /// Create or open an index at the given path
    pub fn open_or_create(path: &Path) -> Result<Self> {
        let schema = get_schema().clone();

        let index = if path.join("meta.json").exists() {
            debug!("Opening existing index at {:?}", path);
            Index::open_in_dir(path)?
        } else {
            info!("Creating new index at {:?}", path);
            std::fs::create_dir_all(path)?;
            Index::create_in_dir(path, schema)?
        };

        let writer = index.writer(50_000_000)?; // 50MB heap

        let id_field = index.schema().get_field(FIELD_ID)?;
        let title_field = index.schema().get_field(FIELD_TITLE)?;
        let body_field = index.schema().get_field(FIELD_BODY)?;
        let source_field = index.schema().get_field(FIELD_SOURCE)?;
        let role_field = index.schema().get_field(FIELD_ROLE)?;
        let timestamp_field = index.schema().get_field(FIELD_TIMESTAMP)?;
        let tags_field = index.schema().get_field(FIELD_TAGS)?;

        Ok(Self {
            writer,
            id_field,
            title_field,
            body_field,
            source_field,
            role_field,
            timestamp_field,
            tags_field,
        })
    }

    /// Add a single session to the index
    pub fn add_session(&mut self, session: &Session) -> Result<()> {
        let mut doc = doc!();

        doc.add_text(self.id_field, &session.id);

        if let Some(title) = &session.title {
            doc.add_text(self.title_field, title);
        }

        // Build body from messages
        let body = session
            .messages
            .iter()
            .map(|m| format!("{:?}: {}", m.role, m.content))
            .collect::<Vec<_>>()
            .join("\n");
        doc.add_text(self.body_field, &body);

        doc.add_text(self.source_field, &session.source);

        if let Some(path) = session.metadata.project_path.as_ref() {
            doc.add_text(self.role_field, path);
        }

        if let Some(ts) = session.started_at {
            doc.add_date(
                self.timestamp_field,
                tantivy::DateTime::from_timestamp_secs(ts.as_second()),
            );
        }

        let tags = session.metadata.tags.join(", ");
        if !tags.is_empty() {
            doc.add_text(self.tags_field, &tags);
        }

        self.writer.add_document(doc)?;
        Ok(())
    }

    /// Add multiple sessions (batch)
    pub fn add_sessions(&mut self, sessions: &[Session]) -> Result<usize> {
        let count = sessions.len();
        for session in sessions {
            self.add_session(session)?;
        }
        debug!("Added {} sessions to index", count);
        Ok(count)
    }

    /// Commit pending changes
    pub fn commit(&mut self) -> Result<()> {
        self.writer.commit()?;
        info!("Index commit complete");
        Ok(())
    }

    /// Commit and return the number of indexed documents
    pub fn commit_with_count(&mut self) -> Result<u64> {
        let opstamp = self.writer.commit()?;
        info!("Index commit complete (opstamp: {})", opstamp);
        Ok(opstamp)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
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
    fn test_create_index() {
        let temp_dir = TempDir::new().unwrap();
        let writer = SessionIndexWriter::open_or_create(temp_dir.path());
        assert!(writer.is_ok());
    }

    #[test]
    fn test_add_and_commit_session() {
        let temp_dir = TempDir::new().unwrap();
        let mut writer = SessionIndexWriter::open_or_create(temp_dir.path()).unwrap();

        let session = make_test_session("s1", "Test Session", "Hello world");
        writer.add_session(&session).unwrap();
        writer.commit().unwrap();
    }
}
