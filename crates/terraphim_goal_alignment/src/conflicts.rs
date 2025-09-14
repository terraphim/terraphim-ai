//! Goal conflict detection and resolution
//!
//! Provides specialized conflict detection algorithms and resolution strategies
//! for different types of goal conflicts.

use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};

use crate::{
    ConflictType, ConstraintType, Goal, GoalAlignmentError, GoalAlignmentResult, GoalConflict,
    GoalConstraint, GoalId,
};

/// Conflict detection engine
pub struct ConflictDetector {
    /// Conflict detection strategies
    strategies: HashMap<ConflictType, Box<dyn ConflictDetectionStrategy>>,
    /// Conflict resolution strategies
    resolvers: HashMap<ConflictType, Box<dyn ConflictResolutionStrategy>>,
}

/// Trait for conflict detection strategies
pub trait ConflictDetectionStrategy: Send + Sync {
    /// Detect conflicts between two goals
    fn detect_conflict(
        &self,
        goal1: &Goal,
        goal2: &Goal,
    ) -> GoalAlignmentResult<Option<GoalConflict>>;

    /// Get strategy name
    fn name(&self) -> &str;
}

/// Trait for conflict resolution strategies
pub trait ConflictResolutionStrategy: Send + Sync {
    /// Resolve a conflict between goals
    fn resolve_conflict(
        &self,
        conflict: &GoalConflict,
        goals: &mut [Goal],
    ) -> GoalAlignmentResult<ConflictResolution>;

    /// Get strategy name
    fn name(&self) -> &str;
}

/// Result of conflict resolution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConflictResolution {
    /// Resolution type applied
    pub resolution_type: ResolutionType,
    /// Goals that were modified
    pub modified_goals: Vec<GoalId>,
    /// Description of the resolution
    pub description: String,
    /// Success of the resolution
    pub success: bool,
    /// Remaining conflicts after resolution
    pub remaining_conflicts: Vec<GoalConflict>,
}

/// Types of conflict resolution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ResolutionType {
    /// Priority adjustment
    PriorityAdjustment,
    /// Resource reallocation
    ResourceReallocation,
    /// Temporal scheduling
    TemporalScheduling,
    /// Goal modification
    GoalModification,
    /// Goal merging
    GoalMerging,
    /// Goal splitting
    GoalSplitting,
    /// Manual intervention required
    ManualIntervention,
}

/// Resource conflict detection strategy
pub struct ResourceConflictDetector;

impl ConflictDetectionStrategy for ResourceConflictDetector {
    fn detect_conflict(
        &self,
        goal1: &Goal,
        goal2: &Goal,
    ) -> GoalAlignmentResult<Option<GoalConflict>> {
        // Check for overlapping assigned agents
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

        // Check for resource constraint conflicts
        let resource_conflicts = self.check_resource_constraints(goal1, goal2)?;
        if !resource_conflicts.is_empty() {
            return Ok(Some(GoalConflict {
                goal1: goal1.goal_id.clone(),
                goal2: goal2.goal_id.clone(),
                conflict_type: ConflictType::Resource,
                severity: 0.7,
                description: format!(
                    "Resource constraint conflicts: {}",
                    resource_conflicts.join(", ")
                ),
                suggested_resolutions: vec![
                    "Adjust resource allocations".to_string(),
                    "Modify resource constraints".to_string(),
                    "Schedule resource usage".to_string(),
                ],
            }));
        }

        Ok(None)
    }

    fn name(&self) -> &str {
        "ResourceConflictDetector"
    }
}

impl ResourceConflictDetector {
    /// Check for resource constraint conflicts
    fn check_resource_constraints(
        &self,
        goal1: &Goal,
        goal2: &Goal,
    ) -> GoalAlignmentResult<Vec<String>> {
        let mut conflicts = Vec::new();

        // Get resource constraints from both goals
        let resource_constraints1: Vec<_> = goal1
            .constraints
            .iter()
            .filter(|c| matches!(c.constraint_type, ConstraintType::Resource))
            .collect();

        let resource_constraints2: Vec<_> = goal2
            .constraints
            .iter()
            .filter(|c| matches!(c.constraint_type, ConstraintType::Resource))
            .collect();

        // Check for conflicting resource requirements
        for constraint1 in &resource_constraints1 {
            for constraint2 in &resource_constraints2 {
                if let (Some(resource_type1), Some(resource_type2)) = (
                    constraint1.parameters.get("resource_type"),
                    constraint2.parameters.get("resource_type"),
                ) {
                    if resource_type1 == resource_type2 {
                        // Same resource type - check for conflicts
                        if let (Some(amount1), Some(amount2)) = (
                            constraint1
                                .parameters
                                .get("amount")
                                .and_then(|v| v.as_f64()),
                            constraint2
                                .parameters
                                .get("amount")
                                .and_then(|v| v.as_f64()),
                        ) {
                            if let Some(total_available) = constraint1
                                .parameters
                                .get("total_available")
                                .and_then(|v| v.as_f64())
                            {
                                if amount1 + amount2 > total_available {
                                    conflicts.push(format!(
                                        "Resource {} over-allocated: {} + {} > {}",
                                        resource_type1, amount1, amount2, total_available
                                    ));
                                }
                            }
                        }
                    }
                }
            }
        }

        Ok(conflicts)
    }
}

/// Temporal conflict detection strategy
pub struct TemporalConflictDetector;

impl ConflictDetectionStrategy for TemporalConflictDetector {
    fn detect_conflict(
        &self,
        goal1: &Goal,
        goal2: &Goal,
    ) -> GoalAlignmentResult<Option<GoalConflict>> {
        // Check for temporal constraint conflicts
        let temporal_constraints1: Vec<_> = goal1
            .constraints
            .iter()
            .filter(|c| matches!(c.constraint_type, ConstraintType::Temporal))
            .collect();

        let temporal_constraints2: Vec<_> = goal2
            .constraints
            .iter()
            .filter(|c| matches!(c.constraint_type, ConstraintType::Temporal))
            .collect();

        for constraint1 in &temporal_constraints1 {
            for constraint2 in &temporal_constraints2 {
                if let Some(conflict) = self.check_temporal_overlap(constraint1, constraint2)? {
                    return Ok(Some(GoalConflict {
                        goal1: goal1.goal_id.clone(),
                        goal2: goal2.goal_id.clone(),
                        conflict_type: ConflictType::Temporal,
                        severity: 0.8,
                        description: conflict,
                        suggested_resolutions: vec![
                            "Adjust goal deadlines".to_string(),
                            "Sequence goal execution".to_string(),
                            "Modify temporal constraints".to_string(),
                        ],
                    }));
                }
            }
        }

        // Check for priority-based temporal conflicts
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

    fn name(&self) -> &str {
        "TemporalConflictDetector"
    }
}

impl TemporalConflictDetector {
    /// Check for temporal overlap between constraints
    fn check_temporal_overlap(
        &self,
        constraint1: &GoalConstraint,
        constraint2: &GoalConstraint,
    ) -> GoalAlignmentResult<Option<String>> {
        // Simple temporal overlap detection
        // In practice, this would parse dates and check for actual overlaps

        if let (Some(deadline1), Some(deadline2)) = (
            constraint1
                .parameters
                .get("deadline")
                .and_then(|v| v.as_str()),
            constraint2
                .parameters
                .get("deadline")
                .and_then(|v| v.as_str()),
        ) {
            if deadline1 == deadline2 {
                return Ok(Some(format!("Goals have same deadline: {}", deadline1)));
            }
        }

        Ok(None)
    }
}

/// Semantic conflict detection strategy
pub struct SemanticConflictDetector {
    /// Similarity threshold for conflict detection
    similarity_threshold: f64,
}

impl SemanticConflictDetector {
    pub fn new(similarity_threshold: f64) -> Self {
        Self {
            similarity_threshold,
        }
    }

    /// Calculate semantic similarity between goals
    fn calculate_semantic_similarity(&self, goal1: &Goal, goal2: &Goal) -> f64 {
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

        // Weighted combination
        concept_similarity * 0.6 + domain_similarity * 0.4
    }
}

impl ConflictDetectionStrategy for SemanticConflictDetector {
    fn detect_conflict(
        &self,
        goal1: &Goal,
        goal2: &Goal,
    ) -> GoalAlignmentResult<Option<GoalConflict>> {
        let similarity = self.calculate_semantic_similarity(goal1, goal2);

        // Low similarity might indicate conflicting objectives
        if similarity < self.similarity_threshold {
            return Ok(Some(GoalConflict {
                goal1: goal1.goal_id.clone(),
                goal2: goal2.goal_id.clone(),
                conflict_type: ConflictType::Semantic,
                severity: 1.0 - similarity,
                description: format!(
                    "Goals have low semantic alignment ({:.2}), indicating potential conflict",
                    similarity
                ),
                suggested_resolutions: vec![
                    "Review goal descriptions for contradictions".to_string(),
                    "Clarify goal scope and boundaries".to_string(),
                    "Consider merging or restructuring goals".to_string(),
                ],
            }));
        }

        Ok(None)
    }

    fn name(&self) -> &str {
        "SemanticConflictDetector"
    }
}

/// Priority-based conflict resolution strategy
pub struct PriorityBasedResolver;

impl ConflictResolutionStrategy for PriorityBasedResolver {
    fn resolve_conflict(
        &self,
        conflict: &GoalConflict,
        goals: &mut [Goal],
    ) -> GoalAlignmentResult<ConflictResolution> {
        let mut modified_goals = Vec::new();

        // Find the conflicting goals
        let goal1_pos = goals.iter().position(|g| g.goal_id == conflict.goal1);
        let goal2_pos = goals.iter().position(|g| g.goal_id == conflict.goal2);

        match (goal1_pos, goal2_pos) {
            (Some(pos1), Some(pos2)) => {
                // Adjust priorities based on current priorities
                let goal1_priority = goals[pos1].priority;
                let goal2_priority = goals[pos2].priority;

                if goal1_priority == goal2_priority {
                    // Increment priority of first goal
                    goals[pos1].priority += 1;
                    goals[pos1].metadata.updated_at = chrono::Utc::now();
                    goals[pos1].metadata.version += 1;
                    modified_goals.push(conflict.goal1.clone());
                }

                Ok(ConflictResolution {
                    resolution_type: ResolutionType::PriorityAdjustment,
                    modified_goals,
                    description: "Adjusted goal priorities to resolve conflict".to_string(),
                    success: true,
                    remaining_conflicts: Vec::new(),
                })
            }
            _ => Ok(ConflictResolution {
                resolution_type: ResolutionType::ManualIntervention,
                modified_goals: Vec::new(),
                description: "Could not find conflicting goals for resolution".to_string(),
                success: false,
                remaining_conflicts: vec![conflict.clone()],
            }),
        }
    }

    fn name(&self) -> &str {
        "PriorityBasedResolver"
    }
}

/// Resource reallocation conflict resolution strategy
pub struct ResourceReallocationResolver;

impl ConflictResolutionStrategy for ResourceReallocationResolver {
    fn resolve_conflict(
        &self,
        conflict: &GoalConflict,
        goals: &mut [Goal],
    ) -> GoalAlignmentResult<ConflictResolution> {
        if !matches!(conflict.conflict_type, ConflictType::Resource) {
            return Ok(ConflictResolution {
                resolution_type: ResolutionType::ManualIntervention,
                modified_goals: Vec::new(),
                description: "Resource reallocation not applicable to this conflict type"
                    .to_string(),
                success: false,
                remaining_conflicts: vec![conflict.clone()],
            });
        }

        let mut modified_goals = Vec::new();

        // Find the conflicting goals
        let goal1_pos = goals.iter().position(|g| g.goal_id == conflict.goal1);
        let goal2_pos = goals.iter().position(|g| g.goal_id == conflict.goal2);

        match (goal1_pos, goal2_pos) {
            (Some(pos1), Some(pos2)) => {
                // Simple resource reallocation: remove shared agents from lower priority goal
                let goal1_priority = goals[pos1].priority;
                let goal2_priority = goals[pos2].priority;

                let (higher_priority_pos, lower_priority_pos) = if goal1_priority > goal2_priority {
                    (pos1, pos2)
                } else {
                    (pos2, pos1)
                };

                // Find shared agents
                let higher_agents: HashSet<_> =
                    goals[higher_priority_pos].assigned_agents.iter().collect();
                let shared_agents: Vec<_> = goals[lower_priority_pos]
                    .assigned_agents
                    .iter()
                    .filter(|agent| higher_agents.contains(agent))
                    .cloned()
                    .collect();

                // Remove shared agents from lower priority goal
                for agent in &shared_agents {
                    goals[lower_priority_pos].unassign_agent(agent)?;
                }

                if !shared_agents.is_empty() {
                    modified_goals.push(goals[lower_priority_pos].goal_id.clone());
                }

                Ok(ConflictResolution {
                    resolution_type: ResolutionType::ResourceReallocation,
                    modified_goals,
                    description: format!(
                        "Reallocated {} shared agents to higher priority goal",
                        shared_agents.len()
                    ),
                    success: !shared_agents.is_empty(),
                    remaining_conflicts: Vec::new(),
                })
            }
            _ => Ok(ConflictResolution {
                resolution_type: ResolutionType::ManualIntervention,
                modified_goals: Vec::new(),
                description: "Could not find conflicting goals for resolution".to_string(),
                success: false,
                remaining_conflicts: vec![conflict.clone()],
            }),
        }
    }

    fn name(&self) -> &str {
        "ResourceReallocationResolver"
    }
}

impl ConflictDetector {
    /// Create new conflict detector with default strategies
    pub fn new() -> Self {
        let mut strategies: HashMap<ConflictType, Box<dyn ConflictDetectionStrategy>> =
            HashMap::new();
        strategies.insert(ConflictType::Resource, Box::new(ResourceConflictDetector));
        strategies.insert(ConflictType::Temporal, Box::new(TemporalConflictDetector));
        strategies.insert(
            ConflictType::Semantic,
            Box::new(SemanticConflictDetector::new(0.6)),
        );

        let mut resolvers: HashMap<ConflictType, Box<dyn ConflictResolutionStrategy>> =
            HashMap::new();
        resolvers.insert(
            ConflictType::Resource,
            Box::new(ResourceReallocationResolver),
        );
        resolvers.insert(ConflictType::Priority, Box::new(PriorityBasedResolver));
        resolvers.insert(ConflictType::Temporal, Box::new(PriorityBasedResolver)); // Use priority for temporal conflicts

        Self {
            strategies,
            resolvers,
        }
    }

    /// Detect all conflicts between a set of goals
    pub fn detect_all_conflicts(&self, goals: &[Goal]) -> GoalAlignmentResult<Vec<GoalConflict>> {
        let mut conflicts = Vec::new();

        for (i, goal1) in goals.iter().enumerate() {
            for goal2 in goals.iter().skip(i + 1) {
                for strategy in self.strategies.values() {
                    if let Some(conflict) = strategy.detect_conflict(goal1, goal2)? {
                        conflicts.push(conflict);
                    }
                }
            }
        }

        Ok(conflicts)
    }

    /// Resolve a conflict using appropriate strategy
    pub fn resolve_conflict(
        &self,
        conflict: &GoalConflict,
        goals: &mut [Goal],
    ) -> GoalAlignmentResult<ConflictResolution> {
        if let Some(resolver) = self.resolvers.get(&conflict.conflict_type) {
            resolver.resolve_conflict(conflict, goals)
        } else {
            Ok(ConflictResolution {
                resolution_type: ResolutionType::ManualIntervention,
                modified_goals: Vec::new(),
                description: format!(
                    "No resolver available for conflict type {:?}",
                    conflict.conflict_type
                ),
                success: false,
                remaining_conflicts: vec![conflict.clone()],
            })
        }
    }

    /// Add custom conflict detection strategy
    pub fn add_detection_strategy(
        &mut self,
        conflict_type: ConflictType,
        strategy: Box<dyn ConflictDetectionStrategy>,
    ) {
        self.strategies.insert(conflict_type, strategy);
    }

    /// Add custom conflict resolution strategy
    pub fn add_resolution_strategy(
        &mut self,
        conflict_type: ConflictType,
        resolver: Box<dyn ConflictResolutionStrategy>,
    ) {
        self.resolvers.insert(conflict_type, resolver);
    }
}

impl Default for ConflictDetector {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{AgentPid, ConstraintType, Goal, GoalConstraint, GoalLevel};

    #[test]
    fn test_resource_conflict_detection() {
        let detector = ResourceConflictDetector;

        let mut goal1 = Goal::new(
            "goal1".to_string(),
            GoalLevel::Local,
            "First goal".to_string(),
            1,
        );

        let mut goal2 = Goal::new(
            "goal2".to_string(),
            GoalLevel::Local,
            "Second goal".to_string(),
            1,
        );

        // Add shared agent
        let shared_agent = AgentPid::new();
        goal1.assign_agent(shared_agent.clone()).unwrap();
        goal2.assign_agent(shared_agent).unwrap();

        let conflict = detector.detect_conflict(&goal1, &goal2).unwrap();
        assert!(conflict.is_some());

        let conflict = conflict.unwrap();
        assert_eq!(conflict.conflict_type, ConflictType::Resource);
        assert!(conflict.severity > 0.0);
    }

    #[test]
    fn test_temporal_conflict_detection() {
        let detector = TemporalConflictDetector;

        let mut goal1 = Goal::new(
            "goal1".to_string(),
            GoalLevel::Local,
            "First goal".to_string(),
            1,
        );
        goal1.update_status(crate::GoalStatus::Active).unwrap();

        let mut goal2 = Goal::new(
            "goal2".to_string(),
            GoalLevel::Local,
            "Second goal".to_string(),
            1, // Same priority
        );
        goal2.update_status(crate::GoalStatus::Active).unwrap();

        let conflict = detector.detect_conflict(&goal1, &goal2).unwrap();
        assert!(conflict.is_some());

        let conflict = conflict.unwrap();
        assert_eq!(conflict.conflict_type, ConflictType::Priority);
    }

    #[test]
    fn test_semantic_conflict_detection() {
        let detector = SemanticConflictDetector::new(0.8);

        let mut goal1 = Goal::new(
            "goal1".to_string(),
            GoalLevel::Local,
            "Planning goal".to_string(),
            1,
        );
        goal1.knowledge_context.concepts = vec!["planning".to_string(), "strategy".to_string()];

        let mut goal2 = Goal::new(
            "goal2".to_string(),
            GoalLevel::Local,
            "Execution goal".to_string(),
            1,
        );
        goal2.knowledge_context.concepts =
            vec!["execution".to_string(), "implementation".to_string()];

        let conflict = detector.detect_conflict(&goal1, &goal2).unwrap();
        assert!(conflict.is_some());

        let conflict = conflict.unwrap();
        assert_eq!(conflict.conflict_type, ConflictType::Semantic);
    }

    #[test]
    fn test_priority_based_resolution() {
        let resolver = PriorityBasedResolver;

        let conflict = GoalConflict {
            goal1: "goal1".to_string(),
            goal2: "goal2".to_string(),
            conflict_type: ConflictType::Priority,
            severity: 0.5,
            description: "Priority conflict".to_string(),
            suggested_resolutions: Vec::new(),
        };

        let mut goals = vec![
            Goal::new(
                "goal1".to_string(),
                GoalLevel::Local,
                "Goal 1".to_string(),
                1,
            ),
            Goal::new(
                "goal2".to_string(),
                GoalLevel::Local,
                "Goal 2".to_string(),
                1,
            ),
        ];

        let resolution = resolver.resolve_conflict(&conflict, &mut goals).unwrap();
        assert!(resolution.success);
        assert_eq!(resolution.modified_goals.len(), 1);

        // Check that priority was adjusted
        assert_eq!(goals[0].priority, 2);
        assert_eq!(goals[1].priority, 1);
    }

    #[test]
    fn test_conflict_detector() {
        let detector = ConflictDetector::new();

        let mut goal1 = Goal::new(
            "goal1".to_string(),
            GoalLevel::Local,
            "First goal".to_string(),
            1,
        );
        goal1.update_status(crate::GoalStatus::Active).unwrap();

        let mut goal2 = Goal::new(
            "goal2".to_string(),
            GoalLevel::Local,
            "Second goal".to_string(),
            1,
        );
        goal2.update_status(crate::GoalStatus::Active).unwrap();

        let goals = vec![goal1, goal2];
        let conflicts = detector.detect_all_conflicts(&goals).unwrap();

        assert!(!conflicts.is_empty());
    }
}
