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

/// Extract issue-closing references from a PR body. Case-insensitive.
///
/// Matches all standard Gitea/GitHub closing keywords followed by `#N`:
/// `closes`, `close`, `fixes`, `fix`, `resolves`, `resolve`.
/// Does NOT match `Refs #N` (which should not trigger auto-close).
///
/// Returns a sorted, deduplicated list of issue numbers.
pub fn extract_fixes(body: &str) -> Vec<u64> {
    const CLOSING_KEYWORDS: &[&str] = &[
        "closes #",
        "close #",
        "fixes #",
        "fix #",
        "resolves #",
        "resolve #",
    ];
    let lower = body.to_lowercase();
    let mut out = std::collections::BTreeSet::new();
    for keyword in CLOSING_KEYWORDS {
        for chunk in lower.split(keyword).skip(1) {
            let digits: String = chunk.chars().take_while(|c| c.is_ascii_digit()).collect();
            if let Ok(n) = digits.parse::<u64>() {
                out.insert(n);
            }
        }
    }
    out.into_iter().collect()
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

    #[test]
    fn extract_fixes_closes_keyword() {
        assert_eq!(extract_fixes("Closes #2850"), vec![2850]);
        assert_eq!(extract_fixes("close #7"), vec![7]);
        assert_eq!(extract_fixes("CLOSES #99"), vec![99]);
    }

    #[test]
    fn extract_fixes_fix_singular() {
        assert_eq!(extract_fixes("Fix #42"), vec![42]);
        assert_eq!(extract_fixes("FIX #100"), vec![100]);
    }

    #[test]
    fn extract_fixes_resolves_keyword() {
        assert_eq!(extract_fixes("Resolves #300"), vec![300]);
        assert_eq!(extract_fixes("resolve #301"), vec![301]);
        assert_eq!(extract_fixes("RESOLVES #302"), vec![302]);
    }

    #[test]
    fn extract_fixes_deduplicates_same_issue() {
        // If both "Fixes #5" and "Closes #5" appear, issue 5 is returned once.
        assert_eq!(extract_fixes("Fixes #5\nCloses #5"), vec![5]);
    }

    #[test]
    fn extract_fixes_mixed_keywords() {
        // Real-world PR body with multiple keywords referencing different issues.
        let body = "Closes #2850\nFixes #2851\nResolves #2852";
        assert_eq!(extract_fixes(body), vec![2850, 2851, 2852]);
    }

    #[test]
    fn extract_fixes_refs_still_excluded() {
        // "Refs #N" must never trigger auto-close.
        assert_eq!(extract_fixes("Refs #42"), Vec::<u64>::new());
        assert_eq!(extract_fixes("refs #99"), Vec::<u64>::new());
    }
}
