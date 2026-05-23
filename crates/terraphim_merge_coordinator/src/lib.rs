//! Merge coordinator -- minimal skeleton (#1805).
//!
//! Full spec at .docs/spec-merge-coordinator.md will be implemented in
//! follow-up commits. This skeleton proves the crate scaffolds correctly.

pub mod types;
pub mod pid_lock;
pub mod gitea;

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
