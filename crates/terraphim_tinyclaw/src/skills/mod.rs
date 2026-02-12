//! Skills system for reusable workflows.
//!
//! Skills are JSON-defined workflows that can be saved, loaded, and executed.
//! Each skill consists of sequential steps (tool calls or LLM prompts).

pub mod executor;
pub mod types;

pub use executor::SkillExecutor;
pub use types::{Skill, SkillResult, SkillStep, SkillStatus};
