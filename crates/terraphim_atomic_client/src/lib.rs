/*!
 * # Atomic Server Client
 *
 * A Rust client library for interacting with Atomic Server, with support for both native and WASM environments.
 *
 * ## Features
 *
 * - Fetch resources from Atomic Server
 * - Create, update, and delete resources
 * - Authenticate requests using Ed25519 signatures
 * - WASM compatibility for browser environments
 *
 * ## Usage
 *
 * ```rust,no_run
 * use atomic_server_client::{types::{Config, Resource}, store::Store};
 * use std::collections::HashMap;
 * use serde_json::json;
 *
 * #[tokio::main]
 * async fn main() {
 *     // Load configuration from environment variables
 *     let config = Config::from_env().unwrap();
 *     let store = Store::new(config).unwrap();
 *
 *     // Fetch a resource
 *     let resource = store.get_resource("https://atomicdata.dev").await.unwrap();
 *     println!("Resource: {:?}", resource);
 *
 *     // Create a resource
 *     let mut properties = HashMap::new();
 *     properties.insert(
 *         "https://atomicdata.dev/properties/description".to_string(),
 *         json!("A test resource")
 *     );
 *
 *     let new_resource = Resource {
 *         subject: "https://example.com/resources/test".to_string(),
 *         properties,
 *     };
 *
 *     store.create_resource(new_resource).await.unwrap();
 * }
 * ```
 */

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

mod auth;
mod error;
pub mod http;
pub mod store;
pub mod time_utils;
pub mod types;

pub use auth::Agent;
#[cfg(feature = "native")]
pub use auth::get_authentication_headers;
pub use error::AtomicError;

/// Result type used throughout the library
pub type Result<T> = std::result::Result<T, AtomicError>;

// Re-export commonly used types for convenience
pub use store::Store;
pub use types::{Commit, CommitBuilder, Config, Resource};

#[cfg(all(target_arch = "wasm32", feature = "export_store"))]
#[wasm_bindgen]
impl Store {
    #[wasm_bindgen(constructor)]
    pub fn wasm_new(server_url: &str) -> Result<Store, JsValue> {
        let config = Config {
            server_url: server_url.to_string(),
            agent: None,
        };
        Store::new(config).map_err(|e| JsValue::from_str(&e.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config() {
        let config = Config {
            server_url: "http://localhost:9883".to_string(),
            agent: None,
        };

        assert_eq!(config.server_url, "http://localhost:9883");
        assert!(config.agent.is_none());
    }
}
