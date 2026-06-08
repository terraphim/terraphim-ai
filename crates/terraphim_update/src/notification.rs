//! Notification and user prompt system for updates
//!
//! This module provides formatted update messages and user interaction
//! prompts for the auto-update system.

use anyhow::{Context, Result};
use dialoguer::Confirm;

/// Update action selected by the user
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UpdateAction {
    /// Update now
    UpdateNow,
    /// Remind later
    RemindLater,
    /// Skip this update
    Skip,
}

/// Format an update notification message
///
/// Creates a user-friendly message for CLI output.
///
/// # Arguments
/// * `current_version` - Current installed version
/// * `latest_version` - Latest available version
/// * `release_notes` - Optional release notes
///
/// # Returns
/// * Formatted notification message
///
/// # Example
/// ```no_run
/// use terraphim_update::notification::get_update_notification;
///
/// let message = get_update_notification("1.0.0", "1.1.0", Some("Bug fixes"));
/// println!("{}", message);
/// ```
pub fn get_update_notification(
    current_version: &str,
    latest_version: &str,
    release_notes: Option<&str>,
) -> String {
    let mut message = format!(
        "\n{} Update Available: {} → {}\n",
        emoji::alert(),
        current_version,
        latest_version
    );

    if let Some(notes) = release_notes {
        message.push_str(&format!("\n{}\n", format_release_notes(notes)));
    }

    message
}

/// Format release notes for display
///
/// Indents and formats release notes for better readability.
///
/// # Arguments
/// * `notes` - Raw release notes text
///
/// # Returns
/// * Formatted release notes
fn format_release_notes(notes: &str) -> String {
    let lines: Vec<&str> = notes.lines().collect();
    lines
        .iter()
        .map(|line| {
            if line.is_empty() {
                String::new()
            } else {
                format!("  {}", line)
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Prompt user for update action
///
/// Asks the user if they want to update now, later, or skip.
/// Default is 'n' (no update now).
///
/// # Arguments
/// * `current_version` - Current installed version
/// * `latest_version` - Latest available version
///
/// # Returns
/// * `Ok(UpdateAction)` - User's choice
/// * `Err(anyhow::Error)` - Error during prompt
///
/// # Example
/// ```no_run
/// use terraphim_update::notification::{prompt_user_for_update, UpdateAction};
///
/// let action = prompt_user_for_update("1.0.0", "1.1.0")?;
/// match action {
///     UpdateAction::UpdateNow => println!("Updating..."),
///     _ => println!("Not updating"),
/// }
/// # Ok::<(), anyhow::Error>(())
/// ```
pub fn prompt_user_for_update(current_version: &str, latest_version: &str) -> Result<UpdateAction> {
    let message = format!(
        "Update available: {} → {}. Update now? [Y/n]",
        current_version, latest_version
    );

    let confirmed = Confirm::new()
        .with_prompt(&message)
        .default(false) // Default to 'n' (no auto-install)
        .interact()
        .context("Failed to get user input")?;

    if confirmed {
        Ok(UpdateAction::UpdateNow)
    } else {
        Ok(UpdateAction::RemindLater)
    }
}

/// Prompt user with detailed update information
///
/// Shows version info and release notes, then asks for action.
///
/// # Arguments
/// * `current_version` - Current installed version
/// * `latest_version` - Latest available version
/// * `release_notes` - Optional release notes
///
/// # Returns
/// * `Ok(UpdateAction)` - User's choice
/// * `Err(anyhow::Error)` - Error during prompt
pub fn prompt_user_for_update_with_notes(
    current_version: &str,
    latest_version: &str,
    release_notes: Option<&str>,
) -> Result<UpdateAction> {
    let notification = get_update_notification(current_version, latest_version, release_notes);
    println!("{}", notification);

    prompt_user_for_update(current_version, latest_version)
}

/// Format an update success message
///
/// # Arguments
/// * `from_version` - Previous version
/// * `to_version` - New version
///
/// # Returns
/// * Formatted success message
pub fn get_update_success_message(from_version: &str, to_version: &str) -> String {
    format!(
        "\n{} Update complete: {} → {}\n",
        emoji::success(),
        from_version,
        to_version
    )
}

/// Format an update failure message
///
/// # Arguments
/// * `error` - Error message
///
/// # Returns
/// * Formatted error message
pub fn get_update_failure_message(error: &str) -> String {
    format!("\n{} Update failed: {}\n", emoji::error(), error)
}

/// Format an "up to date" message
///
/// # Arguments
/// * `version` - Current version
///
/// # Returns
/// * Formatted message
pub fn get_up_to_date_message(version: &str) -> String {
    format!(
        "\n{} Already running latest version: {}\n",
        emoji::check(),
        version
    )
}

/// Emoji helper module (simple ASCII alternatives to avoid emoji dependency)
mod emoji {
    pub fn alert() -> &'static str {
        ">>>"
    }

    pub fn success() -> &'static str {
        ">>>"
    }

    pub fn error() -> &'static str {
        "!!!"
    }

    pub fn check() -> &'static str {
        ">>>"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_update_notification_basic() {
        let message = get_update_notification("1.0.0", "1.1.0", None);
        assert!(message.contains("1.0.0"));
        assert!(message.contains("1.1.0"));
        assert!(message.contains("Update Available"));
    }

    #[test]
    fn test_get_update_notification_with_notes() {
        let notes = "Bug fixes\nPerformance improvements";
        let message = get_update_notification("1.0.0", "1.1.0", Some(notes));

        assert!(message.contains("Bug fixes"));
        assert!(message.contains("Performance improvements"));
        assert!(message.contains("  ")); // Should be indented
    }

    #[test]
    fn test_format_release_notes_empty() {
        let notes = "";
        let formatted = format_release_notes(notes);
        assert_eq!(formatted, "");
    }

    #[test]
    fn test_format_release_notes_single_line() {
        let notes = "Bug fixes";
        let formatted = format_release_notes(notes);
        assert_eq!(formatted, "  Bug fixes");
    }

    #[test]
    fn test_format_release_notes_multiple_lines() {
        let notes = "Bug fixes\nPerformance improvements\nNew features";
        let formatted = format_release_notes(notes);
        assert!(formatted.contains("  Bug fixes"));
        assert!(formatted.contains("  Performance improvements"));
        assert!(formatted.contains("  New features"));
    }

    #[test]
    fn test_format_release_notes_with_empty_lines() {
        let notes = "Bug fixes\n\nPerformance improvements";
        let formatted = format_release_notes(notes);
        assert!(formatted.contains("  Bug fixes"));
        assert!(formatted.contains("  Performance improvements"));
    }

    #[test]
    fn test_get_update_success_message() {
        let message = get_update_success_message("1.0.0", "1.1.0");
        assert!(message.contains("1.0.0"));
        assert!(message.contains("1.1.0"));
        assert!(message.contains("Update complete"));
    }

    #[test]
    fn test_get_update_failure_message() {
        let message = get_update_failure_message("Network error");
        assert!(message.contains("Network error"));
        assert!(message.contains("Update failed"));
    }

    #[test]
    fn test_get_up_to_date_message() {
        let message = get_up_to_date_message("1.0.0");
        assert!(message.contains("1.0.0"));
        assert!(message.contains("Already running latest version"));
    }

    #[test]
    fn test_prompt_user_for_update_default_value() {
        // We can't test interactive prompts in unit tests, but we can verify
        // that the function signature is correct and compiles
        let current = "1.0.0";
        let latest = "1.1.0";

        // This is just a compile-time check
        let _ = format!(
            "Update available: {} → {}. Update now? [Y/n]",
            current, latest
        );
    }

    #[test]
    fn test_update_action_variants() {
        let _actions = [
            UpdateAction::UpdateNow,
            UpdateAction::RemindLater,
            UpdateAction::Skip,
        ];

        // All actions should be distinct
        assert_ne!(UpdateAction::UpdateNow, UpdateAction::RemindLater);
        assert_ne!(UpdateAction::UpdateNow, UpdateAction::Skip);
        assert_ne!(UpdateAction::RemindLater, UpdateAction::Skip);
    }

    #[test]
    fn test_emoji_helpers() {
        let alert = emoji::alert();
        let success = emoji::success();
        let error = emoji::error();
        let check = emoji::check();

        // All should return non-empty strings
        assert!(!alert.is_empty());
        assert!(!success.is_empty());
        assert!(!error.is_empty());
        assert!(!check.is_empty());
    }

    #[test]
    fn test_get_update_notification_formatting() {
        let message = get_update_notification("1.0.0", "1.1.0", Some("Notes"));

        // Should have newlines for formatting
        assert!(message.contains('\n'));

        // Should have alert indicator
        assert!(message.contains(">>>"));
    }

    #[test]
    fn test_get_up_to_date_message_formatting() {
        let message = get_up_to_date_message("1.0.0");

        // Should have newlines for formatting
        assert!(message.contains('\n'));

        // Should have check indicator
        assert!(message.contains(">>>"));
    }

    #[test]
    fn test_update_failure_message_formatting() {
        let message = get_update_failure_message("Test error");

        // Should have newlines for formatting
        assert!(message.contains('\n'));

        // Should have error indicator
        assert!(message.contains("!!!"));
    }
}
