//! Main wizard orchestration
//!
//! Provides the interactive setup wizard flow for first-time users
//! and add-role capability for extending existing configurations.

use dialoguer::{theme::ColorfulTheme, Confirm, Select};
use terraphim_config::Role;
use terraphim_types::RelevanceFunction;

use super::prompts::{
    prompt_haystacks, prompt_knowledge_graph, prompt_llm_config, prompt_relevance_function,
    prompt_role_basics, prompt_theme, PromptResult,
};
use super::templates::{ConfigTemplate, TemplateRegistry};
use super::validation::validate_role;
use super::OnboardingError;

/// Result of running the setup wizard
#[derive(Debug)]
pub enum SetupResult {
    /// User selected a quick-start template
    Template {
        /// The template that was applied
        template: ConfigTemplate,
        /// Custom path if provided
        custom_path: Option<String>,
        /// The built role
        role: Role,
    },
    /// User created a custom role configuration
    Custom {
        /// The configured role
        role: Role,
    },
    /// User cancelled the wizard
    Cancelled,
}

/// Mode for running the setup wizard
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SetupMode {
    /// First-time setup - create new configuration
    FirstRun,
    /// Add a role to existing configuration
    AddRole,
}

/// Quick start menu choices
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuickStartChoice {
    /// Terraphim Engineer with graph embeddings
    TerraphimEngineer,
    /// LLM Enforcer with bun install KG
    LlmEnforcer,
    /// Rust Developer with QueryRs
    RustEngineer,
    /// Local Notes with Ripgrep
    LocalNotes,
    /// AI Engineer with Ollama
    AiEngineer,
    /// Log Analyst with Quickwit
    LogAnalyst,
    /// Custom configuration
    Custom,
}

impl QuickStartChoice {
    /// Get the template ID for this choice
    pub fn template_id(&self) -> Option<&'static str> {
        match self {
            Self::TerraphimEngineer => Some("terraphim-engineer"),
            Self::LlmEnforcer => Some("llm-enforcer"),
            Self::RustEngineer => Some("rust-engineer"),
            Self::LocalNotes => Some("local-notes"),
            Self::AiEngineer => Some("ai-engineer"),
            Self::LogAnalyst => Some("log-analyst"),
            Self::Custom => None,
        }
    }

    /// Get the display name for this choice
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::TerraphimEngineer => {
                "Terraphim Engineer - Semantic search with knowledge graph embeddings"
            }
            Self::LlmEnforcer => "LLM Enforcer - AI agent hooks with bun install knowledge graph",
            Self::RustEngineer => "Rust Developer - Search Rust docs and crates.io via QueryRs",
            Self::LocalNotes => "Local Notes - Search markdown files in a local folder",
            Self::AiEngineer => "AI Engineer - Local Ollama LLM with knowledge graph support",
            Self::LogAnalyst => "Log Analyst - Quickwit integration for log analysis",
            Self::Custom => "Custom Configuration - Build your own role from scratch",
        }
    }

    /// Get all choices in order
    pub fn all() -> Vec<Self> {
        vec![
            Self::TerraphimEngineer,
            Self::LlmEnforcer,
            Self::RustEngineer,
            Self::LocalNotes,
            Self::AiEngineer,
            Self::LogAnalyst,
            Self::Custom,
        ]
    }
}

/// Check if this is a first run (no existing configuration)
#[allow(dead_code)]
pub fn is_first_run(config_path: &std::path::Path) -> bool {
    !config_path.exists()
}

/// Apply a template directly without interactive wizard
///
/// # Arguments
/// * `template_id` - ID of the template to apply
/// * `custom_path` - Optional custom path override
///
/// # Returns
/// The configured Role or an error
pub fn apply_template(
    template_id: &str,
    custom_path: Option<&str>,
) -> Result<Role, OnboardingError> {
    let registry = TemplateRegistry::new();

    let template = registry
        .get(template_id)
        .ok_or_else(|| OnboardingError::TemplateNotFound(template_id.to_string()))?;

    // Check if template requires path and none provided
    if template.requires_path && custom_path.is_none() {
        return Err(OnboardingError::Validation(format!(
            "Template '{}' requires a --path argument",
            template_id
        )));
    }

    let role = template.build_role(custom_path);

    // Validate the built role
    validate_role(&role).map_err(|errors| {
        OnboardingError::Validation(
            errors
                .iter()
                .map(|e| e.to_string())
                .collect::<Vec<_>>()
                .join("; "),
        )
    })?;

    Ok(role)
}

/// Run the interactive setup wizard
///
/// # Arguments
/// * `mode` - Whether this is first-run or add-role mode
///
/// # Returns
/// SetupResult indicating what the user chose
pub async fn run_setup_wizard(mode: SetupMode) -> Result<SetupResult, OnboardingError> {
    // Check if we're running in a TTY
    #[cfg(feature = "repl-interactive")]
    {
        if !atty::is(atty::Stream::Stdin) {
            return Err(OnboardingError::NotATty);
        }
    }

    let theme = ColorfulTheme::default();

    // Display welcome message
    println!();
    match mode {
        SetupMode::FirstRun => {
            println!("Welcome to Terraphim AI Setup");
            println!("-----------------------------");
            println!();
            println!("Let's configure your first role. You can add more roles later.");
        }
        SetupMode::AddRole => {
            println!("Add a New Role");
            println!("--------------");
            println!();
            println!("Configure a new role to add to your existing configuration.");
        }
    }
    println!();

    // Show quick start menu
    let choice = quick_start_menu(&theme)?;

    match choice {
        QuickStartChoice::Custom => {
            // Run full custom wizard
            match custom_wizard(&theme) {
                Ok(role) => Ok(SetupResult::Custom { role }),
                Err(OnboardingError::Cancelled) => Ok(SetupResult::Cancelled),
                Err(OnboardingError::NavigateBack) => {
                    // User went back from first step - show menu again
                    Box::pin(run_setup_wizard(mode)).await
                }
                Err(e) => Err(e),
            }
        }
        _ => {
            // Apply selected template
            let template_id = choice.template_id().unwrap();
            let registry = TemplateRegistry::new();
            let template = registry.get(template_id).unwrap().clone();

            // If template requires path, prompt for it
            let custom_path = if template.requires_path {
                Some(prompt_path_for_template(&theme, &template)?)
            } else if template.default_path.is_some() {
                // Ask if user wants to customize the default path
                let customize = Confirm::with_theme(&theme)
                    .with_prompt(format!(
                        "Default path is '{}'. Would you like to customize it?",
                        template.default_path.as_ref().unwrap()
                    ))
                    .default(false)
                    .interact()
                    .map_err(|_| OnboardingError::Cancelled)?;

                if customize {
                    Some(prompt_path_for_template(&theme, &template)?)
                } else {
                    None
                }
            } else {
                None
            };

            let role = template.build_role(custom_path.as_deref());

            // Validate the role
            validate_role(&role).map_err(|errors| {
                OnboardingError::Validation(
                    errors
                        .iter()
                        .map(|e| e.to_string())
                        .collect::<Vec<_>>()
                        .join("; "),
                )
            })?;

            Ok(SetupResult::Template {
                template,
                custom_path,
                role,
            })
        }
    }
}

/// Display the quick start menu and get user selection
fn quick_start_menu(theme: &ColorfulTheme) -> Result<QuickStartChoice, OnboardingError> {
    let choices = QuickStartChoice::all();
    let display_names: Vec<&str> = choices.iter().map(|c| c.display_name()).collect();

    println!("Select a quick-start template or create a custom configuration:");
    println!();

    let selection = Select::with_theme(theme)
        .items(&display_names)
        .default(0)
        .interact()
        .map_err(|_| OnboardingError::Cancelled)?;

    Ok(choices[selection])
}

/// Prompt user for a path when template requires it
fn prompt_path_for_template(
    theme: &ColorfulTheme,
    template: &ConfigTemplate,
) -> Result<String, OnboardingError> {
    use dialoguer::Input;

    let prompt_text = match template.id.as_str() {
        "local-notes" => "Enter the path to your notes folder",
        "llm-enforcer" => "Enter the path to your knowledge graph folder",
        _ => "Enter the path",
    };

    let default = template.default_path.clone().unwrap_or_default();

    let path: String = Input::with_theme(theme)
        .with_prompt(prompt_text)
        .default(default)
        .interact_text()
        .map_err(|_| OnboardingError::Cancelled)?;

    // Expand tilde and validate path exists
    let expanded = super::validation::expand_tilde(&path);

    if !super::validation::path_exists(&path) {
        // Path doesn't exist - ask user what to do
        println!();
        println!("Warning: Path '{}' does not exist.", expanded);

        let proceed = Confirm::with_theme(theme)
            .with_prompt("Would you like to use this path anyway?")
            .default(false)
            .interact()
            .map_err(|_| OnboardingError::Cancelled)?;

        if !proceed {
            return Err(OnboardingError::PathNotFound(expanded));
        }
    }

    Ok(path)
}

/// Run the full custom configuration wizard
fn custom_wizard(theme: &ColorfulTheme) -> Result<Role, OnboardingError> {
    println!();
    println!("Custom Role Configuration");
    println!("-------------------------");
    println!("Press Ctrl+C at any time to cancel.");
    println!();

    // Step 1: Role basics (name and shortname)
    let (name, shortname) = match prompt_role_basics()? {
        PromptResult::Value(v) => v,
        PromptResult::Back => return Err(OnboardingError::NavigateBack),
    };

    let mut role = Role::new(name);
    role.shortname = shortname;

    // Step 2: Theme selection
    role.theme = match prompt_theme()? {
        PromptResult::Value(t) => t,
        PromptResult::Back => {
            // Go back to role basics - restart wizard
            return Err(OnboardingError::NavigateBack);
        }
    };

    // Step 3: Relevance function
    let relevance = match prompt_relevance_function()? {
        PromptResult::Value(r) => r,
        PromptResult::Back => {
            // Go back - restart wizard
            return Err(OnboardingError::NavigateBack);
        }
    };
    role.relevance_function = relevance;
    // Set terraphim_it based on relevance function (TerraphimGraph requires it)
    role.terraphim_it = matches!(relevance, RelevanceFunction::TerraphimGraph);

    // Step 4: Haystacks
    role.haystacks = match prompt_haystacks()? {
        PromptResult::Value(haystacks) => haystacks,
        PromptResult::Back => {
            return Err(OnboardingError::NavigateBack);
        }
    };

    // Step 5: LLM configuration (optional)
    match prompt_llm_config()? {
        PromptResult::Value(llm_config) => {
            if let Some(provider) = llm_config.provider {
                role.llm_enabled = true;
                role.extra.insert(
                    "llm_provider".to_string(),
                    serde_json::Value::String(provider),
                );
                if let Some(model) = llm_config.model {
                    role.extra
                        .insert("ollama_model".to_string(), serde_json::Value::String(model));
                }
                if let Some(base_url) = llm_config.base_url {
                    role.extra.insert(
                        "ollama_base_url".to_string(),
                        serde_json::Value::String(base_url),
                    );
                }
                if let Some(api_key) = llm_config.api_key {
                    role.extra.insert(
                        "openrouter_api_key".to_string(),
                        serde_json::Value::String(api_key),
                    );
                }
            } else {
                role.llm_enabled = false;
            }
        }
        PromptResult::Back => {
            return Err(OnboardingError::NavigateBack);
        }
    }

    // Step 6: Knowledge graph (optional)
    role.kg = match prompt_knowledge_graph()? {
        PromptResult::Value(kg) => kg,
        PromptResult::Back => {
            return Err(OnboardingError::NavigateBack);
        }
    };

    // Validate the complete role
    validate_role(&role).map_err(|errors| {
        OnboardingError::Validation(
            errors
                .iter()
                .map(|e| e.to_string())
                .collect::<Vec<_>>()
                .join("; "),
        )
    })?;

    // Show summary and confirm
    println!();
    println!("Role Configuration Summary");
    println!("--------------------------");
    println!("Name: {}", role.name);
    if let Some(ref short) = role.shortname {
        println!("Shortname: {}", short);
    }
    println!("Theme: {}", role.theme);
    println!("Relevance: {:?}", role.relevance_function);
    println!("Haystacks: {}", role.haystacks.len());
    println!("LLM Enabled: {}", role.llm_enabled);
    println!(
        "Knowledge Graph: {}",
        if role.kg.is_some() { "Yes" } else { "No" }
    );
    println!();

    let confirm = Confirm::with_theme(theme)
        .with_prompt("Save this configuration?")
        .default(true)
        .interact()
        .map_err(|_| OnboardingError::Cancelled)?;

    if confirm {
        Ok(role)
    } else {
        Err(OnboardingError::Cancelled)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_quick_start_choice_template_ids() {
        assert_eq!(
            QuickStartChoice::TerraphimEngineer.template_id(),
            Some("terraphim-engineer")
        );
        assert_eq!(
            QuickStartChoice::LlmEnforcer.template_id(),
            Some("llm-enforcer")
        );
        assert_eq!(QuickStartChoice::Custom.template_id(), None);
    }

    #[test]
    fn test_quick_start_choice_all() {
        let choices = QuickStartChoice::all();
        assert_eq!(choices.len(), 7);
        assert_eq!(choices[0], QuickStartChoice::TerraphimEngineer);
        assert_eq!(choices[1], QuickStartChoice::LlmEnforcer);
        assert_eq!(choices[6], QuickStartChoice::Custom);
    }

    #[test]
    fn test_apply_template_terraphim_engineer() {
        let role = apply_template("terraphim-engineer", None).unwrap();
        assert_eq!(role.name.to_string(), "Terraphim Engineer");
        assert!(role.kg.is_some());
    }

    #[test]
    fn test_apply_template_with_custom_path() {
        let role = apply_template("terraphim-engineer", Some("/custom/path")).unwrap();
        assert_eq!(role.haystacks[0].location, "/custom/path");
    }

    #[test]
    fn test_apply_template_not_found() {
        let result = apply_template("nonexistent", None);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            OnboardingError::TemplateNotFound(_)
        ));
    }

    #[test]
    fn test_apply_template_requires_path() {
        let result = apply_template("local-notes", None);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            OnboardingError::Validation(_)
        ));
    }

    #[test]
    fn test_apply_template_local_notes_with_path() {
        let role = apply_template("local-notes", Some("/my/notes")).unwrap();
        assert_eq!(role.name.to_string(), "Local Notes");
        assert_eq!(role.haystacks[0].location, "/my/notes");
    }

    #[test]
    fn test_is_first_run_nonexistent_path() {
        let path = PathBuf::from("/nonexistent/config.json");
        assert!(is_first_run(&path));
    }
}
