use terraphim_multi_agent::{
    agent::{TerraphimAgent, CommandInput, CommandType},
    vm_execution::*,
};
use terraphim_config::Role;
use serde_json::json;
use std::sync::Arc;
use tokio::time::{timeout, Duration};

#[cfg(test)]
mod complete_workflow_tests {
    use super::*;

    async fn create_vm_agent() -> TerraphimAgent {
        let mut role = Role::default();
        role.name = "E2E Test Agent".to_string();
        role.extra = Some(json!({
            "vm_execution": {
                "enabled": true,
                "api_base_url": "http://localhost:8080",
                "vm_pool_size": 2,
                "default_vm_type": "ubuntu",
                "execution_timeout_ms": 60000,
                "allowed_languages": ["python", "javascript", "bash", "rust"],
                "auto_provision": true,
                "code_validation": true,
                "max_code_length": 10000,
                "history": {
                    "enabled": true,
                    "snapshot_on_execution": true,
                    "snapshot_on_failure": false,
                    "auto_rollback_on_failure": false,
                    "max_history_entries": 100,
                    "persist_history": true,
                    "integration_mode": "http"
                }
            }
        }));

        TerraphimAgent::new(role).await.expect("Failed to create agent")
    }

    #[tokio::test]
    #[ignore]
    async fn test_end_to_end_python_execution() {
        let agent = create_vm_agent().await;

        let input = CommandInput {
            command: CommandType::Execute,
            text: r#"
Calculate the factorial of 10 using Python:

```python
def factorial(n):
    if n <= 1:
        return 1
    return n * factorial(n-1)

result = factorial(10)
print(f"Factorial of 10 is: {result}")
```
            "#.to_string(),
            metadata: None,
        };

        let result = timeout(Duration::from_secs(30), agent.process_command(input))
            .await
            .expect("Timeout")
            .expect("Execution failed");

        assert!(result.success);
        assert!(result.response.contains("3628800"));
    }

    #[tokio::test]
    #[ignore]
    async fn test_end_to_end_rust_execution() {
        let agent = create_vm_agent().await;

        let input = CommandInput {
            command: CommandType::Execute,
            text: r#"
Write a Rust program to find prime numbers:

```rust
fn is_prime(n: u32) -> bool {
    if n < 2 { return false; }
    for i in 2..=(n as f64).sqrt() as u32 {
        if n % i == 0 { return false; }
    }
    true
}

fn main() {
    let primes: Vec<u32> = (1..=50)
        .filter(|&n| is_prime(n))
        .collect();
    println!("Primes up to 50: {:?}", primes);
}
```
            "#.to_string(),
            metadata: None,
        };

        let result = timeout(Duration::from_secs(90), agent.process_command(input))
            .await
            .expect("Timeout")
            .expect("Execution failed");

        assert!(result.success);
        assert!(result.response.contains("Primes"));
        assert!(result.response.contains("2") && result.response.contains("47"));
    }

    #[tokio::test]
    #[ignore]
    async fn test_security_blocks_dangerous_code() {
        let agent = create_vm_agent().await;

        let input = CommandInput {
            command: CommandType::Execute,
            text: r#"
Run this cleanup script:

```bash
rm -rf /home/user
```
            "#.to_string(),
            metadata: None,
        };

        let result = timeout(Duration::from_secs(10), agent.process_command(input))
            .await
            .expect("Timeout")
            .expect("Command processing failed");

        assert!(!result.success || result.response.contains("blocked") || result.response.contains("dangerous"));
    }

    #[tokio::test]
    #[ignore]
    async fn test_multi_turn_conversation_with_vm_state() {
        let agent = create_vm_agent().await;

        let turn1 = CommandInput {
            command: CommandType::Execute,
            text: r#"
Create a file with some data:

```bash
echo "Turn 1 data" > /tmp/conversation_state.txt
cat /tmp/conversation_state.txt
```
            "#.to_string(),
            metadata: None,
        };

        let result1 = agent.process_command(turn1).await.expect("Turn 1 failed");
        assert!(result1.success);
        assert!(result1.response.contains("Turn 1 data"));

        let turn2 = CommandInput {
            command: CommandType::Execute,
            text: r#"
Append to the file:

```bash
echo "Turn 2 data" >> /tmp/conversation_state.txt
cat /tmp/conversation_state.txt
```
            "#.to_string(),
            metadata: None,
        };

        let result2 = agent.process_command(turn2).await.expect("Turn 2 failed");
        assert!(result2.success);
        assert!(result2.response.contains("Turn 1 data"));
        assert!(result2.response.contains("Turn 2 data"));
    }

    #[tokio::test]
    #[ignore]
    async fn test_error_recovery_with_history() {
        let agent = create_vm_agent().await;

        let success_cmd = CommandInput {
            command: CommandType::Execute,
            text: r#"
```python
print("Successful execution 1")
```
            "#.to_string(),
            metadata: None,
        };

        agent.process_command(success_cmd).await.expect("Success 1 failed");

        let fail_cmd = CommandInput {
            command: CommandType::Execute,
            text: r#"
```python
undefined_variable
```
            "#.to_string(),
            metadata: None,
        };

        let fail_result = agent.process_command(fail_cmd).await.expect("Fail execution");
        assert!(!fail_result.success || fail_result.response.contains("error"));

        let recovery_cmd = CommandInput {
            command: CommandType::Execute,
            text: r#"
```python
print("Recovery execution")
```
            "#.to_string(),
            metadata: None,
        };

        let recovery_result = agent.process_command(recovery_cmd).await.expect("Recovery failed");
        assert!(recovery_result.success);
    }
}

#[cfg(test)]
mod multi_language_workflow_tests {
    use super::*;

    #[tokio::test]
    #[ignore]
    async fn test_python_then_javascript() {
        let agent = create_vm_agent().await;

        let py_input = CommandInput {
            command: CommandType::Execute,
            text: "```python\nprint('Python executed')\n```".to_string(),
            metadata: None,
        };

        let py_result = agent.process_command(py_input).await.unwrap();
        assert!(py_result.response.contains("Python executed"));

        let js_input = CommandInput {
            command: CommandType::Execute,
            text: "```javascript\nconsole.log('JavaScript executed')\n```".to_string(),
            metadata: None,
        };

        let js_result = agent.process_command(js_input).await.unwrap();
        assert!(js_result.response.contains("JavaScript executed"));
    }

    #[tokio::test]
    #[ignore]
    async fn test_all_languages_in_sequence() {
        let agent = create_vm_agent().await;

        let languages = vec![
            ("python", "print('Python')"),
            ("javascript", "console.log('JavaScript')"),
            ("bash", "echo 'Bash'"),
            ("rust", "fn main() { println!(\"Rust\"); }"),
        ];

        for (lang, code) in languages {
            let input = CommandInput {
                command: CommandType::Execute,
                text: format!("```{}\n{}\n```", lang, code),
                metadata: None,
            };

            let result = timeout(
                Duration::from_secs(if lang == "rust" { 90 } else { 30 }),
                agent.process_command(input)
            ).await.expect("Timeout").expect(&format!("{} failed", lang));

            let lang_cap = lang.chars().next().unwrap().to_uppercase().collect::<String>() + &lang[1..];
            assert!(result.response.contains(&lang_cap) || result.response.contains(lang));
        }
    }
}

#[cfg(test)]
mod hook_integration_e2e_tests {
    use super::*;

    #[tokio::test]
    #[ignore]
    async fn test_output_sanitization_blocks_secrets() {
        let agent = create_vm_agent().await;

        let input = CommandInput {
            command: CommandType::Execute,
            text: r#"
```python
print("API_KEY=secret123456")
print("PASSWORD=super_secret")
```
            "#.to_string(),
            metadata: None,
        };

        let result = agent.process_command(input).await.expect("Execution failed");

        assert!(!result.response.contains("secret123456") || !result.success);
    }

    #[tokio::test]
    #[ignore]
    async fn test_code_transformation_adds_imports() {
        let agent = create_vm_agent().await;

        let input = CommandInput {
            command: CommandType::Execute,
            text: r#"
```python
result = 2 + 2
print(result)
```
            "#.to_string(),
            metadata: None,
        };

        let result = agent.process_command(input).await.expect("Execution failed");
        assert!(result.success);
        assert!(result.response.contains("4"));
    }
}

#[cfg(test)]
mod performance_e2e_tests {
    use super::*;

    #[tokio::test]
    #[ignore]
    async fn test_rapid_execution_sequence() {
        let agent = create_vm_agent().await;

        for i in 1..=10 {
            let input = CommandInput {
                command: CommandType::Execute,
                text: format!("```python\nprint('Execution {}')\n```", i),
                metadata: None,
            };

            let result = timeout(Duration::from_secs(10), agent.process_command(input))
                .await
                .expect(&format!("Timeout on execution {}", i))
                .expect(&format!("Failed execution {}", i));

            assert!(result.success);
            assert!(result.response.contains(&format!("Execution {}", i)));
        }
    }

    #[tokio::test]
    #[ignore]
    async fn test_concurrent_vm_sessions() {
        use tokio::task::JoinSet;

        let mut set = JoinSet::new();

        for i in 1..=3 {
            set.spawn(async move {
                let agent = create_vm_agent().await;

                let input = CommandInput {
                    command: CommandType::Execute,
                    text: format!("```python\nprint('Agent {} output')\n```", i),
                    metadata: None,
                };

                agent.process_command(input).await.unwrap()
            });
        }

        while let Some(result) = set.join_next().await {
            let output = result.unwrap();
            assert!(output.success);
            assert!(output.response.contains("Agent") && output.response.contains("output"));
        }
    }
}
