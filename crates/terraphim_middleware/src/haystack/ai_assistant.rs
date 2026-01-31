//! AI Assistant Session Haystack
//!
//! Indexes session logs from AI coding assistants using the terraphim-session-analyzer
//! connector system. Supports:
//! - Claude Code (JSONL) - `~/.claude/projects/`
//! - OpenCode (JSONL) - `~/.opencode/`
//! - Cursor IDE (SQLite) - `~/.config/Cursor/User/`
//! - Aider (Markdown) - `.aider.chat.history.md`
//! - Codex (JSONL) - OpenAI Codex CLI
//!
//! Configure via `extra_parameters["connector"]` with one of:
//! `claude-code`, `opencode`, `cursor`, `aider`, `codex`

use crate::indexer::IndexMiddleware;
use crate::Result;
use std::path::PathBuf;
use terraphim_config::Haystack;
use terraphim_session_analyzer::connectors::{
    ConnectorRegistry, ImportOptions, NormalizedMessage, NormalizedSession,
};
use terraphim_types::{Document, Index};

/// Valid connector names for error messages
const VALID_CONNECTORS: &[&str] = &["claude-code", "opencode", "cursor", "aider", "codex"];

/// Default limit for sessions to prevent memory issues
const DEFAULT_SESSION_LIMIT: usize = 1000;

/// Middleware that indexes AI coding assistant session logs.
///
/// Uses the claude-log-analyzer connector system to support multiple
/// AI assistants with different log formats.
#[derive(Debug, Default)]
pub struct AiAssistantHaystackIndexer;

impl IndexMiddleware for AiAssistantHaystackIndexer {
    // Allow manual_async_fn because the IndexMiddleware trait requires returning
    // `impl Future<Output = Result<Index>> + Send` rather than using async_trait.
    // This pattern is necessary for trait method compatibility.
    #[allow(clippy::manual_async_fn)]
    fn index(
        &self,
        needle: &str,
        haystack: &Haystack,
    ) -> impl std::future::Future<Output = Result<Index>> + Send {
        async move {
            // Get connector name from extra_parameters
            let connector_name = haystack.extra_parameters.get("connector").ok_or_else(|| {
                crate::Error::Indexation(format!(
                    "Missing 'connector' in extra_parameters. Valid connectors: {}",
                    VALID_CONNECTORS.join(", ")
                ))
            })?;

            log::info!(
                "AiAssistant: Indexing with connector '{}' for search term: '{}'",
                connector_name,
                needle
            );

            // Validate connector exists before spawning blocking task
            let registry = ConnectorRegistry::new();
            if registry.get(connector_name).is_none() {
                return Err(crate::Error::Indexation(format!(
                    "Unknown connector '{}'. Valid connectors: {}",
                    connector_name,
                    VALID_CONNECTORS.join(", ")
                )));
            }

            // Build import options from haystack config
            let import_options = build_import_options(haystack);
            let connector_name_owned = connector_name.clone();

            // Import sessions in a blocking task to avoid blocking the async executor.
            // The connector.import() performs synchronous I/O (reading JSONL files,
            // SQLite databases) which would block the tokio runtime if run directly.
            // We create the registry inside the blocking task to satisfy 'static lifetime.
            let sessions = tokio::task::spawn_blocking(move || {
                let registry = ConnectorRegistry::new();
                let connector = registry
                    .get(&connector_name_owned)
                    .expect("Connector validated above");
                connector.import(&import_options)
            })
            .await
            .map_err(|e| {
                crate::Error::Indexation(format!(
                    "Task join error while importing from '{}': {}",
                    connector_name, e
                ))
            })?
            .map_err(|e| {
                crate::Error::Indexation(format!(
                    "Failed to import sessions from '{}': {}",
                    connector_name, e
                ))
            })?;

            log::info!(
                "AiAssistant: Imported {} sessions from '{}'",
                sessions.len(),
                connector_name
            );

            // Convert sessions to documents and filter by needle
            let mut index = Index::new();
            for session in sessions {
                let documents = session_to_documents(&session, needle, connector_name);
                for doc in documents {
                    index.insert(doc.id.clone(), doc);
                }
            }

            log::info!(
                "AiAssistant: Found {} matching documents for '{}'",
                index.len(),
                needle
            );

            Ok(index)
        }
    }
}

/// Build ImportOptions from haystack configuration
fn build_import_options(haystack: &Haystack) -> ImportOptions {
    let mut options = ImportOptions::default();

    // Set path from haystack location (with expansion)
    if !haystack.location.is_empty() {
        let expanded = expand_path(&haystack.location);
        if !expanded.exists() {
            log::warn!(
                "AiAssistant: Haystack path does not exist: {} (expanded from '{}')",
                expanded.display(),
                haystack.location
            );
        }
        options.path = Some(expanded);
    }

    // Parse limit from extra_parameters
    options.limit = haystack
        .extra_parameters
        .get("limit")
        .and_then(|s| s.parse::<usize>().ok())
        .or(Some(DEFAULT_SESSION_LIMIT));

    // Parse since timestamp from extra_parameters
    if let Some(since_str) = haystack.extra_parameters.get("since") {
        match jiff::Timestamp::strptime("%Y-%m-%dT%H:%M:%SZ", since_str) {
            Ok(ts) => options.since = Some(ts),
            Err(e) => log::warn!(
                "Invalid 'since' timestamp '{}': {}. Expected format: YYYY-MM-DDTHH:MM:SSZ",
                since_str,
                e
            ),
        }
    }

    // Parse until timestamp from extra_parameters
    if let Some(until_str) = haystack.extra_parameters.get("until") {
        match jiff::Timestamp::strptime("%Y-%m-%dT%H:%M:%SZ", until_str) {
            Ok(ts) => options.until = Some(ts),
            Err(e) => log::warn!(
                "Invalid 'until' timestamp '{}': {}. Expected format: YYYY-MM-DDTHH:MM:SSZ",
                until_str,
                e
            ),
        }
    }

    // Incremental mode
    options.incremental = haystack
        .extra_parameters
        .get("incremental")
        .map(|s| s == "true")
        .unwrap_or(false);

    options
}

/// Expand path with ~ and environment variables
fn expand_path(path: &str) -> PathBuf {
    let mut result = path.to_string();

    // Expand ~ to home directory
    if result.starts_with('~') {
        if let Some(home) = home::home_dir() {
            result = result.replacen('~', &home.to_string_lossy(), 1);
        }
    }

    // Expand $HOME and other common env vars
    if result.contains("$HOME") {
        if let Some(home) = home::home_dir() {
            result = result.replace("$HOME", &home.to_string_lossy());
        }
    }

    PathBuf::from(result)
}

/// Convert a NormalizedSession to multiple Documents (one per message that matches needle)
fn session_to_documents(
    session: &NormalizedSession,
    needle: &str,
    connector_name: &str,
) -> Vec<Document> {
    let needle_lower = needle.to_lowercase();
    let mut documents = Vec::new();

    // Check if session title matches
    let title_matches = session
        .title
        .as_ref()
        .map(|t| t.to_lowercase().contains(&needle_lower))
        .unwrap_or(false);

    for msg in &session.messages {
        // Check if message content matches the needle
        let content_matches = msg.content.to_lowercase().contains(&needle_lower);

        // Include message if content matches or if we're doing a broad search (empty needle)
        // Also include if title matches and this is the first message
        if content_matches || needle.is_empty() || (title_matches && msg.idx == 0) {
            documents.push(message_to_document(session, msg, connector_name));
        }
    }

    documents
}

/// Convert a single NormalizedMessage to a Document
fn message_to_document(
    session: &NormalizedSession,
    msg: &NormalizedMessage,
    connector_name: &str,
) -> Document {
    let session_title = session
        .title
        .clone()
        .unwrap_or_else(|| "Session".to_string());

    // Truncate title if too long
    let display_title = if session_title.len() > 60 {
        format!("{}...", &session_title[..57])
    } else {
        session_title.clone()
    };

    Document {
        id: format!("{}:{}:{}", connector_name, session.external_id, msg.idx),
        title: format!(
            "[{}] {} #{}",
            connector_name.to_uppercase(),
            display_title,
            msg.idx
        ),
        url: format!("file://{}", session.source_path.display()),
        body: msg.content.clone(),
        description: Some(format!(
            "{} message in {} session ({})",
            msg.role,
            session.source,
            session
                .started_at
                .map(|t| t.strftime("%Y-%m-%d %H:%M").to_string())
                .unwrap_or_else(|| "unknown time".to_string())
        )),
        summarization: None,
        stub: None,
        tags: Some(vec![
            "ai-assistant".to_string(),
            connector_name.to_string(),
            msg.role.clone(),
            format!("session:{}", session.external_id),
        ]),
        rank: msg.created_at.map(|t| t.as_millisecond() as u64),
        source_haystack: None, // Will be set by caller
        doc_type: Default::default(),
        synonyms: None,
        route: None,
        priority: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_expand_path_tilde() {
        let path = expand_path("~/.claude/projects");
        assert!(!path.to_string_lossy().starts_with('~'));
        assert!(path.to_string_lossy().contains(".claude/projects"));
    }

    #[test]
    fn test_expand_path_home_env() {
        let path = expand_path("$HOME/.opencode");
        assert!(!path.to_string_lossy().contains("$HOME"));
    }

    #[test]
    fn test_expand_path_absolute() {
        let path = expand_path("/tmp/test");
        assert_eq!(path, PathBuf::from("/tmp/test"));
    }

    #[test]
    fn test_valid_connectors_list() {
        assert!(VALID_CONNECTORS.contains(&"claude-code"));
        assert!(VALID_CONNECTORS.contains(&"opencode"));
        assert!(VALID_CONNECTORS.contains(&"cursor"));
        assert!(VALID_CONNECTORS.contains(&"aider"));
        assert!(VALID_CONNECTORS.contains(&"codex"));
    }

    #[test]
    fn test_build_import_options_with_limit() {
        let mut haystack = Haystack {
            location: "~/.claude/projects".to_string(),
            service: terraphim_config::ServiceType::AiAssistant,
            read_only: true,
            fetch_content: false,
            atomic_server_secret: None,
            extra_parameters: Default::default(),
        };
        haystack
            .extra_parameters
            .insert("connector".to_string(), "claude-code".to_string());
        haystack
            .extra_parameters
            .insert("limit".to_string(), "50".to_string());

        let options = build_import_options(&haystack);
        assert_eq!(options.limit, Some(50));
    }

    #[test]
    fn test_build_import_options_default_limit() {
        let haystack = Haystack {
            location: "~/.claude/projects".to_string(),
            service: terraphim_config::ServiceType::AiAssistant,
            read_only: true,
            fetch_content: false,
            atomic_server_secret: None,
            extra_parameters: Default::default(),
        };

        let options = build_import_options(&haystack);
        assert_eq!(options.limit, Some(DEFAULT_SESSION_LIMIT));
    }

    #[test]
    fn test_message_to_document() {
        let session = NormalizedSession {
            source: "claude-code".to_string(),
            external_id: "test-session-123".to_string(),
            title: Some("Test Project".to_string()),
            source_path: PathBuf::from("/home/user/.claude/projects/test.jsonl"),
            started_at: None,
            ended_at: None,
            messages: vec![],
            metadata: serde_json::Value::Null,
        };

        let msg = NormalizedMessage {
            idx: 0,
            role: "user".to_string(),
            author: None,
            content: "Hello, can you help me?".to_string(),
            created_at: None,
            extra: serde_json::Value::Null,
        };

        let doc = message_to_document(&session, &msg, "claude-code");

        assert_eq!(doc.id, "claude-code:test-session-123:0");
        assert!(doc.title.contains("[CLAUDE-CODE]"));
        assert!(doc.title.contains("Test Project"));
        assert_eq!(doc.body, "Hello, can you help me?");
        assert!(doc
            .tags
            .as_ref()
            .unwrap()
            .contains(&"ai-assistant".to_string()));
        assert!(doc
            .tags
            .as_ref()
            .unwrap()
            .contains(&"claude-code".to_string()));
        assert!(doc.tags.as_ref().unwrap().contains(&"user".to_string()));
    }

    #[test]
    fn test_session_to_documents_filters_by_needle() {
        let session = NormalizedSession {
            source: "claude-code".to_string(),
            external_id: "test-123".to_string(),
            title: Some("Rust Project".to_string()),
            source_path: PathBuf::from("/test"),
            started_at: None,
            ended_at: None,
            messages: vec![
                NormalizedMessage {
                    idx: 0,
                    role: "user".to_string(),
                    author: None,
                    content: "Help me with Rust async".to_string(),
                    created_at: None,
                    extra: serde_json::Value::Null,
                },
                NormalizedMessage {
                    idx: 1,
                    role: "assistant".to_string(),
                    author: None,
                    content: "Here is how to use tokio".to_string(),
                    created_at: None,
                    extra: serde_json::Value::Null,
                },
                NormalizedMessage {
                    idx: 2,
                    role: "user".to_string(),
                    author: None,
                    content: "Thanks!".to_string(),
                    created_at: None,
                    extra: serde_json::Value::Null,
                },
            ],
            metadata: serde_json::Value::Null,
        };

        // Search for "tokio" - should only match message 1
        let docs = session_to_documents(&session, "tokio", "claude-code");
        assert_eq!(docs.len(), 1);
        assert!(docs[0].body.contains("tokio"));

        // Search for "rust" - should match message 0 (and message 0 also due to title match)
        let docs = session_to_documents(&session, "rust", "claude-code");
        assert_eq!(docs.len(), 1);
        assert!(docs[0].body.to_lowercase().contains("rust"));

        // Empty search - should return all messages
        let docs = session_to_documents(&session, "", "claude-code");
        assert_eq!(docs.len(), 3);
    }

    #[tokio::test]
    async fn test_index_missing_connector_returns_error() {
        let indexer = AiAssistantHaystackIndexer;
        let haystack = Haystack {
            location: "/tmp/test".to_string(),
            service: terraphim_config::ServiceType::AiAssistant,
            read_only: true,
            fetch_content: false,
            atomic_server_secret: None,
            extra_parameters: Default::default(), // No connector specified
        };

        let result = indexer.index("test", &haystack).await;
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(
            err_msg.contains("Missing 'connector'"),
            "Expected error about missing connector, got: {}",
            err_msg
        );
        assert!(
            err_msg.contains("claude-code"),
            "Expected list of valid connectors, got: {}",
            err_msg
        );
    }

    #[tokio::test]
    async fn test_index_invalid_connector_returns_error() {
        let indexer = AiAssistantHaystackIndexer;
        let mut haystack = Haystack {
            location: "/tmp/test".to_string(),
            service: terraphim_config::ServiceType::AiAssistant,
            read_only: true,
            fetch_content: false,
            atomic_server_secret: None,
            extra_parameters: Default::default(),
        };
        haystack
            .extra_parameters
            .insert("connector".to_string(), "invalid-connector".to_string());

        let result = indexer.index("test", &haystack).await;
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(
            err_msg.contains("Unknown connector"),
            "Expected error about unknown connector, got: {}",
            err_msg
        );
        assert!(
            err_msg.contains("invalid-connector"),
            "Expected error to mention the invalid connector name, got: {}",
            err_msg
        );
    }
}
