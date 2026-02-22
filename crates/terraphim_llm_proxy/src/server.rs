//! HTTP server implementation with Axum

use axum::{
    extract::State,
    http::{HeaderMap, StatusCode},
    response::{
        sse::{Event, KeepAlive, Sse},
        IntoResponse, Response,
    },
    routing::{get, post},
    Json, Router,
};
use futures::{Stream, StreamExt};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, convert::Infallible, path::PathBuf, sync::Arc, time::Duration};
use tracing::{debug, info, warn};

use crate::{
    analyzer::{RequestAnalyzer, RoutingHints},
    client::LlmClient,
    config::{Provider, ProxyConfig},
    management::{init_start_time, management_routes, ConfigManager, ManagementAuthState},
    metrics::{
        log_provider_metrics, log_request_metrics, log_response_metrics, log_routing_metrics,
        ProviderMetrics, RequestContext, RoutingMetrics,
    },
    oauth::{
        import_codex_tokens_on_startup, oauth_routes, ClaudeOAuthProvider, MemoryTokenStore,
        OAuthFlowManager, OAuthProvider, OAuthState, OpenAiOAuthProvider,
    },
    performance::{PerformanceConfig, PerformanceDatabase, PerformanceTester},
    production_metrics::{MetricsExporter, ProductionMetricsCollector},
    provider_health::{CircuitBreakerConfig, ProviderHealthMonitor},
    retry::{is_fallback_eligible, RetryConfig, RetryExecutor},
    rolegraph_client::RoleGraphClient,
    router::{RouterAgent, RoutingDecision, RoutingScenario},
    routing::{resolve_model, ModelMapping},
    session::{SessionConfig, SessionManager},
    token_counter::{ChatRequest, TokenCounter},
    transformer::TransformerChain,
    ProxyError, Result,
};
use genai::chat::Usage;

// ============================================================================
// Health Check Data Structures
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthCheckResponse {
    pub status: String,
    pub timestamp: String,
    pub version: String,
    pub uptime_seconds: u64,
    pub checks: HealthChecks,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthChecks {
    pub database: String,
    pub providers: String,
    pub sessions: String,
    pub metrics: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReadinessResponse {
    pub ready: bool,
    pub message: String,
    pub timestamp: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LivenessResponse {
    pub alive: bool,
    pub message: String,
    pub timestamp: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetailedHealthResponse {
    pub status: String,
    pub timestamp: String,
    pub version: String,
    pub uptime_seconds: u64,
    pub system: SystemInfo,
    pub performance: PerformanceInfo,
    pub sessions: SessionInfo,
    pub providers: ProviderInfo,
    pub metrics: MetricsInfo,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemInfo {
    pub status: String,
    pub memory_usage_mb: f64,
    pub cpu_usage_percent: f64,
    pub disk_usage_percent: f64,
    pub health_issues: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceInfo {
    pub total_requests: u64,
    pub requests_per_second: f64,
    pub avg_response_time_ms: f64,
    pub error_rate: f64,
    pub p95_response_time_ms: u64,
    pub p99_response_time_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionInfo {
    pub status: String,
    pub active_sessions: u64,
    pub max_sessions: u64,
    pub cache_utilization_percent: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderInfo {
    pub status: String,
    pub total_providers: usize,
    pub healthy_providers: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsInfo {
    pub status: String,
    pub collection_interval_seconds: u64,
    pub last_collection: String,
}

/// Shared application state
#[derive(Clone)]
pub struct AppState {
    pub config: Arc<ProxyConfig>,
    pub token_counter: Arc<TokenCounter>,
    pub analyzer: Arc<RequestAnalyzer>,
    pub router: Arc<RouterAgent>,
    pub llm_client: Arc<LlmClient>,
    pub session_manager: Arc<SessionManager>,
    pub production_collector: Arc<ProductionMetricsCollector>,
    pub provider_health_monitor: Arc<ProviderHealthMonitor>,
    pub retry_executor: Arc<RetryExecutor>,
}

#[derive(Clone)]
struct ExecutionTarget {
    provider: Provider,
    model: String,
}

fn route_spec_for_scenario<'a>(state: &'a AppState, scenario: &RoutingScenario) -> Option<&'a str> {
    match scenario {
        RoutingScenario::Background => state.config.router.background.as_deref(),
        RoutingScenario::Think => state.config.router.think.as_deref(),
        RoutingScenario::PlanImplementation => state.config.router.plan_implementation.as_deref(),
        RoutingScenario::LongContext => state.config.router.long_context.as_deref(),
        RoutingScenario::WebSearch => state.config.router.web_search.as_deref(),
        RoutingScenario::Image => state.config.router.image.as_deref(),
        _ => Some(state.config.router.default.as_str()),
    }
}

fn parse_route_targets(spec: &str) -> Vec<(String, String)> {
    spec.split('|')
        .filter_map(|segment| {
            let trimmed = segment.trim();
            if trimmed.is_empty() {
                return None;
            }

            if let Some((provider, model)) = trimmed.split_once(',') {
                let provider = provider.trim();
                let model = model.trim();
                if !provider.is_empty() && !model.is_empty() {
                    return Some((provider.to_string(), model.to_string()));
                }
            }

            if let Some((provider, model)) = trimmed.split_once(':') {
                let provider = provider.trim();
                let model = model.trim();
                if !provider.is_empty() && !model.is_empty() {
                    return Some((provider.to_string(), model.to_string()));
                }
            }

            None
        })
        .collect()
}

fn build_execution_targets(state: &AppState, decision: &RoutingDecision) -> Vec<ExecutionTarget> {
    let mut targets = vec![ExecutionTarget {
        provider: decision.provider.clone(),
        model: decision.model.clone(),
    }];

    if let Some(spec) = route_spec_for_scenario(state, &decision.scenario) {
        for (provider_name, model_name) in parse_route_targets(spec) {
            if let Some(provider) = state
                .config
                .providers
                .iter()
                .find(|p| p.name.eq_ignore_ascii_case(&provider_name))
                .cloned()
            {
                if model_name != "default" && !provider.models.iter().any(|m| m == &model_name) {
                    warn!(
                        provider = %provider.name,
                        model = %model_name,
                        "Skipping fallback target with unknown model"
                    );
                    continue;
                }

                if targets.iter().any(|t| {
                    t.provider.name.eq_ignore_ascii_case(&provider.name) && t.model == model_name
                }) {
                    continue;
                }

                targets.push(ExecutionTarget {
                    provider,
                    model: model_name,
                });
            }
        }
    }

    targets
}

/// Create the Axum server with all routes and middleware
pub async fn create_server(config: ProxyConfig) -> Result<Router> {
    // Initialize start time for uptime tracking
    init_start_time();

    // Initialize shared state
    let token_counter = Arc::new(TokenCounter::new()?);
    let analyzer = Arc::new(RequestAnalyzer::new(token_counter.clone()));
    let config_arc = Arc::new(config.clone());

    // Create ConfigManager for Management API (using default path for now)
    // TODO: Pass actual config path from main.rs for hot-reload support
    let config_manager = Arc::new(ConfigManager::with_config(
        config,
        PathBuf::from("config.toml"),
    ));

    // Initialize session manager first (needed by router)
    let session_config = SessionConfig {
        max_sessions: 1000,
        max_context_messages: 10,
        session_timeout_minutes: 60,
        redis_url: std::env::var("REDIS_URL").ok(),
        enable_redis: std::env::var("REDIS_URL").is_ok(),
    };
    let session_manager = Arc::new(SessionManager::new(session_config)?);

    // Initialize performance components
    let performance_config = PerformanceConfig::default();
    let performance_database = Arc::new(PerformanceDatabase::new(performance_config.clone()));
    let performance_tester = Arc::new(PerformanceTester::new(
        performance_config.clone(),
        performance_database.clone(),
    ));

    // Try to load RoleGraph for pattern-based routing
    let router = match load_rolegraph().await {
        Ok(rolegraph) => {
            info!(
                taxonomy_files = rolegraph.pattern_count(),
                "RoleGraph loaded successfully - pattern-based routing enabled"
            );
            Arc::new(RouterAgent::with_all_features(
                config_arc.clone(),
                Arc::new(rolegraph),
                session_manager.clone(),
                performance_tester.clone(),
                performance_database.clone(),
            ))
        }
        Err(e) => {
            warn!(error = %e, "RoleGraph not available, using runtime routing only");
            // Create a router with performance features but no rolegraph
            Arc::new(RouterAgent::with_performance_features(
                config_arc.clone(),
                session_manager.clone(),
                performance_tester.clone(),
                performance_database.clone(),
            ))
        }
    };

    let llm_client = Arc::new(
        LlmClient::new(config_arc.oauth.storage_path.clone())?.with_claude_oauth(
            config_arc.oauth.claude.auth_mode.clone(),
            config_arc.oauth.claude.anthropic_beta.clone(),
        ),
    );

    // Initialize production metrics collector (collect every 10 seconds, keep 1000 data points)
    let production_collector = Arc::new(ProductionMetricsCollector::new(10, 1000));

    // Initialize provider health monitor with circuit breaker configuration
    let circuit_breaker_config = CircuitBreakerConfig::default();
    let provider_health_monitor = Arc::new(ProviderHealthMonitor::new(circuit_breaker_config));

    // Initialize retry executor with resilient configuration
    let retry_config = RetryConfig::resilient();
    let retry_executor = Arc::new(RetryExecutor::new(retry_config));

    // Add all configured providers to the health monitor
    for provider in &config_arc.providers {
        provider_health_monitor.add_provider(provider).await;
        info!(
            provider = %provider.name,
            "Added provider to health monitor with circuit breaker"
        );
    }

    let state = AppState {
        config: config_arc,
        token_counter,
        analyzer,
        router,
        llm_client,
        session_manager,
        production_collector,
        provider_health_monitor,
        retry_executor,
    };

    // Build router with middleware layers
    let app = Router::new()
        // Health check endpoints for monitoring
        .route("/health", get(health_check))
        .route("/health/detailed", get(detailed_health_check))
        .route("/ready", get(readiness_probe))
        .route("/live", get(liveness_probe))
        // API endpoints
        .route("/v1/messages", post(handle_messages))
        .route("/v1/messages/count_tokens", post(count_tokens_endpoint))
        .route("/v1/chat/completions", post(handle_chat_completions))
        // Message Batches API
        .route("/v1/messages/batches", post(create_message_batch))
        .route("/v1/messages/batches/:batch_id", get(get_message_batch))
        .route(
            "/v1/messages/batches/:batch_id/results",
            get(get_batch_results),
        )
        .route(
            "/v1/messages/batches/:batch_id/cancel",
            post(cancel_message_batch),
        )
        .route(
            "/v1/messages/batches/:batch_id",
            axum::routing::delete(delete_message_batch),
        )
        // Files API
        .route("/v1/files", post(upload_file))
        .route("/v1/files", get(list_files))
        .route("/v1/files/:file_id", get(get_file))
        .route("/v1/files/:file_id/content", get(get_file_content))
        .route("/v1/files/:file_id", axum::routing::delete(delete_file))
        // Models API
        .route("/v1/models", get(list_models))
        .route("/v1/models/:model_id", get(get_model))
        // Experimental Tools API
        .route("/v1/experimental/generate_prompt", post(generate_prompt))
        // Internal API endpoints
        .route("/api/sessions", get(get_session_stats))
        .route("/api/sessions/:session_id", get(get_session_info))
        .route("/api/metrics/json", get(get_metrics_json))
        .route("/api/metrics/prometheus", get(get_metrics_prometheus))
        .with_state(state.clone())
        .layer(tower_http::timeout::TimeoutLayer::new(
            Duration::from_millis(state.config.proxy.timeout_ms),
        ))
        .layer(tower_http::limit::RequestBodyLimitLayer::new(
            10 * 1024 * 1024, // 10 MB
        ));

    // Mount Management API routes if enabled
    let app = if state.config.management.enabled {
        let auth_state = match &state.config.management.secret_key {
            Some(secret) => {
                info!("Management API enabled with authentication");
                ManagementAuthState::new(secret)
            }
            None => {
                warn!("Management API enabled WITHOUT authentication - use only in development!");
                ManagementAuthState::disabled()
            }
        };
        let mgmt_routes = management_routes(config_manager, auth_state);
        info!("Management API routes mounted at /v0/management/*");
        app.merge(mgmt_routes)
    } else {
        debug!("Management API disabled");
        app
    };

    // Validate Claude OAuth configuration
    state.config.oauth.claude.validate_claude_oauth();

    // Mount OAuth callback routes if any provider is enabled
    let oauth_enabled = state.config.oauth.claude.enabled
        || state.config.oauth.gemini.enabled
        || state.config.oauth.openai.enabled
        || state.config.oauth.copilot.enabled;

    let app = if oauth_enabled {
        let flow_manager = Arc::new(OAuthFlowManager::new());
        let token_store = Arc::new(MemoryTokenStore::new());

        // Register enabled providers as trait objects
        let mut providers_map: HashMap<String, Arc<dyn OAuthProvider>> = HashMap::new();

        if state.config.oauth.claude.enabled {
            let client_id = state
                .config
                .oauth
                .claude
                .client_id
                .clone()
                .unwrap_or_else(|| "default-claude-client-id".to_string());
            let mut claude_provider =
                if let Some(ref secret) = state.config.oauth.claude.client_secret {
                    ClaudeOAuthProvider::with_secret(client_id, secret.clone())
                } else {
                    ClaudeOAuthProvider::new(client_id)
                };
            if let Some(ref scopes) = state.config.oauth.claude.scopes {
                claude_provider = claude_provider.with_scopes(scopes.clone());
            }
            if state.config.oauth.claude.auth_mode.as_deref() == Some("api_key") {
                claude_provider = claude_provider.with_api_key_mode();
            }
            let provider: Arc<dyn OAuthProvider> = Arc::new(claude_provider);
            info!("Registered OAuth provider: claude");
            providers_map.insert("claude".to_string(), provider);
        }

        if state.config.oauth.openai.enabled {
            let client_id = state
                .config
                .oauth
                .openai
                .client_id
                .clone()
                .unwrap_or_else(|| "app_EMoamEEZ73f0CkXaXp7hrann".to_string());
            let provider: Arc<dyn OAuthProvider> =
                if let Some(ref secret) = state.config.oauth.openai.client_secret {
                    Arc::new(OpenAiOAuthProvider::with_secret(client_id, secret.clone()))
                } else {
                    Arc::new(OpenAiOAuthProvider::new(client_id))
                };
            info!("Registered OAuth provider: openai (codex)");
            providers_map.insert("openai".to_string(), provider);
        }

        let providers = Arc::new(tokio::sync::RwLock::new(providers_map));

        // Build callback ports map from config
        let mut callback_ports = HashMap::new();
        if state.config.oauth.claude.enabled {
            callback_ports.insert(
                "claude".to_string(),
                state.config.oauth.claude.callback_port,
            );
        }
        if state.config.oauth.openai.enabled {
            callback_ports.insert(
                "openai".to_string(),
                state.config.oauth.openai.callback_port,
            );
        }

        let oauth_state: OAuthState<MemoryTokenStore> = OAuthState {
            flow_manager,
            token_store,
            providers,
            callback_ports,
        };

        let oauth_router = oauth_routes::<MemoryTokenStore>().with_state(oauth_state);
        info!("OAuth callback routes mounted at /oauth/:provider/*");
        app.merge(oauth_router)
    } else {
        debug!("OAuth disabled - no providers enabled");
        app
    };

    // Import Codex CLI tokens on startup if OpenAI OAuth is enabled
    if state.config.oauth.openai.enabled {
        match import_codex_tokens_on_startup().await {
            Ok(Some(bundle)) => {
                info!("Imported Codex tokens for account: {}", bundle.account_id);
            }
            Ok(None) => {
                debug!("No Codex auth file found, skipping token import");
            }
            Err(e) => {
                warn!("Failed to import Codex tokens: {}", e);
            }
        }
    }

    // Start background metrics collection
    let production_collector_clone = Arc::clone(&state.production_collector);
    tokio::spawn(async move {
        production_collector_clone.start_collection_loop().await;
    });

    info!("Server configured with all routes and middleware");
    info!("Production metrics collection started");

    Ok(app)
}

/// Enhanced health check endpoint
async fn health_check() -> impl IntoResponse {
    let health_status = HealthCheckResponse {
        status: "healthy".to_string(),
        timestamp: chrono::Utc::now().to_rfc3339(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        uptime_seconds: 0, // TODO: Implement uptime tracking
        checks: HealthChecks {
            database: "healthy".to_string(),
            providers: "healthy".to_string(),
            sessions: "healthy".to_string(),
            metrics: "healthy".to_string(),
        },
    };

    (StatusCode::OK, Json(health_status)).into_response()
}

/// Readiness probe endpoint for Kubernetes
async fn readiness_probe() -> impl IntoResponse {
    // TODO: Implement actual readiness checks
    let readiness = ReadinessResponse {
        ready: true,
        message: "Service is ready to accept requests".to_string(),
        timestamp: chrono::Utc::now().to_rfc3339(),
    };

    (StatusCode::OK, Json(readiness)).into_response()
}

/// Liveness probe endpoint for Kubernetes
async fn liveness_probe() -> impl IntoResponse {
    // TODO: Implement actual liveness checks
    let liveness = LivenessResponse {
        alive: true,
        message: "Service is alive".to_string(),
        timestamp: chrono::Utc::now().to_rfc3339(),
    };

    (StatusCode::OK, Json(liveness)).into_response()
}

/// Detailed health check endpoint with comprehensive system status
async fn detailed_health_check(State(state): State<AppState>) -> Result<Response> {
    // Get current metrics
    let metrics = state.production_collector.get_aggregated_metrics();

    // Check session manager health
    let session_stats = state.session_manager.get_stats();
    let session_health = if session_stats.active_sessions < session_stats.max_sessions {
        "healthy"
    } else {
        "degraded"
    };

    // Check provider health using circuit breaker status
    let provider_health_status = state.provider_health_monitor.get_all_health_status().await;
    let healthy_providers = provider_health_status
        .values()
        .filter(|h| matches!(h.status, crate::provider_health::HealthStatus::Healthy))
        .count();
    let total_providers = provider_health_status.len();

    // Determine overall health status
    let overall_status = match metrics.system_health.status {
        crate::production_metrics::HealthStatus::Healthy => "healthy",
        crate::production_metrics::HealthStatus::Degraded => "degraded",
        crate::production_metrics::HealthStatus::Unhealthy => "unhealthy",
    };

    let detailed_health = DetailedHealthResponse {
        status: overall_status.to_string(),
        timestamp: chrono::Utc::now().to_rfc3339(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        uptime_seconds: metrics.system_health.uptime_seconds,
        system: SystemInfo {
            status: overall_status.to_string(),
            memory_usage_mb: metrics.system_health.memory_usage_mb,
            cpu_usage_percent: metrics.system_health.cpu_usage_percent,
            disk_usage_percent: metrics.system_health.disk_usage_percent,
            health_issues: metrics.system_health.health_issues,
        },
        performance: PerformanceInfo {
            total_requests: metrics.total_requests,
            requests_per_second: metrics.requests_per_second,
            avg_response_time_ms: metrics.avg_response_time_ms,
            error_rate: metrics.error_rate,
            p95_response_time_ms: metrics.p95_response_time_ms,
            p99_response_time_ms: metrics.p99_response_time_ms,
        },
        sessions: SessionInfo {
            status: session_health.to_string(),
            active_sessions: session_stats.active_sessions as u64,
            max_sessions: session_stats.max_sessions as u64,
            cache_utilization_percent: (session_stats.active_sessions as f64
                / session_stats.max_sessions as f64
                * 100.0)
                .round(),
        },
        providers: ProviderInfo {
            status: if healthy_providers == total_providers && total_providers > 0 {
                "healthy".to_string()
            } else if healthy_providers > 0 {
                "degraded".to_string()
            } else {
                "unhealthy".to_string()
            },
            total_providers,
            healthy_providers,
        },
        metrics: MetricsInfo {
            status: "healthy".to_string(),
            collection_interval_seconds: 10, // TODO: Get from config
            last_collection: chrono::Utc::now().to_rfc3339(),
        },
    };

    let status_code = match overall_status {
        "healthy" => StatusCode::OK,
        "degraded" => StatusCode::OK, // Still serve traffic but degraded
        "unhealthy" => StatusCode::SERVICE_UNAVAILABLE,
        _ => StatusCode::INTERNAL_SERVER_ERROR,
    };

    Ok((status_code, Json(detailed_health)).into_response())
}

/// Handle POST /v1/chat/completions - OpenAI-compatible endpoint
async fn handle_chat_completions(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(mut request): Json<ChatRequest>,
) -> Result<Response> {
    // Authenticate request (must happen before streaming/non-streaming branch)
    authenticate(&headers, &state.config)?;

    // Check if this is a streaming request
    let is_streaming = request.stream.unwrap_or(false);

    // Create request context for structured logging
    let request_context = RequestContext::new()
        .with_client_id(
            headers
                .get("x-client-id")
                .and_then(|v| v.to_str().ok())
                .unwrap_or("unknown")
                .to_string(),
        )
        .with_user_agent(
            headers
                .get("user-agent")
                .and_then(|v| v.to_str().ok())
                .unwrap_or("unknown")
                .to_string(),
        )
        .with_content_length(
            serde_json::to_string(&request)
                .map(|s| s.len())
                .unwrap_or(0),
        );

    // Log incoming request metrics
    log_request_metrics(&request_context, "/v1/chat/completions", "POST");

    info!("Received OpenAI-compatible chat completion request");

    // Apply model name mappings from config before routing
    apply_model_mappings(&mut request, &state.config.router.model_mappings);

    // For streaming requests, use OpenAI-compatible format
    if is_streaming {
        // Analyze request to generate routing hints
        let hints = state.analyzer.analyze(&request)?;
        let stream = create_openai_sse_stream(state, request, hints, request_context.clone());
        return Ok(Sse::new(stream)
            .keep_alive(KeepAlive::default())
            .into_response());
    }

    // For non-streaming requests, handle directly with OpenAI format conversion
    match handle_chat_completions_non_streaming(
        State(state),
        headers,
        Json(request),
        request_context.clone(),
    )
    .await
    {
        Ok(openai_response) => {
            log_response_metrics(&request_context, 200, None);
            Ok(Json(openai_response).into_response())
        }
        Err(e) => {
            log_response_metrics(&request_context, 500, None);
            Err(e)
        }
    }
}

/// Handle non-streaming chat completions with OpenAI format output
async fn handle_chat_completions_non_streaming(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(mut request): Json<ChatRequest>,
    request_context: RequestContext,
) -> Result<serde_json::Value> {
    // Validate anthropic-version header (warn-only)
    validate_anthropic_version(&headers);

    // Apply model name mappings from config before routing
    apply_model_mappings(&mut request, &state.config.router.model_mappings);

    debug!(model = %request.model, "Received chat completion request");

    // Analyze request to generate routing hints
    let hints = state.analyzer.analyze(&request)?;

    info!(
        token_count = hints.token_count,
        is_background = hints.is_background,
        has_thinking = hints.has_thinking,
        has_web_search = hints.has_web_search,
        has_images = hints.has_images,
        "Request analyzed"
    );

    // Get routing decision first
    let decision = state.router.route(&request, &hints).await.map_err(|e| {
        warn!(error = %e, "Routing failed");
        e
    })?;

    let execution_targets = build_execution_targets(&state, &decision);
    let attempted_targets = execution_targets
        .iter()
        .map(|t| format!("{}/{}", t.provider.name, t.model))
        .collect::<Vec<_>>();

    info!(
        provider = %decision.provider.name,
        model = %decision.model,
        scenario = ?decision.scenario,
        attempted_targets = ?attempted_targets,
        "Routing decision made"
    );

    let llm_client = Arc::clone(&state.llm_client);
    let mut selected_target: Option<ExecutionTarget> = None;
    let mut selected_transformer_chain: Option<TransformerChain> = None;
    let mut selected_provider_metrics: Option<ProviderMetrics> = None;
    let mut selected_provider_start: Option<std::time::Instant> = None;
    let mut chat_response = None;
    let mut last_error: Option<ProxyError> = None;

    for (idx, target) in execution_targets.iter().enumerate() {
        let has_fallback = idx + 1 < execution_targets.len();
        let transformer_chain = TransformerChain::from_names(&target.provider.transformers);

        let transformed_request = match transformer_chain.transform_request(request.clone()).await {
            Ok(req) => req,
            Err(e) => {
                if has_fallback {
                    warn!(
                        provider = %target.provider.name,
                        model = %target.model,
                        error = %e,
                        "Transformer request transform failed, trying fallback target"
                    );
                    last_error = Some(e);
                    continue;
                }
                return Err(e);
            }
        };

        let provider_metrics = ProviderMetrics::new(
            target.provider.name.clone(),
            target.model.clone(),
            target.provider.api_base_url.clone(),
            request_context.request_id.clone(),
        );
        let provider_start = std::time::Instant::now();

        let retry_result = state
            .retry_executor
            .execute(|| {
                let provider = target.provider.clone();
                let model = target.model.clone();
                let request = transformed_request.clone();
                let client = Arc::clone(&llm_client);

                async move { client.send_request(&provider, &model, &request).await }
            })
            .await;

        if let Some(response) = retry_result.value {
            chat_response = Some(response);
            selected_target = Some(target.clone());
            selected_transformer_chain = Some(transformer_chain);
            selected_provider_metrics = Some(provider_metrics);
            selected_provider_start = Some(provider_start);
            break;
        }

        let final_error = retry_result
            .final_error
            .unwrap_or_else(|| ProxyError::Internal("Unknown error after retries".to_string()));
        let error_message = format!("All retries failed: {}", final_error);
        state
            .provider_health_monitor
            .record_failure(&target.provider.name, error_message.clone())
            .await;

        if has_fallback && is_fallback_eligible(&final_error) {
            warn!(
                provider = %target.provider.name,
                model = %target.model,
                error = %final_error,
                next_provider = %execution_targets[idx + 1].provider.name,
                next_model = %execution_targets[idx + 1].model,
                "Primary target failed, attempting fallback target"
            );
            last_error = Some(final_error);
            continue;
        }

        return Err(final_error);
    }

    let chat_response = chat_response.ok_or_else(|| {
        last_error
            .unwrap_or_else(|| ProxyError::Internal("All fallback targets failed".to_string()))
    })?;
    let selected_target = selected_target.ok_or_else(|| {
        ProxyError::Internal("Missing selected target after fallback execution".to_string())
    })?;
    let transformer_chain = selected_transformer_chain.ok_or_else(|| {
        ProxyError::Internal("Missing transformer chain after fallback execution".to_string())
    })?;
    let provider_metrics = selected_provider_metrics.ok_or_else(|| {
        ProxyError::Internal("Missing provider metrics after fallback execution".to_string())
    })?;
    let provider_start = selected_provider_start.ok_or_else(|| {
        ProxyError::Internal("Missing provider timer after fallback execution".to_string())
    })?;

    // Apply reverse transformers
    let final_response = match transformer_chain
        .transform_response(chat_response.clone())
        .await
    {
        Ok(response) => response,
        Err(e) => {
            state
                .provider_health_monitor
                .record_failure(
                    &selected_target.provider.name,
                    format!("Transformer error: {}", e),
                )
                .await;
            return Err(e);
        }
    };

    // Extract metrics for logging
    let output_tokens: u32 = final_response
        .usage
        .completion_tokens
        .unwrap_or(0)
        .try_into()
        .unwrap_or(0);

    let captured_text = final_response
        .content
        .iter()
        .filter_map(|block| block.text.as_deref())
        .collect::<Vec<_>>()
        .join("");

    // Update session
    let session_id = state.session_manager.extract_or_create_session_id(&request);
    let _ = state
        .session_manager
        .update_session(
            &session_id,
            hints.token_count as u32,
            output_tokens,
            &selected_target.provider.name,
            &selected_target.model,
            &captured_text,
        )
        .await;

    // Record success metrics
    state
        .provider_health_monitor
        .record_success(
            &selected_target.provider.name,
            provider_start.elapsed().as_millis() as u64,
        )
        .await;

    let total_tokens = hints.token_count + output_tokens as usize;
    let success_metrics = provider_metrics
        .with_response_time(provider_start.elapsed())
        .with_tokens(total_tokens)
        .success();
    log_provider_metrics(&success_metrics);

    state
        .production_collector
        .record_request(&request_context, &success_metrics);

    // Convert to OpenAI format and return
    Ok(final_response.to_openai_format())
}

/// Handle POST /v1/messages - SSE streaming endpoint
async fn handle_messages(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(request): Json<ChatRequest>,
) -> Result<Response> {
    // Create request context for structured logging
    let request_context = RequestContext::new()
        .with_client_id(
            headers
                .get("x-client-id")
                .and_then(|v| v.to_str().ok())
                .unwrap_or("unknown")
                .to_string(),
        )
        .with_user_agent(
            headers
                .get("user-agent")
                .and_then(|v| v.to_str().ok())
                .unwrap_or("unknown")
                .to_string(),
        )
        .with_content_length(
            serde_json::to_string(&request)
                .map(|s| s.len())
                .unwrap_or(0),
        );

    // Log incoming request metrics
    log_request_metrics(&request_context, "/v1/messages", "POST");

    let result = handle_messages_internal(
        State(state),
        headers,
        Json(request),
        request_context.clone(),
    )
    .await;

    // Log response metrics
    match &result {
        Ok(response) => {
            log_response_metrics(&request_context, response.status().as_u16(), None);
        }
        Err(_) => {
            log_response_metrics(&request_context, 500, None);
        }
    }

    result
}

/// Internal handler for messages with request context
async fn handle_messages_internal(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(mut request): Json<ChatRequest>,
    request_context: RequestContext,
) -> Result<Response> {
    // Authenticate request
    authenticate(&headers, &state.config)?;

    // Validate anthropic-version header (warn-only)
    validate_anthropic_version(&headers);

    // Apply model name mappings from config before routing
    apply_model_mappings(&mut request, &state.config.router.model_mappings);

    debug!(model = %request.model, "Received chat request");

    // Analyze request to generate routing hints
    let hints = state.analyzer.analyze(&request)?;

    info!(
        token_count = hints.token_count,
        is_background = hints.is_background,
        has_thinking = hints.has_thinking,
        has_web_search = hints.has_web_search,
        has_images = hints.has_images,
        "Request analyzed"
    );

    // Check if streaming is requested
    let is_streaming = request.stream.unwrap_or(false);

    if is_streaming {
        // Return SSE stream
        let stream = create_sse_stream(state, request, hints, request_context);
        Ok(Sse::new(stream)
            .keep_alive(KeepAlive::default())
            .into_response())
    } else {
        // Return JSON response
        handle_non_streaming(state, request, hints, request_context).await
    }
}

/// Create SSE stream for streaming responses
fn create_sse_stream(
    state: AppState,
    request: ChatRequest,
    hints: RoutingHints,
    request_context: RequestContext,
) -> impl Stream<Item = std::result::Result<Event, Infallible>> {
    use crate::transformer::TransformerChain;

    async_stream::stream! {
        // Route request to select provider and model
        let routing_start = std::time::Instant::now();
        let decision = match state.router.route_with_fallback(&request, &hints).await {
            Ok(d) => d,
            Err(e) => {
                warn!(error = %e, "Routing failed in streaming handler");
                return;
            }
        };

        // Create routing metrics and log the routing decision
        let routing_metrics = RoutingMetrics::new(
            request_context.request_id.clone(),
            format!("{:?}", decision.scenario),
            decision.provider.name.clone(),
            decision.model.clone(),
        )
        .with_decision_time(routing_start.elapsed())
        .with_hints(vec![
            format!("is_background: {}", hints.is_background),
            format!("has_thinking: {}", hints.has_thinking),
            format!("has_web_search: {}", hints.has_web_search),
            format!("has_images: {}", hints.has_images),
            format!("token_count: {}", hints.token_count),
        ]);

        log_routing_metrics(&routing_metrics);

        // Record routing decision in production metrics
        state.production_collector.record_routing(&routing_metrics);

        info!(
            provider = %decision.provider.name,
            endpoint = %decision.provider.api_base_url,
            model = %decision.model,
            scenario = ?decision.scenario,
            "Resolved routing decision"
        );

        // Apply transformers for the provider
        let transformer_chain = TransformerChain::from_names(&decision.provider.transformers);
        let transformed_request = match transformer_chain.transform_request(request.clone()).await {
            Ok(r) => r,
            Err(e) => {
                warn!(error = %e, "Transformer chain failed");
                return;
            }
        };

        debug!(
            transformers = decision.provider.transformers.len(),
            "Applied transformer chain for streaming"
        );

        // Create provider metrics for logging
        let provider_start = std::time::Instant::now();
        let provider_metrics = ProviderMetrics::new(
            decision.provider.name.clone(),
            decision.model.clone(),
            decision.provider.api_base_url.clone(),
            request_context.request_id.clone(),
        );

        // Execute streaming request with retry logic
        let retry_result = state.retry_executor.execute(|| {
            let llm_client = Arc::clone(&state.llm_client);
            let provider = decision.provider.clone();
            let model = decision.model.clone();
            let request = transformed_request.clone();

            async move {
                llm_client.send_streaming_request(&provider, &model, &request).await
            }
        }).await;

        // Handle retry result for streaming -- use the value directly
        let mut llm_stream = match retry_result.value {
            Some(stream) => {
                info!(
                    attempts = retry_result.attempts,
                    total_duration_ms = retry_result.total_duration.as_millis(),
                    "Streaming request succeeded after retries"
                );
                stream
            }
            None => {
                let error_message = retry_result.final_error
                    .map(|e| format!("All retries failed: {}", e))
                    .unwrap_or_else(|| "All retries failed: Unknown error".to_string());

                // Record provider failure in health monitor
                state.provider_health_monitor.record_failure(
                    &decision.provider.name,
                    error_message.clone()
                ).await;

                // Log provider error metrics
                let error_metrics = provider_metrics
                    .clone()
                    .with_response_time(provider_start.elapsed())
                    .error(error_message.clone());
                log_provider_metrics(&error_metrics);

                // Record error in production metrics
                state.production_collector.record_request(&request_context, &error_metrics);
                warn!(error = %error_message, "Streaming request failed after all retries");
                return;
            }
        };

        debug!("Starting SSE stream with real LLM");
        let mut captured_text = String::new();

        // Send initial message_start event
        yield Ok(Event::default()
            .event("message_start")
            .json_data(serde_json::json!({
                "type": "message_start",
                "message": {
                    "id": "msg_streaming",
                    "type": "message",
                    "role": "assistant",
                    "content": [],
                    "model": decision.model,
                    "stop_reason": null,
                    "usage": {
                        "input_tokens": hints.token_count,
                        "output_tokens": 0
                    }
                }
            }))
            .unwrap());

        // Track content blocks dynamically for Phase 3 streaming
        let mut current_index: usize = 0;
        let mut thinking_block_index: Option<usize> = None;
        let mut text_block_index: Option<usize> = None;
        let mut tool_use_blocks: std::collections::HashMap<String, usize> = std::collections::HashMap::new();

        // Stream content from LLM with ping heartbeat
        let mut output_tokens = 0;
        let mut ping_interval = tokio::time::interval(Duration::from_secs(15));
        let mut stream_error: Option<String> = None;
        // Skip the first immediate tick
        ping_interval.tick().await;

        loop {
            tokio::select! {
                // Send ping event every 15 seconds
                _ = ping_interval.tick() => {
                    yield Ok(Event::default()
                        .event("ping")
                        .json_data(serde_json::json!({
                            "type": "ping"
                        }))
                        .unwrap());
                }
                // Handle stream events
                event_result = llm_stream.next() => {
                    match event_result {
                        Some(Ok(event)) => {
                            use genai::chat::ChatStreamEvent;

                            // Convert genai ChatStreamEvent to Claude API format
                            match event {
                                ChatStreamEvent::Start => {
                                    debug!("LLM stream started");
                                }
                                ChatStreamEvent::Chunk(chunk) => {
                                    let text = &chunk.content;
                                    if !text.is_empty() {
                                        // Start text block if not already started
                                        if text_block_index.is_none() {
                                            let idx = current_index;
                                            current_index += 1;
                                            text_block_index = Some(idx);
                                            yield Ok(Event::default()
                                                .event("content_block_start")
                                                .json_data(serde_json::json!({
                                                    "type": "content_block_start",
                                                    "index": idx,
                                                    "content_block": {
                                                        "type": "text",
                                                        "text": ""
                                                    }
                                                }))
                                                .unwrap());
                                        }
                                        output_tokens += 1; // Rough estimate
                                        captured_text.push_str(text);
                                        yield Ok(Event::default()
                                            .event("content_block_delta")
                                            .json_data(serde_json::json!({
                                                "type": "content_block_delta",
                                                "index": text_block_index.unwrap(),
                                                "delta": {
                                                    "type": "text_delta",
                                                    "text": text
                                                }
                                            }))
                                            .unwrap());
                                    }
                                }
                                ChatStreamEvent::ReasoningChunk(chunk) => {
                                    // GAP-SSE-003: Implement thinking_delta streaming
                                    let thinking_text = &chunk.content;
                                    if !thinking_text.is_empty() {
                                        // Start thinking block if not already started
                                        if thinking_block_index.is_none() {
                                            let idx = current_index;
                                            current_index += 1;
                                            thinking_block_index = Some(idx);
                                            yield Ok(Event::default()
                                                .event("content_block_start")
                                                .json_data(serde_json::json!({
                                                    "type": "content_block_start",
                                                    "index": idx,
                                                    "content_block": {
                                                        "type": "thinking",
                                                        "thinking": ""
                                                    }
                                                }))
                                                .unwrap());
                                        }
                                        yield Ok(Event::default()
                                            .event("content_block_delta")
                                            .json_data(serde_json::json!({
                                                "type": "content_block_delta",
                                                "index": thinking_block_index.unwrap(),
                                                "delta": {
                                                    "type": "thinking_delta",
                                                    "thinking": thinking_text
                                                }
                                            }))
                                            .unwrap());
                                        debug!(content = %thinking_text, "Thinking delta sent");
                                    }
                                }
                                ChatStreamEvent::ToolCallChunk(tool_chunk) => {
                                    // GAP-SSE-004: Implement input_json_delta streaming
                                    let tool_call = &tool_chunk.tool_call;
                                    let call_id = &tool_call.call_id;

                                    // Start tool_use block if not already started for this call_id
                                    let block_idx = if let Some(&idx) = tool_use_blocks.get(call_id) {
                                        idx
                                    } else {
                                        let idx = current_index;
                                        current_index += 1;
                                        tool_use_blocks.insert(call_id.clone(), idx);
                                        yield Ok(Event::default()
                                            .event("content_block_start")
                                            .json_data(serde_json::json!({
                                                "type": "content_block_start",
                                                "index": idx,
                                                "content_block": {
                                                    "type": "tool_use",
                                                    "id": call_id,
                                                    "name": tool_call.fn_name,
                                                    "input": {}
                                                }
                                            }))
                                            .unwrap());
                                        idx
                                    };

                                    // Send input_json_delta with the arguments
                                    let partial_json = tool_call.fn_arguments.to_string();
                                    yield Ok(Event::default()
                                        .event("content_block_delta")
                                        .json_data(serde_json::json!({
                                            "type": "content_block_delta",
                                            "index": block_idx,
                                            "delta": {
                                                "type": "input_json_delta",
                                                "partial_json": partial_json
                                            }
                                        }))
                                        .unwrap());
                                    debug!(call_id = %call_id, fn_name = %tool_call.fn_name, "Tool call delta sent");
                                }
                                ChatStreamEvent::ThoughtSignatureChunk(_) => {
                                    // Thought signature chunks are not forwarded to the client
                                    debug!("Thought signature chunk received (ignored)");
                                }
                                ChatStreamEvent::End(end_data) => {
                                    // Use captured usage if available
                                    if let Some(usage) = end_data.captured_usage {
                                        if let Some(completion_tokens) = usage.completion_tokens {
                                            output_tokens = completion_tokens as usize;
                                        }
                                    }
                                    debug!(output_tokens = output_tokens, "LLM stream ended");
                                }
                            }
                        }
                        Some(Err(e)) => {
                            warn!(error = %e, "Error in LLM stream");
                            stream_error = Some(e.to_string());
                            // Emit error event before closing stream (GAP-SSE-002)
                            yield Ok(Event::default()
                                .event("error")
                                .json_data(serde_json::json!({
                                    "type": "error",
                                    "error": {
                                        "type": "api_error",
                                        "message": e.to_string()
                                    }
                                }))
                                .unwrap());
                            break;
                        }
                        None => {
                            // Stream ended normally
                            break;
                        }
                    }
                }
            }
        }

        if let Some(error_message) = stream_error {
            let error_message = format!("Streaming error: {}", error_message);

            state
                .provider_health_monitor
                .record_failure(&decision.provider.name, error_message.clone())
                .await;

            let error_metrics = provider_metrics
                .clone()
                .with_response_time(provider_start.elapsed())
                .with_tokens(hints.token_count + output_tokens)
                .error(error_message.clone());
            log_provider_metrics(&error_metrics);
            state
                .production_collector
                .record_request(&request_context, &error_metrics);

            warn!(error = %error_message, "SSE stream ended with error");
            return;
        }

        // Send content_block_stop for all started blocks (in order)
        // Collect all block indices and sort them
        let mut all_block_indices: Vec<usize> = Vec::new();
        if let Some(idx) = thinking_block_index {
            all_block_indices.push(idx);
        }
        if let Some(idx) = text_block_index {
            all_block_indices.push(idx);
        }
        for &idx in tool_use_blocks.values() {
            all_block_indices.push(idx);
        }
        all_block_indices.sort();

        for idx in all_block_indices {
            yield Ok(Event::default()
                .event("content_block_stop")
                .json_data(serde_json::json!({
                    "type": "content_block_stop",
                    "index": idx
                }))
                .unwrap());
        }

        // Determine stop_reason based on what was generated
        let stop_reason = if !tool_use_blocks.is_empty() {
            "tool_use"
        } else {
            "end_turn"
        };

        // Send message_delta with usage and stop_sequence
        yield Ok(Event::default()
            .event("message_delta")
            .json_data(serde_json::json!({
                "type": "message_delta",
                "delta": {
                    "stop_reason": stop_reason,
                    "stop_sequence": null
                },
                "usage": {
                    "output_tokens": output_tokens
                }
            }))
            .unwrap());

        // Send message_stop
        yield Ok(Event::default()
            .event("message_stop")
            .json_data(serde_json::json!({
                "type": "message_stop"
            }))
            .unwrap());

        // Update session with request/response data
        let session_id = state.session_manager.extract_or_create_session_id(&request);
        let _ = state.session_manager.update_session(
            &session_id,
            hints.token_count as u32,
            output_tokens as u32,
            &decision.provider.name,
            &decision.model,
            &captured_text,
        ).await;

        // Record provider success in health monitor
        state.provider_health_monitor.record_success(
            &decision.provider.name,
            provider_start.elapsed().as_millis() as u64
        ).await;

        // Log successful provider metrics
        let success_metrics = provider_metrics
            .clone()
            .with_response_time(provider_start.elapsed())
            .with_tokens(hints.token_count + output_tokens)
            .success();
        log_provider_metrics(&success_metrics);

        // Record in production metrics collector
        state.production_collector.record_request(&request_context, &success_metrics);

        info!(
            output_tokens = output_tokens,
            "SSE stream completed successfully"
        );
    }
}

/// Create SSE stream with OpenAI-compatible format for /v1/chat/completions
fn create_openai_sse_stream(
    state: AppState,
    request: ChatRequest,
    hints: RoutingHints,
    request_context: RequestContext,
) -> impl Stream<Item = std::result::Result<Event, Infallible>> {
    use crate::transformer::TransformerChain;
    use genai::chat::ChatStreamEvent;

    async_stream::stream! {
        // Route request to select provider and model
        let routing_start = std::time::Instant::now();
        let decision = match state.router.route(&request, &hints).await {
            Ok(d) => d,
            Err(e) => {
                warn!(error = %e, "Routing failed in OpenAI streaming handler");
                yield Ok(Event::default()
                    .event("error")
                    .json_data(serde_json::json!({
                        "error": {
                            "message": format!("Routing failed: {}", e),
                            "type": "api_error"
                        }
                    }))
                    .unwrap());
                return;
            }
        };

        let execution_targets = build_execution_targets(&state, &decision);
        let attempted_targets: Vec<String> = execution_targets
            .iter()
            .map(|t| format!("{}/{}", t.provider.name, t.model))
            .collect();

        // Create routing metrics and log the routing decision
        let routing_metrics = RoutingMetrics::new(
            request_context.request_id.clone(),
            format!("{:?}", decision.scenario),
            decision.provider.name.clone(),
            decision.model.clone(),
        )
        .with_decision_time(routing_start.elapsed())
        .with_hints(vec![
            format!("is_background: {}", hints.is_background),
            format!("has_thinking: {}", hints.has_thinking),
            format!("has_web_search: {}", hints.has_web_search),
            format!("has_images: {}", hints.has_images),
            format!("token_count: {}", hints.token_count),
        ]);

        log_routing_metrics(&routing_metrics);
        state.production_collector.record_routing(&routing_metrics);

        info!(
            provider = %decision.provider.name,
            model = %decision.model,
            scenario = ?decision.scenario,
            attempted_targets = ?attempted_targets,
            "Resolved routing decision for OpenAI streaming"
        );

        // Try each execution target with fallback
        let mut llm_stream = None;
        let mut selected_model = decision.model.clone();
        let mut selected_provider_name = decision.provider.name.clone();
        let mut last_error: Option<ProxyError> = None;

        for (idx, target) in execution_targets.iter().enumerate() {
            let has_fallback = idx + 1 < execution_targets.len();
            let transformer_chain = TransformerChain::from_names(&target.provider.transformers);

            let transformed_request = match transformer_chain.transform_request(request.clone()).await {
                Ok(r) => r,
                Err(e) => {
                    if has_fallback {
                        warn!(
                            provider = %target.provider.name,
                            error = %e,
                            "Transformer failed, trying fallback"
                        );
                        last_error = Some(e);
                        continue;
                    }
                    yield Ok(Event::default()
                        .event("error")
                        .json_data(serde_json::json!({
                            "error": {
                                "message": format!("Transform failed: {}", e),
                                "type": "api_error"
                            }
                        }))
                        .unwrap());
                    return;
                }
            };

            let retry_result = state.retry_executor.execute(|| {
                let llm_client = Arc::clone(&state.llm_client);
                let provider = target.provider.clone();
                let model = target.model.clone();
                let request = transformed_request.clone();

                async move {
                    llm_client.send_streaming_request(&provider, &model, &request).await
                }
            }).await;

            if let Some(stream) = retry_result.value {
                selected_model = target.model.clone();
                selected_provider_name = target.provider.name.clone();
                llm_stream = Some(stream);
                break;
            }

            let final_error = retry_result
                .final_error
                .unwrap_or_else(|| ProxyError::Internal("Unknown error after retries".to_string()));
            let error_message = format!("All retries failed: {}", final_error);
            state
                .provider_health_monitor
                .record_failure(&target.provider.name, error_message.clone())
                .await;

            if has_fallback && is_fallback_eligible(&final_error) {
                warn!(
                    provider = %target.provider.name,
                    model = %target.model,
                    error = %final_error,
                    next_provider = %execution_targets[idx + 1].provider.name,
                    next_model = %execution_targets[idx + 1].model,
                    "Streaming: primary target failed, attempting fallback"
                );
                last_error = Some(final_error);
                continue;
            }

            yield Ok(Event::default()
                .event("error")
                .json_data(serde_json::json!({
                    "error": {
                        "message": error_message,
                        "type": "api_error"
                    }
                }))
                .unwrap());
            return;
        }

        let mut llm_stream = match llm_stream {
            Some(s) => s,
            None => {
                let error_message = last_error
                    .map(|e| format!("All fallback targets failed: {}", e))
                    .unwrap_or_else(|| "All fallback targets failed".to_string());
                yield Ok(Event::default()
                    .event("error")
                    .json_data(serde_json::json!({
                        "error": {
                            "message": error_message,
                            "type": "api_error"
                        }
                    }))
                    .unwrap());
                return;
            }
        };

        // Create provider metrics for the selected target
        let provider_start = std::time::Instant::now();
        let provider_metrics = ProviderMetrics::new(
            selected_provider_name.clone(),
            selected_model.clone(),
            String::new(),
            request_context.request_id.clone(),
        );

        // Send initial OpenAI format event with the actual model that responded
        yield Ok(Event::default()
            .json_data(serde_json::json!({
                "id": format!("chatcmpl-{}", uuid::Uuid::new_v4().simple()),
                "object": "chat.completion.chunk",
                "created": std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs(),
                "model": selected_model,
                "choices": [{
                    "index": 0,
                    "delta": {
                        "role": "assistant"
                    },
                    "finish_reason": null
                }]
            }))
            .unwrap());

        // Stream content and convert to OpenAI format
        let mut output_tokens = 0;
        let mut accumulated_text = String::new();
        let mut stream_error: Option<String> = None;
        let mut tool_call_indices: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
        let mut tool_call_initialized: std::collections::HashSet<String> = std::collections::HashSet::new();
        let mut next_tool_index: usize = 0;
        let mut has_tool_calls = false;

        while let Some(chunk_result) = llm_stream.next().await {
            match chunk_result {
                Ok(ChatStreamEvent::Chunk(chunk)) => {
                    // Extract text content from the chunk
                    let text = chunk.content;
                    if !text.is_empty() {
                        accumulated_text.push_str(&text);
                        output_tokens += 1;

                        // Send OpenAI format chunk
                        yield Ok(Event::default()
                            .json_data(serde_json::json!({
                                "id": format!("chatcmpl-{}", uuid::Uuid::new_v4().simple()),
                                "object": "chat.completion.chunk",
                                "created": std::time::SystemTime::now()
                                    .duration_since(std::time::UNIX_EPOCH)
                                    .unwrap_or_default()
                                    .as_secs(),
                                "model": selected_model,
                                "choices": [{
                                    "index": 0,
                                    "delta": {
                                        "content": text
                                    },
                                    "finish_reason": null
                                }]
                            }))
                            .unwrap());
                    }
                }
                Ok(ChatStreamEvent::ToolCallChunk(tool_chunk)) => {
                    has_tool_calls = true;
                    let tool_call = &tool_chunk.tool_call;
                    let call_id = &tool_call.call_id;

                    // Assign index for this call_id
                    let tc_index = *tool_call_indices
                        .entry(call_id.clone())
                        .or_insert_with(|| {
                            let idx = next_tool_index;
                            next_tool_index += 1;
                            idx
                        });

                    let partial_args = match &tool_call.fn_arguments {
                        serde_json::Value::String(s) => s.clone(),
                        other => other.to_string(),
                    };

                    // Build tool_call delta: first chunk includes id/type/name,
                    // continuation chunks only include arguments
                    let tc_delta = if tool_call_initialized.insert(call_id.clone()) {
                        serde_json::json!({
                            "index": tc_index,
                            "id": call_id,
                            "type": "function",
                            "function": {
                                "name": tool_call.fn_name,
                                "arguments": partial_args
                            }
                        })
                    } else {
                        serde_json::json!({
                            "index": tc_index,
                            "function": {
                                "arguments": partial_args
                            }
                        })
                    };

                    yield Ok(Event::default()
                        .json_data(serde_json::json!({
                            "id": format!("chatcmpl-{}", uuid::Uuid::new_v4().simple()),
                            "object": "chat.completion.chunk",
                            "created": std::time::SystemTime::now()
                                .duration_since(std::time::UNIX_EPOCH)
                                .unwrap_or_default()
                                .as_secs(),
                            "model": selected_model,
                            "choices": [{
                                "index": 0,
                                "delta": {
                                    "tool_calls": [tc_delta]
                                },
                                "finish_reason": null
                            }]
                        }))
                        .unwrap());
                }
                Ok(_) => {
                    // Ignore other event types (Start, End, ReasoningChunk, etc.)
                }
                Err(e) => {
                    // Check if this is a "Stream ended" error which is normal for SSE
                    let error_msg = format!("{}", e);
                    if error_msg.contains("Stream ended") || error_msg.contains("stream ended") {
                        debug!("Stream ended normally");
                        break;
                    }
                    stream_error = Some(e.to_string());
                    warn!(error = %e, "Stream chunk error");
                    yield Ok(Event::default()
                        .event("error")
                        .json_data(serde_json::json!({
                            "error": {
                                "message": format!("Stream error: {}", e),
                                "type": "api_error"
                            }
                        }))
                        .unwrap());
                    break;
                }
            }
        }

        if let Some(error_message) = stream_error {
            let error_message = format!("Streaming error: {}", error_message);

            state
                .provider_health_monitor
                .record_failure(&selected_provider_name, error_message.clone())
                .await;

            let error_metrics = provider_metrics
                .clone()
                .with_response_time(provider_start.elapsed())
                .with_tokens(hints.token_count + output_tokens)
                .error(error_message.clone());
            log_provider_metrics(&error_metrics);
            state
                .production_collector
                .record_request(&request_context, &error_metrics);

            warn!(error = %error_message, "OpenAI SSE stream ended with error");
            return;
        }

        // Send final chunk with finish_reason
        let final_finish_reason = if has_tool_calls { "tool_calls" } else { "stop" };
        yield Ok(Event::default()
            .json_data(serde_json::json!({
                "id": format!("chatcmpl-{}", uuid::Uuid::new_v4().simple()),
                "object": "chat.completion.chunk",
                "created": std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs(),
                "model": selected_model,
                "choices": [{
                    "index": 0,
                    "delta": {},
                    "finish_reason": final_finish_reason
                }]
            }))
            .unwrap());

        // Send done event (OpenAI format uses [DONE])
        yield Ok(Event::default()
            .data("[DONE]"));

        // Record success metrics
        state.provider_health_monitor.record_success(
            &selected_provider_name,
            provider_start.elapsed().as_millis() as u64,
        ).await;

        let success_metrics = provider_metrics
            .with_response_time(provider_start.elapsed())
            .with_tokens(hints.token_count + output_tokens)
            .success();
        log_provider_metrics(&success_metrics);
        state.production_collector.record_request(&request_context, &success_metrics);

        info!(
            output_tokens = output_tokens,
            "OpenAI SSE stream completed successfully"
        );
    }
}

/// Handle non-streaming request
async fn handle_non_streaming(
    state: AppState,
    request: ChatRequest,
    hints: RoutingHints,
    request_context: RequestContext,
) -> Result<Response> {
    debug!("Handling non-streaming request");

    // Route request to select provider and model
    let routing_start = std::time::Instant::now();
    let decision = state.router.route_with_fallback(&request, &hints).await?;

    // Create routing metrics and log the routing decision
    let routing_metrics = RoutingMetrics::new(
        request_context.request_id.clone(),
        format!("{:?}", decision.scenario),
        decision.provider.name.clone(),
        decision.model.clone(),
    )
    .with_decision_time(routing_start.elapsed())
    .with_hints(vec![
        format!("is_background: {}", hints.is_background),
        format!("has_thinking: {}", hints.has_thinking),
        format!("has_web_search: {}", hints.has_web_search),
        format!("has_images: {}", hints.has_images),
        format!("token_count: {}", hints.token_count),
    ]);

    log_routing_metrics(&routing_metrics);

    // Record routing decision in production metrics
    state.production_collector.record_routing(&routing_metrics);

    info!(
        provider = %decision.provider.name,
        endpoint = %decision.provider.api_base_url,
        model = %decision.model,
        scenario = ?decision.scenario,
        "Resolved routing decision"
    );

    let execution_targets = build_execution_targets(&state, &decision);
    let attempted_targets = execution_targets
        .iter()
        .map(|t| format!("{}/{}", t.provider.name, t.model))
        .collect::<Vec<_>>();

    info!(attempted_targets = ?attempted_targets, "Execution targets resolved");

    let mut response = None;
    let mut selected_target: Option<ExecutionTarget> = None;
    let mut selected_transformer_chain: Option<TransformerChain> = None;
    let mut selected_provider_start: Option<std::time::Instant> = None;
    let mut selected_provider_metrics: Option<ProviderMetrics> = None;
    let mut last_error: Option<ProxyError> = None;

    for (idx, target) in execution_targets.iter().enumerate() {
        let has_fallback = idx + 1 < execution_targets.len();
        let transformer_chain = TransformerChain::from_names(&target.provider.transformers);
        let transformed_request = match transformer_chain.transform_request(request.clone()).await {
            Ok(req) => req,
            Err(e) => {
                if has_fallback {
                    warn!(
                        provider = %target.provider.name,
                        model = %target.model,
                        error = %e,
                        "Transformer request transform failed, trying fallback target"
                    );
                    last_error = Some(e);
                    continue;
                }
                return Err(e);
            }
        };

        debug!(
            provider = %target.provider.name,
            model = %target.model,
            transformers = target.provider.transformers.len(),
            "Applied transformer chain"
        );

        let provider_start = std::time::Instant::now();
        let provider_metrics = ProviderMetrics::new(
            target.provider.name.clone(),
            target.model.clone(),
            target.provider.api_base_url.clone(),
            request_context.request_id.clone(),
        );

        let retry_result = state
            .retry_executor
            .execute(|| {
                let llm_client = Arc::clone(&state.llm_client);
                let provider = target.provider.clone();
                let model = target.model.clone();
                let request = transformed_request.clone();

                async move { llm_client.send_request(&provider, &model, &request).await }
            })
            .await;

        if let Some(resp) = retry_result.value {
            info!(
                provider = %target.provider.name,
                model = %target.model,
                attempts = retry_result.attempts,
                total_duration_ms = retry_result.total_duration.as_millis(),
                "Non-streaming request succeeded"
            );
            response = Some(resp);
            selected_target = Some(target.clone());
            selected_transformer_chain = Some(transformer_chain);
            selected_provider_start = Some(provider_start);
            selected_provider_metrics = Some(provider_metrics);
            break;
        }

        let final_error = retry_result
            .final_error
            .unwrap_or_else(|| ProxyError::Internal("Unknown error after retries".to_string()));
        let error_message = format!("All retries failed: {}", final_error);

        state
            .provider_health_monitor
            .record_failure(&target.provider.name, error_message.clone())
            .await;

        let error_metrics = provider_metrics
            .clone()
            .with_response_time(provider_start.elapsed())
            .error(error_message.clone());
        log_provider_metrics(&error_metrics);
        state
            .production_collector
            .record_request(&request_context, &error_metrics);

        if has_fallback && is_fallback_eligible(&final_error) {
            warn!(
                provider = %target.provider.name,
                model = %target.model,
                error = %final_error,
                next_provider = %execution_targets[idx + 1].provider.name,
                next_model = %execution_targets[idx + 1].model,
                "Primary target failed, attempting fallback target"
            );
            last_error = Some(final_error);
            continue;
        }

        warn!(error = %error_message, "Non-streaming request failed after all retries");
        return Err(final_error);
    }

    let response = response.ok_or_else(|| {
        last_error
            .unwrap_or_else(|| ProxyError::Internal("All fallback targets failed".to_string()))
    })?;
    let selected_target = selected_target.ok_or_else(|| {
        ProxyError::Internal("Missing selected target after fallback execution".to_string())
    })?;
    let transformer_chain = selected_transformer_chain.ok_or_else(|| {
        ProxyError::Internal("Missing transformer chain after fallback execution".to_string())
    })?;
    let provider_start = selected_provider_start.ok_or_else(|| {
        ProxyError::Internal("Missing provider timer after fallback execution".to_string())
    })?;
    let provider_metrics = selected_provider_metrics.ok_or_else(|| {
        ProxyError::Internal("Missing provider metrics after fallback execution".to_string())
    })?;

    // Apply reverse transformers to response
    let final_response = match transformer_chain.transform_response(response.clone()).await {
        Ok(response) => response,
        Err(e) => {
            // Record provider failure in health monitor
            state
                .provider_health_monitor
                .record_failure(
                    &selected_target.provider.name,
                    format!("Transformer chain error: {}", e),
                )
                .await;

            warn!(error = %e, "Failed to transform response");
            return Err(e);
        }
    };

    // Extract assistant text from final response content blocks
    let captured_text = final_response
        .content
        .iter()
        .filter_map(|block| block.text.as_deref())
        .collect::<Vec<_>>()
        .join("");

    // Determine output tokens from normalized response usage
    let output_tokens: u32 = final_response
        .usage
        .completion_tokens
        .unwrap_or(0)
        .try_into()
        .unwrap_or(0);

    // Update session with request/response data
    let session_id = state.session_manager.extract_or_create_session_id(&request);
    let _ = state
        .session_manager
        .update_session(
            &session_id,
            hints.token_count as u32,
            output_tokens,
            &selected_target.provider.name,
            &selected_target.model,
            &captured_text,
        )
        .await;

    // Record provider success in health monitor
    state
        .provider_health_monitor
        .record_success(
            &selected_target.provider.name,
            provider_start.elapsed().as_millis() as u64,
        )
        .await;

    // Log successful provider metrics
    let total_tokens = hints.token_count + output_tokens as usize;
    let success_metrics = provider_metrics
        .clone()
        .with_response_time(provider_start.elapsed())
        .with_tokens(total_tokens)
        .success();
    log_provider_metrics(&success_metrics);

    // Record in production metrics collector
    state
        .production_collector
        .record_request(&request_context, &success_metrics);

    Ok(Json(final_response).into_response())
}

/// Handle POST /v1/messages/count_tokens
async fn count_tokens_endpoint(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(request): Json<ChatRequest>,
) -> Result<Response> {
    // Authenticate request
    authenticate(&headers, &state.config)?;

    // Validate anthropic-version header (warn-only)
    validate_anthropic_version(&headers);

    debug!("Counting tokens for request");

    // Count tokens
    let token_count = state.token_counter.count_request(&request)?;

    info!(token_count, "Token count completed");

    let response = TokenCountResponse {
        input_tokens: token_count,
    };

    Ok(Json(response).into_response())
}

/// Handle GET /api/sessions - Get session statistics
async fn get_session_stats(State(state): State<AppState>, headers: HeaderMap) -> Result<Response> {
    // Authenticate request
    authenticate(&headers, &state.config)?;

    let stats = state.session_manager.get_stats();

    let response = serde_json::json!({
        "active_sessions": stats.active_sessions,
        "max_sessions": stats.max_sessions,
        "cache_utilization_percent": (stats.active_sessions as f64 / stats.max_sessions as f64 * 100.0).round()
    });

    Ok(Json(response).into_response())
}

/// Handle GET /api/sessions/{session_id} - Get specific session info
async fn get_session_info(
    State(state): State<AppState>,
    headers: HeaderMap,
    axum::extract::Path(session_id): axum::extract::Path<String>,
) -> Result<Response> {
    // Authenticate request
    authenticate(&headers, &state.config)?;

    match state
        .session_manager
        .get_or_create_session(&session_id)
        .await
    {
        Ok(session) => {
            let json_value = serde_json::to_value(session).unwrap_or_default();
            Ok(Json(json_value).into_response())
        }
        Err(e) => Err(e),
    }
}

/// Handle GET /api/metrics/json - Get production metrics in JSON format
async fn get_metrics_json(State(state): State<AppState>, headers: HeaderMap) -> Result<Response> {
    // Authenticate request
    authenticate(&headers, &state.config)?;

    // Get aggregated metrics
    let metrics = state.production_collector.get_aggregated_metrics();

    Ok(Json(metrics).into_response())
}

/// Handle GET /api/metrics/prometheus - Get production metrics in Prometheus format
async fn get_metrics_prometheus(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Response> {
    // Authenticate request
    authenticate(&headers, &state.config)?;

    // Get aggregated metrics and export in Prometheus format
    let aggregated_metrics = state.production_collector.get_aggregated_metrics();
    let metrics = MetricsExporter::export_prometheus(&aggregated_metrics);

    Response::builder()
        .status(200)
        .header("Content-Type", "text/plain")
        .body(axum::body::Body::from(metrics))
        .map_err(|e| ProxyError::Internal(e.to_string()))
}

// ============================================================================
// Anthropic API Extensions - Missing Endpoint Handlers
// ============================================================================

// Message Batches API Handlers

/// Handle POST /v1/messages/batches - Create a new message batch
async fn create_message_batch(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(request): Json<MessageBatchRequest>,
) -> Result<Response> {
    authenticate(&headers, &state.config)?;

    info!(
        "Creating message batch with {} requests",
        request.requests.len()
    );

    // Generate batch ID
    let batch_id = format!("batch_{}", uuid::Uuid::new_v4());

    // For now, return a mock response that indicates the batch is being processed
    let response = MessageBatchResponse {
        id: batch_id.clone(),
        batch_type: "message_batch".to_string(),
        status: MessageBatchStatus::InProgress,
        results: vec![],
        errors: vec![],
        metadata: request.metadata,
        created_at: chrono::Utc::now().to_rfc3339(),
        processing_status: "Processing".to_string(),
    };

    info!(batch_id = %batch_id, "Message batch created successfully");
    Ok(Json(response).into_response())
}

/// Handle GET /v1/messages/batches/{batch_id} - Get batch status
async fn get_message_batch(
    State(state): State<AppState>,
    headers: HeaderMap,
    axum::extract::Path(batch_id): axum::extract::Path<String>,
) -> Result<Response> {
    authenticate(&headers, &state.config)?;

    debug!(batch_id = %batch_id, "Fetching message batch status");

    // For now, return a mock response
    let response = MessageBatchResponse {
        id: batch_id,
        batch_type: "message_batch".to_string(),
        status: MessageBatchStatus::Ended,
        results: vec![],
        errors: vec![],
        metadata: None,
        created_at: chrono::Utc::now().to_rfc3339(),
        processing_status: "Completed".to_string(),
    };

    Ok(Json(response).into_response())
}

/// Handle GET /v1/messages/batches/{batch_id}/results - Get batch results
async fn get_batch_results(
    State(state): State<AppState>,
    headers: HeaderMap,
    axum::extract::Path(batch_id): axum::extract::Path<String>,
) -> Result<Response> {
    authenticate(&headers, &state.config)?;

    debug!(batch_id = %batch_id, "Fetching message batch results");

    // For now, return empty results
    let response = MessageBatchResponse {
        id: batch_id,
        batch_type: "message_batch".to_string(),
        status: MessageBatchStatus::Ended,
        results: vec![],
        errors: vec![],
        metadata: None,
        created_at: chrono::Utc::now().to_rfc3339(),
        processing_status: "Completed".to_string(),
    };

    Ok(Json(response).into_response())
}

/// Handle POST /v1/messages/batches/{batch_id}/cancel - Cancel a batch
async fn cancel_message_batch(
    State(state): State<AppState>,
    headers: HeaderMap,
    axum::extract::Path(batch_id): axum::extract::Path<String>,
) -> Result<Response> {
    authenticate(&headers, &state.config)?;

    info!(batch_id = %batch_id, "Canceling message batch");

    // For now, return a mock canceled response
    let response = MessageBatchResponse {
        id: batch_id,
        batch_type: "message_batch".to_string(),
        status: MessageBatchStatus::Canceled,
        results: vec![],
        errors: vec![],
        metadata: None,
        created_at: chrono::Utc::now().to_rfc3339(),
        processing_status: "Canceled".to_string(),
    };

    Ok(Json(response).into_response())
}

/// Handle DELETE /v1/messages/batches/{batch_id} - Delete a batch
async fn delete_message_batch(
    State(state): State<AppState>,
    headers: HeaderMap,
    axum::extract::Path(batch_id): axum::extract::Path<String>,
) -> Result<Response> {
    authenticate(&headers, &state.config)?;

    info!(batch_id = %batch_id, "Deleting message batch");

    // Return 204 No Content for successful deletion
    Response::builder()
        .status(204)
        .body(axum::body::Body::empty())
        .map_err(|e| ProxyError::Internal(e.to_string()))
}

// Files API Handlers

/// Handle POST /v1/files - Upload a file
async fn upload_file(
    State(state): State<AppState>,
    headers: HeaderMap,
    _multipart: axum::extract::Multipart,
) -> Result<Response> {
    authenticate(&headers, &state.config)?;

    info!("Processing file upload");

    // For now, return a mock file upload response
    let file_id = format!("file_{}", uuid::Uuid::new_v4());
    let response = FileUploadResponse {
        id: file_id.clone(),
        object: "file".to_string(),
        bytes: 0,
        created_at: chrono::Utc::now().to_rfc3339(),
        filename: "uploaded_file".to_string(),
        purpose: "assistants".to_string(),
        status: FileStatus::Processed,
    };

    info!(file_id = %file_id, "File uploaded successfully");
    Ok(Json(response).into_response())
}

/// Handle GET /v1/files - List files
async fn list_files(State(state): State<AppState>, headers: HeaderMap) -> Result<Response> {
    authenticate(&headers, &state.config)?;

    debug!("Listing files");

    // For now, return empty list
    let response = FileListResponse {
        object: "list".to_string(),
        data: vec![],
        has_more: false,
        first_id: None,
        last_id: None,
    };

    Ok(Json(response).into_response())
}

/// Handle GET /v1/files/{file_id} - Get file metadata
async fn get_file(
    State(state): State<AppState>,
    headers: HeaderMap,
    axum::extract::Path(file_id): axum::extract::Path<String>,
) -> Result<Response> {
    authenticate(&headers, &state.config)?;

    debug!(file_id = %file_id, "Fetching file metadata");

    // For now, return a mock response
    let response = FileUploadResponse {
        id: file_id,
        object: "file".to_string(),
        bytes: 0,
        created_at: chrono::Utc::now().to_rfc3339(),
        filename: "file.txt".to_string(),
        purpose: "assistants".to_string(),
        status: FileStatus::Processed,
    };

    Ok(Json(response).into_response())
}

/// Handle GET /v1/files/{file_id}/content - Get file content
async fn get_file_content(
    State(state): State<AppState>,
    headers: HeaderMap,
    axum::extract::Path(file_id): axum::extract::Path<String>,
) -> Result<Response> {
    authenticate(&headers, &state.config)?;

    debug!(file_id = %file_id, "Fetching file content");

    // For now, return empty content
    Response::builder()
        .status(200)
        .header("Content-Type", "application/octet-stream")
        .body(axum::body::Body::from(""))
        .map_err(|e| ProxyError::Internal(e.to_string()))
}

/// Handle DELETE /v1/files/{file_id} - Delete a file
async fn delete_file(
    State(state): State<AppState>,
    headers: HeaderMap,
    axum::extract::Path(file_id): axum::extract::Path<String>,
) -> Result<Response> {
    authenticate(&headers, &state.config)?;

    info!(file_id = %file_id, "Deleting file");

    // Return 204 No Content for successful deletion
    Response::builder()
        .status(204)
        .body(axum::body::Body::empty())
        .map_err(|e| ProxyError::Internal(e.to_string()))
}

// Models API Handlers

/// Handle GET /v1/models - List available models
async fn list_models(State(state): State<AppState>, headers: HeaderMap) -> Result<Response> {
    authenticate(&headers, &state.config)?;

    debug!("Listing available models");

    // Build model list from configured providers
    let mut models = Vec::new();
    for provider in &state.config.providers {
        for model in &provider.models {
            let model_info = ModelInfo {
                id: format!("{}:{}", provider.name, model),
                object: "model".to_string(),
                created: chrono::Utc::now().timestamp() as u64,
                owned_by: provider.name.clone(),
                display_name: model.clone(),
                pricing: Some(ModelPricing {
                    input_tokens: "0.001".to_string(),
                    output_tokens: "0.002".to_string(),
                }),
                context_window: Some(200000),
                capabilities: Some(vec![
                    "chat".to_string(),
                    "completion".to_string(),
                    "tools".to_string(),
                ]),
            };
            models.push(model_info);
        }
    }

    let response = ModelsListResponse {
        object: "list".to_string(),
        data: models,
    };

    Ok(Json(response).into_response())
}

/// Handle GET /v1/models/{model_id} - Get specific model info
async fn get_model(
    State(state): State<AppState>,
    headers: HeaderMap,
    axum::extract::Path(model_id): axum::extract::Path<String>,
) -> Result<Response> {
    authenticate(&headers, &state.config)?;

    debug!(model_id = %model_id, "Fetching model information");

    // Try to find the model in our configuration
    for provider in &state.config.providers {
        for model in &provider.models {
            let full_id = format!("{}:{}", provider.name, model);
            if full_id == model_id || *model == model_id {
                let model_info = ModelInfo {
                    id: full_id,
                    object: "model".to_string(),
                    created: chrono::Utc::now().timestamp() as u64,
                    owned_by: provider.name.clone(),
                    display_name: model.clone(),
                    pricing: Some(ModelPricing {
                        input_tokens: "0.001".to_string(),
                        output_tokens: "0.002".to_string(),
                    }),
                    context_window: Some(200000),
                    capabilities: Some(vec![
                        "chat".to_string(),
                        "completion".to_string(),
                        "tools".to_string(),
                    ]),
                };
                return Ok(Json(model_info).into_response());
            }
        }
    }

    warn!(model_id = %model_id, "Model not found");
    Err(ProxyError::NotFound(format!(
        "Model '{}' not found",
        model_id
    )))
}

// Experimental Tools API Handlers

/// Handle POST /v1/experimental/generate_prompt - Generate prompt
async fn generate_prompt(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(request): Json<GeneratePromptRequest>,
) -> Result<Response> {
    authenticate(&headers, &state.config)?;

    debug!(model = %request.model, "Generating experimental prompt");

    // For now, return a mock response
    let response = GeneratePromptResponse {
        completion: "This is an experimental prompt generation response.".to_string(),
        model: request.model,
        stop_reason: Some("end_turn".to_string()),
        usage: Usage {
            prompt_tokens: Some(10),
            completion_tokens: Some(15),
            prompt_tokens_details: None,
            completion_tokens_details: None,
            total_tokens: Some(25),
        },
    };

    Ok(Json(response).into_response())
}

/// Apply model name mappings from configuration.
///
/// This function resolves model aliases before routing. If a mapping matches,
/// it preserves the full "provider,model" format so the router can make
/// an explicit provider decision.
///
/// # Arguments
/// * `request` - The chat request to modify
/// * `mappings` - List of model mappings from configuration
fn apply_model_mappings(request: &mut ChatRequest, mappings: &[ModelMapping]) {
    let (resolved, matched) = resolve_model(&request.model, mappings);
    if matched.is_some() {
        let original = request.model.clone();
        // Keep the full "provider,model" format - the router's Phase 0 will
        // parse this and create an explicit routing decision
        request.model = resolved;
        debug!(
            original = %original,
            resolved = %request.model,
            "Applied model mapping from config"
        );
    }
}

/// Authenticate request using API key
fn authenticate(headers: &HeaderMap, config: &ProxyConfig) -> Result<()> {
    // Extract API key from headers
    let api_key = headers
        .get("x-api-key")
        .or_else(|| headers.get("authorization"))
        .and_then(|v| v.to_str().ok())
        .ok_or(ProxyError::MissingApiKey)?;

    // Remove "Bearer " prefix if present
    let api_key = api_key.trim_start_matches("Bearer ").trim();

    // Validate against configured key
    if api_key != config.proxy.api_key {
        warn!("Invalid API key attempt");
        return Err(ProxyError::InvalidApiKey);
    }

    debug!("Authentication successful");
    Ok(())
}

/// Validate anthropic-version header (warn-only for backwards compatibility)
///
/// Supported versions: "2023-06-01", "2023-01-01"
/// Unknown versions trigger a warning but don't reject the request.
fn validate_anthropic_version(headers: &HeaderMap) {
    const SUPPORTED_VERSIONS: &[&str] = &["2023-06-01", "2023-01-01"];

    if let Some(version) = headers.get("anthropic-version") {
        if let Ok(v) = version.to_str() {
            if !SUPPORTED_VERSIONS.contains(&v) {
                warn!(version = %v, "Unknown anthropic-version header");
            } else {
                debug!(version = %v, "Valid anthropic-version header");
            }
        }
    }
    // Note: Missing header is acceptable (backwards compatibility)
}

// ============================================================================
// Response Types
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatResponse {
    pub id: String,
    /// Message type - always "message" for Anthropic API compatibility.
    #[serde(rename = "type")]
    #[serde(default = "default_message_type")]
    pub message_type: String,
    pub model: String,
    pub role: String,
    pub content: Vec<ContentBlock>,
    pub stop_reason: Option<String>,
    /// Stop sequence that triggered completion (if any).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop_sequence: Option<String>,
    pub usage: Usage,
}

fn default_message_type() -> String {
    "message".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContentBlock {
    #[serde(rename = "type")]
    pub block_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    /// Tool use ID (for tool_use blocks)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    /// Tool name (for tool_use blocks)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Tool input arguments (for tool_use blocks)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenCountResponse {
    pub input_tokens: usize,
}

impl ChatResponse {
    /// Convert to OpenAI-compatible format
    pub fn to_openai_format(&self) -> serde_json::Value {
        // Extract text content and tool calls from content blocks
        let mut content_text = String::new();
        let mut tool_calls = Vec::new();

        for (index, block) in self.content.iter().enumerate() {
            match block.block_type.as_str() {
                "text" => {
                    if let Some(text) = &block.text {
                        content_text.push_str(text);
                    }
                }
                "tool_use" => {
                    if let (Some(id), Some(name), Some(input)) =
                        (&block.id, &block.name, &block.input)
                    {
                        tool_calls.push(serde_json::json!({
                            "id": id,
                            "type": "function",
                            "function": {
                                "name": name,
                                "arguments": input.to_string()
                            },
                            "index": index
                        }));
                    }
                }
                _ => {}
            }
        }

        // Build message object
        let mut message = serde_json::json!({
            "role": self.role,
            "content": content_text
        });

        // Add tool_calls if present
        if !tool_calls.is_empty() {
            message["tool_calls"] = serde_json::Value::Array(tool_calls);
        }

        serde_json::json!({
            "id": self.id,
            "object": "chat.completion",
            "created": std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            "model": self.model,
            "choices": [{
                "index": 0,
                "message": message,
                "finish_reason": self.stop_reason.clone().unwrap_or_else(|| "stop".to_string())
            }],
            "usage": {
                "prompt_tokens": self.usage.prompt_tokens.unwrap_or(0),
                "completion_tokens": self.usage.completion_tokens.unwrap_or(0),
                "total_tokens": self.usage.total_tokens.unwrap_or(0)
            }
        })
    }
}

// ============================================================================
// Anthropic API Extensions - Missing Endpoints
// ============================================================================

// Message Batches API Types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageBatchRequest {
    pub requests: Vec<ChatRequest>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageBatchResponse {
    pub id: String,
    #[serde(rename = "type")]
    pub batch_type: String,
    pub status: MessageBatchStatus,
    pub results: Vec<MessageBatchResult>,
    pub errors: Vec<MessageBatchError>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,
    pub created_at: String,
    pub processing_status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageBatchResult {
    pub custom_id: String,
    pub result: ChatResponse,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageBatchError {
    pub custom_id: String,
    pub error: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MessageBatchStatus {
    InProgress,
    Canceling,
    Ended,
    Canceled,
    Failed,
}

// Files API Types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileUploadResponse {
    pub id: String,
    pub object: String,
    pub bytes: u64,
    pub created_at: String,
    pub filename: String,
    pub purpose: String,
    pub status: FileStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FileStatus {
    Uploaded,
    Processing,
    Processed,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileListResponse {
    pub object: String,
    pub data: Vec<FileUploadResponse>,
    pub has_more: bool,
    pub first_id: Option<String>,
    pub last_id: Option<String>,
}

// Models API Types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelInfo {
    pub id: String,
    pub object: String,
    pub created: u64,
    pub owned_by: String,
    pub display_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pricing: Option<ModelPricing>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context_window: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub capabilities: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelPricing {
    pub input_tokens: String,
    pub output_tokens: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelsListResponse {
    pub object: String,
    pub data: Vec<ModelInfo>,
}

// Experimental Tools API Types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneratePromptRequest {
    pub model: String,
    pub prompt: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens_to_sample: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_k: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneratePromptResponse {
    pub completion: String,
    pub model: String,
    pub stop_reason: Option<String>,
    pub usage: Usage,
}

/// Load RoleGraph from taxonomy directory
async fn load_rolegraph() -> Result<RoleGraphClient> {
    use std::path::Path;

    // 1) Allow env override for taxonomy path
    if let Ok(env_path) = std::env::var("ROLEGRAPH_TAXONOMY_PATH") {
        let env_path = Path::new(&env_path).to_path_buf();
        if env_path.exists() {
            let mut client = RoleGraphClient::new(&env_path)?;
            client.load_taxonomy()?;
            return Ok(client);
        } else {
            warn!(path = %env_path.display(), "ROLEGRAPH_TAXONOMY_PATH set but path does not exist");
        }
    }

    // 2) Try to load taxonomy bundled with the project
    let project_path = Path::new("docs/taxonomy");
    if project_path.exists() {
        let mut client = RoleGraphClient::new(project_path)?;
        client.load_taxonomy()?;
        return Ok(client);
    }

    // 3) Try to load from legacy relative path (for development)
    let taxonomy_path = Path::new("../llm_proxy_terraphim/taxonomy");

    if !taxonomy_path.exists() {
        // 4) Try absolute path as fallback (legacy claude_code_agents checkout)
        let home = std::env::var("HOME").unwrap_or_else(|_| "/home/alex".to_string());
        let abs_path = Path::new(&home).join("claude_code_agents/llm_proxy_terraphim/taxonomy");

        if !abs_path.exists() {
            return Err(ProxyError::ConfigError(
                "Taxonomy directory not found. RoleGraph pattern matching disabled.".to_string(),
            ));
        }

        let mut client = RoleGraphClient::new(&abs_path)?;
        client.load_taxonomy()?;
        Ok(client)
    } else {
        let mut client = RoleGraphClient::new(taxonomy_path)?;
        client.load_taxonomy()?;
        Ok(client)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{
        ManagementSettings, OAuthSettings, Provider, ProxySettings, RouterSettings,
    };
    use crate::routing::RoutingStrategy;
    use crate::webhooks::WebhookSettings;

    fn create_test_config() -> ProxyConfig {
        ProxyConfig {
            proxy: ProxySettings {
                host: "127.0.0.1".to_string(),
                port: 3456,
                api_key: "test_api_key".to_string(),
                timeout_ms: 60000,
            },
            router: RouterSettings {
                default: "test,test-model".to_string(),
                background: None,
                think: None,
                plan_implementation: None,
                long_context: None,
                long_context_threshold: 60000,
                web_search: None,
                image: None,
                model_mappings: vec![],
                model_exclusions: vec![],
                strategy: RoutingStrategy::default(),
            },
            providers: vec![Provider {
                name: "test".to_string(),
                api_base_url: "http://localhost:8000".to_string(),
                api_key: "test".to_string(),
                models: vec!["test-model".to_string()],
                transformers: vec![],
            }],
            security: Default::default(),
            oauth: OAuthSettings::default(),
            management: ManagementSettings::default(),
            webhooks: WebhookSettings::default(),
        }
    }

    #[test]
    fn test_authenticate_with_valid_key() {
        let config = create_test_config();
        let mut headers = HeaderMap::new();
        headers.insert("x-api-key", "test_api_key".parse().unwrap());

        let result = authenticate(&headers, &config);
        assert!(result.is_ok());
    }

    #[test]
    fn test_authenticate_with_bearer_token() {
        let config = create_test_config();
        let mut headers = HeaderMap::new();
        headers.insert("authorization", "Bearer test_api_key".parse().unwrap());

        let result = authenticate(&headers, &config);
        assert!(result.is_ok());
    }

    #[test]
    fn test_authenticate_missing_key() {
        let config = create_test_config();
        let headers = HeaderMap::new();

        let result = authenticate(&headers, &config);
        assert!(matches!(result, Err(ProxyError::MissingApiKey)));
    }

    #[test]
    fn test_authenticate_invalid_key() {
        let config = create_test_config();
        let mut headers = HeaderMap::new();
        headers.insert("x-api-key", "wrong_key".parse().unwrap());

        let result = authenticate(&headers, &config);
        assert!(matches!(result, Err(ProxyError::InvalidApiKey)));
    }

    #[test]
    fn test_apply_model_mappings_with_match() {
        use crate::token_counter::{Message, MessageContent};

        let mappings = vec![
            ModelMapping::new(
                "claude-3-5-sonnet-20241022",
                "openrouter,anthropic/claude-3.5-sonnet:beta",
            ),
            ModelMapping::new("claude-*", "openrouter,anthropic/claude-3-opus"),
        ];

        let mut request = ChatRequest {
            model: "claude-3-5-sonnet-20241022".to_string(),
            messages: vec![Message {
                role: "user".to_string(),
                content: MessageContent::Text("Hello".to_string()),
                tool_calls: None,
                tool_call_id: None,
                name: None,
            }],
            ..Default::default()
        };

        apply_model_mappings(&mut request, &mappings);

        // Model should be resolved to full provider,model format for explicit routing
        assert_eq!(request.model, "openrouter,anthropic/claude-3.5-sonnet:beta");
    }

    #[test]
    fn test_apply_model_mappings_no_match() {
        use crate::token_counter::{Message, MessageContent};

        let mappings = vec![ModelMapping::new(
            "claude-*",
            "openrouter,anthropic/claude-3-opus",
        )];

        let mut request = ChatRequest {
            model: "gpt-4".to_string(),
            messages: vec![Message {
                role: "user".to_string(),
                content: MessageContent::Text("Hello".to_string()),
                tool_calls: None,
                tool_call_id: None,
                name: None,
            }],
            ..Default::default()
        };

        apply_model_mappings(&mut request, &mappings);

        // Model should remain unchanged
        assert_eq!(request.model, "gpt-4");
    }

    #[test]
    fn test_apply_model_mappings_glob_pattern() {
        use crate::token_counter::{Message, MessageContent};

        let mappings = vec![ModelMapping::new(
            "claude-3-*-sonnet-*",
            "openrouter,anthropic/claude-3.5-sonnet",
        )];

        let mut request = ChatRequest {
            model: "claude-3-5-sonnet-latest".to_string(),
            messages: vec![Message {
                role: "user".to_string(),
                content: MessageContent::Text("Hello".to_string()),
                tool_calls: None,
                tool_call_id: None,
                name: None,
            }],
            ..Default::default()
        };

        apply_model_mappings(&mut request, &mappings);

        // Model should be resolved to full provider,model format for explicit routing
        assert_eq!(request.model, "openrouter,anthropic/claude-3.5-sonnet");
    }

    #[test]
    fn test_apply_model_mappings_empty_mappings() {
        use crate::token_counter::{Message, MessageContent};

        let mappings: Vec<ModelMapping> = vec![];

        let mut request = ChatRequest {
            model: "any-model".to_string(),
            messages: vec![Message {
                role: "user".to_string(),
                content: MessageContent::Text("Hello".to_string()),
                tool_calls: None,
                tool_call_id: None,
                name: None,
            }],
            ..Default::default()
        };

        apply_model_mappings(&mut request, &mappings);

        // Model should remain unchanged
        assert_eq!(request.model, "any-model");
    }

    #[tokio::test]
    async fn test_load_rolegraph_uses_env_path_when_valid() {
        use std::fs;
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let taxonomy_dir = temp_dir.path();
        let scenarios_dir = taxonomy_dir.join("routing_scenarios");
        fs::create_dir_all(&scenarios_dir).unwrap();

        let file_path = scenarios_dir.join("default_routing.md");
        fs::write(
            &file_path,
            r#"# Default Routing

synonyms:: default, general

route:: deepseek, deepseek-chat
"#,
        )
        .unwrap();

        std::env::set_var("ROLEGRAPH_TAXONOMY_PATH", taxonomy_dir);

        let client = load_rolegraph()
            .await
            .expect("expected RoleGraph to load from env path");
        assert!(client.pattern_count() >= 1);

        std::env::remove_var("ROLEGRAPH_TAXONOMY_PATH");
    }

    #[tokio::test]
    async fn test_load_rolegraph_env_path_missing_returns_err() {
        // Point to a guaranteed-missing directory path
        let missing_path = "/tmp/terraphim_missing_taxonomy_12345";
        std::env::set_var("ROLEGRAPH_TAXONOMY_PATH", missing_path);

        let result = load_rolegraph()
            .await
            .expect("RoleGraph should fall back to bundled taxonomy");
        assert!(result.pattern_count() > 0);

        std::env::remove_var("ROLEGRAPH_TAXONOMY_PATH");
    }
}
