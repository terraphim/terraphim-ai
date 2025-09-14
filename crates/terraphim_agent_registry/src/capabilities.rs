//! Agent capability management and matching
//!
//! Provides utilities for managing agent capabilities, capability matching,
//! and capability-based agent discovery.

use std::collections::{HashMap, HashSet};
use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::{AgentCapability, CapabilityMetrics, RegistryError, RegistryResult, ResourceUsage};

/// Capability registry for managing and discovering capabilities
pub struct CapabilityRegistry {
    /// All registered capabilities
    capabilities: HashMap<String, AgentCapability>,
    /// Capability categories
    categories: HashMap<String, Vec<String>>,
    /// Capability dependencies graph
    dependencies: HashMap<String, Vec<String>>,
    /// Capability compatibility matrix
    compatibility: HashMap<String, HashMap<String, f64>>,
}

/// Capability matching query
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapabilityQuery {
    /// Required capabilities
    pub required_capabilities: Vec<String>,
    /// Optional capabilities (nice to have)
    pub optional_capabilities: Vec<String>,
    /// Minimum performance requirements
    pub min_performance: Option<CapabilityMetrics>,
    /// Maximum resource constraints
    pub max_resources: Option<ResourceUsage>,
    /// Capability categories to search in
    pub categories: Vec<String>,
    /// Input/output type requirements
    pub io_requirements: IORequirements,
}

/// Input/output type requirements
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IORequirements {
    /// Required input types
    pub input_types: Vec<String>,
    /// Required output types
    pub output_types: Vec<String>,
    /// Input/output compatibility matrix
    pub compatibility_matrix: HashMap<String, Vec<String>>,
}

impl Default for IORequirements {
    fn default() -> Self {
        Self {
            input_types: Vec::new(),
            output_types: Vec::new(),
            compatibility_matrix: HashMap::new(),
        }
    }
}

/// Capability matching result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapabilityMatch {
    /// Matched capability
    pub capability: AgentCapability,
    /// Match score (0.0 to 1.0)
    pub match_score: f64,
    /// Detailed match breakdown
    pub match_details: CapabilityMatchDetails,
    /// Explanation of the match
    pub explanation: String,
}

/// Detailed capability match information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapabilityMatchDetails {
    /// Exact requirement matches
    pub exact_matches: Vec<String>,
    /// Partial requirement matches
    pub partial_matches: Vec<(String, f64)>,
    /// Missing requirements
    pub missing_requirements: Vec<String>,
    /// Performance score
    pub performance_score: f64,
    /// Resource compatibility score
    pub resource_score: f64,
    /// IO compatibility score
    pub io_score: f64,
}

/// Capability template for creating new capabilities
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapabilityTemplate {
    /// Template name
    pub name: String,
    /// Template description
    pub description: String,
    /// Default category
    pub default_category: String,
    /// Required fields
    pub required_fields: Vec<String>,
    /// Optional fields with defaults
    pub optional_fields: HashMap<String, serde_json::Value>,
    /// Performance benchmarks
    pub performance_benchmarks: CapabilityMetrics,
}

impl CapabilityRegistry {
    /// Create a new capability registry
    pub fn new() -> Self {
        Self {
            capabilities: HashMap::new(),
            categories: HashMap::new(),
            dependencies: HashMap::new(),
            compatibility: HashMap::new(),
        }
    }

    /// Register a new capability
    pub fn register_capability(&mut self, capability: AgentCapability) -> RegistryResult<()> {
        let capability_id = capability.capability_id.clone();

        // Validate capability
        self.validate_capability(&capability)?;

        // Add to category
        self.categories
            .entry(capability.category.clone())
            .or_insert_with(Vec::new)
            .push(capability_id.clone());

        // Register dependencies
        if !capability.dependencies.is_empty() {
            self.dependencies
                .insert(capability_id.clone(), capability.dependencies.clone());
        }

        // Store capability
        self.capabilities.insert(capability_id, capability);

        Ok(())
    }

    /// Unregister a capability
    pub fn unregister_capability(&mut self, capability_id: &str) -> RegistryResult<()> {
        if let Some(capability) = self.capabilities.remove(capability_id) {
            // Remove from category
            if let Some(category_capabilities) = self.categories.get_mut(&capability.category) {
                category_capabilities.retain(|id| id != capability_id);
                if category_capabilities.is_empty() {
                    self.categories.remove(&capability.category);
                }
            }

            // Remove dependencies
            self.dependencies.remove(capability_id);

            // Remove from compatibility matrix
            self.compatibility.remove(capability_id);
            for compatibility_map in self.compatibility.values_mut() {
                compatibility_map.remove(capability_id);
            }

            Ok(())
        } else {
            Err(RegistryError::System(format!(
                "Capability {} not found",
                capability_id
            )))
        }
    }

    /// Get capability by ID
    pub fn get_capability(&self, capability_id: &str) -> Option<&AgentCapability> {
        self.capabilities.get(capability_id)
    }

    /// List all capabilities
    pub fn list_capabilities(&self) -> Vec<&AgentCapability> {
        self.capabilities.values().collect()
    }

    /// List capabilities by category
    pub fn list_capabilities_by_category(&self, category: &str) -> Vec<&AgentCapability> {
        if let Some(capability_ids) = self.categories.get(category) {
            capability_ids
                .iter()
                .filter_map(|id| self.capabilities.get(id))
                .collect()
        } else {
            Vec::new()
        }
    }

    /// Find capabilities matching a query
    pub fn find_capabilities(
        &self,
        query: &CapabilityQuery,
    ) -> RegistryResult<Vec<CapabilityMatch>> {
        let mut matches = Vec::new();

        // Get candidate capabilities
        let candidates = if query.categories.is_empty() {
            self.list_capabilities()
        } else {
            let mut candidates = Vec::new();
            for category in &query.categories {
                candidates.extend(self.list_capabilities_by_category(category));
            }
            candidates
        };

        // Score each candidate
        for capability in candidates {
            if let Ok(capability_match) = self.score_capability_match(capability, query) {
                if capability_match.match_score > 0.0 {
                    matches.push(capability_match);
                }
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

    /// Score how well a capability matches a query
    fn score_capability_match(
        &self,
        capability: &AgentCapability,
        query: &CapabilityQuery,
    ) -> RegistryResult<CapabilityMatch> {
        let mut exact_matches = Vec::new();
        let mut partial_matches = Vec::new();
        let mut missing_requirements = Vec::new();

        // Check required capabilities
        let mut requirement_score = 0.0;
        let mut total_requirements = query.required_capabilities.len();

        for required_cap in &query.required_capabilities {
            if capability.capability_id == *required_cap {
                exact_matches.push(required_cap.clone());
                requirement_score += 1.0;
            } else {
                // Check for partial matches
                let similarity =
                    self.calculate_capability_similarity(&capability.capability_id, required_cap);
                if similarity > 0.5 {
                    partial_matches.push((required_cap.clone(), similarity));
                    requirement_score += similarity;
                } else {
                    missing_requirements.push(required_cap.clone());
                }
            }
        }

        // Check optional capabilities (bonus points)
        for optional_cap in &query.optional_capabilities {
            if capability.capability_id == *optional_cap {
                requirement_score += 0.5; // Bonus for optional matches
            } else {
                let similarity =
                    self.calculate_capability_similarity(&capability.capability_id, optional_cap);
                if similarity > 0.5 {
                    requirement_score += similarity * 0.3; // Smaller bonus for partial optional matches
                }
            }
        }

        // Normalize requirement score
        if total_requirements > 0 {
            requirement_score = (requirement_score / total_requirements as f64).min(1.0);
        } else {
            requirement_score = 1.0;
        }

        // Calculate performance score
        let performance_score = if let Some(min_performance) = &query.min_performance {
            self.calculate_performance_score(&capability.performance_metrics, min_performance)
        } else {
            1.0
        };

        // Calculate resource score
        let resource_score = if let Some(max_resources) = &query.max_resources {
            self.calculate_resource_score(
                &capability.performance_metrics.resource_usage,
                max_resources,
            )
        } else {
            1.0
        };

        // Calculate IO compatibility score
        let io_score = self.calculate_io_score(capability, &query.io_requirements);

        // Calculate overall match score
        let match_score = (requirement_score * 0.4
            + performance_score * 0.25
            + resource_score * 0.2
            + io_score * 0.15)
            .min(1.0)
            .max(0.0);

        let match_details = CapabilityMatchDetails {
            exact_matches,
            partial_matches,
            missing_requirements,
            performance_score,
            resource_score,
            io_score,
        };

        let explanation = self.generate_match_explanation(capability, &match_details, match_score);

        Ok(CapabilityMatch {
            capability: capability.clone(),
            match_score,
            match_details,
            explanation,
        })
    }

    /// Calculate similarity between two capabilities
    fn calculate_capability_similarity(&self, cap1: &str, cap2: &str) -> f64 {
        // Check compatibility matrix first
        if let Some(cap1_compat) = self.compatibility.get(cap1) {
            if let Some(similarity) = cap1_compat.get(cap2) {
                return *similarity;
            }
        }

        // Fallback to string similarity
        self.string_similarity(cap1, cap2)
    }

    /// Calculate string similarity (simple implementation)
    fn string_similarity(&self, s1: &str, s2: &str) -> f64 {
        let s1_lower = s1.to_lowercase();
        let s2_lower = s2.to_lowercase();

        if s1_lower == s2_lower {
            return 1.0;
        }

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

    /// Calculate performance score
    fn calculate_performance_score(
        &self,
        actual: &CapabilityMetrics,
        required: &CapabilityMetrics,
    ) -> f64 {
        let mut score = 1.0;

        // Check success rate
        if actual.success_rate < required.success_rate {
            score *= actual.success_rate / required.success_rate;
        }

        // Check execution time (lower is better)
        if actual.avg_execution_time > required.avg_execution_time {
            let time_ratio =
                required.avg_execution_time.as_secs_f64() / actual.avg_execution_time.as_secs_f64();
            score *= time_ratio.min(1.0);
        }

        // Check quality score
        if actual.quality_score < required.quality_score {
            score *= actual.quality_score / required.quality_score;
        }

        score.max(0.0).min(1.0)
    }

    /// Calculate resource compatibility score
    fn calculate_resource_score(&self, actual: &ResourceUsage, max_allowed: &ResourceUsage) -> f64 {
        let mut score = 1.0;

        // Check memory usage
        if actual.memory_mb > max_allowed.memory_mb {
            score *= max_allowed.memory_mb / actual.memory_mb;
        }

        // Check CPU usage
        if actual.cpu_percent > max_allowed.cpu_percent {
            score *= max_allowed.cpu_percent / actual.cpu_percent;
        }

        // Check network usage
        if actual.network_kbps > max_allowed.network_kbps {
            score *= max_allowed.network_kbps / actual.network_kbps;
        }

        // Check storage usage
        if actual.storage_mb > max_allowed.storage_mb {
            score *= max_allowed.storage_mb / actual.storage_mb;
        }

        score.max(0.0).min(1.0)
    }

    /// Calculate input/output compatibility score
    fn calculate_io_score(
        &self,
        capability: &AgentCapability,
        requirements: &IORequirements,
    ) -> f64 {
        if requirements.input_types.is_empty() && requirements.output_types.is_empty() {
            return 1.0;
        }

        let mut input_score = 1.0;
        let mut output_score = 1.0;

        // Check input type compatibility
        if !requirements.input_types.is_empty() {
            let mut matching_inputs = 0;
            for required_input in &requirements.input_types {
                if capability.input_types.contains(required_input) {
                    matching_inputs += 1;
                } else {
                    // Check compatibility matrix
                    if let Some(compatible_types) =
                        requirements.compatibility_matrix.get(required_input)
                    {
                        if capability
                            .input_types
                            .iter()
                            .any(|input| compatible_types.contains(input))
                        {
                            matching_inputs += 1;
                        }
                    }
                }
            }
            input_score = matching_inputs as f64 / requirements.input_types.len() as f64;
        }

        // Check output type compatibility
        if !requirements.output_types.is_empty() {
            let mut matching_outputs = 0;
            for required_output in &requirements.output_types {
                if capability.output_types.contains(required_output) {
                    matching_outputs += 1;
                } else {
                    // Check compatibility matrix
                    if let Some(compatible_types) =
                        requirements.compatibility_matrix.get(required_output)
                    {
                        if capability
                            .output_types
                            .iter()
                            .any(|output| compatible_types.contains(output))
                        {
                            matching_outputs += 1;
                        }
                    }
                }
            }
            output_score = matching_outputs as f64 / requirements.output_types.len() as f64;
        }

        (input_score + output_score) / 2.0
    }

    /// Generate explanation for capability match
    fn generate_match_explanation(
        &self,
        capability: &AgentCapability,
        details: &CapabilityMatchDetails,
        match_score: f64,
    ) -> String {
        let mut explanation = format!("Capability '{}' ", capability.name);

        if !details.exact_matches.is_empty() {
            explanation.push_str(&format!(
                "exactly matches {} requirements",
                details.exact_matches.len()
            ));
        }

        if !details.partial_matches.is_empty() {
            if !details.exact_matches.is_empty() {
                explanation.push_str(" and ");
            }
            explanation.push_str(&format!(
                "partially matches {} requirements",
                details.partial_matches.len()
            ));
        }

        if !details.missing_requirements.is_empty() {
            explanation.push_str(&format!(
                ", missing {} requirements",
                details.missing_requirements.len()
            ));
        }

        explanation.push_str(&format!(
            ". Performance: {:.1}%, Resources: {:.1}%, I/O: {:.1}%. Overall match: {:.1}%",
            details.performance_score * 100.0,
            details.resource_score * 100.0,
            details.io_score * 100.0,
            match_score * 100.0
        ));

        explanation
    }

    /// Validate capability before registration
    fn validate_capability(&self, capability: &AgentCapability) -> RegistryResult<()> {
        if capability.capability_id.is_empty() {
            return Err(RegistryError::System(
                "Capability ID cannot be empty".to_string(),
            ));
        }

        if capability.name.is_empty() {
            return Err(RegistryError::System(
                "Capability name cannot be empty".to_string(),
            ));
        }

        if capability.category.is_empty() {
            return Err(RegistryError::System(
                "Capability category cannot be empty".to_string(),
            ));
        }

        if capability.performance_metrics.success_rate < 0.0
            || capability.performance_metrics.success_rate > 1.0
        {
            return Err(RegistryError::System(
                "Success rate must be between 0.0 and 1.0".to_string(),
            ));
        }

        if capability.performance_metrics.quality_score < 0.0
            || capability.performance_metrics.quality_score > 1.0
        {
            return Err(RegistryError::System(
                "Quality score must be between 0.0 and 1.0".to_string(),
            ));
        }

        Ok(())
    }

    /// Set capability compatibility
    pub fn set_capability_compatibility(&mut self, cap1: &str, cap2: &str, similarity: f64) {
        self.compatibility
            .entry(cap1.to_string())
            .or_insert_with(HashMap::new)
            .insert(cap2.to_string(), similarity);

        // Set reverse compatibility
        self.compatibility
            .entry(cap2.to_string())
            .or_insert_with(HashMap::new)
            .insert(cap1.to_string(), similarity);
    }

    /// Get capability dependencies
    pub fn get_dependencies(&self, capability_id: &str) -> Vec<String> {
        self.dependencies
            .get(capability_id)
            .cloned()
            .unwrap_or_default()
    }

    /// Check if all dependencies are satisfied
    pub fn check_dependencies(
        &self,
        capability_id: &str,
        available_capabilities: &[String],
    ) -> bool {
        if let Some(dependencies) = self.dependencies.get(capability_id) {
            dependencies
                .iter()
                .all(|dep| available_capabilities.contains(dep))
        } else {
            true // No dependencies
        }
    }

    /// Get capability statistics
    pub fn get_statistics(&self) -> CapabilityRegistryStats {
        let mut categories_count = HashMap::new();
        for (category, capabilities) in &self.categories {
            categories_count.insert(category.clone(), capabilities.len());
        }

        CapabilityRegistryStats {
            total_capabilities: self.capabilities.len(),
            categories_count,
            total_dependencies: self.dependencies.len(),
            compatibility_entries: self.compatibility.values().map(|m| m.len()).sum(),
        }
    }
}

/// Capability registry statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapabilityRegistryStats {
    pub total_capabilities: usize,
    pub categories_count: HashMap<String, usize>,
    pub total_dependencies: usize,
    pub compatibility_entries: usize,
}

impl Default for CapabilityRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn test_capability_registry_creation() {
        let registry = CapabilityRegistry::new();
        assert_eq!(registry.list_capabilities().len(), 0);
    }

    #[test]
    fn test_capability_registration() {
        let mut registry = CapabilityRegistry::new();

        let capability = AgentCapability {
            capability_id: "test_capability".to_string(),
            name: "Test Capability".to_string(),
            description: "A test capability".to_string(),
            category: "testing".to_string(),
            required_domains: vec!["test_domain".to_string()],
            input_types: vec!["text".to_string()],
            output_types: vec!["result".to_string()],
            performance_metrics: CapabilityMetrics::default(),
            dependencies: Vec::new(),
        };

        registry.register_capability(capability.clone()).unwrap();

        assert_eq!(registry.list_capabilities().len(), 1);
        assert!(registry.get_capability("test_capability").is_some());

        let by_category = registry.list_capabilities_by_category("testing");
        assert_eq!(by_category.len(), 1);
    }

    #[test]
    fn test_capability_matching() {
        let mut registry = CapabilityRegistry::new();

        let capability = AgentCapability {
            capability_id: "planning".to_string(),
            name: "Task Planning".to_string(),
            description: "Plan and organize tasks".to_string(),
            category: "planning".to_string(),
            required_domains: vec!["project_management".to_string()],
            input_types: vec!["requirements".to_string()],
            output_types: vec!["plan".to_string()],
            performance_metrics: CapabilityMetrics {
                avg_execution_time: Duration::from_secs(5),
                success_rate: 0.9,
                resource_usage: ResourceUsage {
                    memory_mb: 100.0,
                    cpu_percent: 20.0,
                    network_kbps: 10.0,
                    storage_mb: 50.0,
                },
                quality_score: 0.85,
                last_updated: chrono::Utc::now(),
            },
            dependencies: Vec::new(),
        };

        registry.register_capability(capability).unwrap();

        let query = CapabilityQuery {
            required_capabilities: vec!["planning".to_string()],
            optional_capabilities: Vec::new(),
            min_performance: None,
            max_resources: None,
            categories: Vec::new(),
            io_requirements: IORequirements::default(),
        };

        let matches = registry.find_capabilities(&query).unwrap();
        assert_eq!(matches.len(), 1);
        assert!(matches[0].match_score > 0.0);
    }

    #[test]
    fn test_capability_compatibility() {
        let mut registry = CapabilityRegistry::new();

        registry.set_capability_compatibility("planning", "task_planning", 0.9);

        let similarity = registry.calculate_capability_similarity("planning", "task_planning");
        assert_eq!(similarity, 0.9);
    }

    #[test]
    fn test_dependency_checking() {
        let mut registry = CapabilityRegistry::new();

        let capability = AgentCapability {
            capability_id: "advanced_planning".to_string(),
            name: "Advanced Planning".to_string(),
            description: "Advanced task planning".to_string(),
            category: "planning".to_string(),
            required_domains: Vec::new(),
            input_types: Vec::new(),
            output_types: Vec::new(),
            performance_metrics: CapabilityMetrics::default(),
            dependencies: vec!["basic_planning".to_string()],
        };

        registry.register_capability(capability).unwrap();

        let available = vec!["basic_planning".to_string()];
        assert!(registry.check_dependencies("advanced_planning", &available));

        let unavailable = vec!["other_capability".to_string()];
        assert!(!registry.check_dependencies("advanced_planning", &unavailable));
    }
}
