//! Artefact contamination gate for the merge dispatcher (#2404).
//!
//! Checks a PR's changed-file list against a denylist of known artefact
//! patterns (session files, review temp dirs, bak files, etc.). Any PR
//! that adds a denylisted path is held rather than merged.

use crate::gitea::PrFileChange;

/// Path patterns whose *presence as added files* signals artefact
/// contamination. A path matches if it:
/// - starts with a prefix pattern (entries ending in `/`), or
/// - ends with a suffix pattern (entries starting with `.`), or
/// - equals an exact name (all other entries).
const ARTEFACT_DENYLIST: &[&str] = &[
    // Review artefacts
    ".pr_review_temp/",
    "cursor_new.rs",
    // Session/report artefacts
    "reports/",
    ".sessions/",
    // Backup files
    ".bak",
    // Session JSONL dumps
    ".jsonl",
    // Heredoc residue files accidentally committed
    "EOF",
    "PYEOF",
];

/// Check whether a file path is a denylisted artefact.
pub fn is_contaminated_path(path: &str) -> bool {
    for &pattern in ARTEFACT_DENYLIST {
        if pattern.ends_with('/') {
            // Prefix match: path must start with the prefix.
            if path.starts_with(pattern) {
                return true;
            }
        } else if pattern.starts_with('.') {
            // Suffix match: path must end with the suffix.
            if path.ends_with(pattern) {
                return true;
            }
        } else {
            // Exact filename match: last component of path equals pattern.
            let file_name = path.rsplit('/').next().unwrap_or(path);
            if file_name == pattern {
                return true;
            }
        }
    }
    false
}

/// Inspect a PR's changed files and return the subset of *added* paths
/// that match the artefact denylist. An empty return means the PR is clean.
pub fn check_contamination(files: &[PrFileChange]) -> Vec<String> {
    files
        .iter()
        .filter(|f| f.status == "added" || f.status == "renamed")
        .map(|f| f.filename.as_str())
        .filter(|path| is_contaminated_path(path))
        .map(|s| s.to_string())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn added(filename: &str) -> PrFileChange {
        PrFileChange {
            filename: filename.into(),
            status: "added".into(),
            additions: 1,
        }
    }

    fn modified(filename: &str) -> PrFileChange {
        PrFileChange {
            filename: filename.into(),
            status: "modified".into(),
            additions: 5,
        }
    }

    fn removed(filename: &str) -> PrFileChange {
        PrFileChange {
            filename: filename.into(),
            status: "removed".into(),
            additions: 0,
        }
    }

    // --- is_contaminated_path unit tests ---

    #[test]
    fn clean_rust_source_not_contaminated() {
        assert!(!is_contaminated_path("src/lib.rs"));
        assert!(!is_contaminated_path(
            "crates/terraphim_service/src/main.rs"
        ));
    }

    #[test]
    fn pr_review_temp_prefix_detected() {
        assert!(is_contaminated_path(".pr_review_temp/pr123.diff"));
        assert!(is_contaminated_path(".pr_review_temp/review_notes.md"));
    }

    #[test]
    fn reports_prefix_detected() {
        assert!(is_contaminated_path("reports/spec-validation-20260610.md"));
        assert!(is_contaminated_path("reports/pii-scan-2026-06-10.json"));
    }

    #[test]
    fn sessions_prefix_detected() {
        assert!(is_contaminated_path(".sessions/session-20260611-0115.md"));
        assert!(is_contaminated_path(
            ".sessions/handover-20260611-echo-2398.md"
        ));
    }

    #[test]
    fn bak_suffix_detected() {
        assert!(is_contaminated_path("Cargo.toml.bak"));
        assert!(is_contaminated_path("some/deep/path/config.bak"));
    }

    #[test]
    fn jsonl_suffix_detected() {
        assert!(is_contaminated_path("sessions/data.jsonl"));
        assert!(is_contaminated_path("dump.jsonl"));
    }

    #[test]
    fn cursor_new_rs_exact_match() {
        assert!(is_contaminated_path("cursor_new.rs"));
        // In subdirectory -- exact filename match still fires
        assert!(is_contaminated_path("src/cursor_new.rs"));
    }

    #[test]
    fn eof_pyeof_heredoc_residue() {
        assert!(is_contaminated_path("EOF"));
        assert!(is_contaminated_path("PYEOF"));
        // In a subdirectory
        assert!(is_contaminated_path("scripts/EOF"));
    }

    #[test]
    fn similar_names_not_matched() {
        // "reports" with no trailing slash should NOT match as prefix
        assert!(!is_contaminated_path("report_summary.md"));
        assert!(!is_contaminated_path("reporter.rs"));
        // ".sessions" as infix should not match a random file
        assert!(!is_contaminated_path("src/sessions.rs"));
    }

    // --- check_contamination tests ---

    #[test]
    fn clean_pr_returns_empty() {
        let files = vec![
            added("src/lib.rs"),
            modified("Cargo.toml"),
            removed("old_code.rs"),
        ];
        assert_eq!(check_contamination(&files), Vec::<String>::new());
    }

    #[test]
    fn contaminated_pr_returns_offending_paths() {
        let files = vec![
            added("src/lib.rs"),
            added(".sessions/handover-20260611.md"),
            added("reports/pii-scan.json"),
            modified("Cargo.toml"),
        ];
        let result = check_contamination(&files);
        assert_eq!(result.len(), 2);
        assert!(result.contains(&".sessions/handover-20260611.md".to_string()));
        assert!(result.contains(&"reports/pii-scan.json".to_string()));
    }

    #[test]
    fn removed_contaminated_files_do_not_trigger_gate() {
        // Removing an artefact is fine -- we only gate on additions.
        let files = vec![removed(".sessions/old_session.md"), removed("EOF")];
        assert_eq!(check_contamination(&files), Vec::<String>::new());
    }

    #[test]
    fn modified_contaminated_files_do_not_trigger_gate() {
        // Modifying an already-present artefact is not "adding" it.
        let files = vec![modified(".sessions/existing.md")];
        assert_eq!(check_contamination(&files), Vec::<String>::new());
    }

    #[test]
    fn renamed_file_triggers_gate_if_destination_contaminated() {
        // A rename whose destination is a session path is still "adding" that path.
        let files = vec![PrFileChange {
            filename: ".sessions/new_session.md".into(),
            status: "renamed".into(),
            additions: 10,
        }];
        assert_eq!(
            check_contamination(&files),
            vec![".sessions/new_session.md".to_string()]
        );
    }

    #[test]
    fn multiple_artefact_types_all_reported() {
        let files = vec![
            added("cursor_new.rs"),
            added("Cargo.toml.bak"),
            added(".pr_review_temp/review.diff"),
        ];
        let result = check_contamination(&files);
        assert_eq!(result.len(), 3);
    }
}
