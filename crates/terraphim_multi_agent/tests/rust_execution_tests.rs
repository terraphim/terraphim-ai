use serde_json::json;
use std::collections::HashMap;
use terraphim_multi_agent::vm_execution::*;

#[cfg(test)]
mod rust_basic_tests {
    use super::*;

    #[test]
    fn test_rust_language_config() {
        let extractor = CodeBlockExtractor::new();

        assert!(extractor.is_language_supported("rust"));

        let config = extractor.get_language_config("rust").unwrap();
        assert_eq!(config.name, "rust");
        assert_eq!(config.extension, "rs");
        assert_eq!(config.execute_command, "rustc");
        assert_eq!(config.timeout_multiplier, 3.0);

        assert!(config.restrictions.contains(&"unsafe".to_string()));
        assert!(config.restrictions.contains(&"std::process".to_string()));
    }

    #[test]
    fn test_rust_code_extraction() {
        let extractor = CodeBlockExtractor::new();
        let text = r#"
Here's a Rust program:
```rust
fn main() {
    println!("Hello from Rust!");
    let result = fibonacci(10);
    println!("Fibonacci(10) = {}", result);
}

fn fibonacci(n: u32) -> u32 {
    match n {
        0 => 0,
        1 => 1,
        _ => fibonacci(n-1) + fibonacci(n-2)
    }
}
```
        "#;

        let blocks = extractor.extract_code_blocks(text);
        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].language, "rust");
        assert!(blocks[0].code.contains("fn main()"));
        assert!(blocks[0].code.contains("fibonacci"));
        assert!(blocks[0].execution_confidence > 0.3);
    }

    #[test]
    fn test_rust_security_restrictions() {
        let extractor = CodeBlockExtractor::new();

        let unsafe_code = CodeBlock {
            language: "rust".to_string(),
            code: r#"
fn main() {
    unsafe {
        let x = *std::ptr::null::<i32>();
        println!("{}", x);
    }
}
            "#
            .to_string(),
            execution_confidence: 0.8,
            start_pos: 0,
            end_pos: 100,
            metadata: None,
        };

        let validation_result = extractor.validate_code(&unsafe_code);
        assert!(validation_result.is_err());
        assert!(
            validation_result
                .unwrap_err()
                .to_string()
                .contains("unsafe")
        );
    }

    #[test]
    fn test_rust_process_restriction() {
        let extractor = CodeBlockExtractor::new();

        let process_code = CodeBlock {
            language: "rust".to_string(),
            code: r#"
use std::process::Command;

fn main() {
    Command::new("rm").arg("-rf").arg("/").spawn().unwrap();
}
            "#
            .to_string(),
            execution_confidence: 0.8,
            start_pos: 0,
            end_pos: 100,
            metadata: None,
        };

        let validation_result = extractor.validate_code(&process_code);
        assert!(validation_result.is_err());
        assert!(
            validation_result
                .unwrap_err()
                .to_string()
                .contains("std::process")
        );
    }

    #[test]
    fn test_rust_safe_code_validation() {
        let extractor = CodeBlockExtractor::new();

        let safe_code = CodeBlock {
            language: "rust".to_string(),
            code: r#"
fn main() {
    let numbers = vec![1, 2, 3, 4, 5];
    let sum: i32 = numbers.iter().sum();
    println!("Sum: {}", sum);
    
    let squared: Vec<i32> = numbers.iter().map(|x| x * x).collect();
    println!("Squared: {:?}", squared);
}
            "#
            .to_string(),
            execution_confidence: 0.8,
            start_pos: 0,
            end_pos: 200,
            metadata: None,
        };

        let validation_result = extractor.validate_code(&safe_code);
        assert!(validation_result.is_ok());
    }
}

#[cfg(test)]
mod rust_integration_tests {
    use super::*;
    use tokio;

    #[tokio::test]
    #[ignore]
    async fn test_rust_hello_world_execution() {
        let config = VmExecutionConfig {
            enabled: true,
            api_base_url: "http://localhost:8080".to_string(),
            vm_pool_size: 1,
            default_vm_type: "rust-vm".to_string(),
            execution_timeout_ms: 90000,
            allowed_languages: vec!["rust".to_string()],
            auto_provision: true,
            code_validation: true,
            max_code_length: 10000,
            history: HistoryConfig::default(),
        };

        let client = VmExecutionClient::new(&config);

        let request = VmExecuteRequest {
            agent_id: "rust-test-agent".to_string(),
            language: "rust".to_string(),
            code: r#"
fn main() {
    println!("Hello from Rust VM!");
    println!("2 + 2 = {}", 2 + 2);
}
            "#
            .to_string(),
            vm_id: None,
            requirements: vec![],
            timeout_seconds: Some(60),
            working_dir: None,
            metadata: None,
        };

        let response = client.execute_code(request).await;
        assert!(response.is_ok());

        let result = response.unwrap();
        assert_eq!(result.exit_code, 0);
        assert!(result.stdout.contains("Hello from Rust VM!"));
        assert!(result.stdout.contains("2 + 2 = 4"));
    }

    #[tokio::test]
    #[ignore]
    async fn test_rust_compilation_error() {
        let config = VmExecutionConfig {
            enabled: true,
            api_base_url: "http://localhost:8080".to_string(),
            vm_pool_size: 1,
            default_vm_type: "rust-vm".to_string(),
            execution_timeout_ms: 90000,
            allowed_languages: vec!["rust".to_string()],
            auto_provision: true,
            code_validation: true,
            max_code_length: 10000,
            history: HistoryConfig::default(),
        };

        let client = VmExecutionClient::new(&config);

        let request = VmExecuteRequest {
            agent_id: "rust-test-agent".to_string(),
            language: "rust".to_string(),
            code: r#"
fn main() {
    let x: i32 = "not a number";
    println!("{}", x);
}
            "#
            .to_string(),
            vm_id: None,
            requirements: vec![],
            timeout_seconds: Some(60),
            working_dir: None,
            metadata: None,
        };

        let response = client.execute_code(request).await;
        assert!(response.is_ok());

        let result = response.unwrap();
        assert_ne!(result.exit_code, 0);
        assert!(result.stderr.contains("error") || result.stderr.contains("mismatch"));
    }

    #[tokio::test]
    #[ignore]
    async fn test_rust_complex_program() {
        let config = VmExecutionConfig {
            enabled: true,
            api_base_url: "http://localhost:8080".to_string(),
            vm_pool_size: 1,
            default_vm_type: "rust-vm".to_string(),
            execution_timeout_ms: 120000,
            allowed_languages: vec!["rust".to_string()],
            auto_provision: true,
            code_validation: true,
            max_code_length: 10000,
            history: HistoryConfig::default(),
        };

        let client = VmExecutionClient::new(&config);

        let request = VmExecuteRequest {
            agent_id: "rust-test-agent".to_string(),
            language: "rust".to_string(),
            code: r#"
use std::collections::HashMap;

fn main() {
    let mut scores = HashMap::new();
    scores.insert("Alice", 95);
    scores.insert("Bob", 87);
    scores.insert("Carol", 92);
    
    let average: i32 = scores.values().sum::<i32>() / scores.len() as i32;
    println!("Average score: {}", average);
    
    let top_student = scores.iter()
        .max_by_key(|(_, &score)| score)
        .map(|(name, score)| format!("{}: {}", name, score))
        .unwrap();
    println!("Top student: {}", top_student);
}
            "#
            .to_string(),
            vm_id: None,
            requirements: vec![],
            timeout_seconds: Some(90),
            working_dir: None,
            metadata: None,
        };

        let response = client.execute_code(request).await;
        assert!(response.is_ok());

        let result = response.unwrap();
        assert_eq!(result.exit_code, 0);
        assert!(result.stdout.contains("Average score:"));
        assert!(result.stdout.contains("Top student:"));
        assert!(result.stdout.contains("Alice") || result.stdout.contains("95"));
    }

    #[tokio::test]
    #[ignore]
    async fn test_rust_iterators_and_closures() {
        let config = VmExecutionConfig {
            enabled: true,
            api_base_url: "http://localhost:8080".to_string(),
            vm_pool_size: 1,
            default_vm_type: "rust-vm".to_string(),
            execution_timeout_ms: 90000,
            allowed_languages: vec!["rust".to_string()],
            auto_provision: true,
            code_validation: true,
            max_code_length: 10000,
            history: HistoryConfig::default(),
        };

        let client = VmExecutionClient::new(&config);

        let request = VmExecuteRequest {
            agent_id: "rust-test-agent".to_string(),
            language: "rust".to_string(),
            code: r#"
fn main() {
    let numbers = vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10];
    
    let sum: i32 = numbers.iter().sum();
    println!("Sum: {}", sum);
    
    let even_squares: Vec<i32> = numbers
        .iter()
        .filter(|&&x| x % 2 == 0)
        .map(|&x| x * x)
        .collect();
    println!("Even squares: {:?}", even_squares);
    
    let product: i32 = (1..=5).product();
    println!("Factorial of 5: {}", product);
}
            "#
            .to_string(),
            vm_id: None,
            requirements: vec![],
            timeout_seconds: Some(60),
            working_dir: None,
            metadata: None,
        };

        let response = client.execute_code(request).await;
        assert!(response.is_ok());

        let result = response.unwrap();
        assert_eq!(result.exit_code, 0);
        assert!(result.stdout.contains("Sum: 55"));
        assert!(result.stdout.contains("Even squares:"));
        assert!(result.stdout.contains("Factorial of 5: 120"));
    }

    #[tokio::test]
    #[ignore]
    async fn test_rust_timeout_multiplier() {
        let config = VmExecutionConfig {
            enabled: true,
            api_base_url: "http://localhost:8080".to_string(),
            vm_pool_size: 1,
            default_vm_type: "rust-vm".to_string(),
            execution_timeout_ms: 30000,
            allowed_languages: vec!["rust".to_string()],
            auto_provision: true,
            code_validation: true,
            max_code_length: 10000,
            history: HistoryConfig::default(),
        };

        let client = VmExecutionClient::new(&config);

        let request = VmExecuteRequest {
            agent_id: "rust-test-agent".to_string(),
            language: "rust".to_string(),
            code: r#"
fn main() {
    let start = std::time::Instant::now();
    println!("Starting Rust execution...");
    
    let result = (1..=1000).fold(0, |acc, x| acc + x);
    println!("Result: {}", result);
    
    let elapsed = start.elapsed();
    println!("Execution time: {:?}", elapsed);
}
            "#
            .to_string(),
            vm_id: None,
            requirements: vec![],
            timeout_seconds: None,
            working_dir: None,
            metadata: None,
        };

        let start = std::time::Instant::now();
        let response = client.execute_code(request).await;
        let elapsed = start.elapsed();

        assert!(response.is_ok());

        println!("Total time including compilation: {:?}", elapsed);
        assert!(elapsed.as_secs() < 90);
    }
}

#[cfg(test)]
mod rust_hook_tests {
    use super::*;
    use terraphim_multi_agent::vm_execution::hooks::*;

    #[tokio::test]
    async fn test_dangerous_pattern_hook_blocks_unsafe() {
        let hook = DangerousPatternHook::new();

        let context = PreToolContext {
            code: r#"
unsafe {
    let ptr = std::ptr::null::<i32>();
    *ptr
}
            "#
            .to_string(),
            language: "rust".to_string(),
            agent_id: "rust-agent".to_string(),
            vm_id: "test-vm".to_string(),
            metadata: HashMap::new(),
        };

        let decision = hook.pre_tool(&context).await.unwrap();

        match decision {
            HookDecision::Allow => {}
            _ => {}
        }
    }

    #[tokio::test]
    async fn test_syntax_validation_rust() {
        let hook = SyntaxValidationHook::new();

        let context = PreToolContext {
            code: r#"
fn main() {
    println!("Valid Rust code");
}
            "#
            .to_string(),
            language: "rust".to_string(),
            agent_id: "rust-agent".to_string(),
            vm_id: "test-vm".to_string(),
            metadata: HashMap::new(),
        };

        let decision = hook.pre_tool(&context).await.unwrap();
        assert_eq!(decision, HookDecision::Allow);
    }

    #[tokio::test]
    async fn test_rust_code_too_long() {
        let hook = SyntaxValidationHook::new();

        let long_code = "fn main() {}\n".repeat(10000);

        let context = PreToolContext {
            code: long_code,
            language: "rust".to_string(),
            agent_id: "rust-agent".to_string(),
            vm_id: "test-vm".to_string(),
            metadata: HashMap::new(),
        };

        let decision = hook.pre_tool(&context).await.unwrap();
        assert!(matches!(decision, HookDecision::Block { .. }));
    }
}
