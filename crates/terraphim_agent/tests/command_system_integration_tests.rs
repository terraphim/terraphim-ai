use std::collections::HashMap;
use terraphim_agent::commands::CommandValidator;

#[tokio::test]
async fn test_role_based_command_permissions() {
    let mut validator = CommandValidator::new();

    // Test different role permissions
    let test_cases = vec![
        ("Default", "ls -la", true),                          // Read-only command
        ("Default", "rm file.txt", false),                    // Write command
        ("Default", "systemctl stop nginx", false),           // System command
        ("Terraphim Engineer", "ls -la", true),               // Read command
        ("Terraphim Engineer", "rm file.txt", true),          // Write command
        ("Terraphim Engineer", "systemctl stop nginx", true), // System command
    ];

    // Add debug output to understand validation flow
    for (role, command, should_succeed) in &test_cases {
        println!(
            "DEBUG: Testing role='{}', command='{}', should_succeed={}",
            role, command, should_succeed
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
