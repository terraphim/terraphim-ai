//! LLM Types and Request/Response Structures
//!
//! This module contains the common types used for LLM communication,
//! independent of the specific LLM client implementation.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Message roles for LLM communication
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum MessageRole {
    System,
    User,
    Assistant,
    Tool,
}

/// An individual message in a conversation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmMessage {
    pub role: MessageRole,
    pub content: String,
}

impl LlmMessage {
    pub fn system(content: String) -> Self {
        Self {
            role: MessageRole::System,
            content,
        }
    }

    pub fn user(content: String) -> Self {
        Self {
            role: MessageRole::User,
            content,
        }
    }

    pub fn assistant(content: String) -> Self {
        Self {
            role: MessageRole::Assistant,
            content,
        }
    }

    pub fn tool(content: String) -> Self {
        Self {
            role: MessageRole::Tool,
            content,
        }
    }
}

/// Request to an LLM
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmRequest {
    pub messages: Vec<LlmMessage>,
    pub temperature: Option<f32>,
    pub max_tokens: Option<u64>,
    pub metadata: std::collections::HashMap<String, String>,
}

impl LlmRequest {
    pub fn new(messages: Vec<LlmMessage>) -> Self {
        Self {
            messages,
            temperature: None,
            max_tokens: None,
            metadata: std::collections::HashMap::new(),
        }
    }

    pub fn with_temperature(mut self, temperature: f32) -> Self {
        self.temperature = Some(temperature);
        self
    }

    pub fn with_max_tokens(mut self, max_tokens: u64) -> Self {
        self.max_tokens = Some(max_tokens);
        self
    }

    pub fn with_metadata(mut self, key: String, value: String) -> Self {
        self.metadata.insert(key, value);
        self
    }
}

/// Token usage statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenUsage {
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub total_tokens: u64,
}

impl TokenUsage {
    pub fn new(input_tokens: u64, output_tokens: u64) -> Self {
        Self {
            input_tokens,
            output_tokens,
            total_tokens: input_tokens + output_tokens,
        }
    }

    pub fn zero() -> Self {
        Self {
            input_tokens: 0,
            output_tokens: 0,
            total_tokens: 0,
        }
    }
}

/// Response from an LLM
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmResponse {
    pub content: String,
    pub model: String,
    pub usage: TokenUsage,
    pub request_id: Uuid,
    pub timestamp: DateTime<Utc>,
    pub duration_ms: u64,
    pub finish_reason: String,
}

impl LlmResponse {
    pub fn new(content: String) -> Self {
        Self {
            content,
            model: "unknown".to_string(),
            usage: TokenUsage::zero(),
            request_id: Uuid::new_v4(),
            timestamp: Utc::now(),
            duration_ms: 0,
            finish_reason: "completed".to_string(),
        }
    }
}