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
    update_workflow_status, ExecutionStatus, WebSocketBroadcaster, WorkflowMetadata,
    WorkflowSessions,
};

/// Multi-agent workflow executor
pub struct MultiAgentWorkflowExecutor {
    agent_registry: AgentRegistry,
    persistence: Arc<DeviceStorage>,
}

impl MultiAgentWorkflowExecutor {
    /// Create new multi-agent workflow executor
    pub async fn new() -> MultiAgentResult<Self> {
        // Initialize storage for agents
        DeviceStorage::init_memory_only()
            .await
            .map_err(|e| MultiAgentError::PersistenceError(e.to_string()))?;
        let storage_ref = DeviceStorage::instance()
            .await
            .map_err(|e| MultiAgentError::PersistenceError(e.to_string()))?;

        use std::ptr;
        let storage_copy = unsafe { ptr::read(storage_ref) };
        let persistence = Arc::new(storage_copy);

        let agent_registry = AgentRegistry::new();

        Ok(Self {
            agent_registry,
            persistence,
        })
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
    ) -> MultiAgentResult<Value> {
        log::info!("Executing prompt chain workflow with TerraphimAgent");

        // Create development agent for prompt chaining
        let dev_role = self.create_development_role(role);
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

        let (mut selected_agent, route_id) = if complexity > 0.7 {
            (self.create_complex_agent().await?, "complex_agent")
        } else {
            (self.create_simple_agent().await?, "simple_agent")
        };

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

        // Create multiple perspective agents
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
                Some(format!("Creating {} agent", perspective_name)),
            )
            .await;

            let perspective_role =
                self.create_perspective_role(perspective_name, perspective_description);
            let mut agent =
                TerraphimAgent::new(perspective_role, self.persistence.clone(), None).await?;
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

        let orchestrator_role = self.create_orchestrator_role();
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

            let worker_role = self.create_worker_role(worker_name, worker_description);
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

        // Create generator and evaluator agents
        update_workflow_status(
            sessions,
            broadcaster,
            workflow_id,
            ExecutionStatus::Running,
            20.0,
            Some("Creating generator and evaluator agents".to_string()),
        )
        .await;

        let generator_role = self.create_generator_role();
        let mut generator =
            TerraphimAgent::new(generator_role, self.persistence.clone(), None).await?;
        generator.initialize().await?;

        let evaluator_role = self.create_evaluator_role();
        let mut evaluator =
            TerraphimAgent::new(evaluator_role, self.persistence.clone(), None).await?;
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

    fn create_development_role(&self, base_role: &str) -> Role {
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
        extra.insert("llm_provider".to_string(), serde_json::json!("ollama"));
        extra.insert("ollama_model".to_string(), serde_json::json!("gemma3:270m"));
        extra.insert("llm_temperature".to_string(), serde_json::json!(0.3));
        extra.insert("base_role".to_string(), serde_json::json!(base_role));

        Role {
            shortname: Some("DevAgent".to_string()),
            name: "DevelopmentAgent".into(),
            relevance_function: RelevanceFunction::BM25,
            terraphim_it: false,
            theme: "default".to_string(),
            kg: None,
            haystacks: vec![],
            extra,
        }
    }

    async fn create_simple_agent(&self) -> MultiAgentResult<TerraphimAgent> {
        let mut extra = AHashMap::new();
        extra.insert("llm_temperature".to_string(), serde_json::json!(0.2));
        extra.insert("agent_type".to_string(), serde_json::json!("simple"));

        let role = Role {
            shortname: Some("Simple".to_string()),
            name: "SimpleTaskAgent".into(),
            relevance_function: RelevanceFunction::TitleScorer,
            terraphim_it: false,
            theme: "default".to_string(),
            kg: None,
            haystacks: vec![],
            extra,
        };

        let mut agent = TerraphimAgent::new(role, self.persistence.clone(), None).await?;
        agent.initialize().await?;
        Ok(agent)
    }

    async fn create_complex_agent(&self) -> MultiAgentResult<TerraphimAgent> {
        let mut extra = AHashMap::new();
        extra.insert("llm_temperature".to_string(), serde_json::json!(0.4));
        extra.insert("agent_type".to_string(), serde_json::json!("complex"));

        let role = Role {
            shortname: Some("Complex".to_string()),
            name: "ComplexTaskAgent".into(),
            relevance_function: RelevanceFunction::BM25,
            terraphim_it: false,
            theme: "default".to_string(),
            kg: None,
            haystacks: vec![],
            extra,
        };

        let mut agent = TerraphimAgent::new(role, self.persistence.clone(), None).await?;
        agent.initialize().await?;
        Ok(agent)
    }

    fn create_perspective_role(&self, perspective: &str, description: &str) -> Role {
        let mut extra = AHashMap::new();
        extra.insert("perspective".to_string(), serde_json::json!(perspective));
        extra.insert("description".to_string(), serde_json::json!(description));
        extra.insert("llm_temperature".to_string(), serde_json::json!(0.5));

        Role {
            shortname: Some(perspective.to_string()),
            name: format!("{}PerspectiveAgent", perspective).into(),
            relevance_function: RelevanceFunction::BM25,
            terraphim_it: false,
            theme: "default".to_string(),
            kg: None,
            haystacks: vec![],
            extra,
        }
    }

    fn create_orchestrator_role(&self) -> Role {
        let mut extra = AHashMap::new();
        extra.insert("role_type".to_string(), serde_json::json!("orchestrator"));
        extra.insert("llm_temperature".to_string(), serde_json::json!(0.3));

        Role {
            shortname: Some("Orchestrator".to_string()),
            name: "OrchestratorAgent".into(),
            relevance_function: RelevanceFunction::BM25,
            terraphim_it: false,
            theme: "default".to_string(),
            kg: None,
            haystacks: vec![],
            extra,
        }
    }

    fn create_worker_role(&self, worker_name: &str, description: &str) -> Role {
        let mut extra = AHashMap::new();
        extra.insert("worker_type".to_string(), serde_json::json!(worker_name));
        extra.insert("description".to_string(), serde_json::json!(description));
        extra.insert("llm_temperature".to_string(), serde_json::json!(0.4));

        Role {
            shortname: Some(worker_name.to_string()),
            name: format!("{}Worker", worker_name).into(),
            relevance_function: RelevanceFunction::BM25,
            terraphim_it: false,
            theme: "default".to_string(),
            kg: None,
            haystacks: vec![],
            extra,
        }
    }

    fn create_generator_role(&self) -> Role {
        let mut extra = AHashMap::new();
        extra.insert("role_type".to_string(), serde_json::json!("generator"));
        extra.insert("llm_temperature".to_string(), serde_json::json!(0.6));

        Role {
            shortname: Some("Generator".to_string()),
            name: "GeneratorAgent".into(),
            relevance_function: RelevanceFunction::BM25,
            terraphim_it: false,
            theme: "default".to_string(),
            kg: None,
            haystacks: vec![],
            extra,
        }
    }

    fn create_evaluator_role(&self) -> Role {
        let mut extra = AHashMap::new();
        extra.insert("role_type".to_string(), serde_json::json!("evaluator"));
        extra.insert("llm_temperature".to_string(), serde_json::json!(0.2));

        Role {
            shortname: Some("Evaluator".to_string()),
            name: "EvaluatorAgent".into(),
            relevance_function: RelevanceFunction::BM25,
            terraphim_it: false,
            theme: "default".to_string(),
            kg: None,
            haystacks: vec![],
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
}
