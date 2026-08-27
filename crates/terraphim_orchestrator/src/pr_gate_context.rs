//! Native PR gate evidence construction.
//!
//! This module prepares bounded, deterministic context for PR gate producers.
//! It deliberately does not post comments, statuses, or invoke shell scripts.

use std::path::Path;

use terraphim_automata::{compute_concepts_matched, thesaurus_from_terms};
use terraphim_types::RoleName;
use tokio::process::Command;

use crate::pr_dispatch::ReviewPrRequest;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrGateEvidenceLimits {
    pub max_diff_lines: usize,
    pub max_issue_chars: usize,
    pub max_context_chunks: usize,
    pub max_context_chars: usize,
}

impl Default for PrGateEvidenceLimits {
    fn default() -> Self {
        Self {
            max_diff_lines: 1_200,
            max_issue_chars: 6_000,
            max_context_chunks: 8,
            max_context_chars: 12_000,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrGateEvidencePack {
    pub pr_number: u64,
    pub project: String,
    pub title: String,
    pub author: String,
    pub head_sha: String,
    pub diff_loc: u32,
    pub changed_files: Vec<String>,
    pub diff_excerpt: String,
    pub linked_issue: Option<LinkedIssueEvidence>,
    pub matched_concepts: Vec<String>,
    pub relevant_context: Vec<RelevantContextChunk>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LinkedIssueEvidence {
    pub number: u64,
    pub title: String,
    pub body_excerpt: String,
    pub acceptance_criteria: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RelevantContextChunk {
    pub source: String,
    pub reason: String,
    pub text: String,
}

#[derive(Debug, thiserror::Error)]
pub enum PrGateContextError {
    #[error("authoritative PR evidence requires a working directory")]
    MissingWorkingDirectory,
    #[error("git command failed: {0}")]
    Git(String),
}

pub async fn build_pr_gate_evidence_pack(
    req: &ReviewPrRequest,
    working_dir: Option<&Path>,
    limits: PrGateEvidenceLimits,
) -> Result<PrGateEvidencePack, PrGateContextError> {
    let working_dir = working_dir.ok_or(PrGateContextError::MissingWorkingDirectory)?;
    let git_evidence = collect_git_evidence(working_dir, req, &limits).await?;

    let linked_issue = extract_issue_number(&req.title).map(|number| LinkedIssueEvidence {
        number,
        title: format!("Issue #{number}"),
        body_excerpt: String::new(),
        acceptance_criteria: Vec::new(),
    });

    let text_for_matching = format!(
        "{}\n{}\n{}\n{}",
        req.project,
        req.title,
        git_evidence.changed_files.join("\n"),
        git_evidence.diff_excerpt
    );

    Ok(PrGateEvidencePack {
        pr_number: req.pr_number,
        project: req.project.clone(),
        title: req.title.clone(),
        author: req.author_login.clone(),
        head_sha: req.head_sha.clone(),
        diff_loc: req.diff_loc,
        changed_files: git_evidence.changed_files,
        diff_excerpt: git_evidence.diff_excerpt,
        linked_issue,
        matched_concepts: extract_builtin_concepts(&text_for_matching),
        relevant_context: Vec::new(),
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct GitEvidence {
    changed_files: Vec<String>,
    diff_excerpt: String,
}

async fn collect_git_evidence(
    working_dir: &Path,
    req: &ReviewPrRequest,
    limits: &PrGateEvidenceLimits,
) -> Result<GitEvidence, PrGateContextError> {
    let pr_ref = format!("pull/{}/head:refs/adf/pr-{}", req.pr_number, req.pr_number);
    fetch_first_available(working_dir, &["origin", "gitea"], &pr_ref, req.pr_number).await;
    if let Some(branch_ref) = build_head_branch_refspec(&req.head_ref, req.pr_number) {
        fetch_first_available(
            working_dir,
            &["origin", "gitea"],
            &branch_ref,
            req.pr_number,
        )
        .await;
    }
    fetch_first_available(
        working_dir,
        &["origin", "gitea"],
        "main:refs/adf/base-main",
        req.pr_number,
    )
    .await;

    let pr_branch = format!("refs/adf/pr-{}", req.pr_number);
    let ranges = build_diff_ranges(&pr_branch, &req.head_sha);
    let mut last_error = None;
    for range in ranges {
        match git_output(working_dir, &["diff", "--no-ext-diff", &range]).await {
            Ok(diff) if !diff.trim().is_empty() => {
                return Ok(GitEvidence {
                    changed_files: extract_changed_files(&diff),
                    diff_excerpt: limit_lines(&diff, limits.max_diff_lines),
                });
            }
            Ok(_) => {
                last_error = Some(format!("git diff {range} returned no changes"));
            }
            Err(e) => {
                last_error = Some(e.to_string());
            }
        }
    }

    let reason = last_error.unwrap_or_else(|| "no usable git diff range".to_string());
    Err(PrGateContextError::Git(format!(
        "no authoritative diff for PR {}: {reason}",
        req.pr_number
    )))
}

fn is_safe_head_ref(head_ref: &str) -> bool {
    !head_ref.trim().is_empty() && !head_ref.contains(':') && !head_ref.starts_with('-')
}

fn build_head_branch_refspec(head_ref: &str, pr_number: u64) -> Option<String> {
    is_safe_head_ref(head_ref).then(|| format!("refs/heads/{head_ref}:refs/adf/pr-{pr_number}"))
}

async fn fetch_first_available(
    working_dir: &Path,
    remotes: &[&str],
    refspec: &str,
    pr_number: u64,
) {
    for remote in remotes {
        let output = Command::new("git")
            .current_dir(working_dir)
            .args(["fetch", remote, refspec])
            .output()
            .await;

        match output {
            Ok(output) if output.status.success() => return,
            Ok(output) => {
                tracing::debug!(
                    stderr = %String::from_utf8_lossy(&output.stderr),
                    remote,
                    refspec,
                    pr_number,
                    "native PR gate fetch failed; trying next remote"
                );
            }
            Err(e) => {
                tracing::debug!(
                    error = %e,
                    remote,
                    refspec,
                    pr_number,
                    "native PR gate fetch errored; trying next remote"
                );
            }
        }
    }
}

fn build_diff_ranges(pr_branch: &str, head_sha: &str) -> Vec<String> {
    [
        format!("refs/adf/base-main...{pr_branch}"),
        format!("origin/main...{pr_branch}"),
        format!("main...{pr_branch}"),
        format!("refs/adf/base-main...{head_sha}"),
        format!("origin/main...{head_sha}"),
        format!("main...{head_sha}"),
        format!("refs/adf/base-main..{pr_branch}"),
        format!("refs/adf/base-main..{head_sha}"),
        format!("origin/main..{pr_branch}"),
        format!("origin/main..{head_sha}"),
    ]
    .into_iter()
    .collect()
}

async fn git_output(working_dir: &Path, args: &[&str]) -> Result<String, PrGateContextError> {
    let output = Command::new("git")
        .current_dir(working_dir)
        .args(args)
        .output()
        .await
        .map_err(|e| PrGateContextError::Git(e.to_string()))?;

    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    } else {
        Err(PrGateContextError::Git(
            String::from_utf8_lossy(&output.stderr).trim().to_string(),
        ))
    }
}

pub fn extract_changed_files(diff: &str) -> Vec<String> {
    let mut files = Vec::new();
    for line in diff.lines() {
        if let Some(path) = line.strip_prefix("diff --git a/") {
            if let Some((_, right)) = path.split_once(" b/") {
                let candidate = right.trim().to_string();
                if !files.contains(&candidate) {
                    files.push(candidate);
                }
            }
        }
    }
    files
}

pub fn extract_issue_number(text: &str) -> Option<u64> {
    let bytes = text.as_bytes();
    for index in 0..bytes.len() {
        if bytes[index] == b'#' {
            let digits: String = text[index + 1..]
                .chars()
                .take_while(|c| c.is_ascii_digit())
                .collect();
            if let Ok(number) = digits.parse::<u64>() {
                return Some(number);
            }
        }
    }
    None
}

pub fn limit_lines(text: &str, max_lines: usize) -> String {
    let mut lines = text.lines();
    let mut out = lines
        .by_ref()
        .take(max_lines)
        .collect::<Vec<_>>()
        .join("\n");
    if lines.next().is_some() {
        out.push_str("\n[diff truncated]");
    }
    out
}

fn extract_builtin_concepts(text: &str) -> Vec<String> {
    let terms = [
        "PrGateResult",
        "branch protection",
        "Gitea",
        "orchestrator",
        "validation",
        "verification",
        "review",
        "status",
        "timeout",
        "native runner",
        "knowledge graph",
    ];
    let role = RoleName::new("ADF PR Gate");
    let thesaurus = thesaurus_from_terms(&role, terms.iter().copied());
    let mut concepts = compute_concepts_matched(text, &thesaurus);
    concepts.sort();
    concepts.dedup();
    concepts
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_changed_files_from_git_diff_headers() {
        let diff = "diff --git a/src/lib.rs b/src/lib.rs\nindex 1..2\ndiff --git a/README.md b/README.md\n";
        assert_eq!(
            extract_changed_files(diff),
            vec!["src/lib.rs".to_string(), "README.md".to_string()]
        );
    }

    #[test]
    fn extract_issue_number_from_title() {
        assert_eq!(extract_issue_number("Fix #2334: native gates"), Some(2334));
        assert_eq!(extract_issue_number("No issue here"), None);
    }

    #[test]
    fn limit_lines_marks_truncation() {
        let limited = limit_lines("a\nb\nc", 2);
        assert_eq!(limited, "a\nb\n[diff truncated]");
    }

    #[test]
    fn build_diff_ranges_prefers_fetched_base_and_pr_refs() {
        let ranges = build_diff_ranges("refs/adf/pr-2318", "abc123");
        assert_eq!(ranges[0], "refs/adf/base-main...refs/adf/pr-2318");
        assert!(ranges.contains(&"origin/main...refs/adf/pr-2318".to_string()));
        assert!(ranges.contains(&"refs/adf/base-main..abc123".to_string()));
    }

    #[tokio::test]
    async fn evidence_pack_requires_working_directory() {
        let err = build_pr_gate_evidence_pack(
            &fixture_request(1, "abc"),
            None,
            PrGateEvidenceLimits::default(),
        )
        .await
        .unwrap_err();
        assert!(matches!(err, PrGateContextError::MissingWorkingDirectory));
    }

    #[test]
    fn build_head_branch_refspec_rejects_unsafe_refs() {
        assert_eq!(
            build_head_branch_refspec("task/2301-pr-gate-result-contract", 2318),
            Some("refs/heads/task/2301-pr-gate-result-contract:refs/adf/pr-2318".to_string())
        );
        assert_eq!(build_head_branch_refspec("", 2318), None);
        assert_eq!(
            build_head_branch_refspec("--upload-pack=/bin/sh", 2318),
            None
        );
        assert_eq!(build_head_branch_refspec("feature:x", 2318), None);
    }

    fn fixture_request(pr_number: u64, head_sha: &str) -> ReviewPrRequest {
        ReviewPrRequest {
            pr_number,
            project: "terraphim-ai".to_string(),
            head_sha: head_sha.to_string(),
            head_ref: format!("task/{pr_number}-gate"),
            author_login: "alice".to_string(),
            title: format!("Fix #{pr_number}: gate evidence"),
            diff_loc: 10,
        }
    }

    /// Issue #3293: evidence assembly must be fail-closed. A working
    /// directory with no usable git diff (no repo, no refs) is an
    /// authoritative-evidence failure, not a degraded pack. Callers must
    /// surface the error instead of silently substituting
    /// a degraded evidence pack.
    #[tokio::test]
    async fn build_pr_gate_evidence_pack_returns_err_when_no_diff_found() {
        let tmp = tempfile::TempDir::new().unwrap();
        // Empty directory with a bogus `.git` sentinel: git cannot resolve a
        // repository (an ancestor checkout would otherwise supply a diff).
        std::fs::write(tmp.path().join(".git"), "not a gitfile: sentinel")
            .expect("plant git sentinel");
        let err = build_pr_gate_evidence_pack(
            &fixture_request(641, "deadbeef1234"),
            Some(tmp.path()),
            PrGateEvidenceLimits::default(),
        )
        .await
        .unwrap_err();
        assert!(
            matches!(err, PrGateContextError::Git(_)),
            "expected PrGateContextError::Git, got {err:?}"
        );
    }

    /// Positive counterpart: with real refs bound in the working
    /// directory, the pack carries the authoritative changed files.
    #[tokio::test]
    async fn build_pr_gate_evidence_pack_returns_changed_files_when_refs_bound() {
        let tmp = tempfile::TempDir::new().unwrap();
        crate::tests_support::build_pr_gate_git_fixture(tmp.path());
        let pack = build_pr_gate_evidence_pack(
            &fixture_request(641, crate::tests_support::pr_gate_fixture_head_sha()),
            Some(tmp.path()),
            PrGateEvidenceLimits::default(),
        )
        .await
        .expect("authoritative evidence must assemble from bound refs");
        assert!(
            pack.changed_files.iter().any(|f| f.contains("change.txt")),
            "expected the fixture diff to list change.txt, got {:?}",
            pack.changed_files
        );
        assert!(
            !pack.diff_excerpt.contains("Diff unavailable"),
            "authoritative pack must not carry the degraded diff marker: {}",
            pack.diff_excerpt
        );
    }
}
