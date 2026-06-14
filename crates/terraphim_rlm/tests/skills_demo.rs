//! Quick integration test: exercise all terraphim_rlm skills in one process.
//! Run: cargo test -p terraphim_rlm --test skills_demo -- --nocapture

use terraphim_rlm::{RlmConfig, TerraphimRlm};

#[tokio::test]
async fn demo_all_skills() {
    let config = RlmConfig::minimal();
    let rlm = TerraphimRlm::with_executor(config, terraphim_rlm::LocalExecutor::new()).unwrap();

    // 1. Session create
    let session = rlm.create_session().await.unwrap();
    println!(
        "[session create] id={} state={:?}",
        session.id, session.state
    );

    // 2. Code execution
    let result = rlm.execute_code(&session.id, "print(2+2)").await.unwrap();
    println!(
        "[code: 2+2] exit={} stdout={:?}",
        result.exit_code, result.stdout
    );
    assert!(result.is_success());
    assert!(result.stdout.trim() == "4");

    // 3. Bash execution
    let result = rlm
        .execute_command(&session.id, "echo hello-rlm")
        .await
        .unwrap();
    println!(
        "[bash: echo] exit={} stdout={:?}",
        result.exit_code, result.stdout
    );
    assert!(result.is_success());
    assert!(result.stdout.trim() == "hello-rlm");

    // 4. Context set
    rlm.set_context_variable(&session.id, "env", "production")
        .unwrap();
    println!("[context set] env=production");

    // 5. Context get
    let val = rlm.get_context_variable(&session.id, "env").unwrap();
    println!("[context get] env={:?}", val);
    assert_eq!(val, Some("production".to_string()));

    // 6. Context list
    let vars = rlm.list_context_variables(&session.id).await.unwrap();
    println!("[context list] variables={:?}", vars);
    assert!(vars.contains_key("env"));

    // 7. Context delete
    rlm.delete_context_variable(&session.id, "env")
        .await
        .unwrap();
    let val = rlm.get_context_variable(&session.id, "env").unwrap();
    println!("[context delete] env after delete={:?}", val);
    assert_eq!(val, None);

    // 8. Status
    let status = rlm.get_session_status(&session.id, false).await.unwrap();
    println!(
        "[status] backend={:?} snapshots={}",
        status.backend_type, status.snapshot_count
    );

    // 9. Snapshot list (Local returns not-supported)
    match rlm.list_snapshots(&session.id).await {
        Ok(s) => println!("[snapshot list] snapshots={:?}", s),
        Err(e) => println!("[snapshot list] expected error: {}", e),
    }

    // 10. Stats
    let stats = rlm.get_stats();
    println!(
        "[stats] total={} active={} with_vm={}",
        stats.total_sessions_created, stats.active_sessions, stats.sessions_with_vm
    );

    // 11. Session destroy
    rlm.destroy_session(&session.id).await.unwrap();
    println!("[session destroy] done");

    // 12. Health check
    let healthy = rlm.health_check().await.unwrap();
    println!("[health check] healthy={}", healthy);
    assert!(healthy);

    println!("\nAll skills demonstrated successfully.");
}
