//! BM25-ranked session search adapter
//!
//! Converts sessions into `terraphim_types::Document` instances and uses
//! the existing BM25 scoring infrastructure for ranked full-text search.

use crate::model::{MessageRole, Session};
use terraphim_types::score::{OkapiBM25Scorer, Query, QueryScorer, Scored, Scorer, SearchResults};
use terraphim_types::{Document, DocumentType};

const MAX_BODY_LENGTH: usize = 50_000;
const MAX_SEARCH_RESULTS: usize = 50;
const MIN_SCORE_FRACTION: f64 = 0.1;

/// Adapter that converts a `Session` into a searchable `Document`.
pub fn session_to_document(session: &Session) -> Document {
    let title = session
        .title
        .clone()
        .unwrap_or_else(|| session.source.clone());

    let body = build_body(session);

    Document {
        id: session.id.clone(),
        url: session.source_path.to_string_lossy().to_string(),
        title,
        body,
        description: session.summary(),
        summarization: None,
        stub: None,
        tags: if session.metadata.tags.is_empty() {
            None
        } else {
            Some(session.metadata.tags.clone())
        },
        rank: None,
        source_haystack: Some(session.source.clone()),
        doc_type: DocumentType::default(),
        synonyms: None,
        route: None,
        priority: None,
    }
}

fn build_body(session: &Session) -> String {
    let mut parts: Vec<String> = Vec::new();

    if let Some(path) = &session.metadata.project_path {
        parts.push(path.clone());
    }

    if let Some(model) = &session.metadata.model {
        parts.push(model.clone());
    }

    for msg in &session.messages {
        if msg.content.is_empty() {
            continue;
        }
        let prefix = match msg.role {
            MessageRole::User => "User: ",
            MessageRole::Assistant => "Assistant: ",
            MessageRole::System => "System: ",
            MessageRole::Tool => "Tool: ",
            MessageRole::Other => "",
        };
        parts.push(format!("{}{}", prefix, msg.content));
    }

    let body = parts.join("\n");
    if body.len() > MAX_BODY_LENGTH {
        body[..MAX_BODY_LENGTH].to_string()
    } else {
        body
    }
}

/// Perform BM25-ranked search over sessions.
///
/// Returns sessions ranked by relevance to the query, with BM25 scoring
/// applied to the combined title + message body text.
pub fn search_sessions(sessions: &[Session], query: &str) -> Vec<Scored<Session>> {
    if query.trim().is_empty() || sessions.is_empty() {
        return Vec::new();
    }

    let documents: Vec<Document> = sessions.iter().map(session_to_document).collect();

    let mut bm25 = OkapiBM25Scorer::new();
    bm25.initialize(&documents);

    let mut q = Query::new(query);
    q.name_scorer = QueryScorer::BM25;
    q.size = MAX_SEARCH_RESULTS;

    let mut scorer = Scorer::new()
        .with_scorer(Box::new(bm25))
        .with_similarity(terraphim_types::score::Similarity::None);

    let results: SearchResults<Document> = match scorer.score(&q, documents) {
        Ok(r) => r,
        Err(_) => return Vec::new(),
    };

    let session_map: std::collections::HashMap<&str, &Session> =
        sessions.iter().map(|s| (s.id.as_str(), s)).collect();

    let mut scored: Vec<Scored<Session>> = results
        .as_slice()
        .iter()
        .filter_map(|scored_doc| {
            let score = scored_doc.score();
            session_map
                .get(scored_doc.value().id.as_str())
                .map(|session| Scored::new((*session).clone()).with_score(score))
        })
        .collect();

    scored.sort_by(|a, b| {
        b.score()
            .partial_cmp(&a.score())
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    if let Some(top_score) = scored.first().map(|s| s.score()) {
        if top_score > 0.0 {
            let threshold = top_score * MIN_SCORE_FRACTION;
            scored.retain(|s| s.score() >= threshold);
        }
    }

    scored
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Message, SessionMetadata};
    use std::path::PathBuf;

    fn make_session(id: &str, title: &str, messages: Vec<(&str, MessageRole, &str)>) -> Session {
        Session {
            id: id.to_string(),
            source: "test".to_string(),
            external_id: id.to_string(),
            title: if title.is_empty() {
                None
            } else {
                Some(title.to_string())
            },
            source_path: PathBuf::from(format!("/sessions/{}.jsonl", id)),
            started_at: None,
            ended_at: None,
            messages: messages
                .into_iter()
                .enumerate()
                .map(|(i, (role, role_type, content))| {
                    let mut msg = Message::text(i, role_type, content);
                    msg.author = Some(role.to_string());
                    msg
                })
                .collect(),
            metadata: SessionMetadata::default(),
        }
    }

    #[test]
    fn test_session_to_document_basic() {
        let session = make_session(
            "s1",
            "Rust async help",
            vec![("user", MessageRole::User, "How do I use tokio?")],
        );
        let doc = session_to_document(&session);

        assert_eq!(doc.id, "s1");
        assert_eq!(doc.title, "Rust async help");
        assert!(doc.body.contains("How do I use tokio?"));
        assert!(doc.body.contains("User: "));
        assert_eq!(doc.source_haystack, Some("test".to_string()));
    }

    #[test]
    fn test_session_to_document_no_title() {
        let session = make_session("s2", "", vec![]);
        let doc = session_to_document(&session);

        assert_eq!(doc.title, "test");
    }

    #[test]
    fn test_session_to_document_tags() {
        let mut session = make_session("s3", "test", vec![]);
        session.metadata.tags = vec!["rust".to_string(), "async".to_string()];
        let doc = session_to_document(&session);

        assert_eq!(
            doc.tags,
            Some(vec!["rust".to_string(), "async".to_string()])
        );
    }

    #[test]
    fn test_search_sessions_basic() {
        let sessions = vec![
            make_session(
                "s1",
                "Rust async programming",
                vec![("user", MessageRole::User, "How to use async await in Rust?")],
            ),
            make_session(
                "s2",
                "Python web scraping",
                vec![("user", MessageRole::User, "Best library for web scraping?")],
            ),
            make_session(
                "s3",
                "Rust error handling",
                vec![("user", MessageRole::User, "How to handle errors in Rust?")],
            ),
        ];

        let results = search_sessions(&sessions, "Rust async");

        assert!(!results.is_empty());
        assert_eq!(results[0].value().id, "s1");
    }

    #[test]
    fn test_search_sessions_empty_query() {
        let sessions = vec![make_session("s1", "test", vec![])];
        let results = search_sessions(&sessions, "");
        assert!(results.is_empty());
    }

    #[test]
    fn test_search_sessions_empty_sessions() {
        let results = search_sessions(&[], "test query");
        assert!(results.is_empty());
    }

    #[test]
    fn test_search_sessions_ranking_order() {
        let sessions = vec![
            make_session(
                "s1",
                "Rust async",
                vec![
                    ("user", MessageRole::User, "Rust async Rust async Rust"),
                    ("assistant", MessageRole::Assistant, "async rust patterns"),
                ],
            ),
            make_session(
                "s2",
                "General programming",
                vec![("user", MessageRole::User, "What is async?")],
            ),
            make_session(
                "s3",
                "Unrelated",
                vec![("user", MessageRole::User, "How to bake bread?")],
            ),
        ];

        let results = search_sessions(&sessions, "Rust async");

        assert!(!results.is_empty());
        assert!(results.len() <= 3);

        for window in results.windows(2) {
            assert!(window[0].score() >= window[1].score());
        }
    }

    #[test]
    fn test_build_body_truncation() {
        let long_content = "x".repeat(60_000);
        let session = make_session(
            "s1",
            "test",
            vec![("user", MessageRole::User, long_content.as_str())],
        );

        let body = build_body(&session);
        assert_eq!(body.len(), MAX_BODY_LENGTH);
    }

    #[test]
    fn test_build_body_includes_metadata() {
        let mut session = make_session("s1", "test", vec![]);
        session.metadata.project_path = Some("/my/project".to_string());
        session.metadata.model = Some("claude-3".to_string());

        let body = build_body(&session);
        assert!(body.contains("/my/project"));
        assert!(body.contains("claude-3"));
    }
}
