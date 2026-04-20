//! Pure-function module for parsing structural PR review comments and
//! evaluating whether a PR meets the auto-merge criteria.
//!
//! This module intentionally has zero dependency on orchestrator runtime state
//! and performs no I/O. It parses the output of the `structural-pr-review`
//! Claude Code skill and applies a deterministic policy.
//!
//! See `cto-executive-system/plans/adf-rate-of-change-design.md` §Step 1
//! (Gitea issue `terraphim/adf-fleet#29`).
//!
//! # Expected review comment layout
//!
//! The skill emits HTML `<h3>` section headings (not markdown `###`) so that
//! Gitea/GitHub render them consistently. The relevant anchors for this
//! parser are:
//!
//! - `<h3>Summary</h3>` — summary paragraphs and optionally acceptance criteria
//!   checklist items (`- [x]` / `- [ ]`).
//! - `<h3>Confidence Score: N/5</h3>` — integer `N` in `1..=5`.
//! - `<h3>Inline Findings</h3>` — finding blocks starting with
//!   `**P0 `, `**P1 `, or `**P2 `.
//! - Footer `<sub>Last reviewed commit: <short hash> | Reviews (N)</sub>` —
//!   the `| Reviews (N)` suffix is optional for first-round reviews.

use thiserror::Error;

/// Parsed review verdict extracted from a single review comment body.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReviewVerdict {
    /// Confidence score, 1..=5.
    pub confidence: u8,
    /// Number of `**P0 ...**:` finding blocks.
    pub p0_count: u32,
    /// Number of `**P1 ...**:` finding blocks.
    pub p1_count: u32,
    /// Number of `**P2 ...**:` finding blocks.
    pub p2_count: u32,
    /// True iff every markdown checkbox (`- [x]` / `- [ ]`) is checked, or
    /// there are no checkboxes in the body.
    pub all_criteria_met: bool,
    /// Gitea comment identifier from which this verdict was parsed.
    pub comment_id: u64,
    /// Short hash of the commit the review was performed against, as
    /// extracted from the `Last reviewed commit:` footer.
    pub commit_short_hash: String,
}

/// Thresholds that must be satisfied for a PR to be auto-merged.
///
/// Defaults implement the `"no P0, no P1, green acceptance"` rate-of-change
/// policy with a 500 LoC diff cap and an agent-author requirement.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AutoMergeCriteria {
    pub min_confidence: u8,
    pub max_p0: u32,
    pub max_p1: u32,
    pub require_all_criteria: bool,
    pub max_diff_loc: u32,
    pub require_agent_author: bool,
}

impl Default for AutoMergeCriteria {
    fn default() -> Self {
        Self {
            min_confidence: 5,
            max_p0: 0,
            max_p1: 0,
            require_all_criteria: true,
            max_diff_loc: 500,
            require_agent_author: true,
        }
    }
}

/// Minimum PR metadata required by [`evaluate`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrMetadata {
    pub pr_number: u64,
    pub author_login: String,
    pub diff_loc: u32,
    pub head_sha: String,
    pub base_branch: String,
}

/// Auto-merge decision produced by [`evaluate`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AutoMergeDecision {
    /// PR clears every gate; safe to merge without human review.
    Merge,
    /// At least one gate failed. The attached reason is a human-readable
    /// summary suitable for posting back to the PR.
    HumanReviewNeeded(String),
}

/// Errors raised when a review comment body does not conform to the
/// `structural-pr-review` template.
#[derive(Error, Debug, PartialEq, Eq)]
pub enum VerdictParseError {
    #[error("missing confidence score header in review comment")]
    MissingConfidence,
    #[error("confidence score out of range: got {0}")]
    ConfidenceOutOfRange(u8),
    #[error("missing inline findings section")]
    MissingFindings,
    #[error("malformed footer (expected `Last reviewed commit: <short>`)")]
    MalformedFooter,
}

/// Parse a review comment body emitted by the `structural-pr-review` skill.
///
/// The parser is tolerant of extra whitespace and additional sections but
/// requires the three anchors listed at the module level. Returns a
/// [`VerdictParseError`] when the body does not conform.
pub fn parse_verdict(body: &str, comment_id: u64) -> Result<ReviewVerdict, VerdictParseError> {
    let confidence = parse_confidence(body)?;

    // The `Inline Findings` heading is required — without it we cannot be
    // confident the review is complete. We accept either the literal HTML
    // heading or the markdown fallback, which some reviewers emit.
    if !body.contains("<h3>Inline Findings</h3>") && !body.contains("### Inline Findings") {
        return Err(VerdictParseError::MissingFindings);
    }

    let (p0_count, p1_count, p2_count) = count_findings(body);
    let all_criteria_met = all_checkboxes_checked(body);
    let commit_short_hash = parse_commit_short_hash(body)?;

    Ok(ReviewVerdict {
        confidence,
        p0_count,
        p1_count,
        p2_count,
        all_criteria_met,
        comment_id,
        commit_short_hash,
    })
}

/// Evaluate whether a PR clears every auto-merge gate.
///
/// Gates (all must hold for [`AutoMergeDecision::Merge`]):
///
/// 1. `verdict.confidence >= criteria.min_confidence`
/// 2. `verdict.p0_count <= criteria.max_p0`
/// 3. `verdict.p1_count <= criteria.max_p1`
/// 4. `verdict.all_criteria_met || !criteria.require_all_criteria`
/// 5. `pr.diff_loc <= criteria.max_diff_loc`
/// 6. `!criteria.require_agent_author || author_is_agent(&pr.author_login)`
///
/// When a gate fails the returned [`AutoMergeDecision::HumanReviewNeeded`]
/// carries a short reason string suitable for posting back to the PR.
pub fn evaluate(
    verdict: &ReviewVerdict,
    pr: &PrMetadata,
    criteria: &AutoMergeCriteria,
) -> AutoMergeDecision {
    if verdict.confidence < criteria.min_confidence {
        return AutoMergeDecision::HumanReviewNeeded(format!(
            "confidence {}/5 below auto-merge threshold {}/5",
            verdict.confidence, criteria.min_confidence
        ));
    }
    if verdict.p0_count > criteria.max_p0 {
        return AutoMergeDecision::HumanReviewNeeded(format!(
            "{} P0 finding(s) present (max {})",
            verdict.p0_count, criteria.max_p0
        ));
    }
    if verdict.p1_count > criteria.max_p1 {
        return AutoMergeDecision::HumanReviewNeeded(format!(
            "{} P1 finding(s) present (max {})",
            verdict.p1_count, criteria.max_p1
        ));
    }
    if criteria.require_all_criteria && !verdict.all_criteria_met {
        return AutoMergeDecision::HumanReviewNeeded(
            "unchecked acceptance criteria in review body".to_string(),
        );
    }
    if pr.diff_loc > criteria.max_diff_loc {
        return AutoMergeDecision::HumanReviewNeeded(format!(
            "diff size {} LoC exceeds cap {} LoC",
            pr.diff_loc, criteria.max_diff_loc
        ));
    }
    if criteria.require_agent_author && !author_is_agent(&pr.author_login) {
        return AutoMergeDecision::HumanReviewNeeded(format!(
            "author `{}` is not a recognised agent; human-authored PRs require manual merge",
            pr.author_login
        ));
    }
    AutoMergeDecision::Merge
}

/// Return `true` when `login` belongs to an automation account authorised to
/// open auto-merge-eligible PRs.
///
/// Policy: the login exactly matches `claude-code` or `root`, or starts with
/// `adf-` (the ADF fleet agent prefix). Anything else — including human
/// maintainers, bots from other tenants, and Renovate/Dependabot — is
/// considered non-agent for auto-merge purposes.
pub fn author_is_agent(login: &str) -> bool {
    matches!(login, "claude-code" | "root") || login.starts_with("adf-")
}

fn parse_confidence(body: &str) -> Result<u8, VerdictParseError> {
    let needle = "Confidence Score:";
    let idx = body.find(needle).ok_or(VerdictParseError::MissingConfidence)?;
    let tail = &body[idx + needle.len()..];

    let digits: String = tail
        .chars()
        .skip_while(|c| c.is_whitespace())
        .take_while(|c| c.is_ascii_digit())
        .collect();

    if digits.is_empty() {
        return Err(VerdictParseError::MissingConfidence);
    }
    let score: u8 = digits
        .parse()
        .map_err(|_| VerdictParseError::MissingConfidence)?;
    if !(1..=5).contains(&score) {
        return Err(VerdictParseError::ConfidenceOutOfRange(score));
    }
    Ok(score)
}

fn count_findings(body: &str) -> (u32, u32, u32) {
    let mut p0 = 0;
    let mut p1 = 0;
    let mut p2 = 0;
    for line in body.lines() {
        let t = line.trim_start();
        if t.starts_with("**P0 ") {
            p0 += 1;
        } else if t.starts_with("**P1 ") {
            p1 += 1;
        } else if t.starts_with("**P2 ") {
            p2 += 1;
        }
    }
    (p0, p1, p2)
}

fn all_checkboxes_checked(body: &str) -> bool {
    // No checkboxes = no criteria to fail, treat as met. Any unchecked box
    // (`- [ ]` / `* [ ]`) flips this to false.
    for line in body.lines() {
        let t = line.trim_start();
        if t.starts_with("- [ ]") || t.starts_with("* [ ]") {
            return false;
        }
    }
    true
}

fn parse_commit_short_hash(body: &str) -> Result<String, VerdictParseError> {
    let needle = "Last reviewed commit:";
    let idx = body.find(needle).ok_or(VerdictParseError::MalformedFooter)?;
    let tail = &body[idx + needle.len()..];

    let hash: String = tail
        .chars()
        .skip_while(|c| c.is_whitespace())
        .take_while(|c| c.is_ascii_hexdigit())
        .collect();

    if hash.is_empty() {
        return Err(VerdictParseError::MalformedFooter);
    }
    Ok(hash)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn author_is_agent_policy() {
        assert!(author_is_agent("claude-code"));
        assert!(author_is_agent("root"));
        assert!(author_is_agent("adf-fleet"));
        assert!(author_is_agent("adf-reviewer"));
        assert!(!author_is_agent("alex"));
        assert!(!author_is_agent("dependabot[bot]"));
        assert!(!author_is_agent("renovate[bot]"));
    }

    #[test]
    fn default_criteria_match_policy() {
        let c = AutoMergeCriteria::default();
        assert_eq!(c.min_confidence, 5);
        assert_eq!(c.max_p0, 0);
        assert_eq!(c.max_p1, 0);
        assert!(c.require_all_criteria);
        assert_eq!(c.max_diff_loc, 500);
        assert!(c.require_agent_author);
    }

    #[test]
    fn confidence_out_of_range_is_rejected_inline() {
        let body = "<h3>Confidence Score: 7/5</h3>\n<h3>Inline Findings</h3>\n<sub>Last reviewed commit: abc123</sub>";
        let err = parse_verdict(body, 1).unwrap_err();
        assert_eq!(err, VerdictParseError::ConfidenceOutOfRange(7));
    }
}
