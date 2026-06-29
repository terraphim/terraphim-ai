//! Merge coordinator library (#1805).
//!
//! Evaluates open PRs for auto-merge readiness, merges mergeable PRs, and
//! auto-closes issues referenced via `Fixes #N`. Core logic lives in the
//! [`evaluator`] module; shared types in [`types`].

pub mod evaluator;
pub mod gitea;
pub mod jsonlog;
pub mod pid_lock;
pub mod types;

/// Extract `Fixes #N` references from a PR body. Case-insensitive.
/// Does NOT match `Refs #N` (which should not trigger auto-close).
pub fn extract_fixes(body: &str) -> Vec<u64> {
    let mut out = Vec::new();
    let lower = body.to_lowercase();
    for chunk in lower.split("fixes #").skip(1) {
        let digits: String = chunk.chars().take_while(|c| c.is_ascii_digit()).collect();
        if let Ok(n) = digits.parse::<u64>() {
            out.push(n);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_fixes_matches_fixes_not_refs() {
        assert_eq!(extract_fixes("Fixes #1234"), vec![1234]);
        assert_eq!(extract_fixes("Refs #1234"), Vec::<u64>::new());
    }

    #[test]
    fn extract_fixes_case_insensitive() {
        assert_eq!(extract_fixes("FIXES #5"), vec![5]);
        assert_eq!(extract_fixes("fixes #6"), vec![6]);
    }

    #[test]
    fn extract_fixes_multiple() {
        assert_eq!(extract_fixes("Fixes #1 closes\nFixes #2"), vec![1, 2]);
    }
}
