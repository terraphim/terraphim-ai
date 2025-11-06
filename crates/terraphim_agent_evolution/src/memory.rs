//! Agent memory evolution tracking with time-based versioning

use std::collections::{BTreeMap, HashMap};

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use terraphim_persistence::Persistable;

use crate::{AgentId, EvolutionError, EvolutionResult, MemoryId};

/// Versioned memory evolution system
#[derive(Debug, Clone)]
pub struct MemoryEvolution {
    pub agent_id: AgentId,
    pub current_state: MemoryState,
    pub history: BTreeMap<DateTime<Utc>, MemoryState>,
}

impl MemoryEvolution {
    /// Create a new memory evolution tracker
    pub fn new(agent_id: AgentId) -> Self {
        Self {
            agent_id,
            current_state: MemoryState::default(),
            history: BTreeMap::new(),
        }
    }

    /// Add a new memory item
    pub async fn add_memory(&mut self, memory: MemoryItem) -> EvolutionResult<()> {
        log::debug!("Adding memory item: {}", memory.id);

        // Input validation
        if memory.id.is_empty() {
            return Err(crate::error::EvolutionError::InvalidInput(
                "Memory ID cannot be empty".to_string(),
            ));
        }

        if memory.id.len() > 200 {
            return Err(crate::error::EvolutionError::InvalidInput(
                "Memory ID too long (max 200 characters)".to_string(),
            ));
        }

        if memory.content.len() > 1_000_000 {
            return Err(crate::error::EvolutionError::InvalidInput(
                "Memory content too large (max 1MB)".to_string(),
            ));
        }

        // Prevent duplicate IDs
        if self
            .current_state
            .short_term
            .iter()
            .any(|m| m.id == memory.id)
            || self.current_state.long_term.contains_key(&memory.id)
        {
            return Err(crate::error::EvolutionError::InvalidInput(format!(
                "Memory with ID '{}' already exists",
                memory.id
            )));
        }

        self.current_state.add_memory(memory);
        self.save_current_state().await?;

        Ok(())
    }

    /// Update an existing memory item
    pub async fn update_memory(
        &mut self,
        memory_id: &MemoryId,
        update: MemoryUpdate,
    ) -> EvolutionResult<()> {
        log::debug!("Updating memory item: {}", memory_id);

        self.current_state.update_memory(memory_id, update)?;
        self.save_current_state().await?;

        Ok(())
    }

    /// Consolidate memories (merge related items, archive old ones)
    pub async fn consolidate_memories(&mut self) -> EvolutionResult<ConsolidationResult> {
        log::info!("Consolidating memories for agent {}", self.agent_id);

        let result = self.current_state.consolidate_memories().await?;
        self.save_current_state().await?;

        Ok(result)
    }

    /// Save a versioned snapshot
    pub async fn save_version(&self, timestamp: DateTime<Utc>) -> EvolutionResult<()> {
        let versioned_memory = VersionedMemory {
            agent_id: self.agent_id.clone(),
            timestamp,
            state: self.current_state.clone(),
        };

        versioned_memory.save().await?;
        log::debug!(
            "Saved memory version for agent {} at {}",
            self.agent_id,
            timestamp
        );

        Ok(())
    }

    /// Load memory state at a specific time
    pub async fn load_version(&self, timestamp: DateTime<Utc>) -> EvolutionResult<MemoryState> {
        let mut versioned_memory = VersionedMemory::new(self.get_version_key(timestamp));
        let loaded = versioned_memory.load().await?;
        Ok(loaded.state)
    }

    /// Get the storage key for a specific version
    pub fn get_version_key(&self, timestamp: DateTime<Utc>) -> String {
        format!("agent_{}/memory/v_{}", self.agent_id, timestamp.timestamp())
    }

    /// Save the current state
    async fn save_current_state(&self) -> EvolutionResult<()> {
        let current_memory = CurrentMemoryState {
            agent_id: self.agent_id.clone(),
            state: self.current_state.clone(),
        };

        current_memory.save().await?;
        Ok(())
    }

    /// Record workflow start in memory
    pub async fn record_workflow_start(
        &mut self,
        workflow_id: uuid::Uuid,
        input: &str,
    ) -> EvolutionResult<()> {
        let memory = MemoryItem {
            id: format!("workflow_start_{}", workflow_id),
            item_type: MemoryItemType::WorkflowEvent,
            content: format!("Started workflow {} with input: {}", workflow_id, input),
            created_at: Utc::now(),
            last_accessed: None,
            access_count: 0,
            importance: ImportanceLevel::Medium,
            tags: vec!["workflow".to_string(), "start".to_string()],
            associations: HashMap::new(),
        };

        self.add_memory(memory).await
    }

    /// Record step execution in memory
    pub async fn record_step_result(&mut self, step_id: &str, result: &str) -> EvolutionResult<()> {
        let memory = MemoryItem {
            id: format!("step_result_{}", step_id),
            item_type: MemoryItemType::ExecutionResult,
            content: format!("Step {} completed with result: {}", step_id, result),
            created_at: Utc::now(),
            last_accessed: None,
            access_count: 0,
            importance: ImportanceLevel::Medium,
            tags: vec!["execution".to_string(), "step".to_string()],
            associations: HashMap::new(),
        };

        self.add_memory(memory).await
    }
}

/// Current memory state of an agent
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MemoryState {
    pub short_term: Vec<MemoryItem>,
    pub long_term: HashMap<MemoryId, MemoryItem>,
    pub working_memory: WorkingMemory,
    pub episodic_memory: Vec<Episode>,
    pub semantic_memory: SemanticMemory,
    pub metadata: MemoryMetadata,
}

impl MemoryState {
    /// Add a new memory item
    pub fn add_memory(&mut self, memory: MemoryItem) {
        match memory.importance {
            ImportanceLevel::Critical | ImportanceLevel::High => {
                self.long_term.insert(memory.id.clone(), memory);
            }
            _ => {
                self.short_term.push(memory);
                // Keep short-term memory bounded
                if self.short_term.len() > 100 {
                    self.short_term.remove(0);
                }
            }
        }
        self.metadata.last_updated = Utc::now();
    }

    /// Update an existing memory item
    pub fn update_memory(
        &mut self,
        memory_id: &MemoryId,
        update: MemoryUpdate,
    ) -> EvolutionResult<()> {
        // Try long-term first
        if let Some(memory) = self.long_term.get_mut(memory_id) {
            memory.apply_update(update);
            self.metadata.last_updated = Utc::now();
            return Ok(());
        }

        // Try short-term
        if let Some(memory) = self.short_term.iter_mut().find(|m| m.id == *memory_id) {
            memory.apply_update(update);
            self.metadata.last_updated = Utc::now();
            return Ok(());
        }

        Err(EvolutionError::MemoryNotFound(memory_id.clone()))
    }

    /// Consolidate memories
    pub async fn consolidate_memories(&mut self) -> EvolutionResult<ConsolidationResult> {
        let mut result = ConsolidationResult::default();

        // Move important short-term memories to long-term
        let mut to_promote = Vec::new();
        self.short_term.retain(|memory| {
            if memory.importance >= ImportanceLevel::High || memory.access_count > 5 {
                to_promote.push(memory.clone());
                result.promoted_to_longterm += 1;
                false
            } else {
                true
            }
        });

        for memory in to_promote {
            self.long_term.insert(memory.id.clone(), memory);
        }

        // Archive old memories
        let cutoff = Utc::now() - chrono::Duration::days(30);
        let mut to_archive = Vec::new();

        self.long_term.retain(|id, memory| {
            if memory.created_at < cutoff && memory.access_count < 2 {
                to_archive.push(id.clone());
                result.archived += 1;
                false
            } else {
                true
            }
        });

        result.consolidation_timestamp = Utc::now();
        Ok(result)
    }

    /// Calculate memory coherence score
    pub fn calculate_coherence_score(&self) -> f64 {
        if self.total_size() == 0 {
            return 1.0; // Perfect coherence if no memories
        }

        let total_items = self.total_size() as f64;
        let tagged_items = self.count_tagged_items() as f64;
        let associated_items = self.count_associated_items() as f64;

        // Simple coherence based on organization
        (tagged_items + associated_items) / (total_items * 2.0)
    }

    /// Get total memory size
    pub fn total_size(&self) -> usize {
        self.short_term.len() + self.long_term.len()
    }

    /// Count tagged memory items
    fn count_tagged_items(&self) -> usize {
        self.short_term
            .iter()
            .filter(|m| !m.tags.is_empty())
            .count()
            + self
                .long_term
                .values()
                .filter(|m| !m.tags.is_empty())
                .count()
    }

    /// Count associated memory items
    fn count_associated_items(&self) -> usize {
        self.short_term
            .iter()
            .filter(|m| !m.associations.is_empty())
            .count()
            + self
                .long_term
                .values()
                .filter(|m| !m.associations.is_empty())
                .count()
    }
}

/// Individual memory item
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryItem {
    pub id: MemoryId,
    pub item_type: MemoryItemType,
    pub content: String,
    pub created_at: DateTime<Utc>,
    pub last_accessed: Option<DateTime<Utc>>,
    pub access_count: u32,
    pub importance: ImportanceLevel,
    pub tags: Vec<String>,
    pub associations: HashMap<String, String>,
}

impl MemoryItem {
    /// Apply an update to this memory item
    pub fn apply_update(&mut self, update: MemoryUpdate) {
        if let Some(content) = update.content {
            self.content = content;
        }
        if let Some(importance) = update.importance {
            self.importance = importance;
        }
        if let Some(tags) = update.tags {
            self.tags = tags;
        }
        self.last_accessed = Some(Utc::now());
        self.access_count += 1;
    }
}

/// Types of memory items
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MemoryItemType {
    Fact,
    Experience,
    Skill,
    Concept,
    WorkflowEvent,
    ExecutionResult,
    LessonLearned,
    Goal,
}

/// Importance levels for memory items
#[derive(Debug, Clone, PartialEq, PartialOrd, Serialize, Deserialize)]
pub enum ImportanceLevel {
    Low,
    Medium,
    High,
    Critical,
}

/// Working memory for current context
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct WorkingMemory {
    pub current_context: HashMap<String, String>,
    pub active_goals: Vec<String>,
    pub attention_focus: Vec<String>,
}

/// Episodic memory for specific experiences
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Episode {
    pub id: String,
    pub description: String,
    pub timestamp: DateTime<Utc>,
    pub outcome: EpisodeOutcome,
    pub learned: Vec<String>,
}

/// Episode outcomes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EpisodeOutcome {
    Success,
    Failure,
    PartialSuccess,
    Learning,
}

/// Semantic memory for concepts and relationships
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SemanticMemory {
    pub concepts: HashMap<String, Concept>,
    pub relationships: Vec<ConceptRelationship>,
}

/// Individual concept
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Concept {
    pub name: String,
    pub definition: String,
    pub confidence: f64,
    pub last_reinforced: DateTime<Utc>,
}

/// Relationship between concepts
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConceptRelationship {
    pub from_concept: String,
    pub to_concept: String,
    pub relationship_type: String,
    pub strength: f64,
}

/// Memory metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryMetadata {
    pub created_at: DateTime<Utc>,
    pub last_updated: DateTime<Utc>,
    pub total_consolidations: u32,
    pub memory_efficiency: f64,
}

impl Default for MemoryMetadata {
    fn default() -> Self {
        let now = Utc::now();
        Self {
            created_at: now,
            last_updated: now,
            total_consolidations: 0,
            memory_efficiency: 1.0,
        }
    }
}

/// Update structure for memory items
#[derive(Debug, Clone, Default)]
pub struct MemoryUpdate {
    pub content: Option<String>,
    pub importance: Option<ImportanceLevel>,
    pub tags: Option<Vec<String>>,
}

/// Result of memory consolidation
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ConsolidationResult {
    pub consolidation_timestamp: DateTime<Utc>,
    pub promoted_to_longterm: usize,
    pub archived: usize,
    pub merged: usize,
    pub efficiency_gain: f64,
}

/// Versioned memory for persistence
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionedMemory {
    pub agent_id: AgentId,
    pub timestamp: DateTime<Utc>,
    pub state: MemoryState,
}

#[async_trait]
impl Persistable for VersionedMemory {
    fn new(key: String) -> Self {
        // Extract agent_id from key format: "agent_{agent_id}/memory/v_{timestamp}"
        let agent_id = key
            .split('/')
            .next()
            .and_then(|s| s.strip_prefix("agent_"))
            .unwrap_or_default()
            .to_string();

        Self {
            agent_id,
            timestamp: Utc::now(),
            state: MemoryState::default(),
        }
    }

    async fn save(&self) -> terraphim_persistence::Result<()> {
        self.save_to_all().await
    }

    async fn save_to_one(&self, profile_name: &str) -> terraphim_persistence::Result<()> {
        self.save_to_profile(profile_name).await
    }

    async fn load(&mut self) -> terraphim_persistence::Result<Self> {
        let key = self.get_key();
        self.load_from_operator(
            &key,
            &terraphim_persistence::DeviceStorage::instance()
                .await?
                .fastest_op,
        )
        .await
    }

    fn get_key(&self) -> String {
        format!(
            "agent_{}/memory/v_{}",
            self.agent_id,
            self.timestamp.timestamp()
        )
    }
}

/// Current memory state for persistence
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CurrentMemoryState {
    pub agent_id: AgentId,
    pub state: MemoryState,
}

#[async_trait]
impl Persistable for CurrentMemoryState {
    fn new(key: String) -> Self {
        Self {
            agent_id: key,
            state: MemoryState::default(),
        }
    }

    async fn save(&self) -> terraphim_persistence::Result<()> {
        self.save_to_all().await
    }

    async fn save_to_one(&self, profile_name: &str) -> terraphim_persistence::Result<()> {
        self.save_to_profile(profile_name).await
    }

    async fn load(&mut self) -> terraphim_persistence::Result<Self> {
        let key = self.get_key();
        self.load_from_operator(
            &key,
            &terraphim_persistence::DeviceStorage::instance()
                .await?
                .fastest_op,
        )
        .await
    }

    fn get_key(&self) -> String {
        format!("agent_{}/memory/current", self.agent_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_memory_evolution_creation() {
        let agent_id = "test_agent".to_string();
        let memory = MemoryEvolution::new(agent_id.clone());

        assert_eq!(memory.agent_id, agent_id);
        assert_eq!(memory.current_state.total_size(), 0);
    }

    #[tokio::test]
    async fn test_add_memory_item() {
        let mut memory = MemoryEvolution::new("test_agent".to_string());

        let item = MemoryItem {
            id: "test_memory".to_string(),
            item_type: MemoryItemType::Fact,
            content: "Test memory content".to_string(),
            created_at: Utc::now(),
            last_accessed: None,
            access_count: 0,
            importance: ImportanceLevel::Medium,
            tags: vec!["test".to_string()],
            associations: HashMap::new(),
        };

        memory.add_memory(item).await.unwrap();
        assert_eq!(memory.current_state.short_term.len(), 1);
    }

    #[tokio::test]
    async fn test_memory_consolidation() {
        let mut memory_state = MemoryState::default();

        // Add a medium importance memory that should be promoted due to access count
        let frequently_accessed_memory = MemoryItem {
            id: "frequently_accessed".to_string(),
            item_type: MemoryItemType::Fact,
            content: "Important fact".to_string(),
            created_at: Utc::now(),
            last_accessed: Some(Utc::now()),
            access_count: 6, // More than 5, so should be promoted
            importance: ImportanceLevel::Medium,
            tags: vec![],
            associations: HashMap::new(),
        };

        memory_state.add_memory(frequently_accessed_memory);
        assert_eq!(memory_state.short_term.len(), 1); // Verify it's in short term

        let result = memory_state.consolidate_memories().await.unwrap();
        assert_eq!(result.promoted_to_longterm, 1);
        assert_eq!(memory_state.long_term.len(), 1);
        assert_eq!(memory_state.short_term.len(), 0); // Should be moved out
    }
}
