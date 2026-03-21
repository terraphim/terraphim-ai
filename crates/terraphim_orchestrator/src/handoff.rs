use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Shallow context transferred between agents during handoff.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct HandoffContext {
    /// Unique ID for each handoff.
    pub handoff_id: Uuid,
    /// Source agent name.
    pub from_agent: String,
    /// Target agent name.
    pub to_agent: String,
    /// Task description being handed off.
    pub task: String,
    /// Summary of work completed so far.
    pub progress_summary: String,
    /// Key decisions made.
    pub decisions: Vec<String>,
    /// Files modified.
    pub files_touched: Vec<PathBuf>,
    /// Timestamp of handoff.
    pub timestamp: chrono::DateTime<chrono::Utc>,
    /// Time-to-live in seconds (None = use buffer default).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ttl_secs: Option<u64>,
}

impl HandoffContext {
    /// Create a new HandoffContext with a generated UUID and current timestamp.
    pub fn new(
        from_agent: impl Into<String>,
        to_agent: impl Into<String>,
        task: impl Into<String>,
    ) -> Self {
        Self {
            handoff_id: Uuid::new_v4(),
            from_agent: from_agent.into(),
            to_agent: to_agent.into(),
            task: task.into(),
            progress_summary: String::new(),
            decisions: Vec::new(),
            files_touched: Vec::new(),
            timestamp: chrono::Utc::now(),
            ttl_secs: None,
        }
    }

    /// Serialize to JSON string.
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }

    /// Deserialize from JSON string.
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }

    /// Deserialize from JSON string with lenient defaults for missing new fields.
    /// Provides backward compatibility with old JSON files.
    pub fn from_json_lenient(json: &str) -> Result<Self, serde_json::Error> {
        let mut value: serde_json::Value = serde_json::from_str(json)?;

        // Add default values for new fields if missing
        if let Some(obj) = value.as_object_mut() {
            if !obj.contains_key("handoff_id") {
                obj.insert("handoff_id".to_string(), serde_json::json!(Uuid::new_v4()));
            }
            if !obj.contains_key("from_agent") {
                obj.insert("from_agent".to_string(), serde_json::json!("unknown"));
            }
            if !obj.contains_key("to_agent") {
                obj.insert("to_agent".to_string(), serde_json::json!("unknown"));
            }
            if !obj.contains_key("timestamp") {
                obj.insert(
                    "timestamp".to_string(),
                    serde_json::json!(chrono::Utc::now()),
                );
            }
            // ttl_secs is Option<u64> with serde(default), so it's handled automatically
        }

        serde_json::from_value(value)
    }

    /// Write handoff context to a file.
    pub fn write_to_file(&self, path: impl AsRef<std::path::Path>) -> Result<(), std::io::Error> {
        let json = serde_json::to_string_pretty(self)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        std::fs::write(path, json)
    }

    /// Write handoff context to a file atomically using a temporary file and rename.
    pub fn write_to_file_atomic(
        &self,
        path: impl AsRef<std::path::Path>,
    ) -> Result<(), std::io::Error> {
        let path = path.as_ref();
        let json = serde_json::to_string_pretty(self)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;

        // Create temporary file in the same directory as the target
        let parent = path.parent().unwrap_or(std::path::Path::new("."));
        let file_name = path
            .file_name()
            .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::InvalidInput, "Invalid path"))?
            .to_string_lossy();
        let tmp_path = parent.join(format!(".tmp.{}", file_name));

        // Write to temporary file
        std::fs::write(&tmp_path, json)?;

        // Atomically rename to final path (atomic on same filesystem)
        std::fs::rename(&tmp_path, path)?;

        Ok(())
    }

    /// Read handoff context from a file.
    pub fn read_from_file(path: impl AsRef<std::path::Path>) -> Result<Self, std::io::Error> {
        let content = std::fs::read_to_string(path)?;
        serde_json::from_str(&content)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    fn make_handoff() -> HandoffContext {
        HandoffContext {
            handoff_id: Uuid::new_v4(),
            from_agent: "agent-a".to_string(),
            to_agent: "agent-b".to_string(),
            task: "Fix authentication bug".to_string(),
            progress_summary: "Identified root cause in token validation".to_string(),
            decisions: vec![
                "Use JWT instead of session cookies".to_string(),
                "Add refresh token rotation".to_string(),
            ],
            files_touched: vec![
                PathBuf::from("src/auth/token.rs"),
                PathBuf::from("src/auth/middleware.rs"),
            ],
            timestamp: Utc::now(),
            ttl_secs: Some(3600),
        }
    }

    #[test]
    fn test_handoff_new_generates_uuid() {
        let ctx1 = HandoffContext::new("agent-a", "agent-b", "test task");
        let ctx2 = HandoffContext::new("agent-a", "agent-b", "test task");

        // UUIDs should be different
        assert_ne!(ctx1.handoff_id, ctx2.handoff_id);

        // Other fields should be set correctly
        assert_eq!(ctx1.from_agent, "agent-a");
        assert_eq!(ctx1.to_agent, "agent-b");
        assert_eq!(ctx1.task, "test task");
        assert!(ctx1.progress_summary.is_empty());
        assert!(ctx1.decisions.is_empty());
        assert!(ctx1.files_touched.is_empty());
        assert!(ctx1.ttl_secs.is_none());

        // Timestamp should be recent (within last minute)
        let now = Utc::now();
        let diff = now.signed_duration_since(ctx1.timestamp);
        assert!(diff.num_seconds() < 60);
    }

    #[test]
    fn test_handoff_roundtrip_json() {
        let original = make_handoff();
        let json = original.to_json().unwrap();
        let restored = HandoffContext::from_json(&json).unwrap();
        assert_eq!(original, restored);
    }

    #[test]
    fn test_handoff_roundtrip_json_with_new_fields() {
        let original = HandoffContext {
            handoff_id: Uuid::new_v4(),
            from_agent: "test-from".to_string(),
            to_agent: "test-to".to_string(),
            task: "Test task".to_string(),
            progress_summary: "Test progress".to_string(),
            decisions: vec!["decision1".to_string()],
            files_touched: vec![PathBuf::from("test.rs")],
            timestamp: Utc::now(),
            ttl_secs: Some(7200),
        };

        let json = original.to_json().unwrap();
        let restored = HandoffContext::from_json(&json).unwrap();

        assert_eq!(original.handoff_id, restored.handoff_id);
        assert_eq!(original.from_agent, restored.from_agent);
        assert_eq!(original.to_agent, restored.to_agent);
        assert_eq!(original.task, restored.task);
        assert_eq!(original.ttl_secs, restored.ttl_secs);
        assert_eq!(original, restored);
    }

    #[test]
    fn test_handoff_from_json_lenient_missing_new_fields() {
        // Old format JSON without new fields
        let old_json = r#"{
            "task": "Legacy task",
            "progress_summary": "Legacy progress",
            "decisions": ["decision1"],
            "files_touched": ["file1.rs"],
            "timestamp": "2024-01-15T10:30:00Z"
        }"#;

        let ctx = HandoffContext::from_json_lenient(old_json).unwrap();

        // Legacy fields should be preserved
        assert_eq!(ctx.task, "Legacy task");
        assert_eq!(ctx.progress_summary, "Legacy progress");
        assert_eq!(ctx.decisions, vec!["decision1"]);
        assert_eq!(ctx.files_touched, vec![PathBuf::from("file1.rs")]);

        // New fields should have defaults
        assert_eq!(ctx.from_agent, "unknown");
        assert_eq!(ctx.to_agent, "unknown");
        assert!(ctx.ttl_secs.is_none());

        // UUID should be generated
        // Timestamp should be preserved from old JSON
        let expected_ts: chrono::DateTime<Utc> = "2024-01-15T10:30:00Z".parse().unwrap();
        assert_eq!(ctx.timestamp, expected_ts);
    }

    #[test]
    fn test_handoff_from_json_lenient_partial_new_fields() {
        // JSON with some new fields but missing others
        let partial_json = r#"{
            "handoff_id": "550e8400-e29b-41d4-a716-446655440000",
            "task": "Partial task",
            "progress_summary": "Partial progress",
            "decisions": [],
            "files_touched": [],
            "timestamp": "2024-06-01T12:00:00Z",
            "from_agent": "agent-source"
        }"#;

        let ctx = HandoffContext::from_json_lenient(partial_json).unwrap();

        // Provided fields should be preserved
        assert_eq!(
            ctx.handoff_id,
            Uuid::parse_str("550e8400-e29b-41d4-a716-446655440000").unwrap()
        );
        assert_eq!(ctx.from_agent, "agent-source");
        assert_eq!(ctx.task, "Partial task");

        // Missing fields should have defaults
        assert_eq!(ctx.to_agent, "unknown");
        assert!(ctx.ttl_secs.is_none());
    }

    #[test]
    fn test_handoff_roundtrip_file() {
        let original = make_handoff();
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("handoff.json");

        original.write_to_file(&path).unwrap();
        let restored = HandoffContext::read_from_file(&path).unwrap();
        assert_eq!(original, restored);
    }

    #[test]
    fn test_handoff_write_atomic_creates_file() {
        let original = make_handoff();
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("atomic-handoff.json");

        original.write_to_file_atomic(&path).unwrap();

        // File should exist
        assert!(path.exists());

        // Content should be readable and match
        let restored = HandoffContext::read_from_file(&path).unwrap();
        assert_eq!(original.handoff_id, restored.handoff_id);
        assert_eq!(original.from_agent, restored.from_agent);
        assert_eq!(original.to_agent, restored.to_agent);
        assert_eq!(original.task, restored.task);
    }

    #[test]
    fn test_handoff_write_atomic_no_partial() {
        let original = make_handoff();
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("no-partial.json");

        original.write_to_file_atomic(&path).unwrap();

        // Temporary file should not exist (should be cleaned up by rename)
        let tmp_path = dir.path().join(".tmp.no-partial.json");
        assert!(!tmp_path.exists());

        // Final file should exist
        assert!(path.exists());
    }

    #[test]
    fn test_handoff_empty_decisions() {
        let ctx = HandoffContext::new("from", "to", "simple task");
        let json = ctx.to_json().unwrap();
        let restored = HandoffContext::from_json(&json).unwrap();
        assert_eq!(ctx.handoff_id, restored.handoff_id);
        assert_eq!(ctx.from_agent, restored.from_agent);
        assert_eq!(ctx.to_agent, restored.to_agent);
        assert_eq!(ctx.task, restored.task);
        assert!(restored.decisions.is_empty());
    }

    #[test]
    fn test_ttl_serialization() {
        // Test that ttl_secs is skipped when None
        let ctx_without_ttl = HandoffContext {
            handoff_id: Uuid::new_v4(),
            from_agent: "a".to_string(),
            to_agent: "b".to_string(),
            task: "test".to_string(),
            progress_summary: String::new(),
            decisions: vec![],
            files_touched: vec![],
            timestamp: Utc::now(),
            ttl_secs: None,
        };

        let json = ctx_without_ttl.to_json().unwrap();
        assert!(!json.contains("ttl_secs"));

        // Test that ttl_secs is included when Some
        let ctx_with_ttl = HandoffContext {
            ttl_secs: Some(3600),
            ..ctx_without_ttl
        };

        let json = ctx_with_ttl.to_json().unwrap();
        assert!(json.contains("ttl_secs"));
    }
}
