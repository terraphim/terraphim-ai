//! Goal alignment engine and management
//!
//! Provides the core goal alignment functionality that coordinates goal hierarchy
//! validation, conflict resolution, and alignment optimization.

use std::collections::HashMap;
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;

use terraphim_agent_registry::AgentRegistry;
use terraphim_rolegraph::RoleGraph;

use crate::{
    AlignmentRecommendation, AnalysisType, Goal, GoalAlignmentAnalysis,
    GoalAlignmentAnalysisResult, GoalAlignmentResult, GoalHierarchy, GoalId, GoalLevel,
    KnowledgeGraphGoalAnalyzer,
};

/// Goal alignment engine that manages the complete goal alignment process
pub struct KnowledgeGraphGoalAligner {
    /// Goal hierarchy storage
    goal_hierarchy: Arc<RwLock<GoalHierarchy>>,
    /// Knowledge graph analyzer
    kg_analyzer: Arc<KnowledgeGraphGoalAnalyzer>,
    /// Agent registry for agent-goal assignments
    #[allow(dead_code)]
    agent_registry: Arc<dyn AgentRegistry>,
    /// Role graph for role-based operations
    #[allow(dead_code)]
    role_graph: Arc<RoleGraph>,
    /// Alignment configuration
    config: AlignmentConfig,
    /// Alignment statistics
    statistics: Arc<RwLock<AlignmentStatistics>>,
}

/// Configuration for goal alignment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlignmentConfig {
    /// Enable automatic conflict resolution
    pub auto_resolve_conflicts: bool,
    /// Maximum alignment iterations
    pub max_alignment_iterations: u32,
    /// Alignment convergence threshold
    pub convergence_threshold: f64,
    /// Enable real-time alignment updates
    pub real_time_updates: bool,
    /// Alignment cache TTL in seconds
    pub cache_ttl_secs: u64,
    /// Enable performance monitoring
    pub enable_monitoring: bool,
}

impl Default for AlignmentConfig {
    fn default() -> Self {
        Self {
            auto_resolve_conflicts: false, // Manual resolution by default
            max_alignment_iterations: 10,
            convergence_threshold: 0.95,
            real_time_updates: true,
            cache_ttl_secs: 1800, // 30 minutes
            enable_monitoring: true,
        }
    }
}

/// Alignment statistics and monitoring
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlignmentStatistics {
    /// Total number of goals managed
    pub total_goals: usize,
    /// Goals by level
    pub goals_by_level: HashMap<String, usize>,
    /// Goals by status
    pub goals_by_status: HashMap<String, usize>,
    /// Total alignment analyses performed
    pub total_analyses: u64,
    /// Average alignment score
    pub average_alignment_score: f64,
    /// Total conflicts detected
    pub total_conflicts_detected: u64,
    /// Total conflicts resolved
    pub total_conflicts_resolved: u64,
    /// Average analysis time
    pub average_analysis_time_ms: f64,
    /// Last alignment update
    pub last_alignment_update: chrono::DateTime<chrono::Utc>,
}

impl Default for AlignmentStatistics {
    fn default() -> Self {
        Self {
            total_goals: 0,
            goals_by_level: HashMap::new(),
            goals_by_status: HashMap::new(),
            total_analyses: 0,
            average_alignment_score: 0.0,
            total_conflicts_detected: 0,
            total_conflicts_resolved: 0,
            average_analysis_time_ms: 0.0,
            last_alignment_update: chrono::Utc::now(),
        }
    }
}

/// Goal alignment request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GoalAlignmentRequest {
    /// Goals to align (empty means all goals)
    pub goal_ids: Vec<GoalId>,
    /// Type of alignment to perform
    pub alignment_type: AlignmentType,
    /// Force re-analysis even if cached
    pub force_reanalysis: bool,
    /// Additional context
    pub context: HashMap<String, serde_json::Value>,
}

/// Types of goal alignment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AlignmentType {
    /// Validate goal hierarchy consistency
    HierarchyValidation,
    /// Detect and report conflicts
    ConflictDetection,
    /// Full alignment with optimization
    FullAlignment,
    /// Incremental alignment update
    IncrementalUpdate,
}

/// Goal alignment response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GoalAlignmentResponse {
    /// Alignment analysis results
    pub analysis_result: GoalAlignmentAnalysisResult,
    /// Alignment actions taken
    pub actions_taken: Vec<AlignmentAction>,
    /// Updated goals
    pub updated_goals: Vec<Goal>,
    /// Alignment summary
    pub summary: AlignmentSummary,
}

/// Actions taken during alignment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlignmentAction {
    /// Action type
    pub action_type: AlignmentActionType,
    /// Target goals
    pub target_goals: Vec<GoalId>,
    /// Action description
    pub description: String,
    /// Action result
    pub result: ActionResult,
}

/// Types of alignment actions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AlignmentActionType {
    /// Goal priority adjustment
    PriorityAdjustment,
    /// Goal constraint modification
    ConstraintModification,
    /// Goal dependency addition
    DependencyAddition,
    /// Goal hierarchy restructuring
    HierarchyRestructuring,
    /// Goal merging
    GoalMerging,
    /// Goal splitting
    GoalSplitting,
    /// Agent reassignment
    AgentReassignment,
}

/// Result of alignment action
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ActionResult {
    /// Action completed successfully
    Success,
    /// Action failed with error
    Failed(String),
    /// Action skipped (not applicable)
    Skipped(String),
    /// Action requires manual intervention
    RequiresManualIntervention(String),
}

/// Alignment summary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlignmentSummary {
    /// Overall alignment score before
    pub alignment_score_before: f64,
    /// Overall alignment score after
    pub alignment_score_after: f64,
    /// Number of conflicts detected
    pub conflicts_detected: usize,
    /// Number of conflicts resolved
    pub conflicts_resolved: usize,
    /// Number of goals updated
    pub goals_updated: usize,
    /// Alignment improvement
    pub improvement: f64,
    /// Recommendations not implemented
    pub pending_recommendations: Vec<AlignmentRecommendation>,
}

impl KnowledgeGraphGoalAligner {
    /// Create new goal aligner
    pub fn new(
        kg_analyzer: Arc<KnowledgeGraphGoalAnalyzer>,
        agent_registry: Arc<dyn AgentRegistry>,
        role_graph: Arc<RoleGraph>,
        config: AlignmentConfig,
    ) -> Self {
        Self {
            goal_hierarchy: Arc::new(RwLock::new(GoalHierarchy::new())),
            kg_analyzer,
            agent_registry,
            role_graph,
            config,
            statistics: Arc::new(RwLock::new(AlignmentStatistics::default())),
        }
    }

    /// Add a goal to the alignment system
    pub async fn add_goal(&self, goal: Goal) -> GoalAlignmentResult<()> {
        let mut hierarchy = self.goal_hierarchy.write().await;
        hierarchy.add_goal(goal)?;

        // Update statistics
        self.update_statistics().await?;

        // Trigger real-time alignment if enabled
        if self.config.real_time_updates {
            self.trigger_incremental_alignment().await?;
        }

        Ok(())
    }

    /// Remove a goal from the alignment system
    pub async fn remove_goal(&self, goal_id: &GoalId) -> GoalAlignmentResult<()> {
        let mut hierarchy = self.goal_hierarchy.write().await;
        hierarchy.remove_goal(goal_id)?;

        // Update statistics
        self.update_statistics().await?;

        Ok(())
    }

    /// Update an existing goal
    pub async fn update_goal(&self, goal: Goal) -> GoalAlignmentResult<()> {
        let mut hierarchy = self.goal_hierarchy.write().await;

        // Remove old version and add new version
        hierarchy.remove_goal(&goal.goal_id)?;
        hierarchy.add_goal(goal)?;

        // Update statistics
        drop(hierarchy);
        self.update_statistics().await?;

        // Trigger real-time alignment if enabled
        if self.config.real_time_updates {
            self.trigger_incremental_alignment().await?;
        }

        Ok(())
    }

    /// Get a goal by ID
    pub async fn get_goal(&self, goal_id: &GoalId) -> GoalAlignmentResult<Option<Goal>> {
        let hierarchy = self.goal_hierarchy.read().await;
        Ok(hierarchy.goals.get(goal_id).cloned())
    }

    /// List all goals
    pub async fn list_goals(&self) -> GoalAlignmentResult<Vec<Goal>> {
        let hierarchy = self.goal_hierarchy.read().await;
        Ok(hierarchy.goals.values().cloned().collect())
    }

    /// List goals by level
    pub async fn list_goals_by_level(&self, level: &GoalLevel) -> GoalAlignmentResult<Vec<Goal>> {
        let hierarchy = self.goal_hierarchy.read().await;
        Ok(hierarchy
            .get_goals_by_level(level)
            .into_iter()
            .cloned()
            .collect())
    }

    /// Perform goal alignment
    pub async fn align_goals(
        &self,
        request: GoalAlignmentRequest,
    ) -> GoalAlignmentResult<GoalAlignmentResponse> {
        let start_time = std::time::Instant::now();

        // Get goals to analyze
        let goals = if request.goal_ids.is_empty() {
            self.list_goals().await?
        } else {
            let mut selected_goals = Vec::new();
            for goal_id in &request.goal_ids {
                if let Some(goal) = self.get_goal(goal_id).await? {
                    selected_goals.push(goal);
                }
            }
            selected_goals
        };

        // Perform knowledge graph analysis
        let analysis_type = match request.alignment_type {
            AlignmentType::HierarchyValidation => AnalysisType::HierarchyConsistency,
            AlignmentType::ConflictDetection => AnalysisType::ConflictDetection,
            AlignmentType::FullAlignment => AnalysisType::Comprehensive,
            AlignmentType::IncrementalUpdate => AnalysisType::ConnectivityValidation,
        };

        let analysis = GoalAlignmentAnalysis {
            goals: goals.clone(),
            analysis_type,
            context: request.context,
        };

        let analysis_result = self.kg_analyzer.analyze_goal_alignment(analysis).await?;
        let alignment_score_before = analysis_result.overall_alignment_score;

        // Execute alignment actions based on recommendations
        let mut actions_taken = Vec::new();
        let mut updated_goals = Vec::new();
        let mut conflicts_resolved = 0;

        if self.config.auto_resolve_conflicts
            || matches!(request.alignment_type, AlignmentType::FullAlignment)
        {
            for recommendation in &analysis_result.recommendations {
                let action = self
                    .execute_alignment_recommendation(recommendation)
                    .await?;

                if matches!(action.result, ActionResult::Success) {
                    conflicts_resolved += 1;

                    // Collect updated goals
                    for goal_id in &action.target_goals {
                        if let Some(updated_goal) = self.get_goal(goal_id).await? {
                            updated_goals.push(updated_goal);
                        }
                    }
                }

                actions_taken.push(action);
            }
        }

        // Calculate final alignment score
        let final_analysis = if !actions_taken.is_empty() {
            let updated_goal_list = self.list_goals().await?;
            let final_analysis_request = GoalAlignmentAnalysis {
                goals: updated_goal_list,
                analysis_type: AnalysisType::Comprehensive,
                context: HashMap::new(),
            };
            self.kg_analyzer
                .analyze_goal_alignment(final_analysis_request)
                .await?
        } else {
            analysis_result.clone()
        };

        let alignment_score_after = final_analysis.overall_alignment_score;

        // Create summary
        let summary = AlignmentSummary {
            alignment_score_before,
            alignment_score_after,
            conflicts_detected: analysis_result.conflicts.len(),
            conflicts_resolved,
            goals_updated: updated_goals.len(),
            improvement: alignment_score_after - alignment_score_before,
            pending_recommendations: analysis_result
                .recommendations
                .into_iter()
                .filter(|rec| {
                    !actions_taken.iter().any(|action| {
                        action.target_goals == rec.target_goals
                            && matches!(action.result, ActionResult::Success)
                    })
                })
                .collect(),
        };

        // Update statistics
        {
            let mut stats = self.statistics.write().await;
            stats.total_analyses += 1;
            stats.total_conflicts_detected += summary.conflicts_detected as u64;
            stats.total_conflicts_resolved += conflicts_resolved as u64;

            let analysis_time_ms = start_time.elapsed().as_millis() as f64;
            if stats.total_analyses == 1 {
                stats.average_analysis_time_ms = analysis_time_ms;
            } else {
                let total_time = stats.average_analysis_time_ms * (stats.total_analyses - 1) as f64;
                stats.average_analysis_time_ms =
                    (total_time + analysis_time_ms) / stats.total_analyses as f64;
            }

            stats.average_alignment_score = alignment_score_after;
            stats.last_alignment_update = chrono::Utc::now();
        }

        Ok(GoalAlignmentResponse {
            analysis_result: final_analysis,
            actions_taken,
            updated_goals,
            summary,
        })
    }

    /// Execute an alignment recommendation
    async fn execute_alignment_recommendation(
        &self,
        recommendation: &AlignmentRecommendation,
    ) -> GoalAlignmentResult<AlignmentAction> {
        let action_type = match recommendation.recommendation_type {
            crate::RecommendationType::AdjustPriorities => AlignmentActionType::PriorityAdjustment,
            crate::RecommendationType::AddConstraints => {
                AlignmentActionType::ConstraintModification
            }
            crate::RecommendationType::AddDependencies => AlignmentActionType::DependencyAddition,
            crate::RecommendationType::RestructureHierarchy => {
                AlignmentActionType::HierarchyRestructuring
            }
            crate::RecommendationType::MergeGoals => AlignmentActionType::GoalMerging,
            crate::RecommendationType::SplitGoals => AlignmentActionType::GoalSplitting,
            crate::RecommendationType::ModifyDescription => {
                // For now, skip description modifications as they require manual intervention
                return Ok(AlignmentAction {
                    action_type: AlignmentActionType::PriorityAdjustment,
                    target_goals: recommendation.target_goals.clone(),
                    description: recommendation.description.clone(),
                    result: ActionResult::RequiresManualIntervention(
                        "Description modifications require manual review".to_string(),
                    ),
                });
            }
        };

        let result = match action_type {
            AlignmentActionType::PriorityAdjustment => {
                self.execute_priority_adjustment(&recommendation.target_goals)
                    .await
            }
            AlignmentActionType::DependencyAddition => {
                self.execute_dependency_addition(&recommendation.target_goals)
                    .await
            }
            _ => {
                // Other action types require more complex implementation
                ActionResult::RequiresManualIntervention(format!(
                    "Action type {:?} not yet implemented",
                    action_type
                ))
            }
        };

        Ok(AlignmentAction {
            action_type,
            target_goals: recommendation.target_goals.clone(),
            description: recommendation.description.clone(),
            result,
        })
    }

    /// Execute priority adjustment
    async fn execute_priority_adjustment(&self, goal_ids: &[GoalId]) -> ActionResult {
        if goal_ids.len() < 2 {
            return ActionResult::Skipped(
                "Need at least 2 goals for priority adjustment".to_string(),
            );
        }

        let mut hierarchy = self.goal_hierarchy.write().await;

        // Simple priority adjustment: increment priority of first goal
        if let Some(goal) = hierarchy.goals.get_mut(&goal_ids[0]) {
            goal.priority += 1;
            goal.metadata.updated_at = chrono::Utc::now();
            goal.metadata.version += 1;
            ActionResult::Success
        } else {
            ActionResult::Failed("Goal not found".to_string())
        }
    }

    /// Execute dependency addition
    async fn execute_dependency_addition(&self, goal_ids: &[GoalId]) -> ActionResult {
        if goal_ids.len() < 2 {
            return ActionResult::Skipped(
                "Need at least 2 goals for dependency addition".to_string(),
            );
        }

        let mut hierarchy = self.goal_hierarchy.write().await;

        // Add dependency from second goal to first goal
        if let Some(goal) = hierarchy.goals.get_mut(&goal_ids[1]) {
            if let Err(e) = goal.add_dependency(goal_ids[0].clone()) {
                ActionResult::Failed(e.to_string())
            } else {
                ActionResult::Success
            }
        } else {
            ActionResult::Failed("Goal not found".to_string())
        }
    }

    /// Trigger incremental alignment update
    async fn trigger_incremental_alignment(&self) -> GoalAlignmentResult<()> {
        let request = GoalAlignmentRequest {
            goal_ids: Vec::new(), // All goals
            alignment_type: AlignmentType::IncrementalUpdate,
            force_reanalysis: false,
            context: HashMap::new(),
        };

        let _response = self.align_goals(request).await?;
        Ok(())
    }

    /// Update alignment statistics
    async fn update_statistics(&self) -> GoalAlignmentResult<()> {
        let hierarchy = self.goal_hierarchy.read().await;
        let mut stats = self.statistics.write().await;

        stats.total_goals = hierarchy.goals.len();

        // Count goals by level
        stats.goals_by_level.clear();
        for goal in hierarchy.goals.values() {
            let level_key = format!("{:?}", goal.level);
            *stats.goals_by_level.entry(level_key).or_insert(0) += 1;
        }

        // Count goals by status
        stats.goals_by_status.clear();
        for goal in hierarchy.goals.values() {
            let status_key = format!("{:?}", goal.status);
            *stats.goals_by_status.entry(status_key).or_insert(0) += 1;
        }

        Ok(())
    }

    /// Get alignment statistics
    pub async fn get_statistics(&self) -> AlignmentStatistics {
        self.statistics.read().await.clone()
    }

    /// Validate goal hierarchy consistency
    pub async fn validate_hierarchy(&self) -> GoalAlignmentResult<Vec<String>> {
        let hierarchy = self.goal_hierarchy.read().await;
        let mut issues = Vec::new();

        // Check for dependency cycles
        if let Some(cycle) = hierarchy.has_dependency_cycle() {
            issues.push(format!("Dependency cycle detected: {}", cycle.join(" -> ")));
        }

        // Check hierarchy level consistency
        for (parent_id, children) in &hierarchy.parent_child {
            if let Some(parent_goal) = hierarchy.goals.get(parent_id) {
                for child_id in children {
                    if let Some(child_goal) = hierarchy.goals.get(child_id) {
                        if !parent_goal.level.can_contain(&child_goal.level) {
                            issues.push(format!(
                                "Invalid hierarchy: {:?} goal '{}' cannot contain {:?} goal '{}'",
                                parent_goal.level, parent_id, child_goal.level, child_id
                            ));
                        }
                    }
                }
            }
        }

        Ok(issues)
    }

    /// Get goals that can be started
    pub async fn get_startable_goals(&self) -> GoalAlignmentResult<Vec<Goal>> {
        let hierarchy = self.goal_hierarchy.read().await;
        Ok(hierarchy
            .get_startable_goals()
            .into_iter()
            .cloned()
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{AutomataConfig, Goal, GoalLevel, SimilarityThresholds};
    use async_trait::async_trait;
    use std::sync::Arc;
    use terraphim_agent_registry::{
        AgentMetadata, AgentPid, AgentRegistry, RegistryResult, SupervisorId,
    };
    use terraphim_types::{RoleName, Thesaurus};

    // Mock agent registry for testing
    struct MockAgentRegistry;

    #[async_trait]
    impl AgentRegistry for MockAgentRegistry {
        async fn register_agent(&self, _metadata: AgentMetadata) -> RegistryResult<()> {
            Ok(())
        }

        async fn unregister_agent(&self, _agent_id: &AgentPid) -> RegistryResult<()> {
            Ok(())
        }

        async fn update_agent(&self, _metadata: AgentMetadata) -> RegistryResult<()> {
            Ok(())
        }

        async fn get_agent(&self, _agent_id: &AgentPid) -> RegistryResult<Option<AgentMetadata>> {
            Ok(None)
        }

        async fn list_agents(&self) -> RegistryResult<Vec<AgentMetadata>> {
            Ok(Vec::new())
        }

        async fn discover_agents(
            &self,
            _query: terraphim_agent_registry::AgentDiscoveryQuery,
        ) -> RegistryResult<terraphim_agent_registry::AgentDiscoveryResult> {
            Ok(terraphim_agent_registry::AgentDiscoveryResult {
                matches: Vec::new(),
                query_analysis: terraphim_agent_registry::QueryAnalysis {
                    extracted_concepts: Vec::new(),
                    identified_domains: Vec::new(),
                    suggested_roles: Vec::new(),
                    connectivity_analysis: terraphim_agent_registry::ConnectivityResult {
                        all_connected: true,
                        paths: Vec::new(),
                        disconnected: Vec::new(),
                        strength_score: 1.0,
                    },
                },
                suggestions: Vec::new(),
            })
        }

        async fn find_agents_by_role(&self, _role_id: &str) -> RegistryResult<Vec<AgentMetadata>> {
            Ok(Vec::new())
        }

        async fn find_agents_by_capability(
            &self,
            _capability_id: &str,
        ) -> RegistryResult<Vec<AgentMetadata>> {
            Ok(Vec::new())
        }

        async fn find_agents_by_supervisor(
            &self,
            _supervisor_id: &SupervisorId,
        ) -> RegistryResult<Vec<AgentMetadata>> {
            Ok(Vec::new())
        }

        async fn get_statistics(
            &self,
        ) -> RegistryResult<terraphim_agent_registry::RegistryStatistics> {
            Ok(terraphim_agent_registry::RegistryStatistics {
                total_agents: 0,
                agents_by_status: HashMap::new(),
                agents_by_role: HashMap::new(),
                total_discovery_queries: 0,
                avg_discovery_time_ms: 0.0,
                discovery_cache_hit_rate: 0.0,
                uptime_secs: 0,
                last_updated: chrono::Utc::now(),
            })
        }
    }

    #[tokio::test]
    async fn test_goal_aligner_creation() {
        let role_name = RoleName::new("test_role");
        let thesaurus = Thesaurus::new("test_thesaurus".to_string());
        let role_graph = Arc::new(RoleGraph::new(role_name, thesaurus).await.unwrap());
        let kg_analyzer = Arc::new(KnowledgeGraphGoalAnalyzer::new(
            role_graph.clone(),
            AutomataConfig::default(),
            SimilarityThresholds::default(),
        ));
        let agent_registry = Arc::new(MockAgentRegistry);
        let config = AlignmentConfig::default();

        let aligner =
            KnowledgeGraphGoalAligner::new(kg_analyzer, agent_registry, role_graph, config);

        let stats = aligner.get_statistics().await;
        assert_eq!(stats.total_goals, 0);
    }

    #[tokio::test]
    async fn test_goal_management() {
        let role_name = RoleName::new("test_role");
        let thesaurus = Thesaurus::new("test_thesaurus".to_string());
        let role_graph = Arc::new(RoleGraph::new(role_name, thesaurus).await.unwrap());
        let kg_analyzer = Arc::new(KnowledgeGraphGoalAnalyzer::new(
            role_graph.clone(),
            AutomataConfig::default(),
            SimilarityThresholds::default(),
        ));
        let agent_registry = Arc::new(MockAgentRegistry);
        let config = AlignmentConfig {
            real_time_updates: false, // Disable for testing
            ..AlignmentConfig::default()
        };

        let aligner =
            KnowledgeGraphGoalAligner::new(kg_analyzer, agent_registry, role_graph, config);

        // Add a goal
        let goal = Goal::new(
            "test_goal".to_string(),
            GoalLevel::Local,
            "Test goal for alignment".to_string(),
            1,
        );

        aligner.add_goal(goal.clone()).await.unwrap();

        // Retrieve the goal
        let retrieved = aligner.get_goal(&"test_goal".to_string()).await.unwrap();
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().goal_id, "test_goal");

        // List goals
        let goals = aligner.list_goals().await.unwrap();
        assert_eq!(goals.len(), 1);

        // Check statistics
        let stats = aligner.get_statistics().await;
        assert_eq!(stats.total_goals, 1);
    }

    #[tokio::test]
    async fn test_goal_alignment() {
        let role_name = RoleName::new("test_role");
        let thesaurus = Thesaurus::new("test_thesaurus".to_string());
        let role_graph = Arc::new(RoleGraph::new(role_name, thesaurus).await.unwrap());
        let kg_analyzer = Arc::new(KnowledgeGraphGoalAnalyzer::new(
            role_graph.clone(),
            AutomataConfig::default(),
            SimilarityThresholds::default(),
        ));
        let agent_registry = Arc::new(MockAgentRegistry);
        let config = AlignmentConfig::default();

        let aligner =
            KnowledgeGraphGoalAligner::new(kg_analyzer, agent_registry, role_graph, config);

        // Add test goals
        let goal1 = Goal::new(
            "goal1".to_string(),
            GoalLevel::Global,
            "Global strategic goal".to_string(),
            1,
        );

        let goal2 = Goal::new(
            "goal2".to_string(),
            GoalLevel::Local,
            "Local tactical goal".to_string(),
            2,
        );

        aligner.add_goal(goal1).await.unwrap();
        aligner.add_goal(goal2).await.unwrap();

        // Perform alignment
        let request = GoalAlignmentRequest {
            goal_ids: Vec::new(), // All goals
            alignment_type: AlignmentType::FullAlignment,
            force_reanalysis: true,
            context: HashMap::new(),
        };

        let response = aligner.align_goals(request).await.unwrap();

        assert!(response.analysis_result.overall_alignment_score >= 0.0);
        assert!(response.analysis_result.overall_alignment_score <= 1.0);
        assert_eq!(response.summary.goals_updated, response.updated_goals.len());
    }
}
