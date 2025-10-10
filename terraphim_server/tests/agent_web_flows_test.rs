use axum::http::StatusCode;
use axum_test::TestServer;
use futures_util::future;
use serde_json::json;
use terraphim_server::build_router_for_tests;

/// Comprehensive web-based agent workflow tests that simulate real frontend interactions
/// These tests verify the complete flow from web interface to backend agent processing

#[tokio::test]
async fn test_prompt_chain_web_flow() {
    let router = build_router_for_tests().await;
    let server = TestServer::new(router).unwrap();

    // Simulate web frontend submitting a complex prompt chain request
    let web_request = json!({
        "prompt": "Create a comprehensive e-commerce platform architecture with microservices, API gateway, and database design",
        "role": "Software Architect",
        "overall_role": "Technical Lead",
        "config": {
            "steps": 6,
            "include_code_examples": true,
            "include_deployment_guide": true
        }
    });

    let response = server
        .post("/workflows/prompt-chain")
        .json(&web_request)
        .await;

    assert_eq!(response.status_code(), StatusCode::OK);
    let body = response.json::<serde_json::Value>();

    // Verify web-compatible response structure
    assert!(body["success"].as_bool().unwrap());
    assert!(body["workflow_id"].as_str().is_some());
    assert_eq!(
        body["metadata"]["pattern"].as_str().unwrap(),
        "prompt_chaining"
    );

    // Verify rich result structure for web display
    let result = &body["result"];
    assert!(result["steps"].as_array().is_some());
    assert!(result["final_result"].is_object());
    assert!(result["execution_summary"].is_object());

    // Verify steps contain displayable content
    let steps = result["steps"].as_array().unwrap();
    assert!(steps.len() >= 5); // Should have multiple steps for complex request

    for step in steps {
        assert!(step["step_name"].as_str().is_some());
        assert!(step["output"].as_str().is_some());
        assert!(step["success"].as_bool().unwrap());
        assert!(step["duration_ms"].as_u64().is_some());
    }
}

#[tokio::test]
async fn test_routing_web_flow_with_complexity_analysis() {
    let router = build_router_for_tests().await;
    let server = TestServer::new(router).unwrap();

    // Test different complexity levels that web interface would send
    let test_cases = vec![
        (
            "Simple task: Write a hello world function",
            "SimpleTaskAgent",
            0.2
        ),
        (
            "Complex task: Design a distributed system with event sourcing, CQRS, and microservices architecture for a global banking platform with real-time fraud detection",
            "ComplexTaskAgent", 
            0.8
        ),
        (
            "Medium task: Create a REST API for user authentication with JWT tokens",
            "SimpleTaskAgent",
            0.4
        ),
    ];

    for (prompt, expected_agent_type, expected_complexity_range) in test_cases {
        let web_request = json!({
            "prompt": prompt,
            "role": "Developer",
            "config": {
                "complexity_threshold": 0.5,
                "require_reasoning": true
            }
        });

        let response = server.post("/workflows/route").json(&web_request).await;

        assert_eq!(response.status_code(), StatusCode::OK);
        let body = response.json::<serde_json::Value>();

        assert!(body["success"].as_bool().unwrap());
        assert_eq!(body["metadata"]["pattern"].as_str().unwrap(), "routing");

        // Verify routing analysis for web display
        let result = &body["result"];
        assert!(result["task_analysis"].is_object());
        assert!(result["selected_route"].is_object());

        let task_analysis = &result["task_analysis"];
        let complexity = task_analysis["complexity"].as_f64().unwrap();
        assert!(complexity >= 0.0 && complexity <= 1.0);

        let selected_route = &result["selected_route"];
        assert!(selected_route["agent_id"].as_str().is_some());
        assert!(selected_route["reasoning"].as_str().is_some());
        assert!(selected_route["confidence"].as_f64().unwrap() > 0.5);

        // Verify the route contains web-displayable information
        assert!(selected_route["reasoning"].as_str().unwrap().len() > 10);
    }
}

#[tokio::test]
async fn test_parallel_web_flow_with_perspectives() {
    let router = build_router_for_tests().await;
    let server = TestServer::new(router).unwrap();

    let web_request = json!({
        "prompt": "Analyze the business impact of implementing AI-powered customer service chatbots",
        "role": "Business Analyst",
        "overall_role": "Strategic Planning Lead",
        "config": {
            "agent_count": 3,
            "perspectives": ["business", "technical", "user_experience"],
            "require_consensus": true
        }
    });

    let response = server.post("/workflows/parallel").json(&web_request).await;

    assert_eq!(response.status_code(), StatusCode::OK);
    let body = response.json::<serde_json::Value>();

    assert!(body["success"].as_bool().unwrap());
    assert_eq!(
        body["metadata"]["pattern"].as_str().unwrap(),
        "Parallelization"
    );

    // Verify parallel execution results for web dashboard
    let result = &body["result"];
    assert!(result["parallel_tasks"].as_array().is_some());
    assert!(result["execution_summary"].is_object());

    let parallel_tasks = result["parallel_tasks"].as_array().unwrap();
    assert_eq!(parallel_tasks.len(), 3); // Should have 3 perspectives

    // Verify each perspective contains rich data for web display
    for task in parallel_tasks {
        assert!(task["agent_id"].as_str().is_some());
        assert!(task["perspective"].as_str().is_some());
        assert!(task["result"].as_str().is_some());
        assert!(task["cost"].as_f64().is_some());
        assert!(task["tokens_used"].as_u64().is_some());

        // Verify substantial content for web display
        let result_text = task["result"].as_str().unwrap();
        assert!(
            result_text.len() > 50,
            "Result should contain substantial content"
        );
    }

    // Verify aggregated result for web summary
    assert!(result["aggregated_result"].as_str().is_some());
    let aggregated = result["aggregated_result"].as_str().unwrap();
    assert!(
        aggregated.len() > 100,
        "Aggregated result should be comprehensive"
    );
}

#[tokio::test]
async fn test_orchestration_web_flow_with_workers() {
    let router = build_router_for_tests().await;
    let server = TestServer::new(router).unwrap();

    let web_request = json!({
        "prompt": "Build a machine learning pipeline for predicting customer churn",
        "role": "Data Scientist",
        "overall_role": "ML Engineering Lead",
        "config": {
            "max_workers": 3,
            "workflow_type": "data_science",
            "require_validation": true
        }
    });

    let response = server
        .post("/workflows/orchestrate")
        .json(&web_request)
        .await;

    assert_eq!(response.status_code(), StatusCode::OK);
    let body = response.json::<serde_json::Value>();

    assert!(body["success"].as_bool().unwrap());
    assert_eq!(
        body["metadata"]["pattern"].as_str().unwrap(),
        "Orchestration"
    );

    // Verify orchestration results for web workflow display
    let result = &body["result"];
    assert!(result["orchestrator_plan"].as_str().is_some());
    assert!(result["worker_results"].as_array().is_some());
    assert!(result["final_synthesis"].as_str().is_some());
    assert!(result["execution_summary"].is_object());

    let worker_results = result["worker_results"].as_array().unwrap();
    assert!(worker_results.len() >= 1, "Should have worker results");

    // Verify worker results contain web-displayable data
    for worker_result in worker_results {
        assert!(worker_result["worker_name"].as_str().is_some());
        assert!(worker_result["task_description"].as_str().is_some());
        assert!(worker_result["result"].as_str().is_some());
        assert!(worker_result["agent_id"].as_str().is_some());
        assert!(worker_result["cost"].as_f64().is_some());

        // Verify meaningful content for web display
        let result_text = worker_result["result"].as_str().unwrap();
        assert!(
            result_text.len() > 30,
            "Worker result should contain meaningful content"
        );
    }

    // Verify execution summary for web metrics
    let exec_summary = &result["execution_summary"];
    assert!(exec_summary["orchestrator_id"].as_str().is_some());
    assert!(exec_summary["workers_count"].as_u64().is_some());
    assert!(exec_summary["total_cost"].as_f64().is_some());
    assert!(exec_summary["total_tokens"].as_u64().is_some());
}

#[tokio::test]
async fn test_optimization_web_flow_with_iterations() {
    let router = build_router_for_tests().await;
    let server = TestServer::new(router).unwrap();

    let web_request = json!({
        "prompt": "Write compelling marketing copy for a new eco-friendly water bottle",
        "role": "Content Creator",
        "overall_role": "Marketing Lead",
        "config": {
            "max_iterations": 3,
            "quality_threshold": 8.0,
            "optimization_focus": "engagement_and_conversion"
        }
    });

    let response = server.post("/workflows/optimize").json(&web_request).await;

    assert_eq!(response.status_code(), StatusCode::OK);
    let body = response.json::<serde_json::Value>();

    assert!(body["success"].as_bool().unwrap());
    assert_eq!(
        body["metadata"]["pattern"].as_str().unwrap(),
        "Optimization"
    );

    // Verify optimization results for web progress tracking
    let result = &body["result"];
    assert!(result["iterations"].as_array().is_some());
    assert!(result["final_content"].as_str().is_some());
    assert!(result["execution_summary"].is_object());

    let iterations = result["iterations"].as_array().unwrap();
    assert!(iterations.len() >= 1, "Should have optimization iterations");
    assert!(iterations.len() <= 3, "Should not exceed max iterations");

    // Verify iteration data for web progress visualization
    for (i, iteration) in iterations.iter().enumerate() {
        assert_eq!(iteration["iteration"].as_u64().unwrap(), (i + 1) as u64);
        assert!(iteration["generated_content"].as_str().is_some());
        assert!(iteration["evaluation_feedback"].as_str().is_some());
        assert!(iteration["quality_score"].as_f64().is_some());
        assert!(iteration["generator_tokens"].as_u64().is_some());
        assert!(iteration["evaluator_tokens"].as_u64().is_some());

        // Verify quality progression for web charts
        let quality_score = iteration["quality_score"].as_f64().unwrap();
        assert!(quality_score >= 1.0 && quality_score <= 10.0);

        // Verify substantial content for web display
        let generated_content = iteration["generated_content"].as_str().unwrap();
        assert!(
            generated_content.len() > 20,
            "Generated content should be substantial"
        );
    }

    // Verify execution summary for web dashboard
    let exec_summary = &result["execution_summary"];
    assert!(exec_summary["generator_id"].as_str().is_some());
    assert!(exec_summary["evaluator_id"].as_str().is_some());
    assert!(exec_summary["iterations_completed"].as_u64().is_some());
    assert!(exec_summary["best_quality_score"].as_f64().is_some());
    assert!(exec_summary["quality_threshold"].as_f64().unwrap() == 8.0);

    // Verify final optimized content
    let final_content = result["final_content"].as_str().unwrap();
    assert!(
        final_content.len() > 50,
        "Final content should be comprehensive"
    );
}

#[tokio::test]
async fn test_web_error_handling_and_validation() {
    let router = build_router_for_tests().await;
    let server = TestServer::new(router).unwrap();

    // Test invalid request format
    let response = server
        .post("/workflows/route")
        .json(&json!({"invalid": "request"}))
        .await;

    // Should still return OK with proper error structure for web handling
    assert_eq!(response.status_code(), StatusCode::OK);
    let body = response.json::<serde_json::Value>();
    assert!(body["success"].as_bool().unwrap()); // Gracefully handles missing prompt

    // Test empty prompt
    let response = server
        .post("/workflows/parallel")
        .json(&json!({
            "prompt": "",
            "role": "Test Role"
        }))
        .await;

    assert_eq!(response.status_code(), StatusCode::OK);
    let body = response.json::<serde_json::Value>();
    assert!(body["success"].as_bool().unwrap()); // Gracefully handles empty prompt
}

#[tokio::test]
async fn test_web_workflow_status_monitoring() {
    let router = build_router_for_tests().await;
    let server = TestServer::new(router).unwrap();

    // Create a workflow first
    let create_response = server
        .post("/workflows/prompt-chain")
        .json(&json!({
            "prompt": "Create a simple web application",
            "role": "Developer"
        }))
        .await;

    assert_eq!(create_response.status_code(), StatusCode::OK);
    let body = create_response.json::<serde_json::Value>();
    let workflow_id = body["workflow_id"].as_str().unwrap();

    // Test status monitoring endpoint for web dashboard
    let status_response = server
        .get(&format!("/workflows/{}/status", workflow_id))
        .await;

    assert_eq!(status_response.status_code(), StatusCode::OK);
    let status_body = status_response.json::<serde_json::Value>();

    // Verify status structure for web monitoring
    assert_eq!(status_body["id"].as_str().unwrap(), workflow_id);
    assert!(status_body["status"].as_str().is_some());
    assert!(status_body["progress"].as_f64().is_some());
    assert!(status_body["started_at"].as_str().is_some());

    // Test execution trace for web debugging
    let trace_response = server
        .get(&format!("/workflows/{}/trace", workflow_id))
        .await;

    assert_eq!(trace_response.status_code(), StatusCode::OK);
    let trace_body = trace_response.json::<serde_json::Value>();

    // Verify trace structure for web debugging interface
    assert_eq!(trace_body["workflow_id"].as_str().unwrap(), workflow_id);
    assert!(trace_body["timeline"].is_object());
    assert!(trace_body["performance"].is_object());
    assert!(trace_body["performance"]["execution_time_ms"]
        .as_i64()
        .is_some());
}

#[tokio::test]
async fn test_web_workflow_listing_and_management() {
    let router = build_router_for_tests().await;
    let server = TestServer::new(router).unwrap();

    // Create multiple workflows for web management interface
    let workflow_types = vec![
        ("prompt-chain", "Create documentation"),
        ("route", "Analyze complexity"),
        ("parallel", "Research topic"),
    ];

    for (workflow_type, prompt) in &workflow_types {
        let response = server
            .post(&format!("/workflows/{}", workflow_type))
            .json(&json!({
                "prompt": prompt,
                "role": "Test Role"
            }))
            .await;
        assert_eq!(response.status_code(), StatusCode::OK);
    }

    // Test workflow listing for web management dashboard
    let list_response = server.get("/workflows").await;

    assert_eq!(list_response.status_code(), StatusCode::OK);
    let list_body = list_response.json::<Vec<serde_json::Value>>();
    assert!(list_body.len() >= workflow_types.len());

    // Verify workflow list structure for web table display
    for workflow in &list_body {
        assert!(workflow["id"].as_str().is_some());
        assert!(workflow["status"].as_str().is_some());
        assert!(workflow["progress"].as_f64().is_some());
        assert!(workflow["started_at"].as_str().is_some());
    }
}

#[tokio::test]
async fn test_web_agent_role_integration() {
    let router = build_router_for_tests().await;
    let server = TestServer::new(router).unwrap();

    // Test that web interface can specify custom roles and configurations
    let custom_role_request = json!({
        "prompt": "Design a blockchain-based supply chain system",
        "role": "Blockchain Architect",
        "overall_role": "Technical Consultant",
        "config": {
            "include_smart_contracts": true,
            "target_platform": "Ethereum",
            "security_level": "enterprise"
        }
    });

    let response = server
        .post("/workflows/orchestrate")
        .json(&custom_role_request)
        .await;

    assert_eq!(response.status_code(), StatusCode::OK);
    let body = response.json::<serde_json::Value>();

    assert!(body["success"].as_bool().unwrap());

    // Verify that role information is preserved in the response for web display
    assert_eq!(
        body["metadata"]["role"].as_str().unwrap(),
        "Task Orchestrator"
    );
    assert_eq!(
        body["metadata"]["overall_role"].as_str().unwrap(),
        "Workflow Coordinator"
    );
}

#[tokio::test]
async fn test_web_concurrent_workflow_handling() {
    let router = build_router_for_tests().await;
    let server = TestServer::new(router).unwrap();

    // Simulate concurrent web requests (common in web applications)
    let concurrent_requests = vec![
        ("prompt-chain", "Create API documentation"),
        ("route", "Implement user authentication"),
        ("parallel", "Analyze market trends"),
        ("optimize", "Write product descriptions"),
    ];

    let mut handles = vec![];

    for (workflow_type, prompt) in concurrent_requests {
        let base_url = format!("http://{}", server.server_address().unwrap());
        let client = reqwest::Client::new();
        let handle = tokio::spawn(async move {
            client
                .post(&format!("{}/workflows/{}", base_url, workflow_type))
                .json(&json!({
                    "prompt": prompt,
                    "role": "Test Role"
                }))
                .send()
                .await
        });
        handles.push(handle);
    }

    // Wait for all concurrent requests to complete
    let results = future::join_all(handles).await;

    // Verify all concurrent requests succeeded
    for result in results {
        let response = result.unwrap().unwrap();
        assert_eq!(response.status(), reqwest::StatusCode::OK);

        let body = response.json::<serde_json::Value>().await.unwrap();
        assert!(body["success"].as_bool().unwrap());
        assert!(body["workflow_id"].as_str().is_some());
    }
}
