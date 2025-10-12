//! Knowledge graph-based agent matching and coordination
//!
//! This module provides intelligent agent-task matching using knowledge graph connectivity
//! analysis and capability assessment through semantic understanding.

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use async_trait::async_trait;
use log::{debug, info, warn};
use serde::{Deserialize, Serialize};

use terraphim_rolegraph::RoleGraph;

// Temporary mock functions until dependencies are fixed
fn extract_paragraphs_from_automata(
    _automata: &MockAutomata,
    text: &str,
    max_results: u32,
) -> Result<Vec<String>, String> {
    // Simple mock implementation
    let words: Vec<String> = text
        .split_whitespace()
        .take(max_results as usize)
        .map(|s| s.to_string())
        .collect();
    Ok(words)
}

fn is_all_terms_connected_by_path(
    _automata: &MockAutomata,
    terms: &[&str],
) -> Result<bool, String> {
    // Simple mock implementation - assume connected if terms share characters
    if terms.len() < 2 {
        return Ok(true);
    }
    let first = terms[0].to_lowercase();
    let second = terms[1].to_lowercase();
    Ok(first.chars().any(|c| second.contains(c)))
}

// Shared mock automata type
#[derive(Debug, Clone, Default)]
pub struct MockAutomata;
pub type Automata = MockAutomata;

use crate::{AgentCapability, AgentMetadata, RegistryResult};

/// Task representation for matching
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    /// Task identifier
    pub task_id: String,
    /// Task description
    pub description: String,
    /// Required capabilities
    pub required_capabilities: Vec<String>,
    /// Required domains
    pub required_domains: Vec<String>,
    /// Task complexity level
    pub complexity: TaskComplexity,
    /// Task priority
    pub priority: u32,
    /// Estimated effort
    pub estimated_effort: f64,
    /// Task context keywords
    pub context_keywords: Vec<String>,
    /// Task concepts
    pub concepts: Vec<String>,
}

/// Task complexity levels
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TaskComplexity {
    Simple,
    Moderate,
    Complex,
    VeryComplex,
}

/// Agent-task matching result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentTaskMatch {
    /// Matched agent
    pub agent: AgentMetadata,
    /// Task being matched
    pub task: Task,
    /// Overall match score (0.0 to 1.0)
    pub match_score: f64,
    /// Detailed score breakdown
    pub score_breakdown: TaskMatchScoreBreakdown,
    /// Matching explanation
    pub explanation: String,
    /// Confidence level
    pub confidence: f64,
    /// Estimated completion time
    pub estimated_completion_time: Option<std::time::Duration>,
}

/// Detailed score breakdown for task matching
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskMatchScoreBreakdown {
    /// Capability matching score
    pub capability_score: f64,
    /// Domain expertise score
    pub domain_score: f64,
    /// Knowledge graph connectivity score
    pub connectivity_score: f64,
    /// Agent availability score
    pub availability_score: f64,
    /// Performance history score
    pub performance_score: f64,
    /// Complexity handling score
    pub complexity_score: f64,
}

/// Coordination workflow step
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoordinationStep {
    /// Step identifier
    pub step_id: String,
    /// Step description
    pub description: String,
    /// Assigned agent
    pub assigned_agent: String,
    /// Dependencies on other steps
    pub dependencies: Vec<String>,
    /// Estimated duration
    pub estimated_duration: std::time::Duration,
    /// Step status
    pub status: StepStatus,
}

/// Step execution status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum StepStatus {
    Pending,
    InProgress,
    Completed,
    Failed,
    Blocked,
}

/// Workflow coordination result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoordinationResult {
    /// Workflow identifier
    pub workflow_id: String,
    /// Coordination steps
    pub steps: Vec<CoordinationStep>,
    /// Agent assignments
    pub agent_assignments: HashMap<String, Vec<String>>,
    /// Estimated total duration
    pub estimated_duration: std::time::Duration,
    /// Parallelism factor (0.0 to 1.0)
    pub parallelism_factor: f64,
    /// Bottleneck analysis
    pub bottlenecks: Vec<String>,
}

/// Knowledge graph agent matcher
#[async_trait]
pub trait KnowledgeGraphAgentMatcher: Send + Sync {
    /// Match a task to the best available agents
    async fn match_task_to_agents(
        &self,
        task: &Task,
        available_agents: &[AgentMetadata],
        max_matches: usize,
    ) -> RegistryResult<Vec<AgentTaskMatch>>;

    /// Assess agent capability for a specific task
    async fn assess_agent_capability(
        &self,
        agent: &AgentMetadata,
        task: &Task,
    ) -> RegistryResult<f64>;

    /// Coordinate multiple agents for workflow execution
    async fn coordinate_workflow(
        &self,
        tasks: &[Task],
        available_agents: &[AgentMetadata],
    ) -> RegistryResult<CoordinationResult>;

    /// Monitor workflow progress and detect bottlenecks
    async fn monitor_progress(
        &self,
        workflow_id: &str,
        coordination: &CoordinationResult,
    ) -> RegistryResult<Vec<String>>;
}

/// Knowledge graph-based agent matcher implementation
#[allow(dead_code)]
pub struct TerraphimKnowledgeGraphMatcher {
    /// Knowledge graph automata
    automata: Arc<Automata>,
    /// Role graphs for different roles
    role_graphs: HashMap<String, Arc<RoleGraph>>,
    /// Matching configuration
    config: MatchingConfig,
    /// Performance cache
    cache: Arc<tokio::sync::RwLock<HashMap<String, f64>>>,
}

/// Configuration for knowledge graph matching
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatchingConfig {
    /// Minimum connectivity threshold
    pub min_connectivity_threshold: f64,
    /// Capability weight in scoring
    pub capability_weight: f64,
    /// Domain weight in scoring
    pub domain_weight: f64,
    /// Connectivity weight in scoring
    pub connectivity_weight: f64,
    /// Performance weight in scoring
    pub performance_weight: f64,
    /// Maximum context extraction length
    pub max_context_length: u32,
    /// Enable caching
    pub enable_caching: bool,
}

impl Default for MatchingConfig {
    fn default() -> Self {
        Self {
            min_connectivity_threshold: 0.6,
            capability_weight: 0.25,
            domain_weight: 0.25,
            connectivity_weight: 0.25,
            performance_weight: 0.25,
            max_context_length: 500,
            enable_caching: true,
        }
    }
}

impl TerraphimKnowledgeGraphMatcher {
    /// Create a new knowledge graph matcher
    pub fn new(
        automata: Arc<Automata>,
        role_graphs: HashMap<String, Arc<RoleGraph>>,
        config: MatchingConfig,
    ) -> Self {
        Self {
            automata,
            role_graphs,
            config,
            cache: Arc::new(tokio::sync::RwLock::new(HashMap::new())),
        }
    }

    /// Create with default configuration
    pub fn with_default_config(
        automata: Arc<Automata>,
        role_graphs: HashMap<String, Arc<RoleGraph>>,
    ) -> Self {
        Self::new(automata, role_graphs, MatchingConfig::default())
    }

    /// Extract context from task using knowledge graph
    async fn extract_task_context(&self, task: &Task) -> RegistryResult<Vec<String>> {
        let context_text = format!(
            "{} {} {}",
            task.description,
            task.context_keywords.join(" "),
            task.concepts.join(" ")
        );

        match extract_paragraphs_from_automata(
            &self.automata,
            &context_text,
            self.config.max_context_length,
        ) {
            Ok(paragraphs) => {
                debug!(
                    "Extracted {} context paragraphs for task {}",
                    paragraphs.len(),
                    task.task_id
                );
                Ok(paragraphs)
            }
            Err(e) => {
                warn!("Failed to extract context for task {}: {}", task.task_id, e);
                Ok(Vec::new()) // Return empty context instead of failing
            }
        }
    }

    /// Analyze connectivity between task and agent concepts
    async fn analyze_connectivity(
        &self,
        task_concepts: &[String],
        agent_concepts: &[String],
    ) -> RegistryResult<f64> {
        if task_concepts.is_empty() || agent_concepts.is_empty() {
            return Ok(0.0);
        }

        let mut total_connectivity = 0.0;
        let mut connection_count = 0;

        for task_concept in task_concepts {
            for agent_concept in agent_concepts {
                match is_all_terms_connected_by_path(&self.automata, &[task_concept, agent_concept])
                {
                    Ok(connected) => {
                        if connected {
                            total_connectivity += 1.0;
                        }
                        connection_count += 1;
                    }
                    Err(e) => {
                        debug!(
                            "Connectivity check failed for {} -> {}: {}",
                            task_concept, agent_concept, e
                        );
                    }
                }
            }
        }

        let connectivity_score = if connection_count > 0 {
            total_connectivity / connection_count as f64
        } else {
            0.0
        };

        debug!(
            "Connectivity analysis: {:.2} ({}/{} connections)",
            connectivity_score, total_connectivity as u32, connection_count
        );

        Ok(connectivity_score)
    }

    /// Calculate capability matching score
    fn calculate_capability_score(
        &self,
        task: &Task,
        agent: &AgentMetadata,
    ) -> RegistryResult<f64> {
        if task.required_capabilities.is_empty() {
            return Ok(1.0);
        }

        let mut matched_capabilities = 0;
        let total_required = task.required_capabilities.len();

        for required_capability in &task.required_capabilities {
            for agent_capability in &agent.capabilities {
                if self.capability_matches(required_capability, agent_capability) {
                    matched_capabilities += 1;
                    break;
                }
            }
        }

        let score = matched_capabilities as f64 / total_required as f64;
        debug!(
            "Capability score for agent {}: {:.2} ({}/{})",
            agent.agent_id, score, matched_capabilities, total_required
        );

        Ok(score)
    }

    /// Check if agent capability matches required capability
    fn capability_matches(&self, required: &str, agent_capability: &AgentCapability) -> bool {
        let required_lower = required.to_lowercase();
        let capability_id_lower = agent_capability.capability_id.to_lowercase();
        let capability_name_lower = agent_capability.name.to_lowercase();
        let capability_category_lower = agent_capability.category.to_lowercase();

        // Exact matches
        if required_lower == capability_id_lower
            || required_lower == capability_name_lower
            || required_lower == capability_category_lower
        {
            return true;
        }

        // Substring matches
        if capability_id_lower.contains(&required_lower)
            || capability_name_lower.contains(&required_lower)
            || capability_category_lower.contains(&required_lower)
            || required_lower.contains(&capability_id_lower)
            || required_lower.contains(&capability_name_lower)
        {
            return true;
        }

        false
    }

    /// Calculate domain expertise score
    fn calculate_domain_score(&self, task: &Task, agent: &AgentMetadata) -> RegistryResult<f64> {
        if task.required_domains.is_empty() {
            return Ok(1.0);
        }

        let mut matched_domains = 0;
        let total_required = task.required_domains.len();

        // Check agent's knowledge domains
        for required_domain in &task.required_domains {
            for agent_domain in &agent.knowledge_context.domains {
                if self.domain_matches(required_domain, agent_domain) {
                    matched_domains += 1;
                    break;
                }
            }
        }

        // Also check role-specific domains
        if matched_domains < total_required {
            for required_domain in &task.required_domains {
                for role in agent.get_all_roles() {
                    for role_domain in &role.knowledge_domains {
                        if self.domain_matches(required_domain, role_domain) {
                            matched_domains += 1;
                            break;
                        }
                    }
                    if matched_domains >= total_required {
                        break;
                    }
                }
            }
        }

        let score = (matched_domains.min(total_required)) as f64 / total_required as f64;
        debug!(
            "Domain score for agent {}: {:.2} ({}/{})",
            agent.agent_id, score, matched_domains, total_required
        );

        Ok(score)
    }

    /// Check if agent domain matches required domain
    fn domain_matches(&self, required: &str, agent_domain: &str) -> bool {
        let required_lower = required.to_lowercase();
        let agent_domain_lower = agent_domain.to_lowercase();

        // Exact match
        if required_lower == agent_domain_lower {
            return true;
        }

        // Substring matches
        if agent_domain_lower.contains(&required_lower)
            || required_lower.contains(&agent_domain_lower)
        {
            return true;
        }

        false
    }

    /// Calculate complexity handling score
    fn calculate_complexity_score(
        &self,
        task: &Task,
        agent: &AgentMetadata,
    ) -> RegistryResult<f64> {
        // Simple heuristic based on agent experience and task complexity
        let agent_experience = agent.get_experience_level();
        let complexity_factor = match task.complexity {
            TaskComplexity::Simple => 0.2,
            TaskComplexity::Moderate => 0.5,
            TaskComplexity::Complex => 0.8,
            TaskComplexity::VeryComplex => 1.0,
        };

        // Agents with higher experience can handle more complex tasks better
        let score = if agent_experience >= complexity_factor {
            1.0
        } else {
            agent_experience / complexity_factor
        };

        debug!(
            "Complexity score for agent {} (exp: {:.2}, complexity: {:?}): {:.2}",
            agent.agent_id, agent_experience, task.complexity, score
        );

        Ok(score)
    }

    /// Generate explanation for the match
    fn generate_match_explanation(
        &self,
        task: &Task,
        agent: &AgentMetadata,
        score_breakdown: &TaskMatchScoreBreakdown,
    ) -> String {
        let mut explanations = Vec::new();

        if score_breakdown.capability_score > 0.8 {
            explanations.push("excellent capability match".to_string());
        } else if score_breakdown.capability_score > 0.6 {
            explanations.push("good capability match".to_string());
        } else if score_breakdown.capability_score > 0.3 {
            explanations.push("partial capability match".to_string());
        }

        if score_breakdown.domain_score > 0.8 {
            explanations.push("strong domain expertise".to_string());
        } else if score_breakdown.domain_score > 0.6 {
            explanations.push("relevant domain knowledge".to_string());
        }

        if score_breakdown.connectivity_score > 0.7 {
            explanations.push("high knowledge graph connectivity".to_string());
        } else if score_breakdown.connectivity_score > 0.5 {
            explanations.push("moderate knowledge connectivity".to_string());
        }

        if score_breakdown.performance_score > 0.8 {
            explanations.push("excellent performance history".to_string());
        } else if score_breakdown.performance_score > 0.6 {
            explanations.push("good performance record".to_string());
        }

        if explanations.is_empty() {
            format!(
                "Agent {} has basic compatibility with task {}",
                agent.agent_id, task.task_id
            )
        } else {
            format!(
                "Agent {} matches task {} with: {}",
                agent.agent_id,
                task.task_id,
                explanations.join(", ")
            )
        }
    }

    /// Estimate task completion time for agent
    fn estimate_completion_time(
        &self,
        task: &Task,
        agent: &AgentMetadata,
        match_score: f64,
    ) -> Option<std::time::Duration> {
        // Simple heuristic based on task effort, agent performance, and match quality
        let base_time = std::time::Duration::from_secs((task.estimated_effort * 3600.0) as u64);
        let performance_factor = agent.get_success_rate().max(0.1); // Avoid division by zero
        let match_factor = match_score.max(0.1);

        let adjusted_time = base_time.mul_f64(1.0 / (performance_factor * match_factor));

        Some(adjusted_time)
    }
}

#[async_trait]
impl KnowledgeGraphAgentMatcher for TerraphimKnowledgeGraphMatcher {
    async fn match_task_to_agents(
        &self,
        task: &Task,
        available_agents: &[AgentMetadata],
        max_matches: usize,
    ) -> RegistryResult<Vec<AgentTaskMatch>> {
        info!(
            "Matching task {} to {} available agents",
            task.task_id,
            available_agents.len()
        );

        let mut matches = Vec::new();

        // Extract task context for connectivity analysis
        let task_context = self.extract_task_context(task).await?;
        let task_concepts: Vec<String> = [task.concepts.clone(), task_context].concat();

        for agent in available_agents {
            // Skip unavailable agents
            if !matches!(
                agent.status,
                crate::AgentStatus::Active | crate::AgentStatus::Idle | crate::AgentStatus::Busy
            ) {
                continue;
            }

            // Calculate individual scores
            let capability_score = self.calculate_capability_score(task, agent)?;
            let domain_score = self.calculate_domain_score(task, agent)?;
            let complexity_score = self.calculate_complexity_score(task, agent)?;
            let performance_score = agent.get_success_rate();
            let availability_score = match agent.status {
                crate::AgentStatus::Active | crate::AgentStatus::Idle => 1.0,
                crate::AgentStatus::Busy => 0.5,
                _ => 0.0,
            };

            // Analyze knowledge graph connectivity
            let agent_concepts: Vec<String> = [
                agent.knowledge_context.concepts.clone(),
                agent.knowledge_context.keywords.clone(),
            ]
            .concat();

            let connectivity_score = self
                .analyze_connectivity(&task_concepts, &agent_concepts)
                .await?;

            // Calculate overall match score
            let match_score = capability_score * self.config.capability_weight
                + domain_score * self.config.domain_weight
                + connectivity_score * self.config.connectivity_weight
                + performance_score * self.config.performance_weight;

            // Apply minimum connectivity threshold
            if connectivity_score < self.config.min_connectivity_threshold {
                debug!(
                    "Agent {} filtered out due to low connectivity: {:.2}",
                    agent.agent_id, connectivity_score
                );
                continue;
            }

            let score_breakdown = TaskMatchScoreBreakdown {
                capability_score,
                domain_score,
                connectivity_score,
                availability_score,
                performance_score,
                complexity_score,
            };

            let explanation = self.generate_match_explanation(task, agent, &score_breakdown);
            let estimated_completion_time = self.estimate_completion_time(task, agent, match_score);

            let confidence = (match_score + connectivity_score) / 2.0;

            matches.push(AgentTaskMatch {
                agent: agent.clone(),
                task: task.clone(),
                match_score,
                score_breakdown,
                explanation,
                confidence,
                estimated_completion_time,
            });
        }

        // Sort by match score (highest first)
        matches.sort_by(|a, b| {
            b.match_score
                .partial_cmp(&a.match_score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        // Limit results
        matches.truncate(max_matches);

        info!(
            "Found {} matches for task {} (from {} agents)",
            matches.len(),
            task.task_id,
            available_agents.len()
        );

        Ok(matches)
    }

    async fn assess_agent_capability(
        &self,
        agent: &AgentMetadata,
        task: &Task,
    ) -> RegistryResult<f64> {
        let capability_score = self.calculate_capability_score(task, agent)?;
        let domain_score = self.calculate_domain_score(task, agent)?;
        let complexity_score = self.calculate_complexity_score(task, agent)?;

        // Extract concepts for connectivity analysis
        let task_context = self.extract_task_context(task).await?;
        let task_concepts: Vec<String> = [task.concepts.clone(), task_context].concat();
        let agent_concepts: Vec<String> = [
            agent.knowledge_context.concepts.clone(),
            agent.knowledge_context.keywords.clone(),
        ]
        .concat();

        let connectivity_score = self
            .analyze_connectivity(&task_concepts, &agent_concepts)
            .await?;

        // Weighted average of all capability factors
        let overall_capability = capability_score * 0.3
            + domain_score * 0.3
            + connectivity_score * 0.25
            + complexity_score * 0.15;

        debug!(
            "Agent {} capability assessment for task {}: {:.2}",
            agent.agent_id, task.task_id, overall_capability
        );

        Ok(overall_capability)
    }

    async fn coordinate_workflow(
        &self,
        tasks: &[Task],
        available_agents: &[AgentMetadata],
    ) -> RegistryResult<CoordinationResult> {
        info!(
            "Coordinating workflow with {} tasks and {} agents",
            tasks.len(),
            available_agents.len()
        );

        let workflow_id = format!("workflow_{}", uuid::Uuid::new_v4());
        let mut steps = Vec::new();
        let mut agent_assignments: HashMap<String, Vec<String>> = HashMap::new();
        let mut total_duration = std::time::Duration::ZERO;
        let mut bottlenecks = Vec::new();

        // Match each task to the best available agent
        for (i, task) in tasks.iter().enumerate() {
            let matches = self.match_task_to_agents(task, available_agents, 1).await?;

            if let Some(best_match) = matches.first() {
                let step_id = format!("step_{}", i + 1);
                let assigned_agent = best_match.agent.agent_id.to_string();

                // Calculate dependencies (simplified - sequential for now)
                let dependencies = if i > 0 {
                    vec![format!("step_{}", i)]
                } else {
                    Vec::new()
                };

                let estimated_duration = best_match
                    .estimated_completion_time
                    .unwrap_or(std::time::Duration::from_secs(3600));

                let step = CoordinationStep {
                    step_id: step_id.clone(),
                    description: task.description.clone(),
                    assigned_agent: assigned_agent.clone(),
                    dependencies,
                    estimated_duration,
                    status: StepStatus::Pending,
                };

                steps.push(step);

                // Track agent assignments
                agent_assignments
                    .entry(assigned_agent)
                    .or_default()
                    .push(step_id);

                // Update total duration (sequential execution for now)
                total_duration += estimated_duration;
            } else {
                bottlenecks.push(format!(
                    "No suitable agent found for task: {}",
                    task.description
                ));
            }
        }

        // Calculate parallelism factor (simplified)
        let parallelism_factor = if !tasks.is_empty() {
            let unique_agents = agent_assignments.keys().len();
            (unique_agents as f64 / tasks.len() as f64).min(1.0)
        } else {
            1.0
        };

        // Adjust total duration based on parallelism
        if parallelism_factor > 0.0 {
            total_duration = total_duration.mul_f64(1.0 / parallelism_factor);
        }

        let result = CoordinationResult {
            workflow_id,
            steps,
            agent_assignments,
            estimated_duration: total_duration,
            parallelism_factor,
            bottlenecks,
        };

        info!(
            "Workflow coordination complete: {} steps, {:.1}% parallelism, {} bottlenecks",
            result.steps.len(),
            result.parallelism_factor * 100.0,
            result.bottlenecks.len()
        );

        Ok(result)
    }

    async fn monitor_progress(
        &self,
        workflow_id: &str,
        coordination: &CoordinationResult,
    ) -> RegistryResult<Vec<String>> {
        debug!("Monitoring progress for workflow: {}", workflow_id);

        let mut issues = Vec::new();

        // Check for blocked steps
        let completed_steps: HashSet<String> = coordination
            .steps
            .iter()
            .filter(|step| step.status == StepStatus::Completed)
            .map(|step| step.step_id.clone())
            .collect();

        for step in &coordination.steps {
            if step.status == StepStatus::Pending {
                // Check if all dependencies are completed
                let dependencies_met = step
                    .dependencies
                    .iter()
                    .all(|dep| completed_steps.contains(dep));

                if !dependencies_met {
                    issues.push(format!(
                        "Step {} is blocked waiting for dependencies: {:?}",
                        step.step_id, step.dependencies
                    ));
                }
            }
        }

        // Check for overloaded agents
        for (agent_id, assigned_steps) in &coordination.agent_assignments {
            if assigned_steps.len() > 3 {
                // Arbitrary threshold
                issues.push(format!(
                    "Agent {} may be overloaded with {} assigned steps",
                    agent_id,
                    assigned_steps.len()
                ));
            }
        }

        // Check for long-running steps
        let in_progress_steps: Vec<&CoordinationStep> = coordination
            .steps
            .iter()
            .filter(|step| step.status == StepStatus::InProgress)
            .collect();

        if in_progress_steps.len() > coordination.steps.len() / 2 {
            issues.push("Many steps are currently in progress - potential bottleneck".to_string());
        }

        debug!(
            "Progress monitoring found {} issues for workflow {}",
            issues.len(),
            workflow_id
        );

        Ok(issues)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{AgentMetadata, AgentRole, AgentStatus, CapabilityMetrics};
    use std::sync::Arc;

    fn create_test_automata() -> Arc<Automata> {
        Arc::new(Automata::default())
    }

    async fn create_test_role_graph() -> Arc<RoleGraph> {
        use terraphim_automata::{load_thesaurus, AutomataPath};
        use terraphim_types::RoleName;

        let role_name = RoleName::new("test_role");
        let thesaurus = load_thesaurus(&AutomataPath::local_example())
            .await
            .unwrap();

        let role_graph = RoleGraph::new(role_name, thesaurus).await.unwrap();
        Arc::new(role_graph)
    }

    fn create_test_task() -> Task {
        Task {
            task_id: "test_task".to_string(),
            description: "Analyze data and create visualization".to_string(),
            required_capabilities: vec!["data_analysis".to_string(), "visualization".to_string()],
            required_domains: vec!["analytics".to_string()],
            complexity: TaskComplexity::Moderate,
            priority: 1,
            estimated_effort: 2.0,
            context_keywords: vec!["analyze".to_string(), "visualize".to_string()],
            concepts: vec!["data".to_string(), "chart".to_string()],
        }
    }

    fn create_test_agent() -> AgentMetadata {
        let agent_id = crate::AgentPid::new();
        let supervisor_id = crate::SupervisorId::new();
        let role = AgentRole::new(
            "analyst".to_string(),
            "Data Analyst".to_string(),
            "Analyzes data and creates reports".to_string(),
        );

        let mut agent = AgentMetadata::new(agent_id, supervisor_id, role);
        agent.status = AgentStatus::Active;

        // Add relevant capabilities
        agent
            .add_capability(AgentCapability {
                capability_id: "data_analysis".to_string(),
                name: "Data Analysis".to_string(),
                description: "Analyzes datasets".to_string(),
                category: "analytics".to_string(),
                required_domains: Vec::new(),
                input_types: Vec::new(),
                output_types: Vec::new(),
                performance_metrics: CapabilityMetrics::default(),
                dependencies: Vec::new(),
            })
            .unwrap();

        agent
            .add_capability(AgentCapability {
                capability_id: "visualization".to_string(),
                name: "Data Visualization".to_string(),
                description: "Creates charts and graphs".to_string(),
                category: "analytics".to_string(),
                required_domains: Vec::new(),
                input_types: Vec::new(),
                output_types: Vec::new(),
                performance_metrics: CapabilityMetrics::default(),
                dependencies: Vec::new(),
            })
            .unwrap();

        // Add domain knowledge
        agent
            .knowledge_context
            .domains
            .push("analytics".to_string());
        agent.knowledge_context.concepts.push("data".to_string());
        agent.knowledge_context.keywords.push("analyze".to_string());

        agent
    }

    #[tokio::test]
    async fn test_knowledge_graph_matcher_creation() {
        let automata = create_test_automata();
        let role_graph = create_test_role_graph().await;
        let mut role_graphs = HashMap::new();
        role_graphs.insert("test_role".to_string(), role_graph);

        let matcher = TerraphimKnowledgeGraphMatcher::with_default_config(automata, role_graphs);

        assert_eq!(matcher.config.min_connectivity_threshold, 0.6);
    }

    #[tokio::test]
    async fn test_task_to_agent_matching() {
        let automata = create_test_automata();
        let role_graph = create_test_role_graph().await;
        let mut role_graphs = HashMap::new();
        role_graphs.insert("test_role".to_string(), role_graph);

        let matcher = TerraphimKnowledgeGraphMatcher::with_default_config(automata, role_graphs);

        let task = create_test_task();
        let agent = create_test_agent();
        let agents = vec![agent];

        let matches = matcher
            .match_task_to_agents(&task, &agents, 5)
            .await
            .unwrap();

        assert!(!matches.is_empty());
        assert!(matches[0].match_score > 0.0);
        assert!(matches[0].confidence > 0.0);
    }

    #[tokio::test]
    async fn test_capability_assessment() {
        let automata = create_test_automata();
        let role_graph = create_test_role_graph().await;
        let mut role_graphs = HashMap::new();
        role_graphs.insert("test_role".to_string(), role_graph);

        let matcher = TerraphimKnowledgeGraphMatcher::with_default_config(automata, role_graphs);

        let task = create_test_task();
        let agent = create_test_agent();

        let capability_score = matcher
            .assess_agent_capability(&agent, &task)
            .await
            .unwrap();

        assert!(capability_score > 0.0);
        assert!(capability_score <= 1.0);
    }

    #[tokio::test]
    async fn test_workflow_coordination() {
        let automata = create_test_automata();
        let role_graph = create_test_role_graph().await;
        let mut role_graphs = HashMap::new();
        role_graphs.insert("test_role".to_string(), role_graph);

        let matcher = TerraphimKnowledgeGraphMatcher::with_default_config(automata, role_graphs);

        let tasks = vec![create_test_task()];
        let agents = vec![create_test_agent()];

        let coordination = matcher.coordinate_workflow(&tasks, &agents).await.unwrap();

        assert!(!coordination.steps.is_empty());
        assert!(!coordination.agent_assignments.is_empty());
        assert!(coordination.estimated_duration > std::time::Duration::ZERO);
    }

    #[test]
    fn test_capability_matching() {
        let automata = create_test_automata();
        let role_graph_map = HashMap::new();
        let matcher = TerraphimKnowledgeGraphMatcher::with_default_config(automata, role_graph_map);

        let capability = AgentCapability {
            capability_id: "data_analysis".to_string(),
            name: "Data Analysis".to_string(),
            description: "Analyzes data".to_string(),
            category: "analytics".to_string(),
            required_domains: Vec::new(),
            input_types: Vec::new(),
            output_types: Vec::new(),
            performance_metrics: CapabilityMetrics::default(),
            dependencies: Vec::new(),
        };

        assert!(matcher.capability_matches("data_analysis", &capability));
        assert!(matcher.capability_matches("data", &capability));
        assert!(matcher.capability_matches("analysis", &capability));
        assert!(!matcher.capability_matches("unrelated", &capability));
    }

    #[test]
    fn test_domain_matching() {
        let automata = create_test_automata();
        let role_graph_map = HashMap::new();
        let matcher = TerraphimKnowledgeGraphMatcher::with_default_config(automata, role_graph_map);

        assert!(matcher.domain_matches("analytics", "analytics"));
        assert!(matcher.domain_matches("data", "data_science"));
        assert!(matcher.domain_matches("machine_learning", "ml"));
        assert!(!matcher.domain_matches("finance", "healthcare"));
    }
}
