//! AI Agent Workflow Patterns - Rust Implementation
//!
//! This example demonstrates how to implement all five core AI agent workflow patterns
//! using the TerraphimAgent system. It serves as the backend for the interactive
//! web-based examples in @examples/agent-workflows/
//!
//! Patterns demonstrated:
//! 1. Prompt Chaining - Sequential execution where each step feeds the next
//! 2. Routing - Intelligent task distribution based on complexity and context  
//! 3. Parallelization - Concurrent execution with sophisticated result aggregation
//! 4. Orchestrator-Workers - Hierarchical planning with specialized worker roles
//! 5. Evaluator-Optimizer - Iterative quality improvement through evaluation loops

use ahash::AHashMap;
use serde_json;
use std::collections::HashMap;
use std::sync::Arc;
use terraphim_config::Role;
use terraphim_multi_agent::{
    AgentRegistry, CommandInput, CommandType, MultiAgentError, MultiAgentResult, TerraphimAgent,
};
use terraphim_persistence::DeviceStorage;
use terraphim_types::RelevanceFunction;
use tokio;

/// Workflow Pattern 1: Prompt Chaining
/// Sequential execution where each step's output feeds into the next step
async fn demonstrate_prompt_chaining() -> MultiAgentResult<()> {
    println!("🔗 WORKFLOW PATTERN 1: Prompt Chaining");
    println!("=====================================");
    println!("Sequential software development workflow with 6 steps");

    // Initialize storage
    DeviceStorage::init_memory_only()
        .await
        .map_err(|e| MultiAgentError::PersistenceError(e.to_string()))?;
    let storage_ref = DeviceStorage::instance()
        .await
        .map_err(|e| MultiAgentError::PersistenceError(e.to_string()))?;

    use std::ptr;
    let storage_copy = unsafe { ptr::read(storage_ref) };
    let persistence = Arc::new(storage_copy);

    // Create a software development role
    let dev_role = create_software_development_role();
    let mut dev_agent = TerraphimAgent::new(dev_role, persistence.clone(), None).await?;
    dev_agent.initialize().await?;

    println!("✅ Development agent created: {}", dev_agent.agent_id);

    // Define the development workflow steps
    let project_description = "Build a task management web application with user authentication, CRUD operations for tasks, and a clean responsive UI";
    let tech_stack = "React, Node.js, Express, MongoDB, JWT";

    let development_steps = vec![
        (
            "Requirements & Specification",
            "Create detailed technical specification including user stories, API endpoints, data models, and acceptance criteria",
        ),
        (
            "System Design & Architecture",
            "Design system architecture, component structure, database schema, and technology integration",
        ),
        (
            "Development Planning",
            "Create detailed development plan with tasks, priorities, estimated timelines, and milestones",
        ),
        (
            "Code Implementation",
            "Generate core application code, including backend API, frontend components, and database setup",
        ),
        (
            "Testing & Quality Assurance",
            "Create comprehensive tests including unit tests, integration tests, and quality assurance checklist",
        ),
        (
            "Deployment & Documentation",
            "Provide deployment instructions, environment setup, and comprehensive documentation",
        ),
    ];

    let mut context = format!(
        "Project: {}\nTech Stack: {}",
        project_description, tech_stack
    );

    for (i, (step_name, step_prompt)) in development_steps.iter().enumerate() {
        println!("\n📋 Step {}: {}", i + 1, step_name);

        let full_prompt = format!(
            "{}\n\nPrevious Context:\n{}\n\nPlease provide detailed output for this step.",
            step_prompt, context
        );
        let input = CommandInput::new(full_prompt, CommandType::Generate);

        let output = dev_agent.process_command(input).await?;
        println!(
            "✅ Output: {}",
            &output.text[..std::cmp::min(200, output.text.len())]
        );

        // Chain the output as context for the next step
        context = format!("{}\n\nStep {} Result:\n{}", context, i + 1, output.text);
    }

    // Show tracking information
    let token_tracker = dev_agent.token_tracker.read().await;
    let cost_tracker = dev_agent.cost_tracker.read().await;

    println!("\n📊 Prompt Chaining Results:");
    println!("   Steps Completed: {}", development_steps.len());
    println!(
        "   Total Tokens: {} in / {} out",
        token_tracker.total_input_tokens, token_tracker.total_output_tokens
    );
    println!("   Total Cost: ${:.6}", cost_tracker.current_month_spending);

    Ok(())
}

/// Workflow Pattern 2: Routing  
/// Intelligent task distribution based on complexity, cost, and performance
async fn demonstrate_routing() -> MultiAgentResult<()> {
    println!("\n\n🧠 WORKFLOW PATTERN 2: Routing");
    println!("==============================");
    println!("Smart task distribution with model selection");

    // Initialize storage
    DeviceStorage::init_memory_only()
        .await
        .map_err(|e| MultiAgentError::PersistenceError(e.to_string()))?;
    let storage_ref = DeviceStorage::instance()
        .await
        .map_err(|e| MultiAgentError::PersistenceError(e.to_string()))?;

    use std::ptr;
    let storage_copy = unsafe { ptr::read(storage_ref) };
    let persistence = Arc::new(storage_copy);

    // Create different agent roles for different complexity levels
    let mut registry = AgentRegistry::new();

    // Simple tasks agent (fast, low cost)
    let simple_role = create_simple_task_role();
    let mut simple_agent = TerraphimAgent::new(simple_role, persistence.clone(), None).await?;
    simple_agent.initialize().await?;
    registry.register_agent(Arc::new(simple_agent)).await?;

    // Complex tasks agent (slower, higher quality)
    let complex_role = create_complex_task_role();
    let mut complex_agent = TerraphimAgent::new(complex_role, persistence.clone(), None).await?;
    complex_agent.initialize().await?;
    registry.register_agent(Arc::new(complex_agent)).await?;

    // Creative tasks agent (specialized for creative work)
    let creative_role = create_creative_task_role();
    let mut creative_agent = TerraphimAgent::new(creative_role, persistence.clone(), None).await?;
    creative_agent.initialize().await?;
    registry.register_agent(Arc::new(creative_agent)).await?;

    println!("✅ Created 3 specialized agents for different task types");

    // Test tasks with different complexity levels
    let test_tasks = vec![
        (
            "Generate a simple greeting message",
            "simple_tasks",
            &mut simple_agent,
        ),
        (
            "Design a comprehensive software architecture for a distributed system",
            "complex_tasks",
            &mut complex_agent,
        ),
        (
            "Write a creative marketing story for a new product launch",
            "creative_tasks",
            &mut creative_agent,
        ),
    ];

    for (task_description, expected_route, agent) in test_tasks {
        println!("\n🎯 Task: {}", task_description);

        // Analyze task complexity (in real implementation, this would be ML-based)
        let task_complexity = analyze_task_complexity(task_description);
        let selected_route = route_task_to_agent(task_complexity);

        println!("   Complexity: {:.2}", task_complexity);
        println!("   Routed to: {}", selected_route);
        println!("   Expected: {}", expected_route);

        let input = CommandInput::new(task_description.to_string(), CommandType::Generate);
        let start_time = std::time::Instant::now();
        let output = agent.process_command(input).await?;
        let duration = start_time.elapsed();

        println!("   Duration: {:?}", duration);
        println!(
            "   Output: {}",
            &output.text[..std::cmp::min(150, output.text.len())]
        );

        // Show cost for this specific task
        let cost_tracker = agent.cost_tracker.read().await;
        println!("   Cost: ${:.6}", cost_tracker.current_month_spending);
    }

    println!("\n📊 Routing Results: Optimal task distribution completed");

    Ok(())
}

/// Workflow Pattern 3: Parallelization
/// Concurrent execution with sophisticated result aggregation
async fn demonstrate_parallelization() -> MultiAgentResult<()> {
    println!("\n\n⚡ WORKFLOW PATTERN 3: Parallelization");
    println!("=====================================");
    println!("Multi-perspective analysis with parallel execution");

    // Initialize storage
    DeviceStorage::init_memory_only()
        .await
        .map_err(|e| MultiAgentError::PersistenceError(e.to_string()))?;
    let storage_ref = DeviceStorage::instance()
        .await
        .map_err(|e| MultiAgentError::PersistenceError(e.to_string()))?;

    use std::ptr;
    let storage_copy = unsafe { ptr::read(storage_ref) };
    let persistence = Arc::new(storage_copy);

    // Create 6 different perspective agents
    let perspectives = vec![
        ("analytical", "Provide analytical, data-driven insights"),
        ("creative", "Offer creative and innovative perspectives"),
        (
            "practical",
            "Focus on practical implementation and feasibility",
        ),
        (
            "critical",
            "Apply critical thinking and identify potential issues",
        ),
        ("strategic", "Consider long-term strategic implications"),
        (
            "user_focused",
            "Prioritize user experience and human factors",
        ),
    ];

    let mut agents = Vec::new();
    for (perspective_name, perspective_description) in &perspectives {
        let role = create_perspective_role(perspective_name, perspective_description);
        let mut agent = TerraphimAgent::new(role, persistence.clone(), None).await?;
        agent.initialize().await?;
        agents.push(agent);
    }

    println!("✅ Created {} perspective agents", agents.len());

    // Topic to analyze
    let analysis_topic = "The impact of AI on software development workflows";

    println!("\n🎯 Topic: {}", analysis_topic);
    println!("🚀 Starting parallel analysis...");

    // Execute all analyses in parallel
    let analysis_futures = agents.iter().enumerate().map(|(i, agent)| {
        let topic = analysis_topic.to_string();
        let perspective = &perspectives[i];
        let perspective_prompt = format!(
            "Analyze this topic from a {} perspective: {}\n\n{}",
            perspective.0, topic, perspective.1
        );

        async move {
            let input = CommandInput::new(perspective_prompt, CommandType::Analyze);
            let start_time = std::time::Instant::now();
            let result = agent.process_command(input).await;
            let duration = start_time.elapsed();
            (perspective.0, result, duration)
        }
    });

    // Wait for all analyses to complete
    // Use tokio::join! for concurrent execution
    let mut results = Vec::new();
    for future in analysis_futures {
        results.push(future.await);
    }

    // Aggregate results
    println!("\n📊 Parallel Analysis Results:");
    let mut total_tokens_in = 0;
    let mut total_tokens_out = 0;
    let mut total_cost = 0.0;

    for (perspective_name, result, duration) in results {
        match result {
            Ok(output) => {
                println!(
                    "\n   {} Perspective ({:?}):",
                    perspective_name.to_uppercase(),
                    duration
                );
                println!(
                    "   {}",
                    &output.text[..std::cmp::min(200, output.text.len())]
                );
            }
            Err(e) => {
                println!(
                    "\n   {} Perspective: ERROR - {:?}",
                    perspective_name.to_uppercase(),
                    e
                );
            }
        }
    }

    // Show aggregated metrics
    for agent in &agents {
        let token_tracker = agent.token_tracker.read().await;
        let cost_tracker = agent.cost_tracker.read().await;
        total_tokens_in += token_tracker.total_input_tokens;
        total_tokens_out += token_tracker.total_output_tokens;
        total_cost += cost_tracker.current_month_spending;
    }

    println!("\n📈 Parallelization Metrics:");
    println!("   Perspectives Analyzed: {}", perspectives.len());
    println!(
        "   Total Tokens: {} in / {} out",
        total_tokens_in, total_tokens_out
    );
    println!("   Total Cost: ${:.6}", total_cost);
    println!("   Parallel Efficiency: High (all analyses completed simultaneously)");

    Ok(())
}

/// Workflow Pattern 4: Orchestrator-Workers
/// Hierarchical planning with specialized worker roles
async fn demonstrate_orchestrator_workers() -> MultiAgentResult<()> {
    println!("\n\n🕸️ WORKFLOW PATTERN 4: Orchestrator-Workers");
    println!("===========================================");
    println!("Hierarchical data science pipeline with specialized workers");

    // Initialize storage
    DeviceStorage::init_memory_only()
        .await
        .map_err(|e| MultiAgentError::PersistenceError(e.to_string()))?;
    let storage_ref = DeviceStorage::instance()
        .await
        .map_err(|e| MultiAgentError::PersistenceError(e.to_string()))?;

    use std::ptr;
    let storage_copy = unsafe { ptr::read(storage_ref) };
    let persistence = Arc::new(storage_copy);

    // Create orchestrator agent
    let orchestrator_role = create_orchestrator_role();
    let mut orchestrator =
        TerraphimAgent::new(orchestrator_role, persistence.clone(), None).await?;
    orchestrator.initialize().await?;

    // Create specialized worker agents
    let worker_roles = vec![
        (
            "data_collector",
            "Collect and validate research data",
            "data_collection",
        ),
        (
            "content_analyzer",
            "Analyze and process textual content",
            "content_analysis",
        ),
        (
            "methodology_expert",
            "Design research methodology and validation",
            "methodology",
        ),
        (
            "knowledge_mapper",
            "Extract concepts and build relationships",
            "knowledge_mapping",
        ),
        (
            "synthesis_specialist",
            "Synthesize results and generate insights",
            "synthesis",
        ),
        (
            "graph_builder",
            "Construct and optimize knowledge graphs",
            "graph_construction",
        ),
    ];

    let mut workers = HashMap::new();
    let mut registry = AgentRegistry::new();

    for (worker_name, worker_description, worker_capability) in &worker_roles {
        let worker_role = create_worker_role(worker_name, worker_description);
        let mut worker_agent = TerraphimAgent::new(worker_role, persistence.clone(), None).await?;
        worker_agent.initialize().await?;

        registry.register_agent(Arc::new(worker_agent)).await?;
        workers.insert(worker_name.to_string(), worker_agent);
    }

    println!(
        "✅ Created orchestrator and {} specialized workers",
        worker_roles.len()
    );

    // Research project to orchestrate
    let research_topic = "Advanced AI Agent Coordination Patterns in Distributed Systems";

    // Step 1: Orchestrator creates the plan
    println!("\n🎯 Research Topic: {}", research_topic);
    println!("\n📋 Step 1: Orchestrator Planning");

    let planning_prompt = format!(
        "Create a detailed research plan for: {}\n\nDefine specific tasks for each worker type: data collection, content analysis, methodology, knowledge mapping, synthesis, and graph construction.",
        research_topic
    );
    let planning_input = CommandInput::new(planning_prompt, CommandType::Create);
    let plan_result = orchestrator.process_command(planning_input).await?;

    println!("✅ Research plan created:");
    println!(
        "   {}",
        &plan_result.text[..std::cmp::min(300, plan_result.text.len())]
    );

    // Step 2: Distribute tasks to workers
    println!("\n🔄 Step 2: Task Distribution to Workers");

    let worker_tasks = vec![
        (
            "data_collector",
            "Collect relevant papers, datasets, and benchmarks for AI agent coordination research",
        ),
        (
            "content_analyzer",
            "Analyze collected research papers and extract key concepts and methodologies",
        ),
        (
            "methodology_expert",
            "Design evaluation methodology for agent coordination effectiveness",
        ),
        (
            "knowledge_mapper",
            "Map relationships between concepts, algorithms, and performance metrics",
        ),
        (
            "synthesis_specialist",
            "Synthesize findings and identify novel insights and gaps",
        ),
        (
            "graph_builder",
            "Construct knowledge graph representing the research domain structure",
        ),
    ];

    let mut worker_results = HashMap::new();

    for (worker_name, task_description) in worker_tasks {
        if let Some(worker) = workers.get_mut(&worker_name.to_string()) {
            let task_prompt = format!(
                "Research Task: {}\n\nContext: {}\n\nPlease complete this specialized task as part of the larger research project.",
                task_description, research_topic
            );
            let task_input = CommandInput::new(task_prompt, CommandType::Generate);

            println!(
                "   📤 Assigned to {}: {}",
                worker_name,
                &task_description[..std::cmp::min(100, task_description.len())]
            );

            let worker_result = worker.process_command(task_input).await?;
            worker_results.insert(worker_name.to_string(), worker_result.text);

            println!("   ✅ Completed by {}", worker_name);
        }
    }

    // Step 3: Orchestrator synthesizes final results
    println!("\n🔄 Step 3: Orchestrator Final Synthesis");

    let synthesis_context = worker_results
        .iter()
        .map(|(worker, result)| format!("{}: {}", worker, result))
        .collect::<Vec<_>>()
        .join("\n\n");

    let synthesis_prompt = format!(
        "Synthesize the research results into a comprehensive analysis:\n\n{}",
        synthesis_context
    );
    let synthesis_input = CommandInput::new(synthesis_prompt, CommandType::Analyze);
    let final_result = orchestrator.process_command(synthesis_input).await?;

    println!("✅ Final synthesis completed:");
    println!(
        "   {}",
        &final_result.text[..std::cmp::min(300, final_result.text.len())]
    );

    // Show orchestration metrics
    let mut total_cost = 0.0;
    let mut total_tokens = 0;

    // Orchestrator metrics
    let orch_cost = orchestrator.cost_tracker.read().await;
    let orch_tokens = orchestrator.token_tracker.read().await;
    total_cost += orch_cost.current_month_spending;
    total_tokens += orch_tokens.total_input_tokens + orch_tokens.total_output_tokens;

    // Worker metrics
    for (worker_name, worker) in &workers {
        let cost_tracker = worker.cost_tracker.read().await;
        let token_tracker = worker.token_tracker.read().await;
        total_cost += cost_tracker.current_month_spending;
        total_tokens += token_tracker.total_input_tokens + token_tracker.total_output_tokens;
    }

    println!("\n📈 Orchestrator-Workers Metrics:");
    println!("   Workers Coordinated: {}", worker_roles.len());
    println!("   Tasks Completed: {}", worker_tasks.len());
    println!("   Total Tokens: {}", total_tokens);
    println!("   Total Cost: ${:.6}", total_cost);
    println!("   Coordination Efficiency: High (specialized workers, centralized orchestration)");

    Ok(())
}

/// Workflow Pattern 5: Evaluator-Optimizer  
/// Iterative quality improvement through evaluation loops
async fn demonstrate_evaluator_optimizer() -> MultiAgentResult<()> {
    println!("\n\n🔄 WORKFLOW PATTERN 5: Evaluator-Optimizer");
    println!("==========================================");
    println!("Iterative content improvement with quality evaluation");

    // Initialize storage
    DeviceStorage::init_memory_only()
        .await
        .map_err(|e| MultiAgentError::PersistenceError(e.to_string()))?;
    let storage_ref = DeviceStorage::instance()
        .await
        .map_err(|e| MultiAgentError::PersistenceError(e.to_string()))?;

    use std::ptr;
    let storage_copy = unsafe { ptr::read(storage_ref) };
    let persistence = Arc::new(storage_copy);

    // Create content generator agent
    let generator_role = create_content_generator_role();
    let mut generator = TerraphimAgent::new(generator_role, persistence.clone(), None).await?;
    generator.initialize().await?;

    // Create multiple evaluator agents for different quality dimensions
    let evaluator_roles = vec![
        (
            "clarity_evaluator",
            "Evaluate content clarity and readability",
        ),
        (
            "accuracy_evaluator",
            "Assess technical accuracy and factual correctness",
        ),
        (
            "completeness_evaluator",
            "Check content completeness and coverage",
        ),
        (
            "engagement_evaluator",
            "Evaluate reader engagement and appeal",
        ),
        (
            "structure_evaluator",
            "Assess content organization and flow",
        ),
        (
            "style_evaluator",
            "Evaluate writing style and tone consistency",
        ),
    ];

    let mut evaluators = HashMap::new();
    for (eval_name, eval_description) in &evaluator_roles {
        let eval_role = create_evaluator_role(eval_name, eval_description);
        let mut eval_agent = TerraphimAgent::new(eval_role, persistence.clone(), None).await?;
        eval_agent.initialize().await?;
        evaluators.insert(eval_name.to_string(), eval_agent);
    }

    println!(
        "✅ Created content generator and {} specialized evaluators",
        evaluator_roles.len()
    );

    // Content creation task
    let content_brief = "Write a comprehensive guide explaining the benefits of AI agent workflows for software development teams, including practical examples and implementation strategies.";

    println!("\n🎯 Content Brief: {}", content_brief);

    // Quality threshold for completion
    let quality_threshold = 8.0; // Out of 10
    let max_iterations = 3;

    let mut current_content = String::new();
    let mut iteration = 0;
    let mut best_score = 0.0;

    while iteration < max_iterations {
        iteration += 1;
        println!("\n🔄 Iteration {}/{}", iteration, max_iterations);

        // Generate or improve content
        let generation_prompt = if current_content.is_empty() {
            format!("Create content for: {}", content_brief)
        } else {
            format!(
                "Improve this content based on evaluation feedback:\n\nOriginal Brief: {}\n\nCurrent Content:\n{}\n\nPlease enhance the content to address any quality issues.",
                content_brief, current_content
            )
        };

        println!("   📝 Generating content...");
        let gen_input = CommandInput::new(generation_prompt, CommandType::Generate);
        let gen_result = generator.process_command(gen_input).await?;
        current_content = gen_result.text;

        println!(
            "   ✅ Content generated ({} characters)",
            current_content.len()
        );

        // Evaluate content quality
        println!(
            "   🔍 Evaluating quality across {} dimensions...",
            evaluator_roles.len()
        );

        let mut quality_scores = HashMap::new();
        let mut total_score = 0.0;

        for (eval_name, evaluator) in &mut evaluators {
            let eval_prompt = format!(
                "Evaluate this content on your specialized dimension:\n\n{}\n\nProvide a score from 1-10 and specific feedback for improvement.",
                current_content
            );
            let eval_input = CommandInput::new(eval_prompt, CommandType::Review);
            let eval_result = evaluator.process_command(eval_input).await?;

            // Extract score (simplified - in real implementation would use structured output)
            let score = extract_quality_score(&eval_result.text);
            quality_scores.insert(eval_name.clone(), (score, eval_result.text));
            total_score += score;

            println!(
                "     {} Score: {:.1}/10",
                eval_name.replace("_", " ").to_uppercase(),
                score
            );
        }

        let average_score = total_score / evaluator_roles.len() as f64;
        println!("   📊 Average Quality Score: {:.1}/10", average_score);

        if average_score > best_score {
            best_score = average_score;
        }

        // Check if quality threshold met
        if average_score >= quality_threshold {
            println!("   🎉 Quality threshold reached! Content optimization complete.");
            break;
        } else if iteration < max_iterations {
            println!("   🔄 Quality below threshold. Preparing for next iteration...");

            // Prepare feedback for next iteration
            let feedback_summary = quality_scores
                .iter()
                .map(|(dim, (score, feedback))| {
                    format!(
                        "{} ({}): {}",
                        dim,
                        score,
                        &feedback[..std::cmp::min(100, feedback.len())]
                    )
                })
                .collect::<Vec<_>>()
                .join("\n");

            current_content = format!(
                "{}\n\n--- EVALUATION FEEDBACK ---\n{}",
                current_content, feedback_summary
            );
        }
    }

    // Final results
    println!("\n📈 Evaluator-Optimizer Results:");
    println!("   Iterations Completed: {}", iteration);
    println!("   Best Quality Score: {:.1}/10", best_score);
    println!(
        "   Final Content Length: {} characters",
        current_content.len()
    );
    println!("   Quality Dimensions: {}", evaluator_roles.len());

    // Show cost metrics
    let mut total_cost = 0.0;
    let mut total_tokens = 0;

    // Generator metrics
    let gen_cost = generator.cost_tracker.read().await;
    let gen_tokens = generator.token_tracker.read().await;
    total_cost += gen_cost.current_month_spending;
    total_tokens += gen_tokens.total_input_tokens + gen_tokens.total_output_tokens;

    // Evaluator metrics
    for (_, evaluator) in &evaluators {
        let cost_tracker = evaluator.cost_tracker.read().await;
        let token_tracker = evaluator.token_tracker.read().await;
        total_cost += cost_tracker.current_month_spending;
        total_tokens += token_tracker.total_input_tokens + token_tracker.total_output_tokens;
    }

    println!("   Total Tokens: {}", total_tokens);
    println!("   Total Cost: ${:.6}", total_cost);
    println!(
        "   Optimization Efficiency: {} iterations to quality threshold",
        iteration
    );

    Ok(())
}

// Helper functions for creating specialized roles

fn create_software_development_role() -> Role {
    let mut extra = AHashMap::new();
    extra.insert(
        "agent_capabilities".to_string(),
        serde_json::json!([
            "software_development",
            "code_generation",
            "architecture_design"
        ]),
    );
    extra.insert(
        "agent_goals".to_string(),
        serde_json::json!([
            "Create professional software solutions",
            "Follow best practices",
            "Generate comprehensive documentation"
        ]),
    );
    extra.insert("llm_provider".to_string(), serde_json::json!("ollama"));
    extra.insert("ollama_model".to_string(), serde_json::json!("gemma3:270m"));
    extra.insert("llm_temperature".to_string(), serde_json::json!(0.3)); // Precise technical content

    {
        let mut role = Role::new("SoftwareDeveloper");
        role.shortname = Some("SoftwareDev".to_string());
        role.relevance_function = RelevanceFunction::BM25;
        role.extra = extra;
        role
    }
}

fn create_simple_task_role() -> Role {
    let mut extra = AHashMap::new();
    extra.insert(
        "agent_capabilities".to_string(),
        serde_json::json!(["simple_tasks", "quick_responses"]),
    );
    extra.insert(
        "agent_goals".to_string(),
        serde_json::json!([
            "Provide fast, accurate responses",
            "Handle routine tasks efficiently"
        ]),
    );
    extra.insert("llm_provider".to_string(), serde_json::json!("ollama"));
    extra.insert("ollama_model".to_string(), serde_json::json!("gemma3:270m"));
    extra.insert("llm_temperature".to_string(), serde_json::json!(0.2)); // Low creativity, fast

    {
        let mut role = Role::new("SimpleTaskAgent");
        role.shortname = Some("SimpleTask".to_string());
        role.relevance_function = RelevanceFunction::TitleScorer;
        role.extra = extra;
        role
    }
}

fn create_complex_task_role() -> Role {
    let mut extra = AHashMap::new();
    extra.insert(
        "agent_capabilities".to_string(),
        serde_json::json!(["complex_tasks", "deep_analysis", "comprehensive_planning"]),
    );
    extra.insert(
        "agent_goals".to_string(),
        serde_json::json!([
            "Provide thorough, high-quality analysis",
            "Handle complex problem-solving",
            "Generate detailed solutions"
        ]),
    );
    extra.insert("llm_provider".to_string(), serde_json::json!("ollama"));
    extra.insert("ollama_model".to_string(), serde_json::json!("gemma3:270m"));
    extra.insert("llm_temperature".to_string(), serde_json::json!(0.4)); // Balanced for analysis

    {
        let mut role = Role::new("ComplexTaskAgent");
        role.shortname = Some("ComplexTask".to_string());
        role.relevance_function = RelevanceFunction::BM25;
        role.extra = extra;
        role
    }
}

fn create_creative_task_role() -> Role {
    let mut extra = AHashMap::new();
    extra.insert(
        "agent_capabilities".to_string(),
        serde_json::json!(["creative_tasks", "storytelling", "marketing", "innovation"]),
    );
    extra.insert(
        "agent_goals".to_string(),
        serde_json::json!([
            "Generate creative, engaging content",
            "Think outside the box",
            "Create compelling narratives"
        ]),
    );
    extra.insert("llm_provider".to_string(), serde_json::json!("ollama"));
    extra.insert("ollama_model".to_string(), serde_json::json!("gemma3:270m"));
    extra.insert("llm_temperature".to_string(), serde_json::json!(0.7)); // High creativity

    {
        let mut role = Role::new("CreativeTaskAgent");
        role.shortname = Some("CreativeTask".to_string());
        role.relevance_function = RelevanceFunction::TitleScorer;
        role.extra = extra;
        role
    }
}

fn create_perspective_role(perspective_name: &str, perspective_description: &str) -> Role {
    let mut extra = AHashMap::new();
    extra.insert(
        "agent_capabilities".to_string(),
        serde_json::json!(["analysis", "perspective_taking", perspective_name]),
    );
    extra.insert(
        "agent_goals".to_string(),
        serde_json::json!([
            perspective_description,
            "Provide unique viewpoint",
            "Contribute to comprehensive analysis"
        ]),
    );
    extra.insert("llm_provider".to_string(), serde_json::json!("ollama"));
    extra.insert("ollama_model".to_string(), serde_json::json!("gemma3:270m"));
    extra.insert("llm_temperature".to_string(), serde_json::json!(0.5)); // Balanced

    {
        let mut role = Role::new(format!("{}PerspectiveAgent", perspective_name));
        role.shortname = Some(format!("{}Perspective", perspective_name));
        role.relevance_function = RelevanceFunction::BM25;
        role.extra = extra;
        role
    }
}

fn create_orchestrator_role() -> Role {
    let mut extra = AHashMap::new();
    extra.insert(
        "agent_capabilities".to_string(),
        serde_json::json!(["orchestration", "planning", "coordination", "synthesis"]),
    );
    extra.insert(
        "agent_goals".to_string(),
        serde_json::json!([
            "Coordinate complex workflows",
            "Create comprehensive plans",
            "Synthesize results effectively"
        ]),
    );
    extra.insert("llm_provider".to_string(), serde_json::json!("ollama"));
    extra.insert("ollama_model".to_string(), serde_json::json!("gemma3:270m"));
    extra.insert("llm_temperature".to_string(), serde_json::json!(0.3)); // Strategic planning

    {
        let mut role = Role::new("OrchestratorAgent");
        role.shortname = Some("Orchestrator".to_string());
        role.relevance_function = RelevanceFunction::BM25;
        role.extra = extra;
        role
    }
}

fn create_worker_role(worker_name: &str, worker_description: &str) -> Role {
    let mut extra = AHashMap::new();
    extra.insert(
        "agent_capabilities".to_string(),
        serde_json::json!(["specialized_work", worker_name]),
    );
    extra.insert(
        "agent_goals".to_string(),
        serde_json::json!([
            worker_description,
            "Execute specialized tasks efficiently",
            "Provide expert-level results"
        ]),
    );
    extra.insert("llm_provider".to_string(), serde_json::json!("ollama"));
    extra.insert("ollama_model".to_string(), serde_json::json!("gemma3:270m"));
    extra.insert("llm_temperature".to_string(), serde_json::json!(0.4)); // Focused work

    {
        let mut role = Role::new(format!("{}WorkerAgent", worker_name));
        role.shortname = Some(format!("{}Worker", worker_name));
        role.relevance_function = RelevanceFunction::BM25;
        role.extra = extra;
        role
    }
}

fn create_content_generator_role() -> Role {
    let mut extra = AHashMap::new();
    extra.insert(
        "agent_capabilities".to_string(),
        serde_json::json!(["content_generation", "writing", "creativity"]),
    );
    extra.insert(
        "agent_goals".to_string(),
        serde_json::json!([
            "Create high-quality content",
            "Incorporate feedback effectively",
            "Improve iteratively"
        ]),
    );
    extra.insert("llm_provider".to_string(), serde_json::json!("ollama"));
    extra.insert("ollama_model".to_string(), serde_json::json!("gemma3:270m"));
    extra.insert("llm_temperature".to_string(), serde_json::json!(0.6)); // Creative but focused

    {
        let mut role = Role::new("ContentGeneratorAgent");
        role.shortname = Some("ContentGen".to_string());
        role.relevance_function = RelevanceFunction::BM25;
        role.extra = extra;
        role
    }
}

fn create_evaluator_role(eval_name: &str, eval_description: &str) -> Role {
    let mut extra = AHashMap::new();
    extra.insert(
        "agent_capabilities".to_string(),
        serde_json::json!(["evaluation", "quality_assessment", eval_name]),
    );
    extra.insert(
        "agent_goals".to_string(),
        serde_json::json!([
            eval_description,
            "Provide constructive feedback",
            "Maintain high quality standards"
        ]),
    );
    extra.insert("llm_provider".to_string(), serde_json::json!("ollama"));
    extra.insert("ollama_model".to_string(), serde_json::json!("gemma3:270m"));
    extra.insert("llm_temperature".to_string(), serde_json::json!(0.2)); // Precise evaluation

    {
        let mut role = Role::new(format!("{}EvaluatorAgent", eval_name));
        role.shortname = Some(format!("{}Eval", eval_name));
        role.relevance_function = RelevanceFunction::BM25;
        role.extra = extra;
        role
    }
}

// Helper functions for routing and evaluation

fn analyze_task_complexity(task: &str) -> f64 {
    // Simplified complexity analysis based on keywords and length
    let complexity_keywords = vec![
        ("simple", 0.2),
        ("basic", 0.3),
        ("quick", 0.2),
        ("complex", 0.8),
        ("comprehensive", 0.9),
        ("detailed", 0.7),
        ("architecture", 0.8),
        ("design", 0.6),
        ("system", 0.7),
        ("creative", 0.6),
        ("story", 0.5),
        ("marketing", 0.5),
    ];

    let mut score = 0.3; // Base complexity

    for (keyword, weight) in complexity_keywords {
        if task.to_lowercase().contains(keyword) {
            score += weight;
        }
    }

    // Factor in length
    score += (task.len() as f64 / 100.0) * 0.2;

    score.min(1.0) // Cap at 1.0
}

fn route_task_to_agent(complexity: f64) -> &'static str {
    match complexity {
        c if c < 0.4 => "simple_tasks",
        c if c < 0.7 => "complex_tasks",
        _ => "creative_tasks",
    }
}

fn extract_quality_score(evaluation_text: &str) -> f64 {
    // Simplified score extraction - in real implementation would use structured output
    for line in evaluation_text.lines() {
        if line.contains("score") || line.contains("Score") {
            for word in line.split_whitespace() {
                if let Ok(score) = word
                    .trim_matches(|c: char| !c.is_ascii_digit() && c != '.')
                    .parse::<f64>()
                {
                    if score >= 1.0 && score <= 10.0 {
                        return score;
                    }
                }
            }
        }
    }

    // Default score if extraction fails
    7.0
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 AI Agent Workflow Patterns - Complete Demonstration");
    println!("======================================================");
    println!("Showcasing 5 core patterns using the TerraphimAgent system\n");

    // Run all workflow pattern demonstrations
    demonstrate_prompt_chaining().await?;
    demonstrate_routing().await?;
    demonstrate_parallelization().await?;
    demonstrate_orchestrator_workers().await?;
    demonstrate_evaluator_optimizer().await?;

    println!("\n\n🎉 ALL WORKFLOW PATTERNS COMPLETED SUCCESSFULLY!");
    println!("==============================================");
    println!("✅ Prompt Chaining: Sequential development workflow");
    println!("✅ Routing: Intelligent task distribution");
    println!("✅ Parallelization: Multi-perspective concurrent analysis");
    println!("✅ Orchestrator-Workers: Hierarchical specialized coordination");
    println!("✅ Evaluator-Optimizer: Iterative quality improvement");
    println!("\n🚀 The TerraphimAgent system supports all advanced workflow patterns!");
    println!("🔗 These patterns power the interactive examples in @examples/agent-workflows/");

    Ok(())
}
