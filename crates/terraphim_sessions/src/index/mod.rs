//! Tantivy full-text index for session search.
//!
//! Performance target: <100ms for 10,000 sessions.

use crate::model::{ContentBlock, Session};
use anyhow::{Context, Result};
use std::path::Path;
use tantivy::{
    Index, IndexReader, IndexWriter, ReloadPolicy, TantivyDocument, Term,
    collector::TopDocs,
    query::QueryParser,
    schema::{
        FAST, Field, IndexRecordOption, STORED, STRING, Schema, TEXT, TextFieldIndexing,
        TextOptions, Value,
    },
    tokenizer::{NgramTokenizer, TextAnalyzer},
};

const CODE_NGRAM_TOKENIZER: &str = "code_ngram";
const DEFAULT_HEAP_SIZE: usize = 50_000_000;
const MAX_SEARCH_RESULTS: usize = 50;
const CODE_NGRAM_MIN: usize = 2;
const CODE_NGRAM_MAX: usize = 20;

struct IndexFields {
    session_id: Field,
    source: Field,
    title: Field,
    body_text: Field,
    code_text: Field,
    timestamp_secs: Field,
}

/// A search result returned by [`SessionIndex::search`].
#[derive(Debug, Clone)]
pub struct IndexSearchResult {
    pub session_id: String,
    pub source: String,
    pub title: String,
    pub score: f32,
}

/// Tantivy-backed full-text index for session search.
pub struct SessionIndex {
    index: Index,
    reader: IndexReader,
    fields: IndexFields,
    schema: Schema,
}

impl SessionIndex {
    pub fn create_in_ram() -> Result<Self> {
        let (schema, fields) = build_schema();
        let index = Index::create_in_ram(schema.clone());
        register_tokenizers(&index);
        let reader = index
            .reader_builder()
            .reload_policy(ReloadPolicy::OnCommitWithDelay)
            .try_into()
            .context("failed to create index reader")?;
        Ok(Self {
            index,
            reader,
            fields,
            schema,
        })
    }

    pub fn open_or_create(path: &Path) -> Result<Self> {
        let (schema, fields) = build_schema();
        let index = if path.join("meta.json").exists() {
            Index::open_in_dir(path).context("failed to open existing index")?
        } else {
            std::fs::create_dir_all(path).context("failed to create index directory")?;
            Index::create_in_dir(path, schema.clone())
                .context("failed to create index in directory")?
        };
        register_tokenizers(&index);
        let reader = index
            .reader_builder()
            .reload_policy(ReloadPolicy::OnCommitWithDelay)
            .try_into()
            .context("failed to create index reader")?;
        Ok(Self {
            index,
            reader,
            fields,
            schema,
        })
    }

    fn make_writer(&self) -> Result<IndexWriter> {
        self.index
            .writer(DEFAULT_HEAP_SIZE)
            .context("failed to acquire index writer")
    }

    pub fn add_session(&self, session: &Session) -> Result<()> {
        let mut w = self.make_writer()?;
        w.add_document(session_to_doc(session, &self.fields))?;
        w.commit().context("commit failed")?;
        Ok(())
    }

    pub fn add_sessions(&self, sessions: &[Session]) -> Result<()> {
        let mut w = self.make_writer()?;
        for s in sessions {
            w.add_document(session_to_doc(s, &self.fields))?;
        }
        w.commit().context("batch commit failed")?;
        Ok(())
    }

    pub fn update_session(&self, session: &Session) -> Result<()> {
        let mut w = self.make_writer()?;
        w.delete_term(Term::from_field_text(self.fields.session_id, &session.id));
        w.add_document(session_to_doc(session, &self.fields))?;
        w.commit().context("update commit failed")?;
        Ok(())
    }

    pub fn delete_session(&self, session_id: &str) -> Result<()> {
        let mut w = self.make_writer()?;
        w.delete_term(Term::from_field_text(self.fields.session_id, session_id));
        w.commit().context("delete commit failed")?;
        Ok(())
    }

    pub fn search(&self, query: &str) -> Result<Vec<IndexSearchResult>> {
        self.search_with_limit(query, MAX_SEARCH_RESULTS)
    }

    pub fn search_with_limit(&self, query: &str, limit: usize) -> Result<Vec<IndexSearchResult>> {
        self.reader.reload().context("reader reload failed")?;
        let searcher = self.reader.searcher();
        let mut qp = QueryParser::for_index(
            &self.index,
            vec![
                self.fields.title,
                self.fields.body_text,
                self.fields.code_text,
            ],
        );
        qp.set_field_boost(self.fields.title, 3.0);
        let q = match qp.parse_query(query) {
            Ok(q) => q,
            Err(_) => qp.parse_query_lenient(query).0,
        };
        let top = searcher.search(&q, &TopDocs::with_limit(limit))?;
        let mut out = Vec::with_capacity(top.len());
        for (score, addr) in top {
            let doc: TantivyDocument = searcher.doc(addr)?;
            out.push(IndexSearchResult {
                session_id: get_text(&doc, self.fields.session_id),
                source: get_text(&doc, self.fields.source),
                title: get_text(&doc, self.fields.title),
                score,
            });
        }
        Ok(out)
    }

    pub fn doc_count(&self) -> Result<u64> {
        self.reader.reload()?;
        Ok(self.reader.searcher().num_docs())
    }
}

fn build_schema() -> (Schema, IndexFields) {
    let mut b = Schema::builder();
    let session_id = b.add_text_field("session_id", STRING | STORED);
    let source = b.add_text_field("source", STRING | STORED);
    let title = b.add_text_field("title", TEXT | STORED);
    let body_text = b.add_text_field("body_text", TEXT | STORED);
    let code_opts = TextOptions::default()
        .set_indexing_options(
            TextFieldIndexing::default()
                .set_tokenizer(CODE_NGRAM_TOKENIZER)
                .set_index_option(IndexRecordOption::Basic),
        )
        .set_stored();
    let code_text = b.add_text_field("code_text", code_opts);
    let timestamp_secs = b.add_u64_field("timestamp_secs", STORED | FAST);
    let schema = b.build();
    let fields = IndexFields {
        session_id,
        source,
        title,
        body_text,
        code_text,
        timestamp_secs,
    };
    (schema, fields)
}

fn register_tokenizers(index: &Index) {
    let tok = TextAnalyzer::builder(
        NgramTokenizer::new(CODE_NGRAM_MIN, CODE_NGRAM_MAX, true).expect("valid ngram params"),
    )
    .build();
    index.tokenizers().register(CODE_NGRAM_TOKENIZER, tok);
}

fn session_to_doc(s: &Session, f: &IndexFields) -> TantivyDocument {
    let mut doc = TantivyDocument::default();
    doc.add_text(f.session_id, &s.id);
    doc.add_text(f.source, &s.source);
    doc.add_text(f.title, s.title.as_deref().unwrap_or(&s.source));
    doc.add_text(f.body_text, &build_body(s));
    let code = extract_code(s);
    if !code.is_empty() {
        doc.add_text(f.code_text, &code);
    }
    if let Some(ts) = s.started_at {
        doc.add_u64(f.timestamp_secs, ts.as_second().max(0) as u64);
    }
    doc
}

fn build_body(s: &Session) -> String {
    let mut parts: Vec<&str> = Vec::new();
    if let Some(p) = &s.metadata.project_path {
        parts.push(p.as_str());
    }
    if let Some(m) = &s.metadata.model {
        parts.push(m.as_str());
    }
    parts.extend(
        s.messages
            .iter()
            .filter(|m| !m.content.is_empty())
            .map(|m| m.content.as_str()),
    );
    parts.join("\n")
}

fn extract_code(s: &Session) -> String {
    let mut parts: Vec<String> = Vec::new();
    for msg in &s.messages {
        for block in &msg.blocks {
            match block {
                ContentBlock::ToolUse { name, input, .. } => {
                    parts.push(name.clone());
                    if let Some(v) = input.get("command").and_then(|v| v.as_str()) {
                        parts.push(v.to_string());
                    }
                    if let Some(v) = input.get("path").and_then(|v| v.as_str()) {
                        parts.push(v.to_string());
                    }
                }
                ContentBlock::ToolResult { content, .. } if content.len() < 2000 => {
                    parts.push(content.clone());
                }
                _ => {}
            }
        }
    }
    parts.join("\n")
}

fn get_text(doc: &TantivyDocument, field: Field) -> String {
    doc.get_first(field)
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Message, MessageRole, Session, SessionMetadata};
    use std::path::PathBuf;
    use std::time::Instant;

    fn sess(id: &str, title: &str, msgs: &[(&str, MessageRole, &str)]) -> Session {
        Session {
            id: id.to_string(),
            source: "test".to_string(),
            external_id: id.to_string(),
            title: Some(title.to_string()),
            source_path: PathBuf::from("/tmp"),
            started_at: None,
            ended_at: None,
            messages: msgs
                .iter()
                .enumerate()
                .map(|(i, (_, r, c))| Message {
                    idx: i,
                    role: r.clone(),
                    author: None,
                    content: c.to_string(),
                    blocks: vec![ContentBlock::Text {
                        text: c.to_string(),
                    }],
                    created_at: None,
                    extra: serde_json::Value::Null,
                })
                .collect(),
            metadata: SessionMetadata::new(None, None, vec![], serde_json::Value::Null),
        }
    }

    #[test]
    fn test_create_in_ram() {
        assert_eq!(
            SessionIndex::create_in_ram().unwrap().doc_count().unwrap(),
            0
        );
    }

    #[test]
    fn test_add_session_increments_count() {
        let idx = SessionIndex::create_in_ram().unwrap();
        idx.add_session(&sess("s1", "Rust async", &[])).unwrap();
        assert_eq!(idx.doc_count().unwrap(), 1);
    }

    #[test]
    fn test_add_sessions_batch() {
        let idx = SessionIndex::create_in_ram().unwrap();
        let sessions: Vec<_> = (0..5)
            .map(|i| sess(&format!("s{i}"), &format!("S{i}"), &[]))
            .collect();
        idx.add_sessions(&sessions).unwrap();
        assert_eq!(idx.doc_count().unwrap(), 5);
    }

    #[test]
    fn test_search_returns_relevant_result() {
        let idx = SessionIndex::create_in_ram().unwrap();
        idx.add_sessions(&[
            sess(
                "s1",
                "Rust async",
                &[("u", MessageRole::User, "tokio async runtime")],
            ),
            sess(
                "s2",
                "Python scraping",
                &[("u", MessageRole::User, "BeautifulSoup html")],
            ),
        ])
        .unwrap();
        let r = idx.search("tokio async Rust").unwrap();
        assert!(!r.is_empty());
        assert_eq!(r[0].session_id, "s1");
    }

    #[test]
    fn test_search_empty_index() {
        let idx = SessionIndex::create_in_ram().unwrap();
        assert!(idx.search("query").unwrap().is_empty());
    }

    #[test]
    fn test_update_replaces_document() {
        let idx = SessionIndex::create_in_ram().unwrap();
        idx.add_session(&sess(
            "s1",
            "Original",
            &[("u", MessageRole::User, "original content")],
        ))
        .unwrap();
        idx.update_session(&sess(
            "s1",
            "Updated",
            &[("u", MessageRole::User, "updated different")],
        ))
        .unwrap();
        assert_eq!(idx.doc_count().unwrap(), 1);
        assert!(!idx.search("updated different").unwrap().is_empty());
        assert!(idx.search("original content").unwrap().is_empty());
    }

    #[test]
    fn test_delete_removes_document() {
        let idx = SessionIndex::create_in_ram().unwrap();
        idx.add_session(&sess("s1", "Delete me", &[])).unwrap();
        idx.delete_session("s1").unwrap();
        assert_eq!(idx.doc_count().unwrap(), 0);
    }

    #[test]
    fn test_search_limit() {
        let idx = SessionIndex::create_in_ram().unwrap();
        let sessions: Vec<_> = (0..20)
            .map(|i| {
                sess(
                    &format!("s{i}"),
                    &format!("Rust {i}"),
                    &[("u", MessageRole::User, "Rust async patterns")],
                )
            })
            .collect();
        idx.add_sessions(&sessions).unwrap();
        assert!(idx.search_with_limit("Rust", 5).unwrap().len() <= 5);
    }

    #[test]
    fn test_results_descending_by_score() {
        let idx = SessionIndex::create_in_ram().unwrap();
        idx.add_sessions(&[
            sess(
                "hi",
                "Rust Rust Rust tokio",
                &[("u", MessageRole::User, "Rust tokio Rust tokio Rust")],
            ),
            sess("lo", "Other", &[("u", MessageRole::User, "Rust once")]),
        ])
        .unwrap();
        let r = idx.search("Rust tokio").unwrap();
        assert!(r.len() >= 2);
        for w in r.windows(2) {
            assert!(w[0].score >= w[1].score);
        }
    }

    #[test]
    fn test_persistent_index() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("idx");
        {
            let idx = SessionIndex::open_or_create(&path).unwrap();
            idx.add_session(&sess("s1", "Persistent", &[])).unwrap();
            assert_eq!(idx.doc_count().unwrap(), 1);
        }
        {
            let idx = SessionIndex::open_or_create(&path).unwrap();
            assert_eq!(idx.doc_count().unwrap(), 1);
        }
    }

    #[test]
    fn test_malformed_query_does_not_panic() {
        let idx = SessionIndex::create_in_ram().unwrap();
        idx.add_session(&sess("s1", "Test", &[])).unwrap();
        let _ = idx.search("AND OR NOT (((");
        let _ = idx.search("");
        let _ = idx.search("*");
    }

    #[test]
    fn test_performance_10k_sessions() {
        let idx = SessionIndex::create_in_ram().unwrap();
        let sessions: Vec<_> = (0..10_000)
            .map(|i| {
                sess(
                    &format!("p{i}"),
                    &format!("Session Rust async {i}"),
                    &[
                        ("u", MessageRole::User, "How to use tokio async streams?"),
                        (
                            "a",
                            MessageRole::Assistant,
                            "Use tokio::sync::mpsc for async message passing.",
                        ),
                    ],
                )
            })
            .collect();
        idx.add_sessions(&sessions).unwrap();
        assert_eq!(idx.doc_count().unwrap(), 10_000);
        let t0 = Instant::now();
        let r = idx.search("tokio async streams").unwrap();
        let ms = t0.elapsed().as_millis();
        assert!(!r.is_empty());
        assert!(
            ms < 100,
            "search over 10k sessions took {}ms, expected <100ms",
            ms
        );
    }
}
