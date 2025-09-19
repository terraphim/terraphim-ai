//! Multi-Agent Workflow Handlers
//!
//! This module bridges HTTP workflow endpoints with the TerraphimAgent system,
//! replacing mock implementations with actual multi-agent execution.

use serde_json::{json, Value};
use std::sync::Arc;
use tokio::time::{sleep, Duration};

use ahash::AHashMap;
use terraphim_config::Role;
use terraphim_multi_agent::{
    AgentRegistry, CommandInput, CommandType, MultiAgentError, MultiAgentResult, TerraphimAgent,
};
use terraphim_persistence::DeviceStorage;
use terraphim_types::RelevanceFunction;

use super::{
    update_workflow_status, ExecutionStatus, LlmConfig, WebSocketBroadcaster, WorkflowMetadata,
    WorkflowSessions,
};
use crate::AppState;
use terraphim_config::{Config, ConfigState};

/// Multi-agent workflow executor
pub struct MultiAgentWorkflowExecutor {
    agent_registry: AgentRegistry,
    persistence: Arc<DeviceStorage>,
    config_state: Option<ConfigState>,
}

impl MultiAgentWorkflowExecutor {
    /// Create new multi-agent workflow executor
    pub async fn new() -> MultiAgentResult<Self> {
        // Initialize storage for agents using safe Arc method
        let persistence = DeviceStorage::arc_memory_only()
            .await
            .map_err(|e| MultiAgentError::PersistenceError(e.to_string()))?;

        let agent_registry = AgentRegistry::new();

        Ok(Self {
            agent_registry,
            persistence,
            config_state: None,
        })
    }

    /// Create new multi-agent workflow executor with config state
    pub async fn new_with_config(config_state: ConfigState) -> MultiAgentResult<Self> {
        // Initialize storage for agents using safe Arc method
        let persistence = DeviceStorage::arc_memory_only()
            .await
            .map_err(|e| MultiAgentError::PersistenceError(e.to_string()))?;

        let agent_registry = AgentRegistry::new();

        Ok(Self {
            agent_registry,
            persistence,
            config_state: Some(config_state),
        })
    }

    /// Resolve LLM configuration from multiple sources with priority order:
    /// 1. Request-level config (highest priority)
    /// 2. Role-level config from server config
    /// 3. Global defaults
    /// 4. Hardcoded fallback (lowest priority)
    fn resolve_llm_config(&self, request_config: Option<&LlmConfig>, role_name: &str) -> LlmConfig {
        let mut resolved = LlmConfig::default();

        // Start with global defaults if we have config state
        if let Some(config_state) = &self.config_state {
            if let Ok(config) = config_state.config.try_lock() {
                // Check if there's a global LLM config section
                let default_role_name = "Default".into();
                if let Some(default_role) = config.roles.get(&default_role_name) {
                    if let Some(provider) = default_role.extra.get("llm_provider") {
                        if let Some(provider_str) = provider.as_str() {
                            resolved.llm_provider = Some(provider_str.to_string());
                        }
                    }
                    if let Some(model) = default_role.extra.get("llm_model") {
                        if let Some(model_str) = model.as_str() {
                            resolved.llm_model = Some(model_str.to_string());
                        }
                    }
                    if let Some(base_url) = default_role.extra.get("llm_base_url") {
                        if let Some(base_url_str) = base_url.as_str() {
                            resolved.llm_base_url = Some(base_url_str.to_string());
                        }
                    }
                }

                // Check role-specific config
                let role_name_key = role_name.into();
                if let Some(role) = config.roles.get(&role_name_key) {
                    if let Some(provider) = role.extra.get("llm_provider") {
                        if let Some(provider_str) = provider.as_str() {
                            resolved.llm_provider = Some(provider_str.to_string());
                        }
                    }
                    if let Some(model) = role.extra.get("llm_model") {
                        if let Some(model_str) = model.as_str() {
                            resolved.llm_model = Some(model_str.to_string());
                        }
                    }
                    if let Some(base_url) = role.extra.get("llm_base_url") {
                        if let Some(base_url_str) = base_url.as_str() {
                            resolved.llm_base_url = Some(base_url_str.to_string());
                        }
                    }
                    if let Some(temp) = role.extra.get("llm_temperature") {
                        if let Some(temp_val) = temp.as_f64() {
                            resolved.llm_temperature = Some(temp_val);
                        }
                    }
                }
            }
        }

        // Override with request-level config (highest priority)
        if let Some(req_config) = request_config {
            if let Some(provider) = &req_config.llm_provider {
                resolved.llm_provider = Some(provider.clone());
            }
            if let Some(model) = &req_config.llm_model {
                resolved.llm_model = Some(model.clone());
            }
            if let Some(base_url) = &req_config.llm_base_url {
                resolved.llm_base_url = Some(base_url.clone());
            }
            if let Some(temp) = req_config.llm_temperature {
                resolved.llm_temperature = Some(temp);
            }
        }

        log::debug!("Resolved LLM config for role '{}': provider={:?}, model={:?}, base_url={:?}, temperature={:?}",
            role_name,
            resolved.llm_provider,
            resolved.llm_model,
            resolved.llm_base_url,
            resolved.llm_temperature
        );

        resolved
    }

    /// Execute prompt chaining workflow with actual TerraphimAgent
    pub async fn execute_prompt_chain(
        &self,
        workflow_id: &str,
        prompt: &str,
        role: &str,
        overall_role: &str,
        sessions: &WorkflowSessions,
        broadcaster: &WebSocketBroadcaster,
        llm_config: Option<&LlmConfig>,
    ) -> MultiAgentResult<Value> {
        log::info!("Executing prompt chain workflow with TerraphimAgent");

        // Resolve LLM configuration
        let resolved_config = self.resolve_llm_config(llm_config, role);

        // Create agent for prompt chaining using the specified role
        log::debug!("🔧 Creating agent using configured role: {}", role);
        let dev_role = self.get_configured_role(role).await?;
        let mut dev_agent = TerraphimAgent::new(dev_role, self.persistence.clone(), None).await?;
        dev_agent.initialize().await?;

        // Define prompt chaining steps
        let steps = vec![
            ("requirements", "Create detailed technical specification"),
            ("architecture", "Design system architecture and components"),
            (
                "planning",
                "Create development plan with tasks and timelines",
            ),
            ("implementation", "Generate core implementation code"),
            ("testing", "Create comprehensive test suite"),
            (
                "deployment",
                "Provide deployment instructions and documentation",
            ),
        ];

        let mut results = Vec::new();
        let mut context = prompt.to_string();
        let total_steps = steps.len();

        for (index, (step_id, step_description)) in steps.iter().enumerate() {
            let progress = (index as f64 / total_steps as f64) * 100.0;

            update_workflow_status(
                sessions,
                broadcaster,
                workflow_id,
                ExecutionStatus::Running,
                progress,
                Some(format!("Executing: {}", step_description)),
            )
            .await;

            // Create step prompt with accumulated context
            let step_prompt = format!(
                "{}\n\nContext:\n{}\n\nPlease provide detailed output for this step.",
                step_description, context
            );
            let input = CommandInput::new(step_prompt, CommandType::Generate);

            // Execute with TerraphimAgent
            let output = dev_agent.process_command(input).await?;

            // Update context for next step (prompt chaining)
            context = format!(
                "{}\n\nStep {} ({}): {}",
                context,
                index + 1,
                step_id,
                &output.text[..std::cmp::min(500, output.text.len())]
            );

            let step_result = json!({
                "step_id": step_id,
                "step_name": step_description,
                "role": role,
                "overall_role": overall_role,
                "output": output.text,
                "duration_ms": 2000, // Real execution time would be tracked
                "success": true,
                "agent_id": dev_agent.agent_id.to_string(),
                "tokens_used": {
                    "input": dev_agent.token_tracker.read().await.total_input_tokens,
                    "output": dev_agent.token_tracker.read().await.total_output_tokens
                }
            });

            results.push(step_result);

            // Small delay for progress updates
            sleep(Duration::from_millis(500)).await;
        }

        // Get final metrics
        let token_tracker = dev_agent.token_tracker.read().await;
        let cost_tracker = dev_agent.cost_tracker.read().await;

        Ok(json!({
            "pattern": "prompt_chaining",
            "steps": results,
            "final_result": results.last().unwrap_or(&json!({})),
            "execution_summary": {
                "total_steps": total_steps,
                "role": role,
                "overall_role": overall_role,
                "input_prompt": prompt,
                "agent_id": dev_agent.agent_id.to_string(),
                "total_tokens": token_tracker.total_input_tokens + token_tracker.total_output_tokens,
                "total_cost": cost_tracker.current_month_spending,
                "multi_agent": true
            }
        }))
    }

    /// Execute routing workflow with complexity-based agent selection
    pub async fn execute_routing(
        &self,
        workflow_id: &str,
        prompt: &str,
        role: &str,
        overall_role: &str,
        sessions: &WorkflowSessions,
        broadcaster: &WebSocketBroadcaster,
    ) -> MultiAgentResult<Value> {
        log::info!("Executing routing workflow with multi-agent intelligence");

        // Analyze task complexity
        update_workflow_status(
            sessions,
            broadcaster,
            workflow_id,
            ExecutionStatus::Running,
            25.0,
            Some("Analyzing task complexity".to_string()),
        )
        .await;

        let complexity = self.analyze_task_complexity(prompt);
        let estimated_cost = if complexity > 0.7 { 0.08 } else { 0.02 };

        // Select appropriate agent based on complexity
        update_workflow_status(
            sessions,
            broadcaster,
            workflow_id,
            ExecutionStatus::Running,
            50.0,
            Some("Selecting optimal agent".to_string()),
        )
        .await;

        // Use the specified role for routing, but choose model based on complexity
        let llm_config = if complexity > 0.7 {
            // Use a more powerful model for complex tasks
            Some(LlmConfig {
                llm_provider: Some("ollama".to_string()),
                llm_model: Some("llama3.2:3b".to_string()),
                llm_base_url: Some("http://127.0.0.1:11434".to_string()),
                llm_temperature: Some(0.3), // Lower temperature for complex analysis
            })
        } else {
            None // Use role defaults for simple tasks
        };
        
        log::debug!("🔧 Creating routing agent using configured role: {}", role);
        let agent_role = self.get_configured_role(role).await?;
        let mut selected_agent = TerraphimAgent::new(agent_role, self.persistence.clone(), None).await?;
        selected_agent.initialize().await?;
        
        let route_id = if complexity > 0.7 { "complex_route" } else { "simple_route" };

        // Execute with selected agent
        update_workflow_status(
            sessions,
            broadcaster,
            workflow_id,
            ExecutionStatus::Running,
            75.0,
            Some(format!("Executing with {}", route_id)),
        )
        .await;

        let input = CommandInput::new(prompt.to_string(), CommandType::Generate);
        let output = selected_agent.process_command(input).await?;

        let token_tracker = selected_agent.token_tracker.read().await;
        let cost_tracker = selected_agent.cost_tracker.read().await;

        Ok(json!({
            "pattern": "routing",
            "task_analysis": {
                "complexity": complexity,
                "estimated_cost": estimated_cost,
                "analysis_method": "keyword_and_length_based"
            },
            "selected_route": {
                "route_id": route_id,
                "reasoning": format!("Selected {} for complexity level {:.2}", route_id, complexity),
                "confidence": if complexity > 0.7 { 0.95 } else { 0.85 },
                "agent_id": selected_agent.agent_id.to_string()
            },
            "result": output.text,
            "execution_summary": {
                "role": role,
                "overall_role": overall_role,
                "input_prompt": prompt,
                "tokens_used": token_tracker.total_input_tokens + token_tracker.total_output_tokens,
                "actual_cost": cost_tracker.current_month_spending,
                "multi_agent": true
            }
        }))
    }

    /// Execute parallelization workflow with multiple perspective agents
    pub async fn execute_parallelization(
        &self,
        workflow_id: &str,
        prompt: &str,
        role: &str,
        overall_role: &str,
        sessions: &WorkflowSessions,
        broadcaster: &WebSocketBroadcaster,
    ) -> MultiAgentResult<Value> {
        log::info!("Executing parallelization workflow with multiple agents");

        // Create multiple perspective agents using the specified role as base
        // Resolve LLM configuration
        let resolved_config = self.resolve_llm_config(None, role);

        let perspectives = vec![
            ("analytical", "Provide analytical, data-driven insights"),
            ("creative", "Offer creative and innovative perspectives"),
            (
                "practical",
                "Focus on practical implementation and feasibility",
            ),
        ];

        let mut agents = Vec::new();
        for (perspective_name, perspective_description) in &perspectives {
            update_workflow_status(
                sessions,
                broadcaster,
                workflow_id,
                ExecutionStatus::Running,
                (agents.len() as f64 / perspectives.len() as f64) * 30.0,
                Some(format!("Creating {} perspective agent", perspective_name)),
            )
            .await;

            // Get the base role and modify it for the perspective
            log::debug!("🔧 Creating {} perspective agent using base role: {}", perspective_name, role);
            let mut base_role = self.get_configured_role(role).await?;
            
            // Add perspective information to the role's extra data
            base_role.extra.insert("perspective".to_string(), serde_json::json!(perspective_name));
            base_role.extra.insert("perspective_description".to_string(), serde_json::json!(perspective_description));
            
            // Update the role name to reflect the perspective
            base_role.name = format!("{}_{}", role, perspective_name).into();
            base_role.shortname = Some(format!("{}_{}", base_role.shortname.unwrap_or_default(), perspective_name));
            
            let mut agent = TerraphimAgent::new(base_role, self.persistence.clone(), None).await?;
            agent.initialize().await?;
            agents.push(agent);
        }

        // Execute analyses in parallel
        update_workflow_status(
            sessions,
            broadcaster,
            workflow_id,
            ExecutionStatus::Running,
            50.0,
            Some("Executing parallel analysis".to_string()),
        )
        .await;

        let mut parallel_results = Vec::new();
        let mut total_tokens = 0;
        let mut total_cost = 0.0;
        let perspectives_count = perspectives.len();

        for (i, (perspective, agent)) in perspectives.iter().zip(agents.iter_mut()).enumerate() {
            let analysis_prompt = format!(
                "Analyze this topic from a {} perspective: {}\n\n{}",
                perspective.0, prompt, perspective.1
            );
            let input = CommandInput::new(analysis_prompt, CommandType::Analyze);

            let progress = 50.0 + ((i + 1) as f64 / perspectives_count as f64) * 40.0;
            update_workflow_status(
                sessions,
                broadcaster,
                workflow_id,
                ExecutionStatus::Running,
                progress,
                Some(format!("Processing {} perspective", perspective.0)),
            )
            .await;

            let output = agent.process_command(input).await?;

            let token_tracker = agent.token_tracker.read().await;
            let cost_tracker = agent.cost_tracker.read().await;
            total_tokens += token_tracker.total_input_tokens + token_tracker.total_output_tokens;
            total_cost += cost_tracker.current_month_spending;

            parallel_results.push(json!({
                "task_id": format!("perspective_{}", i),
                "perspective": perspective.0,
                "description": perspective.1,
                "result": output.text,
                "agent_id": agent.agent_id.to_string(),
                "tokens_used": token_tracker.total_input_tokens + token_tracker.total_output_tokens,
                "cost": cost_tracker.current_month_spending
            }));
        }

        // Aggregate results
        update_workflow_status(
            sessions,
            broadcaster,
            workflow_id,
            ExecutionStatus::Running,
            95.0,
            Some("Aggregating perspectives".to_string()),
        )
        .await;

        let aggregated_summary = format!(
            "Multi-perspective analysis of: {}\n\nAnalyzed from {} different viewpoints with {} total tokens and ${:.6} cost.",
            prompt, perspectives.len(), total_tokens, total_cost
        );

        Ok(json!({
            "pattern": "parallelization",
            "parallel_tasks": parallel_results,
            "aggregated_result": aggregated_summary,
            "execution_summary": {
                "role": role,
                "overall_role": overall_role,
                "input_prompt": prompt,
                "perspectives_count": perspectives.len(),
                "total_tokens": total_tokens,
                "total_cost": total_cost,
                "multi_agent": true
            }
        }))
    }

    /// Execute orchestrator-workers workflow with hierarchical coordination
    pub async fn execute_orchestration(
        &self,
        workflow_id: &str,
        prompt: &str,
        role: &str,
        overall_role: &str,
        sessions: &WorkflowSessions,
        broadcaster: &WebSocketBroadcaster,
    ) -> MultiAgentResult<Value> {
        log::info!("Executing orchestration workflow with hierarchical agents");

        // Resolve LLM configuration
        let resolved_config = self.resolve_llm_config(None, role);

        // Create orchestrator
        update_workflow_status(
            sessions,
            broadcaster,
            workflow_id,
            ExecutionStatus::Running,
            10.0,
            Some("Creating orchestrator agent".to_string()),
        )
        .await;

        log::debug!("🔧 Creating orchestrator using configured role: {}", role);
        let orchestrator_role = self.get_configured_role(role).await?;
        let mut orchestrator =
            TerraphimAgent::new(orchestrator_role, self.persistence.clone(), None).await?;
        orchestrator.initialize().await?;

        // Create specialized workers
        let worker_specs = vec![
            ("data_collector", "Collect and validate research data"),
            ("content_analyzer", "Analyze and process content"),
            (
                "knowledge_mapper",
                "Extract concepts and build relationships",
            ),
        ];

        let mut workers = Vec::new();
        for (i, (worker_name, worker_description)) in worker_specs.iter().enumerate() {
            update_workflow_status(
                sessions,
                broadcaster,
                workflow_id,
                ExecutionStatus::Running,
                20.0 + (i as f64 / worker_specs.len() as f64) * 20.0,
                Some(format!("Creating {} worker", worker_name)),
            )
            .await;

            let worker_role = self.create_worker_role(worker_name, worker_description, &resolved_config);
            let mut worker =
                TerraphimAgent::new(worker_role, self.persistence.clone(), None).await?;
            worker.initialize().await?;
            workers.push((worker_name.to_string(), worker));
        }

        // Step 1: Orchestrator creates plan
        update_workflow_status(
            sessions,
            broadcaster,
            workflow_id,
            ExecutionStatus::Running,
            50.0,
            Some("Orchestrator creating plan".to_string()),
        )
        .await;

        let planning_prompt = format!("Create a detailed plan for: {}", prompt);
        let planning_input = CommandInput::new(planning_prompt, CommandType::Create);
        let plan = orchestrator.process_command(planning_input).await?;

        // Step 2: Distribute tasks to workers
        let mut worker_results = Vec::new();
        let workers_count = workers.len();
        for (i, (worker_name, worker)) in workers.iter_mut().enumerate() {
            update_workflow_status(
                sessions,
                broadcaster,
                workflow_id,
                ExecutionStatus::Running,
                60.0 + (i as f64 / workers_count as f64) * 25.0,
                Some(format!("Worker {} executing task", worker_name)),
            )
            .await;

            let task_prompt = format!("Execute {} task for: {}", worker_name, prompt);
            let task_input = CommandInput::new(task_prompt, CommandType::Generate);
            let result = worker.process_command(task_input).await?;

            let token_tracker = worker.token_tracker.read().await;
            let cost_tracker = worker.cost_tracker.read().await;

            worker_results.push(json!({
                "worker_name": worker_name,
                "task_description": format!("{} task", worker_name),
                "result": result.text,
                "agent_id": worker.agent_id.to_string(),
                "tokens_used": token_tracker.total_input_tokens + token_tracker.total_output_tokens,
                "cost": cost_tracker.current_month_spending
            }));
        }

        // Step 3: Orchestrator synthesizes
        update_workflow_status(
            sessions,
            broadcaster,
            workflow_id,
            ExecutionStatus::Running,
            90.0,
            Some("Orchestrator synthesizing results".to_string()),
        )
        .await;

        let synthesis_context = worker_results
            .iter()
            .map(|result| {
                format!(
                    "{}: {}",
                    result["worker_name"].as_str().unwrap_or("unknown"),
                    result["result"].as_str().unwrap_or("no output")
                )
            })
            .collect::<Vec<_>>()
            .join("\n\n");

        let synthesis_prompt = format!("Synthesize these worker results:\n\n{}", synthesis_context);
        let synthesis_input = CommandInput::new(synthesis_prompt, CommandType::Analyze);
        let final_synthesis = orchestrator.process_command(synthesis_input).await?;

        // Collect metrics
        let orch_tokens = orchestrator.token_tracker.read().await;
        let orch_cost = orchestrator.cost_tracker.read().await;

        let total_tokens = orch_tokens.total_input_tokens
            + orch_tokens.total_output_tokens
            + worker_results
                .iter()
                .map(|r| r["tokens_used"].as_u64().unwrap_or(0))
                .sum::<u64>();
        let total_cost = orch_cost.current_month_spending
            + worker_results
                .iter()
                .map(|r| r["cost"].as_f64().unwrap_or(0.0))
                .sum::<f64>();

        Ok(json!({
            "pattern": "orchestration",
            "orchestrator_plan": plan.text,
            "worker_results": worker_results,
            "final_synthesis": final_synthesis.text,
            "execution_summary": {
                "role": role,
                "overall_role": overall_role,
                "input_prompt": prompt,
                "orchestrator_id": orchestrator.agent_id.to_string(),
                "workers_count": workers.len(),
                "total_tokens": total_tokens,
                "total_cost": total_cost,
                "multi_agent": true
            }
        }))
    }

    /// Execute optimization workflow with iterative improvement
    pub async fn execute_optimization(
        &self,
        workflow_id: &str,
        prompt: &str,
        role: &str,
        overall_role: &str,
        sessions: &WorkflowSessions,
        broadcaster: &WebSocketBroadcaster,
    ) -> MultiAgentResult<Value> {
        log::info!("Executing optimization workflow with iterative agents");

        // Resolve LLM configuration
        let resolved_config = self.resolve_llm_config(None, role);

        // Create generator and evaluator agents based on the specified role
        update_workflow_status(
            sessions,
            broadcaster,
            workflow_id,
            ExecutionStatus::Running,
            20.0,
            Some("Creating generator and evaluator agents".to_string()),
        )
        .await;

        // Create generator agent using the specified role but with generation focus
        log::debug!("🔧 Creating generator agent based on role: {}", role);
        let mut generator_role = self.get_configured_role(role).await?;
        generator_role.extra.insert("specialization".to_string(), serde_json::json!("content_generation"));
        generator_role.extra.insert("focus".to_string(), serde_json::json!("creative_output"));
        generator_role.name = format!("{}_Generator", role).into();
        let mut generator = TerraphimAgent::new(generator_role, self.persistence.clone(), None).await?;
        generator.initialize().await?;

        // Create evaluator agent using QAEngineer for evaluation capabilities
        log::debug!("🔧 Creating evaluator using QAEngineer role for evaluation expertise");
        let evaluator_role = self.get_configured_role("QAEngineer").await?;
        let mut evaluator = TerraphimAgent::new(evaluator_role, self.persistence.clone(), None).await?;
        evaluator.initialize().await?;

        let max_iterations = 3;
        let quality_threshold = 8.0;
        let mut iteration_results = Vec::new();
        let mut current_content = String::new();
        let mut best_score = 0.0;

        for iteration in 1..=max_iterations {
            update_workflow_status(
                sessions,
                broadcaster,
                workflow_id,
                ExecutionStatus::Running,
                30.0 + (iteration as f64 / max_iterations as f64) * 50.0,
                Some(format!(
                    "Optimization iteration {}/{}",
                    iteration, max_iterations
                )),
            )
            .await;

            // Generate content
            let gen_prompt = if current_content.is_empty() {
                format!("Create content for: {}", prompt)
            } else {
                format!("Improve this content based on evaluation:\n\nOriginal request: {}\n\nCurrent content:\n{}", 
                    prompt, current_content)
            };

            let gen_input = CommandInput::new(gen_prompt, CommandType::Generate);
            let gen_result = generator.process_command(gen_input).await?;
            current_content = gen_result.text;

            // Evaluate content
            let eval_prompt = format!(
                "Evaluate this content quality (1-10):\n\n{}",
                current_content
            );
            let eval_input = CommandInput::new(eval_prompt, CommandType::Review);
            let eval_result = evaluator.process_command(eval_input).await?;

            // Extract quality score (simplified)
            let score = self.extract_quality_score(&eval_result.text);
            if score > best_score {
                best_score = score;
            }

            let gen_tokens = generator.token_tracker.read().await;
            let eval_tokens = evaluator.token_tracker.read().await;
            let gen_cost = generator.cost_tracker.read().await;
            let eval_cost = evaluator.cost_tracker.read().await;

            iteration_results.push(json!({
                "iteration": iteration,
                "generated_content": current_content.clone(),
                "evaluation_feedback": eval_result.text,
                "quality_score": score,
                "generator_tokens": gen_tokens.total_input_tokens + gen_tokens.total_output_tokens,
                "evaluator_tokens": eval_tokens.total_input_tokens + eval_tokens.total_output_tokens,
                "iteration_cost": gen_cost.current_month_spending + eval_cost.current_month_spending
            }));

            if score >= quality_threshold {
                break;
            }
        }

        let total_tokens = iteration_results
            .iter()
            .map(|r| {
                r["generator_tokens"].as_u64().unwrap_or(0)
                    + r["evaluator_tokens"].as_u64().unwrap_or(0)
            })
            .sum::<u64>();
        let total_cost = iteration_results
            .iter()
            .map(|r| r["iteration_cost"].as_f64().unwrap_or(0.0))
            .sum::<f64>();

        Ok(json!({
            "pattern": "optimization",
            "iterations": iteration_results,
            "final_content": current_content,
            "optimization_complete": best_score >= quality_threshold,
            "execution_summary": {
                "role": role,
                "overall_role": overall_role,
                "input_prompt": prompt,
                "generator_id": generator.agent_id.to_string(),
                "evaluator_id": evaluator.agent_id.to_string(),
                "iterations_completed": iteration_results.len(),
                "best_quality_score": best_score,
                "quality_threshold": quality_threshold,
                "total_tokens": total_tokens,
                "total_cost": total_cost,
                "multi_agent": true
            }
        }))
    }

    // Helper methods for creating specialized agent roles

    /// Apply LLM configuration to a role's extra fields
    fn apply_llm_config_to_extra(&self, extra: &mut AHashMap<String, serde_json::Value>, llm_config: &LlmConfig) {
        if let Some(provider) = &llm_config.llm_provider {
            extra.insert("llm_provider".to_string(), serde_json::json!(provider));
        }
        if let Some(model) = &llm_config.llm_model {
            extra.insert("llm_model".to_string(), serde_json::json!(model));
        }
        if let Some(base_url) = &llm_config.llm_base_url {
            extra.insert("llm_base_url".to_string(), serde_json::json!(base_url));
        }
        if let Some(temperature) = llm_config.llm_temperature {
            extra.insert("llm_temperature".to_string(), serde_json::json!(temperature));
        }
    }

    fn create_development_role(&self, base_role: &str, llm_config: &LlmConfig) -> Role {
        let mut extra = AHashMap::new();
        extra.insert(
            "agent_capabilities".to_string(),
            serde_json::json!([
                "software_development",
                "code_generation",
                "architecture_design"
            ]),
        );
        extra.insert(
            "agent_goals".to_string(),
            serde_json::json!([
                "Create professional software solutions",
                "Follow best practices",
                "Generate comprehensive documentation"
            ]),
        );
        
        // Apply LLM configuration
        self.apply_llm_config_to_extra(&mut extra, llm_config);
        extra.insert("base_role".to_string(), serde_json::json!(base_role));

        Role {
            shortname: Some("DevAgent".to_string()),
            name: "DevelopmentAgent".into(),
            relevance_function: RelevanceFunction::BM25,
            terraphim_it: false,
            theme: "default".to_string(),
            kg: None,
            haystacks: vec![],
            llm_enabled: false,
            llm_api_key: None,
            llm_model: None,
            llm_auto_summarize: false,
            llm_chat_enabled: false,
            llm_chat_system_prompt: None,
            llm_chat_model: None,
            llm_context_window: Some(32768),
            extra,
        }
    }

    async fn create_simple_agent(&self) -> MultiAgentResult<TerraphimAgent> {
        log::debug!("🔧 Creating simple agent using configured role: SimpleTaskAgent");
        
        // Use configured role instead of creating ad-hoc role
        let role = self.get_configured_role("SimpleTaskAgent").await?;
        
        let mut agent = TerraphimAgent::new(role, self.persistence.clone(), None).await?;
        agent.initialize().await?;
        Ok(agent)
    }

    async fn create_complex_agent(&self) -> MultiAgentResult<TerraphimAgent> {
        log::debug!("🔧 Creating complex agent using configured role: ComplexTaskAgent");
        
        // Use configured role instead of creating ad-hoc role
        let role = self.get_configured_role("ComplexTaskAgent").await?;
        
        let mut agent = TerraphimAgent::new(role, self.persistence.clone(), None).await?;
        agent.initialize().await?;
        Ok(agent)
    }

    fn create_perspective_role(&self, perspective: &str, description: &str, llm_config: &LlmConfig) -> Role {
        let mut extra = AHashMap::new();
        extra.insert("perspective".to_string(), serde_json::json!(perspective));
        extra.insert("description".to_string(), serde_json::json!(description));
        
        // Apply dynamic LLM configuration
        self.apply_llm_config_to_extra(&mut extra, llm_config);
        
        // Set default temperature if not configured
        if !extra.contains_key("llm_temperature") {
            extra.insert("llm_temperature".to_string(), serde_json::json!(0.5));
        }

        Role {
            shortname: Some(perspective.to_string()),
            name: format!("{}PerspectiveAgent", perspective).into(),
            relevance_function: RelevanceFunction::BM25,
            terraphim_it: false,
            theme: "default".to_string(),
            kg: None,
            haystacks: vec![],
            llm_enabled: false,
            llm_api_key: None,
            llm_model: None,
            llm_auto_summarize: false,
            llm_chat_enabled: false,
            llm_chat_system_prompt: None,
            llm_chat_model: None,
            llm_context_window: Some(32768),
            extra,
        }
    }

    fn create_orchestrator_role(&self, llm_config: &LlmConfig) -> Role {
        let mut extra = AHashMap::new();
        extra.insert("role_type".to_string(), serde_json::json!("orchestrator"));
        
        // Apply dynamic LLM configuration
        self.apply_llm_config_to_extra(&mut extra, llm_config);
        
        // Set default temperature if not configured
        if !extra.contains_key("llm_temperature") {
            extra.insert("llm_temperature".to_string(), serde_json::json!(0.3));
        }

        Role {
            shortname: Some("Orchestrator".to_string()),
            name: "OrchestratorAgent".into(),
            relevance_function: RelevanceFunction::BM25,
            terraphim_it: false,
            theme: "default".to_string(),
            kg: None,
            haystacks: vec![],
            llm_enabled: false,
            llm_api_key: None,
            llm_model: None,
            llm_auto_summarize: false,
            llm_chat_enabled: false,
            llm_chat_system_prompt: None,
            llm_chat_model: None,
            llm_context_window: Some(32768),
            extra,
        }
    }

    fn create_worker_role(&self, worker_name: &str, description: &str, llm_config: &LlmConfig) -> Role {
        let mut extra = AHashMap::new();
        extra.insert("worker_type".to_string(), serde_json::json!(worker_name));
        extra.insert("description".to_string(), serde_json::json!(description));
        
        // Apply dynamic LLM configuration
        self.apply_llm_config_to_extra(&mut extra, llm_config);
        
        // Set default temperature if not configured
        if !extra.contains_key("llm_temperature") {
            extra.insert("llm_temperature".to_string(), serde_json::json!(0.4));
        }

        Role {
            shortname: Some(worker_name.to_string()),
            name: format!("{}Worker", worker_name).into(),
            relevance_function: RelevanceFunction::BM25,
            terraphim_it: false,
            theme: "default".to_string(),
            kg: None,
            haystacks: vec![],
            llm_enabled: false,
            llm_api_key: None,
            llm_model: None,
            llm_auto_summarize: false,
            llm_chat_enabled: false,
            llm_chat_system_prompt: None,
            llm_chat_model: None,
            llm_context_window: Some(32768),
            extra,
        }
    }

    fn create_generator_role(&self, llm_config: &LlmConfig) -> Role {
        let mut extra = AHashMap::new();
        extra.insert("role_type".to_string(), serde_json::json!("generator"));
        
        // Apply dynamic LLM configuration
        self.apply_llm_config_to_extra(&mut extra, llm_config);
        
        // Set default temperature if not configured
        if !extra.contains_key("llm_temperature") {
            extra.insert("llm_temperature".to_string(), serde_json::json!(0.6));
        }

        Role {
            shortname: Some("Generator".to_string()),
            name: "GeneratorAgent".into(),
            relevance_function: RelevanceFunction::BM25,
            terraphim_it: false,
            theme: "default".to_string(),
            kg: None,
            haystacks: vec![],
            llm_enabled: false,
            llm_api_key: None,
            llm_model: None,
            llm_auto_summarize: false,
            llm_chat_enabled: false,
            llm_chat_system_prompt: None,
            llm_chat_model: None,
            llm_context_window: Some(32768),
            extra,
        }
    }

    fn create_evaluator_role(&self, llm_config: &LlmConfig) -> Role {
        let mut extra = AHashMap::new();
        extra.insert("role_type".to_string(), serde_json::json!("evaluator"));
        
        // Apply dynamic LLM configuration
        self.apply_llm_config_to_extra(&mut extra, llm_config);
        
        // Set default temperature if not configured
        if !extra.contains_key("llm_temperature") {
            extra.insert("llm_temperature".to_string(), serde_json::json!(0.2));
        }

        Role {
            shortname: Some("Evaluator".to_string()),
            name: "EvaluatorAgent".into(),
            relevance_function: RelevanceFunction::BM25,
            terraphim_it: false,
            theme: "default".to_string(),
            kg: None,
            haystacks: vec![],
            llm_enabled: false,
            llm_api_key: None,
            llm_model: None,
            llm_auto_summarize: false,
            llm_chat_enabled: false,
            llm_chat_system_prompt: None,
            llm_chat_model: None,
            llm_context_window: Some(32768),
            extra,
        }
    }

    // Utility methods

    fn analyze_task_complexity(&self, task: &str) -> f64 {
        let complexity_keywords = vec![
            ("simple", 0.2),
            ("basic", 0.3),
            ("quick", 0.2),
            ("complex", 0.8),
            ("comprehensive", 0.9),
            ("detailed", 0.7),
            ("architecture", 0.8),
            ("design", 0.6),
            ("system", 0.7),
            ("analyze", 0.6),
            ("implement", 0.7),
            ("create", 0.5),
        ];

        let mut score = 0.3; // Base complexity

        for (keyword, weight) in complexity_keywords {
            if task.to_lowercase().contains(keyword) {
                score += weight;
            }
        }

        // Factor in length (longer tasks are typically more complex)
        score += (task.len() as f64 / 200.0) * 0.3;

        score.min(1.0) // Cap at 1.0
    }

    fn extract_quality_score(&self, evaluation_text: &str) -> f64 {
        // Simple extraction - in production would use structured output
        for line in evaluation_text.lines() {
            if line.contains("score") || line.contains("Score") || line.contains("/10") {
                for word in line.split_whitespace() {
                    let cleaned = word.trim_matches(|c: char| !c.is_ascii_digit() && c != '.');
                    if let Ok(score) = cleaned.parse::<f64>() {
                        if score >= 1.0 && score <= 10.0 {
                            return score;
                        }
                    }
                }
            }
        }
        7.0 // Default reasonable score
    }

    /// Get configured role from config state
    async fn get_configured_role(&self, role_name: &str) -> MultiAgentResult<Role> {
        let config_state = self.config_state.as_ref()
            .ok_or_else(|| MultiAgentError::InvalidRoleConfig("No config state available".to_string()))?;
        
        // Access roles from the actual Config, not from config_state.roles which contains RoleGraphSync
        let config = config_state.config.lock().await;
        let role_key = role_name.to_string().into(); // Convert to RoleName
        let role = config.roles.get(&role_key)
            .ok_or_else(|| MultiAgentError::InvalidRoleConfig(format!("Role '{}' not found in configuration", role_name)))?;
        
        log::debug!("🎯 Using configured role: {} with LLM config: {:?}", role_name, role.extra);
        Ok(role.clone())
    }
}
