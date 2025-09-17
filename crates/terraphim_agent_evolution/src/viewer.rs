//! Agent evolution viewer for visualizing agent development over time
//!
//! This module provides tools to view and analyze agent memory, task, and lesson evolution.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::{
    AgentEvolutionSystem, AgentId, EvolutionResult, LessonsState, MemoryState, TasksState,
};

/// Viewer for agent evolution that enables querying and visualization
pub struct MemoryEvolutionViewer {
    agent_id: AgentId,
}

impl MemoryEvolutionViewer {
    /// Create a new evolution viewer for the specified agent
    pub fn new(agent_id: AgentId) -> Self {
        Self { agent_id }
    }

    /// Get evolution timeline for a specific time range
    pub async fn get_timeline(
        &self,
        evolution_system: &AgentEvolutionSystem,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> EvolutionResult<EvolutionTimeline> {
        let summary = evolution_system.get_evolution_summary(start, end).await?;
        let mut events = Vec::new();

        // Create events from completed tasks
        let tasks_state = &evolution_system.tasks.current_state;
        for completed_task in &tasks_state.completed {
            if completed_task.completed_at >= start && completed_task.completed_at <= end {
                events.push(EvolutionEvent {
                    timestamp: completed_task.completed_at,
                    event_type: EventType::TaskCompletion,
                    description: format!(
                        "Completed task: {}",
                        completed_task.original_task.content
                    ),
                    impact_score: 0.7,
                });
            }
        }

        // Create events from learned lessons
        let lessons_state = &evolution_system.lessons.current_state;

        // Collect all lessons from different categories
        let mut all_lessons = Vec::new();
        all_lessons.extend(&lessons_state.technical_lessons);
        all_lessons.extend(&lessons_state.process_lessons);
        all_lessons.extend(&lessons_state.domain_lessons);
        all_lessons.extend(&lessons_state.failure_lessons);
        all_lessons.extend(&lessons_state.success_patterns);

        for lesson in all_lessons {
            if lesson.learned_at >= start && lesson.learned_at <= end {
                let (event_type, impact_score) = match lesson.category {
                    crate::lessons::LessonCategory::SuccessPattern => {
                        (EventType::PerformanceImprovement, 0.8)
                    }
                    crate::lessons::LessonCategory::Failure => {
                        (EventType::PerformanceRegression, 0.6)
                    }
                    _ => (EventType::LessonLearned, 0.5),
                };

                events.push(EvolutionEvent {
                    timestamp: lesson.learned_at,
                    event_type,
                    description: format!("Learned: {}", lesson.title),
                    impact_score,
                });
            }
        }

        // Create events from memory consolidations (if any occurred in the time range)
        let memory_state = &evolution_system.memory.current_state;
        if memory_state.metadata.last_updated >= start && memory_state.metadata.last_updated <= end && memory_state.metadata.total_consolidations > 0 {
            events.push(EvolutionEvent {
                timestamp: memory_state.metadata.last_updated,
                event_type: EventType::MemoryConsolidation,
                description: format!(
                    "Consolidated {} memories for better organization",
                    memory_state.total_size()
                ),
                impact_score: 0.4,
            });
        }

        // Sort events by timestamp
        events.sort_by(|a, b| a.timestamp.cmp(&b.timestamp));

        Ok(EvolutionTimeline {
            agent_id: self.agent_id.clone(),
            start_time: start,
            end_time: end,
            total_snapshots: summary.snapshot_count,
            memory_growth_rate: summary.memory_growth.growth_rate,
            task_completion_rate: summary.task_completion_rate,
            learning_velocity: summary.learning_velocity,
            alignment_trend: summary.alignment_trend,
            events,
        })
    }

    /// Get detailed view of agent state at specific time
    pub async fn get_state_at_time(
        &self,
        evolution_system: &AgentEvolutionSystem,
        timestamp: DateTime<Utc>,
    ) -> EvolutionResult<AgentStateView> {
        let snapshot = evolution_system.load_snapshot(timestamp).await?;

        Ok(AgentStateView {
            timestamp,
            memory_summary: MemorySummary::from_state(&snapshot.memory),
            task_summary: TaskSummary::from_state(&snapshot.tasks),
            lesson_summary: LessonSummary::from_state(&snapshot.lessons),
            alignment_score: snapshot.alignment_score,
        })
    }

    /// Compare agent state between two time points
    pub async fn compare_states(
        &self,
        evolution_system: &AgentEvolutionSystem,
        time1: DateTime<Utc>,
        time2: DateTime<Utc>,
    ) -> EvolutionResult<StateComparison> {
        let state1 = self.get_state_at_time(evolution_system, time1).await?;
        let state2 = self.get_state_at_time(evolution_system, time2).await?;

        Ok(StateComparison {
            earlier_state: state1,
            later_state: state2,
            memory_changes: MemoryChanges::default(),
            task_changes: TaskChanges::default(),
            lesson_changes: LessonChanges::default(),
            alignment_change: 0.0, // Would calculate the difference
        })
    }

    /// Get evolution insights and patterns
    pub async fn get_insights(
        &self,
        evolution_system: &AgentEvolutionSystem,
        period: TimePeriod,
    ) -> EvolutionResult<EvolutionInsights> {
        let now = Utc::now();
        let start = match period {
            TimePeriod::LastHour => now - chrono::Duration::hours(1),
            TimePeriod::LastDay => now - chrono::Duration::days(1),
            TimePeriod::LastWeek => now - chrono::Duration::weeks(1),
            TimePeriod::LastMonth => now - chrono::Duration::days(30),
        };

        let _summary = evolution_system.get_evolution_summary(start, now).await?;

        Ok(EvolutionInsights {
            period,
            key_patterns: vec![],
            performance_trends: vec![],
            learning_highlights: vec![],
            alignment_analysis: AlignmentAnalysis::default(),
            recommendations: vec![],
        })
    }
}

/// Timeline view of agent evolution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvolutionTimeline {
    pub agent_id: AgentId,
    pub start_time: DateTime<Utc>,
    pub end_time: DateTime<Utc>,
    pub total_snapshots: usize,
    pub memory_growth_rate: f64,
    pub task_completion_rate: f64,
    pub learning_velocity: f64,
    pub alignment_trend: crate::AlignmentTrend,
    pub events: Vec<EvolutionEvent>,
}

/// Individual event in agent evolution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvolutionEvent {
    pub timestamp: DateTime<Utc>,
    pub event_type: EventType,
    pub description: String,
    pub impact_score: f64,
}

/// Types of evolution events
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EventType {
    MemoryConsolidation,
    TaskCompletion,
    LessonLearned,
    AlignmentShift,
    PerformanceImprovement,
    PerformanceRegression,
}

/// Time period for analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TimePeriod {
    LastHour,
    LastDay,
    LastWeek,
    LastMonth,
}

/// Detailed view of agent state at a specific time
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentStateView {
    pub timestamp: DateTime<Utc>,
    pub memory_summary: MemorySummary,
    pub task_summary: TaskSummary,
    pub lesson_summary: LessonSummary,
    pub alignment_score: f64,
}

/// Summary of memory state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemorySummary {
    pub total_items: usize,
    pub short_term_count: usize,
    pub long_term_count: usize,
    pub working_memory_items: usize,
    pub episodic_memories: usize,
    pub semantic_concepts: usize,
    pub coherence_score: f64,
}

impl MemorySummary {
    fn from_state(state: &MemoryState) -> Self {
        Self {
            total_items: state.total_size(),
            short_term_count: state.short_term.len(),
            long_term_count: state.long_term.len(),
            working_memory_items: state.working_memory.current_context.len(),
            episodic_memories: state.episodic_memory.len(),
            semantic_concepts: state.semantic_memory.concepts.len(),
            coherence_score: state.calculate_coherence_score(),
        }
    }
}

/// Summary of task state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskSummary {
    pub total_tasks: usize,
    pub pending_tasks: usize,
    pub in_progress_tasks: usize,
    pub completed_tasks: usize,
    pub blocked_tasks: usize,
    pub completion_rate: f64,
    pub average_complexity: f64,
}

impl TaskSummary {
    fn from_state(state: &TasksState) -> Self {
        Self {
            total_tasks: state.total_tasks(),
            pending_tasks: state.pending_count(),
            in_progress_tasks: state.in_progress_count(),
            completed_tasks: state.completed_tasks(),
            blocked_tasks: state.blocked_count(),
            completion_rate: state.calculate_completion_rate(),
            average_complexity: state.calculate_average_complexity(),
        }
    }
}

/// Summary of lesson state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LessonSummary {
    pub total_lessons: usize,
    pub technical_lessons: usize,
    pub process_lessons: usize,
    pub domain_lessons: usize,
    pub validated_lessons: usize,
    pub success_rate: f64,
    pub knowledge_coverage: f64,
}

impl LessonSummary {
    fn from_state(state: &LessonsState) -> Self {
        Self {
            total_lessons: state.total_lessons(),
            technical_lessons: state.get_lessons_by_category("technical").len(),
            process_lessons: state.get_lessons_by_category("process").len(),
            domain_lessons: state.get_lessons_by_category("domain").len(),
            validated_lessons: state.metadata.validated_lessons as usize,
            success_rate: state.calculate_success_rate(),
            knowledge_coverage: state.calculate_knowledge_coverage(),
        }
    }
}

/// Comparison between two agent states
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateComparison {
    pub earlier_state: AgentStateView,
    pub later_state: AgentStateView,
    pub memory_changes: MemoryChanges,
    pub task_changes: TaskChanges,
    pub lesson_changes: LessonChanges,
    pub alignment_change: f64,
}

/// Changes in memory between states
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MemoryChanges {
    pub items_added: usize,
    pub items_removed: usize,
    pub consolidations: usize,
    pub coherence_change: f64,
}

/// Changes in tasks between states
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TaskChanges {
    pub tasks_added: usize,
    pub tasks_completed: usize,
    pub tasks_blocked: usize,
    pub completion_rate_change: f64,
}

/// Changes in lessons between states
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LessonChanges {
    pub lessons_learned: usize,
    pub lessons_validated: usize,
    pub knowledge_areas_expanded: usize,
    pub success_rate_change: f64,
}

/// Insights about agent evolution patterns
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvolutionInsights {
    pub period: TimePeriod,
    pub key_patterns: Vec<EvolutionPattern>,
    pub performance_trends: Vec<PerformanceTrend>,
    pub learning_highlights: Vec<LearningHighlight>,
    pub alignment_analysis: AlignmentAnalysis,
    pub recommendations: Vec<String>,
}

/// Detected pattern in agent evolution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvolutionPattern {
    pub pattern_type: String,
    pub description: String,
    pub confidence: f64,
    pub impact: String,
}

/// Performance trend over time
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceTrend {
    pub metric: String,
    pub direction: TrendDirection,
    pub magnitude: f64,
    pub significance: f64,
}

/// Direction of a trend
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TrendDirection {
    Improving,
    Declining,
    Stable,
}

/// Significant learning achievement
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LearningHighlight {
    pub achievement: String,
    pub impact: String,
    pub timestamp: DateTime<Utc>,
}

/// Analysis of goal alignment evolution
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AlignmentAnalysis {
    pub current_score: f64,
    pub trend: String,
    pub key_factors: Vec<String>,
    pub improvement_areas: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_evolution_viewer_creation() {
        let viewer = MemoryEvolutionViewer::new("test_agent".to_string());
        assert_eq!(viewer.agent_id, "test_agent");
    }

    #[test]
    fn test_memory_summary_creation() {
        let state = MemoryState::default();
        let summary = MemorySummary::from_state(&state);
        assert_eq!(summary.total_items, 0);
        assert_eq!(summary.coherence_score, 1.0); // Perfect coherence when empty
    }
}
