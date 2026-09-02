//! Pure mapping functions: `LinearIssue` → `terraphim_types::Document`.
//!
//! These functions are pure (no I/O), so they're the easiest to test in
//! isolation. The actual GraphQL fetch lives in `client.rs`; the
//! `HaystackProvider` impl in `lib.rs` wires them together.

use terraphim_types::{Document, DocumentType};

use crate::client::LinearIssue;

/// Maximum number of comments included in a Document's body. More comments
/// per issue would blow up the body size for haystack ranking — the haystack
/// is meant to surface relevance, not duplicate the full Linear discussion.
pub const DEFAULT_COMMENT_LIMIT: usize = 3;

/// Translate a `SearchQuery` into a Linear identifier that we can use for
/// the `searchable_content` filter. We use the `search_term` directly; the
/// haystack ranker is expected to have already normalised this.
///
/// Linear's `searchableContent` is documented to match title + description
/// (verified via API probes 2026-07-29 — Spike 1 in the design docs).
pub fn build_searchable_content(query: &str) -> Option<String> {
    let trimmed = query.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

/// Translate one Linear issue into a `terraphim_types::Document`.
///
/// Mapping:
/// - `id` = issue identifier (e.g. "ODITECH-580")
/// - `url` = canonical Linear URL
/// - `title` = "[IDENTIFIER] title"
/// - `body` = "## Description\n\n<description>\n\n## Comments (top N)\n\n..."
/// - `tags` = [team_key, state_name, priority_label, ...labels]
/// - `description` = the issue summary (short, used as haystack summary)
/// - `source_haystack` = Some("linear:<team_key>")
/// - `rank` = None (let the rolegraph ranker set it)
/// - `stub` = Some(short description)
pub fn issue_to_document(issue: &LinearIssue) -> Document {
    let mut body = String::new();
    if let Some(desc) = &issue.description
        && !desc.is_empty()
    {
        body.push_str("## Description\n\n");
        body.push_str(desc);
        body.push_str("\n\n");
    }
    if !issue.comments.is_empty() {
        body.push_str(&format!(
            "## Comments (top {})\n\n",
            issue.comments.len().min(DEFAULT_COMMENT_LIMIT)
        ));
        for (i, c) in issue
            .comments
            .iter()
            .take(DEFAULT_COMMENT_LIMIT)
            .enumerate()
        {
            body.push_str(&format!(
                "### Comment {} by {}\n{}\n\n",
                i + 1,
                c.user_name.as_deref().unwrap_or("Unknown"),
                c.body
            ));
        }
    }
    if body.is_empty() {
        body = "(no description or comments)".to_string();
    }

    let mut tags: Vec<String> = Vec::new();
    if let Some(team) = &issue.team_key {
        tags.push(team.clone());
    }
    if let Some(state) = &issue.state_name {
        tags.push(state.clone());
    }
    tags.push(priority_label(issue.priority).to_string());
    for label in &issue.labels {
        tags.push(label.clone());
    }

    let description = issue.description.as_deref().map(|d| {
        if d.len() > 200 {
            format!("{}…", &d[..200])
        } else {
            d.to_string()
        }
    });

    let source_haystack = issue.team_key.as_deref().map(|t| format!("linear:{}", t));

    Document {
        id: issue.identifier.clone(),
        url: issue.url.clone(),
        title: format!("[{}] {}", issue.identifier, issue.title),
        body,
        description,
        summarization: None,
        stub: issue.description.as_deref().map(|d| truncate(d, 80)),
        tags: Some(tags),
        rank: None,
        source_haystack,
        doc_type: DocumentType::Document,
        synonyms: None,
        route: None,
        priority: None,
        quality_score: None,
    }
}

fn priority_label(priority: i32) -> &'static str {
    match priority {
        1 => "P1-urgent",
        2 => "P2-high",
        3 => "P3-medium",
        4 => "P4-low",
        _ => "P0-none",
    }
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        let cut = s.char_indices().nth(max).map(|(i, _)| i).unwrap_or(s.len());
        format!("{}…", &s[..cut])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::client::{LinearComment, LinearIssue};

    fn sample_issue() -> LinearIssue {
        LinearIssue {
            id: "uuid-1".into(),
            identifier: "ODITECH-580".into(),
            title: "fix(docker): mentor-service build fails".into(),
            description: Some("## Problem\n\nThe CI build fails at compile time.".into()),
            url: "https://linear.app/zestic-ai/issue/ODITECH-580".into(),
            priority: 2,
            state_name: Some("Done".into()),
            state_type: Some("completed".into()),
            assignee_name: Some("Alex".into()),
            team_key: Some("ODITECH".into()),
            team_name: Some("Odilo-Tech".into()),
            labels: vec!["bug".into(), "infra".into()],
            comments: vec![LinearComment {
                id: "c1".into(),
                body: "Fixed in PR #1234".into(),
                user_name: Some("Alex".into()),
                created_at: "2026-07-29T00:00:00Z".into(),
            }],
        }
    }

    #[test]
    fn build_searchable_content_trims_whitespace() {
        assert_eq!(
            build_searchable_content("  pgvector  "),
            Some("pgvector".into())
        );
        assert_eq!(build_searchable_content(""), None);
        assert_eq!(build_searchable_content("   "), None);
    }

    #[test]
    fn issue_to_document_basic_fields() {
        let doc = issue_to_document(&sample_issue());
        assert_eq!(doc.id, "ODITECH-580");
        assert_eq!(doc.url, "https://linear.app/zestic-ai/issue/ODITECH-580");
        assert_eq!(
            doc.title,
            "[ODITECH-580] fix(docker): mentor-service build fails"
        );
        assert_eq!(doc.source_haystack.as_deref(), Some("linear:ODITECH"));
    }

    #[test]
    fn issue_to_document_body_includes_description_and_comments() {
        let doc = issue_to_document(&sample_issue());
        assert!(
            doc.body.contains("## Description"),
            "body should have Description header"
        );
        assert!(
            doc.body.contains("CI build fails at compile time"),
            "body should have description text"
        );
        assert!(
            doc.body.contains("## Comments (top 1)"),
            "body should have Comments header"
        );
        assert!(
            doc.body.contains("Fixed in PR #1234"),
            "body should have comment body"
        );
    }

    #[test]
    fn issue_to_document_tags_include_team_state_priority_labels() {
        let doc = issue_to_document(&sample_issue());
        let tags = doc.tags.expect("tags should be set");
        assert!(tags.contains(&"ODITECH".to_string()));
        assert!(tags.contains(&"Done".to_string()));
        assert!(tags.contains(&"P2-high".to_string()));
        assert!(tags.contains(&"bug".to_string()));
        assert!(tags.contains(&"infra".to_string()));
    }

    #[test]
    fn issue_to_document_handles_missing_description_and_comments() {
        let mut issue = sample_issue();
        issue.description = None;
        issue.comments = vec![];
        let doc = issue_to_document(&issue);
        assert!(doc.body.contains("(no description or comments)"));
        assert!(doc.description.is_none());
        assert!(doc.stub.is_none());
    }

    #[test]
    fn issue_to_document_stub_truncates_long_descriptions() {
        let mut issue = sample_issue();
        issue.description = Some("a".repeat(500));
        let doc = issue_to_document(&issue);
        let stub = doc.stub.expect("stub should be set");
        // Contract: 80 chars + ellipsis suffix = 81 chars max.
        let chars: Vec<char> = stub.chars().collect();
        assert!(
            chars.len() <= 81,
            "stub should be ~80 chars + ellipsis, got {}",
            chars.len()
        );
        assert!(stub.ends_with('…'));
    }

    #[test]
    fn issue_to_document_description_truncates_at_200() {
        let mut issue = sample_issue();
        issue.description = Some("b".repeat(500));
        let doc = issue_to_document(&issue);
        let desc = doc.description.expect("description should be set");
        // Contract: 200 chars + the ellipsis suffix (1 char) = 201 chars max.
        let chars: Vec<char> = desc.chars().collect();
        assert!(
            chars.len() <= 201,
            "description should be ~200 chars + ellipsis, got {}",
            chars.len()
        );
        assert!(desc.ends_with('…'));
    }

    #[test]
    fn priority_label_covers_all_cases() {
        assert_eq!(priority_label(0), "P0-none");
        assert_eq!(priority_label(1), "P1-urgent");
        assert_eq!(priority_label(2), "P2-high");
        assert_eq!(priority_label(3), "P3-medium");
        assert_eq!(priority_label(4), "P4-low");
        assert_eq!(priority_label(99), "P0-none");
    }

    #[test]
    fn truncate_respects_char_boundaries() {
        // Multi-byte: "café" is 5 bytes but 4 chars.
        assert_eq!(truncate("café", 3), "caf…");
        assert_eq!(truncate("hi", 10), "hi");
        assert_eq!(truncate("", 5), "");
    }
}
