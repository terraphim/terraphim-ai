//! Agent lessons learned evolution with comprehensive learning management

use std::collections::{BTreeMap, HashMap};

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use terraphim_persistence::Persistable;
use uuid::Uuid;

use crate::{AgentId, EvolutionError, EvolutionResult, LessonId, MemoryId, TaskId};

/// Versioned lessons learned evolution system
#[derive(Debug, Clone)]
pub struct LessonsEvolution {
    pub agent_id: AgentId,
    pub current_state: LessonsState,
    pub history: BTreeMap<DateTime<Utc>, LessonsState>,
}

impl LessonsEvolution {
    /// Create a new lessons evolution tracker
    pub fn new(agent_id: AgentId) -> Self {
        Self {
            agent_id,
            current_state: LessonsState::default(),
            history: BTreeMap::new(),
        }
    }

    /// Add a new lesson
    pub async fn add_lesson(&mut self, lesson: Lesson) -> EvolutionResult<()> {
        log::info!("Adding lesson: {} - {}", lesson.id, lesson.title);

        self.current_state.add_lesson(lesson);
        self.save_current_state().await?;

        Ok(())
    }

    /// Apply a lesson to a task or situation
    pub async fn apply_lesson(
        &mut self,
        lesson_id: &LessonId,
        context: &str,
    ) -> EvolutionResult<ApplicationResult> {
        log::debug!("Applying lesson {} in context: {}", lesson_id, context);

        let result = self.current_state.apply_lesson(lesson_id, context)?;
        self.save_current_state().await?;

        Ok(result)
    }

    /// Validate a lesson with evidence
    pub async fn validate_lesson(
        &mut self,
        lesson_id: &LessonId,
        evidence: Evidence,
    ) -> EvolutionResult<()> {
        log::debug!("Validating lesson {} with evidence", lesson_id);

        self.current_state.validate_lesson(lesson_id, evidence)?;
        self.save_current_state().await?;

        Ok(())
    }

    /// Find applicable lessons for a given context
    pub async fn find_applicable_lessons(&self, context: &str) -> EvolutionResult<Vec<Lesson>> {
        Ok(self.current_state.find_applicable_lessons(context))
    }

    /// Get lessons by tag
    pub async fn get_lessons_by_tag(&self, tag: &str) -> EvolutionResult<Vec<Lesson>> {
        Ok(self.current_state.get_lessons_by_tag(tag))
    }

    /// Get lessons by multiple tags
    pub async fn get_lessons_by_tags(&self, tags: &[&str]) -> EvolutionResult<Vec<Lesson>> {
        Ok(self.current_state.get_lessons_by_tags(tags))
    }

    /// Save a versioned snapshot
    pub async fn save_version(&self, timestamp: DateTime<Utc>) -> EvolutionResult<()> {
        let versioned_lessons = VersionedLessons {
            agent_id: self.agent_id.clone(),
            timestamp,
            state: self.current_state.clone(),
        };

        versioned_lessons.save().await?;
        log::debug!(
            "Saved lessons version for agent {} at {}",
            self.agent_id,
            timestamp
        );

        Ok(())
    }

    /// Load lessons state at a specific time
    pub async fn load_version(&self, timestamp: DateTime<Utc>) -> EvolutionResult<LessonsState> {
        let mut versioned_lessons = VersionedLessons::new(self.get_version_key(timestamp));
        let loaded = versioned_lessons.load().await?;
        Ok(loaded.state)
    }

    /// Get the storage key for a specific version
    pub fn get_version_key(&self, timestamp: DateTime<Utc>) -> String {
        format!(
            "agent_{}/lessons/v_{}",
            self.agent_id,
            timestamp.timestamp()
        )
    }

    /// Save the current state
    async fn save_current_state(&self) -> EvolutionResult<()> {
        let current_lessons = CurrentLessonsState {
            agent_id: self.agent_id.clone(),
            state: self.current_state.clone(),
        };

        current_lessons.save().await?;
        Ok(())
    }
}

/// Current lessons state of an agent
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LessonsState {
    pub technical_lessons: Vec<Lesson>,
    pub process_lessons: Vec<Lesson>,
    pub domain_lessons: Vec<Lesson>,
    pub failure_lessons: Vec<Lesson>,
    pub success_patterns: Vec<Lesson>,
    pub lesson_index: HashMap<String, Vec<LessonId>>, // Tag -> Lesson IDs
    pub metadata: LessonsMetadata,
}

impl LessonsState {
    /// Add a new lesson
    pub fn add_lesson(&mut self, lesson: Lesson) {
        // Categorize the lesson
        match lesson.category {
            LessonCategory::Technical => self.technical_lessons.push(lesson.clone()),
            LessonCategory::Process => self.process_lessons.push(lesson.clone()),
            LessonCategory::Domain => self.domain_lessons.push(lesson.clone()),
            LessonCategory::Failure => self.failure_lessons.push(lesson.clone()),
            LessonCategory::SuccessPattern => self.success_patterns.push(lesson.clone()),
        }

        // Update index
        for tag in &lesson.tags {
            self.lesson_index
                .entry(tag.clone())
                .or_default()
                .push(lesson.id.clone());
        }

        self.metadata.last_updated = Utc::now();
        self.metadata.total_lessons += 1;
    }

    /// Apply a lesson and track its usage
    pub fn apply_lesson(
        &mut self,
        lesson_id: &LessonId,
        context: &str,
    ) -> EvolutionResult<ApplicationResult> {
        if let Some(lesson) = self.find_lesson_mut(lesson_id) {
            lesson.applied_count += 1;
            lesson.last_applied = Some(Utc::now());
            lesson.contexts.push(context.to_string());

            // Keep contexts bounded
            if lesson.contexts.len() > 10 {
                lesson.contexts.remove(0);
            }

            let previous_applications = lesson.applied_count - 1;
            let success_rate = lesson.success_rate;

            self.metadata.last_updated = Utc::now();
            self.metadata.total_applications += 1;

            Ok(ApplicationResult {
                lesson_id: lesson_id.clone(),
                applied_at: Utc::now(),
                context: context.to_string(),
                previous_applications,
                success_rate,
            })
        } else {
            Err(EvolutionError::LessonNotFound(lesson_id.clone()))
        }
    }

    /// Validate a lesson with evidence
    pub fn validate_lesson(
        &mut self,
        lesson_id: &LessonId,
        evidence: Evidence,
    ) -> EvolutionResult<()> {
        if let Some(lesson) = self.find_lesson_mut(lesson_id) {
            lesson.evidence.push(evidence.clone());
            lesson.validated = true;
            lesson.last_validated = Some(Utc::now());

            // Update success rate based on evidence
            lesson.update_success_rate(&evidence);

            self.metadata.last_updated = Utc::now();
            self.metadata.validated_lessons += 1;

            Ok(())
        } else {
            Err(EvolutionError::LessonNotFound(lesson_id.clone()))
        }
    }

    /// Find applicable lessons for a context
    pub fn find_applicable_lessons(&self, context: &str) -> Vec<Lesson> {
        let mut applicable = Vec::new();
        let context_lower = context.to_lowercase();

        // Search in all categories
        let all_lessons = self.get_all_lessons();

        for lesson in all_lessons {
            // Check if context matches lesson context or tags
            let context_match = lesson.context.to_lowercase().contains(&context_lower)
                || lesson.insight.to_lowercase().contains(&context_lower);

            let tag_match = lesson.tags.iter().any(|tag| {
                context_lower.contains(&tag.to_lowercase())
                    || tag.to_lowercase().contains(&context_lower)
            });

            if context_match || tag_match {
                applicable.push(lesson.clone());
            }
        }

        // Sort by relevance (success rate and application count)
        applicable.sort_by(|a, b| {
            let a_score = a.success_rate * (1.0 + (a.applied_count as f64 / 10.0));
            let b_score = b.success_rate * (1.0 + (b.applied_count as f64 / 10.0));
            b_score
                .partial_cmp(&a_score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        applicable
    }

    /// Get lessons by tag
    pub fn get_lessons_by_tag(&self, tag: &str) -> Vec<Lesson> {
        if let Some(lesson_ids) = self.lesson_index.get(tag) {
            lesson_ids
                .iter()
                .filter_map(|id| self.find_lesson(id))
                .cloned()
                .collect()
        } else {
            Vec::new()
        }
    }

    /// Get lessons by multiple tags
    pub fn get_lessons_by_tags(&self, tags: &[&str]) -> Vec<Lesson> {
        let mut lessons = Vec::new();

        for tag in tags {
            lessons.extend(self.get_lessons_by_tag(tag));
        }

        // Remove duplicates
        lessons.dedup_by(|a, b| a.id == b.id);
        lessons
    }

    /// Get total number of lessons
    pub fn total_lessons(&self) -> usize {
        self.technical_lessons.len()
            + self.process_lessons.len()
            + self.domain_lessons.len()
            + self.failure_lessons.len()
            + self.success_patterns.len()
    }

    /// Get all lessons
    fn get_all_lessons(&self) -> Vec<&Lesson> {
        let mut all_lessons = Vec::new();
        all_lessons.extend(&self.technical_lessons);
        all_lessons.extend(&self.process_lessons);
        all_lessons.extend(&self.domain_lessons);
        all_lessons.extend(&self.failure_lessons);
        all_lessons.extend(&self.success_patterns);
        all_lessons
    }

    /// Find a lesson by ID
    fn find_lesson(&self, lesson_id: &LessonId) -> Option<&Lesson> {
        self.get_all_lessons()
            .into_iter()
            .find(|l| l.id == *lesson_id)
    }

    /// Find a mutable lesson by ID
    fn find_lesson_mut(&mut self, lesson_id: &LessonId) -> Option<&mut Lesson> {
        if let Some(lesson) = self
            .technical_lessons
            .iter_mut()
            .find(|l| l.id == *lesson_id)
        {
            return Some(lesson);
        }
        if let Some(lesson) = self.process_lessons.iter_mut().find(|l| l.id == *lesson_id) {
            return Some(lesson);
        }
        if let Some(lesson) = self.domain_lessons.iter_mut().find(|l| l.id == *lesson_id) {
            return Some(lesson);
        }
        if let Some(lesson) = self.failure_lessons.iter_mut().find(|l| l.id == *lesson_id) {
            return Some(lesson);
        }
        if let Some(lesson) = self
            .success_patterns
            .iter_mut()
            .find(|l| l.id == *lesson_id)
        {
            return Some(lesson);
        }
        None
    }

    /// Calculate success rate of all lessons
    pub fn calculate_success_rate(&self) -> f64 {
        let all_lessons = self.get_all_lessons();
        if all_lessons.is_empty() {
            0.0
        } else {
            let total_success_rate: f64 =
                all_lessons.iter().map(|lesson| lesson.success_rate).sum();
            total_success_rate / all_lessons.len() as f64
        }
    }

    /// Calculate knowledge coverage (percentage of different categories covered)
    pub fn calculate_knowledge_coverage(&self) -> f64 {
        let mut covered_categories = 0;
        let total_categories = 5; // Technical, Process, Domain, Failure, SuccessPattern

        if !self.technical_lessons.is_empty() {
            covered_categories += 1;
        }
        if !self.process_lessons.is_empty() {
            covered_categories += 1;
        }
        if !self.domain_lessons.is_empty() {
            covered_categories += 1;
        }
        if !self.failure_lessons.is_empty() {
            covered_categories += 1;
        }
        if !self.success_patterns.is_empty() {
            covered_categories += 1;
        }

        (covered_categories as f64 / total_categories as f64) * 100.0
    }

    /// Get lessons by category using existing categorized vectors
    pub fn get_lessons_by_category(&self, category: &str) -> Vec<&Lesson> {
        match category.to_lowercase().as_str() {
            "technical" => self.technical_lessons.iter().collect(),
            "process" => self.process_lessons.iter().collect(),
            "domain" => self.domain_lessons.iter().collect(),
            "failure" => self.failure_lessons.iter().collect(),
            "success" | "successpattern" => self.success_patterns.iter().collect(),
            _ => Vec::new(),
        }
    }
}

/// Individual lesson learned
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Lesson {
    pub id: LessonId,
    pub title: String,
    pub context: String,
    pub insight: String,
    pub category: LessonCategory,
    pub evidence: Vec<Evidence>,
    pub impact: ImpactLevel,
    pub confidence: f64,
    pub learned_at: DateTime<Utc>,
    pub last_applied: Option<DateTime<Utc>>,
    pub last_validated: Option<DateTime<Utc>>,
    pub validated: bool,
    pub applied_count: u32,
    pub success_rate: f64,
    pub related_tasks: Vec<TaskId>,
    pub related_memories: Vec<MemoryId>,
    pub knowledge_graph_refs: Vec<String>,
    pub tags: Vec<String>,
    pub contexts: Vec<String>, // Contexts where this lesson was applied
    pub metadata: HashMap<String, serde_json::Value>,
}

impl Lesson {
    /// Create a new lesson
    pub fn new(title: String, context: String, insight: String, category: LessonCategory) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            title,
            context,
            insight,
            category,
            evidence: Vec::new(),
            impact: ImpactLevel::Medium,
            confidence: 0.7,
            learned_at: Utc::now(),
            last_applied: None,
            last_validated: None,
            validated: false,
            applied_count: 0,
            success_rate: 0.5,
            related_tasks: Vec::new(),
            related_memories: Vec::new(),
            knowledge_graph_refs: Vec::new(),
            tags: Vec::new(),
            contexts: Vec::new(),
            metadata: HashMap::new(),
        }
    }

    /// Update success rate based on evidence
    pub fn update_success_rate(&mut self, evidence: &Evidence) {
        let weight = 0.2; // How much new evidence affects the rate
        let evidence_success = if evidence.outcome == EvidenceOutcome::Success {
            1.0
        } else {
            0.0
        };

        self.success_rate = (1.0 - weight) * self.success_rate + weight * evidence_success;

        // Update confidence based on evidence count
        let evidence_factor = (self.evidence.len() as f64 / 10.0).min(1.0);
        self.confidence = 0.5 + 0.4 * evidence_factor + 0.1 * self.success_rate;
    }

    /// Check if lesson is relevant for a context
    pub fn is_relevant_for(&self, context: &str) -> bool {
        let context_lower = context.to_lowercase();

        self.context.to_lowercase().contains(&context_lower)
            || self.insight.to_lowercase().contains(&context_lower)
            || self.tags.iter().any(|tag| {
                context_lower.contains(&tag.to_lowercase())
                    || tag.to_lowercase().contains(&context_lower)
            })
    }
}

/// Lesson categories
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LessonCategory {
    Technical,      // Code/implementation insights
    Process,        // Workflow improvements
    Domain,         // Subject matter insights
    Failure,        // What went wrong and why
    SuccessPattern, // What worked well
}

/// Impact levels for lessons
#[derive(Debug, Clone, PartialEq, PartialOrd, Serialize, Deserialize)]
pub enum ImpactLevel {
    Low,
    Medium,
    High,
    Critical,
}

/// Evidence supporting a lesson
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Evidence {
    pub description: String,
    pub source: EvidenceSource,
    pub outcome: EvidenceOutcome,
    pub confidence: f64,
    pub timestamp: DateTime<Utc>,
    pub metadata: HashMap<String, String>,
}

/// Sources of evidence
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EvidenceSource {
    TaskExecution,
    UserFeedback,
    PerformanceMetric,
    ExternalValidation,
    SelfReflection,
}

/// Evidence outcomes
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum EvidenceOutcome {
    Success,
    Failure,
    Mixed,
    Inconclusive,
}

/// Result of applying a lesson
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApplicationResult {
    pub lesson_id: LessonId,
    pub applied_at: DateTime<Utc>,
    pub context: String,
    pub previous_applications: u32,
    pub success_rate: f64,
}

/// Lessons metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LessonsMetadata {
    pub created_at: DateTime<Utc>,
    pub last_updated: DateTime<Utc>,
    pub total_lessons: u32,
    pub validated_lessons: u32,
    pub total_applications: u32,
    pub average_success_rate: f64,
}

impl Default for LessonsMetadata {
    fn default() -> Self {
        let now = Utc::now();
        Self {
            created_at: now,
            last_updated: now,
            total_lessons: 0,
            validated_lessons: 0,
            total_applications: 0,
            average_success_rate: 0.0,
        }
    }
}

/// Versioned lessons for persistence
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionedLessons {
    pub agent_id: AgentId,
    pub timestamp: DateTime<Utc>,
    pub state: LessonsState,
}

#[async_trait]
impl Persistable for VersionedLessons {
    fn new(_key: String) -> Self {
        Self {
            agent_id: String::new(),
            timestamp: Utc::now(),
            state: LessonsState::default(),
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
            "agent_{}/lessons/v_{}",
            self.agent_id,
            self.timestamp.timestamp()
        )
    }
}

/// Current lessons state for persistence
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CurrentLessonsState {
    pub agent_id: AgentId,
    pub state: LessonsState,
}

#[async_trait]
impl Persistable for CurrentLessonsState {
    fn new(key: String) -> Self {
        Self {
            agent_id: key,
            state: LessonsState::default(),
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
        format!("agent_{}/lessons/current", self.agent_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_lessons_evolution_creation() {
        let agent_id = "test_agent".to_string();
        let lessons = LessonsEvolution::new(agent_id.clone());

        assert_eq!(lessons.agent_id, agent_id);
        assert_eq!(lessons.current_state.total_lessons(), 0);
    }

    #[tokio::test]
    async fn test_add_lesson() {
        let mut lessons = LessonsEvolution::new("test_agent".to_string());

        let lesson = Lesson::new(
            "Test lesson".to_string(),
            "Testing context".to_string(),
            "Testing is important".to_string(),
            LessonCategory::Technical,
        );

        lessons.add_lesson(lesson).await.unwrap();
        assert_eq!(lessons.current_state.technical_lessons.len(), 1);
        assert_eq!(lessons.current_state.total_lessons(), 1);
    }

    #[tokio::test]
    async fn test_lesson_application() {
        let mut lessons_state = LessonsState::default();

        let mut lesson = Lesson::new(
            "Test lesson".to_string(),
            "Testing context".to_string(),
            "Testing is important".to_string(),
            LessonCategory::Technical,
        );
        lesson.tags.push("testing".to_string());

        let lesson_id = lesson.id.clone();
        lessons_state.add_lesson(lesson);

        let result = lessons_state
            .apply_lesson(&lesson_id, "Unit testing")
            .unwrap();
        assert_eq!(result.previous_applications, 0);

        let lesson = lessons_state.find_lesson(&lesson_id).unwrap();
        assert_eq!(lesson.applied_count, 1);
    }

    #[tokio::test]
    async fn test_find_applicable_lessons() {
        let mut lessons_state = LessonsState::default();

        let mut lesson1 = Lesson::new(
            "Testing lesson".to_string(),
            "Unit testing context".to_string(),
            "Unit tests prevent bugs".to_string(),
            LessonCategory::Technical,
        );
        lesson1.tags.push("testing".to_string());
        lesson1.tags.push("quality".to_string());

        let mut lesson2 = Lesson::new(
            "Performance lesson".to_string(),
            "Optimization context".to_string(),
            "Profile before optimizing".to_string(),
            LessonCategory::Technical,
        );
        lesson2.tags.push("performance".to_string());

        lessons_state.add_lesson(lesson1);
        lessons_state.add_lesson(lesson2);

        let applicable = lessons_state.find_applicable_lessons("testing code quality");
        assert_eq!(applicable.len(), 1);
        assert!(applicable[0].title.contains("Testing"));

        let performance_lessons = lessons_state.find_applicable_lessons("performance optimization");
        assert_eq!(performance_lessons.len(), 1);
        assert!(performance_lessons[0].title.contains("Performance"));
    }

    #[tokio::test]
    async fn test_lesson_validation() {
        let mut lessons_state = LessonsState::default();

        let lesson = Lesson::new(
            "Test lesson".to_string(),
            "Testing context".to_string(),
            "Testing is important".to_string(),
            LessonCategory::Technical,
        );
        let lesson_id = lesson.id.clone();

        lessons_state.add_lesson(lesson);

        let evidence = Evidence {
            description: "Unit tests caught 5 bugs".to_string(),
            source: EvidenceSource::TaskExecution,
            outcome: EvidenceOutcome::Success,
            confidence: 0.9,
            timestamp: Utc::now(),
            metadata: HashMap::new(),
        };

        lessons_state.validate_lesson(&lesson_id, evidence).unwrap();

        let validated_lesson = lessons_state.find_lesson(&lesson_id).unwrap();
        assert!(validated_lesson.validated);
        assert_eq!(validated_lesson.evidence.len(), 1);
        assert!(validated_lesson.success_rate > 0.5); // Should improve with successful evidence
    }
}
