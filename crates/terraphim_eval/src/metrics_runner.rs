//! Parsers for cargo's `--message-format=json` machine-readable output.
//!
//! Both `cargo clippy` and `cargo test` emit one JSON object per line
//! (newline-delimited JSON). Each line is an independent message; the parser
//! accumulates counts across all lines. Non-JSON lines (e.g. compiler
//! progress markers when not using `--message-format=json`) are silently
//! skipped — they carry no countable signal.
//!
//! Schema references:
//! - clippy: each diagnostic line has `reason: "compiler-message"` and a
//!   `message.level` of `"warning"` or `"error"`.
//! - test:  each event line has `type: "test"` and an `event` field of
//!   `"ok"`, `"failed"`, or `"ignored"`.

use std::path::Path;

use crate::error::EvalError;
use crate::types::{MetricCounts, MetricRecord};

/// Executor that runs `cargo clippy` / `cargo test` and normalises their
/// machine-readable JSON output into [`MetricRecord`]s.
///
/// This is the high-level entry point of the Metrics Runner slice of
/// `docs/specifications/terraphim-codebase-eval-check.md` (§3). It owns no
/// mutable state; each call spawns an independent subprocess.
///
/// [`MetricRecord`]: crate::MetricRecord
#[derive(Debug, Clone, Default)]
pub struct MetricsRunner;

impl MetricsRunner {
    /// Construct a new runner.
    #[must_use]
    pub fn new() -> Self {
        Self
    }

    /// Run `cargo clippy --message-format=json --quiet` in `manifest_dir` and
    /// return the normalised [`MetricRecord`](crate::MetricRecord).
    ///
    /// # Errors
    /// See [`crate::run_clippy`].
    pub fn run_clippy(&self, manifest_dir: impl AsRef<Path>) -> Result<MetricRecord, EvalError> {
        crate::run_clippy(manifest_dir)
    }

    /// Run `cargo test --message-format=json --quiet` in `manifest_dir` and
    /// return the normalised [`MetricRecord`](crate::MetricRecord).
    ///
    /// # Errors
    /// See [`crate::run_test`].
    pub fn run_test(&self, manifest_dir: impl AsRef<Path>) -> Result<MetricRecord, EvalError> {
        crate::run_test(manifest_dir)
    }
}
/// Parse the captured stdout of `cargo clippy --message-format=json`.
///
/// Counts `warning`- and `error`-level diagnostics. Lines that are not valid
/// JSON or not compiler messages are ignored (cargo may emit status lines).
pub fn parse_clippy_json(output: &str) -> Result<MetricCounts, EvalError> {
    let mut counts = MetricCounts::default();
    for raw in output.lines() {
        let line = raw.trim();
        if line.is_empty() || !line.starts_with('{') {
            continue;
        }
        let v: serde_json::Value = match serde_json::from_str(line) {
            Ok(v) => v,
            Err(_) => {
                return Err(EvalError::Parse {
                    line: line.to_string(),
                });
            }
        };
        // Only count compiler messages; other reasons (e.g. "compiler-artifact")
        // carry no diagnostic severity.
        let reason = v.get("reason").and_then(|r| r.as_str()).unwrap_or("");
        if reason != "compiler-message" {
            continue;
        }
        let level = v
            .get("message")
            .and_then(|m| m.get("level"))
            .and_then(|l| l.as_str())
            .unwrap_or("");
        match level {
            "warning" => counts.warnings += 1,
            "error" => counts.errors += 1,
            _ => {}
        }
    }
    Ok(counts)
}

/// Parse the captured stdout of `cargo test --message-format=json`.
///
/// Counts test events: `"ok"` → passed, `"failed"` → failed,
/// `"ignored"` → ignored. Only `type: "test"` lines are counted.
pub fn parse_test_json(output: &str) -> Result<MetricCounts, EvalError> {
    let mut counts = MetricCounts::default();
    for raw in output.lines() {
        let line = raw.trim();
        if line.is_empty() || !line.starts_with('{') {
            continue;
        }
        let v: serde_json::Value = match serde_json::from_str(line) {
            Ok(v) => v,
            Err(_) => {
                return Err(EvalError::Parse {
                    line: line.to_string(),
                });
            }
        };
        let ty = v.get("type").and_then(|t| t.as_str()).unwrap_or("");
        if ty != "test" {
            continue;
        }
        let event = v.get("event").and_then(|e| e.as_str()).unwrap_or("");
        match event {
            "ok" => counts.passed += 1,
            "failed" => counts.failed += 1,
            "ignored" => counts.ignored += 1,
            _ => {}
        }
    }
    Ok(counts)
}

#[cfg(test)]
mod tests {
    use super::*;

    const CLIPPY_WARNING: &str = r#"{"reason":"compiler-message","message":{"level":"warning","message":"unused variable","spans":[]}}"#;
    const CLIPPY_ERROR: &str = r#"{"reason":"compiler-message","message":{"level":"error","message":"cannot find value","spans":[]}}"#;
    const CLIPPY_ARTIFACT: &str =
        r#"{"reason":"compiler-artifact","target":{"name":"foo","kind":["lib"]}}"#;
    const TEST_OK: &str = r#"{"type":"test","event":"ok","name":"tests::it_works"}"#;
    const TEST_FAILED: &str = r#"{"type":"test","event":"failed","name":"tests::it_breaks"}"#;
    const TEST_IGNORED: &str = r#"{"type":"test","event":"ignored","name":"tests::slow"}"#;
    const TEST_SUITE: &str = r#"{"type":"suite","event":"ok"}"#;

    #[test]
    fn parse_clippy_counts_warnings_and_errors() {
        let output =
            format!("{CLIPPY_ARTIFACT}\n{CLIPPY_WARNING}\n{CLIPPY_WARNING}\n{CLIPPY_ERROR}\n");
        let counts = parse_clippy_json(&output).unwrap();
        assert_eq!(counts.warnings, 2);
        assert_eq!(counts.errors, 1);
        assert_eq!(counts.passed, 0);
    }

    #[test]
    fn parse_clippy_handles_empty_output() {
        let counts = parse_clippy_json("").unwrap();
        assert_eq!(counts, MetricCounts::default());
    }

    #[test]
    fn parse_clippy_ignores_non_json_and_non_compiler_lines() {
        let output = format!("    Compiling foo v0.1.0\n{CLIPPY_ARTIFACT}\n   Finished\n");
        let counts = parse_clippy_json(&output).unwrap();
        assert_eq!(counts, MetricCounts::default());
    }

    #[test]
    fn parse_test_counts_pass_fail_ignored() {
        let output = format!("{TEST_OK}\n{TEST_OK}\n{TEST_FAILED}\n{TEST_IGNORED}\n{TEST_SUITE}\n");
        let counts = parse_test_json(&output).unwrap();
        assert_eq!(counts.passed, 2);
        assert_eq!(counts.failed, 1);
        assert_eq!(counts.ignored, 1);
    }

    #[test]
    fn parse_test_handles_empty_output() {
        let counts = parse_test_json("").unwrap();
        assert_eq!(counts, MetricCounts::default());
    }

    #[test]
    fn parse_test_ignores_suite_events() {
        let output = format!("{TEST_SUITE}\n");
        let counts = parse_test_json(&output).unwrap();
        assert_eq!(counts, MetricCounts::default());
    }

    #[test]
    fn parse_clippy_error_on_invalid_json_object_line() {
        // A line that starts with '{' but is not valid JSON is an error:
        // cargo's contract is one valid JSON object per line.
        let counts = parse_clippy_json("{ not valid json }");
        assert!(matches!(counts, Err(EvalError::Parse { .. })));
    }

    #[test]
    fn test_clippy_clean_project_emits_zero_warnings() {
        // A clippy-clean project emits no `compiler-message` warnings or
        // errors — only build-script / artifact lines, which are skipped.
        let output = format!("{CLIPPY_ARTIFACT}\n    Finished\n");
        let counts = parse_clippy_json(&output).unwrap();
        assert_eq!(counts.warnings, 0);
        assert_eq!(counts.errors, 0);
    }

    #[test]
    fn test_test_results_parse_pass_fail_counts() {
        // Two passing, one failed, one ignored — plus a suite event that
        // must NOT be counted as a test.
        let output = format!("{TEST_OK}\n{TEST_OK}\n{TEST_FAILED}\n{TEST_IGNORED}\n{TEST_SUITE}\n");
        let counts = parse_test_json(&output).unwrap();
        assert_eq!(counts.passed, 2);
        assert_eq!(counts.failed, 1);
        assert_eq!(counts.ignored, 1);
    }
}
