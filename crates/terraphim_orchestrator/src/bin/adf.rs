use std::path::PathBuf;

use terraphim_orchestrator::AgentOrchestrator;
use terraphim_types::capability::{Capability, CostLevel, Latency, Provider, ProviderType};
use tracing_subscriber::EnvFilter;

/// Register LLM providers for keyword-based model routing.
///
/// These providers are matched against task prompts via KeywordRouter.
/// The selected model_id is passed to the CLI tool via -m/--model flags.
fn register_providers(orchestrator: &mut AgentOrchestrator) {
    let router = orchestrator.router_mut();

    // Deep thinking: o3 (OpenAI reasoning model)
    router.add_provider(Provider {
        id: "openai-o3".into(),
        name: "OpenAI o3".into(),
        provider_type: ProviderType::Llm {
            model_id: "o3".into(),
            api_endpoint: "https://api.openai.com".into(),
        },
        capabilities: vec![
            Capability::DeepThinking,
            Capability::SecurityAudit,
            Capability::Architecture,
        ],
        cost_level: CostLevel::Expensive,
        latency: Latency::Slow,
        keywords: vec![
            "reason".into(),
            "think".into(),
            "analyze deeply".into(),
            "security".into(),
            "vulnerability".into(),
            "CVE".into(),
            "architecture".into(),
        ],
    });

    // General coding: o4-mini (fast, capable)
    router.add_provider(Provider {
        id: "openai-o4-mini".into(),
        name: "OpenAI o4-mini".into(),
        provider_type: ProviderType::Llm {
            model_id: "o4-mini".into(),
            api_endpoint: "https://api.openai.com".into(),
        },
        capabilities: vec![
            Capability::CodeGeneration,
            Capability::CodeReview,
            Capability::Testing,
            Capability::Refactoring,
        ],
        cost_level: CostLevel::Moderate,
        latency: Latency::Medium,
        keywords: vec![
            "implement".into(),
            "code".into(),
            "review".into(),
            "test".into(),
            "refactor".into(),
            "check".into(),
        ],
    });

    // Fast/cheap: gpt-4.1-nano
    router.add_provider(Provider {
        id: "openai-4.1-nano".into(),
        name: "OpenAI GPT-4.1 Nano".into(),
        provider_type: ProviderType::Llm {
            model_id: "gpt-4.1-nano".into(),
            api_endpoint: "https://api.openai.com".into(),
        },
        capabilities: vec![
            Capability::FastThinking,
            Capability::Explanation,
            Capability::Documentation,
        ],
        cost_level: CostLevel::Cheap,
        latency: Latency::Fast,
        keywords: vec![
            "quick".into(),
            "fast".into(),
            "summary".into(),
            "explain".into(),
            "document".into(),
        ],
    });

    // Performance optimization: o4-mini (good at analysis)
    router.add_provider(Provider {
        id: "openai-o4-mini-perf".into(),
        name: "OpenAI o4-mini (perf)".into(),
        provider_type: ProviderType::Llm {
            model_id: "o4-mini".into(),
            api_endpoint: "https://api.openai.com".into(),
        },
        capabilities: vec![Capability::Performance],
        cost_level: CostLevel::Moderate,
        latency: Latency::Medium,
        keywords: vec!["performance".into(), "optimize".into(), "benchmark".into()],
    });

    tracing::info!("registered 4 LLM providers for keyword routing");
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    let config_path = std::env::args()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/opt/ai-dark-factory/orchestrator.toml"));

    tracing::info!(config = %config_path.display(), "loading orchestrator config");

    let mut orchestrator = AgentOrchestrator::from_config_file(&config_path)?;

    // Register LLM providers for keyword-based model selection
    register_providers(&mut orchestrator);

    #[cfg(feature = "quickwit")]
    {
        if let Some(qw_config) = orchestrator.quickwit_config().cloned() {
            if qw_config.enabled {
                let sink = terraphim_orchestrator::quickwit::QuickwitSink::new(
                    qw_config.endpoint.clone(),
                    qw_config.index_id.clone(),
                    qw_config.batch_size,
                    qw_config.flush_interval_secs,
                );
                orchestrator.set_quickwit_sink(sink);
                tracing::info!(
                    endpoint = %qw_config.endpoint,
                    index = %qw_config.index_id,
                    "Quickwit logging enabled"
                );
            }
        }
    }

    // Handle SIGTERM/SIGINT for graceful shutdown
    let shutdown_flag = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let flag = shutdown_flag.clone();

    tokio::spawn(async move {
        tokio::signal::ctrl_c().await.ok();
        tracing::info!("received shutdown signal");
        flag.store(true, std::sync::atomic::Ordering::SeqCst);
    });

    tracing::info!("starting AI Dark Factory orchestrator");
    orchestrator.run().await?;

    Ok(())
}
