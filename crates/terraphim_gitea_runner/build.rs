//! Emits source provenance for the runner binary, including tracked dirtiness.
//!
//! Normal repository builds use Git's full `HEAD` commit. Source archives can
//! set `TERRAPHIM_SOURCE_COMMIT` to the full 40-character commit SHA used to
//! create the archive; an unstamped archive remains explicitly identifiable as
//! `source-archive:unknown`.

use std::path::{Path, PathBuf};
use std::process::Command;

const SOURCE_COMMIT_ENV: &str = "TERRAPHIM_SOURCE_COMMIT";

pub(crate) fn full_sha(value: &str) -> bool {
    value.len() == 40 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

pub(crate) fn git_output(manifest_dir: &Path, args: &[&str]) -> Option<String> {
    let output = Command::new("git")
        .args(args)
        .current_dir(manifest_dir)
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    Some(String::from_utf8(output.stdout).ok()?.trim().to_owned())
}

pub(crate) fn source_identity(manifest_dir: &Path, archive_commit: Option<&str>) -> String {
    if let Some(commit) = archive_commit {
        let commit = commit.trim();
        assert!(
            full_sha(commit),
            "{SOURCE_COMMIT_ENV} must be a full 40-character hexadecimal commit SHA"
        );
        return format!("source-archive:{commit}");
    }

    let Some(commit) = git_output(manifest_dir, &["rev-parse", "--verify", "HEAD"])
        .filter(|commit| full_sha(commit))
    else {
        return "source-archive:unknown".to_owned();
    };
    let Some(status) = git_output(
        manifest_dir,
        &["status", "--porcelain", "--untracked-files=no"],
    ) else {
        return "source-archive:unknown".to_owned();
    };
    format!(
        "git:{commit}{}",
        if status.is_empty() { "" } else { "-dirty" }
    )
}

pub(crate) fn tracked_inputs(manifest_dir: &Path) -> Option<Vec<PathBuf>> {
    let root = git_output(manifest_dir, &["rev-parse", "--show-toplevel"])?;
    let files = git_output(manifest_dir, &["ls-files", "--full-name"])?;
    Some(
        files
            .lines()
            .map(|file| Path::new(&root).join(file))
            .collect(),
    )
}

#[cfg_attr(test, allow(dead_code))]
fn main() {
    println!("cargo:rerun-if-env-changed={SOURCE_COMMIT_ENV}");
    let manifest_dir = std::env::var_os("CARGO_MANIFEST_DIR")
        .map(std::path::PathBuf::from)
        .expect("Cargo must set CARGO_MANIFEST_DIR for build scripts");

    // Worktrees keep HEAD separately from the common ref store. Track both the
    // worktree HEAD and its resolved branch ref where Git can expose them.
    if let Some(head) = git_output(&manifest_dir, &["rev-parse", "--git-path", "HEAD"]) {
        println!("cargo:rerun-if-changed={head}");
    }
    let symbolic_ref_path =
        git_output(&manifest_dir, &["symbolic-ref", "HEAD"]).and_then(|symbolic_ref| {
            git_output(
                &manifest_dir,
                &["rev-parse", "--git-path", symbolic_ref.as_str()],
            )
        });
    if let Some(ref_path) = symbolic_ref_path {
        println!("cargo:rerun-if-changed={ref_path}");
    }
    if let Some(packed_refs) =
        git_output(&manifest_dir, &["rev-parse", "--git-path", "packed-refs"])
    {
        println!("cargo:rerun-if-changed={packed_refs}");
    }
    if let Some(index) = git_output(&manifest_dir, &["rev-parse", "--git-path", "index"]) {
        println!("cargo:rerun-if-changed={index}");
    }
    // Once `rerun-if-changed` is emitted Cargo stops its package-wide default
    // scan. Track every versioned input so unstaged edits refresh the dirty
    // suffix instead of reusing a stale clean stamp.
    if let Some(files) = tracked_inputs(&manifest_dir) {
        for file in files {
            println!("cargo:rerun-if-changed={}", file.display());
        }
    }

    let archive_commit = std::env::var(SOURCE_COMMIT_ENV).ok();
    let identity = source_identity(&manifest_dir, archive_commit.as_deref());

    println!("cargo:rustc-env=TERRAPHIM_RUNNER_BUILD_COMMIT={identity}");
}
