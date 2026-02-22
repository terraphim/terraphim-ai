//! Performance benchmarks for Terraphim LLM Proxy
//!
//! Measures latency, throughput, and resource usage.

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use std::sync::Arc;
use terraphim_llm_proxy::{
    analyzer::RequestAnalyzer,
    config::{
        ManagementSettings, OAuthSettings, Provider, ProxyConfig, ProxySettings, RouterSettings,
        SecuritySettings,
    },
    router::RouterAgent,
    routing::RoutingStrategy,
    token_counter::{ChatRequest, Message, MessageContent, SystemPrompt, TokenCounter},
    webhooks::WebhookSettings,
};

/// Create a simple test request
fn create_simple_request() -> ChatRequest {
    ChatRequest {
        model: "claude-3-5-sonnet-20241022".to_string(),
        messages: vec![Message {
            role: "user".to_string(),
            content: MessageContent::Text("Hello, world!".to_string()),
            ..Default::default()
        }],
        system: None,
        tools: None,
        max_tokens: None,
        temperature: None,
        stream: None,
        thinking: None,
        ..Default::default()
    }
}

/// Create a complex request with system prompt and tools
fn create_complex_request() -> ChatRequest {
    ChatRequest {
        model: "claude-3-5-sonnet-20241022".to_string(),
        messages: vec![
            Message {
                role: "user".to_string(),
                content: MessageContent::Text("Analyze this code".to_string()),
                ..Default::default()
            },
            Message {
                role: "assistant".to_string(),
                content: MessageContent::Text("I'll analyze that for you.".to_string()),
                ..Default::default()
            },
            Message {
                role: "user".to_string(),
                content: MessageContent::Text("Please continue".to_string()),
                ..Default::default()
            },
        ],
        system: Some(SystemPrompt::Text(
            "You are an expert code reviewer with 10 years of experience.".to_string(),
        )),
        tools: Some(vec![serde_json::from_value(serde_json::json!({
            "name": "analyze_code",
            "description": "Analyze code for quality",
            "input_schema": {
                "type": "object",
                "properties": {
                    "code": {"type": "string"},
                    "language": {"type": "string"}
                }
            }
        }))
        .unwrap()]),
        max_tokens: Some(2048),
        temperature: Some(0.3),
        stream: Some(false),
        thinking: None,
        ..Default::default()
    }
}

/// Create test configuration
fn create_test_config() -> ProxyConfig {
    ProxyConfig {
        proxy: ProxySettings {
            host: "127.0.0.1".to_string(),
            port: 3456,
            api_key: "test_key".to_string(),
            timeout_ms: 60000,
        },
        router: RouterSettings {
            default: "deepseek,deepseek-chat".to_string(),
            background: Some("ollama,qwen2.5-coder:latest".to_string()),
            think: Some("deepseek,deepseek-reasoner".to_string()),
            plan_implementation: None,
            long_context: Some("openrouter,google/gemini-2.0-flash-exp".to_string()),
            long_context_threshold: 60000,
            web_search: Some("openrouter,perplexity/llama-3.1-sonar".to_string()),
            image: Some("openrouter,anthropic/claude-3.5-sonnet".to_string()),
            model_mappings: vec![],
            model_exclusions: vec![],
            strategy: RoutingStrategy::default(),
        },
        providers: vec![
            Provider {
                name: "deepseek".to_string(),
                api_base_url: "https://api.deepseek.com".to_string(),
                api_key: "test".to_string(),
                models: vec!["deepseek-chat".to_string()],
                transformers: vec!["deepseek".to_string()],
            },
            Provider {
                name: "ollama".to_string(),
                api_base_url: "http://localhost:11434".to_string(),
                api_key: "ollama".to_string(),
                models: vec!["qwen2.5-coder:latest".to_string()],
                transformers: vec!["ollama".to_string()],
            },
            Provider {
                name: "openrouter".to_string(),
                api_base_url: "https://openrouter.ai".to_string(),
                api_key: "test".to_string(),
                models: vec![
                    "google/gemini-2.0-flash-exp".to_string(),
                    "perplexity/llama-3.1-sonar".to_string(),
                    "anthropic/claude-3.5-sonnet".to_string(),
                ],
                transformers: vec!["openrouter".to_string()],
            },
        ],
        security: SecuritySettings::default(),
        oauth: OAuthSettings::default(),
        management: ManagementSettings::default(),
        webhooks: WebhookSettings::default(),
    }
}

/// Benchmark token counting performance
fn bench_token_counting(c: &mut Criterion) {
    let counter = TokenCounter::new().unwrap();

    let mut group = c.benchmark_group("token_counting");

    // Simple request
    let simple_req = create_simple_request();
    group.bench_function("simple_request", |b| {
        b.iter(|| counter.count_request(black_box(&simple_req)))
    });

    // Complex request
    let complex_req = create_complex_request();
    group.bench_function("complex_request", |b| {
        b.iter(|| counter.count_request(black_box(&complex_req)))
    });

    // Just text counting
    group.bench_function("text_only", |b| {
        b.iter(|| counter.count_text(black_box("The quick brown fox jumps over the lazy dog")))
    });

    group.finish();
}

/// Benchmark request analysis
fn bench_request_analysis(c: &mut Criterion) {
    let token_counter = Arc::new(TokenCounter::new().unwrap());
    let analyzer = RequestAnalyzer::new(token_counter);

    let mut group = c.benchmark_group("request_analysis");

    let simple_req = create_simple_request();
    group.bench_function("simple_request", |b| {
        b.iter(|| analyzer.analyze(black_box(&simple_req)))
    });

    let complex_req = create_complex_request();
    group.bench_function("complex_request", |b| {
        b.iter(|| analyzer.analyze(black_box(&complex_req)))
    });

    group.finish();
}

/// Benchmark routing decisions
fn bench_routing(c: &mut Criterion) {
    let config = Arc::new(create_test_config());
    let router = RouterAgent::new(config);
    let token_counter = Arc::new(TokenCounter::new().unwrap());
    let analyzer = RequestAnalyzer::new(token_counter);

    let mut group = c.benchmark_group("routing");

    let simple_req = create_simple_request();
    let hints = analyzer.analyze(&simple_req).unwrap();

    group.bench_function("default_scenario", |b| {
        b.iter(|| {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(router.route(black_box(&simple_req), black_box(&hints)))
        })
    });

    group.finish();
}

/// Benchmark complete request processing pipeline (without LLM call)
fn bench_pipeline(c: &mut Criterion) {
    let config = Arc::new(create_test_config());
    let token_counter = Arc::new(TokenCounter::new().unwrap());
    let analyzer = RequestAnalyzer::new(token_counter.clone());
    let router = RouterAgent::new(config);

    let mut group = c.benchmark_group("pipeline");

    let simple_req = create_simple_request();

    group.bench_function("analyze_and_route", |b| {
        b.iter(|| {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                let hints = analyzer.analyze(black_box(&simple_req)).unwrap();
                router
                    .route(black_box(&simple_req), black_box(&hints))
                    .await
            })
        })
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_token_counting,
    bench_request_analysis,
    bench_routing,
    bench_pipeline
);
criterion_main!(benches);
