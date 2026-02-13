//! Skills system for reusable workflows.
//!
//! Skills are JSON-defined workflows that can be saved, loaded, and executed.
//! Each skill consists of sequential steps (tool calls or LLM prompts).

pub mod executor;
pub mod monitor;
pub mod types;

#[allow(unused_imports)]
pub use executor::SkillExecutor;
#[allow(unused_imports)]
pub use monitor::{ExecutionReport, ProgressTracker, SkillMonitor};
#[allow(unused_imports)]
pub use types::{Skill, SkillInput, SkillResult, SkillStatus, SkillStep};
