# Terraphim Atomic Client

The `terraphim_atomic_client` crate provides a comprehensive client implementation for the Atomic Data protocol, enabling seamless integration with Atomic Data servers and resources.

## Architecture

### Core Components

#### Atomic Client
The main client struct for Atomic Data operations:

```rust
pub struct AtomicClient {
    server_url: String,
    credentials: Option<Credentials>,
    client: reqwest::Client,
}
```

**Key Features**:
- HTTP client for Atomic Data protocol
- Authentication and authorization
- Resource management
- WASM compatibility

#### Resource Management
Atomic Data resource handling:

```rust
pub struct AtomicResource {
    pub subject: String,
    pub properties: HashMap<String, Vec<AtomicValue>>,
    pub collections: HashMap<String, Vec<String>>,
}
```

## API Reference

### Basic Usage

#### Creating a Client
```rust
use terraphim_atomic_client::AtomicClient;

// Basic client without authentication
let client = AtomicClient::new("https://atomic.example.com")?;

// Client with authentication
let credentials = Credentials::new("username", "password")?;
let client = AtomicClient::with_credentials("https://atomic.example.com", credentials)?;
```

#### Resource Operations

##### Get Resource
```rust
let resource = client.get_resource("https://atomic.example.com/resource1").await?;
println!("Resource: {:?}", resource);
```

##### Create Resource
```rust
let mut resource = AtomicResource::new("https://atomic.example.com/new-resource");
resource.set_property(
    "https://schema.org/name",
    AtomicValue::String("My Resource".to_string())
)?;

let created = client.create_resource(&resource).await?;
```

##### Update Resource
```rust
let mut resource = client.get_resource("https://atomic.example.com/resource1").await?;
resource.set_property(
    "https://schema.org/description",
    AtomicValue::String("Updated description".to_string())
)?;

let updated = client.update_resource(&resource).await?;
```

##### Delete Resource
```rust
client.delete_resource("https://atomic.example.com/resource1").await?;
```

### Collection Operations

#### Get Collection
```rust
let collection = client.get_collection("https://atomic.example.com/collection1").await?;
for resource in collection.resources {
    println!("Resource: {}", resource.subject);
}
```

#### Add to Collection
```rust
client.add_to_collection(
    "https://atomic.example.com/collection1",
    "https://atomic.example.com/resource1"
).await?;
```

#### Remove from Collection
```rust
client.remove_from_collection(
    "https://atomic.example.com/collection1",
    "https://atomic.example.com/resource1"
).await?;
```

### Search Operations

#### Basic Search
```rust
let results = client.search("rust programming").await?;
for result in results {
    println!("Found: {}", result.subject);
}
```

#### Advanced Search
```rust
let query = SearchQuery {
    query: "rust programming".to_string(),
    limit: Some(10),
    offset: Some(0),
    filters: Some(vec![
        ("https://schema.org/type".to_string(), "Article".to_string())
    ]),
};

let results = client.search_advanced(&query).await?;
```

### Authentication

#### Basic Authentication
```rust
let credentials = Credentials::new("username", "password")?;
let client = AtomicClient::with_credentials(server_url, credentials)?;
```

#### Token Authentication
```rust
let credentials = Credentials::with_token("your-auth-token")?;
let client = AtomicClient::with_credentials(server_url, credentials)?;
```

#### Session Management
```rust
// Login and get session
let session = client.login("username", "password").await?;

// Use session for subsequent requests
let client = AtomicClient::with_session(server_url, session)?;
```

## Error Handling

### Custom Error Types
```rust
#[derive(Debug, thiserror::Error)]
pub enum AtomicClientError {
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),

    #[error("Authentication failed: {0}")]
    Auth(String),

    #[error("Resource not found: {0}")]
    NotFound(String),

    #[error("Invalid resource: {0}")]
    InvalidResource(String),

    #[error("Server error: {0}")]
    Server(String),
}
```

### Result Types
```rust
pub type Result<T> = std::result::Result<T, AtomicClientError>;

// Usage
let resource = client.get_resource(subject).await?;
let results = client.search(query).await?;
```

## Resource Management

### Property Operations
```rust
let mut resource = AtomicResource::new(subject);

// Set string property
resource.set_property(
    "https://schema.org/name",
    AtomicValue::String("Resource Name".to_string())
)?;

// Set number property
resource.set_property(
    "https://schema.org/position",
    AtomicValue::Integer(1)
)?;

// Set boolean property
resource.set_property(
    "https://schema.org/isPartOf",
    AtomicValue::Boolean(true)
)?;

// Set URL property
resource.set_property(
    "https://schema.org/url",
    AtomicValue::Url("https://example.com".to_string())
)?;
```

### Collection Operations
```rust
// Add to collection
resource.add_to_collection(
    "https://schema.org/hasPart",
    "https://atomic.example.com/part1"
)?;

// Remove from collection
resource.remove_from_collection(
    "https://schema.org/hasPart",
    "https://atomic.example.com/part1"
)?;
```

## WASM Compatibility

### WebAssembly Support
The client is designed for WASM deployment:

```rust
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub async fn wasm_get_resource(subject: &str) -> Result<JsValue> {
    let client = AtomicClient::new("https://atomic.example.com")?;
    let resource = client.get_resource(subject).await?;
    Ok(JsValue::from_serde(&resource)?)
}
```

### Feature Flags
```toml
[dependencies.terraphim_atomic_client]
version = "0.1.0"
features = ["wasm-bindgen"]
```

## Testing

### Unit Tests
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_get_resource() {
        let client = AtomicClient::new("https://atomic.example.com")?;
        let resource = client.get_resource("https://atomic.example.com/test").await?;
        assert_eq!(resource.subject, "https://atomic.example.com/test");
    }

    #[tokio::test]
    async fn test_create_resource() {
        let client = AtomicClient::new("https://atomic.example.com")?;
        let mut resource = AtomicResource::new("https://atomic.example.com/new");
        resource.set_property(
            "https://schema.org/name",
            AtomicValue::String("Test Resource".to_string())
        )?;

        let created = client.create_resource(&resource).await?;
        assert_eq!(created.subject, "https://atomic.example.com/new");
    }
}
```

### Integration Tests
```rust
#[test]
fn test_authentication() {
    let credentials = Credentials::new("test", "password")?;
    let client = AtomicClient::with_credentials("https://atomic.example.com", credentials)?;

    // Test authenticated operations
    let resource = client.get_resource("https://atomic.example.com/private").await?;
    assert!(resource.properties.contains_key("https://schema.org/name"));
}
```

## Configuration

### Client Configuration
```rust
let config = ClientConfig {
    timeout: Duration::from_secs(30),
    retry_attempts: 3,
    user_agent: "TerraphimAtomicClient/1.0".to_string(),
};

let client = AtomicClient::with_config("https://atomic.example.com", config)?;
```

### Authentication Configuration
```rust
let auth_config = AuthConfig {
    token: Some("your-token".to_string()),
    username: Some("username".to_string()),
    password: Some("password".to_string()),
};

let client = AtomicClient::with_auth_config("https://atomic.example.com", auth_config)?;
```

## Best Practices

### Resource Management
```rust
// Use RAII for client management
let client = AtomicClient::new(server_url)?;

// Handle errors gracefully
match client.get_resource(subject).await {
    Ok(resource) => {
        // Process resource
    }
    Err(AtomicClientError::NotFound(_)) => {
        // Handle not found
    }
    Err(e) => {
        // Handle other errors
    }
}
```

### Authentication
```rust
// Store credentials securely
let credentials = Credentials::new(username, password)?;
let client = AtomicClient::with_credentials(server_url, credentials)?;

// Use session management for long-running operations
let session = client.login(username, password).await?;
let client = AtomicClient::with_session(server_url, session)?;
```

### Error Handling
```rust
// Implement retry logic for transient errors
let mut attempts = 0;
let max_attempts = 3;

while attempts < max_attempts {
    match client.get_resource(subject).await {
        Ok(resource) => return Ok(resource),
        Err(AtomicClientError::Http(_)) if attempts < max_attempts - 1 => {
            attempts += 1;
            tokio::time::sleep(Duration::from_secs(2u64.pow(attempts))).await;
            continue;
        }
        Err(e) => return Err(e),
    }
}
```

## Integration Examples

### Desktop Application
```rust
// In desktop app
let client = AtomicClient::new(server_url)?;
let resources = client.search("rust programming").await?;

// Update UI with results
update_search_results(resources);
```

### Web Application (WASM)
```rust
#[wasm_bindgen]
pub async fn search_atomic_resources(query: &str) -> Result<JsValue> {
    let client = AtomicClient::new("https://atomic.example.com")?;
    let results = client.search(query).await?;
    Ok(JsValue::from_serde(&results)?)
}
```

### Server Integration
```rust
// Server-side Atomic Data integration
let client = AtomicClient::with_credentials(server_url, credentials)?;

// Sync local documents with Atomic Data
for document in local_documents {
    let resource = document_to_atomic_resource(document)?;
    client.create_resource(&resource).await?;
}
```

## Dependencies

### Internal Dependencies
- `terraphim_types`: Core type definitions
- `reqwest`: HTTP client implementation
- `serde`: Serialization support

### External Dependencies
- `tokio`: Async runtime
- `wasm-bindgen`: WASM bindings (optional)
- `thiserror`: Error handling

## Migration Guide

### From HTTP Client
1. Replace direct HTTP calls with Atomic Client methods
2. Use `AtomicResource` for resource management
3. Implement proper error handling
4. Add authentication if required

### From Other Atomic Data Libraries
1. Update import statements
2. Replace client instantiation
3. Update method calls to match API
4. Test authentication and error handling
