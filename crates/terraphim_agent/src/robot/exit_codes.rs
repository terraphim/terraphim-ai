//! Exit codes for robot mode
//!
//! Standard exit codes for machine consumption, following Unix conventions
//! with domain-specific extensions.

use std::process::Termination;

/// Exit codes for terraphim-agent robot mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum ExitCode {
    /// Operation completed successfully
    Success = 0,
    /// General/unspecified error
    ErrorGeneral = 1,
    /// Invalid arguments or syntax error
    ErrorUsage = 2,
    /// Required index not initialized
    ErrorIndexMissing = 3,
    /// No results found for query
    ErrorNotFound = 4,
    /// Authentication required or failed
    ErrorAuth = 5,
    /// Network or connectivity issue
    ErrorNetwork = 6,
    /// Operation timed out
    ErrorTimeout = 7,
}

impl ExitCode {
    /// Get the numeric exit code value
    pub fn code(self) -> u8 {
        self as u8
    }

    /// Get a human-readable description
    pub fn description(self) -> &'static str {
        match self {
            ExitCode::Success => "Operation completed successfully",
            ExitCode::ErrorGeneral => "General error",
            ExitCode::ErrorUsage => "Invalid arguments or syntax",
            ExitCode::ErrorIndexMissing => "Required index not initialized",
            ExitCode::ErrorNotFound => "No results found",
            ExitCode::ErrorAuth => "Authentication required",
            ExitCode::ErrorNetwork => "Network error",
            ExitCode::ErrorTimeout => "Operation timed out",
        }
    }

    /// Get the exit code name for JSON output
    pub fn name(self) -> &'static str {
        match self {
            ExitCode::Success => "SUCCESS",
            ExitCode::ErrorGeneral => "ERROR_GENERAL",
            ExitCode::ErrorUsage => "ERROR_USAGE",
            ExitCode::ErrorIndexMissing => "ERROR_INDEX_MISSING",
            ExitCode::ErrorNotFound => "ERROR_NOT_FOUND",
            ExitCode::ErrorAuth => "ERROR_AUTH",
            ExitCode::ErrorNetwork => "ERROR_NETWORK",
            ExitCode::ErrorTimeout => "ERROR_TIMEOUT",
        }
    }

    /// Convert from u8
    pub fn from_code(code: u8) -> Self {
        match code {
            0 => ExitCode::Success,
            2 => ExitCode::ErrorUsage,
            3 => ExitCode::ErrorIndexMissing,
            4 => ExitCode::ErrorNotFound,
            5 => ExitCode::ErrorAuth,
            6 => ExitCode::ErrorNetwork,
            7 => ExitCode::ErrorTimeout,
            _ => ExitCode::ErrorGeneral,
        }
    }
}

impl From<ExitCode> for std::process::ExitCode {
    fn from(code: ExitCode) -> Self {
        std::process::ExitCode::from(code.code())
    }
}

impl Termination for ExitCode {
    fn report(self) -> std::process::ExitCode {
        self.into()
    }
}

impl std::fmt::Display for ExitCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} ({})", self.name(), self.code())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_exit_code_values() {
        assert_eq!(ExitCode::Success.code(), 0);
        assert_eq!(ExitCode::ErrorGeneral.code(), 1);
        assert_eq!(ExitCode::ErrorUsage.code(), 2);
        assert_eq!(ExitCode::ErrorIndexMissing.code(), 3);
        assert_eq!(ExitCode::ErrorNotFound.code(), 4);
        assert_eq!(ExitCode::ErrorAuth.code(), 5);
        assert_eq!(ExitCode::ErrorNetwork.code(), 6);
        assert_eq!(ExitCode::ErrorTimeout.code(), 7);
    }

    #[test]
    fn test_exit_code_from_code() {
        assert_eq!(ExitCode::from_code(0), ExitCode::Success);
        assert_eq!(ExitCode::from_code(2), ExitCode::ErrorUsage);
        assert_eq!(ExitCode::from_code(99), ExitCode::ErrorGeneral); // Unknown maps to general
    }
}
