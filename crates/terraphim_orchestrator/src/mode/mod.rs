//! Agent execution modes.
//!
//! Provides different scheduling strategies:
//! - Time-driven: Cron-based scheduling
//! - Issue-driven: Issue tracker polling

pub mod issue;
pub mod time;

pub use issue::IssueMode;
pub use time::TimeMode;
