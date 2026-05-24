use std::path::PathBuf;
use std::process::Command;

fn adf_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_adf"))
}

fn write_config(
    dir: &tempfile::TempDir,
    project_dir: &std::path::Path,
) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let config_path = dir.path().join("orchestrator.toml");
    std::fs::write(
        &config_path,
        format!(
            r#"
working_dir = "{}"

[nightwatch]

[compound_review]
schedule = "0 2 * * *"
repo_path = "{}"

[evolution]
enabled = true

[[projects]]
id = "terraphim"
working_dir = "{}"

[[agents]]
name = "builder"
layer = "Core"
cli_tool = "echo"
task = "Build"
model = "minimax-coding-plan/MiniMax-M2.7-highspeed"
project = "terraphim"
evolution_enabled = true
gitea_issue = 42
"#,
            dir.path().display(),
            project_dir.display(),
            project_dir.display()
        ),
    )?;
    Ok(config_path)
}

#[test]
fn adf_agent_validate_json_reports_project_runtime() -> Result<(), Box<dyn std::error::Error>> {
    let dir = tempfile::tempdir_in("/tmp")?;
    let project_dir = dir.path().join("project");
    std::fs::create_dir_all(&project_dir)?;
    let config_path = write_config(&dir, &project_dir)?;

    let out = Command::new(adf_bin())
        .args([
            "agent",
            "validate",
            "--config",
            &config_path.display().to_string(),
            "--project",
            "terraphim",
            "--format",
            "json",
            "builder",
        ])
        .output()?;

    assert!(out.status.success(), "expected success, got {out:?}");
    let json: serde_json::Value = serde_json::from_slice(&out.stdout)?;
    assert_eq!(json["agent_name"], "builder");
    assert_eq!(json["project"], "terraphim");
    assert_eq!(json["cli_tool"], "echo");
    assert_eq!(json["model"], "minimax-coding-plan/MiniMax-M2.7-highspeed");
    assert_eq!(json["repo_ok"], true);
    assert_eq!(json["runnable"], true);
    assert_eq!(json["evolution_available"], true);
    Ok(())
}

#[test]
fn adf_agent_validate_missing_agent_fails() -> Result<(), Box<dyn std::error::Error>> {
    let dir = tempfile::tempdir_in("/tmp")?;
    let project_dir = dir.path().join("project");
    std::fs::create_dir_all(&project_dir)?;
    let config_path = write_config(&dir, &project_dir)?;

    let out = Command::new(adf_bin())
        .args([
            "agent",
            "validate",
            "--config",
            &config_path.display().to_string(),
            "missing",
        ])
        .output()?;

    assert!(!out.status.success(), "expected failure, got {out:?}");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("agent 'missing' not found"),
        "stderr: {stderr}"
    );
    Ok(())
}
