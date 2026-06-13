//! Repository checkout for the native runner.
//!
//! Before a task's compiled workflow runs, the target repository must be present
//! on disk at the task's commit -- otherwise build commands execute against an
//! empty (or unrelated) working directory. This module ensures a git working
//! tree exists at `<checkout_root>/<owner>/<repo>`, checked out at the requested
//! `sha`, reusing the on-disk clone across tasks for the same repository.
//!
//! ## Authentication
//! The per-job repository token is injected via `GIT_CONFIG_COUNT` /
//! `GIT_CONFIG_KEY_0` / `GIT_CONFIG_VALUE_0` environment variables rather than
//! a `-c http.extraHeader=...` command-line argument. Environment variables are
//! not visible in `/proc/PID/cmdline` or `ps` output, so the token cannot be
//! read by other local users on the runner host. Gitea's git-over-HTTP backend
//! expects HTTP Basic auth (not the `Authorization: token` API scheme), so we
//! mirror `actions/checkout`: synthetic user `x-access-token`, token as the
//! password. This keeps the token out of `.git/config`, out of any remote URL,
//! and out of process arguments.

use crate::{Result, RunnerError};
use base64::Engine;
use std::path::{Component, Path, PathBuf};
use std::process::Stdio;
use tokio::process::Command;

/// Outcome of a single git invocation: exit status plus captured stderr.
struct GitOutput {
    success: bool,
    stderr: String,
}

/// Build a `git` command rooted at `dir` with a clean, non-interactive
/// environment. The token (when present) is injected via `GIT_CONFIG_COUNT`
/// environment variables so it never appears in process arguments, `.git/config`,
/// or any remote URL.
fn git_command(dir: Option<&Path>, token: Option<&str>) -> Command {
    let mut cmd = Command::new("git");
    // Never prompt for credentials interactively; a missing/invalid credential
    // must fail fast rather than hang the runner.
    cmd.env("GIT_TERMINAL_PROMPT", "0");
    cmd.env("GIT_ASKPASS", "");
    cmd.env("GCM_INTERACTIVE", "never");
    if let Some(dir) = dir {
        cmd.arg("-C").arg(dir);
    }
    if let Some(token) = token {
        // Inject auth via git's env-var config protocol instead of a `-c` argument.
        // Environment variables are not visible in /proc/PID/cmdline or `ps`,
        // so the token cannot be read by other local users on the runner host.
        // Gitea smart-HTTP requires Basic (not `Authorization: token`); the
        // synthetic `x-access-token` user matches actions/checkout.
        let basic =
            base64::engine::general_purpose::STANDARD.encode(format!("x-access-token:{token}"));
        cmd.env("GIT_CONFIG_COUNT", "1");
        cmd.env("GIT_CONFIG_KEY_0", "http.extraHeader");
        cmd.env(
            "GIT_CONFIG_VALUE_0",
            format!("Authorization: Basic {basic}"),
        );
    }
    cmd.stdin(Stdio::null());
    cmd
}

/// Run a prepared git command, capturing its outcome. `what` names the operation
/// for error context (never includes the token-bearing argument).
async fn run_git(mut cmd: Command, what: &str) -> Result<GitOutput> {
    let output = cmd
        .output()
        .await
        .map_err(|e| RunnerError::Execution(format!("git {what}: spawn failed: {e}")))?;
    Ok(GitOutput {
        success: output.status.success(),
        stderr: String::from_utf8_lossy(&output.stderr).trim().to_string(),
    })
}

/// Construct the clone URL for `owner/repo` against `instance_url`.
///
/// `instance_url` is normally an HTTPS Gitea base (e.g. `https://git.example`),
/// but a filesystem path (or `file://` URL) is accepted so a local bare/working
/// repository can stand in as the remote for tests without any network.
fn clone_url(instance_url: &str, owner: &str, repo: &str) -> String {
    let base = instance_url.trim_end_matches('/');
    // Local paths (used by tests) point straight at the on-disk repository and
    // must not gain a `.git` suffix; only remote http(s) URLs follow the
    // `<base>/<owner>/<repo>.git` convention.
    if base.starts_with("http://") || base.starts_with("https://") {
        format!("{base}/{owner}/{repo}.git")
    } else {
        format!("{base}/{owner}/{repo}")
    }
}

/// Validate that a path component (`owner` or `repo`) contains only plain
/// directory names — no `..`, no absolute paths, no empty strings.
///
/// Gitea repository names are typically simple identifiers, but a compromised
/// or forged task payload could supply `"../../sensitive"` as the `repo` field.
/// Rejecting anything other than [`Component::Normal`] at each segment prevents
/// the checkout path from escaping `checkout_root`.
fn validate_path_component(label: &str, value: &str) -> Result<()> {
    if value.is_empty() {
        return Err(RunnerError::Execution(format!(
            "invalid {label}: must not be empty"
        )));
    }
    for component in Path::new(value).components() {
        match component {
            Component::Normal(_) => {}
            other => {
                return Err(RunnerError::Execution(format!(
                    "invalid {label} `{value}`: contains disallowed path component {other:?}"
                )));
            }
        }
    }
    Ok(())
}

/// Ensure a working tree for `owner/repo` exists at `sha` under `checkout_root`,
/// and return the resolved target directory.
///
/// Algorithm:
/// 1. target = `<checkout_root>/<owner>/<repo>` (parents created).
/// 2. if `<target>/.git` is absent: `git init` and add the `origin` remote.
/// 3. shallow-fetch the requested `sha`; on failure (servers may reject
///    want-sha), fall back to a full `fetch origin` and log a warning.
/// 4. `git checkout --force --detach <sha>`.
///
/// All git invocations inherit a clean, non-interactive environment; the auth
/// token is passed only via `http.extraHeader` and never persisted.
pub async fn ensure_checkout(
    instance_url: &str,
    owner: &str,
    repo: &str,
    sha: &str,
    token: Option<&str>,
    checkout_root: &Path,
) -> Result<PathBuf> {
    // Guard against path-traversal in owner/repo before any filesystem access.
    // A forged payload such as `repository: "../../sensitive"` would otherwise
    // escape checkout_root; only plain directory name segments are permitted.
    validate_path_component("owner", owner)?;
    validate_path_component("repo", repo)?;

    let target = checkout_root.join(owner).join(repo);

    // Defence-in-depth: even after per-component validation, verify the resolved
    // path is still under checkout_root (guards against edge cases on exotic OSes
    // or future refactors that alter how the path is constructed).
    if !target.starts_with(checkout_root) {
        return Err(RunnerError::Execution(format!(
            "checkout path `{}` escapes root `{}`",
            target.display(),
            checkout_root.display()
        )));
    }

    if let Some(parent) = target.parent() {
        tokio::fs::create_dir_all(parent).await.map_err(|e| {
            RunnerError::Execution(format!("create checkout parent {}: {e}", parent.display()))
        })?;
    }

    let url = clone_url(instance_url, owner, repo);
    let git_dir = target.join(".git");
    let first_clone = !git_dir.exists();

    if first_clone {
        tokio::fs::create_dir_all(&target).await.map_err(|e| {
            RunnerError::Execution(format!("create checkout dir {}: {e}", target.display()))
        })?;

        let mut init = git_command(None, None);
        init.arg("init").arg("-q").arg(&target);
        let out = run_git(init, "init").await?;
        if !out.success {
            return Err(RunnerError::Execution(format!(
                "git init {} failed: {}",
                target.display(),
                out.stderr
            )));
        }

        let mut remote = git_command(Some(&target), None);
        remote.arg("remote").arg("add").arg("origin").arg(&url);
        let out = run_git(remote, "remote add origin").await?;
        if !out.success {
            return Err(RunnerError::Execution(format!(
                "git remote add origin failed: {}",
                out.stderr
            )));
        }
    }

    // Try a shallow, pinned-sha fetch first (cheapest); fall back to a full
    // fetch if the server refuses to serve an arbitrary commit by sha.
    let mut shallow = git_command(Some(&target), token);
    shallow
        .arg("fetch")
        .arg("--depth")
        .arg("1")
        .arg("origin")
        .arg(sha);
    let shallow_out = run_git(shallow, "fetch --depth 1 origin <sha>").await?;

    if !shallow_out.success {
        // Some servers reject want-sha for shallow fetch; fetch everything and
        // resolve the sha locally instead. Do not echo the failing command.
        log::warn!(
            "shallow pinned-sha fetch failed for {owner}/{repo}; falling back to full fetch"
        );
        let mut full = git_command(Some(&target), token);
        full.arg("fetch").arg("origin");
        let full_out = run_git(full, "fetch origin").await?;
        if !full_out.success {
            return Err(RunnerError::Execution(format!(
                "git fetch origin failed for {owner}/{repo}: {}",
                full_out.stderr
            )));
        }
    }

    // Detach onto the requested commit regardless of which fetch path ran.
    let mut checkout = git_command(Some(&target), None);
    checkout
        .arg("checkout")
        .arg("--force")
        .arg("--detach")
        .arg(sha);
    let checkout_out = run_git(checkout, "checkout --force --detach <sha>").await?;
    if !checkout_out.success {
        return Err(RunnerError::Execution(format!(
            "git checkout {sha} failed for {owner}/{repo}: {}",
            checkout_out.stderr
        )));
    }

    Ok(target)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Initialise a source repo with one commit and return (repo_dir, sha).
    async fn init_source_repo(dir: &Path, file: &str, contents: &str) -> String {
        let run = |args: &[&str]| {
            let mut c = git_command(Some(dir), None);
            // Identity + non-signing config so commits work in a clean CI env.
            c.env("GIT_AUTHOR_NAME", "Test")
                .env("GIT_AUTHOR_EMAIL", "test@example.invalid")
                .env("GIT_COMMITTER_NAME", "Test")
                .env("GIT_COMMITTER_EMAIL", "test@example.invalid");
            c.args(args);
            c
        };

        tokio::fs::create_dir_all(dir).await.unwrap();
        assert!(
            run(&["init", "-q", "-b", "main"])
                .status()
                .await
                .unwrap()
                .success()
        );
        // git -C <dir> writes the worktree; create the tracked file directly.
        tokio::fs::write(dir.join(file), contents).await.unwrap();
        assert!(run(&["add", "."]).status().await.unwrap().success());
        assert!(
            run(&[
                "-c",
                "commit.gpgsign=false",
                "commit",
                "-q",
                "-m",
                "initial"
            ])
            .status()
            .await
            .unwrap()
            .success()
        );

        let out = run(&["rev-parse", "HEAD"]).output().await.unwrap();
        assert!(out.status.success());
        String::from_utf8_lossy(&out.stdout).trim().to_string()
    }

    #[tokio::test]
    async fn checks_out_repo_content_at_sha_from_local_remote() {
        let tmp = tempfile::tempdir().unwrap();
        let source = tmp.path().join("source");
        let sha = init_source_repo(&source, "hello.txt", "world\n").await;

        // The "instance_url" is the directory holding the owner/repo tree, so
        // clone_url resolves to <root>/<owner>/<repo>. Lay the source repo out
        // accordingly: <remote_root>/acme/widget.
        let remote_root = tmp.path().join("remote_root");
        let placed = remote_root.join("acme").join("widget");
        tokio::fs::create_dir_all(placed.parent().unwrap())
            .await
            .unwrap();
        tokio::fs::rename(&source, &placed).await.unwrap();

        let checkout_root = tmp.path().join("checkouts");
        let target = ensure_checkout(
            remote_root.to_str().unwrap(),
            "acme",
            "widget",
            &sha,
            None,
            &checkout_root,
        )
        .await
        .expect("checkout succeeds");

        assert_eq!(target, checkout_root.join("acme").join("widget"));
        assert!(target.join(".git").exists(), "working tree initialised");
        let body = tokio::fs::read_to_string(target.join("hello.txt"))
            .await
            .expect("checked-out file present");
        assert_eq!(body, "world\n", "content matches committed file");
    }

    #[tokio::test]
    async fn second_call_same_sha_is_idempotent_cache_hit() {
        let tmp = tempfile::tempdir().unwrap();
        let placed = tmp.path().join("remote_root").join("acme").join("widget");
        let sha = init_source_repo(&placed, "data.txt", "payload\n").await;
        let remote_root = tmp.path().join("remote_root");
        let checkout_root = tmp.path().join("checkouts");

        // First call clones; second call must hit the existing .git and succeed.
        let first = ensure_checkout(
            remote_root.to_str().unwrap(),
            "acme",
            "widget",
            &sha,
            None,
            &checkout_root,
        )
        .await
        .expect("first checkout succeeds");
        assert!(first.join(".git").exists());

        let second = ensure_checkout(
            remote_root.to_str().unwrap(),
            "acme",
            "widget",
            &sha,
            None,
            &checkout_root,
        )
        .await
        .expect("idempotent re-checkout succeeds");
        assert_eq!(first, second);
        assert_eq!(
            tokio::fs::read_to_string(second.join("data.txt"))
                .await
                .unwrap(),
            "payload\n"
        );
    }

    /// Verify that the token and its base64 encoding are absent from every
    /// process argument. Environment variable injection must be used instead
    /// of `-c http.extraHeader=...` to prevent `/proc/PID/cmdline` exposure.
    #[test]
    fn token_never_in_cmd_args() {
        let token = "super-secret-token-DO-NOT-EXPOSE";
        let cmd = git_command(None, Some(token));
        let basic =
            base64::engine::general_purpose::STANDARD.encode(format!("x-access-token:{token}"));
        for arg in cmd.as_std().get_args() {
            let s = arg.to_string_lossy();
            assert!(
                !s.contains(token),
                "raw token must not appear in process args: {s}"
            );
            assert!(
                !s.contains(basic.as_str()),
                "base64 token must not appear in process args: {s}"
            );
        }
    }

    #[tokio::test]
    async fn token_never_lands_in_git_config() {
        let tmp = tempfile::tempdir().unwrap();
        let placed = tmp.path().join("remote_root").join("acme").join("widget");
        let sha = init_source_repo(&placed, "f.txt", "x\n").await;
        let remote_root = tmp.path().join("remote_root");
        let checkout_root = tmp.path().join("checkouts");

        // Pass a recognisable token; it must not be persisted anywhere on disk.
        let token = "super-secret-token-DO-NOT-PERSIST";
        let target = ensure_checkout(
            remote_root.to_str().unwrap(),
            "acme",
            "widget",
            &sha,
            Some(token),
            &checkout_root,
        )
        .await
        .expect("checkout with token succeeds");

        let config = tokio::fs::read_to_string(target.join(".git").join("config"))
            .await
            .expect(".git/config present");
        assert!(
            !config.contains(token),
            "token must not appear in .git/config: {config}"
        );
    }

    // --- Path-traversal regression tests (Refs #2398) ---

    #[tokio::test]
    async fn owner_with_parent_dir_traversal_is_rejected() {
        let tmp = tempfile::tempdir().unwrap();
        let checkout_root = tmp.path().join("checkouts");
        let result = ensure_checkout(
            "http://localhost",
            "..",
            "repo",
            "abc123",
            None,
            &checkout_root,
        )
        .await;
        assert!(result.is_err(), "expected error for owner='..'");
        let msg = result.unwrap_err().to_string();
        assert!(
            msg.contains("owner"),
            "error should mention 'owner', got: {msg}"
        );
    }

    #[tokio::test]
    async fn repo_with_path_traversal_is_rejected() {
        let tmp = tempfile::tempdir().unwrap();
        let checkout_root = tmp.path().join("checkouts");
        let result = ensure_checkout(
            "http://localhost",
            "acme",
            "../../etc/passwd",
            "abc123",
            None,
            &checkout_root,
        )
        .await;
        assert!(
            result.is_err(),
            "expected error for repo='../../etc/passwd'"
        );
        let msg = result.unwrap_err().to_string();
        assert!(
            msg.contains("repo"),
            "error should mention 'repo', got: {msg}"
        );
    }

    #[tokio::test]
    async fn absolute_repo_path_is_rejected() {
        let tmp = tempfile::tempdir().unwrap();
        let checkout_root = tmp.path().join("checkouts");
        let result = ensure_checkout(
            "http://localhost",
            "acme",
            "/etc/passwd",
            "abc123",
            None,
            &checkout_root,
        )
        .await;
        assert!(result.is_err(), "expected error for repo='/etc/passwd'");
        let msg = result.unwrap_err().to_string();
        assert!(
            msg.contains("repo"),
            "error should mention 'repo', got: {msg}"
        );
    }

    #[tokio::test]
    async fn empty_owner_is_rejected() {
        let tmp = tempfile::tempdir().unwrap();
        let checkout_root = tmp.path().join("checkouts");
        let result = ensure_checkout(
            "http://localhost",
            "",
            "repo",
            "abc123",
            None,
            &checkout_root,
        )
        .await;
        assert!(result.is_err(), "expected error for empty owner");
        let msg = result.unwrap_err().to_string();
        assert!(
            msg.contains("owner"),
            "error should mention 'owner', got: {msg}"
        );
    }
}
