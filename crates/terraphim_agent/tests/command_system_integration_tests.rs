use std::collections::HashMap;
use terraphim_agent::commands::{CommandValidator, ExecutionMode};

#[tokio::test]
async fn test_role_based_command_permissions() {
    let mut validator = CommandValidator::new();

    // Test different role permissions
    // Note: The validator routes dangerous commands to Firecracker isolation rather than blocking
    // So "systemctl" commands succeed but are routed to Firecracker VM for safety
    let test_cases = vec![
        ("Default", "ls -la", true, None), // Read-only command - hybrid
        ("Default", "rm file.txt", false, None), // Write command - blocked for Default
        (
            "Default",
            "systemctl stop nginx",
            true,
            Some(ExecutionMode::Firecracker),
        ), // System command - allowed but sandboxed
        ("Terraphim Engineer", "ls -la", true, None), // Read command
        ("Terraphim Engineer", "rm file.txt", true, None), // Write command
        ("Terraphim Engineer", "systemctl stop nginx", true, None), // System command
    ];

    // Add debug output to understand validation flow
    for (role, command, should_succeed, expected_mode) in &test_cases {
        println!(
            "DEBUG: Testing role='{}', command='{}', should_succeed={}, expected_mode={:?}",
            role, command, should_succeed, expected_mode
        );

        let result = validator
            .validate_command_execution(command, role, &HashMap::new())
            .await;

        println!("DEBUG: Validation result: {:?}", result);

        if *should_succeed {
            assert!(
                result.is_ok(),
                "Role '{}' should be able to execute '{}'",
                role,
                command
            );
            // Verify execution mode if specified
            if let Some(expected) = expected_mode {
                let mode = result.unwrap();
                assert_eq!(
                    &mode, expected,
                    "Expected {:?} mode for role '{}' command '{}'",
                    expected, role, command
                );
            }
        } else {
            assert!(
                result.is_err(),
                "Role '{}' should not be able to execute '{}'",
                role,
                command
            );
        }
    }
}
