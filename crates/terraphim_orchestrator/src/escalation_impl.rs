//! Escalation and finding-reporting capability for `AgentOrchestrator`:
//! filing Gitea issues for compound-review findings, sanitising text for
//! issue titles/bodies, and escalating unknown error signatures. Split from
//! lib.rs as part of the Gitea #1910 god-file decomposition; behaviour
//! unchanged.
#![allow(clippy::too_many_lines)]

use terraphim_types::*;
use tracing::{info, warn};

use crate::{error_signatures, AgentOrchestrator, CompoundReviewResult, OutputPoster};

impl AgentOrchestrator {
    /// Sanitise finding text for use in issue title.
    /// Strips JSON syntax characters that break title parsing.
    pub(crate) fn sanitise_for_title(input: &str) -> String {
        let mut result = String::with_capacity(input.len());
        for ch in input.chars() {
            match ch {
                '{' | '}' | '[' | ']' | '"' => result.push(' '),
                '\n' | '\r' => result.push(' '),
                _ => result.push(ch),
            }
        }
        let trimmed = result.split_whitespace().collect::<Vec<_>>().join(" ");
        if trimmed.len() > 80 {
            trimmed[..77].to_string() + "..."
        } else {
            trimmed
        }
    }

    /// Sanitise finding text for use in issue body (markdown).
    /// Escapes markdown special characters that could break rendering.
    pub(crate) fn sanitise_for_body(input: &str) -> String {
        let mut result = String::with_capacity(input.len());
        for ch in input.chars() {
            match ch {
                '`' => result.push_str("``"),
                '*' | '_' | '[' | ']' => result.push('\\'),
                _ => result.push(ch),
            }
        }
        result
    }

    /// File a Gitea issue for a compound review finding.
    ///
    /// Deduplicates by searching for existing open issues with similar titles
    /// before creating a new one.
    pub(crate) async fn file_finding_issue(
        &self,
        poster: &OutputPoster,
        result: &CompoundReviewResult,
        finding: &ReviewFinding,
    ) -> Result<(), String> {
        use terraphim_types::FindingSeverity;

        let sev_str = match finding.severity {
            FindingSeverity::Critical => "CRITICAL",
            FindingSeverity::High => "HIGH",
            FindingSeverity::Medium => "MEDIUM",
            FindingSeverity::Low => "LOW",
            FindingSeverity::Info => "INFO",
        };

        // Build a short keyword from the finding for dedup search
        let dedup_keyword = if finding.finding.len() > 40 {
            &finding.finding[..40]
        } else {
            &finding.finding
        };

        // Dedup: check if an open issue with similar title already exists
        match poster.tracker().search_issues_by_title(dedup_keyword).await {
            Ok(existing) if !existing.is_empty() => {
                info!(
                    severity = %sev_str,
                    existing_issues = existing.len(),
                    keyword = %dedup_keyword,
                    "skipping finding issue (already filed)"
                );
                return Ok(());
            }
            Err(e) => {
                // Dedup search failed — proceed with filing (fail-open)
                warn!(error = %e, "dedup search failed, proceeding to file issue");
            }
            _ => {}
        }

        let title = format!(
            "[Compound Review] {}: {}",
            sev_str,
            Self::sanitise_for_title(&finding.finding)
        );

        let mut body = "## Automated Finding from Compound Review\n\n".to_string();
        body.push_str(&format!("- **Severity**: {}\n", sev_str));
        if !finding.file.is_empty() {
            body.push_str(&format!(
                "- **File**: {}{}\n",
                finding.file,
                if finding.line > 0 {
                    format!(":{}", finding.line)
                } else {
                    String::new()
                }
            ));
        }
        body.push_str(&format!(
            "- **Confidence**: {:.0}%\n",
            finding.confidence * 100.0
        ));
        body.push_str(&format!("- **Review ID**: {}\n\n", result.correlation_id));
        body.push_str(&format!(
            "### Finding\n\n{}\n\n",
            Self::sanitise_for_body(&finding.finding)
        ));
        if let Some(ref suggestion) = finding.suggestion {
            if !suggestion.is_empty() {
                body.push_str(&format!(
                    "### Suggested Fix\n\n{}\n",
                    Self::sanitise_for_body(suggestion)
                ));
            }
        }

        // Skip labels for now - Gitea API has issues with label format
        // TODO: Fix labels format for Gitea API
        match poster.tracker().create_issue(&title, &body, &[]).await {
            Ok(issue) => {
                info!(
                    issue_number = issue.number,
                    severity = %sev_str,
                    title = %title,
                    "filed finding issue"
                );
                // Trigger implementation-swarm via mention comment so
                // mention polling dispatches the agent automatically.
                let trigger = format!(
                    "@adf:implementation-swarm please implement this finding for issue #{}",
                    issue.number
                );
                if let Err(e) = poster.tracker().post_comment(issue.number, &trigger).await {
                    warn!(
                        issue_number = issue.number,
                        error = %e,
                        "failed to post implementation trigger comment"
                    );
                }
                Ok(())
            }
            Err(e) => Err(format!("failed to create issue '{}': {}", title, e)),
        }
    }

    /// Open a `[ADF] unknown error signature on <provider>/<model>` Gitea
    /// issue so fleet-meta can classify the pattern. Deduped by
    /// [`error_signatures::unknown_dedupe_key`] within the process lifetime
    /// so retries of the same stderr shape don't spam the tracker.
    ///
    /// The target tracker is the orchestrator's default [`GiteaTracker`]
    /// from [`OutputPoster::tracker`], which points at the fleet-meta repo
    /// configured in `orchestrator.toml`. If no `OutputPoster` is wired
    /// (tests, legacy configs), this is a no-op.
    pub(crate) async fn escalate_unknown_error(
        &self,
        provider: &str,
        model: Option<&str>,
        stderr_lines: &[String],
    ) {
        let joined = stderr_lines.join("\n");
        let dedupe_key = error_signatures::unknown_dedupe_key(provider, &joined);
        {
            let mut set = match self.unknown_error_dedupe.lock() {
                Ok(g) => g,
                Err(_) => {
                    warn!(
                        provider = %provider,
                        "unknown_error_dedupe lock poisoned; skipping escalation"
                    );
                    return;
                }
            };
            if !set.insert(dedupe_key.clone()) {
                // Already escalated this shape in this process. Skip quietly.
                return;
            }
        }

        let Some(poster) = self.output_poster.as_ref() else {
            info!(
                provider = %provider,
                dedupe_key = %dedupe_key,
                "no output_poster configured; unknown stderr logged only"
            );
            return;
        };

        // Cap stderr in the body so a runaway CLI never posts megabytes.
        const MAX_STDERR_CHARS: usize = 4000;
        let truncated: String = if joined.len() > MAX_STDERR_CHARS {
            format!(
                "{}\n...[truncated, original {} chars]",
                joined.chars().take(MAX_STDERR_CHARS).collect::<String>(),
                joined.len()
            )
        } else {
            joined
        };
        let model_slug = model.unwrap_or("<unknown-model>");
        let title = format!(
            "[ADF] unknown error signature on {}/{}",
            provider, model_slug
        );
        let body = format!(
            "A spawned agent produced stderr that matched neither the \
             throttle nor the flake regex lists for provider `{}` \
             (model `{}`). Please review and extend the provider's \
             `error_signatures` config so future occurrences classify \
             correctly.\n\n\
             **Dedupe key:** `{}`\n\n\
             ## Captured stderr\n\n```\n{}\n```\n",
            provider, model_slug, dedupe_key, truncated
        );
        let labels = ["adf", "error-signature", "triage"];
        let tracker = poster.tracker();
        if let Err(e) = tracker.create_issue(&title, &body, &labels).await {
            warn!(
                provider = %provider,
                model = %model_slug,
                error = %e,
                "failed to escalate unknown-error signature to Gitea"
            );
        } else {
            info!(
                provider = %provider,
                model = %model_slug,
                dedupe_key = %dedupe_key,
                "escalated unknown error signature to fleet-meta"
            );
        }
    }
}
