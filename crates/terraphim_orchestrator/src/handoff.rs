use std::collections::HashMap;
use std::fs::OpenOptions;
use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;

use chrono::{DateTime, Utc};
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

/// Entry in the handoff buffer with expiry timestamp.
#[derive(Debug, Clone)]
struct BufferEntry {
    context: HandoffContext,
    expiry: DateTime<Utc>,
}

/// In-memory buffer for handoff contexts with TTL-based expiry.
#[derive(Debug)]
pub struct HandoffBuffer {
    entries: HashMap<Uuid, BufferEntry>,
    default_ttl_secs: u64,
}

impl HandoffBuffer {
    /// Create a new HandoffBuffer with the specified default TTL in seconds.
    pub fn new(default_ttl_secs: u64) -> Self {
        Self {
            entries: HashMap::new(),
            default_ttl_secs,
        }
    }

    /// Insert a handoff context into the buffer.
    /// Computes expiry from ctx.ttl_secs or falls back to default_ttl.
    pub fn insert(&mut self, context: HandoffContext) -> Uuid {
        let ttl_secs = context.ttl_secs.unwrap_or(self.default_ttl_secs);
        // Cap at ~100 years to avoid chrono::Duration overflow
        const MAX_TTL_SECS: i64 = 100 * 365 * 24 * 3600;
        let ttl_i64 = i64::try_from(ttl_secs)
            .unwrap_or(MAX_TTL_SECS)
            .min(MAX_TTL_SECS);
        let expiry = Utc::now() + chrono::Duration::seconds(ttl_i64);
        let id = context.handoff_id;

        self.entries.insert(id, BufferEntry { context, expiry });
        id
    }

    /// Get a reference to a handoff context by ID.
    /// Returns None if not found or if expired.
    pub fn get(&self, id: &Uuid) -> Option<&HandoffContext> {
        self.entries.get(id).and_then(|entry| {
            if Utc::now() < entry.expiry {
                Some(&entry.context)
            } else {
                None
            }
        })
    }

    /// Get the most recent handoff for a specific target agent.
    /// Returns the handoff with the latest timestamp that hasn't expired.
    pub fn latest_for_agent(&self, to_agent: &str) -> Option<&HandoffContext> {
        let now = Utc::now();
        self.entries
            .values()
            .filter(|entry| entry.context.to_agent == to_agent && now < entry.expiry)
            .max_by_key(|entry| entry.context.timestamp)
            .map(|entry| &entry.context)
    }

    /// Remove all expired entries and return the count swept.
    pub fn sweep_expired(&mut self) -> usize {
        let now = Utc::now();
        let initial_count = self.entries.len();
        self.entries.retain(|_, entry| now < entry.expiry);
        initial_count - self.entries.len()
    }

    /// Get the number of entries in the buffer.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Check if the buffer is empty.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Iterate over all entries (including expired ones).
    /// The iterator yields (id, context, expiry) tuples.
    pub fn iter(&self) -> impl Iterator<Item = (&Uuid, &HandoffContext, &DateTime<Utc>)> {
        self.entries
            .iter()
            .map(|(id, entry)| (id, &entry.context, &entry.expiry))
    }

    /// Get the default TTL in seconds.
    pub fn default_ttl_secs(&self) -> u64 {
        self.default_ttl_secs
    }
}

/// Append-only JSONL ledger for handoff contexts.
/// Provides durable, append-only storage for handoff history.
#[derive(Debug)]
pub struct HandoffLedger {
    path: PathBuf,
}

impl HandoffLedger {
    /// Create a new HandoffLedger with the specified file path.
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    /// Append a handoff context to the ledger.
    /// Opens the file with O_APPEND + create flags, writes JSON line + newline, and fsyncs.
    pub fn append(&self, context: &HandoffContext) -> Result<(), std::io::Error> {
        let json = serde_json::to_string(context)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;

        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)?;

        writeln!(file, "{}", json)?;
        file.sync_all()?;

        Ok(())
    }

    /// Read all entries from the ledger file.
    /// Returns `Vec<HandoffContext>` in order of insertion.
    pub fn read_all(&self) -> Result<Vec<HandoffContext>, std::io::Error> {
        let file = OpenOptions::new().read(true).open(&self.path)?;

        let reader = BufReader::new(file);
        let mut entries = Vec::new();

        for line in reader.lines() {
            let line = line?;
            if line.trim().is_empty() {
                continue;
            }
            let context: HandoffContext = serde_json::from_str(&line)
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
            entries.push(context);
        }

        Ok(entries)
    }

    /// Count entries in the ledger without loading all into memory.
    /// Efficiently counts lines in the file.
    pub fn count(&self) -> Result<usize, std::io::Error> {
        let metadata = std::fs::metadata(&self.path)?;
        if metadata.len() == 0 {
            return Ok(0);
        }

        let file = OpenOptions::new().read(true).open(&self.path)?;

        let reader = BufReader::new(file);
        let mut count = 0;

        for line in reader.lines() {
            let line = line?;
            if !line.trim().is_empty() {
                count += 1;
            }
        }

        Ok(count)
    }

    /// Return the file size in bytes for monitoring.
    pub fn size_bytes(&self) -> Result<u64, std::io::Error> {
        let metadata = std::fs::metadata(&self.path)?;
        Ok(metadata.len())
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

    // =========================================================================
    // HandoffBuffer Tests
    // =========================================================================

    #[test]
    fn test_buffer_new() {
        let buffer = HandoffBuffer::new(3600);
        assert_eq!(buffer.len(), 0);
        assert!(buffer.is_empty());
        assert_eq!(buffer.default_ttl_secs(), 3600);
    }

    #[test]
    fn test_buffer_insert_and_get() {
        let mut buffer = HandoffBuffer::new(3600);
        let ctx = HandoffContext::new("agent-a", "agent-b", "test task");
        let id = ctx.handoff_id;

        buffer.insert(ctx.clone());

        assert_eq!(buffer.len(), 1);
        assert!(!buffer.is_empty());

        let retrieved = buffer.get(&id);
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().handoff_id, id);
        assert_eq!(retrieved.unwrap().from_agent, "agent-a");
        assert_eq!(retrieved.unwrap().to_agent, "agent-b");
    }

    #[test]
    fn test_buffer_get_returns_none_for_unknown() {
        let buffer = HandoffBuffer::new(3600);
        let unknown_id = Uuid::new_v4();

        let retrieved = buffer.get(&unknown_id);
        assert!(retrieved.is_none());
    }

    #[test]
    fn test_buffer_latest_for_agent() {
        let mut buffer = HandoffBuffer::new(3600);

        // Insert two handoffs for the same target agent
        let ctx1 = HandoffContext::new("agent-a", "agent-c", "task 1");
        let ctx2 = HandoffContext::new("agent-b", "agent-c", "task 2");

        buffer.insert(ctx1.clone());
        buffer.insert(ctx2.clone());

        // Get latest for agent-c
        let latest = buffer.latest_for_agent("agent-c");
        assert!(latest.is_some());
        // Should return the most recent one
        assert_eq!(latest.unwrap().handoff_id, ctx2.handoff_id);
    }

    #[test]
    fn test_buffer_latest_for_agent_returns_none_for_unknown() {
        let buffer = HandoffBuffer::new(3600);

        let latest = buffer.latest_for_agent("unknown-agent");
        assert!(latest.is_none());
    }

    #[test]
    fn test_buffer_sweep_expired() {
        let mut buffer = HandoffBuffer::new(0); // TTL = 0 means immediate expiry
        let ctx = HandoffContext::new("agent-a", "agent-b", "test task");
        let id = ctx.handoff_id;

        buffer.insert(ctx);
        assert_eq!(buffer.len(), 1);

        // Sweep should remove the immediately expired entry
        let swept = buffer.sweep_expired();
        assert_eq!(swept, 1);
        assert_eq!(buffer.len(), 0);
        assert!(buffer.is_empty());

        // Get should return None for expired
        let retrieved = buffer.get(&id);
        assert!(retrieved.is_none());
    }

    #[test]
    fn test_buffer_sweep_preserves_live() {
        let mut buffer = HandoffBuffer::new(3600); // 1 hour TTL
        let ctx = HandoffContext::new("agent-a", "agent-b", "test task");
        let id = ctx.handoff_id;

        buffer.insert(ctx);
        assert_eq!(buffer.len(), 1);

        // Sweep should not remove entries with 1 hour TTL
        let swept = buffer.sweep_expired();
        assert_eq!(swept, 0);
        assert_eq!(buffer.len(), 1);

        // Get should still work
        let retrieved = buffer.get(&id);
        assert!(retrieved.is_some());
    }

    #[test]
    fn test_buffer_get_returns_none_for_expired() {
        let mut buffer = HandoffBuffer::new(0); // TTL = 0 means immediate expiry
        let ctx = HandoffContext::new("agent-a", "agent-b", "test task");
        let id = ctx.handoff_id;

        buffer.insert(ctx);
        assert_eq!(buffer.len(), 1);

        // Get should return None because entry is expired (TTL=0)
        let retrieved = buffer.get(&id);
        assert!(retrieved.is_none());

        // But the entry is still in the buffer until sweep
        assert_eq!(buffer.len(), 1);
    }

    #[test]
    fn test_buffer_iter() {
        let mut buffer = HandoffBuffer::new(3600);
        let ctx1 = HandoffContext::new("agent-a", "agent-b", "task 1");
        let ctx2 = HandoffContext::new("agent-c", "agent-d", "task 2");

        buffer.insert(ctx1.clone());
        buffer.insert(ctx2.clone());

        let mut count = 0;
        for (id, ctx, expiry) in buffer.iter() {
            count += 1;
            assert!(*id == ctx1.handoff_id || *id == ctx2.handoff_id);
            assert!(!ctx.task.is_empty());
            assert!(expiry > &Utc::now());
        }
        assert_eq!(count, 2);
    }

    #[test]
    fn test_buffer_uses_context_ttl() {
        let mut buffer = HandoffBuffer::new(3600); // default 1 hour
        let mut ctx = HandoffContext::new("agent-a", "agent-b", "test task");
        ctx.ttl_secs = Some(0); // Override with 0 TTL
        let id = ctx.handoff_id;

        buffer.insert(ctx);

        // Get should return None because context TTL=0
        let retrieved = buffer.get(&id);
        assert!(retrieved.is_none());
    }

    #[test]
    fn test_buffer_default_ttl_when_context_ttl_none() {
        let mut buffer = HandoffBuffer::new(3600); // default 1 hour
        let ctx = HandoffContext::new("agent-a", "agent-b", "test task");
        // ctx.ttl_secs is None, so it should use default
        let id = ctx.handoff_id;

        buffer.insert(ctx);

        // Get should work because default TTL=3600
        let retrieved = buffer.get(&id);
        assert!(retrieved.is_some());
    }

    #[test]
    fn test_buffer_multiple_agents() {
        let mut buffer = HandoffBuffer::new(3600);

        // Insert handoffs to different agents
        buffer.insert(HandoffContext::new("agent-a", "target-1", "task 1"));
        buffer.insert(HandoffContext::new("agent-a", "target-2", "task 2"));
        buffer.insert(HandoffContext::new("agent-b", "target-1", "task 3"));

        assert_eq!(buffer.len(), 3);

        // Get latest for target-1 should return task 3
        let latest = buffer.latest_for_agent("target-1");
        assert!(latest.is_some());
        assert_eq!(latest.unwrap().task, "task 3");

        // Get latest for target-2 should return task 2
        let latest = buffer.latest_for_agent("target-2");
        assert!(latest.is_some());
        assert_eq!(latest.unwrap().task, "task 2");
    }

    // =========================================================================
    // HandoffLedger Tests
    // =========================================================================

    #[test]
    fn test_ledger_append_and_read_all() {
        let dir = tempfile::tempdir().unwrap();
        let ledger_path = dir.path().join("handoff-ledger.jsonl");
        let ledger = HandoffLedger::new(&ledger_path);

        // Create and append 3 entries
        let ctx1 = HandoffContext::new("agent-a", "agent-b", "task 1");
        let ctx2 = HandoffContext::new("agent-b", "agent-c", "task 2");
        let ctx3 = HandoffContext::new("agent-c", "agent-d", "task 3");

        ledger.append(&ctx1).unwrap();
        ledger.append(&ctx2).unwrap();
        ledger.append(&ctx3).unwrap();

        // Read all entries and verify
        let entries = ledger.read_all().unwrap();
        assert_eq!(entries.len(), 3);

        // Verify each entry matches what was appended
        assert_eq!(entries[0].from_agent, "agent-a");
        assert_eq!(entries[0].to_agent, "agent-b");
        assert_eq!(entries[0].task, "task 1");

        assert_eq!(entries[1].from_agent, "agent-b");
        assert_eq!(entries[1].to_agent, "agent-c");
        assert_eq!(entries[1].task, "task 2");

        assert_eq!(entries[2].from_agent, "agent-c");
        assert_eq!(entries[2].to_agent, "agent-d");
        assert_eq!(entries[2].task, "task 3");
    }

    #[test]
    fn test_ledger_append_creates_file() {
        let dir = tempfile::tempdir().unwrap();
        let ledger_path = dir.path().join("new-ledger.jsonl");

        // File should not exist yet
        assert!(!ledger_path.exists());

        let ledger = HandoffLedger::new(&ledger_path);
        let ctx = HandoffContext::new("agent-a", "agent-b", "test task");

        // Append to nonexistent file
        ledger.append(&ctx).unwrap();

        // File should now exist
        assert!(ledger_path.exists());

        // Should be able to read it back
        let entries = ledger.read_all().unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].task, "test task");
    }

    #[test]
    fn test_ledger_count() {
        let dir = tempfile::tempdir().unwrap();
        let ledger_path = dir.path().join("count-ledger.jsonl");
        let ledger = HandoffLedger::new(&ledger_path);

        // First append creates the file
        let ctx = HandoffContext::new("agent-a", "agent-b", "first");
        ledger.append(&ctx).unwrap();

        // Count N entries
        let n = 5;
        for i in 1..n {
            let ctx = HandoffContext::new("agent-a", "agent-b", format!("task {}", i));
            ledger.append(&ctx).unwrap();
        }

        let count = ledger.count().unwrap();
        assert_eq!(count, n);
    }

    #[test]
    fn test_ledger_append_is_one_line_per_entry() {
        let dir = tempfile::tempdir().unwrap();
        let ledger_path = dir.path().join("line-ledger.jsonl");
        let ledger = HandoffLedger::new(&ledger_path);

        let ctx = HandoffContext::new("agent-a", "agent-b", "test task");
        ledger.append(&ctx).unwrap();
        ledger.append(&ctx).unwrap();
        ledger.append(&ctx).unwrap();

        // Read the raw file and count lines
        let content = std::fs::read_to_string(&ledger_path).unwrap();
        let lines: Vec<&str> = content.lines().collect();

        // Should have exactly 3 lines
        assert_eq!(lines.len(), 3);

        // Each line should end with newline (content.lines() strips them)
        // Verify each line is valid JSON
        for (i, line) in lines.iter().enumerate() {
            assert!(!line.is_empty(), "Line {} should not be empty", i);
            let parsed: serde_json::Value = serde_json::from_str(line).unwrap();
            assert!(parsed.is_object());
        }
    }

    #[test]
    fn test_ledger_handles_special_chars() {
        let dir = tempfile::tempdir().unwrap();
        let ledger_path = dir.path().join("special-ledger.jsonl");
        let ledger = HandoffLedger::new(&ledger_path);

        // Create context with special characters
        let mut ctx = HandoffContext::new("agent-a", "agent-b", "line1\nline2\nline3");
        ctx.progress_summary = "Contains \"quotes\" and \t tabs".to_string();
        ctx.decisions = vec![
            "Unicode: 日本語".to_string(),
            "Emoji: 🎉🚀".to_string(),
            "Backslash: C:\\path\\to\\file".to_string(),
        ];

        ledger.append(&ctx).unwrap();

        // Read back and verify
        let entries = ledger.read_all().unwrap();
        assert_eq!(entries.len(), 1);

        let restored = &entries[0];
        assert_eq!(restored.task, "line1\nline2\nline3");
        assert_eq!(restored.progress_summary, "Contains \"quotes\" and \t tabs");
        assert_eq!(restored.decisions.len(), 3);
        assert_eq!(restored.decisions[0], "Unicode: 日本語");
        assert_eq!(restored.decisions[1], "Emoji: 🎉🚀");
        assert_eq!(restored.decisions[2], "Backslash: C:\\path\\to\\file");
    }

    #[test]
    fn test_ledger_size_bytes() {
        let dir = tempfile::tempdir().unwrap();
        let ledger_path = dir.path().join("size-ledger.jsonl");
        let ledger = HandoffLedger::new(&ledger_path);

        // Size should be 0 before any entries (file doesn't exist)
        // Note: size_bytes returns error for non-existent file
        let ctx = HandoffContext::new("agent-a", "agent-b", "test task");
        ledger.append(&ctx).unwrap();

        let size = ledger.size_bytes().unwrap();
        assert!(
            size > 0,
            "Ledger file should have non-zero size after append"
        );

        // Size should increase after second append
        ledger.append(&ctx).unwrap();
        let new_size = ledger.size_bytes().unwrap();
        assert!(
            new_size > size,
            "Ledger size should increase after second append"
        );
    }

    #[test]
    fn test_ttl_overflow_saturates() {
        let mut buffer = HandoffBuffer::new(3600);
        let mut ctx = HandoffContext::new("agent-a", "agent-b", "overflow test");
        ctx.ttl_secs = Some(u64::MAX); // would overflow i64 if cast with `as`

        // Should not panic -- saturates to i64::MAX
        let id = buffer.insert(ctx);

        // Entry should be retrievable (expiry is far in the future)
        let retrieved = buffer.get(&id);
        assert!(retrieved.is_some());
    }
}
