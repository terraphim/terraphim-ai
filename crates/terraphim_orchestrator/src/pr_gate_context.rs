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
                    diff_excerpt: build_bounded_diff_excerpt(&diff, limits.max_diff_lines),
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

    Ok(GitEvidence {
        changed_files: Vec::new(),
        diff_excerpt: format!(
            "Diff unavailable: {}",
            last_error.unwrap_or_else(|| "no usable git diff range".to_string())
        ),
    })
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

/// Split a unified diff into per-file chunks keyed by the post-image path.
fn split_diff_into_file_chunks(diff: &str) -> Vec<(String, String)> {
    let mut chunks = Vec::new();
    let mut current_path: Option<String> = None;
    let mut current_lines: Vec<&str> = Vec::new();

    for line in diff.lines() {
        if let Some(rest) = line.strip_prefix("diff --git a/") {
            if let Some((_, right)) = rest.split_once(" b/") {
                if let Some(path) = current_path.take() {
                    chunks.push((path, current_lines.join("\n")));
                    current_lines.clear();
                }
                current_path = Some(right.to_string());
            }
        }
        if current_path.is_some() {
            current_lines.push(line);
        }
    }

    if let Some(path) = current_path {
        chunks.push((path, current_lines.join("\n")));
    }

    chunks
}

/// Higher-priority paths are included before large deletion hunks so validators
/// still see security-sensitive removals when the excerpt budget is tight.
fn file_evidence_priority(path: &str) -> u8 {
    if path.starts_with('~')
        || path.contains("private")
        || path.contains(".env")
        || path.contains("secret")
    {
        0
    } else if path == ".gitignore" {
        1
    } else {
        2
    }
}

fn summarize_omitted_file_chunk(path: &str, chunk: &str) -> String {
    let deleted = chunk.contains("deleted file mode");
    let added = chunk.contains("new file mode");
    let mode = if deleted {
        "deleted"
    } else if added {
        "added"
    } else {
        "modified"
    };
    format!(
        "### {path}\n({mode}, {} diff lines omitted from excerpt)\n",
        chunk.lines().count()
    )
}

/// Build a bounded diff excerpt that always retains high-priority file hunks
/// (e.g. literal `~/…` paths) even when large cleanup deletions dominate LoC.
pub fn build_bounded_diff_excerpt(diff: &str, max_lines: usize) -> String {
    let chunks = split_diff_into_file_chunks(diff);
    if chunks.is_empty() {
        return limit_lines(diff, max_lines);
    }

    let mut ordered: Vec<(u8, usize, String, String)> = chunks
        .into_iter()
        .map(|(path, body)| {
            let priority = file_evidence_priority(&path);
            let line_count = body.lines().count().max(1);
            (priority, line_count, path, body)
        })
        .collect();
    ordered.sort_by(|a, b| (a.0, a.1).cmp(&(b.0, b.1)));

    let mut parts = Vec::new();
    let mut used_lines = 0usize;
    let mut included = Vec::new();

    for (priority, _, path, body) in &ordered {
        if used_lines >= max_lines {
            break;
        }
        let remaining = max_lines.saturating_sub(used_lines);
        let chunk = if *priority == 0 || body.lines().count() <= remaining {
            body.clone()
        } else {
            limit_lines(body, remaining)
        };
        used_lines += chunk.lines().count();
        included.push(path.clone());
        parts.push(chunk);
    }

    let omitted: Vec<_> = ordered
        .iter()
        .filter(|(_, _, path, _)| !included.contains(path))
        .collect();
    if !omitted.is_empty() {
        parts.push(format!(
            "\n## Files omitted from truncated excerpt ({})\n",
            omitted.len()
        ));
        for (_, _, path, body) in omitted {
            parts.push(summarize_omitted_file_chunk(path, body));
        }
    }

    parts.join("\n")
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
    fn build_bounded_diff_excerpt_prioritizes_tilde_paths() {
        let mut large = String::from("diff --git a/session-ses.md b/session-ses.md\n");
        for i in 0..2000 {
            large.push_str(&format!("-{i}\n"));
        }
        large.push_str(
            "diff --git a/~/projects/personal/private_agents_settings/bin/tool b/~/projects/personal/private_agents_settings/bin/tool\n",
        );
        large.push_str("deleted file mode 100755\n-old content\n");

        let excerpt = build_bounded_diff_excerpt(&large, 50);
        assert!(
            excerpt.contains("~/projects/personal/private_agents_settings"),
            "security-sensitive tilde path must appear in excerpt: {excerpt}"
        );
    }

    #[test]
    fn build_diff_ranges_prefers_fetched_base_and_pr_refs() {
        let ranges = build_diff_ranges("refs/adf/pr-2318", "abc123");
        assert_eq!(ranges[0], "refs/adf/base-main...refs/adf/pr-2318");
        assert!(ranges.contains(&"origin/main...refs/adf/pr-2318".to_string()));
        assert!(ranges.contains(&"refs/adf/base-main..abc123".to_string()));
    }

    #[test]
    fn fallback_pack_uses_automata_concept_matching() {
        let req = ReviewPrRequest {
            pr_number: 1,
            project: "terraphim-ai".to_string(),
            head_sha: "abc".to_string(),
            head_ref: "task/2334-native-gates".to_string(),
            author_login: "alice".to_string(),
            title: "Fix #2334: PrGateResult validation timeout".to_string(),
            diff_loc: 10,
        };
        let pack = fallback_evidence_pack(&req, "test");
        assert!(pack.matched_concepts.contains(&"validation".to_string()));
        assert!(pack.matched_concepts.contains(&"timeout".to_string()));
        assert_eq!(pack.linked_issue.as_ref().map(|i| i.number), Some(2334));
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
}
