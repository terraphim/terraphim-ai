use std::str::FromStr;
use terraphim_tui::repl::commands::*;

/// Test VM management command parsing
#[test]
fn test_vm_list_command_parsing() {
    let command = ReplCommand::from_str("/vm list").unwrap();

    match command {
        ReplCommand::Vm { subcommand } => {
            assert_eq!(subcommand, VmSubcommand::List);
        }
        _ => panic!("Expected Vm command with List subcommand"),
    }
}

#[test]
fn test_vm_pool_command_parsing() {
    let command = ReplCommand::from_str("/vm pool").unwrap();

    match command {
        ReplCommand::Vm { subcommand } => {
            assert_eq!(subcommand, VmSubcommand::Pool);
        }
        _ => panic!("Expected Vm command with Pool subcommand"),
    }
}

#[test]
fn test_vm_status_command_parsing() {
    let command = ReplCommand::from_str("/vm status vm-123").unwrap();

    match command {
        ReplCommand::Vm { subcommand } => {
            if let VmSubcommand::Status { vm_id } = subcommand {
                assert_eq!(vm_id, Some("vm-123".to_string()));
            } else {
                panic!("Expected Status subcommand");
            }
        }
        _ => panic!("Expected Vm command"),
    }
}

#[test]
fn test_vm_status_without_id_command_parsing() {
    let command = ReplCommand::from_str("/vm status").unwrap();

    match command {
        ReplCommand::Vm { subcommand } => {
            if let VmSubcommand::Status { vm_id } = subcommand {
                assert_eq!(vm_id, None);
            } else {
                panic!("Expected Status subcommand");
            }
        }
        _ => panic!("Expected Vm command"),
    }
}

#[test]
fn test_vm_metrics_command_parsing() {
    let command = ReplCommand::from_str("/vm metrics vm-456").unwrap();

    match command {
        ReplCommand::Vm { subcommand } => {
            if let VmSubcommand::Metrics { vm_id } = subcommand {
                assert_eq!(vm_id, Some("vm-456".to_string()));
            } else {
                panic!("Expected Metrics subcommand");
            }
        }
        _ => panic!("Expected Vm command"),
    }
}

#[test]
fn test_vm_metrics_all_command_parsing() {
    let command = ReplCommand::from_str("/vm metrics").unwrap();

    match command {
        ReplCommand::Vm { subcommand } => {
            if let VmSubcommand::Metrics { vm_id } = subcommand {
                assert_eq!(vm_id, None);
            } else {
                panic!("Expected Metrics subcommand");
            }
        }
        _ => panic!("Expected Vm command"),
    }
}

#[test]
fn test_vm_execute_command_parsing() {
    let command =
        ReplCommand::from_str("/vm execute python print('hello') --vm-id vm-789").unwrap();

    match command {
        ReplCommand::Vm { subcommand } => {
            if let VmSubcommand::Execute {
                code,
                language,
                vm_id,
            } = subcommand
            {
                assert_eq!(language, "python");
                assert_eq!(code, "print('hello')");
                assert_eq!(vm_id, Some("vm-789".to_string()));
            } else {
                panic!("Expected Execute subcommand");
            }
        }
        _ => panic!("Expected Vm command"),
    }
}

#[test]
fn test_vm_execute_without_vm_id_command_parsing() {
    let command = ReplCommand::from_str("/vm execute javascript console.log('test')").unwrap();

    match command {
        ReplCommand::Vm { subcommand } => {
            if let VmSubcommand::Execute {
                code,
                language,
                vm_id,
            } = subcommand
            {
                assert_eq!(language, "javascript");
                assert_eq!(code, "console.log('test')");
                assert_eq!(vm_id, None);
            } else {
                panic!("Expected Execute subcommand");
            }
        }
        _ => panic!("Expected Vm command"),
    }
}

#[test]
fn test_vm_agent_command_parsing() {
    let command = ReplCommand::from_str("/vm agent dev-agent run-tests --vm-id vm-101").unwrap();

    match command {
        ReplCommand::Vm { subcommand } => {
            if let VmSubcommand::Agent {
                agent_id,
                task,
                vm_id,
            } = subcommand
            {
                assert_eq!(agent_id, "dev-agent");
                assert_eq!(task, "run-tests");
                assert_eq!(vm_id, Some("vm-101".to_string()));
            } else {
                panic!("Expected Agent subcommand");
            }
        }
        _ => panic!("Expected Vm command"),
    }
}

#[test]
fn test_vm_agent_without_vm_id_command_parsing() {
    let command = ReplCommand::from_str("/vm agent test-agent deploy").unwrap();

    match command {
        ReplCommand::Vm { subcommand } => {
            if let VmSubcommand::Agent {
                agent_id,
                task,
                vm_id,
            } = subcommand
            {
                assert_eq!(agent_id, "test-agent");
                assert_eq!(task, "deploy");
                assert_eq!(vm_id, None);
            } else {
                panic!("Expected Agent subcommand");
            }
        }
        _ => panic!("Expected Vm command"),
    }
}

#[test]
fn test_vm_tasks_command_parsing() {
    let command = ReplCommand::from_str("/vm tasks vm-202").unwrap();

    match command {
        ReplCommand::Vm { subcommand } => {
            if let VmSubcommand::Tasks { vm_id } = subcommand {
                assert_eq!(vm_id, "vm-202");
            } else {
                panic!("Expected Tasks subcommand");
            }
        }
        _ => panic!("Expected Vm command"),
    }
}

#[test]
fn test_vm_allocate_command_parsing() {
    let command = ReplCommand::from_str("/vm allocate vm-303").unwrap();

    match command {
        ReplCommand::Vm { subcommand } => {
            if let VmSubcommand::Allocate { vm_id } = subcommand {
                assert_eq!(vm_id, "vm-303");
            } else {
                panic!("Expected Allocate subcommand");
            }
        }
        _ => panic!("Expected Vm command"),
    }
}

#[test]
fn test_vm_release_command_parsing() {
    let command = ReplCommand::from_str("/vm release vm-404").unwrap();

    match command {
        ReplCommand::Vm { subcommand } => {
            if let VmSubcommand::Release { vm_id } = subcommand {
                assert_eq!(vm_id, "vm-404");
            } else {
                panic!("Expected Release subcommand");
            }
        }
        _ => panic!("Expected Vm command"),
    }
}

#[test]
fn test_vm_monitor_command_parsing() {
    let command = ReplCommand::from_str("/vm monitor vm-505 --refresh 10").unwrap();

    match command {
        ReplCommand::Vm { subcommand } => {
            if let VmSubcommand::Monitor { vm_id, refresh } = subcommand {
                assert_eq!(vm_id, "vm-505");
                assert_eq!(refresh, Some(10));
            } else {
                panic!("Expected Monitor subcommand");
            }
        }
        _ => panic!("Expected Vm command"),
    }
}

#[test]
fn test_vm_monitor_default_refresh_command_parsing() {
    let command = ReplCommand::from_str("/vm monitor vm-606").unwrap();

    match command {
        ReplCommand::Vm { subcommand } => {
            if let VmSubcommand::Monitor { vm_id, refresh } = subcommand {
                assert_eq!(vm_id, "vm-606");
                assert_eq!(refresh, None);
            } else {
                panic!("Expected Monitor subcommand");
            }
        }
        _ => panic!("Expected Vm command"),
    }
}

#[test]
fn test_vm_invalid_command_parsing() {
    let result = ReplCommand::from_str("/vm");
    assert!(result.is_err(), "Should fail when no subcommand provided");
}

#[test]
fn test_vm_unknown_subcommand_parsing() {
    let result = ReplCommand::from_str("/vm unknown");
    assert!(result.is_err(), "Should fail with unknown subcommand");
}

#[test]
fn test_vm_monitor_missing_vm_id() {
    let result = ReplCommand::from_str("/vm monitor");
    assert!(result.is_err(), "Should fail when VM ID is missing");
}

#[test]
fn test_vm_execute_missing_language() {
    let result = ReplCommand::from_str("/vm execute");
    assert!(result.is_err(), "Should fail when language is missing");
}

#[test]
fn test_vm_agent_missing_agent_id() {
    let result = ReplCommand::from_str("/vm agent");
    assert!(result.is_err(), "Should fail when agent ID is missing");
}

#[test]
fn test_vm_tasks_missing_vm_id() {
    let result = ReplCommand::from_str("/vm tasks");
    assert!(result.is_err(), "Should fail when VM ID is missing");
}

#[test]
fn test_vm_execute_empty_code() {
    let result = ReplCommand::from_str("/vm execute python");
    assert!(result.is_err(), "Should fail when code is empty");
}

#[test]
fn test_vm_agent_empty_task() {
    let result = ReplCommand::from_str("/vm agent test-agent");
    assert!(result.is_err(), "Should fail when task is empty");
}

#[test]
fn test_vm_monitor_invalid_refresh() {
    let result = ReplCommand::from_str("/vm monitor vm-123 --refresh invalid");
    assert!(result.is_err(), "Should fail with invalid refresh rate");
}

#[test]
fn test_vm_execute_with_quotes() {
    let command = ReplCommand::from_str("/vm execute python print(\"hello world\")").unwrap();

    match command {
        ReplCommand::Vm { subcommand } => {
            if let VmSubcommand::Execute {
                code,
                language,
                vm_id,
            } = subcommand
            {
                assert_eq!(language, "python");
                assert_eq!(code, "print(\"hello world\")");
                assert_eq!(vm_id, None);
            } else {
                panic!("Expected Execute subcommand");
            }
        }
        _ => panic!("Expected Vm command"),
    }
}

#[test]
fn test_vm_agent_with_complex_task() {
    let command =
        ReplCommand::from_str("/vm agent dev-agent build and test --vm-id vm-999").unwrap();

    match command {
        ReplCommand::Vm { subcommand } => {
            if let VmSubcommand::Agent {
                agent_id,
                task,
                vm_id,
            } = subcommand
            {
                assert_eq!(agent_id, "dev-agent");
                assert_eq!(task, "build and test");
                assert_eq!(vm_id, Some("vm-999".to_string()));
            } else {
                panic!("Expected Agent subcommand");
            }
        }
        _ => panic!("Expected Vm command"),
    }
}

#[test]
fn test_vm_available_commands_includes_vm() {
    let commands = ReplCommand::available_commands();
    assert!(
        commands.contains(&"vm"),
        "Available commands should include 'vm'"
    );
}
