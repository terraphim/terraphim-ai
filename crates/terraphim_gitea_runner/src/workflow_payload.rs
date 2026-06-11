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

/// Workflow trigger name for commit-status context (e.g. `push`, `workflow_dispatch`).
pub fn event_name(task: &Task) -> String {
    context_str(task, &["event_name", "event"]).unwrap_or_else(|| "push".to_string())
}

/// First job id under `jobs:` in the SingleWorkflow YAML (e.g. `build`).
pub fn first_job_id(task: &Task) -> Option<String> {
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(task.workflow_payload.trim())
        .ok()?;
    let yaml = if bytes.len() >= 2 && bytes[0] == 0x1f && bytes[1] == 0x8b {
        use std::io::Read;
        let mut decoder = flate2::read::GzDecoder::new(bytes.as_slice());
        let mut s = String::new();
        decoder.read_to_string(&mut s).ok()?;
        s
    } else {
        String::from_utf8(bytes).ok()?
    };
    parse_first_job_id(&yaml)
}

fn parse_first_job_id(yaml: &str) -> Option<String> {
    let mut in_jobs = false;
    for line in yaml.lines() {
        if line.trim() == "jobs:" {
            in_jobs = true;
            continue;
        }
        if !in_jobs {
            continue;
        }
        // Top-level job key: exactly two leading spaces, no third.
        if line.starts_with("  ") && !line.starts_with("   ") {
            let key = line.trim().trim_end_matches(':').trim();
            if !key.is_empty()
                && key
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
            {
                return Some(key.to_string());
            }
        }
    }
    None
}

/// Gitea branch-protection context for native-ci (e.g. `native-ci / build (push)`).
pub fn commit_status_context(task: &Task, workflow: &ParsedWorkflow) -> String {
    let wf_name = if workflow.name.is_empty() {
        "native-ci".to_string()
    } else {
        workflow.name.clone()
    };
    let job = first_job_id(task).unwrap_or_else(|| "build".to_string());
    let event = event_name(task);
    format!("{wf_name} / {job} ({event})")
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

    #[test]
    fn commit_status_context_matches_gitea_format() {
        let yaml = "name: native-ci\njobs:\n  build:\n    runs-on: terraphim-native\n    steps:\n      - run: cargo fmt --all -- --check\n";
        let task = Task {
            id: 2,
            workflow_payload: base64::engine::general_purpose::STANDARD.encode(yaml),
            context: serde_json::json!({"event_name": "workflow_dispatch"}),
            ..Task::default()
        };
        let wf = compile_task(&task).unwrap();
        assert_eq!(
            commit_status_context(&task, &wf),
            "native-ci / build (workflow_dispatch)"
        );
    }
}
