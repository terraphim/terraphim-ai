use std::str::FromStr;

use cron::Schedule;
use tokio::sync::mpsc;

use crate::config::{AgentDefinition, AgentLayer};
use crate::error::OrchestratorError;

/// Schedule event indicating an agent should be spawned or stopped.
#[derive(Debug, Clone)]
pub enum ScheduleEvent {
    /// Time to spawn this agent.
    Spawn(Box<AgentDefinition>),
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

/// Parse a cron expression, normalising to 7-field format for the `cron` crate.
///
/// Accepts:
/// - 5 fields (standard cron): min hour dom month dow -> prepend sec, append year
/// - 6 fields: sec min hour dom month dow -> append year
/// - 7 fields: passed through as-is
fn parse_cron(expr: &str) -> Result<Schedule, OrchestratorError> {
    let parts: Vec<&str> = expr.split_whitespace().collect();
    let full_expr = match parts.len() {
        5 => format!("0 {} *", expr),
        6 => format!("{} *", expr),
        7 => expr.to_string(),
        _ => {
            return Err(OrchestratorError::SchedulerError(format!(
                "invalid cron '{}': expected 5, 6, or 7 fields, got {}",
                expr,
                parts.len()
            )));
        }
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

    #[test]
    fn test_parse_cron_weekly_day_of_week() {
        let agents = vec![
            make_agent("weekly-sun", AgentLayer::Core, Some("0 2 * * SUN")),
            make_agent("weekly-mon", AgentLayer::Core, Some("0 4 * * MON")),
        ];
        let scheduler = TimeScheduler::new(&agents, None).unwrap();
        let scheduled = scheduler.scheduled_agents();
        assert_eq!(scheduled.len(), 2);
    }

    #[test]
    fn test_parse_cron_field_counts() {
        assert!(parse_cron("0 3 * * *").is_ok());
        assert!(parse_cron("0 2 * * SUN").is_ok());
        assert!(parse_cron("0 0 3 * * *").is_ok());
        assert!(parse_cron("0 0 3 * * * *").is_ok());
        assert!(parse_cron("* * *").is_err());
    }
}
