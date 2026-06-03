//! Compile a repository `BUILD.md` into a [`ParsedWorkflow`].
//!
//! `BUILD.md` is the single command source shared with the interim ADF lane
//! (`build-runner-llm.sh`). Each non-comment command line inside a ```` ```bash ````
//! fence becomes a workflow step. This lets the native runner execute the exact
//! same commands as the interim lane, so cutover is a config flip.

use crate::{Result, RunnerError};
use terraphim_github_runner::{ParsedWorkflow, WorkflowStep};

/// Command prefixes considered build commands (mirrors build-runner-llm.sh:48).
const COMMAND_PREFIXES: &[&str] = &[
    "cargo", "make", "bun", "bunx", "npm", "yarn", "pnpm", "rch", "docker", "poetry", "uv", "go ",
    "zig ",
];

/// Parse the `BUILD.md` contents into a workflow of one step per command line.
pub fn parse_build_md(contents: &str) -> Result<ParsedWorkflow> {
    let mut steps = Vec::new();
    let mut in_bash = false;
    for line in contents.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("```bash") {
            in_bash = true;
            continue;
        }
        if trimmed.starts_with("```") {
            in_bash = false;
            continue;
        }
        if !in_bash {
            continue;
        }
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        if COMMAND_PREFIXES.iter().any(|p| trimmed.starts_with(p)) {
            steps.push(WorkflowStep {
                name: trimmed.to_string(),
                command: trimmed.to_string(),
                working_dir: "/workspace".to_string(),
                continue_on_error: false,
                timeout_seconds: 1800,
            });
        }
    }
    if steps.is_empty() {
        return Err(RunnerError::Compile(
            "no build commands found in BUILD.md bash blocks".to_string(),
        ));
    }
    Ok(ParsedWorkflow {
        name: "BUILD.md".to_string(),
        trigger: "gitea".to_string(),
        steps,
        ..ParsedWorkflow::default()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_commands_from_bash_block() {
        let md = "# BUILD\n\n```bash\ncargo fmt --all -- --check\n# a comment\ncargo build --workspace\n```\n\nprose after\n";
        let wf = parse_build_md(md).unwrap();
        assert_eq!(wf.steps.len(), 2);
        assert_eq!(wf.steps[0].command, "cargo fmt --all -- --check");
        assert_eq!(wf.steps[1].command, "cargo build --workspace");
    }

    #[test]
    fn errors_when_no_commands() {
        assert!(parse_build_md("# just prose, no fence\n").is_err());
    }
}
