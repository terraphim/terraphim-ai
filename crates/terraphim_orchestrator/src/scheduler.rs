use std::str::FromStr;

use cron::Schedule;
use tokio::sync::mpsc;
use tracing::{debug, error, info, warn};

use crate::config::{AgentDefinition, AgentLayer};
use crate::dispatcher::{DispatchQueue, DispatchTask};
use crate::error::OrchestratorError;

/// Schedule event indicating an agent should be spawned or stopped.
#[derive(Debug, Clone)]
pub enum ScheduleEvent {
    /// Time to spawn this agent.
    Spawn(AgentDefinition),
    /// Time to stop this agent.
    Stop { agent_name: String },
    /// Time to run compound review.
    CompoundReview,
}

/// A parsed schedule entry for one agent.
#[derive(Debug)]
struct ScheduleEntry {
    agent: AgentDefinition,
    schedule: Option<Schedule>,
}

/// Cron-based scheduler for agent lifecycle events.
#[derive(Debug)]
pub struct TimeScheduler {
    schedules: Vec<ScheduleEntry>,
    compound_schedule: Option<Schedule>,
    event_tx: mpsc::Sender<ScheduleEvent>,
    event_rx: mpsc::Receiver<ScheduleEvent>,
}

impl TimeScheduler {
    /// Create a new scheduler from agent definitions.
    ///
    /// Safety-layer agents have no schedule (always on).
    /// Core-layer agents must have a cron schedule.
    /// Growth-layer agents have no schedule (on-demand).
    pub fn new(
        agents: &[AgentDefinition],
        compound_schedule: Option<&str>,
    ) -> Result<Self, OrchestratorError> {
        let mut schedules = Vec::new();

        for agent in agents {
            let parsed = match &agent.schedule {
                Some(cron_expr) => {
                    let schedule = parse_cron(cron_expr)?;
                    Some(schedule)
                }
                None => None,
            };
            schedules.push(ScheduleEntry {
                agent: agent.clone(),
                schedule: parsed,
            });
        }

        let compound = match compound_schedule {
            Some(expr) => Some(parse_cron(expr)?),
            None => None,
        };

        let (event_tx, event_rx) = mpsc::channel(64);

        Ok(Self {
            schedules,
            compound_schedule: compound,
            event_tx,
            event_rx,
        })
    }

    /// Get the next scheduled event (async, used in select!).
    pub async fn next_event(&mut self) -> ScheduleEvent {
        self.event_rx
            .recv()
            .await
            .expect("scheduler event channel should never close while scheduler exists")
    }

    /// Get the sender for external event injection (used by the orchestrator
    /// background task).
    pub fn event_sender(&self) -> mpsc::Sender<ScheduleEvent> {
        self.event_tx.clone()
    }

    /// Get agents that should be running immediately (Safety layer).
    pub fn immediate_agents(&self) -> Vec<AgentDefinition> {
        self.schedules
            .iter()
            .filter(|e| e.agent.layer == AgentLayer::Safety)
            .map(|e| e.agent.clone())
            .collect()
    }

    /// Get all scheduled (Core layer) entries with their parsed cron schedules.
    pub fn scheduled_agents(&self) -> Vec<(&AgentDefinition, &Schedule)> {
        self.schedules
            .iter()
            .filter_map(|e| e.schedule.as_ref().map(|s| (&e.agent, s)))
            .collect()
    }

    /// Get the compound review schedule if configured.
    pub fn compound_review_schedule(&self) -> Option<&Schedule> {
        self.compound_schedule.as_ref()
    }
}

/// TimeMode wraps the TimeScheduler and integrates with the dispatch queue.
/// Supports both legacy mode (direct spawn) and queue-based dispatch.
pub struct TimeMode {
    /// The underlying scheduler.
    scheduler: TimeScheduler,
    /// Optional dispatch queue for queue-based mode.
    /// If None, operates in legacy mode (direct spawn).
    dispatch_queue: Option<DispatchQueue>,
    /// Whether to use legacy mode (spawn directly instead of queueing).
    legacy_mode: bool,
}

impl TimeMode {
    /// Create a new TimeMode in legacy mode (spawns directly).
    pub fn new_legacy(
        agents: &[AgentDefinition],
        compound_schedule: Option<&str>,
    ) -> Result<Self, OrchestratorError> {
        let scheduler = TimeScheduler::new(agents, compound_schedule)?;
        Ok(Self {
            scheduler,
            dispatch_queue: None,
            legacy_mode: true,
        })
    }

    /// Create a new TimeMode with dispatch queue integration.
    pub fn new_with_queue(
        agents: &[AgentDefinition],
        compound_schedule: Option<&str>,
        dispatch_queue: DispatchQueue,
    ) -> Result<Self, OrchestratorError> {
        let scheduler = TimeScheduler::new(agents, compound_schedule)?;
        Ok(Self {
            scheduler,
            dispatch_queue: Some(dispatch_queue),
            legacy_mode: false,
        })
    }

    /// Check if running in legacy mode.
    pub fn is_legacy(&self) -> bool {
        self.legacy_mode
    }

    /// Process the next scheduled event.
    /// In legacy mode, returns the event for direct handling.
    /// In queue mode, submits TimeTask to dispatch queue and returns None.
    pub async fn process_next_event(&mut self) -> Option<ScheduleEvent> {
        let event = self.scheduler.next_event().await;

        if self.legacy_mode {
            // Legacy mode: return event for direct handling
            Some(event)
        } else {
            // Queue mode: convert to TimeTask and submit
            self.submit_to_queue(event).await;
            None
        }
    }

    /// Submit a schedule event to the dispatch queue as a TimeTask.
    async fn submit_to_queue(&mut self, event: ScheduleEvent) {
        let Some(ref mut queue) = self.dispatch_queue else {
            error!("No dispatch queue configured but not in legacy mode");
            return;
        };

        match event {
            ScheduleEvent::Spawn(agent) => {
                if let Some(ref schedule) = agent.schedule {
                    let task = DispatchTask::TimeTask(agent.name.clone(), schedule.clone());
                    match queue.submit(task) {
                        Ok(()) => {
                            info!("Submitted TimeTask for agent '{}' to dispatch queue", agent.name);
                        }
                        Err(e) => {
                            warn!("Failed to submit TimeTask for agent '{}': {}", agent.name, e);
                        }
                    }
                } else {
                    // Safety agents with no schedule should still be spawned immediately
                    debug!("Safety agent '{}' has no schedule, skipping queue", agent.name);
                }
            }
            ScheduleEvent::Stop { agent_name } => {
                debug!("Stop event for agent '{}' - not implemented in queue mode", agent_name);
            }
            ScheduleEvent::CompoundReview => {
                debug!("CompoundReview event - handled separately");
            }
        }
    }

    /// Get the event sender for external event injection.
    pub fn event_sender(&self) -> mpsc::Sender<ScheduleEvent> {
        self.scheduler.event_sender()
    }

    /// Get agents that should be running immediately (Safety layer).
    /// In legacy mode, these should be spawned directly.
    /// In queue mode, these are handled by the orchestrator.
    pub fn immediate_agents(&self) -> Vec<AgentDefinition> {
        self.scheduler.immediate_agents()
    }

    /// Get all scheduled (Core layer) entries with their parsed cron schedules.
    pub fn scheduled_agents(&self) -> Vec<(&AgentDefinition, &Schedule)> {
        self.scheduler.scheduled_agents()
    }

    /// Get the compound review schedule if configured.
    pub fn compound_review_schedule(&self) -> Option<&Schedule> {
        self.scheduler.compound_review_schedule()
    }

    /// Get a reference to the dispatch queue (if in queue mode).
    pub fn dispatch_queue(&self) -> Option<&DispatchQueue> {
        self.dispatch_queue.as_ref()
    }

    /// Get a mutable reference to the dispatch queue (if in queue mode).
    pub fn dispatch_queue_mut(&mut self) -> Option<&mut DispatchQueue> {
        self.dispatch_queue.as_mut()
    }
}

/// Parse a cron expression, prepending seconds field if needed.
fn parse_cron(expr: &str) -> Result<Schedule, OrchestratorError> {
    // The `cron` crate expects 7 fields (sec min hour dom month dow year)
    // Standard cron has 5 fields (min hour dom month dow).
    // Prepend "0" for seconds if the expression has 5 fields.
    let parts: Vec<&str> = expr.split_whitespace().collect();
    let full_expr = if parts.len() == 5 {
        format!("0 {}", expr)
    } else {
        expr.to_string()
    };

    Schedule::from_str(&full_expr)
        .map_err(|e| OrchestratorError::SchedulerError(format!("invalid cron '{}': {}", expr, e)))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_agent(name: &str, layer: AgentLayer, schedule: Option<&str>) -> AgentDefinition {
        AgentDefinition {
            name: name.to_string(),
            layer,
            cli_tool: "codex".to_string(),
            task: "test task".to_string(),
            model: None,
            schedule: schedule.map(String::from),
            capabilities: vec![],
            max_memory_bytes: None,
            provider: None,
            fallback_provider: None,
            fallback_model: None,
            provider_tier: None,
            persona_name: None,
            persona_symbol: None,
            persona_vibe: None,
            meta_cortex_connections: vec![],
            skill_chain: vec![],
        }
    }

    #[test]
    fn test_schedule_parse_cron() {
        let agents = vec![make_agent("sync", AgentLayer::Core, Some("0 3 * * *"))];
        let scheduler = TimeScheduler::new(&agents, None).unwrap();
        let scheduled = scheduler.scheduled_agents();
        assert_eq!(scheduled.len(), 1);
        assert_eq!(scheduled[0].0.name, "sync");
    }

    #[test]
    fn test_schedule_invalid_cron() {
        let agents = vec![make_agent("bad", AgentLayer::Core, Some("not a cron"))];
        let result = TimeScheduler::new(&agents, None);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("invalid cron"));
    }

    #[test]
    fn test_schedule_safety_always() {
        let agents = vec![
            make_agent("sentinel", AgentLayer::Safety, None),
            make_agent("sync", AgentLayer::Core, Some("0 3 * * *")),
            make_agent("reviewer", AgentLayer::Growth, None),
        ];
        let scheduler = TimeScheduler::new(&agents, None).unwrap();

        let immediate = scheduler.immediate_agents();
        assert_eq!(immediate.len(), 1);
        assert_eq!(immediate[0].name, "sentinel");
        assert_eq!(immediate[0].layer, AgentLayer::Safety);
    }

    #[test]
    fn test_schedule_compound_review() {
        let agents = vec![make_agent("sentinel", AgentLayer::Safety, None)];
        let scheduler = TimeScheduler::new(&agents, Some("0 2 * * *")).unwrap();
        assert!(scheduler.compound_review_schedule().is_some());
    }

    #[test]
    fn test_schedule_no_compound_review() {
        let agents = vec![make_agent("sentinel", AgentLayer::Safety, None)];
        let scheduler = TimeScheduler::new(&agents, None).unwrap();
        assert!(scheduler.compound_review_schedule().is_none());
    }

    // TimeMode tests
    #[test]
    fn test_timemode_legacy_mode() {
        let agents = vec![
            make_agent("sentinel", AgentLayer::Safety, None),
            make_agent("sync", AgentLayer::Core, Some("0 3 * * *")),
        ];

        let time_mode = TimeMode::new_legacy(&agents, None).unwrap();

        assert!(time_mode.is_legacy());
        assert!(time_mode.dispatch_queue().is_none());
        assert_eq!(time_mode.immediate_agents().len(), 1);
        assert_eq!(time_mode.immediate_agents()[0].name, "sentinel");
    }

    #[test]
    fn test_timemode_queue_mode() {
        let agents = vec![
            make_agent("sentinel", AgentLayer::Safety, None),
            make_agent("sync", AgentLayer::Core, Some("0 3 * * *")),
        ];

        let queue = DispatchQueue::new(10);
        let time_mode = TimeMode::new_with_queue(&agents, None, queue).unwrap();

        assert!(!time_mode.is_legacy());
        assert!(time_mode.dispatch_queue().is_some());
        assert_eq!(time_mode.immediate_agents().len(), 1);
    }

    #[test]
    fn test_timemode_scheduled_agents() {
        let agents = vec![
            make_agent("sentinel", AgentLayer::Safety, None),
            make_agent("sync", AgentLayer::Core, Some("0 3 * * *")),
            make_agent("backup", AgentLayer::Core, Some("0 4 * * *")),
        ];

        let time_mode = TimeMode::new_legacy(&agents, None).unwrap();
        let scheduled = time_mode.scheduled_agents();

        assert_eq!(scheduled.len(), 2);
        assert!(scheduled.iter().any(|(a, _)| a.name == "sync"));
        assert!(scheduled.iter().any(|(a, _)| a.name == "backup"));
    }

    #[test]
    fn test_timemode_compound_review() {
        let agents = vec![make_agent("sentinel", AgentLayer::Safety, None)];
        let time_mode = TimeMode::new_legacy(&agents, Some("0 2 * * *")).unwrap();

        assert!(time_mode.compound_review_schedule().is_some());
    }

    #[test]
    fn test_timemode_dispatch_queue_access() {
        let agents = vec![make_agent("sync", AgentLayer::Core, Some("0 3 * * *"))];
        let queue = DispatchQueue::new(10);

        let mut time_mode = TimeMode::new_with_queue(&agents, None, queue).unwrap();

        // Test mutable access
        {
            let q = time_mode.dispatch_queue_mut().unwrap();
            assert_eq!(q.len(), 0);
        }

        // Test immutable access
        let q = time_mode.dispatch_queue().unwrap();
        assert_eq!(q.len(), 0);
    }

    #[tokio::test]
    async fn test_timemode_process_event_legacy() {
        let agents = vec![make_agent("sync", AgentLayer::Core, Some("0 3 * * *"))];

        let mut time_mode = TimeMode::new_legacy(&agents, None).unwrap();

        // Get the event sender and inject a spawn event
        let sender = time_mode.event_sender();

        // Spawn a task to send the event
        let agent = agents[0].clone();
        tokio::spawn(async move {
            tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
            sender.send(ScheduleEvent::Spawn(agent)).await.unwrap();
        });

        // Process the event (should return Some in legacy mode)
        let event = tokio::time::timeout(
            tokio::time::Duration::from_millis(100),
            time_mode.process_next_event(),
        )
        .await
        .unwrap();

        assert!(event.is_some());
        match event.unwrap() {
            ScheduleEvent::Spawn(a) => assert_eq!(a.name, "sync"),
            _ => panic!("Expected Spawn event"),
        }
    }

    #[tokio::test]
    async fn test_timemode_process_event_queue() {
        let agents = vec![make_agent("sync", AgentLayer::Core, Some("0 3 * * *"))];

        let queue = DispatchQueue::new(10);
        let mut time_mode = TimeMode::new_with_queue(&agents, None, queue).unwrap();

        // Get the event sender and inject a spawn event
        let sender = time_mode.event_sender();

        // Spawn a task to send the event
        let agent = agents[0].clone();
        tokio::spawn(async move {
            tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
            sender.send(ScheduleEvent::Spawn(agent)).await.unwrap();
        });

        // Process the event (should return None in queue mode, task is queued)
        let event = tokio::time::timeout(
            tokio::time::Duration::from_millis(100),
            time_mode.process_next_event(),
        )
        .await
        .unwrap();

        assert!(event.is_none());

        // Verify the task was queued
        let q = time_mode.dispatch_queue().unwrap();
        assert_eq!(q.len(), 1);
    }

    #[test]
    fn test_timemode_backward_compatibility() {
        // This test verifies that legacy mode still works as before
        let agents = vec![
            make_agent("sentinel", AgentLayer::Safety, None),
            make_agent("sync", AgentLayer::Core, Some("0 3 * * *")),
        ];

        // Creating TimeMode in legacy mode should behave like the old TimeScheduler
        let time_mode = TimeMode::new_legacy(&agents, None).unwrap();

        // All the old TimeScheduler methods should work through TimeMode
        assert_eq!(time_mode.immediate_agents().len(), 1);
        assert_eq!(time_mode.scheduled_agents().len(), 1);
        assert!(time_mode.compound_review_schedule().is_none());

        // Should have event sender available
        let _sender = time_mode.event_sender();
    }
}
