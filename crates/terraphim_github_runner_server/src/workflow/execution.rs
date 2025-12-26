use anyhow::Result;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;
use terraphim_github_runner::Result as RunnerResult;
use terraphim_github_runner::{
    ExecutionStatus, InMemoryLearningCoordinator, LearningCoordinator, ParsedWorkflow,
    SessionManager, SessionManagerConfig, VmCommandExecutor, VmProvider, WorkflowContext,
    WorkflowExecutor, WorkflowExecutorConfig, WorkflowStep,
};
use tracing::{error, info, warn};

/// VM provider that delegates to VmCommandExecutor
struct FirecrackerVmProvider {
    _api_base_url: String,
    _auth_token: Option<String>,
}

#[async_trait::async_trait]
impl VmProvider for FirecrackerVmProvider {
    async fn allocate(&self, _vm_type: &str) -> RunnerResult<(String, Duration)> {
        // This is a placeholder - in real implementation, we'd call the Firecracker API
        // For now, return a mock VM ID
        Ok((
            format!("fc-vm-{}", uuid::Uuid::new_v4()),
            Duration::from_millis(100),
        ))
    }

    async fn release(&self, _vm_id: &str) -> RunnerResult<()> {
        Ok(())
    }
}

/// Parse a GitHub Actions workflow YAML into a ParsedWorkflow
/// This is a simplified parser that doesn't use LLM
pub fn parse_workflow_yaml_simple(workflow_path: &Path) -> Result<ParsedWorkflow> {
    let workflow_yaml = fs::read_to_string(workflow_path)?;

    // Simple YAML parsing to extract job steps
    let workflow_name = workflow_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown")
        .to_string();

    let mut steps = vec![];
    let mut setup_commands = vec![];
    let mut in_jobs_section = false;
    let mut current_job: Option<String> = None;
    let mut in_steps = false;
    let mut indent_level = 0;
    let mut step_name = String::new();

    for line in workflow_yaml.lines() {
        let trimmed = line.trim();

        // Skip empty lines and comments
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        // Track jobs section
        if trimmed.starts_with("jobs:") {
            in_jobs_section = true;
            continue;
        }

        // Exit jobs section if we hit a top-level key
        if in_jobs_section && !line.starts_with(' ') && !trimmed.starts_with('-') {
            in_jobs_section = false;
            current_job = None;
            in_steps = false;
        }

        // Track job names
        if in_jobs_section && trimmed.ends_with(':') && !line.contains("steps:") {
            current_job = Some(trimmed.trim_end_matches(':').to_string());
            in_steps = false;
            continue;
        }

        // Track steps section
        if current_job.is_some() && trimmed.starts_with("steps:") {
            in_steps = true;
            // Calculate indentation
            indent_level = line.len() - line.trim_start().len();
            continue;
        }

        // Parse steps
        if in_steps {
            let current_indent = line.len() - line.trim_start().len();

            // Check if we're still in the steps section
            if current_indent <= indent_level && !line.starts_with('-') {
                in_steps = false;
                step_name.clear();
                continue;
            }

            // Parse step with "name:"
            if trimmed.starts_with("- name:") || trimmed.starts_with("name:") {
                step_name = trimmed
                    .strip_prefix("- name:")
                    .or_else(|| trimmed.strip_prefix("name:"))
                    .map(|s| s.trim().to_string())
                    .unwrap_or_default();
                continue;
            }

            // Parse step with "run:"
            if trimmed.starts_with("- run:") || trimmed.starts_with("run:") {
                let command = trimmed
                    .strip_prefix("- run:")
                    .or_else(|| trimmed.strip_prefix("run:"))
                    .map(|s| s.trim().to_string())
                    .unwrap_or_default();

                if !command.is_empty() {
                    let name = if !step_name.is_empty() {
                        step_name.clone()
                    } else {
                        format!("Execute: {}", &command[..command.len().min(30)])
                    };

                    steps.push(WorkflowStep {
                        name,
                        command: if command.contains('\n') {
                            command.lines().collect::<Vec<_>>().join(" && ")
                        } else {
                            command
                        },
                        working_dir: "/workspace".to_string(),
                        continue_on_error: false,
                        timeout_seconds: 300,
                    });

                    step_name.clear();
                }
            } else if trimmed.starts_with("- uses:") || trimmed.starts_with("uses:") {
                // GitHub Actions - skip or translate to shell equivalent
                let action = trimmed
                    .strip_prefix("- uses:")
                    .or_else(|| trimmed.strip_prefix("uses:"))
                    .map(|s| s.trim().to_string())
                    .unwrap_or_default();

                warn!(
                    "GitHub Action '{}' will be skipped (not translated to shell command)",
                    action
                );
                step_name.clear();
            }
        }
    }

    // Add default setup commands for CI/CD
    if !steps.is_empty() {
        setup_commands.push("echo 'Starting workflow execution'".to_string());
        setup_commands.push("cd /workspace || mkdir -p /workspace".to_string());
    }

    Ok(ParsedWorkflow {
        name: workflow_name,
        trigger: "webhook".to_string(),
        environment: std::collections::HashMap::new(),
        setup_commands,
        steps,
        cleanup_commands: vec!["echo 'Workflow execution complete'".to_string()],
        cache_paths: vec![],
    })
}

/// Execute a single workflow in a VM
pub async fn execute_workflow_in_vm(
    workflow_path: &Path,
    gh_event: &terraphim_github_runner::GitHubEvent,
    firecracker_api_url: &str,
    firecracker_auth_token: Option<&str>,
) -> Result<String> {
    info!("Executing workflow: {:?}", workflow_path.file_name());

    // Parse workflow
    let workflow = parse_workflow_yaml_simple(workflow_path)?;

    // Create VM provider
    let vm_provider: Arc<dyn VmProvider> = Arc::new(FirecrackerVmProvider {
        _api_base_url: firecracker_api_url.to_string(),
        _auth_token: firecracker_auth_token.map(|s| s.to_string()),
    });

    // Create VM command executor
    let command_executor: Arc<VmCommandExecutor> =
        Arc::new(if let Some(token) = firecracker_auth_token {
            VmCommandExecutor::with_auth(firecracker_api_url, token.to_string())
        } else {
            VmCommandExecutor::new(firecracker_api_url)
        });

    // Create learning coordinator
    let _learning_coordinator: Arc<dyn LearningCoordinator> =
        Arc::new(InMemoryLearningCoordinator::new("github-runner"));

    // Create session manager with VM provider
    let session_config = SessionManagerConfig::default();
    let session_manager = Arc::new(SessionManager::with_provider(
        vm_provider.clone(),
        session_config,
    ));

    // Create workflow executor
    let config = WorkflowExecutorConfig::default();
    let workflow_executor =
        WorkflowExecutor::with_executor(command_executor.clone(), session_manager, config);

    // Create workflow context with all required fields
    let context = WorkflowContext {
        session_id: terraphim_github_runner::SessionId(uuid::Uuid::new_v4()),
        event: gh_event.clone(),
        vm_id: None,
        started_at: chrono::Utc::now(),
        env_vars: std::collections::HashMap::new(),
        working_dir: "/workspace".to_string(),
        snapshots: vec![],
        execution_history: vec![],
    };

    // Execute workflow
    info!("Starting workflow execution: {}", workflow.name);
    let result = workflow_executor
        .execute_workflow(&workflow, &context)
        .await;

    match result {
        Ok(workflow_result) => {
            let success_count = workflow_result
                .steps
                .iter()
                .filter(|s| matches!(s.status, ExecutionStatus::Success))
                .count();

            let output = format!(
                "✅ Workflow '{}' completed successfully\n\
                 Steps executed: {}/{}\n\
                 Duration: {}s\n\
                 Snapshots created: {}",
                workflow.name,
                success_count,
                workflow_result.steps.len(),
                workflow_result.total_duration_ms / 1000,
                workflow_result
                    .final_snapshot
                    .as_ref()
                    .map(|_| 1)
                    .unwrap_or(0)
            );

            // Log individual step results
            for step in &workflow_result.steps {
                if matches!(step.status, ExecutionStatus::Success) {
                    info!("✅ Step '{}': {}", step.name, step.stdout.trim());
                } else {
                    error!(
                        "❌ Step '{}': {}",
                        step.name,
                        if !step.stderr.is_empty() {
                            &step.stderr
                        } else {
                            &step.stdout
                        }
                    );
                }
            }

            Ok(output)
        }
        Err(e) => {
            error!("Workflow execution failed: {}", e);
            Err(e.into())
        }
    }
}

/// Execute multiple workflows for a GitHub event
pub async fn execute_workflows_in_vms(
    workflow_paths: Vec<PathBuf>,
    gh_event: &terraphim_github_runner::GitHubEvent,
    firecracker_api_url: &str,
    firecracker_auth_token: Option<&str>,
) -> Result<String> {
    if workflow_paths.is_empty() {
        return Ok("No workflows to execute".to_string());
    }

    let mut results = vec![];

    for workflow_path in &workflow_paths {
        match execute_workflow_in_vm(
            workflow_path,
            gh_event,
            firecracker_api_url,
            firecracker_auth_token,
        )
        .await
        {
            Ok(output) => {
                results.push(format!(
                    "## {}\n{}",
                    workflow_path
                        .file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("unknown"),
                    output
                ));
            }
            Err(e) => {
                let error_msg = format!(
                    "## ❌ {}\n\nExecution failed: {}",
                    workflow_path
                        .file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("unknown"),
                    e
                );
                results.push(error_msg);
            }
        }
    }

    Ok(results.join("\n\n"))
}
