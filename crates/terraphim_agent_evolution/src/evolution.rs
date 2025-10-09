//! Core agent evolution system that coordinates all three tracking components

use chrono::{DateTime, Utc};
use futures::try_join;
use serde::{Deserialize, Serialize};

use crate::{AgentId, EvolutionResult, LessonsEvolution, MemoryEvolution, TasksEvolution};

/// Complete agent evolution system that tracks memory, tasks, and lessons
#[derive(Debug, Clone)]
pub struct AgentEvolutionSystem {
    pub agent_id: AgentId,
    pub memory: MemoryEvolution,
    pub tasks: TasksEvolution,
    pub lessons: LessonsEvolution,
}

impl AgentEvolutionSystem {
    /// Create a new agent evolution system
    pub fn new(agent_id: AgentId) -> Self {
        Self {
            agent_id: agent_id.clone(),
            memory: MemoryEvolution::new(agent_id.clone()),
            tasks: TasksEvolution::new(agent_id.clone()),
            lessons: LessonsEvolution::new(agent_id.clone()),
        }
    }

    /// Create a snapshot with a description  
    pub async fn create_snapshot(&self, description: String) -> EvolutionResult<()> {
        log::info!("Creating snapshot: {}", description);
        self.save_snapshot().await
    }

    /// Save a complete snapshot of all three components with atomic versioning
    pub async fn save_snapshot(&self) -> EvolutionResult<()> {
        let timestamp = Utc::now();

        log::debug!(
            "Saving evolution snapshot for agent {} at {}",
            self.agent_id,
            timestamp
        );

        // Save all three components concurrently with the same timestamp for consistency
        let (_, _, _, _) = try_join!(
            self.memory.save_version(timestamp),
            self.tasks.save_version(timestamp),
            self.lessons.save_version(timestamp),
            self.save_evolution_index(timestamp)
        )?;

        log::info!(
            "✅ Saved complete evolution snapshot for agent {}",
            self.agent_id
        );
        Ok(())
    }

    /// Load complete state at any point in time
    pub async fn load_snapshot(&self, timestamp: DateTime<Utc>) -> EvolutionResult<AgentSnapshot> {
        log::debug!(
            "Loading evolution snapshot for agent {} at {}",
            self.agent_id,
            timestamp
        );

        Ok(AgentSnapshot {
            agent_id: self.agent_id.clone(),
            timestamp,
            memory: self.memory.load_version(timestamp).await?,
            tasks: self.tasks.load_version(timestamp).await?,
            lessons: self.lessons.load_version(timestamp).await?,
            alignment_score: self.calculate_alignment_at(timestamp).await?,
        })
    }

    /// Get evolution summary for a time range
    pub async fn get_evolution_summary(
        &self,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> EvolutionResult<EvolutionSummary> {
        let snapshots = self.get_snapshots_in_range(start, end).await?;

        Ok(EvolutionSummary {
            agent_id: self.agent_id.clone(),
            time_range: (start, end),
            snapshot_count: snapshots.len(),
            memory_growth: self.calculate_memory_growth(&snapshots),
            task_completion_rate: self.calculate_task_completion_rate(&snapshots),
            learning_velocity: self.calculate_learning_velocity(&snapshots),
            alignment_trend: self.calculate_alignment_trend(&snapshots),
        })
    }

    /// Save evolution index for efficient querying
    async fn save_evolution_index(&self, timestamp: DateTime<Utc>) -> EvolutionResult<()> {
        use terraphim_persistence::Persistable;

        let index = EvolutionIndex {
            agent_id: self.agent_id.clone(),
            timestamp,
            memory_snapshot_key: self.memory.get_version_key(timestamp),
            tasks_snapshot_key: self.tasks.get_version_key(timestamp),
            lessons_snapshot_key: self.lessons.get_version_key(timestamp),
        };

        index.save().await?;
        Ok(())
    }

    /// Calculate goal alignment at a specific time
    async fn calculate_alignment_at(&self, timestamp: DateTime<Utc>) -> EvolutionResult<f64> {
        // Simplified alignment calculation - can be enhanced
        let memory_state = self.memory.load_version(timestamp).await?;
        let tasks_state = self.tasks.load_version(timestamp).await?;

        // Calculate alignment based on task completion and memory coherence
        let task_alignment = tasks_state.calculate_alignment_score();
        let memory_alignment = memory_state.calculate_coherence_score();

        Ok(task_alignment * 0.6 + memory_alignment * 0.4)
    }

    /// Get all snapshots in a time range
    async fn get_snapshots_in_range(
        &self,
        _start: DateTime<Utc>,
        _end: DateTime<Utc>,
    ) -> EvolutionResult<Vec<AgentSnapshot>> {
        // Implementation would query the evolution index and load snapshots
        // For now, return empty vector
        Ok(vec![])
    }

    /// Calculate memory growth metrics
    fn calculate_memory_growth(&self, snapshots: &[AgentSnapshot]) -> MemoryGrowthMetrics {
        if snapshots.is_empty() {
            return MemoryGrowthMetrics::default();
        }

        let start_memory_size = snapshots
            .first()
            .map(|s| s.memory.total_size())
            .unwrap_or(0);
        let end_memory_size = snapshots.last().map(|s| s.memory.total_size()).unwrap_or(0);

        MemoryGrowthMetrics {
            initial_size: start_memory_size,
            final_size: end_memory_size,
            growth_rate: if start_memory_size > 0 {
                (end_memory_size as f64 - start_memory_size as f64) / start_memory_size as f64
            } else {
                0.0
            },
            consolidation_events: 0, // Would track memory consolidation
        }
    }

    /// Calculate task completion rate
    fn calculate_task_completion_rate(&self, snapshots: &[AgentSnapshot]) -> f64 {
        if snapshots.is_empty() {
            return 0.0;
        }

        let total_tasks: usize = snapshots.iter().map(|s| s.tasks.total_tasks()).sum();
        let completed_tasks: usize = snapshots.iter().map(|s| s.tasks.completed_tasks()).sum();

        if total_tasks > 0 {
            completed_tasks as f64 / total_tasks as f64
        } else {
            0.0
        }
    }

    /// Calculate learning velocity
    fn calculate_learning_velocity(&self, snapshots: &[AgentSnapshot]) -> f64 {
        if snapshots.len() < 2 {
            return 0.0;
        }

        let first_snapshot = snapshots
            .first()
            .expect("snapshots should have at least 2 elements");
        let last_snapshot = snapshots
            .last()
            .expect("snapshots should have at least 2 elements");
        let start_lessons = first_snapshot.lessons.total_lessons();
        let end_lessons = last_snapshot.lessons.total_lessons();
        let time_diff = last_snapshot.timestamp - first_snapshot.timestamp;

        if time_diff.num_hours() > 0 {
            (end_lessons - start_lessons) as f64 / time_diff.num_hours() as f64
        } else {
            0.0
        }
    }

    /// Calculate alignment trend
    fn calculate_alignment_trend(&self, snapshots: &[AgentSnapshot]) -> AlignmentTrend {
        if snapshots.len() < 2 {
            return AlignmentTrend::Stable;
        }

        let first_alignment = snapshots
            .first()
            .expect("snapshots should have at least 2 elements")
            .alignment_score;
        let last_alignment = snapshots
            .last()
            .expect("snapshots should have at least 2 elements")
            .alignment_score;
        let diff = last_alignment - first_alignment;

        if diff > 0.1 {
            AlignmentTrend::Improving
        } else if diff < -0.1 {
            AlignmentTrend::Declining
        } else {
            AlignmentTrend::Stable
        }
    }
}

/// Complete snapshot of agent state at a specific time
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentSnapshot {
    pub agent_id: AgentId,
    pub timestamp: DateTime<Utc>,
    pub memory: crate::MemoryState,
    pub tasks: crate::TasksState,
    pub lessons: crate::LessonsState,
    pub alignment_score: f64,
}

/// Evolution index for efficient querying
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvolutionIndex {
    pub agent_id: AgentId,
    pub timestamp: DateTime<Utc>,
    pub memory_snapshot_key: String,
    pub tasks_snapshot_key: String,
    pub lessons_snapshot_key: String,
}

#[async_trait::async_trait]
impl terraphim_persistence::Persistable for EvolutionIndex {
    fn new(key: String) -> Self {
        Self {
            agent_id: key,
            timestamp: Utc::now(),
            memory_snapshot_key: String::new(),
            tasks_snapshot_key: String::new(),
            lessons_snapshot_key: String::new(),
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
            "agent_{}/evolution/index/{}",
            self.agent_id,
            self.timestamp.timestamp()
        )
    }
}

/// Summary of agent evolution over a time period
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvolutionSummary {
    pub agent_id: AgentId,
    pub time_range: (DateTime<Utc>, DateTime<Utc>),
    pub snapshot_count: usize,
    pub memory_growth: MemoryGrowthMetrics,
    pub task_completion_rate: f64,
    pub learning_velocity: f64,
    pub alignment_trend: AlignmentTrend,
}

/// Memory growth metrics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MemoryGrowthMetrics {
    pub initial_size: usize,
    pub final_size: usize,
    pub growth_rate: f64,
    pub consolidation_events: usize,
}

/// Alignment trend over time
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AlignmentTrend {
    Improving,
    Stable,
    Declining,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_agent_evolution_system_creation() {
        let agent_id = "test_agent".to_string();
        let evolution = AgentEvolutionSystem::new(agent_id.clone());

        assert_eq!(evolution.agent_id, agent_id);
        assert_eq!(evolution.memory.agent_id, agent_id);
        assert_eq!(evolution.tasks.agent_id, agent_id);
        assert_eq!(evolution.lessons.agent_id, agent_id);
    }

    #[tokio::test]
    async fn test_evolution_summary_calculation() {
        let agent_id = "test_agent".to_string();
        let evolution = AgentEvolutionSystem::new(agent_id);

        let now = Utc::now();
        let earlier = now - chrono::Duration::hours(1);

        let summary = evolution.get_evolution_summary(earlier, now).await.unwrap();
        assert_eq!(summary.snapshot_count, 0); // No snapshots yet
        assert_eq!(summary.task_completion_rate, 0.0);
        assert_eq!(summary.learning_velocity, 0.0);
    }
}
