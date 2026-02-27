//! Skills system for reusable workflows.
//!
//! Skills are workflow definitions that can be represented as JSON or markdown
//! with YAML frontmatter.
//! Each skill consists of sequential steps (tool calls or LLM prompts).

pub mod executor;
pub mod markdown;
pub mod monitor;
pub mod types;

#[allow(unused_imports)]
pub use executor::SkillExecutor;
#[allow(unused_imports)]
pub use markdown::{MarkdownSkillParseError, parse_markdown_skill, parse_markdown_skill_file};
#[allow(unused_imports)]
pub use monitor::{ExecutionReport, ProgressTracker, SkillMonitor};
#[allow(unused_imports)]
pub use types::{Skill, SkillInput, SkillResult, SkillStatus, SkillStep};
