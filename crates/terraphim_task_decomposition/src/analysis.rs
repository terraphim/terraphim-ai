//! Task analysis and complexity assessment
//!
//! This module provides sophisticated task analysis capabilities that leverage
//! knowledge graph traversal to assess task complexity, identify required
//! capabilities, and provide insights for optimal task decomposition.

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

/// Task analysis result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskAnalysis {
    /// Task being analyzed
    pub task_id: TaskId,
    /// Assessed complexity level
    pub complexity: TaskComplexity,
    /// Required capabilities identified
    pub required_capabilities: Vec<String>,
    /// Knowledge domains involved
    pub knowledge_domains: Vec<String>,
    /// Complexity factors that influenced the assessment
    pub complexity_factors: Vec<ComplexityFactor>,
    /// Recommended decomposition strategy
    pub recommended_strategy: Option<String>,
    /// Analysis confidence score (0.0 to 1.0)
    pub confidence_score: f64,
    /// Estimated effort in hours
    pub estimated_effort_hours: f64,
    /// Risk factors identified
    pub risk_factors: Vec<RiskFactor>,
}

/// Factors that contribute to task complexity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplexityFactor {
    /// Factor name
    pub name: String,
    /// Factor description
    pub description: String,
    /// Impact on complexity (0.0 to 1.0)
    pub impact: f64,
    /// Factor category
    pub category: ComplexityCategory,
}

/// Categories of complexity factors
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum ComplexityCategory {
    /// Knowledge graph connectivity complexity
    KnowledgeConnectivity,
    /// Domain expertise requirements
    DomainExpertise,
    /// Technical implementation complexity
    Technical,
    /// Coordination and communication complexity
    Coordination,
    /// Resource and constraint complexity
    Resources,
    /// Temporal and scheduling complexity
    Temporal,
}

/// Risk factors that may affect task execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskFactor {
    /// Risk name
    pub name: String,
    /// Risk description
    pub description: String,
    /// Risk probability (0.0 to 1.0)
    pub probability: f64,
    /// Risk impact if it occurs (0.0 to 1.0)
    pub impact: f64,
    /// Risk category
    pub category: RiskCategory,
    /// Suggested mitigation strategies
    pub mitigation_strategies: Vec<String>,
}

/// Categories of risk factors
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum RiskCategory {
    /// Technical risks
    Technical,
    /// Resource availability risks
    Resource,
    /// Knowledge and expertise risks
    Knowledge,
    /// Dependency and coordination risks
    Dependency,
    /// External factor risks
    External,
}

/// Configuration for task analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisConfig {
    /// Minimum confidence threshold for analysis results
    pub min_confidence_threshold: f64,
    /// Maximum number of concepts to analyze
    pub max_concepts: u32,
    /// Knowledge graph traversal depth
    pub traversal_depth: u32,
    /// Whether to include risk analysis
    pub include_risk_analysis: bool,
    /// Whether to analyze role requirements
    pub analyze_role_requirements: bool,
    /// Complexity assessment sensitivity (0.0 to 1.0)
    pub complexity_sensitivity: f64,
}

impl Default for AnalysisConfig {
    fn default() -> Self {
        Self {
            min_confidence_threshold: 0.6,
            max_concepts: 50,
            traversal_depth: 3,
            include_risk_analysis: true,
            analyze_role_requirements: true,
            complexity_sensitivity: 0.7,
        }
    }
}

/// Task analyzer trait
#[async_trait]
pub trait TaskAnalyzer: Send + Sync {
    /// Analyze a task and assess its complexity
    async fn analyze_task(
        &self,
        task: &Task,
        config: &AnalysisConfig,
    ) -> TaskDecompositionResult<TaskAnalysis>;

    /// Analyze multiple tasks and identify relationships
    async fn analyze_task_batch(
        &self,
        tasks: &[Task],
        config: &AnalysisConfig,
    ) -> TaskDecompositionResult<Vec<TaskAnalysis>>;

    /// Compare two tasks for similarity
    async fn compare_tasks(&self, task1: &Task, task2: &Task) -> TaskDecompositionResult<f64>;
}

/// Knowledge graph-based task analyzer
pub struct KnowledgeGraphTaskAnalyzer {
    /// Knowledge graph automata
    automata: Arc<Automata>,
    /// Role graph for capability analysis
    role_graph: Arc<RoleGraph>,
    /// Analysis cache for performance
    cache: tokio::sync::RwLock<HashMap<String, TaskAnalysis>>,
}

impl KnowledgeGraphTaskAnalyzer {
    /// Create a new task analyzer
    pub fn new(automata: Arc<Automata>, role_graph: Arc<RoleGraph>) -> Self {
        Self {
            automata,
            role_graph,
            cache: tokio::sync::RwLock::new(HashMap::new()),
        }
    }

    /// Extract and analyze concepts from task description
    async fn extract_and_analyze_concepts(
        &self,
        task: &Task,
        config: &AnalysisConfig,
    ) -> TaskDecompositionResult<(Vec<String>, Vec<ComplexityFactor>)> {
        let text = format!(
            "{} {} {}",
            task.description,
            task.knowledge_context.keywords.join(" "),
            task.knowledge_context.concepts.join(" ")
        );

        let concepts =
            match extract_paragraphs_from_automata(&self.automata, &text, config.max_concepts) {
                Ok(paragraphs) => paragraphs
                    .into_iter()
                    .flat_map(|p| {
                        p.split_whitespace()
                            .map(|s| s.to_lowercase())
                            .collect::<Vec<_>>()
                    })
                    .collect::<HashSet<_>>()
                    .into_iter()
                    .collect::<Vec<_>>(),
                Err(e) => {
                    warn!(
                        "Failed to extract concepts from task {}: {}",
                        task.task_id, e
                    );
                    return Err(TaskDecompositionError::AnalysisFailed(
                        task.task_id.clone(),
                        format!("Concept extraction failed: {}", e),
                    ));
                }
            };

        debug!(
            "Extracted {} concepts from task {}",
            concepts.len(),
            task.task_id
        );

        // Analyze concept connectivity to determine complexity factors
        let mut complexity_factors = Vec::new();

        // Factor 1: Concept diversity
        let concept_diversity = concepts.len() as f64 / config.max_concepts as f64;
        complexity_factors.push(ComplexityFactor {
            name: "Concept Diversity".to_string(),
            description: format!("Task involves {} distinct concepts", concepts.len()),
            impact: concept_diversity * config.complexity_sensitivity,
            category: ComplexityCategory::KnowledgeConnectivity,
        });

        // Factor 2: Knowledge graph connectivity
        let connectivity_score = self.analyze_concept_connectivity(&concepts).await?;
        complexity_factors.push(ComplexityFactor {
            name: "Knowledge Connectivity".to_string(),
            description: "Degree of interconnection between task concepts".to_string(),
            impact: connectivity_score * config.complexity_sensitivity,
            category: ComplexityCategory::KnowledgeConnectivity,
        });

        // Factor 3: Domain specialization
        let domain_specialization = self.analyze_domain_specialization(&concepts, task).await?;
        complexity_factors.push(ComplexityFactor {
            name: "Domain Specialization".to_string(),
            description: "Level of specialized domain knowledge required".to_string(),
            impact: domain_specialization * config.complexity_sensitivity,
            category: ComplexityCategory::DomainExpertise,
        });

        Ok((concepts, complexity_factors))
    }

    /// Analyze connectivity between concepts
    async fn analyze_concept_connectivity(
        &self,
        concepts: &[String],
    ) -> TaskDecompositionResult<f64> {
        if concepts.len() < 2 {
            return Ok(0.0);
        }

        let mut connected_pairs = 0;
        let mut total_pairs = 0;

        for i in 0..concepts.len() {
            for j in (i + 1)..concepts.len() {
                total_pairs += 1;

                match is_all_terms_connected_by_path(&self.automata, &[&concepts[i], &concepts[j]])
                {
                    Ok(connected) => {
                        if connected {
                            connected_pairs += 1;
                        }
                    }
                    Err(_) => {
                        // Ignore connectivity check errors
                        continue;
                    }
                }
            }
        }

        let connectivity_ratio = if total_pairs > 0 {
            connected_pairs as f64 / total_pairs as f64
        } else {
            0.0
        };

        debug!(
            "Concept connectivity: {}/{} pairs connected",
            connected_pairs, total_pairs
        );
        Ok(connectivity_ratio)
    }

    /// Analyze domain specialization requirements
    async fn analyze_domain_specialization(
        &self,
        _concepts: &[String],
        task: &Task,
    ) -> TaskDecompositionResult<f64> {
        // Simple heuristic: more domains = higher specialization
        let unique_domains: HashSet<String> =
            task.knowledge_context.domains.iter().cloned().collect();
        let domain_count = unique_domains.len();

        // Normalize by a reasonable maximum (e.g., 5 domains)
        let specialization_score = (domain_count as f64 / 5.0).min(1.0);

        debug!(
            "Domain specialization score: {} (based on {} domains)",
            specialization_score, domain_count
        );

        Ok(specialization_score)
    }

    /// Assess overall task complexity based on factors
    fn assess_complexity(&self, factors: &[ComplexityFactor]) -> TaskComplexity {
        let total_impact: f64 = factors.iter().map(|f| f.impact).sum();
        let average_impact = if factors.is_empty() {
            0.0
        } else {
            total_impact / factors.len() as f64
        };

        match average_impact {
            x if x < 0.25 => TaskComplexity::Simple,
            x if x < 0.5 => TaskComplexity::Moderate,
            x if x < 0.75 => TaskComplexity::Complex,
            _ => TaskComplexity::VeryComplex,
        }
    }

    /// Identify required capabilities from concepts and task context
    async fn identify_required_capabilities(
        &self,
        concepts: &[String],
        task: &Task,
        _config: &AnalysisConfig,
    ) -> TaskDecompositionResult<Vec<String>> {
        let mut capabilities = HashSet::new();

        // Add explicitly specified capabilities
        for capability in &task.required_capabilities {
            capabilities.insert(capability.clone());
        }

        // Infer capabilities from knowledge domains
        for domain in &task.knowledge_context.domains {
            capabilities.insert(format!("{}_expertise", domain.to_lowercase()));
        }

        // Infer capabilities from concepts (simplified heuristic)
        for concept in concepts {
            if concept.contains("analysis") || concept.contains("analyze") {
                capabilities.insert("analytical_thinking".to_string());
            }
            if concept.contains("design") || concept.contains("create") {
                capabilities.insert("design_thinking".to_string());
            }
            if concept.contains("code") || concept.contains("program") {
                capabilities.insert("programming".to_string());
            }
            if concept.contains("test") || concept.contains("verify") {
                capabilities.insert("testing".to_string());
            }
        }

        debug!(
            "Identified {} capabilities for task {}",
            capabilities.len(),
            task.task_id
        );
        Ok(capabilities.into_iter().collect())
    }

    /// Identify risk factors for the task
    async fn identify_risk_factors(
        &self,
        task: &Task,
        concepts: &[String],
        complexity: &TaskComplexity,
    ) -> TaskDecompositionResult<Vec<RiskFactor>> {
        let mut risks = Vec::new();

        // Risk 1: High complexity risk
        if matches!(
            complexity,
            TaskComplexity::Complex | TaskComplexity::VeryComplex
        ) {
            risks.push(RiskFactor {
                name: "High Complexity".to_string(),
                description: "Task complexity may lead to implementation challenges".to_string(),
                probability: 0.6,
                impact: 0.8,
                category: RiskCategory::Technical,
                mitigation_strategies: vec![
                    "Break down into smaller subtasks".to_string(),
                    "Assign experienced agents".to_string(),
                    "Increase testing and validation".to_string(),
                ],
            });
        }

        // Risk 2: Knowledge gap risk
        if concepts.len() > 10 {
            risks.push(RiskFactor {
                name: "Knowledge Breadth".to_string(),
                description: "Task requires knowledge across many concepts".to_string(),
                probability: 0.4,
                impact: 0.6,
                category: RiskCategory::Knowledge,
                mitigation_strategies: vec![
                    "Ensure diverse agent capabilities".to_string(),
                    "Provide additional context and documentation".to_string(),
                ],
            });
        }

        // Risk 3: Dependency risk
        if task.dependencies.len() > 3 {
            risks.push(RiskFactor {
                name: "High Dependencies".to_string(),
                description: "Task has many dependencies that could cause delays".to_string(),
                probability: 0.5,
                impact: 0.7,
                category: RiskCategory::Dependency,
                mitigation_strategies: vec![
                    "Monitor dependency completion closely".to_string(),
                    "Prepare alternative execution paths".to_string(),
                ],
            });
        }

        // Risk 4: Resource constraint risk
        if task.constraints.len() > 2 {
            risks.push(RiskFactor {
                name: "Resource Constraints".to_string(),
                description: "Multiple constraints may limit execution options".to_string(),
                probability: 0.3,
                impact: 0.5,
                category: RiskCategory::Resource,
                mitigation_strategies: vec![
                    "Validate resource availability early".to_string(),
                    "Plan for constraint relaxation if needed".to_string(),
                ],
            });
        }

        debug!(
            "Identified {} risk factors for task {}",
            risks.len(),
            task.task_id
        );
        Ok(risks)
    }

    /// Calculate analysis confidence score
    fn calculate_confidence_score(
        &self,
        concepts: &[String],
        factors: &[ComplexityFactor],
        task: &Task,
    ) -> f64 {
        let mut score = 0.0;

        // Factor 1: Concept extraction success
        let concept_score = if concepts.is_empty() {
            0.0
        } else {
            (concepts.len() as f64 / 20.0).min(1.0) // Normalize by expected concept count
        };
        score += concept_score * 0.4;

        // Factor 2: Complexity factor coverage
        let factor_categories: HashSet<ComplexityCategory> =
            factors.iter().map(|f| f.category.clone()).collect();
        let category_coverage = factor_categories.len() as f64 / 6.0; // 6 total categories
        score += category_coverage * 0.3;

        // Factor 3: Task context richness
        let context_richness = (task.knowledge_context.domains.len()
            + task.knowledge_context.concepts.len()
            + task.knowledge_context.keywords.len()) as f64
            / 30.0; // Normalize by expected total
        score += context_richness.min(1.0) * 0.3;

        score.min(1.0).max(0.0)
    }

    /// Estimate effort in hours based on complexity and factors
    fn estimate_effort_hours(
        &self,
        complexity: &TaskComplexity,
        factors: &[ComplexityFactor],
    ) -> f64 {
        let base_hours = match complexity {
            TaskComplexity::Simple => 2.0,
            TaskComplexity::Moderate => 8.0,
            TaskComplexity::Complex => 24.0,
            TaskComplexity::VeryComplex => 72.0,
        };

        // Adjust based on complexity factors
        let factor_multiplier = factors
            .iter()
            .map(|f| 1.0 + f.impact * 0.5) // Each factor can add up to 50% more effort
            .fold(1.0, |acc, mult| acc * mult);

        base_hours * factor_multiplier
    }
}

#[async_trait]
impl TaskAnalyzer for KnowledgeGraphTaskAnalyzer {
    async fn analyze_task(
        &self,
        task: &Task,
        config: &AnalysisConfig,
    ) -> TaskDecompositionResult<TaskAnalysis> {
        info!("Analyzing task: {}", task.task_id);

        // Check cache first
        let cache_key = format!("{}_{}", task.task_id, task.metadata.version);
        {
            let cache = self.cache.read().await;
            if let Some(cached_analysis) = cache.get(&cache_key) {
                debug!("Using cached analysis for task {}", task.task_id);
                return Ok(cached_analysis.clone());
            }
        }

        // Extract and analyze concepts
        let (concepts, complexity_factors) =
            self.extract_and_analyze_concepts(task, config).await?;

        // Assess complexity
        let complexity = self.assess_complexity(&complexity_factors);

        // Identify required capabilities
        let required_capabilities = self
            .identify_required_capabilities(&concepts, task, config)
            .await?;

        // Identify risk factors
        let risk_factors = if config.include_risk_analysis {
            self.identify_risk_factors(task, &concepts, &complexity)
                .await?
        } else {
            Vec::new()
        };

        // Calculate confidence score
        let confidence_score =
            self.calculate_confidence_score(&concepts, &complexity_factors, task);

        // Estimate effort
        let estimated_effort_hours = self.estimate_effort_hours(&complexity, &complexity_factors);

        // Extract knowledge domains
        let knowledge_domains = task.knowledge_context.domains.clone();

        let analysis = TaskAnalysis {
            task_id: task.task_id.clone(),
            complexity,
            required_capabilities,
            knowledge_domains,
            complexity_factors,
            recommended_strategy: None, // TODO: Implement strategy recommendation
            confidence_score,
            estimated_effort_hours,
            risk_factors,
        };

        // Cache the analysis
        {
            let mut cache = self.cache.write().await;
            cache.insert(cache_key, analysis.clone());
        }

        info!(
            "Completed analysis for task {}: complexity={:?}, confidence={:.2}",
            task.task_id, analysis.complexity, analysis.confidence_score
        );

        Ok(analysis)
    }

    async fn analyze_task_batch(
        &self,
        tasks: &[Task],
        config: &AnalysisConfig,
    ) -> TaskDecompositionResult<Vec<TaskAnalysis>> {
        info!("Analyzing batch of {} tasks", tasks.len());

        let mut analyses = Vec::new();
        for task in tasks {
            let analysis = self.analyze_task(task, config).await?;
            analyses.push(analysis);
        }

        info!("Completed batch analysis of {} tasks", analyses.len());
        Ok(analyses)
    }

    async fn compare_tasks(&self, task1: &Task, task2: &Task) -> TaskDecompositionResult<f64> {
        // Simple similarity based on shared concepts and domains
        let concepts1: HashSet<String> = task1.knowledge_context.concepts.iter().cloned().collect();
        let concepts2: HashSet<String> = task2.knowledge_context.concepts.iter().cloned().collect();

        let domains1: HashSet<String> = task1.knowledge_context.domains.iter().cloned().collect();
        let domains2: HashSet<String> = task2.knowledge_context.domains.iter().cloned().collect();

        let concept_intersection = concepts1.intersection(&concepts2).count();
        let concept_union = concepts1.union(&concepts2).count();

        let domain_intersection = domains1.intersection(&domains2).count();
        let domain_union = domains1.union(&domains2).count();

        let concept_similarity = if concept_union > 0 {
            concept_intersection as f64 / concept_union as f64
        } else {
            0.0
        };

        let domain_similarity = if domain_union > 0 {
            domain_intersection as f64 / domain_union as f64
        } else {
            0.0
        };

        // Weighted average (concepts are more important than domains)
        let similarity = concept_similarity * 0.7 + domain_similarity * 0.3;

        debug!(
            "Task similarity between {} and {}: {:.2}",
            task1.task_id, task2.task_id, similarity
        );

        Ok(similarity)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analysis::Automata;
    use crate::{Task, TaskComplexity};
    use std::sync::Arc;
    use terraphim_rolegraph::RoleGraph;

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
        let mut task = Task::new(
            "test_task".to_string(),
            "Analyze data and create visualization".to_string(),
            TaskComplexity::Moderate,
            1,
        );

        task.knowledge_context.domains =
            vec!["data_analysis".to_string(), "visualization".to_string()];
        task.knowledge_context.concepts = vec![
            "analysis".to_string(),
            "chart".to_string(),
            "data".to_string(),
        ];
        task.knowledge_context.keywords = vec!["analyze".to_string(), "visualize".to_string()];
        task.knowledge_context.input_types = vec!["dataset".to_string()];
        task.knowledge_context.output_types = vec!["chart".to_string()];

        task
    }

    #[tokio::test]
    async fn test_task_analyzer_creation() {
        let automata = create_test_automata();
        let role_graph = create_test_role_graph().await;

        let analyzer = KnowledgeGraphTaskAnalyzer::new(automata, role_graph);
        assert!(analyzer.cache.read().await.is_empty());
    }

    #[tokio::test]
    async fn test_task_analysis() {
        let automata = create_test_automata();
        let role_graph = create_test_role_graph().await;
        let analyzer = KnowledgeGraphTaskAnalyzer::new(automata, role_graph);

        let task = create_test_task();
        let config = AnalysisConfig::default();

        let result = analyzer.analyze_task(&task, &config).await;
        assert!(result.is_ok());

        let analysis = result.unwrap();
        assert_eq!(analysis.task_id, "test_task");
        assert!(!analysis.complexity_factors.is_empty());
        assert!(analysis.confidence_score > 0.0);
        assert!(analysis.estimated_effort_hours > 0.0);
    }

    #[tokio::test]
    async fn test_task_comparison() {
        let automata = create_test_automata();
        let role_graph = create_test_role_graph().await;
        let analyzer = KnowledgeGraphTaskAnalyzer::new(automata, role_graph);

        let task1 = create_test_task();
        let mut task2 = create_test_task();
        task2.task_id = "test_task_2".to_string();

        let similarity = analyzer.compare_tasks(&task1, &task2).await.unwrap();
        assert!(similarity > 0.8); // Should be very similar since they're nearly identical
    }

    #[tokio::test]
    async fn test_complexity_assessment() {
        let automata = create_test_automata();
        let role_graph = create_test_role_graph().await;
        let analyzer = KnowledgeGraphTaskAnalyzer::new(automata, role_graph);

        let factors = vec![ComplexityFactor {
            name: "Test Factor".to_string(),
            description: "Test".to_string(),
            impact: 0.3,
            category: ComplexityCategory::Technical,
        }];

        let complexity = analyzer.assess_complexity(&factors);
        assert_eq!(complexity, TaskComplexity::Moderate);
    }

    #[test]
    fn test_analysis_config_defaults() {
        let config = AnalysisConfig::default();
        assert_eq!(config.min_confidence_threshold, 0.6);
        assert_eq!(config.max_concepts, 50);
        assert_eq!(config.traversal_depth, 3);
        assert!(config.include_risk_analysis);
        assert!(config.analyze_role_requirements);
        assert_eq!(config.complexity_sensitivity, 0.7);
    }
}
