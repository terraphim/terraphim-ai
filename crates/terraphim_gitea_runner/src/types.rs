//! Connect-JSON wire structs for the Gitea `RunnerService`.
//!
//! Gitea Actions uses Connect-RPC; with `Content-Type: application/json` the
//! bodies follow the proto3 JSON mapping (lowerCamelCase field names) of
//! `code.gitea.io/actions-proto-go`. Shapes are from
//! `gitea/docs/ACTIONS_RUNNERS.md`. Exact field casing for each message is
//! confirmed against a live dev Gitea during dark-launch (plan §B3); the
//! `rename_all = "camelCase"` below is the proto3 JSON default.

use serde::{Deserialize, Deserializer, Serialize};
use std::collections::BTreeMap;

/// Deserialize an `i64` that protojson may encode as either a JSON number or a
/// JSON string. Proto3 JSON maps 64-bit integers to strings (`"2"`) to avoid
/// JavaScript precision loss, so every int64 field Gitea sends must accept both.
fn de_i64<'de, D: Deserializer<'de>>(d: D) -> Result<i64, D::Error> {
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum NumOrStr {
        Num(i64),
        Str(String),
    }
    match NumOrStr::deserialize(d)? {
        NumOrStr::Num(n) => Ok(n),
        NumOrStr::Str(s) => s.trim().parse().map_err(serde::de::Error::custom),
    }
}

/// `runner.v1.Result` enum values (verified against actions-proto-go v0.4.1).
///
/// There is no "running" value: a task is in-progress while `UNSPECIFIED` is
/// reported, and the server treats any non-`UNSPECIFIED` result as terminal
/// (`routers/api/actions/runner/runner.go`: `if State.Result != RESULT_UNSPECIFIED`).
pub mod result {
    /// In-progress / no terminal result yet.
    pub const UNSPECIFIED: i32 = 0;
    /// Job succeeded.
    pub const SUCCESS: i32 = 1;
    /// Job failed.
    pub const FAILURE: i32 = 2;
    /// Job cancelled.
    pub const CANCELLED: i32 = 3;
    /// Job skipped.
    pub const SKIPPED: i32 = 4;
}

/// `Register` request (no auth headers; uses a registration token).
///
/// Fields per `runner.v1.RegisterRequest`: the server reads `token`, `name`,
/// `labels`. There is no `ephemeral` field -- protojson rejects unknown fields,
/// so it must not be sent.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RegisterRequest {
    pub token: String,
    pub name: String,
    pub version: String,
    pub labels: Vec<String>,
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
    #[serde(default, deserialize_with = "de_i64")]
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
    #[serde(default, deserialize_with = "de_i64")]
    pub tasks_version: i64,
}

/// A single assigned task.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Task {
    #[serde(default, deserialize_with = "de_i64")]
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
    /// One of [`result`] (0=unspecified/in-progress, 1=success, 2=failure).
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
    #[serde(default, deserialize_with = "de_i64")]
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
    #[serde(default, deserialize_with = "de_i64")]
    pub ack_index: i64,
}

#[cfg(test)]
mod wire_contract_tests {
    use super::*;

    // Fixtures mirror exactly what Gitea/connect-go emits: protojson default
    // (lowerCamelCase field names), bytes as base64, enums as numbers/strings.
    // Verified against actions-proto-go v0.4.1 field descriptors. If Gitea's wire
    // format ever diverges, these fail rather than the runner silently no-opping.

    #[test]
    fn fetch_task_response_camelcase_deserialises() {
        // `workflowPayload` is base64 (proto bytes); `tasksVersion` camelCase;
        // `context` a JSON object (proto Struct).
        // int64 fields arrive as JSON strings (proto3 JSON), e.g. "42"/"7".
        let json = r#"{
            "task": {
                "id": "42",
                "workflowPayload": "am9iczoKICBqOgogICAgc3RlcHM6CiAgICAgIC0gcnVuOiBlY2hvIGhp",
                "context": {"github": {"repository": "terraphim/proof", "sha": "abc"}},
                "secrets": {},
                "vars": {},
                "needs": {}
            },
            "tasksVersion": "7"
        }"#;
        let resp: FetchTaskResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.tasks_version, 7);
        let task = resp.task.expect("task present");
        assert_eq!(task.id, 42);
        assert!(
            !task.workflow_payload.is_empty(),
            "workflowPayload populated"
        );
        assert_eq!(
            task.context["github"]["repository"], "terraphim/proof",
            "context Struct decoded"
        );
    }

    #[test]
    fn fetch_task_empty_response_deserialises() {
        // Accept both string- and number-encoded int64 (protojson uses strings).
        let resp: FetchTaskResponse = serde_json::from_str(r#"{"tasksVersion": "7"}"#).unwrap();
        assert!(resp.task.is_none());
        assert_eq!(resp.tasks_version, 7);
        let numeric: FetchTaskResponse = serde_json::from_str(r#"{"tasksVersion": 7}"#).unwrap();
        assert_eq!(numeric.tasks_version, 7);
    }

    #[test]
    fn register_response_id_as_string_deserialises() {
        // Exact shape observed from live git.terraphim.cloud (id is a JSON string).
        let json = r#"{"runner":{"id":"2","uuid":"u-1","token":"tok","name":"r","version":"0.1.0","labels":["terraphim-native"]}}"#;
        let resp: RegisterResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.runner.id, 2);
        assert_eq!(resp.runner.uuid, "u-1");
    }

    #[test]
    fn update_log_and_task_responses_deserialise() {
        let log: UpdateLogResponse = serde_json::from_str(r#"{"ackIndex": "5"}"#).unwrap();
        assert_eq!(log.ack_index, 5);
        let task: UpdateTaskResponse =
            serde_json::from_str(r#"{"tasksVersion": "8", "sentOutputs": {}}"#).unwrap();
        assert_eq!(task.tasks_version, 8);
    }

    #[test]
    fn register_request_serialises_without_unknown_fields() {
        // Must contain ONLY token/name/version/labels -- no `ephemeral` (the proto
        // RegisterRequest has no such field and protojson rejects unknown fields).
        let req = RegisterRequest {
            token: "t".into(),
            name: "n".into(),
            version: "0.1.0".into(),
            labels: vec!["terraphim-native".into()],
        };
        let v: serde_json::Value = serde_json::to_value(&req).unwrap();
        let obj = v.as_object().unwrap();
        assert!(!obj.contains_key("ephemeral"), "must not send ephemeral");
        assert_eq!(obj.len(), 4, "exactly token/name/version/labels");
    }
}
