//! Gate-specific prompt rendering for native PR gate producers.

use crate::pr_gate_context::PrGateEvidencePack;
use crate::pr_gate_result::PrGateMeta;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrGateKind {
    Review,
    Validation,
    Verification,
}

impl PrGateKind {
    pub fn for_agent(agent_name: &str) -> Self {
        match agent_name {
            "pr-validator" => Self::Validation,
            "pr-verifier" => Self::Verification,
            _ => Self::Review,
        }
    }

    pub fn agent_name(self) -> &'static str {
        match self {
            Self::Review => "pr-reviewer",
            Self::Validation => "pr-validator",
            Self::Verification => "pr-verifier",
        }
    }

    pub fn context(self) -> &'static str {
        match self {
            Self::Review => "adf/pr-reviewer",
            Self::Validation => "adf/validation",
            Self::Verification => "adf/verification",
        }
    }

    pub fn title(self) -> &'static str {
        match self {
            Self::Review => "Structural PR review",
            Self::Validation => "Requirements validation",
            Self::Verification => "Design and test verification",
        }
    }

    fn instructions(self) -> &'static str {
        match self {
            Self::Review => {
                "Assess structural correctness, security, behavioural regressions, maintainability, and missing tests. Findings must be ordered by severity."
            }
            Self::Validation => {
                "Validate the change against the linked issue, stated acceptance criteria, user-visible behaviour, and release readiness."
            }
            Self::Verification => {
                "Verify implementation evidence against design intent, changed files, test evidence, and the canonical PR gate result contract."
            }
        }
    }
}

pub fn build_pr_gate_prompt(
    gate: PrGateKind,
    meta: &PrGateMeta,
    evidence: &PrGateEvidencePack,
) -> String {
    let changed_files = if evidence.changed_files.is_empty() {
        "- No changed file list available".to_string()
    } else {
        evidence
            .changed_files
            .iter()
            .map(|file| format!("- {file}"))
            .collect::<Vec<_>>()
            .join("\n")
    };

    let concepts = if evidence.matched_concepts.is_empty() {
        "- No matched Terraphim concepts".to_string()
    } else {
        evidence
            .matched_concepts
            .iter()
            .map(|concept| format!("- {concept}"))
            .collect::<Vec<_>>()
            .join("\n")
    };

    let linked_issue = evidence
        .linked_issue
        .as_ref()
        .map(|issue| format!("#{} - {}", issue.number, issue.title))
        .unwrap_or_else(|| "No linked issue evidence available".to_string());

    format!(
        r#"You are a bounded PR gate report producer.

Gate: {gate_title}
Agent: {agent}
Context: {context}

Rules:
- Use only the evidence in this prompt.
- Do not call tools.
- Do not post comments or statuses.
- Do not invent files or tests not present in evidence.
- Keep the human report under 1200 words.
- End with exactly one canonical adf:gate-result block.

Gate-specific instructions:
{instructions}

PR metadata:
- Project: {project}
- PR: #{pr_number}
- Title: {title}
- Author: {author}
- Head SHA: {head_sha}
- Diff LOC: {diff_loc}
- Linked issue: {linked_issue}

Changed files:
{changed_files}

Terraphim matched concepts:
{concepts}

Diff evidence:
```diff
{diff_excerpt}
```

Required output:
1. Human report with Summary, Findings, Evidence, and Verdict sections.
2. Exactly one final HTML comment block on its own lines.
3. In the JSON block, choose exactly one status value: "pass", "concerns", or "fail".
4. Set confidence to an integer from 1 to 5 and blocking_findings to the count of P0/P1 findings.
5. Replace the summary text with a specific one-line summary of your verdict.

Required final block shape:
<!-- adf:gate-result
{{
  "schema_version": 1,
  "agent": "{agent}",
  "context": "{context}",
  "pr_number": {pr_number},
  "head_sha": "{head_sha}",
  "status": "pass",
  "confidence": 4,
  "blocking_findings": 0,
  "summary": "replace with one line summary"
}}
-->
"#,
        gate_title = gate.title(),
        agent = &meta.agent_name,
        context = &meta.context,
        instructions = gate.instructions(),
        project = &evidence.project,
        pr_number = meta.pr_number,
        title = &evidence.title,
        author = &evidence.author,
        head_sha = &meta.head_sha,
        diff_loc = evidence.diff_loc,
        linked_issue = linked_issue,
        changed_files = changed_files,
        concepts = concepts,
        diff_excerpt = &evidence.diff_excerpt,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pr_gate_context::PrGateEvidencePack;
    use crate::pr_gate_result::PrGateMeta;

    fn evidence() -> PrGateEvidencePack {
        PrGateEvidencePack {
            pr_number: 2334,
            project: "terraphim-ai".to_string(),
            title: "Fix #2334: native gate producers".to_string(),
            author: "alice".to_string(),
            head_sha: "abcdef".to_string(),
            diff_loc: 12,
            changed_files: vec!["crates/terraphim_orchestrator/src/pr_gate_prompt.rs".to_string()],
            diff_excerpt: "diff --git a/a b/a".to_string(),
            linked_issue: None,
            matched_concepts: vec!["PrGateResult".to_string()],
            relevant_context: Vec::new(),
        }
    }

    fn meta(agent_name: &str, context: &str) -> PrGateMeta {
        PrGateMeta {
            pr_number: 2334,
            project: "terraphim-ai".to_string(),
            agent_name: agent_name.to_string(),
            context: context.to_string(),
            head_sha: "abcdef".to_string(),
        }
    }

    #[test]
    fn gate_kind_maps_agents_and_contexts() {
        assert_eq!(PrGateKind::for_agent("pr-reviewer"), PrGateKind::Review);
        assert_eq!(
            PrGateKind::for_agent("pr-validator"),
            PrGateKind::Validation
        );
        assert_eq!(
            PrGateKind::for_agent("pr-verifier"),
            PrGateKind::Verification
        );
        assert_eq!(PrGateKind::Validation.context(), "adf/validation");
    }

    #[test]
    fn prompt_contains_contract_for_each_gate() {
        for gate in [
            PrGateKind::Review,
            PrGateKind::Validation,
            PrGateKind::Verification,
        ] {
            let meta = meta(gate.agent_name(), gate.context());
            let prompt = build_pr_gate_prompt(gate, &meta, &evidence());
            assert!(prompt.contains("<!-- adf:gate-result"));
            assert!(prompt.contains("\"schema_version\": 1"));
            assert!(prompt.contains(gate.agent_name()));
            assert!(prompt.contains(gate.context()));
        }
    }

    #[test]
    fn prompt_disallows_tools_and_status_posts() {
        let meta = meta("custom-reviewer", "adf/custom-reviewer");
        let prompt = build_pr_gate_prompt(PrGateKind::Review, &meta, &evidence());
        assert!(prompt.contains("Do not call tools"));
        assert!(prompt.contains("Do not post comments or statuses"));
        assert!(!prompt.contains("gtr comment"));
    }

    #[test]
    fn prompt_uses_dispatch_metadata_for_canonical_block() {
        let meta = meta("renamed-gate", "adf/renamed-gate");
        let prompt = build_pr_gate_prompt(PrGateKind::Review, &meta, &evidence());
        assert!(prompt.contains("\"agent\": \"renamed-gate\""));
        assert!(prompt.contains("\"context\": \"adf/renamed-gate\""));
        assert!(!prompt.contains("pass|concerns|fail"));
    }
}
