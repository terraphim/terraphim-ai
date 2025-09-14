//! Integration module for connecting workflows with the agent evolution system
//!
//! This module provides the bridge between individual workflow patterns and the
//! comprehensive agent evolution tracking system.

use std::collections::HashMap;
use std::sync::Arc;

use chrono::Utc;

use crate::{
    llm_adapter::LlmAdapterFactory,
    workflows::{TaskAnalysis, WorkflowFactory, WorkflowInput, WorkflowParameters},
    AgentEvolutionSystem, AgentId, EvolutionResult, LlmAdapter,
};

/// Integrated evolution workflow manager that combines workflow execution with evolution tracking
pub struct EvolutionWorkflowManager {
    evolution_system: AgentEvolutionSystem,
    default_llm_adapter: Arc<dyn LlmAdapter>,
}

impl EvolutionWorkflowManager {
    /// Create a new evolution workflow manager
    pub fn new(agent_id: AgentId) -> Self {
        let evolution_system = AgentEvolutionSystem::new(agent_id);
        let default_llm_adapter = LlmAdapterFactory::create_mock("default");

        Self {
            evolution_system,
            default_llm_adapter,
        }
    }

    /// Create with custom LLM adapter
    pub fn with_adapter(agent_id: AgentId, adapter: Arc<dyn LlmAdapter>) -> Self {
        let evolution_system = AgentEvolutionSystem::new(agent_id);

        Self {
            evolution_system,
            default_llm_adapter: adapter,
        }
    }

    /// Execute a task using the most appropriate workflow pattern
    pub async fn execute_task(
        &mut self,
        task_id: String,
        prompt: String,
        context: Option<String>,
    ) -> EvolutionResult<String> {
        // Analyze the task to determine the best workflow pattern
        let task_analysis = self.analyze_task(&prompt).await?;

        // Create workflow input
        let workflow_input = WorkflowInput {
            task_id: task_id.clone(),
            agent_id: self.evolution_system.agent_id.clone(),
            prompt: prompt.clone(),
            context,
            parameters: WorkflowParameters::default(),
            timestamp: Utc::now(),
        };

        // Select and create appropriate workflow pattern
        let workflow =
            WorkflowFactory::create_for_task(&task_analysis, self.default_llm_adapter.clone());

        log::info!(
            "Executing task {} with workflow pattern: {}",
            task_id,
            workflow.pattern_name()
        );

        // Execute the workflow
        let workflow_output = workflow.execute(workflow_input).await?;

        // Update agent evolution state based on the execution
        self.update_evolution_state(&workflow_output, &task_analysis)
            .await?;

        Ok(workflow_output.result)
    }

    /// Execute a task with a specific workflow pattern
    pub async fn execute_with_pattern(
        &mut self,
        task_id: String,
        prompt: String,
        context: Option<String>,
        pattern_name: &str,
    ) -> EvolutionResult<String> {
        // Create workflow input
        let workflow_input = WorkflowInput {
            task_id: task_id.clone(),
            agent_id: self.evolution_system.agent_id.clone(),
            prompt: prompt.clone(),
            context,
            parameters: WorkflowParameters::default(),
            timestamp: Utc::now(),
        };

        // Create specified workflow pattern
        let workflow =
            WorkflowFactory::create_by_name(pattern_name, self.default_llm_adapter.clone())?;

        log::info!(
            "Executing task {} with specified workflow pattern: {}",
            task_id,
            pattern_name
        );

        // Execute the workflow
        let workflow_output = workflow.execute(workflow_input).await?;

        // Analyze task for evolution tracking
        let task_analysis = self.analyze_task(&prompt).await?;

        // Update agent evolution state
        self.update_evolution_state(&workflow_output, &task_analysis)
            .await?;

        Ok(workflow_output.result)
    }

    /// Get the agent evolution system for direct access
    pub fn evolution_system(&self) -> &AgentEvolutionSystem {
        &self.evolution_system
    }

    /// Get mutable access to the evolution system
    pub fn evolution_system_mut(&mut self) -> &mut AgentEvolutionSystem {
        &mut self.evolution_system
    }

    /// Save the current evolution state
    pub async fn save_evolution_state(&self) -> EvolutionResult<()> {
        self.evolution_system
            .create_snapshot("Workflow execution checkpoint".to_string())
            .await
    }

    /// Analyze a task to determine its characteristics
    async fn analyze_task(&self, prompt: &str) -> EvolutionResult<TaskAnalysis> {
        // Simple heuristic-based analysis
        // In a real implementation, this could use ML models for better analysis

        let complexity = if prompt.len() > 2000 {
            crate::workflows::TaskComplexity::VeryComplex
        } else if prompt.len() > 1000 {
            crate::workflows::TaskComplexity::Complex
        } else if prompt.len() > 500 {
            crate::workflows::TaskComplexity::Moderate
        } else {
            crate::workflows::TaskComplexity::Simple
        };

        let domain = if prompt.to_lowercase().contains("code")
            || prompt.to_lowercase().contains("program")
        {
            "coding".to_string()
        } else if prompt.to_lowercase().contains("analyze")
            || prompt.to_lowercase().contains("research")
        {
            "analysis".to_string()
        } else if prompt.to_lowercase().contains("write")
            || prompt.to_lowercase().contains("create")
        {
            "creative".to_string()
        } else if prompt.to_lowercase().contains("math")
            || prompt.to_lowercase().contains("calculate")
        {
            "mathematics".to_string()
        } else {
            "general".to_string()
        };

        let requires_decomposition = prompt.contains("step by step")
            || prompt.contains("break down")
            || matches!(
                complexity,
                crate::workflows::TaskComplexity::Complex
                    | crate::workflows::TaskComplexity::VeryComplex
            );

        let suitable_for_parallel = prompt.contains("compare")
            || prompt.contains("multiple")
            || prompt.contains("different approaches");

        let quality_critical = prompt.contains("important")
            || prompt.contains("critical")
            || prompt.contains("precise")
            || prompt.contains("accurate");

        let estimated_steps = match complexity {
            crate::workflows::TaskComplexity::Simple => 1,
            crate::workflows::TaskComplexity::Moderate => 2,
            crate::workflows::TaskComplexity::Complex => 4,
            crate::workflows::TaskComplexity::VeryComplex => 6,
        };

        Ok(TaskAnalysis {
            complexity,
            domain,
            requires_decomposition,
            suitable_for_parallel,
            quality_critical,
            estimated_steps,
        })
    }

    /// Update the agent evolution state based on workflow execution
    async fn update_evolution_state(
        &mut self,
        workflow_output: &crate::workflows::WorkflowOutput,
        task_analysis: &TaskAnalysis,
    ) -> EvolutionResult<()> {
        // Add task to task list
        let task_id = workflow_output.task_id.clone();
        let agent_task = crate::tasks::AgentTask {
            id: task_id.clone(),
            content: format!("Task: {}", task_analysis.domain),
            active_form: format!("Working on: {}", task_analysis.domain),
            status: crate::tasks::TaskStatus::InProgress,
            priority: crate::tasks::Priority::Medium,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            deadline: None,
            dependencies: vec![],
            subtasks: vec![],
            estimated_duration: Some(workflow_output.metadata.execution_time),
            actual_duration: None,
            parent_task: None,
            goal_alignment_score: workflow_output.metadata.quality_score.unwrap_or(0.5),
            metadata: {
                let mut meta = std::collections::HashMap::new();
                meta.insert(
                    "workflow".to_string(),
                    serde_json::json!(workflow_output.metadata.pattern_used),
                );
                meta
            },
        };
        self.evolution_system.tasks.add_task(agent_task).await?;

        // Mark task as completed
        self.evolution_system
            .tasks
            .complete_task(&task_id, &workflow_output.result)
            .await?;

        // Add memory entries for execution trace
        for (i, step) in workflow_output.execution_trace.iter().enumerate() {
            let memory_id = format!("{}_{}", task_id, i);
            let memory_item = crate::memory::MemoryItem {
                id: memory_id,
                item_type: crate::memory::MemoryItemType::Experience,
                content: format!("Step {}: {}", i + 1, step.step_id),
                created_at: chrono::Utc::now(),
                last_accessed: None,
                access_count: 0,
                importance: crate::memory::ImportanceLevel::Medium,
                tags: vec![task_id.clone(), "execution_trace".to_string()],
                associations: std::collections::HashMap::new(),
            };
            self.evolution_system.memory.add_memory(memory_item).await?;
        }

        // Extract lessons from the execution
        if let Some(quality_score) = workflow_output.metadata.quality_score {
            let lesson_type = if quality_score > 0.8 {
                "success_pattern"
            } else if quality_score < 0.5 {
                "failure_analysis"
            } else {
                "improvement_opportunity"
            };

            let lesson_content = format!(
                "Workflow '{}' achieved quality score {:.2} for {} task in domain '{}'",
                workflow_output.metadata.pattern_used,
                quality_score,
                format!("{:?}", task_analysis.complexity).to_lowercase(),
                task_analysis.domain
            );

            let lesson = crate::lessons::Lesson {
                id: format!("lesson_{}", chrono::Utc::now().timestamp()),
                title: lesson_type.to_string(),
                context: lesson_content.clone(),
                insight: format!(
                    "Workflow {} performed well for {} tasks",
                    workflow_output.metadata.pattern_used, task_analysis.domain
                ),
                category: crate::lessons::LessonCategory::Process,
                evidence: vec![crate::lessons::Evidence {
                    description: format!("Quality score of {:.2}", quality_score),
                    source: crate::lessons::EvidenceSource::PerformanceMetric,
                    outcome: if quality_score > 0.7 {
                        crate::lessons::EvidenceOutcome::Success
                    } else {
                        crate::lessons::EvidenceOutcome::Mixed
                    },
                    confidence: quality_score,
                    timestamp: chrono::Utc::now(),
                    metadata: std::collections::HashMap::new(),
                }],
                impact: if quality_score > 0.8 {
                    crate::lessons::ImpactLevel::High
                } else {
                    crate::lessons::ImpactLevel::Medium
                },
                confidence: quality_score,
                learned_at: chrono::Utc::now(),
                last_applied: None,
                applied_count: 0,
                tags: vec![
                    task_analysis.domain.clone(),
                    workflow_output.metadata.pattern_used.clone(),
                ],
                last_validated: None,
                validated: false,
                success_rate: 0.0,
                related_tasks: vec![],
                related_memories: vec![],
                knowledge_graph_refs: vec![],
                contexts: vec![],
                metadata: HashMap::new(),
            };
            self.evolution_system.lessons.add_lesson(lesson).await?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_workflow_manager_creation() {
        let manager = EvolutionWorkflowManager::new("test_agent".to_string());
        assert_eq!(manager.evolution_system().agent_id, "test_agent");
    }

    #[tokio::test]
    async fn test_task_analysis() {
        let manager = EvolutionWorkflowManager::new("test_agent".to_string());

        let simple_analysis = manager.analyze_task("Hello world").await.unwrap();
        assert!(matches!(
            simple_analysis.complexity,
            crate::workflows::TaskComplexity::Simple
        ));

        let complex_analysis = manager.analyze_task(&"x".repeat(1500)).await.unwrap();
        assert!(matches!(
            complex_analysis.complexity,
            crate::workflows::TaskComplexity::Complex
        ));
    }

    #[tokio::test]
    async fn test_execute_task_integration() {
        let mut manager = EvolutionWorkflowManager::new("test_agent".to_string());

        let result = manager
            .execute_task(
                "test_task".to_string(),
                "Analyze the benefits of Rust programming".to_string(),
                None,
            )
            .await;

        assert!(result.is_ok());

        // Verify task was added to evolution system
        let tasks_state = &manager.evolution_system().tasks.current_state;
        assert!(tasks_state.completed_tasks() > 0);
    }
}
