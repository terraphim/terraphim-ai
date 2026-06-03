use std::path::{Path, PathBuf};

use terraphim_spawner::SpawnContext;

/// Configuration for local skill discovery in a project directory.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalSkillConfig {
    /// Path to the `.terraphim/skills` directory containing skill definitions.
    pub skills_dir: PathBuf,
}

/// CLI tools that natively support loading local skills.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum SupportedSkillCli {
    /// The `opencode` CLI tool.
    Opencode,
    /// The `claude` or `claude-code` CLI tool.
    Claude,
}

/// Discover a local skill configuration in the given project root, if present.
pub fn discover_local_skills(project_root: &Path) -> Option<LocalSkillConfig> {
    let skills_dir = project_root.join(".terraphim/skills");
    skills_dir
        .is_dir()
        .then_some(LocalSkillConfig { skills_dir })
}

/// Detect which supported skill CLI (if any) matches the given tool name.
pub fn detect_skill_cli(cli_tool: &str) -> Option<SupportedSkillCli> {
    match cli_name(cli_tool) {
        "opencode" => Some(SupportedSkillCli::Opencode),
        "claude" | "claude-code" => Some(SupportedSkillCli::Claude),
        _ => None,
    }
}

/// Augment a spawn context with local skill loading directives for the given CLI.
pub fn prepare_local_skill_loading(
    cli_tool: &str,
    project_root: &Path,
    ctx: SpawnContext,
) -> SpawnContext {
    let Some(skills) = discover_local_skills(project_root) else {
        return ctx;
    };
    let Some(cli) = detect_skill_cli(cli_tool) else {
        return ctx.with_env(
            "TERRAPHIM_LOCAL_SKILLS_DIR",
            skills.skills_dir.to_string_lossy().into_owned(),
        );
    };

    if let Err(err) = ensure_native_skill_bridge(cli, project_root, &skills.skills_dir) {
        tracing::warn!(
            cli = ?cli,
            skills_dir = %skills.skills_dir.display(),
            error = %err,
            "failed to prepare native local skill bridge"
        );
    }

    ctx.with_env(
        "TERRAPHIM_LOCAL_SKILLS_DIR",
        skills.skills_dir.to_string_lossy().into_owned(),
    )
}

fn cli_name(cli_tool: &str) -> &str {
    Path::new(cli_tool)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or(cli_tool)
}

fn native_skill_dir(cli: SupportedSkillCli, project_root: &Path) -> PathBuf {
    match cli {
        SupportedSkillCli::Opencode => project_root.join(".opencode/skill"),
        SupportedSkillCli::Claude => project_root.join(".claude/skills"),
    }
}

fn ensure_native_skill_bridge(
    cli: SupportedSkillCli,
    project_root: &Path,
    skills_dir: &Path,
) -> std::io::Result<()> {
    let native_dir = native_skill_dir(cli, project_root);
    if native_dir.exists() {
        return Ok(());
    }

    if let Some(parent) = native_dir.parent() {
        std::fs::create_dir_all(parent)?;
    }

    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(skills_dir, native_dir)?;
    }

    #[cfg(windows)]
    {
        std::os::windows::fs::symlink_dir(skills_dir, native_dir)?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn project_with_skills() -> TempDir {
        let tmp = TempDir::new().unwrap();
        let skill_dir = tmp.path().join(".terraphim/skills/test-skill");
        std::fs::create_dir_all(&skill_dir).unwrap();
        std::fs::write(skill_dir.join("SKILL.md"), "# Test skill\n").unwrap();
        tmp
    }

    #[test]
    fn discover_local_skills_returns_none_when_missing() {
        let tmp = TempDir::new().unwrap();
        assert_eq!(discover_local_skills(tmp.path()), None);
    }

    #[test]
    fn discover_local_skills_finds_project_skills_dir() {
        let tmp = project_with_skills();
        let skills = discover_local_skills(tmp.path()).expect("skills directory should exist");
        assert_eq!(skills.skills_dir, tmp.path().join(".terraphim/skills"));
    }

    #[test]
    fn detect_skill_cli_handles_supported_names_and_paths() {
        assert_eq!(
            detect_skill_cli("opencode"),
            Some(SupportedSkillCli::Opencode)
        );
        assert_eq!(
            detect_skill_cli("/usr/local/bin/opencode"),
            Some(SupportedSkillCli::Opencode)
        );
        assert_eq!(detect_skill_cli("claude"), Some(SupportedSkillCli::Claude));
        assert_eq!(
            detect_skill_cli("claude-code"),
            Some(SupportedSkillCli::Claude)
        );
        assert_eq!(detect_skill_cli("/bin/echo"), None);
    }

    #[test]
    fn prepare_local_skill_loading_is_noop_when_skills_missing() {
        let tmp = TempDir::new().unwrap();
        let ctx = prepare_local_skill_loading("opencode", tmp.path(), SpawnContext::global());
        assert!(ctx.env_overrides.is_empty());
        assert!(!tmp.path().join(".opencode/skill").exists());
    }

    #[test]
    fn prepare_local_skill_loading_bridges_opencode_project_skills() {
        let tmp = project_with_skills();
        let ctx = prepare_local_skill_loading("opencode", tmp.path(), SpawnContext::global());
        assert_eq!(
            ctx.env_overrides.get("TERRAPHIM_LOCAL_SKILLS_DIR"),
            Some(
                &tmp.path()
                    .join(".terraphim/skills")
                    .to_string_lossy()
                    .into_owned()
            )
        );
        assert!(tmp.path().join(".opencode/skill").exists());
    }

    #[test]
    fn prepare_local_skill_loading_bridges_claude_project_skills() {
        let tmp = project_with_skills();
        let ctx = prepare_local_skill_loading("claude", tmp.path(), SpawnContext::global());
        assert_eq!(
            ctx.env_overrides.get("TERRAPHIM_LOCAL_SKILLS_DIR"),
            Some(
                &tmp.path()
                    .join(".terraphim/skills")
                    .to_string_lossy()
                    .into_owned()
            )
        );
        assert!(tmp.path().join(".claude/skills").exists());
    }

    #[test]
    fn prepare_local_skill_loading_does_not_overwrite_existing_native_dir() {
        let tmp = project_with_skills();
        let existing = tmp.path().join(".opencode/skill/existing");
        std::fs::create_dir_all(&existing).unwrap();

        let _ = prepare_local_skill_loading("opencode", tmp.path(), SpawnContext::global());

        assert!(existing.is_dir());
    }

    #[test]
    fn unsupported_cli_exports_terraphim_skill_dir_without_native_bridge() {
        let tmp = project_with_skills();
        let ctx = prepare_local_skill_loading("/bin/echo", tmp.path(), SpawnContext::global());
        assert_eq!(
            ctx.env_overrides.get("TERRAPHIM_LOCAL_SKILLS_DIR"),
            Some(
                &tmp.path()
                    .join(".terraphim/skills")
                    .to_string_lossy()
                    .into_owned()
            )
        );
        assert!(!tmp.path().join(".opencode/skill").exists());
        assert!(!tmp.path().join(".claude/skills").exists());
    }
}
