//! Procedure replay engine for executing stored procedures.
//!
//! This module provides the ability to replay captured procedures by
//! executing their steps in sequence. It includes dry-run support and
//! guard-pattern safety checks to prevent destructive commands.

use std::process::Command;

use terraphim_types::procedure::CapturedProcedure;

use crate::guard_patterns::{CommandGuard, GuardDecision};

/// Outcome of executing a single procedure step.
#[derive(Debug, Clone)]
pub enum StepOutcome {
    /// Step executed successfully.
    Success { stdout: String },
    /// Step execution failed.
    Failed { stderr: String, exit_code: i32 },
    /// Step was skipped (e.g., privileged or blocked by guard).
    Skipped { reason: String },
}

/// Result of replaying an entire procedure.
#[derive(Debug)]
pub struct ReplayResult {
    /// Outcomes for each step, paired with ordinal number.
    pub outcomes: Vec<(u32, StepOutcome)>,
    /// Whether all executed steps succeeded.
    pub overall_success: bool,
}

/// Replay a captured procedure by executing its steps in order.
///
/// If `dry_run` is true, each step's command is printed without execution
/// and all steps are reported as successful.
///
/// Safety rules:
/// - Privileged steps are always skipped.
/// - Commands blocked by the guard pattern system are skipped.
/// - On any step failure, replay stops immediately (no further steps run).
pub fn replay_procedure(
    procedure: &CapturedProcedure,
    dry_run: bool,
) -> Result<ReplayResult, std::io::Error> {
    let guard = CommandGuard::new();
    let mut outcomes: Vec<(u32, StepOutcome)> = Vec::new();
    let mut overall_success = true;

    for step in &procedure.steps {
        // Skip privileged steps
        if step.privileged {
            outcomes.push((
                step.ordinal,
                StepOutcome::Skipped {
                    reason: "step is marked as privileged".to_string(),
                },
            ));
            continue;
        }

        // Check guard patterns for destructive commands
        let guard_result = guard.check(&step.command);
        if guard_result.decision == GuardDecision::Block {
            let reason = guard_result
                .reason
                .unwrap_or_else(|| "blocked by guard pattern".to_string());
            outcomes.push((
                step.ordinal,
                StepOutcome::Skipped {
                    reason: format!("BLOCKED: {}", reason),
                },
            ));
            // A blocked step is not a failure -- it is a skip. Continue.
            continue;
        }

        // Print precondition as info if present
        if let Some(ref precondition) = step.precondition {
            println!("  [precondition] {}", precondition);
        }

        if dry_run {
            println!("  [dry-run] step {}: {}", step.ordinal, step.command);
            outcomes.push((
                step.ordinal,
                StepOutcome::Success {
                    stdout: "(dry-run)".to_string(),
                },
            ));
            continue;
        }

        // Execute the command
        let output = Command::new("sh").arg("-c").arg(&step.command).output()?;

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();

        if output.status.success() {
            outcomes.push((step.ordinal, StepOutcome::Success { stdout }));
        } else {
            let exit_code = output.status.code().unwrap_or(-1);
            outcomes.push((step.ordinal, StepOutcome::Failed { stderr, exit_code }));
            overall_success = false;
            // Stop on first failure
            break;
        }
    }

    Ok(ReplayResult {
        outcomes,
        overall_success,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use terraphim_types::procedure::{CapturedProcedure, ProcedureStep};

    fn make_procedure(steps: Vec<ProcedureStep>) -> CapturedProcedure {
        let mut proc = CapturedProcedure::new(
            "test-replay-id".to_string(),
            "Test Replay".to_string(),
            "A procedure for testing replay".to_string(),
        );
        proc.add_steps(steps);
        proc
    }

    fn echo_step(ordinal: u32, msg: &str) -> ProcedureStep {
        ProcedureStep {
            ordinal,
            command: format!("echo {}", msg),
            precondition: None,
            postcondition: None,
            working_dir: None,
            privileged: false,
            tags: vec![],
        }
    }

    #[test]
    fn test_dry_run_reports_all_success() {
        let proc = make_procedure(vec![echo_step(1, "hello"), echo_step(2, "world")]);

        let result = replay_procedure(&proc, true).unwrap();
        assert!(result.overall_success);
        assert_eq!(result.outcomes.len(), 2);

        for (_, outcome) in &result.outcomes {
            match outcome {
                StepOutcome::Success { stdout } => {
                    assert_eq!(stdout, "(dry-run)");
                }
                other => panic!("Expected Success for dry-run, got: {:?}", other),
            }
        }
    }

    #[test]
    fn test_real_replay_success() {
        let proc = make_procedure(vec![echo_step(1, "hello"), echo_step(2, "world")]);

        let result = replay_procedure(&proc, false).unwrap();
        assert!(result.overall_success);
        assert_eq!(result.outcomes.len(), 2);

        match &result.outcomes[0].1 {
            StepOutcome::Success { stdout } => {
                assert!(
                    stdout.contains("hello"),
                    "Expected 'hello' in stdout, got: {}",
                    stdout
                );
            }
            other => panic!("Expected Success, got: {:?}", other),
        }

        match &result.outcomes[1].1 {
            StepOutcome::Success { stdout } => {
                assert!(
                    stdout.contains("world"),
                    "Expected 'world' in stdout, got: {}",
                    stdout
                );
            }
            other => panic!("Expected Success, got: {:?}", other),
        }
    }

    #[test]
    fn test_replay_stops_on_failure() {
        let proc = make_procedure(vec![
            ProcedureStep {
                ordinal: 1,
                command: "false".to_string(), // always exits with code 1
                precondition: None,
                postcondition: None,
                working_dir: None,
                privileged: false,
                tags: vec![],
            },
            echo_step(2, "should-not-run"),
        ]);

        let result = replay_procedure(&proc, false).unwrap();
        assert!(!result.overall_success);
        // Only the first step should have run
        assert_eq!(result.outcomes.len(), 1);

        match &result.outcomes[0].1 {
            StepOutcome::Failed { exit_code, .. } => {
                assert_eq!(*exit_code, 1);
            }
            other => panic!("Expected Failed, got: {:?}", other),
        }
    }

    #[test]
    fn test_privileged_step_is_skipped() {
        let proc = make_procedure(vec![
            ProcedureStep {
                ordinal: 1,
                command: "echo privileged-cmd".to_string(),
                precondition: None,
                postcondition: None,
                working_dir: None,
                privileged: true,
                tags: vec![],
            },
            echo_step(2, "after-privileged"),
        ]);

        let result = replay_procedure(&proc, false).unwrap();
        assert!(result.overall_success);
        assert_eq!(result.outcomes.len(), 2);

        match &result.outcomes[0].1 {
            StepOutcome::Skipped { reason } => {
                assert!(
                    reason.contains("privileged"),
                    "Expected privileged reason, got: {}",
                    reason
                );
            }
            other => panic!("Expected Skipped for privileged step, got: {:?}", other),
        }

        match &result.outcomes[1].1 {
            StepOutcome::Success { stdout } => {
                assert!(stdout.contains("after-privileged"));
            }
            other => panic!("Expected Success, got: {:?}", other),
        }
    }

    #[test]
    fn test_destructive_command_is_skipped() {
        let proc = make_procedure(vec![
            ProcedureStep {
                ordinal: 1,
                command: "rm -rf /".to_string(),
                precondition: None,
                postcondition: None,
                working_dir: None,
                privileged: false,
                tags: vec![],
            },
            echo_step(2, "after-blocked"),
        ]);

        let result = replay_procedure(&proc, false).unwrap();
        // Blocked steps are skipped, not failed -- so overall success is true
        assert!(result.overall_success);
        assert_eq!(result.outcomes.len(), 2);

        match &result.outcomes[0].1 {
            StepOutcome::Skipped { reason } => {
                assert!(
                    reason.contains("BLOCKED"),
                    "Expected BLOCKED reason, got: {}",
                    reason
                );
            }
            other => panic!("Expected Skipped for destructive command, got: {:?}", other),
        }
    }
}
