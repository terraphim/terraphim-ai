//! Knowledge graph-based worker agent implementation
//!
//! This module provides a specialized GenAgent implementation for domain-specific
//! task execution using knowledge graph context and thesaurus systems.

use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use log::{debug, info, warn};
use serde::{Deserialize, Serialize};

use terraphim_automata::Automata;
use terraphim_gen_agent::{GenAgent, GenAgentResult};
use terraphim_rolegraph::RoleGraph;
use terraphim_task_decomposition::Task;

use crate::{KgAgentError, KgAgentResult};

/// Message types for the worker agent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WorkerMessage {
    /// Execute a task
    ExecuteTask { task: Task },
    /// Check task compatibility
    CheckCompatibility { task: Task },
    /// Update domain specialization
    UpdateSpecialization {
        domain: String,
        expertise_level: f64,
    },
    /// Get execution status
    GetStatus,
    /// Pause execution
    Pause,
    /// Resume execution
    Resume,
}

/// Worker agent state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkerState {
    /// Current execution status
    pub status: WorkerStatus,
    /// Domain specializations
    pub specializations: HashMap<String, DomainSpecialization>,
    /// Execution history
    pub execution_history: Vec<TaskExecution>,
    /// Performance metrics
    pub metrics: WorkerMetrics,
    /// Configuration
    pub config: WorkerConfig,
}

/// Worker execution status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum WorkerStatus {
    Idle,
    Executing,
    Paused,
    Error,
}

/// Domain specialization information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DomainSpecialization {
    /// Domain name
    pub domain: String,
    /// Expertise level (0.0 to 1.0)
    pub expertise_level: f64,
    /// Number of tasks completed in this domain
    pub tasks_completed: u64,
    /// Success rate in this domain
    pub success_rate: f64,
    /// Average execution time
    pub average_execution_time: std::time::Duration,
    /// Domain-specific knowledge concepts
    pub knowledge_concepts: Vec<String>,
}

/// Task execution record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskExecution {
    /// Task identifier
    pub task_id: String,
    /// Task domain
    pub domain: String,
    /// Execution start time
    pub start_time: std::time::SystemTime,
    /// Execution duration
    pub duration: std::time::Duration,
    /// Success status
    pub success: bool,
    /// Error message if failed
    pub error_message: Option<String>,
    /// Knowledge concepts used
    pub concepts_used: Vec<String>,
    /// Confidence score
    pub confidence: f64,
}

/// Worker performance metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkerMetrics {
    /// Total tasks executed
    pub total_tasks: u64,
    /// Successful tasks
    pub successful_tasks: u64,
    /// Average execution time
    pub average_execution_time: std::time::Duration,
    /// Overall success rate
    pub success_rate: f64,
    /// Domain distribution
    pub domain_distribution: HashMap<String, u64>,
}

impl Default for WorkerMetrics {
    fn default() -> Self {
        Self {
            total_tasks: 0,
            successful_tasks: 0,
            average_execution_time: std::time::Duration::ZERO,
            success_rate: 0.0,
            domain_distribution: HashMap::new(),
        }
    }
}

/// Worker agent configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkerConfig {
    /// Maximum concurrent tasks
    pub max_concurrent_tasks: usize,
    /// Task execution timeout
    pub execution_timeout: std::time::Duration,
    /// Minimum confidence threshold for task acceptance
    pub min_confidence_threshold: f64,
    /// Enable domain learning
    pub enable_domain_learning: bool,
    /// Knowledge graph query timeout
    pub kg_query_timeout: std::time::Duration,
}

impl Default for WorkerConfig {
    fn default() -> Self {
        Self {
            max_concurrent_tasks: 5,
            execution_timeout: std::time::Duration::from_secs(300),
            min_confidence_threshold: 0.5,
            enable_domain_learning: true,
            kg_query_timeout: std::time::Duration::from_secs(10),
        }
    }
}

impl Default for WorkerState {
    fn default() -> Self {
        Self {
            status: WorkerStatus::Idle,
            specializations: HashMap::new(),
            execution_history: Vec::new(),
            metrics: WorkerMetrics::default(),
            config: WorkerConfig::default(),
        }
    }
}

/// Knowledge graph-based worker agent
pub struct KnowledgeGraphWorkerAgent {
    /// Agent identifier
    agent_id: String,
    /// Knowledge graph automata
    automata: Arc<Automata>,
    /// Role graph for domain specialization
    role_graph: Arc<RoleGraph>,
    /// Agent state
    state: WorkerState,
}

impl KnowledgeGraphWorkerAgent {
    /// Create a new worker agent
    pub fn new(
        agent_id: String,
        automata: Arc<Automata>,
        role_graph: Arc<RoleGraph>,
        config: WorkerConfig,
    ) -> Self {
        let state = WorkerState {
            status: WorkerStatus::Idle,
            specializations: HashMap::new(),
            execution_history: Vec::new(),
            metrics: WorkerMetrics::default(),
            config,
        };

        Self {
            agent_id,
            automata,
            role_graph,
            state,
        }
    }

    /// Execute a task using knowledge graph context
    async fn execute_task(&mut self, task: Task) -> KgAgentResult<TaskExecution> {
        info!("Executing task: {}", task.task_id);

        if self.state.status != WorkerStatus::Idle {
            return Err(KgAgentError::WorkerError(format!(
                "Worker {} is not idle (status: {:?})",
                self.agent_id, self.state.status
            )));
        }

        self.state.status = WorkerStatus::Executing;
        let start_time = std::time::SystemTime::now();

        // Check task compatibility first
        let compatibility = self.check_task_compatibility(&task).await?;
        if compatibility < self.state.config.min_confidence_threshold {
            self.state.status = WorkerStatus::Idle;
            return Err(KgAgentError::CompatibilityError(format!(
                "Task {} compatibility {} below threshold {}",
                task.task_id, compatibility, self.state.config.min_confidence_threshold
            )));
        }

        // Extract knowledge context for the task
        let knowledge_context = self.extract_knowledge_context(&task).await?;

        // Simulate task execution (in a real implementation, this would be domain-specific)
        let execution_result = self.perform_task_execution(&task, &knowledge_context).await;

        let duration = start_time.elapsed().unwrap_or(std::time::Duration::ZERO);
        let success = execution_result.is_ok();
        let error_message = if let Err(ref e) = execution_result {
            Some(e.to_string())
        } else {
            None
        };

        // Create execution record
        let execution = TaskExecution {
            task_id: task.task_id.clone(),
            domain: task
                .required_domains
                .first()
                .unwrap_or(&"general".to_string())
                .clone(),
            start_time,
            duration,
            success,
            error_message,
            concepts_used: knowledge_context,
            confidence: compatibility,
        };

        // Update metrics and specializations
        self.update_metrics(&execution);
        self.update_specializations(&execution);

        // Store execution history
        self.state.execution_history.push(execution.clone());

        // Limit history size
        if self.state.execution_history.len() > 1000 {
            self.state.execution_history.remove(0);
        }

        self.state.status = WorkerStatus::Idle;

        if success {
            info!(
                "Task {} executed successfully in {:.2}s",
                task.task_id,
                duration.as_secs_f64()
            );
        } else {
            warn!(
                "Task {} execution failed after {:.2}s: {:?}",
                task.task_id,
                duration.as_secs_f64(),
                error_message
            );
        }

        Ok(execution)
    }

    /// Check task compatibility using knowledge graph analysis
    async fn check_task_compatibility(&self, task: &Task) -> KgAgentResult<f64> {
        debug!("Checking compatibility for task: {}", task.task_id);

        let mut compatibility_score = 0.0;
        let mut factors = 0;

        // Check domain specialization
        for required_domain in &task.required_domains {
            if let Some(specialization) = self.state.specializations.get(required_domain) {
                compatibility_score += specialization.expertise_level * specialization.success_rate;
                factors += 1;
            }
        }

        // Check knowledge graph connectivity
        let task_concepts = &task.concepts;
        if !task_concepts.is_empty() {
            let connectivity_score = self.analyze_concept_connectivity(task_concepts).await?;
            compatibility_score += connectivity_score;
            factors += 1;
        }

        // Check capability requirements
        for required_capability in &task.required_capabilities {
            // Simple heuristic: check if we have experience with similar capabilities
            let capability_score = self.assess_capability_compatibility(required_capability);
            compatibility_score += capability_score;
            factors += 1;
        }

        let final_score = if factors > 0 {
            compatibility_score / factors as f64
        } else {
            0.5 // Default compatibility if no specific factors
        };

        debug!(
            "Task {} compatibility: {:.2} (based on {} factors)",
            task.task_id, final_score, factors
        );

        Ok(final_score)
    }

    /// Extract knowledge context using automata
    async fn extract_knowledge_context(&self, task: &Task) -> KgAgentResult<Vec<String>> {
        let context_text = format!(
            "{} {} {}",
            task.description,
            task.context_keywords.join(" "),
            task.concepts.join(" ")
        );

        // Mock implementation - in reality would use extract_paragraphs_from_automata
        let concepts = context_text
            .split_whitespace()
            .take(10)
            .map(|s| s.to_lowercase())
            .collect();

        debug!(
            "Extracted {} knowledge concepts for task {}",
            concepts.len(),
            task.task_id
        );

        Ok(concepts)
    }

    /// Perform the actual task execution
    async fn perform_task_execution(
        &self,
        task: &Task,
        knowledge_context: &[String],
    ) -> KgAgentResult<String> {
        debug!(
            "Performing execution for task {} with {} context concepts",
            task.task_id,
            knowledge_context.len()
        );

        // Simulate task execution based on complexity
        let execution_time = match task.complexity {
            terraphim_task_decomposition::TaskComplexity::Simple => {
                tokio::time::sleep(std::time::Duration::from_millis(100)).await;
            }
            terraphim_task_decomposition::TaskComplexity::Moderate => {
                tokio::time::sleep(std::time::Duration::from_millis(500)).await;
            }
            terraphim_task_decomposition::TaskComplexity::Complex => {
                tokio::time::sleep(std::time::Duration::from_millis(1000)).await;
            }
            terraphim_task_decomposition::TaskComplexity::VeryComplex => {
                tokio::time::sleep(std::time::Duration::from_millis(2000)).await;
            }
        };

        // Simulate success/failure based on compatibility and knowledge context
        let success_probability = if knowledge_context.len() > 5 {
            0.9
        } else {
            0.7
        };
        let random_value: f64 = rand::random();

        if random_value < success_probability {
            Ok(format!("Task {} completed successfully", task.task_id))
        } else {
            Err(KgAgentError::ExecutionFailed(format!(
                "Task {} execution failed due to insufficient context",
                task.task_id
            )))
        }
    }

    /// Analyze concept connectivity in knowledge graph
    async fn analyze_concept_connectivity(&self, concepts: &[String]) -> KgAgentResult<f64> {
        if concepts.len() < 2 {
            return Ok(1.0);
        }

        // Mock implementation - would use is_all_terms_connected_by_path
        let mut connectivity_score = 0.0;
        let mut pairs = 0;

        for i in 0..concepts.len() {
            for j in (i + 1)..concepts.len() {
                pairs += 1;
                // Simple heuristic: concepts are connected if they share characters
                let concept1 = &concepts[i];
                let concept2 = &concepts[j];
                if concept1.chars().any(|c| concept2.contains(c)) {
                    connectivity_score += 1.0;
                }
            }
        }

        let final_score = if pairs > 0 {
            connectivity_score / pairs as f64
        } else {
            0.0
        };

        Ok(final_score)
    }

    /// Assess capability compatibility
    fn assess_capability_compatibility(&self, capability: &str) -> f64 {
        // Check if we have experience with similar capabilities
        for execution in &self.state.execution_history {
            if execution.success
                && execution
                    .concepts_used
                    .iter()
                    .any(|c| c.contains(capability))
            {
                return 0.8;
            }
        }

        // Check domain specializations
        for specialization in self.state.specializations.values() {
            if specialization
                .knowledge_concepts
                .iter()
                .any(|c| c.contains(capability))
            {
                return specialization.expertise_level;
            }
        }

        0.3 // Default low compatibility
    }

    /// Update performance metrics
    fn update_metrics(&mut self, execution: &TaskExecution) {
        self.state.metrics.total_tasks += 1;
        if execution.success {
            self.state.metrics.successful_tasks += 1;
        }

        // Update success rate
        self.state.metrics.success_rate =
            self.state.metrics.successful_tasks as f64 / self.state.metrics.total_tasks as f64;

        // Update average execution time
        let total_time = self.state.metrics.average_execution_time.as_secs_f64()
            * (self.state.metrics.total_tasks - 1) as f64
            + execution.duration.as_secs_f64();
        self.state.metrics.average_execution_time =
            std::time::Duration::from_secs_f64(total_time / self.state.metrics.total_tasks as f64);

        // Update domain distribution
        *self
            .state
            .metrics
            .domain_distribution
            .entry(execution.domain.clone())
            .or_insert(0) += 1;
    }

    /// Update domain specializations
    fn update_specializations(&mut self, execution: &TaskExecution) {
        if !self.state.config.enable_domain_learning {
            return;
        }

        let specialization = self
            .state
            .specializations
            .entry(execution.domain.clone())
            .or_insert_with(|| DomainSpecialization {
                domain: execution.domain.clone(),
                expertise_level: 0.1,
                tasks_completed: 0,
                success_rate: 0.0,
                average_execution_time: std::time::Duration::ZERO,
                knowledge_concepts: Vec::new(),
            });

        specialization.tasks_completed += 1;

        // Update success rate
        let previous_successes =
            (specialization.success_rate * (specialization.tasks_completed - 1) as f64) as u64;
        let new_successes = if execution.success {
            previous_successes + 1
        } else {
            previous_successes
        };
        specialization.success_rate = new_successes as f64 / specialization.tasks_completed as f64;

        // Update expertise level based on success rate and experience
        let experience_factor = (specialization.tasks_completed as f64).ln().max(1.0) / 10.0;
        specialization.expertise_level =
            (specialization.success_rate * 0.7 + experience_factor * 0.3).min(1.0);

        // Update average execution time
        let total_time = specialization.average_execution_time.as_secs_f64()
            * (specialization.tasks_completed - 1) as f64
            + execution.duration.as_secs_f64();
        specialization.average_execution_time =
            std::time::Duration::from_secs_f64(total_time / specialization.tasks_completed as f64);

        // Update knowledge concepts
        for concept in &execution.concepts_used {
            if !specialization.knowledge_concepts.contains(concept) {
                specialization.knowledge_concepts.push(concept.clone());
            }
        }

        // Limit concept list size
        if specialization.knowledge_concepts.len() > 100 {
            specialization.knowledge_concepts.truncate(100);
        }
    }
}

#[async_trait]
impl GenAgent<WorkerState> for KnowledgeGraphWorkerAgent {
    type Message = WorkerMessage;

    async fn init(&mut self, _init_args: serde_json::Value) -> GenAgentResult<()> {
        info!("Initializing worker agent: {}", self.agent_id);
        self.state.status = WorkerStatus::Idle;
        Ok(())
    }

    async fn handle_call(&mut self, message: Self::Message) -> GenAgentResult<serde_json::Value> {
        match message {
            WorkerMessage::ExecuteTask { task } => {
                let execution = self.execute_task(task).await.map_err(|e| {
                    terraphim_gen_agent::GenAgentError::ExecutionError(
                        self.agent_id.clone(),
                        e.to_string(),
                    )
                })?;
                Ok(serde_json::to_value(execution).unwrap())
            }
            WorkerMessage::CheckCompatibility { task } => {
                let compatibility = self.check_task_compatibility(&task).await.map_err(|e| {
                    terraphim_gen_agent::GenAgentError::ExecutionError(
                        self.agent_id.clone(),
                        e.to_string(),
                    )
                })?;
                Ok(serde_json::to_value(compatibility).unwrap())
            }
            WorkerMessage::GetStatus => Ok(serde_json::to_value(&self.state.status).unwrap()),
            _ => {
                // Other messages don't return values in call context
                Ok(serde_json::Value::Null)
            }
        }
    }

    async fn handle_cast(&mut self, message: Self::Message) -> GenAgentResult<()> {
        match message {
            WorkerMessage::ExecuteTask { task } => {
                let _ = self.execute_task(task).await;
            }
            WorkerMessage::UpdateSpecialization {
                domain,
                expertise_level,
            } => {
                let specialization = self
                    .state
                    .specializations
                    .entry(domain.clone())
                    .or_insert_with(|| DomainSpecialization {
                        domain: domain.clone(),
                        expertise_level: 0.1,
                        tasks_completed: 0,
                        success_rate: 0.0,
                        average_execution_time: std::time::Duration::ZERO,
                        knowledge_concepts: Vec::new(),
                    });
                specialization.expertise_level = expertise_level.clamp(0.0, 1.0);
            }
            WorkerMessage::Pause => {
                if self.state.status == WorkerStatus::Executing {
                    self.state.status = WorkerStatus::Paused;
                }
            }
            WorkerMessage::Resume => {
                if self.state.status == WorkerStatus::Paused {
                    self.state.status = WorkerStatus::Executing;
                }
            }
            _ => {
                // Other messages handled in call context
            }
        }
        Ok(())
    }

    async fn handle_info(&mut self, _message: serde_json::Value) -> GenAgentResult<()> {
        // Handle system messages, health checks, etc.
        Ok(())
    }

    async fn terminate(&mut self, _reason: String) -> GenAgentResult<()> {
        info!("Terminating worker agent: {}", self.agent_id);
        self.state.status = WorkerStatus::Idle;
        Ok(())
    }

    fn get_state(&self) -> &WorkerState {
        &self.state
    }

    fn get_state_mut(&mut self) -> &mut WorkerState {
        &mut self.state
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use terraphim_task_decomposition::TaskComplexity;

    fn create_test_task() -> Task {
        let mut task = Task::new(
            "test_task".to_string(),
            "Test task for worker".to_string(),
            TaskComplexity::Simple,
            1,
        );
        task.required_domains = vec!["testing".to_string()];
        task.required_capabilities = vec!["test_execution".to_string()];
        task.concepts = vec!["test".to_string(), "execution".to_string()];
        task
    }

    async fn create_test_agent() -> KnowledgeGraphWorkerAgent {
        use terraphim_automata::{load_thesaurus, AutomataPath};
        use terraphim_types::RoleName;

        let automata = Arc::new(terraphim_automata::Automata::default());

        let role_name = RoleName::new("worker");
        let thesaurus = load_thesaurus(&AutomataPath::local_example())
            .await
            .unwrap();
        let role_graph = Arc::new(RoleGraph::new(role_name, thesaurus).await.unwrap());

        KnowledgeGraphWorkerAgent::new(
            "test_worker".to_string(),
            automata,
            role_graph,
            WorkerConfig::default(),
        )
    }

    #[tokio::test]
    async fn test_worker_agent_creation() {
        let agent = create_test_agent().await;
        assert_eq!(agent.agent_id, "test_worker");
        assert_eq!(agent.state.status, WorkerStatus::Idle);
    }

    #[tokio::test]
    async fn test_task_compatibility_check() {
        let agent = create_test_agent().await;
        let task = create_test_task();

        let compatibility = agent.check_task_compatibility(&task).await.unwrap();
        assert!(compatibility >= 0.0 && compatibility <= 1.0);
    }

    #[tokio::test]
    async fn test_knowledge_context_extraction() {
        let agent = create_test_agent().await;
        let task = create_test_task();

        let context = agent.extract_knowledge_context(&task).await.unwrap();
        assert!(!context.is_empty());
    }

    #[tokio::test]
    async fn test_concept_connectivity_analysis() {
        let agent = create_test_agent().await;
        let concepts = vec!["test".to_string(), "execution".to_string()];

        let connectivity = agent.analyze_concept_connectivity(&concepts).await.unwrap();
        assert!(connectivity >= 0.0 && connectivity <= 1.0);
    }

    #[tokio::test]
    async fn test_capability_compatibility() {
        let agent = create_test_agent().await;
        let compatibility = agent.assess_capability_compatibility("test_execution");
        assert!(compatibility >= 0.0 && compatibility <= 1.0);
    }

    #[tokio::test]
    async fn test_gen_agent_interface() {
        let mut agent = create_test_agent().await;

        // Test initialization
        let init_result = agent.init(serde_json::json!({})).await;
        assert!(init_result.is_ok());

        // Test call message
        let task = create_test_task();
        let message = WorkerMessage::CheckCompatibility { task };
        let call_result = agent.handle_call(message).await;
        assert!(call_result.is_ok());

        // Test cast message
        let message = WorkerMessage::Pause;
        let cast_result = agent.handle_cast(message).await;
        assert!(cast_result.is_ok());

        // Test termination
        let terminate_result = agent.terminate("test".to_string()).await;
        assert!(terminate_result.is_ok());
    }
}
