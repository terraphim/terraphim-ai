//! Interactive prompt builders for the setup wizard
//!
//! Uses dialoguer for cross-platform terminal prompts with themes.

use crate::onboarding::{validation, OnboardingError};
use dialoguer::{theme::ColorfulTheme, Confirm, Input, Password, Select};
use std::path::PathBuf;
use terraphim_automata::AutomataPath;
use terraphim_config::{Haystack, KnowledgeGraph, KnowledgeGraphLocal, ServiceType};
use terraphim_types::{KnowledgeGraphInputType, RelevanceFunction};

/// Available themes for role configuration
pub const AVAILABLE_THEMES: &[&str] = &[
    "spacelab",
    "cosmo",
    "lumen",
    "darkly",
    "united",
    "journal",
    "readable",
    "pulse",
    "superhero",
    "default",
];

/// Back option constant for navigation
const BACK_OPTION: &str = "<< Go Back";

/// Result that can include a "go back" navigation
pub enum PromptResult<T> {
    Value(T),
    Back,
}

/// Prompt for role basic info (name, shortname)
pub fn prompt_role_basics() -> Result<PromptResult<(String, Option<String>)>, OnboardingError> {
    let theme = ColorfulTheme::default();

    // Role name
    let name: String = Input::with_theme(&theme)
        .with_prompt("Role name")
        .validate_with(|input: &String| {
            if input.trim().is_empty() {
                Err("Name cannot be empty")
            } else {
                Ok(())
            }
        })
        .interact_text()?;

    // Check for back
    if name.to_lowercase() == "back" {
        return Ok(PromptResult::Back);
    }

    // Shortname (optional)
    let use_shortname = Confirm::with_theme(&theme)
        .with_prompt("Add a shortname? (for quick role switching)")
        .default(true)
        .interact()?;

    let shortname = if use_shortname {
        let short: String = Input::with_theme(&theme)
            .with_prompt("Shortname (2-8 characters)")
            .validate_with(|input: &String| {
                if input.len() < 2 || input.len() > 8 {
                    Err("Shortname should be 2-8 characters")
                } else {
                    Ok(())
                }
            })
            .interact_text()?;
        Some(short)
    } else {
        None
    };

    Ok(PromptResult::Value((name, shortname)))
}

/// Prompt for theme selection
pub fn prompt_theme() -> Result<PromptResult<String>, OnboardingError> {
    let theme = ColorfulTheme::default();

    let mut options: Vec<&str> = AVAILABLE_THEMES.to_vec();
    options.push(BACK_OPTION);

    let selection = Select::with_theme(&theme)
        .with_prompt("Select theme")
        .items(&options)
        .default(0)
        .interact()?;

    if selection == options.len() - 1 {
        return Ok(PromptResult::Back);
    }

    Ok(PromptResult::Value(options[selection].to_string()))
}

/// Prompt for relevance function selection
pub fn prompt_relevance_function() -> Result<PromptResult<RelevanceFunction>, OnboardingError> {
    let theme = ColorfulTheme::default();

    let options = vec![
        "terraphim-graph - Semantic graph-based ranking (requires KG)",
        "title-scorer    - Basic text matching",
        "bm25            - Classic information retrieval",
        "bm25f           - BM25 with field boosting",
        "bm25plus        - Enhanced BM25",
        BACK_OPTION,
    ];

    let selection = Select::with_theme(&theme)
        .with_prompt("Select relevance function")
        .items(&options)
        .default(1) // Default to title-scorer (simpler)
        .interact()?;

    if selection == options.len() - 1 {
        return Ok(PromptResult::Back);
    }

    let func = match selection {
        0 => RelevanceFunction::TerraphimGraph,
        1 => RelevanceFunction::TitleScorer,
        2 => RelevanceFunction::BM25,
        3 => RelevanceFunction::BM25F,
        4 => RelevanceFunction::BM25Plus,
        _ => RelevanceFunction::TitleScorer,
    };

    Ok(PromptResult::Value(func))
}

/// Prompt for haystack configuration (can add multiple)
pub fn prompt_haystacks() -> Result<PromptResult<Vec<Haystack>>, OnboardingError> {
    let mut haystacks = Vec::new();
    let theme = ColorfulTheme::default();

    loop {
        let service_options = vec![
            "Ripgrep  - Local filesystem search",
            "QueryRs  - Rust docs and Reddit",
            "Quickwit - Log analysis",
            "Atomic   - Atomic Data server",
            BACK_OPTION,
        ];

        println!("\n--- Add Haystack {} ---", haystacks.len() + 1);

        let selection = Select::with_theme(&theme)
            .with_prompt("Select haystack service type")
            .items(&service_options)
            .default(0)
            .interact()?;

        if selection == service_options.len() - 1 {
            if haystacks.is_empty() {
                return Ok(PromptResult::Back);
            } else {
                // Can't go back if we have haystacks, user can remove them
                println!(
                    "At least one haystack is required. Use 'done' to finish or continue adding."
                );
                continue;
            }
        }

        let service = match selection {
            0 => ServiceType::Ripgrep,
            1 => ServiceType::QueryRs,
            2 => ServiceType::Quickwit,
            3 => ServiceType::Atomic,
            _ => ServiceType::Ripgrep,
        };

        // Get location based on service type
        let location = prompt_haystack_location(&service)?;

        // Validate path for Ripgrep
        if service == ServiceType::Ripgrep {
            let expanded = validation::expand_tilde(&location);
            if !validation::path_exists(&location) && !location.starts_with(".") {
                println!("Warning: Path '{}' does not exist.", expanded);
                let proceed = Confirm::with_theme(&theme)
                    .with_prompt("Continue anyway?")
                    .default(false)
                    .interact()?;
                if !proceed {
                    // Let user enter a different path
                    let alt_location: String = Input::with_theme(&theme)
                        .with_prompt("Enter alternative path")
                        .interact_text()?;

                    haystacks.push(Haystack {
                        location: alt_location,
                        service,
                        read_only: true,
                        fetch_content: false,
                        atomic_server_secret: None,
                        extra_parameters: Default::default(),
                    });
                } else {
                    haystacks.push(Haystack {
                        location,
                        service,
                        read_only: true,
                        fetch_content: false,
                        atomic_server_secret: None,
                        extra_parameters: Default::default(),
                    });
                }
            } else {
                haystacks.push(Haystack {
                    location,
                    service,
                    read_only: true,
                    fetch_content: false,
                    atomic_server_secret: None,
                    extra_parameters: Default::default(),
                });
            }
        } else {
            // For URL-based services, prompt for auth if needed
            let extra_parameters =
                if service == ServiceType::Quickwit || service == ServiceType::Atomic {
                    prompt_service_auth(&service)?
                } else {
                    Default::default()
                };

            haystacks.push(Haystack {
                location,
                service,
                read_only: true,
                fetch_content: false,
                atomic_server_secret: None,
                extra_parameters,
            });
        }

        // Ask if user wants to add more
        let add_another = Confirm::with_theme(&theme)
            .with_prompt("Add another haystack?")
            .default(false)
            .interact()?;

        if !add_another {
            break;
        }
    }

    Ok(PromptResult::Value(haystacks))
}

/// Prompt for haystack location based on service type
fn prompt_haystack_location(service: &ServiceType) -> Result<String, OnboardingError> {
    let theme = ColorfulTheme::default();

    let (prompt, default) = match service {
        ServiceType::Ripgrep => ("Path to search (e.g., ~/Documents)", "."),
        ServiceType::QueryRs => ("QueryRs URL", "https://query.rs"),
        ServiceType::Quickwit => ("Quickwit URL", "http://localhost:7280"),
        ServiceType::Atomic => ("Atomic Server URL", "http://localhost:9883"),
        _ => ("Location", ""),
    };

    let location: String = Input::with_theme(&theme)
        .with_prompt(prompt)
        .default(default.to_string())
        .interact_text()?;

    Ok(location)
}

/// Prompt for service authentication parameters
fn prompt_service_auth(
    service: &ServiceType,
) -> Result<std::collections::HashMap<String, String>, OnboardingError> {
    let theme = ColorfulTheme::default();
    let mut params = std::collections::HashMap::new();

    let configure_auth = Confirm::with_theme(&theme)
        .with_prompt("Configure authentication?")
        .default(false)
        .interact()?;

    if !configure_auth {
        return Ok(params);
    }

    // Check for environment variables first
    let env_vars = match service {
        ServiceType::Quickwit => vec!["QUICKWIT_TOKEN", "QUICKWIT_PASSWORD"],
        ServiceType::Atomic => vec!["ATOMIC_SERVER_SECRET"],
        _ => vec![],
    };

    for var in &env_vars {
        if std::env::var(var).is_ok() {
            println!("Found {} environment variable", var);
            let use_env = Confirm::with_theme(&theme)
                .with_prompt(format!("Use {} from environment?", var))
                .default(true)
                .interact()?;

            if use_env {
                params.insert("auth_from_env".to_string(), var.to_string());
                return Ok(params);
            }
        }
    }

    // Check for 1Password integration
    let use_1password = Confirm::with_theme(&theme)
        .with_prompt("Use 1Password reference? (op://vault/item/field)")
        .default(false)
        .interact()?;

    if use_1password {
        let op_ref: String = Input::with_theme(&theme)
            .with_prompt("1Password reference")
            .with_initial_text("op://")
            .interact_text()?;
        params.insert("auth_1password".to_string(), op_ref);
        return Ok(params);
    }

    // Fallback to direct input (masked)
    match service {
        ServiceType::Quickwit => {
            let auth_type = Select::with_theme(&theme)
                .with_prompt("Authentication type")
                .items(&["Bearer token", "Basic auth (username/password)"])
                .default(0)
                .interact()?;

            if auth_type == 0 {
                let token: String = Password::with_theme(&theme)
                    .with_prompt("Bearer token")
                    .interact()?;
                params.insert("auth_token".to_string(), format!("Bearer {}", token));
            } else {
                let username: String = Input::with_theme(&theme)
                    .with_prompt("Username")
                    .interact_text()?;
                let password: String = Password::with_theme(&theme)
                    .with_prompt("Password")
                    .interact()?;
                params.insert("auth_username".to_string(), username);
                params.insert("auth_password".to_string(), password);
            }
        }
        ServiceType::Atomic => {
            let secret: String = Password::with_theme(&theme)
                .with_prompt("Atomic server secret")
                .interact()?;
            params.insert("auth_secret".to_string(), secret);
        }
        _ => {}
    }

    Ok(params)
}

/// Prompt for LLM provider configuration
pub fn prompt_llm_config() -> Result<PromptResult<LlmConfig>, OnboardingError> {
    let theme = ColorfulTheme::default();

    let options = vec![
        "Ollama (local)",
        "OpenRouter (cloud)",
        "Skip LLM configuration",
        BACK_OPTION,
    ];

    let selection = Select::with_theme(&theme)
        .with_prompt("Select LLM provider")
        .items(&options)
        .default(0)
        .interact()?;

    if selection == options.len() - 1 {
        return Ok(PromptResult::Back);
    }

    if selection == 2 {
        return Ok(PromptResult::Value(LlmConfig {
            provider: None,
            model: None,
            api_key: None,
            base_url: None,
        }));
    }

    let (provider, default_model, default_url) = match selection {
        0 => ("ollama", "llama3.2:3b", "http://127.0.0.1:11434"),
        1 => (
            "openrouter",
            "anthropic/claude-3-haiku",
            "https://openrouter.ai/api/v1",
        ),
        _ => ("ollama", "llama3.2:3b", "http://127.0.0.1:11434"),
    };

    let model: String = Input::with_theme(&theme)
        .with_prompt("Model name")
        .default(default_model.to_string())
        .interact_text()?;

    let base_url: String = Input::with_theme(&theme)
        .with_prompt("Base URL")
        .default(default_url.to_string())
        .interact_text()?;

    // API key handling for OpenRouter
    let api_key = if provider == "openrouter" {
        // Check env var first
        if std::env::var("OPENROUTER_API_KEY").is_ok() {
            println!("Found OPENROUTER_API_KEY environment variable");
            let use_env = Confirm::with_theme(&theme)
                .with_prompt("Use API key from environment?")
                .default(true)
                .interact()?;

            if use_env {
                Some("$OPENROUTER_API_KEY".to_string())
            } else {
                let key: String = Password::with_theme(&theme)
                    .with_prompt("OpenRouter API key")
                    .interact()?;
                Some(key)
            }
        } else {
            let key: String = Password::with_theme(&theme)
                .with_prompt("OpenRouter API key")
                .interact()?;
            Some(key)
        }
    } else {
        None
    };

    // Optional: test connection for Ollama
    if provider == "ollama" {
        let test_connection = Confirm::with_theme(&theme)
            .with_prompt("Test Ollama connection now?")
            .default(false)
            .interact()?;

        if test_connection {
            println!("Testing connection to {}...", base_url);
            // We can't do async here easily, so just note it
            println!("Note: Connection will be verified when you first use LLM features.");
        }
    }

    Ok(PromptResult::Value(LlmConfig {
        provider: Some(provider.to_string()),
        model: Some(model),
        api_key,
        base_url: Some(base_url),
    }))
}

/// LLM configuration from wizard
#[derive(Debug, Clone)]
pub struct LlmConfig {
    pub provider: Option<String>,
    pub model: Option<String>,
    pub api_key: Option<String>,
    pub base_url: Option<String>,
}

/// Prompt for knowledge graph configuration
pub fn prompt_knowledge_graph() -> Result<PromptResult<Option<KnowledgeGraph>>, OnboardingError> {
    let theme = ColorfulTheme::default();

    let options = vec![
        "Remote URL (pre-built automata)",
        "Local markdown files (build at startup)",
        "Skip knowledge graph",
        BACK_OPTION,
    ];

    let selection = Select::with_theme(&theme)
        .with_prompt("Knowledge graph source")
        .items(&options)
        .default(0)
        .interact()?;

    if selection == options.len() - 1 {
        return Ok(PromptResult::Back);
    }

    if selection == 2 {
        return Ok(PromptResult::Value(None));
    }

    match selection {
        0 => {
            // Remote URL
            let url: String = Input::with_theme(&theme)
                .with_prompt("Remote automata URL")
                .default(
                    "https://system-operator.s3.eu-west-2.amazonaws.com/term_to_id.json"
                        .to_string(),
                )
                .interact_text()?;

            // Validate URL on setup
            println!("Validating URL...");
            if let Err(e) = validation::validate_url(&url) {
                println!("Warning: {}", e);
                let proceed = Confirm::with_theme(&theme)
                    .with_prompt("Continue anyway?")
                    .default(false)
                    .interact()?;
                if !proceed {
                    return prompt_knowledge_graph(); // Retry
                }
            }

            Ok(PromptResult::Value(Some(KnowledgeGraph {
                automata_path: Some(AutomataPath::Remote(url)),
                knowledge_graph_local: None,
                public: true,
                publish: false,
            })))
        }
        1 => {
            // Local markdown
            let path: String = Input::with_theme(&theme)
                .with_prompt("Local KG markdown path")
                .default("docs/src/kg".to_string())
                .interact_text()?;

            // Validate path exists
            let expanded = validation::expand_tilde(&path);
            if !validation::path_exists(&path) {
                println!("Warning: Path '{}' does not exist.", expanded);
                let proceed = Confirm::with_theme(&theme)
                    .with_prompt("Continue anyway? (Path must exist when agent runs)")
                    .default(true)
                    .interact()?;
                if !proceed {
                    return prompt_knowledge_graph(); // Retry
                }
            }

            Ok(PromptResult::Value(Some(KnowledgeGraph {
                automata_path: None,
                knowledge_graph_local: Some(KnowledgeGraphLocal {
                    input_type: KnowledgeGraphInputType::Markdown,
                    path: PathBuf::from(path),
                }),
                public: false,
                publish: false,
            })))
        }
        _ => Ok(PromptResult::Value(None)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_available_themes_not_empty() {
        assert!(AVAILABLE_THEMES.len() > 1);
        assert!(AVAILABLE_THEMES.contains(&"spacelab"));
        assert!(AVAILABLE_THEMES.contains(&"darkly"));
    }

    #[test]
    fn test_llm_config_default() {
        let config = LlmConfig {
            provider: None,
            model: None,
            api_key: None,
            base_url: None,
        };
        assert!(config.provider.is_none());
    }
}
