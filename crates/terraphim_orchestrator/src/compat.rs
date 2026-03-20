//! Symphony Compatibility Layer
//!
//! This module provides backward compatibility adapters and type aliases
//! for migrating from the Symphony orchestrator to the unified terraphim
//! orchestrator.
//!
//! ## Usage
//!
//! Import this module when migrating existing code:
//! ```rust
//! use terraphim_orchestrator::compat::SymphonyAdapter;
//! ```

use crate::{
    AgentDefinition, AgentLayer, AgentOrchestrator, DispatchQueue, DispatchTask, ModeCoordinator,
    OrchestratorConfig, WorkflowConfig, WorkflowMode,
};
use std::path::PathBuf;

/// Type alias for backward compatibility with Symphony code.
pub type SymphonyOrchestrator = AgentOrchestrator;

/// Type alias for Symphony agent definitions.
pub type SymphonyAgent = AgentDefinition;

/// Type alias for Symphony workflow modes.
pub type SymphonyMode = WorkflowMode;

/// Adapter for migrating from Symphony to the unified orchestrator.
///
/// Provides helper methods to convert Symphony-style configurations
/// to the new dual-mode format.
pub struct SymphonyAdapter;

impl SymphonyAdapter {
    /// Create a legacy (time-only) configuration from a Symphony-style config.
    ///
    /// This ensures backward compatibility with existing orchestrator.toml
    /// files that don't have the workflow section.
    pub fn to_legacy_config(config: OrchestratorConfig) -> OrchestratorConfig {
        // If no workflow config, it's already in legacy mode
        if config.workflow.is_none() {
            return config;
        }

        // Otherwise, force time-only mode
        let mut new_config = config;
        if let Some(ref mut workflow) = new_config.workflow {
            workflow.mode = WorkflowMode::TimeOnly;
        }
        new_config.tracker = None;
        new_config
    }

    /// Enable dual mode for an existing configuration.
    ///
    /// Adds workflow and tracker configuration to enable both
    /// time-based and issue-driven scheduling.
    pub fn enable_dual_mode(
        mut config: OrchestratorConfig,
        tracker_config: crate::config::TrackerConfig,
        poll_interval_secs: u64,
    ) -> OrchestratorConfig {
        config.workflow = Some(WorkflowConfig {
            mode: WorkflowMode::Dual,
            poll_interval_secs,
            max_concurrent_tasks: 5,
        });
        config.tracker = Some(tracker_config);
        config.concurrency = Some(crate::config::ConcurrencyConfig {
            max_parallel_agents: 3,
            queue_depth: 100,
            starvation_timeout_secs: 300,
        });
        config
    }

    /// Create a time-only (legacy) orchestrator configuration.
    ///
    /// This is the default mode for backward compatibility.
    pub fn create_legacy_config(
        working_dir: PathBuf,
        agents: Vec<AgentDefinition>,
    ) -> OrchestratorConfig {
        OrchestratorConfig {
            working_dir,
            nightwatch: crate::config::NightwatchConfig::default(),
            compound_review: crate::config::CompoundReviewConfig {
                schedule: "0 2 * * *".to_string(),
                max_duration_secs: 1800,
                repo_path: PathBuf::from("/tmp"),
                create_prs: false,
            },
            agents,
            restart_cooldown_secs: 60,
            max_restart_count: 10,
            tick_interval_secs: 30,
            allowed_providers: vec![],
            banned_providers: vec![],
            skill_registry: Default::default(),
            stagger_delay_ms: 5000,
            review_pairs: vec![],
            drift_detection: crate::config::DriftDetectionConfig::default(),
            session_rotation: crate::config::SessionRotationConfig::default(),
            convergence: Default::default(),
            workflow: None, // No workflow = legacy mode
            tracker: None,
            concurrency: None,
        }
    }

    /// Check if a configuration is in legacy mode (time-only).
    pub fn is_legacy_mode(config: &OrchestratorConfig) -> bool {
        config.workflow.is_none()
            || matches!(
                config.workflow.as_ref().map(|w| w.mode),
                Some(WorkflowMode::TimeOnly)
            )
    }

    /// Check if a configuration has dual mode enabled.
    pub fn is_dual_mode(config: &OrchestratorConfig) -> bool {
        matches!(
            config.workflow.as_ref().map(|w| w.mode),
            Some(WorkflowMode::Dual)
        )
    }

    /// Get a human-readable description of the workflow mode.
    pub fn describe_mode(config: &OrchestratorConfig) -> String {
        match config.workflow.as_ref().map(|w| w.mode) {
            None => "Legacy (Time-Only)".to_string(),
            Some(WorkflowMode::TimeOnly) => "Time-Only".to_string(),
            Some(WorkflowMode::IssueOnly) => "Issue-Only".to_string(),
            Some(WorkflowMode::Dual) => "Dual (Time + Issue)".to_string(),
        }
    }
}

/// Extension trait for AgentOrchestrator to provide Symphony-compatible methods.
pub trait SymphonyOrchestratorExt {
    /// Check if this orchestrator is running in legacy mode.
    fn is_legacy_mode(&self) -> bool;

    /// Check if dual mode is active.
    fn is_dual_mode(&self) -> bool;

    /// Get the active workflow mode description.
    fn mode_description(&self) -> String;

    /// Create a basic legacy orchestrator (for testing/migration).
    fn new_legacy(
        working_dir: PathBuf,
        agents: Vec<AgentDefinition>,
    ) -> Result<Self, crate::OrchestratorError>
    where
        Self: Sized;
}

impl SymphonyOrchestratorExt for AgentOrchestrator {
    fn is_legacy_mode(&self) -> bool {
        self.workflow_mode().is_none()
            || matches!(self.workflow_mode(), Some(WorkflowMode::TimeOnly))
    }

    fn is_dual_mode(&self) -> bool {
        matches!(self.workflow_mode(), Some(WorkflowMode::Dual))
    }

    fn mode_description(&self) -> String {
        match self.workflow_mode() {
            None => "Legacy (Time-Only)".to_string(),
            Some(WorkflowMode::TimeOnly) => "Time-Only".to_string(),
            Some(WorkflowMode::IssueOnly) => "Issue-Only".to_string(),
            Some(WorkflowMode::Dual) => "Dual (Time + Issue)".to_string(),
        }
    }

    fn new_legacy(
        working_dir: PathBuf,
        agents: Vec<AgentDefinition>,
    ) -> Result<Self, crate::OrchestratorError> {
        let config = SymphonyAdapter::create_legacy_config(working_dir, agents);
        Self::new(config)
    }
}

/// Helper functions for common migration tasks.
pub mod migration {
    use super::*;

    /// Migrate a legacy config file to add dual mode support.
    ///
    /// This reads the existing config and adds the workflow section
    /// while preserving all other settings.
    pub fn migrate_config_to_dual_mode(
        config_path: &std::path::Path,
        tracker_config: crate::config::TrackerConfig,
    ) -> Result<OrchestratorConfig, Box<dyn std::error::Error>> {
        let config = OrchestratorConfig::from_file(config_path)?;

        if SymphonyAdapter::is_dual_mode(&config) {
            return Ok(config); // Already migrated
        }

        Ok(SymphonyAdapter::enable_dual_mode(
            config,
            tracker_config,
            60, // Default poll interval
        ))
    }

    /// Validate that a migrated config is correct.
    pub fn validate_migrated_config(config: &OrchestratorConfig) -> Result<(), String> {
        // Check for required fields
        if config.workflow.is_none() {
            return Err("Missing workflow configuration".to_string());
        }

        let workflow = config.workflow.as_ref().unwrap();

        // Validate mode
        match workflow.mode {
            WorkflowMode::TimeOnly | WorkflowMode::Dual => {
                // These modes are valid
            }
            WorkflowMode::IssueOnly => {
                // Issue-only requires tracker
                if config.tracker.is_none() {
                    return Err("Issue-only mode requires tracker configuration".to_string());
                }
            }
        }

        // Validate poll interval
        if workflow.poll_interval_secs == 0 {
            return Err("Poll interval must be greater than 0".to_string());
        }

        // Validate concurrent tasks
        if workflow.max_concurrent_tasks == 0 {
            return Err("Max concurrent tasks must be greater than 0".to_string());
        }

        Ok(())
    }

    /// Generate migration report showing before/after comparison.
    pub fn generate_migration_report(
        old_config: &OrchestratorConfig,
        new_config: &OrchestratorConfig,
    ) -> String {
        let mut report = String::new();

        report.push_str("# Configuration Migration Report\n\n");

        report.push_str("## Before\n");
        report.push_str(&format!(
            "- Mode: {}\n",
            SymphonyAdapter::describe_mode(old_config)
        ));
        report.push_str(&format!("- Agents: {}\n", old_config.agents.len()));
        report.push_str(&format!(
            "- Has Tracker: {}\n\n",
            old_config.tracker.is_some()
        ));

        report.push_str("## After\n");
        report.push_str(&format!(
            "- Mode: {}\n",
            SymphonyAdapter::describe_mode(new_config)
        ));
        report.push_str(&format!("- Agents: {}\n", new_config.agents.len()));
        report.push_str(&format!(
            "- Has Tracker: {}\n",
            new_config.tracker.is_some()
        ));

        if let Some(ref workflow) = new_config.workflow {
            report.push_str(&format!(
                "- Poll Interval: {}s\n",
                workflow.poll_interval_secs
            ));
            report.push_str(&format!(
                "- Max Concurrent Tasks: {}\n",
                workflow.max_concurrent_tasks
            ));
        }

        report
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_symphony_adapter_legacy_detection() {
        let legacy_config = SymphonyAdapter::create_legacy_config(PathBuf::from("/tmp"), vec![]);

        assert!(SymphonyAdapter::is_legacy_mode(&legacy_config));
        assert!(!SymphonyAdapter::is_dual_mode(&legacy_config));
        assert_eq!(
            SymphonyAdapter::describe_mode(&legacy_config),
            "Legacy (Time-Only)"
        );
    }

    #[test]
    fn test_symphony_adapter_dual_mode_detection() {
        let mut config = SymphonyAdapter::create_legacy_config(PathBuf::from("/tmp"), vec![]);

        config.workflow = Some(WorkflowConfig {
            mode: WorkflowMode::Dual,
            poll_interval_secs: 60,
            max_concurrent_tasks: 5,
        });

        assert!(!SymphonyAdapter::is_legacy_mode(&config));
        assert!(SymphonyAdapter::is_dual_mode(&config));
        assert_eq!(
            SymphonyAdapter::describe_mode(&config),
            "Dual (Time + Issue)"
        );
    }

    #[test]
    fn test_symphony_orchestrator_ext() {
        let config = SymphonyAdapter::create_legacy_config(PathBuf::from("/tmp"), vec![]);

        let orch = AgentOrchestrator::new(config).unwrap();

        assert!(orch.is_legacy_mode());
        assert!(!orch.is_dual_mode());
        assert_eq!(orch.mode_description(), "Legacy (Time-Only)");
    }

    #[test]
    fn test_migration_validate_config() {
        use migration::validate_migrated_config;

        // Valid config
        let mut config = SymphonyAdapter::create_legacy_config(PathBuf::from("/tmp"), vec![]);
        config.workflow = Some(WorkflowConfig {
            mode: WorkflowMode::Dual,
            poll_interval_secs: 60,
            max_concurrent_tasks: 5,
        });

        assert!(validate_migrated_config(&config).is_ok());

        // Invalid: zero poll interval
        let mut bad_config = config.clone();
        bad_config.workflow.as_mut().unwrap().poll_interval_secs = 0;
        assert!(validate_migrated_config(&bad_config).is_err());

        // Invalid: missing workflow
        let mut bad_config = config.clone();
        bad_config.workflow = None;
        assert!(validate_migrated_config(&bad_config).is_err());
    }

    #[test]
    fn test_migration_report() {
        let old_config = SymphonyAdapter::create_legacy_config(
            PathBuf::from("/tmp"),
            vec![AgentDefinition {
                name: "test".to_string(),
                layer: AgentLayer::Safety,
                cli_tool: "echo".to_string(),
                task: "test".to_string(),
                model: None,
                schedule: None,
                capabilities: vec![],
                max_memory_bytes: None,
                provider: None,
                fallback_provider: None,
                fallback_model: None,
                provider_tier: None,
                persona_name: None,
                persona_symbol: None,
                persona_vibe: None,
                meta_cortex_connections: vec![],
                skill_chain: vec![],
            }],
        );

        let mut new_config = old_config.clone();
        new_config.workflow = Some(WorkflowConfig {
            mode: WorkflowMode::Dual,
            poll_interval_secs: 60,
            max_concurrent_tasks: 5,
        });

        let report = migration::generate_migration_report(&old_config, &new_config);

        assert!(report.contains("Before"));
        assert!(report.contains("After"));
        assert!(report.contains("Legacy (Time-Only)"));
        assert!(report.contains("Dual (Time + Issue)"));
        assert!(report.contains("Agents: 1"));
    }
}
