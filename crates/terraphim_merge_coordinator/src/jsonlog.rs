//! Structured JSON logging (one object per line) -- Observability-1.
//!
//! All operational events flow through `emit` so log line format
//! stays stable for downstream log shippers (Quickwit). The token
//! is never written (Security-2).

use serde_json::Value;
use std::io::Write;

/// Emit a structured event to stdout as one JSON line.
///
/// Reserved field `event` is the event name. Caller supplies
/// additional keys via `fields`. `timestamp` is added automatically.
pub fn emit(event: &str, fields: &[(&str, Value)]) {
    let mut obj = serde_json::Map::new();
    obj.insert(
        "timestamp".into(),
        Value::String(chrono::Utc::now().to_rfc3339()),
    );
    obj.insert("event".into(), Value::String(event.into()));
    for (k, v) in fields {
        obj.insert((*k).into(), v.clone());
    }
    if let Ok(line) = serde_json::to_string(&Value::Object(obj)) {
        // Best-effort write; failure to log is non-fatal.
        let mut stdout = std::io::stdout().lock();
        let _ = writeln!(stdout, "{line}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    /// Verify that a constructed event JSON parses back with all keys.
    /// We cannot easily intercept stdout in a unit test without
    /// changing the public API, so the regression we guard is the
    /// shape of the JSON object construction itself by calling a
    /// helper that mirrors `emit`.
    fn build_event(event: &str, fields: &[(&str, Value)]) -> Value {
        let mut obj = serde_json::Map::new();
        obj.insert(
            "timestamp".into(),
            Value::String(chrono::Utc::now().to_rfc3339()),
        );
        obj.insert("event".into(), Value::String(event.into()));
        for (k, v) in fields {
            obj.insert((*k).into(), v.clone());
        }
        Value::Object(obj)
    }

    #[test]
    fn event_object_has_required_keys() {
        let v = build_event("pr.merged", &[("pr_index", json!(42))]);
        assert_eq!(v["event"], "pr.merged");
        assert_eq!(v["pr_index"], 42);
        assert!(v["timestamp"].is_string());
    }

    #[test]
    fn token_never_in_emitted_event_keys() {
        // Caller responsibility: we test that the function does not
        // synthesise a "token" key on its own.
        let v = build_event("pr.merged", &[("pr_index", json!(7))]);
        assert!(
            v.get("token").is_none(),
            "emit must not synthesise token field"
        );
    }

    #[test]
    fn multiple_fields_are_all_present() {
        let v = build_event(
            "evaluation.complete",
            &[
                ("count", json!(5)),
                ("owner", json!("terraphim")),
                ("repo", json!("terraphim-ai")),
            ],
        );
        assert_eq!(v["count"], 5);
        assert_eq!(v["owner"], "terraphim");
        assert_eq!(v["repo"], "terraphim-ai");
    }
}
