//! Multi-Agent Coordination Example
//!
//! This example demonstrates advanced multi-agent coordination capabilities:
//! - Creating multiple specialized agents
//! - Agent registry and discovery
//! - Coordinated task execution
//! - Knowledge sharing between agents

use ahash::AHashMap;
use std::sync::Arc;
use terraphim_config::Role;
use terraphim_multi_agent::{
    test_utils::create_test_role, AgentRegistry, CommandInput, CommandType, MultiAgentResult,
    TerraphimAgent,
};
use terraphim_persistence::DeviceStorage;
use terraphim_types::RelevanceFunction;

/// Create specialized agent roles for coordination example
fn create_specialized_roles() -> Vec<Role> {
    vec![
        // Code Review Agent
        Role {
            shortname: Some("reviewer".to_string()),
            name: "CodeReviewer".into(),
            relevance_function: RelevanceFunction::BM25,
            terraphim_it: false,
            theme: "default".to_string(),
            kg: None,
            haystacks: vec![],
            llm_enabled: false,
            llm_api_key: None,
            llm_model: None,
            llm_auto_summarize: false,
            llm_chat_enabled: false,
            llm_chat_system_prompt: None,
            llm_chat_model: None,
            llm_context_window: None,
            llm_router_enabled: false,
            llm_router_config: None,
            extra: {
                let mut extra = AHashMap::new();
                extra.insert(
                    "capabilities".to_string(),
                    serde_json::json!(["code_review", "security_analysis", "best_practices"]),
                );
                extra.insert(
                    "goals".to_string(),
                    serde_json::json!([
                        "Ensure code quality",
                        "Identify security issues",
                        "Enforce best practices"
                    ]),
                );
                extra.insert("llm_provider".to_string(), serde_json::json!("ollama"));
                extra.insert(
                    "ollama_base_url".to_string(),
                    serde_json::json!("http://127.0.0.1:11434"),
                );
                extra.insert("ollama_model".to_string(), serde_json::json!("gemma3:270m"));
                extra.insert("llm_temperature".to_string(), serde_json::json!(0.3)); // Focused analysis
                extra
            },
        },
        // Documentation Agent
        Role {
            shortname: Some("documenter".to_string()),
            name: "DocumentationWriter".into(),
            relevance_function: RelevanceFunction::BM25,
            terraphim_it: false,
            theme: "default".to_string(),
            kg: None,
            haystacks: vec![],
            llm_enabled: false,
            llm_api_key: None,
            llm_model: None,
            llm_auto_summarize: false,
            llm_chat_enabled: false,
            llm_chat_system_prompt: None,
            llm_chat_model: None,
            llm_context_window: None,
            llm_router_enabled: false,
            llm_router_config: None,
            extra: {
                let mut extra = AHashMap::new();
                extra.insert(
                    "capabilities".to_string(),
                    serde_json::json!(["documentation", "technical_writing", "api_docs"]),
                );
                extra.insert(
                    "goals".to_string(),
                    serde_json::json!([
                        "Create clear documentation",
                        "Explain complex concepts",
                        "Maintain consistency"
                    ]),
                );
                extra.insert("llm_provider".to_string(), serde_json::json!("ollama"));
                extra.insert(
                    "ollama_base_url".to_string(),
                    serde_json::json!("http://127.0.0.1:11434"),
                );
                extra.insert("ollama_model".to_string(), serde_json::json!("gemma3:270m"));
                extra.insert("llm_temperature".to_string(), serde_json::json!(0.5)); // Balanced creativity
                extra
            },
        },
        // Performance Optimizer Agent
        Role {
            shortname: Some("optimizer".to_string()),
            name: "PerformanceOptimizer".into(),
            relevance_function: RelevanceFunction::BM25,
            terraphim_it: false,
            theme: "default".to_string(),
            kg: None,
            haystacks: vec![],
            llm_enabled: false,
            llm_api_key: None,
            llm_model: None,
            llm_auto_summarize: false,
            llm_chat_enabled: false,
            llm_chat_system_prompt: None,
            llm_chat_model: None,
            llm_context_window: None,
            llm_router_enabled: false,
            llm_router_config: None,
            extra: {
                let mut extra = AHashMap::new();
                extra.insert(
                    "capabilities".to_string(),
                    serde_json::json!(["performance_analysis", "optimization", "profiling"]),
                );
                extra.insert(
                    "goals".to_string(),
                    serde_json::json!([
                        "Maximize performance",
                        "Reduce resource usage",
                        "Eliminate bottlenecks"
                    ]),
                );
                extra.insert("llm_provider".to_string(), serde_json::json!("ollama"));
                extra.insert(
                    "ollama_base_url".to_string(),
                    serde_json::json!("http://127.0.0.1:11434"),
                );
                extra.insert("ollama_model".to_string(), serde_json::json!("gemma3:270m"));
                extra.insert("llm_temperature".to_string(), serde_json::json!(0.4)); // Technical precision
                extra
            },
        },
    ]
}

/// Example 1: Agent Registry and Discovery
async fn example_agent_registry() -> MultiAgentResult<()> {
    println!("🏢 Example 1: Agent Registry and Discovery");
    println!("==========================================");

    // Initialize storage
    DeviceStorage::init_memory_only()
        .await
        .map_err(|e| terraphim_multi_agent::MultiAgentError::PersistenceError(e.to_string()))?;
    let storage = DeviceStorage::instance()
        .await
        .map_err(|e| terraphim_multi_agent::MultiAgentError::PersistenceError(e.to_string()))?;

    use std::ptr;
    let storage_copy = unsafe { ptr::read(storage) };
    let persistence = Arc::new(storage_copy);

    // Create registry
    let registry = AgentRegistry::new();

    // Create and register specialized agents
    let roles = create_specialized_roles();

    for role in roles {
        let role_name = role.name.clone();
        let agent = TerraphimAgent::new(role, persistence.clone(), None).await?;
        agent.initialize().await?;

        let agent_id = agent.agent_id;
        let _capabilities = agent.get_capabilities();

        // Register agent using the new API
        let agent_arc = Arc::new(agent);
        registry.register_agent(agent_arc).await?;

        println!("✅ Registered agent: {} (ID: {})", role_name, agent_id);
    }

    // Discover agents by capability using the new API
    let code_review_agents = registry.find_agents_by_capability("code_review").await;
    println!("🔍 Code review agents: {:?}", code_review_agents);

    let documentation_agents = registry.find_agents_by_capability("documentation").await;
    println!("🔍 Documentation agents: {:?}", documentation_agents);

    let performance_agents = registry
        .find_agents_by_capability("performance_analysis")
        .await;
    println!("🔍 Performance agents: {:?}", performance_agents);

    println!(
        "📊 Total registered agents: {}",
        registry.get_all_agents().await.len()
    );

    Ok(())
}

/// Example 2: Coordinated Task Execution using Registry
async fn example_coordinated_execution() -> MultiAgentResult<()> {
    println!("\n🤝 Example 2: Coordinated Task Execution using Registry");
    println!("=====================================================");

    // Initialize storage
    DeviceStorage::init_memory_only()
        .await
        .map_err(|e| terraphim_multi_agent::MultiAgentError::PersistenceError(e.to_string()))?;
    let storage = DeviceStorage::instance()
        .await
        .map_err(|e| terraphim_multi_agent::MultiAgentError::PersistenceError(e.to_string()))?;

    use std::ptr;
    let storage_copy = unsafe { ptr::read(storage) };
    let persistence = Arc::new(storage_copy);

    // Create registry and register specialized agents
    let registry = AgentRegistry::new();
    let roles = create_specialized_roles();

    for role in roles {
        let agent = TerraphimAgent::new(role, persistence.clone(), None).await?;
        agent.initialize().await?;
        let agent_arc = Arc::new(agent);
        registry.register_agent(agent_arc).await?;
    }

    // Task: Create and review a Rust function
    let task = "Create a Rust function that calculates the factorial of a number";

    println!("🎯 Collaborative Task: {}", task);
    println!();

    // Get all agents from registry for coordination
    let all_agents = registry.get_all_agents().await;
    if all_agents.len() < 3 {
        println!("⚠️ Need at least 3 agents for coordinated execution");
        return Ok(());
    }

    // Step 1: Code generation (using first agent)
    println!("👨‍💻 Step 1: Code Generation");
    let code_input = CommandInput::new(task.to_string(), CommandType::Generate);
    let code_result = all_agents[0].process_command(code_input).await?;
    println!("Generated code:\n{}\n", code_result.text);

    // Step 2: Code review (using agents with code review capability)
    println!("🔍 Step 2: Code Review");
    let code_review_agents = registry.find_agents_by_capability("code_review").await;
    if !code_review_agents.is_empty() {
        if let Some(reviewer_agent) = registry.get_agent(&code_review_agents[0]).await {
            let review_input = CommandInput::new(
                format!(
                    "Review this Rust code for quality and security:\n{}",
                    code_result.text
                ),
                CommandType::Review,
            );
            let review_result = reviewer_agent.process_command(review_input).await?;
            println!("Review feedback:\n{}\n", review_result.text);
        }
    } else {
        println!("No code review agents found, using general agent");
        let review_input = CommandInput::new(
            format!(
                "Review this Rust code for quality and security:\n{}",
                code_result.text
            ),
            CommandType::Review,
        );
        let review_result = all_agents[0].process_command(review_input).await?;
        println!("Review feedback:\n{}\n", review_result.text);
    }

    // Step 3: Documentation (using documentation agents)
    println!("📝 Step 3: Documentation Generation");
    let doc_agents = registry.find_agents_by_capability("documentation").await;
    if !doc_agents.is_empty() {
        if let Some(doc_agent) = registry.get_agent(&doc_agents[0]).await {
            let doc_input = CommandInput::new(
                format!(
                    "Create documentation for this Rust function:\n{}",
                    code_result.text
                ),
                CommandType::Generate,
            );
            let doc_result = doc_agent.process_command(doc_input).await?;
            println!("Documentation:\n{}\n", doc_result.text);
        }
    } else {
        println!("No documentation agents found, using general agent");
        let doc_input = CommandInput::new(
            format!(
                "Create documentation for this Rust function:\n{}",
                code_result.text
            ),
            CommandType::Generate,
        );
        let doc_result = all_agents[1].process_command(doc_input).await?;
        println!("Documentation:\n{}\n", doc_result.text);
    }

    // Step 4: Performance analysis (using performance agents)
    println!("⚡ Step 4: Performance Analysis");
    let perf_agents = registry
        .find_agents_by_capability("performance_analysis")
        .await;
    if !perf_agents.is_empty() {
        if let Some(perf_agent) = registry.get_agent(&perf_agents[0]).await {
            let perf_input = CommandInput::new(
                format!(
                    "Analyze the performance of this Rust function and suggest optimizations:\n{}",
                    code_result.text
                ),
                CommandType::Analyze,
            );
            let perf_result = perf_agent.process_command(perf_input).await?;
            println!("Performance analysis:\n{}\n", perf_result.text);
        }
    } else {
        println!("No performance agents found, using general agent");
        let perf_input = CommandInput::new(
            format!(
                "Analyze the performance of this Rust function and suggest optimizations:\n{}",
                code_result.text
            ),
            CommandType::Analyze,
        );
        let perf_result = all_agents[2].process_command(perf_input).await?;
        println!("Performance analysis:\n{}\n", perf_result.text);
    }

    println!("✅ Collaborative task completed with registry-based multi-agent coordination!");

    Ok(())
}

/// Example 3: Parallel Agent Processing
async fn example_parallel_processing() -> MultiAgentResult<()> {
    println!("\n⚡ Example 3: Parallel Agent Processing");
    println!("=======================================");

    // Initialize storage
    DeviceStorage::init_memory_only()
        .await
        .map_err(|e| terraphim_multi_agent::MultiAgentError::PersistenceError(e.to_string()))?;
    let storage = DeviceStorage::instance()
        .await
        .map_err(|e| terraphim_multi_agent::MultiAgentError::PersistenceError(e.to_string()))?;

    use std::ptr;
    let storage_copy = unsafe { ptr::read(storage) };
    let persistence = Arc::new(storage_copy);

    // Create multiple instances of the same agent for parallel processing
    let mut agents = Vec::new();
    for i in 0..3 {
        let role = create_test_role();
        let agent = TerraphimAgent::new(role, persistence.clone(), None).await?;
        agent.initialize().await?;
        agents.push(agent);
        println!("✅ Created agent {} for parallel processing", i + 1);
    }

    // Parallel tasks
    let tasks = vec![
        "Explain async/await in Rust",
        "Describe Rust ownership rules",
        "Explain Rust error handling patterns",
    ];

    println!("\n🚀 Processing {} tasks in parallel...", tasks.len());

    // Execute tasks in parallel using tokio::join!
    let futures = tasks.into_iter().enumerate().map(|(i, task)| {
        let agent = &agents[i];
        let input = CommandInput::new(task.to_string(), CommandType::Answer);
        async move {
            let start = std::time::Instant::now();
            let result = agent.process_command(input).await;
            let duration = start.elapsed();
            (i + 1, task, result, duration)
        }
    });

    // Wait for all tasks to complete
    let results = futures::future::join_all(futures).await;

    // Display results
    for (agent_num, task, result, duration) in results {
        match result {
            Ok(output) => {
                println!(
                    "✅ Agent {} completed in {:?}: {}",
                    agent_num, duration, task
                );
                println!("   Response: {}\n", output.text);
            }
            Err(e) => {
                println!("❌ Agent {} failed: {:?}\n", agent_num, e);
            }
        }
    }

    println!("🎉 Parallel processing completed!");

    Ok(())
}

/// Example 4: Agent Performance Comparison
async fn example_performance_comparison() -> MultiAgentResult<()> {
    println!("\n📊 Example 4: Agent Performance Comparison");
    println!("==========================================");

    // Initialize storage
    DeviceStorage::init_memory_only()
        .await
        .map_err(|e| terraphim_multi_agent::MultiAgentError::PersistenceError(e.to_string()))?;
    let storage = DeviceStorage::instance()
        .await
        .map_err(|e| terraphim_multi_agent::MultiAgentError::PersistenceError(e.to_string()))?;

    use std::ptr;
    let storage_copy = unsafe { ptr::read(storage) };
    let persistence = Arc::new(storage_copy);

    // Create agents with different configurations
    let roles = create_specialized_roles();
    let mut agents = Vec::new();

    for role in roles {
        let agent_name = role.name.clone();
        let agent = TerraphimAgent::new(role, persistence.clone(), None).await?;
        agent.initialize().await?;
        agents.push((agent_name, agent));
    }

    // Common task for comparison
    let task = "Explain the benefits of using Rust for systems programming";

    println!("🎯 Common Task: {}", task);
    println!();

    // Process task with each agent and compare metrics
    for (name, agent) in &agents {
        println!("🤖 Testing agent: {}", name);

        let input = CommandInput::new(task.to_string(), CommandType::Answer);
        let start = std::time::Instant::now();
        let _output = agent.process_command(input).await?;
        let duration = start.elapsed();

        // Get tracking information
        let token_tracker = agent.token_tracker.read().await;
        let cost_tracker = agent.cost_tracker.read().await;

        println!("   ⏱️  Duration: {:?}", duration);
        println!(
            "   🎫 Tokens: {} in, {} out",
            token_tracker.total_input_tokens, token_tracker.total_output_tokens
        );
        println!("   💰 Cost: ${:.6}", cost_tracker.current_month_spending);
        println!();
    }

    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🤝 Terraphim Multi-Agent Coordination Examples");
    println!("===============================================\n");

    // Run all coordination examples
    example_agent_registry().await?;
    example_coordinated_execution().await?;
    example_parallel_processing().await?;
    example_performance_comparison().await?;

    println!("\n✅ All coordination examples completed successfully!");
    println!("🎉 Multi-agent coordination is working perfectly!");

    Ok(())
}
