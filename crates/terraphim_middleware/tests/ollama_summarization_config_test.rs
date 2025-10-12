use serde_json::Value;
use std::fs;

/// Test that validates all roles have proper Ollama summarization configuration
#[test]
fn test_ollama_summarization_configuration() {
    println!("🧪 Ollama Summarization Configuration Test");
    println!("==========================================");

    let config_files = vec![
        "../../terraphim_server/default/combined_roles_config.json",
        "../../terraphim_server/default/rust_engineer_config.json",
        "../../terraphim_server/default/terraphim_engineer_config.json",
    ];

    for config_path in config_files {
        println!("\n📋 Testing configuration file: {}", config_path);

        match fs::read_to_string(config_path) {
            Ok(config_content) => {
                match serde_json::from_str::<Value>(&config_content) {
                    Ok(config) => {
                        if let Some(roles) = config.get("roles") {
                            if let Some(roles_obj) = roles.as_object() {
                                for (role_name, role_config) in roles_obj {
                                    println!("  🔍 Checking role: {}", role_name);

                                    // Check for required Ollama configuration fields
                                    let llm_provider = role_config.get("llm_provider");
                                    let ollama_base_url = role_config.get("ollama_base_url");
                                    let ollama_model = role_config.get("ollama_model");
                                    let llm_auto_summarize = role_config.get("llm_auto_summarize");
                                    let llm_system_prompt = role_config.get("llm_system_prompt");

                                    // Validate Ollama provider
                                    assert!(
                                        llm_provider.is_some(),
                                        "Role '{}' in '{}' should have 'llm_provider' field",
                                        role_name,
                                        config_path
                                    );
                                    assert_eq!(
                                        llm_provider.unwrap().as_str().unwrap(),
                                        "ollama",
                                        "Role '{}' in '{}' should have llm_provider set to 'ollama'",
                                        role_name,
                                        config_path
                                    );

                                    // Validate Ollama base URL
                                    assert!(
                                        ollama_base_url.is_some(),
                                        "Role '{}' in '{}' should have 'ollama_base_url' field",
                                        role_name,
                                        config_path
                                    );
                                    assert_eq!(
                                        ollama_base_url.unwrap().as_str().unwrap(),
                                        "http://127.0.0.1:11434",
                                        "Role '{}' in '{}' should have ollama_base_url set to 'http://127.0.0.1:11434'",
                                        role_name,
                                        config_path
                                    );

                                    // Validate Ollama model
                                    assert!(
                                        ollama_model.is_some(),
                                        "Role '{}' in '{}' should have 'ollama_model' field",
                                        role_name,
                                        config_path
                                    );
                                    assert_eq!(
                                        ollama_model.unwrap().as_str().unwrap(),
                                        "llama3.2:3b",
                                        "Role '{}' in '{}' should have ollama_model set to 'llama3.2:3b'",
                                        role_name,
                                        config_path
                                    );

                                    // Validate auto summarization
                                    assert!(
                                        llm_auto_summarize.is_some(),
                                        "Role '{}' in '{}' should have 'llm_auto_summarize' field",
                                        role_name,
                                        config_path
                                    );
                                    assert!(
                                        llm_auto_summarize.unwrap().as_bool().unwrap(),
                                        "Role '{}' in '{}' should have llm_auto_summarize set to true",
                                        role_name,
                                        config_path
                                    );

                                    // Validate system prompt
                                    assert!(
                                        llm_system_prompt.is_some(),
                                        "Role '{}' in '{}' should have 'llm_system_prompt' field",
                                        role_name,
                                        config_path
                                    );
                                    assert!(
                                        !llm_system_prompt.unwrap().as_str().unwrap().is_empty(),
                                        "Role '{}' in '{}' should have a non-empty llm_system_prompt",
                                        role_name,
                                        config_path
                                    );

                                    println!(
                                        "    ✅ Role '{}' has complete Ollama configuration",
                                        role_name
                                    );
                                    println!(
                                        "      🤖 Provider: {}",
                                        llm_provider.unwrap().as_str().unwrap()
                                    );
                                    println!(
                                        "      🌐 Base URL: {}",
                                        ollama_base_url.unwrap().as_str().unwrap()
                                    );
                                    println!(
                                        "      📦 Model: {}",
                                        ollama_model.unwrap().as_str().unwrap()
                                    );
                                    println!(
                                        "      🔄 Auto Summarize: {}",
                                        llm_auto_summarize.unwrap().as_bool().unwrap()
                                    );
                                    println!(
                                        "      💬 System Prompt: {}...",
                                        &llm_system_prompt.unwrap().as_str().unwrap()[..50.min(
                                            llm_system_prompt.unwrap().as_str().unwrap().len()
                                        )]
                                    );
                                }
                            } else {
                                panic!("Roles should be an object in '{}'", config_path);
                            }
                        } else {
                            panic!(
                                "Configuration should have 'roles' field in '{}'",
                                config_path
                            );
                        }

                        println!("  ✅ Configuration file '{}' is valid", config_path);
                    }
                    Err(e) => {
                        panic!(
                            "Configuration file '{}' is not valid JSON: {}",
                            config_path, e
                        );
                    }
                }
            }
            Err(e) => {
                panic!("Could not read configuration file '{}': {}", config_path, e);
            }
        }
    }

    println!("\n🎉 ALL ROLES CONFIGURED FOR OLLAMA SUMMARIZATION!");
    println!("✅ Default role: Ollama summarization enabled");
    println!("✅ Rust Engineer role: Ollama summarization enabled");
    println!("✅ Terraphim Engineer role: Ollama summarization enabled");
    println!("🤖 All roles ready for AI-powered document summarization");
}

/// Test that validates role-specific system prompts are appropriate
#[test]
fn test_role_specific_system_prompts() {
    println!("🧪 Role-Specific System Prompts Test");
    println!("===================================");

    let config_path = "../../terraphim_server/default/combined_roles_config.json";

    match fs::read_to_string(config_path) {
        Ok(config_content) => {
            match serde_json::from_str::<Value>(&config_content) {
                Ok(config) => {
                    if let Some(roles) = config.get("roles") {
                        if let Some(roles_obj) = roles.as_object() {
                            // Test Rust Engineer prompt
                            if let Some(rust_engineer) = roles_obj.get("Rust Engineer") {
                                let prompt = rust_engineer
                                    .get("llm_system_prompt")
                                    .unwrap()
                                    .as_str()
                                    .unwrap();
                                assert!(
                                    prompt.contains("Rust"),
                                    "Rust Engineer prompt should mention 'Rust'"
                                );
                                assert!(
                                    prompt.contains("developer"),
                                    "Rust Engineer prompt should mention 'developer'"
                                );
                                println!(
                                    "✅ Rust Engineer prompt is appropriate: {}...",
                                    &prompt[..50]
                                );
                            }

                            // Test Terraphim Engineer prompt
                            if let Some(terraphim_engineer) = roles_obj.get("Terraphim Engineer") {
                                let prompt = terraphim_engineer
                                    .get("llm_system_prompt")
                                    .unwrap()
                                    .as_str()
                                    .unwrap();
                                assert!(
                                    prompt.contains("Terraphim"),
                                    "Terraphim Engineer prompt should mention 'Terraphim'"
                                );
                                assert!(
                                    prompt.contains("knowledge graphs"),
                                    "Terraphim Engineer prompt should mention 'knowledge graphs'"
                                );
                                println!(
                                    "✅ Terraphim Engineer prompt is appropriate: {}...",
                                    &prompt[..50]
                                );
                            }

                            // Test Default role prompt
                            if let Some(default_role) = roles_obj.get("Default") {
                                let prompt = default_role
                                    .get("llm_system_prompt")
                                    .unwrap()
                                    .as_str()
                                    .unwrap();
                                assert!(
                                    prompt.contains("helpful"),
                                    "Default role prompt should be helpful and general"
                                );
                                println!(
                                    "✅ Default role prompt is appropriate: {}...",
                                    &prompt[..50]
                                );
                            }
                        }
                    }
                }
                Err(e) => {
                    panic!("Configuration file is not valid JSON: {}", e);
                }
            }
        }
        Err(e) => {
            panic!("Could not read configuration file: {}", e);
        }
    }

    println!("\n✅ All role-specific system prompts are appropriate");
}
