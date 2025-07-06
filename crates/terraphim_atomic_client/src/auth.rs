//! Authentication utilities for Atomic Server.
//!
//! This module provides functions for creating authentication headers
//! using Ed25519 signatures, as required by the Atomic Server API.

use crate::{
    error::AtomicError,
    Result,
};
#[cfg(feature = "native")]
use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
#[cfg(not(feature = "native"))]
use std::collections::HashMap;
#[cfg(feature = "native")]
use std::time::{SystemTime, UNIX_EPOCH};
use std::sync::Arc;
use base64::{engine::general_purpose::STANDARD, Engine};
use serde_json;
use ed25519_dalek::{Keypair, PublicKey, Signer};

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
pub fn get_authentication_headers(agent: &Agent, subject: &str, _method: &str) -> Result<HeaderMap> {
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
pub fn get_authentication_headers(agent: &Agent, subject: &str, _method: &str) -> Result<HashMap<String,String>> {
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
    /// The Ed25519 keypair for signing requests
    pub keypair: Arc<Keypair>,
}

impl Agent {
    /// Creates a new agent with a randomly generated keypair.
    ///
    /// # Returns
    ///
    /// A new agent with a random keypair
    pub fn new() -> Self {
        // Create a keypair using the rand 0.5 compatible OsRng
        use rand_core::{OsRng as RngCore};
        let mut csprng = RngCore;
        let keypair = Keypair::generate(&mut csprng);
        let public_key_b64 = STANDARD.encode(keypair.public.as_bytes());
        
        Self {
            subject: format!("http://localhost:9883/agents/{}", public_key_b64),
            keypair: Arc::new(keypair),
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
        let private_key = secret["privateKey"].as_str()
            .ok_or_else(|| AtomicError::Authentication("Missing privateKey in secret".to_string()))?;
        let subject = secret["subject"].as_str()
            .ok_or_else(|| AtomicError::Authentication("Missing subject in secret".to_string()))?;
        
        // Decode the private key
        let private_key_bytes = STANDARD.decode(private_key)?;
        
        // Create the keypair from the private key bytes
        // For Ed25519 version 1.0, we need to use from_bytes
        let mut keypair_bytes = [0u8; 64];
        // Copy the private key bytes to the first 32 bytes of the keypair
        keypair_bytes[..32].copy_from_slice(&private_key_bytes);
        
        // Get the public key from the secret or derive it from the private key
        let public_key_bytes = match secret["publicKey"].as_str() {
            Some(public_key_str) => {
                match STANDARD.decode(public_key_str) {
                    Ok(bytes) => bytes,
                    Err(_) => {
                        // If we can't decode the public key, derive it from the private key
                        let secret_key = ed25519_dalek::SecretKey::from_bytes(&private_key_bytes)
                            .map_err(|e| AtomicError::Authentication(format!("Failed to create secret key: {:?}", e)))?;
                        let public_key = PublicKey::from(&secret_key);
                        public_key.as_bytes().to_vec()
                    }
                }
            },
            None => {
                // If there's no public key in the secret, derive it from the private key
                let secret_key = ed25519_dalek::SecretKey::from_bytes(&private_key_bytes)
                    .map_err(|e| AtomicError::Authentication(format!("Failed to create secret key: {:?}", e)))?;
                let public_key = PublicKey::from(&secret_key);
                public_key.as_bytes().to_vec()
            }
        };
        
        // Copy the public key bytes to the last 32 bytes of the keypair
        keypair_bytes[32..].copy_from_slice(&public_key_bytes);
        
        let keypair = Keypair::from_bytes(&keypair_bytes)
            .map_err(|e| AtomicError::Authentication(format!("Failed to create keypair: {:?}", e)))?;
        
        Ok(Self {
            subject: subject.to_string(),
            keypair: Arc::new(keypair),
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
        STANDARD.encode(self.keypair.public.as_bytes())
    }
}
