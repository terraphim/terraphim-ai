//! Task decomposition engine using knowledge graph analysis
//!
//! This module provides intelligent task decomposition capabilities that leverage
//! Terraphim's knowledge graph infrastructure to break down complex tasks into
//! manageable subtasks with proper dependencies and execution ordering.

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use async_trait::async_trait;
use log::{debug, info, warn};
use serde::{Deserialize, Serialize};

// use terraphim_automata::{extract_paragraphs_from_automata, is_all_terms_connected_by_path};
use terraphim_rolegraph::RoleGraph;
// use terraphim_types::Automata;

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

use crate::{Automata, MockAutomata};

use crate::{Task, TaskComplexity, TaskDecompositionError, TaskDecompositionResult, TaskId};

/// Task decomposition strategy
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum DecompositionStrategy {
    /// Decompose based on knowledge graph connectivity
    KnowledgeGraphBased,
    /// Decompose based on task complexity analysis
    ComplexityBased,
    /// Decompose based on role requirements
    RoleBased,
    /// Hybrid approach combining multiple strategies
    Hybrid,
}

/// Decomposition configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecompositionConfig {
    /// Maximum decomposition depth
    pub max_depth: u32,
    /// Minimum subtask complexity threshold
    pub min_subtask_complexity: TaskComplexity,
    /// Maximum number of subtasks per task
    pub max_subtasks_per_task: u32,
    /// Strategy to use for decomposition
    pub strategy: DecompositionStrategy,
    /// Knowledge graph similarity threshold
    pub similarity_threshold: f64,
    /// Whether to preserve task dependencies during decomposition
    pub preserve_dependencies: bool,
    /// Whether to optimize for parallel execution
    pub optimize_for_parallelism: bool,
}

impl Default for DecompositionConfig {
    fn default() -> Self {
        Self {
            max_depth: 3,
            min_subtask_complexity: TaskComplexity::Simple,
            max_subtasks_per_task: 10,
            strategy: DecompositionStrategy::Hybrid,
            similarity_threshold: 0.7,
            preserve_dependencies: true,
            optimize_for_parallelism: true,
        }
    }
}

/// Result of task decomposition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecompositionResult {
    /// Original task that was decomposed
    pub original_task: TaskId,
    /// Generated subtasks
    pub subtasks: Vec<Task>,
    /// Dependency relationships between subtasks
    pub dependencies: HashMap<TaskId, Vec<TaskId>>,
    /// Decomposition metadata
    pub metadata: DecompositionMetadata,
}

/// Metadata about the decomposition process
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecompositionMetadata {
    /// Strategy used for decomposition
    pub strategy_used: DecompositionStrategy,
    /// Decomposition depth achieved
    pub depth: u32,
    /// Number of subtasks created
    pub subtask_count: u32,
    /// Knowledge graph concepts involved
    pub concepts_analyzed: Vec<String>,
    /// Roles identified for subtasks
    pub roles_identified: Vec<String>,
    /// Decomposition confidence score (0.0 to 1.0)
    pub confidence_score: f64,
    /// Estimated parallelism factor
    pub parallelism_factor: f64,
}

/// Knowledge graph-based task decomposer
#[async_trait]
pub trait TaskDecomposer: Send + Sync {
    /// Decompose a task into subtasks
    async fn decompose_task(
        &self,
        task: &Task,
        config: &DecompositionConfig,
    ) -> TaskDecompositionResult<DecompositionResult>;

    /// Analyze task complexity for decomposition planning
    async fn analyze_complexity(&self, task: &Task) -> TaskDecompositionResult<TaskComplexity>;

    /// Validate decomposition result
    async fn validate_decomposition(
        &self,
        result: &DecompositionResult,
    ) -> TaskDecompositionResult<bool>;
}

/// Knowledge graph-based task decomposer implementation
pub struct KnowledgeGraphTaskDecomposer {
    /// Knowledge graph automata
    automata: Arc<Automata>,
    /// Role graph for role-based decomposition
    role_graph: Arc<RoleGraph>,
    /// Decomposition cache for performance
    cache: Arc<tokio::sync::RwLock<HashMap<String, DecompositionResult>>>,
}

impl KnowledgeGraphTaskDecomposer {
    /// Create a new knowledge graph task decomposer
    pub fn new(automata: Arc<Automata>, role_graph: Arc<RoleGraph>) -> Self {
        Self {
            automata,
            role_graph,
            cache: Arc::new(tokio::sync::RwLock::new(HashMap::new())),
        }
    }

    /// Extract knowledge concepts from task description
    async fn extract_task_concepts(&self, task: &Task) -> TaskDecompositionResult<Vec<String>> {
        let text = format!(
            "{} {}",
            task.description,
            task.knowledge_context.keywords.join(" ")
        );

        match extract_paragraphs_from_automata(&self.automata, &text, 10) {
            Ok(paragraphs) => {
                let concepts: Vec<String> = paragraphs
                    .into_iter()
                    .flat_map(|p| {
                        p.split_whitespace()
                            .map(|s| s.to_lowercase())
                            .collect::<Vec<_>>()
                    })
                    .collect::<HashSet<_>>()
                    .into_iter()
                    .collect();

                debug!(
                    "Extracted {} concepts from task {}",
                    concepts.len(),
                    task.task_id
                );
                Ok(concepts)
            }
            Err(e) => {
                warn!(
                    "Failed to extract concepts from task {}: {}",
                    task.task_id, e
                );
                Err(TaskDecompositionError::KnowledgeGraphError(format!(
                    "Concept extraction failed: {}",
                    e
                )))
            }
        }
    }

    /// Analyze knowledge graph connectivity for decomposition
    async fn analyze_connectivity(
        &self,
        concepts: &[String],
        _threshold: f64,
    ) -> TaskDecompositionResult<Vec<Vec<String>>> {
        let mut concept_groups = Vec::new();
        let mut processed = HashSet::new();

        for concept in concepts {
            if processed.contains(concept) {
                continue;
            }

            let mut group = vec![concept.clone()];
            processed.insert(concept.clone());

            // Find connected concepts
            for other_concept in concepts {
                if processed.contains(other_concept) {
                    continue;
                }

                match is_all_terms_connected_by_path(&self.automata, &[concept, other_concept]) {
                    Ok(connected) => {
                        if connected {
                            group.push(other_concept.clone());
                            processed.insert(other_concept.clone());
                        }
                    }
                    Err(e) => {
                        debug!(
                            "Connectivity check failed for {} -> {}: {}",
                            concept, other_concept, e
                        );
                    }
                }
            }

            if group.len() > 1 {
                concept_groups.push(group);
            }
        }

        debug!("Found {} concept groups", concept_groups.len());
        Ok(concept_groups)
    }

    /// Generate subtasks from concept groups
    async fn generate_subtasks_from_concepts(
        &self,
        _original_task: &Task,
        concept_groups: &[Vec<String>],
        config: &DecompositionConfig,
    ) -> TaskDecompositionResult<Vec<Task>> {
        let mut subtasks = Vec::new();
        let base_priority = _original_task.priority;

        for (i, group) in concept_groups.iter().enumerate() {
            if subtasks.len() >= config.max_subtasks_per_task as usize {
                break;
            }

            let subtask_id = format!("{}_{}", _original_task.task_id, i + 1);
            let description = format!(
                "Subtask of '{}' focusing on: {}",
                _original_task.description,
                group.join(", ")
            );

            let mut subtask = Task::new(
                subtask_id,
                description,
                config.min_subtask_complexity.clone(),
                base_priority,
            );

            // Set knowledge context
            subtask.knowledge_context.domains = _original_task.knowledge_context.domains.clone();
            subtask.knowledge_context.concepts = group.clone();
            subtask.knowledge_context.relationships =
                _original_task.knowledge_context.relationships.clone();
            subtask.knowledge_context.keywords = group.clone();
            subtask.knowledge_context.input_types =
                _original_task.knowledge_context.input_types.clone();
            subtask.knowledge_context.output_types =
                _original_task.knowledge_context.output_types.clone();
            subtask.knowledge_context.similarity_thresholds = _original_task
                .knowledge_context
                .similarity_thresholds
                .clone();

            // Inherit some constraints
            for constraint in &_original_task.constraints {
                use crate::TaskConstraintType;
                if matches!(
                    constraint.constraint_type,
                    TaskConstraintType::Quality | TaskConstraintType::Security
                ) {
                    subtask.add_constraint(constraint.clone())?;
                }
            }

            // Set parent goal
            subtask.parent_goal = _original_task.parent_goal.clone();

            // Estimate effort (distribute original effort)
            let effort_fraction = 1.0 / concept_groups.len() as f64;
            subtask.estimated_effort = _original_task.estimated_effort.mul_f64(effort_fraction);

            subtasks.push(subtask);
        }

        info!(
            "Generated {} subtasks for task {}",
            subtasks.len(),
            _original_task.task_id
        );
        Ok(subtasks)
    }

    /// Generate dependencies between subtasks
    async fn generate_subtask_dependencies(
        &self,
        subtasks: &[Task],
        _original_task: &Task,
        config: &DecompositionConfig,
    ) -> TaskDecompositionResult<HashMap<TaskId, Vec<TaskId>>> {
        let mut dependencies = HashMap::new();

        if !config.preserve_dependencies {
            return Ok(dependencies);
        }

        // Analyze concept relationships to determine dependencies
        for (i, subtask) in subtasks.iter().enumerate() {
            let mut deps = Vec::new();

            // Check if this subtask's concepts depend on previous subtasks' concepts
            for (j, other_subtask) in subtasks.iter().enumerate() {
                if i == j {
                    continue;
                }

                let has_dependency = self
                    .check_concept_dependency(
                        &subtask.knowledge_context.concepts,
                        &other_subtask.knowledge_context.concepts,
                    )
                    .await?;

                if has_dependency && j < i {
                    deps.push(other_subtask.task_id.clone());
                }
            }

            if !deps.is_empty() {
                dependencies.insert(subtask.task_id.clone(), deps);
            }
        }

        debug!("Generated {} dependency relationships", dependencies.len());
        Ok(dependencies)
    }

    /// Check if one set of concepts depends on another
    async fn check_concept_dependency(
        &self,
        dependent_concepts: &[String],
        prerequisite_concepts: &[String],
    ) -> TaskDecompositionResult<bool> {
        // Simple heuristic: check if any dependent concept is connected to prerequisite concepts
        for dep_concept in dependent_concepts {
            for prereq_concept in prerequisite_concepts {
                match is_all_terms_connected_by_path(&self.automata, &[prereq_concept, dep_concept])
                {
                    Ok(connected) => {
                        if connected {
                            return Ok(true);
                        }
                    }
                    Err(_) => {
                        // Ignore connectivity check errors
                        continue;
                    }
                }
            }
        }

        Ok(false)
    }

    /// Calculate decomposition confidence score
    fn calculate_confidence_score(
        &self,
        original_task: &Task,
        subtasks: &[Task],
        concept_groups: &[Vec<String>],
    ) -> f64 {
        let mut score = 0.0;

        // Factor 1: Concept coverage (how well subtasks cover original concepts)
        let original_concepts: HashSet<String> = original_task
            .knowledge_context
            .concepts
            .iter()
            .cloned()
            .collect();
        let subtask_concepts: HashSet<String> = subtasks
            .iter()
            .flat_map(|t| t.knowledge_context.concepts.iter().cloned())
            .collect();

        let coverage = if original_concepts.is_empty() {
            1.0
        } else {
            subtask_concepts.intersection(&original_concepts).count() as f64
                / original_concepts.len() as f64
        };

        score += coverage * 0.4;

        // Factor 2: Decomposition balance (how evenly concepts are distributed)
        let concept_distribution = concept_groups.iter().map(|g| g.len()).collect::<Vec<_>>();

        let mean_size =
            concept_distribution.iter().sum::<usize>() as f64 / concept_distribution.len() as f64;
        let variance = concept_distribution
            .iter()
            .map(|&size| (size as f64 - mean_size).powi(2))
            .sum::<f64>()
            / concept_distribution.len() as f64;

        let balance_score = 1.0 / (1.0 + variance);
        score += balance_score * 0.3;

        // Factor 3: Complexity appropriateness
        let complexity_score = if original_task.complexity.requires_decomposition() {
            if subtasks.len() > 1 {
                1.0
            } else {
                0.5
            }
        } else if subtasks.len() <= 2 {
            1.0
        } else {
            0.7
        };

        score += complexity_score * 0.3;

        score.min(1.0).max(0.0)
    }

    /// Calculate parallelism factor
    fn calculate_parallelism_factor(&self, dependencies: &HashMap<TaskId, Vec<TaskId>>) -> f64 {
        if dependencies.is_empty() {
            return 1.0; // All tasks can run in parallel
        }

        // Simple heuristic: ratio of independent tasks to total tasks
        let total_tasks = dependencies.keys().len();
        let independent_tasks = dependencies.values().filter(|deps| deps.is_empty()).count();

        if total_tasks == 0 {
            1.0
        } else {
            independent_tasks as f64 / total_tasks as f64
        }
    }
}

#[async_trait]
impl TaskDecomposer for KnowledgeGraphTaskDecomposer {
    async fn decompose_task(
        &self,
        task: &Task,
        config: &DecompositionConfig,
    ) -> TaskDecompositionResult<DecompositionResult> {
        info!("Starting decomposition of task: {}", task.task_id);

        // Check cache first
        let cache_key = format!("{}_{:?}", task.task_id, config.strategy);
        {
            let cache = self.cache.read().await;
            if let Some(cached_result) = cache.get(&cache_key) {
                debug!("Using cached decomposition for task {}", task.task_id);
                return Ok(cached_result.clone());
            }
        }

        // Extract concepts from task
        let concepts = self.extract_task_concepts(task).await?;

        if concepts.is_empty() {
            return Err(TaskDecompositionError::DecompositionFailed(
                task.task_id.clone(),
                "No concepts could be extracted from task".to_string(),
            ));
        }

        // Analyze concept connectivity
        let concept_groups = self
            .analyze_connectivity(&concepts, config.similarity_threshold)
            .await?;

        if concept_groups.is_empty() || concept_groups.len() == 1 {
            // Task doesn't need decomposition or can't be meaningfully decomposed
            let result = DecompositionResult {
                original_task: task.task_id.clone(),
                subtasks: vec![task.clone()],
                dependencies: HashMap::new(),
                metadata: DecompositionMetadata {
                    strategy_used: config.strategy.clone(),
                    depth: 0,
                    subtask_count: 1,
                    concepts_analyzed: concepts,
                    roles_identified: Vec::new(),
                    confidence_score: 0.8,
                    parallelism_factor: 1.0,
                },
            };

            return Ok(result);
        }

        // Generate subtasks
        let subtasks = self
            .generate_subtasks_from_concepts(task, &concept_groups, config)
            .await?;

        // Generate dependencies
        let dependencies = self
            .generate_subtask_dependencies(&subtasks, task, config)
            .await?;

        // Calculate metadata
        let confidence_score = self.calculate_confidence_score(task, &subtasks, &concept_groups);
        let parallelism_factor = self.calculate_parallelism_factor(&dependencies);

        let result = DecompositionResult {
            original_task: task.task_id.clone(),
            subtasks: subtasks.clone(),
            dependencies,
            metadata: DecompositionMetadata {
                strategy_used: config.strategy.clone(),
                depth: 1, // For now, we only do single-level decomposition
                subtask_count: subtasks.len() as u32,
                concepts_analyzed: concepts,
                roles_identified: Vec::new(), // TODO: Implement role identification
                confidence_score,
                parallelism_factor,
            },
        };

        // Cache the result
        {
            let mut cache = self.cache.write().await;
            cache.insert(cache_key, result.clone());
        }

        info!(
            "Completed decomposition of task {} into {} subtasks",
            task.task_id,
            result.subtasks.len()
        );

        Ok(result)
    }

    async fn analyze_complexity(&self, task: &Task) -> TaskDecompositionResult<TaskComplexity> {
        // Extract concepts to analyze complexity
        let concepts = self.extract_task_concepts(task).await?;

        let complexity = match concepts.len() {
            0..=2 => TaskComplexity::Simple,
            3..=5 => TaskComplexity::Moderate,
            6..=10 => TaskComplexity::Complex,
            _ => TaskComplexity::VeryComplex,
        };

        debug!(
            "Analyzed complexity for task {}: {:?} (based on {} concepts)",
            task.task_id,
            complexity,
            concepts.len()
        );

        Ok(complexity)
    }

    async fn validate_decomposition(
        &self,
        result: &DecompositionResult,
    ) -> TaskDecompositionResult<bool> {
        // Basic validation checks
        if result.subtasks.is_empty() {
            return Ok(false);
        }

        // Check for circular dependencies
        let mut visited = HashSet::new();
        let mut rec_stack = HashSet::new();

        for subtask in &result.subtasks {
            if self.has_circular_dependency(
                &subtask.task_id,
                &result.dependencies,
                &mut visited,
                &mut rec_stack,
            ) {
                return Ok(false);
            }
        }

        // Check confidence score threshold
        if result.metadata.confidence_score < 0.5 {
            return Ok(false);
        }

        Ok(true)
    }
}

impl KnowledgeGraphTaskDecomposer {
    /// Check for circular dependencies using DFS
    fn has_circular_dependency(
        &self,
        task_id: &str,
        dependencies: &HashMap<TaskId, Vec<TaskId>>,
        visited: &mut HashSet<String>,
        rec_stack: &mut HashSet<String>,
    ) -> bool {
        visited.insert(task_id.to_string());
        rec_stack.insert(task_id.to_string());

        if let Some(deps) = dependencies.get(task_id) {
            for dep in deps {
                if !visited.contains(dep) {
                    if self.has_circular_dependency(dep, dependencies, visited, rec_stack) {
                        return true;
                    }
                } else if rec_stack.contains(dep) {
                    return true;
                }
            }
        }

        rec_stack.remove(task_id);
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    use crate::decomposition::Automata;
    use terraphim_rolegraph::RoleGraph;

    fn create_test_automata() -> Arc<Automata> {
        // Create a simple test automata
        Arc::new(Automata::default())
    }

    async fn create_test_role_graph() -> Arc<RoleGraph> {
        use terraphim_automata::{load_thesaurus, AutomataPath};
        use terraphim_types::RoleName;

        // Use the existing test pattern from rolegraph crate
        let role_name = RoleName::new("test_role");
        let thesaurus = load_thesaurus(&AutomataPath::local_example())
            .await
            .unwrap();

        let role_graph = RoleGraph::new(role_name, thesaurus).await.unwrap();

        Arc::new(role_graph)
    }

    #[tokio::test]
    async fn test_task_decomposer_creation() {
        let automata = create_test_automata();
        let role_graph = create_test_role_graph().await;

        let decomposer = KnowledgeGraphTaskDecomposer::new(automata, role_graph);

        // Test that decomposer was created successfully
        assert!(decomposer.cache.read().await.is_empty());
    }

    #[tokio::test]
    async fn test_simple_task_decomposition() {
        let automata = create_test_automata();
        let role_graph = create_test_role_graph().await;
        let decomposer = KnowledgeGraphTaskDecomposer::new(automata, role_graph);

        let task = Task::new(
            "test_task".to_string(),
            "Simple test task".to_string(),
            TaskComplexity::Simple,
            1,
        );

        let config = DecompositionConfig::default();
        let result = decomposer.decompose_task(&task, &config).await;

        assert!(result.is_ok());
        let decomposition = result.unwrap();
        assert_eq!(decomposition.original_task, "test_task");
        assert!(!decomposition.subtasks.is_empty());
    }

    #[tokio::test]
    async fn test_complexity_analysis() {
        let automata = create_test_automata();
        let role_graph = create_test_role_graph().await;
        let decomposer = KnowledgeGraphTaskDecomposer::new(automata, role_graph);

        let simple_task = Task::new(
            "simple".to_string(),
            "Simple task".to_string(),
            TaskComplexity::Simple,
            1,
        );

        let result = decomposer.analyze_complexity(&simple_task).await;
        assert!(result.is_ok());
    }

    #[test]
    fn test_decomposition_config_defaults() {
        let config = DecompositionConfig::default();

        assert_eq!(config.max_depth, 3);
        assert_eq!(config.min_subtask_complexity, TaskComplexity::Simple);
        assert_eq!(config.max_subtasks_per_task, 10);
        assert_eq!(config.strategy, DecompositionStrategy::Hybrid);
        assert_eq!(config.similarity_threshold, 0.7);
        assert!(config.preserve_dependencies);
        assert!(config.optimize_for_parallelism);
    }
}
