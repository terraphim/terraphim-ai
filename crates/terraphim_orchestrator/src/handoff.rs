use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// Shallow context transferred between agents during handoff.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct HandoffContext {
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
}

impl HandoffContext {
    /// Serialize to JSON string.
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }

    /// Deserialize from JSON string.
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }

    /// Write handoff context to a file.
    pub fn write_to_file(&self, path: impl AsRef<std::path::Path>) -> Result<(), std::io::Error> {
        let json = serde_json::to_string_pretty(self)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        std::fs::write(path, json)
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
        }
    }

    #[test]
    fn test_handoff_roundtrip_json() {
        let original = make_handoff();
        let json = original.to_json().unwrap();
        let restored = HandoffContext::from_json(&json).unwrap();
        assert_eq!(original, restored);
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
    fn test_handoff_empty_decisions() {
        let ctx = HandoffContext {
            task: "simple task".to_string(),
            progress_summary: String::new(),
            decisions: vec![],
            files_touched: vec![],
            timestamp: Utc::now(),
        };
        let json = ctx.to_json().unwrap();
        let restored = HandoffContext::from_json(&json).unwrap();
        assert_eq!(ctx, restored);
        assert!(restored.decisions.is_empty());
    }
}
