//! Robot Mode - Machine-readable output for AI agents
//!
//! This module provides structured JSON output and self-documentation
//! capabilities for integration with AI agents and automation tools.

#[allow(dead_code)]
pub mod docs;
#[allow(dead_code)]
pub mod exit_codes;
#[allow(dead_code)]
pub mod output;
#[allow(dead_code)]
pub mod schema;

#[allow(unused_imports)]
pub use docs::{ArgumentDoc, Capabilities, CommandDoc, ExampleDoc, FlagDoc, SelfDocumentation};
#[allow(unused_imports)]
pub use exit_codes::ExitCode;
#[allow(unused_imports)]
pub use output::{FieldMode, OutputFormat, RobotConfig, RobotFormatter};
#[allow(unused_imports)]
pub use schema::{
    AutoCorrection, Pagination, ResponseMeta, RobotError, RobotResponse, TokenBudget,
};
