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
    #[error("git command failed: {0}")]
    Git(String),
}

pub async fn build_pr_gate_evidence_pack(
    req: &ReviewPrRequest,
    working_dir: Option<&Path>,
    limits: PrGateEvidenceLimits,
) -> Result<PrGateEvidencePack, PrGateContextError> {
    let git_evidence = match working_dir {
        Some(path) => collect_git_evidence(path, req, &limits).await?,
        None => GitEvidence::unavailable("no working directory provided"),
    };

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

impl GitEvidence {
    fn unavailable(reason: &str) -> Self {
        Self {
            changed_files: Vec::new(),
            diff_excerpt: format!("Diff unavailable: {reason}"),
        }
    }
}

async fn collect_git_evidence(
    working_dir: &Path,
    req: &ReviewPrRequest,
    limits: &PrGateEvidenceLimits,
) -> Result<GitEvidence, PrGateContextError> {
    let pr_ref = format!("pull/{}/head:refs/adf/pr-{}", req.pr_number, req.pr_number);
    let fetch_result = Command::new("git")
        .current_dir(working_dir)
        .args(["fetch", "gitea", &pr_ref])
        .output()
        .await;

    if let Ok(output) = fetch_result {
        if !output.status.success() {
            tracing::debug!(
                stderr = %String::from_utf8_lossy(&output.stderr),
                pr_number = req.pr_number,
                "native PR gate fetch failed; trying existing refs"
            );
        }
    }

    let pr_branch = format!("refs/adf/pr-{}", req.pr_number);
    let diff_range = format!("main...{pr_branch}");
    let diff = git_output(working_dir, &["diff", "--no-ext-diff", &diff_range]).await;
    let diff = match diff {
        Ok(diff) if !diff.trim().is_empty() => diff,
        _ => {
            let head_range = format!("main...{}", req.head_sha);
            git_output(working_dir, &["diff", "--no-ext-diff", &head_range])
                .await
                .unwrap_or_else(|e| format!("Diff unavailable: {e}"))
        }
    };

    Ok(GitEvidence {
        changed_files: extract_changed_files(&diff),
        diff_excerpt: limit_lines(&diff, limits.max_diff_lines),
    })
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

pub fn fallback_evidence_pack(req: &ReviewPrRequest, reason: &str) -> PrGateEvidencePack {
    PrGateEvidencePack {
        pr_number: req.pr_number,
        project: req.project.clone(),
        title: req.title.clone(),
        author: req.author_login.clone(),
        head_sha: req.head_sha.clone(),
        diff_loc: req.diff_loc,
        changed_files: Vec::new(),
        diff_excerpt: format!("Diff unavailable: {reason}"),
        linked_issue: extract_issue_number(&req.title).map(|number| LinkedIssueEvidence {
            number,
            title: format!("Issue #{number}"),
            body_excerpt: String::new(),
            acceptance_criteria: Vec::new(),
        }),
        matched_concepts: extract_builtin_concepts(&req.title),
        relevant_context: Vec::new(),
    }
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
    fn fallback_pack_uses_automata_concept_matching() {
        let req = ReviewPrRequest {
            pr_number: 1,
            project: "terraphim-ai".to_string(),
            head_sha: "abc".to_string(),
            author_login: "alice".to_string(),
            title: "Fix #2334: PrGateResult validation timeout".to_string(),
            diff_loc: 10,
        };
        let pack = fallback_evidence_pack(&req, "test");
        assert!(pack.matched_concepts.contains(&"validation".to_string()));
        assert!(pack.matched_concepts.contains(&"timeout".to_string()));
        assert_eq!(pack.linked_issue.as_ref().map(|i| i.number), Some(2334));
    }
}
