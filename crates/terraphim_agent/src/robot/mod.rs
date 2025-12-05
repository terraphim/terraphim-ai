//! Robot Mode - Machine-readable output for AI agents
//!
//! This module provides structured JSON output and self-documentation
//! capabilities for integration with AI agents and automation tools.

pub mod docs;
pub mod exit_codes;
pub mod output;
pub mod schema;

pub use docs::{ArgumentDoc, Capabilities, CommandDoc, ExampleDoc, FlagDoc, SelfDocumentation};
pub use exit_codes::ExitCode;
pub use output::{FieldMode, OutputFormat, RobotConfig, RobotFormatter};
pub use schema::{
    AutoCorrection, Pagination, ResponseMeta, RobotError, RobotResponse, TokenBudget,
};
