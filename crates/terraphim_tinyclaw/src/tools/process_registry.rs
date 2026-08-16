//! Background-process registry and `process` tool.
//!
//! Port of Hermes `tools/process_registry.py` (core subset): spawn a
//! background process with rolling output buffering, then poll / read-log /
//! wait / kill / write-stdin / list.

use crate::tools::{Tool, ToolError};
use async_trait::async_trait;
use serde::Serialize;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::sync::Mutex as TokioMutex;

/// Rolling output buffer cap (matches Hermes `MAX_OUTPUT_CHARS`).
pub const MAX_OUTPUT_CHARS: usize = 200_000;

fn now_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

/// Append to a rolling buffer, keeping only the last `max` chars.
fn append_output(buf: &Arc<Mutex<String>>, data: &str, max: usize) {
    let mut b = buf.lock().unwrap();
    b.push_str(data);
    if b.len() > max {
        let cut = b.len() - max;
        let mut idx = cut;
        while idx < b.len() && !b.is_char_boundary(idx) {
            idx += 1;
        }
        let tail = b[idx..].to_string();
        *b = tail;
    }
}

/// Snapshot of a tracked background process.
#[derive(Debug, Clone, Serialize)]
pub struct ProcessSession {
    pub id: String,
    pub command: String,
    pub task_id: String,
    pub session_key: String,
    pub pid: Option<u32>,
    pub cwd: Option<PathBuf>,
    pub started_at: u64,
    pub exited: bool,
    pub exit_code: Option<i32>,
}

/// Internal handle: session + child process + optional stdin.
struct Managed {
    session: Mutex<ProcessSession>,
    output: Arc<Mutex<String>>,
    child: TokioMutex<tokio::process::Child>,
    stdin: Option<TokioMutex<tokio::process::ChildStdin>>,
}

/// Thread-safe registry of running and finished background processes.
#[derive(Default)]
pub struct ProcessRegistry {
    processes: Mutex<HashMap<String, Arc<Managed>>>,
}

impl ProcessRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    fn get(&self, id: &str) -> Option<Arc<Managed>> {
        self.processes.lock().unwrap().get(id).cloned()
    }

    fn snapshot(managed: &Managed) -> ProcessSession {
        managed.session.lock().unwrap().clone()
    }

    fn output_of(managed: &Managed) -> String {
        managed.output.lock().unwrap().clone()
    }

    /// Spawn a background process and begin buffering its output.
    pub async fn spawn(
        &self,
        command: &str,
        cwd: Option<PathBuf>,
        task_id: &str,
        session_key: &str,
    ) -> Result<String, ToolError> {
        let id = format!("proc_{}", now_millis());

        let mut cmd = tokio::process::Command::new("sh");
        cmd.arg("-c")
            .arg(command)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped());
        if let Some(ref cwd) = cwd {
            cmd.current_dir(cwd);
        }

        let mut child = cmd.spawn().map_err(|e| ToolError::ExecutionFailed {
            tool: "process".to_string(),
            message: format!("Failed to spawn process: {}", e),
        })?;

        let pid = child.id();
        let output: Arc<Mutex<String>> = Arc::new(Mutex::new(String::new()));

        if let Some(mut stdout) = child.stdout.take() {
            let buf = output.clone();
            tokio::spawn(async move {
                use tokio::io::AsyncReadExt;
                let mut chunk = [0u8; 4096];
                loop {
                    match stdout.read(&mut chunk).await {
                        Ok(0) | Err(_) => break,
                        Ok(n) => {
                            let s = String::from_utf8_lossy(&chunk[..n]);
                            append_output(&buf, &s, MAX_OUTPUT_CHARS);
                        }
                    }
                }
            });
        }
        if let Some(mut stderr) = child.stderr.take() {
            let buf = output.clone();
            tokio::spawn(async move {
                use tokio::io::AsyncReadExt;
                let mut chunk = [0u8; 4096];
                loop {
                    match stderr.read(&mut chunk).await {
                        Ok(0) | Err(_) => break,
                        Ok(n) => {
                            let s = String::from_utf8_lossy(&chunk[..n]);
                            append_output(&buf, &s, MAX_OUTPUT_CHARS);
                        }
                    }
                }
            });
        }

        let stdin = child.stdin.take().map(TokioMutex::new);

        let session = ProcessSession {
            id: id.clone(),
            command: command.to_string(),
            task_id: task_id.to_string(),
            session_key: session_key.to_string(),
            pid,
            cwd,
            started_at: now_millis(),
            exited: false,
            exit_code: None,
        };

        self.processes.lock().unwrap().insert(
            id.clone(),
            Arc::new(Managed {
                session: Mutex::new(session),
                output,
                child: TokioMutex::new(child),
                stdin,
            }),
        );

        Ok(id)
    }

    /// Poll a process, updating exit state if it has finished.
    pub async fn poll(&self, session_id: &str) -> serde_json::Value {
        let Some(m) = self.get(session_id) else {
            return serde_json::json!({"error": "process not found"});
        };
        let mut child = m.child.lock().await;
        if let Ok(Some(status)) = child.try_wait() {
            let mut session = m.session.lock().unwrap();
            session.exited = true;
            session.exit_code = status.code();
        }
        drop(child);
        let session = Self::snapshot(&m);
        serde_json::json!({
            "session_id": session.id,
            "command": session.command,
            "pid": session.pid,
            "exited": session.exited,
            "exit_code": session.exit_code,
            "output": Self::output_of(&m),
        })
    }

    /// Read log lines from a process output buffer.
    pub fn read_log(&self, session_id: &str, offset: usize, limit: usize) -> serde_json::Value {
        let Some(m) = self.get(session_id) else {
            return serde_json::json!({"error": "process not found"});
        };
        let output = Self::output_of(&m);
        let lines: Vec<&str> = output.lines().collect();
        let total = lines.len();
        let slice: Vec<String> = lines
            .into_iter()
            .skip(offset)
            .take(limit)
            .map(str::to_string)
            .collect();
        serde_json::json!({
            "session_id": session_id,
            "total_lines": total,
            "offset": offset,
            "lines": slice,
        })
    }

    /// Block until the process exits or the timeout elapses.
    pub async fn wait(&self, session_id: &str, timeout_secs: Option<u64>) -> serde_json::Value {
        let Some(m) = self.get(session_id) else {
            return serde_json::json!({"error": "process not found"});
        };
        let mut child = m.child.lock().await;

        let result = match timeout_secs {
            Some(secs) => {
                match tokio::time::timeout(Duration::from_secs(secs), child.wait()).await {
                    Ok(res) => res,
                    Err(_) => {
                        return serde_json::json!({
                            "status": "timeout",
                            "session_id": session_id,
                            "output": Self::output_of(&m),
                        });
                    }
                }
            }
            None => child.wait().await,
        };
        drop(child);

        match result {
            Ok(status) => {
                let mut session = m.session.lock().unwrap();
                session.exited = true;
                session.exit_code = status.code();
                let session = session.clone();
                serde_json::json!({
                    "status": "exited",
                    "session_id": session.id,
                    "exit_code": session.exit_code,
                    "output": Self::output_of(&m),
                })
            }
            Err(e) => serde_json::json!({"error": format!("wait failed: {}", e)}),
        }
    }

    /// Kill a process.
    pub async fn kill(&self, session_id: &str) -> serde_json::Value {
        let Some(m) = self.get(session_id) else {
            return serde_json::json!({"error": "process not found"});
        };
        let mut child = m.child.lock().await;
        match child.start_kill() {
            Ok(()) => serde_json::json!({"status": "killed", "session_id": session_id}),
            Err(e) => serde_json::json!({"error": format!("kill failed: {}", e)}),
        }
    }

    /// Write raw data to process stdin (no trailing newline).
    pub async fn write_stdin(&self, session_id: &str, data: &str) -> serde_json::Value {
        let Some(m) = self.get(session_id) else {
            return serde_json::json!({"error": "process not found"});
        };
        let Some(stdin) = &m.stdin else {
            return serde_json::json!({"error": "process stdin unavailable"});
        };
        use tokio::io::AsyncWriteExt;
        match stdin.lock().await.write_all(data.as_bytes()).await {
            Ok(()) => serde_json::json!({"status": "written", "session_id": session_id}),
            Err(e) => serde_json::json!({"error": format!("write failed: {}", e)}),
        }
    }

    /// Write data + newline (submit an answer to a prompt).
    pub async fn submit_stdin(&self, session_id: &str, data: &str) -> serde_json::Value {
        self.write_stdin(session_id, &format!("{}\n", data)).await
    }

    /// List sessions, optionally filtered by task_id.
    pub fn list_sessions(&self, task_id: Option<&str>) -> Vec<ProcessSession> {
        let procs = self.processes.lock().unwrap();
        let mut out: Vec<ProcessSession> = procs
            .values()
            .map(|m| m.session.lock().unwrap().clone())
            .filter(|s| match task_id {
                Some(t) if !t.is_empty() => s.task_id == t,
                _ => true,
            })
            .collect();
        out.sort_by_key(|s| s.started_at);
        out
    }

    /// Whether any non-exited process exists for the given task_id.
    pub fn has_active(&self, task_id: &str) -> bool {
        let procs = self.processes.lock().unwrap();
        procs.values().any(|m| {
            let s = m.session.lock().unwrap();
            s.task_id == task_id && !s.exited
        })
    }
}

/// The `process` tool. Manages background processes via the registry.
pub struct ProcessTool {
    registry: Arc<ProcessRegistry>,
}

impl ProcessTool {
    pub fn new(registry: Arc<ProcessRegistry>) -> Self {
        Self { registry }
    }
}

#[async_trait]
impl Tool for ProcessTool {
    fn name(&self) -> &str {
        "process"
    }

    fn description(&self) -> &str {
        "Manage background processes started with a terminal/shell background \
         run. Actions: 'list' (show all), 'poll' (status + new output), \
         'log' (output with pagination), 'wait' (block until done or timeout), \
         'kill' (terminate), 'write' (raw stdin), 'submit' (data + Enter)."
    }

    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "enum": ["list", "poll", "log", "wait", "kill", "write", "submit"]
                },
                "session_id": { "type": "string" },
                "data": { "type": "string" },
                "timeout": { "type": "integer", "minimum": 1 },
                "offset": { "type": "integer" },
                "limit": { "type": "integer", "minimum": 1 }
            },
            "required": ["action"]
        })
    }

    async fn execute(&self, args: serde_json::Value) -> Result<String, ToolError> {
        let action = args["action"]
            .as_str()
            .ok_or_else(|| ToolError::InvalidArguments {
                tool: "process".to_string(),
                message: "Missing required 'action' parameter".to_string(),
            })?;

        let session_id = args
            .get("session_id")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        let result = match action {
            "list" => serde_json::json!({"processes": self.registry.list_sessions(None)}),
            "poll" | "log" | "wait" | "kill" | "write" | "submit" if session_id.is_empty() => {
                return Err(ToolError::InvalidArguments {
                    tool: "process".to_string(),
                    message: format!("session_id is required for {action}"),
                });
            }
            "poll" => self.registry.poll(session_id).await,
            "log" => {
                let offset = args["offset"].as_u64().unwrap_or(0) as usize;
                let limit = args["limit"].as_u64().unwrap_or(200) as usize;
                self.registry.read_log(session_id, offset, limit)
            }
            "wait" => {
                let timeout = args["timeout"].as_u64();
                self.registry.wait(session_id, timeout).await
            }
            "kill" => self.registry.kill(session_id).await,
            "write" => {
                let data = args["data"].as_str().unwrap_or("");
                self.registry.write_stdin(session_id, data).await
            }
            "submit" => {
                let data = args["data"].as_str().unwrap_or("");
                self.registry.submit_stdin(session_id, data).await
            }
            other => {
                return Err(ToolError::InvalidArguments {
                    tool: "process".to_string(),
                    message: format!("Unknown process action: {other}"),
                });
            }
        };

        serde_json::to_string(&result).map_err(|e| ToolError::ExecutionFailed {
            tool: "process".to_string(),
            message: format!("Failed to serialise result: {}", e),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn append_output_rolls_over() {
        let buf = Arc::new(Mutex::new(String::new()));
        append_output(&buf, "0123456789", 5);
        assert_eq!(*buf.lock().unwrap(), "56789");
    }

    #[test]
    fn append_output_under_cap() {
        let buf = Arc::new(Mutex::new(String::new()));
        append_output(&buf, "hi", 100);
        assert_eq!(*buf.lock().unwrap(), "hi");
    }

    #[tokio::test]
    async fn spawn_and_poll_completes() {
        let registry = ProcessRegistry::new();
        let id = registry.spawn("echo hello", None, "", "").await.unwrap();
        tokio::time::sleep(Duration::from_millis(200)).await;
        let poll = registry.poll(&id).await;
        assert_eq!(poll["session_id"], id);
        assert!(poll["exited"].as_bool().unwrap());
        assert!(poll["output"].as_str().unwrap().contains("hello"));
    }

    #[tokio::test]
    async fn kill_long_running() {
        let registry = ProcessRegistry::new();
        let id = registry.spawn("sleep 30", None, "", "").await.unwrap();
        let killed = registry.kill(&id).await;
        assert_eq!(killed["status"], "killed");
    }

    #[tokio::test]
    async fn list_and_has_active() {
        let registry = ProcessRegistry::new();
        let id = registry
            .spawn("sleep 30", None, "task-x", "")
            .await
            .unwrap();
        assert!(registry.has_active("task-x"));
        assert_eq!(registry.list_sessions(Some("task-x")).len(), 1);
        registry.kill(&id).await;
    }

    #[tokio::test]
    async fn read_log_paginates() {
        let registry = ProcessRegistry::new();
        let id = registry
            .spawn("printf 'a\\nb\\nc\\n'", None, "", "")
            .await
            .unwrap();
        tokio::time::sleep(Duration::from_millis(200)).await;
        let log = registry.read_log(&id, 0, 2);
        assert_eq!(log["lines"].as_array().unwrap().len(), 2);
    }

    #[tokio::test]
    async fn tool_unknown_action_is_error() {
        let tool = ProcessTool::new(Arc::new(ProcessRegistry::new()));
        let result = tool.execute(serde_json::json!({"action": "bogus"})).await;
        assert!(matches!(result, Err(ToolError::InvalidArguments { .. })));
    }

    #[tokio::test]
    async fn tool_missing_session_id_is_error() {
        let tool = ProcessTool::new(Arc::new(ProcessRegistry::new()));
        let result = tool.execute(serde_json::json!({"action": "poll"})).await;
        assert!(matches!(result, Err(ToolError::InvalidArguments { .. })));
    }
}
