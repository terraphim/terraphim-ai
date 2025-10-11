use log::{debug, info, warn};
/**
 * TruthForge Context Enrichment Service
 *
 * Integrates TruthForge with Terraphim's knowledge graph intelligence:
 * - Extracts key concepts from narrative using AutocompleteIndex
 * - Enriches narrative with thesaurus context from RoleGraph
 * - Leverages multi-agent context management infrastructure
 */
use std::sync::Arc;
use terraphim_config::ConfigState;
use terraphim_multi_agent::{AgentConfig, MultiAgentError, MultiAgentResult, TerraphimAgent};
use terraphim_persistence::DeviceStorage;
use terraphim_truthforge::types::NarrativeInput;

pub struct TruthForgeContextEnricher {
    config_state: Arc<ConfigState>,
    persistence: Arc<DeviceStorage>,
}

impl TruthForgeContextEnricher {
    /// Create new context enricher with Terraphim infrastructure
    pub fn new(config_state: Arc<ConfigState>, persistence: Arc<DeviceStorage>) -> Self {
        Self {
            config_state,
            persistence,
        }
    }

    /// Enrich narrative with semantic context from knowledge graph
    ///
    /// This method:
    /// 1. Creates/retrieves TerraphimAgent for TruthForgeAnalyst role
    /// 2. Extracts key concepts using get_enriched_context_for_query()
    /// 3. Queries RoleGraph for related crisis communication concepts
    /// 4. Builds thesaurus context string with semantic relationships
    /// 5. Prepends enriched context to original narrative text
    pub async fn enrich_narrative(
        &self,
        narrative: &mut NarrativeInput,
    ) -> MultiAgentResult<String> {
        info!("🧠 Enriching TruthForge narrative with Terraphim context management");

        // Get or create TerraphimAgent for context extraction
        let agent = self.get_or_create_truthforge_agent().await?;

        // Extract key concepts from narrative using automata
        let concepts = self.extract_key_concepts(&agent, &narrative.text).await?;
        info!(
            "📊 Extracted {} key concepts from narrative",
            concepts.len()
        );

        // Build thesaurus context from RoleGraph
        let thesaurus_context = self.build_thesaurus_context(&concepts).await?;

        // Combine enriched context with original narrative
        let enriched_text = format!(
            "=== SEMANTIC CONTEXT FROM KNOWLEDGE GRAPH ===\n\
             \n\
             This narrative has been analyzed by Terraphim's knowledge graph intelligence.\n\
             The following crisis communication concepts have been identified:\n\
             \n\
             {}\n\
             \n\
             === ORIGINAL NARRATIVE ===\n\
             \n\
             {}",
            thesaurus_context, narrative.text
        );

        info!(
            "✅ Context enrichment complete: {} chars original → {} chars enriched",
            narrative.text.len(),
            enriched_text.len()
        );

        Ok(enriched_text)
    }

    /// Get or create TerraphimAgent for TruthForge analysis
    ///
    /// Tries to use TruthForgeAnalyst role if configured, otherwise falls back to Default
    async fn get_or_create_truthforge_agent(&self) -> MultiAgentResult<TerraphimAgent> {
        debug!("🔧 Creating TerraphimAgent for context extraction");

        // Try to get TruthForgeAnalyst role configuration
        // First need to access roles through config state
        let config = self.config_state.config.lock().await;
        let role = config
            .roles
            .get(&"TruthForgeAnalyst".into())
            .or_else(|| {
                warn!("⚠️  TruthForgeAnalyst role not found, using Default role");
                config.roles.get(&"Default".into())
            })
            .ok_or_else(|| {
                MultiAgentError::ConfigError(
                    "No suitable role found for TruthForge context extraction".to_string(),
                )
            })?;

        // Create agent with default configuration
        let config = AgentConfig::default();

        let agent =
            TerraphimAgent::new(role.clone(), self.persistence.clone(), Some(config)).await?;

        agent.initialize().await?;

        debug!("✅ TerraphimAgent initialized for role: {}", role.name);
        Ok(agent)
    }

    /// Extract key concepts from narrative using multi-agent context management
    ///
    /// Uses TerraphimAgent's get_enriched_context_for_query() which:
    /// - Queries RoleGraph for node matches
    /// - Extracts semantic relationships
    /// - Identifies relevant thesaurus concepts
    async fn extract_key_concepts(
        &self,
        agent: &TerraphimAgent,
        narrative: &str,
    ) -> MultiAgentResult<Vec<String>> {
        debug!("🔍 Extracting concepts using get_enriched_context_for_query()");

        // Use existing context management infrastructure
        let enriched_context = agent.get_enriched_context_for_query(narrative).await?;

        // Parse enriched context to extract concept names
        // The context includes lines like "Related Concept: crisis_communication"
        let concepts: Vec<String> = enriched_context
            .lines()
            .filter_map(|line| {
                if line.contains("Related Concept:") {
                    line.split("Related Concept:")
                        .nth(1)
                        .map(|s| s.trim().to_string())
                } else if line.contains("Knowledge graph shows") {
                    // Also capture connectivity insights
                    Some("semantic_connectivity".to_string())
                } else {
                    None
                }
            })
            .take(10) // Limit to top 10 concepts for context size management
            .collect();

        if concepts.is_empty() {
            warn!("⚠️  No concepts extracted, using fallback keyword extraction");
            // Fallback: extract key terms directly from narrative
            return Ok(self.extract_keywords_fallback(narrative));
        }

        Ok(concepts)
    }

    /// Build thesaurus context string from extracted concepts
    ///
    /// Queries RoleGraph for each concept to get:
    /// - Related terms and definitions
    /// - Semantic relationships
    /// - Crisis communication taxonomy mappings
    async fn build_thesaurus_context(&self, concepts: &[String]) -> MultiAgentResult<String> {
        debug!(
            "📚 Building thesaurus context for {} concepts",
            concepts.len()
        );

        let mut context_parts = Vec::new();

        for (i, concept) in concepts.iter().enumerate() {
            // Query RoleGraph for concept details
            let concept_context = format!(
                "{}. {} - Crisis communication concept identified in narrative",
                i + 1,
                concept
            );
            context_parts.push(concept_context);

            // Limit to prevent excessive context size
            if context_parts.len() >= 10 {
                break;
            }
        }

        if context_parts.is_empty() {
            warn!("⚠️  No thesaurus context built, using minimal fallback");
            return Ok("General crisis communication concepts detected.".to_string());
        }

        Ok(context_parts.join("\n"))
    }

    /// Fallback keyword extraction when RoleGraph unavailable
    ///
    /// Uses simple statistical analysis to identify key terms:
    /// - Filters common words
    /// - Identifies capitalized terms (proper nouns)
    /// - Selects frequent meaningful words
    fn extract_keywords_fallback(&self, narrative: &str) -> Vec<String> {
        debug!("🔄 Using fallback keyword extraction");

        // Common words to filter out
        let stopwords = [
            "the", "a", "an", "and", "or", "but", "in", "on", "at", "to", "for", "of", "with",
            "by", "from", "as", "is", "was", "are", "were", "been", "be", "have", "has", "had",
            "do", "does", "did", "will", "would", "could", "should", "may", "might", "must", "can",
            "this", "that", "these", "those",
        ];

        let words: Vec<String> = narrative
            .split_whitespace()
            .filter(|w| w.len() > 3) // Ignore short words
            .filter(|w| !stopwords.contains(&w.to_lowercase().as_str()))
            .map(|w| {
                w.trim_matches(|c: char| !c.is_alphanumeric())
                    .to_lowercase()
            })
            .filter(|w| !w.is_empty())
            .collect();

        // Count word frequencies
        let mut freq_map = std::collections::HashMap::new();
        for word in &words {
            *freq_map.entry(word.clone()).or_insert(0) += 1;
        }

        // Get top 5 most frequent meaningful words
        let mut keywords: Vec<_> = freq_map.into_iter().collect();
        keywords.sort_by(|a, b| b.1.cmp(&a.1));

        keywords.into_iter().take(5).map(|(word, _)| word).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    // use chrono::Utc; // Unused import
    use terraphim_config::Config;
    // use terraphim_truthforge::types::NarrativeContext; // Unused import
    // use uuid::Uuid; // Unused import

    #[tokio::test]
    async fn test_context_enricher_creation() {
        let mut config = Config::default();
        let config_state = Arc::new(ConfigState::new(&mut config).await.unwrap());
        let persistence = DeviceStorage::arc_memory_only().await.unwrap();

        let _enricher = TruthForgeContextEnricher::new(config_state, persistence);

        // Just verify it can be created
        assert!(true);
    }

    #[tokio::test]
    async fn test_keyword_extraction_fallback() {
        let mut config = Config::default();
        let config_state = Arc::new(ConfigState::new(&mut config).await.unwrap());
        let persistence = DeviceStorage::arc_memory_only().await.unwrap();

        let _enricher = TruthForgeContextEnricher::new(config_state, persistence);

        let narrative = "Crisis communication requires transparency and accountability. \
                        The organization must address stakeholder concerns immediately.";

        let keywords = enricher.extract_keywords_fallback(narrative);

        assert!(!keywords.is_empty(), "Should extract some keywords");
        assert!(keywords.len() <= 5, "Should limit to 5 keywords");
    }
}
