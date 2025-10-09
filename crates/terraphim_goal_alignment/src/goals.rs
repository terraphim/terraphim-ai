//! Goal representation and management
//!
//! Provides core goal structures and management functionality for the goal alignment system.

use std::collections::{HashMap, HashSet};
use std::time::Duration;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::{AgentPid, GoalAlignmentError, GoalAlignmentResult};

/// Goal identifier type
pub type GoalId = String;

/// Goal representation with knowledge graph context
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Goal {
    /// Unique goal identifier
    pub goal_id: GoalId,
    /// Goal hierarchy level
    pub level: GoalLevel,
    /// Human-readable goal description
    pub description: String,
    /// Goal priority (higher number = higher priority)
    pub priority: u32,
    /// Goal constraints and requirements
    pub constraints: Vec<GoalConstraint>,
    /// Dependencies on other goals
    pub dependencies: Vec<GoalId>,
    /// Knowledge graph concepts related to this goal
    pub knowledge_context: GoalKnowledgeContext,
    /// Agent roles that can work on this goal
    pub assigned_roles: Vec<String>,
    /// Agents currently assigned to this goal
    pub assigned_agents: Vec<AgentPid>,
    /// Current goal status
    pub status: GoalStatus,
    /// Goal metadata and tracking
    pub metadata: GoalMetadata,
}

/// Goal hierarchy levels
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum GoalLevel {
    /// System-wide strategic objectives
    Global,
    /// Department or team-level objectives
    HighLevel,
    /// Individual agent or task-level objectives
    Local,
}

impl GoalLevel {
    /// Get the numeric level for hierarchy comparisons
    pub fn numeric_level(&self) -> u32 {
        match self {
            GoalLevel::Global => 0,
            GoalLevel::HighLevel => 1,
            GoalLevel::Local => 2,
        }
    }

    /// Check if this level can contain the other level
    pub fn can_contain(&self, other: &GoalLevel) -> bool {
        self.numeric_level() < other.numeric_level()
    }

    /// Get parent level
    pub fn parent_level(&self) -> Option<GoalLevel> {
        match self {
            GoalLevel::Global => None,
            GoalLevel::HighLevel => Some(GoalLevel::Global),
            GoalLevel::Local => Some(GoalLevel::HighLevel),
        }
    }

    /// Get child levels
    pub fn child_levels(&self) -> Vec<GoalLevel> {
        match self {
            GoalLevel::Global => vec![GoalLevel::HighLevel],
            GoalLevel::HighLevel => vec![GoalLevel::Local],
            GoalLevel::Local => vec![],
        }
    }
}

/// Goal constraints and requirements
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GoalConstraint {
    /// Constraint type
    pub constraint_type: ConstraintType,
    /// Constraint description
    pub description: String,
    /// Constraint parameters
    pub parameters: HashMap<String, serde_json::Value>,
    /// Whether this constraint is hard (must be satisfied) or soft (preferred)
    pub is_hard: bool,
    /// Constraint priority for conflict resolution
    pub priority: u32,
}

/// Types of goal constraints
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ConstraintType {
    /// Time-based constraints
    Temporal,
    /// Resource constraints
    Resource,
    /// Dependency constraints
    Dependency,
    /// Quality constraints
    Quality,
    /// Security constraints
    Security,
    /// Business rule constraints
    BusinessRule,
    /// Custom constraint type
    Custom(String),
}

/// Knowledge graph context for goals
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GoalKnowledgeContext {
    /// Knowledge domains this goal operates in
    pub domains: Vec<String>,
    /// Ontology concepts related to this goal
    pub concepts: Vec<String>,
    /// Relationships this goal involves
    pub relationships: Vec<String>,
    /// Keywords for semantic matching
    pub keywords: Vec<String>,
    /// Semantic similarity thresholds
    pub similarity_thresholds: HashMap<String, f64>,
}

impl Default for GoalKnowledgeContext {
    fn default() -> Self {
        Self {
            domains: Vec::new(),
            concepts: Vec::new(),
            relationships: Vec::new(),
            keywords: Vec::new(),
            similarity_thresholds: HashMap::new(),
        }
    }
}

/// Goal execution status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum GoalStatus {
    /// Goal is defined but not yet started
    Pending,
    /// Goal is actively being worked on
    Active,
    /// Goal is temporarily paused
    Paused,
    /// Goal has been completed successfully
    Completed,
    /// Goal has failed or been cancelled
    Failed(String),
    /// Goal is blocked by dependencies or constraints
    Blocked(String),
}

/// Goal metadata and tracking information
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GoalMetadata {
    /// When the goal was created
    pub created_at: DateTime<Utc>,
    /// When the goal was last updated
    pub updated_at: DateTime<Utc>,
    /// Goal creator/owner
    pub created_by: String,
    /// Goal version for change tracking
    pub version: u32,
    /// Expected completion time
    pub expected_duration: Option<Duration>,
    /// Actual start time
    pub started_at: Option<DateTime<Utc>>,
    /// Actual completion time
    pub completed_at: Option<DateTime<Utc>>,
    /// Goal progress (0.0 to 1.0)
    pub progress: f64,
    /// Success metrics
    pub success_criteria: Vec<SuccessCriterion>,
    /// Tags for categorization
    pub tags: Vec<String>,
    /// Custom metadata fields
    pub custom_fields: HashMap<String, serde_json::Value>,
}

impl Default for GoalMetadata {
    fn default() -> Self {
        Self {
            created_at: Utc::now(),
            updated_at: Utc::now(),
            created_by: "system".to_string(),
            version: 1,
            expected_duration: None,
            started_at: None,
            completed_at: None,
            progress: 0.0,
            success_criteria: Vec::new(),
            tags: Vec::new(),
            custom_fields: HashMap::new(),
        }
    }
}

/// Success criteria for goal completion
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SuccessCriterion {
    /// Criterion description
    pub description: String,
    /// Metric to measure
    pub metric: String,
    /// Target value
    pub target_value: f64,
    /// Current value
    pub current_value: f64,
    /// Whether this criterion has been met
    pub is_met: bool,
    /// Weight of this criterion (0.0 to 1.0)
    pub weight: f64,
}

impl Goal {
    /// Create a new goal
    pub fn new(goal_id: GoalId, level: GoalLevel, description: String, priority: u32) -> Self {
        Self {
            goal_id,
            level,
            description,
            priority,
            constraints: Vec::new(),
            dependencies: Vec::new(),
            knowledge_context: GoalKnowledgeContext::default(),
            assigned_roles: Vec::new(),
            assigned_agents: Vec::new(),
            status: GoalStatus::Pending,
            metadata: GoalMetadata::default(),
        }
    }

    /// Add a constraint to the goal
    pub fn add_constraint(&mut self, constraint: GoalConstraint) -> GoalAlignmentResult<()> {
        // Validate constraint
        self.validate_constraint(&constraint)?;
        self.constraints.push(constraint);
        self.metadata.updated_at = Utc::now();
        self.metadata.version += 1;
        Ok(())
    }

    /// Add a dependency to the goal
    pub fn add_dependency(&mut self, dependency_goal_id: GoalId) -> GoalAlignmentResult<()> {
        if dependency_goal_id == self.goal_id {
            return Err(GoalAlignmentError::DependencyCycle(format!(
                "Goal {} cannot depend on itself",
                self.goal_id
            )));
        }

        if !self.dependencies.contains(&dependency_goal_id) {
            self.dependencies.push(dependency_goal_id);
            self.metadata.updated_at = Utc::now();
            self.metadata.version += 1;
        }

        Ok(())
    }

    /// Assign an agent to the goal
    pub fn assign_agent(&mut self, agent_id: AgentPid) -> GoalAlignmentResult<()> {
        if !self.assigned_agents.contains(&agent_id) {
            self.assigned_agents.push(agent_id);
            self.metadata.updated_at = Utc::now();
            self.metadata.version += 1;
        }
        Ok(())
    }

    /// Remove an agent from the goal
    pub fn unassign_agent(&mut self, agent_id: &AgentPid) -> GoalAlignmentResult<()> {
        self.assigned_agents.retain(|id| id != agent_id);
        self.metadata.updated_at = Utc::now();
        self.metadata.version += 1;
        Ok(())
    }

    /// Update goal status
    pub fn update_status(&mut self, status: GoalStatus) -> GoalAlignmentResult<()> {
        let old_status = self.status.clone();
        self.status = status;
        self.metadata.updated_at = Utc::now();
        self.metadata.version += 1;

        // Update timestamps based on status changes
        match (&old_status, &self.status) {
            (GoalStatus::Pending, GoalStatus::Active) => {
                self.metadata.started_at = Some(Utc::now());
            }
            (_, GoalStatus::Completed) | (_, GoalStatus::Failed(_)) => {
                self.metadata.completed_at = Some(Utc::now());
                self.metadata.progress = if matches!(self.status, GoalStatus::Completed) {
                    1.0
                } else {
                    self.metadata.progress
                };
            }
            _ => {}
        }

        Ok(())
    }

    /// Update goal progress
    pub fn update_progress(&mut self, progress: f64) -> GoalAlignmentResult<()> {
        if !(0.0..=1.0).contains(&progress) {
            return Err(GoalAlignmentError::InvalidGoalSpec(
                self.goal_id.clone(),
                "Progress must be between 0.0 and 1.0".to_string(),
            ));
        }

        self.metadata.progress = progress;
        self.metadata.updated_at = Utc::now();
        self.metadata.version += 1;

        // Auto-complete if progress reaches 100%
        if progress >= 1.0 && !matches!(self.status, GoalStatus::Completed) {
            self.update_status(GoalStatus::Completed)?;
        }

        Ok(())
    }

    /// Check if goal can be started (all dependencies met)
    pub fn can_start(&self, completed_goals: &HashSet<GoalId>) -> bool {
        self.dependencies
            .iter()
            .all(|dep| completed_goals.contains(dep))
    }

    /// Check if goal is blocked
    pub fn is_blocked(&self) -> bool {
        matches!(self.status, GoalStatus::Blocked(_))
    }

    /// Check if goal is active
    pub fn is_active(&self) -> bool {
        matches!(self.status, GoalStatus::Active)
    }

    /// Check if goal is completed
    pub fn is_completed(&self) -> bool {
        matches!(self.status, GoalStatus::Completed)
    }

    /// Check if goal has failed
    pub fn has_failed(&self) -> bool {
        matches!(self.status, GoalStatus::Failed(_))
    }

    /// Get goal duration if completed
    pub fn get_duration(&self) -> Option<chrono::Duration> {
        if let (Some(started), Some(completed)) =
            (self.metadata.started_at, self.metadata.completed_at)
        {
            Some(completed - started)
        } else {
            None
        }
    }

    /// Calculate overall success score based on criteria
    pub fn calculate_success_score(&self) -> f64 {
        if self.metadata.success_criteria.is_empty() {
            return if self.is_completed() { 1.0 } else { 0.0 };
        }

        let total_weight: f64 = self
            .metadata
            .success_criteria
            .iter()
            .map(|c| c.weight)
            .sum();
        if total_weight == 0.0 {
            return 0.0;
        }

        let weighted_score: f64 = self
            .metadata
            .success_criteria
            .iter()
            .map(|criterion| {
                let score = if criterion.is_met {
                    1.0
                } else {
                    (criterion.current_value / criterion.target_value)
                        .min(1.0)
                        .max(0.0)
                };
                score * criterion.weight
            })
            .sum();

        weighted_score / total_weight
    }

    /// Validate the goal
    pub fn validate(&self) -> GoalAlignmentResult<()> {
        if self.goal_id.is_empty() {
            return Err(GoalAlignmentError::InvalidGoalSpec(
                self.goal_id.clone(),
                "Goal ID cannot be empty".to_string(),
            ));
        }

        if self.description.is_empty() {
            return Err(GoalAlignmentError::InvalidGoalSpec(
                self.goal_id.clone(),
                "Goal description cannot be empty".to_string(),
            ));
        }

        if !(0.0..=1.0).contains(&self.metadata.progress) {
            return Err(GoalAlignmentError::InvalidGoalSpec(
                self.goal_id.clone(),
                "Progress must be between 0.0 and 1.0".to_string(),
            ));
        }

        // Validate constraints
        for constraint in &self.constraints {
            self.validate_constraint(constraint)?;
        }

        // Validate success criteria weights
        let total_weight: f64 = self
            .metadata
            .success_criteria
            .iter()
            .map(|c| c.weight)
            .sum();
        if !self.metadata.success_criteria.is_empty() && (total_weight - 1.0).abs() > 0.01 {
            return Err(GoalAlignmentError::InvalidGoalSpec(
                self.goal_id.clone(),
                "Success criteria weights must sum to 1.0".to_string(),
            ));
        }

        Ok(())
    }

    /// Validate a constraint
    fn validate_constraint(&self, constraint: &GoalConstraint) -> GoalAlignmentResult<()> {
        if constraint.description.is_empty() {
            return Err(GoalAlignmentError::InvalidGoalSpec(
                self.goal_id.clone(),
                "Constraint description cannot be empty".to_string(),
            ));
        }

        // Add constraint-specific validation based on type
        match &constraint.constraint_type {
            ConstraintType::Temporal => {
                // Validate temporal constraint parameters
                if !constraint.parameters.contains_key("deadline")
                    && !constraint.parameters.contains_key("duration")
                {
                    return Err(GoalAlignmentError::InvalidGoalSpec(
                        self.goal_id.clone(),
                        "Temporal constraint must specify deadline or duration".to_string(),
                    ));
                }
            }
            ConstraintType::Resource => {
                // Validate resource constraint parameters
                if !constraint.parameters.contains_key("resource_type") {
                    return Err(GoalAlignmentError::InvalidGoalSpec(
                        self.goal_id.clone(),
                        "Resource constraint must specify resource_type".to_string(),
                    ));
                }
            }
            _ => {
                // Other constraint types can be validated as needed
            }
        }

        Ok(())
    }
}

/// Goal hierarchy for managing goal relationships
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GoalHierarchy {
    /// All goals in the hierarchy
    pub goals: HashMap<GoalId, Goal>,
    /// Parent-child relationships
    pub parent_child: HashMap<GoalId, Vec<GoalId>>,
    /// Child-parent relationships (reverse index)
    pub child_parent: HashMap<GoalId, GoalId>,
    /// Dependency graph
    pub dependencies: HashMap<GoalId, Vec<GoalId>>,
}

impl GoalHierarchy {
    /// Create a new goal hierarchy
    pub fn new() -> Self {
        Self {
            goals: HashMap::new(),
            parent_child: HashMap::new(),
            child_parent: HashMap::new(),
            dependencies: HashMap::new(),
        }
    }

    /// Add a goal to the hierarchy
    pub fn add_goal(&mut self, goal: Goal) -> GoalAlignmentResult<()> {
        if self.goals.contains_key(&goal.goal_id) {
            return Err(GoalAlignmentError::GoalAlreadyExists(goal.goal_id.clone()));
        }

        // Validate goal
        goal.validate()?;

        // Add dependencies
        if !goal.dependencies.is_empty() {
            self.dependencies
                .insert(goal.goal_id.clone(), goal.dependencies.clone());
        }

        self.goals.insert(goal.goal_id.clone(), goal);
        Ok(())
    }

    /// Remove a goal from the hierarchy
    pub fn remove_goal(&mut self, goal_id: &GoalId) -> GoalAlignmentResult<()> {
        if !self.goals.contains_key(goal_id) {
            return Err(GoalAlignmentError::GoalNotFound(goal_id.clone()));
        }

        // Remove from parent-child relationships
        if let Some(parent_id) = self.child_parent.remove(goal_id) {
            if let Some(children) = self.parent_child.get_mut(&parent_id) {
                children.retain(|id| id != goal_id);
            }
        }

        // Remove children relationships
        if let Some(children) = self.parent_child.remove(goal_id) {
            for child_id in children {
                self.child_parent.remove(&child_id);
            }
        }

        // Remove dependencies
        self.dependencies.remove(goal_id);

        // Remove from other goals' dependencies
        for deps in self.dependencies.values_mut() {
            deps.retain(|id| id != goal_id);
        }

        self.goals.remove(goal_id);
        Ok(())
    }

    /// Set parent-child relationship
    pub fn set_parent_child(
        &mut self,
        parent_id: GoalId,
        child_id: GoalId,
    ) -> GoalAlignmentResult<()> {
        // Validate both goals exist
        if !self.goals.contains_key(&parent_id) {
            return Err(GoalAlignmentError::GoalNotFound(parent_id));
        }
        if !self.goals.contains_key(&child_id) {
            return Err(GoalAlignmentError::GoalNotFound(child_id));
        }

        // Validate hierarchy levels
        let parent_level = &self.goals[&parent_id].level;
        let child_level = &self.goals[&child_id].level;

        if !parent_level.can_contain(child_level) {
            return Err(GoalAlignmentError::HierarchyValidationFailed(format!(
                "Goal level {:?} cannot contain {:?}",
                parent_level, child_level
            )));
        }

        // Add relationship
        self.parent_child
            .entry(parent_id.clone())
            .or_insert_with(Vec::new)
            .push(child_id.clone());
        self.child_parent.insert(child_id, parent_id);

        Ok(())
    }

    /// Get children of a goal
    pub fn get_children(&self, goal_id: &GoalId) -> Vec<&Goal> {
        if let Some(child_ids) = self.parent_child.get(goal_id) {
            child_ids
                .iter()
                .filter_map(|id| self.goals.get(id))
                .collect()
        } else {
            Vec::new()
        }
    }

    /// Get parent of a goal
    pub fn get_parent(&self, goal_id: &GoalId) -> Option<&Goal> {
        self.child_parent
            .get(goal_id)
            .and_then(|parent_id| self.goals.get(parent_id))
    }

    /// Get all goals at a specific level
    pub fn get_goals_by_level(&self, level: &GoalLevel) -> Vec<&Goal> {
        self.goals
            .values()
            .filter(|goal| &goal.level == level)
            .collect()
    }

    /// Check for dependency cycles
    pub fn has_dependency_cycle(&self) -> Option<Vec<GoalId>> {
        // Use DFS to detect cycles
        let mut visited = HashSet::new();
        let mut rec_stack = HashSet::new();

        for goal_id in self.goals.keys() {
            if !visited.contains(goal_id) {
                if let Some(cycle) = self.dfs_cycle_check(goal_id, &mut visited, &mut rec_stack) {
                    return Some(cycle);
                }
            }
        }

        None
    }

    /// DFS helper for cycle detection
    fn dfs_cycle_check(
        &self,
        goal_id: &GoalId,
        visited: &mut HashSet<GoalId>,
        rec_stack: &mut HashSet<GoalId>,
    ) -> Option<Vec<GoalId>> {
        visited.insert(goal_id.clone());
        rec_stack.insert(goal_id.clone());

        if let Some(dependencies) = self.dependencies.get(goal_id) {
            for dep_id in dependencies {
                if !visited.contains(dep_id) {
                    if let Some(cycle) = self.dfs_cycle_check(dep_id, visited, rec_stack) {
                        return Some(cycle);
                    }
                } else if rec_stack.contains(dep_id) {
                    // Found cycle
                    return Some(vec![goal_id.clone(), dep_id.clone()]);
                }
            }
        }

        rec_stack.remove(goal_id);
        None
    }

    /// Get goals that can be started (no pending dependencies)
    pub fn get_startable_goals(&self) -> Vec<&Goal> {
        let completed_goals: HashSet<GoalId> = self
            .goals
            .values()
            .filter(|goal| goal.is_completed())
            .map(|goal| goal.goal_id.clone())
            .collect();

        self.goals
            .values()
            .filter(|goal| {
                matches!(goal.status, GoalStatus::Pending) && goal.can_start(&completed_goals)
            })
            .collect()
    }
}

impl Default for GoalHierarchy {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_goal_creation() {
        let goal = Goal::new(
            "test_goal".to_string(),
            GoalLevel::Local,
            "Test goal description".to_string(),
            1,
        );

        assert_eq!(goal.goal_id, "test_goal");
        assert_eq!(goal.level, GoalLevel::Local);
        assert_eq!(goal.priority, 1);
        assert_eq!(goal.status, GoalStatus::Pending);
        assert_eq!(goal.metadata.progress, 0.0);
    }

    #[test]
    fn test_goal_level_hierarchy() {
        assert!(GoalLevel::Global.can_contain(&GoalLevel::HighLevel));
        assert!(GoalLevel::HighLevel.can_contain(&GoalLevel::Local));
        assert!(!GoalLevel::Local.can_contain(&GoalLevel::Global));

        assert_eq!(GoalLevel::Global.numeric_level(), 0);
        assert_eq!(GoalLevel::HighLevel.numeric_level(), 1);
        assert_eq!(GoalLevel::Local.numeric_level(), 2);
    }

    #[test]
    fn test_goal_constraints() {
        let mut goal = Goal::new(
            "test_goal".to_string(),
            GoalLevel::Local,
            "Test goal".to_string(),
            1,
        );

        let constraint = GoalConstraint {
            constraint_type: ConstraintType::Temporal,
            description: "Must complete within 1 hour".to_string(),
            parameters: {
                let mut params = HashMap::new();
                params.insert("duration".to_string(), serde_json::json!("1h"));
                params
            },
            is_hard: true,
            priority: 1,
        };

        goal.add_constraint(constraint).unwrap();
        assert_eq!(goal.constraints.len(), 1);
        assert_eq!(goal.metadata.version, 2); // Version incremented
    }

    #[test]
    fn test_goal_dependencies() {
        let mut goal = Goal::new(
            "test_goal".to_string(),
            GoalLevel::Local,
            "Test goal".to_string(),
            1,
        );

        // Add valid dependency
        goal.add_dependency("dependency_goal".to_string()).unwrap();
        assert_eq!(goal.dependencies.len(), 1);

        // Try to add self-dependency (should fail)
        let result = goal.add_dependency("test_goal".to_string());
        assert!(result.is_err());
    }

    #[test]
    fn test_goal_progress() {
        let mut goal = Goal::new(
            "test_goal".to_string(),
            GoalLevel::Local,
            "Test goal".to_string(),
            1,
        );

        // Update progress
        goal.update_progress(0.5).unwrap();
        assert_eq!(goal.metadata.progress, 0.5);

        // Complete goal
        goal.update_progress(1.0).unwrap();
        assert_eq!(goal.metadata.progress, 1.0);
        assert!(goal.is_completed());

        // Invalid progress should fail
        let result = goal.update_progress(1.5);
        assert!(result.is_err());
    }

    #[test]
    fn test_goal_hierarchy() {
        let mut hierarchy = GoalHierarchy::new();

        let global_goal = Goal::new(
            "global_goal".to_string(),
            GoalLevel::Global,
            "Global objective".to_string(),
            1,
        );

        let local_goal = Goal::new(
            "local_goal".to_string(),
            GoalLevel::Local,
            "Local objective".to_string(),
            1,
        );

        hierarchy.add_goal(global_goal).unwrap();
        hierarchy.add_goal(local_goal).unwrap();

        // Set parent-child relationship
        hierarchy
            .set_parent_child("global_goal".to_string(), "local_goal".to_string())
            .unwrap();

        let children = hierarchy.get_children(&"global_goal".to_string());
        assert_eq!(children.len(), 1);
        assert_eq!(children[0].goal_id, "local_goal");

        let parent = hierarchy.get_parent(&"local_goal".to_string());
        assert!(parent.is_some());
        assert_eq!(parent.unwrap().goal_id, "global_goal");
    }

    #[test]
    fn test_dependency_cycle_detection() {
        let mut hierarchy = GoalHierarchy::new();

        let mut goal1 = Goal::new(
            "goal1".to_string(),
            GoalLevel::Local,
            "Goal 1".to_string(),
            1,
        );
        let mut goal2 = Goal::new(
            "goal2".to_string(),
            GoalLevel::Local,
            "Goal 2".to_string(),
            1,
        );

        goal1.add_dependency("goal2".to_string()).unwrap();
        goal2.add_dependency("goal1".to_string()).unwrap();

        hierarchy.add_goal(goal1).unwrap();
        hierarchy.add_goal(goal2).unwrap();

        let cycle = hierarchy.has_dependency_cycle();
        assert!(cycle.is_some());
    }
}
