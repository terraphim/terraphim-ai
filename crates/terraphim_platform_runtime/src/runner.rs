//! Runner-kind vocabulary for the Terraphim platform runtime.

/// Which CI or execution runner hosts a step.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum RunnerKind {
    /// Local host runner.
    Local,
    /// Gitea Actions runner.
    Gitea,
    /// GitHub Actions runner.
    Github,
}
