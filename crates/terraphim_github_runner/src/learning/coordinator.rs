//! Learning coordination for knowledge graph updates
//!
//! This module coordinates learning from workflow execution outcomes:
//! - Records success and failure patterns
//! - Creates lessons after failure threshold (3 occurrences)
//! - Updates knowledge graph with successful paths
//! - Learns optimal command sequences from execution history

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

use super::knowledge_graph::{CommandGraphStats, CommandKnowledgeGraph};
use crate::models::{WorkflowContext, WorkflowResult};
use crate::Result;

/// Threshold for identical failures before creating a lesson
const FAILURE_THRESHOLD: u32 = 3;

/// Coordinator for learning from workflow execution outcomes
#[async_trait]
pub trait LearningCoordinator: Send + Sync {
    /// Record a successful command execution
    async fn record_success(
        &self,
        command: &str,
        duration_ms: u64,
        context: &WorkflowContext,
    ) -> Result<()>;

    /// Record a failed command execution
    async fn record_failure(
        &self,
        command: &str,
        error: &str,
        context: &WorkflowContext,
    ) -> Result<()>;

    /// Record a complete workflow result
    async fn record_workflow_result(&self, result: &WorkflowResult) -> Result<()>;

    /// Suggest optimizations for a workflow based on learned patterns
    async fn suggest_optimizations(
        &self,
        context: &WorkflowContext,
    ) -> Result<Vec<WorkflowOptimization>>;

    /// Get applicable lessons for a workflow
    async fn get_applicable_lessons(
        &self,
        context: &WorkflowContext,
    ) -> Result<Vec<ApplicableLesson>>;
}

/// Suggested workflow optimization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowOptimization {
    /// Optimization type
    pub optimization_type: OptimizationType,
    /// Description of the optimization
    pub description: String,
    /// Confidence score (0.0-1.0)
    pub confidence: f64,
    /// Expected improvement
    pub expected_improvement: Option<String>,
    /// Related command or step
    pub related_command: Option<String>,
}

/// Types of workflow optimizations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OptimizationType {
    /// Cache certain operations
    CacheOperation,
    /// Parallelize independent steps
    ParallelizeSteps,
    /// Skip unnecessary step
    SkipStep,
    /// Use faster alternative
    UseAlternative,
    /// Avoid known failure pattern
    AvoidFailurePattern,
}

/// A lesson applicable to the current workflow
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApplicableLesson {
    /// Lesson ID
    pub id: String,
    /// Lesson title
    pub title: String,
    /// Why this lesson is applicable
    pub reason: String,
    /// Recommendation based on the lesson
    pub recommendation: String,
    /// Confidence score (0.0-1.0)
    pub confidence: f64,
}

/// Tracks failure occurrences for threshold-based lesson creation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FailureTracker {
    /// Command that failed
    pub command: String,
    /// Error signature (first line or hash)
    pub error_signature: String,
    /// Number of occurrences
    pub occurrences: u32,
    /// First occurrence timestamp
    pub first_seen: DateTime<Utc>,
    /// Last occurrence timestamp
    pub last_seen: DateTime<Utc>,
    /// Contexts where this failure occurred
    pub contexts: Vec<String>,
}

/// Tracks successful command patterns
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuccessPattern {
    /// Command that succeeded
    pub command: String,
    /// Average execution time in milliseconds
    pub avg_duration_ms: f64,
    /// Number of successful executions
    pub success_count: u32,
    /// Failure count (for success rate calculation)
    pub failure_count: u32,
    /// Repository patterns where this works well
    pub repo_patterns: Vec<String>,
    /// Last successful execution
    pub last_success: DateTime<Utc>,
}

/// In-memory learning coordinator implementation
///
/// This implementation tracks patterns locally and can be extended
/// to integrate with terraphim_agent_evolution when the github-runner
/// feature is enabled.
///
/// When a knowledge graph is attached, it also:
/// - Records successful command sequences as weighted edges
/// - Records failures as separate failure edges
/// - Tracks workflow membership for related commands
pub struct InMemoryLearningCoordinator {
    /// Failed command occurrences
    failure_tracker: DashMap<String, FailureTracker>,
    /// Successful command patterns
    success_patterns: DashMap<String, SuccessPattern>,
    /// Created lessons (command -> lesson ID)
    created_lessons: DashMap<String, String>,
    /// Agent ID for lessons (used for lesson creation attribution)
    #[allow(dead_code)]
    agent_id: String,
    /// Optional knowledge graph for command pattern learning
    knowledge_graph: Option<Arc<CommandKnowledgeGraph>>,
    /// Track previous command per session for sequence learning
    previous_command: DashMap<String, String>,
}

impl InMemoryLearningCoordinator {
    /// Create a new in-memory learning coordinator
    pub fn new(agent_id: impl Into<String>) -> Self {
        Self {
            failure_tracker: DashMap::new(),
            success_patterns: DashMap::new(),
            created_lessons: DashMap::new(),
            agent_id: agent_id.into(),
            knowledge_graph: None,
            previous_command: DashMap::new(),
        }
    }

    /// Create a new learning coordinator with knowledge graph integration
    ///
    /// This enables learning command sequences and updating graph weights
    /// based on execution outcomes.
    pub async fn with_knowledge_graph(agent_id: impl Into<String>) -> Result<Self> {
        let kg = CommandKnowledgeGraph::new().await?;
        Ok(Self {
            failure_tracker: DashMap::new(),
            success_patterns: DashMap::new(),
            created_lessons: DashMap::new(),
            agent_id: agent_id.into(),
            knowledge_graph: Some(Arc::new(kg)),
            previous_command: DashMap::new(),
        })
    }

    /// Attach an existing knowledge graph to this coordinator
    pub fn with_existing_knowledge_graph(
        agent_id: impl Into<String>,
        kg: Arc<CommandKnowledgeGraph>,
    ) -> Self {
        Self {
            failure_tracker: DashMap::new(),
            success_patterns: DashMap::new(),
            created_lessons: DashMap::new(),
            agent_id: agent_id.into(),
            knowledge_graph: Some(kg),
            previous_command: DashMap::new(),
        }
    }

    /// Get knowledge graph statistics if available
    pub async fn get_knowledge_graph_stats(&self) -> Option<CommandGraphStats> {
        if let Some(ref kg) = self.knowledge_graph {
            Some(kg.get_stats().await)
        } else {
            None
        }
    }

    /// Check if knowledge graph is attached
    pub fn has_knowledge_graph(&self) -> bool {
        self.knowledge_graph.is_some()
    }

    /// Generate a failure signature from error text
    fn error_signature(error: &str) -> String {
        // Use first line or hash for signature
        let first_line = error.lines().next().unwrap_or(error);
        // Truncate to 100 chars for comparison
        if first_line.len() > 100 {
            first_line[..100].to_string()
        } else {
            first_line.to_string()
        }
    }

    /// Generate a key for failure tracking
    fn failure_key(command: &str, error_signature: &str) -> String {
        format!("{}::{}", command, error_signature)
    }

    /// Check if threshold reached and lesson should be created
    fn should_create_lesson(&self, key: &str) -> bool {
        if let Some(tracker) = self.failure_tracker.get(key) {
            tracker.occurrences >= FAILURE_THRESHOLD && !self.created_lessons.contains_key(key)
        } else {
            false
        }
    }

    /// Create a lesson from failure pattern
    async fn create_lesson_from_failure(&self, key: &str) -> Result<String> {
        let tracker = self.failure_tracker.get(key).ok_or_else(|| {
            crate::error::GitHubRunnerError::Internal("Failure tracker not found".to_string())
        })?;

        let lesson_id = Uuid::new_v4().to_string();

        // Store the lesson ID
        self.created_lessons
            .insert(key.to_string(), lesson_id.clone());

        log::info!(
            "Created lesson {} for command '{}' after {} failures",
            lesson_id,
            tracker.command,
            tracker.occurrences
        );

        Ok(lesson_id)
    }

    /// Update success pattern statistics
    fn update_success_pattern(&self, command: &str, duration_ms: u64, repo_name: &str) {
        self.success_patterns
            .entry(command.to_string())
            .and_modify(|pattern| {
                // Update running average
                let total_duration =
                    pattern.avg_duration_ms * (pattern.success_count as f64) + (duration_ms as f64);
                pattern.success_count += 1;
                pattern.avg_duration_ms = total_duration / (pattern.success_count as f64);
                pattern.last_success = Utc::now();

                // Track repo patterns
                if !pattern.repo_patterns.contains(&repo_name.to_string()) {
                    pattern.repo_patterns.push(repo_name.to_string());
                }
            })
            .or_insert_with(|| SuccessPattern {
                command: command.to_string(),
                avg_duration_ms: duration_ms as f64,
                success_count: 1,
                failure_count: 0,
                repo_patterns: vec![repo_name.to_string()],
                last_success: Utc::now(),
            });
    }

    /// Get statistics about tracked patterns
    pub fn get_stats(&self) -> LearningStats {
        let failure_count = self.failure_tracker.len();
        let success_count = self.success_patterns.len();
        let lessons_created = self.created_lessons.len();

        let total_failures: u32 = self
            .failure_tracker
            .iter()
            .map(|entry| entry.occurrences)
            .sum();
        let total_successes: u32 = self
            .success_patterns
            .iter()
            .map(|entry| entry.success_count)
            .sum();

        LearningStats {
            unique_failure_patterns: failure_count,
            unique_success_patterns: success_count,
            lessons_created,
            total_failures,
            total_successes,
        }
    }
}

/// Statistics about learning progress
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LearningStats {
    /// Number of unique failure patterns tracked
    pub unique_failure_patterns: usize,
    /// Number of unique success patterns tracked
    pub unique_success_patterns: usize,
    /// Number of lessons created
    pub lessons_created: usize,
    /// Total failure occurrences
    pub total_failures: u32,
    /// Total success occurrences
    pub total_successes: u32,
}

#[async_trait]
impl LearningCoordinator for InMemoryLearningCoordinator {
    async fn record_success(
        &self,
        command: &str,
        duration_ms: u64,
        context: &WorkflowContext,
    ) -> Result<()> {
        let repo_name = &context.event.repository.full_name;
        self.update_success_pattern(command, duration_ms, repo_name);

        // Update knowledge graph if available
        if let Some(ref kg) = self.knowledge_graph {
            let session_key = context.session_id.to_string();

            // Record success sequence if there was a previous command
            if let Some(prev_cmd) = self.previous_command.get(&session_key) {
                let context_id = format!("{}:{}", session_key, Uuid::new_v4());
                if let Err(e) = kg
                    .record_success_sequence(&prev_cmd, command, &context_id)
                    .await
                {
                    log::warn!(
                        "Failed to record success sequence in knowledge graph: {}",
                        e
                    );
                }
            }

            // Update previous command for this session
            self.previous_command
                .insert(session_key, command.to_string());
        }

        log::debug!(
            "Recorded success for command '{}' in {} ({}ms)",
            command,
            repo_name,
            duration_ms
        );

        Ok(())
    }

    async fn record_failure(
        &self,
        command: &str,
        error: &str,
        context: &WorkflowContext,
    ) -> Result<()> {
        let error_sig = Self::error_signature(error);
        let key = Self::failure_key(command, &error_sig);
        let repo_name = &context.event.repository.full_name;

        // Update failure tracking
        self.failure_tracker
            .entry(key.clone())
            .and_modify(|tracker| {
                tracker.occurrences += 1;
                tracker.last_seen = Utc::now();
                if !tracker.contexts.contains(&repo_name.to_string()) {
                    tracker.contexts.push(repo_name.to_string());
                }
            })
            .or_insert_with(|| FailureTracker {
                command: command.to_string(),
                error_signature: error_sig.clone(),
                occurrences: 1,
                first_seen: Utc::now(),
                last_seen: Utc::now(),
                contexts: vec![repo_name.to_string()],
            });

        // Update failure count in success pattern if exists
        if let Some(mut pattern) = self.success_patterns.get_mut(command) {
            pattern.failure_count += 1;
        }

        // Record failure in knowledge graph if available
        if let Some(ref kg) = self.knowledge_graph {
            let session_key = context.session_id.to_string();
            let context_id = format!("{}:{}", session_key, Uuid::new_v4());

            if let Err(e) = kg.record_failure(command, &error_sig, &context_id).await {
                log::warn!("Failed to record failure in knowledge graph: {}", e);
            }

            // Clear previous command on failure to break the sequence
            self.previous_command.remove(&session_key);
        }

        log::debug!(
            "Recorded failure for command '{}' in {}: {}",
            command,
            repo_name,
            error_sig
        );

        // Check if we should create a lesson
        if self.should_create_lesson(&key) {
            self.create_lesson_from_failure(&key).await?;
        }

        Ok(())
    }

    async fn record_workflow_result(&self, result: &WorkflowResult) -> Result<()> {
        // Record each step's outcome
        for step in &result.steps {
            match step.status {
                crate::models::ExecutionStatus::Success => {
                    log::debug!("Step '{}' succeeded in {}ms", step.name, step.duration_ms);
                }
                crate::models::ExecutionStatus::Failed => {
                    let error_msg = if step.stderr.is_empty() {
                        "unknown error"
                    } else {
                        &step.stderr
                    };
                    log::debug!("Step '{}' failed: {}", step.name, error_msg);
                }
                _ => {}
            }
        }

        // Record workflow in knowledge graph if available and successful
        if let Some(ref kg) = self.knowledge_graph {
            if result.success {
                // Extract step names as commands for workflow recording
                let commands: Vec<String> = result
                    .steps
                    .iter()
                    .filter(|s| matches!(s.status, crate::models::ExecutionStatus::Success))
                    .map(|s| s.name.clone())
                    .collect();

                if commands.len() >= 2 {
                    let session_id = result.session_id.to_string();
                    if let Err(e) = kg.record_workflow(&commands, &session_id).await {
                        log::warn!("Failed to record workflow in knowledge graph: {}", e);
                    }
                }
            }
        }

        if result.success {
            log::info!(
                "Workflow completed successfully in {}ms",
                result.total_duration_ms
            );
        } else {
            log::warn!("Workflow failed: {}", result.summary);
        }

        Ok(())
    }

    async fn suggest_optimizations(
        &self,
        context: &WorkflowContext,
    ) -> Result<Vec<WorkflowOptimization>> {
        let mut optimizations = Vec::new();
        let _repo_name = &context.event.repository.full_name;

        // Check for known failure patterns
        for entry in self.failure_tracker.iter() {
            let tracker = entry.value();
            if tracker.occurrences >= FAILURE_THRESHOLD {
                optimizations.push(WorkflowOptimization {
                    optimization_type: OptimizationType::AvoidFailurePattern,
                    description: format!(
                        "Command '{}' has failed {} times with error: {}",
                        tracker.command, tracker.occurrences, tracker.error_signature
                    ),
                    confidence: 0.8,
                    expected_improvement: Some("Avoid repeated failures".to_string()),
                    related_command: Some(tracker.command.clone()),
                });
            }
        }

        // Suggest caching for slow successful commands
        for entry in self.success_patterns.iter() {
            let pattern = entry.value();
            if pattern.avg_duration_ms > 30000.0 && pattern.success_count >= 5 {
                optimizations.push(WorkflowOptimization {
                    optimization_type: OptimizationType::CacheOperation,
                    description: format!(
                        "Command '{}' takes ~{:.0}ms on average. Consider caching.",
                        pattern.command, pattern.avg_duration_ms
                    ),
                    confidence: 0.6,
                    expected_improvement: Some(format!(
                        "Save ~{:.0}ms per execution",
                        pattern.avg_duration_ms * 0.8
                    )),
                    related_command: Some(pattern.command.clone()),
                });
            }
        }

        // Use knowledge graph for sequence-based recommendations
        if let Some(ref kg) = self.knowledge_graph {
            let session_key = context.session_id.to_string();

            // Get previous command for this session
            if let Some(prev_cmd) = self.previous_command.get(&session_key) {
                // Find commands that frequently follow the previous command
                if let Ok(related) = kg.find_related_commands(&prev_cmd, 3).await {
                    for cmd in related {
                        let prob = kg.predict_success(&prev_cmd, &cmd).await;
                        if prob > 0.7 {
                            optimizations.push(WorkflowOptimization {
                                optimization_type: OptimizationType::UseAlternative,
                                description: format!(
                                    "Command '{}' has {:.0}% success rate after '{}'",
                                    cmd,
                                    prob * 100.0,
                                    prev_cmd.as_str()
                                ),
                                confidence: prob,
                                expected_improvement: Some(
                                    "Follow successful execution patterns".to_string(),
                                ),
                                related_command: Some(cmd),
                            });
                        }
                    }
                }
            }
        }

        Ok(optimizations)
    }

    async fn get_applicable_lessons(
        &self,
        _context: &WorkflowContext,
    ) -> Result<Vec<ApplicableLesson>> {
        let mut lessons = Vec::new();

        // Return lessons for any created failure patterns
        for entry in self.created_lessons.iter() {
            let key = entry.key();
            let lesson_id = entry.value();

            if let Some(tracker) = self.failure_tracker.get(key) {
                lessons.push(ApplicableLesson {
                    id: lesson_id.clone(),
                    title: format!("Avoid failure: {}", tracker.error_signature),
                    reason: format!(
                        "This command has failed {} times in similar contexts",
                        tracker.occurrences
                    ),
                    recommendation: format!(
                        "Review command '{}' for potential issues before running",
                        tracker.command
                    ),
                    confidence: 0.7 + (tracker.occurrences as f64 * 0.05).min(0.3),
                });
            }
        }

        Ok(lessons)
    }
}

/// Learning coordinator that integrates with terraphim_agent_evolution
#[cfg(feature = "github-runner")]
pub struct EvolutionLearningCoordinator {
    /// Base in-memory coordinator
    inner: InMemoryLearningCoordinator,
    /// Lessons evolution system (using tokio::sync::RwLock for async compatibility)
    lessons: tokio::sync::RwLock<terraphim_agent_evolution::LessonsEvolution>,
}

#[cfg(feature = "github-runner")]
impl EvolutionLearningCoordinator {
    /// Create a new evolution-based learning coordinator
    pub fn new(agent_id: impl Into<String>) -> Self {
        let agent_id = agent_id.into();
        Self {
            inner: InMemoryLearningCoordinator::new(agent_id.clone()),
            lessons: tokio::sync::RwLock::new(terraphim_agent_evolution::LessonsEvolution::new(
                agent_id,
            )),
        }
    }

    /// Create a lesson from failure and store in evolution system
    async fn create_and_store_lesson(&self, tracker: &FailureTracker) -> Result<String> {
        use terraphim_agent_evolution::{Lesson, LessonCategory};

        let lesson = Lesson::new(
            format!("Avoid: {} - {}", tracker.command, tracker.error_signature),
            format!(
                "GitHub workflow execution: {} failures in {} contexts",
                tracker.occurrences,
                tracker.contexts.len()
            ),
            format!(
                "Command '{}' fails with error '{}'. Consider verifying prerequisites or using alternative approach.",
                tracker.command, tracker.error_signature
            ),
            LessonCategory::Failure,
        );

        let lesson_id = lesson.id.clone();

        // Store in evolution system
        let mut lessons = self.lessons.write().await;

        lessons.add_lesson(lesson).await.map_err(|e| {
            crate::error::GitHubRunnerError::Internal(format!("Failed to add lesson: {}", e))
        })?;

        Ok(lesson_id)
    }
}

#[cfg(feature = "github-runner")]
#[async_trait]
impl LearningCoordinator for EvolutionLearningCoordinator {
    async fn record_success(
        &self,
        command: &str,
        duration_ms: u64,
        context: &WorkflowContext,
    ) -> Result<()> {
        self.inner
            .record_success(command, duration_ms, context)
            .await
    }

    async fn record_failure(
        &self,
        command: &str,
        error: &str,
        context: &WorkflowContext,
    ) -> Result<()> {
        // Track in inner coordinator
        self.inner.record_failure(command, error, context).await?;

        // Check if we should create an evolution lesson
        let error_sig = InMemoryLearningCoordinator::error_signature(error);
        let key = InMemoryLearningCoordinator::failure_key(command, &error_sig);

        if self.inner.should_create_lesson(&key) {
            if let Some(tracker) = self.inner.failure_tracker.get(&key) {
                let lesson_id = self.create_and_store_lesson(&tracker).await?;
                self.inner.created_lessons.insert(key, lesson_id);
            }
        }

        Ok(())
    }

    async fn record_workflow_result(&self, result: &WorkflowResult) -> Result<()> {
        self.inner.record_workflow_result(result).await
    }

    async fn suggest_optimizations(
        &self,
        context: &WorkflowContext,
    ) -> Result<Vec<WorkflowOptimization>> {
        self.inner.suggest_optimizations(context).await
    }

    async fn get_applicable_lessons(
        &self,
        context: &WorkflowContext,
    ) -> Result<Vec<ApplicableLesson>> {
        let mut lessons = self.inner.get_applicable_lessons(context).await?;

        // Also check evolution system for additional lessons
        let lessons_reader = self.lessons.read().await;

        let context_str = format!(
            "github workflow {} {}",
            context.event.repository.full_name,
            context.event.action.as_deref().unwrap_or("unknown")
        );

        let applicable = lessons_reader
            .find_applicable_lessons(&context_str)
            .await
            .map_err(|e| {
                crate::error::GitHubRunnerError::Internal(format!("Failed to find lessons: {}", e))
            })?;

        for lesson in applicable {
            lessons.push(ApplicableLesson {
                id: lesson.id,
                title: lesson.title,
                reason: format!("Matches context: {}", lesson.context),
                recommendation: lesson.insight,
                confidence: lesson.confidence,
            });
        }

        Ok(lessons)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{GitHubEvent, GitHubEventType, RepositoryInfo};

    fn create_test_context() -> WorkflowContext {
        WorkflowContext::new(GitHubEvent {
            event_type: GitHubEventType::PullRequest,
            action: Some("opened".to_string()),
            repository: RepositoryInfo {
                full_name: "test/repo".to_string(),
                clone_url: Some("https://github.com/test/repo.git".to_string()),
                default_branch: Some("main".to_string()),
            },
            pull_request: None,
            git_ref: None,
            sha: Some("abc123".to_string()),
            extra: std::collections::HashMap::new(),
        })
    }

    #[tokio::test]
    async fn test_record_success() {
        let coordinator = InMemoryLearningCoordinator::new("test_agent");
        let context = create_test_context();

        coordinator
            .record_success("cargo build", 5000, &context)
            .await
            .unwrap();

        let stats = coordinator.get_stats();
        assert_eq!(stats.unique_success_patterns, 1);
        assert_eq!(stats.total_successes, 1);
    }

    #[tokio::test]
    async fn test_record_failure() {
        let coordinator = InMemoryLearningCoordinator::new("test_agent");
        let context = create_test_context();

        coordinator
            .record_failure("cargo build", "error[E0432]: unresolved import", &context)
            .await
            .unwrap();

        let stats = coordinator.get_stats();
        assert_eq!(stats.unique_failure_patterns, 1);
        assert_eq!(stats.total_failures, 1);
    }

    #[tokio::test]
    async fn test_lesson_creation_threshold() {
        let coordinator = InMemoryLearningCoordinator::new("test_agent");
        let context = create_test_context();

        // Record same failure 3 times
        for _ in 0..3 {
            coordinator
                .record_failure("cargo test", "test failed: assertion failed", &context)
                .await
                .unwrap();
        }

        let stats = coordinator.get_stats();
        assert_eq!(stats.lessons_created, 1);
    }

    #[tokio::test]
    async fn test_different_errors_not_counted_together() {
        let coordinator = InMemoryLearningCoordinator::new("test_agent");
        let context = create_test_context();

        // Record different failures
        coordinator
            .record_failure("cargo build", "error[E0432]: unresolved import", &context)
            .await
            .unwrap();
        coordinator
            .record_failure("cargo build", "error[E0433]: failed to resolve", &context)
            .await
            .unwrap();

        let stats = coordinator.get_stats();
        assert_eq!(stats.unique_failure_patterns, 2);
        assert_eq!(stats.lessons_created, 0); // Neither reached threshold
    }

    #[tokio::test]
    async fn test_suggest_optimizations_for_failures() {
        let coordinator = InMemoryLearningCoordinator::new("test_agent");
        let context = create_test_context();

        // Record same failure 3 times
        for _ in 0..3 {
            coordinator
                .record_failure("cargo test", "test failed", &context)
                .await
                .unwrap();
        }

        let optimizations = coordinator.suggest_optimizations(&context).await.unwrap();
        assert!(!optimizations.is_empty());
        assert!(matches!(
            optimizations[0].optimization_type,
            OptimizationType::AvoidFailurePattern
        ));
    }

    #[tokio::test]
    async fn test_success_pattern_stats() {
        let coordinator = InMemoryLearningCoordinator::new("test_agent");
        let context = create_test_context();

        // Record multiple successes with different durations
        coordinator
            .record_success("cargo build", 1000, &context)
            .await
            .unwrap();
        coordinator
            .record_success("cargo build", 2000, &context)
            .await
            .unwrap();
        coordinator
            .record_success("cargo build", 3000, &context)
            .await
            .unwrap();

        let stats = coordinator.get_stats();
        assert_eq!(stats.unique_success_patterns, 1);
        assert_eq!(stats.total_successes, 3);

        // Check average duration is calculated
        let pattern = coordinator.success_patterns.get("cargo build").unwrap();
        assert_eq!(pattern.avg_duration_ms, 2000.0); // Average of 1000, 2000, 3000
    }

    #[tokio::test]
    async fn test_get_applicable_lessons() {
        let coordinator = InMemoryLearningCoordinator::new("test_agent");
        let context = create_test_context();

        // Create a lesson by reaching failure threshold
        for _ in 0..3 {
            coordinator
                .record_failure("cargo clippy", "warning: unused variable", &context)
                .await
                .unwrap();
        }

        let lessons = coordinator.get_applicable_lessons(&context).await.unwrap();
        assert_eq!(lessons.len(), 1);
        assert!(lessons[0].title.contains("unused variable"));
    }
}
