//! Connect-JSON wire structs for the Gitea `RunnerService`.
//!
//! Gitea Actions uses Connect-RPC; with `Content-Type: application/json` the
//! bodies follow the proto3 JSON mapping (lowerCamelCase field names) of
//! `code.gitea.io/actions-proto-go`. Shapes are from
//! `gitea/docs/ACTIONS_RUNNERS.md`. Exact field casing for each message is
//! confirmed against a live dev Gitea during dark-launch (plan §B3); the
//! `rename_all = "camelCase"` below is the proto3 JSON default.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Result/Status enum values used by `UpdateTask` (Gitea `runner.v1.Result`).
pub mod result {
    /// Job still running.
    pub const RUNNING: i32 = 6;
    /// Job succeeded.
    pub const SUCCESS: i32 = 1;
    /// Job failed.
    pub const FAILURE: i32 = 2;
}

/// `Register` request (no auth headers; uses a registration token).
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RegisterRequest {
    pub token: String,
    pub name: String,
    pub version: String,
    pub labels: Vec<String>,
    pub ephemeral: bool,
}

/// `Register` response.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RegisterResponse {
    pub runner: RunnerInfo,
}

/// Runner identity returned by `Register`.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RunnerInfo {
    #[serde(default)]
    pub id: i64,
    pub uuid: String,
    pub token: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub version: String,
    #[serde(default)]
    pub labels: Vec<String>,
    #[serde(default)]
    pub ephemeral: bool,
}

/// `Declare` request (auth via `x-runner-uuid` + `x-runner-token`).
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeclareRequest {
    pub version: String,
    pub labels: Vec<String>,
}

/// `Declare` response.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeclareResponse {
    #[serde(default)]
    pub version: String,
    #[serde(default)]
    pub labels: Vec<String>,
}

/// `FetchTask` request.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FetchTaskRequest {
    pub tasks_version: i64,
}

/// `FetchTask` response. `task` is absent when no work is available.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FetchTaskResponse {
    #[serde(default)]
    pub task: Option<Task>,
    #[serde(default)]
    pub tasks_version: i64,
}

/// A single assigned task.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Task {
    #[serde(default)]
    pub id: i64,
    /// SingleWorkflow YAML for this job, base64-encoded (maybe gzip).
    #[serde(default)]
    pub workflow_payload: String,
    /// `github.*` execution context (repository, sha, ref, ...).
    #[serde(default)]
    pub context: serde_json::Value,
    #[serde(default)]
    pub secrets: BTreeMap<String, String>,
    #[serde(default)]
    pub vars: BTreeMap<String, String>,
    #[serde(default)]
    pub needs: serde_json::Value,
}

/// `UpdateTask` request.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateTaskRequest {
    pub state: TaskState,
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub outputs: BTreeMap<String, String>,
}

/// Task state reported in `UpdateTask`.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskState {
    pub id: i64,
    /// One of [`result`] (1=success, 2=failure, 6=running).
    pub result: i32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub started_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stopped_at: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub steps: Vec<StepState>,
}

/// Per-step state reported in `UpdateTask`.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StepState {
    pub id: i64,
    pub result: i32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub log_index: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub log_length: Option<i64>,
}

/// `UpdateTask` response.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateTaskResponse {
    #[serde(default)]
    pub tasks_version: i64,
    #[serde(default)]
    pub sent_outputs: BTreeMap<String, bool>,
}

/// `UpdateLog` request.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateLogRequest {
    pub task_id: i64,
    /// Index of the first row in this batch.
    pub index: i64,
    pub rows: Vec<LogRow>,
    /// When true, the server finalises the log to permanent storage.
    pub no_more: bool,
}

/// A single log row.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LogRow {
    /// RFC3339 timestamp.
    pub time: String,
    pub content: String,
}

/// `UpdateLog` response.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateLogResponse {
    /// Server's last acknowledged row index.
    #[serde(default)]
    pub ack_index: i64,
}
