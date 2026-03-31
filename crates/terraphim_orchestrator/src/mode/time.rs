//! Time-driven mode controller.
//!
//! Manages cron-scheduled agents using the unified dispatcher.

use crate::{
    AgentDefinition, ConcurrencyController, DispatchTask, Dispatcher, ScheduleEvent, TimeScheduler,
};
use tracing::{error, info, warn};

/// Time-driven mode controller.
pub struct TimeMode {
    /// Underlying scheduler.
    scheduler: TimeScheduler,
    /// Dispatcher queue.
    dispatcher: Dispatcher,
    /// Concurrency controller.
    concurrency: ConcurrencyController,
    /// Currently running agent names.
    running: std::collections::HashSet<String>,
}

impl TimeMode {
    /// Create a new time mode controller.
    pub fn new(scheduler: TimeScheduler, concurrency: ConcurrencyController) -> Self {
        Self {
            scheduler,
            dispatcher: Dispatcher::new(),
            concurrency,
            running: std::collections::HashSet::new(),
        }
    }

    /// Get immediate agents (Safety layer) that should start now.
    pub fn immediate_agents(&self) -> Vec<AgentDefinition> {
        self.scheduler.immediate_agents()
    }

    /// Start a Safety agent immediately.
    pub async fn start_safety_agent(&mut self, agent: AgentDefinition) -> Result<(), String> {
        // Safety agents bypass concurrency limits (they're always on)
        let task = DispatchTask::TimeDriven {
            name: agent.name.clone(),
            task: agent.task.clone(),
            layer: agent.layer,
        };

        self.dispatcher.enqueue(task);
        let name = agent.name.clone();
        self.running.insert(name.clone());

        info!(
            agent_name = %name,
            layer = ?agent.layer,
            "started Safety agent"
        );

        Ok(())
    }

    /// Run the time mode event loop.
    pub async fn run(mut self, mut shutdown: tokio::sync::watch::Receiver<bool>) {
        info!("starting time-driven mode");

        // Start Safety agents immediately
        for agent in self.immediate_agents() {
            if let Err(e) = self.start_safety_agent(agent).await {
                error!("failed to start Safety agent: {}", e);
            }
        }

        loop {
            tokio::select! {
                event = self.scheduler.next_event() => {
                    match event {
                        ScheduleEvent::Spawn(agent) => {
                            if let Err(e) = self.handle_spawn(*agent).await {
                                error!("failed to spawn agent: {}", e);
                            }
                        }
                        ScheduleEvent::Stop { agent_name } => {
                            self.handle_stop(agent_name).await;
                        }
                        ScheduleEvent::CompoundReview => {
                            // Compound review is handled by orchestrator
                            info!("compound review scheduled");
                        }
                        ScheduleEvent::Flow(flow) => {
                            info!(flow_name = %flow.name, "flow scheduled");
                        }
                    }
                }
                _ = shutdown.changed() => {
                    if *shutdown.borrow() {
                        info!("shutting down time-driven mode");
                        break;
                    }
                }
            }
        }
    }

    /// Handle a spawn event.
    async fn handle_spawn(&mut self, agent: AgentDefinition) -> Result<(), String> {
        // Try to acquire a time-driven slot
        match self.concurrency.acquire_time_driven().await {
            Some(permit) => {
                let task = DispatchTask::TimeDriven {
                    name: agent.name.clone(),
                    task: agent.task.clone(),
                    layer: agent.layer,
                };

                self.dispatcher.enqueue(task);
                self.running.insert(agent.name.clone());

                info!(
                    agent_name = %agent.name,
                    layer = ?agent.layer,
                    "dispatched time-driven agent"
                );

                // Keep permit until agent completes (simplified)
                drop(permit);

                Ok(())
            }
            None => {
                warn!(
                    agent_name = %agent.name,
                    "no concurrency slot available for agent"
                );
                Err("concurrency limit reached".into())
            }
        }
    }

    /// Handle a stop event.
    async fn handle_stop(&mut self, agent_name: String) {
        info!(agent_name = %agent_name, "stopping agent");
        self.running.remove(&agent_name);
    }

    /// Get dispatcher statistics.
    pub fn dispatcher_stats(&self) -> &crate::dispatcher::DispatcherStats {
        self.dispatcher.stats()
    }

    /// Get the scheduler event sender.
    pub fn event_sender(&self) -> tokio::sync::mpsc::Sender<ScheduleEvent> {
        self.scheduler.event_sender()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{AgentLayer, ModeQuotas};

    fn test_agent(name: &str, layer: AgentLayer) -> AgentDefinition {
        AgentDefinition {
            name: name.into(),
            layer,
            cli_tool: "echo".into(),
            task: "test".into(),
            model: None,
            schedule: None,
            capabilities: vec![],
            max_memory_bytes: None,
            budget_monthly_cents: None,
            provider: None,
            persona: None,
            terraphim_role: None,
            skill_chain: vec![],
            sfia_skills: vec![],
            fallback_provider: None,
            fallback_model: None,
            grace_period_secs: None,
            max_cpu_seconds: None,
            gitea_issue: None,
        }
    }

    #[tokio::test]
    async fn test_safety_agent_bypasses_concurrency() {
        let scheduler =
            TimeScheduler::new(&[test_agent("safety", AgentLayer::Safety)], None).unwrap();

        let concurrency = ConcurrencyController::new(
            10,
            ModeQuotas {
                time_max: 0, // No time slots available
                issue_max: 10,
            },
            crate::FairnessPolicy::RoundRobin,
        );

        let mut mode = TimeMode::new(scheduler, concurrency);

        // Safety agents should still start even with no slots
        let agent = test_agent("safety", AgentLayer::Safety);
        let result = mode.start_safety_agent(agent).await;
        assert!(result.is_ok());
    }
}
