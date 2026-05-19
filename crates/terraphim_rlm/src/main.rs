//! CLI entry point for terraphim_rlm.
//!
//! Usage:
//!   terraphim_rlm session create
//!   terraphim_rlm code --session-id <id> < args.json
//!   terraphim_rlm bash --session-id <id> < args.json
//!   terraphim_rlm query --session-id <id> < args.json
//!   terraphim_rlm context --session-id <id> < args.json
//!   terraphim_rlm snapshot --session-id <id> < args.json
//!   terraphim_rlm status --session-id <id> < args.json

use std::io::{self, Read};

use clap::Parser;
use log::info;

use terraphim_rlm::{RlmConfig, SessionId, TerraphimRlm};

mod cli;

use cli::*;

#[tokio::main]
async fn main() {
    // Initialize logging
    env_logger::init();

    let cli = Cli::parse();

    // Execute command
    match run(cli).await {
        Ok(response) => {
            println!("{}", serde_json::to_string_pretty(&response).unwrap());
            std::process::exit(0);
        }
        Err(e) => {
            let response = CliResponse::error("InternalError", e.to_string());
            eprintln!("{}", serde_json::to_string_pretty(&response).unwrap());
            std::process::exit(1);
        }
    }
}

async fn run(cli: Cli) -> Result<CliResponse, Box<dyn std::error::Error>> {
    // Initialize RLM with default config
    // This will fall back to LocalExecutor if Firecracker/Docker unavailable
    let config = RlmConfig::default();
    let rlm = TerraphimRlm::new(config).await?;

    match cli.command {
        Commands::Session { action } => handle_session(&rlm, action).await,
        Commands::Code { session_id } => handle_code(&rlm, &session_id).await,
        Commands::Bash { session_id } => handle_bash(&rlm, &session_id).await,
        Commands::Query { session_id } => handle_query(&rlm, &session_id).await,
        Commands::Context { session_id } => handle_context(&rlm, &session_id).await,
        Commands::Snapshot { session_id } => handle_snapshot(&rlm, &session_id).await,
        Commands::Status { session_id } => handle_status(&rlm, &session_id).await,
    }
}

/// Resolve a session ID, auto-creating one if empty or "auto".
/// Returns (session_id, was_auto_created).
async fn resolve_session(
    rlm: &TerraphimRlm,
    session_id: &str,
) -> Result<(SessionId, bool), Box<dyn std::error::Error>> {
    if session_id.is_empty() || session_id == "auto" {
        let session = rlm.create_session().await?;
        info!("Auto-created session: {}", session.id);
        Ok((session.id, true))
    } else {
        Ok((SessionId::from_string(session_id)?, false))
    }
}

async fn handle_session(
    rlm: &TerraphimRlm,
    action: SessionAction,
) -> Result<CliResponse, Box<dyn std::error::Error>> {
    match action {
        SessionAction::Create => {
            let session = rlm.create_session().await?;
            let response = SessionCreateResponse {
                session_id: session.id.to_string(),
                state: format!("{:?}", session.state),
                created_at: session.created_at.to_string(),
                expires_at: session.expires_at.to_string(),
            };
            Ok(CliResponse::success(response))
        }
        SessionAction::Destroy { session_id } => {
            let sid = SessionId::from_string(&session_id)?;
            rlm.destroy_session(&sid).await?;
            Ok(CliResponse::success(serde_json::json!({"destroyed": true})))
        }
    }
}

async fn handle_code(
    rlm: &TerraphimRlm,
    session_id: &str,
) -> Result<CliResponse, Box<dyn std::error::Error>> {
    let req: CodeRequest = read_stdin_json()?;
    let (sid, auto_created) = resolve_session(rlm, session_id).await?;

    info!("Executing code in session {}", sid);
    let result = rlm.execute_code(&sid, &req.code).await?;
    let success = result.is_success();

    // Cleanup auto-created session
    if auto_created {
        let _ = rlm.destroy_session(&sid).await;
    }

    let response = ExecutionResponse {
        stdout: result.stdout,
        stderr: result.stderr,
        exit_code: result.exit_code,
        execution_time_ms: result.execution_time_ms,
        success,
    };
    Ok(CliResponse::success(response))
}

async fn handle_bash(
    rlm: &TerraphimRlm,
    session_id: &str,
) -> Result<CliResponse, Box<dyn std::error::Error>> {
    let req: BashRequest = read_stdin_json()?;
    let (sid, auto_created) = resolve_session(rlm, session_id).await?;

    info!("Executing bash in session {}", sid);
    let result = rlm.execute_command(&sid, &req.command).await?;
    let success = result.is_success();

    // Cleanup auto-created session
    if auto_created {
        let _ = rlm.destroy_session(&sid).await;
    }

    let response = ExecutionResponse {
        stdout: result.stdout,
        stderr: result.stderr,
        exit_code: result.exit_code,
        execution_time_ms: result.execution_time_ms,
        success,
    };
    Ok(CliResponse::success(response))
}

async fn handle_query(
    rlm: &TerraphimRlm,
    session_id: &str,
) -> Result<CliResponse, Box<dyn std::error::Error>> {
    let req: QueryRequest = read_stdin_json()?;
    let (sid, auto_created) = resolve_session(rlm, session_id).await?;

    info!("Querying LLM in session {}", sid);
    let result = rlm.query_llm(&sid, &req.prompt).await?;

    // Cleanup auto-created session
    if auto_created {
        let _ = rlm.destroy_session(&sid).await;
    }

    let response = QueryResponse {
        response: result.response,
        tokens_used: result.tokens_used,
        model: result.model,
    };
    Ok(CliResponse::success(response))
}

async fn handle_context(
    rlm: &TerraphimRlm,
    session_id: &str,
) -> Result<CliResponse, Box<dyn std::error::Error>> {
    let req: ContextRequest = read_stdin_json()?;
    let (sid, auto_created) = resolve_session(rlm, session_id).await?;

    let result = match req.action.as_str() {
        "get" => {
            let key = req.key.ok_or("Missing 'key' for get")?;
            let value = rlm.get_context_variable(&sid, &key)?;
            let response = ContextResponse {
                action: "get".to_string(),
                key: Some(key),
                value,
                variables: None,
            };
            Ok(CliResponse::success(response))
        }
        "set" => {
            let key = req.key.ok_or("Missing 'key' for set")?;
            let value = req.value.ok_or("Missing 'value' for set")?;
            rlm.set_context_variable(&sid, &key, &value)?;
            let response = ContextResponse {
                action: "set".to_string(),
                key: Some(key),
                value: Some(value),
                variables: None,
            };
            Ok(CliResponse::success(response))
        }
        "list" => {
            let variables = rlm.list_context_variables(&sid).await?;
            let response = ContextResponse {
                action: "list".to_string(),
                key: None,
                value: None,
                variables: Some(variables),
            };
            Ok(CliResponse::success(response))
        }
        "delete" => {
            let key = req.key.ok_or("Missing 'key' for delete")?;
            rlm.delete_context_variable(&sid, &key).await?;
            let response = ContextResponse {
                action: "delete".to_string(),
                key: Some(key),
                value: None,
                variables: None,
            };
            Ok(CliResponse::success(response))
        }
        _ => Err(format!("Invalid action: {}", req.action).into()),
    };

    // Cleanup auto-created session
    if auto_created {
        let _ = rlm.destroy_session(&sid).await;
    }

    result
}

async fn handle_snapshot(
    rlm: &TerraphimRlm,
    session_id: &str,
) -> Result<CliResponse, Box<dyn std::error::Error>> {
    let req: SnapshotRequest = read_stdin_json()?;
    let (sid, auto_created) = resolve_session(rlm, session_id).await?;

    let result = match req.action.as_str() {
        "create" => {
            let name = req
                .snapshot_name
                .ok_or("Missing 'snapshot_name' for create")?;
            let snapshot = rlm.create_snapshot(&sid, &name).await?;
            let response = SnapshotResponse {
                action: "create".to_string(),
                snapshot_name: Some(name),
                snapshot_id: Some(snapshot.name),
                snapshots: None,
            };
            Ok(CliResponse::success(response))
        }
        "restore" => {
            let name = req
                .snapshot_name
                .ok_or("Missing 'snapshot_name' for restore")?;
            rlm.restore_snapshot(&sid, &name).await?;
            let response = SnapshotResponse {
                action: "restore".to_string(),
                snapshot_name: Some(name),
                snapshot_id: None,
                snapshots: None,
            };
            Ok(CliResponse::success(response))
        }
        "list" => {
            let snapshots = rlm.list_snapshots(&sid).await?;
            let names: Vec<String> = snapshots.iter().map(|s| s.name.clone()).collect();
            let response = SnapshotResponse {
                action: "list".to_string(),
                snapshot_name: None,
                snapshot_id: None,
                snapshots: Some(names),
            };
            Ok(CliResponse::success(response))
        }
        "delete" => {
            let name = req
                .snapshot_name
                .ok_or("Missing 'snapshot_name' for delete")?;
            rlm.delete_snapshot(&sid, &name).await?;
            let response = SnapshotResponse {
                action: "delete".to_string(),
                snapshot_name: Some(name),
                snapshot_id: None,
                snapshots: None,
            };
            Ok(CliResponse::success(response))
        }
        _ => Err(format!("Invalid action: {}", req.action).into()),
    };

    // Cleanup auto-created session
    if auto_created {
        let _ = rlm.destroy_session(&sid).await;
    }

    result
}

async fn handle_status(
    rlm: &TerraphimRlm,
    session_id: &str,
) -> Result<CliResponse, Box<dyn std::error::Error>> {
    let req: StatusRequest = read_stdin_json().unwrap_or_default();
    let (sid, auto_created) = resolve_session(rlm, session_id).await?;

    let status = rlm.get_session_status(&sid, req.include_history).await?;

    // Cleanup auto-created session
    if auto_created {
        let _ = rlm.destroy_session(&sid).await;
    }

    Ok(CliResponse::success(status))
}

fn read_stdin_json<T: serde::de::DeserializeOwned>() -> Result<T, Box<dyn std::error::Error>> {
    let mut buffer = String::new();
    io::stdin().read_to_string(&mut buffer)?;
    if buffer.trim().is_empty() {
        return Err("No JSON input provided on stdin".into());
    }
    Ok(serde_json::from_str(&buffer)?)
}
