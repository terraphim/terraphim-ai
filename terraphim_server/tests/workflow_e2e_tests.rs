use axum::http::StatusCode;
use axum_test::TestServer;
use serde_json::json;
use terraphim_server::build_router_for_tests;

fn should_run_workflow_e2e_tests() -> bool {
    std::env::var("RUN_WORKFLOW_E2E_TESTS")
        .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
        .unwrap_or(false)
}

#[tokio::test]
async fn test_prompt_chain_workflow() {
    if !should_run_workflow_e2e_tests() {
        eprintln!("Skipping: set RUN_WORKFLOW_E2E_TESTS=1 to run workflow e2e tests");
        return;
    }
    // Build the test router
    let router = build_router_for_tests().await;
    let server = TestServer::new(router).unwrap();

    // Test prompt chaining workflow
    let response = server
        .post("/workflows/prompt-chain")
        .json(&json!({
            "prompt": "Build a REST API for a todo application",
            "role": "Technical Writer",
            "overall_role": "Software Development Lead"
        }))
        .await;

    assert_eq!(response.status_code(), StatusCode::OK);

    let body = response.json::<serde_json::Value>();
    assert!(body["success"].as_bool().unwrap());
    assert!(body["workflow_id"].as_str().is_some());
    assert_eq!(
        body["metadata"]["pattern"].as_str().unwrap(),
        "PromptChaining"
    );
    assert!(body["metadata"]["execution_time_ms"].as_u64().is_some());

    // Verify the result contains expected chain steps
    let result = &body["result"];
    assert_eq!(result["workflow_type"].as_str().unwrap(), "PromptChaining");
    assert!(result["result"]["steps"].as_array().is_some());

    let steps = result["result"]["steps"].as_array().unwrap();
    assert!(!steps.is_empty());

    // Verify each step has required fields
    for step in steps {
        assert!(step["step_name"].as_str().is_some());
        assert!(step["execution_time_ms"].as_u64().is_some());
        assert!(step["success"].as_bool().is_some());
    }
}

#[tokio::test]
async fn test_routing_workflow() {
    if !should_run_workflow_e2e_tests() {
        eprintln!("Skipping: set RUN_WORKFLOW_E2E_TESTS=1 to run workflow e2e tests");
        return;
    }
    let router = build_router_for_tests().await;
    let server = TestServer::new(router).unwrap();

    // Test routing workflow for different complexity levels
    let test_cases = vec![
        ("Write a simple hello world function", "low"),
        (
            "Design a microservices architecture for an e-commerce platform",
            "high",
        ),
        ("Create a responsive landing page with animations", "medium"),
    ];

    for (prompt, _expected_complexity) in test_cases {
        let response = server
            .post("/workflows/route")
            .json(&json!({
                "prompt": prompt,
                "role": "Technical Writer"
            }))
            .await;

        assert_eq!(response.status_code(), StatusCode::OK);

        let body = response.json::<serde_json::Value>();
        assert!(body["success"].as_bool().unwrap());
        assert_eq!(body["metadata"]["pattern"].as_str().unwrap(), "Routing");

        let result = &body["result"];
        assert_eq!(result["workflow_type"].as_str().unwrap(), "Routing");

        // Verify routing decisions
        let routing_result = &result["result"];
        assert!(routing_result["selected_route"].is_object());
        assert!(
            routing_result["task_analysis"]["complexity"]["level"]
                .as_str()
                .is_some()
        );

        // The complexity detection might not be perfect, but verify it exists
        let complexity = routing_result["task_analysis"]["complexity"]["level"]
            .as_str()
            .unwrap();
        assert!(["low", "medium", "high"].contains(&complexity));
    }
}

#[tokio::test]
async fn test_parallel_workflow() {
    if !should_run_workflow_e2e_tests() {
        eprintln!("Skipping: set RUN_WORKFLOW_E2E_TESTS=1 to run workflow e2e tests");
        return;
    }
    let router = build_router_for_tests().await;
    let server = TestServer::new(router).unwrap();

    let response = server
        .post("/workflows/parallel")
        .json(&json!({
            "prompt": "Analyze the feasibility of implementing blockchain in supply chain management",
            "role": "Business Analyst"
        }))
        .await;

    assert_eq!(response.status_code(), StatusCode::OK);

    let body = response.json::<serde_json::Value>();
    assert!(body["success"].as_bool().unwrap());
    assert_eq!(
        body["metadata"]["pattern"].as_str().unwrap(),
        "Parallelization"
    );

    let result = &body["result"];
    assert_eq!(result["workflow_type"].as_str().unwrap(), "Parallelization");

    // Verify parallel analysis results
    let parallel_result = &result["result"];
    assert!(parallel_result["consensus_points"].as_array().is_some());
    assert!(parallel_result["conflicting_views"].as_array().is_some());
    assert!(parallel_result["comprehensive_analysis"].as_str().is_some());
    assert!(
        parallel_result["execution_summary"]["total_agents"]
            .as_u64()
            .unwrap()
            > 0
    );

    // Verify confidence distribution
    let confidence_dist = parallel_result["confidence_distribution"]
        .as_array()
        .unwrap();
    assert!(!confidence_dist.is_empty());
    for agent_confidence in confidence_dist {
        assert!(agent_confidence["agent_name"].as_str().is_some());
        assert!(agent_confidence["confidence"].as_f64().is_some());
    }
}

#[tokio::test]
async fn test_orchestration_workflow() {
    if !should_run_workflow_e2e_tests() {
        eprintln!("Skipping: set RUN_WORKFLOW_E2E_TESTS=1 to run workflow e2e tests");
        return;
    }
    let router = build_router_for_tests().await;
    let server = TestServer::new(router).unwrap();

    let response = server
        .post("/workflows/orchestrate")
        .json(&json!({
            "prompt": "Build a machine learning pipeline for customer churn prediction",
            "role": "Data Scientist",
            "overall_role": "ML Engineering Lead"
        }))
        .await;

    assert_eq!(response.status_code(), StatusCode::OK);

    let body = response.json::<serde_json::Value>();
    assert!(body["success"].as_bool().unwrap());
    assert_eq!(
        body["metadata"]["pattern"].as_str().unwrap(),
        "Orchestration"
    );

    let result = &body["result"];
    assert_eq!(result["workflow_type"].as_str().unwrap(), "Orchestration");

    // Verify orchestration results
    let orchestration_result = &result["result"];
    let summary = &orchestration_result["orchestrator_summary"];
    assert!(summary["total_tasks_assigned"].as_u64().unwrap() > 0);
    assert!(summary["successful_completions"].as_u64().is_some());
    assert!(summary["resource_efficiency"].as_f64().is_some());

    // Verify worker results
    let worker_results = orchestration_result["worker_results"].as_array().unwrap();
    assert!(!worker_results.is_empty());
    for worker_result in worker_results {
        assert!(worker_result["worker_id"].as_str().is_some());
        assert!(worker_result["task_id"].as_str().is_some());
        assert!(worker_result["quality_score"].as_f64().is_some());
        assert!(worker_result["completion_status"].as_str().is_some());
    }

    // Verify execution timeline
    let timeline = orchestration_result["execution_timeline"]
        .as_array()
        .unwrap();
    assert!(!timeline.is_empty());
}

#[tokio::test]
async fn test_optimization_workflow() {
    if !should_run_workflow_e2e_tests() {
        eprintln!("Skipping: set RUN_WORKFLOW_E2E_TESTS=1 to run workflow e2e tests");
        return;
    }
    let router = build_router_for_tests().await;
    let server = TestServer::new(router).unwrap();

    let response = server
        .post("/workflows/optimize")
        .json(&json!({
            "prompt": "Create compelling marketing copy for a new eco-friendly product",
            "role": "Content Creator"
        }))
        .await;

    assert_eq!(response.status_code(), StatusCode::OK);

    let body = response.json::<serde_json::Value>();
    assert!(body["success"].as_bool().unwrap());
    assert_eq!(
        body["metadata"]["pattern"].as_str().unwrap(),
        "Optimization"
    );

    let result = &body["result"];
    assert_eq!(result["workflow_type"].as_str().unwrap(), "Optimization");

    // Verify optimization results
    let optimization_result = &result["result"];
    let summary = &optimization_result["optimization_summary"];
    assert!(summary["total_iterations"].as_u64().unwrap() > 0);
    assert!(summary["variants_generated"].as_u64().unwrap() > 0);
    assert!(summary["evaluations_performed"].as_u64().unwrap() > 0);
    assert!(summary["final_quality_score"].as_f64().is_some());
    assert!(summary["total_improvement"].as_f64().is_some());

    // Verify iteration history
    let iterations = optimization_result["iteration_history"].as_array().unwrap();
    assert!(!iterations.is_empty());
    for iteration in iterations {
        assert!(iteration["iteration_number"].as_u64().is_some());
        assert!(iteration["generated_variants"].as_array().is_some());
        assert!(iteration["evaluation_results"].as_array().is_some());
        assert!(iteration["improvement_delta"].as_f64().is_some());
    }

    // Verify final optimized content
    let final_content = &optimization_result["final_optimized_content"];
    assert!(final_content["content"].as_str().is_some());
    assert!(
        final_content["quality_metrics"]["overall_quality"]
            .as_f64()
            .is_some()
    );
}

#[tokio::test]
async fn test_workflow_status_endpoint() {
    if !should_run_workflow_e2e_tests() {
        eprintln!("Skipping: set RUN_WORKFLOW_E2E_TESTS=1 to run workflow e2e tests");
        return;
    }
    let router = build_router_for_tests().await;
    let server = TestServer::new(router).unwrap();

    // First create a workflow
    let create_response = server
        .post("/workflows/prompt-chain")
        .json(&json!({
            "prompt": "Test workflow",
            "role": "Test Role"
        }))
        .await;

    assert_eq!(create_response.status_code(), StatusCode::OK);
    let body = create_response.json::<serde_json::Value>();
    let workflow_id = body["workflow_id"].as_str().unwrap();

    // Now check the status
    let status_response = server
        .get(&format!("/workflows/{}/status", workflow_id))
        .await;

    assert_eq!(status_response.status_code(), StatusCode::OK);
    let status_body = status_response.json::<serde_json::Value>();
    assert_eq!(status_body["id"].as_str().unwrap(), workflow_id);
    assert!(status_body["status"].as_str().is_some());
    assert!(status_body["progress"].as_f64().is_some());
}

#[tokio::test]
async fn test_workflow_trace_endpoint() {
    if !should_run_workflow_e2e_tests() {
        eprintln!("Skipping: set RUN_WORKFLOW_E2E_TESTS=1 to run workflow e2e tests");
        return;
    }
    let router = build_router_for_tests().await;
    let server = TestServer::new(router).unwrap();

    // First create a workflow
    let create_response = server
        .post("/workflows/route")
        .json(&json!({
            "prompt": "Build a web scraper",
            "role": "Developer"
        }))
        .await;

    assert_eq!(create_response.status_code(), StatusCode::OK);
    let body = create_response.json::<serde_json::Value>();
    let workflow_id = body["workflow_id"].as_str().unwrap();

    // Now get the execution trace
    let trace_response = server
        .get(&format!("/workflows/{}/trace", workflow_id))
        .await;

    assert_eq!(trace_response.status_code(), StatusCode::OK);
    let trace_body = trace_response.json::<serde_json::Value>();
    assert_eq!(trace_body["workflow_id"].as_str().unwrap(), workflow_id);
    assert!(trace_body["timeline"]["started_at"].as_str().is_some());
    assert!(
        trace_body["performance"]["execution_time_ms"]
            .as_i64()
            .is_some()
    );
}

#[tokio::test]
async fn test_list_workflows_endpoint() {
    if !should_run_workflow_e2e_tests() {
        eprintln!("Skipping: set RUN_WORKFLOW_E2E_TESTS=1 to run workflow e2e tests");
        return;
    }
    let router = build_router_for_tests().await;
    let server = TestServer::new(router).unwrap();

    // Create multiple workflows
    for i in 0..3 {
        let response = server
            .post("/workflows/prompt-chain")
            .json(&json!({
                "prompt": format!("Test workflow {}", i),
                "role": "Test Role"
            }))
            .await;
        assert_eq!(response.status_code(), StatusCode::OK);
    }

    // List all workflows
    let list_response = server.get("/workflows").await;

    assert_eq!(list_response.status_code(), StatusCode::OK);
    let list_body = list_response.json::<Vec<serde_json::Value>>();
    assert!(list_body.len() >= 3);
}

#[tokio::test]
async fn test_workflow_with_custom_config() {
    if !should_run_workflow_e2e_tests() {
        eprintln!("Skipping: set RUN_WORKFLOW_E2E_TESTS=1 to run workflow e2e tests");
        return;
    }
    let router = build_router_for_tests().await;
    let server = TestServer::new(router).unwrap();

    let response = server
        .post("/workflows/parallel")
        .json(&json!({
            "prompt": "Analyze market trends",
            "role": "Market Analyst",
            "overall_role": "Business Strategy Lead",
            "config": {
                "max_agents": 6,
                "timeout_ms": 10000,
                "confidence_threshold": 0.8
            }
        }))
        .await;

    assert_eq!(response.status_code(), StatusCode::OK);
    let body = response.json::<serde_json::Value>();
    assert!(body["success"].as_bool().unwrap());
    assert_eq!(body["metadata"]["role"].as_str().unwrap(), "Market Analyst");
    assert_eq!(
        body["metadata"]["overall_role"].as_str().unwrap(),
        "Business Strategy Lead"
    );
}

#[tokio::test]
async fn test_workflow_error_handling() {
    if !should_run_workflow_e2e_tests() {
        eprintln!("Skipping: set RUN_WORKFLOW_E2E_TESTS=1 to run workflow e2e tests");
        return;
    }
    let router = build_router_for_tests().await;
    let server = TestServer::new(router).unwrap();

    // Test with empty prompt
    let response = server
        .post("/workflows/route")
        .json(&json!({
            "prompt": "",
            "role": "Test Role"
        }))
        .await;

    // Should still return OK but handle gracefully
    assert_eq!(response.status_code(), StatusCode::OK);
    let body = response.json::<serde_json::Value>();
    assert!(body["success"].as_bool().unwrap());
}

#[tokio::test]
async fn test_concurrent_workflows() {
    if !should_run_workflow_e2e_tests() {
        eprintln!("Skipping: set RUN_WORKFLOW_E2E_TESTS=1 to run workflow e2e tests");
        return;
    }
    let router = build_router_for_tests().await;
    let server = TestServer::new(router).unwrap();

    // Launch multiple workflows sequentially (testing concurrent server handling)
    let mut responses = Vec::new();
    for i in 0..5 {
        let response = server
            .post("/workflows/prompt-chain")
            .json(&json!({
                "prompt": format!("Concurrent test {}", i),
                "role": "Test Role"
            }))
            .await;
        responses.push(response);
    }

    // Verify all succeeded
    for response in responses {
        assert_eq!(response.status_code(), StatusCode::OK);
        let body = response.json::<serde_json::Value>();
        assert!(body["success"].as_bool().unwrap());
    }
}

// WebSocket tests would require a different testing approach
// as axum-test doesn't directly support WebSocket testing.
// These would typically be in a separate integration test file.

#[cfg(test)]
mod websocket_tests {
    #[allow(unused_imports)]
    use super::*;

    // Note: WebSocket testing would require using a different testing library
    // or manual WebSocket client implementation. Here's a placeholder for the test structure:

    #[tokio::test]
    #[ignore] // Ignored as it requires special WebSocket testing setup
    async fn test_websocket_workflow_updates() {
        // This would require:
        // 1. Starting the server
        // 2. Connecting a WebSocket client
        // 3. Starting a workflow via HTTP
        // 4. Verifying WebSocket messages are received
        // 5. Testing subscription/unsubscription
    }
}
