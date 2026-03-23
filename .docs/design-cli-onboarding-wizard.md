# Implementation Plan: CLI Onboarding Wizard for terraphim-agent

**Status**: Draft
**Research Doc**: `.docs/research-cli-onboarding-wizard.md`
**Author**: AI Design Agent
**Date**: 2026-01-28
**Estimated Effort**: 2-3 days

## Overview

### Summary
Implement a CLI onboarding wizard that provides feature parity with the desktop ConfigWizard.svelte, allowing users to add roles to existing configuration, select from pre-built templates, or create custom configurations interactively.

### Approach
Implement "Quick Start + Advanced" wizard flow (Option B from research):
1. Quick start mode with template selection for common use cases
2. Full custom wizard accessible via "Custom setup" option
3. Add-role capability for extending existing configuration

### Scope

**In Scope:**
- `setup` subcommand with flags: `--template`, `--add-role`, `--list-templates`
- Interactive wizard using dialoguer for prompts
- Template loading from embedded JSON configurations
- Role creation with haystack, theme, relevance function configuration
- LLM provider configuration (Ollama, OpenRouter)
- Knowledge graph configuration (local path, remote URL)
- First-run detection and auto-prompting
- Configuration validation before save

**Out of Scope:**
- Service connectivity testing (defer to Phase 2)
- Role editing (only add new roles in v1)
- Graphical progress animations (keep simple with indicatif)
- MCP server configuration (complex, defer)

## Architecture

### Component Diagram
```
┌─────────────────────────────────────────────────────────────────────┐
│                        terraphim_agent                               │
├─────────────────────────────────────────────────────────────────────┤
│  main.rs                                                            │
│  ├── Cli struct (clap)                                              │
│  │   └── Command::Setup { template, add_role, list_templates }     │
│  └── run_offline_command() -> handle_setup_command()                │
├─────────────────────────────────────────────────────────────────────┤
│  onboarding/                                                        │
│  ├── mod.rs           # Module root, re-exports                     │
│  ├── templates.rs     # Embedded templates, template listing        │
│  ├── wizard.rs        # Interactive wizard flow                     │
│  ├── prompts.rs       # Individual prompt builders                  │
│  └── validation.rs    # Configuration validation                    │
├─────────────────────────────────────────────────────────────────────┤
│  service.rs                                                         │
│  └── TuiService::add_role()  # New method for adding roles          │
└─────────────────────────────────────────────────────────────────────┘
                                    │
                                    ▼
┌─────────────────────────────────────────────────────────────────────┐
│                     terraphim_config                                 │
│  ├── Config, Role, Haystack, KnowledgeGraph                         │
│  └── ConfigBuilder::add_role()                                      │
└─────────────────────────────────────────────────────────────────────┘
```

### Data Flow
```
User runs: terraphim-agent setup
    │
    ▼
Check TTY → Non-TTY? → Apply template directly (--template required)
    │
    ▼ (TTY)
Quick Start Menu:
    ├── [1] Terraphim Engineer   → Full KG + graph embeddings
    ├── [2] LLM Enforcer         → bun install KG for AI agent hooks
    ├── [3] Rust Developer       → QueryRs for Rust docs
    ├── [4] Local Notes Search   → Prompt for folder path
    ├── [5] Log Analyst          → Quickwit for logs
    └── [6] Custom setup...      → Full wizard flow
                                      │
                                      ▼
                                Role name → Haystack(s) → LLM → KG → Review
                                      │
                                      ▼
                                Validation → Save → Confirm
```

### Key Design Decisions

| Decision | Rationale | Alternatives Rejected |
|----------|-----------|----------------------|
| Use dialoguer for prompts | Mature, cross-platform, good UX | inquire (less mature), manual input parsing |
| Embed templates as JSON strings | No file system dependency, works in all deployments | Load from disk (fragile paths) |
| Quick Start as primary flow | Faster onboarding for most users | Full wizard first (overwhelming) |
| Add-role mode separate from new config | Preserves existing config, prevents data loss | Single mode (confusing) |
| Skip service connectivity tests in v1 | Reduces complexity, async testing is tricky in CLI | Test immediately (slow, error-prone) |
| Terraphim Engineer as first option | Primary use case with full KG and graph embeddings | Generic options first |
| LLM Enforcer with bun KG | Enables AI agent hooks for npm->bun replacement | Skip specialized templates |

## Template Registry

### Required Templates (Priority Order)

| ID | Name | Description | Key Features |
|----|------|-------------|--------------|
| `terraphim-engineer` | Terraphim Engineer | Full-featured with knowledge graph and semantic search | TerraphimGraph relevance, remote KG automata, local docs haystack |
| `llm-enforcer` | LLM Enforcer | AI agent hooks with bun install knowledge graph | bun.md KG for npm->bun replacement, hooks integration |
| `rust-engineer` | Rust Developer | Search Rust docs and crates.io | QueryRs haystack, title-scorer relevance |
| `local-notes` | Local Notes | Search markdown files in a folder | Ripgrep haystack, user-provided path |
| `ai-engineer` | AI Engineer | Local Ollama with knowledge graph | Ollama LLM, TerraphimGraph, local KG |
| `log-analyst` | Log Analyst | Quickwit for log analysis | Quickwit haystack, BM25 relevance |

### Template Configurations

#### terraphim-engineer (Primary)
```json
{
    "id": "terraphim-engineer",
    "name": "Terraphim Engineer",
    "description": "Full-featured semantic search with knowledge graph embeddings",
    "role": {
        "name": "Terraphim Engineer",
        "shortname": "terra",
        "relevance_function": "terraphim-graph",
        "terraphim_it": true,
        "theme": "spacelab",
        "kg": {
            "automata_path": {
                "remote": "https://system-operator.s3.eu-west-2.amazonaws.com/term_to_id.json"
            },
            "public": true,
            "publish": false
        },
        "haystacks": [
            {
                "location": "~/Documents",
                "service": "Ripgrep",
                "read_only": true
            }
        ],
        "llm_enabled": false
    },
    "has_llm": false,
    "has_kg": true
}
```

#### llm-enforcer (AI Hooks)
```json
{
    "id": "llm-enforcer",
    "name": "LLM Enforcer",
    "description": "AI agent hooks with bun install knowledge graph for npm replacement",
    "role": {
        "name": "LLM Enforcer",
        "shortname": "enforce",
        "relevance_function": "title-scorer",
        "terraphim_it": true,
        "theme": "darkly",
        "kg": {
            "knowledge_graph_local": {
                "input_type": "markdown",
                "path": "docs/src/kg"
            },
            "public": false,
            "publish": false
        },
        "haystacks": [
            {
                "location": ".",
                "service": "Ripgrep",
                "read_only": true
            }
        ],
        "llm_enabled": false
    },
    "has_llm": false,
    "has_kg": true
}
```

## File Changes

### New Files

| File | Purpose |
|------|---------|
| `crates/terraphim_agent/src/onboarding/mod.rs` | Module root with re-exports |
| `crates/terraphim_agent/src/onboarding/templates.rs` | Embedded templates and template listing |
| `crates/terraphim_agent/src/onboarding/wizard.rs` | Main wizard flow orchestration |
| `crates/terraphim_agent/src/onboarding/prompts.rs` | Individual prompt builders for each config section |
| `crates/terraphim_agent/src/onboarding/validation.rs` | Configuration validation utilities |

### Modified Files

| File | Changes |
|------|---------|
| `crates/terraphim_agent/src/main.rs` | Add `Setup` command to enum, handle in `run_offline_command` |
| `crates/terraphim_agent/src/service.rs` | Add `add_role()` and `update_config()` methods |
| `crates/terraphim_agent/Cargo.toml` | Add `dialoguer` dependency |

## API Design

### Public Types

```rust
// onboarding/templates.rs

/// A pre-built configuration template for quick start
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigTemplate {
    /// Unique identifier for the template
    pub id: String,
    /// Human-readable name
    pub name: String,
    /// Short description of use case
    pub description: String,
    /// The role configuration from this template
    pub role: Role,
    /// Whether this template includes LLM configuration
    pub has_llm: bool,
    /// Whether this template includes knowledge graph
    pub has_kg: bool,
}

/// Available templates for quick start
#[derive(Debug, Clone)]
pub struct TemplateRegistry {
    templates: Vec<ConfigTemplate>,
}

// onboarding/wizard.rs

/// Result of running the setup wizard
#[derive(Debug)]
pub struct SetupResult {
    /// The role that was created or selected
    pub role: Role,
    /// Whether this was a new config or added to existing
    pub mode: SetupMode,
    /// Whether configuration was saved successfully
    pub saved: bool,
}

/// Mode of setup operation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SetupMode {
    /// Creating a new configuration from scratch
    NewConfig,
    /// Adding a role to existing configuration
    AddRole,
    /// Applied a template directly
    Template,
}

// onboarding/prompts.rs

/// Service type options for haystack configuration
#[derive(Debug, Clone)]
pub struct HaystackChoice {
    pub service: ServiceType,
    pub label: String,
    pub description: String,
    pub requires_location: bool,
    pub default_location: Option<String>,
}

/// LLM provider choice for configuration
#[derive(Debug, Clone)]
pub struct LlmProviderChoice {
    pub id: String,
    pub label: String,
    pub requires_api_key: bool,
    pub default_model: String,
    pub available_models: Vec<String>,
}
```

### Public Functions

```rust
// onboarding/mod.rs

/// Run the interactive setup wizard
///
/// # Arguments
/// * `service` - TuiService for config management
/// * `add_role_only` - If true, only add role to existing config
///
/// # Returns
/// SetupResult indicating what was configured and if it was saved
///
/// # Errors
/// Returns error if user cancels or configuration fails validation
pub async fn run_setup_wizard(
    service: &TuiService,
    add_role_only: bool,
) -> Result<SetupResult, OnboardingError>;

/// Apply a template by ID
///
/// # Arguments
/// * `service` - TuiService for config management
/// * `template_id` - ID of template to apply
///
/// # Returns
/// SetupResult with applied template
///
/// # Errors
/// Returns error if template not found or config fails
pub async fn apply_template(
    service: &TuiService,
    template_id: &str,
) -> Result<SetupResult, OnboardingError>;

/// List available templates
pub fn list_templates() -> Vec<ConfigTemplate>;

/// Check if this is first run (no existing config)
pub async fn is_first_run(service: &TuiService) -> bool;

// onboarding/templates.rs

impl TemplateRegistry {
    /// Create registry with all embedded templates
    pub fn new() -> Self;

    /// Get template by ID
    pub fn get(&self, id: &str) -> Option<&ConfigTemplate>;

    /// List all templates
    pub fn list(&self) -> &[ConfigTemplate];
}

// onboarding/wizard.rs

/// Run the quick start menu
pub fn quick_start_menu() -> Result<QuickStartChoice, OnboardingError>;

/// Run the full custom wizard
pub async fn custom_wizard(service: &TuiService) -> Result<Role, OnboardingError>;

// onboarding/prompts.rs

/// Prompt for role basic info (name, shortname)
pub fn prompt_role_basics() -> Result<(String, Option<String>), OnboardingError>;

/// Prompt for theme selection
pub fn prompt_theme() -> Result<String, OnboardingError>;

/// Prompt for relevance function
pub fn prompt_relevance_function() -> Result<RelevanceFunction, OnboardingError>;

/// Prompt for haystack configuration (can add multiple)
pub fn prompt_haystacks() -> Result<Vec<Haystack>, OnboardingError>;

/// Prompt for LLM provider configuration
pub fn prompt_llm_config() -> Result<LlmConfig, OnboardingError>;

/// Prompt for knowledge graph configuration
pub fn prompt_knowledge_graph() -> Result<Option<KnowledgeGraph>, OnboardingError>;

// onboarding/validation.rs

/// Validate a role configuration
pub fn validate_role(role: &Role) -> Result<(), Vec<ValidationError>>;

/// Validate a haystack configuration
pub fn validate_haystack(haystack: &Haystack) -> Result<(), ValidationError>;
```

### Error Types

```rust
// onboarding/mod.rs

#[derive(Debug, thiserror::Error)]
pub enum OnboardingError {
    #[error("User cancelled setup")]
    Cancelled,

    #[error("Template not found: {0}")]
    TemplateNotFound(String),

    #[error("Validation failed: {0}")]
    Validation(String),

    #[error("Configuration error: {0}")]
    Config(#[from] terraphim_config::TerraphimConfigError),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Not a TTY - interactive mode requires a terminal")]
    NotATty,

    #[error("Role already exists: {0}")]
    RoleExists(String),
}
```

### CLI Command Structure

```rust
// main.rs additions

#[derive(Subcommand, Debug)]
enum Command {
    // ... existing commands ...

    /// Interactive setup wizard for configuring terraphim-agent
    Setup {
        /// Apply a specific template directly (non-interactive)
        #[arg(long)]
        template: Option<String>,

        /// Add a new role to existing configuration
        #[arg(long)]
        add_role: bool,

        /// List available templates and exit
        #[arg(long)]
        list_templates: bool,

        /// Skip setup wizard even on first run
        #[arg(long)]
        skip: bool,
    },
}
```

## Test Strategy

### Unit Tests

| Test | Location | Purpose |
|------|----------|---------|
| `test_template_registry_has_terraphim_engineer` | `templates.rs` | Verify terraphim-engineer template exists |
| `test_template_registry_has_llm_enforcer` | `templates.rs` | Verify llm-enforcer template exists |
| `test_template_deserialization` | `templates.rs` | Verify templates parse correctly |
| `test_validate_role_valid` | `validation.rs` | Happy path validation |
| `test_validate_role_missing_name` | `validation.rs` | Error on empty name |
| `test_validate_role_missing_haystack` | `validation.rs` | Error on no haystacks |
| `test_validate_haystack_valid_ripgrep` | `validation.rs` | Ripgrep haystack validation |
| `test_validate_haystack_invalid_service` | `validation.rs` | Error handling |
| `test_terraphim_engineer_has_kg` | `templates.rs` | KG config present |
| `test_llm_enforcer_has_bun_kg` | `templates.rs` | Local KG path is docs/src/kg |

### Integration Tests

| Test | Location | Purpose |
|------|----------|---------|
| `test_apply_template_terraphim_engineer` | `tests/onboarding.rs` | Full template application flow |
| `test_apply_template_llm_enforcer` | `tests/onboarding.rs` | LLM Enforcer with bun KG |
| `test_add_role_to_existing` | `tests/onboarding.rs` | Add role preserves existing |
| `test_first_run_detection` | `tests/onboarding.rs` | First-run logic |

### Manual Testing Checklist

```markdown
## Manual Test Cases

### Quick Start Flow
- [ ] `terraphim-agent setup` shows quick start menu
- [ ] Terraphim Engineer is first option
- [ ] LLM Enforcer is second option
- [ ] Selecting template applies correctly
- [ ] Custom setup leads to full wizard
- [ ] ESC/Ctrl+C cancels gracefully

### Template Application
- [ ] `terraphim-agent setup --list-templates` shows all 6 templates
- [ ] `terraphim-agent setup --template terraphim-engineer` applies with KG
- [ ] `terraphim-agent setup --template llm-enforcer` applies with bun KG
- [ ] Invalid template shows helpful error

### Add Role Mode
- [ ] `terraphim-agent setup --add-role` adds to existing config
- [ ] Existing roles are preserved
- [ ] Duplicate role name prompts for rename

### Custom Wizard Steps
- [ ] Role name/shortname prompt works
- [ ] Theme selection shows all options
- [ ] Relevance function selection includes terraphim-graph
- [ ] Haystack addition flow (add multiple, remove)
- [ ] LLM config optional skip works
- [ ] Knowledge graph shows local and remote options
- [ ] Review step shows correct JSON
- [ ] Save confirmation works

### Edge Cases
- [ ] Non-TTY shows helpful message
- [ ] Empty config file handled
- [ ] Corrupted config shows error, offers reset
```

## Implementation Steps

### Step 1: Add Dependencies and Module Structure
**Files:** `Cargo.toml`, `src/onboarding/mod.rs`
**Description:** Add dialoguer dependency and create module skeleton
**Tests:** Compilation check
**Estimated:** 30 minutes

```toml
# Cargo.toml addition
[dependencies]
dialoguer = "0.11"
```

```rust
// src/onboarding/mod.rs
mod templates;
mod wizard;
mod prompts;
mod validation;

pub use templates::{ConfigTemplate, TemplateRegistry};
pub use wizard::{run_setup_wizard, SetupResult, SetupMode};

#[derive(Debug, thiserror::Error)]
pub enum OnboardingError {
    // ... errors ...
}
```

### Step 2: Implement Template Registry
**Files:** `src/onboarding/templates.rs`
**Description:** Embed JSON templates, implement registry
**Tests:** `test_template_registry_*` tests
**Dependencies:** Step 1
**Estimated:** 2 hours

Key templates to embed (in order):
1. `terraphim-engineer` - Full KG with graph embeddings (primary)
2. `llm-enforcer` - bun install KG for AI agent hooks
3. `rust-engineer` - QueryRs for Rust docs
4. `local-notes` - Ripgrep for local folder
5. `ai-engineer` - Ollama with KG
6. `log-analyst` - Quickwit for logs

```rust
// Embedded template example - Terraphim Engineer
const TERRAPHIM_ENGINEER_TEMPLATE: &str = r#"{
    "id": "terraphim-engineer",
    "name": "Terraphim Engineer",
    "description": "Full-featured semantic search with knowledge graph embeddings",
    "role": {
        "name": "Terraphim Engineer",
        "shortname": "terra",
        "relevance_function": "terraphim-graph",
        "terraphim_it": true,
        "theme": "spacelab",
        "kg": {
            "automata_path": {
                "remote": "https://system-operator.s3.eu-west-2.amazonaws.com/term_to_id.json"
            },
            "public": true,
            "publish": false
        },
        "haystacks": [{
            "location": "~/Documents",
            "service": "Ripgrep",
            "read_only": true
        }]
    },
    "has_llm": false,
    "has_kg": true
}"#;

// LLM Enforcer with bun install KG
const LLM_ENFORCER_TEMPLATE: &str = r#"{
    "id": "llm-enforcer",
    "name": "LLM Enforcer",
    "description": "AI agent hooks with bun install knowledge graph for npm replacement",
    "role": {
        "name": "LLM Enforcer",
        "shortname": "enforce",
        "relevance_function": "title-scorer",
        "terraphim_it": true,
        "theme": "darkly",
        "kg": {
            "knowledge_graph_local": {
                "input_type": "markdown",
                "path": "docs/src/kg"
            },
            "public": false,
            "publish": false
        },
        "haystacks": [{
            "location": ".",
            "service": "Ripgrep",
            "read_only": true
        }]
    },
    "has_llm": false,
    "has_kg": true
}"#;
```

### Step 3: Implement Validation
**Files:** `src/onboarding/validation.rs`
**Description:** Validation functions for Role, Haystack, KG
**Tests:** All validation unit tests
**Dependencies:** Step 1
**Estimated:** 1 hour

```rust
pub fn validate_role(role: &Role) -> Result<(), Vec<ValidationError>> {
    let mut errors = Vec::new();

    if role.name.to_string().is_empty() {
        errors.push(ValidationError::EmptyField("name".into()));
    }

    if role.haystacks.is_empty() {
        errors.push(ValidationError::MissingHaystack);
    }

    for haystack in &role.haystacks {
        if let Err(e) = validate_haystack(haystack) {
            errors.push(e);
        }
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}
```

### Step 4: Implement Individual Prompts
**Files:** `src/onboarding/prompts.rs`
**Description:** Each prompt function for wizard steps
**Tests:** Manual testing (dialoguer is interactive)
**Dependencies:** Step 1
**Estimated:** 3 hours

```rust
use dialoguer::{theme::ColorfulTheme, Select, Input, Confirm, MultiSelect};

pub fn prompt_role_basics() -> Result<(String, Option<String>), OnboardingError> {
    let theme = ColorfulTheme::default();

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

    let use_shortname = Confirm::with_theme(&theme)
        .with_prompt("Add a shortname? (for quick switching)")
        .default(false)
        .interact()?;

    let shortname = if use_shortname {
        Some(Input::with_theme(&theme)
            .with_prompt("Shortname")
            .interact_text()?)
    } else {
        None
    };

    Ok((name, shortname))
}

pub fn prompt_theme() -> Result<String, OnboardingError> {
    let themes = vec![
        "spacelab", "cosmo", "lumen", "darkly", "united",
        "journal", "readable", "pulse", "superhero", "default"
    ];

    let selection = Select::with_theme(&ColorfulTheme::default())
        .with_prompt("Select theme")
        .items(&themes)
        .default(0)
        .interact()?;

    Ok(themes[selection].to_string())
}

pub fn prompt_knowledge_graph() -> Result<Option<KnowledgeGraph>, OnboardingError> {
    let theme = ColorfulTheme::default();

    let kg_options = vec![
        "Remote URL (pre-built automata)",
        "Local markdown files (build at startup)",
        "Skip (no knowledge graph)",
    ];

    let selection = Select::with_theme(&theme)
        .with_prompt("Knowledge graph source")
        .items(&kg_options)
        .default(0)
        .interact()?;

    match selection {
        0 => {
            let url: String = Input::with_theme(&theme)
                .with_prompt("Remote automata URL")
                .default("https://system-operator.s3.eu-west-2.amazonaws.com/term_to_id.json".into())
                .interact_text()?;

            Ok(Some(KnowledgeGraph {
                automata_path: Some(AutomataPath::Remote(url)),
                knowledge_graph_local: None,
                public: true,
                publish: false,
            }))
        }
        1 => {
            let path: String = Input::with_theme(&theme)
                .with_prompt("Local KG markdown path")
                .default("docs/src/kg".into())
                .interact_text()?;

            Ok(Some(KnowledgeGraph {
                automata_path: None,
                knowledge_graph_local: Some(KnowledgeGraphLocal {
                    input_type: KnowledgeGraphInputType::Markdown,
                    path: PathBuf::from(path),
                }),
                public: false,
                publish: false,
            }))
        }
        2 => Ok(None),
        _ => unreachable!(),
    }
}

pub fn prompt_haystacks() -> Result<Vec<Haystack>, OnboardingError> {
    let mut haystacks = Vec::new();
    let theme = ColorfulTheme::default();

    loop {
        let service_options = vec![
            ("Ripgrep - Local filesystem search", ServiceType::Ripgrep),
            ("QueryRs - Rust docs and Reddit", ServiceType::QueryRs),
            ("Quickwit - Log analysis", ServiceType::Quickwit),
            ("Atomic - Atomic Data server", ServiceType::Atomic),
        ];

        let selection = Select::with_theme(&theme)
            .with_prompt("Select haystack service type")
            .items(&service_options.iter().map(|(l, _)| *l).collect::<Vec<_>>())
            .interact()?;

        let service = service_options[selection].1;

        let location: String = Input::with_theme(&theme)
            .with_prompt("Location (path or URL)")
            .interact_text()?;

        let read_only = Confirm::with_theme(&theme)
            .with_prompt("Read-only?")
            .default(true)
            .interact()?;

        haystacks.push(Haystack::new(location, service, read_only));

        let add_another = Confirm::with_theme(&theme)
            .with_prompt("Add another haystack?")
            .default(false)
            .interact()?;

        if !add_another {
            break;
        }
    }

    Ok(haystacks)
}
```

### Step 5: Implement Wizard Flow
**Files:** `src/onboarding/wizard.rs`
**Description:** Main wizard orchestration with quick start menu
**Tests:** Integration tests
**Dependencies:** Steps 2, 3, 4
**Estimated:** 2 hours

```rust
pub fn quick_start_menu() -> Result<QuickStartChoice, OnboardingError> {
    let options = vec![
        "[1] Terraphim Engineer - Full KG with graph embeddings (recommended)",
        "[2] LLM Enforcer       - AI agent hooks with bun install KG",
        "[3] Rust Developer     - Search Rust docs and crates.io",
        "[4] Local Notes        - Search markdown files in a folder",
        "[5] AI Engineer        - Local Ollama with knowledge graph",
        "[6] Log Analyst        - Quickwit for log analysis",
        "[7] Custom setup...    - Configure everything manually",
    ];

    let selection = Select::with_theme(&ColorfulTheme::default())
        .with_prompt("Quick Start - Choose a template")
        .items(&options)
        .default(0)
        .interact()?;

    match selection {
        0 => Ok(QuickStartChoice::Template("terraphim-engineer".into())),
        1 => Ok(QuickStartChoice::Template("llm-enforcer".into())),
        2 => Ok(QuickStartChoice::Template("rust-engineer".into())),
        3 => Ok(QuickStartChoice::Template("local-notes".into())),
        4 => Ok(QuickStartChoice::Template("ai-engineer".into())),
        5 => Ok(QuickStartChoice::Template("log-analyst".into())),
        6 => Ok(QuickStartChoice::Custom),
        _ => unreachable!(),
    }
}

pub async fn custom_wizard(service: &TuiService) -> Result<Role, OnboardingError> {
    println!("\n=== Custom Role Setup ===\n");

    // Step 1: Basic info
    let (name, shortname) = prompts::prompt_role_basics()?;

    // Step 2: Theme
    let theme = prompts::prompt_theme()?;

    // Step 3: Relevance function
    let relevance = prompts::prompt_relevance_function()?;

    // Step 4: Haystacks
    let haystacks = prompts::prompt_haystacks()?;

    // Step 5: LLM (optional)
    let llm_config = if Confirm::with_theme(&ColorfulTheme::default())
        .with_prompt("Configure LLM provider?")
        .default(false)
        .interact()?
    {
        Some(prompts::prompt_llm_config()?)
    } else {
        None
    };

    // Step 6: Knowledge Graph (optional)
    let kg = if relevance == RelevanceFunction::TerraphimGraph {
        println!("TerraphimGraph requires a knowledge graph.");
        Some(prompts::prompt_knowledge_graph()?)
    } else if Confirm::with_theme(&ColorfulTheme::default())
        .with_prompt("Configure knowledge graph?")
        .default(false)
        .interact()?
    {
        Some(prompts::prompt_knowledge_graph()?)
    } else {
        None
    };

    // Build role
    let mut role = Role::new(&name);
    role.shortname = shortname;
    role.theme = theme;
    role.relevance_function = relevance;
    role.haystacks = haystacks;
    role.kg = kg;

    if let Some(llm) = llm_config {
        role.llm_enabled = true;
        role.llm_model = Some(llm.model);
        // ... apply other LLM settings
    }

    // Validate
    validation::validate_role(&role)?;

    // Review
    println!("\n=== Review ===");
    println!("{}", serde_json::to_string_pretty(&role)?);

    if !Confirm::with_theme(&ColorfulTheme::default())
        .with_prompt("Save this configuration?")
        .default(true)
        .interact()?
    {
        return Err(OnboardingError::Cancelled);
    }

    Ok(role)
}
```

### Step 6: CLI Integration
**Files:** `src/main.rs`
**Description:** Add Setup command, wire up to wizard
**Tests:** Integration tests, manual testing
**Dependencies:** Step 5
**Estimated:** 1 hour

```rust
// In Command enum
Setup {
    #[arg(long)]
    template: Option<String>,
    #[arg(long)]
    add_role: bool,
    #[arg(long)]
    list_templates: bool,
},

// In run_offline_command
Command::Setup { template, add_role, list_templates } => {
    if list_templates {
        println!("Available templates:\n");
        for t in onboarding::list_templates() {
            println!("  {:<20} - {}", t.id, t.description);
        }
        return Ok(());
    }

    if let Some(template_id) = template {
        let result = onboarding::apply_template(&service, &template_id).await?;
        println!("Applied template: {}", template_id);
        println!("Role '{}' added to configuration.", result.role.name);
        return Ok(());
    }

    // Check TTY
    if !atty::is(atty::Stream::Stdout) {
        return Err(anyhow::anyhow!(
            "Interactive setup requires a terminal. Use --template for non-interactive mode.\n\
             Available templates: terraphim-engineer, llm-enforcer, rust-engineer, local-notes, ai-engineer, log-analyst"
        ));
    }

    let result = onboarding::run_setup_wizard(&service, add_role).await?;
    println!("\nSetup complete! Role '{}' configured.", result.role.name);
    println!("Use 'terraphim-agent roles select {}' to switch to this role.", result.role.name);
    Ok(())
}
```

### Step 7: Service Layer Updates
**Files:** `src/service.rs`
**Description:** Add methods for role management
**Tests:** Unit tests for add_role
**Dependencies:** Step 1
**Estimated:** 1 hour

```rust
impl TuiService {
    /// Add a new role to the configuration
    pub async fn add_role(&self, role: Role) -> Result<()> {
        let mut config = self.config_state.config.lock().await;

        if config.roles.contains_key(&role.name) {
            return Err(anyhow::anyhow!("Role '{}' already exists", role.name));
        }

        config.roles.insert(role.name.clone(), role);
        drop(config);

        self.save_config().await?;
        Ok(())
    }

    /// Check if any roles exist (first-run detection)
    pub async fn has_roles(&self) -> bool {
        let config = self.config_state.config.lock().await;
        !config.roles.is_empty()
    }
}
```

### Step 8: Documentation and Polish
**Files:** `README.md` sections, inline docs
**Description:** User-facing documentation, help text
**Tests:** Doc tests
**Dependencies:** Steps 1-7
**Estimated:** 1 hour

## Rollback Plan

If issues discovered:
1. Remove `Setup` command from CLI
2. Revert Cargo.toml changes
3. Remove `onboarding/` module

The setup wizard is additive and doesn't modify existing code paths, so rollback is straightforward.

## Dependencies

### New Dependencies

| Crate | Version | Justification |
|-------|---------|---------------|
| dialoguer | 0.11 | Industry-standard CLI prompts with themes |

### No Dependency Updates Required

The existing `indicatif` and `console` crates are already available for progress indicators if needed.

## Performance Considerations

### Expected Performance

| Metric | Target | Measurement |
|--------|--------|-------------|
| Wizard startup | < 200ms | Manual timing |
| Template application | < 100ms | Manual timing |
| Config save | < 500ms | Already measured |

### No Benchmarks Needed

This is a user-interactive feature; performance is dominated by user input time.

## Open Items

| Item | Status | Owner |
|------|--------|-------|
| Confirm template selection with stakeholder | Done | Alex |
| Decide on first-run auto-prompt behavior | Pending | Alex |

## Approval Checklist

- [x] Template selection confirmed (Terraphim Engineer + LLM Enforcer)
- [ ] Test strategy approved
- [ ] CLI command structure approved
- [ ] Human approval received

## Next Steps

After Phase 2 approval:
1. ~~Conduct specification interview (Phase 2.5) to refine edge cases~~ DONE
2. Proceed to implementation (Phase 3)
3. Create GitHub issue for tracking

---

## Specification Interview Findings

**Interview Date**: 2026-01-28
**Dimensions Covered**: Failure Modes, Edge Cases, User Mental Models, Security, Integration Effects, Migration, Operational Concerns
**Convergence Status**: Complete (3 rounds, no new concerns in final round)

### Key Decisions from Interview

#### Path Handling & Validation
- **Template paths**: If hardcoded path (e.g., `~/Documents`) doesn't exist, prompt user for alternative path
- **Relative paths**: Validate exists first; if not found, prompt for alternative or expand to absolute
- **KG remote URLs**: Validate on setup via HTTP HEAD request (may be slow but catches errors early)

#### First-Run Experience
- **Auto-launch wizard**: When `terraphim-agent` is run with no subcommand and no config exists, prompt "No config found. Run setup wizard? [Y/n]"
- **Review step suffices**: No `--dry-run` flag needed; existing review step before save serves this purpose

#### Cancellation & Error Handling
- **Ctrl+C handling**: Discard completely, no partial state saved, clean exit code 0
- **Corrupt config**: Backup to `config.json.bak` and start fresh with empty config
- **Save failure**: Show complete JSON to stdout + suggest alternative path + provide export command (all three)

#### Template System
- **Parameterized templates**: Templates like `local-notes` require `--path` flag in non-interactive mode: `terraphim-agent setup --template local-notes --path ~/notes`
- **Multi-template support**: After applying first template, offer "Add another template/role? [y/N]"
- **Name conflicts**: When role name already exists, offer to overwrite with confirmation: "Role X exists. Overwrite? [y/N]"

#### Credential Management
- **Priority chain for LLM API keys**:
  1. Check common environment variables (OPENROUTER_API_KEY, OLLAMA_BASE_URL, etc.)
  2. Check for 1Password integration, prompt for op:// references
  3. Final fallback: masked password input (asterisks, not stored in history)

#### Navigation & UX
- **Back navigation**: Add "Go back" option at each step of the custom wizard
- **Role activation**: After setup, ask "Activate this role now? [Y/n]" instead of auto-activating

#### Integration & Compatibility
- **Desktop merge**: Full merge with existing config regardless of source (CLI reads, adds, saves back)
- **Service checks**: Optional for Ollama: "Test Ollama connection now? [y/N]" during wizard
- **Auth params**: Full auth configuration support for Quickwit/QueryRs (username/password or token)

#### CI/Automation
- **Output format**: Respect existing `--format json|human` flag for non-interactive template application
- **Silent mode**: Use `--format json` for machine-readable output in CI pipelines

### Updated CLI Flags

Based on interview, the Setup command should support:

```rust
Setup {
    #[arg(long)]
    template: Option<String>,

    #[arg(long)]
    add_role: bool,

    #[arg(long)]
    list_templates: bool,

    /// Path for templates that require location input (e.g., local-notes)
    #[arg(long)]
    path: Option<String>,

    /// Skip first-run wizard prompt
    #[arg(long)]
    skip: bool,
}
```

### Deferred Items
- **MCP server configuration**: Complex; defer to future version
- **Role editing**: v1 supports add only; edit requires manual config.json modification
- **Accessibility (screen reader)**: Standard CLI; no special handling needed for v1

### Interview Summary

The specification interview clarified 18 key decisions across error handling, user experience, and integration concerns. The most significant findings are:

1. **Graceful degradation**: The wizard should validate paths and URLs during setup rather than failing silently at runtime. This includes KG URL validation via HTTP HEAD and local path existence checks with prompts for alternatives.

2. **Credential security**: API keys follow a priority chain (env vars -> 1Password -> masked input) rather than storing plaintext in config files.

3. **Desktop compatibility**: CLI and desktop share config files with full merge semantics, ensuring roles created in either interface are preserved.

4. **Navigation freedom**: Users can go back at each step of the custom wizard, reducing frustration from accidental selections.

5. **CI-friendly**: The `--format` flag and `--path` parameter enable fully non-interactive operation for automation scenarios.
