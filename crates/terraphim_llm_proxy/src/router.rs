use crate::{
    analyzer::RoutingHints,
    client_detection::{ClientInfo, ClientType},
    config::{Provider, ProxyConfig},
    cost::{BudgetManager, CostCalculator, PricingDatabase},
    performance::{PerformanceDatabase, PerformanceTester},
    rolegraph_client::RoleGraphClient,
    routing::model_mapper::ModelMapper,
    session::{SessionInfo, SessionManager},
    token_counter::ChatRequest,
    ProxyError, Result,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{debug, info, warn};

// ============================================================================
// Local Type Definitions
// ============================================================================

#[derive(Debug, Clone, PartialEq)]
pub enum RoutingScenario {
    Default,
    Background,
    Think,
    /// Plan implementation routing - for tactical/implementation tasks
    PlanImplementation,
    LongContext,
    WebSearch,
    Image,
    Pattern(String),
    Priority,
    Custom(String),
}

impl std::fmt::Display for RoutingScenario {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RoutingScenario::Default => write!(f, "default"),
            RoutingScenario::Background => write!(f, "background"),
            RoutingScenario::Think => write!(f, "think"),
            RoutingScenario::PlanImplementation => write!(f, "plan_implementation"),
            RoutingScenario::LongContext => write!(f, "long_context"),
            RoutingScenario::WebSearch => write!(f, "web_search"),
            RoutingScenario::Image => write!(f, "image"),
            RoutingScenario::Pattern(name) => write!(f, "pattern({})", name),
            RoutingScenario::Priority => write!(f, "priority"),
            RoutingScenario::Custom(name) => write!(f, "custom({})", name),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Eq, Ord, Serialize, Deserialize)]
pub enum Priority {
    Low = 1,
    Medium = 2,
    High = 3,
    Critical = 4,
}

impl Priority {
    pub fn new(value: u8) -> Self {
        match value {
            0..=25 => Priority::Low,
            26..=75 => Priority::Medium,
            76..=95 => Priority::High,
            96..=100 => Priority::Critical,
            _ => Priority::Critical, // Values above 100 are treated as critical
        }
    }

    pub fn value(&self) -> u8 {
        match self {
            Priority::Low => 1,
            Priority::Medium => 2,
            Priority::High => 3,
            Priority::Critical => 4,
        }
    }
}

/// Router agent for intelligent provider/model selection
pub struct RouterAgent {
    config: Arc<ProxyConfig>,
    rolegraph: Option<Arc<RoleGraphClient>>,
    session_manager: Option<Arc<SessionManager>>,
    performance_tester: Option<Arc<PerformanceTester>>,
    performance_database: Option<Arc<PerformanceDatabase>>,
    #[allow(dead_code)]
    pricing_database: Option<Arc<PricingDatabase>>,
    cost_calculator: Option<Arc<CostCalculator>>,
    budget_manager: Option<Arc<BudgetManager>>,
    model_mapper: Option<ModelMapper>,
}

/// Routing decision with provider and model
#[derive(Debug, Clone)]
pub struct RoutingDecision {
    pub provider: Provider,
    pub model: String,
    /// The original model name from the client request (for logging/debugging)
    pub original_model: Option<String>,
    /// The detected client type
    pub client_type: Option<ClientType>,
    pub scenario: RoutingScenario,
    pub priority: Priority,
    pub confidence: f64,
    pub rule_id: Option<String>,
    pub reason: String,
}

impl RoutingDecision {
    /// Create a new routing decision with optional client info
    pub fn new(
        provider: Provider,
        model: String,
        scenario: RoutingScenario,
        priority: Priority,
        confidence: f64,
        reason: String,
    ) -> Self {
        Self {
            provider,
            model,
            original_model: None,
            client_type: None,
            scenario,
            priority,
            confidence,
            rule_id: None,
            reason,
        }
    }

    /// Set client information for this decision
    pub fn with_client_info(mut self, client_info: &ClientInfo, original_model: String) -> Self {
        self.client_type = Some(client_info.client_type);
        self.original_model = Some(original_model);
        self
    }

    /// Set the translated model name
    pub fn with_translated_model(mut self, model: String) -> Self {
        self.model = model;
        self
    }
}

impl RouterAgent {
    fn route_provider_name(route: &str) -> Option<&str> {
        route.split_once(',').map(|(provider, _)| provider.trim())
    }

    fn route_targets_provider(route: &str, provider_name: &str) -> bool {
        Self::route_provider_name(route)
            .map(|name| name.eq_ignore_ascii_case(provider_name))
            .unwrap_or(false)
    }

    /// Create a new router agent
    pub fn new(config: Arc<ProxyConfig>) -> Self {
        Self {
            config,
            rolegraph: None,
            session_manager: None,
            performance_tester: None,
            performance_database: None,
            pricing_database: None,
            cost_calculator: None,
            budget_manager: None,
            model_mapper: None,
        }
    }

    /// Create a new router agent with model mapper enabled
    pub fn with_model_mapper(config: Arc<ProxyConfig>) -> Self {
        Self {
            config,
            rolegraph: None,
            session_manager: None,
            performance_tester: None,
            performance_database: None,
            pricing_database: None,
            cost_calculator: None,
            budget_manager: None,
            model_mapper: Some(ModelMapper::with_defaults()),
        }
    }

    /// Create router with RoleGraph client
    pub fn with_rolegraph(config: Arc<ProxyConfig>, rolegraph: Arc<RoleGraphClient>) -> Self {
        Self {
            config,
            rolegraph: Some(rolegraph),
            session_manager: None,
            performance_tester: None,
            performance_database: None,
            pricing_database: None,
            cost_calculator: None,
            budget_manager: None,
            model_mapper: None,
        }
    }

    /// Create router with all features enabled
    pub fn with_all_features(
        config: Arc<ProxyConfig>,
        rolegraph: Arc<RoleGraphClient>,
        session_manager: Arc<SessionManager>,
        performance_tester: Arc<PerformanceTester>,
        performance_database: Arc<PerformanceDatabase>,
    ) -> Self {
        Self {
            config,
            rolegraph: Some(rolegraph),
            session_manager: Some(session_manager),
            performance_tester: Some(performance_tester),
            performance_database: Some(performance_database),
            pricing_database: None,
            cost_calculator: None,
            budget_manager: None,
            model_mapper: None,
        }
    }

    /// Create router with session manager
    pub fn with_session_manager(
        config: Arc<ProxyConfig>,
        session_manager: Arc<SessionManager>,
    ) -> Self {
        Self {
            config,
            rolegraph: None,
            session_manager: Some(session_manager),
            performance_tester: None,
            performance_database: None,
            pricing_database: None,
            cost_calculator: None,
            budget_manager: None,
            model_mapper: None,
        }
    }

    /// Create router with performance features but no rolegraph
    pub fn with_performance_features(
        config: Arc<ProxyConfig>,
        session_manager: Arc<SessionManager>,
        performance_tester: Arc<PerformanceTester>,
        performance_database: Arc<PerformanceDatabase>,
    ) -> Self {
        Self {
            config,
            rolegraph: None,
            session_manager: Some(session_manager),
            performance_tester: Some(performance_tester),
            performance_database: Some(performance_database),
            pricing_database: None,
            cost_calculator: None,
            budget_manager: None,
            model_mapper: None,
        }
    }

    /// Create router with cost optimization features
    pub fn with_cost_optimization(
        config: Arc<ProxyConfig>,
        session_manager: Arc<SessionManager>,
        performance_tester: Arc<PerformanceTester>,
        performance_database: Arc<PerformanceDatabase>,
        pricing_database: Arc<PricingDatabase>,
        cost_calculator: Arc<CostCalculator>,
        budget_manager: Arc<BudgetManager>,
    ) -> Self {
        Self {
            config,
            rolegraph: None,
            session_manager: Some(session_manager),
            performance_tester: Some(performance_tester),
            performance_database: Some(performance_database),
            pricing_database: Some(pricing_database),
            cost_calculator: Some(cost_calculator),
            budget_manager: Some(budget_manager),
            model_mapper: None,
        }
    }

    /// Translate model name using the model mapper if available
    ///
    /// This method takes a routing decision and translates the model name
    /// based on the original client request and target provider.
    fn translate_model_name(
        &self,
        mut decision: RoutingDecision,
        original_model: &str,
        client_info: Option<&ClientInfo>,
    ) -> RoutingDecision {
        // Store original model for debugging
        decision.original_model = Some(original_model.to_string());

        // Store client type if available
        if let Some(info) = client_info {
            decision.client_type = Some(info.client_type);
        }

        // If no model mapper is configured, return decision as-is
        let mut model_mapper = match &self.model_mapper {
            Some(mapper) => mapper.clone(),
            None => return decision,
        };

        // Attempt to translate the model name
        let available_models = decision.provider.models.clone();
        let client_type = client_info
            .map(|i| i.client_type)
            .unwrap_or(ClientType::Unknown);

        match model_mapper.translate(
            original_model,
            client_type,
            &decision.provider.name,
            &available_models,
        ) {
            Ok(translated_model) => {
                if translated_model != original_model {
                    info!(
                        original_model = %original_model,
                        translated_model = %translated_model,
                        provider = %decision.provider.name,
                        "Model name translated for provider"
                    );
                    decision.model = translated_model;
                }
            }
            Err(e) => {
                debug!(
                    original_model = %original_model,
                    provider = %decision.provider.name,
                    error = %e,
                    "Model translation failed, using original model name"
                );
            }
        }

        decision
    }

    /// Main routing method - Multi-phase routing with pattern matching, optimization, and fallback
    pub async fn route(
        &self,
        request: &ChatRequest,
        hints: &RoutingHints,
    ) -> Result<RoutingDecision> {
        // Store original model name for translation later
        let original_model = request.model.clone();

        // Phase 0: Check for explicit provider specification in model name
        if let Some((provider_name, model_name)) =
            self.parse_explicit_provider_model(&request.model)
        {
            debug!(
                explicit_provider = %provider_name,
                model = %model_name,
                "Phase 0: Using explicit provider specification from model name"
            );
            let decision = self.create_explicit_decision(&provider_name, &model_name)?;
            return Ok(self.translate_model_name(decision, &original_model, None));
        }

        // Extract session ID for context-aware routing
        let _session_id = self
            .session_manager
            .as_ref()
            .map(|session_manager| session_manager.extract_or_create_session_id(request));

        // Phase 1: Priority-based pattern routing (if RoleGraph available)
        if let Some(rolegraph) = &self.rolegraph {
            if let Some(query) = self.extract_query(request) {
                if let Some(pattern_match) = rolegraph.query_routing_priority(&query) {
                    // Validate the provider and model from pattern match
                    let provider_name = &pattern_match.provider;
                    let model = &pattern_match.model;

                    if let Ok(provider) = self.find_provider(provider_name) {
                        if self.validate_model(provider, model).is_ok() {
                            let concept = pattern_match.concept.clone();
                            let priority = pattern_match.priority;
                            let score = pattern_match.score;
                            let rule_id = pattern_match.rule_id.clone();
                            let priority_value = priority.value();

                            let decision = RoutingDecision {
                                provider: provider.clone(),
                                model: model.to_string(),
                                original_model: None,
                                client_type: None,
                                scenario: RoutingScenario::Pattern(concept.clone()),
                                priority,
                                confidence: score,
                                rule_id: Some(rule_id),
                                reason: format!(
                                    "Priority pattern match: {} (priority: {}, score: {})",
                                    concept, priority_value, score
                                ),
                            };
                            return Ok(self.translate_model_name(decision, &original_model, None));
                        }
                    }
                }
            }
        }

        // Phase 2: Session-aware pattern routing (if session manager available)
        if let (Some(session_manager), Some(session_id_hint)) =
            (&self.session_manager, &hints.session_id)
        {
            if let Ok(session) = session_manager.get_or_create_session(session_id_hint).await {
                if let Some(query) = self.extract_query(request) {
                    // Try pattern-based routing with session context
                    if let Some(rolegraph) = &self.rolegraph {
                        if let Some(pattern_match) = rolegraph.query_routing(&query) {
                            if let Ok((provider_name, model_name)) = self
                                .select_provider_with_session_context(
                                    &pattern_match.concept,
                                    &session,
                                )
                                .await
                            {
                                if let Ok(provider) = self.find_provider(&provider_name) {
                                    info!(
                                        concept = %pattern_match.concept,
                                        provider = %provider_name,
                                        model = %model_name,
                                        "Phase 2: Using session-aware pattern routing"
                                    );
                                    let concept = pattern_match.concept.clone();
                                    let score = pattern_match.score;

                                    let decision = RoutingDecision {
                                        provider: provider.clone(),
                                        model: model_name,
                                        original_model: None,
                                        client_type: None,
                                        scenario: RoutingScenario::Pattern(concept.clone()),
                                        priority: Priority::Medium,
                                        confidence: score,
                                        rule_id: Some(format!("rule-{}", concept)),
                                        reason: format!("Session-aware pattern match: {}", concept),
                                    };
                                    return Ok(self.translate_model_name(
                                        decision,
                                        &original_model,
                                        None,
                                    ));
                                }
                            }
                        }
                    }
                }
            }
        }

        // Phase 3: Cost-optimized routing (if cost features available)
        if self.cost_calculator.is_some() && self.budget_manager.is_some() {
            match self
                .get_cost_optimized_decision(request, hints, &original_model)
                .await
            {
                Ok(decision) => return Ok(decision),
                Err(e) => {
                    debug!("Phase 3: Cost optimization failed: {}", e);
                }
            }
        }

        // Phase 4: Performance-optimized routing (if performance features available)
        if self.performance_tester.is_some() && self.performance_database.is_some() {
            match self
                .get_performance_optimized_decision(request, hints, &original_model)
                .await
            {
                Ok(decision) => return Ok(decision),
                Err(e) => {
                    debug!("Phase 4: Performance optimization failed: {}", e);
                }
            }
        }

        // Phase 5: Scenario-based routing (background, think, long_context, etc.)
        let scenario = self.determine_scenario(hints, self.extract_query(request).as_deref());
        let mut decision = self.create_decision_from_scenario(scenario.clone())?;

        // Apply model translation for scenario-based routing
        decision = self.translate_model_name(decision, &original_model, None);

        info!(
            scenario = ?decision.scenario,
            provider = %decision.provider.name,
            model = %decision.model,
            "Phase 5: Using scenario-based routing"
        );

        Ok(decision)
    }

    /// Create routing decision from scenario
    fn create_decision_from_scenario(&self, scenario: RoutingScenario) -> Result<RoutingDecision> {
        let (provider_name, model_name) = self.get_provider_model_for_scenario(&scenario)?;
        let provider = self.find_provider(&provider_name)?;
        self.validate_model(provider, &model_name)?;

        let decision = RoutingDecision {
            provider: provider.clone(),
            model: model_name.clone(),
            original_model: None,
            client_type: None,
            scenario: scenario.clone(),
            priority: Priority::Medium,
            confidence: 0.6,
            rule_id: None,
            reason: format!("Routing decision for scenario: {}", scenario),
        };

        info!(
            provider = %provider_name,
            model = %model_name,
            scenario = ?decision.scenario,
            "Routing decision made"
        );

        Ok(decision)
    }

    /// Extract query text from request for pattern matching
    fn extract_query(&self, request: &ChatRequest) -> Option<String> {
        // Get last user message as the query
        request
            .messages
            .iter()
            .rev()
            .find(|msg| msg.role == "user")
            .and_then(|msg| match &msg.content {
                crate::token_counter::MessageContent::Text(text) => Some(text.clone()),
                crate::token_counter::MessageContent::Array(blocks) => {
                    // Concatenate text blocks
                    let text: String = blocks
                        .iter()
                        .filter_map(|block| match block {
                            crate::token_counter::ContentBlock::Text { text } => Some(text.clone()),
                            _ => None,
                        })
                        .collect::<Vec<_>>()
                        .join(" ");
                    if text.is_empty() {
                        None
                    } else {
                        Some(text)
                    }
                }
                crate::token_counter::MessageContent::Null => None,
            })
    }

    /// Determine routing scenario from hints and RoleGraph patterns
    fn determine_scenario(&self, hints: &RoutingHints, query: Option<&str>) -> RoutingScenario {
        // Priority order (most specific to least specific):

        // 1. Image routing (if images detected and configured)
        if hints.has_images && self.config.router.image.is_some() {
            return RoutingScenario::Image;
        }

        // 2. Web search routing (if web_search tool detected and configured)
        if hints.has_web_search && self.config.router.web_search.is_some() {
            return RoutingScenario::WebSearch;
        }

        // 2b. Guardrail: web-search/tool-heavy requests without explicit web_search route
        // should avoid default Groq routing when possible.
        if hints.has_web_search && self.config.router.web_search.is_none() {
            let default_is_groq = Self::route_targets_provider(&self.config.router.default, "groq");
            if default_is_groq {
                if let Some(think_route) = &self.config.router.think {
                    if !Self::route_targets_provider(think_route, "groq") {
                        info!("Web search request guardrail: rerouting to think scenario to avoid default Groq");
                        return RoutingScenario::Think;
                    }
                }

                if let Some(long_context_route) = &self.config.router.long_context {
                    if !Self::route_targets_provider(long_context_route, "groq") {
                        info!("Web search request guardrail: rerouting to long_context scenario to avoid default Groq");
                        return RoutingScenario::LongContext;
                    }
                }
            }
        }

        // 3. Long context routing (if token count exceeds threshold)
        if hints.token_count >= self.config.router.long_context_threshold
            && self.config.router.long_context.is_some()
        {
            return RoutingScenario::LongContext;
        }

        // 4. RoleGraph pattern-based routing (NEW - check for scenario patterns)
        if let (Some(query_text), Some(rolegraph)) = (query, &self.rolegraph) {
            if let Some(pattern_match) = rolegraph.query_routing(query_text) {
                // Map RoleGraph concepts to routing scenarios
                match pattern_match.concept.as_str() {
                    "think_routing" if self.config.router.think.is_some() => {
                        info!(
                            concept = %pattern_match.concept,
                            score = pattern_match.score,
                            "RoleGraph pattern matched: Intelligent routing"
                        );
                        return RoutingScenario::Think;
                    }
                    "background_routing" if self.config.router.background.is_some() => {
                        info!(
                            concept = %pattern_match.concept,
                            score = pattern_match.score,
                            "RoleGraph pattern matched: Background routing"
                        );
                        return RoutingScenario::Background;
                    }
                    "long_context_routing" if self.config.router.long_context.is_some() => {
                        info!(
                            concept = %pattern_match.concept,
                            score = pattern_match.score,
                            "RoleGraph pattern matched: Long context routing"
                        );
                        return RoutingScenario::LongContext;
                    }
                    "web_search_routing" if self.config.router.web_search.is_some() => {
                        info!(
                            concept = %pattern_match.concept,
                            score = pattern_match.score,
                            "RoleGraph pattern matched: Web search routing"
                        );
                        return RoutingScenario::WebSearch;
                    }
                    "image_routing" if self.config.router.image.is_some() => {
                        info!(
                            concept = %pattern_match.concept,
                            score = pattern_match.score,
                            "RoleGraph pattern matched: Image routing"
                        );
                        return RoutingScenario::Image;
                    }
                    _ => {
                        // Unknown pattern concept, continue to other phases
                        debug!(
                            concept = %pattern_match.concept,
                            "Unknown RoleGraph pattern concept, skipping"
                        );
                    }
                }
            }
        }

        // 5. Check for plan implementation keywords (BEFORE think check to take priority)
        // This distinguishes "plan" (strategic) from "plan implementation" (tactical)
        if let Some(query_text) = query {
            let query_lower = query_text.to_lowercase();
            let has_plan = query_lower.contains("plan");
            let implementation_keywords = [
                "implementation",
                "implement",
                "build",
                "code",
                "develop",
                "write",
                "create",
            ];
            let has_implementation = implementation_keywords
                .iter()
                .any(|kw| query_lower.contains(kw));

            if has_plan && has_implementation && self.config.router.plan_implementation.is_some() {
                info!("Plan implementation detected - routing to smaller model");
                return RoutingScenario::PlanImplementation;
            }
        }

        // 6. Check for think keywords in query (including "plan" for strategic planning)
        if let Some(query_text) = query {
            let think_keywords = ["think", "reason", "analyze deeply", "step by step", "plan"];
            let query_lower = query_text.to_lowercase();
            if think_keywords.iter().any(|kw| query_lower.contains(kw))
                && self.config.router.think.is_some()
            {
                info!("Think keyword detected in query");
                return RoutingScenario::Think;
            }
        }

        // 7. Background routing (if configured and hints indicate background)
        if hints.is_background && self.config.router.background.is_some() {
            return RoutingScenario::Background;
        }

        // 8. Default routing
        RoutingScenario::Default
    }

    /// Parse explicit provider specification from model name
    /// Supports formats: "provider:model" or "provider,model"
    fn parse_explicit_provider_model(&self, model: &str) -> Option<(String, String)> {
        // Try "provider:model" format first
        if let Some((provider, model_name)) = model.split_once(':') {
            return Some((provider.to_string(), model_name.to_string()));
        }

        // Try "provider,model" format
        if let Some((provider, model_name)) = model.split_once(',') {
            return Some((provider.to_string(), model_name.to_string()));
        }

        None
    }

    /// Find provider by name
    fn find_provider(&self, name: &str) -> Result<&Provider> {
        self.config
            .providers
            .iter()
            .find(|p| p.name == name)
            .ok_or_else(|| ProxyError::ConfigError(format!("Provider '{}' not found", name)))
    }

    /// Validate that a model is available on a provider
    fn validate_model(&self, provider: &Provider, model: &str) -> Result<()> {
        // Special case: if provider has "*" in models list, it supports all models
        if provider.models.iter().any(|m| m == "*") {
            return Ok(());
        }

        // Check if model is in provider's model list
        // Support exact match or glob patterns
        for available_model in &provider.models {
            if Self::model_matches_pattern(model, available_model) {
                return Ok(());
            }
        }

        Err(ProxyError::ConfigError(format!(
            "Model '{}' not available on provider '{}'. Available models: {:?}",
            model, provider.name, provider.models
        )))
    }

    /// Check if a model matches an available model pattern
    fn model_matches_pattern(model: &str, pattern: &str) -> bool {
        // Exact match
        if model == pattern {
            return true;
        }

        // Glob pattern support (e.g., "claude-3-5-*" matches "claude-3-5-sonnet")
        if pattern.contains('*') {
            let parts: Vec<&str> = pattern.split('*').collect();
            if parts.len() == 2 {
                let prefix = parts[0];
                let suffix = parts[1];
                return model.starts_with(prefix) && model.ends_with(suffix);
            }
        }

        false
    }

    /// Get provider and model for a routing scenario from config
    fn get_provider_model_for_scenario(
        &self,
        scenario: &RoutingScenario,
    ) -> Result<(String, String)> {
        let routing_spec = match scenario {
            RoutingScenario::Default => &self.config.router.default,
            RoutingScenario::Background => {
                self.config.router.background.as_ref().ok_or_else(|| {
                    ProxyError::ConfigError("Background routing not configured".to_string())
                })?
            }
            RoutingScenario::Think => self.config.router.think.as_ref().ok_or_else(|| {
                ProxyError::ConfigError("Think routing not configured".to_string())
            })?,
            RoutingScenario::PlanImplementation => self
                .config
                .router
                .plan_implementation
                .as_ref()
                .ok_or_else(|| {
                    ProxyError::ConfigError(
                        "Plan implementation routing not configured".to_string(),
                    )
                })?,
            RoutingScenario::LongContext => {
                self.config.router.long_context.as_ref().ok_or_else(|| {
                    ProxyError::ConfigError("Long context routing not configured".to_string())
                })?
            }
            RoutingScenario::WebSearch => {
                self.config.router.web_search.as_ref().ok_or_else(|| {
                    ProxyError::ConfigError("Web search routing not configured".to_string())
                })?
            }
            RoutingScenario::Image => self.config.router.image.as_ref().ok_or_else(|| {
                ProxyError::ConfigError("Image routing not configured".to_string())
            })?,
            _ => &self.config.router.default,
        };

        let primary_target = routing_spec
            .split('|')
            .next()
            .unwrap_or(routing_spec)
            .trim();

        // Parse "provider,model" or "provider:model" format
        if let Some((provider, model)) = primary_target.split_once(',') {
            Ok((provider.to_string(), model.to_string()))
        } else if let Some((provider, model)) = primary_target.split_once(':') {
            Ok((provider.to_string(), model.to_string()))
        } else {
            // Just provider name, use default model
            Ok((primary_target.to_string(), "default".to_string()))
        }
    }

    /// Select provider with session context
    async fn select_provider_with_session_context(
        &self,
        _concept: &str,
        _session: &SessionInfo,
    ) -> Result<(String, String)> {
        // For now, return default provider
        // TODO: Implement session-based provider selection
        let parts: Vec<&str> = self.config.router.default.split(',').collect();
        if parts.len() >= 2 {
            Ok((parts[0].to_string(), parts[1].to_string()))
        } else {
            Err(ProxyError::ConfigError(
                "Invalid default routing format. Expected 'provider,model'".to_string(),
            ))
        }
    }

    /// Get performance-optimized routing decision
    async fn get_performance_optimized_decision(
        &self,
        _request: &ChatRequest,
        hints: &RoutingHints,
        original_model: &str,
    ) -> Result<RoutingDecision> {
        let scenario = self.determine_scenario(hints, None);
        let (provider_name, model_name) = self.get_provider_model_for_scenario(&scenario)?;
        let provider = self.find_provider(&provider_name)?;

        info!(
            provider = %provider_name,
            model = %model_name,
            "Phase 4: Using performance-optimized routing"
        );

        let decision = RoutingDecision {
            provider: provider.clone(),
            model: model_name,
            original_model: None,
            client_type: None,
            scenario: scenario.clone(),
            priority: Priority::Medium,
            confidence: 0.8,
            rule_id: None,
            reason: format!("Performance-optimized routing for scenario: {}", scenario),
        };

        Ok(self.translate_model_name(decision, original_model, None))
    }

    /// Get cost-optimized routing decision
    async fn get_cost_optimized_decision(
        &self,
        _request: &ChatRequest,
        hints: &RoutingHints,
        original_model: &str,
    ) -> Result<RoutingDecision> {
        let scenario = self.determine_scenario(hints, None);
        let (provider_name, model_name) = self.get_provider_model_for_scenario(&scenario)?;
        let provider = self.find_provider(&provider_name)?;

        info!(
            provider = %provider_name,
            model = %model_name,
            "Phase 3: Using cost-optimized routing"
        );

        let decision = RoutingDecision {
            provider: provider.clone(),
            model: model_name,
            original_model: None,
            client_type: None,
            scenario: scenario.clone(),
            priority: Priority::Medium,
            confidence: 0.7,
            rule_id: None,
            reason: format!("Cost-optimized routing for scenario: {}", scenario),
        };

        Ok(self.translate_model_name(decision, original_model, None))
    }

    /// Create routing decision for explicit provider specification
    fn create_explicit_decision(
        &self,
        provider_name: &str,
        model_name: &str,
    ) -> Result<RoutingDecision> {
        let provider = self.find_provider(provider_name)?;
        self.validate_model(provider, model_name)?;

        Ok(RoutingDecision {
            provider: provider.clone(),
            model: model_name.to_string(),
            original_model: None,
            client_type: None,
            scenario: RoutingScenario::Default, // Use Default for explicit specs
            priority: Priority::High,           // Explicit specs get high priority
            confidence: 1.0,                    // Maximum confidence for explicit specs
            rule_id: None,
            reason: "Explicit provider specification".to_string(),
        })
    }

    /// Route with fallback to default on error (3-phase routing)
    pub async fn route_with_fallback(
        &self,
        request: &ChatRequest,
        hints: &RoutingHints,
    ) -> Result<RoutingDecision> {
        // Try optimal routing
        match self.route(request, hints).await {
            Ok(decision) => Ok(decision),
            Err(e) => {
                warn!("Routing failed, falling back to default: {}", e);
                // Fall back to default routing
                let scenario = RoutingScenario::Default;
                let decision = self.create_decision_from_scenario(scenario.clone())?;
                Ok(self.translate_model_name(decision, &request.model, None))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_config_with_plan_impl() -> Arc<ProxyConfig> {
        use crate::config::{Provider, RouterSettings, SecuritySettings};
        use crate::routing::RoutingStrategy;

        let provider = Provider {
            name: "test".to_string(),
            api_base_url: "https://test.com".to_string(),
            api_key: "test".to_string(),
            models: vec!["model1".to_string()],
            transformers: vec![],
        };

        Arc::new(ProxyConfig {
            proxy: crate::config::ProxySettings {
                host: "127.0.0.1".to_string(),
                port: 3000,
                api_key: "test".to_string(),
                timeout_ms: 30000,
            },
            router: RouterSettings {
                default: "test,model1".to_string(),
                background: None,
                think: Some("test,think-model".to_string()),
                plan_implementation: Some("test,impl-model".to_string()),
                long_context: None,
                long_context_threshold: 100000,
                web_search: None,
                image: None,
                model_mappings: vec![],
                model_exclusions: vec![],
                strategy: RoutingStrategy::default(),
            },
            providers: vec![provider],
            security: SecuritySettings::default(),
            management: Default::default(),
            oauth: Default::default(),
            webhooks: Default::default(),
        })
    }

    fn create_test_config_with_think() -> Arc<ProxyConfig> {
        use crate::config::{Provider, RouterSettings, SecuritySettings};
        use crate::routing::RoutingStrategy;

        let provider = Provider {
            name: "test".to_string(),
            api_base_url: "https://test.com".to_string(),
            api_key: "test".to_string(),
            models: vec!["model1".to_string()],
            transformers: vec![],
        };

        Arc::new(ProxyConfig {
            proxy: crate::config::ProxySettings {
                host: "127.0.0.1".to_string(),
                port: 3000,
                api_key: "test".to_string(),
                timeout_ms: 30000,
            },
            router: RouterSettings {
                default: "test,model1".to_string(),
                background: None,
                think: Some("test,think-model".to_string()),
                plan_implementation: None,
                long_context: None,
                long_context_threshold: 100000,
                web_search: None,
                image: None,
                model_mappings: vec![],
                model_exclusions: vec![],
                strategy: RoutingStrategy::default(),
            },
            providers: vec![provider],
            security: SecuritySettings::default(),
            management: Default::default(),
            oauth: Default::default(),
            webhooks: Default::default(),
        })
    }

    fn create_test_config() -> Arc<ProxyConfig> {
        use crate::config::{Provider, RouterSettings, SecuritySettings};
        use crate::routing::RoutingStrategy;

        let provider = Provider {
            name: "test".to_string(),
            api_base_url: "https://test.com".to_string(),
            api_key: "test".to_string(),
            models: vec!["model1".to_string()],
            transformers: vec![],
        };

        Arc::new(ProxyConfig {
            proxy: crate::config::ProxySettings {
                host: "127.0.0.1".to_string(),
                port: 3000,
                api_key: "test".to_string(),
                timeout_ms: 30000,
            },
            router: RouterSettings {
                default: "test,model1".to_string(),
                background: None,
                think: None,
                plan_implementation: None,
                long_context: None,
                long_context_threshold: 100000,
                web_search: None,
                image: None,
                model_mappings: vec![],
                model_exclusions: vec![],
                strategy: RoutingStrategy::default(),
            },
            providers: vec![provider],
            security: SecuritySettings::default(),
            management: Default::default(),
            oauth: Default::default(),
            webhooks: Default::default(),
        })
    }

    #[test]
    fn test_parse_explicit_provider_model_colon() {
        let config = create_test_config();
        let router = RouterAgent::new(config);

        let result = router.parse_explicit_provider_model("openrouter:anthropic/claude-3.5-sonnet");
        assert_eq!(
            result,
            Some((
                "openrouter".to_string(),
                "anthropic/claude-3.5-sonnet".to_string()
            ))
        );
    }

    #[test]
    fn test_parse_explicit_provider_model_comma() {
        let config = create_test_config();
        let router = RouterAgent::new(config);

        let result = router.parse_explicit_provider_model("groq,llama-3.1-8b-instant");
        assert_eq!(
            result,
            Some(("groq".to_string(), "llama-3.1-8b-instant".to_string()))
        );
    }

    #[test]
    fn test_parse_explicit_provider_model_none() {
        let config = create_test_config();
        let router = RouterAgent::new(config);

        let result = router.parse_explicit_provider_model("claude-3-5-sonnet");
        assert_eq!(result, None);
    }

    #[test]
    fn test_model_matches_pattern_exact() {
        assert!(RouterAgent::model_matches_pattern(
            "claude-3-5-sonnet",
            "claude-3-5-sonnet"
        ));
    }

    #[test]
    fn test_model_matches_pattern_glob() {
        assert!(RouterAgent::model_matches_pattern(
            "claude-3-5-sonnet",
            "claude-3-5-*"
        ));
        assert!(RouterAgent::model_matches_pattern(
            "claude-3-5-haiku",
            "claude-3-5-*"
        ));
        assert!(!RouterAgent::model_matches_pattern(
            "claude-3-opus",
            "claude-3-5-*"
        ));
    }

    #[test]
    fn test_routing_decision_new() {
        use crate::config::Provider;

        let provider = Provider {
            name: "test".to_string(),
            api_base_url: "https://test.com".to_string(),
            api_key: "test".to_string(),
            models: vec!["model1".to_string()],
            transformers: vec![],
        };

        let decision = RoutingDecision::new(
            provider.clone(),
            "test-model".to_string(),
            RoutingScenario::Default,
            Priority::Medium,
            0.8,
            "Test decision".to_string(),
        );

        assert_eq!(decision.model, "test-model");
        assert_eq!(decision.scenario, RoutingScenario::Default);
        assert!(decision.original_model.is_none());
        assert!(decision.client_type.is_none());
    }

    #[test]
    fn test_routing_decision_with_client_info() {
        use crate::client_detection::DetectionMethod;
        use crate::config::Provider;

        let provider = Provider {
            name: "test".to_string(),
            api_base_url: "https://test.com".to_string(),
            api_key: "test".to_string(),
            models: vec!["model1".to_string()],
            transformers: vec![],
        };

        let decision = RoutingDecision::new(
            provider.clone(),
            "test-model".to_string(),
            RoutingScenario::Default,
            Priority::Medium,
            0.8,
            "Test decision".to_string(),
        );

        let client_info = ClientInfo::new(ClientType::ClaudeCode, DetectionMethod::HeaderPattern);
        let decision = decision.with_client_info(&client_info, "original-model".to_string());

        assert_eq!(decision.original_model, Some("original-model".to_string()));
        assert_eq!(decision.client_type, Some(ClientType::ClaudeCode));
    }

    #[test]
    fn test_router_agent_with_model_mapper() {
        let config = create_test_config();
        let router = RouterAgent::with_model_mapper(config);

        assert!(router.model_mapper.is_some());
    }

    #[test]
    fn test_router_agent_without_model_mapper() {
        let config = create_test_config();
        let router = RouterAgent::new(config);

        assert!(router.model_mapper.is_none());
    }

    #[test]
    fn test_priority_new() {
        assert_eq!(Priority::new(10), Priority::Low);
        assert_eq!(Priority::new(50), Priority::Medium);
        assert_eq!(Priority::new(80), Priority::High);
        assert_eq!(Priority::new(100), Priority::Critical);
        assert_eq!(Priority::new(150), Priority::Critical); // Above 100 = Critical
    }

    #[test]
    fn test_priority_value() {
        assert_eq!(Priority::Low.value(), 1);
        assert_eq!(Priority::Medium.value(), 2);
        assert_eq!(Priority::High.value(), 3);
        assert_eq!(Priority::Critical.value(), 4);
    }

    #[test]
    fn test_routing_scenario_display() {
        assert_eq!(RoutingScenario::Default.to_string(), "default");
        assert_eq!(RoutingScenario::Think.to_string(), "think");
        assert_eq!(
            RoutingScenario::Pattern("test".to_string()).to_string(),
            "pattern(test)"
        );
        assert_eq!(
            RoutingScenario::PlanImplementation.to_string(),
            "plan_implementation"
        );
    }

    #[test]
    fn test_plan_keyword_triggers_think_routing() {
        use crate::analyzer::RoutingHints;

        let config = create_test_config_with_think();
        let router = RouterAgent::new(config);

        let hints = RoutingHints {
            token_count: 100,
            is_background: false,
            has_thinking: false,
            has_web_search: false,
            has_images: false,
            session_id: None,
        };

        // Query with "plan" should trigger think routing
        let scenario = router.determine_scenario(&hints, Some("plan this architecture"));
        assert_eq!(scenario, RoutingScenario::Think);

        // Query with "plan" and "design" should trigger think
        let scenario = router.determine_scenario(&hints, Some("create a plan for the system"));
        assert_eq!(scenario, RoutingScenario::Think);
    }

    #[test]
    fn test_plan_implementation_detection() {
        use crate::analyzer::RoutingHints;

        let config = create_test_config_with_plan_impl();
        let router = RouterAgent::new(config);

        let hints = RoutingHints {
            token_count: 100,
            is_background: false,
            has_thinking: false,
            has_web_search: false,
            has_images: false,
            session_id: None,
        };

        // "plan implementation" should trigger PlanImplementation
        let scenario = router.determine_scenario(&hints, Some("plan implementation of the API"));
        assert_eq!(scenario, RoutingScenario::PlanImplementation);

        // "plan to build" should trigger PlanImplementation
        let scenario = router.determine_scenario(&hints, Some("plan to build the feature"));
        assert_eq!(scenario, RoutingScenario::PlanImplementation);

        // "plan" alone should trigger Think
        let scenario = router.determine_scenario(&hints, Some("plan the architecture"));
        assert_eq!(scenario, RoutingScenario::Think);
    }

    #[test]
    fn test_web_search_guardrail_prefers_non_groq_think_when_web_search_unset() {
        use crate::analyzer::RoutingHints;
        use crate::config::{Provider, RouterSettings, SecuritySettings};
        use crate::routing::RoutingStrategy;

        let providers = vec![
            Provider {
                name: "groq".to_string(),
                api_base_url: "https://api.groq.com/openai/v1".to_string(),
                api_key: "test".to_string(),
                models: vec!["llama-3.3-70b-versatile".to_string()],
                transformers: vec![],
            },
            Provider {
                name: "openrouter".to_string(),
                api_base_url: "https://openrouter.ai/api/v1".to_string(),
                api_key: "test".to_string(),
                models: vec!["anthropic/claude-3.5-sonnet".to_string()],
                transformers: vec![],
            },
        ];

        let config = Arc::new(ProxyConfig {
            proxy: crate::config::ProxySettings {
                host: "127.0.0.1".to_string(),
                port: 3000,
                api_key: "test".to_string(),
                timeout_ms: 30000,
            },
            router: RouterSettings {
                default: "groq,llama-3.3-70b-versatile".to_string(),
                background: None,
                think: Some("openrouter,anthropic/claude-3.5-sonnet".to_string()),
                plan_implementation: None,
                long_context: None,
                long_context_threshold: 100000,
                web_search: None,
                image: None,
                model_mappings: vec![],
                model_exclusions: vec![],
                strategy: RoutingStrategy::default(),
            },
            providers,
            security: SecuritySettings::default(),
            management: Default::default(),
            oauth: Default::default(),
            webhooks: Default::default(),
        });

        let router = RouterAgent::new(config);
        let hints = RoutingHints {
            token_count: 80000,
            is_background: false,
            has_thinking: false,
            has_web_search: true,
            has_images: false,
            session_id: None,
        };

        let scenario = router.determine_scenario(&hints, Some("search latest updates"));
        assert_eq!(scenario, RoutingScenario::Think);
    }

    #[test]
    fn test_web_search_guardrail_not_applied_when_default_not_groq() {
        use crate::analyzer::RoutingHints;
        use crate::config::{Provider, RouterSettings, SecuritySettings};
        use crate::routing::RoutingStrategy;

        let providers = vec![Provider {
            name: "openrouter".to_string(),
            api_base_url: "https://openrouter.ai/api/v1".to_string(),
            api_key: "test".to_string(),
            models: vec!["anthropic/claude-3.5-sonnet".to_string()],
            transformers: vec![],
        }];

        let config = Arc::new(ProxyConfig {
            proxy: crate::config::ProxySettings {
                host: "127.0.0.1".to_string(),
                port: 3000,
                api_key: "test".to_string(),
                timeout_ms: 30000,
            },
            router: RouterSettings {
                default: "openrouter,anthropic/claude-3.5-sonnet".to_string(),
                background: None,
                think: Some("openrouter,anthropic/claude-3.5-sonnet".to_string()),
                plan_implementation: None,
                long_context: None,
                long_context_threshold: 100000,
                web_search: None,
                image: None,
                model_mappings: vec![],
                model_exclusions: vec![],
                strategy: RoutingStrategy::default(),
            },
            providers,
            security: SecuritySettings::default(),
            management: Default::default(),
            oauth: Default::default(),
            webhooks: Default::default(),
        });

        let router = RouterAgent::new(config);
        let hints = RoutingHints {
            token_count: 80000,
            is_background: false,
            has_thinking: false,
            has_web_search: true,
            has_images: false,
            session_id: None,
        };

        let scenario = router.determine_scenario(&hints, Some("search latest updates"));
        assert_eq!(scenario, RoutingScenario::Default);
    }
}
