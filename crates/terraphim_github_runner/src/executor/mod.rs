//! Action execution in Firecracker VMs

pub mod vm_execution;
pub mod artifact_manager;

pub use vm_execution::VmExecutor;
pub use artifact_manager::ArtifactManager;

use crate::{
    InterpretedAction, RunnerConfig, RunnerResult, RunnerError, Step, StepResult, ArtifactRef,
};
use crate::interpreter::ActionInterpreter;
use crate::knowledge_graph::ActionGraph;
use crate::history::ExecutionHistory;
use ahash::AHashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Main action executor orchestrating VM execution
pub struct ActionExecutor {
    /// Configuration
    config: RunnerConfig,
    /// VM executor
    vm_executor: VmExecutor,
    /// Artifact manager
    artifact_manager: ArtifactManager,
    /// Action interpreter
    interpreter: ActionInterpreter,
    /// Action graph (optional)
    graph: Option<Arc<ActionGraph>>,
    /// Execution history
    history: Arc<RwLock<ExecutionHistory>>,
}

impl ActionExecutor {
    /// Create a new action executor
    pub fn new(config: RunnerConfig) -> Self {
        let vm_pool_size = config.vm_pool_size;
        let work_dir = config.work_directory.clone();
        let enable_llm = config.enable_llm;
        let llm_config = config.llm_config.clone();

        let mut interpreter = ActionInterpreter::new();
        if enable_llm {
            if let Some(llm) = llm_config {
                interpreter = interpreter.with_llm(
                    &llm.provider,
                    &llm.model,
                    llm.base_url.as_deref(),
                );
            }
        }

        Self {
            config,
            vm_executor: VmExecutor::new(vm_pool_size),
            artifact_manager: ArtifactManager::new(&work_dir),
            interpreter,
            graph: None,
            history: Arc::new(RwLock::new(ExecutionHistory::new())),
        }
    }

    /// Set the action graph for validation
    pub fn with_graph(mut self, graph: ActionGraph) -> Self {
        let graph = Arc::new(graph);
        self.graph = Some(Arc::clone(&graph));
        self.interpreter = ActionInterpreter::new().with_graph(
            // Create a new graph for the interpreter
            // In real impl, would share or clone
            ActionGraph::from_thesaurus(
                crate::knowledge_graph::ThesaurusBuilder::new(&self.config.work_directory)
            )
        );
        self
    }

    /// Execute a workflow step
    pub async fn execute_step(
        &self,
        step: &Step,
        job_id: &str,
        step_index: usize,
        env: &AHashMap<String, String>,
    ) -> RunnerResult<StepResult> {
        let step_id = step.id.clone().unwrap_or_else(|| format!("step-{}", step_index));
        log::info!("Executing step: {} ({})", step.name.as_deref().unwrap_or(&step_id), step_id);

        // Interpret the action
        let interpreted = self.interpreter.interpret_step(step).await?;
        log::debug!("Interpreted: {:?}", interpreted);

        // Check for cached result
        if interpreted.cacheable {
            if let Some(cached) = self.check_cache(&interpreted).await {
                log::info!("Cache hit for step: {}", step_id);
                return Ok(cached);
            }
        }

        // Validate against knowledge graph
        if let Some(graph) = &self.graph {
            graph.validate_interpretation(&interpreted)?;
        }

        // Merge environment variables
        let mut full_env = env.clone();
        if let Some(step_env) = &step.env {
            full_env.extend(step_env.clone());
        }

        // Execute in VM
        let start_time = std::time::Instant::now();
        let vm_result = self.vm_executor
            .execute(&interpreted.commands, &full_env, step.timeout_minutes)
            .await?;
        let duration_ms = start_time.elapsed().as_millis() as u64;

        // Collect artifacts
        let artifacts = self.artifact_manager.collect_artifacts(&interpreted.produces).await?;

        // Build result
        let result = StepResult {
            step_id: step_id.clone(),
            exit_code: vm_result.exit_code,
            stdout: vm_result.stdout,
            stderr: vm_result.stderr,
            duration_ms,
            outputs: vm_result.outputs,
            artifacts,
            vm_snapshot_id: vm_result.snapshot_id,
        };

        // Record in history
        self.history.write().await.record_step(
            job_id,
            step_index,
            &interpreted,
            &result,
        ).await?;

        // Cache successful result if cacheable
        if result.exit_code == 0 && interpreted.cacheable {
            self.cache_result(&interpreted, &result).await?;
        }

        Ok(result)
    }

    /// Execute all steps in a job
    pub async fn execute_job(
        &self,
        steps: &[Step],
        job_id: &str,
        env: &AHashMap<String, String>,
    ) -> RunnerResult<Vec<StepResult>> {
        let mut results = Vec::new();

        for (idx, step) in steps.iter().enumerate() {
            // Check condition
            if let Some(condition) = &step.condition {
                if !self.evaluate_condition(condition, &results) {
                    log::info!("Skipping step {} due to condition: {}", idx, condition);
                    continue;
                }
            }

            let result = self.execute_step(step, job_id, idx, env).await;

            match result {
                Ok(r) => {
                    let success = r.exit_code == 0;
                    results.push(r);

                    if !success && !step.continue_on_error.unwrap_or(false) {
                        log::error!("Step {} failed, stopping job", idx);
                        break;
                    }
                }
                Err(e) => {
                    if step.continue_on_error.unwrap_or(false) {
                        log::warn!("Step {} failed but continuing: {}", idx, e);
                        results.push(StepResult {
                            step_id: step.id.clone().unwrap_or_else(|| format!("step-{}", idx)),
                            exit_code: 1,
                            stdout: String::new(),
                            stderr: e.to_string(),
                            duration_ms: 0,
                            outputs: AHashMap::new(),
                            artifacts: Vec::new(),
                            vm_snapshot_id: None,
                        });
                    } else {
                        return Err(e);
                    }
                }
            }
        }

        Ok(results)
    }

    /// Check cache for a step result
    async fn check_cache(&self, _interpreted: &InterpretedAction) -> Option<StepResult> {
        // In real implementation, would check:
        // 1. VM snapshot cache
        // 2. Artifact cache
        // 3. History for identical executions
        None
    }

    /// Cache a successful result
    async fn cache_result(&self, _interpreted: &InterpretedAction, _result: &StepResult) -> RunnerResult<()> {
        // In real implementation, would:
        // 1. Save VM snapshot
        // 2. Cache artifacts
        // 3. Store execution record
        Ok(())
    }

    /// Evaluate a step condition
    fn evaluate_condition(&self, condition: &str, previous_results: &[StepResult]) -> bool {
        // Simple condition evaluation
        // In real implementation, would use expression parser

        if condition.contains("success()") {
            return previous_results.iter().all(|r| r.exit_code == 0);
        }

        if condition.contains("failure()") {
            return previous_results.iter().any(|r| r.exit_code != 0);
        }

        if condition.contains("always()") {
            return true;
        }

        if condition.contains("cancelled()") {
            return false; // Would need cancellation state
        }

        // Default to true for unknown conditions
        true
    }

    /// Get execution history
    pub async fn history(&self) -> ExecutionHistory {
        self.history.read().await.clone()
    }

    /// Shutdown executor and cleanup resources
    pub async fn shutdown(&self) -> RunnerResult<()> {
        self.vm_executor.shutdown().await?;
        Ok(())
    }
}
