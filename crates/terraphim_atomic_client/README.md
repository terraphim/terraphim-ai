# Atomic Server Client

A Rust client library for interacting with [Atomic Server](https://atomicdata.dev/), with support for both native and WebAssembly (WASM) environments.

## Features

- Fetch resources from Atomic Server
- Create, update, and delete resources using signed commits
- Authenticate requests using Ed25519 signatures
- Search for resources
- WASM compatibility for browser environments

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
atomic-server-client = { git = "https://github.com/yourusername/atomic-server-client" }
```

## Usage

### Environment Variables

Set these environment variables to configure the client:

- `ATOMIC_SERVER_URL`: The URL of your Atomic Server instance (e.g., `http://localhost:9883`)
- `ATOMIC_SERVER_SECRET`: Your agent's secret in base64 format

### CLI Usage

```bash
# Create a resource
atomic-server-client create <shortname> <name> <description> <class>

# Update a resource
atomic-server-client update <resource_url> <property> <value>

# Delete a resource
atomic-server-client delete <resource_url>

# Search for resources
atomic-server-client search <query>
```

### Library Usage

```rust
use atomic_server_client::{Config, Store};
use std::collections::HashMap;
use serde_json::json;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load configuration from environment variables
    let config = Config::from_env()?;
    let store = Store::new(config)?;

    // Create a resource
    let mut properties = HashMap::new();
    properties.insert(
        "https://atomicdata.dev/properties/shortname".to_string(),
        json!("my-resource"),
    );
    properties.insert(
        "https://atomicdata.dev/properties/name".to_string(),
        json!("My Resource"),
    );
    properties.insert(
        "https://atomicdata.dev/properties/description".to_string(),
        json!("A test resource"),
    );
    properties.insert(
        "https://atomicdata.dev/properties/isA".to_string(),
        json!(["https://atomicdata.dev/classes/Thing"]),
    );

    let subject = format!("{}/{}", store.config.server_url, "my-resource");
    let result = store.create_with_commit(&subject, properties).await?;
    println!("Resource created: {:#?}", result);

    Ok(())
}
```

## WebAssembly Support

To use this library in a WebAssembly environment, enable the `wasm` feature and disable the default `native` feature:

```toml
[dependencies]
atomic-server-client = { git = "https://github.com/yourusername/atomic-server-client", default-features = false, features = ["wasm"] }
```

## License

MIT
