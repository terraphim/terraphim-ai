//! Claude Log Analyzer Integration
//!
//! This module provides enhanced session import capabilities
//! when the `claude-log-analyzer` feature is enabled.

mod connector;

pub use connector::{ClaClaudeConnector, ClaCursorConnector};

use crate::model::{ContentBlock, Message, MessageRole, Session, SessionMetadata};
use claude_log_analyzer::connectors::{NormalizedMessage, NormalizedSession};

/// Convert a CLA NormalizedSession to our Session type
pub(crate) fn from_normalized_session(ns: NormalizedSession, prefix: &str) -> Session {
    let messages: Vec<Message> = ns
        .messages
        .into_iter()
        .map(|m| from_normalized_message(m))
        .collect();

    Session {
        id: format!("{}:{}", prefix, ns.external_id),
        source: ns.source,
        external_id: ns.external_id,
        title: ns.title,
        source_path: ns.source_path,
        started_at: ns.started_at,
        ended_at: ns.ended_at,
        messages,
        metadata: SessionMetadata {
            project_path: ns
                .metadata
                .get("project_path")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            model: None,
            tags: vec![],
            extra: ns.metadata,
        },
    }
}

/// Convert a CLA NormalizedMessage to our Message type
fn from_normalized_message(nm: NormalizedMessage) -> Message {
    let role = MessageRole::from(nm.role.as_str());

    Message {
        idx: nm.idx,
        role,
        author: nm.author,
        content: nm.content.clone(),
        blocks: vec![ContentBlock::Text { text: nm.content }],
        created_at: nm.created_at,
        extra: nm.extra,
    }
}
