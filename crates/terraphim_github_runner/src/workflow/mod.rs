//! Workflow parsing and execution
//!
//! This module provides:
//! - LLM-based workflow understanding (parser.rs)
//! - Step-by-step execution with snapshots (executor.rs)

pub mod executor;
pub mod parser;

pub use executor::{
    CommandExecutor, CommandResult, MockCommandExecutor, WorkflowExecutor, WorkflowExecutorConfig,
};
pub use parser::{ParsedWorkflow, WorkflowParser, WorkflowStep};
