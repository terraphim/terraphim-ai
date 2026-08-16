//! Shared debug-session infrastructure for tools.
//!
//! Port of Hermes `tools/debug_helpers.py`. A per-tool [`DebugSession`] that
//! records tool calls to a JSON log file, activated by a tool-specific
//! environment variable (e.g. `WEB_TOOLS_DEBUG=true`). When disabled, all
//! methods are cheap no-ops.

use serde_json::Value;
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

/// Milliseconds since the Unix epoch, used for session ids and timestamps.
fn now_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

/// Per-tool debug session that records tool calls to a JSON log file.
pub struct DebugSession {
    enabled: bool,
    tool_name: String,
    session_id: String,
    log_dir: PathBuf,
    start_time: u64,
    calls: Mutex<Vec<Value>>,
}

impl DebugSession {
    /// Create a debug session for `tool_name`, gated by `env_var`.
    pub fn new(tool_name: &str, env_var: &str) -> Self {
        let enabled = std::env::var(env_var)
            .map(|v| v.eq_ignore_ascii_case("true"))
            .unwrap_or(false);

        Self {
            enabled,
            tool_name: tool_name.to_string(),
            session_id: if enabled {
                format!("{}", now_millis())
            } else {
                String::new()
            },
            log_dir: PathBuf::from("./logs"),
            start_time: now_millis(),
            calls: Mutex::new(Vec::new()),
        }
    }

    /// Whether debug logging is active.
    pub fn active(&self) -> bool {
        self.enabled
    }

    /// Append a tool-call entry to the in-memory log (no-op when disabled).
    pub fn log_call(&self, call_name: &str, call_data: Value) {
        if !self.enabled {
            return;
        }
        let entry = match call_data {
            Value::Object(mut map) => {
                map.insert("timestamp".to_string(), Value::from(now_millis()));
                map.insert("tool_name".to_string(), Value::from(call_name.to_string()));
                Value::Object(map)
            }
            other => serde_json::json!({
                "timestamp": now_millis(),
                "tool_name": call_name,
                "data": other,
            }),
        };
        if let Ok(mut calls) = self.calls.lock() {
            calls.push(entry);
        }
    }

    /// Flush the in-memory log to a JSON file (no-op when disabled).
    pub fn save(&self) {
        if !self.enabled {
            return;
        }
        let payload = {
            let calls = self.calls.lock().unwrap();
            serde_json::json!({
                "session_id": self.session_id,
                "start_time": self.start_time,
                "end_time": now_millis(),
                "debug_enabled": true,
                "total_calls": calls.len(),
                "tool_calls": &*calls,
            })
        };

        let filename = format!("{}_debug_{}.json", self.tool_name, self.session_id);
        let filepath = self.log_dir.join(filename);

        if let Err(e) = std::fs::create_dir_all(&self.log_dir) {
            log::error!("Error creating debug log dir: {}", e);
            return;
        }
        if let Err(e) = std::fs::write(&filepath, payload.to_string()) {
            log::error!("Error saving {} debug log: {}", self.tool_name, e);
            return;
        }
        log::debug!("{} debug log saved: {}", self.tool_name, filepath.display());
    }

    /// Return a summary dict (Hermes `get_session_info`).
    pub fn get_session_info(&self) -> Value {
        if !self.enabled {
            return serde_json::json!({
                "enabled": false,
                "session_id": null,
                "log_path": null,
                "total_calls": 0,
            });
        }
        let total = self.calls.lock().map(|c| c.len()).unwrap_or(0);
        let log_path = self
            .log_dir
            .join(format!("{}_debug_{}.json", self.tool_name, self.session_id));
        serde_json::json!({
            "enabled": true,
            "session_id": self.session_id,
            "log_path": log_path.to_string_lossy(),
            "total_calls": total,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn disabled_by_default() {
        // Use an env var name that is almost certainly unset in CI.
        let session = DebugSession::new("test_tool", "TINYCLAW_DEFINITELY_UNSET_DEBUG");
        assert!(!session.active());
        assert_eq!(session.get_session_info()["enabled"], false);
        assert_eq!(session.get_session_info()["total_calls"], 0);
        // log_call and save are no-ops when disabled.
        session.log_call("x", serde_json::json!({"a": 1}));
        session.save();
        assert_eq!(session.get_session_info()["total_calls"], 0);
    }

    #[test]
    fn enabled_records_calls() {
        unsafe {
            std::env::set_var("TINYCLAW_TEST_DEBUG", "true");
        }
        let session = DebugSession::new("test_tool", "TINYCLAW_TEST_DEBUG");
        assert!(session.active());

        session.log_call("do_thing", serde_json::json!({"result": 42}));
        let info = session.get_session_info();
        assert_eq!(info["enabled"], true);
        assert_eq!(info["total_calls"], 1);
        assert!(info["session_id"].is_string());
        assert!(info["log_path"].is_string());
        unsafe {
            std::env::remove_var("TINYCLAW_TEST_DEBUG");
        }
    }
}
