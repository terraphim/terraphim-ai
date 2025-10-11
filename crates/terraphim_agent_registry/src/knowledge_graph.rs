//! Knowledge graph integration for agent registry
//!
//! Integrates with Terraphim's existing knowledge graph infrastructure to provide
//! intelligent agent discovery and capability matching using automata and role graphs.

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use serde::{Deserialize, Serialize};

use terraphim_automata::extract_paragraphs_from_automata;
use terraphim_rolegraph::RoleGraph;

use crate::{AgentMetadata, RegistryError, RegistryResult};

/// Knowledge graph-based agent discovery and matching
#[allow(dead_code)]
pub struct KnowledgeGraphIntegration {
    /// Role graph for role-based agent specialization
    role_graph: Arc<RoleGraph>,
    /// Automata for knowledge extraction and context analysis
    automata_config: AutomataConfig,
    /// Cached knowledge graph queries for performance
    query_cache: Arc<tokio::sync::RwLock<HashMap<String, QueryResult>>>,
    /// Semantic similarity thresholds
    similarity_thresholds: SimilarityThresholds,
}

/// Configuration for automata-based knowledge extraction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutomataConfig {
    /// Minimum confidence threshold for extraction
    pub min_confidence: f64,
    /// Maximum number of paragraphs to extract
    pub max_paragraphs: usize,
    /// Context window size for extraction
    pub context_window: usize,
    /// Language models to use for extraction
    pub language_models: Vec<String>,
}

impl Default for AutomataConfig {
    fn default() -> Self {
        Self {
            min_confidence: 0.7,
            max_paragraphs: 10,
            context_window: 512,
            language_models: vec!["default".to_string()],
        }
    }
}

/// Similarity thresholds for different types of matching
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimilarityThresholds {
    /// Role similarity threshold
    pub role_similarity: f64,
    /// Capability similarity threshold
    pub capability_similarity: f64,
    /// Domain similarity threshold
    pub domain_similarity: f64,
    /// Concept similarity threshold
    pub concept_similarity: f64,
}

impl Default for SimilarityThresholds {
    fn default() -> Self {
        Self {
            role_similarity: 0.8,
            capability_similarity: 0.75,
            domain_similarity: 0.7,
            concept_similarity: 0.65,
        }
    }
}

/// Cached query result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryResult {
    /// Query hash for cache key
    pub query_hash: String,
    /// Extracted concepts and relationships
    pub concepts: Vec<String>,
    /// Connectivity analysis results
    pub connectivity: ConnectivityResult,
    /// Timestamp when cached
    pub cached_at: chrono::DateTime<chrono::Utc>,
    /// Cache expiry time
    pub expires_at: chrono::DateTime<chrono::Utc>,
}

/// Result of connectivity analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectivityResult {
    /// Whether all terms are connected
    pub all_connected: bool,
    /// Connection paths found
    pub paths: Vec<Vec<String>>,
    /// Disconnected terms
    pub disconnected: Vec<String>,
    /// Connection strength score
    pub strength_score: f64,
}

/// Agent discovery query
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentDiscoveryQuery {
    /// Required roles
    pub required_roles: Vec<String>,
    /// Required capabilities
    pub required_capabilities: Vec<String>,
    /// Required knowledge domains
    pub required_domains: Vec<String>,
    /// Task description for context extraction
    pub task_description: Option<String>,
    /// Minimum success rate
    pub min_success_rate: Option<f64>,
    /// Maximum resource usage
    pub max_resource_usage: Option<crate::ResourceUsage>,
    /// Preferred agent tags
    pub preferred_tags: Vec<String>,
}

/// Agent discovery result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentDiscoveryResult {
    /// Matching agents with scores
    pub matches: Vec<AgentMatch>,
    /// Query analysis results
    pub query_analysis: QueryAnalysis,
    /// Suggestions for improving the query
    pub suggestions: Vec<String>,
}

/// Individual agent match result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentMatch {
    /// Agent metadata
    pub agent: AgentMetadata,
    /// Overall match score (0.0 to 1.0)
    pub match_score: f64,
    /// Detailed scoring breakdown
    pub score_breakdown: ScoreBreakdown,
    /// Explanation of the match
    pub explanation: String,
}

/// Detailed scoring breakdown
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScoreBreakdown {
    /// Role compatibility score
    pub role_score: f64,
    /// Capability match score
    pub capability_score: f64,
    /// Domain expertise score
    pub domain_score: f64,
    /// Performance score
    pub performance_score: f64,
    /// Availability score
    pub availability_score: f64,
}

/// Query analysis results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryAnalysis {
    /// Extracted concepts from task description
    pub extracted_concepts: Vec<String>,
    /// Identified knowledge domains
    pub identified_domains: Vec<String>,
    /// Suggested roles based on analysis
    pub suggested_roles: Vec<String>,
    /// Connectivity analysis of requirements
    pub connectivity_analysis: ConnectivityResult,
}

impl KnowledgeGraphIntegration {
    /// Create new knowledge graph integration
    pub fn new(
        role_graph: Arc<RoleGraph>,
        automata_config: AutomataConfig,
        similarity_thresholds: SimilarityThresholds,
    ) -> Self {
        Self {
            role_graph,
            automata_config,
            query_cache: Arc::new(tokio::sync::RwLock::new(HashMap::new())),
            similarity_thresholds,
        }
    }

    /// Discover agents based on requirements using knowledge graph analysis
    pub async fn discover_agents(
        &self,
        query: AgentDiscoveryQuery,
        available_agents: &[AgentMetadata],
    ) -> RegistryResult<AgentDiscoveryResult> {
        // Analyze the query using knowledge graph
        let query_analysis = self.analyze_query(&query).await?;

        // Score and rank agents
        let mut matches = Vec::new();
        for agent in available_agents {
            if let Ok(agent_match) = self.score_agent_match(agent, &query, &query_analysis).await {
                matches.push(agent_match);
            }
        }

        // Sort by match score (highest first)
        matches.sort_by(|a, b| {
            b.match_score
                .partial_cmp(&a.match_score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        // Generate suggestions for improving the query
        let suggestions = self
            .generate_query_suggestions(&query, &query_analysis, &matches)
            .await;

        Ok(AgentDiscoveryResult {
            matches,
            query_analysis,
            suggestions,
        })
    }

    /// Analyze a discovery query using knowledge graph
    async fn analyze_query(&self, query: &AgentDiscoveryQuery) -> RegistryResult<QueryAnalysis> {
        let mut extracted_concepts = Vec::new();
        let mut identified_domains = Vec::new();
        let mut suggested_roles = Vec::new();

        // Extract concepts from task description if provided
        if let Some(task_description) = &query.task_description {
            extracted_concepts = self.extract_concepts_from_text(task_description).await?;
            identified_domains = self
                .identify_domains_from_concepts(&extracted_concepts)
                .await?;
        }

        // Analyze required roles and suggest additional ones
        for role_id in &query.required_roles {
            if let Some(related_roles) = self.find_related_roles(role_id).await? {
                suggested_roles.extend(related_roles);
            }
        }

        // Analyze connectivity of all requirements
        let all_terms: Vec<String> = query
            .required_roles
            .iter()
            .chain(query.required_capabilities.iter())
            .chain(query.required_domains.iter())
            .chain(extracted_concepts.iter())
            .cloned()
            .collect();

        let connectivity_analysis = self.analyze_term_connectivity(&all_terms).await?;

        Ok(QueryAnalysis {
            extracted_concepts,
            identified_domains,
            suggested_roles,
            connectivity_analysis,
        })
    }

    /// Score how well an agent matches a query
    async fn score_agent_match(
        &self,
        agent: &AgentMetadata,
        query: &AgentDiscoveryQuery,
        query_analysis: &QueryAnalysis,
    ) -> RegistryResult<AgentMatch> {
        // Calculate individual scores
        let role_score = self
            .calculate_role_score(agent, &query.required_roles)
            .await?;
        let capability_score = self
            .calculate_capability_score(agent, &query.required_capabilities)
            .await?;
        let domain_score = self
            .calculate_domain_score(
                agent,
                &query.required_domains,
                &query_analysis.identified_domains,
            )
            .await?;
        let performance_score = self.calculate_performance_score(agent, query).await?;
        let availability_score = self.calculate_availability_score(agent).await?;

        // Weighted overall score
        let match_score = (role_score * 0.25
            + capability_score * 0.25
            + domain_score * 0.20
            + performance_score * 0.20
            + availability_score * 0.10)
            .min(1.0)
            .max(0.0);

        let score_breakdown = ScoreBreakdown {
            role_score,
            capability_score,
            domain_score,
            performance_score,
            availability_score,
        };

        let explanation = self.generate_match_explanation(agent, &score_breakdown, query_analysis);

        Ok(AgentMatch {
            agent: agent.clone(),
            match_score,
            score_breakdown,
            explanation,
        })
    }

    /// Extract concepts from text using automata
    async fn extract_concepts_from_text(&self, text: &str) -> RegistryResult<Vec<String>> {
        // Create an empty thesaurus for basic text processing
        let thesaurus = terraphim_types::Thesaurus::new("agent_registry".to_string());

        // Use the existing extract_paragraphs_from_automata function
        let paragraphs = extract_paragraphs_from_automata(
            text, thesaurus, true, // include_term
        )
        .map_err(|e| {
            RegistryError::KnowledgeGraphError(format!("Failed to extract paragraphs: {}", e))
        })?;

        // Extract concepts from paragraphs
        let mut concepts = HashSet::new();
        for (_matched, paragraph_text) in paragraphs {
            // Simple concept extraction - in practice, this would use more sophisticated NLP
            let words: Vec<&str> = paragraph_text.split_whitespace().collect();
            for word in words {
                if word.len() > 3 && !word.chars().all(|c| c.is_ascii_punctuation()) {
                    concepts.insert(word.to_lowercase());
                }
            }
        }

        Ok(concepts.into_iter().collect())
    }

    /// Identify knowledge domains from concepts
    async fn identify_domains_from_concepts(
        &self,
        concepts: &[String],
    ) -> RegistryResult<Vec<String>> {
        // This would typically use domain classification models
        // For now, we'll use simple keyword matching
        let mut domains = HashSet::new();

        for concept in concepts {
            let concept_lower = concept.to_lowercase();

            // Simple domain classification based on keywords
            if concept_lower.contains("plan") || concept_lower.contains("strategy") {
                domains.insert("planning".to_string());
            }
            if concept_lower.contains("data") || concept_lower.contains("analysis") {
                domains.insert("data_analysis".to_string());
            }
            if concept_lower.contains("execute") || concept_lower.contains("implement") {
                domains.insert("execution".to_string());
            }
            if concept_lower.contains("coordinate") || concept_lower.contains("manage") {
                domains.insert("coordination".to_string());
            }
        }

        Ok(domains.into_iter().collect())
    }

    /// Find related roles using role graph
    async fn find_related_roles(&self, _role_id: &str) -> RegistryResult<Option<Vec<String>>> {
        // TODO: Implement role graph querying when the appropriate methods are available
        // For now, return empty to maintain functionality
        Ok(Some(Vec::new()))
    }

    /// Analyze connectivity of terms using knowledge graph
    async fn analyze_term_connectivity(
        &self,
        terms: &[String],
    ) -> RegistryResult<ConnectivityResult> {
        if terms.is_empty() {
            return Ok(ConnectivityResult {
                all_connected: true,
                paths: Vec::new(),
                disconnected: Vec::new(),
                strength_score: 1.0,
            });
        }

        // Check cache first
        let cache_key = format!("connectivity_{}", terms.join("_"));
        {
            let cache = self.query_cache.read().await;
            if let Some(cached_result) = cache.get(&cache_key) {
                if cached_result.expires_at > chrono::Utc::now() {
                    return Ok(cached_result.connectivity.clone());
                }
            }
        }

        // Use the existing is_all_terms_connected_by_path method on role_graph
        let text = terms.join(" ");
        let all_connected = self.role_graph.is_all_terms_connected_by_path(&text);

        // For now, we'll create a simplified connectivity result
        // In practice, this would involve more sophisticated graph analysis
        let connectivity_result = ConnectivityResult {
            all_connected,
            paths: if all_connected {
                vec![terms.to_vec()]
            } else {
                Vec::new()
            },
            disconnected: if all_connected {
                Vec::new()
            } else {
                terms.to_vec()
            },
            strength_score: if all_connected { 1.0 } else { 0.0 },
        };

        // Cache the result
        {
            let mut cache = self.query_cache.write().await;
            cache.insert(
                cache_key.clone(),
                QueryResult {
                    query_hash: cache_key,
                    concepts: terms.to_vec(),
                    connectivity: connectivity_result.clone(),
                    cached_at: chrono::Utc::now(),
                    expires_at: chrono::Utc::now() + chrono::Duration::hours(1),
                },
            );
        }

        Ok(connectivity_result)
    }

    /// Calculate role compatibility score
    async fn calculate_role_score(
        &self,
        agent: &AgentMetadata,
        required_roles: &[String],
    ) -> RegistryResult<f64> {
        if required_roles.is_empty() {
            return Ok(1.0);
        }

        let mut total_score: f64 = 0.0;
        let mut role_count = 0;

        for required_role in required_roles {
            let mut best_score: f64 = 0.0;

            // Check exact match with primary role
            if agent.primary_role.role_id == *required_role {
                best_score = 1.0;
            } else {
                // Check secondary roles
                for secondary_role in &agent.secondary_roles {
                    if secondary_role.role_id == *required_role {
                        best_score = best_score.max(0.9);
                    }
                }

                // Check role hierarchy compatibility
                if let Some(related_roles) = self.find_related_roles(required_role).await? {
                    if related_roles.contains(&agent.primary_role.role_id) {
                        best_score = best_score.max(0.7);
                    }

                    for secondary_role in &agent.secondary_roles {
                        if related_roles.contains(&secondary_role.role_id) {
                            best_score = best_score.max(0.6);
                        }
                    }
                }
            }

            total_score += best_score;
            role_count += 1;
        }

        Ok(if role_count > 0 {
            total_score / role_count as f64
        } else {
            1.0
        })
    }

    /// Calculate capability match score
    async fn calculate_capability_score(
        &self,
        agent: &AgentMetadata,
        required_capabilities: &[String],
    ) -> RegistryResult<f64> {
        if required_capabilities.is_empty() {
            return Ok(1.0);
        }

        let mut total_score: f64 = 0.0;
        let mut capability_count = 0;

        for required_capability in required_capabilities {
            let mut best_score: f64 = 0.0;

            for agent_capability in &agent.capabilities {
                if agent_capability.capability_id == *required_capability {
                    // Exact match, weighted by performance
                    best_score = best_score.max(agent_capability.performance_metrics.success_rate);
                } else if agent_capability
                    .name
                    .to_lowercase()
                    .contains(&required_capability.to_lowercase())
                    || required_capability
                        .to_lowercase()
                        .contains(&agent_capability.name.to_lowercase())
                {
                    // Partial name match
                    best_score =
                        best_score.max(agent_capability.performance_metrics.success_rate * 0.7);
                } else if agent_capability
                    .category
                    .to_lowercase()
                    .contains(&required_capability.to_lowercase())
                {
                    // Category match
                    best_score =
                        best_score.max(agent_capability.performance_metrics.success_rate * 0.5);
                }
            }

            total_score += best_score;
            capability_count += 1;
        }

        Ok(if capability_count > 0 {
            total_score / capability_count as f64
        } else {
            1.0
        })
    }

    /// Calculate domain expertise score
    async fn calculate_domain_score(
        &self,
        agent: &AgentMetadata,
        required_domains: &[String],
        identified_domains: &[String],
    ) -> RegistryResult<f64> {
        let all_domains: HashSet<String> = required_domains
            .iter()
            .chain(identified_domains.iter())
            .cloned()
            .collect();

        if all_domains.is_empty() {
            return Ok(1.0);
        }

        let mut total_score: f64 = 0.0;
        let mut domain_count = 0;

        for domain in &all_domains {
            let mut best_score: f64 = 0.0;

            // Check if agent can handle this domain
            if agent.can_handle_domain(domain) {
                best_score = 1.0;
            } else {
                // Check for partial matches in knowledge context
                for agent_domain in &agent.knowledge_context.domains {
                    if agent_domain.to_lowercase().contains(&domain.to_lowercase())
                        || domain.to_lowercase().contains(&agent_domain.to_lowercase())
                    {
                        best_score = best_score.max(0.7);
                    }
                }
            }

            total_score += best_score;
            domain_count += 1;
        }

        Ok(if domain_count > 0 {
            total_score / domain_count as f64
        } else {
            1.0
        })
    }

    /// Calculate performance score
    async fn calculate_performance_score(
        &self,
        agent: &AgentMetadata,
        query: &AgentDiscoveryQuery,
    ) -> RegistryResult<f64> {
        let mut score = agent.get_success_rate();

        // Apply minimum success rate filter
        if let Some(min_success_rate) = query.min_success_rate {
            if score < min_success_rate {
                score *= 0.5; // Penalize agents below minimum
            }
        }

        // Consider resource usage if specified
        if let Some(max_resource_usage) = &query.max_resource_usage {
            if let Some((_, latest_usage)) = agent.statistics.resource_history.last() {
                if latest_usage.memory_mb > max_resource_usage.memory_mb
                    || latest_usage.cpu_percent > max_resource_usage.cpu_percent
                {
                    score *= 0.7; // Penalize high resource usage
                }
            }
        }

        Ok(score)
    }

    /// Calculate availability score
    async fn calculate_availability_score(&self, agent: &AgentMetadata) -> RegistryResult<f64> {
        match agent.status {
            crate::AgentStatus::Active => Ok(1.0),
            crate::AgentStatus::Idle => Ok(1.0),
            crate::AgentStatus::Busy => Ok(0.5),
            crate::AgentStatus::Hibernating => Ok(0.8),
            crate::AgentStatus::Initializing => Ok(0.3),
            crate::AgentStatus::Terminating => Ok(0.0),
            crate::AgentStatus::Terminated => Ok(0.0),
            crate::AgentStatus::Failed(_) => Ok(0.0),
        }
    }

    /// Generate explanation for agent match
    fn generate_match_explanation(
        &self,
        agent: &AgentMetadata,
        score_breakdown: &ScoreBreakdown,
        _query_analysis: &QueryAnalysis,
    ) -> String {
        let mut explanation = format!("Agent {} ({})", agent.agent_id, agent.primary_role.name);

        if score_breakdown.role_score > 0.8 {
            explanation.push_str(" has excellent role compatibility");
        } else if score_breakdown.role_score > 0.6 {
            explanation.push_str(" has good role compatibility");
        } else {
            explanation.push_str(" has limited role compatibility");
        }

        if score_breakdown.capability_score > 0.8 {
            explanation.push_str(" and strong capability match");
        } else if score_breakdown.capability_score > 0.6 {
            explanation.push_str(" and moderate capability match");
        } else {
            explanation.push_str(" but limited capability match");
        }

        if score_breakdown.performance_score > 0.8 {
            explanation.push_str(". Performance history is excellent");
        } else if score_breakdown.performance_score > 0.6 {
            explanation.push_str(". Performance history is good");
        } else {
            explanation.push_str(". Performance history needs improvement");
        }

        explanation.push('.');
        explanation
    }

    /// Generate suggestions for improving the query
    async fn generate_query_suggestions(
        &self,
        query: &AgentDiscoveryQuery,
        query_analysis: &QueryAnalysis,
        matches: &[AgentMatch],
    ) -> Vec<String> {
        let mut suggestions = Vec::new();

        // Suggest additional roles if connectivity analysis shows gaps
        if !query_analysis.connectivity_analysis.all_connected {
            suggestions
                .push("Consider adding related roles to improve agent connectivity".to_string());
        }

        // Suggest relaxing requirements if no good matches
        if matches.is_empty() || matches.iter().all(|m| m.match_score < 0.5) {
            suggestions.push(
                "Consider relaxing some requirements to find more suitable agents".to_string(),
            );
        }

        // Suggest additional capabilities based on identified domains
        if !query_analysis.identified_domains.is_empty() && query.required_capabilities.is_empty() {
            suggestions.push(format!(
                "Consider specifying capabilities for domains: {}",
                query_analysis.identified_domains.join(", ")
            ));
        }

        suggestions
    }

    /// Clear expired cache entries
    pub async fn cleanup_cache(&self) {
        let mut cache = self.query_cache.write().await;
        let now = chrono::Utc::now();
        cache.retain(|_, result| result.expires_at > now);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{AgentMetadata, AgentPid, AgentRole, SupervisorId};

    #[tokio::test]
    async fn test_knowledge_graph_integration_creation() {
        let role_graph = Arc::new(RoleGraph::new());
        let automata_config = AutomataConfig::default();
        let similarity_thresholds = SimilarityThresholds::default();

        let kg_integration =
            KnowledgeGraphIntegration::new(role_graph, automata_config, similarity_thresholds);

        // Test that the integration was created successfully
        assert_eq!(kg_integration.similarity_thresholds.role_similarity, 0.8);
    }

    #[tokio::test]
    async fn test_agent_discovery_query() {
        let query = AgentDiscoveryQuery {
            required_roles: vec!["planner".to_string()],
            required_capabilities: vec!["task_planning".to_string()],
            required_domains: vec!["project_management".to_string()],
            task_description: Some(
                "Plan and coordinate a software development project".to_string(),
            ),
            min_success_rate: Some(0.8),
            max_resource_usage: None,
            preferred_tags: vec!["experienced".to_string()],
        };

        assert_eq!(query.required_roles.len(), 1);
        assert_eq!(query.required_capabilities.len(), 1);
        assert!(query.task_description.is_some());
    }

    #[tokio::test]
    async fn test_score_calculation() {
        let role_graph = Arc::new(RoleGraph::new());
        let automata_config = AutomataConfig::default();
        let similarity_thresholds = SimilarityThresholds::default();

        let kg_integration =
            KnowledgeGraphIntegration::new(role_graph, automata_config, similarity_thresholds);

        // Create test agent
        let agent_id = AgentPid::new();
        let supervisor_id = SupervisorId::new();
        let role = AgentRole::new(
            "planner".to_string(),
            "Planning Agent".to_string(),
            "Responsible for task planning".to_string(),
        );

        let agent = AgentMetadata::new(agent_id, supervisor_id, role);

        // Test availability score calculation
        let availability_score = kg_integration
            .calculate_availability_score(&agent)
            .await
            .unwrap();
        assert!(availability_score >= 0.0 && availability_score <= 1.0);
    }
}
