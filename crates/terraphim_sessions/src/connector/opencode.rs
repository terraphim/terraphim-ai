//! OpenCode Session Connector
//!
//! Imports OpenCode sessions from SQLite database and legacy JSONL.
//! Primary format: SQLite (~/.local/share/opencode/opencode.db)
//! Legacy format: JSONL (~/.local/state/opencode/prompt-history.jsonl)

use super::{ConnectorStatus, ImportOptions, SessionConnector};
use crate::model::{ContentBlock, Message, MessageRole, Session, SessionMetadata};
use anyhow::{Context, Result};
use async_trait::async_trait;
use serde::Deserialize;
use std::path::PathBuf;

#[derive(Debug, Default)]
pub struct OpenCodeConnector;

#[derive(Debug, Clone, Deserialize)]
struct LegacyEntry {
    #[serde(default)]
    input: Option<String>,
    #[serde(default)]
    parts: Vec<serde_json::Value>,
    #[serde(default)]
    mode: Option<String>,
}

#[async_trait]
impl SessionConnector for OpenCodeConnector {
    fn source_id(&self) -> &str {
        "opencode"
    }
    fn display_name(&self) -> &str {
        "OpenCode"
    }

    fn detect(&self) -> ConnectorStatus {
        if let Some(db) = self.sqlite_db_path() {
            if db.exists() {
                return ConnectorStatus::Available {
                    path: db,
                    sessions_estimate: None,
                };
            }
        }
        if let Some(path) = self.legacy_path() {
            let hf = path.join("prompt-history.jsonl");
            if hf.exists() {
                let n = std::fs::read_to_string(&hf)
                    .map(|c| c.lines().filter(|l| !l.trim().is_empty()).count())
                    .unwrap_or(0);
                return ConnectorStatus::Available {
                    path,
                    sessions_estimate: Some(if n > 0 { 1 } else { 0 }),
                };
            }
        }
        ConnectorStatus::NotFound
    }

    fn default_path(&self) -> Option<PathBuf> {
        self.sqlite_db_path().or_else(|| self.legacy_path())
    }

    async fn import(&self, options: &ImportOptions) -> Result<Vec<Session>> {
        // If a custom path is provided, use it (test harness / manual override)
        if let Some(ref path) = options.path {
            return self.import_jsonl(options).await;
        }
        // Auto-detect: try SQLite first, fall back to legacy JSONL
        if let Some(db) = self.sqlite_db_path() {
            if db.exists() {
                return self.import_sqlite(&db, options).await;
            }
        }
        self.import_jsonl(options).await
    }
}

impl OpenCodeConnector {
    fn sqlite_db_path(&self) -> Option<PathBuf> {
        dirs::data_local_dir().map(|d| d.join("opencode").join("opencode.db"))
    }

    fn legacy_path(&self) -> Option<PathBuf> {
        dirs::home_dir().map(|h| h.join(".local").join("state").join("opencode"))
    }

    #[cfg(feature = "opencode-connector")]
    async fn import_sqlite(
        &self,
        db_path: &PathBuf,
        options: &ImportOptions,
    ) -> Result<Vec<Session>> {
        use rusqlite::Connection;
        use std::collections::BTreeMap;

        let uri = format!("file:{}?mode=ro", db_path.display());
        let conn = Connection::open_with_flags(
            &uri,
            rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY | rusqlite::OpenFlags::SQLITE_OPEN_URI,
        )
        .context("Failed to open OpenCode SQLite DB")?;

        let mut stmt = conn.prepare(
            "SELECT s.id, s.title, s.directory, s.time_created, s.model, s.cost, s.tokens_input, s.tokens_output,
                    m.id, m.time_created, m.data, p.id, p.time_created, p.data
             FROM session s JOIN message m ON m.session_id = s.id
             JOIN part p ON p.message_id = m.id
             WHERE json_extract(p.data, '$.type') = 'text'
             ORDER BY s.id, m.time_created, p.id"
        )?;

        let rows = stmt
            .query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, i64>(3)?,
                    row.get::<_, Option<String>>(4)?,
                    row.get::<_, f64>(5)?,
                    row.get::<_, i64>(6)?,
                    row.get::<_, i64>(7)?,
                    row.get::<_, String>(8)?,
                    row.get::<_, i64>(9)?,
                    row.get::<_, String>(10)?,
                    row.get::<_, String>(11)?,
                    row.get::<_, i64>(12)?,
                    row.get::<_, String>(13)?,
                ))
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;

        let mut smap: BTreeMap<String, (Session, BTreeMap<String, (MessageRole, Vec<String>)>)> =
            BTreeMap::new();

        for (sid, title, dir, _created, model, cost, ti, to, mid, mts, mdat, _pid, _pts, pdat) in
            rows
        {
            let role = serde_json::from_str::<serde_json::Value>(&mdat)
                .ok()
                .and_then(|v| v.get("role").and_then(|r| r.as_str().map(String::from)))
                .map(|r| match r.as_str() {
                    "user" => MessageRole::User,
                    "assistant" => MessageRole::Assistant,
                    _ => MessageRole::Other,
                })
                .unwrap_or(MessageRole::Other);
            let text = serde_json::from_str::<serde_json::Value>(&pdat)
                .ok()
                .and_then(|v| v.get("text").and_then(|t| t.as_str().map(String::from)))
                .unwrap_or_default();
            if text.is_empty() {
                continue;
            }
            let ts = jiff::Timestamp::from_second((mts / 1000).try_into().unwrap_or(0)).ok();
            let s = smap.entry(sid.clone()).or_insert_with(|| (Session {
                id: format!("opencode:{}", sid), source: "opencode".into(), external_id: sid.clone(),
                title: Some(title.clone()), source_path: db_path.clone(), started_at: ts, ended_at: ts,
                messages: vec![], metadata: SessionMetadata::new(Some(dir), model, vec!["opencode".into()],
                    serde_json::json!({"cost": cost, "tokens_input": ti, "tokens_output": to})),
            }, BTreeMap::new()));
            s.1.entry(mid)
                .or_insert_with(|| (role, vec![]))
                .1
                .push(text);
        }

        let mut sessions = vec![];
        for (_sid, (mut session, msgs)) in smap {
            let mut v: Vec<(String, MessageRole, String)> = msgs
                .into_iter()
                .map(|(mid, (r, t))| (mid, r, t.join("\n")))
                .collect();
            v.sort_by_key(|(mid, _, _)| mid.clone());
            for (idx, (_mid, role, content)) in v.into_iter().enumerate() {
                session.messages.push(Message {
                    idx,
                    role,
                    author: None,
                    content: content.clone(),
                    blocks: vec![ContentBlock::Text { text: content }],
                    created_at: None,
                    extra: serde_json::Value::Null,
                });
                if options.limit.map_or(false, |l| session.messages.len() >= l) {
                    break;
                }
            }
            if !session.messages.is_empty() {
                sessions.push(session);
                if options.limit.map_or(false, |l| sessions.len() >= l) {
                    break;
                }
            }
        }
        Ok(sessions)
    }

    #[cfg(not(feature = "opencode-connector"))]
    async fn import_sqlite(
        &self,
        _db_path: &PathBuf,
        _options: &ImportOptions,
    ) -> Result<Vec<Session>> {
        Ok(vec![])
    }

    async fn import_jsonl(&self, options: &ImportOptions) -> Result<Vec<Session>> {
        let path = options
            .path
            .clone()
            .or_else(|| self.legacy_path())
            .ok_or_else(|| anyhow::anyhow!("No path"))?;
        let hf = path.join("prompt-history.jsonl");
        if !hf.exists() {
            return Ok(vec![]);
        }
        let content = tokio::fs::read_to_string(&hf).await?;
        let mut msgs = vec![];
        for (idx, line) in content.lines().enumerate() {
            if line.trim().is_empty() {
                continue;
            }
            if let Ok(e) = serde_json::from_str::<LegacyEntry>(line) {
                if let Some(input) = e.input {
                    if !input.is_empty() {
                        msgs.push(Message {
                            idx,
                            role: MessageRole::User,
                            author: None,
                            content: input.clone(),
                            blocks: vec![ContentBlock::Text { text: input }],
                            created_at: None,
                            extra: serde_json::json!({"mode": e.mode, "parts": e.parts}),
                        });
                    }
                }
            }
        }
        if let Some(l) = options.limit {
            if l > 0 {
                msgs.truncate(l);
            }
        }
        if msgs.is_empty() {
            return Ok(vec![]);
        }
        Ok(vec![Session {
            id: "opencode:prompt-history".into(),
            source: "opencode".into(),
            external_id: "prompt-history".into(),
            title: Some("OpenCode Prompt History".into()),
            source_path: hf,
            started_at: None,
            ended_at: None,
            messages: msgs,
            metadata: SessionMetadata::new(
                None,
                None,
                vec!["opencode".into()],
                serde_json::json!({"type": "prompt-history"}),
            ),
        }])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_source_id() {
        let c = OpenCodeConnector;
        assert_eq!(c.source_id(), "opencode");
        assert_eq!(c.display_name(), "OpenCode");
    }
    #[test]
    fn test_sqlite_path() {
        let c = OpenCodeConnector;
        assert!(
            c.sqlite_db_path()
                .unwrap()
                .ends_with("opencode/opencode.db")
        );
    }
    #[test]
    fn test_legacy_path() {
        let c = OpenCodeConnector;
        assert!(c.legacy_path().unwrap().ends_with(".local/state/opencode"));
    }
    #[test]
    fn test_parse_legacy() {
        let e: LegacyEntry =
            serde_json::from_str(r#"{"input":"hi","parts":[],"mode":"normal"}"#).unwrap();
        assert_eq!(e.input.unwrap(), "hi");
    }
    #[tokio::test]
    async fn test_import_jsonl() {
        let d = tempfile::tempdir().unwrap();
        let f = d.path().join("prompt-history.jsonl");
        tokio::fs::write(&f, r#"{"input":"hello","parts":[],"mode":"normal"}"#)
            .await
            .unwrap();
        let opts = ImportOptions::default().with_path(d.path().to_path_buf());
        let s = OpenCodeConnector.import(&opts).await.unwrap();
        assert_eq!(s.len(), 1);
        assert_eq!(s[0].messages[0].content, "hello");
    }
}
