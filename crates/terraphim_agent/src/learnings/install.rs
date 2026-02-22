//! Hook installation for AI agents.
//!
//! This module provides functionality to install hooks for various AI agents
//! (Claude Code, Codex, opencode) to capture failed commands as learnings.
//!
//! # Usage
//!
//! ```rust,ignore
//! use terraphim_agent::learnings::{AgentType, install_hook};
//!
//! install_hook(AgentType::Claude).await?;
//! ```

use std::path::PathBuf;

use thiserror::Error;

/// AI agent type for hook installation.
#[derive(Debug, Clone, Copy, PartialEq, clap::ValueEnum)]
#[allow(dead_code)]
pub enum AgentType {
    /// Claude Code (Claude CLI)
    Claude,
    /// OpenAI Codex CLI
    Codex,
    /// Opencode CLI
    Opencode,
}

impl AgentType {
    /// Get the agent's name as a string.
    pub fn as_str(&self) -> &'static str {
        match self {
            AgentType::Claude => "claude",
            AgentType::Codex => "codex",
            AgentType::Opencode => "opencode",
        }
    }

    /// Get the agent's config directory.
    pub fn config_dir(&self) -> Option<PathBuf> {
        match self {
            AgentType::Claude => dirs::config_dir().map(|d| d.join("claude")),
            AgentType::Codex => dirs::config_dir().map(|d| d.join("codex")),
            AgentType::Opencode => dirs::config_dir().map(|d| d.join("opencode")),
        }
    }

    /// Get the hook script content for this agent.
    pub fn hook_script(&self) -> String {
        match self {
            AgentType::Claude => r#"#!/bin/bash
# Terraphim Agent Hook for Claude Code
# This hook captures failed commands for learning

set -e

# Capture the tool execution result and pipe to terraphim-agent
if command -v terraphim-agent >/dev/null 2>&1; then
    # Read stdin
    INPUT=$(cat)

    # Pass through to terraphim-agent hook
    echo "$INPUT" | terraphim-agent learn hook --format claude 2>/dev/null || true

    # Always pass through original input (fail-open)
    echo "$INPUT"
else
    # terraphim-agent not installed, pass through unchanged
    cat
fi
"#
            .to_string(),
            AgentType::Codex => r#"#!/bin/bash
# Terraphim Agent Hook for OpenAI Codex
# This hook captures failed commands for learning

set -e

# Capture the tool execution result and pipe to terraphim-agent
if command -v terraphim-agent >/dev/null 2>&1; then
    # Read stdin
    INPUT=$(cat)

    # Pass through to terraphim-agent hook
    echo "$INPUT" | terraphim-agent learn hook --format codex 2>/dev/null || true

    # Always pass through original input (fail-open)
    echo "$INPUT"
else
    # terraphim-agent not installed, pass through unchanged
    cat
fi
"#
            .to_string(),
            AgentType::Opencode => r#"#!/bin/bash
# Terraphim Agent Hook for Opencode
# This hook captures failed commands for learning

set -e

# Capture the tool execution result and pipe to terraphim-agent
if command -v terraphim-agent >/dev/null 2>&1; then
    # Read stdin
    INPUT=$(cat)

    # Pass through to terraphim-agent hook
    echo "$INPUT" | terraphim-agent learn hook --format opencode 2>/dev/null || true

    # Always pass through original input (fail-open)
    echo "$INPUT"
else
    # terraphim-agent not installed, pass through unchanged
    cat
fi
"#
            .to_string(),
        }
    }

    /// Get the hook file path for this agent.
    pub fn hook_path(&self) -> Option<PathBuf> {
        self.config_dir().map(|d| d.join("terraphim-hook.sh"))
    }
}

/// Errors that can occur during hook installation.
#[derive(Debug, Error)]
#[allow(dead_code)]
pub enum InstallError {
    /// Failed to create config directory
    #[error("failed to create config directory: {0}")]
    ConfigDirError(String),
    /// Failed to write hook file
    #[error("failed to write hook file: {0}")]
    WriteError(#[from] std::io::Error),
    /// Agent config directory not found
    #[error("agent config directory not found")]
    ConfigNotFound,
    /// Hook already exists
    #[error("hook already exists at {0}")]
    AlreadyExists(String),
}

/// Install hook for the specified AI agent.
///
/// Creates a hook script in the agent's config directory that captures
/// failed commands and forwards them to terraphim-agent for learning.
///
/// # Arguments
///
/// * `agent` - The AI agent type to install the hook for
///
/// # Returns
///
/// Ok(()) if installation succeeds, Err(InstallError) otherwise.
///
/// # Examples
///
/// ```rust,ignore
/// use terraphim_agent::learnings::{AgentType, install_hook};
///
/// install_hook(AgentType::Claude).await?;
/// ```
pub async fn install_hook(agent: AgentType) -> Result<(), InstallError> {
    let config_dir = agent.config_dir().ok_or(InstallError::ConfigNotFound)?;
    let hook_path = agent.hook_path().ok_or(InstallError::ConfigNotFound)?;

    // Create config directory if it doesn't exist
    tokio::fs::create_dir_all(&config_dir)
        .await
        .map_err(|e| InstallError::ConfigDirError(e.to_string()))?;

    // Check if hook already exists
    if hook_path.exists() {
        return Err(InstallError::AlreadyExists(
            hook_path.to_string_lossy().to_string(),
        ));
    }

    // Write hook script
    let hook_content = agent.hook_script();
    tokio::fs::write(&hook_path, hook_content)
        .await
        .map_err(InstallError::WriteError)?;

    // Make hook executable (Unix only)
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let metadata = tokio::fs::metadata(&hook_path)
            .await
            .map_err(InstallError::WriteError)?;
        let mut permissions = metadata.permissions();
        permissions.set_mode(0o755);
        tokio::fs::set_permissions(&hook_path, permissions)
            .await
            .map_err(InstallError::WriteError)?;
    }

    println!(
        "Installed Terraphim hook for {} at: {}",
        agent.as_str(),
        hook_path.display()
    );
    println!();
    println!("To activate the hook, add the following to your agent configuration:");
    println!();
    match agent {
        AgentType::Claude => {
            println!("  Claude Code: Set the CLAUDE_HOOK environment variable:");
            println!("    export CLAUDE_HOOK={}", hook_path.display());
        }
        AgentType::Codex => {
            println!("  OpenAI Codex: Set the CODEX_HOOK environment variable:");
            println!("    export CODEX_HOOK={}", hook_path.display());
        }
        AgentType::Opencode => {
            println!("  Opencode: Set the OPCODE_HOOK environment variable:");
            println!("    export OPCODE_HOOK={}", hook_path.display());
        }
    }
    println!();
    println!("Or add the above line to your shell profile (~/.bashrc, ~/.zshrc, etc.)");

    Ok(())
}

/// Uninstall hook for the specified AI agent.
///
/// Removes the hook script from the agent's config directory.
///
/// # Arguments
///
/// * `agent` - The AI agent type to uninstall the hook for
///
/// # Returns
///
/// Ok(()) if uninstallation succeeds, Err(InstallError) otherwise.
#[allow(dead_code)]
pub async fn uninstall_hook(agent: AgentType) -> Result<(), InstallError> {
    let hook_path = agent.hook_path().ok_or(InstallError::ConfigNotFound)?;

    if !hook_path.exists() {
        println!(
            "No hook found for {} at: {}",
            agent.as_str(),
            hook_path.display()
        );
        return Ok(());
    }

    tokio::fs::remove_file(&hook_path)
        .await
        .map_err(InstallError::WriteError)?;

    println!(
        "Uninstalled Terraphim hook for {} from: {}",
        agent.as_str(),
        hook_path.display()
    );

    Ok(())
}

/// Check if a hook is installed for the specified agent.
///
/// # Arguments
///
/// * `agent` - The AI agent type to check
///
/// # Returns
///
/// true if the hook is installed, false otherwise.
#[allow(dead_code)]
pub fn is_hook_installed(agent: AgentType) -> bool {
    agent.hook_path().map(|p| p.exists()).unwrap_or(false)
}

/// Get installation status for all supported agents.
///
/// # Returns
///
/// A vector of tuples containing the agent type and installation status.
#[allow(dead_code)]
pub fn get_installation_status() -> Vec<(AgentType, bool)> {
    vec![
        (AgentType::Claude, is_hook_installed(AgentType::Claude)),
        (AgentType::Codex, is_hook_installed(AgentType::Codex)),
        (AgentType::Opencode, is_hook_installed(AgentType::Opencode)),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_agent_type_as_str() {
        assert_eq!(AgentType::Claude.as_str(), "claude");
        assert_eq!(AgentType::Codex.as_str(), "codex");
        assert_eq!(AgentType::Opencode.as_str(), "opencode");
    }

    #[test]
    fn test_agent_type_variants_distinct() {
        assert_ne!(AgentType::Claude, AgentType::Codex);
        assert_ne!(AgentType::Claude, AgentType::Opencode);
        assert_ne!(AgentType::Codex, AgentType::Opencode);
    }

    #[test]
    fn test_hook_script_contains_agent_name() {
        let claude_script = AgentType::Claude.hook_script();
        assert!(claude_script.contains("terraphim-agent"));
        assert!(claude_script.contains("learn hook"));

        let codex_script = AgentType::Codex.hook_script();
        assert!(codex_script.contains("terraphim-agent"));
        assert!(codex_script.contains("learn hook"));

        let opencode_script = AgentType::Opencode.hook_script();
        assert!(opencode_script.contains("terraphim-agent"));
        assert!(opencode_script.contains("learn hook"));
    }

    #[test]
    fn test_hook_script_fail_open() {
        // All scripts should pass through unchanged if terraphim-agent fails
        let script = AgentType::Claude.hook_script();
        assert!(script.contains("2>/dev/null || true"));
        assert!(script.contains("cat"));
    }

    #[test]
    fn test_install_error_display() {
        let err = InstallError::ConfigNotFound;
        assert_eq!(err.to_string(), "agent config directory not found");

        let err = InstallError::ConfigDirError("permission denied".to_string());
        assert_eq!(
            err.to_string(),
            "failed to create config directory: permission denied"
        );
    }
}
