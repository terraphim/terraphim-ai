//! Knowledge graph integration for goal alignment
//!
//! Integrates with Terraphim's knowledge graph infrastructure to provide intelligent
//! goal analysis, conflict detection, and alignment validation.

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use serde::{Deserialize, Serialize};

use terraphim_rolegraph::RoleGraph;

use crate::{Goal, GoalAlignmentResult, GoalId};

/// Knowledge graph-based goal analysis and alignment
pub struct KnowledgeGraphGoalAnalyzer {
    /// Role graph for role-based goal propagation
    role_graph: Arc<RoleGraph>,
    /// Configuration for automata-based analysis
    automata_config: AutomataConfig,
    /// Cached analysis results for performance
    analysis_cache: Arc<tokio::sync::RwLock<HashMap<String, AnalysisResult>>>,
    /// Semantic similarity thresholds
    similarity_thresholds: SimilarityThresholds,
}

/// Configuration for automata-based goal analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutomataConfig {
    /// Minimum confidence threshold for concept extraction
    pub min_confidence: f64,
    /// Maximum number of paragraphs to extract
    pub max_paragraphs: usize,
    /// Context window size for analysis
    pub context_window: usize,
    /// Language models to use
    pub language_models: Vec<String>,
}

impl Default for AutomataConfig {
    fn default() -> Self {
        Self {
            min_confidence: 0.75,
            max_paragraphs: 15,
            context_window: 1024,
            language_models: vec!["default".to_string()],
        }
    }
}

/// Similarity thresholds for goal analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimilarityThresholds {
    /// Goal concept similarity threshold
    pub concept_similarity: f64,
    /// Goal domain similarity threshold
    pub domain_similarity: f64,
    /// Goal relationship similarity threshold
    pub relationship_similarity: f64,
    /// Conflict detection threshold
    pub conflict_threshold: f64,
}

impl Default for SimilarityThresholds {
    fn default() -> Self {
        Self {
            concept_similarity: 0.8,
            domain_similarity: 0.75,
            relationship_similarity: 0.7,
            conflict_threshold: 0.6,
        }
    }
}

/// Cached analysis result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisResult {
    /// Analysis hash for cache key
    pub analysis_hash: String,
    /// Extracted concepts and relationships
    pub concepts: Vec<String>,
    /// Connectivity analysis results
    pub connectivity: ConnectivityResult,
    /// Semantic analysis results
    pub semantic_analysis: SemanticAnalysis,
    /// Timestamp when cached
    pub cached_at: chrono::DateTime<chrono::Utc>,
    /// Cache expiry time
    pub expires_at: chrono::DateTime<chrono::Utc>,
}

/// Result of connectivity analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectivityResult {
    /// Whether all concepts are connected
    pub all_connected: bool,
    /// Connection paths found
    pub paths: Vec<Vec<String>>,
    /// Disconnected concepts
    pub disconnected: Vec<String>,
    /// Connection strength score
    pub strength_score: f64,
    /// Suggested connections
    pub suggested_connections: Vec<(String, String, f64)>,
}

/// Semantic analysis of goals
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SemanticAnalysis {
    /// Primary semantic domains
    pub primary_domains: Vec<String>,
    /// Secondary semantic domains
    pub secondary_domains: Vec<String>,
    /// Key concepts identified
    pub key_concepts: Vec<String>,
    /// Semantic relationships
    pub relationships: Vec<SemanticRelationship>,
    /// Complexity score
    pub complexity_score: f64,
}

/// Semantic relationship between concepts
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SemanticRelationship {
    /// Source concept
    pub source: String,
    /// Target concept
    pub target: String,
    /// Relationship type
    pub relationship_type: String,
    /// Relationship strength
    pub strength: f64,
}

/// Goal alignment analysis request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GoalAlignmentAnalysis {
    /// Goals to analyze
    pub goals: Vec<Goal>,
    /// Analysis type
    pub analysis_type: AnalysisType,
    /// Additional context
    pub context: HashMap<String, serde_json::Value>,
}

/// Types of goal analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AnalysisType {
    /// Analyze goal hierarchy consistency
    HierarchyConsistency,
    /// Detect goal conflicts
    ConflictDetection,
    /// Validate goal connectivity
    ConnectivityValidation,
    /// Analyze goal propagation paths
    PropagationAnalysis,
    /// Comprehensive analysis (all types)
    Comprehensive,
}

/// Goal alignment analysis result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GoalAlignmentAnalysisResult {
    /// Analysis results by goal
    pub goal_analyses: HashMap<GoalId, GoalAnalysisResult>,
    /// Overall alignment score
    pub overall_alignment_score: f64,
    /// Detected conflicts
    pub conflicts: Vec<GoalConflict>,
    /// Connectivity issues
    pub connectivity_issues: Vec<ConnectivityIssue>,
    /// Recommendations
    pub recommendations: Vec<AlignmentRecommendation>,
}

/// Analysis result for individual goal
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GoalAnalysisResult {
    /// Goal being analyzed
    pub goal_id: GoalId,
    /// Semantic analysis
    pub semantic_analysis: SemanticAnalysis,
    /// Connectivity analysis
    pub connectivity: ConnectivityResult,
    /// Alignment score with other goals
    pub alignment_scores: HashMap<GoalId, f64>,
    /// Potential conflicts
    pub potential_conflicts: Vec<GoalId>,
}

/// Detected goal conflict
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GoalConflict {
    /// First conflicting goal
    pub goal1: GoalId,
    /// Second conflicting goal
    pub goal2: GoalId,
    /// Conflict type
    pub conflict_type: ConflictType,
    /// Conflict severity (0.0 to 1.0)
    pub severity: f64,
    /// Conflict description
    pub description: String,
    /// Suggested resolutions
    pub suggested_resolutions: Vec<String>,
}

/// Types of goal conflicts
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ConflictType {
    /// Resource conflicts
    Resource,
    /// Temporal conflicts
    Temporal,
    /// Semantic conflicts
    Semantic,
    /// Priority conflicts
    Priority,
    /// Dependency conflicts
    Dependency,
    /// Constraint conflicts
    Constraint,
}

/// Connectivity issue
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectivityIssue {
    /// Goal with connectivity issue
    pub goal_id: GoalId,
    /// Issue type
    pub issue_type: ConnectivityIssueType,
    /// Issue description
    pub description: String,
    /// Suggested fixes
    pub suggested_fixes: Vec<String>,
}

/// Types of connectivity issues
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConnectivityIssueType {
    /// Disconnected concepts
    DisconnectedConcepts,
    /// Weak connections
    WeakConnections,
    /// Missing relationships
    MissingRelationships,
    /// Circular dependencies
    CircularDependencies,
}

/// Alignment recommendation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlignmentRecommendation {
    /// Recommendation type
    pub recommendation_type: RecommendationType,
    /// Target goal(s)
    pub target_goals: Vec<GoalId>,
    /// Recommendation description
    pub description: String,
    /// Expected impact
    pub expected_impact: f64,
    /// Implementation priority
    pub priority: u32,
}

/// Types of alignment recommendations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RecommendationType {
    /// Modify goal description
    ModifyDescription,
    /// Add goal constraints
    AddConstraints,
    /// Adjust goal priorities
    AdjustPriorities,
    /// Restructure goal hierarchy
    RestructureHierarchy,
    /// Add goal dependencies
    AddDependencies,
    /// Merge similar goals
    MergeGoals,
    /// Split complex goals
    SplitGoals,
}

impl KnowledgeGraphGoalAnalyzer {
    /// Create new knowledge graph goal analyzer
    pub fn new(
        role_graph: Arc<RoleGraph>,
        automata_config: AutomataConfig,
        similarity_thresholds: SimilarityThresholds,
    ) -> Self {
        Self {
            role_graph,
            automata_config,
            analysis_cache: Arc::new(tokio::sync::RwLock::new(HashMap::new())),
            similarity_thresholds,
        }
    }

    /// Analyze goal alignment using knowledge graph
    pub async fn analyze_goal_alignment(
        &self,
        analysis: GoalAlignmentAnalysis,
    ) -> GoalAlignmentResult<GoalAlignmentAnalysisResult> {
        let mut goal_analyses = HashMap::new();
        let mut conflicts = Vec::new();
        let mut connectivity_issues = Vec::new();

        // Analyze each goal individually
        for goal in &analysis.goals {
            let goal_analysis = self.analyze_individual_goal(goal, &analysis.goals).await?;

            // Check for conflicts with other goals
            for other_goal in &analysis.goals {
                if goal.goal_id != other_goal.goal_id {
                    if let Some(conflict) = self.detect_goal_conflict(goal, other_goal).await? {
                        conflicts.push(conflict);
                    }
                }
            }

            // Check connectivity issues
            if let Some(issue) = self.check_goal_connectivity(goal).await? {
                connectivity_issues.push(issue);
            }

            goal_analyses.insert(goal.goal_id.clone(), goal_analysis);
        }

        // Calculate overall alignment score
        let overall_alignment_score = self.calculate_overall_alignment_score(&goal_analyses);

        // Generate recommendations
        let recommendations = self
            .generate_alignment_recommendations(&analysis.goals, &conflicts, &connectivity_issues)
            .await?;

        Ok(GoalAlignmentAnalysisResult {
            goal_analyses,
            overall_alignment_score,
            conflicts,
            connectivity_issues,
            recommendations,
        })
    }

    /// Analyze individual goal using knowledge graph
    async fn analyze_individual_goal(
        &self,
        goal: &Goal,
        all_goals: &[Goal],
    ) -> GoalAlignmentResult<GoalAnalysisResult> {
        // Extract concepts from goal description
        let concepts = self.extract_goal_concepts(goal).await?;

        // Perform semantic analysis
        let semantic_analysis = self.perform_semantic_analysis(goal, &concepts).await?;

        // Analyze connectivity
        let connectivity = self.analyze_goal_connectivity(goal, &concepts).await?;

        // Calculate alignment scores with other goals
        let mut alignment_scores = HashMap::new();
        let mut potential_conflicts = Vec::new();

        for other_goal in all_goals {
            if goal.goal_id != other_goal.goal_id {
                let alignment_score = self
                    .calculate_goal_alignment_score(goal, other_goal)
                    .await?;
                alignment_scores.insert(other_goal.goal_id.clone(), alignment_score);

                if alignment_score < self.similarity_thresholds.conflict_threshold {
                    potential_conflicts.push(other_goal.goal_id.clone());
                }
            }
        }

        Ok(GoalAnalysisResult {
            goal_id: goal.goal_id.clone(),
            semantic_analysis,
            connectivity,
            alignment_scores,
            potential_conflicts,
        })
    }

    /// Extract concepts from goal using automata
    async fn extract_goal_concepts(&self, goal: &Goal) -> GoalAlignmentResult<Vec<String>> {
        // Check cache first
        let cache_key = format!("concepts_{}", goal.goal_id);
        {
            let cache = self.analysis_cache.read().await;
            if let Some(cached_result) = cache.get(&cache_key) {
                if cached_result.expires_at > chrono::Utc::now() {
                    return Ok(cached_result.concepts.clone());
                }
            }
        }

        // TODO: Re-enable automata-based concept extraction when thesaurus is available
        // For now, use simple keyword extraction
        let mut concepts = HashSet::new();

        // Extract concepts from description
        let words: Vec<&str> = goal.description.split_whitespace().collect();
        for word in words {
            if word.len() > 3 && !word.chars().all(|c| c.is_ascii_punctuation()) {
                concepts.insert(word.to_lowercase());
            }
        }

        // Add existing knowledge context
        concepts.extend(
            goal.knowledge_context
                .concepts
                .iter()
                .map(|c| c.to_lowercase()),
        );
        concepts.extend(
            goal.knowledge_context
                .domains
                .iter()
                .map(|d| d.to_lowercase()),
        );

        let concept_list: Vec<String> = concepts.into_iter().collect();

        // Cache the result
        {
            let mut cache = self.analysis_cache.write().await;
            cache.insert(
                cache_key,
                AnalysisResult {
                    analysis_hash: format!("concepts_{}", goal.goal_id),
                    concepts: concept_list.clone(),
                    connectivity: ConnectivityResult {
                        all_connected: true,
                        paths: Vec::new(),
                        disconnected: Vec::new(),
                        strength_score: 1.0,
                        suggested_connections: Vec::new(),
                    },
                    semantic_analysis: SemanticAnalysis {
                        primary_domains: Vec::new(),
                        secondary_domains: Vec::new(),
                        key_concepts: concept_list.clone(),
                        relationships: Vec::new(),
                        complexity_score: 0.5,
                    },
                    cached_at: chrono::Utc::now(),
                    expires_at: chrono::Utc::now() + chrono::Duration::hours(2),
                },
            );
        }

        Ok(concept_list)
    }

    /// Perform semantic analysis of goal
    async fn perform_semantic_analysis(
        &self,
        goal: &Goal,
        concepts: &[String],
    ) -> GoalAlignmentResult<SemanticAnalysis> {
        // Identify primary and secondary domains
        let primary_domains = goal.knowledge_context.domains.clone();
        let mut secondary_domains = Vec::new();

        // TODO: Re-enable role graph domain extraction when get_role() API is available
        // For now, use existing knowledge context domains
        secondary_domains.extend(
            goal.knowledge_context
                .keywords
                .iter()
                .filter(|k| !primary_domains.contains(k))
                .cloned(),
        );

        // Identify key concepts (most frequent or important)
        let key_concepts = concepts
            .iter()
            .take(10) // Take top 10 concepts
            .cloned()
            .collect();

        // Identify semantic relationships
        let relationships = self.identify_semantic_relationships(concepts).await?;

        // Calculate complexity score based on number of concepts and relationships
        let complexity_score = (concepts.len() as f64 * 0.1 + relationships.len() as f64 * 0.2)
            .min(1.0)
            .max(0.0);

        Ok(SemanticAnalysis {
            primary_domains,
            secondary_domains,
            key_concepts,
            relationships,
            complexity_score,
        })
    }

    /// Identify semantic relationships between concepts
    async fn identify_semantic_relationships(
        &self,
        concepts: &[String],
    ) -> GoalAlignmentResult<Vec<SemanticRelationship>> {
        let mut relationships = Vec::new();

        // Simple relationship identification based on concept co-occurrence
        for (i, concept1) in concepts.iter().enumerate() {
            for concept2 in concepts.iter().skip(i + 1) {
                // Calculate relationship strength based on semantic similarity
                let strength = self.calculate_concept_similarity(concept1, concept2);

                if strength > self.similarity_thresholds.relationship_similarity {
                    relationships.push(SemanticRelationship {
                        source: concept1.clone(),
                        target: concept2.clone(),
                        relationship_type: "related_to".to_string(),
                        strength,
                    });
                }
            }
        }

        Ok(relationships)
    }

    /// Calculate semantic similarity between concepts
    fn calculate_concept_similarity(&self, concept1: &str, concept2: &str) -> f64 {
        // Simple string-based similarity for now
        // In practice, this would use more sophisticated semantic similarity measures
        let c1_lower = concept1.to_lowercase();
        let c2_lower = concept2.to_lowercase();

        if c1_lower == c2_lower {
            return 1.0;
        }

        if c1_lower.contains(&c2_lower) || c2_lower.contains(&c1_lower) {
            return 0.8;
        }

        // Check for common substrings
        let c1_words: HashSet<&str> = c1_lower.split_whitespace().collect();
        let c2_words: HashSet<&str> = c2_lower.split_whitespace().collect();

        let intersection = c1_words.intersection(&c2_words).count();
        let union = c1_words.union(&c2_words).count();

        if union > 0 {
            intersection as f64 / union as f64
        } else {
            0.0
        }
    }

    /// Analyze goal connectivity using knowledge graph
    async fn analyze_goal_connectivity(
        &self,
        goal: &Goal,
        concepts: &[String],
    ) -> GoalAlignmentResult<ConnectivityResult> {
        if concepts.is_empty() {
            return Ok(ConnectivityResult {
                all_connected: true,
                paths: Vec::new(),
                disconnected: Vec::new(),
                strength_score: 1.0,
                suggested_connections: Vec::new(),
            });
        }

        // TODO: Re-enable connectivity analysis when is_all_terms_connected_by_path is available
        // For now, assume all concepts are connected
        let all_connected = true;

        // For now, create a simplified connectivity result
        let connectivity_result = ConnectivityResult {
            all_connected,
            paths: if all_connected {
                vec![concepts.to_vec()]
            } else {
                Vec::new()
            },
            disconnected: if all_connected {
                Vec::new()
            } else {
                concepts.to_vec()
            },
            strength_score: if all_connected { 1.0 } else { 0.5 },
            suggested_connections: Vec::new(),
        };

        Ok(connectivity_result)
    }

    /// Calculate alignment score between two goals
    async fn calculate_goal_alignment_score(
        &self,
        goal1: &Goal,
        goal2: &Goal,
    ) -> GoalAlignmentResult<f64> {
        // Calculate concept overlap
        let concepts1: HashSet<String> = goal1.knowledge_context.concepts.iter().cloned().collect();
        let concepts2: HashSet<String> = goal2.knowledge_context.concepts.iter().cloned().collect();

        let intersection = concepts1.intersection(&concepts2).count();
        let union = concepts1.union(&concepts2).count();

        let concept_similarity = if union > 0 {
            intersection as f64 / union as f64
        } else {
            0.0
        };

        // Calculate domain overlap
        let domains1: HashSet<String> = goal1.knowledge_context.domains.iter().cloned().collect();
        let domains2: HashSet<String> = goal2.knowledge_context.domains.iter().cloned().collect();

        let domain_intersection = domains1.intersection(&domains2).count();
        let domain_union = domains1.union(&domains2).count();

        let domain_similarity = if domain_union > 0 {
            domain_intersection as f64 / domain_union as f64
        } else {
            0.0
        };

        // Calculate role overlap
        let roles1: HashSet<String> = goal1.assigned_roles.iter().cloned().collect();
        let roles2: HashSet<String> = goal2.assigned_roles.iter().cloned().collect();

        let role_intersection = roles1.intersection(&roles2).count();
        let role_union = roles1.union(&roles2).count();

        let role_similarity = if role_union > 0 {
            role_intersection as f64 / role_union as f64
        } else {
            0.0
        };

        // Weighted combination
        let alignment_score =
            concept_similarity * 0.4 + domain_similarity * 0.4 + role_similarity * 0.2;

        Ok(alignment_score)
    }

    /// Detect conflicts between two goals
    async fn detect_goal_conflict(
        &self,
        goal1: &Goal,
        goal2: &Goal,
    ) -> GoalAlignmentResult<Option<GoalConflict>> {
        // Check for resource conflicts
        if let Some(conflict) = self.check_resource_conflict(goal1, goal2).await? {
            return Ok(Some(conflict));
        }

        // Check for temporal conflicts
        if let Some(conflict) = self.check_temporal_conflict(goal1, goal2).await? {
            return Ok(Some(conflict));
        }

        // Check for semantic conflicts
        if let Some(conflict) = self.check_semantic_conflict(goal1, goal2).await? {
            return Ok(Some(conflict));
        }

        Ok(None)
    }

    /// Check for resource conflicts between goals
    async fn check_resource_conflict(
        &self,
        goal1: &Goal,
        goal2: &Goal,
    ) -> GoalAlignmentResult<Option<GoalConflict>> {
        // Check if goals have overlapping assigned agents
        let agents1: HashSet<_> = goal1.assigned_agents.iter().collect();
        let agents2: HashSet<_> = goal2.assigned_agents.iter().collect();

        let overlapping_agents = agents1.intersection(&agents2).count();

        if overlapping_agents > 0 {
            let severity = overlapping_agents as f64 / agents1.len().max(agents2.len()) as f64;

            return Ok(Some(GoalConflict {
                goal1: goal1.goal_id.clone(),
                goal2: goal2.goal_id.clone(),
                conflict_type: ConflictType::Resource,
                severity,
                description: format!(
                    "Goals share {} agents, which may cause resource contention",
                    overlapping_agents
                ),
                suggested_resolutions: vec![
                    "Prioritize one goal over the other".to_string(),
                    "Assign different agents to each goal".to_string(),
                    "Schedule goals sequentially".to_string(),
                ],
            }));
        }

        Ok(None)
    }

    /// Check for temporal conflicts between goals
    async fn check_temporal_conflict(
        &self,
        goal1: &Goal,
        goal2: &Goal,
    ) -> GoalAlignmentResult<Option<GoalConflict>> {
        // Simple temporal conflict detection based on priority and status
        if goal1.priority == goal2.priority
            && goal1.status == crate::GoalStatus::Active
            && goal2.status == crate::GoalStatus::Active
        {
            return Ok(Some(GoalConflict {
                goal1: goal1.goal_id.clone(),
                goal2: goal2.goal_id.clone(),
                conflict_type: ConflictType::Priority,
                severity: 0.5,
                description: "Goals have same priority and are both active".to_string(),
                suggested_resolutions: vec![
                    "Adjust goal priorities".to_string(),
                    "Sequence goal execution".to_string(),
                ],
            }));
        }

        Ok(None)
    }

    /// Check for semantic conflicts between goals
    async fn check_semantic_conflict(
        &self,
        goal1: &Goal,
        goal2: &Goal,
    ) -> GoalAlignmentResult<Option<GoalConflict>> {
        // Check for contradictory concepts or objectives
        let alignment_score = self.calculate_goal_alignment_score(goal1, goal2).await?;

        if alignment_score < self.similarity_thresholds.conflict_threshold {
            return Ok(Some(GoalConflict {
                goal1: goal1.goal_id.clone(),
                goal2: goal2.goal_id.clone(),
                conflict_type: ConflictType::Semantic,
                severity: 1.0 - alignment_score,
                description: "Goals have low semantic alignment, indicating potential conflict"
                    .to_string(),
                suggested_resolutions: vec![
                    "Review goal descriptions for contradictions".to_string(),
                    "Clarify goal scope and boundaries".to_string(),
                    "Consider merging or restructuring goals".to_string(),
                ],
            }));
        }

        Ok(None)
    }

    /// Check goal connectivity issues
    async fn check_goal_connectivity(
        &self,
        goal: &Goal,
    ) -> GoalAlignmentResult<Option<ConnectivityIssue>> {
        let concepts = self.extract_goal_concepts(goal).await?;
        let connectivity = self.analyze_goal_connectivity(goal, &concepts).await?;

        if !connectivity.all_connected {
            return Ok(Some(ConnectivityIssue {
                goal_id: goal.goal_id.clone(),
                issue_type: ConnectivityIssueType::DisconnectedConcepts,
                description: format!(
                    "Goal has {} disconnected concepts: {}",
                    connectivity.disconnected.len(),
                    connectivity.disconnected.join(", ")
                ),
                suggested_fixes: vec![
                    "Add bridging concepts to connect disconnected elements".to_string(),
                    "Refine goal description to improve concept connectivity".to_string(),
                    "Split goal into smaller, more focused sub-goals".to_string(),
                ],
            }));
        }

        Ok(None)
    }

    /// Calculate overall alignment score
    fn calculate_overall_alignment_score(
        &self,
        goal_analyses: &HashMap<GoalId, GoalAnalysisResult>,
    ) -> f64 {
        if goal_analyses.is_empty() {
            return 1.0;
        }

        let total_score: f64 = goal_analyses
            .values()
            .map(|analysis| {
                let avg_alignment: f64 = if analysis.alignment_scores.is_empty() {
                    1.0
                } else {
                    analysis.alignment_scores.values().sum::<f64>()
                        / analysis.alignment_scores.len() as f64
                };

                let connectivity_score = analysis.connectivity.strength_score;

                (avg_alignment + connectivity_score) / 2.0
            })
            .sum();

        total_score / goal_analyses.len() as f64
    }

    /// Generate alignment recommendations
    async fn generate_alignment_recommendations(
        &self,
        goals: &[Goal],
        conflicts: &[GoalConflict],
        connectivity_issues: &[ConnectivityIssue],
    ) -> GoalAlignmentResult<Vec<AlignmentRecommendation>> {
        let mut recommendations = Vec::new();

        // Generate recommendations based on conflicts
        for conflict in conflicts {
            match conflict.conflict_type {
                ConflictType::Resource => {
                    recommendations.push(AlignmentRecommendation {
                        recommendation_type: RecommendationType::AdjustPriorities,
                        target_goals: vec![conflict.goal1.clone(), conflict.goal2.clone()],
                        description: "Adjust goal priorities to resolve resource conflicts"
                            .to_string(),
                        expected_impact: conflict.severity,
                        priority: (conflict.severity * 10.0) as u32,
                    });
                }
                ConflictType::Semantic => {
                    recommendations.push(AlignmentRecommendation {
                        recommendation_type: RecommendationType::ModifyDescription,
                        target_goals: vec![conflict.goal1.clone(), conflict.goal2.clone()],
                        description: "Clarify goal descriptions to resolve semantic conflicts"
                            .to_string(),
                        expected_impact: conflict.severity,
                        priority: (conflict.severity * 8.0) as u32,
                    });
                }
                _ => {}
            }
        }

        // Generate recommendations based on connectivity issues
        for issue in connectivity_issues {
            match issue.issue_type {
                ConnectivityIssueType::DisconnectedConcepts => {
                    recommendations.push(AlignmentRecommendation {
                        recommendation_type: RecommendationType::ModifyDescription,
                        target_goals: vec![issue.goal_id.clone()],
                        description: "Improve concept connectivity in goal description".to_string(),
                        expected_impact: 0.7,
                        priority: 5,
                    });
                }
                _ => {}
            }
        }

        // Sort recommendations by priority
        recommendations.sort_by(|a, b| b.priority.cmp(&a.priority));

        Ok(recommendations)
    }

    /// Clear expired cache entries
    pub async fn cleanup_cache(&self) {
        let mut cache = self.analysis_cache.write().await;
        let now = chrono::Utc::now();
        cache.retain(|_, result| result.expires_at > now);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Goal, GoalLevel};

    #[tokio::test]
    async fn test_knowledge_graph_analyzer_creation() {
        let role_graph = Arc::new(RoleGraph::new());
        let automata_config = AutomataConfig::default();
        let similarity_thresholds = SimilarityThresholds::default();

        let analyzer =
            KnowledgeGraphGoalAnalyzer::new(role_graph, automata_config, similarity_thresholds);

        assert_eq!(analyzer.similarity_thresholds.concept_similarity, 0.8);
    }

    #[tokio::test]
    async fn test_concept_similarity() {
        let role_graph = Arc::new(RoleGraph::new());
        let analyzer = KnowledgeGraphGoalAnalyzer::new(
            role_graph,
            AutomataConfig::default(),
            SimilarityThresholds::default(),
        );

        // Test exact match
        assert_eq!(
            analyzer.calculate_concept_similarity("planning", "planning"),
            1.0
        );

        // Test partial match
        assert!(analyzer.calculate_concept_similarity("planning", "plan") > 0.0);

        // Test no match
        assert_eq!(
            analyzer.calculate_concept_similarity("planning", "execution"),
            0.0
        );
    }

    #[tokio::test]
    async fn test_goal_alignment_score() {
        let role_graph = Arc::new(RoleGraph::new());
        let analyzer = KnowledgeGraphGoalAnalyzer::new(
            role_graph,
            AutomataConfig::default(),
            SimilarityThresholds::default(),
        );

        let mut goal1 = Goal::new(
            "goal1".to_string(),
            GoalLevel::Local,
            "Planning task".to_string(),
            1,
        );
        goal1.knowledge_context.concepts = vec!["planning".to_string(), "task".to_string()];
        goal1.knowledge_context.domains = vec!["project_management".to_string()];

        let mut goal2 = Goal::new(
            "goal2".to_string(),
            GoalLevel::Local,
            "Execution task".to_string(),
            1,
        );
        goal2.knowledge_context.concepts = vec!["execution".to_string(), "task".to_string()];
        goal2.knowledge_context.domains = vec!["project_management".to_string()];

        let alignment_score = analyzer
            .calculate_goal_alignment_score(&goal1, &goal2)
            .await
            .unwrap();

        // Should have some alignment due to shared "task" concept and domain
        assert!(alignment_score > 0.0);
        assert!(alignment_score < 1.0);
    }
}
