//! Manual compound review trigger.
//! Usage: cargo test -p terraphim_orchestrator --test manual_compound_review -- --nocapture

use terraphim_orchestrator::compound::CompoundReviewWorkflow;
use terraphim_orchestrator::config::CompoundReviewConfig;

#[tokio::test]
async fn manual_compound_review() {
    let config = CompoundReviewConfig {
        schedule: "0 2 * * *".to_string(),
        max_duration_secs: 1800,
        repo_path: "/home/alex/terraphim-ai".into(),
        base_branch: "main".to_string(),
        create_prs: false,
        worktree_root: "/home/alex/terraphim-ai/.worktrees".into(),
        cli_tool: "/home/alex/.bun/bin/opencode".to_string(),
        provider: "kimi-for-coding".to_string(),
        model: "k2p5".to_string(),
        groups: vec![],
    };

    let workflow = CompoundReviewWorkflow::from_compound_config(config);
    let result = workflow.run("HEAD", "main").await.unwrap();

    println!("\n===== COMPOUND REVIEW RESULT =====");
    println!("Correlation ID: {}", result.correlation_id);
    println!("Agents run: {}", result.agents_run);
    println!("Agents failed: {}", result.agents_failed);
    println!("Total findings: {}", result.findings.len());
    println!("Pass: {}", result.pass);
    println!("Duration: {:.1}s", result.duration.as_secs_f64());

    for output in &result.agent_outputs {
        println!("\n--- {} ---", output.agent);
        println!("Findings: {}", output.findings.len());
        println!("Pass: {}", output.pass);
        println!("Summary: {}", output.summary);
        for finding in &output.findings {
            println!(
                "  [{:?}] {}:{} - {} (conf: {:.1})",
                finding.severity, finding.file, finding.line, finding.finding, finding.confidence
            );
        }
    }

    assert!(result.agents_run > 0, "Should have run agents");
    assert!(result.findings.len() > 0, "Should have found issues in 8800 lines of changes");
}
