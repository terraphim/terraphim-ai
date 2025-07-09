# Haystack Extra Parameters

This document describes the new haystack extra parameters functionality that enables advanced filtering and configuration for both Ripgrep and Atomic Server haystacks.

## Overview

The haystack refactoring introduces two major improvements:

1. **Security Enhancement**: Prevents atomic server secrets from being exposed in configurations for non-Atomic haystacks
2. **Extra Parameters Support**: Enables advanced filtering and customization through key-value parameters

## Security Features

### Conditional Secret Serialization

The `Haystack` struct now uses custom serialization logic that conditionally includes `atomic_server_secret`:

- **Ripgrep haystacks**: Never serialize `atomic_server_secret` (security protection)
- **Atomic haystacks with secret**: Include `atomic_server_secret` when present
- **Atomic haystacks without secret**: Exclude `atomic_server_secret` field entirely

```rust
// ✅ Secure: Ripgrep haystack won't expose secrets
let ripgrep_haystack = Haystack::new(
    "docs/".to_string(),
    ServiceType::Ripgrep,
    false,
);
// JSON output: {"location":"docs/","service":"Ripgrep","read_only":false}

// ✅ Secure: Atomic haystack includes secret only when needed
let atomic_haystack = Haystack::new(
    "http://localhost:9883".to_string(),
    ServiceType::Atomic,
    true,
).with_atomic_secret(Some("secret123".to_string()));
// JSON output includes: "atomic_server_secret": "secret123"
```

## Extra Parameters

### Supported Ripgrep Parameters

The `extra_parameters` HashMap supports the following ripgrep filtering options:

| Parameter | Description | Example | Ripgrep Args |
|-----------|-------------|---------|--------------|
| `tag` | Filter files containing specific tags | `"#rust"` | `--glob *#rust*` |
| `glob` | Direct glob pattern | `"*.rs"` | `--glob *.rs` |
| `type` | File type filter | `"md"` | `-t md` |
| `max_count` | Maximum matches per file | `"5"` | `--max-count 5` |
| `context` | Context lines around matches | `"7"` | `-C 7` |
| `case_sensitive` | Enable case-sensitive search | `"true"` | `--case-sensitive` |

### Usage Examples

#### Tag-Based Filtering

```rust
use terraphim_config::{Haystack, ServiceType};
use std::collections::HashMap;

// Filter only files tagged with #rust
let rust_haystack = Haystack::new(
    "src/".to_string(),
    ServiceType::Ripgrep,
    false,
)
.with_extra_parameter("tag".to_string(), "#rust".to_string())
.with_extra_parameter("type".to_string(), "rs".to_string());
```

#### Documentation Search

```rust
// Search documentation with extended context
let docs_haystack = Haystack::new(
    "documentation/".to_string(),
    ServiceType::Ripgrep,
    true,
)
.with_extra_parameter("tag".to_string(), "#docs".to_string())
.with_extra_parameter("type".to_string(), "md".to_string())
.with_extra_parameter("context".to_string(), "5".to_string());
```

#### Multiple Parameters

```rust
let mut params = HashMap::new();
params.insert("tag".to_string(), "#testing".to_string());
params.insert("case_sensitive".to_string(), "true".to_string());
params.insert("max_count".to_string(), "10".to_string());

let test_haystack = Haystack::new(
    "tests/".to_string(),
    ServiceType::Ripgrep,
    true,
)
.with_extra_parameters(params);
```

## Builder API

### Construction Methods

```rust
// Basic construction
let haystack = Haystack::new(location, service_type, read_only);

// With atomic secret (only affects Atomic service)
let haystack = haystack.with_atomic_secret(Some("secret".to_string()));

// With extra parameters
let haystack = haystack.with_extra_parameters(params_map);

// Add single parameter
let haystack = haystack.with_extra_parameter("key".to_string(), "value".to_string());

// Get parameters reference
let params = haystack.get_extra_parameters();
```

### Chaining Example

```rust
let advanced_haystack = Haystack::new(
    "research/".to_string(),
    ServiceType::Ripgrep,
    false,
)
.with_extra_parameter("tag".to_string(), "#research".to_string())
.with_extra_parameter("type".to_string(), "md".to_string())
.with_extra_parameter("context".to_string(), "3".to_string())
.with_extra_parameter("max_count".to_string(), "15".to_string());
```

## Configuration Examples

### Role Configuration with Tag Filtering

```json
{
  "roles": {
    "Rust Developer": {
      "name": "Rust Developer",
      "relevance_function": "TitleScorer",
      "theme": "superhero",
      "haystacks": [
        {
          "location": "src/",
          "service": "Ripgrep",
          "read_only": false,
          "extra_parameters": {
            "tag": "#rust",
            "type": "rs",
            "max_count": "20"
          }
        }
      ]
    }
  }
}
```

### Multi-Haystack Role with Different Filters

```json
{
  "roles": {
    "Full Stack Developer": {
      "name": "Full Stack Developer",
      "relevance_function": "TitleScorer",
      "theme": "lumen",
      "haystacks": [
        {
          "location": "backend/",
          "service": "Ripgrep",
          "read_only": false,
          "extra_parameters": {
            "tag": "#backend",
            "type": "rs"
          }
        },
        {
          "location": "frontend/",
          "service": "Ripgrep",
          "read_only": false,
          "extra_parameters": {
            "tag": "#frontend",
            "glob": "*.{ts,js,svelte}"
          }
        },
        {
          "location": "http://localhost:9883",
          "service": "Atomic",
          "read_only": true,
          "atomic_server_secret": "your_secret_here",
          "extra_parameters": {
            "timeout": "30"
          }
        }
      ]
    }
  }
}
```

## Implementation Details

### RipgrepCommand Integration

The `RipgrepCommand` automatically processes extra parameters:

```rust
// In RipgrepIndexer::index()
let extra_args = self.command.parse_extra_parameters(haystack.get_extra_parameters());
let messages = if extra_args.is_empty() {
    self.command.run(needle, haystack_path).await?
} else {
    self.command.run_with_extra_args(needle, haystack_path, &extra_args).await?
};
```

### Parameter Processing

The `parse_extra_parameters()` method converts HashMap entries to ripgrep arguments:

```rust
pub fn parse_extra_parameters(&self, extra_params: &HashMap<String, String>) -> Vec<String> {
    let mut args = Vec::new();
    for (key, value) in extra_params {
        match key.as_str() {
            "tag" => {
                args.push("--glob".to_string());
                args.push(format!("*{}*", value));
            }
            "type" => {
                args.push("-t".to_string());
                args.push(value.clone());
            }
            // ... other parameters
        }
    }
    args
}
```

## Testing

The functionality is thoroughly tested with these test scenarios:

- **Security**: Verification that atomic secrets are not exposed for Ripgrep haystacks
- **Parameter Parsing**: Validation of all supported parameter types
- **Builder API**: Testing of all construction and chaining methods
- **Integration**: End-to-end testing with RipgrepIndexer
- **Serialization**: Complete JSON serialization scenarios
- **Use Cases**: Real-world tag filtering examples

Run tests with:
```bash
cd crates/terraphim_middleware
cargo test haystack_extra_parameters_test -- --nocapture
```

## Migration Guide

### Existing Configurations

Existing haystack configurations remain fully compatible:

```rust
// Old style (still works)
let haystack = Haystack {
    location: "docs/".to_string(),
    service: ServiceType::Ripgrep,
    read_only: false,
    atomic_server_secret: None,
    extra_parameters: std::collections::HashMap::new(),
};

// New style (recommended)
let haystack = Haystack::new("docs/".to_string(), ServiceType::Ripgrep, false);
```

### Adding Extra Parameters

To add filtering to existing haystacks:

```rust
// Add tag filtering to existing role
.with_extra_parameter("tag".to_string(), "#documentation".to_string())
.with_extra_parameter("type".to_string(), "md".to_string())
```

## Best Practices

1. **Use tag filtering** for focused searches within large codebases
2. **Combine type and tag filters** for maximum precision
3. **Set appropriate context lines** for your use case (default is 3)
4. **Use max_count** to limit results for performance
5. **Enable case_sensitive** for exact term matching when needed
6. **Never manually set atomic_server_secret** for Ripgrep haystacks (will be ignored)

## Performance Considerations

- Tag filtering with glob patterns may be slower than type filtering
- Multiple parameters are processed sequentially by ripgrep
- Consider using `max_count` to limit results for better performance
- Context lines (`-C`) increase memory usage for large files

## Future Enhancements

Potential future additions to extra parameters:

- `before_context` and `after_context` for asymmetric context
- `max_depth` for directory traversal limits
- `exclude_patterns` for negative filtering
- `encoding` for non-UTF8 file handling
- Custom atomic server query parameters 