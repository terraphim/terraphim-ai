//! Shared test-only helpers for the PR gate evidence slice (#3293).
//!
//! `build_pr_gate_git_fixture` creates a deterministic git repository whose
//! PR head commit SHA is stable across runs (fixed author/committer
//! identities and dates), so tests can reference the head SHA without
//! threading it through every constructor.
#![allow(dead_code)]

use std::path::Path;

/// Deterministic identities/dates so commit SHAs are reproducible.
const FIXTURE_NAME: &str = "ADF Fixture";
const FIXTURE_EMAIL: &str = "adf-fixture@example.invalid";
const FIXTURE_BASE_DATE: &str = "2026-01-01T00:00:00+0000";
const FIXTURE_HEAD_DATE: &str = "2026-01-01T00:00:01+0000";

/// The PR number the shared fixture binds refs for
/// (`refs/adf/pr-{n}` / `refs/adf/base-main`).
pub const FIXTURE_PR_NUMBER: u64 = 641;

fn fixture_git(dir: &Path, args: &[&str], date: &str) {
    let status = std::process::Command::new("git")
        .current_dir(dir)
        .args(args)
        .env("GIT_AUTHOR_NAME", FIXTURE_NAME)
        .env("GIT_AUTHOR_EMAIL", FIXTURE_EMAIL)
        .env("GIT_COMMITTER_NAME", FIXTURE_NAME)
        .env("GIT_COMMITTER_EMAIL", FIXTURE_EMAIL)
        .env("GIT_AUTHOR_DATE", date)
        .env("GIT_COMMITTER_DATE", date)
        .status()
        .unwrap_or_else(|e| panic!("failed to run git {args:?}: {e}"));
    assert!(
        status.success(),
        "git {args:?} failed in fixture {}",
        dir.display()
    );
}

fn rev_parse(dir: &Path, rev: &str) -> String {
    let output = std::process::Command::new("git")
        .current_dir(dir)
        .args(["rev-parse", rev])
        .output()
        .expect("git rev-parse runs");
    assert!(
        output.status.success(),
        "git rev-parse {rev} failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout)
        .expect("utf8 sha")
        .trim()
        .to_string()
}

/// Build the deterministic PR fixture inside `dir` and return the PR head
/// commit SHA. The repo has a base commit on `main` and a PR head commit,
/// with the refs the evidence builder's diff ranges look for already
/// bound (`refs/adf/base-main`, `refs/adf/pr-641`).
pub fn build_pr_gate_git_fixture(dir: &Path) -> String {
    fixture_git(dir, &["init", "-q", "-b", "main"], FIXTURE_BASE_DATE);
    std::fs::write(dir.join("base.txt"), "base\n").expect("write base.txt");
    fixture_git(dir, &["add", "."], FIXTURE_BASE_DATE);
    fixture_git(
        dir,
        &[
            "-c",
            "commit.gpgsign=false",
            "commit",
            "-q",
            "-m",
            "fixture base",
        ],
        FIXTURE_BASE_DATE,
    );

    std::fs::write(dir.join("change.txt"), "pr change\n").expect("write change.txt");
    fixture_git(dir, &["add", "."], FIXTURE_HEAD_DATE);
    fixture_git(
        dir,
        &[
            "-c",
            "commit.gpgsign=false",
            "commit",
            "-q",
            "-m",
            "fixture pr head",
        ],
        FIXTURE_HEAD_DATE,
    );

    let head = rev_parse(dir, "HEAD");
    let base = rev_parse(dir, "HEAD~1");
    fixture_git(
        dir,
        &[
            "update-ref",
            &format!("refs/adf/pr-{FIXTURE_PR_NUMBER}"),
            &head,
        ],
        FIXTURE_HEAD_DATE,
    );
    fixture_git(
        dir,
        &["update-ref", "refs/adf/base-main", &base],
        FIXTURE_HEAD_DATE,
    );
    head
}

/// The deterministic head SHA of [`build_pr_gate_git_fixture`].
///
/// Computed on first use by building a throwaway fixture; every fixture
/// built by [`build_pr_gate_git_fixture`] yields the same SHA because the
/// commit inputs (tree, parent, identities, dates, message) are fixed.
pub fn pr_gate_fixture_head_sha() -> &'static str {
    static HEAD_SHA: std::sync::OnceLock<String> = std::sync::OnceLock::new();
    HEAD_SHA.get_or_init(|| {
        let tmp = tempfile::TempDir::new().expect("scratch fixture tempdir");
        build_pr_gate_git_fixture(tmp.path())
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fixture_is_deterministic() {
        let a = tempfile::TempDir::new().unwrap();
        let b = tempfile::TempDir::new().unwrap();
        assert_eq!(
            build_pr_gate_git_fixture(a.path()),
            build_pr_gate_git_fixture(b.path()),
            "fixture commits must be byte-identical across runs"
        );
    }
}
