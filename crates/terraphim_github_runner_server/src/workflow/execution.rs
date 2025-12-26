use anyhow::Result;
use reqwest::Client;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};
use terraphim_github_runner::Result as RunnerResult;
use terraphim_github_runner::{
    ExecutionStatus, InMemoryLearningCoordinator, LearningCoordinator, ParsedWorkflow,
    SessionManager, SessionManagerConfig, VmCommandExecutor, VmProvider, WorkflowContext,
    WorkflowExecutor, WorkflowExecutorConfig, WorkflowParser, WorkflowStep,
};
use tracing::{error, info, warn};

/// VM provider that allocates real Firecracker VMs via fcctl-web API
struct FirecrackerVmProvider {
    api_base_url: String,
    auth_token: Option<String>,
    client: Arc<Client>,
}

impl FirecrackerVmProvider {
    pub fn new(api_base_url: String, auth_token: Option<String>, client: Arc<Client>) -> Self {
        Self {
            api_base_url,
            auth_token,
            client,
        }
    }
}

#[async_trait::async_trait]
impl VmProvider for FirecrackerVmProvider {
    async fn allocate(&self, vm_type: &str) -> RunnerResult<(String, Duration)> {
        let start = Instant::now();
        let url = format!("{}/api/vms", self.api_base_url);

        let payload = serde_json::json!({
            "vm_type": vm_type,
            "vm_name": format!("github-runner-{}", uuid::Uuid::new_v4())
        });

        let mut request = self.client.post(&url).json(&payload);

        if let Some(ref token) = self.auth_token {
            request = request.bearer_auth(token);
        }

        let response = request.send().await.map_err(|e| {
            terraphim_github_runner::GitHubRunnerError::VmAllocation(format!(
                "API request failed: {}",
                e
            ))
        })?;

        if !response.status().is_success() {
            return Err(terraphim_github_runner::GitHubRunnerError::VmAllocation(
                format!("Allocation failed with status: {}", response.status()),
            ));
        }

        let result: serde_json::Value = response.json().await.map_err(|e| {
            terraphim_github_runner::GitHubRunnerError::VmAllocation(format!(
                "Failed to parse response: {}",
                e
            ))
        })?;

        let vm_id = result["id"]
            .as_str()
            .ok_or_else(|| {
                terraphim_github_runner::GitHubRunnerError::VmAllocation(
                    "No VM ID in response".to_string(),
                )
            })?
            .to_string();

        let duration = start.elapsed();

        info!("Allocated VM {} in {:?}", vm_id, duration);

        Ok((vm_id, duration))
    }

    async fn release(&self, vm_id: &str) -> RunnerResult<()> {
        let url = format!("{}/api/vms/{}", self.api_base_url, vm_id);

        let mut request = self.client.delete(&url);

        if let Some(ref token) = self.auth_token {
            request = request.bearer_auth(token);
        }

        let response = request.send().await.map_err(|e| {
            terraphim_github_runner::GitHubRunnerError::VmAllocation(format!(
                "Release API request failed: {}",
                e
            ))
        })?;

        if !response.status().is_success() {
            return Err(terraphim_github_runner::GitHubRunnerError::VmAllocation(
                format!("Release failed with status: {}", response.status()),
            ));
        }

        info!("Released VM {}", vm_id);

        Ok(())
    }
}

/// Parse a GitHub Actions workflow YAML into a ParsedWorkflow
/// Uses LLM-based parsing if LLM client is available, otherwise falls back to simple parser
pub async fn parse_workflow_yaml_with_llm(
    workflow_path: &Path,
    llm_parser: Option<&WorkflowParser>,
) -> Result<ParsedWorkflow> {
    let workflow_yaml = fs::read_to_string(workflow_path)?;
    let workflow_name = workflow_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown")
        .to_string();

    // Use LLM parser if available and enabled
    if let Some(parser) = llm_parser {
        if env::var("USE_LLM_PARSER").unwrap_or_default() == "true" {
            info!("🤖 Using LLM-based workflow parsing for: {}", workflow_name);
            match parser.parse_workflow_yaml(&workflow_yaml).await {
                Ok(workflow) => {
                    info!("✅ LLM successfully parsed workflow: {}", workflow_name);
                    info!("   - {} steps extracted", workflow.steps.len());
                    info!("   - {} setup commands", workflow.setup_commands.len());
                    for (i, step) in workflow.steps.iter().enumerate() {
                        info!(
                            "   - Step {}: {} (command: {})",
                            i + 1,
                            step.name,
                            step.command.chars().take(50).collect::<String>()
                        );
                    }
                    return Ok(workflow);
                }
                Err(e) => {
                    warn!(
                        "⚠️  LLM parsing failed, falling back to simple parser: {}",
                        e
                    );
                    // Fall through to simple parser
                }
            }
        }
    }

    // Fallback to simple YAML parser
    info!("📋 Using simple YAML parser for: {}", workflow_name);
    parse_workflow_yaml_simple(workflow_path)
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
    llm_parser: Option<&WorkflowParser>,
) -> Result<String> {
    info!("=========================================================");
    info!("🚀 EXECUTING WORKFLOW: {:?}", workflow_path.file_name());
    info!("=========================================================");

    // Parse workflow (with LLM if available)
    let workflow = parse_workflow_yaml_with_llm(workflow_path, llm_parser).await?;

    // Create shared HTTP client with connection pool limits
    info!("🌐 Creating shared HTTP client with connection pooling");

    // Configure timeouts via environment variables
    let client_timeout_secs = std::env::var("HTTP_CLIENT_TIMEOUT_SECS")
        .ok()
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(30); // Default 30 seconds

    let http_client = Arc::new(
        Client::builder()
            .pool_max_idle_per_host(10) // Limit idle connections per host
            .pool_idle_timeout(Duration::from_secs(90))
            .timeout(Duration::from_secs(client_timeout_secs))
            .build()
            .expect("Failed to create HTTP client"),
    );

    info!("⏱️  HTTP client timeout: {}s", client_timeout_secs);

    // Create VM provider
    info!("🔧 Initializing Firecracker VM provider");
    info!("   - API URL: {}", firecracker_api_url);
    info!(
        "   - Auth: {}",
        if firecracker_auth_token.is_some() {
            "Yes"
        } else {
            "No"
        }
    );
    let vm_provider: Arc<dyn VmProvider> = Arc::new(FirecrackerVmProvider::new(
        firecracker_api_url.to_string(),
        firecracker_auth_token.map(|s| s.to_string()),
        http_client.clone(),
    ));

    // Create VM command executor
    info!("⚡ Creating VmCommandExecutor for Firecracker HTTP API");
    let command_executor: Arc<VmCommandExecutor> =
        Arc::new(if let Some(token) = firecracker_auth_token {
            VmCommandExecutor::with_auth(firecracker_api_url, token.to_string(), http_client)
        } else {
            VmCommandExecutor::new(firecracker_api_url, http_client)
        });

    // Create learning coordinator
    info!("🧠 Initializing LearningCoordinator for pattern tracking");
    let _learning_coordinator: Arc<dyn LearningCoordinator> =
        Arc::new(InMemoryLearningCoordinator::new("github-runner"));

    // Create session manager with VM provider
    info!("🎯 Creating SessionManager with Firecracker VM provider");
    let session_config = SessionManagerConfig::default();
    let session_manager = Arc::new(SessionManager::with_provider(
        vm_provider.clone(),
        session_config,
    ));

    // Create workflow executor
    info!("🔨 Creating WorkflowExecutor with VM command executor");
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
    _llm_parser: Option<&WorkflowParser>,
) -> Result<String> {
    if workflow_paths.is_empty() {
        return Ok("No workflows to execute".to_string());
    }

    info!(
        "🚀 Executing {} workflows in parallel with VM isolation",
        workflow_paths.len()
    );

    // Use JoinSet for bounded parallel execution
    use tokio::task::JoinSet;
    let mut join_set = JoinSet::new();

    // Configure max concurrent workflows
    // Each workflow gets its own VM, so this limits VM usage
    let max_concurrent = std::env::var("MAX_CONCURRENT_WORKFLOWS")
        .ok()
        .and_then(|s| s.parse::<usize>().ok())
        .unwrap_or(5); // Default to 5 concurrent workflows

    info!("📊 Max concurrent workflows: {}", max_concurrent);

    // Spawn workflow tasks with bounded concurrency
    for workflow_path in workflow_paths {
        // Wait for available slot if we've reached max concurrent
        while join_set.len() >= max_concurrent {
            if let Some(result) = join_set.join_next().await {
                // Collect completed result (ignore errors, they're already logged)
                let _ = result;
            }
        }

        let workflow_path = workflow_path.clone();
        let gh_event = gh_event.clone();
        let firecracker_api_url = firecracker_api_url.to_string();
        let firecracker_auth_token = firecracker_auth_token.map(|s| s.to_string());

        // Spawn task for each workflow
        // Each task creates its own HTTP client and VM, ensuring isolation
        // Note: LLM parser not used in parallel execution to avoid lifetime issues
        join_set.spawn(async move {
            let workflow_name = workflow_path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("unknown")
                .to_string();

            info!("📋 Starting workflow: {}", workflow_name);

            let result = execute_workflow_in_vm(
                &workflow_path,
                &gh_event,
                &firecracker_api_url,
                firecracker_auth_token.as_deref(),
                None, // No LLM parser in parallel execution
            )
            .await;

            match result {
                Ok(output) => {
                    info!("✅ Workflow succeeded: {}", workflow_name);
                    format!("## {}\n{}", workflow_name, output)
                }
                Err(e) => {
                    warn!("❌ Workflow failed: {} - {}", workflow_name, e);
                    format!("## ❌ {}\n\nExecution failed: {}", workflow_name, e)
                }
            }
        });
    }

    // Collect all remaining results
    let mut results = vec![];
    while let Some(result) = join_set.join_next().await {
        match result {
            Ok(output) => results.push(output),
            Err(e) => {
                warn!("Workflow task panicked: {}", e);
                results.push("## ❌ Workflow panicked during execution".to_string());
            }
        }
    }

    info!("✅ All {} workflows completed", results.len());

    Ok(results.join("\n\n"))
}
