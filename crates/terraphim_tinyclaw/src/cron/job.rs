//! Cron job model and schedule parsing.
//!
//! Wave 3 of the Hermes parity arc. Matches Hermes' `cron/jobs.py` surface:
//! - `Schedule::Delay` for relative delays ("30m", "2h", "1d")
//! - `Schedule::Interval` for recurring intervals ("every 2h")
//! - `Schedule::Cron` for cron expressions ("0 9 * * *") — parsed via
//!   the `cron` crate (also used by `terraphim_orchestrator::TimeScheduler`)
//! - `Schedule::At` for one-shot ISO timestamps

use chrono::{DateTime, Utc};
use cron::Schedule as CronSchedule;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use std::time::Duration;
use uuid::Uuid;

use super::CronError;

/// Schedule specification.
///
/// Hermes supports 4 formats. We model them as a tagged enum so JSON storage is
/// unambiguous and parsing is explicit.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Schedule {
    /// One-shot: fires once after `secs` from creation time.
    Delay {
        /// Delay in seconds.
        secs: u64,
    },
    /// Recurring: fires every `secs`.
    Interval {
        /// Interval in seconds.
        secs: u64,
    },
    /// Cron expression: "0 9 * * *". Parsed via the `cron` crate.
    Cron {
        /// 5-field cron expression.
        expr: String,
    },
    /// One-shot at exact time.
    At {
        /// ISO 8601 timestamp.
        timestamp: DateTime<Utc>,
    },
}

impl Schedule {
    /// Parse a schedule string in any of the 4 supported formats.
    pub fn parse(input: &str) -> Result<Self, CronError> {
        let trimmed = input.trim();
        if trimmed.is_empty() {
            return Err(CronError::InvalidSchedule("empty string".into()));
        }

        if let Some(rest) = trimmed.strip_prefix("every ") {
            return parse_duration_secs(rest)
                .map(|secs| Schedule::Interval { secs })
                .ok_or_else(|| {
                    CronError::InvalidSchedule(format!("invalid interval: {}", trimmed))
                });
        }

        if let Ok(ts) = DateTime::parse_from_rfc3339(trimmed) {
            return Ok(Schedule::At {
                timestamp: ts.with_timezone(&Utc),
            });
        }

        // 5-field cron expression. The `cron` crate requires 6 fields
        // (seconds + standard 5), so we pad with a leading "0" for seconds.
        let parts: Vec<&str> = trimmed.split_whitespace().collect();
        if parts.len() == 5 {
            let padded = format!("0 {trimmed}");
            if CronSchedule::from_str(&padded).is_ok() {
                return Ok(Schedule::Cron {
                    expr: trimmed.to_string(),
                });
            }
        } else if parts.len() == 6 && CronSchedule::from_str(trimmed).is_ok() {
            return Ok(Schedule::Cron {
                expr: trimmed.to_string(),
            });
        }

        // Relative delay: "30m", "2h", "1d"
        parse_duration_secs(trimmed)
            .map(|secs| Schedule::Delay { secs })
            .ok_or_else(|| CronError::InvalidSchedule(format!("unrecognised: {}", trimmed)))
    }

    /// Compute the next fire time given `now` and the last fire time (for
    /// intervals).
    pub fn next_after(
        &self,
        now: DateTime<Utc>,
        last: Option<DateTime<Utc>>,
    ) -> Option<DateTime<Utc>> {
        match self {
            Schedule::Delay { secs } => Some(now + Duration::from_secs(*secs)),
            Schedule::Interval { secs } => {
                let base = last.unwrap_or(now);
                Some(base + Duration::from_secs(*secs))
            }
            Schedule::Cron { expr } => {
                let padded = if expr.split_whitespace().count() == 5 {
                    format!("0 {expr}")
                } else {
                    expr.clone()
                };
                let schedule = CronSchedule::from_str(&padded).ok()?;
                // The `cron` crate's `after()` returns an iterator of fire
                // times strictly after the given instant. Use `now` for fresh
                // jobs and `last` for recurring (so we don't re-fire the same
                // occurrence).
                let anchor = last.unwrap_or(now - Duration::from_secs(1));
                schedule.after(&anchor).next()
            }
            Schedule::At { timestamp } => {
                if *timestamp > now {
                    Some(*timestamp)
                } else {
                    None
                }
            }
        }
    }
}

/// Job lifecycle state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum JobState {
    /// Active, will fire at next scheduled time.
    Scheduled,
    /// Suspended — won't fire until resumed.
    Paused,
    /// Currently executing (transient state).
    Running,
    /// Repeat count exhausted or one-shot that has fired.
    Completed,
}

/// Repeat configuration for recurring jobs.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct RepeatConfig {
    /// Maximum number of fires. `None` means infinite.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub times: Option<u32>,
    /// Number of times the job has fired.
    #[serde(default)]
    pub completed: u32,
}

impl RepeatConfig {
    /// Whether the job has exhausted its repeat count.
    pub fn exhausted(&self) -> bool {
        self.times.is_some_and(|max| self.completed >= max)
    }
}

/// A scheduled job.
///
/// JSON shape mirrors Hermes' `cron/jobs.py` job record.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct CronJob {
    /// Unique job identifier.
    pub id: String,
    /// Human-readable name.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Prompt to execute when the job fires.
    pub prompt: String,
    /// Schedule specification.
    pub schedule: Schedule,
    /// Skills to inject at job start.
    #[serde(default)]
    pub skills: Vec<String>,
    /// Delivery target (e.g. "telegram:-1001234567890:topic").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deliver: Option<String>,
    /// Repeat configuration for recurring jobs.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub repeat: Option<RepeatConfig>,
    /// Current lifecycle state.
    #[serde(default = "default_state")]
    pub state: JobState,
    /// Whether the job is enabled.
    #[serde(default = "default_enabled")]
    pub enabled: bool,
    /// Next scheduled fire time.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_run_at: Option<DateTime<Utc>>,
    /// Last fire time.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_run_at: Option<DateTime<Utc>>,
    /// Last fire status ("ok" or error message).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_status: Option<String>,
    /// Creation timestamp.
    #[serde(default = "Utc::now")]
    pub created_at: DateTime<Utc>,
    /// Override model for this job.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    /// Override provider for this job.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider: Option<String>,
    /// Script path (alternative to prompt).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub script: Option<String>,
}

fn default_state() -> JobState {
    JobState::Scheduled
}

fn default_enabled() -> bool {
    true
}

impl CronJob {
    /// Create a new job with a generated ID.
    pub fn new(prompt: impl Into<String>, schedule: Schedule) -> Self {
        Self {
            id: Uuid::new_v4().simple().to_string(),
            name: None,
            prompt: prompt.into(),
            schedule,
            skills: Vec::new(),
            deliver: None,
            repeat: None,
            state: JobState::Scheduled,
            enabled: true,
            next_run_at: None,
            last_run_at: None,
            last_status: None,
            created_at: Utc::now(),
            model: None,
            provider: None,
            script: None,
        }
    }

    /// Compute and set `next_run_at` relative to `now`.
    pub fn recompute_next_run(&mut self, now: DateTime<Utc>) {
        if !self.enabled || self.state == JobState::Paused || self.state == JobState::Completed {
            self.next_run_at = None;
            return;
        }
        self.next_run_at = self.schedule.next_after(now, self.last_run_at);
    }

    /// Whether this job is due to fire at `now`.
    pub fn is_due(&self, now: DateTime<Utc>) -> bool {
        self.enabled
            && self.state == JobState::Scheduled
            && self.next_run_at.is_some_and(|t| t <= now)
    }
}

// --- parsing helpers ---

fn parse_duration_secs(input: &str) -> Option<u64> {
    let input = input.trim();
    if input.is_empty() {
        return None;
    }
    let (num_str, suffix) = input.split_at(
        input
            .find(|c: char| !c.is_ascii_digit())
            .unwrap_or(input.len()),
    );
    let num: u64 = num_str.parse().ok()?;
    let multiplier = match suffix.trim() {
        "s" | "" => 1,
        "m" | "min" => 60,
        "h" | "hr" | "hour" => 3600,
        "d" | "day" => 86400,
        "w" | "wk" | "week" => 604_800,
        _ => return None,
    };
    Some(num * multiplier)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_schedule_parsing_delay_minutes() {
        let s = Schedule::parse("30m").unwrap();
        assert_eq!(s, Schedule::Delay { secs: 1800 });
    }

    #[test]
    fn test_schedule_parsing_delay_hours() {
        let s = Schedule::parse("2h").unwrap();
        assert_eq!(s, Schedule::Delay { secs: 7200 });
    }

    #[test]
    fn test_schedule_parsing_delay_days() {
        let s = Schedule::parse("1d").unwrap();
        assert_eq!(s, Schedule::Delay { secs: 86400 });
    }

    #[test]
    fn test_schedule_parsing_interval() {
        let s = Schedule::parse("every 2h").unwrap();
        assert_eq!(s, Schedule::Interval { secs: 7200 });
    }

    #[test]
    fn test_schedule_parsing_cron() {
        let s = Schedule::parse("0 9 * * *").unwrap();
        assert_eq!(
            s,
            Schedule::Cron {
                expr: "0 9 * * *".into()
            }
        );
    }

    #[test]
    fn test_schedule_parsing_cron_uses_cron_crate() {
        // Verify the cron crate actually parsed it (next_after returns a valid time)
        let s = Schedule::Cron {
            expr: "0 9 * * *".into(),
        };
        let now = Utc::now();
        let next = s.next_after(now, None);
        assert!(
            next.is_some(),
            "cron crate should parse 5-field expressions"
        );
    }

    #[test]
    fn test_schedule_parsing_at_iso() {
        let s = Schedule::parse("2026-12-25T09:00:00Z").unwrap();
        if let Schedule::At { timestamp } = s {
            assert_eq!(timestamp.to_rfc3339(), "2026-12-25T09:00:00+00:00");
        } else {
            panic!("expected At, got {:?}", s);
        }
    }

    #[test]
    fn test_schedule_parsing_invalid() {
        assert!(Schedule::parse("").is_err());
        assert!(Schedule::parse("nonsense").is_err());
    }

    #[test]
    fn test_repeat_config_exhausted() {
        let r = RepeatConfig {
            times: Some(3),
            completed: 3,
        };
        assert!(r.exhausted());

        let r = RepeatConfig {
            times: Some(3),
            completed: 2,
        };
        assert!(!r.exhausted());

        let r = RepeatConfig {
            times: None,
            completed: 999,
        };
        assert!(!r.exhausted());
    }

    #[test]
    fn test_job_creation_defaults() {
        let job = CronJob::new("test prompt", Schedule::Delay { secs: 60 });
        assert_eq!(job.state, JobState::Scheduled);
        assert!(job.enabled);
        assert!(job.id.len() > 10);
    }

    #[test]
    fn test_job_is_due() {
        let mut job = CronJob::new("test", Schedule::Delay { secs: 60 });
        let now = Utc::now();
        job.next_run_at = Some(now - Duration::from_secs(1));
        assert!(job.is_due(now));

        job.next_run_at = Some(now + Duration::from_secs(60));
        assert!(!job.is_due(now));
    }

    #[test]
    fn test_job_is_not_due_when_paused() {
        let mut job = CronJob::new("test", Schedule::Delay { secs: 60 });
        let now = Utc::now();
        job.next_run_at = Some(now - Duration::from_secs(1));
        job.state = JobState::Paused;
        assert!(!job.is_due(now));
    }
}
