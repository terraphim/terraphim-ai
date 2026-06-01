//! Summary / AI capability for `TerraphimService` (LLM-backed description
//! enhancement and document summarisation). Split from lib.rs as part of the
//! Gitea #1910 god-file decomposition; behaviour unchanged. Methods remain on
//! `TerraphimService`, so the public API is identical.

use terraphim_config::Role;
use terraphim_types::Document;

use super::{Result, ServiceError, TerraphimService};

impl TerraphimService {
    /// Enhance document descriptions with AI-generated summaries using OpenRouter
    ///
    /// This method uses the OpenRouter service to generate intelligent summaries
    /// of document content, replacing basic text excerpts with AI-powered descriptions.
    #[allow(dead_code)] // Used in 7+ places but compiler can't see due to async/feature boundaries
    async fn enhance_descriptions_with_ai(
        &self,
        mut documents: Vec<Document>,
        role: &Role,
    ) -> Result<Vec<Document>> {
        use crate::llm::{SummarizeOptions, build_llm_from_role};

        eprintln!("🤖 Attempting to build LLM client for role: {}", role.name);
        let llm = match build_llm_from_role(role) {
            Some(client) => {
                eprintln!("✅ LLM client successfully created: {}", client.name());
                client
            }
            None => {
                eprintln!("❌ No LLM client available for role: {}", role.name);
                return Ok(documents);
            }
        };

        log::info!(
            "Enhancing {} document descriptions with LLM provider: {}",
            documents.len(),
            llm.name()
        );

        let mut enhanced_count = 0;
        let mut error_count = 0;

        for document in &mut documents {
            if self.should_generate_ai_summary(document) {
                let summary_length = 250;
                match llm
                    .summarize(
                        &document.body,
                        SummarizeOptions {
                            max_length: summary_length,
                        },
                    )
                    .await
                {
                    Ok(ai_summary) => {
                        log::debug!(
                            "Generated AI summary for '{}': {} characters",
                            document.title,
                            ai_summary.len()
                        );
                        document.description = Some(ai_summary);
                        enhanced_count += 1;
                    }
                    Err(e) => {
                        log::warn!(
                            "Failed to generate AI summary for '{}': {}",
                            document.title,
                            e
                        );
                        error_count += 1;
                    }
                }
            }
        }

        log::info!(
            "LLM enhancement complete: {} enhanced, {} errors, {} skipped",
            enhanced_count,
            error_count,
            documents.len() - enhanced_count - error_count
        );

        Ok(documents)
    }

    /// Determine if a document should receive an AI-generated summary
    ///
    /// This helper method checks various criteria to decide whether a document
    /// would benefit from AI summarization.
    #[allow(dead_code)] // Used by enhance_descriptions_with_ai, compiler can't see due to async boundaries
    fn should_generate_ai_summary(&self, document: &Document) -> bool {
        // Don't enhance if the document body is too short to summarize meaningfully
        if document.body.trim().len() < 200 {
            return false;
        }

        // Don't enhance if we already have a high-quality description
        if let Some(ref description) = document.description {
            // If the description is substantial and doesn't look like a simple excerpt, keep it
            if description.len() > 100 && !description.ends_with("...") {
                return false;
            }
        }

        // Don't enhance very large documents (cost control)
        if document.body.len() > 8000 {
            return false;
        }

        // Good candidates for AI summarization
        true
    }

    /// Get the role for the given search query

    /// Generate a summary for a document using OpenRouter
    ///
    /// This method takes a document and generates an AI-powered summary using the OpenRouter service.
    /// The summary is generated based on the document's content and can be customized with different
    /// models and length constraints.
    ///
    /// # Arguments
    ///
    /// * `document` - The document to summarize
    /// * `api_key` - The OpenRouter API key
    /// * `model` - The model to use for summarization (e.g., "openai/gpt-3.5-turbo")
    /// * `max_length` - Maximum length of the summary in characters
    ///
    /// # Returns
    ///
    /// Returns a `Result<String>` containing the generated summary or an error if summarization fails.
    #[cfg(feature = "openrouter")]
    pub async fn generate_document_summary(
        &self,
        document: &Document,
        api_key: &str,
        model: &str,
        max_length: usize,
    ) -> Result<String> {
        use crate::openrouter::OpenRouterService;

        log::debug!(
            "Generating summary for document '{}' using model '{}'",
            document.id,
            model
        );

        // Create the OpenRouter service
        let openrouter_service =
            OpenRouterService::new(api_key, model).map_err(ServiceError::OpenRouter)?;

        // Use the document body for summarization
        let content = &document.body;

        if content.trim().is_empty() {
            return Err(ServiceError::Config(
                "Document body is empty, cannot generate summary".to_string(),
            ));
        }

        // Generate the summary
        let summary = openrouter_service
            .generate_summary(content, max_length)
            .await
            .map_err(ServiceError::OpenRouter)?;

        log::info!(
            "Generated {}-character summary for document '{}' using model '{}'",
            summary.len(),
            document.id,
            model
        );

        Ok(summary)
    }

    /// Generate a summary for a document using OpenRouter (stub when feature is disabled)
    #[cfg(not(feature = "openrouter"))]
    pub async fn generate_document_summary(
        &self,
        _document: &Document,
        _api_key: &str,
        _model: &str,
        _max_length: usize,
    ) -> Result<String> {
        Err(ServiceError::Config(
            "OpenRouter feature not enabled during compilation".to_string(),
        ))
    }
}
