#[cfg(feature = "evolution")]
use std::collections::HashMap;

use crate::config::EvolutionConfig;
#[cfg(feature = "evolution")]
use terraphim_agent_evolution::{
    AgentEvolutionSystem, AgentId, ImportanceLevel, LessonCategory, MemoryItem, MemoryItemType,
    TaskId,
};

#[derive(Debug)]
pub struct EvolutionManager {
    #[cfg(feature = "evolution")]
    systems: HashMap<AgentId, AgentEvolutionSystem>,
    #[cfg_attr(not(feature = "evolution"), allow(dead_code))]
    config: EvolutionConfig,
    enabled: bool,
}

#[derive(Debug)]
pub struct EvolutionOutput {
    pub agent_id: String,
    pub content: String,
    pub event_type: String,
    pub importance: String,
}

impl EvolutionManager {
    pub fn new(config: EvolutionConfig) -> Self {
        let enabled = config.enabled;
        Self {
            #[cfg(feature = "evolution")]
            systems: HashMap::new(),
            config,
            enabled,
        }
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    #[cfg(feature = "evolution")]
    pub fn ensure_agent(&mut self, agent_id: &str) {
        if !self.enabled {
            return;
        }
        self.systems
            .entry(agent_id.to_string())
            .or_insert_with(|| AgentEvolutionSystem::new(agent_id.to_string()));
    }

    #[cfg(not(feature = "evolution"))]
    pub fn ensure_agent(&mut self, _agent_id: &str) {}

    #[cfg(feature = "evolution")]
    pub fn record_output(&mut self, output: EvolutionOutput) -> Result<(), String> {
        if !self.enabled {
            return Ok(());
        }
        let system = self
            .systems
            .get_mut(&output.agent_id)
            .ok_or_else(|| format!("agent {} not initialised in evolution", output.agent_id))?;

        let item_type = match output.event_type.as_str() {
            "stdout" => MemoryItemType::ExecutionResult,
            "stderr" => MemoryItemType::ExecutionResult,
            "workflow" => MemoryItemType::WorkflowEvent,
            "lesson" => MemoryItemType::LessonLearned,
            _ => MemoryItemType::Experience,
        };
        let importance = match output.importance.as_str() {
            "critical" => ImportanceLevel::Critical,
            "high" => ImportanceLevel::High,
            "medium" => ImportanceLevel::Medium,
            _ => ImportanceLevel::Low,
        };
        let memory = MemoryItem {
            id: ulid::Ulid::new().to_string(),
            item_type,
            content: output.content,
            created_at: chrono::Utc::now(),
            last_accessed: None,
            access_count: 0,
            importance,
            tags: vec![],
            associations: std::collections::HashMap::new(),
        };

        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(system.memory.add_memory(memory))
        })
        .map_err(|e| format!("evolution record_output: {e}"))
    }

    #[cfg(not(feature = "evolution"))]
    pub fn record_output(&mut self, _output: EvolutionOutput) -> Result<(), String> {
        Ok(())
    }

    #[cfg(feature = "evolution")]
    pub fn record_task_start(&mut self, agent_id: &str, task_content: &str) -> Option<TaskId> {
        if !self.enabled {
            return None;
        }
        let system = self.systems.get_mut(agent_id)?;
        let task = terraphim_agent_evolution::AgentTask::new(task_content.to_string());
        let task_id = task.id.clone();
        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(system.tasks.add_task(task))
        })
        .ok();
        Some(task_id)
    }

    #[cfg(not(feature = "evolution"))]
    pub fn record_task_start(&mut self, _agent_id: &str, _task_content: &str) -> Option<String> {
        None
    }

    #[cfg(feature = "evolution")]
    pub fn record_task_complete(
        &mut self,
        agent_id: &str,
        task_id: &str,
        result: &str,
    ) -> Result<(), String> {
        if !self.enabled {
            return Ok(());
        }
        let system = self
            .systems
            .get_mut(agent_id)
            .ok_or_else(|| format!("agent {agent_id} not in evolution"))?;
        let task_id_owned: TaskId = task_id.to_string();
        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current()
                .block_on(system.tasks.complete_task(&task_id_owned, result))
        })
        .map_err(|e| format!("evolution record_task_complete: {e}"))
    }

    #[cfg(not(feature = "evolution"))]
    pub fn record_task_complete(
        &mut self,
        _agent_id: &str,
        _task_id: &str,
        _result: &str,
    ) -> Result<(), String> {
        Ok(())
    }

    #[cfg(feature = "evolution")]
    pub fn record_lesson(
        &mut self,
        agent_id: &str,
        title: &str,
        context: &str,
        insight: &str,
        category: LessonCategory,
    ) -> Result<(), String> {
        if !self.enabled {
            return Ok(());
        }
        let system = self
            .systems
            .get_mut(agent_id)
            .ok_or_else(|| format!("agent {agent_id} not in evolution"))?;
        let lesson = terraphim_agent_evolution::Lesson::new(
            title.to_string(),
            context.to_string(),
            insight.to_string(),
            category,
        );
        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(system.lessons.add_lesson(lesson))
        })
        .map_err(|e| format!("evolution record_lesson: {e}"))
    }

    #[cfg(not(feature = "evolution"))]
    pub fn record_lesson(
        &mut self,
        _agent_id: &str,
        _title: &str,
        _context: &str,
        _insight: &str,
        _category: &str,
    ) -> Result<(), String> {
        Ok(())
    }

    #[cfg(feature = "evolution")]
    pub fn snapshot_on_exit(&mut self, agent_id: &str) -> Option<String> {
        if !self.enabled {
            return None;
        }
        let system = self.systems.get_mut(agent_id)?;
        match tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(system.save_snapshot())
        }) {
            Ok(()) => {
                let now = chrono::Utc::now();
                Some(format!(
                    "agent_{}/evolution/index/{}",
                    agent_id,
                    now.timestamp_millis()
                ))
            }
            Err(e) => {
                tracing::warn!(agent = agent_id, error = %e, "evolution snapshot failed");
                None
            }
        }
    }

    #[cfg(not(feature = "evolution"))]
    pub fn snapshot_on_exit(&mut self, _agent_id: &str) -> Option<String> {
        None
    }

    #[cfg(feature = "evolution")]
    pub fn render_context(&self, agent_id: &str) -> String {
        if !self.enabled {
            return String::new();
        }
        let Some(system) = self.systems.get(agent_id) else {
            return String::new();
        };
        let state = &system.memory.current_state;
        let total = state.total_size();
        if total == 0 {
            return String::new();
        }

        let mut sections = Vec::new();
        sections.push(format!("## Agent Evolution Memory ({} items)\n", total));

        if !state.short_term.is_empty() {
            sections.push("### Recent Memories".to_string());
            for item in state.short_term.iter().take(5) {
                sections.push(format!(
                    "- [{:?}] {}",
                    item.item_type,
                    truncate_str(&item.content, 120)
                ));
            }
        }

        if !state.long_term.is_empty() {
            sections.push(format!(
                "\n### Long-term Memory ({} items)",
                state.long_term.len()
            ));
            for (_, item) in state.long_term.iter().take(3) {
                sections.push(format!(
                    "- [{:?}] {}",
                    item.item_type,
                    truncate_str(&item.content, 120)
                ));
            }
        }

        if !state.episodic_memory.is_empty() {
            sections.push(format!(
                "\n### Episodes ({} total)",
                state.episodic_memory.len()
            ));
            for ep in state.episodic_memory.iter().rev().take(3) {
                sections.push(format!("- {} ({:?})", ep.description, ep.outcome));
            }
        }

        let context = sections.join("\n");
        if context.len() > self.config.max_memory_tokens {
            context[..self.config.max_memory_tokens].to_string()
        } else {
            context
        }
    }

    #[cfg(not(feature = "evolution"))]
    pub fn render_context(&self, _agent_id: &str) -> String {
        String::new()
    }

    #[cfg(feature = "evolution")]
    pub fn consolidate_all(&mut self) -> usize {
        if !self.enabled {
            return 0;
        }
        let mut count = 0;
        for system in self.systems.values_mut() {
            if tokio::task::block_in_place(|| {
                tokio::runtime::Handle::current().block_on(system.memory.consolidate_memories())
            })
            .is_ok()
            {
                count += 1;
            }
        }
        count
    }

    #[cfg(not(feature = "evolution"))]
    pub fn consolidate_all(&mut self) -> usize {
        0
    }
}

#[cfg(feature = "evolution")]
fn truncate_str(s: &str, max: usize) -> &str {
    if s.len() <= max {
        s
    } else {
        match s.char_indices().take_while(|(i, _)| *i < max).last() {
            Some((i, c)) => {
                let end = i + c.len_utf8();
                &s[..end]
            }
            None => s,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_evolution_config_default_disabled() {
        let config = EvolutionConfig::default();
        assert!(!config.enabled);
        assert_eq!(config.max_memory_tokens, 1500);
        assert_eq!(config.max_snapshots_per_agent, 100);
        assert_eq!(config.consolidation_interval_ticks, 200);
    }

    #[test]
    fn test_manager_new_disabled() {
        let config = EvolutionConfig::default();
        let mgr = EvolutionManager::new(config);
        assert!(!mgr.is_enabled());
    }

    #[test]
    fn test_ensure_agent_noop_when_disabled() {
        let config = EvolutionConfig::default();
        let mut mgr = EvolutionManager::new(config);
        mgr.ensure_agent("test-agent");
        assert!(!mgr.is_enabled());
    }

    #[test]
    fn test_record_output_noop_when_disabled() {
        let config = EvolutionConfig::default();
        let mut mgr = EvolutionManager::new(config);
        let output = EvolutionOutput {
            agent_id: "test".to_string(),
            content: "hello".to_string(),
            event_type: "stdout".to_string(),
            importance: "medium".to_string(),
        };
        assert!(mgr.record_output(output).is_ok());
    }

    #[test]
    fn test_snapshot_noop_when_disabled() {
        let config = EvolutionConfig::default();
        let mut mgr = EvolutionManager::new(config);
        assert!(mgr.snapshot_on_exit("test").is_none());
    }

    #[test]
    fn test_render_context_empty_when_disabled() {
        let config = EvolutionConfig::default();
        let mgr = EvolutionManager::new(config);
        assert!(mgr.render_context("test").is_empty());
    }

    #[test]
    fn test_consolidate_noop_when_disabled() {
        let config = EvolutionConfig::default();
        let mut mgr = EvolutionManager::new(config);
        assert_eq!(mgr.consolidate_all(), 0);
    }

    #[test]
    #[cfg(feature = "evolution")]
    fn test_truncate_str_short() {
        assert_eq!(truncate_str("hello", 10), "hello");
    }

    #[test]
    #[cfg(feature = "evolution")]
    fn test_truncate_str_long() {
        let result = truncate_str("hello world this is a long string", 11);
        assert_eq!(result, "hello world");
    }

    #[test]
    fn test_task_start_noop_when_disabled() {
        let config = EvolutionConfig::default();
        let mut mgr = EvolutionManager::new(config);
        assert!(mgr.record_task_start("test", "do stuff").is_none());
    }

    #[test]
    fn test_task_complete_noop_when_disabled() {
        let config = EvolutionConfig::default();
        let mut mgr = EvolutionManager::new(config);
        assert!(mgr.record_task_complete("test", "t1", "ok").is_ok());
    }

    #[cfg(not(feature = "evolution"))]
    #[test]
    fn test_lesson_noop_when_disabled() {
        let config = EvolutionConfig::default();
        let mut mgr = EvolutionManager::new(config);
        assert!(mgr
            .record_lesson("test", "title", "ctx", "insight", "Technical")
            .is_ok());
    }

    #[cfg(feature = "evolution")]
    #[tokio::test(flavor = "multi_thread")]
    async fn test_lesson_with_feature() {
        let mut config = EvolutionConfig::default();
        config.enabled = true;
        let mut mgr = EvolutionManager::new(config);
        mgr.ensure_agent("test-agent");
        let result = mgr.record_lesson(
            "test-agent",
            "Test lesson",
            "Testing context",
            "Testing works",
            LessonCategory::Technical,
        );
        assert!(result.is_ok());
    }

    #[cfg(feature = "evolution")]
    #[tokio::test(flavor = "multi_thread")]
    async fn test_record_output_with_feature() {
        let mut config = EvolutionConfig::default();
        config.enabled = true;
        let mut mgr = EvolutionManager::new(config);
        mgr.ensure_agent("test-agent");
        let output = EvolutionOutput {
            agent_id: "test-agent".to_string(),
            content: "Test output content".to_string(),
            event_type: "stdout".to_string(),
            importance: "high".to_string(),
        };
        assert!(mgr.record_output(output).is_ok());
    }

    #[cfg(feature = "evolution")]
    #[tokio::test(flavor = "multi_thread")]
    async fn test_render_context_with_data() {
        let mut config = EvolutionConfig::default();
        config.enabled = true;
        let mut mgr = EvolutionManager::new(config);
        mgr.ensure_agent("test-agent");
        let output = EvolutionOutput {
            agent_id: "test-agent".to_string(),
            content: "Some important output".to_string(),
            event_type: "stdout".to_string(),
            importance: "high".to_string(),
        };
        mgr.record_output(output).unwrap();
        let ctx = mgr.render_context("test-agent");
        assert!(!ctx.is_empty());
        assert!(ctx.contains("Agent Evolution Memory"));
    }

    #[cfg(feature = "evolution")]
    #[tokio::test(flavor = "multi_thread")]
    async fn test_snapshot_creates_key() {
        let mut config = EvolutionConfig::default();
        config.enabled = true;
        let mut mgr = EvolutionManager::new(config);
        mgr.ensure_agent("test-agent");
        let key = mgr.snapshot_on_exit("test-agent");
        assert!(key.is_some());
        assert!(key
            .unwrap()
            .starts_with("agent_test-agent/evolution/index/"));
    }

    #[cfg(feature = "evolution")]
    #[tokio::test(flavor = "multi_thread")]
    async fn test_task_lifecycle_with_feature() {
        let mut config = EvolutionConfig::default();
        config.enabled = true;
        let mut mgr = EvolutionManager::new(config);
        mgr.ensure_agent("test-agent");
        let task_id = mgr.record_task_start("test-agent", "implement feature X");
        assert!(task_id.is_some());
        let result = mgr.record_task_complete("test-agent", &task_id.unwrap(), "feature done");
        assert!(result.is_ok());
    }
}
