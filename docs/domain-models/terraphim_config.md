# terraphim_config - Configuration Management

## Overview

`terraphim_config` provides configuration management for the Terraphim AI system. It handles role definitions, haystack configurations, knowledge graph settings, and LLM integration. The crate supports environment variable expansion and multi-source configuration loading.

## Domain Model

### Core Concepts

#### TerraphimConfig
Main configuration container with roles and global settings.

```rust
pub struct TerraphimConfig {
    pub roles: AHashMap<RoleName, Role>,
}
```

**Key Responsibilities:**
- Store role configurations
- Provide role lookup
- Support configuration updates
- Enable configuration persistence

#### Role
User profile with specific knowledge domains, search preferences, and LLM settings.

```rust
pub struct Role {
    pub shortname: Option<String>,
    pub name: RoleName,
    pub relevance_function: RelevanceFunction,
    pub terraphim_it: bool,
    pub theme: String,
    pub kg: Option<KnowledgeGraph>,
    pub haystacks: Vec<Haystack>,
    pub llm_enabled: bool,
    pub llm_api_key: Option<String>,
    pub llm_model: Option<String>,
    pub llm_auto_summarize: bool,
    pub llm_chat_enabled: bool,
    pub llm_chat_system_prompt: Option<String>,
    pub llm_chat_model: Option<String>,
    pub llm_context_window: Option<u64>,
    pub extra: AHashMap<String, Value>,
    pub llm_router_enabled: bool,
    pub llm_router_config: Option<LlmRouterConfig>,
}
```

**Key Responsibilities:**
- Define user knowledge domains
- Configure search relevance
- Manage LLM integration
- Specify data sources (haystacks)

#### Haystack
Data source containing searchable documents.

```rust
pub struct Haystack {
    pub location: String,
    pub service: ServiceType,
    pub read_only: bool,
    pub fetch_content: bool,
    pub atomic_server_secret: Option<String>,
    pub extra_parameters: std::collections::HashMap<String, String>,
}
```

**Key Responsibilities:**
- Define data source location
- Specify indexing service
- Control read/write behaviour
- Support service-specific parameters

## Data Models

### Role Configuration

#### RelevanceFunction
Algorithm for ranking search results.

```rust
pub enum RelevanceFunction {
    TitleScorer,
    BM25,
    BM25F,
    BM25Plus,
    TerraphimGraph,
}
```

**Use Cases:**
- `TitleScorer`: Simple title matching
- `BM25`: Okapi BM25 algorithm
- `BM25F`: Field-length normalised BM25
- `BM25Plus`: BM25 with additional features
- `TerraphimGraph`: Knowledge graph-based ranking

#### KnowledgeGraph
Knowledge graph configuration for a role.

```rust
pub struct KnowledgeGraph {
    pub automata_path: Option<String>,
    pub knowledge_graph_local: Option<LocalKnowledgeGraph>,
    pub graph_type: Option<String>,
}
```

**Use Cases:**
- Specify remote automata URL
- Configure local knowledge graph path
- Define graph type

#### LocalKnowledgeGraph
Local knowledge graph source configuration.

```rust
pub struct LocalKnowledgeGraph {
    pub path: String,
    pub format: Option<String>,
}
```

**Use Cases:**
- Specify local file path
- Define graph format (optional)
- Enable local graph loading

### Service Configuration

#### ServiceType
Supported indexing services.

```rust
pub enum ServiceType {
    Ripgrep,
    Atomic,
    QueryRs,
    ClickUp,
    Mcp,
    Perplexity,
    GrepApp,
    AiAssistant,
    Quickwit,
    Jmap,
}
```

**Use Cases:**
- `Ripgrep`: Local filesystem search
- `Atomic`: Atomic Data server
- `QueryRs`: Reddit + Rust docs search
- `ClickUp`: Task management
- `Mcp`: Model Context Protocol
- `Perplexity`: AI-powered web search
- `GrepApp`: GitHub code search
- `AiAssistant`: AI coding assistant logs
- `Quickwit`: Log and observability data
- `Jmap`: Email protocol

### LLM Configuration

#### LlmRouterConfig
Intelligent LLM routing configuration.

```rust
pub struct LlmRouterConfig {
    pub default_provider: Option<String>,
    pub default_model: Option<String>,
    pub routing_rules: Vec<RoutingRule>,
}
```

**Use Cases:**
- Specify default provider
- Configure default model
- Define routing rules

#### RoutingRule
Rule-based LLM provider selection.

```rust
pub struct RoutingRule {
    pub capability: String,
    pub provider: String,
    pub model: String,
    pub priority: Priority,
}
```

**Use Cases:**
- Define capability-based routing
- Specify provider and model
- Set routing priority

#### Priority
Priority levels for routing decisions.

```rust
pub enum Priority {
    High,
    Medium,
    Low,
}
```

**Use Cases:**
- Rule ordering
- Fallback prioritisation
- Resource allocation

## Implementation Patterns

### Configuration Loading

#### Path Expansion
```rust
pub fn expand_path(path: &str) -> PathBuf {
    let mut result = path.to_string();

    /// Get home directory using multiple fallback strategies
    fn get_home_dir() -> Option<PathBuf> {
        if let Some(home) = dirs::home_dir() {
            return Some(home);
        }
        if let Ok(home) = std::env::var("HOME") {
            return Some(PathBuf::from(home));
        }
        if let Ok(profile) = std::env::var("USERPROFILE") {
            return Some(PathBuf::from(profile));
        }
        None
    }

    // Handle ${VAR:-default} syntax
    loop {
        if let Some(start) = result.find("${") {
            if let Some(colon_pos) = result[start..].find(":-") {
                let colon_pos = start + colon_pos;
                let var_name = &result[start + 2..colon_pos];
                let after_colon = colon_pos + 2;
                let mut depth = 1;
                let mut end_pos = after_colon;
                for (i, c) in result[after_colon..].char_indices() {
                    match c {
                        '{' => depth += 1,
                        '}' => {
                            depth -= 1;
                            if depth == 0 {
                                end_pos = after_colon + i;
                                break;
                            }
                        }
                        _ => {}
                    }
                }
                if depth == 0 {
                    let default_value = &result[after_colon..end_pos];
                    let replacement = std::env::var(var_name)
                        .unwrap_or_else(|_| default_value.to_string());
                    result = format!("{}{}{}", &result[..start], replacement, &result[end_pos + 1..]);
                    continue;
                }
            }
        }
        break;
    }

    // Handle ${VAR} syntax
    let re_braces = regex::Regex::new(r"\$\{([^}]+)\}").unwrap();
    result = re_braces.replace_all(&result, |caps: &regex::Captures| {
        let var_name = &caps[1];
        if var_name == "HOME" {
            get_home_dir()
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_else(|| format!("${{{}}", var_name))
        } else {
            std::env::var(var_name).unwrap_or_else(|_| format!("${{{}}", var_name))
        }
    }).to_string();

    // Handle $VAR syntax
    let re_dollar = regex::Regex::new(r"\$([A-Za-z_][A-Za-z0-9_]*)").unwrap();
    result = re_dollar.replace_all(&result, |caps: &regex::Captures| {
        let var_name = &caps[1];
        if var_name == "HOME" {
            get_home_dir()
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_else(|| format!("${}", var_name))
        } else {
            std::env::var(var_name).unwrap_or_else(|_| format!("${}", var_name))
        }
    }).to_string();

    // Handle ~ at beginning
    if result.starts_with('~') {
        if let Some(home) = get_home_dir() {
            result = result.replacen('~', &home.to_string_lossy(), 1);
        }
    }

    PathBuf::from(result)
}
```

**Pattern:**
- Support shell-like variable expansion
- Handle `${VAR:-default}` syntax
- Handle `${VAR}` and `$VAR` syntax
- Expand `~` to home directory
- Use multiple fallback strategies

#### Default Context Window
```rust
fn default_context_window() -> Option<u64> {
    Some(32768)
}
```

**Default:** 32,768 tokens (~262,144 characters)

### Role Management

#### Role Creation
```rust
impl Role {
    pub fn new(name: impl Into<RoleName>) -> Self {
        Self {
            shortname: None,
            name: name.into(),
            relevance_function: RelevanceFunction::TitleScorer,
            terraphim_it: false,
            theme: "default".to_string(),
            kg: None,
            haystacks: vec![],
            llm_enabled: false,
            llm_api_key: None,
            llm_model: None,
            llm_auto_summarize: false,
            llm_chat_enabled: false,
            llm_chat_system_prompt: None,
            llm_chat_model: None,
            llm_context_window: default_context_window(),
            extra: AHashMap::new(),
            llm_router_enabled: false,
            llm_router_config: None,
        }
    }
}
```

**Pattern:**
- Provide sensible defaults
- Use builder pattern via `new()`
- Support all optional fields
- Default to safe values

#### LLM Validation
```rust
impl Role {
    pub fn has_llm_config(&self) -> bool {
        self.llm_enabled && self.llm_api_key.is_some() && self.llm_model.is_some()
    }

    pub fn get_llm_model(&self) -> Option<&str> {
        self.llm_model.as_deref()
    }
}
```

**Pattern:**
- Check all required fields present
- Provide convenience accessors
- Return safe defaults

### Haystack Management

#### Haystack Creation
```rust
impl Haystack {
    pub fn new(location: String, service: ServiceType, read_only: bool) -> Self {
        Self {
            location,
            service,
            read_only,
            fetch_content: false,
            atomic_server_secret: None,
            extra_parameters: std::collections::HashMap::new(),
        }
    }

    pub fn new_with_atomic_secret(
        location: String,
        service: ServiceType,
        read_only: bool,
        atomic_server_secret: String
    ) -> Self {
        Self {
            location,
            service,
            read_only,
            fetch_content: false,
            atomic_server_secret: Some(atomic_server_secret),
            extra_parameters: std::collections::HashMap::new(),
        }
    }
}
```

**Pattern:**
- Basic constructor for common cases
- Atomic secret constructor for Atomic service
- Support extra parameters via HashMap

#### Haystack Serialisation
```rust
impl Serialize for Haystack {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;

        let mut field_count = 3; // location, service, read_only

        let include_atomic_secret =
            self.service == ServiceType::Atomic && self.atomic_server_secret.is_some();
        if include_atomic_secret {
            field_count += 1;
        }

        if !self.extra_parameters.is_empty() {
            field_count += 1;
        }

        let mut state = serializer.serialize_struct("Haystack", field_count)?;
        state.serialize_field("location", &self.location)?;
        state.serialize_field("service", &self.service)?;
        state.serialize_field("read_only", &self.read_only)?;

        if include_atomic_secret {
            state.serialize_field("atomic_server_secret", &self.atomic_server_secret)?;
        }

        if !self.extra_parameters.is_empty() {
            state.serialize_field("extra_parameters", &self.extra_parameters)?;
        }

        state.end()
    }
}
```

**Pattern:**
- Conditionally include optional fields
- Only include atomic secret for Atomic service
- Only include extra_parameters if not empty
- Dynamic field count calculation

## Error Handling

### Error Types

```rust
#[derive(Error, Debug)]
pub enum TerraphimConfigError {
    #[error("Unable to load config")]
    NotFound,

    #[error("At least one role is required")]
    NoRoles,

    #[error("Profile error")]
    Profile(String),

    #[error("Persistence error")]
    Persistence(Box<terraphim_persistence::Error>),

    #[error("Serde JSON error")]
    Json(#[from] serde_json::Error),

    #[error("Cannot initialize tracing subscriber")]
    TracingSubscriber(Box<dyn std::error::Error + Send + Sync>),

    #[error("Pipe error")]
    Pipe(#[from] terraphim_rolegraph::Error),

    #[error("Automata error")]
    Automata(#[from] terraphim_automata::TerraphimAutomataError),

    #[error("Url error")]
    Url(#[from] url::ParseError),

    #[error("IO error")]
    Io(#[from] std::io::Error),

    #[error("Config error")]
    Config(String),
}
```

**Categories:**
- **Configuration**: Config loading errors
- **Validation**: Role/profile validation
- **Integration**: Dependency errors
- **I/O**: File system errors

## Performance Optimisations

### Lazy Evaluation

#### Configuration Access
```rust
impl TerraphimConfig {
    pub fn get_role(&self, role_name: &RoleName) -> Option<&Role> {
        self.roles.get(role_name)
    }
}
```

**Pattern:**
- Use `AHashMap` for fast lookups
- Return references to avoid cloning
- Use `Option<T>` for safe access

#### Default Values
```rust
impl Role {
    pub fn get_llm_model(&self) -> Option<&str> {
        self.llm_model.as_deref()
    }
}
```

**Pattern:**
- Provide convenience accessors
- Return references to strings
- Handle `None` gracefully

## Testing Patterns

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_role_creation() {
        let role = Role::new("DataScientist");
        assert_eq!(role.name.as_str(), "DataScientist");
        assert_eq!(role.relevance_function, RelevanceFunction::TitleScorer);
    }

    #[test]
    fn test_llm_validation() {
        let mut role = Role::new("Test");
        role.llm_enabled = true;
        role.llm_api_key = Some("test-key".to_string());
        role.llm_model = Some("gpt-3.5".to_string());

        assert!(role.has_llm_config());
        assert_eq!(role.get_llm_model(), Some("gpt-3.5"));
    }

    #[test]
    fn test_path_expansion() {
        std::env::set_var("HOME", "/home/user");
        std::env::set_var("TEST_VAR", "test-value");

        let expanded = expand_path("${HOME}/test/${TEST_VAR:-default}");
        assert_eq!(expanded, PathBuf::from("/home/user/test/test-value"));

        let expanded = expand_path("~/test");
        assert_eq!(expanded, PathBuf::from("/home/user/test"));
    }

    #[test]
    fn test_haystack_serialisation() {
        let mut haystack = Haystack::new(
            "/path/to/data".to_string(),
            ServiceType::Ripgrep,
            false
        );

        haystack.extra_parameters.insert("filter".to_string(), "*.md".to_string());

        let json = serde_json::to_string(&haystack).unwrap();
        let deserialised: Haystack = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialised.location, haystack.location);
        assert_eq!(deserialised.service, haystack.service);
        assert_eq!(
            deserialised.extra_parameters.get("filter"),
            haystack.extra_parameters.get("filter")
        );
    }
}
```

## Best Practices

### Configuration Design

- Provide sensible defaults
- Validate at load time
- Support environment variables
- Document all options

### Path Handling

- Support shell-like expansion
- Handle cross-platform differences
- Use absolute paths internally
- Preserve user-friendly paths in config

### Role Management

- Use unique identifiers
- Support role switching
- Validate role consistency
- Provide role templates

### LLM Integration

- Secure API key handling
- Model versioning support
- Fallback provider configuration
- Context window management

## Future Enhancements

### Planned Features

#### Configuration Validation
```rust
pub fn validate_config(&self) -> Result<Vec<ValidationError>> {
    // Validate all roles
    // Check haystack connectivity
    // Validate LLM credentials
}
```

#### Configuration Migration
```rust
pub fn migrate_config(&mut self, from_version: &str) -> Result<()> {
    // Handle schema changes
    // Migrate old formats
    // Preserve user data
}
```

#### Configuration Profiles
```rust
pub struct ConfigProfile {
    pub name: String,
    pub roles: AHashMap<RoleName, Role>,
    pub settings: HashMap<String, Value>,
}

pub fn switch_profile(&mut self, profile: &str) -> Result<()> {
    // Switch active profile
}
```

## References

- [Serde documentation](https://serde.rs/)
- [Regex documentation](https://docs.rs/regex/)
- [Dirs crate for paths](https://docs.rs/dirs/)
- [ThisError for error handling](https://docs.rs/thiserror/)
