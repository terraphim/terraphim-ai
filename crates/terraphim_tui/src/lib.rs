pub mod client;
pub mod service;

#[cfg(feature = "repl")]
pub mod repl;

#[cfg(feature = "repl-custom")]
pub mod commands;

pub use client::*;

#[cfg(feature = "repl")]
pub use repl::*;

#[cfg(feature = "repl-custom")]
pub use commands::*;

// Test-specific exports - make modules available in tests with required features
#[cfg(test)]
pub mod test_exports {
    #[cfg(feature = "repl")]
    pub use crate::repl::*;

    #[cfg(feature = "repl-custom")]
    pub use crate::commands::*;
}
