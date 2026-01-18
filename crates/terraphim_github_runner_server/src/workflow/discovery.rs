use anyhow::Result;
use std::fs;
use std::path::{Path, PathBuf};
use tracing::{debug, info};

/// Discover workflow files that should be triggered by the given event
///
/// # Arguments
/// * `workflow_dir` - Path to .github/workflows directory
/// * `event_type` - Type of GitHub event (e.g., "pull_request", "push")
/// * `branch` - Branch name (for push events)
///
/// # Returns
/// * List of workflow file paths that should be executed
pub async fn discover_workflows_for_event(
    workflow_dir: &Path,
    event_type: &str,
    branch: Option<&str>,
) -> Result<Vec<PathBuf>> {
    let mut relevant_workflows = vec![];

    if !workflow_dir.exists() {
        info!("Workflow directory {:?} does not exist", workflow_dir);
        return Ok(relevant_workflows);
    }

    let entries = match fs::read_dir(workflow_dir) {
        Ok(entries) => entries,
        Err(e) => {
            info!(
                "Failed to read workflow directory {:?}: {}",
                workflow_dir, e
            );
            return Ok(relevant_workflows);
        }
    };

    for entry in entries {
        let entry = match entry {
            Ok(e) => e,
            Err(e) => {
                debug!("Failed to read directory entry: {}", e);
                continue;
            }
        };

        let path = entry.path();

        // Only process .yml and .yaml files
        if path.extension().and_then(|s| s.to_str()) != Some("yml")
            && path.extension().and_then(|s| s.to_str()) != Some("yaml")
        {
            continue;
        }

        debug!("Checking workflow file: {:?}", path);

        // Try to parse the workflow and check if it matches the event
        if let Ok(workflow_content) = fs::read_to_string(&path) {
            if matches_event(&workflow_content, event_type, branch) {
                info!(
                    "Workflow {:?} matches event type '{}'",
                    path.file_name(),
                    event_type
                );
                relevant_workflows.push(path);
            }
        }
    }

    Ok(relevant_workflows)
}

/// Check if a workflow file matches the given event
///
/// # Arguments
/// * `workflow_content` - YAML content of the workflow file
/// * `event_type` - Type of GitHub event
/// * `branch` - Branch name (for push events)
///
/// # Returns
/// * true if workflow should be triggered by this event
fn matches_event(workflow_content: &str, event_type: &str, branch: Option<&str>) -> bool {
    // Simple YAML parsing to check the 'on' trigger
    // For production, this should use a proper YAML parser

    let lines: Vec<&str> = workflow_content.lines().collect();

    // Find the 'on:' section
    let mut in_on_section = false;
    let mut has_pull_request = false;
    let mut has_push = false;
    let mut in_push_section = false;
    let mut push_branches: Vec<String> = vec![];

    for line in &lines {
        let trimmed = line.trim();

        // Check for 'on:' or 'on :' keyword
        if trimmed == "on:" || trimmed == "on :" {
            in_on_section = true;
            continue;
        }

        // Check if we've exited the 'on' section by checking if line is not indented
        // A line that's not empty and not starting with whitespace means we've exited
        if in_on_section && !line.is_empty() && !line.starts_with(' ') && !line.starts_with('\t') {
            in_on_section = false;
            in_push_section = false;
        }

        if in_on_section {
            // Check for pull_request trigger
            if trimmed.contains("pull_request") || trimmed.contains("pull_request:") {
                has_pull_request = true;
            }

            // Check for push trigger
            if trimmed.starts_with("push:") || trimmed.starts_with("push ") {
                has_push = true;
                in_push_section = true;
            }

            // If we're in a push section (or anywhere after "push:" was found),
            // look for branch arrays
            if in_push_section && trimmed.contains("branches:") {
                // Simple extraction of branch names from [main, develop] format
                if let Some(start) = trimmed.find('[') {
                    if let Some(end) = trimmed.find(']') {
                        let branches_str = &trimmed[start + 1..end];
                        for branch_name in branches_str.split(',') {
                            let branch = branch_name.trim().trim_matches('"').trim_matches('\'');
                            if !branch.is_empty() {
                                push_branches.push(branch.to_string());
                            }
                        }
                    }
                }
            }
        }
    }

    match event_type {
        "pull_request" => has_pull_request,
        "push" => {
            if !has_push {
                false
            } else if push_branches.is_empty() {
                // No branch filter, match all
                true
            } else if let Some(branch_name) = branch {
                // Check if the branch is in the allowed list
                push_branches.iter().any(|b| b == branch_name)
            } else {
                // Has branch filter but no branch provided, don't match
                false
            }
        }
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_matches_pull_request_event() {
        let workflow = r#"
on:
  pull_request:
    branches: [main, develop]
  push:
    branches: [main]

jobs:
  test:
    runs-on: ubuntu-latest
"#;

        // pull_request should match (we don't filter by branch in the parser)
        assert!(matches_event(workflow, "pull_request", None));
        // push should not match without a branch when branch filter exists
        assert!(!matches_event(workflow, "push", None));
        // push should match with the correct branch
        assert!(matches_event(workflow, "push", Some("main")));
    }

    #[test]
    fn test_matches_push_event() {
        let workflow = r#"
on:
  push:
    branches: [main, develop]
  pull_request:

jobs:
  test:
    runs-on: ubuntu-latest
"#;

        assert!(matches_event(workflow, "push", Some("main")));
        assert!(!matches_event(workflow, "push", Some("feature")));
    }

    #[test]
    fn test_no_matching_trigger() {
        let workflow = r#"
on:
  workflow_dispatch:

jobs:
  test:
    runs-on: ubuntu-latest
"#;

        assert!(!matches_event(workflow, "pull_request", None));
        assert!(!matches_event(workflow, "push", None));
    }
}
