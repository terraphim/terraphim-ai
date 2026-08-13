//! Contract tests for the merge-authoritative `.gitea/workflows/native-ci.yml`
//! (Refs #3222 Tasks 1-3).
//!
//! Run 23137 failed because the workflow named a repository script directly and
//! the runner's policy rejected the literal first token before execution. These
//! tests pin the workflow file itself against the *same* planner the deployed
//! runner uses, so a step that the runner would reject can never be merged.
//!
//! No mocks: the real parser, the real embedded default policy, and -- for the
//! preflight -- a real `bash` process are used.

use std::path::PathBuf;
use std::process::Command;

use terraphim_gitea_runner::TaxonomyPlanner;
use terraphim_gitea_runner::policy::PolicyPlanner;
use terraphim_github_runner::parse_single_workflow_yaml;

/// Repository-root-relative path to the authoritative workflow.
fn workflow_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(".gitea/workflows/native-ci.yml")
}

fn workflow_yaml() -> String {
    let path = workflow_path();
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()))
}

/// Every `run:` step in the authoritative workflow must compile under the same
/// embedded default policy the deployed runner loads. This is the regression
/// guard for run 23137.
#[tokio::test]
async fn native_ci_workflow_compiles_under_the_default_runner_policy() {
    let wf = parse_single_workflow_yaml(&workflow_yaml()).expect("native-ci.yml must parse");
    assert!(!wf.steps.is_empty(), "workflow must declare run: steps");

    // `rch_available = false` is the stricter of the two planner configurations
    // for acceptance purposes: it exercises the host route without rewriting.
    let plan = TaxonomyPlanner::default_policy(false)
        .compile(wf)
        .await
        .expect(
            "every native-ci step must be accepted by the embedded default policy; \
             invoke repository scripts as `bash ./scripts/<name>.sh`",
        );
    assert!(!plan.routes.is_empty());
}

/// The workflow must be dispatchable for exact-SHA verification and must cover
/// pull requests, so branch protection can require an observed PR-event context.
#[tokio::test]
async fn native_ci_workflow_declares_push_pull_request_and_dispatch_triggers() {
    let yaml = workflow_yaml();
    for trigger in ["push:", "pull_request:", "workflow_dispatch:"] {
        assert!(
            yaml.lines().any(|l| l.trim() == trigger),
            "native-ci.yml must declare the `{trigger}` trigger"
        );
    }
}

/// A claimed job must be bounded at the Actions layer as well as by the runner,
/// so a hung step cannot sit until the server-side zombie timeout.
#[tokio::test]
async fn native_ci_workflow_declares_a_job_timeout() {
    let yaml = workflow_yaml();
    let declared = yaml
        .lines()
        .find_map(|l| l.trim().strip_prefix("timeout-minutes:"))
        .map(|v| {
            v.trim()
                .parse::<u64>()
                .expect("timeout-minutes must be an integer")
        })
        .expect("native-ci.yml must declare a job `timeout-minutes:` bound");
    assert!(
        declared > 0 && declared <= 180,
        "job timeout must be a bounded, non-zero number of minutes, got {declared}"
    );
}

/// `main` does not carry `scripts/check-tinyclaw-test-hermeticity.sh` -- it exists
/// only on the TinyClaw candidate refs. The preflight step must therefore succeed
/// on a ref where the script is absent instead of failing the merge gate.
///
/// This runs the step's *actual* command with a real `bash` in an empty
/// directory, so it proves behaviour rather than restating the YAML.
#[tokio::test]
async fn optional_preflight_step_succeeds_when_the_candidate_script_is_absent() {
    let wf = parse_single_workflow_yaml(&workflow_yaml()).expect("native-ci.yml must parse");
    let preflight = wf
        .steps
        .iter()
        .find(|s| s.command.contains("check-tinyclaw-test-hermeticity.sh"))
        .expect(
            "native-ci.yml must carry the optional hermeticity preflight step; \
             it is the candidate gate PR #3221 needs",
        );

    let empty = tempfile::tempdir().expect("temp dir");
    let out = Command::new("bash")
        .arg("-c")
        .arg(&preflight.command)
        .current_dir(empty.path())
        .output()
        .expect("spawn bash");
    assert!(
        out.status.success(),
        "preflight must be a no-op when the script is absent (exit {:?}); stderr: {}",
        out.status.code(),
        String::from_utf8_lossy(&out.stderr)
    );
}

/// The other half of the same contract: when the script *is* present (as on the
/// candidate SHA), the preflight must actually execute it and propagate failure.
/// A guard that silently swallows the real gate would be worse than no guard.
#[tokio::test]
async fn optional_preflight_step_runs_and_propagates_failure_when_the_script_is_present() {
    let wf = parse_single_workflow_yaml(&workflow_yaml()).expect("native-ci.yml must parse");
    let preflight = wf
        .steps
        .iter()
        .find(|s| s.command.contains("check-tinyclaw-test-hermeticity.sh"))
        .expect("preflight step");

    let dir = tempfile::tempdir().expect("temp dir");
    let scripts = dir.path().join("scripts");
    std::fs::create_dir_all(&scripts).expect("mkdir scripts");
    std::fs::write(
        scripts.join("check-tinyclaw-test-hermeticity.sh"),
        "#!/usr/bin/env bash\necho ran-hermeticity-check\nexit 3\n",
    )
    .expect("write stand-in script");

    let out = Command::new("bash")
        .arg("-c")
        .arg(&preflight.command)
        .current_dir(dir.path())
        .output()
        .expect("spawn bash");
    assert_eq!(
        out.status.code(),
        Some(3),
        "the preflight must run the script and propagate its exit code; stdout: {}",
        String::from_utf8_lossy(&out.stdout)
    );
    assert!(
        String::from_utf8_lossy(&out.stdout).contains("ran-hermeticity-check"),
        "the script must actually have been executed"
    );
}
