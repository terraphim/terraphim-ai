//! Summarization Agent for Agent-Based Article Summary
//!
//! This agent specializes in creating concise, informative summaries of articles
//! using the new generic LLM interface instead of OpenRouter-specific code.

use crate::{GenAiLlmClient, LlmMessage, LlmRequest, MultiAgentResult, TerraphimAgent};
use log::{debug, info};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Configuration for article summarization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SummarizationConfig {
    /// Maximum length of the summary in words
    pub max_summary_words: u32,
    /// Style of summary (brief, detailed, bullet_points)
    pub summary_style: SummaryStyle,
    /// Whether to include key quotes
    pub include_quotes: bool,
    /// Focus areas for summarization
    pub focus_areas: Vec<String>,
}

impl Default for SummarizationConfig {
    fn default() -> Self {
        Self {
            max_summary_words: 200,
            summary_style: SummaryStyle::Brief,
            include_quotes: false,
            focus_areas: vec![],
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SummaryStyle {
    Brief,
    Detailed,
    BulletPoints,
    Executive,
}

/// Specialized agent for document summarization
pub struct SummarizationAgent {
    /// Core Terraphim agent with role-based configuration
    terraphim_agent: TerraphimAgent,
    /// LLM client for generating summaries
    llm_client: Arc<GenAiLlmClient>,
    /// Summarization configuration
    config: SummarizationConfig,
}

impl SummarizationAgent {
    /// Create a new SummarizationAgent
    pub async fn new(
        terraphim_agent: TerraphimAgent,
        config: Option<SummarizationConfig>,
    ) -> MultiAgentResult<Self> {
        // Extract LLM configuration from the agent's role
        let role = &terraphim_agent.role_config;

        // Create LLM client based on role configuration
        let llm_client = if let Some(provider) = role.extra.get("llm_provider") {
            let provider_str = provider.as_str().unwrap_or("ollama");
            let model = role
                .extra
                .get("llm_model")
                .and_then(|m| m.as_str())
                .map(|s| s.to_string());

            Arc::new(GenAiLlmClient::from_config(provider_str, model)?)
        } else {
            // Default to Ollama with gemma3:270m
            Arc::new(GenAiLlmClient::new_ollama(Some("gemma3:270m".to_string()))?)
        };

        info!(
            "Created SummarizationAgent with provider: {}",
            llm_client.provider()
        );

        Ok(Self {
            terraphim_agent,
            llm_client,
            config: config.unwrap_or_default(),
        })
    }

    /// Generate a summary for the given text
    pub async fn summarize(&self, content: &str) -> MultiAgentResult<String> {
        info!(
            "Generating summary for content of {} characters",
            content.len()
        );

        let system_prompt = self.create_system_prompt();
        let user_prompt = self.create_user_prompt(content);

        let messages = vec![
            LlmMessage::system(system_prompt),
            LlmMessage::user(user_prompt),
        ];

        // Use context window from role config, fallback to reasonable default for summaries
        // Get context window from role extra config or use default
        let context_window = self
            .terraphim_agent
            .role_config
            .extra
            .get("llm_context_window")
            .and_then(|v| v.as_u64())
            .map(|v| v as usize);
        
        let max_tokens = context_window
            .map(|cw| ((cw / 4).min(1000)) as u64) // Use 1/4 of context window, max 1000 for summaries
            .unwrap_or(500); // Default fallback

        let request = LlmRequest::new(messages)
            .with_temperature(0.3) // Lower temperature for more consistent summaries
            .with_max_tokens(max_tokens);

        debug!("Sending summarization request to LLM");
        let response = self.llm_client.generate(request).await?;

        info!("Generated summary of {} characters", response.content.len());
        Ok(response.content.trim().to_string())
    }

    /// Summarize multiple documents and create a consolidated summary
    pub async fn summarize_multiple(&self, documents: &[(&str, &str)]) -> MultiAgentResult<String> {
        info!(
            "Generating consolidated summary for {} documents",
            documents.len()
        );

        // Generate individual summaries first
        let mut individual_summaries = Vec::new();
        for (title, content) in documents {
            let summary = self.summarize(content).await?;
            individual_summaries.push(format!("**{}**: {}", title, summary));
        }

        // Create consolidated summary
        let consolidated_content = individual_summaries.join("\n\n");
        let system_prompt = "You are an expert at creating consolidated summaries. Take multiple document summaries and create a cohesive overview that identifies common themes, key insights, and important differences.";

        let user_prompt = format!(
            "Create a consolidated summary from these individual document summaries:\n\n{}\n\nProvide a cohesive overview that highlights:\n1. Common themes across documents\n2. Key insights and findings\n3. Important differences or contrasts\n4. Overall conclusions\n\nKeep the consolidated summary to approximately {} words.",
            consolidated_content,
            self.config.max_summary_words
        );

        let messages = vec![
            LlmMessage::system(system_prompt.to_string()),
            LlmMessage::user(user_prompt),
        ];

        let request = LlmRequest::new(messages)
            .with_temperature(0.3)
            .with_max_tokens(600);

        let response = self.llm_client.generate(request).await?;
        Ok(response.content.trim().to_string())
    }

    /// Create system prompt based on configuration
    fn create_system_prompt(&self) -> String {
        let style_instruction = match self.config.summary_style {
            SummaryStyle::Brief => "Create a brief, concise summary that captures the essential points.",
            SummaryStyle::Detailed => "Create a detailed summary that covers all major points and supporting details.",
            SummaryStyle::BulletPoints => "Create a summary in bullet point format, organizing information clearly.",
            SummaryStyle::Executive => "Create an executive summary suitable for business stakeholders, focusing on key insights and actionable information.",
        };

        let quote_instruction = if self.config.include_quotes {
            " Include 1-2 key quotes that best represent the main ideas."
        } else {
            ""
        };

        let focus_instruction = if !self.config.focus_areas.is_empty() {
            format!(
                " Pay special attention to these areas: {}.",
                self.config.focus_areas.join(", ")
            )
        } else {
            String::new()
        };

        format!(
            "You are an expert summarization specialist. {} The summary should be approximately {} words.{}{}",
            style_instruction,
            self.config.max_summary_words,
            quote_instruction,
            focus_instruction
        )
    }

    /// Create user prompt with the content to summarize
    fn create_user_prompt(&self, content: &str) -> String {
        format!(
            "Please summarize the following content:\n\n{}\n\nProvide a clear, informative summary that captures the key points and main insights.",
            content
        )
    }

    /// Update summarization configuration
    pub fn update_config(&mut self, config: SummarizationConfig) {
        self.config = config;
        info!("Updated summarization configuration");
    }

    /// Get current configuration
    pub fn get_config(&self) -> &SummarizationConfig {
        &self.config
    }

    /// Access the underlying Terraphim agent
    pub fn terraphim_agent(&self) -> &TerraphimAgent {
        &self.terraphim_agent
    }

    /// Access the LLM client
    pub fn llm_client(&self) -> &GenAiLlmClient {
        &self.llm_client
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::create_test_agent;

    #[tokio::test]
    async fn test_summarization_agent_creation() {
        let agent = create_test_agent().await.unwrap();
        let summarization_agent = SummarizationAgent::new(agent, None).await.unwrap();

        assert_eq!(summarization_agent.config.max_summary_words, 200);
        assert_eq!(summarization_agent.llm_client.provider(), "ollama");
    }

    #[tokio::test]
    async fn test_system_prompt_generation() {
        let agent = create_test_agent().await.unwrap();
        let mut config = SummarizationConfig::default();
        config.include_quotes = true;
        config.focus_areas = vec!["technology".to_string(), "innovation".to_string()];

        let summarization_agent = SummarizationAgent::new(agent, Some(config)).await.unwrap();
        let prompt = summarization_agent.create_system_prompt();

        assert!(prompt.contains("200 words"));
        assert!(prompt.contains("key quotes"));
        assert!(prompt.contains("technology, innovation"));
    }
}
