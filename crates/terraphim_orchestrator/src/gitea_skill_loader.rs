//! Fetches SKILL.md files from a Gitea repository and caches them to disk.
//!
//! This module implements Phase 1 of GitOps-style agent configuration loading
//! (issue #1434). Skills are fetched at orchestrator startup via the Gitea
//! raw-content API and written to a local cache directory. The cache is then
//! used as the first skill root, so remote skill definitions take priority over
//! local ones. On any network or curl failure the local directories are used
//! as fallback (offline resilience, constraint C2 from the research document).
//!
//! Security invariant: only read-only SKILL.md content is loaded remotely.
//! Gate rules and agent definitions remain in local TOML (ADR-006).

use std::path::PathBuf;

use tracing::{info, warn};

use crate::config::GiteaSkillRepoConfig;

/// Fetch all configured skills from Gitea and write them to the cache directory.
///
/// Downloads `<skill_name>/SKILL.md` for each skill in `config.skills` via
/// `curl`. Existing cache files are skipped unless `force` is `true`.
/// All errors are logged as warnings; the function always returns the cache
/// directory path so callers can use it as a skill root regardless of partial
/// fetch failures.
pub async fn populate_skill_cache(config: &GiteaSkillRepoConfig, force: bool) -> PathBuf {
    let cache_dir = &config.cache_dir;

    if config.skills.is_empty() {
        info!("gitea_skill_repo.skills is empty — no remote skills to fetch");
        return cache_dir.clone();
    }

    if let Err(e) = tokio::fs::create_dir_all(cache_dir).await {
        warn!(
            dir = %cache_dir.display(),
            error = %e,
            "failed to create skill cache directory, using local fallback"
        );
        return cache_dir.clone();
    }

    let token = config
        .token
        .clone()
        .or_else(|| std::env::var("GITEA_TOKEN").ok());

    let mut fetched = 0usize;
    let mut skipped = 0usize;
    let mut failed = 0usize;

    for skill_name in &config.skills {
        let skill_dir = cache_dir.join(skill_name);
        let dest_path = skill_dir.join("SKILL.md");

        if dest_path.exists() && !force {
            skipped += 1;
            continue;
        }

        if let Err(e) = tokio::fs::create_dir_all(&skill_dir).await {
            warn!(skill = %skill_name, error = %e, "failed to create skill cache subdir");
            failed += 1;
            continue;
        }

        // Gitea API raw content endpoint: accepts branch names, tags, AND commit SHAs.
        // The web-UI path (/owner/repo/raw/branch/<name>) only resolves branch names;
        // SHA refs produce 404. The API endpoint (/api/v1/repos/.../raw/...?ref=<ref>)
        // accepts any ref type, which is required for pinned-SHA production deployments.
        let url = format!(
            "{}/api/v1/repos/{}/{}/raw/{}/SKILL.md?ref={}",
            config.url.trim_end_matches('/'),
            config.owner,
            config.repo,
            skill_name,
            config.git_ref,
        );

        let mut cmd = tokio::process::Command::new("curl");
        cmd.arg("--silent")
            .arg("--fail")
            .arg("--max-time")
            .arg(config.fetch_timeout_secs.to_string())
            .arg("--header")
            .arg("Accept: application/octet-stream")
            .arg("--output")
            .arg(&dest_path);

        if let Some(ref tok) = token {
            cmd.arg("--header")
                .arg(format!("Authorization: token {tok}"));
        }
        cmd.arg(&url);

        match cmd.status().await {
            Ok(status) if status.success() => {
                info!(
                    skill = %skill_name,
                    "fetched and cached skill from Gitea"
                );
                fetched += 1;
            }
            Ok(status) => {
                warn!(
                    skill = %skill_name,
                    url = %url,
                    exit_code = ?status.code(),
                    "curl returned non-zero for skill (HTTP 404 or auth error), using local fallback"
                );
                // Remove partial download
                let _ = tokio::fs::remove_file(&dest_path).await;
                failed += 1;
            }
            Err(e) => {
                warn!(
                    skill = %skill_name,
                    error = %e,
                    "failed to spawn curl for skill fetch, using local fallback"
                );
                failed += 1;
            }
        }
    }

    info!(
        cache_dir = %cache_dir.display(),
        fetched,
        skipped,
        failed,
        total = config.skills.len(),
        "Gitea skill cache population complete"
    );

    cache_dir.clone()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::GiteaSkillRepoConfig;
    use std::path::PathBuf;

    fn make_config(cache_dir: PathBuf, skills: Vec<&str>) -> GiteaSkillRepoConfig {
        GiteaSkillRepoConfig {
            url: "https://invalid.local".to_string(),
            owner: "test".to_string(),
            repo: "skills".to_string(),
            git_ref: "main".to_string(),
            cache_dir,
            token: None,
            fetch_timeout_secs: 1,
            skills: skills.into_iter().map(String::from).collect(),
        }
    }

    #[tokio::test]
    async fn empty_skills_returns_cache_dir() {
        let tmp = tempfile::tempdir().unwrap();
        let config = make_config(tmp.path().to_path_buf(), vec![]);
        let result = populate_skill_cache(&config, false).await;
        assert_eq!(result, tmp.path());
    }

    #[tokio::test]
    async fn unreachable_server_returns_cache_dir_gracefully() {
        let tmp = tempfile::tempdir().unwrap();
        let config = make_config(tmp.path().to_path_buf(), vec!["disciplined-research"]);
        // curl will fail because invalid.local doesn't resolve; function must not panic
        let result = populate_skill_cache(&config, false).await;
        assert_eq!(result, tmp.path());
    }

    #[tokio::test]
    async fn cached_file_skipped_when_not_forced() {
        let tmp = tempfile::tempdir().unwrap();
        let skill_dir = tmp.path().join("my-skill");
        tokio::fs::create_dir_all(&skill_dir).await.unwrap();
        let skill_file = skill_dir.join("SKILL.md");
        tokio::fs::write(&skill_file, b"cached content")
            .await
            .unwrap();

        let config = make_config(tmp.path().to_path_buf(), vec!["my-skill"]);
        // force=false should skip existing file without hitting network
        let result = populate_skill_cache(&config, false).await;
        assert_eq!(result, tmp.path());

        // File must still contain original content (not overwritten)
        let contents = tokio::fs::read_to_string(&skill_file).await.unwrap();
        assert_eq!(contents, "cached content");
    }
}
