use serde_json;
use terraphim_agent::client::*;

/// Test VM-related API types serialization
#[test]
fn test_vm_pool_list_response_serialization() {
    let response = VmPoolListResponse {
        vms: vec![
            VmWithIp {
                vm_id: "vm-123".to_string(),
                ip_address: "172.26.0.10".to_string(),
            },
            VmWithIp {
                vm_id: "vm-456".to_string(),
                ip_address: "172.26.0.11".to_string(),
            },
        ],
        stats: VmPoolStatsResponse {
            total_ips: 253,
            allocated_ips: 2,
            available_ips: 251,
            utilization_percent: 1,
        },
    };

    // Should be serializable to JSON
    let json_result = serde_json::to_string(&response);
    assert!(
        json_result.is_ok(),
        "VmPoolListResponse should be serializable"
    );

    let json_str = json_result.unwrap();
    assert!(json_str.contains("vm-123"));
    assert!(json_str.contains("172.26.0.10"));
    assert!(json_str.contains("253"));

    // Should be deserializable from JSON
    let deserialized: Result<VmPoolListResponse, _> = serde_json::from_str(&json_str);
    assert!(
        deserialized.is_ok(),
        "VmPoolListResponse should be deserializable"
    );

    let deserialized_response = deserialized.unwrap();
    assert_eq!(deserialized_response.vms.len(), 2);
    assert_eq!(deserialized_response.vms[0].vm_id, "vm-123");
    assert_eq!(deserialized_response.stats.total_ips, 253);
}

#[test]
fn test_vm_status_response_serialization() {
    let response = VmStatusResponse {
        vm_id: "vm-789".to_string(),
        status: "running".to_string(),
        ip_address: "172.26.0.15".to_string(),
        created_at: "2025-01-18T10:30:00Z".to_string(),
        updated_at: Some("2025-01-18T10:35:00Z".to_string()),
    };

    let json_result = serde_json::to_string(&response);
    assert!(
        json_result.is_ok(),
        "VmStatusResponse should be serializable"
    );

    let json_str = json_result.unwrap();
    assert!(json_str.contains("vm-789"));
    assert!(json_str.contains("running"));

    let deserialized: Result<VmStatusResponse, _> = serde_json::from_str(&json_str);
    assert!(
        deserialized.is_ok(),
        "VmStatusResponse should be deserializable"
    );

    let deserialized_response = deserialized.unwrap();
    assert_eq!(deserialized_response.vm_id, "vm-789");
    assert_eq!(deserialized_response.status, "running");
    assert_eq!(
        deserialized_response.updated_at,
        Some("2025-01-18T10:35:00Z".to_string())
    );
}

#[test]
fn test_vm_execute_request_serialization() {
    let request = VmExecuteRequest {
        code: "print('Hello, World!')".to_string(),
        language: "python".to_string(),
        agent_id: "tui-user".to_string(),
        vm_id: Some("vm-101".to_string()),
        timeout_ms: Some(30000),
    };

    let json_result = serde_json::to_string(&request);
    assert!(
        json_result.is_ok(),
        "VmExecuteRequest should be serializable"
    );

    let json_str = json_result.unwrap();
    assert!(json_str.contains("print('Hello, World!')"));
    assert!(json_str.contains("python"));
    assert!(json_str.contains("30000"));

    let deserialized: Result<VmExecuteRequest, _> = serde_json::from_str(&json_str);
    assert!(
        deserialized.is_ok(),
        "VmExecuteRequest should be deserializable"
    );

    let deserialized_request = deserialized.unwrap();
    assert_eq!(deserialized_request.code, "print('Hello, World!')");
    assert_eq!(deserialized_request.language, "python");
    assert_eq!(deserialized_request.vm_id, Some("vm-101".to_string()));
}

#[test]
fn test_vm_execute_response_serialization() {
    let response = VmExecuteResponse {
        execution_id: "exec-12345".to_string(),
        vm_id: "vm-202".to_string(),
        exit_code: 0,
        stdout: "Hello, World!\n".to_string(),
        stderr: "".to_string(),
        duration_ms: 1500,
        started_at: "2025-01-18T10:30:00Z".to_string(),
        completed_at: "2025-01-18T10:30:01.5Z".to_string(),
        error: None,
    };

    let json_result = serde_json::to_string(&response);
    assert!(
        json_result.is_ok(),
        "VmExecuteResponse should be serializable"
    );

    let json_str = json_result.unwrap();
    assert!(json_str.contains("exec-12345"));
    assert!(json_str.contains("Hello, World!"));
    assert!(json_str.contains("1500"));

    let deserialized: Result<VmExecuteResponse, _> = serde_json::from_str(&json_str);
    assert!(
        deserialized.is_ok(),
        "VmExecuteResponse should be deserializable"
    );

    let deserialized_response = deserialized.unwrap();
    assert_eq!(deserialized_response.execution_id, "exec-12345");
    assert_eq!(deserialized_response.exit_code, 0);
    assert_eq!(deserialized_response.stdout, "Hello, World!\n");
}

#[test]
fn test_vm_execute_response_with_error() {
    let response = VmExecuteResponse {
        execution_id: "exec-67890".to_string(),
        vm_id: "vm-303".to_string(),
        exit_code: 1,
        stdout: "".to_string(),
        stderr: "FileNotFoundError: [Errno 2] No such file or directory".to_string(),
        duration_ms: 800,
        started_at: "2025-01-18T10:45:00Z".to_string(),
        completed_at: "2025-01-18T10:45:00.8Z".to_string(),
        error: Some("Execution failed".to_string()),
    };

    let json_result = serde_json::to_string(&response);
    assert!(
        json_result.is_ok(),
        "VmExecuteResponse with error should be serializable"
    );

    let json_str = json_result.unwrap();
    assert!(json_str.contains("FileNotFoundError"));
    assert!(json_str.contains("Execution failed"));

    let deserialized: Result<VmExecuteResponse, _> = serde_json::from_str(&json_str);
    assert!(
        deserialized.is_ok(),
        "VmExecuteResponse with error should be deserializable"
    );

    let deserialized_response = deserialized.unwrap();
    assert_eq!(deserialized_response.exit_code, 1);
    assert_eq!(
        deserialized_response.error,
        Some("Execution failed".to_string())
    );
}

#[test]
fn test_vm_metrics_response_serialization() {
    let response = VmMetricsResponse {
        vm_id: "vm-404".to_string(),
        status: "running".to_string(),
        cpu_usage_percent: 75.5,
        memory_usage_mb: 1024,
        disk_usage_percent: 45.2,
        network_io_mbps: 12.8,
        uptime_seconds: 3600,
        process_count: 15,
        updated_at: Some("2025-01-18T10:50:00Z".to_string()),
    };

    let json_result = serde_json::to_string(&response);
    assert!(
        json_result.is_ok(),
        "VmMetricsResponse should be serializable"
    );

    let json_str = json_result.unwrap();
    assert!(json_str.contains("75.5"));
    assert!(json_str.contains("1024"));
    assert!(json_str.contains("12.8"));

    let deserialized: Result<VmMetricsResponse, _> = serde_json::from_str(&json_str);
    assert!(
        deserialized.is_ok(),
        "VmMetricsResponse should be deserializable"
    );

    let deserialized_response = deserialized.unwrap();
    assert_eq!(deserialized_response.vm_id, "vm-404");
    assert_eq!(deserialized_response.cpu_usage_percent, 75.5);
    assert_eq!(deserialized_response.memory_usage_mb, 1024);
    assert_eq!(deserialized_response.process_count, 15);
}

#[test]
fn test_vm_agent_request_serialization() {
    let request = VmAgentRequest {
        agent_id: "dev-agent".to_string(),
        task: "run integration tests".to_string(),
        vm_id: Some("vm-505".to_string()),
        timeout_ms: Some(60000),
    };

    let json_result = serde_json::to_string(&request);
    assert!(json_result.is_ok(), "VmAgentRequest should be serializable");

    let json_str = json_result.unwrap();
    assert!(json_str.contains("dev-agent"));
    assert!(json_str.contains("run integration tests"));
    assert!(json_str.contains("60000"));

    let deserialized: Result<VmAgentRequest, _> = serde_json::from_str(&json_str);
    assert!(
        deserialized.is_ok(),
        "VmAgentRequest should be deserializable"
    );

    let deserialized_request = deserialized.unwrap();
    assert_eq!(deserialized_request.agent_id, "dev-agent");
    assert_eq!(deserialized_request.task, "run integration tests");
    assert_eq!(deserialized_request.vm_id, Some("vm-505".to_string()));
}

#[test]
fn test_vm_agent_response_serialization() {
    let response = VmAgentResponse {
        task_id: "task-11111".to_string(),
        agent_id: "test-agent".to_string(),
        vm_id: Some("vm-606".to_string()),
        status: "completed".to_string(),
        result: "All tests passed successfully\n15 tests run, 0 failures".to_string(),
        duration_ms: 45000,
        started_at: "2025-01-18T11:00:00Z".to_string(),
        completed_at: "2025-01-18T11:00:45Z".to_string(),
        snapshot_id: Some("snap-abc123".to_string()),
        error: None,
    };

    let json_result = serde_json::to_string(&response);
    assert!(
        json_result.is_ok(),
        "VmAgentResponse should be serializable"
    );

    let json_str = json_result.unwrap();
    assert!(json_str.contains("task-11111"));
    assert!(json_str.contains("All tests passed"));
    assert!(json_str.contains("snap-abc123"));

    let deserialized: Result<VmAgentResponse, _> = serde_json::from_str(&json_str);
    assert!(
        deserialized.is_ok(),
        "VmAgentResponse should be deserializable"
    );

    let deserialized_response = deserialized.unwrap();
    assert_eq!(deserialized_response.task_id, "task-11111");
    assert_eq!(deserialized_response.status, "completed");
    assert_eq!(
        deserialized_response.snapshot_id,
        Some("snap-abc123".to_string())
    );
}

#[test]
fn test_vm_tasks_response_serialization() {
    let response = VmTasksResponse {
        tasks: vec![
            VmTask {
                id: "task-001".to_string(),
                vm_id: "vm-707".to_string(),
                status: "completed".to_string(),
                created_at: "2025-01-18T10:00:00Z".to_string(),
                updated_at: Some("2025-01-18T10:01:00Z".to_string()),
            },
            VmTask {
                id: "task-002".to_string(),
                vm_id: "vm-707".to_string(),
                status: "running".to_string(),
                created_at: "2025-01-18T10:02:00Z".to_string(),
                updated_at: None,
            },
        ],
        vm_id: "vm-707".to_string(),
        total: 2,
    };

    let json_result = serde_json::to_string(&response);
    assert!(
        json_result.is_ok(),
        "VmTasksResponse should be serializable"
    );

    let json_str = json_result.unwrap();
    assert!(json_str.contains("task-001"));
    assert!(json_str.contains("vm-707"));

    let deserialized: Result<VmTasksResponse, _> = serde_json::from_str(&json_str);
    assert!(
        deserialized.is_ok(),
        "VmTasksResponse should be deserializable"
    );

    let deserialized_response = deserialized.unwrap();
    assert_eq!(deserialized_response.tasks.len(), 2);
    assert_eq!(deserialized_response.vm_id, "vm-707");
    assert_eq!(deserialized_response.total, 2);
}

#[test]
fn test_vm_allocate_request_serialization() {
    let request = VmAllocateRequest {
        vm_id: "vm-808".to_string(),
    };

    let json_result = serde_json::to_string(&request);
    assert!(
        json_result.is_ok(),
        "VmAllocateRequest should be serializable"
    );

    let json_str = json_result.unwrap();
    assert!(json_str.contains("vm-808"));

    let deserialized: Result<VmAllocateRequest, _> = serde_json::from_str(&json_str);
    assert!(
        deserialized.is_ok(),
        "VmAllocateRequest should be deserializable"
    );

    let deserialized_request = deserialized.unwrap();
    assert_eq!(deserialized_request.vm_id, "vm-808");
}

#[test]
fn test_vm_allocate_response_serialization() {
    let response = VmAllocateResponse {
        vm_id: "vm-808".to_string(),
        ip_address: "172.26.0.20".to_string(),
    };

    let json_result = serde_json::to_string(&response);
    assert!(
        json_result.is_ok(),
        "VmAllocateResponse should be serializable"
    );

    let json_str = json_result.unwrap();
    assert!(json_str.contains("vm-808"));
    assert!(json_str.contains("172.26.0.20"));

    let deserialized: Result<VmAllocateResponse, _> = serde_json::from_str(&json_str);
    assert!(
        deserialized.is_ok(),
        "VmAllocateResponse should be deserializable"
    );

    let deserialized_response = deserialized.unwrap();
    assert_eq!(deserialized_response.vm_id, "vm-808");
    assert_eq!(deserialized_response.ip_address, "172.26.0.20");
}

#[test]
fn test_vm_pool_stats_response_edge_cases() {
    let response = VmPoolStatsResponse {
        total_ips: 253,
        allocated_ips: 0, // No VMs allocated
        available_ips: 253,
        utilization_percent: 0, // 0% utilization
    };

    let json_result = serde_json::to_string(&response);
    assert!(
        json_result.is_ok(),
        "VmPoolStatsResponse should handle empty pool"
    );

    let json_str = json_result.unwrap();
    assert!(json_str.contains("0"));

    let deserialized: Result<VmPoolStatsResponse, _> = serde_json::from_str(&json_str);
    assert!(
        deserialized.is_ok(),
        "VmPoolStatsResponse should be deserializable"
    );

    let deserialized_response = deserialized.unwrap();
    assert_eq!(deserialized_response.utilization_percent, 0);
    assert_eq!(deserialized_response.allocated_ips, 0);
}

#[test]
fn test_vm_metrics_response_high_values() {
    let response = VmMetricsResponse {
        vm_id: "vm-999".to_string(),
        status: "overloaded".to_string(),
        cpu_usage_percent: 99.9,
        memory_usage_mb: 4096,
        disk_usage_percent: 95.5,
        network_io_mbps: 999.9,
        uptime_seconds: 86400, // 24 hours
        process_count: 500,
        updated_at: Some("2025-01-18T12:00:00Z".to_string()),
    };

    let json_result = serde_json::to_string(&response);
    assert!(
        json_result.is_ok(),
        "VmMetricsResponse should handle high values"
    );

    let deserialized: Result<VmMetricsResponse, _> = serde_json::from_str(&json_result.unwrap());
    assert!(
        deserialized.is_ok(),
        "VmMetricsResponse should be deserializable"
    );

    let deserialized_response = deserialized.unwrap();
    assert_eq!(deserialized_response.cpu_usage_percent, 99.9);
    assert_eq!(deserialized_response.memory_usage_mb, 4096);
    assert_eq!(deserialized_response.process_count, 500);
}
