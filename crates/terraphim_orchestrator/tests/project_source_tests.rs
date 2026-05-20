use std::fs;
use std::path::{Path, PathBuf};

use terraphim_orchestrator::config::OrchestratorConfig;

fn write_base_config(dir: &Path, body: &str) -> PathBuf {
    let path = dir.join("orchestrator.toml");
    fs::write(
        &path,
        format!(
            r#"
working_dir = "{root}/work"

[nightwatch]

[compound_review]
schedule = "0 2 * * *"
repo_path = "{root}/repo"

{body}
"#,
            root = dir.display(),
        ),
    )
    .unwrap();
    path
}

fn write_project_adf(root: &Path, body: &str) {
    fs::create_dir_all(root.join(".terraphim")).unwrap();
    fs::write(root.join(".terraphim/adf.toml"), body).unwrap();
}

#[test]
fn from_file_loads_enabled_project_source() {
    let tmp = tempfile::tempdir().unwrap();
    let project_root = tmp.path().join("alpha");
    write_project_adf(
        &project_root,
        r#"
project_id = "alpha"
name = "Alpha"

[[agents]]
name = "alpha-worker"
layer = "Core"
cli_tool = "codex"
task = "Implement alpha"
model = "kimi-for-coding/k2p6"

[[pr_dispatch]]
name = "alpha-worker"
context = "adf/build"
"#,
    );

    let base = write_base_config(
        tmp.path(),
        &format!(
            r#"
[[project_sources]]
id = "alpha"
root = "{}"
config = ".terraphim/adf.toml"
"#,
            project_root.display()
        ),
    );

    let config = OrchestratorConfig::from_file(&base).unwrap();
    assert_eq!(config.project_sources.len(), 1);
    assert_eq!(config.projects.len(), 1);
    assert_eq!(config.projects[0].id, "alpha");
    assert_eq!(config.projects[0].working_dir, project_root);
    assert_eq!(config.agents.len(), 1);
    assert_eq!(config.agents[0].project.as_deref(), Some("alpha"));
    assert_eq!(config.agents[0].name, "alpha-worker");
    assert_eq!(
        config.agents_on_pr_open_for_project("alpha")[0].name,
        "alpha-worker"
    );
    config.validate().unwrap();
}

#[test]
fn disabled_project_source_is_not_merged() {
    let tmp = tempfile::tempdir().unwrap();
    let project_root = tmp.path().join("disabled");
    write_project_adf(
        &project_root,
        r#"
project_id = "disabled"
name = "Disabled"
"#,
    );

    let base = write_base_config(
        tmp.path(),
        &format!(
            r#"
[[project_sources]]
id = "disabled"
root = "{}"
config = ".terraphim/adf.toml"
enabled = false
"#,
            project_root.display()
        ),
    );

    let config = OrchestratorConfig::from_file(&base).unwrap();
    assert_eq!(config.project_sources.len(), 1);
    assert!(config.projects.is_empty());
    assert!(config.agents.is_empty());
    config.validate().unwrap();
}

#[test]
fn malformed_enabled_project_source_reports_id_and_path() {
    let tmp = tempfile::tempdir().unwrap();
    let project_root = tmp.path().join("bad");
    fs::create_dir_all(project_root.join(".terraphim")).unwrap();
    fs::write(project_root.join(".terraphim/adf.toml"), "not valid {{{").unwrap();

    let base = write_base_config(
        tmp.path(),
        &format!(
            r#"
[[project_sources]]
id = "bad"
root = "{}"
config = ".terraphim/adf.toml"
"#,
            project_root.display()
        ),
    );

    let err = OrchestratorConfig::from_file(&base)
        .unwrap_err()
        .to_string();
    assert!(err.contains("bad"));
    assert!(err.contains(".terraphim/adf.toml"));
    assert!(err.contains("failed to parse"));
}

#[test]
fn project_source_id_must_match_project_local_id() {
    let tmp = tempfile::tempdir().unwrap();
    let project_root = tmp.path().join("alpha");
    write_project_adf(
        &project_root,
        r#"
project_id = "beta"
name = "Beta"
"#,
    );

    let base = write_base_config(
        tmp.path(),
        &format!(
            r#"
[[project_sources]]
id = "alpha"
root = "{}"
config = ".terraphim/adf.toml"
"#,
            project_root.display()
        ),
    );

    let err = OrchestratorConfig::from_file(&base)
        .unwrap_err()
        .to_string();
    assert!(err.contains("project source 'alpha'"));
    assert!(err.contains("project_id 'beta'"));
}

#[test]
fn duplicate_project_source_ids_fail() {
    let tmp = tempfile::tempdir().unwrap();
    let root_a = tmp.path().join("a");
    let root_b = tmp.path().join("b");
    write_project_adf(&root_a, "project_id = \"alpha\"\nname = \"Alpha\"\n");
    write_project_adf(&root_b, "project_id = \"alpha\"\nname = \"Alpha 2\"\n");

    let base = write_base_config(
        tmp.path(),
        &format!(
            r#"
[[project_sources]]
id = "alpha"
root = "{}"
config = ".terraphim/adf.toml"

[[project_sources]]
id = "alpha"
root = "{}"
config = ".terraphim/adf.toml"
"#,
            root_a.display(),
            root_b.display()
        ),
    );

    let err = OrchestratorConfig::from_file(&base)
        .unwrap_err()
        .to_string();
    assert!(err.contains("duplicate project_sources id 'alpha'"));
}

#[test]
fn duplicate_agent_names_are_scoped_by_project() {
    let tmp = tempfile::tempdir().unwrap();
    let alpha = tmp.path().join("alpha");
    let beta = tmp.path().join("beta");
    let adf = |id: &str| {
        format!(
            r#"
project_id = "{id}"
name = "{id}"

[[agents]]
name = "worker"
layer = "Safety"
cli_tool = "echo"
task = "work"
"#
        )
    };
    write_project_adf(&alpha, &adf("alpha"));
    write_project_adf(&beta, &adf("beta"));

    let base = write_base_config(
        tmp.path(),
        &format!(
            r#"
[[project_sources]]
id = "alpha"
root = "{}"
config = ".terraphim/adf.toml"

[[project_sources]]
id = "beta"
root = "{}"
config = ".terraphim/adf.toml"
"#,
            alpha.display(),
            beta.display()
        ),
    );

    let config = OrchestratorConfig::from_file(&base).unwrap();
    assert_eq!(config.agents.len(), 2);
    config.validate().unwrap();
}

#[test]
fn duplicate_agent_names_within_project_fail_validation() {
    let tmp = tempfile::tempdir().unwrap();
    let project_root = tmp.path().join("alpha");
    write_project_adf(
        &project_root,
        r#"
project_id = "alpha"
name = "Alpha"

[[agents]]
name = "worker"
layer = "Safety"
cli_tool = "echo"
task = "one"

[[agents]]
name = "worker"
layer = "Safety"
cli_tool = "echo"
task = "two"
"#,
    );

    let base = write_base_config(
        tmp.path(),
        &format!(
            r#"
[[project_sources]]
id = "alpha"
root = "{}"
config = ".terraphim/adf.toml"
"#,
            project_root.display()
        ),
    );

    let config = OrchestratorConfig::from_file(&base).unwrap();
    let err = config.validate().unwrap_err().to_string();
    assert!(err.contains("duplicate agent 'worker' in project 'alpha'"));
}

#[test]
fn relative_project_source_root_is_resolved_from_base_config_dir() {
    let tmp = tempfile::tempdir().unwrap();
    let project_root = tmp.path().join("relative-alpha");
    write_project_adf(
        &project_root,
        r#"
project_id = "relative-alpha"
name = "Relative Alpha"
"#,
    );

    let base = write_base_config(
        tmp.path(),
        r#"
[[project_sources]]
id = "relative-alpha"
root = "relative-alpha"
config = ".terraphim/adf.toml"
"#,
    );

    let config = OrchestratorConfig::from_file(&base).unwrap();
    assert_eq!(config.projects.len(), 1);
    assert_eq!(config.projects[0].working_dir, project_root);
    config.validate().unwrap();
}
