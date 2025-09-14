//! Agent metadata management with role integration
//!
//! Provides comprehensive metadata storage and management for agents, including
//! role-based specialization using the existing terraphim_rolegraph infrastructure.

use std::collections::HashMap;
use std::time::{Duration, SystemTime};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::{AgentPid, RegistryError, RegistryResult, SupervisorId};

/// Agent role definition integrating with terraphim_rolegraph
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct AgentRole {
    /// Role identifier from the role graph
    pub role_id: String,
    /// Human-readable role name
    pub name: String,
    /// Role description and responsibilities
    pub description: String,
    /// Role hierarchy level (0 = root, higher = more specialized)
    pub hierarchy_level: u32,
    /// Parent roles in the role graph
    pub parent_roles: Vec<String>,
    /// Child roles that can be delegated to
    pub child_roles: Vec<String>,
    /// Role-specific permissions and capabilities
    pub permissions: Vec<String>,
    /// Knowledge domains this role specializes in
    pub knowledge_domains: Vec<String>,
}

impl AgentRole {
    pub fn new(role_id: String, name: String, description: String) -> Self {
        Self {
            role_id,
            name,
            description,
            hierarchy_level: 0,
            parent_roles: Vec::new(),
            child_roles: Vec::new(),
            permissions: Vec::new(),
            knowledge_domains: Vec::new(),
        }
    }

    /// Check if this role can delegate to another role
    pub fn can_delegate_to(&self, other_role: &str) -> bool {
        self.child_roles.contains(&other_role.to_string())
    }

    /// Check if this role inherits from another role
    pub fn inherits_from(&self, parent_role: &str) -> bool {
        self.parent_roles.contains(&parent_role.to_string())
    }

    /// Check if this role has a specific permission
    pub fn has_permission(&self, permission: &str) -> bool {
        self.permissions.contains(&permission.to_string())
    }

    /// Check if this role specializes in a knowledge domain
    pub fn specializes_in_domain(&self, domain: &str) -> bool {
        self.knowledge_domains.contains(&domain.to_string())
    }
}

/// Agent capability definition
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AgentCapability {
    /// Capability identifier
    pub capability_id: String,
    /// Human-readable capability name
    pub name: String,
    /// Detailed capability description
    pub description: String,
    /// Capability category (e.g., "planning", "execution", "analysis")
    pub category: String,
    /// Required knowledge domains
    pub required_domains: Vec<String>,
    /// Input types this capability can handle
    pub input_types: Vec<String>,
    /// Output types this capability produces
    pub output_types: Vec<String>,
    /// Performance metrics and constraints
    pub performance_metrics: CapabilityMetrics,
    /// Dependencies on other capabilities
    pub dependencies: Vec<String>,
}

/// Performance metrics for capabilities
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CapabilityMetrics {
    /// Average execution time
    pub avg_execution_time: Duration,
    /// Success rate (0.0 to 1.0)
    pub success_rate: f64,
    /// Resource usage (memory, CPU, etc.)
    pub resource_usage: ResourceUsage,
    /// Quality score (0.0 to 1.0)
    pub quality_score: f64,
    /// Last updated timestamp
    pub last_updated: DateTime<Utc>,
}

/// Resource usage metrics
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ResourceUsage {
    /// Memory usage in MB
    pub memory_mb: f64,
    /// CPU usage percentage
    pub cpu_percent: f64,
    /// Network bandwidth in KB/s
    pub network_kbps: f64,
    /// Storage usage in MB
    pub storage_mb: f64,
}

impl Default for ResourceUsage {
    fn default() -> Self {
        Self {
            memory_mb: 0.0,
            cpu_percent: 0.0,
            network_kbps: 0.0,
            storage_mb: 0.0,
        }
    }
}

impl Default for CapabilityMetrics {
    fn default() -> Self {
        Self {
            avg_execution_time: Duration::from_secs(1),
            success_rate: 1.0,
            resource_usage: ResourceUsage::default(),
            quality_score: 1.0,
            last_updated: Utc::now(),
        }
    }
}

/// Comprehensive agent metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentMetadata {
    /// Agent identifier
    pub agent_id: AgentPid,
    /// Supervisor managing this agent
    pub supervisor_id: SupervisorId,
    /// Agent's primary role
    pub primary_role: AgentRole,
    /// Additional roles this agent can assume
    pub secondary_roles: Vec<AgentRole>,
    /// Agent capabilities
    pub capabilities: Vec<AgentCapability>,
    /// Agent status and health
    pub status: AgentStatus,
    /// Creation and lifecycle timestamps
    pub lifecycle: AgentLifecycle,
    /// Knowledge graph context
    pub knowledge_context: KnowledgeContext,
    /// Performance and usage statistics
    pub statistics: AgentStatistics,
    /// Custom metadata fields
    pub custom_fields: HashMap<String, serde_json::Value>,
    /// Tags for categorization and search
    pub tags: Vec<String>,
}

/// Agent status information
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum AgentStatus {
    /// Agent is initializing
    Initializing,
    /// Agent is active and available
    Active,
    /// Agent is busy executing tasks
    Busy,
    /// Agent is idle but available
    Idle,
    /// Agent is hibernating to save resources
    Hibernating,
    /// Agent is being terminated
    Terminating,
    /// Agent has been terminated
    Terminated,
    /// Agent has failed and needs attention
    Failed(String),
}

/// Agent lifecycle information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentLifecycle {
    /// When the agent was created
    pub created_at: DateTime<Utc>,
    /// When the agent was last started
    pub started_at: Option<DateTime<Utc>>,
    /// When the agent was last stopped
    pub stopped_at: Option<DateTime<Utc>>,
    /// Total uptime
    pub total_uptime: Duration,
    /// Number of restarts
    pub restart_count: u32,
    /// Last health check timestamp
    pub last_health_check: DateTime<Utc>,
    /// Agent version
    pub version: String,
}

impl Default for AgentLifecycle {
    fn default() -> Self {
        Self {
            created_at: Utc::now(),
            started_at: None,
            stopped_at: None,
            total_uptime: Duration::ZERO,
            restart_count: 0,
            last_health_check: Utc::now(),
            version: "1.0.0".to_string(),
        }
    }
}

/// Knowledge graph context for the agent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeContext {
    /// Knowledge domains the agent operates in
    pub domains: Vec<String>,
    /// Ontology concepts the agent understands
    pub concepts: Vec<String>,
    /// Relationships the agent can work with
    pub relationships: Vec<String>,
    /// Context extraction patterns
    pub extraction_patterns: Vec<String>,
    /// Semantic similarity thresholds
    pub similarity_thresholds: HashMap<String, f64>,
}

impl Default for KnowledgeContext {
    fn default() -> Self {
        Self {
            domains: Vec::new(),
            concepts: Vec::new(),
            relationships: Vec::new(),
            extraction_patterns: Vec::new(),
            similarity_thresholds: HashMap::new(),
        }
    }
}

/// Agent performance and usage statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentStatistics {
    /// Total tasks completed
    pub tasks_completed: u64,
    /// Total tasks failed
    pub tasks_failed: u64,
    /// Average task completion time
    pub avg_completion_time: Duration,
    /// Messages processed
    pub messages_processed: u64,
    /// Knowledge graph queries made
    pub kg_queries: u64,
    /// Role transitions performed
    pub role_transitions: u64,
    /// Resource usage over time
    pub resource_history: Vec<(DateTime<Utc>, ResourceUsage)>,
    /// Performance trends
    pub performance_trends: HashMap<String, Vec<f64>>,
}

impl Default for AgentStatistics {
    fn default() -> Self {
        Self {
            tasks_completed: 0,
            tasks_failed: 0,
            avg_completion_time: Duration::ZERO,
            messages_processed: 0,
            kg_queries: 0,
            role_transitions: 0,
            resource_history: Vec::new(),
            performance_trends: HashMap::new(),
        }
    }
}

impl AgentMetadata {
    /// Create new agent metadata with a primary role
    pub fn new(agent_id: AgentPid, supervisor_id: SupervisorId, primary_role: AgentRole) -> Self {
        Self {
            agent_id,
            supervisor_id,
            primary_role,
            secondary_roles: Vec::new(),
            capabilities: Vec::new(),
            status: AgentStatus::Initializing,
            lifecycle: AgentLifecycle::default(),
            knowledge_context: KnowledgeContext::default(),
            statistics: AgentStatistics::default(),
            custom_fields: HashMap::new(),
            tags: Vec::new(),
        }
    }

    /// Add a secondary role to the agent
    pub fn add_secondary_role(&mut self, role: AgentRole) -> RegistryResult<()> {
        if self
            .secondary_roles
            .iter()
            .any(|r| r.role_id == role.role_id)
        {
            return Err(RegistryError::System(format!(
                "Role {} already exists",
                role.role_id
            )));
        }
        self.secondary_roles.push(role);
        Ok(())
    }

    /// Remove a secondary role from the agent
    pub fn remove_secondary_role(&mut self, role_id: &str) -> RegistryResult<()> {
        let initial_len = self.secondary_roles.len();
        self.secondary_roles.retain(|r| r.role_id != role_id);

        if self.secondary_roles.len() == initial_len {
            return Err(RegistryError::System(format!("Role {} not found", role_id)));
        }

        Ok(())
    }

    /// Check if agent has a specific role (primary or secondary)
    pub fn has_role(&self, role_id: &str) -> bool {
        self.primary_role.role_id == role_id
            || self.secondary_roles.iter().any(|r| r.role_id == role_id)
    }

    /// Get all roles (primary + secondary)
    pub fn get_all_roles(&self) -> Vec<&AgentRole> {
        let mut roles = vec![&self.primary_role];
        roles.extend(self.secondary_roles.iter());
        roles
    }

    /// Add a capability to the agent
    pub fn add_capability(&mut self, capability: AgentCapability) -> RegistryResult<()> {
        if self
            .capabilities
            .iter()
            .any(|c| c.capability_id == capability.capability_id)
        {
            return Err(RegistryError::System(format!(
                "Capability {} already exists",
                capability.capability_id
            )));
        }
        self.capabilities.push(capability);
        Ok(())
    }

    /// Check if agent has a specific capability
    pub fn has_capability(&self, capability_id: &str) -> bool {
        self.capabilities
            .iter()
            .any(|c| c.capability_id == capability_id)
    }

    /// Get capabilities by category
    pub fn get_capabilities_by_category(&self, category: &str) -> Vec<&AgentCapability> {
        self.capabilities
            .iter()
            .filter(|c| c.category == category)
            .collect()
    }

    /// Update agent status
    pub fn update_status(&mut self, status: AgentStatus) {
        self.status = status;
        self.lifecycle.last_health_check = Utc::now();
    }

    /// Record task completion
    pub fn record_task_completion(&mut self, completion_time: Duration, success: bool) {
        if success {
            self.statistics.tasks_completed += 1;
        } else {
            self.statistics.tasks_failed += 1;
        }

        // Update average completion time
        let total_tasks = self.statistics.tasks_completed + self.statistics.tasks_failed;
        if total_tasks > 0 {
            let total_time =
                self.statistics.avg_completion_time.as_nanos() as f64 * (total_tasks - 1) as f64;
            let new_avg = (total_time + completion_time.as_nanos() as f64) / total_tasks as f64;
            self.statistics.avg_completion_time = Duration::from_nanos(new_avg as u64);
        }
    }

    /// Record resource usage
    pub fn record_resource_usage(&mut self, usage: ResourceUsage) {
        self.statistics.resource_history.push((Utc::now(), usage));

        // Keep only the last 1000 entries
        if self.statistics.resource_history.len() > 1000 {
            self.statistics.resource_history.remove(0);
        }
    }

    /// Get success rate
    pub fn get_success_rate(&self) -> f64 {
        let total_tasks = self.statistics.tasks_completed + self.statistics.tasks_failed;
        if total_tasks == 0 {
            1.0
        } else {
            self.statistics.tasks_completed as f64 / total_tasks as f64
        }
    }

    /// Check if agent can handle a specific knowledge domain
    pub fn can_handle_domain(&self, domain: &str) -> bool {
        // Check if any role specializes in this domain
        self.get_all_roles().iter().any(|role| role.specializes_in_domain(domain)) ||
        // Check if knowledge context includes this domain
        self.knowledge_context.domains.contains(&domain.to_string())
    }

    /// Validate metadata consistency
    pub fn validate(&self) -> RegistryResult<()> {
        // Validate role hierarchy
        for secondary_role in &self.secondary_roles {
            if secondary_role.role_id == self.primary_role.role_id {
                return Err(RegistryError::MetadataValidationFailed(
                    self.agent_id.clone(),
                    "Secondary role cannot be the same as primary role".to_string(),
                ));
            }
        }

        // Validate capabilities
        for capability in &self.capabilities {
            if capability.performance_metrics.success_rate < 0.0
                || capability.performance_metrics.success_rate > 1.0
            {
                return Err(RegistryError::MetadataValidationFailed(
                    self.agent_id.clone(),
                    format!(
                        "Invalid success rate for capability {}",
                        capability.capability_id
                    ),
                ));
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_agent_role_creation() {
        let role = AgentRole::new(
            "planner".to_string(),
            "Planning Agent".to_string(),
            "Responsible for task planning and coordination".to_string(),
        );

        assert_eq!(role.role_id, "planner");
        assert_eq!(role.name, "Planning Agent");
        assert_eq!(role.hierarchy_level, 0);
        assert!(role.parent_roles.is_empty());
        assert!(role.child_roles.is_empty());
    }

    #[test]
    fn test_agent_metadata_creation() {
        let agent_id = AgentPid::new();
        let supervisor_id = SupervisorId::new();
        let role = AgentRole::new(
            "worker".to_string(),
            "Worker Agent".to_string(),
            "Executes assigned tasks".to_string(),
        );

        let metadata = AgentMetadata::new(agent_id.clone(), supervisor_id.clone(), role.clone());

        assert_eq!(metadata.agent_id, agent_id);
        assert_eq!(metadata.supervisor_id, supervisor_id);
        assert_eq!(metadata.primary_role, role);
        assert!(metadata.secondary_roles.is_empty());
        assert!(metadata.capabilities.is_empty());
        assert_eq!(metadata.status, AgentStatus::Initializing);
    }

    #[test]
    fn test_role_management() {
        let agent_id = AgentPid::new();
        let supervisor_id = SupervisorId::new();
        let primary_role = AgentRole::new(
            "primary".to_string(),
            "Primary Role".to_string(),
            "Primary role description".to_string(),
        );

        let mut metadata = AgentMetadata::new(agent_id, supervisor_id, primary_role);

        // Add secondary role
        let secondary_role = AgentRole::new(
            "secondary".to_string(),
            "Secondary Role".to_string(),
            "Secondary role description".to_string(),
        );

        metadata.add_secondary_role(secondary_role.clone()).unwrap();
        assert!(metadata.has_role("secondary"));
        assert_eq!(metadata.get_all_roles().len(), 2);

        // Remove secondary role
        metadata.remove_secondary_role("secondary").unwrap();
        assert!(!metadata.has_role("secondary"));
        assert_eq!(metadata.get_all_roles().len(), 1);
    }

    #[test]
    fn test_capability_management() {
        let agent_id = AgentPid::new();
        let supervisor_id = SupervisorId::new();
        let role = AgentRole::new(
            "test".to_string(),
            "Test Role".to_string(),
            "Test role".to_string(),
        );

        let mut metadata = AgentMetadata::new(agent_id, supervisor_id, role);

        let capability = AgentCapability {
            capability_id: "test_capability".to_string(),
            name: "Test Capability".to_string(),
            description: "A test capability".to_string(),
            category: "testing".to_string(),
            required_domains: vec!["test_domain".to_string()],
            input_types: vec!["text".to_string()],
            output_types: vec!["result".to_string()],
            performance_metrics: CapabilityMetrics::default(),
            dependencies: Vec::new(),
        };

        metadata.add_capability(capability).unwrap();
        assert!(metadata.has_capability("test_capability"));

        let test_capabilities = metadata.get_capabilities_by_category("testing");
        assert_eq!(test_capabilities.len(), 1);
    }

    #[test]
    fn test_statistics_tracking() {
        let agent_id = AgentPid::new();
        let supervisor_id = SupervisorId::new();
        let role = AgentRole::new(
            "test".to_string(),
            "Test Role".to_string(),
            "Test role".to_string(),
        );

        let mut metadata = AgentMetadata::new(agent_id, supervisor_id, role);

        // Record successful task
        metadata.record_task_completion(Duration::from_secs(1), true);
        assert_eq!(metadata.statistics.tasks_completed, 1);
        assert_eq!(metadata.statistics.tasks_failed, 0);
        assert_eq!(metadata.get_success_rate(), 1.0);

        // Record failed task
        metadata.record_task_completion(Duration::from_secs(2), false);
        assert_eq!(metadata.statistics.tasks_completed, 1);
        assert_eq!(metadata.statistics.tasks_failed, 1);
        assert_eq!(metadata.get_success_rate(), 0.5);
    }

    #[test]
    fn test_metadata_validation() {
        let agent_id = AgentPid::new();
        let supervisor_id = SupervisorId::new();
        let role = AgentRole::new(
            "test".to_string(),
            "Test Role".to_string(),
            "Test role".to_string(),
        );

        let metadata = AgentMetadata::new(agent_id, supervisor_id, role);

        // Valid metadata should pass validation
        assert!(metadata.validate().is_ok());
    }
}
