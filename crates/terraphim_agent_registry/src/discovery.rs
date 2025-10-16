//! Agent discovery utilities and algorithms
//!
//! Provides specialized discovery algorithms and utilities for finding agents
//! based on various criteria and requirements.

use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};

use crate::{
    AgentDiscoveryQuery, AgentDiscoveryResult, AgentMatch, AgentMetadata, ConnectivityResult,
    QueryAnalysis, RegistryError, RegistryResult, ScoreBreakdown,
};

/// Discovery algorithm types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DiscoveryAlgorithm {
    /// Simple exact matching
    ExactMatch,
    /// Fuzzy matching with similarity scores
    FuzzyMatch,
    /// Knowledge graph-based semantic matching
    SemanticMatch,
    /// Machine learning-based matching
    MLMatch,
    /// Hybrid approach combining multiple algorithms
    Hybrid(Vec<DiscoveryAlgorithm>),
}

/// Discovery context for maintaining state across queries
#[derive(Debug, Clone)]
pub struct DiscoveryContext {
    /// Previous queries for learning
    pub query_history: Vec<AgentDiscoveryQuery>,
    /// Agent performance feedback
    pub performance_feedback: HashMap<String, f64>,
    /// User preferences
    pub user_preferences: UserPreferences,
    /// Discovery algorithm to use
    pub algorithm: DiscoveryAlgorithm,
}

/// User preferences for agent discovery
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserPreferences {
    /// Preferred agent roles
    pub preferred_roles: Vec<String>,
    /// Preferred capabilities
    pub preferred_capabilities: Vec<String>,
    /// Performance weight (0.0 to 1.0)
    pub performance_weight: f64,
    /// Availability weight (0.0 to 1.0)
    pub availability_weight: f64,
    /// Experience weight (0.0 to 1.0)
    pub experience_weight: f64,
    /// Cost sensitivity (0.0 to 1.0)
    pub cost_sensitivity: f64,
}

impl Default for UserPreferences {
    fn default() -> Self {
        Self {
            preferred_roles: Vec::new(),
            preferred_capabilities: Vec::new(),
            performance_weight: 0.3,
            availability_weight: 0.3,
            experience_weight: 0.2,
            cost_sensitivity: 0.2,
        }
    }
}

/// Agent discovery engine
pub struct DiscoveryEngine {
    /// Discovery context
    context: DiscoveryContext,
    /// Algorithm implementations
    algorithms: HashMap<String, Box<dyn DiscoveryAlgorithmImpl>>,
}

/// Trait for discovery algorithm implementations
pub trait DiscoveryAlgorithmImpl: Send + Sync {
    /// Execute the discovery algorithm
    fn discover(
        &self,
        query: &AgentDiscoveryQuery,
        agents: &[AgentMetadata],
        context: &DiscoveryContext,
    ) -> RegistryResult<Vec<AgentMatch>>;

    /// Get algorithm name
    fn name(&self) -> &str;

    /// Get algorithm description
    fn description(&self) -> &str;
}

/// Exact match discovery algorithm
pub struct ExactMatchAlgorithm;

impl DiscoveryAlgorithmImpl for ExactMatchAlgorithm {
    #[allow(unused_variables)]
    fn discover(
        &self,
        query: &AgentDiscoveryQuery,
        agents: &[AgentMetadata],
        context: &DiscoveryContext,
    ) -> RegistryResult<Vec<AgentMatch>> {
        let mut matches = Vec::new();

        for agent in agents {
            let mut match_score: f64 = 0.0;
            let mut matches_count = 0;
            let mut total_requirements = 0;

            // Check role requirements
            if !query.required_roles.is_empty() {
                total_requirements += query.required_roles.len();
                for required_role in &query.required_roles {
                    if agent.has_role(required_role) {
                        matches_count += 1;
                    }
                }
            }

            // Check capability requirements
            if !query.required_capabilities.is_empty() {
                total_requirements += query.required_capabilities.len();
                for required_capability in &query.required_capabilities {
                    if agent.has_capability(required_capability) {
                        matches_count += 1;
                    }
                }
            }

            // Check domain requirements
            if !query.required_domains.is_empty() {
                total_requirements += query.required_domains.len();
                for required_domain in &query.required_domains {
                    if agent.can_handle_domain(required_domain) {
                        matches_count += 1;
                    }
                }
            }

            // Calculate match score
            if total_requirements > 0 {
                match_score = matches_count as f64 / total_requirements as f64;
            }

            // Apply minimum success rate filter
            if let Some(min_success_rate) = query.min_success_rate {
                if agent.get_success_rate() < min_success_rate {
                    continue;
                }
            }

            // Only include agents with some match
            if match_score > 0.0 {
                let score_breakdown = ScoreBreakdown {
                    role_score: if query.required_roles.is_empty() {
                        1.0
                    } else {
                        query
                            .required_roles
                            .iter()
                            .map(|role| if agent.has_role(role) { 1.0 } else { 0.0 })
                            .sum::<f64>()
                            / query.required_roles.len() as f64
                    },
                    capability_score: if query.required_capabilities.is_empty() {
                        1.0
                    } else {
                        query
                            .required_capabilities
                            .iter()
                            .map(|cap| if agent.has_capability(cap) { 1.0 } else { 0.0 })
                            .sum::<f64>()
                            / query.required_capabilities.len() as f64
                    },
                    domain_score: if query.required_domains.is_empty() {
                        1.0
                    } else {
                        query
                            .required_domains
                            .iter()
                            .map(|domain| {
                                if agent.can_handle_domain(domain) {
                                    1.0
                                } else {
                                    0.0
                                }
                            })
                            .sum::<f64>()
                            / query.required_domains.len() as f64
                    },
                    performance_score: agent.get_success_rate(),
                    availability_score: match agent.status {
                        crate::AgentStatus::Active | crate::AgentStatus::Idle => 1.0,
                        crate::AgentStatus::Busy => 0.5,
                        _ => 0.0,
                    },
                };

                let explanation = format!(
                    "Agent {} matches {}/{} requirements with {:.1}% success rate",
                    agent.agent_id,
                    matches_count,
                    total_requirements,
                    agent.get_success_rate() * 100.0
                );

                matches.push(AgentMatch {
                    agent: agent.clone(),
                    match_score,
                    score_breakdown,
                    explanation,
                });
            }
        }

        // Sort by match score (highest first)
        matches.sort_by(|a, b| {
            b.match_score
                .partial_cmp(&a.match_score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        Ok(matches)
    }

    fn name(&self) -> &str {
        "ExactMatch"
    }

    fn description(&self) -> &str {
        "Exact matching algorithm that requires precise role, capability, and domain matches"
    }
}

/// Fuzzy match discovery algorithm
pub struct FuzzyMatchAlgorithm {
    /// Similarity threshold for fuzzy matching
    similarity_threshold: f64,
}

impl FuzzyMatchAlgorithm {
    pub fn new(similarity_threshold: f64) -> Self {
        Self {
            similarity_threshold,
        }
    }

    /// Calculate string similarity using Levenshtein distance
    fn string_similarity(&self, s1: &str, s2: &str) -> f64 {
        let s1_lower = s1.to_lowercase();
        let s2_lower = s2.to_lowercase();

        if s1_lower == s2_lower {
            return 1.0;
        }

        // Simple substring matching for now
        if s1_lower.contains(&s2_lower) || s2_lower.contains(&s1_lower) {
            return 0.7;
        }

        // Check for common words
        let s1_words: HashSet<&str> = s1_lower.split_whitespace().collect();
        let s2_words: HashSet<&str> = s2_lower.split_whitespace().collect();

        let intersection = s1_words.intersection(&s2_words).count();
        let union = s1_words.union(&s2_words).count();

        if union > 0 {
            intersection as f64 / union as f64
        } else {
            0.0
        }
    }
}

impl DiscoveryAlgorithmImpl for FuzzyMatchAlgorithm {
    #[allow(unused_variables, unused_assignments, clippy::manual_clamp)]
    fn discover(
        &self,
        query: &AgentDiscoveryQuery,
        agents: &[AgentMetadata],
        context: &DiscoveryContext,
    ) -> RegistryResult<Vec<AgentMatch>> {
        let mut matches = Vec::new();

        for agent in agents {
            let mut role_score: f64 = 0.0;
            let mut capability_score: f64 = 0.0;
            let mut domain_score: f64 = 0.0;

            // Calculate fuzzy role matching
            if !query.required_roles.is_empty() {
                let mut total_role_score: f64 = 0.0;
                for required_role in &query.required_roles {
                    let mut best_role_score: f64 = 0.0;

                    // Check primary role
                    best_role_score = best_role_score
                        .max(self.string_similarity(required_role, &agent.primary_role.role_id));
                    best_role_score = best_role_score
                        .max(self.string_similarity(required_role, &agent.primary_role.name));

                    // Check secondary roles
                    for secondary_role in &agent.secondary_roles {
                        best_role_score = best_role_score
                            .max(self.string_similarity(required_role, &secondary_role.role_id));
                        best_role_score = best_role_score
                            .max(self.string_similarity(required_role, &secondary_role.name));
                    }

                    total_role_score += best_role_score;
                }
                role_score = total_role_score / query.required_roles.len() as f64;
            } else {
                role_score = 1.0;
            }

            // Calculate fuzzy capability matching
            if !query.required_capabilities.is_empty() {
                let mut total_capability_score: f64 = 0.0;
                for required_capability in &query.required_capabilities {
                    let mut best_capability_score: f64 = 0.0;

                    for agent_capability in &agent.capabilities {
                        let id_similarity = self.string_similarity(
                            required_capability,
                            &agent_capability.capability_id,
                        );
                        let name_similarity =
                            self.string_similarity(required_capability, &agent_capability.name);
                        let category_similarity =
                            self.string_similarity(required_capability, &agent_capability.category);

                        let capability_similarity = id_similarity
                            .max(name_similarity)
                            .max(category_similarity * 0.7);
                        best_capability_score = best_capability_score.max(capability_similarity);
                    }

                    total_capability_score += best_capability_score;
                }
                capability_score =
                    total_capability_score / query.required_capabilities.len() as f64;
            } else {
                capability_score = 1.0;
            }

            // Calculate fuzzy domain matching
            if !query.required_domains.is_empty() {
                let mut total_domain_score: f64 = 0.0;
                for required_domain in &query.required_domains {
                    let mut best_domain_score: f64 = 0.0;

                    for agent_domain in &agent.knowledge_context.domains {
                        let domain_similarity =
                            self.string_similarity(required_domain, agent_domain);
                        best_domain_score = best_domain_score.max(domain_similarity);
                    }

                    // Also check role knowledge domains
                    for role in agent.get_all_roles() {
                        for role_domain in &role.knowledge_domains {
                            let domain_similarity =
                                self.string_similarity(required_domain, role_domain);
                            best_domain_score = best_domain_score.max(domain_similarity);
                        }
                    }

                    total_domain_score += best_domain_score;
                }
                domain_score = total_domain_score / query.required_domains.len() as f64;
            } else {
                domain_score = 1.0;
            }

            // Calculate overall match score
            let match_score = (role_score + capability_score + domain_score) / 3.0;

            // Apply similarity threshold
            if match_score >= self.similarity_threshold {
                // Apply performance and availability factors
                let performance_score = agent.get_success_rate();
                let availability_score = match agent.status {
                    crate::AgentStatus::Active | crate::AgentStatus::Idle => 1.0,
                    crate::AgentStatus::Busy => 0.5,
                    crate::AgentStatus::Hibernating => 0.8,
                    _ => 0.0,
                };

                let final_score =
                    match_score * 0.6 + performance_score * 0.25 + availability_score * 0.15;

                let score_breakdown = ScoreBreakdown {
                    role_score,
                    capability_score,
                    domain_score,
                    performance_score,
                    availability_score,
                };

                let explanation = format!(
                    "Agent {} fuzzy matches with {:.1}% similarity (role: {:.1}%, capability: {:.1}%, domain: {:.1}%)",
                    agent.agent_id,
                    match_score * 100.0,
                    role_score * 100.0,
                    capability_score * 100.0,
                    domain_score * 100.0
                );

                matches.push(AgentMatch {
                    agent: agent.clone(),
                    match_score: final_score,
                    score_breakdown,
                    explanation,
                });
            }
        }

        // Sort by match score (highest first)
        matches.sort_by(|a, b| {
            b.match_score
                .partial_cmp(&a.match_score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        Ok(matches)
    }

    fn name(&self) -> &str {
        "FuzzyMatch"
    }

    fn description(&self) -> &str {
        "Fuzzy matching algorithm that uses similarity scoring for approximate matches"
    }
}

impl DiscoveryEngine {
    /// Create a new discovery engine
    pub fn new(context: DiscoveryContext) -> Self {
        let mut algorithms: HashMap<String, Box<dyn DiscoveryAlgorithmImpl>> = HashMap::new();

        // Register built-in algorithms
        algorithms.insert("exact".to_string(), Box::new(ExactMatchAlgorithm));
        algorithms.insert("fuzzy".to_string(), Box::new(FuzzyMatchAlgorithm::new(0.5)));

        Self {
            context,
            algorithms,
        }
    }

    /// Register a custom discovery algorithm
    pub fn register_algorithm(&mut self, name: String, algorithm: Box<dyn DiscoveryAlgorithmImpl>) {
        self.algorithms.insert(name, algorithm);
    }

    /// Execute discovery using the configured algorithm
    pub fn discover(
        &self,
        query: &AgentDiscoveryQuery,
        agents: &[AgentMetadata],
    ) -> RegistryResult<AgentDiscoveryResult> {
        let matches = match &self.context.algorithm {
            DiscoveryAlgorithm::ExactMatch => self
                .algorithms
                .get("exact")
                .ok_or_else(|| RegistryError::System("ExactMatch algorithm not found".to_string()))?
                .discover(query, agents, &self.context)?,
            DiscoveryAlgorithm::FuzzyMatch => self
                .algorithms
                .get("fuzzy")
                .ok_or_else(|| RegistryError::System("FuzzyMatch algorithm not found".to_string()))?
                .discover(query, agents, &self.context)?,
            DiscoveryAlgorithm::SemanticMatch => {
                // Would use knowledge graph integration
                return Err(RegistryError::System(
                    "SemanticMatch not implemented yet".to_string(),
                ));
            }
            DiscoveryAlgorithm::MLMatch => {
                // Would use machine learning models
                return Err(RegistryError::System(
                    "MLMatch not implemented yet".to_string(),
                ));
            }
            DiscoveryAlgorithm::Hybrid(algorithms) => {
                // Combine results from multiple algorithms
                let mut all_matches = Vec::new();
                for algorithm in algorithms {
                    let temp_context = DiscoveryContext {
                        algorithm: algorithm.clone(),
                        ..self.context.clone()
                    };
                    // Create a temporary engine with empty algorithms since we're using a specific algorithm
                    let temp_engine = DiscoveryEngine {
                        context: temp_context,
                        algorithms: HashMap::new(),
                    };
                    let mut algorithm_matches = temp_engine.discover(query, agents)?;
                    all_matches.append(&mut algorithm_matches.matches);
                }

                // Deduplicate and merge scores
                let mut agent_scores: HashMap<String, (AgentMatch, f64, usize)> = HashMap::new();
                for agent_match in all_matches {
                    let agent_id = agent_match.agent.agent_id.to_string();
                    if let Some((_existing_match, total_score, count)) = agent_scores.get(&agent_id)
                    {
                        let new_total_score = total_score + agent_match.match_score;
                        let new_count = count + 1;
                        let avg_score = new_total_score / new_count as f64;

                        let mut updated_match = agent_match.clone();
                        updated_match.match_score = avg_score;

                        agent_scores.insert(agent_id, (updated_match, new_total_score, new_count));
                    } else {
                        let match_score = agent_match.match_score;
                        agent_scores.insert(agent_id, (agent_match, match_score, 1));
                    }
                }

                let mut final_matches: Vec<AgentMatch> = agent_scores
                    .into_values()
                    .map(|(agent_match, _, _)| agent_match)
                    .collect();

                final_matches.sort_by(|a, b| {
                    b.match_score
                        .partial_cmp(&a.match_score)
                        .unwrap_or(std::cmp::Ordering::Equal)
                });
                final_matches
            }
        };

        // Create query analysis (simplified for now)
        let query_analysis = QueryAnalysis {
            extracted_concepts: Vec::new(),
            identified_domains: query.required_domains.clone(),
            suggested_roles: Vec::new(),
            connectivity_analysis: ConnectivityResult {
                all_connected: true,
                paths: Vec::new(),
                disconnected: Vec::new(),
                strength_score: 1.0,
            },
        };

        // Generate suggestions
        let suggestions = self.generate_suggestions(query, &matches);

        Ok(AgentDiscoveryResult {
            matches,
            query_analysis,
            suggestions,
        })
    }

    /// Generate suggestions for improving discovery results
    fn generate_suggestions(
        &self,
        query: &AgentDiscoveryQuery,
        matches: &[AgentMatch],
    ) -> Vec<String> {
        let mut suggestions = Vec::new();

        if matches.is_empty() {
            suggestions.push("No agents found. Consider relaxing your requirements.".to_string());

            if !query.required_roles.is_empty() {
                suggestions.push(
                    "Try removing some role requirements or using more general roles.".to_string(),
                );
            }

            if !query.required_capabilities.is_empty() {
                suggestions
                    .push("Consider reducing the number of required capabilities.".to_string());
            }

            if query.min_success_rate.is_some() {
                suggestions.push("Try lowering the minimum success rate requirement.".to_string());
            }
        } else if matches.len() < 3 {
            suggestions
                .push("Few agents found. Consider broadening your search criteria.".to_string());
        } else if matches.iter().all(|m| m.match_score < 0.7) {
            suggestions.push(
                "Match scores are low. Consider adjusting your requirements for better matches."
                    .to_string(),
            );
        }

        suggestions
    }

    /// Update discovery context with feedback
    pub fn update_context(&mut self, feedback: HashMap<String, f64>) {
        self.context.performance_feedback.extend(feedback);
    }

    /// Get available algorithms
    pub fn get_available_algorithms(&self) -> Vec<String> {
        self.algorithms.keys().cloned().collect()
    }
}

impl Default for DiscoveryContext {
    fn default() -> Self {
        Self {
            query_history: Vec::new(),
            performance_feedback: HashMap::new(),
            user_preferences: UserPreferences::default(),
            algorithm: DiscoveryAlgorithm::FuzzyMatch,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{AgentMetadata, AgentRole};

    #[test]
    fn test_exact_match_algorithm() {
        let algorithm = ExactMatchAlgorithm;
        assert_eq!(algorithm.name(), "ExactMatch");

        // Create test data
        let agent_id = crate::AgentPid::new();
        let supervisor_id = crate::SupervisorId::new();
        let role = AgentRole::new(
            "planner".to_string(),
            "Planning Agent".to_string(),
            "Plans tasks".to_string(),
        );

        let agent = AgentMetadata::new(agent_id, supervisor_id, role);
        let agents = vec![agent];

        let query = AgentDiscoveryQuery {
            required_roles: vec!["planner".to_string()],
            required_capabilities: Vec::new(),
            required_domains: Vec::new(),
            task_description: None,
            min_success_rate: None,
            max_resource_usage: None,
            preferred_tags: Vec::new(),
        };

        let context = DiscoveryContext::default();
        let matches = algorithm.discover(&query, &agents, &context).unwrap();

        assert_eq!(matches.len(), 1);
        assert!(matches[0].match_score > 0.0);
    }

    #[test]
    fn test_fuzzy_match_algorithm() {
        let algorithm = FuzzyMatchAlgorithm::new(0.5);
        assert_eq!(algorithm.name(), "FuzzyMatch");

        // Test string similarity
        assert_eq!(algorithm.string_similarity("planner", "planner"), 1.0);
        // Note: The fuzzy match algorithm may return 0.0 for these strings
        // This is a non-critical test that checks the basic functionality
        assert!(algorithm.string_similarity("planner", "executor") < 0.5);
    }

    #[test]
    fn test_discovery_engine() {
        let context = DiscoveryContext::default();
        let engine = DiscoveryEngine::new(context);

        let algorithms = engine.get_available_algorithms();
        assert!(algorithms.contains(&"exact".to_string()));
        assert!(algorithms.contains(&"fuzzy".to_string()));
    }
}
