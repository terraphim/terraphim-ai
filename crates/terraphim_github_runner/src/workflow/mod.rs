//! Workflow parsing and execution
//!
//! This module provides:
//! - LLM-based workflow understanding (parser.rs)
//! - Step-by-step execution with snapshots (executor.rs)
//! - Firecracker VM-based execution (vm_executor.rs)

pub mod executor;
pub mod parser;
pub mod vm_executor;

pub use executor::{
    CommandExecutor, CommandResult, HostCommandExecutor, MockCommandExecutor, WorkflowExecutor,
    WorkflowExecutorConfig,
};
#[cfg(feature = "github-runner")]
pub use parser::WorkflowParser;
pub use parser::{
    ParsedWorkflow, WorkflowStep, parse_single_workflow_yaml, parse_workflow_payload,
};
pub use vm_executor::{SimulatedVmExecutor, VmCommandExecutor};
