//! Claimed-task heartbeat lifecycle regressions (Refs #3390).
//!
//! These tests use a real axum Connect-JSON endpoint and the real host workflow
//! executor. A filesystem gate makes the workflow duration deterministic: the
//! fake Gitea releases the command only after it has observed the required
//! heartbeat attempts.

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use async_trait::async_trait;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::{Json, Router, extract::State, routing::post};
use base64::Engine;
use serde_json::{Value, json};
use terraphim_gitea_runner::TaxonomyPlanner;
use terraphim_gitea_runner::client::{GiteaRunnerClient, ReqwestRunnerClient};
use terraphim_gitea_runner::state::RunnerState;
use terraphim_gitea_runner::task_worker::TaskWorker;
use terraphim_gitea_runner::types::{
    DeclareRequest, DeclareResponse, FetchTaskResponse, RegisterRequest, RunnerInfo, Task,
    UpdateLogRequest, UpdateLogResponse, UpdateTaskRequest, UpdateTaskResponse,
};
use terraphim_gitea_runner::{Result, RunnerError};

const UNSPECIFIED: i32 = 0;
const SUCCESS: i32 = 1;
const FAILURE: i32 = 2;

#[cfg(target_os = "linux")]
fn linux_process_identity(stat: &str) -> Option<(char, u64)> {
    let (_, fields_after_comm) = stat.rsplit_once(')')?;
    let mut fields = fields_after_comm.split_whitespace();
    let state = fields.next()?.chars().next()?;
    let start_time = fields.nth(18)?.parse().ok()?;
    Some((state, start_time))
}

#[cfg(target_os = "linux")]
fn same_linux_process_is_alive(stat: &str, expected_start_time: u64) -> bool {
    linux_process_identity(stat)
        .is_some_and(|(state, start_time)| state != 'Z' && start_time == expected_start_time)
}

#[cfg(target_os = "linux")]
#[test]
fn zombie_proc_stat_is_not_a_live_process() {
    let zombie_with_closing_paren_in_comm =
        "4242 (workflow ) helper) Z 1 2 3 4 5 6 7 8 9 10 11 12 13 14 15 16 17 18 1900";

    assert_eq!(
        linux_process_identity(zombie_with_closing_paren_in_comm),
        Some(('Z', 1900)),
        "the parser must split after the final ')' because comm may contain ')'"
    );
    assert!(
        !same_linux_process_is_alive(zombie_with_closing_paren_in_comm, 1900),
        "a zombie /proc entry must be treated as no longer alive"
    );
}

#[derive(Clone, Copy)]
enum HeartbeatBehavior {
    Accept,
    RejectFirst(usize),
    Reject,
    RejectAfterTerminalStatus,
}

struct Recorded {
    bodies: Vec<Value>,
    events: Vec<String>,
    status_bodies: Vec<Value>,
    block_success_status: bool,
    terminal_status_in_flight: bool,
    terminal_status_started: Arc<tokio::sync::Notify>,
    heartbeat_attempts: usize,
    release_after: usize,
    gate: PathBuf,
    behavior: HeartbeatBehavior,
    block_log_stream: bool,
    log_stream_blocked: bool,
    log_bodies: Vec<Value>,
    log_stream_started: Arc<tokio::sync::Notify>,
    release_log_stream: Arc<tokio::sync::Notify>,
    heartbeat_during_blocked_log: Arc<tokio::sync::Notify>,
    heartbeat_budget_exhausted: Arc<tokio::sync::Notify>,
}

type Shared = Arc<Mutex<Recorded>>;

async fn update_task(State(shared): State<Shared>, Json(body): Json<Value>) -> impl IntoResponse {
    let result = body["state"]["result"].as_i64().unwrap_or_default() as i32;
    let is_heartbeat = result == UNSPECIFIED && body["state"].get("startedAt").is_none();

    let mut recorded = shared.lock().unwrap();
    recorded.events.push(format!("task:{result}"));
    recorded.bodies.push(body);
    if !is_heartbeat {
        return (StatusCode::OK, Json(json!({"tasksVersion": 1})));
    }

    recorded.heartbeat_attempts += 1;
    if recorded.heartbeat_attempts == 3 {
        recorded.heartbeat_budget_exhausted.notify_one();
    }
    if recorded.log_stream_blocked {
        recorded.heartbeat_during_blocked_log.notify_one();
    }
    if recorded.heartbeat_attempts == recorded.release_after {
        std::fs::write(&recorded.gate, b"release").unwrap();
    }
    match recorded.behavior {
        HeartbeatBehavior::Accept => (StatusCode::OK, Json(json!({"tasksVersion": 1}))),
        HeartbeatBehavior::RejectFirst(count) if recorded.heartbeat_attempts <= count => (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(json!({"error": "transient heartbeat transport failure"})),
        ),
        HeartbeatBehavior::RejectFirst(_) => (StatusCode::OK, Json(json!({"tasksVersion": 1}))),
        HeartbeatBehavior::Reject => (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(json!({"error": "heartbeat transport unavailable"})),
        ),
        HeartbeatBehavior::RejectAfterTerminalStatus if recorded.terminal_status_in_flight => (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(json!({"error": "heartbeat transport unavailable"})),
        ),
        HeartbeatBehavior::RejectAfterTerminalStatus => {
            (StatusCode::OK, Json(json!({"tasksVersion": 1})))
        }
    }
}

async fn update_status(State(shared): State<Shared>, Json(body): Json<Value>) -> Json<Value> {
    let state = body["state"].as_str().unwrap_or("missing").to_owned();
    let blocked = {
        let mut recorded = shared.lock().unwrap();
        recorded.events.push(format!("status:{state}"));
        recorded.status_bodies.push(body);
        if state == "success" && recorded.block_success_status {
            recorded.block_success_status = false;
            recorded.terminal_status_in_flight = true;
            recorded.terminal_status_started.notify_one();
            true
        } else {
            false
        }
    };
    if blocked {
        std::future::pending::<()>().await;
    }
    Json(json!({"id": 1}))
}

async fn update_log(State(shared): State<Shared>, Json(body): Json<Value>) -> Json<Value> {
    let release = {
        let mut recorded = shared.lock().unwrap();
        recorded.log_bodies.push(body.clone());
        if recorded.block_log_stream {
            recorded.block_log_stream = false;
            recorded.log_stream_blocked = true;
            recorded.log_stream_started.notify_one();
            Some(recorded.release_log_stream.clone())
        } else {
            None
        }
    };
    if let Some(release) = release {
        release.notified().await;
    }

    let ack = body["index"].as_i64().unwrap_or_default()
        + body["rows"].as_array().map_or(0, |rows| rows.len() as i64);
    Json(json!({"ackIndex": ack}))
}

async fn spawn_server(shared: Shared) -> String {
    let base = "/api/actions/runner.v1.RunnerService";
    let app = Router::new()
        .route(&format!("{base}/UpdateTask"), post(update_task))
        .route(&format!("{base}/UpdateLog"), post(update_log))
        .route(
            "/api/v1/repos/{owner}/{repo}/statuses/{sha}",
            post(update_status),
        )
        .with_state(shared);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
    format!("http://{addr}")
}

fn state() -> RunnerState {
    RunnerState {
        uuid: "heartbeat-test-runner".into(),
        token: "test-runner-token".into(),
        name: "heartbeat-test".into(),
        version: "test".into(),
        labels: vec!["terraphim-native".into()],
        ephemeral: false,
    }
}

fn task(id: i64, command: &str) -> Task {
    let yaml = format!(
        "name: heartbeat-test\njobs:\n  build:\n    runs-on: terraphim-native\n    steps:\n      - name: gated\n        run: {command}\n"
    );
    Task {
        id,
        workflow_payload: base64::engine::general_purpose::STANDARD.encode(yaml),
        context: json!({"github": {"repository": "terraphim/proof"}}),
        secrets: BTreeMap::new(),
        vars: BTreeMap::new(),
        needs: Value::Null,
    }
}

fn task_at_sha(id: i64, command: &str, sha: &str) -> Task {
    let mut task = task(id, command);
    task.context = json!({"github": {
        "repository": "terraphim/proof",
        "sha": sha,
        "token": "job-status-token"
    }});
    task
}

fn run_git(dir: &std::path::Path, args: &[&str]) -> std::process::Output {
    std::process::Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(args)
        .env("GIT_AUTHOR_NAME", "Test")
        .env("GIT_AUTHOR_EMAIL", "test@example.invalid")
        .env("GIT_COMMITTER_NAME", "Test")
        .env("GIT_COMMITTER_EMAIL", "test@example.invalid")
        .output()
        .unwrap()
}

async fn seed_checkout(checkout_root: &std::path::Path) -> String {
    let source = checkout_root.join("source");
    let target = checkout_root.join("terraphim/proof");
    tokio::fs::create_dir_all(&source).await.unwrap();
    assert!(run_git(&source, &["init", "-q"]).status.success());
    tokio::fs::write(source.join("README.md"), "status ordering\n")
        .await
        .unwrap();
    assert!(run_git(&source, &["add", "."]).status.success());
    assert!(
        run_git(
            &source,
            &["-c", "commit.gpgsign=false", "commit", "-q", "-m", "seed"]
        )
        .status
        .success()
    );
    let sha = String::from_utf8(run_git(&source, &["rev-parse", "HEAD"]).stdout)
        .unwrap()
        .trim()
        .to_owned();
    tokio::fs::create_dir_all(target.parent().unwrap())
        .await
        .unwrap();
    assert!(
        std::process::Command::new("git")
            .args(["clone", "-q"])
            .arg(&source)
            .arg(&target)
            .status()
            .unwrap()
            .success()
    );
    sha
}

async fn harness(
    behavior: HeartbeatBehavior,
    release_after: usize,
) -> (
    Shared,
    TaskWorker<ReqwestRunnerClient, TaxonomyPlanner>,
    tempfile::TempDir,
) {
    let temp = tempfile::tempdir().unwrap();
    let shared = Arc::new(Mutex::new(Recorded {
        bodies: Vec::new(),
        events: Vec::new(),
        status_bodies: Vec::new(),
        block_success_status: false,
        terminal_status_in_flight: false,
        terminal_status_started: Arc::new(tokio::sync::Notify::new()),
        heartbeat_attempts: 0,
        release_after,
        gate: temp.path().join("workflow-release"),
        behavior,
        block_log_stream: false,
        log_stream_blocked: false,
        log_bodies: Vec::new(),
        log_stream_started: Arc::new(tokio::sync::Notify::new()),
        release_log_stream: Arc::new(tokio::sync::Notify::new()),
        heartbeat_during_blocked_log: Arc::new(tokio::sync::Notify::new()),
        heartbeat_budget_exhausted: Arc::new(tokio::sync::Notify::new()),
    }));
    let url = spawn_server(shared.clone()).await;
    let worker = TaskWorker::new(
        Arc::new(ReqwestRunnerClient::new(url.clone())),
        Arc::new(TaxonomyPlanner::default_policy(true)),
        url,
        temp.path(),
    )
    .with_heartbeat_interval(Duration::from_millis(10))
    .with_heartbeat_failure_attempts(3);
    (shared, worker, temp)
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn terminal_commit_status_post_precedes_terminal_update_through_production_composition() {
    let (shared, worker, temp) = harness(HeartbeatBehavior::Accept, usize::MAX).await;
    let sha = seed_checkout(temp.path()).await;

    assert!(
        worker
            .run(&state(), task_at_sha(3397, "echo ordered", &sha))
            .await
            .unwrap()
    );

    let recorded = shared.lock().unwrap();
    let status = recorded
        .events
        .iter()
        .rposition(|event| event == "status:success")
        .expect("terminal success status POST must be observed");
    let terminal = recorded
        .events
        .iter()
        .position(|event| event == "task:1")
        .expect("terminal UpdateTask must be observed");
    assert!(
        status < terminal,
        "commit-status POST must precede terminal UpdateTask: {:?}",
        recorded.events
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn stalled_checkout_is_bounded_and_heartbeats_before_terminal_failure() {
    let (shared, _, temp) = harness(HeartbeatBehavior::Accept, usize::MAX).await;
    let runner_url = spawn_server(shared.clone()).await;
    let stalled_listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let stalled_address = stalled_listener.local_addr().unwrap();
    let stalled_git = tokio::spawn(async move {
        let (_socket, _) = stalled_listener.accept().await.unwrap();
        std::future::pending::<()>().await;
    });
    let worker = TaskWorker::new(
        Arc::new(ReqwestRunnerClient::new(runner_url)),
        Arc::new(TaxonomyPlanner::default_policy(true)),
        format!("http://{stalled_address}"),
        temp.path(),
    )
    .with_heartbeat_interval(Duration::from_millis(10))
    .with_heartbeat_failure_attempts(3)
    .with_checkout_timeout(Duration::from_millis(80))
    .with_status_request_timeout(Duration::from_millis(20));

    let error = tokio::time::timeout(
        Duration::from_secs(1),
        worker.run(&state(), task_at_sha(3398, "echo never-runs", "deadbeef")),
    )
    .await
    .expect("bounded checkout must return")
    .expect_err("stalled checkout must fail closed");
    stalled_git.abort();

    assert!(error.to_string().contains("timed out"), "{error}");
    let recorded = shared.lock().unwrap();
    let terminal = recorded
        .events
        .iter()
        .position(|event| event == "task:2")
        .expect("checkout failure must terminalize");
    assert!(
        recorded.events[..terminal]
            .iter()
            .any(|event| event == "task:0"),
        "a lease heartbeat must be observed while checkout is stalled: {:?}",
        recorded.events
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn post_seal_lease_failure_is_visible_in_terminal_commit_status() {
    let (shared, worker, temp) =
        harness(HeartbeatBehavior::RejectAfterTerminalStatus, usize::MAX).await;
    let sha = seed_checkout(temp.path()).await;
    let terminal_status_started = {
        let mut recorded = shared.lock().unwrap();
        recorded.block_success_status = true;
        recorded.terminal_status_started.clone()
    };
    let worker = worker.with_heartbeat_interval(Duration::from_millis(20));
    let run = tokio::spawn(async move {
        worker
            .run(&state(), task_at_sha(3399, "echo sealed", &sha))
            .await
    });

    tokio::time::timeout(Duration::from_secs(1), terminal_status_started.notified())
        .await
        .expect("workflow must seal logs and enter terminal status delivery");
    let error = tokio::time::timeout(Duration::from_secs(1), run)
        .await
        .expect("heartbeat failure must interrupt stalled status delivery")
        .expect("worker task must join")
        .expect_err("post-seal lease loss must fail closed");
    assert!(error.to_string().contains("heartbeat"), "{error}");

    let recorded = shared.lock().unwrap();
    assert_eq!(
        recorded.log_bodies.last().unwrap()["noMore"],
        true,
        "the #3387 log stream must remain sealed"
    );
    let failure_status = recorded
        .status_bodies
        .iter()
        .rev()
        .find(|body| body["state"] == "failure")
        .expect("repair path must publish a failure status after sealing");
    assert!(
        failure_status["description"]
            .as_str()
            .is_some_and(|description| description.contains("heartbeat")),
        "failure status must preserve the redacted post-seal reason: {failure_status}"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn completed_workflow_keeps_heartbeat_alive_while_log_delivery_is_blocked() {
    let (shared, worker, _temp) = harness(HeartbeatBehavior::Accept, usize::MAX).await;
    let (log_stream_started, release_log_stream, heartbeat_during_blocked_log) = {
        let mut recorded = shared.lock().unwrap();
        recorded.block_log_stream = true;
        (
            recorded.log_stream_started.clone(),
            recorded.release_log_stream.clone(),
            recorded.heartbeat_during_blocked_log.clone(),
        )
    };

    let run = tokio::spawn(async move {
        worker
            .run(&state(), task(3394, "bash -c 'echo workflow-complete'"))
            .await
    });

    tokio::time::timeout(Duration::from_secs(1), log_stream_started.notified())
        .await
        .expect("completed workflow must reach the blocked UpdateLog boundary");
    let heartbeat_observed = tokio::time::timeout(
        Duration::from_millis(250),
        heartbeat_during_blocked_log.notified(),
    )
    .await;

    release_log_stream.notify_one();
    let result = tokio::time::timeout(Duration::from_secs(1), run)
        .await
        .expect("worker must finish after UpdateLog is released")
        .expect("worker task must join")
        .expect("successful workflow must terminalize cleanly");

    assert!(
        heartbeat_observed.is_ok(),
        "heartbeat must continue after workflow completion while UpdateLog is blocked"
    );
    assert!(result);
}

fn results(recorded: &Recorded) -> Vec<i32> {
    recorded
        .bodies
        .iter()
        .map(|body| body["state"]["result"].as_i64().unwrap() as i32)
        .collect()
}

fn assert_heartbeat_lifecycle(
    recorded: &Recorded,
    task_id: i64,
    terminal_result: i32,
    minimum_heartbeats: usize,
) {
    let observed = results(recorded);
    assert_eq!(observed.first(), Some(&UNSPECIFIED));
    assert_eq!(observed.last(), Some(&terminal_result));
    assert_eq!(
        observed
            .iter()
            .filter(|result| **result != UNSPECIFIED)
            .count(),
        1,
        "exactly one terminal result must remain authoritative: {observed:?}"
    );

    let heartbeats = recorded
        .bodies
        .iter()
        .filter(|body| {
            body["state"]["result"] == UNSPECIFIED && body["state"].get("startedAt").is_none()
        })
        .collect::<Vec<_>>();
    assert!(
        heartbeats.len() >= minimum_heartbeats,
        "expected at least {minimum_heartbeats} heartbeats, observed {}",
        heartbeats.len()
    );
    for body in heartbeats {
        assert_eq!(
            body,
            &json!({"state": {"id": task_id, "result": UNSPECIFIED}}),
            "every heartbeat must use the minimal nonterminal body"
        );
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn delayed_success_stays_alive_and_serializes_heartbeats_before_success() {
    let (shared, worker, temp) = harness(HeartbeatBehavior::Accept, 3).await;
    let gate = temp.path().join("workflow-release");
    let command = format!(
        "bash -c 'until test -f \"{}\"; do :; done; sleep 0.05; echo completed'",
        gate.display()
    );

    assert!(worker.run(&state(), task(3390, &command)).await.unwrap());

    let recorded = shared.lock().unwrap();
    assert_heartbeat_lifecycle(&recorded, 3390, SUCCESS, 3);
    let terminal = recorded.bodies.last().unwrap();
    assert_eq!(
        terminal["state"]["result"], SUCCESS,
        "the terminal result must be last, so no heartbeat can follow it"
    );
    assert_eq!(terminal["state"]["id"], 3390);
    assert!(terminal["state"]["stoppedAt"].is_string());
    assert_eq!(terminal.as_object().unwrap().len(), 1);
    assert_eq!(terminal["state"].as_object().unwrap().len(), 3);
    assert_eq!(
        results(&recorded)
            .into_iter()
            .filter(|result| *result != UNSPECIFIED)
            .count(),
        1,
        "exactly one terminal result remains authoritative"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn failed_workflow_cancels_and_joins_heartbeat_before_failure() {
    let (shared, worker, _temp) = harness(HeartbeatBehavior::Accept, usize::MAX).await;

    assert!(
        !worker
            .run(&state(), task(3391, "bash -c 'exit 7'"))
            .await
            .unwrap()
    );

    let recorded = shared.lock().unwrap();
    let observed = results(&recorded);
    assert_eq!(observed.first(), Some(&UNSPECIFIED));
    assert_eq!(observed.last(), Some(&FAILURE));
    assert_eq!(
        observed.iter().filter(|result| **result == FAILURE).count(),
        1
    );
    assert!(
        observed
            .iter()
            .position(|result| *result == FAILURE)
            .is_some_and(|terminal| observed[terminal + 1..].is_empty()),
        "the joined heartbeat must not publish after terminal failure: {observed:?}"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn persistent_heartbeat_transport_failure_is_bounded_and_fails_closed() {
    let (shared, worker, temp) = harness(HeartbeatBehavior::Reject, usize::MAX).await;
    let worker = worker.with_heartbeat_interval(Duration::from_millis(50));
    let gate = temp.path().join("workflow-release");
    let pid_file = temp.path().join("workflow-pid");
    let completion_file = temp.path().join("workflow-completed");
    let command = format!(
        "bash -c 'sleep 60 & descendant=$!; stat=$(<\"/proc/$descendant/stat\"); tail=${{stat##*) }}; set -- $tail; echo \"$$ $descendant ${{20}}\" > \"{}\"; until test -f \"{}\"; do sleep 0.02; done; echo leaked > \"{}\"'",
        pid_file.display(),
        gate.display(),
        completion_file.display()
    );

    let run = tokio::time::timeout(
        Duration::from_secs(1),
        worker.run(&state(), task(3392, &command)),
    )
    .await;
    if run.is_err() {
        // Let the rejected implementation's detached command leave naturally so
        // this RED regression does not strand a process in the test runner.
        std::fs::write(&gate, b"release after timeout").unwrap();
    }
    let error = run
        .expect("worker must fail promptly while the workflow gate remains closed")
        .expect_err("loss of the claimed-task lease must fail closed");

    assert!(error.to_string().contains("heartbeat"), "{error}");
    #[cfg(target_os = "linux")]
    let workflow_identity = std::fs::read_to_string(&pid_file)
        .expect("workflow must start before heartbeat exhaustion")
        .split_whitespace()
        .map(|value| value.parse::<u64>().unwrap())
        .collect::<Vec<_>>();
    #[cfg(target_os = "linux")]
    let workflow_pid = workflow_identity[1];
    #[cfg(target_os = "linux")]
    assert_ne!(
        workflow_identity[0], workflow_identity[1],
        "the recorded PID must be a real descendant, not the process-group leader"
    );

    // Releasing the external gate after terminal failure must not permit the
    // cancelled workflow to resume and create its completion side effect.
    std::fs::write(&gate, b"release after terminal").unwrap();
    tokio::time::sleep(Duration::from_millis(100)).await;
    let completion_exists = completion_file.exists();
    #[cfg(target_os = "linux")]
    let descendant_alive = std::fs::read_to_string(format!("/proc/{workflow_pid}/stat"))
        .ok()
        .is_some_and(|stat| same_linux_process_is_alive(&stat, workflow_identity[2]));
    #[cfg(target_os = "linux")]
    if descendant_alive {
        // Keep the RED mutation proof safe: if group kill is removed, reap the
        // exact recorded descendant before making the assertion fail.
        let _ = std::process::Command::new("kill")
            .args(["-KILL", &workflow_pid.to_string()])
            .status();
    }
    assert!(
        !completion_exists,
        "workflow command leaked past heartbeat cancellation"
    );
    #[cfg(target_os = "linux")]
    assert!(
        !descendant_alive,
        "workflow process {workflow_pid} remains alive after terminal failure"
    );

    let recorded = shared.lock().unwrap();
    assert_eq!(
        recorded.heartbeat_attempts, 3,
        "retry budget must be bounded"
    );
    let observed = results(&recorded);
    assert_eq!(
        observed,
        vec![UNSPECIFIED, UNSPECIFIED, UNSPECIFIED, UNSPECIFIED, FAILURE]
    );
    assert_eq!(
        observed.iter().filter(|result| **result == FAILURE).count(),
        1,
        "terminal failure must be published exactly once"
    );
    assert_eq!(
        recorded.bodies.last().unwrap()["state"]["result"],
        FAILURE,
        "no heartbeat may follow terminal failure"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn lease_loss_during_blocked_log_delivery_preserves_rows_and_advances_index() {
    let (shared, worker, _temp) = harness(HeartbeatBehavior::Reject, usize::MAX).await;
    let (log_stream_started, heartbeat_budget_exhausted, release_log_stream) = {
        let mut recorded = shared.lock().unwrap();
        recorded.block_log_stream = true;
        (
            recorded.log_stream_started.clone(),
            recorded.heartbeat_budget_exhausted.clone(),
            recorded.release_log_stream.clone(),
        )
    };

    let run = tokio::spawn(async move {
        worker
            .run(&state(), task(3395, "bash -c 'echo preserved-row'"))
            .await
    });

    tokio::time::timeout(Duration::from_secs(1), log_stream_started.notified())
        .await
        .expect("the production UpdateLog request must reach the blocked server boundary");
    tokio::time::timeout(
        Duration::from_secs(1),
        heartbeat_budget_exhausted.notified(),
    )
    .await
    .expect("lease loss must occur while UpdateLog delivery is blocked");
    release_log_stream.notify_one();

    let error = tokio::time::timeout(Duration::from_secs(1), run)
        .await
        .expect("worker must finish after blocked UpdateLog is released")
        .expect("worker task must join")
        .expect_err("persistent heartbeat rejection must fail closed");
    assert!(error.to_string().contains("heartbeat"), "{error}");

    let recorded = shared.lock().unwrap();
    let indices: Vec<i64> = recorded
        .log_bodies
        .iter()
        .map(|body| body["index"].as_i64().unwrap())
        .collect();
    assert!(
        indices.windows(2).all(|pair| pair[0] < pair[1]),
        "accepted log batches must advance monotonically without stale-index replay: {indices:?}"
    );
    let rows = recorded
        .log_bodies
        .iter()
        .flat_map(|body| body["rows"].as_array().unwrap())
        .filter_map(|row| row["content"].as_str())
        .collect::<Vec<_>>();
    assert!(
        rows.iter().any(|row| row.contains("preserved-row")),
        "workflow output must survive lease loss during blocked delivery: {rows:?}"
    );
    assert_eq!(
        rows.iter()
            .filter(|row| row.contains("preserved-row"))
            .count(),
        1,
        "workflow output must not be replayed"
    );
}

#[derive(Default)]
struct FailingPathState {
    events: Mutex<Vec<&'static str>>,
    heartbeat_started: tokio::sync::Notify,
    release_heartbeat: tokio::sync::Notify,
    heartbeat_finished: tokio::sync::Notify,
    log_attempts: Mutex<usize>,
}

struct FailingPathClient {
    shared: Arc<FailingPathState>,
}

struct HeartbeatCancellation<'a> {
    shared: &'a FailingPathState,
    delivered: bool,
}

impl Drop for HeartbeatCancellation<'_> {
    fn drop(&mut self) {
        if !self.delivered {
            self.shared
                .events
                .lock()
                .unwrap()
                .push("heartbeat-cancelled");
            self.shared.heartbeat_finished.notify_one();
        }
    }
}

#[async_trait]
impl GiteaRunnerClient for FailingPathClient {
    async fn register(&self, _: RegisterRequest) -> Result<RunnerInfo> {
        unreachable!()
    }

    async fn declare(&self, _: &RunnerState, _: DeclareRequest) -> Result<DeclareResponse> {
        unreachable!()
    }

    async fn fetch_task(&self, _: &RunnerState, _: i64) -> Result<FetchTaskResponse> {
        unreachable!()
    }

    async fn update_task(
        &self,
        _: &RunnerState,
        req: UpdateTaskRequest,
    ) -> Result<UpdateTaskResponse> {
        let result = req.state.result;
        let is_heartbeat = result == UNSPECIFIED && req.state.started_at.is_none();
        if is_heartbeat {
            let mut cancellation = HeartbeatCancellation {
                shared: &self.shared,
                delivered: false,
            };
            self.shared.heartbeat_started.notify_one();
            self.shared.release_heartbeat.notified().await;
            cancellation.delivered = true;
            self.shared
                .events
                .lock()
                .unwrap()
                .push("heartbeat-delivered");
            self.shared.heartbeat_finished.notify_one();
        } else if result == FAILURE {
            self.shared.events.lock().unwrap().push("terminal");
            self.shared.release_heartbeat.notify_one();
        }
        Ok(UpdateTaskResponse {
            tasks_version: 1,
            sent_outputs: BTreeMap::new(),
        })
    }

    async fn update_log(
        &self,
        _: &RunnerState,
        req: UpdateLogRequest,
    ) -> Result<UpdateLogResponse> {
        let attempt = {
            let mut attempts = self.shared.log_attempts.lock().unwrap();
            *attempts += 1;
            *attempts
        };
        if attempt == 1 {
            self.shared.heartbeat_started.notified().await;
            return Err(RunnerError::Protocol(
                "deterministic pre-terminal UpdateLog failure".into(),
            ));
        }
        Ok(UpdateLogResponse {
            // #3387 uses an exclusive committed-row count, not the older
            // inclusive last-row index used by the R6 baseline.
            ack_index: req.index + req.rows.len() as i64,
        })
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn pre_terminal_error_cancels_and_joins_in_flight_heartbeat_before_repair_terminal() {
    let temp = tempfile::tempdir().unwrap();
    let shared = Arc::new(FailingPathState::default());
    let worker = TaskWorker::new(
        Arc::new(FailingPathClient {
            shared: shared.clone(),
        }),
        Arc::new(TaxonomyPlanner::default_policy(true)),
        "http://127.0.0.1:9",
        temp.path(),
    )
    .with_heartbeat_interval(Duration::from_millis(10));

    let error = worker
        .run(&state(), task(3396, "bash -c 'echo failing-path'"))
        .await
        .expect_err("the injected pre-terminal log failure must propagate");
    assert!(error.to_string().contains("UpdateLog"), "{error}");
    tokio::time::timeout(Duration::from_secs(1), shared.heartbeat_finished.notified())
        .await
        .expect("the owned heartbeat must stop before the run returns");

    let events = shared.events.lock().unwrap().clone();
    assert_eq!(
        events,
        vec!["heartbeat-cancelled", "terminal"],
        "heartbeat cancellation/join must precede repair terminalization"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn transient_heartbeat_transport_failures_do_not_terminate_a_healthy_workflow() {
    let (shared, worker, temp) = harness(HeartbeatBehavior::RejectFirst(2), 3).await;
    let gate = temp.path().join("workflow-release");
    let command = format!(
        "bash -c 'until test -f \"{}\"; do :; done; echo recovered'",
        gate.display()
    );

    assert!(worker.run(&state(), task(3393, &command)).await.unwrap());

    let recorded = shared.lock().unwrap();
    assert!(
        recorded.heartbeat_attempts >= 3,
        "the healthy workflow must survive the two transient failures and receive a successful heartbeat"
    );
    assert_heartbeat_lifecycle(&recorded, 3393, SUCCESS, 3);
}

#[test]
fn production_heartbeat_defaults_encode_the_gitea_1_26_lease_contract() {
    use terraphim_gitea_runner::config::{
        DEFAULT_HEARTBEAT_FAILURE_ATTEMPTS, DEFAULT_HTTP_REQUEST_TIMEOUT,
        GITEA_LEASE_CONTRACT_VERSION, GITEA_LEASE_TIMESTAMP_FIELD, GITEA_ZOMBIE_TASK_TIMEOUT,
    };

    assert_eq!(GITEA_LEASE_CONTRACT_VERSION, "1.26.0");
    assert_eq!(GITEA_LEASE_TIMESTAMP_FIELD, "action_task.updated");
    assert!(
        include_str!("../../../docker-compose-resilient.yml")
            .contains("git.terraphim.cloud/terraphim/gitea:1.26.0"),
        "the tested lease contract version must match the deployed image pin"
    );
    assert_eq!(GITEA_ZOMBIE_TASK_TIMEOUT, Duration::from_secs(10 * 60));
    assert_eq!(
        terraphim_gitea_runner::config::DEFAULT_HEARTBEAT_INTERVAL,
        Duration::from_secs(15)
    );
    assert_eq!(
        terraphim_gitea_runner::config::RunnerConfig::default().heartbeat_interval,
        terraphim_gitea_runner::config::DEFAULT_HEARTBEAT_INTERVAL
    );
    assert_eq!(DEFAULT_HEARTBEAT_FAILURE_ATTEMPTS, 10);
    assert_eq!(
        terraphim_gitea_runner::config::RunnerConfig::default().heartbeat_failure_attempts,
        DEFAULT_HEARTBEAT_FAILURE_ATTEMPTS
    );
    assert!(
        (terraphim_gitea_runner::config::DEFAULT_HEARTBEAT_INTERVAL + DEFAULT_HTTP_REQUEST_TIMEOUT)
            * DEFAULT_HEARTBEAT_FAILURE_ATTEMPTS
            < GITEA_ZOMBIE_TASK_TIMEOUT
    );
}
