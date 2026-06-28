//! Codebase evaluation metrics runner.
//!
//! Implements the smallest actionable slice of
//! `docs/specifications/terraphim-codebase-eval-check.md` (§3 Metrics Runner):
//! executes `cargo clippy` and `cargo test`, parses their machine-readable
//! JSON output, and emits normalised [`MetricRecord`]s.
//!
//! The Verdict Engine (§4), Artifacts Store (§5), and CI Integration (§6)
//! described in the spec are intentionally out of scope for this slice.

mod error;
mod metrics_runner;
mod types;

pub use error::{EvalError, Result};
pub use metrics_runner::{MetricsRunner, parse_clippy_json, parse_test_json};
pub use types::{MetricCounts, MetricRecord, PassFail};

use std::path::{Path, PathBuf};
use std::process::Command;

/// Run `cargo clippy --message-format=json --quiet` in `manifest_dir` and
/// normalise the result into a [`MetricRecord`].
///
/// # Errors
/// - [`EvalError::NotADirectory`] if `manifest_dir` is not a directory.
/// - [`EvalError::CargoFailed`] if cargo itself fails to run (non-zero exit
///   **and** no JSON was produced). Note: clippy exits non-zero when it finds
///   lints, but the JSON output is still parsed and returned as a record.
pub fn run_clippy(manifest_dir: impl AsRef<Path>) -> Result<MetricRecord> {
    run_cargo(
        manifest_dir.as_ref(),
        "clippy",
        "cargo-clippy",
        &["clippy", "--message-format=json", "--quiet"],
    )
}

/// Run `cargo test --message-format=json --quiet` in `manifest_dir` and
/// normalise the result into a [`MetricRecord`].
///
/// # Errors
/// - [`EvalError::NotADirectory`] if `manifest_dir` is not a directory.
/// - [`EvalError::CargoFailed`] if cargo fails to spawn.
pub fn run_test(manifest_dir: impl AsRef<Path>) -> Result<MetricRecord> {
    run_cargo(
        manifest_dir.as_ref(),
        "test",
        "cargo-test",
        &["test", "--message-format=json", "--quiet"],
    )
}

fn run_cargo(
    manifest_dir: &Path,
    metric_id: &str,
    tool: &str,
    args: &[&str],
) -> Result<MetricRecord> {
    let manifest_dir = canonicalize_dir(manifest_dir)?;
    let output = Command::new("cargo")
        .args(args)
        .current_dir(&manifest_dir)
        .output()?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let code = output.status.code();

    let counts = match metric_id {
        "clippy" => parse_clippy_json(&stdout)?,
        "test" => parse_test_json(&stdout)?,
        other => unreachable!("unsupported metric_id: {other}"),
    };

    // If cargo failed to even produce parseable output, surface the error.
    // Clippy exits 101 when it finds errors, but stdout is still valid JSON
    // and counts.errors > 0 — that is a normal "fail" record, not an error.
    if counts == MetricCounts::default() && !output.status.success() && code.is_some() {
        return Err(EvalError::CargoFailed {
            code: code.unwrap_or(-1),
            stderr: stderr.to_string(),
        });
    }

    Ok(MetricRecord::new(
        metric_id,
        tool,
        counts,
        code,
        Some(manifest_dir),
    ))
}

fn canonicalize_dir(path: &Path) -> Result<PathBuf> {
    if !path.is_dir() {
        return Err(EvalError::NotADirectory(path.to_path_buf()));
    }
    Ok(path.canonicalize().unwrap_or_else(|_| path.to_path_buf()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn run_clippy_on_missing_dir_returns_not_a_directory() {
        let result = run_clippy("/nonexistent/path/does/not/exist");
        assert!(matches!(result, Err(EvalError::NotADirectory(_))));
    }

    #[test]
    fn run_test_on_missing_dir_returns_not_a_directory() {
        let result = run_test("/nonexistent/path/does/not/exist");
        assert!(matches!(result, Err(EvalError::NotADirectory(_))));
    }

    #[test]
    fn run_clippy_on_existing_dir_does_not_return_not_a_directory() {
        // The crate's own manifest dir exists; this sanity-checks that a real
        // directory is not wrongly rejected. (Cargo may still fail if the dir
        // lacks a Cargo.toml, but that is a different error, not NotADirectory.)
        let result = run_clippy(env!("CARGO_MANIFEST_DIR"));
        assert!(!matches!(result, Err(EvalError::NotADirectory(_))));
    }
}
