//! Decode a Gitea `WorkflowPayload` into a [`ParsedWorkflow`], and extract the
//! repository name from the task context (for the coexistence guard).

use crate::types::Task;
use crate::{Result, RunnerError};
use base64::Engine;
use terraphim_github_runner::{ParsedWorkflow, parse_workflow_payload};

/// Decode `task.workflow_payload` (base64 SingleWorkflow YAML, maybe gzip) into a
/// [`ParsedWorkflow`] using the shared parser.
pub fn compile_task(task: &Task) -> Result<ParsedWorkflow> {
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(task.workflow_payload.trim())
        .map_err(|e| RunnerError::Compile(format!("workflow_payload is not valid base64: {e}")))?;
    parse_workflow_payload(&bytes).map_err(|e| RunnerError::Compile(e.to_string()))
}

/// Read a string field from the task context, tolerating both shapes Gitea
/// uses: the whole context Struct mirrors the GitHub `github` context, so the
/// value is normally top-level (`context.<key>`), but some fixtures/older
/// payloads nest it under a `github` object (`context.github.<key>`). We try
/// the nested form first, then the top-level form, for each candidate key.
fn context_str(task: &Task, keys: &[&str]) -> Option<String> {
    let ctx = &task.context;
    for key in keys {
        if let Some(s) = ctx
            .get("github")
            .and_then(|g| g.get(key))
            .and_then(|v| v.as_str())
        {
            return Some(s.to_string());
        }
        if let Some(s) = ctx.get(key).and_then(|v| v.as_str()) {
            return Some(s.to_string());
        }
    }
    None
}

/// Extract the repository (`owner/name`) from the task context. Live Gitea sends
/// it as a top-level string; an object form (`{ "full_name": "owner/name" }`)
/// is also tolerated.
pub fn repository(task: &Task) -> Option<String> {
    context_str(task, &["repository"]).or_else(|| {
        // Object form: `repository.full_name`.
        let ctx = &task.context;
        ctx.get("github")
            .and_then(|g| g.get("repository"))
            .or_else(|| ctx.get("repository"))
            .and_then(|r| r.get("full_name"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
    })
}

/// Extract the commit SHA from the task context. Live Gitea sends `sha` at the
/// top level; `head_sha`/`headSha` are tolerated as alternates.
pub fn head_sha(task: &Task) -> Option<String> {
    context_str(task, &["sha", "head_sha", "headSha"])
}

/// Extract the per-job repository token used to authenticate the checkout.
///
/// Gitea generates an automatic token for each workflow run, exposed as
/// `${{ github.token }}` (context `token`) and `${{ secrets.GITHUB_TOKEN }}`.
/// This token -- not the runner's registration token -- grants git read access
/// to the repository for the duration of the job. Prefer the context value,
/// then the secret.
pub fn job_token(task: &Task) -> Option<String> {
    context_str(task, &["token"]).or_else(|| task.secrets.get("GITHUB_TOKEN").cloned())
}

#[cfg(test)]
mod tests {
    use super::*;
    use base64::Engine;

    #[test]
    fn decodes_payload_and_context() {
        let yaml = "jobs:\n  j:\n    steps:\n      - run: echo hi\n";
        let task = Task {
            id: 1,
            workflow_payload: base64::engine::general_purpose::STANDARD.encode(yaml),
            context: serde_json::json!({"github": {"repository": "terraphim/terraphim-core", "sha": "abc123"}}),
            ..Task::default()
        };
        let wf = compile_task(&task).unwrap();
        assert_eq!(wf.steps.len(), 1);
        assert_eq!(
            repository(&task).as_deref(),
            Some("terraphim/terraphim-core")
        );
        assert_eq!(head_sha(&task).as_deref(), Some("abc123"));
    }
}
