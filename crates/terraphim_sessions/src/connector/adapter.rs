//! Adapter for converting sync connectors from terraphim_session_connectors
//! to async connectors used by terraphim_sessions.

use crate::connector::SessionConnector;
use crate::model::{Message, MessageRole, Session, SessionMetadata};
use anyhow::Result;
use async_trait::async_trait;
use terraphim_session_connectors::{NormalizedMessage, NormalizedSession, SessionConnector as SyncConnector};

/// Adapter that wraps a sync connector into an async one.
pub struct AsyncConnectorAdapter<C: SyncConnector> {
    inner: C,
}

impl<C: SyncConnector> AsyncConnectorAdapter<C> {
    /// Create a new adapter wrapping the given connector.
    pub fn new(inner: C) -> Self {
        Self { inner }
    }
}

#[async_trait]
impl<C: SyncConnector> SessionConnector for AsyncConnectorAdapter<C> {
    fn source_id(&self) -> &str {
        self.inner.source_id()
    }

    fn display_name(&self) -> &str {
        self.inner.display_name()
    }

    fn detect(&self) -> terraphim_session_connectors::ConnectorStatus {
        self.inner.detect()
    }

    fn default_path(&self) -> Option<std::path::PathBuf> {
        self.inner.default_path()
    }

    async fn import(
        &self,
        options: &terraphim_session_connectors::ImportOptions,
    ) -> Result<Vec<Session>> {
        let normalized = self.inner.import(options)?;
        Ok(normalized.into_iter().map(convert_session).collect())
    }
}

fn convert_session(ns: NormalizedSession) -> Session {
    Session {
        id: format!("{}:{}", ns.source, ns.external_id),
        source: ns.source,
        external_id: ns.external_id,
        title: ns.title,
        source_path: ns.source_path,
        started_at: ns.started_at,
        ended_at: ns.ended_at,
        messages: ns.messages.into_iter().map(convert_message).collect(),
        metadata: SessionMetadata {
            ..Default::default()
        },
    }
}

fn convert_message(nm: NormalizedMessage) -> Message {
    Message {
        idx: nm.idx,
        role: MessageRole::from(nm.role.as_str()),
        author: nm.author,
        content: nm.content,
        blocks: vec![],
        created_at: nm.created_at,
        extra: nm.extra,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_convert_message_role() {
        let nm = NormalizedMessage {
            idx: 0,
            role: "user".to_string(),
            author: None,
            content: "test".to_string(),
            created_at: None,
            extra: serde_json::Value::Null,
        };

        let msg = convert_message(nm);
        assert_eq!(msg.role, MessageRole::User);
    }
}
