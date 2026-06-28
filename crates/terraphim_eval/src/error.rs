//! Error type for the codebase evaluation metrics runner.

/// Errors emitted by the metrics runner.
#[derive(Debug, thiserror::Error)]
pub enum EvalError {
    /// The cargo subprocess exited non-zero.
    #[error("cargo command failed (exit {code}): {stderr}")]
    CargoFailed { code: i32, stderr: String },

    /// An I/O error occurred spawning or reading from the subprocess.
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    /// A line of cargo JSON output could not be parsed.
    #[error("failed to parse cargo JSON line: {line}")]
    Parse { line: String },

    /// The supplied manifest path is not a directory.
    #[error("manifest path is not a directory: {0}")]
    NotADirectory(std::path::PathBuf),
}

/// Result alias used throughout the crate.
pub type Result<T> = std::result::Result<T, EvalError>;
