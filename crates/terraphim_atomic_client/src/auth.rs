//! Authentication utilities for Atomic Server.
//!
//! This module provides functions for creating authentication headers
//! using Ed25519 signatures, as required by the Atomic Server API.

use crate::{Result, error::AtomicError};
use base64::{Engine, engine::general_purpose::STANDARD};
use ed25519_dalek::{Signer, SigningKey};
#[cfg(feature = "native")]
use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
#[cfg(not(feature = "native"))]
use std::collections::HashMap;
use std::sync::Arc;

/// Gets the authentication headers for a request to the given subject.
///
/// # Arguments
///
/// * `agent` - The agent to use for authentication
/// * `subject` - The subject URL of the resource being accessed
/// * `method` - The HTTP method being used
///
/// # Returns
///
/// A Result containing the authentication headers or an error if authentication fails
#[cfg(feature = "native")]
pub fn get_authentication_headers(
    agent: &Agent,
    subject: &str,
    _method: &str,
) -> Result<HeaderMap> {
    let mut headers = HeaderMap::new();

    // Get the current timestamp (seconds)
    let timestamp = crate::time_utils::unix_timestamp_secs().to_string();

    // Message format: "{subject} {timestamp}" as specified in Atomic Data authentication docs
    let canonical_subject = subject.trim_end_matches('/');
    let message = format!("{} {}", canonical_subject, timestamp);
    let signature = agent.sign(message.as_bytes())?;

    headers.insert(
        HeaderName::from_static("x-atomic-public-key"),
        HeaderValue::from_str(&agent.get_public_key_base64())?,
    );
    headers.insert(
        HeaderName::from_static("x-atomic-signature"),
        HeaderValue::from_str(&signature)?,
    );
    headers.insert(
        HeaderName::from_static("x-atomic-timestamp"),
        HeaderValue::from_str(&timestamp)?,
    );
    headers.insert(
        HeaderName::from_static("x-atomic-agent"),
        HeaderValue::from_str(&agent.subject)?,
    );
    Ok(headers)
}

#[cfg(not(feature = "native"))]
pub fn get_authentication_headers(
    agent: &Agent,
    subject: &str,
    _method: &str,
) -> Result<HashMap<String, String>> {
    let mut headers = HashMap::new();

    let timestamp = crate::time_utils::unix_timestamp_secs().to_string();

    let canonical_subject = subject.trim_end_matches('/');
    let message = format!("{} {}", canonical_subject, timestamp);
    let signature = agent.sign(message.as_bytes())?;

    headers.insert("x-atomic-public-key".into(), agent.get_public_key_base64());
    headers.insert("x-atomic-signature".into(), signature);
    headers.insert("x-atomic-timestamp".into(), timestamp);
    headers.insert("x-atomic-agent".into(), agent.subject.clone());
    Ok(headers)
}

/// Agent represents an entity that can authenticate with an Atomic Server.
#[derive(Debug, Clone)]
pub struct Agent {
    /// The subject URL of the agent
    pub subject: String,
    /// The Ed25519 signing key for signing requests
    pub keypair: Arc<SigningKey>,
    /// The timestamp when the agent was created
    pub created_at: i64,
    /// The name of the agent (optional)
    pub name: Option<String>,
}

impl Default for Agent {
    fn default() -> Self {
        Self::new()
    }
}

impl Agent {
    /// Creates a new agent with a randomly generated keypair.
    ///
    /// # Returns
    ///
    /// A new agent with a random keypair
    pub fn new() -> Self {
        // Create a keypair using the rand 0.5 compatible OsRng
        use rand_core::OsRng as RngCore;
        let mut csprng = RngCore;
        let signing_key = SigningKey::generate(&mut csprng);
        let public_key_b64 = STANDARD.encode(signing_key.verifying_key().as_bytes());

        Self {
            subject: format!("http://localhost:9883/agents/{}", public_key_b64),
            keypair: Arc::new(signing_key),
            created_at: crate::time_utils::unix_timestamp_secs(),
            name: None,
        }
    }

    /// Creates an agent from a base64-encoded secret.
    ///
    /// # Arguments
    ///
    /// * `secret_base64` - The base64-encoded secret
    ///
    /// # Returns
    ///
    /// A new agent or an error if the secret is invalid
    pub fn from_base64(secret_base64: &str) -> Result<Self> {
        // Decode the base64 string
        let secret_bytes = STANDARD.decode(secret_base64)?;

        // Parse the JSON
        let secret: serde_json::Value = serde_json::from_slice(&secret_bytes)?;

        // Extract the private key and subject
        let private_key = secret["privateKey"].as_str().ok_or_else(|| {
            AtomicError::Authentication("Missing privateKey in secret".to_string())
        })?;
        let subject = secret["subject"]
            .as_str()
            .ok_or_else(|| AtomicError::Authentication("Missing subject in secret".to_string()))?;

        // Decode the private key with padding fix
        let private_key_bytes = {
            let mut padded_key = private_key.to_string();
            while padded_key.len() % 4 != 0 {
                padded_key.push('=');
            }
            STANDARD.decode(&padded_key)?
        };

        // Create the keypair from the private key bytes
        // Create signing key from private key bytes
        let private_key_array: [u8; 32] = private_key_bytes
            .try_into()
            .map_err(|_| AtomicError::Authentication("Invalid private key length".to_string()))?;
        let signing_key = SigningKey::from_bytes(&private_key_array);

        // Get the public key from the secret or derive it from the private key
        let _public_key_bytes = match secret["publicKey"].as_str() {
            Some(public_key_str) => {
                let res = {
                    let mut padded_key = public_key_str.to_string();
                    while padded_key.len() % 4 != 0 {
                        padded_key.push('=');
                    }
                    STANDARD.decode(&padded_key)
                };
                match res {
                    Ok(bytes) => bytes,
                    Err(_) => {
                        // If we can't decode the public key, derive it from the private key
                        let public_key = signing_key.verifying_key();
                        public_key.as_bytes().to_vec()
                    }
                }
            }
            None => {
                // If there's no public key in the secret, derive it from the private key
                let public_key = signing_key.verifying_key();
                public_key.as_bytes().to_vec()
            }
        };

        Ok(Self {
            subject: subject.to_string(),
            keypair: Arc::new(signing_key),
            created_at: crate::time_utils::unix_timestamp_secs(),
            name: None,
        })
    }

    /// Signs a message using the agent's private key.
    ///
    /// # Arguments
    ///
    /// * `message` - The message to sign
    ///
    /// # Returns
    ///
    /// The signature as a base64-encoded string
    pub fn sign(&self, message: &[u8]) -> Result<String> {
        let signature = self.keypair.sign(message);
        Ok(STANDARD.encode(signature.to_bytes()))
    }

    /// Gets the agent's public key as a base64-encoded string.
    ///
    /// # Returns
    ///
    /// The public key as a base64-encoded string
    pub fn get_public_key_base64(&self) -> String {
        STANDARD.encode(self.keypair.verifying_key().as_bytes())
    }

    /// Creates a new agent with the given name and randomly generated keypair.
    ///
    /// # Arguments
    ///
    /// * `name` - The name of the agent
    /// * `server_url` - The base URL of the atomic server
    ///
    /// # Returns
    ///
    /// A new agent with the given name and a random keypair
    pub fn new_with_name(name: String, server_url: String) -> Self {
        use rand_core::OsRng as RngCore;
        let mut csprng = RngCore;
        let signing_key = SigningKey::generate(&mut csprng);
        let public_key_b64 = STANDARD.encode(signing_key.verifying_key().as_bytes());

        Self {
            subject: format!(
                "{}/agents/{}",
                server_url.trim_end_matches('/'),
                public_key_b64
            ),
            keypair: Arc::new(signing_key),
            created_at: crate::time_utils::unix_timestamp_secs(),
            name: Some(name),
        }
    }

    /// Creates a new agent from a private key.
    ///
    /// # Arguments
    ///
    /// * `private_key_base64` - The base64-encoded private key
    /// * `server_url` - The base URL of the atomic server
    /// * `name` - The name of the agent (optional)
    ///
    /// # Returns
    ///
    /// A new agent or an error if the private key is invalid
    pub fn new_from_private_key(
        private_key_base64: &str,
        server_url: String,
        name: Option<String>,
    ) -> Result<Self> {
        // Decode the private key with padding fix
        let private_key_bytes = {
            let mut padded_key = private_key_base64.to_string();
            while padded_key.len() % 4 != 0 {
                padded_key.push('=');
            }
            STANDARD.decode(&padded_key)?
        };

        // Create the keypair from the private key bytes
        let mut keypair_bytes = [0u8; 64];
        keypair_bytes[..32].copy_from_slice(&private_key_bytes);

        // Derive the public key from the private key
        let private_key_array: [u8; 32] = private_key_bytes
            .try_into()
            .map_err(|_| AtomicError::Authentication("Invalid private key length".to_string()))?;
        let signing_key = SigningKey::from_bytes(&private_key_array);
        let public_key = signing_key.verifying_key();
        let public_key_bytes = public_key.as_bytes();

        // In ed25519-dalek 2.x, we don't need to create a keypair bytes array
        // Just use the signing_key directly

        let public_key_b64 = STANDARD.encode(public_key_bytes);

        Ok(Self {
            subject: format!(
                "{}/agents/{}",
                server_url.trim_end_matches('/'),
                public_key_b64
            ),
            keypair: Arc::new(signing_key),
            created_at: crate::time_utils::unix_timestamp_secs(),
            name,
        })
    }

    /// Creates a new agent from a public key only (read-only agent).
    ///
    /// # Arguments
    ///
    /// * `public_key_base64` - The base64-encoded public key
    /// * `server_url` - The base URL of the atomic server
    ///
    /// # Returns
    ///
    /// A new read-only agent or an error if the public key is invalid
    pub fn new_from_public_key(public_key_base64: &str, server_url: String) -> Result<Self> {
        // Decode and validate the public key with padding fix
        let public_key_bytes = {
            let mut padded_key = public_key_base64.to_string();
            while padded_key.len() % 4 != 0 {
                padded_key.push('=');
            }
            STANDARD.decode(&padded_key)?
        };
        if public_key_bytes.len() != 32 {
            return Err(AtomicError::Authentication(
                "Invalid public key length, should be 32 bytes".to_string(),
            ));
        }

        // Create a dummy keypair with zeros for the private key (this agent won't be able to sign)
        let mut keypair_bytes = [0u8; 64];
        keypair_bytes[32..].copy_from_slice(&public_key_bytes);

        // For read-only agents, we need to create a signing key from the public key bytes
        // This is a workaround since ed25519-dalek 2.x doesn't have Keypair::from_bytes
        let mut signing_key_bytes = [0u8; 32];
        signing_key_bytes.copy_from_slice(&public_key_bytes);
        let signing_key = SigningKey::from_bytes(&signing_key_bytes);

        Ok(Self {
            subject: format!(
                "{}/agents/{}",
                server_url.trim_end_matches('/'),
                public_key_base64
            ),
            keypair: Arc::new(signing_key),
            created_at: crate::time_utils::unix_timestamp_secs(),
            name: None,
        })
    }

    /// Gets the name of the agent.
    ///
    /// # Returns
    ///
    /// The name of the agent, if set
    pub fn get_name(&self) -> Option<&str> {
        self.name.as_deref()
    }

    /// Sets the name of the agent.
    ///
    /// # Arguments
    ///
    /// * `name` - The name to set
    pub fn set_name(&mut self, name: String) {
        self.name = Some(name);
    }

    /// Gets the creation timestamp of the agent.
    ///
    /// # Returns
    ///
    /// The creation timestamp as a Unix timestamp
    pub fn get_created_at(&self) -> i64 {
        self.created_at
    }
}
