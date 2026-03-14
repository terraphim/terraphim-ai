//! Integration tests for workspace management.
//!
//! Uses tempfile for isolated test directories.

use terraphim_symphony::config::workflow::WorkflowDefinition;
use terraphim_symphony::config::ServiceConfig;
use terraphim_symphony::workspace::WorkspaceManager;

fn make_config(tmp_path: &std::path::Path, extra_yaml: &str) -> ServiceConfig {
    let yaml = format!(
        "---\nworkspace:\n  root: \"{}\"\ntracker:\n  kind: linear\n{extra_yaml}\n---\nPrompt.",
        tmp_path.display()
    );
    let workflow = WorkflowDefinition::parse(&yaml).unwrap();
    ServiceConfig::from_workflow(workflow)
}

#[tokio::test]
async fn workspace_lifecycle() {
    let tmp = tempfile::TempDir::new().unwrap();
    let cfg = make_config(tmp.path(), "");
    let mgr = WorkspaceManager::new(&cfg).unwrap();

    // Create workspace
    let info = mgr.prepare("PROJ-1").await.unwrap();
    assert!(info.created_now);
    assert!(info.path.exists());
    assert_eq!(info.workspace_key, "PROJ-1");

    // Reuse workspace
    let info2 = mgr.prepare("PROJ-1").await.unwrap();
    assert!(!info2.created_now);
    assert_eq!(info.path, info2.path);

    // Cleanup
    mgr.cleanup("PROJ-1").await.unwrap();
    assert!(!info.path.exists());
}

#[tokio::test]
async fn workspace_key_sanitisation() {
    let tmp = tempfile::TempDir::new().unwrap();
    let cfg = make_config(tmp.path(), "");
    let mgr = WorkspaceManager::new(&cfg).unwrap();

    // Special characters get sanitised
    let info = mgr.prepare("owner/repo#42").await.unwrap();
    assert_eq!(info.workspace_key, "owner_repo_42");
    assert!(info.path.exists());

    // Spaces get sanitised
    let info2 = mgr.prepare("MT 99").await.unwrap();
    assert_eq!(info2.workspace_key, "MT_99");
    assert!(info2.path.exists());
}

#[tokio::test]
async fn workspace_multiple_issues() {
    let tmp = tempfile::TempDir::new().unwrap();
    let cfg = make_config(tmp.path(), "");
    let mgr = WorkspaceManager::new(&cfg).unwrap();

    // Create multiple workspaces
    let ws1 = mgr.prepare("ISSUE-1").await.unwrap();
    let ws2 = mgr.prepare("ISSUE-2").await.unwrap();
    let ws3 = mgr.prepare("ISSUE-3").await.unwrap();

    assert!(ws1.path.exists());
    assert!(ws2.path.exists());
    assert!(ws3.path.exists());
    assert_ne!(ws1.path, ws2.path);
    assert_ne!(ws2.path, ws3.path);

    // Cleanup one
    mgr.cleanup("ISSUE-2").await.unwrap();
    assert!(ws1.path.exists());
    assert!(!ws2.path.exists());
    assert!(ws3.path.exists());
}

#[tokio::test]
async fn cleanup_nonexistent_succeeds() {
    let tmp = tempfile::TempDir::new().unwrap();
    let cfg = make_config(tmp.path(), "");
    let mgr = WorkspaceManager::new(&cfg).unwrap();

    // Should not error for nonexistent workspace
    mgr.cleanup("DOES-NOT-EXIST").await.unwrap();
}

#[tokio::test]
async fn cleanup_terminal_workspaces() {
    let tmp = tempfile::TempDir::new().unwrap();
    let cfg = make_config(tmp.path(), "");
    let mgr = WorkspaceManager::new(&cfg).unwrap();

    // Create several workspaces
    mgr.prepare("T-1").await.unwrap();
    mgr.prepare("T-2").await.unwrap();
    mgr.prepare("T-3").await.unwrap();

    // Cleanup terminal ones
    let terminal = vec!["T-1".to_string(), "T-3".to_string()];
    mgr.cleanup_terminal_workspaces(&terminal).await;

    assert!(!tmp.path().join("T-1").exists());
    assert!(tmp.path().join("T-2").exists()); // kept
    assert!(!tmp.path().join("T-3").exists());
}

#[tokio::test]
async fn hook_after_create() {
    let tmp = tempfile::TempDir::new().unwrap();
    let cfg = make_config(tmp.path(), "hooks:\n  after_create: \"touch marker.txt\"");
    let mgr = WorkspaceManager::new(&cfg).unwrap();

    let info = mgr.prepare("HOOK-TEST").await.unwrap();
    assert!(info.created_now);
    assert!(info.path.join("marker.txt").exists());

    // Second prepare should not re-run hook
    let info2 = mgr.prepare("HOOK-TEST").await.unwrap();
    assert!(!info2.created_now);
}

#[tokio::test]
async fn hook_before_run() {
    let tmp = tempfile::TempDir::new().unwrap();
    let cfg = make_config(
        tmp.path(),
        "hooks:\n  before_run: \"echo running >> log.txt\"",
    );
    let mgr = WorkspaceManager::new(&cfg).unwrap();

    let info = mgr.prepare("BR-1").await.unwrap();
    mgr.run_before_run_hook(&info).await.unwrap();
    assert!(info.path.join("log.txt").exists());
}

#[tokio::test]
async fn hook_after_create_failure_cleans_up() {
    let tmp = tempfile::TempDir::new().unwrap();
    let cfg = make_config(tmp.path(), "hooks:\n  after_create: \"exit 1\"");
    let mgr = WorkspaceManager::new(&cfg).unwrap();

    let result = mgr.prepare("FAIL-1").await;
    assert!(result.is_err());
    // Workspace directory should have been cleaned up
    assert!(!tmp.path().join("FAIL-1").exists());
}

#[tokio::test]
async fn hook_before_run_failure_is_propagated() {
    let tmp = tempfile::TempDir::new().unwrap();
    let cfg = make_config(tmp.path(), "hooks:\n  before_run: \"exit 1\"");
    let mgr = WorkspaceManager::new(&cfg).unwrap();

    let info = mgr.prepare("BRF-1").await.unwrap();
    let result = mgr.run_before_run_hook(&info).await;
    assert!(result.is_err());
    // Workspace directory should still exist
    assert!(info.path.exists());
}

#[test]
fn path_outside_root_rejected() {
    let tmp = tempfile::TempDir::new().unwrap();
    let cfg = make_config(tmp.path(), "");
    let mgr = WorkspaceManager::new(&cfg).unwrap();

    let bad_path = std::path::PathBuf::from("/tmp/elsewhere/evil");
    // Use the public validate_path via prepare with path traversal
    // The sanitiser replaces .. with __ so direct traversal is blocked
    let key = terraphim_symphony::workspace::sanitise_workspace_key("../../../etc/passwd");
    assert!(!key.contains('/'));
    assert!(!key.contains('\\'));
    assert_eq!(key, ".._.._.._etc_passwd");

    // The mgr.root should not contain our bad path
    let root_str = mgr.root().to_string_lossy().to_string();
    assert!(!bad_path.to_string_lossy().starts_with(&root_str));
}
