pub mod client;
pub mod service;

// Robot mode - always available for AI agent integration
pub mod robot;

// Forgiving CLI - always available for typo-tolerant parsing
pub mod forgiving;

#[cfg(feature = "repl")]
pub mod repl;

#[cfg(feature = "repl-custom")]
pub mod commands;

pub use client::*;

// Re-export robot mode types
pub use robot::{
    ExitCode, FieldMode, OutputFormat, RobotConfig, RobotError, RobotFormatter, RobotResponse,
    SelfDocumentation,
};

// Re-export forgiving CLI types
pub use forgiving::{AliasRegistry, ForgivingParser, ParseResult};

#[cfg(feature = "repl")]
pub use repl::*;

#[cfg(feature = "repl-custom")]
pub use commands::*;

// Test-specific exports - make modules available in tests with required features
#[cfg(test)]
pub mod test_exports {
    #[cfg(feature = "repl")]
    pub use crate::repl::*;

    #[cfg(feature = "repl")]
    pub use std::str::FromStr;

    #[cfg(feature = "repl-custom")]
    pub use crate::commands::*;

    pub use crate::forgiving::*;
    pub use crate::robot::*;
}
