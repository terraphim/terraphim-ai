use crate::auth::Agent;
use crate::{error::AtomicError, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

/// Configuration for connecting to an Atomic Server.
#[derive(Debug, Clone)]
pub struct Config {
    /// The base URL of the Atomic Server
    pub server_url: String,
    /// Optional agent for authentication
    pub agent: Option<Agent>,
}

impl Config {
    /// Creates a new Config from environment variables.
    ///
    /// # Environment variables
    ///
    /// * `ATOMIC_SERVER_URL` - The URL of the Atomic Server
    /// * `ATOMIC_SERVER_SECRET` - The secret for authentication
    ///
    /// # Returns
    ///
    /// A Result containing the Config or an error if the environment variables are not set
    pub fn from_env() -> Result<Self> {
        // Load .env file if present
        dotenvy::dotenv().ok();

        // Print all environment variables for debugging
        println!("Reading environment variables...");
        for (key, value) in std::env::vars() {
            if key.starts_with("ATOMIC_") {
                println!(
                    "Found env var: {} = {}",
                    key,
                    if key.contains("SECRET") {
                        "[REDACTED]"
                    } else {
                        &value
                    }
                );
            }
        }

        let server_url = std::env::var("ATOMIC_SERVER_URL")
            .map_err(|_| AtomicError::Parse("ATOMIC_SERVER_URL not set".to_string()))?;

        let agent = match std::env::var("ATOMIC_SERVER_SECRET") {
            Ok(secret) => {
                println!("Found ATOMIC_SERVER_SECRET, length: {}", secret.len());
                // Try to create an agent from the secret
                match Agent::from_base64(&secret) {
                    Ok(agent) => {
                        println!("Agent created successfully with subject: {}", agent.subject);
                        Some(agent)
                    }
                    Err(e) => {
                        eprintln!("Failed to create agent from secret: {}", e);
                        None
                    }
                }
            }
            Err(e) => {
                eprintln!("ATOMIC_SERVER_SECRET not set: {}", e);
                None
            }
        };

        Ok(Self { server_url, agent })
    }
}

/// Represents a resource in the Atomic Server.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Resource {
    /// The subject URL of the resource
    #[serde(rename = "@id")]
    pub subject: String,
    /// The properties of the resource
    #[serde(flatten)]
    pub properties: HashMap<String, Value>,
}

/// Represents a commit to create, update, or delete a resource.
///
/// A commit is signed by an agent and sent to the server to modify a resource.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Commit {
    /// The subject URL of the resource being modified
    #[serde(rename = "https://atomicdata.dev/properties/subject")]
    pub subject: String,

    /// The resource's properties as key-value pairs
    #[serde(
        rename = "https://atomicdata.dev/properties/set",
        skip_serializing_if = "Option::is_none"
    )]
    pub set: Option<HashMap<String, Value>>,

    /// The set of property URLs that need to be removed
    #[serde(
        rename = "https://atomicdata.dev/properties/remove",
        skip_serializing_if = "Option::is_none"
    )]
    pub remove: Option<Vec<String>>,

    /// List of Properties and Arrays to be appended to them
    #[serde(
        rename = "https://atomicdata.dev/properties/push",
        skip_serializing_if = "Option::is_none"
    )]
    pub push: Option<HashMap<String, Value>>,

    /// Whether to delete the resource
    #[serde(
        rename = "https://atomicdata.dev/properties/destroy",
        skip_serializing_if = "Option::is_none"
    )]
    pub destroy: Option<bool>,

    /// The signature of the commit
    #[serde(
        rename = "https://atomicdata.dev/properties/signature",
        skip_serializing_if = "Option::is_none"
    )]
    pub signature: Option<String>,

    /// The signer of the commit
    #[serde(rename = "https://atomicdata.dev/properties/signer")]
    pub signer: String,

    /// The timestamp of the commit
    #[serde(rename = "https://atomicdata.dev/properties/createdAt")]
    pub created_at: i64,

    /// The previously applied commit to this Resource
    #[serde(
        rename = "https://atomicdata.dev/properties/previousCommit",
        skip_serializing_if = "Option::is_none"
    )]
    pub previous_commit: Option<String>,

    /// The classes of the commit
    #[serde(rename = "https://atomicdata.dev/properties/isA")]
    pub is_a: Vec<String>,

    /// The URL of the Commit
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
}

impl Commit {
    /// Creates a new commit to create or update a resource.
    ///
    /// # Arguments
    ///
    /// * `subject` - The subject URL of the resource to modify
    /// * `properties` - The resource's properties as key-value pairs
    /// * `agent` - The agent to sign the commit with
    ///
    /// # Returns
    ///
    /// A Result containing the new Commit instance or an error
    pub fn new_create_or_update(
        subject: String,
        properties: HashMap<String, Value>,
        agent: &Agent,
    ) -> Result<Self> {
        let now = crate::time_utils::unix_timestamp_secs();

        let commit = Self {
            subject,
            set: Some(properties),
            remove: None,
            push: None,
            destroy: None,
            signature: None,
            signer: agent.subject.clone(),
            created_at: now,
            previous_commit: None,
            is_a: vec!["https://atomicdata.dev/classes/Commit".to_string()],
            url: None,
        };

        Ok(commit)
    }

    /// Creates a new commit to delete a resource.
    ///
    /// # Arguments
    ///
    /// * `subject` - The subject URL of the resource to delete
    /// * `agent` - The agent to sign the commit with
    ///
    /// # Returns
    ///
    /// A Result containing the new Commit instance or an error
    pub fn new_delete(subject: String, agent: &Agent) -> Result<Self> {
        let now = crate::time_utils::unix_timestamp_secs();

        let commit = Self {
            subject,
            set: None,
            remove: None,
            push: None,
            destroy: Some(true),
            signature: None,
            signer: agent.subject.clone(),
            created_at: now,
            previous_commit: None,
            is_a: vec!["https://atomicdata.dev/classes/Commit".to_string()],
            url: None,
        };

        Ok(commit)
    }

    /// Adds a property to remove from the resource.
    ///
    /// # Arguments
    ///
    /// * `property` - The property URL to remove
    ///
    /// # Returns
    ///
    /// A mutable reference to self for method chaining
    pub fn add_remove(&mut self, property: String) -> &mut Self {
        if let Some(ref mut remove) = self.remove {
            remove.push(property);
        } else {
            self.remove = Some(vec![property]);
        }
        self
    }

    /// Adds a property to push to an array.
    ///
    /// # Arguments
    ///
    /// * `property` - The property URL to push to
    /// * `value` - The value to push
    ///
    /// # Returns
    ///
    /// A mutable reference to self for method chaining
    pub fn add_push(&mut self, property: String, value: Value) -> &mut Self {
        if let Some(ref mut push) = self.push {
            push.insert(property, value);
        } else {
            let mut push_map = HashMap::new();
            push_map.insert(property, value);
            self.push = Some(push_map);
        }
        self
    }

    /// Sets the previous commit for audit trail.
    ///
    /// # Arguments
    ///
    /// * `previous_commit` - The URL of the previous commit
    ///
    /// # Returns
    ///
    /// A mutable reference to self for method chaining
    pub fn set_previous_commit(&mut self, previous_commit: String) -> &mut Self {
        self.previous_commit = Some(previous_commit);
        self
    }

    /// Sets the URL of the commit.
    ///
    /// # Arguments
    ///
    /// * `url` - The URL of the commit
    ///
    /// # Returns
    ///
    /// A mutable reference to self for method chaining
    pub fn set_url(&mut self, url: String) -> &mut Self {
        self.url = Some(url);
        self
    }

    /// Validates the commit for basic consistency.
    ///
    /// # Returns
    ///
    /// A Result indicating whether the commit is valid or an error
    pub fn validate(&self) -> Result<()> {
        // Check for circular parent references
        if let Some(ref set) = self.set {
            if let Some(parent) = set.get("https://atomicdata.dev/properties/parent") {
                if parent.as_str() == Some(&self.subject) {
                    return Err(AtomicError::Parse("Circular parent reference".to_string()));
                }
            }
        }

        // Check timestamp is not in the future (with some tolerance)
        let now = crate::time_utils::unix_timestamp_secs();
        if self.created_at > now + 60 {
            return Err(AtomicError::Parse(
                "Commit timestamp is in the future".to_string(),
            ));
        }

        // Check timestamp is not too old (24 hours)
        if self.created_at < now - 86400 {
            return Err(AtomicError::Parse(
                "Commit timestamp is too old".to_string(),
            ));
        }

        Ok(())
    }

    /// Signs the commit using the given agent.
    ///
    /// # Arguments
    ///
    /// * `agent` - The agent to sign the commit with
    ///
    /// # Returns
    ///
    /// A Result containing the signed Commit instance or an error
    pub fn sign(mut self, agent: &Agent) -> Result<Self> {
        // Validate the commit before signing
        self.validate()?;

        // Serialize to canonical JSON
        let commit_json = serde_jcs::to_string(&self).map_err(AtomicError::Json)?;

        // Sign the commit
        let signature = agent.sign(commit_json.as_bytes())?;
        self.signature = Some(signature);

        Ok(self)
    }

    /// Converts the commit to a JSON value.
    ///
    /// # Returns
    ///
    /// A Result containing the JSON value or an error
    pub fn to_json(&self) -> Result<Value> {
        let json = serde_json::to_value(self).map_err(AtomicError::Json)?;
        Ok(json)
    }
}

/// Builder for creating commits with a fluent interface.
/// Use this for more complex commit operations.
#[derive(Debug, Clone)]
pub struct CommitBuilder {
    /// The subject URL that is to be modified by this commit
    subject: String,
    /// The set of properties that need to be added or updated
    set: HashMap<String, Value>,
    /// The set of property URLs that need to be removed
    remove: Vec<String>,
    /// The set of properties and values to be appended to arrays
    push: HashMap<String, Value>,
    /// If set to true, deletes the entire resource
    destroy: bool,
    /// The previous commit that was applied to the target resource
    previous_commit: Option<String>,
}

impl CommitBuilder {
    /// Creates a new CommitBuilder for the given subject.
    ///
    /// # Arguments
    ///
    /// * `subject` - The subject URL of the resource to modify
    ///
    /// # Returns
    ///
    /// A new CommitBuilder instance
    pub fn new(subject: String) -> Self {
        Self {
            subject,
            set: HashMap::new(),
            remove: Vec::new(),
            push: HashMap::new(),
            destroy: false,
            previous_commit: None,
        }
    }

    /// Sets a property on the resource.
    ///
    /// # Arguments
    ///
    /// * `property` - The property URL to set
    /// * `value` - The value to set
    ///
    /// # Returns
    ///
    /// Self for method chaining
    pub fn set(mut self, property: String, value: Value) -> Self {
        self.set.insert(property, value);
        self
    }

    /// Adds a property to remove from the resource.
    ///
    /// # Arguments
    ///
    /// * `property` - The property URL to remove
    ///
    /// # Returns
    ///
    /// Self for method chaining
    pub fn remove(mut self, property: String) -> Self {
        self.remove.push(property);
        self
    }

    /// Adds a value to push to an array property.
    ///
    /// # Arguments
    ///
    /// * `property` - The property URL to push to
    /// * `value` - The value to push
    ///
    /// # Returns
    ///
    /// Self for method chaining
    pub fn push(mut self, property: String, value: Value) -> Self {
        self.push.insert(property, value);
        self
    }

    /// Marks the resource for deletion.
    ///
    /// # Arguments
    ///
    /// * `destroy` - Whether to destroy the resource
    ///
    /// # Returns
    ///
    /// Self for method chaining
    pub fn destroy(mut self, destroy: bool) -> Self {
        self.destroy = destroy;
        self
    }

    /// Sets the previous commit for audit trail.
    ///
    /// # Arguments
    ///
    /// * `previous_commit` - The URL of the previous commit
    ///
    /// # Returns
    ///
    /// Self for method chaining
    pub fn set_previous_commit(mut self, previous_commit: String) -> Self {
        self.previous_commit = Some(previous_commit);
        self
    }

    /// Builds and signs the commit with the given agent.
    ///
    /// # Arguments
    ///
    /// * `agent` - The agent to sign the commit with
    ///
    /// # Returns
    ///
    /// A Result containing the signed Commit or an error
    pub fn sign(self, agent: &Agent) -> Result<Commit> {
        let now = crate::time_utils::unix_timestamp_secs();

        let commit = Commit {
            subject: self.subject,
            set: if self.set.is_empty() {
                None
            } else {
                Some(self.set)
            },
            remove: if self.remove.is_empty() {
                None
            } else {
                Some(self.remove)
            },
            push: if self.push.is_empty() {
                None
            } else {
                Some(self.push)
            },
            destroy: if self.destroy { Some(true) } else { None },
            signature: None,
            signer: agent.subject.clone(),
            created_at: now,
            previous_commit: self.previous_commit,
            is_a: vec!["https://atomicdata.dev/classes/Commit".to_string()],
            url: None,
        };

        commit.sign(agent)
    }

    /// Builds the commit without signing it.
    ///
    /// # Arguments
    ///
    /// * `agent` - The agent to use for the signer field
    ///
    /// # Returns
    ///
    /// A Result containing the unsigned Commit or an error
    pub fn build(self, agent: &Agent) -> Result<Commit> {
        let now = crate::time_utils::unix_timestamp_secs();

        let commit = Commit {
            subject: self.subject,
            set: if self.set.is_empty() {
                None
            } else {
                Some(self.set)
            },
            remove: if self.remove.is_empty() {
                None
            } else {
                Some(self.remove)
            },
            push: if self.push.is_empty() {
                None
            } else {
                Some(self.push)
            },
            destroy: if self.destroy { Some(true) } else { None },
            signature: None,
            signer: agent.subject.clone(),
            created_at: now,
            previous_commit: self.previous_commit,
            is_a: vec!["https://atomicdata.dev/classes/Commit".to_string()],
            url: None,
        };

        Ok(commit)
    }
}
