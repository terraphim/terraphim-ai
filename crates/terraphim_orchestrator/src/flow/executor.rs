use std::path::PathBuf;

use chrono::Utc;
use tokio::process::Command;
use tokio::time::{timeout, Duration};

use super::config::{FailStrategy, FlowDefinition, FlowStepDef, StepKind};
use super::envelope::StepEnvelope;
use super::state::{FlowRunState, FlowRunStatus};
use crate::config::{AgentDefinition, AgentLayer};
use crate::error::OrchestratorError;
use terraphim_spawner::{AgentSpawner, OutputEvent, SpawnRequest};
use terraphim_types::capability::Provider;

pub struct FlowExecutor {
    pub working_dir: PathBuf,
    pub spawner: AgentSpawner,
    pub flow_state_dir: PathBuf,
}

impl FlowExecutor {
    pub fn new(working_dir: PathBuf, flow_state_dir: PathBuf) -> Self {
        Self {
            working_dir: working_dir.clone(),
            spawner: AgentSpawner::new().with_working_dir(&working_dir),
            flow_state_dir,
        }
    }

    /// Execute an action step (shell command).
    /// Runs the command via `bash -lc` to get login shell environment.
    /// Captures stdout, stderr, and exit code into a StepEnvelope.
    pub async fn execute_action(
        &self,
        step: &FlowStepDef,
        flow: &FlowDefinition,
        state: &FlowRunState,
    ) -> Result<StepEnvelope, OrchestratorError> {
        let command = step
            .command
            .as_ref()
            .ok_or_else(|| OrchestratorError::FlowFailed {
                flow_name: flow.name.clone(),
                reason: format!("action step '{}' missing 'command' field", step.name),
            })?;

        let resolved_command = self.resolve_templates(command, flow, state);
        let started_at = Utc::now();

        let result = timeout(
            Duration::from_secs(step.timeout_secs),
            Command::new("bash")
                .arg("-lc")
                .arg(&resolved_command)
                .current_dir(&self.working_dir)
                .output(),
        )
        .await;

        match result {
            Ok(Ok(output)) => {
                let stdout = String::from_utf8_lossy(&output.stdout).to_string();
                let stderr = String::from_utf8_lossy(&output.stderr).to_string();

                // Parse token usage from combined output
                let combined_output = format!("{} {}", stdout, stderr);
                let token_usage = crate::flow::token_parser::parse_token_usage(&combined_output);

                let mut envelope = StepEnvelope {
                    step_name: step.name.clone(),
                    started_at,
                    finished_at: Utc::now(),
                    exit_code: output.status.code().unwrap_or(-1),
                    stdout,
                    stderr,
                    cost_usd: token_usage.cost_usd,
                    session_id: None,
                    input_tokens: token_usage.input_tokens,
                    output_tokens: token_usage.output_tokens,
                    stdout_file: None,
                };
                envelope.truncate_output();
                Ok(envelope)
            }
            Ok(Err(e)) => Err(OrchestratorError::FlowFailed {
                flow_name: flow.name.clone(),
                reason: format!("action step '{}' spawn failed: {}", step.name, e),
            }),
            Err(_) => Err(OrchestratorError::FlowFailed {
                flow_name: flow.name.clone(),
                reason: format!(
                    "action step '{}' timed out after {}s",
                    step.name, step.timeout_secs
                ),
            }),
        }
    }

    /// Evaluate a gate condition against completed step outputs.
    /// Supports simple expressions: `steps.<name>.exit_code == 0` and `steps.<name>.exit_code != 0`
    pub fn evaluate_gate(
        &self,
        step: &FlowStepDef,
        flow: &FlowDefinition,
        state: &FlowRunState,
    ) -> Result<bool, OrchestratorError> {
        let condition = step
            .condition
            .as_ref()
            .ok_or_else(|| OrchestratorError::FlowFailed {
                flow_name: flow.name.clone(),
                reason: format!("gate step '{}' missing 'condition' field", step.name),
            })?;

        let resolved = self.resolve_templates(condition, flow, state);

        // Parse simple expressions: "X == Y" or "X != Y"
        if let Some((lhs, rhs)) = resolved.split_once(" == ") {
            Ok(lhs.trim() == rhs.trim())
        } else if let Some((lhs, rhs)) = resolved.split_once(" != ") {
            Ok(lhs.trim() != rhs.trim())
        } else {
            Err(OrchestratorError::FlowFailed {
                flow_name: flow.name.clone(),
                reason: format!(
                    "gate step '{}': unsupported condition expression: {}",
                    step.name, resolved
                ),
            })
        }
    }

    /// Execute an agent step by spawning a CLI tool.
    /// Creates an AgentDefinition from the FlowStepDef and uses the spawner.
    pub async fn execute_agent(
        &self,
        step: &FlowStepDef,
        flow: &FlowDefinition,
        state: &FlowRunState,
    ) -> Result<StepEnvelope, OrchestratorError> {
        let cli_tool = step
            .cli_tool
            .as_ref()
            .ok_or_else(|| OrchestratorError::FlowFailed {
                flow_name: flow.name.clone(),
                reason: format!("agent step '{}' missing 'cli_tool' field", step.name),
            })?;

        // Get task from task_file or task field
        let task = if let Some(ref task_file) = step.task_file {
            let path = if std::path::Path::new(task_file).is_absolute() {
                std::path::PathBuf::from(task_file)
            } else {
                self.working_dir.join(task_file)
            };
            std::fs::read_to_string(&path).map_err(|e| OrchestratorError::FlowFailed {
                flow_name: flow.name.clone(),
                reason: format!(
                    "agent step '{}' failed to read task_file '{}': {}",
                    step.name,
                    path.display(),
                    e
                ),
            })?
        } else if let Some(ref task) = step.task {
            self.resolve_templates(task, flow, state)
        } else {
            return Err(OrchestratorError::FlowFailed {
                flow_name: flow.name.clone(),
                reason: format!(
                    "agent step '{}' missing both 'task' and 'task_file' fields",
                    step.name
                ),
            });
        };

        let started_at = Utc::now();

        // Create an AgentDefinition from the step
        let agent_def = AgentDefinition {
            name: format!("flow-{}-{}", flow.name, step.name),
            layer: AgentLayer::Core,
            cli_tool: cli_tool.clone(),
            task: task.clone(),
            model: step.model.clone(),
            schedule: None,
            capabilities: vec![],
            max_memory_bytes: None,
            budget_monthly_cents: None,
            provider: step.provider.clone(),
            persona: step.persona.clone(),
            terraphim_role: None,
            skill_chain: vec![],
            sfia_skills: vec![],
            fallback_provider: None,
            fallback_model: None,
            grace_period_secs: None,
            max_cpu_seconds: None,
            pre_check: None,
            gitea_issue: None,
            project: None,
        };

        // Build provider for spawner
        let provider = Provider {
            id: agent_def.name.clone(),
            name: agent_def.name.clone(),
            provider_type: terraphim_types::capability::ProviderType::Agent {
                agent_id: agent_def.name.clone(),
                cli_command: agent_def.cli_tool.clone(),
                working_dir: self.working_dir.clone(),
            },
            capabilities: vec![],
            cost_level: terraphim_types::capability::CostLevel::Cheap,
            latency: terraphim_types::capability::Latency::Medium,
            keywords: vec![],
        };

        // Build spawn request
        let mut request = SpawnRequest::new(provider, &agent_def.task);

        if let Some(ref model) = agent_def.model {
            request = request.with_primary_model(model);
        }

        // Spawn the agent
        let mut handle = self
            .spawner
            .spawn_with_fallback(&request)
            .await
            .map_err(|e| OrchestratorError::FlowFailed {
                flow_name: flow.name.clone(),
                reason: format!("agent step '{}' spawn failed: {}", step.name, e),
            })?;

        // Subscribe to output events BEFORE waiting for completion
        let mut output_rx = handle.subscribe_output();

        // Wait for process to exit naturally (with timeout)
        let wait_result = timeout(Duration::from_secs(step.timeout_secs), handle.wait()).await;

        let finished_at = Utc::now();

        // Try to get exit status
        let exit_code = match wait_result {
            Ok(Ok(status)) => status.code().unwrap_or(-1),
            Ok(Err(_)) => -1,
            Err(_) => {
                // Timeout - kill the process
                let _ = handle.shutdown(Duration::from_secs(5)).await;
                return Err(OrchestratorError::FlowFailed {
                    flow_name: flow.name.clone(),
                    reason: format!(
                        "agent step '{}' timed out after {}s",
                        step.name, step.timeout_secs
                    ),
                });
            }
        };

        // Drain output events to capture stdout and stderr
        let mut stdout_lines = Vec::new();
        let mut stderr_lines = Vec::new();
        while let Ok(event) = output_rx.try_recv() {
            match event {
                OutputEvent::Stdout { line, .. } => stdout_lines.push(line),
                OutputEvent::Stderr { line, .. } => stderr_lines.push(line),
                _ => {}
            }
        }

        // Parse token usage from combined output
        let stdout = stdout_lines.join("\n");
        let stderr = stderr_lines.join("\n");
        let combined_output = format!("{} {}", stdout, stderr);
        let token_usage = crate::flow::token_parser::parse_token_usage(&combined_output);

        // Build envelope with captured output
        let mut envelope = StepEnvelope {
            step_name: step.name.clone(),
            started_at,
            finished_at,
            exit_code,
            stdout,
            stderr,
            cost_usd: token_usage.cost_usd,
            session_id: None,
            input_tokens: token_usage.input_tokens,
            output_tokens: token_usage.output_tokens,
            stdout_file: None,
        };

        // Truncate stdout and stderr if they exceed limits
        envelope.truncate_output();

        Ok(envelope)
    }

    /// Run a flow from start (or resume from checkpoint state).
    /// This method MUST be tokio::spawn'd by the caller (never awaited inline).
    pub async fn run(
        &self,
        flow: &FlowDefinition,
        resume_state: Option<FlowRunState>,
    ) -> Result<FlowRunState, OrchestratorError> {
        // 1. Initialise or resume state
        let mut state = resume_state.unwrap_or_else(|| FlowRunState::new(&flow.name));

        // 2. Validate all task_file references at start (fail-fast)
        for step in &flow.steps {
            if let Some(ref task_file) = step.task_file {
                let path = if std::path::Path::new(task_file).is_absolute() {
                    std::path::PathBuf::from(task_file)
                } else {
                    self.working_dir.join(task_file)
                };
                if !path.exists() {
                    return Err(OrchestratorError::FlowFailed {
                        flow_name: flow.name.clone(),
                        reason: format!("task_file not found: {}", path.display()),
                    });
                }
            }
        }

        // 3. Calculate flow deadline for global timeout
        let flow_deadline = tokio::time::Instant::now() + Duration::from_secs(flow.timeout_secs);

        // 4. Execute steps sequentially from next_step_index
        let start_index = state.next_step_index;
        for (i, step) in flow.steps.iter().enumerate().skip(start_index) {
            state.next_step_index = i;

            // Check global flow timeout before each step
            if tokio::time::Instant::now() >= flow_deadline {
                state.status = FlowRunStatus::Failed;
                state.finished_at = Some(Utc::now());
                state.error = Some(format!(
                    "flow '{}' exceeded global timeout of {}s",
                    flow.name, flow.timeout_secs
                ));
                let _ = state.save_to_file(&self.flow_state_dir);
                return Err(OrchestratorError::FlowFailed {
                    flow_name: flow.name.clone(),
                    reason: format!("flow exceeded global timeout of {}s", flow.timeout_secs),
                });
            }

            // Handle checkpoint step: persist state and pause
            if step.kind == StepKind::Checkpoint {
                state.status = FlowRunStatus::Paused;
                state.next_step_index = i + 1;
                let _ = state.save_to_file(&self.flow_state_dir);
                tracing::info!(flow = %flow.name, step = %step.name, next_index = i + 1, "flow paused at checkpoint");
                return Ok(state);
            }

            let result = match step.kind {
                StepKind::Action => self.execute_action(step, flow, &state).await,
                StepKind::Agent => self.execute_agent(step, flow, &state).await,
                StepKind::Gate => {
                    match self.evaluate_gate(step, flow, &state) {
                        Ok(true) => {
                            // Gate passed -- create a synthetic envelope
                            Ok(StepEnvelope {
                                step_name: step.name.clone(),
                                started_at: Utc::now(),
                                finished_at: Utc::now(),
                                exit_code: 0,
                                stdout: "gate passed".to_string(),
                                stderr: String::new(),
                                cost_usd: None,
                                session_id: None,
                                input_tokens: None,
                                output_tokens: None,
                                stdout_file: None,
                            })
                        }
                        Ok(false) => {
                            // Gate failed
                            match step.on_fail {
                                FailStrategy::Abort => {
                                    // Record the gate failure before aborting
                                    let gate_envelope = StepEnvelope {
                                        step_name: step.name.clone(),
                                        started_at: Utc::now(),
                                        finished_at: Utc::now(),
                                        exit_code: 1,
                                        stdout: "gate failed".to_string(),
                                        stderr: String::new(),
                                        cost_usd: None,
                                        session_id: None,
                                        input_tokens: None,
                                        output_tokens: None,
                                        stdout_file: None,
                                    };
                                    state.step_envelopes.push(gate_envelope);
                                    state.status = FlowRunStatus::Aborted;
                                    state.finished_at = Some(Utc::now());
                                    state.error = Some(format!("gate '{}' rejected", step.name));
                                    let _ = state.save_to_file(&self.flow_state_dir);
                                    return Ok(state);
                                }
                                FailStrategy::SkipFailed | FailStrategy::Continue => {
                                    // Create a synthetic failed envelope and continue
                                    Ok(StepEnvelope {
                                        step_name: step.name.clone(),
                                        started_at: Utc::now(),
                                        finished_at: Utc::now(),
                                        exit_code: 1,
                                        stdout: "gate failed".to_string(),
                                        stderr: String::new(),
                                        cost_usd: None,
                                        session_id: None,
                                        input_tokens: None,
                                        output_tokens: None,
                                        stdout_file: None,
                                    })
                                }
                            }
                        }
                        Err(e) => Err(e),
                    }
                }
                StepKind::Checkpoint => {
                    unreachable!("Checkpoint should be handled before the match")
                }
            };

            match result {
                Ok(mut envelope) => {
                    // Write stdout to temp file for downstream action steps
                    let stdout_file_path =
                        format!("/tmp/flow-{}-{}.stdout", state.correlation_id, step.name);
                    if let Err(e) = std::fs::write(&stdout_file_path, &envelope.stdout) {
                        tracing::warn!(step = %step.name, error = %e, "failed to write stdout temp file");
                    } else {
                        envelope.stdout_file = Some(stdout_file_path);
                    }

                    state.step_envelopes.push(envelope);
                    state.next_step_index = i + 1;
                    let _ = state.save_to_file(&self.flow_state_dir);
                }
                Err(e) => match step.on_fail {
                    FailStrategy::Abort => {
                        state.status = FlowRunStatus::Failed;
                        state.finished_at = Some(Utc::now());
                        state.error = Some(e.to_string());
                        let _ = state.save_to_file(&self.flow_state_dir);
                        return Err(e);
                    }
                    FailStrategy::SkipFailed | FailStrategy::Continue => {
                        tracing::warn!(step = %step.name, error = %e, "step failed, continuing per on_fail policy");
                        continue;
                    }
                },
            }
        }

        // 4. All steps complete
        state.status = FlowRunStatus::Completed;
        state.finished_at = Some(Utc::now());
        let _ = state.save_to_file(&self.flow_state_dir);

        // Temp file cleanup: stdout files in /tmp/flow-{correlation_id}-{step_name}.stdout
        // are intentionally NOT cleaned up here to preserve debug data.
        // Cleanup happens on orchestrator restart via a sweep of /tmp/flow-* files.

        Ok(state)
    }

    /// Resolve template variables in a string.
    /// Supports: {{repo_path}}, {{base_branch}}, {{flow.name}}, {{flow.correlation_id}},
    /// {{steps.<name>.stdout}}, {{steps.<name>.stderr}}, {{steps.<name>.exit_code}},
    /// {{steps.<name>.stdout_file}}
    pub fn resolve_templates(
        &self,
        template: &str,
        flow: &FlowDefinition,
        state: &FlowRunState,
    ) -> String {
        let mut result = template.to_string();
        result = result.replace("{{repo_path}}", &flow.repo_path);
        result = result.replace("{{base_branch}}", &flow.base_branch);
        result = result.replace("{{flow.name}}", &flow.name);
        result = result.replace("{{flow.correlation_id}}", &state.correlation_id.to_string());

        // Resolve step references: {{steps.<name>.stdout}}, etc.
        for envelope in &state.step_envelopes {
            let prefix = format!("{{{{steps.{}", envelope.step_name);
            if result.contains(&prefix) {
                result = result.replace(
                    &format!("{{{{steps.{}.stdout}}}}", envelope.step_name),
                    &envelope.stdout,
                );
                result = result.replace(
                    &format!("{{{{steps.{}.stderr}}}}", envelope.step_name),
                    &envelope.stderr,
                );
                result = result.replace(
                    &format!("{{{{steps.{}.exit_code}}}}", envelope.step_name),
                    &envelope.exit_code.to_string(),
                );
                if let Some(ref stdout_file) = envelope.stdout_file {
                    result = result.replace(
                        &format!("{{{{steps.{}.stdout_file}}}}", envelope.step_name),
                        stdout_file,
                    );
                }
            }
        }

        // Remove any unresolved {{...}} references (resolve to empty string)
        let re = regex::Regex::new(r"\{\{[^}]+\}\}").unwrap();
        result = re.replace_all(&result, "").to_string();

        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::flow::envelope::StepEnvelope;
    use chrono::Utc;

    fn create_test_flow() -> FlowDefinition {
        FlowDefinition {
            name: "test-flow".to_string(),
            project: "test".to_string(),
            schedule: None,
            repo_path: "/home/user/project".to_string(),
            base_branch: "develop".to_string(),
            timeout_secs: 3600,
            steps: vec![],
        }
    }

    fn create_test_state_with_envelope(
        step_name: &str,
        stdout: &str,
        exit_code: i32,
    ) -> FlowRunState {
        let mut state = FlowRunState::new("test-flow");
        state.step_envelopes.push(StepEnvelope {
            step_name: step_name.to_string(),
            started_at: Utc::now(),
            finished_at: Utc::now(),
            exit_code,
            stdout: stdout.to_string(),
            stderr: "error output".to_string(),
            cost_usd: None,
            session_id: None,
            input_tokens: None,
            output_tokens: None,
            stdout_file: Some(format!("/tmp/{}.out", step_name)),
        });
        state
    }

    #[test]
    fn test_resolve_templates_basic() {
        let executor = FlowExecutor::new(PathBuf::from("/tmp"), PathBuf::from("/tmp/state"));
        let flow = create_test_flow();
        let state = FlowRunState::new("test-flow");

        let template = "cd {{repo_path}} && git checkout {{base_branch}}";
        let result = executor.resolve_templates(template, &flow, &state);

        assert_eq!(result, "cd /home/user/project && git checkout develop");
    }

    #[test]
    fn test_resolve_templates_flow_vars() {
        let executor = FlowExecutor::new(PathBuf::from("/tmp"), PathBuf::from("/tmp/state"));
        let flow = create_test_flow();
        let state = FlowRunState::new("test-flow");

        let template = "Flow: {{flow.name}}, ID: {{flow.correlation_id}}";
        let result = executor.resolve_templates(template, &flow, &state);

        assert!(result.contains("Flow: test-flow"));
        assert!(result.contains("ID: "));
        // The correlation_id should be a valid UUID in the result
        assert!(!result.contains("{{flow.correlation_id}}"));
    }

    #[test]
    fn test_resolve_templates_step_ref() {
        let executor = FlowExecutor::new(PathBuf::from("/tmp"), PathBuf::from("/tmp/state"));
        let flow = create_test_flow();
        let state = create_test_state_with_envelope("gather-changes", "file1.rs\nfile2.rs", 0);

        let template = "Files changed: {{steps.gather-changes.stdout}}";
        let result = executor.resolve_templates(template, &flow, &state);

        assert_eq!(result, "Files changed: file1.rs\nfile2.rs");
    }

    #[test]
    fn test_resolve_templates_step_stderr() {
        let executor = FlowExecutor::new(PathBuf::from("/tmp"), PathBuf::from("/tmp/state"));
        let flow = create_test_flow();
        let state = create_test_state_with_envelope("build", "success", 0);

        let template = "Output: {{steps.build.stderr}}";
        let result = executor.resolve_templates(template, &flow, &state);

        assert_eq!(result, "Output: error output");
    }

    #[test]
    fn test_resolve_templates_step_exit_code() {
        let executor = FlowExecutor::new(PathBuf::from("/tmp"), PathBuf::from("/tmp/state"));
        let flow = create_test_flow();
        let state = create_test_state_with_envelope("test", "", 1);

        let template = "Exit code was: {{steps.test.exit_code}}";
        let result = executor.resolve_templates(template, &flow, &state);

        assert_eq!(result, "Exit code was: 1");
    }

    #[test]
    fn test_resolve_templates_step_stdout_file() {
        let executor = FlowExecutor::new(PathBuf::from("/tmp"), PathBuf::from("/tmp/state"));
        let flow = create_test_flow();
        let state = create_test_state_with_envelope("build", "output", 0);

        let template = "cat {{steps.build.stdout_file}}";
        let result = executor.resolve_templates(template, &flow, &state);

        assert_eq!(result, "cat /tmp/build.out");
    }

    #[test]
    fn test_resolve_templates_missing_step() {
        let executor = FlowExecutor::new(PathBuf::from("/tmp"), PathBuf::from("/tmp/state"));
        let flow = create_test_flow();
        let state = FlowRunState::new("test-flow");

        let template = "Output: {{steps.nonexistent.stdout}}";
        let result = executor.resolve_templates(template, &flow, &state);

        // Unresolved template should be replaced with empty string
        assert_eq!(result, "Output: ");
    }

    #[test]
    fn test_resolve_templates_multiple_refs() {
        let executor = FlowExecutor::new(PathBuf::from("/tmp"), PathBuf::from("/tmp/state"));
        let flow = create_test_flow();

        let mut state = FlowRunState::new("test-flow");
        state.step_envelopes.push(StepEnvelope {
            step_name: "step1".to_string(),
            started_at: Utc::now(),
            finished_at: Utc::now(),
            exit_code: 0,
            stdout: "output1".to_string(),
            stderr: "stderr1".to_string(),
            cost_usd: None,
            session_id: None,
            input_tokens: None,
            output_tokens: None,
            stdout_file: None,
        });
        state.step_envelopes.push(StepEnvelope {
            step_name: "step2".to_string(),
            started_at: Utc::now(),
            finished_at: Utc::now(),
            exit_code: 1,
            stdout: "output2".to_string(),
            stderr: "stderr2".to_string(),
            cost_usd: None,
            session_id: None,
            input_tokens: None,
            output_tokens: None,
            stdout_file: None,
        });

        let template = "{{repo_path}}: {{steps.step1.stdout}} (exit: {{steps.step1.exit_code}}), {{steps.step2.stdout}} (exit: {{steps.step2.exit_code}})";
        let result = executor.resolve_templates(template, &flow, &state);

        assert_eq!(
            result,
            "/home/user/project: output1 (exit: 0), output2 (exit: 1)"
        );
    }

    #[test]
    fn test_resolve_templates_partial_missing() {
        let executor = FlowExecutor::new(PathBuf::from("/tmp"), PathBuf::from("/tmp/state"));
        let flow = create_test_flow();
        let state = create_test_state_with_envelope("exists", "found", 0);

        let template = "{{steps.exists.stdout}} and {{steps.missing.stdout}}";
        let result = executor.resolve_templates(template, &flow, &state);

        assert_eq!(result, "found and ");
    }

    #[test]
    fn test_resolve_templates_no_templates() {
        let executor = FlowExecutor::new(PathBuf::from("/tmp"), PathBuf::from("/tmp/state"));
        let flow = create_test_flow();
        let state = FlowRunState::new("test-flow");

        let template = "Just a plain string with no templates";
        let result = executor.resolve_templates(template, &flow, &state);

        assert_eq!(result, "Just a plain string with no templates");
    }

    #[test]
    fn test_resolve_templates_empty_string() {
        let executor = FlowExecutor::new(PathBuf::from("/tmp"), PathBuf::from("/tmp/state"));
        let flow = create_test_flow();
        let state = FlowRunState::new("test-flow");

        let template = "";
        let result = executor.resolve_templates(template, &flow, &state);

        assert_eq!(result, "");
    }

    #[test]
    fn test_resolve_templates_complex_condition() {
        let executor = FlowExecutor::new(PathBuf::from("/tmp"), PathBuf::from("/tmp/state"));
        let flow = create_test_flow();
        let state = create_test_state_with_envelope("test", "", 42);

        // Simulating a gate condition
        let template = "steps.test.exit_code == 0 && {{steps.test.exit_code}} == 0";
        let result = executor.resolve_templates(template, &flow, &state);

        assert_eq!(result, "steps.test.exit_code == 0 && 42 == 0");
    }

    #[tokio::test]
    async fn test_execute_action_echo() {
        let temp_dir = tempfile::tempdir().unwrap();
        let executor =
            FlowExecutor::new(temp_dir.path().to_path_buf(), temp_dir.path().to_path_buf());

        let flow = FlowDefinition {
            name: "test-flow".to_string(),
            project: "test".to_string(),
            schedule: None,
            repo_path: "/tmp/repo".to_string(),
            base_branch: "main".to_string(),
            timeout_secs: 3600,
            steps: vec![],
        };

        let state = FlowRunState::new("test-flow");

        let step = FlowStepDef {
            name: "echo-step".to_string(),
            kind: StepKind::Action,
            command: Some("echo hello".to_string()),
            cli_tool: None,
            model: None,
            task: None,
            task_file: None,
            condition: None,
            timeout_secs: 10,
            on_fail: FailStrategy::Abort,
            provider: None,
            persona: None,
        };

        let envelope = executor.execute_action(&step, &flow, &state).await.unwrap();

        assert_eq!(envelope.step_name, "echo-step");
        assert_eq!(envelope.exit_code, 0);
        assert_eq!(envelope.stdout.trim(), "hello");
    }

    #[tokio::test]
    async fn test_execute_action_timeout() {
        let temp_dir = tempfile::tempdir().unwrap();
        let executor =
            FlowExecutor::new(temp_dir.path().to_path_buf(), temp_dir.path().to_path_buf());

        let flow = FlowDefinition {
            name: "test-flow".to_string(),
            project: "test".to_string(),
            schedule: None,
            repo_path: "/tmp/repo".to_string(),
            base_branch: "main".to_string(),
            timeout_secs: 3600,
            steps: vec![],
        };

        let state = FlowRunState::new("test-flow");

        let step = FlowStepDef {
            name: "sleep-step".to_string(),
            kind: StepKind::Action,
            command: Some("sleep 10".to_string()),
            cli_tool: None,
            model: None,
            task: None,
            task_file: None,
            condition: None,
            timeout_secs: 1,
            on_fail: FailStrategy::Abort,
            provider: None,
            persona: None,
        };

        let result = executor.execute_action(&step, &flow, &state).await;

        assert!(result.is_err());
        let err_str = result.unwrap_err().to_string();
        assert!(err_str.contains("timed out"));
    }

    #[test]
    fn test_evaluate_gate_exit_code_zero() {
        let executor = FlowExecutor::new(PathBuf::from("/tmp"), PathBuf::from("/tmp/state"));

        let flow = FlowDefinition {
            name: "test-flow".to_string(),
            project: "test".to_string(),
            schedule: None,
            repo_path: "/tmp/repo".to_string(),
            base_branch: "main".to_string(),
            timeout_secs: 3600,
            steps: vec![],
        };

        let mut state = FlowRunState::new("test-flow");
        state.step_envelopes.push(StepEnvelope {
            step_name: "test-step".to_string(),
            started_at: Utc::now(),
            finished_at: Utc::now(),
            exit_code: 0,
            stdout: String::new(),
            stderr: String::new(),
            cost_usd: None,
            session_id: None,
            input_tokens: None,
            output_tokens: None,
            stdout_file: None,
        });

        let step = FlowStepDef {
            name: "gate-step".to_string(),
            kind: StepKind::Gate,
            command: None,
            cli_tool: None,
            model: None,
            task: None,
            task_file: None,
            condition: Some("{{steps.test-step.exit_code}} == 0".to_string()),
            timeout_secs: 10,
            on_fail: FailStrategy::Abort,
            provider: None,
            persona: None,
        };

        let result = executor.evaluate_gate(&step, &flow, &state).unwrap();

        assert!(result, "Gate should pass when exit_code == 0");
    }

    #[test]
    fn test_evaluate_gate_exit_code_nonzero() {
        let executor = FlowExecutor::new(PathBuf::from("/tmp"), PathBuf::from("/tmp/state"));

        let flow = FlowDefinition {
            name: "test-flow".to_string(),
            project: "test".to_string(),
            schedule: None,
            repo_path: "/tmp/repo".to_string(),
            base_branch: "main".to_string(),
            timeout_secs: 3600,
            steps: vec![],
        };

        let mut state = FlowRunState::new("test-flow");
        state.step_envelopes.push(StepEnvelope {
            step_name: "test-step".to_string(),
            started_at: Utc::now(),
            finished_at: Utc::now(),
            exit_code: 1,
            stdout: String::new(),
            stderr: String::new(),
            cost_usd: None,
            session_id: None,
            input_tokens: None,
            output_tokens: None,
            stdout_file: None,
        });

        let step = FlowStepDef {
            name: "gate-step".to_string(),
            kind: StepKind::Gate,
            command: None,
            cli_tool: None,
            model: None,
            task: None,
            task_file: None,
            condition: Some("{{steps.test-step.exit_code}} == 0".to_string()),
            timeout_secs: 10,
            on_fail: FailStrategy::Abort,
            provider: None,
            persona: None,
        };

        let result = executor.evaluate_gate(&step, &flow, &state).unwrap();

        assert!(!result, "Gate should fail when exit_code != 0");
    }

    #[tokio::test]
    async fn test_flow_executor_two_actions() {
        let temp_dir = tempfile::tempdir().unwrap();
        let executor =
            FlowExecutor::new(temp_dir.path().to_path_buf(), temp_dir.path().to_path_buf());

        let flow = FlowDefinition {
            name: "test-flow".to_string(),
            project: "test".to_string(),
            schedule: None,
            repo_path: "/tmp/repo".to_string(),
            base_branch: "main".to_string(),
            timeout_secs: 3600,
            steps: vec![
                FlowStepDef {
                    name: "step1".to_string(),
                    kind: StepKind::Action,
                    command: Some("echo 'first output'".to_string()),
                    cli_tool: None,
                    model: None,
                    task: None,
                    task_file: None,
                    condition: None,
                    timeout_secs: 10,
                    on_fail: FailStrategy::Abort,
                    provider: None,
                    persona: None,
                },
                FlowStepDef {
                    name: "step2".to_string(),
                    kind: StepKind::Action,
                    command: Some("echo 'got: {{steps.step1.stdout}}'".to_string()),
                    cli_tool: None,
                    model: None,
                    task: None,
                    task_file: None,
                    condition: None,
                    timeout_secs: 10,
                    on_fail: FailStrategy::Abort,
                    provider: None,
                    persona: None,
                },
            ],
        };

        let state = executor.run(&flow, None).await.unwrap();

        assert_eq!(state.status, FlowRunStatus::Completed);
        assert_eq!(state.step_envelopes.len(), 2);

        // Check that step2 resolved the template from step1
        let step2_output = &state.step_envelopes[1].stdout;
        assert!(
            step2_output.contains("first output"),
            "step2 should resolve template from step1: {}",
            step2_output
        );
    }

    #[tokio::test]
    async fn test_flow_executor_gate_pass() {
        let temp_dir = tempfile::tempdir().unwrap();
        let executor =
            FlowExecutor::new(temp_dir.path().to_path_buf(), temp_dir.path().to_path_buf());

        let flow = FlowDefinition {
            name: "test-flow".to_string(),
            project: "test".to_string(),
            schedule: None,
            repo_path: "/tmp/repo".to_string(),
            base_branch: "main".to_string(),
            timeout_secs: 3600,
            steps: vec![
                FlowStepDef {
                    name: "action1".to_string(),
                    kind: StepKind::Action,
                    command: Some("echo 'test'".to_string()),
                    cli_tool: None,
                    model: None,
                    task: None,
                    task_file: None,
                    condition: None,
                    timeout_secs: 10,
                    on_fail: FailStrategy::Abort,
                    provider: None,
                    persona: None,
                },
                FlowStepDef {
                    name: "gate1".to_string(),
                    kind: StepKind::Gate,
                    command: None,
                    cli_tool: None,
                    model: None,
                    task: None,
                    task_file: None,
                    condition: Some("{{steps.action1.exit_code}} == 0".to_string()),
                    timeout_secs: 10,
                    on_fail: FailStrategy::Abort,
                    provider: None,
                    persona: None,
                },
                FlowStepDef {
                    name: "action2".to_string(),
                    kind: StepKind::Action,
                    command: Some("echo 'after gate'".to_string()),
                    cli_tool: None,
                    model: None,
                    task: None,
                    task_file: None,
                    condition: None,
                    timeout_secs: 10,
                    on_fail: FailStrategy::Abort,
                    provider: None,
                    persona: None,
                },
            ],
        };

        let state = executor.run(&flow, None).await.unwrap();

        assert_eq!(state.status, FlowRunStatus::Completed);
        assert_eq!(state.step_envelopes.len(), 3);
    }

    #[tokio::test]
    async fn test_flow_executor_gate_fail_abort() {
        let temp_dir = tempfile::tempdir().unwrap();
        let executor =
            FlowExecutor::new(temp_dir.path().to_path_buf(), temp_dir.path().to_path_buf());

        let flow = FlowDefinition {
            name: "test-flow".to_string(),
            project: "test".to_string(),
            schedule: None,
            repo_path: "/tmp/repo".to_string(),
            base_branch: "main".to_string(),
            timeout_secs: 3600,
            steps: vec![
                FlowStepDef {
                    name: "action1".to_string(),
                    kind: StepKind::Action,
                    command: Some("echo 'test'".to_string()),
                    cli_tool: None,
                    model: None,
                    task: None,
                    task_file: None,
                    condition: None,
                    timeout_secs: 10,
                    on_fail: FailStrategy::Abort,
                    provider: None,
                    persona: None,
                },
                FlowStepDef {
                    name: "gate1".to_string(),
                    kind: StepKind::Gate,
                    command: None,
                    cli_tool: None,
                    model: None,
                    task: None,
                    task_file: None,
                    condition: Some("{{steps.action1.exit_code}} != 0".to_string()),
                    timeout_secs: 10,
                    on_fail: FailStrategy::Abort,
                    provider: None,
                    persona: None,
                },
                FlowStepDef {
                    name: "action2".to_string(),
                    kind: StepKind::Action,
                    command: Some("echo 'should not run'".to_string()),
                    cli_tool: None,
                    model: None,
                    task: None,
                    task_file: None,
                    condition: None,
                    timeout_secs: 10,
                    on_fail: FailStrategy::Abort,
                    provider: None,
                    persona: None,
                },
            ],
        };

        let state = executor.run(&flow, None).await.unwrap();

        // Flow should abort at the gate
        assert_eq!(state.status, FlowRunStatus::Aborted);
        assert_eq!(state.step_envelopes.len(), 2); // Only action1 and gate1
    }

    #[tokio::test]
    async fn test_flow_executor_state_persistence() {
        let temp_dir = tempfile::tempdir().unwrap();
        let state_dir = temp_dir.path().join("state");
        std::fs::create_dir_all(&state_dir).unwrap();

        let executor = FlowExecutor::new(temp_dir.path().to_path_buf(), state_dir.clone());

        let flow = FlowDefinition {
            name: "test-flow".to_string(),
            project: "test".to_string(),
            schedule: None,
            repo_path: "/tmp/repo".to_string(),
            base_branch: "main".to_string(),
            timeout_secs: 3600,
            steps: vec![
                FlowStepDef {
                    name: "step1".to_string(),
                    kind: StepKind::Action,
                    command: Some("echo 'step1'".to_string()),
                    cli_tool: None,
                    model: None,
                    task: None,
                    task_file: None,
                    condition: None,
                    timeout_secs: 10,
                    on_fail: FailStrategy::Abort,
                    provider: None,
                    persona: None,
                },
                FlowStepDef {
                    name: "step2".to_string(),
                    kind: StepKind::Action,
                    command: Some("echo 'step2'".to_string()),
                    cli_tool: None,
                    model: None,
                    task: None,
                    task_file: None,
                    condition: None,
                    timeout_secs: 10,
                    on_fail: FailStrategy::Abort,
                    provider: None,
                    persona: None,
                },
                FlowStepDef {
                    name: "step3".to_string(),
                    kind: StepKind::Action,
                    command: Some("echo 'step3'".to_string()),
                    cli_tool: None,
                    model: None,
                    task: None,
                    task_file: None,
                    condition: None,
                    timeout_secs: 10,
                    on_fail: FailStrategy::Abort,
                    provider: None,
                    persona: None,
                },
            ],
        };

        let state = executor.run(&flow, None).await.unwrap();

        assert_eq!(state.status, FlowRunStatus::Completed);
        assert_eq!(state.step_envelopes.len(), 3);

        // Verify state file was created
        let state_files: Vec<_> = std::fs::read_dir(&state_dir)
            .unwrap()
            .filter_map(|e| e.ok())
            .filter(|e| {
                e.file_name()
                    .to_string_lossy()
                    .starts_with("flow-test-flow-")
            })
            .collect();

        assert!(!state_files.is_empty(), "State file should be created");
    }

    #[tokio::test]
    async fn test_flow_executor_resume_from_checkpoint() {
        let temp_dir = tempfile::tempdir().unwrap();
        let state_dir = temp_dir.path().join("state");
        std::fs::create_dir_all(&state_dir).unwrap();

        let executor = FlowExecutor::new(temp_dir.path().to_path_buf(), state_dir.clone());

        let flow = FlowDefinition {
            name: "test-flow".to_string(),
            project: "test".to_string(),
            schedule: None,
            repo_path: "/tmp/repo".to_string(),
            base_branch: "main".to_string(),
            timeout_secs: 3600,
            steps: vec![
                FlowStepDef {
                    name: "step1".to_string(),
                    kind: StepKind::Action,
                    command: Some("echo 'step1'".to_string()),
                    cli_tool: None,
                    model: None,
                    task: None,
                    task_file: None,
                    condition: None,
                    timeout_secs: 10,
                    on_fail: FailStrategy::Abort,
                    provider: None,
                    persona: None,
                },
                FlowStepDef {
                    name: "step2".to_string(),
                    kind: StepKind::Action,
                    command: Some("echo 'step2'".to_string()),
                    cli_tool: None,
                    model: None,
                    task: None,
                    task_file: None,
                    condition: None,
                    timeout_secs: 10,
                    on_fail: FailStrategy::Abort,
                    provider: None,
                    persona: None,
                },
                FlowStepDef {
                    name: "step3".to_string(),
                    kind: StepKind::Action,
                    command: Some("echo 'step3'".to_string()),
                    cli_tool: None,
                    model: None,
                    task: None,
                    task_file: None,
                    condition: None,
                    timeout_secs: 10,
                    on_fail: FailStrategy::Abort,
                    provider: None,
                    persona: None,
                },
            ],
        };

        // Create a state at step 1 (step 0 already completed)
        let mut resume_state = FlowRunState::new("test-flow");
        resume_state.next_step_index = 1;
        resume_state.step_envelopes.push(StepEnvelope {
            step_name: "step1".to_string(),
            started_at: Utc::now(),
            finished_at: Utc::now(),
            exit_code: 0,
            stdout: "already completed".to_string(),
            stderr: String::new(),
            cost_usd: None,
            session_id: None,
            input_tokens: None,
            output_tokens: None,
            stdout_file: None,
        });

        let state = executor.run(&flow, Some(resume_state)).await.unwrap();

        // Should only run steps 2 and 3
        assert_eq!(state.status, FlowRunStatus::Completed);
        assert_eq!(state.step_envelopes.len(), 3);
        assert_eq!(state.step_envelopes[0].stdout, "already completed");
        assert!(state.step_envelopes[1].stdout.contains("step2"));
        assert!(state.step_envelopes[2].stdout.contains("step3"));
    }

    #[tokio::test]
    async fn test_flow_executor_checkpoint_pauses() {
        let dir = std::env::temp_dir();
        let state_dir = dir.join("test-checkpoint-pause");
        let _ = std::fs::remove_dir_all(&state_dir);
        std::fs::create_dir_all(&state_dir).unwrap();
        let executor = FlowExecutor::new(dir.clone(), state_dir.clone());

        let flow = FlowDefinition {
            name: "checkpoint-test".to_string(),
            project: "test".to_string(),
            schedule: None,
            repo_path: dir.to_string_lossy().to_string(),
            base_branch: "main".to_string(),
            timeout_secs: 3600,
            steps: vec![
                FlowStepDef {
                    name: "step1".to_string(),
                    kind: StepKind::Action,
                    command: Some("echo hello".to_string()),
                    cli_tool: None,
                    model: None,
                    task: None,
                    task_file: None,
                    condition: None,
                    timeout_secs: 10,
                    on_fail: FailStrategy::Abort,
                    provider: None,
                    persona: None,
                },
                FlowStepDef {
                    name: "checkpoint1".to_string(),
                    kind: StepKind::Checkpoint,
                    command: None,
                    cli_tool: None,
                    model: None,
                    task: None,
                    task_file: None,
                    condition: None,
                    timeout_secs: 10,
                    on_fail: FailStrategy::Abort,
                    provider: None,
                    persona: None,
                },
                FlowStepDef {
                    name: "step2".to_string(),
                    kind: StepKind::Action,
                    command: Some("echo world".to_string()),
                    cli_tool: None,
                    model: None,
                    task: None,
                    task_file: None,
                    condition: None,
                    timeout_secs: 10,
                    on_fail: FailStrategy::Abort,
                    provider: None,
                    persona: None,
                },
            ],
        };

        let result = executor.run(&flow, None).await.unwrap();

        // Flow should be paused, not completed
        assert_eq!(result.status, FlowRunStatus::Paused);
        // Only step1 should have run
        assert_eq!(result.step_envelopes.len(), 1);
        assert_eq!(result.step_envelopes[0].step_name, "step1");
        // Next step index should be past the checkpoint (index 2, which is step2)
        assert_eq!(result.next_step_index, 2);

        // Resume from checkpoint -- should run step2 and complete
        let resumed = executor.run(&flow, Some(result)).await.unwrap();
        assert_eq!(resumed.status, FlowRunStatus::Completed);
        // Should now have 2 step envelopes (step1 from before + step2 from resume)
        assert_eq!(resumed.step_envelopes.len(), 2);
        assert_eq!(resumed.step_envelopes[1].step_name, "step2");

        // Clean up test state dir
        let _ = std::fs::remove_dir_all(&state_dir);
    }

    #[tokio::test]
    async fn test_flow_global_timeout() {
        let dir = std::env::temp_dir();
        let state_dir = dir.join("test-flow-global-timeout");
        let _ = std::fs::remove_dir_all(&state_dir);
        let executor = FlowExecutor::new(dir.clone(), state_dir.clone());

        let flow = FlowDefinition {
            name: "timeout-test".to_string(),
            project: "test".to_string(),
            schedule: None,
            repo_path: dir.to_string_lossy().to_string(),
            base_branch: "main".to_string(),
            timeout_secs: 1,
            steps: vec![
                FlowStepDef {
                    name: "slow-step".to_string(),
                    kind: StepKind::Action,
                    command: Some("sleep 2".to_string()),
                    cli_tool: None,
                    model: None,
                    task: None,
                    task_file: None,
                    condition: None,
                    timeout_secs: 10,
                    on_fail: FailStrategy::Abort,
                    provider: None,
                    persona: None,
                },
                FlowStepDef {
                    name: "never-runs".to_string(),
                    kind: StepKind::Action,
                    command: Some("echo unreachable".to_string()),
                    cli_tool: None,
                    model: None,
                    task: None,
                    task_file: None,
                    condition: None,
                    timeout_secs: 10,
                    on_fail: FailStrategy::Abort,
                    provider: None,
                    persona: None,
                },
            ],
        };

        let result = executor.run(&flow, None).await;
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("global timeout"),
            "Expected global timeout error, got: {}",
            err
        );

        let _ = std::fs::remove_dir_all(&state_dir);
    }
}
