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

/// Extract the `github.repository` value (`owner/name`) from the task context.
pub fn repository(task: &Task) -> Option<String> {
    task.context
        .get("github")
        .and_then(|g| g.get("repository"))
        .and_then(|r| r.as_str())
        .map(|s| s.to_string())
        .or_else(|| {
            task.context
                .get("repository")
                .and_then(|r| r.as_str())
                .map(|s| s.to_string())
        })
}

/// Extract the commit SHA (`github.sha`) from the task context.
pub fn head_sha(task: &Task) -> Option<String> {
    task.context
        .get("github")
        .and_then(|g| g.get("sha"))
        .and_then(|s| s.as_str())
        .map(|s| s.to_string())
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
