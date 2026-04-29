//! Integration tests for user-prompt-submit hook patterns.
//!
//! Tests that `terraphim-agent learn hook --learn-hook-type user-prompt-submit`
//! correctly captures tool preference corrections from user prompts and writes
//! `CorrectionType::ToolPreference` files.

use std::path::Path;
use std::process::{Command, Stdio};

fn agent_binary() -> String {
    let output = Command::new("cargo")
        .args(["build", "-p", "terraphim_agent"])
        .output()
        .expect("cargo build should succeed");
    if !output.status.success() {
        panic!(
            "cargo build failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    let workspace_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap();
    workspace_root
        .join("target/debug/terraphim-agent")
        .to_string_lossy()
        .to_string()
}

/// Run the user-prompt-submit hook with a JSON payload, returning whether it succeeded.
fn run_user_prompt_submit(binary: &str, prompt: &str, env_home: &str) -> bool {
    let json = format!(r#"{{"user_prompt":"{}"}}"#, prompt);
    let output = Command::new(binary)
        .args(["learn", "hook", "--learn-hook-type", "user-prompt-submit"])
        .env("HOME", env_home)
        .env("XDG_DATA_HOME", format!("{}/data", env_home))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("should spawn hook process")
        .communicate(json.into_bytes())
        .expect("should communicate with hook process");

    output.status.success()
}

trait Communicate {
    fn communicate(self, stdin: Vec<u8>) -> std::io::Result<std::process::Output>;
}

impl Communicate for std::process::Child {
    fn communicate(mut self, stdin: Vec<u8>) -> std::io::Result<std::process::Output> {
        use std::io::Write;
        if let Some(mut child_stdin) = self.stdin.take() {
            child_stdin.write_all(&stdin)?;
        }
        self.wait_with_output()
    }
}

/// Return all correction markdown files in the learnings directory.
fn find_correction_files(home: &str) -> Vec<std::path::PathBuf> {
    let learnings_dir = Path::new(home)
        .join("data")
        .join("terraphim")
        .join("learnings");
    if !learnings_dir.exists() {
        return vec![];
    }
    std::fs::read_dir(&learnings_dir)
        .expect("should read learnings dir")
        .filter_map(|entry| entry.ok().map(|e| e.path()))
        .filter(|path| {
            path.file_name()
                .and_then(|n| n.to_str())
                .map_or(false, |name| {
                    name.starts_with("correction-") && name.ends_with(".md")
                })
        })
        .collect()
}

/// Clear all correction files from a previous test run.
fn clear_correction_files(home: &str) {
    for path in find_correction_files(home) {
        let _ = std::fs::remove_file(path);
    }
}

#[test]
fn user_prompt_submit_use_instead_of_creates_tool_preference() {
    let binary = agent_binary();
    let tmp = tempfile::tempdir().expect("create temp dir");
    let home = tmp.path().to_string_lossy().to_string();

    clear_correction_files(&home);
    let success = run_user_prompt_submit(&binary, "use uv instead of pip", &home);
    assert!(success, "hook should exit 0");

    let files = find_correction_files(&home);
    assert_eq!(
        files.len(),
        1,
        "expected exactly one correction file, found: {:?}",
        files
    );

    let content = std::fs::read_to_string(&files[0]).expect("should read correction file");
    assert!(
        content.contains("tool-preference"),
        "correction should be ToolPreference, got:\n{}",
        content
    );
    assert!(
        content.contains("uv"),
        "correction should contain corrected tool 'uv', got:\n{}",
        content
    );
    assert!(
        content.contains("pip"),
        "correction should contain original tool 'pip', got:\n{}",
        content
    );
}

#[test]
fn user_prompt_submit_use_not_creates_tool_preference() {
    let binary = agent_binary();
    let tmp = tempfile::tempdir().expect("create temp dir");
    let home = tmp.path().to_string_lossy().to_string();

    clear_correction_files(&home);
    let success = run_user_prompt_submit(&binary, "use cargo not make", &home);
    assert!(success, "hook should exit 0");

    let files = find_correction_files(&home);
    assert_eq!(
        files.len(),
        1,
        "expected exactly one correction file, found: {:?}",
        files
    );

    let content = std::fs::read_to_string(&files[0]).expect("should read correction file");
    assert!(
        content.contains("tool-preference"),
        "correction should be ToolPreference, got:\n{}",
        content
    );
    assert!(
        content.contains("cargo"),
        "correction should contain corrected tool 'cargo', got:\n{}",
        content
    );
    assert!(
        content.contains("make"),
        "correction should contain original tool 'make', got:\n{}",
        content
    );
}

#[test]
fn user_prompt_submit_prefer_over_creates_tool_preference() {
    let binary = agent_binary();
    let tmp = tempfile::tempdir().expect("create temp dir");
    let home = tmp.path().to_string_lossy().to_string();

    clear_correction_files(&home);
    let success = run_user_prompt_submit(&binary, "prefer bunx over npx", &home);
    assert!(success, "hook should exit 0");

    let files = find_correction_files(&home);
    assert_eq!(
        files.len(),
        1,
        "expected exactly one correction file, found: {:?}",
        files
    );

    let content = std::fs::read_to_string(&files[0]).expect("should read correction file");
    assert!(
        content.contains("tool-preference"),
        "correction should be ToolPreference, got:\n{}",
        content
    );
    assert!(
        content.contains("bunx"),
        "correction should contain corrected tool 'bunx', got:\n{}",
        content
    );
    assert!(
        content.contains("npx"),
        "correction should contain original tool 'npx', got:\n{}",
        content
    );
}

#[test]
fn user_prompt_submit_personal_preference_does_not_capture() {
    let binary = agent_binary();
    let tmp = tempfile::tempdir().expect("create temp dir");
    let home = tmp.path().to_string_lossy().to_string();

    clear_correction_files(&home);
    let success = run_user_prompt_submit(&binary, "I prefer tea over coffee", &home);
    assert!(success, "hook should exit 0 (fail-open)");

    let files = find_correction_files(&home);
    assert!(
        files.is_empty(),
        "personal preference should NOT create a correction file, found: {:?}",
        files
    );
}
